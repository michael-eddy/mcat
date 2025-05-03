use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use image::{DynamicImage, ImageFormat};
use rasteroid::image_extended::InlineImage;
use termimad::{FmtText, MadSkin, crossterm::style::Color};

use crate::converter::{self};

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
    });
    let inline_encoder =
        &rasteroid::InlineEncoder::auto_detect(encoder.kitty, encoder.iterm, encoder.sixel);
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
        converter::inline_a_video(path.to_string_lossy(), out, inline_encoder, opts.center)?;
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
        ("md", "html") => {
            let html = converter::md_to_html(&string_result.unwrap(), if opts.style_html {opts.style} else {None});
            out.write_all(html.as_bytes())?;
            Ok(CatType::Html)
        },
        ("md", "pretty") => {
            let res = string_result.unwrap();
            let mut skin = MadSkin::default();
            skin.set_headers_fg(Color::Green);
            skin.bold.set_fg(Color::Yellow);
            skin.italic.set_fg(Color::Magenta);
            skin.inline_code.set_fg(Color::Blue);
            skin.strikeout.set_fg(Color::Red);
            skin.code_block.set_fg(Color::White);
            skin.table.set_fg(Color::Cyan);

            let fmt_text = FmtText::from(&skin, &res, None);
            write!(out, "{}", &fmt_text)?;
            Ok(CatType::Pretty)
        },
        ("md", "image") => {
            let html = converter::md_to_html(&string_result.unwrap(), opts.style);
            let image = converter::html_to_image(&html)?;
            out.write_all(&image)?;
            Ok(CatType::Image)
        },
        ("md", "inline") => {
            let html = converter::md_to_html(&string_result.unwrap(), opts.style);
            let image = converter::html_to_image(&html)?;
            let dyn_img = image::load_from_memory(&image)?;
            let dyn_img = dyn_img.zoom_pan(opts.zoom, opts.x, opts.y);
            let (img, center) = dyn_img.resize_plus(opts.width, opts.height)?;
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, inline_encoder)?;
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
            let (img, center) = dyn_img.resize_plus(opts.width, opts.height)?;
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, inline_encoder)?;
            Ok(CatType::InlineImage)
        },
        ("image", "image") => {
            let buf = fs::read(path)?;
            out.write_all(&buf)?;
            Ok(CatType::Image)
        },
        ("md", _) => {
            //default for md
            out.write_all(string_result.unwrap().as_bytes())?;
            Ok(CatType::Markdown)
        }
        ("html", _) => {
            // default for html
            out.write_all(string_result.unwrap().as_bytes())?;
            Ok(CatType::Html)
        },
        ("image", _) => {
            // default for image
            let image_result = image_result.unwrap().zoom_pan(opts.zoom, opts.x, opts.y);
            let (img, center) = image_result.resize_plus(opts.width, opts.height)?;
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, inline_encoder)?;
            Ok(CatType::InlineImage)
        },
        _ => Err(format!(
            "converting: {} to: {}, is not supported.\nsupported pipeline is: md -> html -> image -> inline_image",
            from, to
        ).into()),
    }
}

pub fn is_video(input: &str) -> bool {
    matches!(
        input,
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "wmv" | "flv" | "m4v" | "ts" | "gif"
    )
}
