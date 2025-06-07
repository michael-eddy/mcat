use std::collections::HashMap;

use clap::ArgMatches;
use rasteroid::{InlineEncoder, term_misc};

#[derive(Debug)]
pub struct InlineOptions<'a> {
    pub center: bool,
    pub width: Option<&'a str>,
    pub height: Option<&'a str>,
    pub spx: &'a str,
    pub sc: &'a str,
    pub scale: Option<f32>,
    pub zoom: Option<usize>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub inline: bool,
}

impl<'a> Default for InlineOptions<'a> {
    fn default() -> Self {
        InlineOptions {
            center: true,
            width: Some("80%"),
            height: Some("80%"),
            spx: "1920x1080",
            sc: "100x20",
            scale: None,
            zoom: None,
            x: None,
            y: None,
            inline: false,
        }
    }
}

impl<'a> InlineOptions<'a> {
    pub fn extend_from_string(&mut self, s: &'a str) -> &mut Self {
        let map: HashMap<_, _> = s
            .split(',')
            .filter_map(|pair| {
                let mut split = pair.splitn(2, '=');
                let key = split.next()?.trim();
                let value = split.next()?.trim();
                Some((key, value))
            })
            .collect();

        let get = |key: &str| map.get(key).copied();
        let get_size = |key: &str, default: Option<&'a str>| match map.get(key) {
            Some(v) => {
                if v.eq_ignore_ascii_case("none") {
                    None
                } else {
                    Some(*v)
                }
            }
            None => default,
        };

        self.width = get_size("width", self.width);
        self.height = get_size("height", self.height);
        self.spx = get("spx").unwrap_or(self.spx);
        self.sc = get("sc").unwrap_or(self.sc);
        self.scale = get("scale").and_then(|v| v.parse().ok()).or(self.scale);
        self.zoom = get("zoom").and_then(|v| v.parse().ok()).or(self.zoom);
        self.x = get("x").and_then(|v| v.parse().ok()).or(self.x);
        self.y = get("y").and_then(|v| v.parse().ok()).or(self.y);
        self.center = get("center")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(self.center);
        self.inline = get("inline")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(self.inline);
        self
    }
}

pub struct LsixOptions<'a> {
    pub x_padding: &'a str,
    pub y_padding: &'a str,
    pub min_width: &'a str,
    pub max_width: &'a str,
    pub height: &'a str,
    pub max_items_per_row: usize,
}

impl<'a> Default for LsixOptions<'a> {
    fn default() -> Self {
        LsixOptions {
            x_padding: "3c",
            y_padding: "2c",
            min_width: "2c",
            max_width: "16c",
            height: "2c",
            max_items_per_row: 20,
        }
    }
}

impl<'a> LsixOptions<'a> {
    pub fn extend_from_string(&mut self, s: &'a str) -> &mut Self {
        let map: HashMap<_, _> = s
            .split(',')
            .filter_map(|pair| {
                let mut split = pair.splitn(2, '=');
                let key = split.next()?.trim();
                let value = split.next()?.trim();
                Some((key, value))
            })
            .collect();

        let get = |key: &str| map.get(key).copied();

        self.x_padding = get("x_padding").unwrap_or(self.x_padding);
        self.y_padding = get("y_padding").unwrap_or(self.y_padding);
        self.min_width = get("min_width").unwrap_or(self.min_width);
        self.max_width = get("max_width").unwrap_or(self.max_width);
        self.height = get("height").unwrap_or(self.height);
        self.max_items_per_row = get("items_per_row")
            .and_then(|v| v.parse().ok())
            .unwrap_or(self.max_items_per_row);
        self
    }
}

// values only in the args
// output: Option<&'a str>,
// delete_all_images: bool,
// report: bool,
// fetch_chromium: bool,
// fetch_ffmpeg: bool,
// fetch_clean: bool,
// generate_completions: bool,
pub struct McatConfig<'a> {
    pub input: Vec<String>,
    pub output: Option<&'a str>,
    pub is_ls: bool,
    pub inline_encoder: InlineEncoder,
    pub ls_options: LsixOptions<'a>,
    pub inline_options: InlineOptions<'a>,
    pub is_tmux: bool,
    pub silent: bool,
    pub hidden: bool,
    pub report: bool,
    pub no_linenumbers: bool,
    pub horizontal_image_stacking: bool,
    pub style_html: bool,
    pub theme: &'a str,
    pub fn_and_leave: Option<FnAndLeave>,
}

