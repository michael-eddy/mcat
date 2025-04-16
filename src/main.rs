mod catter;
mod converter;
mod image_extended;
mod iterm_encoder;
mod kitty_encoder;
mod prompter;
mod reader;
mod sixel_encoder;
mod term_misc;

use std::io::Write;

#[macro_use]
extern crate lazy_static;

use catter::{Catter, EncoderForce};
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
                .help("add style to raw html too")
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
        .get_matches();

    let input = opts.get_one::<String>("input").unwrap();
    let output = opts.get_one::<String>("output");
    let style = opts.get_one::<String>("theme").unwrap();
    let style_html = *opts.get_one::<bool>("style-html").unwrap();

    let kitty = *opts.get_one::<bool>("kitty").unwrap();
    let iterm = *opts.get_one::<bool>("iterm").unwrap();
    let sixel = *opts.get_one::<bool>("sixel").unwrap();
    let encoder = EncoderForce {
        kitty,
        iterm,
        sixel,
    };

    let mut out = std::io::stdout();
    let catter = Catter::new(input.clone());
    match catter.cat(output, Some(style), style_html, Some(encoder)) {
        Ok((val, _)) => out.write_all(&val).expect("failed writing to stdout"),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
