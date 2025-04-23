mod catter;
mod converter;
mod markitdown;
mod prompter;
mod rasteroid;
mod scrapy;

use std::{
    collections::HashMap,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

#[macro_use]
extern crate lazy_static;

use catter::{CatOpts, EncoderForce};
use clap::{
    Arg, ColorChoice, Command,
    builder::{Styles, styling::AnsiColor},
};
use image::ImageFormat;
use tempfile::NamedTempFile;

fn main() {
    let opts = Command::new("mcat")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .color(ColorChoice::Always)
        .styles(
            Styles::styled()
                .header(AnsiColor::Green.on_default().bold())
                .literal(AnsiColor::Blue.on_default()),
        )
        .arg(Arg::new("input").index(1).num_args(1..).help("file / dir").required(true))
        .arg(
            Arg::new("output")
                .short('o')
                .help("the format to output")
                .value_parser(["html", "md", "image", "inline"]),
        )
        .arg(
            Arg::new("theme")
                .short('t')
                .help("alternative css file for images, valid options: [default, makurai, <local file>]",)
                .default_value("default")
        )
        .arg(
            Arg::new("style-html")
                .short('s')
                .help("add style to html too (when html is the output)")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("inline-options")
                .long("inline-options")
                .help("options for the --output inline\n*  center=<bool>\n*  width=<string> [only for images]\n*  height=<string> [only for images]\n*  spx=<string>\n*  sc=<string>\n*  zoom=<usize> [doesn't work yet]\n*  x=<int> [doesn't work yet]\n*  y=<int> [doesn't work yet]\n*  exmp: --inline-options 'center=false,width=80%,height=20c,spx=1920x1080,sc=100x20,zoom=2,x=16,y=8'\n")
        )
        .arg(
            Arg::new("kitty")
                .long("kitty")
                .help("makes the inline image encoded to kitty")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("iterm")
                .long("iterm")
                .help("makes the inline image encoded to iterm")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("sixel")
                .long("sixel")
                .help("makes the inline image encoded to sixel")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("raw")
                .long("raw")
                .short('r')
                .help("allows raw html to run (put only on your content)")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("inline")
                .short('i')
                .help("shortcut for putting --output inline")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("makurai-theme")
                .short('m')
                .help("shortcut for putting --theme makurai")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // main
    let input: Vec<String> = opts.get_many::<String>("input").unwrap().cloned().collect();
    let output = opts.get_one::<String>("output");
    let style = opts.get_one::<String>("theme").unwrap();
    let style_html = *opts.get_one::<bool>("style-html").unwrap();
    let raw_html = *opts.get_one::<bool>("raw").unwrap();
    let inline_options = opts.get_one::<String>("inline-options").map(|s| s.as_str());
    let inline_options = InlineOptions::from_string(inline_options.unwrap_or_default());

    // shortcuts
    let makurai = *opts.get_one::<bool>("makurai-theme").unwrap();
    let style: &str = if makurai { "makurai" } else { style };

    let inline = *opts.get_one::<bool>("inline").unwrap();
    let output: Option<&str> = if inline {
        Some("inline".as_ref())
    } else {
        match output {
            Some(o) => Some(o.as_ref()),
            None => None,
        }
    };

    // encoders
    let kitty = *opts.get_one::<bool>("kitty").unwrap();
    let iterm = *opts.get_one::<bool>("iterm").unwrap();
    let sixel = *opts.get_one::<bool>("sixel").unwrap();
    let encoder = EncoderForce {
        kitty,
        iterm,
        sixel,
    };

    let opts = CatOpts {
        to: output,
        width: inline_options.width,
        height: inline_options.height,
        center: inline_options.center,
        encoder: Some(encoder),
        style: Some(style),
        style_html,
        raw_html,
    };

    let mut tmp_files = Vec::new(); //for lifetime
    let mut path_bufs = Vec::new();
    let mut base_dir = None;
    for i in input {
        let path = Path::new(&i);
        if i.starts_with("https://") {
            if let Ok(tmp) = scrapy::scrape_biggest_media(&i) {
                let path = tmp.path().to_path_buf();
                tmp_files.push(tmp);
                path_bufs.push(path);
            } else {
                eprintln!("{} didn't contain any supported media", i);
            }
        } else {
            if path.is_dir() {
                path_bufs.clear();
                let selected_files = prompter::prompt_for_files(path).unwrap_or_default();
                path_bufs.extend_from_slice(&selected_files);
                base_dir = Some(path.to_string_lossy().into_owned());
                break;
            } else {
                path_bufs.push(path.to_path_buf());
            }
        }
    }

    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout);
    let main_format = check_unified_format(&path_bufs);
    let mut path_bufs = assign_names(&path_bufs, base_dir.as_ref());
    path_bufs.sort_by_key(|(path, _)| *path);
    match main_format {
        "text" => {
            let tmp = concat_text(path_bufs);
            catter::cat(tmp.path(), &mut out, Some(opts)).unwrap();
        }
        "video" => {
            if path_bufs.len() == 1 {
                catter::cat(path_bufs[0].0, &mut out, Some(opts)).unwrap();
            } else {
                todo!()
            }
        }
        "image" => {
            if path_bufs.len() == 1 {
                catter::cat(path_bufs[0].0, &mut out, Some(opts)).unwrap();
            } else {
                todo!()
            }
        }
        _ => {}
    }
    out.flush().unwrap();
}

fn concat_text(paths: Vec<(&PathBuf, Option<String>)>) -> NamedTempFile {
    let mut markdown = String::new();
    for (path, name) in paths {
        if let Ok(md) = markitdown::convert(&path, name.as_ref()) {
            markdown.push_str(&format!("{}\n\n", md));
        } else {
            markdown.push_str("**[Failed Reading]**\n\n");
        }
    }

    let mut tmp_file = NamedTempFile::with_suffix(".md").expect("failed to create tmp file");
    tmp_file
        .write_all(markdown.trim().as_bytes())
        .expect("failed writing to tmp file");

    tmp_file
}

fn check_unified_format(paths: &[PathBuf]) -> &'static str {
    if paths.is_empty() {
        return "text"; // Default if no files
    }

    let mut detected_format: Option<&'static str> = None;

    for path in paths {
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                let ext = ext_str.to_lowercase();

                let current_format = if catter::is_video(&ext) {
                    "video"
                } else if ImageFormat::from_extension(&ext).is_some() {
                    "image"
                } else {
                    "text"
                };

                if let Some(prev_format) = detected_format {
                    if prev_format != current_format {
                        // Found conflicting formats
                        eprintln!(
                            "Error: Cannot have 2 different formats [text / images / videos]"
                        );
                        std::process::exit(1);
                    }
                } else {
                    // First file, set the format
                    detected_format = Some(current_format);
                }
            }
        } else {
            // Files with no extension are considered text
            if detected_format.is_some() && detected_format.unwrap() != "text" {
                eprintln!("Error: Cannot have 2 different formats");
                std::process::exit(1);
            }
            detected_format = Some("text");
        }
    }
    detected_format.unwrap_or("text")
}

