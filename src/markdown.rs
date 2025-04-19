use std::{fs, path::Path};

use crate::{converter, prompter};

pub fn read_file(input: &impl AsRef<Path>) -> Result<(String, &str), Box<dyn std::error::Error>> {
    let path = input.as_ref();

    if path.is_file() {
        let res = read_file_markdown(path, "", true)?;
        return Ok((res, "md".into()));
    }

    if path.is_dir() {
        let res = read_dir_markdown(path)?;
        return Ok((res, "md".into()));
    }

    Err("Unknown path type".into())
}

/// Format a file into markdown with its relative path from the root
fn read_file_markdown(
    path: &Path,
    base: impl AsRef<Path>,
    single: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("text");
    let rel_path = path.strip_prefix(base).unwrap_or(path);
    let name = rel_path.display();

    let content: String = if converter::is_markitdown_supported(path) {
        if let Ok(content) = converter::markitdown_convert(&path.to_string_lossy().into_owned()) {
            content
        } else {
            "failed to read the file".to_string()
        }
    } else {
        let cont = fs::read_to_string(path)?;
        format!("```{}\n{}\n```", ext, cont)
    };

    if single {
        Ok(content)
    } else {
        Ok(format!("# `{}`\n\n{}", name, content))
    }
}

/// Handle directory prompting and file selection
fn read_dir_markdown(path: &Path) -> Result<String, String> {
    let mut selected_files = prompter::prompt_for_files(path)?;
    selected_files.sort();

    let mut markdown = String::new();
    for file in selected_files {
        if let Ok(md) = read_file_markdown(&file, path, false) {
            markdown.push_str(&md);
            markdown.push_str("\n\n");
        }
    }

    Ok(markdown.trim().to_string())
}
