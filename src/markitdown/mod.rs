mod docx;
mod opendoc;
mod pdf;
mod pptx;
mod sheets;

use std::{
    fs::{self, File},
    path::Path,
};

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
        "csv" => sheets::csv_converter(path)?,
        "docx" => todo!(),
        "pdf" => pdf::pdf_convert(path)?,
        "pptx" => pptx::pptx_converter(path)?,
        "xlsx" | "xls" | "xlsm" | "xlsb" | "xla" | "xlam" | "ods" => sheets::sheets_convert(path)?,
        "zip" => zip_convert(path)?,
        "odt" => opendoc::opendoc_convert(path)?,
        "odp" => opendoc::opendoc_convert(path)?,
        _ => {
            let content = fs::read_to_string(path)?;
            markitdown_fallback(&content, name_header, &ext)
        }
    };

    Ok(result)
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

fn markitdown_fallback(content: &String, name: Option<&String>, ext: &String) -> String {
    let md = format!("```{}\n{}\n```", ext, content);

    match name {
        Some(name) => format!("# `{}`\n\n{}", name, md),
        None => md,
    }
}
