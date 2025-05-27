use std::{
    env,
    fs::{self, File},
    io::{Write, stdout},
    path::Path,
};

use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode},
    tty::IsTty,
};
use image::{DynamicImage, ImageFormat};
use pager::Pager;
use rasteroid::{
    InlineEncoder,
    image_extended::{InlineImage, Viewport},
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
            // let dyn_img = dyn_img.zoom_pan(opts.zoom, opts.x, opts.y);
            let (img, center, _, _) = dyn_img.resize_plus(opts.width, opts.height, resize_for_ascii, false)?;
            if opts.report {
                rasteroid::term_misc::report_size(opts.width.unwrap_or_default(), opts.height.unwrap_or_default());
            }
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, None, opts.encoder)?;
            Ok(CatType::InlineImage)
        },
        ("md", "interactive") => {
            let html = markdown::md_to_html(&string_result.unwrap(), opts.style);
            let image = converter::html_to_image(&html)?;
            let dyn_img = image::load_from_memory(&image)?;
            let term_height = term_misc::get_wininfo().sc_height;
            let term_width = rasteroid::term_misc::get_wininfo().sc_width;
            let height_cells = term_misc::dim_to_cells(opts.height.unwrap_or_default(), term_misc::SizeDirection::Height)?;
            let height = (term_height-3).min(height_cells as u16);
            run_interactive_viewer(|state| {
                clear_screen(out ).unwrap();
                // let new_img = dyn_img.clone().zoom_pan(Some(state.zoom), Some(state.x), Some(state.y));
                let (img, center, _, _) = dyn_img.resize_plus(opts.width, Some(&format!("{height}c")), resize_for_ascii, false).unwrap();
                if resize_for_ascii {
                    disable_raw_mode().unwrap();
                }
                rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, None, opts.encoder).unwrap();
                show_help_prompt(out, term_width, term_height, state).unwrap();
                out.flush().unwrap();
                if resize_for_ascii {
                    enable_raw_mode().unwrap();
                }
                false
            }).unwrap();
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
            // let dyn_img = dyn_img.zoom_pan(opts.zoom, opts.x, opts.y);
            let (img, center, _, _) = dyn_img.resize_plus(opts.width, opts.height, resize_for_ascii, false)?;
            if opts.report {
                rasteroid::term_misc::report_size(opts.width.unwrap_or_default(), opts.height.unwrap_or_default());
            }
            rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, None, opts.encoder)?;
            Ok(CatType::InlineImage)
        },
        ("html", "interactive") => {
            let html = &string_result.unwrap();
            let image = converter::html_to_image(&html)?;
            let dyn_img = image::load_from_memory(&image)?;
            let term_height = term_misc::get_wininfo().sc_height;
            let term_width = rasteroid::term_misc::get_wininfo().sc_width;
            let height_cells = term_misc::dim_to_cells(opts.height.unwrap_or_default(), term_misc::SizeDirection::Height)?;
            let height = (term_height-3).min(height_cells as u16);
            run_interactive_viewer(|state| {
                clear_screen(out ).unwrap();
                // let new_img = dyn_img.clone().zoom_pan(Some(state.zoom), Some(state.x), Some(state.y));
                let (img, center, _, _) = dyn_img.resize_plus(opts.width, Some(&format!("{height}c")), resize_for_ascii, false).unwrap();
                if resize_for_ascii {
                    disable_raw_mode().unwrap();
                }
                rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, None, opts.encoder).unwrap();
                show_help_prompt(out, term_width, term_height, state).unwrap();
                out.flush().unwrap();
                if resize_for_ascii {
                    enable_raw_mode().unwrap();
                }
                false
            }).unwrap();
            Ok(CatType::Interactive)
        },
        ("image", "image") => {
            let buf = fs::read(path)?;
            out.write_all(&buf)?;
            Ok(CatType::Image)
        },
        ("image", "interactive") => {
            let dyn_img = image_result.unwrap();
            let term_height = term_misc::get_wininfo().sc_height;
            let term_width = rasteroid::term_misc::get_wininfo().sc_width;
            let height_cells = term_misc::dim_to_cells(opts.height.unwrap_or_default(), term_misc::SizeDirection::Height)?;
            let height = (term_height-3).min(height_cells as u16);
            run_interactive_viewer(|state| {
                clear_screen(out).unwrap();
                let mut viewport = Viewport::new(&dyn_img);
                viewport.zoom(state.zoom as f32, None, None);
                viewport.pan(state.x, state.y);
                let new_img = viewport.apply(&dyn_img);
                let (img, center, _, _) = new_img.resize_plus(opts.width, Some(&format!("{height}c")), resize_for_ascii, false).unwrap();
                if resize_for_ascii {
                    disable_raw_mode().unwrap();
                }
                rasteroid::inline_an_image(&img, out, if opts.center {Some(center)} else {None}, None, opts.encoder).unwrap();
                show_help_prompt(out, term_width, term_height, state).unwrap();
                out.flush().unwrap();
                if resize_for_ascii {
                    enable_raw_mode().unwrap();
                }
                false
            }).unwrap();
            Ok(CatType::Interactive)
        },
        ("md", _) => {
            //default for md
            let res = string_result.unwrap();
            if stdout().is_tty() {
                let ansi = markdown::md_to_ansi(&res, opts.style);
                if ansi.lines().count() > term_misc::get_wininfo().sc_height as usize {
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
            // let image_result = image_result.unwrap().zoom_pan(opts.zoom, opts.x, opts.y);
            let image_result = image_result.unwrap();
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
