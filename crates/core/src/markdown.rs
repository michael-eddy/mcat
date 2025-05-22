use std::{
    collections::HashMap,
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
};

use comrak::{
    Arena, ComrakOptions, ComrakPlugins, markdown_to_html_with_plugins,
    nodes::{AstNode, NodeCode, NodeMath, NodeValue, Sourcepos},
    plugins::syntect::SyntectAdapter,
};
use rasteroid::term_misc;
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, ScopeSelectors, Style, StyleModifier, Theme, ThemeSettings},
    parsing::SyntaxSet,
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};

const RESET: &str = "\x1B[0m";
const BOLD: &str = "\x1B[1m";
const ITALIC: &str = "\x1B[3m";
const UNDERLINE: &str = "\x1B[4m";
const STRIKETHROUGH: &str = "\x1B[9m";

const FG_BLACK: &str = "\x1B[30m";
const FG_RED: &str = "\x1B[31m";
const FG_GREEN: &str = "\x1B[32m";
const FG_YELLOW: &str = "\x1B[33m";
const FG_BLUE: &str = "\x1B[34m";
const FG_MAGENTA: &str = "\x1B[35m";
const FG_CYAN: &str = "\x1B[36m";
const FG_WHITE: &str = "\x1B[37m";
const FG_BRIGHT_BLACK: &str = "\x1B[90m";
const FG_BRIGHT_RED: &str = "\x1B[91m";
const FG_BRIGHT_GREEN: &str = "\x1B[92m";
const FG_BRIGHT_YELLOW: &str = "\x1B[93m";
const FG_BRIGHT_BLUE: &str = "\x1B[94m";
const FG_BRIGHT_MAGENTA: &str = "\x1B[95m";
const FG_BRIGHT_CYAN: &str = "\x1B[96m";
const FG_BRIGHT_WHITE: &str = "\x1B[97m";

const BG_BLACK: &str = "\x1B[40m";
const BG_RED: &str = "\x1B[41m";
const BG_GREEN: &str = "\x1B[42m";
const BG_YELLOW: &str = "\x1B[43m";
const BG_BLUE: &str = "\x1B[44m";
const BG_MAGENTA: &str = "\x1B[45m";
const BG_CYAN: &str = "\x1B[46m";
const BG_WHITE: &str = "\x1B[47m";
const BG_BRIGHT_BLACK: &str = "\x1B[100m";
const BG_BRIGHT_RED: &str = "\x1B[101m";
const BG_BRIGHT_GREEN: &str = "\x1B[102m";
const BG_BRIGHT_YELLOW: &str = "\x1B[103m";
const BG_BRIGHT_BLUE: &str = "\x1B[104m";
const BG_BRIGHT_MAGENTA: &str = "\x1B[105m";
const BG_BRIGHT_CYAN: &str = "\x1B[106m";
const BG_BRIGHT_WHITE: &str = "\x1B[107m";

