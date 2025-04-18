use std::{fs, io::Write, path::Path};

use image::ImageFormat;

use crate::{
    converter::{self},
    image_extended::InlineImage,
    markdown,
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
    let path = Path::new(&input);
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
        &converter::InlineEncoder::auto_detect(encoder.kitty, encoder.iterm, encoder.sixel);

    //video
    if is_video(path) {
        converter::inline_a_video(input, out, &inline_encoder)?;
        return Ok(CatType::InlineVideo);
    }
    //image
    if let Some(ext) = path.extension() {
        if let Some(_) = ImageFormat::from_extension(ext) {
            let buf = fs::read(path)?;
            let dyn_img = image::load_from_memory(&buf)?;
            let (img, center) = dyn_img.resize_plus(None, None)?;
            converter::inline_an_image(&img, out, Some(center), inline_encoder)?;
            return Ok(CatType::InlineImage);
        }
    }
    // local file or dir
    let (result, from) = {
        let ext = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        match ext.as_ref() {
            "md" | "html" => {
                let r = fs::read_to_string(path)?;
                (r, ext)
            }
            _ => markdown::read_file(&path)?,
        }
    };
    if opts.to.is_none() || opts.to.unwrap() == "md" {
        out.write_all(&result.as_bytes().to_vec())?;
        return Ok(CatType::Markdown);
    }

    // converting
    if let Some(to) = opts.to {
        match (from.as_ref(), to.as_ref()) {
            ("md", "html") => {
                let html = converter::md_to_html(&result, if opts.style_html {opts.style} else {None}, opts.raw_html);
                out.write_all(&html.as_bytes().to_vec())?;
                return Ok(CatType::Html);
            },
            ("md", "image") => {
                let html = converter::md_to_html(&result, opts.style, opts.raw_html);
                let image = converter::wkhtmltox_convert(&html)?;
                out.write_all(&image)?;
                return Ok(CatType::Image);
            },
            ("md", "inline") => {
                let html = converter::md_to_html(&result, opts.style, opts.raw_html);
                let image = converter::wkhtmltox_convert(&html)?;
                let dyn_img = image::load_from_memory(&image)?;
                let (img, center) = dyn_img.resize_plus(None, None)?;
                converter::inline_an_image(&img, out, Some(center), inline_encoder)?;
                return Ok(CatType::InlineImage)
            },
            ("html", "image") => {
                let image = converter::wkhtmltox_convert(&result)?;
                out.write_all(&image)?;
                return Ok(CatType::Image);
            },
            ("html", "inline") => {
                let image = converter::wkhtmltox_convert(&result)?;
                let dyn_img = image::load_from_memory(&image)?;
                let (img, center) = dyn_img.resize_plus(None, None)?;
                converter::inline_an_image(&img, out, Some(center), inline_encoder)?;
                return Ok(CatType::InlineImage)
            },
            _ => return Err(format!(
                "converting: {} to: {}, is not supported.\nsupported pipeline is: md -> html -> image -> inline_image",
                from, to
            ).into()),
        };
    }

    Err("Input type is not supported yet".into())
}

fn is_video(input: &Path) -> bool {
    let supported_extensions = [
        "mp4", "mov", "avi", "mkv", "webm", "wmv", "flv", "m4v", "ts", "gif",
    ];

    input
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| supported_extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}
