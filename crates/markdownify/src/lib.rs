pub mod docx;
pub mod opendoc;
pub mod pdf;
pub mod pptx;
pub mod sheets;

use std::{
    fs::{self, File},
    path::Path,
};

use tempfile::Builder;
use zip::ZipArchive;

/// convert `any` document into markdown
/// # example:
/// ```
/// use std::path::Path;
/// use markdownify::convert;
///
/// let path = Path::new("path/to/file.docx");
/// match convert(&path, None) {
///     Ok(md) => println!("{}", md),
///     Err(e) => eprintln!("Error: {}", e)
/// }
/// ```
///
/// # with name:
///
/// ```
/// use std::path::Path;
/// use markdownify::convert;
///
/// let path = Path::new("path/to/file.docx");
/// let name = "file.docx".to_string();
/// match convert(&path, Some(&name)) {
///     Ok(md) => println!("{}", md),
///     Err(e) => eprintln!("Error: {}", e)
/// }
/// ```
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
            format!("{}\n\n", res)
        }
        _ => {
            let content = fs::read_to_string(path)?;
            markitdown_fallback(&content, &ext)
        }
    };

    let result = match name_header {
        Some(name) => format!("<!-- S-TITLE: {name} -->\n{result}\n---"),
        None => result,
    };

    Ok(result)
}

/// convert `zip` into markdown
/// # usage:
/// ```
/// use std::path::Path;
/// use markdownify::zip_convert;
///
/// let path = Path::new("path/to/archive.zip");
/// match zip_convert(&path) {
///     Ok(md) => println!("{}", md),
///     Err(e) => eprintln!("Error: {}", e)
/// }
/// ```
pub fn zip_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
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
        let md = match convert(&temp_path, None) {
            Ok(result) => result,
            Err(err) => format!("**[Failed Reading: {}]**", err),
        };
        output += &format!("# `{}`\n\n{}\n\n", name, md);
    }

    Ok(output)
}

fn markitdown_fallback(content: &String, ext: &String) -> String {
    format!("```{}\n{}\n```", ext, content)
}
