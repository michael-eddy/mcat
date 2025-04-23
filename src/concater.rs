use std::{fs::File, io::Write, path::PathBuf};

use image::{GenericImage, ImageFormat};
use tempfile::NamedTempFile;

use crate::{catter, converter, markitdown};

pub fn concat_text(paths: Vec<(&PathBuf, Option<String>)>) -> NamedTempFile {
    let mut markdown = String::new();
    for (path, name) in paths {
        if let Ok(md) = markitdown::convert(&path, name.as_ref()) {
            markdown.push_str(&format!("{}\n\n", md));
        } else {
            markdown.push_str("**[Failed Reading]**\n\n");
        }
    }

    let mut tmp_file = NamedTempFile::with_suffix(".md").expect("failed to create tmp file");
    tmp_file
        .write_all(markdown.trim().as_bytes())
        .expect("failed writing to tmp file");

    tmp_file
}

pub fn concat_images(
    image_paths: Vec<PathBuf>,
    horizontal: bool,
) -> Result<NamedTempFile, Box<dyn std::error::Error>> {
    // Load all images
    let mut images = Vec::new();
    for path in &image_paths {
        if !path.exists() {
            return Err(format!("{} is invalid path", path.display()).into());
        }
        if path.extension().is_some_and(|e| e == "svg") {
            let file = File::open(path)?;
            let dyn_img = converter::svg_to_image(file, None, None)?;
            images.push(dyn_img);
            continue;
        }
        let img = image::open(path)?;
        images.push(img);
    }

    // Calculate dimensions of the output image
    let (width, height) = if horizontal {
        // For horizontal concatenation, sum widths and take max height
        let total_width: u32 = images.iter().map(|img| img.width()).sum();
        let max_height: u32 = images.iter().map(|img| img.height()).max().unwrap_or(0);
        (total_width, max_height)
    } else {
        // For vertical concatenation, sum heights and take max width
        let max_width: u32 = images.iter().map(|img| img.width()).max().unwrap_or(0);
        let total_height: u32 = images.iter().map(|img| img.height()).sum();
        (max_width, total_height)
    };

    // Create a new image with the calculated dimensions
    let mut output = image::RgbaImage::new(width, height);

    // Place each image in the output
    let mut x_offset = 0;
    let mut y_offset = 0;

    for img in images {
        output.copy_from(&img, x_offset, y_offset)?;

        if horizontal {
            x_offset += img.width();
        } else {
            y_offset += img.height();
        }
    }

    // Create a temporary file with .png extension
    let temp_file = NamedTempFile::with_suffix(".png")?;
    output.save_with_format(temp_file.path(), image::ImageFormat::Png)?;
    Ok(temp_file)
}

pub fn check_unified_format(paths: &[PathBuf]) -> &'static str {
    if paths.is_empty() {
        return "text"; // Default if no files
    }

    let mut detected_format: Option<&'static str> = None;

    for path in paths {
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                let ext = ext_str.to_lowercase();

                let current_format = if catter::is_video(&ext) {
                    "video"
                } else if ImageFormat::from_extension(&ext).is_some() || ext == "svg" {
                    "image"
                } else {
                    "text"
                };

                if let Some(prev_format) = detected_format {
                    if prev_format != current_format {
                        // Found conflicting formats
                        eprintln!(
                            "Error: Cannot have 2 different formats [text / images / videos]"
                        );
                        std::process::exit(1);
                    }
                } else {
                    // First file, set the format
                    detected_format = Some(current_format);
                }
            }
        } else {
            // Files with no extension are considered text
            if detected_format.is_some() && detected_format.unwrap() != "text" {
                eprintln!("Error: Cannot have 2 different formats");
                std::process::exit(1);
            }
            detected_format = Some("text");
        }
    }
    detected_format.unwrap_or("text")
}

pub fn assign_names<'a>(
    paths: &'a [PathBuf],
    base_dir: Option<&'a String>,
) -> Vec<(&'a PathBuf, Option<String>)> {
    let is_one_element = paths.len() == 1;
    let result: Vec<(&PathBuf, Option<String>)> = paths
        .iter()
        .map(|path| {
            let name = if is_one_element {
                None
            } else {
                match base_dir {
                    Some(base) => {
                        let rel_path = path.strip_prefix(base).unwrap_or(path);
                        Some(rel_path.to_string_lossy().into_owned())
                    }
                    None => {
                        let name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        Some(name)
                    }
                }
            };
            (path, name)
        })
        .collect();

    result
}
