mod catter;
mod converter;
mod image_extended;
mod iterm_encoder;
mod kitty_encoder;
mod markdown;
mod markitdown;
mod prompter;
mod sixel_encoder;
mod term_misc;

use std::io::{BufWriter, Write};

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
