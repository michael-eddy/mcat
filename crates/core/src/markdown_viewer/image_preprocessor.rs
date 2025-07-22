use std::{collections::HashMap, fs};

use comrak::nodes::{AstNode, NodeValue};
use image::{DynamicImage, GenericImageView, ImageFormat};
use rasteroid::{
    InlineEncoder,
    image_extended::InlineImage,
    inline_an_image,
    term_misc::{self},
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use tempfile::NamedTempFile;

use crate::{
    config::{McatConfig, MdImageRender},
    converter::svg_to_image,
    scrapy::scrape_biggest_media,
};

pub struct ImagePreprocessor {
    pub mapper: HashMap<String, ImageElement>,
}

impl ImagePreprocessor {
    pub fn new<'a>(node: &'a AstNode<'a>, conf: &McatConfig) -> Self {
        let mut urls = Vec::new();
        extract_image_urls(node, &mut urls);

        let items: Vec<(&ImageUrl, Vec<u8>)> = urls
            .par_iter()
            .filter_map(|url| {
                // fail everything early if needed.
                if conf.md_image_render == MdImageRender::None
                    || conf.inline_encoder == InlineEncoder::Sixel
                    || conf.inline_encoder == InlineEncoder::Ascii
                {
                    return None;
                }

                let tmp = scrape_biggest_media(&url.base_url, conf.silent).ok()?;
                let img = render_image(tmp, url.width, url.height)?;

                let (width, height) = img.dimensions();
                let width = url.width.map(|v| v as u32).unwrap_or(width);
                let height = url.height.map(|v| v as u32).unwrap_or(height);
                let width_fm = if width as f32 > term_misc::get_wininfo().spx_width as f32 * 0.8 {
                    "80%"
                } else {
                    &format!("{width}px")
                };
                let height_fm = if height as f32 > term_misc::get_wininfo().spx_height as f32 * 0.4
                {
                    "40%"
                } else {
                    &format!("{height}px")
                };

                // let cols = term_misc::dim_to_cells(width_fm, term_misc::SizeDirection::Width)
                //     .unwrap_or_default();
                // let rows = term_misc::dim_to_cells(height_fm, term_misc::SizeDirection::Height)
                //     .unwrap_or_default();

                let img = img
                    .resize_plus(Some(&width_fm), Some(&height_fm), false, false)
                    .ok()?;

                return Some((url, img.0));
            })
            .collect();

        let mut mapper: HashMap<String, ImageElement> = HashMap::new();
        for (i, item) in items.iter().enumerate() {
            let mut buffer = Vec::new();
            if inline_an_image(&item.1, &mut buffer, None, None, &conf.inline_encoder).is_ok() {
                let img = String::from_utf8(buffer).unwrap_or_default();
                let img = ImageElement {
                    is_ok: true,
                    placeholder: create_placeholder(&img, i),
                    img,
                };
                mapper.insert(item.0.original_url.clone(), img);
            }
        }

        ImagePreprocessor { mapper }
    }
}

fn create_placeholder(img: &str, id: usize) -> String {
    let fg_color = 16 + (id % 216); // 256-color palette (16-231)
    let bg_color = 16 + ((id / 216) % 216);

    let placeholder = "\u{10EEEE}";
    let first_line = img.lines().next().unwrap_or("");
    let count = first_line.matches(placeholder).count();

    format!(
        "\x1b[38;5;{}m\x1b[48;5;{}m{}\x1b[0m",
        fg_color,
        bg_color,
        "█".repeat(count)
    )
}

fn render_image(
    tmp: NamedTempFile,
    width: Option<u16>,
    height: Option<u16>,
) -> Option<DynamicImage> {
    let width = width.map(|v| v.to_string());
    let height = height.map(|v| v.to_string());
    let ext = tmp.path().extension().unwrap_or_default().to_string_lossy();
    let dyn_img = if ext == "svg" {
        let buf = fs::read(tmp).ok()?;
        svg_to_image(buf.as_slice(), width.as_deref(), height.as_deref()).ok()?
    } else if ImageFormat::from_extension(ext.as_ref()).is_some() {
        let buf = fs::read(tmp).ok()?;
        image::load_from_memory(&buf).ok()?
    } else {
        return None;
    };

    Some(dyn_img)
}

pub struct ImageElement {
    pub is_ok: bool,
    pub placeholder: String,
    pub img: String,
}

#[derive(Debug)]
struct ImageUrl {
    base_url: String,
    original_url: String,
    width: Option<u16>,
    height: Option<u16>,
}
fn extract_image_urls<'a>(node: &'a AstNode<'a>, urls: &mut Vec<ImageUrl>) {
    let data = node.data.borrow();

    if let NodeValue::Image(image_node) = &data.value {
        // regex for; <URL>#<Width>x<Height>
        // width and height are optional.
        let regex = Regex::new(r"^(.+?)(?:#(\d+)?x(\d+)?)?$").unwrap();
        if let Some(captures) = regex.captures(&image_node.url) {
            if let Some(base_url) = captures.get(1) {
                let width = captures.get(2).and_then(|v| v.as_str().parse::<u16>().ok());
                let height = captures.get(3).and_then(|v| v.as_str().parse::<u16>().ok());
                urls.push(ImageUrl {
                    base_url: base_url.as_str().to_owned(),
                    original_url: image_node.url.clone(),
                    width,
                    height,
                });
            }
        }
    }

    for child in node.children() {
        extract_image_urls(child, urls);
    }
}
