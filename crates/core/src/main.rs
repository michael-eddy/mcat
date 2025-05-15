mod catter;
mod concater;
mod converter;
mod fetch_manager;
mod inspector;
mod prompter;
mod scrapy;

use std::{
    collections::HashMap,
    io::{BufWriter, Read, Write},
    path::Path,
};

use catter::{CatOpts, EncoderForce};
use clap::{
    Arg, ColorChoice, Command,
    builder::{Styles, styling::AnsiColor},
};
use crossterm::tty::IsTty;
use dirs::home_dir;
use rasteroid::term_misc;

fn main() {
    let stdin_steamed = !std::io::stdin().is_tty();
    let mut input_arg = Arg::new("input")
        .index(1)
        .num_args(1..)
        .help("file / dir / url");
    if !stdin_steamed {
        input_arg = input_arg.required_unless_present_any([
            "fetch-clean",
            "fetch-chromium",
            "fetch-ffmpeg",
            "report",
        ]);
    }
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
        .arg(input_arg)
        .arg(
            Arg::new("output")
                .short('o')
                .help("the format to output")
                .value_parser(["html", "md", "pretty", "image", "video", "inline"]),
        )
        .arg(
            Arg::new("theme")
                .short('t')
                .help("alternative css file for images, valid options: [light, dark, <local file>]",)
                .default_value("light")
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
            Arg::new("ascii")
                .long("ascii")
                .help("makes the inline image encoded to ascii")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("inline")
                .short('i')
                .help("shortcut for putting --output inline")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("pretty")
                .short('p')
                .help("shortcut for putting --output pretty")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("dark-theme")
                .short('d')
                .help("shortcut for putting --theme dark")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("horizontal")
                .long("hori")
                .action(clap::ArgAction::SetTrue)
                .help("concat images horizontal instead of vertical"))
        .arg(
            Arg::new("inline-options")
                .long("inline-options")
                .help("options for the --output inline\n*  center=<bool>\n*  width=<string> [only for images]\n*  height=<string> [only for images]\n*  scale=<f32>\n*  spx=<string>\n*  sc=<string>\n*  zoom=<usize> [only for images]\n*  x=<int> [only for images]\n*  y=<int> [only for images]\n*  exmp: --inline-options 'center=false,width=80%,height=20c,scale=0.5,spx=1920x1080,sc=100x20,zoom=2,x=16,y=8'\n")
        )
        .arg(
            Arg::new("report")
                .long("report")
                .action(clap::ArgAction::SetTrue)
                .help("reports image / video dimensions when drawing images. along with reporting more info when not drawing images")
        )
        .arg(
            Arg::new("fetch-chromium")
                .long("fetch-chromium")
                .help("download and prepare chromium")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("fetch-ffmpeg")
                .long("fetch-ffmpeg")
                .help("download and prepare ffmpeg")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("fetch-clean")
                .long("fetch-clean")
                .help("Clean up the local binaries")
                .action(clap::ArgAction::SetTrue))
        .get_matches();

    //subcommand
    if opts.get_flag("fetch-chromium") {
        fetch_manager::fetch_chromium().unwrap_or_exit();
        return;
    }
    if opts.get_flag("fetch-ffmpeg") {
        fetch_manager::fetch_ffmpeg().unwrap_or_exit();
        return;
    }
    if opts.get_flag("fetch-clean") {
        fetch_manager::clean().unwrap_or_exit();
        return;
    }
    let report = opts.get_flag("report");

    // main
    let input: Vec<String> = opts
        .get_many::<String>("input")
        .unwrap_or_default()
        .cloned()
        .collect();

    // setting the winsize
    let inline_options = opts.get_one::<String>("inline-options").map(|s| s.as_str());
    let inline_options = InlineOptions::from_string(inline_options.unwrap_or_default());
    let _ = term_misc::init_winsize(
        &term_misc::break_size_string(inline_options.spx.unwrap_or_default()).unwrap_or_exit(),
        &term_misc::break_size_string(inline_options.sc.unwrap_or_default()).unwrap_or_exit(),
        inline_options.scale,
    );

    // reporting and leaving
    if report && input.is_empty() {
        report_and_leave();
    }

    // rest
    let output = opts.get_one::<String>("output");
    let style = opts.get_one::<String>("theme").unwrap();
    let style_html = opts.get_flag("style-html");
    let hori = *opts.get_one::<bool>("horizontal").unwrap();

    // shortcuts
    let dark = opts.get_flag("dark-theme");
    let style: &str = if dark { "dark" } else { style };

    let inline = opts.get_flag("inline");
    let pretty = opts.get_flag("pretty");
    let output: Option<&str> = if inline {
        Some("inline")
    } else if pretty {
        Some("pretty")
    } else {
        match output {
            Some(o) => Some(o.as_ref()),
            None => None,
        }
    };

    // encoders
    let kitty = opts.get_flag("kitty");
    let iterm = opts.get_flag("iterm");
    let sixel = opts.get_flag("sixel");
    let ascii = opts.get_flag("ascii");
    let encoder = EncoderForce {
        kitty,
        iterm,
        sixel,
        ascii,
    };

    let opts = CatOpts {
        to: output,
        width: inline_options.width,
        height: inline_options.height,
        center: inline_options.center,
        zoom: inline_options.zoom,
        x: inline_options.x,
        y: inline_options.y,
        encoder: Some(encoder),
        style: Some(style),
        style_html,
        report,
    };

    let mut tmp_files = Vec::new(); //for lifetime
    let mut path_bufs = Vec::new();
    // if stdin is streamed into
    if stdin_steamed {
        let mut buffer = Vec::new();
        std::io::stdin().read_to_end(&mut buffer).unwrap_or_exit();

        let inter = inspector::InspectedBytes::from_bytes(&buffer).unwrap_or_exit();
        match inter {
            inspector::InspectedBytes::File(named_temp_file) => {
                let path = named_temp_file.path().to_path_buf();
                path_bufs.push((path, Some("stdin input".to_string())));
                tmp_files.push(named_temp_file);
            }
            inspector::InspectedBytes::Path(path_buf) => path_bufs.push((path_buf, None)),
        };
    }
    for i in input {
        if i.starts_with("https://") {
            if let Ok(tmp) = scrapy::scrape_biggest_media(&i) {
                let path = tmp.path().to_path_buf();
                tmp_files.push(tmp);
                path_bufs.push((path, Some(i)));
            } else {
                eprintln!("{} didn't contain any supported media", i);
            }
        } else {
            let i = expand_tilde(&i);
            let path = Path::new(&i);
            if !path.exists() {
                eprintln!("{} doesn't exists", path.display());
                std::process::exit(1);
            }
            if path.is_dir() {
                path_bufs.clear();
                let mut selected_files = prompter::prompt_for_files(path).unwrap_or_exit();
                selected_files.sort();
                path_bufs.extend_from_slice(&selected_files);
                break;
            } else {
                path_bufs.push((path.to_path_buf(), None));
            }
        }
    }

    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout);
    let main_format = concater::check_unified_format(&path_bufs);
    match main_format {
        "text" => {
            let path_bufs = concater::assign_names(&path_bufs);
            let tmp = concater::concat_text(path_bufs);
            catter::cat(tmp.path(), &mut out, Some(opts)).unwrap_or_exit();
        }
        "video" => {
            if path_bufs.len() == 1 {
                catter::cat(&path_bufs[0].0, &mut out, Some(opts)).unwrap_or_exit();
            } else {
                #[allow(unused_variables)] //for lifetime
                let (dir, path) = concater::concat_video(&path_bufs).unwrap_or_exit();
                catter::cat(&path, &mut out, Some(opts)).unwrap_or_exit();
            }
        }
        "image" => {
            if path_bufs.len() == 1 {
                catter::cat(&path_bufs[0].0, &mut out, Some(opts)).unwrap_or_exit();
            } else {
                let img = concater::concat_images(path_bufs, hori).unwrap_or_exit();
                catter::cat(img.path(), &mut out, Some(opts)).unwrap_or_exit();
            }
        }
        _ => {}
    }
    out.flush().unwrap();
}