struct AnsiContext {
    ps: SyntaxSet,
    theme: CustomTheme,
    line: AtomicUsize,
    output: String,
}
impl AnsiContext {
    fn write(&mut self, val: &str) {
        self.output.push_str(val);
    }
    fn cr(&mut self) {
        self.output.push('\n');
    }
    fn sps(&mut self, sps: Sourcepos) {
        let current_line = self.line.load(Ordering::SeqCst);

        if sps.start.line > current_line {
            let offset = sps.start.line - current_line;
            self.line.store(sps.end.line, Ordering::SeqCst);
            self.output.push_str(&"\n".repeat(offset));
        }
    }
    fn collect<'a>(&self, node: &'a AstNode<'a>) -> String {
        let mut buffer = String::new();
        let line = AtomicUsize::new(node.data.borrow().sourcepos.start.line);
        self.collect_text(node, &mut buffer, &line);
        buffer
    }
    fn collect_and_write<'a>(&mut self, node: &'a AstNode<'a>) {
        let text = self.collect(node);
        self.write(&text);
    }
    fn collect_text<'a>(&self, node: &'a AstNode<'a>, output: &mut String, line: &AtomicUsize) {
        let data = node.data.borrow();

        let sps = data.sourcepos;
        let current_line = line.load(Ordering::SeqCst);
        eprintln!("current: {}. pre: {}", sps.start.line, current_line);
        if sps.start.line > current_line {
            let offset = sps.start.line - current_line;
            line.store(sps.end.line, Ordering::SeqCst);
            output.push_str(&"\n".repeat(offset));
        }

        match &data.value {
            NodeValue::Text(literal) => output.push_str(literal),
            NodeValue::SoftBreak => output.push(' '),
            NodeValue::LineBreak => output.push('\n'),
            NodeValue::Math(NodeMath { literal, .. }) => output.push_str(literal),
            NodeValue::Strong => {
                let mut content = String::new();
                for n in node.children() {
                    self.collect_text(n, &mut content, line);
                }
                output.push_str(&format!("{BOLD}{content}{RESET}"));
            }
            NodeValue::Emph => {
                let mut content = String::new();
                for n in node.children() {
                    self.collect_text(n, &mut content, line);
                }
                output.push_str(&format!("{ITALIC}{content}{RESET}"));
            }
            NodeValue::Strikethrough => {
                let mut content = String::new();
                for n in node.children() {
                    self.collect_text(n, &mut content, line);
                }
                output.push_str(&format!("{STRIKETHROUGH}{content}{RESET}"));
            }
            NodeValue::Link(node_link) => {
                let mut content = String::new();
                for n in node.children() {
                    self.collect_text(n, &mut content, line);
                }
                output.push_str(&format!(
                    "{UNDERLINE}{FG_CYAN}\u{eb01} {}{RESET} {FG_BRIGHT_BLACK}({}){RESET}",
                    content, node_link.url
                ));
            }
            NodeValue::Image(node_link) => {
                let mut content = String::new();
                for n in node.children() {
                    self.collect_text(n, &mut content, line);
                }
                output.push_str(&format!(
                    "{UNDERLINE}{FG_CYAN}\u{f03e} {}{RESET} {FG_BRIGHT_BLACK}({}){RESET}",
                    content, node_link.url
                ));
            }
            NodeValue::Code(node_code) => {
                let surface = self.theme.surface.bg.clone();
                let mut content = String::new();
                for n in node.children() {
                    self.collect_text(n, &mut content, line);
                }
                output.push_str(&format!(
                    "{surface}{FG_GREEN} {} {RESET}",
                    node_code.literal
                ));
            }
            NodeValue::FootnoteReference(_) => {} //disabled
            _ => {
                for n in node.children() {
                    self.collect_text(n, output, line);
                }
            }
        }
    }
}
pub fn md_to_ansi(md: &str) -> String {
    let arena = Arena::new();
    let opts = comrak_options();
    let root = comrak::parse_document(&arena, md, &opts);

    let ps = SyntaxSet::load_defaults_newlines();
    let theme = CustomTheme::dark();
    let mut ctx = AnsiContext {
        ps,
        theme,
        output: String::new(),
        line: AtomicUsize::new(1),
    };
    format_ast_node(root, &mut ctx);

    ctx.output
}

fn comrak_options<'a>() -> ComrakOptions<'a> {
    let mut options = ComrakOptions::default();
    // ➕ Enable extensions
    options.extension.strikethrough = true;
    options.extension.superscript = true;
    options.extension.tagfilter = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.description_lists = true;
    options.extension.math_code = true;
    options.extension.alerts = true;

    // 🎯 Parsing options
    options.parse.smart = true; // fancy quotes, dashes, ellipses
    options.parse.relaxed_tasklist_matching = true;

    // 💄 Render options
    options.render.unsafe_ = true;

    options
}

pub fn md_to_html(markdown: &str, css_path: Option<&str>) -> String {
    let options = comrak_options();

    let mut plugins = ComrakPlugins::default();
    let adapter = SyntectAdapter::new(None);
    plugins.render.codefence_syntax_highlighter = Some(&adapter);

    let css_content = match css_path {
        Some("dark") => Some(include_str!("../styles/dark.css").to_string()),
        Some("light") => Some(include_str!("../styles/light.css").to_string()),
        Some(path) => std::fs::read_to_string(path).ok(),
        None => None,
    };

    let html = markdown_to_html_with_plugins(markdown, &options, &plugins);
    match css_content {
        Some(css) => format!(
            r#"
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <style>{}</style>
</head>
<body>
  {}
</body>
</html>
"#,
            css, html
        ),
        None => html,
    }
}

