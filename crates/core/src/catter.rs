use std::{
    error::Error,
    fs::{self, File},
    io::{Write, stdout},
    path::Path,
    process::{Command, Stdio},
};

use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode},
    tty::IsTty,
};
use image::{DynamicImage, ImageFormat};
use rasteroid::{
    InlineEncoder,
    image_extended::{InlineImage, ZoomPanViewport},
    term_misc,
};

use crate::{
    config::McatConfig,
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

pub fn cat(
    path: &Path,
    out: &mut impl Write,
    opts: &McatConfig,
) -> Result<CatType, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("invalid path: {}", path.display()).into());
    }

    let resize_for_ascii = match opts.inline_encoder {
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
    let to = opts.output.unwrap_or("unknown");

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
            &opts.inline_encoder,
            opts.inline_options.width,
            opts.inline_options.height,
            opts.inline_options.center,
            opts.silent,
        )?;
        return Ok(CatType::InlineVideo);
    }
    //svg
    (image_result, from) = if ext == "svg" {
        let file = File::open(path)?;
        let dyn_img =
            converter::svg_to_image(file, opts.inline_options.width, opts.inline_options.height)?;
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
            let html = markdown::md_to_html(&string_result.unwrap(), if opts.style_html {Some(opts.theme)} else {None});
            out.write_all(html.as_bytes())?;
            Ok(CatType::Html)
        },
        ("md", "image") => {
            let html = markdown::md_to_html(&string_result.unwrap(), Some(opts.theme));
            let image = converter::html_to_image(&html)?;
            out.write_all(&image)?;
            Ok(CatType::Image)
        },
        ("md", "inline") => {
            let html = markdown::md_to_html(&string_result.unwrap(), Some(opts.theme));
            let image = converter::html_to_image(&html)?;
            let dyn_img = image::load_from_memory(&image)?;
            let dyn_img = apply_pan_zoom_once(dyn_img, &opts);
            let (img, center, _, _) = dyn_img.resize_plus(opts.inline_options.width, opts.inline_options.height, resize_for_ascii, false)?;
            if opts.report {
                rasteroid::term_misc::report_size(opts.inline_options.width.unwrap_or_default(), opts.inline_options.height.unwrap_or_default());
            }
            rasteroid::inline_an_image(&img, out, if opts.inline_options.center {Some(center)} else {None}, None, &opts.inline_encoder)?;
            Ok(CatType::InlineImage)
        },
        ("md", "interactive") => {
            let html = markdown::md_to_html(&string_result.unwrap(), Some(opts.theme));
            let img_bytes = converter::html_to_image(&html)?;
            let img = image::load_from_memory(&img_bytes)?;
            interact_with_image(img, opts, out, resize_for_ascii)?;
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
            let dyn_img = apply_pan_zoom_once(dyn_img, &opts);
            let (img, center, _, _) = dyn_img.resize_plus(opts.inline_options.width, opts.inline_options.height, resize_for_ascii, false)?;
            if opts.report {
                rasteroid::term_misc::report_size(opts.inline_options.width.unwrap_or_default(), opts.inline_options.height.unwrap_or_default());
            }
            rasteroid::inline_an_image(&img, out, if opts.inline_options.center {Some(center)} else {None}, None, &opts.inline_encoder)?;
            Ok(CatType::InlineImage)
        },
        ("html", "interactive") => {
            let html = &string_result.unwrap();
            let img_bytes = converter::html_to_image(&html)?;
            let img = image::load_from_memory(&img_bytes)?;
            interact_with_image(img, opts, out, resize_for_ascii)?;
            Ok(CatType::Interactive)
        },
        ("image", "image") => {
            let buf = fs::read(path)?;
            out.write_all(&buf)?;
            Ok(CatType::Image)
        },
        ("image", "interactive") => {
            let img = image_result.unwrap();
            interact_with_image(img, opts, out, resize_for_ascii)?;
            Ok(CatType::Interactive)
        },
        ("md", _) => {
            //default for md
            let res = string_result.unwrap();
            let is_tty = stdout().is_tty();
            let use_color = opts.color.should_use(is_tty);
            let content = match use_color {
                true => markdown::md_to_ansi(&res, Some(opts.theme), opts.no_linenumbers),
                false => res,
            };
            let use_pager = opts.paging.should_use(is_tty && content.lines().count() > term_misc::get_wininfo().sc_height as usize);
            if use_pager {
                if let Some(pager) = Pager::new(opts.pager) {
                    if pager.page(&content).is_err() {
                        out.write_all(content.as_bytes())?;
                    }
                } else {
                    out.write_all(content.as_bytes())?;
                }
                Ok(CatType::Pretty)
            } else {
                out.write_all(content.as_bytes())?;
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
            let image_result = apply_pan_zoom_once(image_result.unwrap(), &opts);
            let (img, center, _, _) = image_result.resize_plus(opts.inline_options.width, opts.inline_options.height, resize_for_ascii, false)?;
            if opts.report {
                rasteroid::term_misc::report_size(opts.inline_options.width.unwrap_or_default(), opts.inline_options.height.unwrap_or_default());
            }
            rasteroid::inline_an_image(&img, out, if opts.inline_options.center {Some(center)} else {None}, None, &opts.inline_encoder)?;
            Ok(CatType::InlineImage)
        },
        _ => Err(format!(
            "converting: {} to: {}, is not supported.\nsupported pipeline is: md -> html -> image -> inline_image",
            from, to
        ).into()),
    }
}

