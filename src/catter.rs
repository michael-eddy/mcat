use std::{fs, path::Path};

use image::ImageFormat;

use crate::{
    converter::{self},
    image_extended::InlineImage,
    reader,
};

pub struct Catter {
    input: String,
}

pub enum CatType {
    Markdown,
    Html,
    Image,
    InlineImage,
}

pub struct EncoderForce {
    pub kitty: bool,
    pub iterm: bool,
    pub sixel: bool,
}

impl Catter {
    pub fn new(input: String) -> Self {
        Catter { input }
    }
    pub fn cat(
        &self,
        to: Option<&String>,
        style: Option<&str>,
        style_html: bool,
        encoder: Option<EncoderForce>,
    ) -> Result<(Vec<u8>, CatType), Box<dyn std::error::Error>> {
        let path = Path::new(&self.input);
        let encoder = match encoder {
            Some(en) => en,
            None => EncoderForce {
                kitty: false,
                iterm: false,
                sixel: false,
            },
        };

        //image
        if let Some(ext) = path.extension() {
            if let Some(_) = ImageFormat::from_extension(ext) {
                let buf = fs::read(path)?;
                let dyn_img = image::load_from_memory(&buf)?;
                let (img, center) = dyn_img.resize_plus(None, None)?;
                let inline_encoder = &converter::InlineEncoder::auto_detect(
                    encoder.kitty,
                    encoder.iterm,
                    encoder.sixel,
                );
                let inline_img = converter::inline_an_image(&img, Some(center), inline_encoder)?;
                return Ok((inline_img, CatType::InlineImage));
            }
        }
        // local file or dir
        let (result, from) = if path.exists() {
            let (r, f) = reader::read_file(&path)?;
            if to.is_none() {
                return Ok((r.as_bytes().to_vec(), CatType::Markdown));
            }
            (r, f)
        } else {
            return Err(format!("invalid path: {}", path.display()).into());
        };

        // converting
        if let Some(to) = to {
            match (from.as_ref(), to.as_ref()) {
                ("md", "html") => {
                    let html = converter::md_to_html(&result, if style_html {style} else {None});
                    return Ok((html.as_bytes().to_vec(), CatType::Html));
                },
                ("md", "image") => {
                    let html = converter::md_to_html(&result, style);
                    let image = converter::wkhtmltox_convert(&html)?;
                    return Ok((image, CatType::Image));
                },
                ("md", "inline") => {
                    let html = converter::md_to_html(&result, style);
                    let image = converter::wkhtmltox_convert(&html)?;
                    let dyn_img = image::load_from_memory(&image)?;
                    let (img, center) = dyn_img.resize_plus(None, None)?;
                    let inline_encoder = &converter::InlineEncoder::auto_detect(
                        encoder.kitty,
                        encoder.iterm,
                        encoder.sixel,
                    );
                    let inline_img = converter::inline_an_image(&img, Some(center), inline_encoder)?;
                    return Ok((inline_img, CatType::InlineImage))
                },
                ("html", "image") => {
                    let image = converter::wkhtmltox_convert(&result)?;
                    return Ok((image, CatType::Image));
                },
                ("html", "inline") => {
                    let image = converter::wkhtmltox_convert(&result)?;
                    let dyn_img = image::load_from_memory(&image)?;
                    let (img, center) = dyn_img.resize_plus(None, None)?;
                    let inline_encoder = &converter::InlineEncoder::auto_detect(
                        encoder.kitty,
                        encoder.iterm,
                        encoder.sixel,
                    );
                    let inline_img = converter::inline_an_image(&img, Some(center), inline_encoder)?;
                    return Ok((inline_img, CatType::InlineImage))
                },
                _ => return Err(format!(
                    "converting: {} to: {}, is not supported.\nsupported pipeline is: md -> html -> image -> inline_image",
                    from, to
                ).into()),
            };
        }

        Err("Input type is not supported yet".into())
    }
}