fn format_ast_node<'a>(node: &'a AstNode<'a>, ctx: &mut AnsiContext) {
    let data = node.data.borrow();
    let sps = data.sourcepos;
    ctx.sps(sps);

    match &data.value {
        NodeValue::Document => {
            for child in node.children() {
                format_ast_node(child, ctx);
            }
        }
        NodeValue::FrontMatter(_) => {
            ctx.write(&format!(
                "FrontMatter [{}-{}]",
                sps.start.line, sps.end.line
            ));
        }
        NodeValue::BlockQuote => {
            for child in node.children() {
                let block_content = ctx.collect(child);

                for line in block_content.lines() {
                    ctx.write(&format!("{FG_YELLOW}▌ {RESET}{}\n", line));
                }
            }
            ctx.write(&format!("BlockQuote [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::List(_) => {
            ctx.write(&format!("List [{}-{}]", sps.start.line, sps.end.line));
            ctx.cr();
        }
        NodeValue::Item(_) => ctx.write(&format!("Item [{}-{}]", sps.start.line, sps.end.line)),
        NodeValue::DescriptionList => ctx.write(&format!(
            "DescriptionList [{}-{}]",
            sps.start.line, sps.end.line
        )),
        NodeValue::DescriptionItem(_) => ctx.write(&format!(
            "DescriptionItem [{}-{}]",
            sps.start.line, sps.end.line
        )),
        NodeValue::DescriptionTerm => ctx.write(&format!(
            "DescriptionTerm [{}-{}]",
            sps.start.line, sps.end.line
        )),
        NodeValue::DescriptionDetails => ctx.write(&format!(
            "DescriptionDetails [{}-{}]",
            sps.start.line, sps.end.line
        )),
        NodeValue::CodeBlock(node_code_block) => {
            let code = &node_code_block.literal;
            let lang = &node_code_block.info;
            let lang = if lang.is_empty() {
                &"txt".to_string()
            } else {
                lang
            };

            let header = match get_lang_icon_and_color(lang) {
                Some((icon, color)) => &format!("{color}{icon} {lang}",),
                None => lang,
            };

            format_code(code, lang, &header, ctx);
            // ctx.write(&format!("CodeBlock [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::HtmlBlock(_) => {
            ctx.write(&format!("HtmlBlock [{}-{}]", sps.start.line, sps.end.line));
        }
        NodeValue::Paragraph => {
            ctx.collect_and_write(node);
            ctx.write(&format!("Paragraph [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::Heading(node_heading) => {
            let prefix = match node_heading.level {
                1 => "㊀",
                2 => "㊁",
                3 => "㊂",
                4 => "㊃",
                5 => "㊄",
                6 => "㊅",
                _ => "",
            };
            let content = ctx.collect(node);
            ctx.write(&format!(
                "{BOLD}{FG_BRIGHT_MAGENTA}{prefix} {content}{RESET}"
            ));
            // ctx.write(&format!("Heading [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::ThematicBreak => {
            let br = br();
            ctx.write(&format!("{FG_BRIGHT_BLACK}{br}{RESET}"));
        }
        NodeValue::FootnoteDefinition(_) => {} //disabled,
        NodeValue::Table(_) => ctx.write(&format!("Table [{}-{}]", sps.start.line, sps.end.line)),
        NodeValue::TableRow(_) => {
            ctx.write(&format!("TableRow [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::TableCell => {
            ctx.write(&format!("TableCell [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::Text(_) => ctx.write(&format!("Text [{}-{}]", sps.start.line, sps.end.line)),
        NodeValue::TaskItem(_) => {
            ctx.write(&format!("TaskItem [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::SoftBreak => {
            ctx.write(&format!("SoftBreak [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::LineBreak => {
            ctx.write(&format!("LineBreak [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::Code(node_code) => {
            // let surface = ctx.theme.surface.bg.clone();
            // ctx.write(&format!(
            //     "{surface}{FG_BRIGHT_MAGENTA} {} {RESET}",
            //     node_code.literal
            // ));
            //todo
            ctx.write(&format!("Code [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::HtmlInline(_) => {
            ctx.write(&format!("HtmlInline [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::Raw(_) => ctx.write(&format!("Raw [{}-{}]", sps.start.line, sps.end.line)),
        NodeValue::Emph => {}          // in text collection
        NodeValue::Strong => {}        //in text collection
        NodeValue::Strikethrough => {} // in text collection
        NodeValue::Superscript => {}   //in text collection
        NodeValue::Link(_) => ctx.write(&format!("Link [{}-{}]", sps.start.line, sps.end.line)),
        NodeValue::Image(_) => ctx.write(&format!("Image [{}-{}]", sps.start.line, sps.end.line)),
        NodeValue::FootnoteReference(_) => {} //disabled
        NodeValue::Math(_) => ctx.write(&format!("Math [{}-{}]", sps.start.line, sps.end.line)),
        NodeValue::MultilineBlockQuote(_) => {
            //not sure what it is
            // handle it like blockquote
        }
        NodeValue::Escaped => ctx.write(&format!("Escaped [{}-{}]", sps.start.line, sps.end.line)),
        NodeValue::WikiLink(_) => {
            ctx.write(&format!("WikiLink [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::Underline => {
            ctx.write(&format!("Underline [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::Subscript => {
            ctx.write(&format!("Subscript [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::SpoileredText => ctx.write(&format!(
            "SpoileredText [{}-{}]",
            sps.start.line, sps.end.line
        )),
        NodeValue::EscapedTag(_) => {
            ctx.write(&format!("EscapedTag [{}-{}]", sps.start.line, sps.end.line))
        }
        NodeValue::Alert(_) => ctx.write(&format!("Alert [{}-{}]", sps.start.line, sps.end.line)),
    }
}

pub fn get_lang_icon_and_color(lang: &str) -> Option<(&'static str, &'static str)> {
    let map: HashMap<&str, (&str, &str)> = [
        ("python", ("\u{e235}", "\x1b[38;5;214m")), // Python yellow-orange
        ("py", ("\u{e235}", "\x1b[38;5;214m")),
        ("rust", ("\u{e7a8}", "\x1b[38;5;166m")), // Rust orange
        ("rs", ("\u{e7a8}", "\x1b[38;5;166m")),
        ("javascript", ("\u{e74e}", "\x1b[38;5;227m")), // JS yellow
        ("js", ("\u{e74e}", "\x1b[38;5;227m")),
        ("typescript", ("\u{e628}", "\x1b[38;5;75m")), // TS blue
        ("ts", ("\u{e628}", "\x1b[38;5;75m")),
        ("go", ("\u{e627}", "\x1b[38;5;81m")), // Go cyan
        ("golang", ("\u{e627}", "\x1b[38;5;81m")),
        ("c", ("\u{e61e}", "\x1b[38;5;68m")),    // C blue
        ("cpp", ("\u{e61d}", "\x1b[38;5;204m")), // C++ pink-red
        ("c++", ("\u{e61d}", "\x1b[38;5;204m")),
        ("cc", ("\u{e61d}", "\x1b[38;5;204m")),
        ("cxx", ("\u{e61d}", "\x1b[38;5;204m")),
        ("java", ("\u{e738}", "\x1b[38;5;208m")), // Java orange
        ("csharp", ("\u{f81a}", "\x1b[38;5;129m")), // C# purple
        ("cs", ("\u{f81a}", "\x1b[38;5;129m")),
        ("ruby", ("\u{e21e}", "\x1b[38;5;196m")), // Ruby red
        ("rb", ("\u{e21e}", "\x1b[38;5;196m")),
        ("php", ("\u{e73d}", "\x1b[38;5;99m")), // PHP purple
        ("swift", ("\u{e755}", "\x1b[38;5;202m")), // Swift orange
        ("kotlin", ("\u{e634}", "\x1b[38;5;141m")), // Kotlin purple
        ("kt", ("\u{e634}", "\x1b[38;5;141m")),
        ("dart", ("\u{e798}", "\x1b[38;5;39m")), // Dart blue
        ("lua", ("\u{e620}", "\x1b[38;5;33m")),  // Lua blue
        ("sh", ("\u{f489}", "\x1b[38;5;34m")),   // Shell green
        ("bash", ("\u{f489}", "\x1b[38;5;34m")),
        ("zsh", ("\u{f489}", "\x1b[38;5;34m")),
        ("fish", ("\u{f489}", "\x1b[38;5;34m")),
        ("html", ("\u{e736}", "\x1b[38;5;202m")), // HTML orange
        ("htm", ("\u{e736}", "\x1b[38;5;202m")),
        ("css", ("\u{e749}", "\x1b[38;5;75m")),   // CSS blue
        ("scss", ("\u{e749}", "\x1b[38;5;199m")), // SCSS pink
        ("sass", ("\u{e74b}", "\x1b[38;5;199m")), // Sass pink
        ("less", ("\u{e758}", "\x1b[38;5;54m")),  // Less purple
        ("jsx", ("\u{e7ba}", "\x1b[38;5;81m")),   // React cyan
        ("tsx", ("\u{e7ba}", "\x1b[38;5;81m")),
        ("vue", ("\u{fd42}", "\x1b[38;5;83m")),   // Vue green
        ("json", ("\u{e60b}", "\x1b[38;5;185m")), // JSON yellow
        ("yaml", ("\u{f481}", "\x1b[38;5;167m")), // YAML orange-red
        ("yml", ("\u{f481}", "\x1b[38;5;167m")),
        ("toml", ("\u{e60b}", "\x1b[38;5;67m")), // TOML blue
        ("xml", ("\u{e619}", "\x1b[38;5;172m")), // XML orange
        ("md", ("\u{f48a}", "\x1b[38;5;255m")),  // Markdown white
        ("markdown", ("\u{f48a}", "\x1b[38;5;255m")),
        ("rst", ("\u{f15c}", "\x1b[38;5;248m")), // reStructuredText gray
        ("tex", ("\u{e600}", "\x1b[38;5;25m")),  // LaTeX blue
        ("latex", ("\u{e600}", "\x1b[38;5;25m")),
        ("txt", ("\u{f15c}", "\x1b[38;5;248m")), // Text gray
        ("text", ("\u{f15c}", "\x1b[38;5;248m")),
        ("log", ("\u{f18d}", "\x1b[38;5;242m")), // Log dark gray
        ("ini", ("\u{f17a}", "\x1b[38;5;172m")), // INI orange
        ("conf", ("\u{f17a}", "\x1b[38;5;172m")), // Config orange
        ("config", ("\u{f17a}", "\x1b[38;5;172m")),
        ("env", ("\u{f462}", "\x1b[38;5;227m")), // Environment yellow
        ("dockerfile", ("\u{f308}", "\x1b[38;5;39m")), // Docker cyan
        ("docker", ("\u{f308}", "\x1b[38;5;39m")),
        ("asm", ("\u{f471}", "\x1b[38;5;124m")), // Assembly dark red
        ("s", ("\u{f471}", "\x1b[38;5;124m")),
        ("haskell", ("\u{e777}", "\x1b[38;5;99m")), // Haskell purple
        ("hs", ("\u{e777}", "\x1b[38;5;99m")),
        ("elm", ("\u{e62c}", "\x1b[38;5;33m")),     // Elm blue
        ("clojure", ("\u{e768}", "\x1b[38;5;34m")), // Clojure green
        ("clj", ("\u{e768}", "\x1b[38;5;34m")),
        ("scala", ("\u{e737}", "\x1b[38;5;196m")), // Scala red
        ("erlang", ("\u{e7b1}", "\x1b[38;5;125m")), // Erlang magenta
        ("erl", ("\u{e7b1}", "\x1b[38;5;125m")),
        ("elixir", ("\u{e62d}", "\x1b[38;5;99m")), // Elixir purple
        ("ex", ("\u{e62d}", "\x1b[38;5;99m")),
        ("exs", ("\u{e62d}", "\x1b[38;5;99m")),
        ("perl", ("\u{e769}", "\x1b[38;5;33m")), // Perl blue
        ("pl", ("\u{e769}", "\x1b[38;5;33m")),
        ("r", ("\u{f25d}", "\x1b[38;5;33m")),       // R blue
        ("matlab", ("\u{f799}", "\x1b[38;5;202m")), // MATLAB orange
        ("m", ("\u{f799}", "\x1b[38;5;202m")),
        ("octave", ("\u{f799}", "\x1b[38;5;202m")), // Octave orange
    ]
    .into();

    map.get(lang.to_lowercase().as_str()).copied()
}

fn format_code(code: &str, lang: &str, header: &str, ctx: &mut AnsiContext) {
    let br = br();
    let surface = ctx.theme.surface.bg.clone();
    let ts = ctx.theme.to_syntect_theme();
    let syntax = ctx
        .ps
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| ctx.ps.find_syntax_plain_text());
    let mut highlighter = HighlightLines::new(syntax, &ts);

    let max_lines = code.lines().count();
    let num_width = max_lines.to_string().chars().count() + 2;
    let term_width = term_misc::get_winsize().sc_width;

    let top_header = format!(
        "{FG_BRIGHT_BLACK}{}┬{}{RESET}",
        "─".repeat(num_width),
        "-".repeat(term_width as usize - num_width - 1)
    );
    let middle_header = format!(
        "{FG_BRIGHT_BLACK}{}│ {header}{RESET}",
        " ".repeat(num_width),
    );
    let bottom_header = format!(
        "{FG_BRIGHT_BLACK}{}┼{}{RESET}",
        "─".repeat(num_width),
        "-".repeat(term_width as usize - num_width - 1)
    );
    ctx.write(&top_header);
    ctx.cr();
    ctx.write(&middle_header);
    ctx.cr();
    ctx.write(&bottom_header);
    ctx.cr();

    let mut num = 1;
    for line in LinesWithEndings::from(code) {
        let left_space = num_width - num.to_string().chars().count();
        let left_offset = left_space / 2;
        let right_offset = left_space - left_offset;
        let ranges: Vec<(Style, &str)> = highlighter.highlight_line(line, &ctx.ps).unwrap();
        let highlighted = as_24_bit_terminal_escaped(&ranges[..], false);
        ctx.write(&format!(
            "{FG_BRIGHT_BLACK}{}{num}{}│ {RESET}{}",
            " ".repeat(left_offset),
            " ".repeat(right_offset),
            highlighted
        ));
        num += 1;
    }

    let last_border = format!(
        "{FG_BRIGHT_BLACK}{}┴{}{RESET}",
        "─".repeat(num_width),
        "-".repeat(term_width as usize - num_width - 1)
    );
    ctx.write(&last_border);
}

fn br() -> String {
    "─".repeat(term_misc::get_winsize().sc_width as usize)
}

#[derive(Debug, Clone)]
pub struct ThemeColor {
    value: String,
    color: Color,
    bg: String,
    fg: String,
}

impl From<&str> for ThemeColor {
    fn from(hex_color: &str) -> Self {
        let color = hex_to_rgba(&hex_color);
        let (r, g, b) = (color.r, color.g, color.b);

        ThemeColor {
            value: hex_color.to_owned(),
            color,
            bg: format!("\x1b[48;2;{};{};{}m", r, g, b),
            fg: format!("\x1b[38;2;{};{};{}m", r, g, b),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CustomTheme {
    pub keyword: ThemeColor,
    pub function: ThemeColor,
    pub string: ThemeColor,
    pub module: ThemeColor,
    pub constant: ThemeColor,
    pub comment: ThemeColor,
    pub foreground: ThemeColor,
    pub guide: ThemeColor,
    pub background: ThemeColor,
    pub surface: ThemeColor,
}

fn hex_to_rgba(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
    Color { r, g, b, a: 255 }
}

impl CustomTheme {
    pub fn dark() -> Self {
        CustomTheme {
            keyword: "#FF7733".into(),
            function: "#FFEE99".into(),
            string: "#95FB79".into(),
            module: "#82AAFF".into(),
            constant: "#D2A6FF".into(),
            comment: "#5C6773".into(),
            foreground: "#FFFFFF".into(),
            guide: "#2D3640".into(),
            background: "#15161B".into(),
            surface: "#20202b".into(),
        }
    }

    pub fn to_syntect_theme(&self) -> Theme {
        let mut settings = ThemeSettings::default();
        settings.foreground = Some(self.foreground.color);
        settings.background = Some(self.background.color);
        settings.guide = Some(self.guide.color);

        let mut theme = Theme {
            name: None,
            author: None,
            settings,
            scopes: vec![],
        };

        fn create_selectors(selectors: &str) -> ScopeSelectors {
            ScopeSelectors::from_str(selectors).unwrap_or_default()
        }
        fn create_style(color: Color) -> StyleModifier {
            StyleModifier {
                foreground: Some(color),
                background: None,
                font_style: None,
            }
        }

        theme.scopes.push(syntect::highlighting::ThemeItem {
            scope: create_selectors("keyword, storage.modifier, storage.type"),
            style: create_style(self.keyword.color),
        });

        theme.scopes.push(syntect::highlighting::ThemeItem {
            scope: create_selectors("entity.name.function, support.function, variable.function"),
            style: create_style(self.function.color),
        });

        theme.scopes.push(syntect::highlighting::ThemeItem {
            scope: create_selectors("module, struct, enum, generic, path"),
            style: create_style(self.module.color),
        });

        theme.scopes.push(syntect::highlighting::ThemeItem {
            scope: create_selectors("string, punctuation.string"),
            style: create_style(self.string.color),
        });

        theme.scopes.push(syntect::highlighting::ThemeItem {
            scope: create_selectors("constant, support.type"),
            style: create_style(self.constant.color),
        });

        theme.scopes.push(syntect::highlighting::ThemeItem {
            scope: create_selectors("comment, punctuation.comment"),
            style: create_style(self.comment.color),
        });

        theme.scopes.push(syntect::highlighting::ThemeItem {
            scope: create_selectors("variable, operator, punctuation, block"),
            style: create_style(self.foreground.color),
        });

        theme
    }

    pub fn to_html_style(&self) -> String {
        todo!()
    }
}
