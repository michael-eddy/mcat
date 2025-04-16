use base64::{Engine, engine::general_purpose};
use ffmpeg_sidecar::command::FfmpegCommand;
use std::{
    collections::HashSet,
    env, error, fs,
    io::Read,
    path::Path,
    process::{Command, Stdio},
};
use tempfile::Builder;

use comrak::{ComrakOptions, markdown_to_html};
pub use pyo3::types::PyModule;
use pyo3::{prelude::*, prepare_freethreaded_python};
use std::io::Write;

use crate::{iterm_encoder, kitty_encoder, sixel_encoder, term_misc};

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
    offset: Option<u16>,
    inline_encoder: &InlineEncoder,
) -> Result<Vec<u8>, Box<dyn error::Error>> {
    match inline_encoder {
        InlineEncoder::Kitty => kitty_encoder::encode_image(img, offset),
        InlineEncoder::Iterm => iterm_encoder::encode_image(img, offset),
        InlineEncoder::Sixel => sixel_encoder::encode_image(img, offset),
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

pub fn markitdown_convert(input: &str) -> PyResult<String> {
    unsafe {
        env::set_var("PYTHONWARNINGS", "ignore");
    }
    prepare_freethreaded_python();
    Python::with_gil(|py| {
        // Attempt to import 'markitdown'
        let result = PyModule::import(py, "markitdown");

        if result.is_err() {
            // If import fails, install 'markitdown' using pip
            let subprocess = PyModule::import(py, "subprocess")?;
            subprocess.call_method1(
                "check_call",
                (vec![
                    "python".to_string(),
                    "-m".to_string(),
                    "pip".to_string(),
                    "install".to_string(),
                    "markitdown[all]".to_string(),
                    "--quiet".to_string(),
                ],),
            )?;
        }

        // silent
        let io = PyModule::import(py, "io")?;
        let sys = PyModule::import(py, "sys")?;
        let devnull = io.getattr("StringIO")?.call0()?;
        sys.setattr("stdout", &devnull)?;
        sys.setattr("stderr", &devnull)?;

        let markitdown = PyModule::import(py, "markitdown")?;
        let converter = markitdown.getattr("MarkItDown")?.call0()?;
        let result = converter.call_method1("convert", (input,))?;
        let text_content: String = result.getattr("text_content")?.extract()?;

        Ok(text_content)
    })
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

fn to_file_url(path: &str) -> Option<String> {
    let abs_path = dunce::canonicalize(Path::new(path)).ok()?;
    let path_str = abs_path.to_string_lossy().replace('\\', "/");
    Some(format!("file:///{}", path_str))
}

pub fn md_to_html(markdown: &str, css_path: Option<&str>) -> String {
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
    options.render.hardbreaks = false;
    options.render.github_pre_lang = true; // <pre lang="rust">
    options.render.full_info_string = true;

    let css_path: Option<&str> = match css_path {
        Some("makurai") => Some("./styles/makurai.css"),
        Some("default") => Some("./styles/default.css"),
        Some(p) => Some(p),
        None => None,
    };

    let html = markdown_to_html(markdown, &options);
    match css_path {
        Some(path) => {
            let css_tag = to_file_url(path)
                .map(|url| format!(r#"<link rel="stylesheet" href="{}">"#, url))
                .unwrap_or_default();
            format!(
                r#"
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  {}
</head>
<body>
  {}
</body>
</html>
"#,
                css_tag, html
            )
        }
        None => html,
    }
}

pub fn inline_a_video(
    input: impl AsRef<str>,
    inline_encoder: &InlineEncoder,
) -> Result<Vec<u8>, Box<dyn error::Error>> {
    match inline_encoder {
        InlineEncoder::Kitty => todo!(),
        InlineEncoder::Iterm => {
            let gif = video_to_gif(input)?;
            let dyn_img = image::load_from_memory_with_format(&gif, image::ImageFormat::Gif)?;
            let offset = term_misc::center_image(dyn_img.width() as u16);
            let inline = iterm_encoder::encode_image(&gif, Some(offset))?;
            Ok(inline)
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
