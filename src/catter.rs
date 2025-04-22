use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use image::{DynamicImage, ImageFormat};
use tempfile::NamedTempFile;

use crate::{
    converter::{self},
    markitdown,
    rasteroid::{self, image_extended::InlineImage},
    scrapy,
};

pub enum CatType {
    Markdown,
    Html,
    Image,
    InlineImage,
    InlineVideo,
}

pub struct EncoderForce {
    pub kitty: bool,
    pub iterm: bool,
    pub sixel: bool,
}

pub struct CatOpts<'a> {
    pub to: Option<&'a str>,
    pub encoder: Option<EncoderForce>,
    pub style: Option<&'a str>,
    pub style_html: bool,
    pub raw_html: bool,
}
impl<'a> CatOpts<'a> {
    pub fn default() -> Self {
        CatOpts {
            to: None,
            encoder: None,
            style: None,
            style_html: false,
            raw_html: false,
        }
    }
}

pub fn cat(
    input: String,
    out: &mut impl Write,
    opts: Option<CatOpts>,
) -> Result<CatType, Box<dyn std::error::Error>> {
    let maybe_temp_file: Option<NamedTempFile> = match input.starts_with("https://") {
        true => Some(scrapy::scrape_biggest_media(&input)?),
        false => None,
    };
    let path = match &maybe_temp_file {
        Some(tmp_file) => tmp_file.path(),
        None => Path::new(&input),
    };
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
    let width = Some("80%");
    let height = Some("80%");
    let to = opts.to.unwrap_or("unknown");

    //video
    if is_video(&ext) {
        converter::inline_a_video(path.to_string_lossy().into_owned(), out, &inline_encoder)?;
        return Ok(CatType::InlineVideo);
    }
    //svg
    (image_result, from) = if ext == "svg" {
        let file = File::open(path)?;
        let dyn_img = converter::svg_to_image(file, width, height)?;
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
            let (img, center) = dyn_img.resize_plus(width, height)?;
            rasteroid::inline_an_image(&img, out, Some(center), inline_encoder)?;
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
            let (img, center) = dyn_img.resize_plus(width, height)?;
            rasteroid::inline_an_image(&img, out, Some(center), inline_encoder)?;
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
            let (img, center) = image_result.unwrap().resize_plus(width, height)?;
            rasteroid::inline_an_image(&img, out, Some(center), inline_encoder)?;
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
