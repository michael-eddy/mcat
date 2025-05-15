use chromiumoxide::{Browser, BrowserConfig, BrowserFetcher, BrowserFetcherOptions};
use ffmpeg_sidecar::event::OutputVideoFrame;
use futures::{lock::Mutex, stream::StreamExt};
use image::{DynamicImage, ImageBuffer, Rgba};
use indicatif::{ProgressBar, ProgressStyle};
use rasteroid::{Frame, image_extended::InlineImage};
use regex::Regex;
use resvg::{
    tiny_skia,
    usvg::{self, Options, Tree},
};
use std::{
    error, fs,
    io::{BufRead, Read},
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
        Some("dark") => Some(include_str!("../styles/dark.css").to_string()),
        Some("light") => Some(include_str!("../styles/light.css").to_string()),
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

pub struct VideoFrames {
    timestamp: f32,
    img: Vec<u8>,
    width: u16,
    height: u16,
}
impl Frame for VideoFrames {
    fn timestamp(&self) -> f32 {
        self.timestamp
    }
    fn data(&self) -> &[u8] {
        &self.img
    }
    fn width(&self) -> u16 {
        self.width as u16
    }
    fn height(&self) -> u16 {
        self.height as u16
    }
}

///width and height only needed for ascii videos atm
pub fn inline_a_video(
    input: impl AsRef<str>,
    out: &mut impl Write,
    inline_encoder: &rasteroid::InlineEncoder,
    width: Option<&str>,
    height: Option<&str>,
    center: bool,
    silent: bool,
) -> Result<(), Box<dyn error::Error>> {
    match inline_encoder {
        rasteroid::InlineEncoder::Kitty => {
            let frames = video_to_frames(input)?;
            let mut kitty_frames = frames.map(|f| VideoFrames {
                width: f.width as u16,
                height: f.height as u16,
                img: f.data,
                timestamp: f.timestamp,
            });
            let id = rand::random::<u32>();
            rasteroid::kitty_encoder::encode_frames(&mut kitty_frames, out, id, center)?;
            Ok(())
        }
        rasteroid::InlineEncoder::Iterm => {
            let gif = video_to_gif(input, silent)?;
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
        rasteroid::InlineEncoder::Ascii | rasteroid::InlineEncoder::Sixel => {
            let frames = video_to_frames(input)?;
            let mut ascii_frames = frames.map(|f| {
                let rgb_image = image::RgbImage::from_raw(f.width, f.height, f.data.clone())
                    .unwrap_or_default();
                let img = image::DynamicImage::ImageRgb8(rgb_image);
                let (img, _) = img.resize_plus(width, height, true).unwrap_or_default();
                VideoFrames {
                    timestamp: f.timestamp,
                    img,
                    width: 0,
                    height: 0,
                }
            });
            rasteroid::ascii_encoder::encode_frames(&mut ascii_frames, out, center, true)?;
            Ok(())
        }
    }
}

fn video_to_gif(input: impl AsRef<str>, silent: bool) -> Result<Vec<u8>, Box<dyn error::Error>> {
    let input = input.as_ref();
    if input.ends_with(".gif") {
        let path = Path::new(input);
        let bytes = fs::read(path)?;
        return Ok(bytes);
    }

    // Create indeterminate progress bar since we don't know total frames
    let pb = if !silent {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:50.blue/white}] {pos}/{len} frames ({percent}%)")?
                .progress_chars("█▓▒░"),
        );
        Some(pb)
    } else {
        None
    };

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
        .args(&["-progress", "pipe:2"]) // Request progress output
        .output("-");

    let mut child = command.spawn()?;
    let mut stdout = child
        .take_stdout()
        .ok_or("failed to get stdout for ffmpeg")?;
    let stderr = child
        .take_stderr()
        .ok_or("failed to get stderr for ffmpeg")?;

    // Read stdout in a separate thread
    let output_thread = std::thread::spawn(move || {
        let mut output_bytes = Vec::new();
        stdout.read_to_end(&mut output_bytes).unwrap();
        output_bytes
    });

    // Process stderr for progress updates
    let duration_re = Regex::new(r"Duration: (\d+):(\d+):([\d.]+)")?;
    let fps_re = Regex::new(r"(\d+(?:\.\d+)?) fps")?;
    let frame_re = Regex::new(r"frame=\s*(\d+)")?;

    let mut total_frames = None;
    let mut fps = None;
    let mut duration_secs = None;
    for line in std::io::BufReader::new(stderr).lines() {
        let line = line?;
        if let Some(cap) = duration_re.captures(&line) {
            let hours: f64 = cap[1].parse().unwrap_or(0.0);
            let minutes: f64 = cap[2].parse().unwrap_or(0.0);
            let seconds: f64 = cap[3].parse().unwrap_or(0.0);
            duration_secs = Some(hours * 3600.0 + minutes * 60.0 + seconds);
        }
        if fps.is_none() {
            if let Some(cap) = fps_re.captures(&line) {
                fps = Some(cap[1].parse::<f64>().unwrap_or(0.0));
            }
        }
        if total_frames.is_none() {
            if let (Some(dur), Some(f)) = (duration_secs, fps) {
                let frames = (dur * f).round();
                total_frames = Some(frames);
                if !silent {
                    pb.as_ref().unwrap().set_length(frames as u64);
                }
            }
        }

        // Parse frame count from progress output
        if let Some(cap) = frame_re.captures(&line) {
            let current_frame: usize = cap[1].parse().unwrap_or(0);
            if !silent {
                pb.as_ref().unwrap().set_position(current_frame as u64);
            }
        }
    }

    let output_bytes = output_thread
        .join()
        .map_err(|_| "failed to capture output")?;
    child.wait()?;

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