pub enum FnAndLeave {
    ShellGenerate(String),
    DeleteImages,
    FetchChromium,
    FetchFfmpeg,
    FetchClean,
    Report,
}

impl<'a> Default for McatConfig<'a> {
    fn default() -> Self {
        McatConfig {
            input: Vec::new(),
            output: None,
            is_ls: false,
            inline_encoder: InlineEncoder::Ascii,
            is_tmux: false,
            ls_options: LsixOptions::default(),
            inline_options: InlineOptions::default(),
            silent: false,
            hidden: true,
            report: false,
            no_linenumbers: false,
            horizontal_image_stacking: false,
            style_html: false,
            theme: "dark",
            fn_and_leave: None,
        }
    }
}

impl<'a> McatConfig<'a> {
    pub fn extend_from_args(&mut self, opts: &'a ArgMatches) -> &mut Self {
        self.input = opts
            .get_many::<String>("input")
            .unwrap_or_default()
            .cloned()
            .collect();
        self.is_ls = self.input.get(0).unwrap_or(&"".to_owned()).to_lowercase() == "ls";

        // encoder
        let kitty = opts.get_flag("kitty");
        let iterm = opts.get_flag("iterm");
        let sixel = opts.get_flag("sixel");
        let ascii = opts.get_flag("ascii");
        let mut env = term_misc::EnvIdentifiers::new();
        self.inline_encoder =
            rasteroid::InlineEncoder::auto_detect(kitty, iterm, sixel, ascii, &mut env);
        self.is_tmux = env.is_tmux();

        // fn and leave
        if let Some(shell) = opts.get_one::<String>("generate-completions") {
            self.fn_and_leave = Some(FnAndLeave::ShellGenerate(shell.clone()));
            return self;
        }
        if opts.get_flag("delete-all-images") {
            self.fn_and_leave = Some(FnAndLeave::DeleteImages);
            return self;
        }
        if opts.get_flag("fetch-chromium") {
            self.fn_and_leave = Some(FnAndLeave::FetchChromium);
            return self;
        }
        if opts.get_flag("fetch-ffmpeg") {
            self.fn_and_leave = Some(FnAndLeave::FetchFfmpeg);
            return self;
        }
        if opts.get_flag("fetch-clean") {
            self.fn_and_leave = Some(FnAndLeave::FetchClean);
            return self;
        }
        self.report = opts
            .get_one::<bool>("report")
            .copied()
            .unwrap_or(self.report);
        if self.report && self.input.is_empty() {
            self.fn_and_leave = Some(FnAndLeave::Report);
            return self;
        }

        // simple Assignment
        if let Some(ls_options) = opts.get_one::<String>("ls-options") {
            self.ls_options.extend_from_string(&ls_options);
        }
        if let Some(inline_options) = opts.get_one::<String>("inline-options") {
            self.inline_options.extend_from_string(&inline_options);
        }
        self.silent = opts
            .get_one::<bool>("silent")
            .copied()
            .unwrap_or(self.silent);
        self.hidden = opts
            .get_one::<bool>("hidden")
            .copied()
            .unwrap_or(self.hidden);
        self.no_linenumbers = opts
            .get_one::<bool>("no-linenumbers")
            .copied()
            .unwrap_or(self.no_linenumbers);
        self.no_linenumbers = opts
            .get_one::<bool>("no-linenumbers")
            .copied()
            .unwrap_or(self.no_linenumbers);
        self.horizontal_image_stacking = opts
            .get_one::<bool>("horizontal")
            .copied()
            .unwrap_or(self.horizontal_image_stacking);
        self.style_html = opts
            .get_one::<bool>("style-html")
            .copied()
            .unwrap_or(self.style_html);
        self.theme = opts
            .get_one::<String>("theme")
            .map(|v| v.as_ref())
            .unwrap_or(self.theme);

        // output
        let inline = opts.get_flag("inline");
        self.output = if inline {
            Some("inline")
        } else {
            opts.get_one::<String>("output").map(|v| v.as_ref())
        };

        self
    }
}
