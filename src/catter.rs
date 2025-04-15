use std::path::Path;

use crate::{
    converter::{self},
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

impl Catter {
    pub fn new(input: String) -> Self {
        Catter { input }
    }
    pub fn cat(
        &self,
        to: Option<&String>,
        style: Option<&str>,
        style_html: bool,
    ) -> Result<(Vec<u8>, CatType), Box<dyn std::error::Error>> {
        let mut from = String::new();
        let mut result = String::new();

        let path = Path::new(&self.input);

        // local file or dir
        if path.exists() {
            (result, from) = reader::read_file(&path)?;
            if to.is_none() {
                return Ok((result.as_bytes().to_vec(), CatType::Markdown));
            }
        } else {
            return Err(format!("invalid path: {}", path.display()).into());
        }

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
                ("md", "inline_image") => todo!(),
                ("html", "image") => {
                    let image = converter::wkhtmltox_convert(&result)?;
                    return Ok((image, CatType::Image));
                },
                ("html", "inline_image") => todo!(),
                ("image", "inline_image") => todo!(),
                _ => return Err(format!(
                    "converting: {} to: {}, is not supported.\nsupported pipeline is: md -> html -> image -> inline_image",
                    from, to
                ).into()),
            };
        }

        Err("Input type is not supported yet".into())
    }
}
