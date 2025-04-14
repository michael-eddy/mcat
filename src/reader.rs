use std::{fs, path::Path};

use crate::prompter;

pub fn read_file(input: &str) -> Result<String, String> {
    let path = Path::new(input);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", input));
    }

    if path.is_file() {
        return read_file_markdown(path, "");
    }

    if path.is_dir() {
        return read_dir_markdown(path);
    }

    Err("Unknown path type".into())
}

/// Format a file into markdown with its relative path from the root
fn read_file_markdown<P: AsRef<Path>>(path: &Path, base: P) -> Result<String, String> {
    let rel_path = path.strip_prefix(base).unwrap_or(path);
    let name = rel_path.display();
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("text");

    Ok(format!("## `{}`\n\n```{}\n{}\n```", name, ext, content))
}

/// Handle directory prompting and file selection
fn read_dir_markdown(path: &Path) -> Result<String, String> {
    let mut selected_files = prompter::prompt_for_files(path)?;
    selected_files.sort();

    let mut markdown = String::new();
    for file in selected_files {
        if let Ok(md) = read_file_markdown(&file, path) {
            markdown.push_str(&md);
            markdown.push_str("\n\n");
        }
    }

    Ok(markdown.trim().to_string())
}
