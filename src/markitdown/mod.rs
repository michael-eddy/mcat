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

pub fn convert(
    path: &Path,
    name_header: Option<&String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let path = Path::new(path);
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
        "docx" => docx::docx_convert(path)?,
        "pdf" => pdf::pdf_convert(path)?,
        "pptx" => pptx::pptx_converter(path)?,
        "xlsx" | "xls" | "xlsm" | "xlsb" | "xla" | "xlam" | "ods" => sheets::sheets_convert(path)?,
        "zip" => zip_convert(path)?,
        "odt" => opendoc::opendoc_convert(path)?,
        "odp" => opendoc::opendoc_convert(path)?,
        "md" | "html" => {
            let res = fs::read_to_string(path)?;
            match name_header {
                Some(name) => format!("# {}\n\n{}\n\n", name, res),
                None => format!("{}\n\n", res),
            }
        }
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

fn markitdown_fallback(content: &String, name: Option<&String>, ext: &String) -> String {
    let md = format!("```{}\n{}\n```", ext, content);

    match name {
        Some(name) => format!("# `{}`\n\n{}", name, md),
        None => md,
    }
}
