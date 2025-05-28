use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{Write, stdout},
    path::Path,
    process::{Command, Stdio},
};

use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode},
    tty::IsTty,
};
use image::{DynamicImage, ImageFormat};
use rasteroid::{
    InlineEncoder,
    image_extended::{InlineImage, ZoomPanViewport},
    term_misc,
};

use crate::{
    converter::{self},
    image_viewer::{clear_screen, run_interactive_viewer, show_help_prompt},
    markdown,
};

pub enum CatType {
    Markdown,
    Pretty,
    Html,
    Image,
    Video,
    InlineImage,
    InlineVideo,
    Interactive,
}

#[derive(Clone, Copy)]
pub struct EncoderForce {
    pub kitty: bool,
    pub iterm: bool,
    pub sixel: bool,
    pub ascii: bool,
}

#[derive(Clone, Copy)]
pub struct CatOpts<'a> {
    pub to: Option<&'a str>,
    pub encoder: &'a InlineEncoder,
    pub style: Option<&'a str>,
    pub width: Option<&'a str>,
    pub height: Option<&'a str>,
    pub zoom: Option<usize>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub style_html: bool,
    pub center: bool,
    pub report: bool,
    pub silent: bool,
}
impl CatOpts<'_> {
    pub fn default() -> Self {
        CatOpts {
            to: None,
            encoder: &InlineEncoder::Ascii,
            width: Some("80%"),
            height: Some("80%"),
            zoom: None,
            x: None,
            y: None,
            style: None,
            style_html: false,
            center: false,
            report: false,
            silent: false,
        }
    }
}

