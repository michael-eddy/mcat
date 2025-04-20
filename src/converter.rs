use base64::{Engine, engine::general_purpose};
use chromiumoxide::{Browser, BrowserConfig, BrowserFetcher, BrowserFetcherOptions};
use ffmpeg_sidecar::{command::FfmpegCommand, event::OutputVideoFrame};
use futures::stream::StreamExt;
use image::{DynamicImage, ImageBuffer, Rgba};
use resvg::{
    tiny_skia,
    usvg::{self, Options, Tree},
};
use std::{
    error, fs,
    io::Read,
    path::{Path, PathBuf},
};

use comrak::{
    ComrakOptions, ComrakPlugins, markdown_to_html_with_plugins, plugins::syntect::SyntectAdapter,
};
use std::io::Write;

use crate::{image_extended, iterm_encoder, kitty_encoder, sixel_encoder, term_misc};

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

pub fn svg_to_image(
    mut reader: impl Read,
    width: Option<&str>,
    height: Option<&str>,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let mut svg_data = Vec::new();
    reader.read_to_end(&mut svg_data)?;

    // Create options for parsing SVG
    let mut opt = Options::default();

    // allowing text
    let mut fontdb = fontdb::Database::new();
    fontdb.load_system_fonts();
    opt.fontdb = std::sync::Arc::new(fontdb);
    opt.text_rendering = usvg::TextRendering::OptimizeLegibility;

    // Parse SVG
    let tree = Tree::from_data(&svg_data, &opt)?;

    // Get size of the SVG
    let pixmap_size = tree.size();
    let src_width = pixmap_size.width();
    let src_height = pixmap_size.height();
    let width = match width {
        Some(w) => term_misc::dim_to_px(w, term_misc::SizeDirection::Width)?,
        None => src_width as u32,
    };
    let height = match height {
        Some(h) => term_misc::dim_to_px(h, term_misc::SizeDirection::Height)?,
        None => src_height as u32,
    };
    let (target_width, target_height) =
        image_extended::calc_fit(src_width as u32, src_height as u32, width, height);
    let scale_x = target_width as f32 / src_width;
    let scale_y = target_height as f32 / src_height;
    let scale = scale_x.min(scale_y);

    // Create a Pixmap to render to
    let mut pixmap = tiny_skia::Pixmap::new(target_width, target_height)
        .ok_or("Failed to create pixmap for svg")?;
    let transform = tiny_skia::Transform::from_scale(scale, scale);

    // Render SVG to Pixmap
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert Pixmap to ImageBuffer
    let image_buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
        target_width as u32,
        target_height as u32,
        pixmap.data().to_vec(),
    )
    .ok_or("Failed to create image buffer for svg")?;

    // Convert ImageBuffer to DynamicImage
    Ok(DynamicImage::ImageRgba8(image_buffer))
}

fn get_chromium_install_path() -> PathBuf {
    let base_dir = dirs::cache_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| std::env::temp_dir());

    let p = base_dir.join("chromiumoxide").join("chromium");
    if !p.exists() {
        eprintln!("couldn't find chromium installed, trying to install.. it may take a little.");
        let _ = fs::create_dir_all(p.clone());
    }
    p
}
pub fn headless_chrome_convert(html: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let encoded_html = urlencoding::encode(&html);
    let data_uri = format!("data:text/html;charset=utf-8,{}", encoded_html);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let config = match BrowserConfig::builder().new_headless_mode().build() {
            Ok(c) => c,
            Err(_) => {
                let download_path = get_chromium_install_path();
                let fetcher = BrowserFetcher::new(
                    BrowserFetcherOptions::builder()
                        .with_path(&download_path)
                        .build()?,
                );
                let info = fetcher.fetch().await?;
                BrowserConfig::builder()
                    .chrome_executable(info.executable_path)
                    .new_headless_mode()
                    .build()?
            }
        };
        let (browser, mut handler) = Browser::launch(config).await.map_err(|e| format!("failed to launch chromium\nplease remove: {} and rerun. or install chrome\noriginal error: {}", get_chromium_install_path().display(), e))?;
        tokio::spawn(async move { while let Some(_) = handler.next().await {} });

        let page = browser.new_page(data_uri).await?;

        let mut prms = chromiumoxide::page::ScreenshotParams::default();
        prms.full_page = Some(true);
        prms.omit_background = Some(true);
        let screenshot = page.screenshot(prms).await?;

        Ok(screenshot)
    })
}

pub fn md_to_html(markdown: &str, css_path: Option<&str>, raw_html: bool) -> String {
    let mut options = ComrakOptions::default();

    let mut plugins = ComrakPlugins::default();
    let adapter = SyntectAdapter::new(None);
    plugins.render.codefence_syntax_highlighter = Some(&adapter);

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

    let html = markdown_to_html_with_plugins(markdown, &options, &plugins);
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
    if !ffmpeg_sidecar::command::ffmpeg_is_installed() {
        eprintln!("ffmpeg isn't installed, installing.. it may take a little");
        ffmpeg_sidecar::download::auto_download()?;
    }

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
    if !ffmpeg_sidecar::command::ffmpeg_is_installed() {
        eprintln!("ffmpeg isn't installed, installing.. it may take a little");
        ffmpeg_sidecar::download::auto_download()?;
    }

    let mut command = FfmpegCommand::new();
    command.hwaccel("auto").input(input).rawvideo();

    let mut child = command.spawn()?;
    let frames = child.iter()?.filter_frames();

    Ok(Box::new(frames))
}
