use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use image::{DynamicImage, ImageFormat};

use crate::{
    converter::{self},
    markitdown,
    rasteroid::{self, image_extended::InlineImage},
};

pub enum CatType {
    Markdown,
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
    pub style_html: bool,
    pub raw_html: bool,
    pub center: bool,
}
impl<'a> CatOpts<'a> {
    pub fn default() -> Self {
        CatOpts {
            to: None,
            encoder: None,
            width: Some("80%"),
            height: Some("80%"),
            style: None,
            style_html: false,
            raw_html: false,
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
        converter::inline_a_video(
            path.to_string_lossy().into_owned(),
            out,
            &inline_encoder,
            opts.center,
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
                    let f = markitdown::convert(&path, None)?;
                    (Some(f), "md")
                }
            }
        };
    }

    // converting
    match (from.as_ref(), to.as_ref()) {
        ("md", "html") => {
            let html = converter::md_to_html(&string_result.unwrap(), if opts.style_html {opts.style} else {None}, opts.raw_html);
            out.write_all(&html.as_bytes().to_vec())?;
            return Ok(CatType::Html);
        },
        ("md", "image") => {
            let html = converter::md_to_html(&string_result.unwrap(), opts.style, opts.raw_html);
            let image = converter::html_to_image(&html)?;
            out.write_all(&image)?;
            return Ok(CatType::Image);
        },
        ("md", "inline") => {
            let html = converter::md_to_html(&string_result.unwrap(), opts.style, opts.raw_html);
            let image = converter::html_to_image(&html)?;
            let dyn_img = image::load_from_memory(&image)?;
            let (img, center) = dyn_img.resize_plus(opts.width, opts.height)?;
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, inline_encoder)?;
            return Ok(CatType::InlineImage)
        },
        ("html", "image") => {
            let image = converter::html_to_image(&string_result.unwrap())?;
            out.write_all(&image)?;
            return Ok(CatType::Image);
        },
        ("html", "inline") => {
            let image = converter::html_to_image(&string_result.unwrap())?;
            let dyn_img = image::load_from_memory(&image)?;
            let (img, center) = dyn_img.resize_plus(opts.width, opts.height)?;
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, inline_encoder)?;
            return Ok(CatType::InlineImage)
        },
        ("image", "image") => {
            let buf = fs::read(path)?;
            out.write_all(&buf)?;
            return Ok(CatType::Image)
        },
        ("md", _) => {
            //default for md
            out.write_all(&string_result.unwrap().as_bytes())?;
            return Ok(CatType::Markdown);
        }
        ("html", _) => {
            // default for html
            out.write_all(&string_result.unwrap().as_bytes())?;
            return Ok(CatType::Html);
        },
        ("image", _) => {
            // default for image
            let (img, center) = image_result.unwrap().resize_plus(opts.width, opts.height)?;
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, inline_encoder)?;
            return Ok(CatType::InlineImage)
        },
        _ => return Err(format!(
            "converting: {} to: {}, is not supported.\nsupported pipeline is: md -> html -> image -> inline_image",
            from, to
        ).into()),
    };
}

pub fn is_video(input: &str) -> bool {
    matches!(
        input,
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "wmv" | "flv" | "m4v" | "ts" | "gif"
    )
}