pub fn cat(
    path: &Path,
    out: &mut impl Write,
    opts: Option<CatOpts>,
) -> Result<CatType, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("invalid path: {}", path.display()).into());
    }

    let opts = match opts {
        Some(o) => o,
        None => CatOpts::default(),
    };
    let resize_for_ascii = match opts.encoder {
        rasteroid::InlineEncoder::Ascii => true,
        _ => false,
    };
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let mut image_result: Option<DynamicImage> = None;
    let mut string_result: Option<String> = None;
    let mut from: &str = "unknown";
    let to = opts.to.unwrap_or("unknown");

    //video
    if is_video(&ext) {
        if to == "video" {
            let content = fs::read(path)?;
            out.write_all(&content)?;
            return Ok(CatType::Video);
        }
        converter::inline_a_video(
            path.to_string_lossy(),
            out,
            opts.encoder,
            opts.width,
            opts.height,
            opts.center,
            opts.silent,
        )?;
        return Ok(CatType::InlineVideo);
    }
    //svg
    (image_result, from) = if ext == "svg" {
        let file = File::open(path)?;
        let dyn_img = converter::svg_to_image(file, opts.width, opts.height)?;
        (Some(dyn_img), "image")
    } else {
        (image_result, from)
    };
    //image
    (image_result, from) = if ImageFormat::from_extension(&ext).is_some() {
        let buf = fs::read(path)?;
        let dyn_img = image::load_from_memory(&buf)?;
        (Some(dyn_img), "image")
    } else {
        (image_result, from)
    };
    // local file or dir
    if from == "unknown" {
        (string_result, from) = {
            match ext.as_ref() {
                "md" | "html" => {
                    let r = fs::read_to_string(path)?;
                    (Some(r), ext.as_ref())
                }
                _ => {
                    let f = markdownify::convert(path, None)?;
                    (Some(f), "md")
                }
            }
        };
    }

    // converting
    match (from, to) {
        ("md", "md") => {
            out.write_all(string_result.unwrap().as_bytes())?;
            Ok(CatType::Markdown)
        }
        ("md", "html") => {
            let html = markdown::md_to_html(&string_result.unwrap(), if opts.style_html {opts.style} else {None});
            out.write_all(html.as_bytes())?;
            Ok(CatType::Html)
        },
        ("md", "image") => {
            let html = markdown::md_to_html(&string_result.unwrap(), opts.style);
            let image = converter::html_to_image(&html)?;
            out.write_all(&image)?;
            Ok(CatType::Image)
        },
        ("md", "inline") => {
            let html = markdown::md_to_html(&string_result.unwrap(), opts.style);
            let image = converter::html_to_image(&html)?;
            let dyn_img = image::load_from_memory(&image)?;
            let dyn_img = apply_pan_zoom_once(dyn_img, &opts);
            let (img, center, _, _) = dyn_img.resize_plus(opts.width, opts.height, resize_for_ascii, false)?;
            if opts.report {
                rasteroid::term_misc::report_size(opts.width.unwrap_or_default(), opts.height.unwrap_or_default());
            }
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, None, opts.encoder)?;
            Ok(CatType::InlineImage)
        },
        ("md", "interactive") => {
            let html = markdown::md_to_html(&string_result.unwrap(), opts.style);
            let img_bytes = converter::html_to_image(&html)?;
            let img = image::load_from_memory(&img_bytes)?;
            interact_with_image(img, opts, out, resize_for_ascii)?;
            Ok(CatType::Interactive)
        },
        ("html", "image") => {
            let image = converter::html_to_image(&string_result.unwrap())?;
            out.write_all(&image)?;
            Ok(CatType::Image)
        },
        ("html", "inline") => {
            let image = converter::html_to_image(&string_result.unwrap())?;
            let dyn_img = image::load_from_memory(&image)?;
            let dyn_img = apply_pan_zoom_once(dyn_img, &opts);
            let (img, center, _, _) = dyn_img.resize_plus(opts.width, opts.height, resize_for_ascii, false)?;
            if opts.report {
                rasteroid::term_misc::report_size(opts.width.unwrap_or_default(), opts.height.unwrap_or_default());
            }
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, None, opts.encoder)?;
            Ok(CatType::InlineImage)
        },
        ("html", "interactive") => {
            let html = &string_result.unwrap();
            let img_bytes = converter::html_to_image(&html)?;
            let img = image::load_from_memory(&img_bytes)?;
            interact_with_image(img, opts, out, resize_for_ascii)?;
            Ok(CatType::Interactive)
        },
        ("image", "image") => {
            let buf = fs::read(path)?;
            out.write_all(&buf)?;
            Ok(CatType::Image)
        },
        ("image", "interactive") => {
            let img = image_result.unwrap();
            interact_with_image(img, opts, out, resize_for_ascii)?;
            Ok(CatType::Interactive)
        },
        ("md", _) => {
            //default for md
            let res = string_result.unwrap();
            if stdout().is_tty() {
                let ansi = markdown::md_to_ansi(&res, opts.style);
                if ansi.lines().count() > term_misc::get_wininfo().sc_height as usize {
                    let pager = Pager::new();
                    if pager.page(&ansi).is_err() {
                        out.write_all(ansi.as_bytes())?;
                    }
                } else {
                    out.write_all(ansi.as_bytes())?;
                }
                Ok(CatType::Pretty)
            } else {
                out.write_all(res.as_bytes())?;
                return Ok(CatType::Markdown)
            }
        },
        ("html", _) => {
            // default for html
            out.write_all(string_result.unwrap().as_bytes())?;
            Ok(CatType::Html)
        },
        ("image", _) => {
            // default for image
            let image_result = apply_pan_zoom_once(image_result.unwrap(), &opts);
            let (img, center, _, _) = image_result.resize_plus(opts.width, opts.height, resize_for_ascii, false)?;
            if opts.report {
                rasteroid::term_misc::report_size(opts.width.unwrap_or_default(), opts.height.unwrap_or_default());
            }
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, None, opts.encoder)?;
            Ok(CatType::InlineImage)
        },
        _ => Err(format!(
            "converting: {} to: {}, is not supported.\nsupported pipeline is: md -> html -> image -> inline_image",
            from, to
        ).into()),
    }
}