fn assign_names<'a>(
    paths: &'a [PathBuf],
    base_dir: Option<&'a String>,
) -> Vec<(&'a PathBuf, Option<String>)> {
    let is_one_element = paths.len() == 1;
    let result: Vec<(&PathBuf, Option<String>)> = paths
        .iter()
        .map(|path| {
            let name = if is_one_element {
                None
            } else {
                match base_dir {
                    Some(base) => {
                        let rel_path = path.strip_prefix(base).unwrap_or(path);
                        Some(rel_path.to_string_lossy().into_owned())
                    }
                    None => {
                        let name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        Some(name)
                    }
                }
            };
            (path, name)
        })
        .collect();

    result
}

#[derive(Debug)]
struct InlineOptions<'a> {
    width: Option<&'a str>,
    height: Option<&'a str>,
    spx: Option<&'a str>,
    sc: Option<&'a str>,
    zoom: Option<usize>,
    x: Option<i32>,
    y: Option<i32>,
    center: bool,
}

impl<'a> InlineOptions<'a> {
    pub fn from_string(s: &'a str) -> Self {
        let mut options = InlineOptions {
            width: Some("80%"),
            height: Some("80%"),
            spx: Some("1920x1080"),
            sc: Some("100x20"),
            zoom: Some(1),
            x: Some(0),
            y: Some(0),
            center: true,
        };
        let map: HashMap<_, _> = s
            .split(',')
            .filter_map(|pair| {
                let mut split = pair.splitn(2, '=');
                let key = split.next()?.trim();
                let value = split.next()?.trim();
                Some((key, value))
            })
            .collect();

        if let Some(&val) = map.get("width") {
            options.width = Some(val);
        }
        if let Some(&val) = map.get("height") {
            options.height = Some(val);
        }
        if let Some(&val) = map.get("spx") {
            options.spx = Some(val);
        }
        if let Some(&val) = map.get("sc") {
            options.sc = Some(val);
        }
        if let Some(&val) = map.get("zoom") {
            options.zoom = val.parse().ok();
        }
        if let Some(&val) = map.get("x") {
            options.x = val.parse().ok();
        }
        if let Some(&val) = map.get("y") {
            options.y = val.parse().ok();
        }
        if let Some(&val) = map.get("center") {
            options.center = val == "true" || val == "1";
        }

        options
    }
}