#[derive(Debug)]
struct InlineOptions<'a> {
    width: Option<&'a str>,
    height: Option<&'a str>,
    spx: Option<&'a str>,
    sc: Option<&'a str>,
    scale: Option<f32>,
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
            scale: None,
            zoom: None,
            x: None,
            y: None,
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
        if let Some(&val) = map.get("scale") {
            options.scale = val.parse().ok();
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

trait UnwrapOrExit<T> {
    fn unwrap_or_exit(self) -> T;
}

impl<T, E: std::fmt::Display> UnwrapOrExit<T> for Result<T, E> {
    fn unwrap_or_exit(self) -> T {
        match self {
            Ok(value) => value,
            Err(err) => {
                eprintln!("{}", err);
                std::process::exit(1);
            }
        }
    }
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~") {
        if let Some(home) = home_dir() {
            return path.replace("~", &home.to_string_lossy().into_owned());
        }
    }
    path.to_string()
}

fn report_and_leave() {
    let is_chromium_installed = fetch_manager::is_chromium_installed();
    let is_ffmpeg_installed = fetch_manager::is_ffmpeg_installed();
    let env = term_misc::EnvIdentifiers::new();
    let kitty = rasteroid::kitty_encoder::is_kitty_capable(&env);
    let iterm = rasteroid::iterm_encoder::is_iterm_capable(&env);
    let sixel = rasteroid::sixel_encoder::is_sixel_capable(&env);
    let ascii = true; //not sure what doesn't support it
    let winsize = term_misc::get_winsize();

    // Print header with fancy box
    println!("┌────────────────────────────────────────────────────┐");
    println!("│               SYSTEM CAPABILITIES                  │");
    println!("├────────────────────────────────────────────────────┤");

    // Color function helpers
    fn green(text: &str) -> String {
        format!("\x1b[32m{}\x1b[0m", text)
    }

    fn red(text: &str) -> String {
        format!("\x1b[31m{}\x1b[0m", text)
    }

    fn format_status(status: bool) -> String {
        if status {
            green("✓ INSTALLED")
        } else {
            red("× MISSING")
        }
    }
    fn format_capability(status: bool) -> String {
        if status {
            green("✓ SUPPORTED")
        } else {
            red("× UNSUPPORTED")
        }
    }

    // Print required dependencies
    println!("│ Dependencies:                                      │");
    println!(
        "│   Chromium: {:<47} │",
        format_status(is_chromium_installed)
    );
    println!("│   FFmpeg:   {:<47} │", format_status(is_ffmpeg_installed));

    // Print terminal capabilities
    println!("├────────────────────────────────────────────────────┤");
    println!("│ Terminal Graphics Support:                         │");
    println!("│   Kitty:    {:<47} │", format_capability(kitty));
    println!("│   iTerm2:   {:<47} │", format_capability(iterm));
    println!("│   Sixel:    {:<47} │", format_capability(sixel));
    println!("│   ASCII:    {:<47} │", format_capability(ascii));

    // Print terminal dimensions
    println!("├────────────────────────────────────────────────────┤");
    println!("│ Terminal Dimensions:                               │");
    println!("│   Width:        {:<34} │", winsize.sc_width);
    println!("│   Height:       {:<34} │", winsize.sc_height);
    println!("│   Pixel Width:  {:<34} │", winsize.spx_width);
    println!("│   Pixel Height: {:<34} │", winsize.spx_height);

    // Print footer
    println!("└────────────────────────────────────────────────────┘");

    std::process::exit(0);
}
