use base64::{Engine, engine::general_purpose};
use ffmpeg_sidecar::{command::FfmpegCommand, event::OutputVideoFrame};
use std::{
    collections::HashSet,
    error, fs,
    io::Read,
    path::Path,
    process::{Command, Stdio},
};
use tempfile::Builder;

use comrak::{ComrakOptions, markdown_to_html};
use std::io::Write;

use crate::{iterm_encoder, kitty_encoder, markitdown, sixel_encoder, term_misc};

pub enum InlineEncoder {
    Kitty,
    Iterm,
    Sixel,
}
impl InlineEncoder {
    pub fn auto_detect(force_kitty: bool, force_iterm: bool, force_sixel: bool) -> Self {
        if force_kitty {
            return Self::Kitty;
        }
        if force_iterm {
            return Self::Iterm;
        }
        if force_sixel {
            return Self::Sixel;
        }

        let env = term_misc::EnvIdentifiers::new();
        if kitty_encoder::is_kitty_capable(&env) {
            return Self::Kitty;
        }
        if iterm_encoder::is_iterm_capable(&env) {
            return Self::Iterm;
        }
        if sixel_encoder::is_sixel_capable(&env) {
            return Self::Sixel;
        }

        return Self::Iterm;
    }
}

pub fn image_to_base64(img: &Vec<u8>) -> String {
    general_purpose::STANDARD.encode(&img)
}

pub fn inline_an_image(
    img: &Vec<u8>,
    out: impl Write,
    offset: Option<u16>,
    inline_encoder: &InlineEncoder,
) -> Result<(), Box<dyn error::Error>> {
    match inline_encoder {
        InlineEncoder::Kitty => kitty_encoder::encode_image(img, out, offset),
        InlineEncoder::Iterm => iterm_encoder::encode_image(img, out, offset),
        InlineEncoder::Sixel => sixel_encoder::encode_image(img, out, offset),
    }
}

pub fn offset_to_terminal(offset: Option<u16>) -> String {
    match offset {
        Some(offset) => format!("\x1b[{}C", offset),
        None => "".to_string(),
    }
}

pub fn wkhtmltox_convert(html: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Write HTML to a temp file
    let mut temp = Builder::new().suffix(".html").tempfile()?;
    write!(temp, "{}", html)?;

    // Run wkhtmltoimage, read from file, output to stdout
    let output = Command::new("wkhtmltoimage")
        .arg("--quiet")
        .arg("--enable-local-file-access")
        .arg(temp.path())
        .arg("-") // write to stdout
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()?;

    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "wkhtmltoimage failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}

pub fn markitdown_convert(input: &str) -> Result<String, Box<dyn error::Error>> {
    let mut converter = markitdown::MARKITDOWN.lock()?;
    let result = converter.convert(input)?;

    Ok(result)
}

pub fn is_markitdown_supported(path: &Path) -> bool {
    let extension = match path.extension() {
        Some(ext) => ext.to_string_lossy().to_lowercase(),
        None => return false,
    };

    // Create a HashSet of supported formats/extensions for markitdown
    let supported_formats: HashSet<&str> = [
        "docx", "doc", "dotx", "dot", // Word documents
        "pdf", "zip", "epub", //others
        "xlsx", "xls", "xlsm", // Excel spreadsheets
        "pptx", "ppt", "pptm", // PowerPoint presentations
        "odt", "ods", "odp", // OpenDocument formats
    ]
    .iter()
    .cloned()
    .collect();

    supported_formats.contains(extension.as_str())
}

pub fn md_to_html(markdown: &str, css_path: Option<&str>, raw_html: bool) -> String {
    let mut options = ComrakOptions::default();
    // ➕ Enable extensions
    options.extension.strikethrough = true;
    options.extension.tagfilter = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.description_lists = true;

    // 🎯 Parsing options
    options.parse.smart = true; // fancy quotes, dashes, ellipses

    // 💄 Render options
    options.render.unsafe_ = raw_html;
    options.render.hardbreaks = false;
    options.render.github_pre_lang = true; // <pre lang="rust">
    options.render.full_info_string = true;

    let css_content = match css_path {
        Some("makurai") => Some(include_str!("../styles/makurai.css").to_string()),
        Some("default") => Some(include_str!("../styles/default.css").to_string()),
        Some(path) => std::fs::read_to_string(path).ok(),
        None => None,
    };

    let html = markdown_to_html(markdown, &options);
    match css_content {
        Some(css) => format!(
            r#"
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <style>{}</style>
</head>
<body>
  {}
</body>
</html>
"#,
            css, html
        ),
        None => html,
    }
}

pub fn inline_a_video(
    input: impl AsRef<str>,
    out: &mut impl Write,
    inline_encoder: &InlineEncoder,
) -> Result<(), Box<dyn error::Error>> {
    match inline_encoder {
        InlineEncoder::Kitty => {
            let frames = video_to_frames(input)?;
            let id = rand::random::<u32>();
            kitty_encoder::encode_frames(frames, out, id)?;
            Ok(())
        }
        InlineEncoder::Iterm => {
            let gif = video_to_gif(input)?;
            let dyn_img = image::load_from_memory_with_format(&gif, image::ImageFormat::Gif)?;
            let offset = term_misc::center_image(dyn_img.width() as u16);
            iterm_encoder::encode_image(&gif, out, Some(offset))?;
            Ok(())
        }
        InlineEncoder::Sixel => return Err("Cannot view videos in sixel".into()),
    }
}

fn video_to_gif(input: impl AsRef<str>) -> Result<Vec<u8>, Box<dyn error::Error>> {
    let input = input.as_ref();
    if input.ends_with(".gif") {
        let path = Path::new(input);
        let bytes = fs::read(path)?;
        return Ok(bytes);
    }
    ffmpeg_sidecar::download::auto_download()?;

    let mut command = FfmpegCommand::new();
    command
        .hwaccel("auto")
        .input(input)
        .format("gif")
        .output("-");

    let mut child = command.spawn()?;
    let mut stdout = child
        .take_stdout()
        .ok_or("failed to get stdout for ffmpeg")?;

    let mut output_bytes = Vec::new();
    stdout.read_to_end(&mut output_bytes)?;

    child.wait()?; // ensure process finishes cleanly

    Ok(output_bytes)
}

fn video_to_frames(
    input: impl AsRef<str>,
) -> Result<Box<dyn Iterator<Item = OutputVideoFrame>>, Box<dyn error::Error>> {
    let input = input.as_ref();
    ffmpeg_sidecar::download::auto_download()?;

    let mut command = FfmpegCommand::new();
    command.hwaccel("auto").input(input).rawvideo();

    let mut child = command.spawn()?;
    let frames = child.iter()?.filter_frames();

    Ok(Box::new(frames))
}
