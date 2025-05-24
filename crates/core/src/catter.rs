use std::{
    env,
    fs::{self, File},
    io::{Write, stdout},
    path::Path,
};

use crossterm::tty::IsTty;
use image::{DynamicImage, ImageFormat};
use pager::Pager;
use rasteroid::{image_extended::InlineImage, term_misc};

use crate::{
    converter::{self},
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
    pub encoder: Option<EncoderForce>,
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
            encoder: None,
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
    let encoder = opts.encoder.unwrap_or(EncoderForce {
        kitty: false,
        iterm: false,
        sixel: false,
        ascii: false,
    });
    let inline_encoder = &rasteroid::InlineEncoder::auto_detect(
        encoder.kitty,
        encoder.iterm,
        encoder.sixel,
        encoder.ascii,
    );
    let resize_for_ascii = match inline_encoder {
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
            inline_encoder,
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
            let dyn_img = dyn_img.zoom_pan(opts.zoom, opts.x, opts.y);
            let (img, center, _, _) = dyn_img.resize_plus(opts.width, opts.height, resize_for_ascii, false)?;
            if opts.report {
                rasteroid::term_misc::report_size(opts.width.unwrap_or_default(), opts.height.unwrap_or_default());
            }
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, None, inline_encoder)?;
            Ok(CatType::InlineImage)
        },
        ("html", "image") => {
            let image = converter::html_to_image(&string_result.unwrap())?;
            out.write_all(&image)?;
            Ok(CatType::Image)
        },
        ("html", "inline") => {
            let image = converter::html_to_image(&string_result.unwrap())?;
            let dyn_img = image::load_from_memory(&image)?;
            let dyn_img = dyn_img.zoom_pan(opts.zoom, opts.x, opts.y);
            let (img, center, _, _) = dyn_img.resize_plus(opts.width, opts.height, resize_for_ascii, false)?;
            if opts.report {
                rasteroid::term_misc::report_size(opts.width.unwrap_or_default(), opts.height.unwrap_or_default());
            }
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, None, inline_encoder)?;
            Ok(CatType::InlineImage)
        },
        ("image", "image") => {
            let buf = fs::read(path)?;
            out.write_all(&buf)?;
            Ok(CatType::Image)
        },
        ("md", _) => {
            //default for md
            let res = string_result.unwrap();
            if stdout().is_tty() {
                let ansi = markdown::md_to_ansi(&res, opts.style);
                if ansi.lines().count() > term_misc::get_winsize().sc_height as usize {
                    setup_pager();
                }
                out.write_all(ansi.as_bytes())?;
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
            let image_result = image_result.unwrap().zoom_pan(opts.zoom, opts.x, opts.y);
            let (img, center, _, _) = image_result.resize_plus(opts.width, opts.height, resize_for_ascii, false)?;
            if opts.report {
                rasteroid::term_misc::report_size(opts.width.unwrap_or_default(), opts.height.unwrap_or_default());
            }
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, None, inline_encoder)?;
            Ok(CatType::InlineImage)
        },
        _ => Err(format!(
            "converting: {} to: {}, is not supported.\nsupported pipeline is: md -> html -> image -> inline_image",
            from, to
        ).into()),
    }
}

pub fn is_video(ext: &str) -> bool {
    matches!(
        ext,
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "wmv" | "flv" | "m4v" | "ts" | "gif"
    )
}

fn setup_pager() {
    let pager = if which::which("moar").is_ok() {
        "moar --no-linenumbers"
    } else {
        "less -r"
    };

    unsafe {
        env::set_var("PAGER", pager);
    }
    Pager::new().setup();
}