fn apply_pan_zoom_once(img: DynamicImage, opts: &McatConfig) -> DynamicImage {
    let zoom = opts.inline_options.zoom.unwrap_or(1);
    let x = opts.inline_options.x.unwrap_or_default();
    let y = opts.inline_options.y.unwrap_or_default();
    if zoom == 1 && x == 0 && y == 0 {
        return img;
    }

    let tinfo = term_misc::get_wininfo();
    let container_width = tinfo.spx_width as u32;
    let container_height = tinfo.spx_height as u32;
    let image_width = img.width();
    let image_height = img.height();

    let mut vp = ZoomPanViewport::new(container_width, container_height, image_width, image_height);
    vp.set_zoom(zoom);
    vp.set_pan(x, y);
    vp.apply_to_image(&img)
}

fn interact_with_image(
    img: DynamicImage,
    opts: &McatConfig,
    out: &mut impl Write,
    resize_for_ascii: bool,
) -> Result<(), Box<dyn Error>> {
    let tinfo = term_misc::get_wininfo();
    let container_width = tinfo.spx_width as u32;
    let container_height = tinfo.spx_height as u32;
    let image_width = img.width();
    let image_height = img.height();

    let height_cells = term_misc::dim_to_cells(
        opts.inline_options.height.unwrap_or_default(),
        term_misc::SizeDirection::Height,
    )?;
    let height = (tinfo.sc_height - 3).min(height_cells as u16);
    let should_disable_raw_mode = match opts.inline_encoder {
        InlineEncoder::Kitty => tinfo.is_tmux,
        InlineEncoder::Ascii => true,
        InlineEncoder::Iterm | InlineEncoder::Sixel => false,
    };

    run_interactive_viewer(
        container_width,
        container_height,
        image_width,
        image_height,
        |vp| {
            let new_img = vp.apply_to_image(&img);
            let (img, center, _, _) = new_img
                .resize_plus(
                    opts.inline_options.width,
                    Some(&format!("{height}c")),
                    resize_for_ascii,
                    false,
                )
                .ok()?;
            if should_disable_raw_mode {
                disable_raw_mode().ok()?;
            }
            let mut buf = Vec::new();
            rasteroid::inline_an_image(
                &img,
                &mut buf,
                if opts.inline_options.center {
                    Some(center)
                } else {
                    None
                },
                None,
                &opts.inline_encoder,
            )
            .ok()?;
            show_help_prompt(&mut buf, tinfo.sc_width, tinfo.sc_height, vp).ok()?;
            clear_screen(out, Some(buf)).ok()?;
            out.flush().ok()?;
            if should_disable_raw_mode {
                enable_raw_mode().ok()?;
            }

            Some(())
        },
    )?;
    clear_screen(out, None)?;
    Ok(())
}

pub fn is_video(ext: &str) -> bool {
    matches!(
        ext,
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "wmv" | "flv" | "m4v" | "ts" | "gif"
    )
}

pub struct Pager {
    command: String,
    args: Vec<String>,
}

impl Pager {
    pub fn command_and_args_from_string(full: &str) -> Option<(String, Vec<String>)> {
        let parts = shell_words::split(full).ok()?;
        let (cmd, args) = parts.split_first()?;
        return Some((cmd.clone(), args.to_vec()));
    }
    pub fn new(def_command: &str) -> Option<Self> {
        let (command, args) = Pager::command_and_args_from_string(def_command)?;
        if which::which(&command).is_ok() {
            return Some(Self { command, args });
        }
        None
    }

    pub fn page(&self, content: &str) -> Result<(), Box<dyn Error>> {
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdin(Stdio::piped())
            .spawn()?;

        if let Some(stdin) = child.stdin.as_mut() {
            // ignoring cuz the pipe will break when the user quits most likely
            let _ = stdin.write_all(content.as_bytes());
        }

        child.wait()?;

        Ok(())
    }
}
