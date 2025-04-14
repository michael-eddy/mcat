use std::path::Path;

use crate::reader;
use pulldown_cmark::{Options, Parser, html};

pub struct Catter {
    input: String,
}

impl Catter {
    pub fn new(input: String) -> Self {
        Catter { input }
    }
    pub fn cat(&self, to: Option<&String>) -> Result<String, Box<dyn std::error::Error>> {
        let mut from = String::from("");
        let mut result = String::from("");

        let path = Path::new(&self.input);

        // local file or dir
        if path.exists() {
            (result, from) = reader::read_file(&path)?;
        }

        if let Some(to) = to {
            result = match (from.as_ref(), to.as_ref()) {
                ("md", "html") => Ok(md_to_html(&result)),
                ("md", "image") => todo!(),
                ("md", "inline_image") => todo!(),
                ("html", "image") => todo!(),
                ("html", "inline_image") => todo!(),
                ("image", "inline_image") => todo!(),
                _ => Err(format!(
                    "converting: {} to: {}, is not supported.\nsupported pipeline is: md -> html -> image -> inline_image",
                    from, to
                )),
            }?;
        }

        if !result.is_empty() {
            return Ok(result);
        }

        Err("Input type is not supported yet".into())
    }
}

fn md_to_html(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, Options::empty());
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}
