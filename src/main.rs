mod catter;
mod converter;
mod prompter;
mod reader;

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
                .value_parser(["html", "md"]),
        )
        .get_matches();

    let input = opts.get_one::<String>("input").unwrap();
    let output = opts.get_one::<String>("output");

    let catter = Catter::new(input.clone());
    match catter.cat(output) {
        Ok(md) => println!("{}", md),
        Err(e) => eprint!("Error: {}", e),
    }
}
