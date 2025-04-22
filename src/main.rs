mod catter;
mod converter;
mod markitdown;
mod prompter;
mod rasteroid;
mod scrapy;

use std::{
    collections::HashMap,
    io::{BufWriter, Write},
};

#[macro_use]
extern crate lazy_static;

use catter::{CatOpts, EncoderForce};
use clap::{
    Arg, ColorChoice, Command,
    builder::{Styles, styling::AnsiColor},
};

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
        .arg(Arg::new("input").index(1).help("file / dir").required(true))
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
    let input = opts.get_one::<String>("input").unwrap();
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

    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout);
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
    match catter::cat(input.clone(), &mut out, Some(opts)) {
        Ok(_) => {
            out.flush().unwrap();
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
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
