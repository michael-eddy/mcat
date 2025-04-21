use std::{
    fs::{self, File},
    io::{self, BufRead},
    path::Path,
};

use calamine::Reader;
use tempfile::Builder;
use zip::ZipArchive;

use crate::prompter;

pub fn convert(
    path: &Path,
    name_header: Option<&String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(format!("{} doesn't exists", path.display()).into());
    }
    if path.is_dir() {
        let content = dir_converter(path)?;
        return Ok(content);
    }
    if !path.is_file() {
        return Err(format!("Unknown path type for {}", path.display()).into());
    }

    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    let result = match ext.as_str() {
        "csv" => csv_converter(path)?,
        "docx" => todo!(),
        "pdf" => pdf_convert(path)?,
        "pptx" => todo!(),
        "xlsx" | "xls" | "xlsm" | "xlsb" | "xla" | "xlam" | "ods" => sheets_convert(path)?,
        "zip" => zip_convert(path)?,
        "odt" => todo!(),
        "odp" => todo!(),
        _ => {
            let content = fs::read_to_string(path)?;
            markitdown_fallback(&content, name_header, &ext)
        }
    };

    Ok(result)
}

fn pdf_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let doc = lopdf::Document::load(path)?;
    let mut result = String::new();

    let num_pages = doc.get_pages().len();
    for i in 1..=num_pages {
        let page_text = doc.extract_text(&[i as u32])?.replace("  ", " ");

        result.push_str(&format!("## Page {}\n\n", i));

        result.push_str(&page_text);
        result.push_str("\n\n");
    }

    let mut output = String::with_capacity(result.len());
    for line in result.lines() {
        output.push_str(line.trim());
        output.push('\n');
    }

    let output = output.replace("\n\n\n", "\0");
    let output = output.replace("\n\n", " ");
    let output = output.replace("\n", " ");
    let output = output.replace("\0", "\n\n\n");

    Ok(output)
}

fn zip_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut output = String::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let extension = Path::new(&name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        if entry.is_dir() {
            continue;
        }

        let mut temp = Builder::new()
            .suffix(&format!(".{}", extension))
            .tempfile()?;
        std::io::copy(&mut entry, &mut temp)?;
        let temp_path = temp.path().to_path_buf(); // clone path before move

        // convert using original convert function
        let md = convert(&temp_path, None).unwrap_or("**[Failed Reading]**".into());
        output += &format!("# `{}`\n\n{}\n\n", name, md);
    }

    Ok(output)
}

fn dir_converter(path: &Path) -> Result<String, String> {
    let mut selected_files = prompter::prompt_for_files(path)?;
    selected_files.sort();

    let mut markdown = String::new();
    for file in selected_files {
        let rel_path = file.strip_prefix(path).unwrap_or(path);
        let name = rel_path.to_string_lossy().into_owned();
        if let Ok(md) = convert(&file, Some(&name)) {
            markdown.push_str(&md);
            markdown.push_str("\n\n");
        } else {
            markdown.push_str("**[Failed Reading]**".into());
            markdown.push_str("\n\n");
        }
    }

    Ok(markdown.trim().to_string())
}

pub fn markitdown_fallback(content: &String, name: Option<&String>, ext: &String) -> String {
    let md = format!("```{}\n{}\n```", ext, content);

    match name {
        Some(name) => format!("# `{}`\n\n{}", name, md),
        None => md,
    }
}

fn detect_delimiter(line: &str) -> u8 {
    let candidates = [',', ';', '\t', '|'];
    candidates
        .iter()
        .map(|&c| (c, line.matches(c).count()))
        .max_by_key(|&(_, count)| count)
        .map(|(c, _)| c as u8)
        .unwrap_or(b',') // fallback to comma
}

fn to_markdown_table(headers: &[String], rows: &[Vec<String>]) -> String {
    let mut output = String::new();
    output += &format!("| {} |\n", headers.join(" | "));
    output += &format!("|{}|\n", vec!["---"; headers.len()].join("|"));

    for row in rows {
        output += &format!("| {} |\n", row.join(" | "));
    }

    output
}

pub fn sheets_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut workbook = calamine::open_workbook_auto(path)?;
    let mut output = String::new();

    for sheet_name in workbook.sheet_names().to_owned() {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            let mut rows = range.rows();
            if let Some(header_row) = rows.next() {
                let headers = header_row
                    .iter()
                    .map(|cell| cell.to_string())
                    .collect::<Vec<_>>();
                let body = rows
                    .map(|r| r.iter().map(|cell| cell.to_string()).collect::<Vec<_>>())
                    .collect::<Vec<_>>();

                output += &format!("# {}\n\n", sheet_name);
                output += &to_markdown_table(&headers, &body);
                output += "\n";
            }
        }
    }

    if output.is_empty() {
        Err("No readable sheets found.".into())
    } else {
        Ok(output)
    }
}

pub fn csv_converter(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut first_line = String::new();
    let _ = io::BufReader::new(&mut file).read_line(&mut first_line)?;

    let delimiter = detect_delimiter(&first_line);
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .from_path(path)?;

    let headers = reader
        .headers()?
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let rows = reader
        .records()
        .map(|r| r.map(|rec| rec.iter().map(|s| s.to_string()).collect::<Vec<_>>()))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(to_markdown_table(&headers, &rows))
}
