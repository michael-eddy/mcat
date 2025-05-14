use chromiumoxide::{Browser, BrowserConfig, BrowserFetcher, BrowserFetcherOptions};
use ffmpeg_sidecar::event::OutputVideoFrame;
use futures::{lock::Mutex, stream::StreamExt};
use image::{DynamicImage, ImageBuffer, Rgba};
use rasteroid::kitty_encoder::Frame;
use resvg::{
    tiny_skia,
    usvg::{self, Options, Tree},
};
use std::{
    error, fs,
    io::Read,
    path::Path,
    sync::{Arc, atomic::Ordering},
};
use tokio::sync::oneshot;

use comrak::{
    ComrakOptions, ComrakPlugins, markdown_to_html_with_plugins, plugins::syntect::SyntectAdapter,
};
use std::io::Write;

use crate::fetch_manager;

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
        Some(w) => rasteroid::term_misc::dim_to_px(w, rasteroid::term_misc::SizeDirection::Width)?,
        None => src_width as u32,
    };
    let height = match height {
        Some(h) => rasteroid::term_misc::dim_to_px(h, rasteroid::term_misc::SizeDirection::Height)?,
        None => src_height as u32,
    };
    let (target_width, target_height) =
        rasteroid::image_extended::calc_fit(src_width as u32, src_height as u32, width, height);
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
    let image_buffer =
        ImageBuffer::<Rgba<u8>, _>::from_raw(target_width, target_height, pixmap.data().to_vec())
            .ok_or("Failed to create image buffer for svg")?;

    // Convert ImageBuffer to DynamicImage
    Ok(DynamicImage::ImageRgba8(image_buffer))
}

pub fn html_to_image(html: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let encoded_html = urlencoding::encode(html);
    let data_uri = format!("data:text/html;charset=utf-8,{}", encoded_html);
    let data = screenshot_uri(&data_uri)?;

    Ok(data)
}

fn screenshot_uri(data_uri: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let config = match BrowserConfig::builder().new_headless_mode().build() {
            Ok(c) => c,
            Err(_) => {
                let cache_path = fetch_manager::get_cache_path();
                let download_path = cache_path.join("chromium");
                if download_path.join("installed.txt").exists() {
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
                } else {
                    return Err("chromium isn't installed. either install it manually (chrome/msedge will do so too) or call `mcat --fetch-chromium`".into())
                }
            }
        };

        let (cancel_tx, cancel_rx) = oneshot::channel();
        let shutdown = rasteroid::term_misc::setup_signal_handler();

        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| format!("failed to launch chromium\noriginal error: {}", e))?;
        let browser_arc = Arc::new(Mutex::new(browser));
        let signal_browser = browser_arc.clone();

        // freeing the browser when process is killed, to avoid zombie process
        tokio::spawn(async move {
            loop {
                if shutdown.load(Ordering::SeqCst) {
                    if !cancel_tx.is_closed() {
                        let _ = cancel_tx.send(true);
                        let mut browser = signal_browser.lock().await;
                        let _ = browser.close().await;
                        let _ = browser.wait().await;
                        std::process::exit(1);
                    }
                };
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        // main function
        tokio::spawn(async move { while handler.next().await.is_some() {} });

        let data_uri = data_uri.to_string();
        let page = tokio::spawn(async move {
            tokio::select! {
                result = async {
                    let browser = browser_arc.lock().await;
                    browser.new_page(data_uri).await
                } => Some(result),
                _ = cancel_rx => None
            }
        })
        .await?;
        let page = page.ok_or("Canceled")??;

        let prms = chromiumoxide::page::ScreenshotParams::builder()
            .full_page(true)
            .omit_background(true)
            .build();
        let screenshot = page.screenshot(prms).await?;

        Ok(screenshot)
    })
}

pub fn md_to_html(markdown: &str, css_path: Option<&str>) -> String {
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
    options.render.unsafe_ = true;
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

pub struct KittyFrames(pub OutputVideoFrame);
impl Frame for KittyFrames {
    fn width(&self) -> u16 {
        self.0.width as u16
    }
    fn height(&self) -> u16 {
        self.0.height as u16
    }
    fn timestamp(&self) -> f32 {
        self.0.timestamp
    }
    fn data(&self) -> &[u8] {
        &self.0.data
    }
}

pub fn inline_a_video(
    input: impl AsRef<str>,
    out: &mut impl Write,
    inline_encoder: &rasteroid::InlineEncoder,
    center: bool,
) -> Result<(), Box<dyn error::Error>> {
    match inline_encoder {
        rasteroid::InlineEncoder::Kitty => {
            let frames = video_to_frames(input)?;
            let mut kitty_frames = frames.map(KittyFrames);
            let id = rand::random::<u32>();
            rasteroid::kitty_encoder::encode_frames(&mut kitty_frames, out, id, center)?;
            Ok(())
        }
        rasteroid::InlineEncoder::Iterm => {
            let gif = video_to_gif(input)?;
            let dyn_img = image::load_from_memory_with_format(&gif, image::ImageFormat::Gif)?;
            let offset = match center {
                true => Some(rasteroid::term_misc::center_image(
                    dyn_img.width() as u16,
                    false,
                )),
                false => None,
            };
            rasteroid::iterm_encoder::encode_image(&gif, out, offset)?;
            Ok(())
        }
        rasteroid::InlineEncoder::Sixel => Err("Cannot view videos in sixel".into()),
        rasteroid::InlineEncoder::Ascii => {
            let frames = video_to_frames(input)?;
            let mut kitty_frames = frames.map(KittyFrames);
            rasteroid::ascii_encoder::encode_frames(&mut kitty_frames, out, center)?;
            Ok(())
        }
    }
}

fn video_to_gif(input: impl AsRef<str>) -> Result<Vec<u8>, Box<dyn error::Error>> {
    let input = input.as_ref();
    if input.ends_with(".gif") {
        let path = Path::new(input);
        let bytes = fs::read(path)?;
        return Ok(bytes);
    }

    let mut command =
        match fetch_manager::get_ffmpeg() {
            Some(c) => c,
            None => return Err(
                "ffmpeg isn't installed. either install it manually, or call `mcat --fetch-ffmpeg`"
                    .into(),
            ),
        };
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

    let mut command =
        match fetch_manager::get_ffmpeg() {
            Some(c) => c,
            None => return Err(
                "ffmpeg isn't installed. either install it manually, or call `mcat --fetch-ffmpeg`"
                    .into(),
            ),
        };
    command.hwaccel("auto").input(input).rawvideo();

    let mut child = command.spawn()?;
    let frames = child.iter()?.filter_frames();

    Ok(Box::new(frames))
}