fn apply_pan_zoom_once(img: DynamicImage, opts: &CatOpts) -> DynamicImage {
    let zoom = opts.zoom.unwrap_or(1);
    let x = opts.x.unwrap_or_default();
    let y = opts.y.unwrap_or_default();
    if zoom == 1 && x == 0 && y == 0 {
        return img;
    }

    let tinfo = term_misc::get_wininfo();
    let container_width = tinfo.spx_width as u32;
    let container_height = tinfo.spx_height as u32;
    let image_width = img.width();
    let image_height = img.height();

    let mut vp = ZoomPanViewport::new(container_width, container_height, image_width, image_height);
    vp.set_zoom(zoom);
    vp.set_pan(x, y);
    vp.apply_to_image(&img)
}

fn interact_with_image(
    img: DynamicImage,
    opts: CatOpts,
    out: &mut impl Write,
    resize_for_ascii: bool,
) -> Result<(), Box<dyn Error>> {
    let tinfo = term_misc::get_wininfo();
    let container_width = tinfo.spx_width as u32;
    let container_height = tinfo.spx_height as u32;
    let image_width = img.width();
    let image_height = img.height();

    let height_cells = term_misc::dim_to_cells(
        opts.height.unwrap_or_default(),
        term_misc::SizeDirection::Height,
    )?;
    let height = (tinfo.sc_height - 3).min(height_cells as u16);

    run_interactive_viewer(
        container_width,
        container_height,
        image_width,
        image_height,
        |vp| {
            clear_screen(out).ok()?;
            let new_img = vp.apply_to_image(&img);
            let (img, center, _, _) = new_img
                .resize_plus(
                    opts.width,
                    Some(&format!("{height}c")),
                    resize_for_ascii,
                    false,
                )
                .ok()?;
            if resize_for_ascii {
                disable_raw_mode().ok()?;
            }
            rasteroid::inline_an_image(
                &img,
                out,
                if opts.center { Some(center) } else { None },
                None,
                opts.encoder,
            )
            .ok()?;
            show_help_prompt(out, tinfo.sc_width, tinfo.sc_height, vp).ok()?;
            out.flush().ok()?;
            if resize_for_ascii {
                enable_raw_mode().ok()?;
            }

            Some(())
        },
    )?;
    Ok(())
}

pub fn is_video(ext: &str) -> bool {
    matches!(
        ext,
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "wmv" | "flv" | "m4v" | "ts" | "gif"
    )
}

pub struct Pager {
    command: Option<(String, Vec<String>)>,
}

impl Pager {
    pub fn new() -> Self {
        let command = Self::find_pager();
        Self { command }
    }

    fn find_pager() -> Option<(String, Vec<String>)> {
        // Check MCATPAGER first, then PAGER
        if let Ok(pager_env) = env::var("MCATPAGER").or_else(|_| env::var("PAGER")) {
            let pager_name = pager_env.split_whitespace().next()?;

            // Only support known pagers that handle ANSI codes well
            match pager_name {
                "less" => {
                    if which::which("less").is_ok() {
                        return Some(("less".to_string(), vec!["-r".to_string()]));
                    }
                }
                "moar" => {
                    if which::which("moar").is_ok() {
                        return Some(("moar".to_string(), vec!["--no-linenumbers".to_string()]));
                    }
                }
                _ => {} // Ignore unsupported pagers
            }
        }

        // Try default pagers
        if which::which("less").is_ok() {
            Some(("less".to_string(), vec!["-r".to_string()]))
        } else if which::which("moar").is_ok() {
            Some(("moar".to_string(), vec!["--no-linenumbers".to_string()]))
        } else {
            None
        }
    }

    pub fn page(&self, content: &str) -> Result<(), Box<dyn Error>> {
        if let Some((cmd, args)) = &self.command {
            let mut child = Command::new(cmd).args(args).stdin(Stdio::piped()).spawn()?;

            if let Some(stdin) = child.stdin.as_mut() {
                // ignoring cuz the pipe will break when the user quits most likely
                let _ = stdin.write_all(content.as_bytes());
            }

            child.wait()?;
        } else {
            return Err("no pager was found in the system".into());
        }

        Ok(())
    }
}
