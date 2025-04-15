mod catter;
mod converter;
mod prompter;
mod reader;

use std::io::Write;

use catter::Catter;
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
                .value_parser(["html", "md", "image"]),
        )
        .arg(
            Arg::new("style")
                .short('s')
                .help("alternative css file for images, valid options: [default, makurai, <local file>]",)
                .default_value("default")
        )
        .get_matches();

    let input = opts.get_one::<String>("input").unwrap();
    let output = opts.get_one::<String>("output");
    let style = opts.get_one::<String>("style").unwrap();

    let mut out = std::io::stdout();
    let catter = Catter::new(input.clone());
    match catter.cat(output, Some(style)) {
        Ok((val, _)) => out.write_all(&val).expect("failed writing to stdout"),
        Err(e) => eprint!("Error: {}", e),
    }
}
