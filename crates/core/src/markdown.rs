use core::str;
use std::{
    collections::HashMap,
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
};

use comrak::{
    Arena, ComrakOptions, ComrakPlugins, markdown_to_html_with_plugins,
    nodes::{AstNode, NodeMath, NodeValue, Sourcepos},
    plugins::syntect::SyntectAdapterBuilder,
};
use itertools::Itertools;
use rasteroid::term_misc::{self, break_size_string};
use regex::Regex;
use strip_ansi_escapes::strip_str;
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, ScopeSelectors, Style, StyleModifier, Theme, ThemeSet, ThemeSettings},
    parsing::SyntaxSet,
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};
use unicode_width::UnicodeWidthStr;

use crate::{UnwrapOrExit, config::McatConfig, html2md};

const RESET: &str = "\x1B[0m";
const BOLD: &str = "\x1B[1m";
const ITALIC: &str = "\x1B[3m";
const UNDERLINE: &str = "\x1B[4m";
const STRIKETHROUGH: &str = "\x1B[9m";
const FAINT: &str = "\x1b[2m";

struct AnsiContext<'a> {
    ps: SyntaxSet,
    theme: CustomTheme,
    hide_line_numbers: bool,
    line: AtomicUsize,
    _config: &'a McatConfig,
    centered_lines: &'a [usize],
}
impl<'a> AnsiContext<'a> {
    fn sps(&self, sps: Sourcepos) -> Option<String> {
        let current_line = self.line.load(Ordering::SeqCst);

        let out = if sps.start.line > current_line {
            let offset = sps.start.line - current_line;
            Some("\n".repeat(offset))
        } else {
            None
        };

        self.line.store(sps.end.line, Ordering::SeqCst);
        out
    }
}
pub fn md_to_ansi(md: &str, config: &McatConfig) -> String {
    let res = &html2md::process(md);
    let md = &res.content;

    let arena = Arena::new();
    let opts = comrak_options();
    let root = comrak::parse_document(&arena, md, &opts);

    // changing to forced inline in case of images rendered
    let _ = term_misc::init_wininfo(
        &break_size_string(&config.inline_options.spx).unwrap_or_exit(),
        &break_size_string(&config.inline_options.spx).unwrap_or_exit(),
        config.inline_options.scale,
        config.is_tmux,
        true,
    );

    let ps = SyntaxSet::load_defaults_newlines();
    let theme = get_theme(Some(&config.theme));
    let mut ctx = AnsiContext {
        ps,
        theme,
        hide_line_numbers: config.no_linenumbers,
        line: AtomicUsize::new(1),
        _config: config,
        centered_lines: &res.centered_lines,
    };

    let mut output = String::new();
    output.push_str(&ctx.theme.foreground.fg);
    output.push_str(&parse_node(root, &mut ctx));

    // making sure its wrapped to fit into the termianl size
    let lines: Vec<String> = textwrap::wrap(&output, term_misc::get_wininfo().sc_width as usize)
        .into_iter()
        .map(|cow| cow.into_owned())
        .collect();
    let res = lines.join("\n");

    // force at max 2 \n at a row (we're adding newlines based on sourcepos)
    let re = Regex::new(r"\n{2,}").unwrap();
    re.replace_all(&res, "\n\n").to_string()
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
    options.extension.wikilinks_title_after_pipe = true;
    options.extension.spoiler = true;
    options.extension.multiline_block_quotes = true;
    options.parse.relaxed_tasklist_matching = true;

    // 🎯 Parsing options
    options.parse.smart = true;

    // 💄 Render options
    options.render.unsafe_ = true;

    options
}

fn get_theme(s: Option<&str>) -> CustomTheme {
    match s.unwrap_or("dark") {
        "catppuccin" => CustomTheme::catppuccin(),
        "nord" => CustomTheme::nord(),
        "monokai" => CustomTheme::monokai(),
        "dracula" => CustomTheme::dracula(),
        "gruvbox" => CustomTheme::gruvbox(),
        "one_dark" => CustomTheme::one_dark(),
        "solarized" => CustomTheme::solarized(),
        "tokyo_night" => CustomTheme::tokyo_night(),
        "makurai_light" => CustomTheme::makurai_light(),
        "makurai_dark" => CustomTheme::makurai_dark(),
        "ayu" => CustomTheme::ayu(),
        "ayu_mirage" => CustomTheme::ayu_mirage(),
        "github" => CustomTheme::github(),
        "synthwave" => CustomTheme::synthwave(),
        "material" => CustomTheme::material(),
        "rose_pine" => CustomTheme::rose_pine(),
        "kanagawa" => CustomTheme::kanagawa(),
        "vscode" => CustomTheme::vscode(),
        "everforest" => CustomTheme::everforest(),
        "autumn" => CustomTheme::autumn(),
        "spring" => CustomTheme::spring(),
        _ => CustomTheme::github(),
    }
}

pub fn md_to_html(markdown: &str, style: Option<&str>) -> String {
    let options = comrak_options();

    let theme = get_theme(style);
    let mut theme_set = ThemeSet::load_defaults();
    let mut plugins = ComrakPlugins::default();
    theme_set
        .themes
        .insert("dark".to_string(), theme.to_syntect_theme());
    let adapter = SyntectAdapterBuilder::new()
        .theme("dark")
        .theme_set(theme_set)
        .build();
    if style.is_some() {
        plugins.render.codefence_syntax_highlighter = Some(&adapter);
    }

    let full_css = match style {
        Some(_) => Some(theme.to_html_style()),
        None => None,
    };

    let html = markdown_to_html_with_plugins(markdown, &options, &plugins);
    match full_css {
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

fn collect<'a>(node: &'a AstNode<'a>, ctx: &AnsiContext) -> String {
    let mut buffer = String::new();

    for child in node.children() {
        buffer.push_str(&parse_node(child, ctx));
    }

    buffer
}
fn parse_node<'a>(node: &'a AstNode<'a>, ctx: &AnsiContext) -> String {
    let data = node.data.borrow();
    let sps = data.sourcepos;
    let mut buffer = String::new();
    if let Some(newlines) = ctx.sps(sps) {
        buffer.push_str(&newlines);
    }

    buffer.push_str(&match &data.value {
        NodeValue::Document => format_document(node, ctx),
        NodeValue::FrontMatter(str) => format_front_matter(str),
        NodeValue::BlockQuote => format_blockquote(node, ctx, 0),
        NodeValue::List(node_list) => format_list(node, node_list, ctx),
        NodeValue::Item(_) => format_item(node, ctx),
        NodeValue::CodeBlock(node_code_block) => format_code_block(node_code_block, ctx, sps),
        NodeValue::HtmlBlock(node_html_block) => format_html_block(node_html_block, ctx, sps),
        NodeValue::Paragraph => format_paragraph(node, ctx, sps),
        NodeValue::Heading(node_heading) => format_heading(node, node_heading, ctx, sps),
        NodeValue::ThematicBreak => format_tb(ctx, sps.start.column),
        NodeValue::FootnoteDefinition(_) => String::new(),
        NodeValue::Table(table) => format_table(node, table, ctx, sps),
        NodeValue::Text(literal) => literal.clone(),
        NodeValue::SoftBreak => " ".to_string(),
        NodeValue::LineBreak => String::new(),
        NodeValue::Math(NodeMath { literal, .. }) => literal.clone(),
        NodeValue::Strong => format_strong(node, ctx),
        NodeValue::Emph => format_emph(node, ctx),
        NodeValue::Strikethrough => format_strikethrough(node, ctx),
        NodeValue::Link(n) => format_link(node, n, ctx),
        NodeValue::Image(_) => format_image(node, ctx),
        NodeValue::Code(node_code) => format_code(node_code, ctx),
        NodeValue::TaskItem(task) => format_task_item(node, task, ctx, sps),
        NodeValue::HtmlInline(html) => format_html_inline(html, ctx),
        NodeValue::Raw(str) => str.clone(),
        NodeValue::Superscript => format_superscript(node, ctx),
        NodeValue::MultilineBlockQuote(node_multi_line) => {
            format_multiline_block_quote(node, node_multi_line, ctx)
        }
        NodeValue::WikiLink(_) => format_wiki_link(node, ctx),
        NodeValue::SpoileredText => format_spoilered_text(node, ctx),
        NodeValue::Alert(node_alert) => format_alert(node, node_alert, ctx),
        NodeValue::TableRow(_) => String::new(),
        NodeValue::TableCell => String::new(),
        NodeValue::Escaped => String::new(),
        NodeValue::DescriptionList => String::new(),
        NodeValue::DescriptionItem(_) => String::new(),
        NodeValue::DescriptionTerm => String::new(),
        NodeValue::DescriptionDetails => String::new(),
        NodeValue::EscapedTag(_) => String::new(),
        NodeValue::Underline => String::new(),
        NodeValue::Subscript => String::new(),
        NodeValue::FootnoteReference(_) => String::new(),
    });
    buffer
}

fn format_code<'a>(node_code: &comrak::nodes::NodeCode, ctx: &AnsiContext) -> String {
    let fg = &ctx.theme.green.fg;
    format!("{fg}{}{RESET}", node_code.literal)
}

fn format_document<'a>(node: &'a AstNode<'a>, ctx: &AnsiContext) -> String {
    node.children()
        .map(|child| parse_node(child, ctx))
        .collect()
}

fn format_front_matter(str: &str) -> String {
    //dk what is that.
    str.to_string()
}

fn format_list<'a>(
    node: &'a AstNode<'a>,
    node_list: &comrak::nodes::NodeList,
    ctx: &AnsiContext,
) -> String {
    let list_type = &node_list.list_type;
    let mut index: i32 = match list_type {
        comrak::nodes::ListType::Bullet => 0,
        comrak::nodes::ListType::Ordered => node_list.start as i32,
    };
    let content = collect(node, ctx);
    let mut result = String::new();
    let bullets = ["●", "○", "◆", "◇"];
    let mut depth = 0;
    let mut pre_offset = 0;

    for (i, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if i != 0 {
            result.push('\n');
        }

        let offset = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();

        let is_nested = offset > pre_offset && i != 0;
        if is_nested {
            depth += 1;
            index -= 1;
        } else if offset < pre_offset {
            depth = 0;
        }
        pre_offset = offset;

        let line = line.trim();
        let bullet = match list_type {
            comrak::nodes::ListType::Bullet => bullets[depth % 4],
            comrak::nodes::ListType::Ordered => &format!("{}.", index + i as i32),
        };
        let offset = " ".repeat(offset);

        if bullet.is_empty() {
            result.push_str(&format!("{offset}{line}"));
        } else {
            let line = if line.contains("\0") {
                line.replace("\0", bullet)
            } else {
                let mut l = line.to_owned();
                for b in bullets {
                    if l.contains(b) {
                        l = l.replace(b, bullet);
                    }
                }
                l
            };
            result.push_str(&format!("{offset}{line}"));
        }
    }
    result
}

fn format_item<'a>(node: &'a AstNode<'a>, ctx: &AnsiContext) -> String {
    let data = node.data.borrow();
    let yellow = ctx.theme.yellow.fg.clone();
    let content = collect(node, ctx);
    format!(
        "{}{yellow}\0{RESET} {content}",
        " ".repeat(data.sourcepos.start.column - 1)
    )
}

fn format_code_block(
    node_code_block: &comrak::nodes::NodeCodeBlock,
    ctx: &AnsiContext,
    sps: Sourcepos,
) -> String {
    let code = &node_code_block.literal;
    let lang = &node_code_block.info;
    let lang = if lang.is_empty() {
        &"txt".to_string()
    } else {
        lang
    };

    let indent = sps.start.column;
    if ctx.hide_line_numbers || code.lines().count() < 10 {
        format_code_simple(code, lang, ctx, indent)
    } else {
        format_code_full(code, lang, ctx, indent)
    }
}

fn format_html_block(
    node_html_block: &comrak::nodes::NodeHtmlBlock,
    ctx: &AnsiContext,
    sps: comrak::nodes::Sourcepos,
) -> String {
    let re = Regex::new(r#"<!--\s*S-TITLE:\s*(.*?)\s*-->"#).unwrap();
    if let Some(caps) = re.captures(&node_html_block.literal) {
        let title = caps.get(1).unwrap().as_str();
        let width = term_misc::get_wininfo().sc_width;
        let text_size = string_len(title);
        let border_width = text_size + 4;
        let center_padding = (width as usize - border_width) / 2;

        let fg_yellow = ctx.theme.yellow.fg.clone();
        let border_line = "─".repeat(border_width);
        let spaces = " ".repeat(center_padding);

        return format!(
            "{spaces}┌{border_line}┐\n{spaces}│  {fg_yellow}{BOLD}{title}{RESET}  │\n{spaces}└{border_line}┘\n"
        );
    }

    if node_html_block.literal.contains("<!--HR-->") {
        return format_tb(ctx, sps.start.column);
    }

    let mut result = String::new();
    let comment = &ctx.theme.comment.fg;
    for line in node_html_block.literal.lines() {
        result.push_str(&format!("{comment}{line}{RESET}\n"));
    }
    if result.ends_with('\n') {
        result.pop();
    }
    result
}

fn format_paragraph<'a>(node: &'a AstNode<'a>, ctx: &AnsiContext, sps: Sourcepos) -> String {
    let lines = collect(node, ctx);
    let sw = term_misc::get_wininfo().sc_width;

    lines
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let current_line_num = sps.start.line + i;
            if ctx.centered_lines.contains(&current_line_num) {
                let line = trim_ansi_string(line.into());
                let le = string_len(&line);
                // 1 based index
                let offset = sps.start.column.saturating_sub(1);
                let offset = (sw as usize - offset).saturating_sub(le).saturating_div(2);
                format!("{}{line}", " ".repeat(offset))
            } else {
                line.into()
            }
        })
        .join("\n")
}

fn format_heading<'a>(
    node: &'a AstNode<'a>,
    node_heading: &comrak::nodes::NodeHeading,
    ctx: &AnsiContext,
    sps: Sourcepos,
) -> String {
    let content = collect(node, ctx);
    let nh = node_heading.level as usize - 1;
    let content = match node_heading.level {
        1 => format!("{}󰲡 {content}", " ".repeat(nh)),
        2 => format!("{}󰲣 {content}", " ".repeat(nh)),
        3 => format!("{}󰲥 {content}", " ".repeat(nh)),
        4 => format!("{}󰲧 {content}", " ".repeat(nh)),
        5 => format!("{}󰲩 {content}", " ".repeat(nh)),
        6 => format!("{}󰲫 {content}", " ".repeat(nh)),
        _ => unreachable!(),
    };
    let bg = &ctx.theme.keyword_bg.bg;
    let main_color = &ctx.theme.keyword.fg;
    let content = content.replace(RESET, &format!("{RESET}{bg}"));

    // TODO handle centering
    if !ctx.centered_lines.contains(&sps.start.line) {
        let padding = " ".repeat(
            term_misc::get_wininfo()
                .sc_width
                .saturating_sub(string_len(&content) as u16)
                .into(),
        );
        format!("{main_color}{bg}{content}{padding}{RESET}")
    } else {
        // center here
        let sw = term_misc::get_wininfo().sc_width as usize;
        let le = string_len(&content);
        let left_space = sw.saturating_sub(le);
        let padding_left = left_space.saturating_div(2);
        let padding_rigth = left_space - padding_left;
        format!(
            "{main_color}{bg}{}{content}{}{RESET}",
            " ".repeat(padding_left),
            " ".repeat(padding_rigth)
        )
    }
}

fn format_table<'a>(
    node: &'a AstNode<'a>,
    table: &comrak::nodes::NodeTable,
    ctx: &AnsiContext,
    sps: Sourcepos,
) -> String {
    let alignments = &table.alignments;
    let mut rows: Vec<Vec<String>> = Vec::new();

    for child in node.children() {
        let mut row_cells: Vec<String> = Vec::new();
        for cell_node in child.children() {
            let cell_content = collect(cell_node, ctx);
            let cell_content = cell_content.trim();
            row_cells.push(cell_content.to_string());
        }
        rows.push(row_cells);
    }

    let mut column_widths: Vec<usize> = vec![0; alignments.len()];
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            let c = string_len(cell.trim());
            if c > column_widths[i] {
                column_widths[i] = c;
            }
        }
    }

    let color = &ctx.theme.border.fg.clone();
    let mut result = String::new();

    if !rows.is_empty() {
        let cols = column_widths.len();

        let build_line = |left: &str, mid: &str, right: &str, fill: &str| -> String {
            let mut line = String::new();
            line.push_str(color);
            line.push_str(left);
            for (i, &width) in column_widths.iter().enumerate() {
                line.push_str(&fill.repeat(width + 2));
                if i < cols - 1 {
                    line.push_str(mid);
                }
            }
            line.push_str(right);
            line.push_str(RESET);
            line
        };

        let top_border = build_line("╭", "┬", "╮", "─");
        let middle_border = build_line("├", "┼", "┤", "─");
        let bottom_border = build_line("╰", "┴", "╯", "─");

        result.push_str(&top_border);
        result.push('\n');

        for (i, row) in rows.iter().enumerate() {
            result.push_str(&format!("{color}│{RESET}"));
            for (j, cell) in row.iter().enumerate() {
                let width = column_widths[j];
                let padding = width - string_len(cell);
                let (left_pad, right_pad) = match alignments[j] {
                    comrak::nodes::TableAlignment::Center => (padding / 2, padding - (padding / 2)),
                    comrak::nodes::TableAlignment::Right => (padding, 0),
                    _ => (0, padding),
                };
                result.push_str(&format!(
                    " {}{}{} {color}│{RESET}",
                    " ".repeat(left_pad),
                    cell,
                    " ".repeat(right_pad)
                ));
            }
            result.push('\n');

            if i == 0 {
                result.push_str(&middle_border);
                result.push('\n');
            }
        }
        result.push_str(&bottom_border);
    }

    if ctx.centered_lines.contains(&sps.start.line) {
        let le = string_len(result.lines().nth(1).unwrap_or_default());
        let tw = term_misc::get_wininfo().sc_width;
        let offset = sps.start.column.saturating_sub(1);
        let offset = (tw as usize - offset).saturating_sub(le).saturating_div(2);

        return result
            .lines()
            .map(|line| format!("{}{line}", " ".repeat(offset)))
            .join("\n");
    }

    result
}

fn format_strong<'a>(node: &'a AstNode<'a>, ctx: &AnsiContext) -> String {
    let content = collect(node, ctx);
    format!("{BOLD}{content}{RESET}")
}

fn format_emph<'a>(node: &'a AstNode<'a>, ctx: &AnsiContext) -> String {
    let content = collect(node, ctx);
    format!("{ITALIC}{content}{RESET}")
}

fn format_strikethrough<'a>(node: &'a AstNode<'a>, ctx: &AnsiContext) -> String {
    let content = collect(node, ctx);
    format!("{STRIKETHROUGH}{content}{RESET}")
}

fn format_link<'a>(
    node: &'a AstNode<'a>,
    _n: &comrak::nodes::NodeLink,
    ctx: &AnsiContext,
) -> String {
    let content = collect(node, ctx);
    let cyan = ctx.theme.cyan.fg.clone();
    format!("{UNDERLINE}{cyan}\u{f0339} {content}{RESET}")
}

fn format_image<'a>(node: &'a AstNode<'a>, ctx: &AnsiContext) -> String {
    let content = collect(node, ctx);
    let cyan = ctx.theme.cyan.fg.clone();
    format!("{UNDERLINE}{cyan}\u{f0976} {}{RESET}", content)
}

fn format_task_item<'a>(
    node: &'a AstNode<'a>,
    task: &Option<char>,
    ctx: &AnsiContext,
    sps: Sourcepos,
) -> String {
    let offset = " ".repeat(sps.start.column - 1);

    let (icon, colour) = match task.map(|c| c.to_ascii_lowercase()) {
        Some('x') => ("󰱒", &ctx.theme.green.fg),
        Some('-') | Some('~') => ("󰛲", &ctx.theme.yellow.fg),
        Some('!') => ("󰳤", &ctx.theme.red.fg),
        _ => ("󰄱", &ctx.theme.red.fg),
    };

    let content = collect(node, ctx);
    format!("{offset}{colour}{icon}{RESET}  {content}")
}

fn format_html_inline(html: &str, ctx: &AnsiContext) -> String {
    let string_color = ctx.theme.string.fg.clone();
    format!("{string_color}{html}{RESET}")
}

fn format_superscript<'a>(node: &'a AstNode<'a>, ctx: &AnsiContext) -> String {
    // mostly for html
    collect(node, ctx)
}

fn format_multiline_block_quote<'a>(
    node: &'a AstNode<'a>,
    node_multi_line: &comrak::nodes::NodeMultilineBlockQuote,
    ctx: &AnsiContext,
) -> String {
    let offset = node_multi_line.fence_offset;
    format_blockquote(node, ctx, offset)
}

fn format_wiki_link<'a>(node: &'a AstNode<'a>, ctx: &AnsiContext) -> String {
    let content = collect(node, ctx);
    let cyan = &ctx.theme.cyan.fg;
    format!("{cyan}\u{f15d6} {}{RESET}", content)
}

fn format_spoilered_text<'a>(node: &'a AstNode<'a>, ctx: &AnsiContext) -> String {
    let content = collect(node, ctx);
    let comment = &ctx.theme.comment.fg;
    format!("{FAINT}{comment}{content}{RESET}")
}

fn format_alert<'a>(
    node: &'a AstNode<'a>,
    node_alert: &comrak::nodes::NodeAlert,
    ctx: &AnsiContext,
) -> String {
    let kind = &node_alert.alert_type;
    let blue = ctx.theme.blue.fg.clone();
    let red = ctx.theme.red.fg.clone();
    let green = ctx.theme.green.fg.clone();
    let cyan = ctx.theme.cyan.fg.clone();
    let yellow = ctx.theme.yellow.fg.clone();

    let (prefix, color) = match kind {
        comrak::nodes::AlertType::Note => ("\u{f05d6} NOTE", blue),
        comrak::nodes::AlertType::Tip => ("\u{f400} TIP", green),
        comrak::nodes::AlertType::Important => ("\u{f017e} INFO", cyan),
        comrak::nodes::AlertType::Warning => ("\u{ea6c} WARNING", yellow),
        comrak::nodes::AlertType::Caution => ("\u{f0ce6} DANGER", red),
    };

    let mut result = format!("{}▌ {BOLD}{}{RESET}", color, prefix);

    for child in node.children() {
        let alert_content = collect(child, ctx);
        for line in alert_content.lines() {
            result.push_str(&format!("\n{}▌{RESET} {}", color, line));
        }
    }

    result
}

fn format_tb(ctx: &AnsiContext, offset: usize) -> String {
    let br = br(offset);
    let border = &ctx.theme.guide.fg;
    format!("{border}{br}{RESET}")
}

fn format_blockquote<'a>(node: &'a AstNode<'a>, ctx: &AnsiContext, fence_offset: usize) -> String {
    let content = collect(node, ctx);
    let guide = ctx.theme.guide.fg.clone();
    let comment = ctx.theme.comment.fg.clone();
    content
        .lines()
        .map(|line| {
            let offset = " ".repeat(fence_offset + 1);
            format!("{guide}▌{offset}{comment}{line}{RESET}")
        })
        .join("\n")
}

fn string_len(str: &str) -> usize {
    strip_ansi_escapes::strip_str(&str).width()
}

fn trim_ansi_string(mut str: String) -> String {
    let stripped = strip_str(&str);
    let mut leading = stripped.chars().take_while(|c| c.is_whitespace()).count();
    let mut trailing = stripped
        .chars()
        .rev()
        .take_while(|c| c.is_whitespace())
        .count();

    if leading == 0 && trailing == 0 {
        return str;
    }

    // Remove first N spaces
    str.retain(|c| {
        if c == ' ' && leading > 0 {
            leading -= 1;
            false
        } else {
            true
        }
    });

    // Remove last N spaces
    let mut i = str.len();
    while i > 0 && trailing > 0 {
        i -= 1;
        if str.as_bytes()[i] == b' ' {
            str.remove(i);
            trailing -= 1;
        }
    }

    str
}

fn format_code_simple(code: &str, lang: &str, ctx: &AnsiContext, indent: usize) -> String {
    let (title, color) = match get_lang_icon_and_color(lang) {
        Some((icon, color)) => (format!("{color}{icon} {lang}"), color),
        None => (lang.to_owned(), ""),
    };

    let top = format!(" {color}{}{RESET}\n", title);
    let surface = ctx.theme.surface.bg.clone();

    let ts = ctx.theme.to_syntect_theme();
    let syntax = ctx
        .ps
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| ctx.ps.find_syntax_plain_text());
    let mut highlighter = HighlightLines::new(syntax, &ts);

    let mut buf = String::new();
    let twidth = term_misc::get_wininfo().sc_width - indent.saturating_sub(1) as u16;
    buf.push_str(&top);
    let count = code.lines().count();
    for (i, line) in LinesWithEndings::from(code).enumerate() {
        if i == count && line.trim().is_empty() {
            continue;
        }
        let ranges: Vec<(Style, &str)> = highlighter.highlight_line(line, &ctx.ps).unwrap();
        let highlighted = as_24_bit_terminal_escaped(&ranges[..], false);
        let highlighted = wrap_highlighted_line(highlighted, twidth as usize - 4, "  ");
        buf.push_str(&highlighted);
    }

    let mut bg_formatted_lines = String::new();
    for (i, line) in buf.lines().enumerate() {
        let left_space = (twidth as usize).saturating_sub(string_len(line));
        if i == 0 {
            let suffix = format!("{surface}{}", " ".repeat(left_space));
            bg_formatted_lines.push_str(&format!("{surface}{line}{suffix}{RESET}"));
        } else {
            let suffix = format!("{surface}{}", " ".repeat(left_space.saturating_sub(2)));
            bg_formatted_lines.push_str(&format!("\n{surface}  {line}{suffix}{RESET}"));
        }
    }

    bg_formatted_lines
}
fn format_code_full<'a>(code: &str, lang: &str, ctx: &AnsiContext, indent: usize) -> String {
    let ts = ctx.theme.to_syntect_theme();
    let syntax = ctx
        .ps
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| ctx.ps.find_syntax_plain_text());
    let mut highlighter = HighlightLines::new(syntax, &ts);

    let header = match get_lang_icon_and_color(lang) {
        Some((icon, color)) => &format!("{color}{icon} {lang}",),
        None => lang,
    };

    let max_lines = code.lines().count();
    let num_width = max_lines.to_string().chars().count() + 2;
    let term_width = term_misc::get_wininfo().sc_width - indent.saturating_sub(1) as u16;
    let text_size = term_width as usize - num_width;
    let color = ctx.theme.border.fg.clone();
    let mut buffer = String::new();

    let top_header = format!(
        "{color}{}┬{}{RESET}",
        "─".repeat(num_width),
        "─".repeat(term_width as usize - num_width - 1)
    );
    let middle_header = format!("{color}{}│ {header}{RESET}", " ".repeat(num_width),);
    let bottom_header = format!(
        "{color}{}┼{}{RESET}",
        "─".repeat(num_width),
        "─".repeat(term_width as usize - num_width - 1)
    );
    buffer.push_str(&format!("{top_header}\n{middle_header}\n{bottom_header}\n"));

    let mut num = 1;
    let prefix = format!("{}{color}│{RESET}  ", " ".repeat(num_width));
    for line in LinesWithEndings::from(code) {
        let left_space = num_width - num.to_string().chars().count();
        let left_offset = left_space / 2;
        let right_offset = left_space - left_offset;
        let ranges: Vec<(Style, &str)> = highlighter.highlight_line(line, &ctx.ps).unwrap();
        let highlighted = as_24_bit_terminal_escaped(&ranges[..], false);
        let highlighted = wrap_highlighted_line(highlighted, text_size - 2, &prefix);
        buffer.push_str(&format!(
            "{color}{}{num}{}│ {RESET}{}",
            " ".repeat(left_offset),
            " ".repeat(right_offset),
            highlighted
        ));
        num += 1;
    }

    let last_border = format!(
        "{color}{}┴{}{RESET}",
        "─".repeat(num_width),
        "─".repeat(term_width as usize - num_width - 1)
    );
    buffer.push_str(&last_border);
    buffer
}

pub fn get_lang_icon_and_color(lang: &str) -> Option<(&'static str, &'static str)> {
    let map: HashMap<&str, (&str, &str)> = [
        // code
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
        ("sh", ("\u{ebca}", "\x1b[38;5;34m")),   // Shell green
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
        ("yaml", ("\u{f0c5}", "\x1b[38;5;167m")), // YAML orange-red
        ("yml", ("\u{f0c5}", "\x1b[38;5;167m")),
        ("toml", ("\u{e6b2}", "\x1b[38;5;131m")),
        ("svg", ("\u{f0721}", "\x1b[38;5;178m")),
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
        ("conf", ("\u{f0ad}", "\x1b[38;5;172m")), // Config orange
        ("config", ("\u{f0ad}", "\x1b[38;5;172m")),
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
        ("zig", ("\u{e6a9}", "\x1b[38;5;214m")),
        ("h", ("\u{e61e}", "\x1b[38;5;110m")),
        ("lock", ("\u{f023}", "\x1b[38;5;244m")),
        // images
        ("png", ("\u{f1c5}", "\x1b[38;5;117m")),
        ("jpg", ("\u{f1c5}", "\x1b[38;5;110m")),
        ("jpeg", ("\u{f1c5}", "\x1b[38;5;110m")),
        ("gif", ("\u{f1c5}", "\x1b[38;5;213m")),
        ("bmp", ("\u{f1c5}", "\x1b[38;5;103m")),
        ("webp", ("\u{f1c5}", "\x1b[38;5;149m")),
        ("tiff", ("\u{f1c5}", "\x1b[38;5;144m")),
        ("ico", ("\u{f1c5}", "\x1b[38;5;221m")),
        // videos
        ("mp4", ("\u{f03d}", "\x1b[38;5;203m")),
        ("mkv", ("\u{f03d}", "\x1b[38;5;132m")),
        ("webm", ("\u{f03d}", "\x1b[38;5;111m")),
        ("mov", ("\u{f03d}", "\x1b[38;5;173m")),
        ("avi", ("\u{f03d}", "\x1b[38;5;167m")),
        ("flv", ("\u{f03d}", "\x1b[38;5;131m")),
        // audio
        ("mp3", ("\u{f001}", "\x1b[38;5;215m")),
        ("ogg", ("\u{f001}", "\x1b[38;5;109m")),
        ("flac", ("\u{f001}", "\x1b[38;5;113m")),
        ("wav", ("\u{f001}", "\x1b[38;5;123m")),
        ("m4a", ("\u{f001}", "\x1b[38;5;174m")),
        // archive
        ("zip", ("\u{f410}", "\x1b[38;5;180m")),
        ("tar", ("\u{f410}", "\x1b[38;5;180m")),
        ("gz", ("\u{f410}", "\x1b[38;5;180m")),
        ("rar", ("\u{f410}", "\x1b[38;5;180m")),
        ("7z", ("\u{f410}", "\x1b[38;5;180m")),
        ("xz", ("\u{f410}", "\x1b[38;5;180m")),
        // documents
        ("pdf", ("\u{f1c1}", "\x1b[38;5;196m")),
        ("doc", ("\u{f1c2}", "\x1b[38;5;33m")),
        ("docx", ("\u{f1c2}", "\x1b[38;5;33m")),
        ("xls", ("\u{f1c3}", "\x1b[38;5;70m")),
        ("xlsx", ("\u{f1c3}", "\x1b[38;5;70m")),
        ("ppt", ("\u{f1c4}", "\x1b[38;5;166m")),
        ("pptx", ("\u{f1c4}", "\x1b[38;5;166m")),
        ("odt", ("\u{f1c2}", "\x1b[38;5;33m")),
        ("epub", ("\u{f02d}", "\x1b[38;5;135m")),
        ("csv", ("\u{f1c3}", "\x1b[38;5;190m")),
        // fonts
        ("ttf", ("\u{f031}", "\x1b[38;5;98m")),
        ("otf", ("\u{f031}", "\x1b[38;5;98m")),
        ("woff", ("\u{f031}", "\x1b[38;5;98m")),
        ("woff2", ("\u{f031}", "\x1b[38;5;98m")),
    ]
    .into();

    map.get(lang.to_lowercase().as_str()).copied()
}
fn find_last_fg_color_sequence(text: &str) -> Option<String> {
    let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    let mut last_fg_color = None;

    for m in re.find_iter(text) {
        let seq = m.as_str();
        let codes_str = &seq[2..seq.len() - 1];

        if codes_str.is_empty() || codes_str == "0" {
            last_fg_color = None;
        } else {
            for code in codes_str.split(';') {
                if let Ok(num) = code.parse::<u32>() {
                    if (30..=37).contains(&num) || (90..=97).contains(&num) || num == 38 {
                        last_fg_color = Some(seq.to_string());
                        break;
                    }
                }
            }
        }
    }

    last_fg_color
}
fn wrap_highlighted_line(original: String, width: usize, sub_prefix: &str) -> String {
    if string_len(&original) <= width {
        return original;
    }
    let lines: Vec<String> = textwrap::wrap(&original, width)
        .into_iter()
        .map(|cow| cow.into_owned())
        .collect();
    let mut buf = String::new();
    let mut pre_padding = 0;
    let mut pre_last_color = None;
    for (i, line) in lines.iter().enumerate() {
        if i == 0 || line.trim().is_empty() {
            buf.push_str(line);
        } else {
            if pre_padding > 0 {
                // index is pointed to the start so +1, +4 for just visual indent
                pre_padding += 5;
            }
            let last_color = match pre_last_color {
                Some(color) => color,
                None => "".into(),
            };
            let padding = " ".repeat(pre_padding);
            buf.push_str(&format!("\n{sub_prefix}{padding}{last_color}{line}"));
        }
        pre_padding = (strip_str(line)).rfind("  ").unwrap_or(0);
        pre_last_color = find_last_fg_color_sequence(line);
    }
    buf.push('\n');
    buf
}

fn br(indent: usize) -> String {
    let w = term_misc::get_wininfo().sc_width as usize;
    // sps starts at 1
    "━".repeat(w.saturating_sub(indent.saturating_sub(1)))
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
    pub border: ThemeColor,
    pub keyword_bg: ThemeColor,

    red: ThemeColor,
    green: ThemeColor,
    blue: ThemeColor,
    cyan: ThemeColor,
    yellow: ThemeColor,

    #[allow(dead_code)]
    magenta: ThemeColor,
    #[allow(dead_code)]
    white: ThemeColor,
    #[allow(dead_code)]
    black: ThemeColor,
}

fn hex_to_rgba(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
    Color { r, g, b, a: 255 }
}

impl CustomTheme {
    pub fn autumn() -> Self {
        CustomTheme {
            keyword: "#fc6501".into(),
            keyword_bg: "#2A1A0D".into(),
            function: "#fac25a".into(),
            string: "#a1cd32".into(),
            module: "#fc4c4c".into(),
            constant: "#FF6B9D".into(),
            comment: "#5C6773".into(),
            foreground: "#FFFFFF".into(),
            guide: "#2D3640".into(),
            background: "#14161f".into(),
            surface: "#1E2129".into(),
            border: "#5C6773".into(),

            red: "#fc4c4c".into(),
            green: "#a1cd32".into(),
            blue: "#5abffa".into(),
            cyan: "#5abffa".into(),
            magenta: "#FF6B9D".into(),
            yellow: "#fac25a".into(),
            white: "#FFFFFF".into(),
            black: "#2e3339".into(),
        }
    }

    pub fn spring() -> Self {
        CustomTheme {
            keyword: "#FFB347".into(),
            keyword_bg: "#2A1F0D".into(),
            function: "#D4FF59".into(),
            string: "#37dbb5".into(),
            module: "#66E6FF".into(),
            constant: "#D8A5FF".into(),
            comment: "#5C6773".into(),
            foreground: "#FFFFFF".into(),
            guide: "#2D3640".into(),
            background: "#14161f".into(),
            surface: "#1E2129".into(),
            border: "#5C6773".into(),

            red: "#FF5555".into(),
            green: "#D4FF59".into(),
            blue: "#66E6FF".into(),
            cyan: "#66E6FF".into(),
            magenta: "#D2A6FF".into(),
            yellow: "#FFB347".into(),
            white: "#FFFFFF".into(),
            black: "#2e3339".into(),
        }
    }

    pub fn makurai_dark() -> Self {
        CustomTheme {
            keyword: "#FF7733".into(),
            keyword_bg: "#261810".into(),
            function: "#FFEE99".into(),
            string: "#95FB79".into(),
            module: "#82AAFF".into(),
            constant: "#D2A6FF".into(),
            comment: "#5C6773".into(),
            foreground: "#FFFFFF".into(),
            guide: "#2D3640".into(),
            background: "#14161f".into(),
            surface: "#1E212A".into(),
            border: "#5C6773".into(),

            red: "#FF5555".into(),
            green: "#95FB79".into(),
            blue: "#82AAFF".into(),
            cyan: "#66D9EF".into(),
            magenta: "#FF77FF".into(),
            yellow: "#FFEE99".into(),
            white: "#FFFFFF".into(),
            black: "#14161f".into(),
        }
    }

    pub fn makurai_light() -> Self {
        CustomTheme {
            keyword: "#E35043".into(),
            keyword_bg: "#FDF2F1".into(),
            function: "#3D76F3".into(),
            string: "#51A150".into(),
            module: "#AB31A9".into(),
            constant: "#976700".into(),
            comment: "#A0A1A7".into(),
            foreground: "#323640".into(),
            guide: "#D1D5DB".into(),
            background: "#f8f8fc".into(),
            surface: "#E8E8F0".into(),
            border: "#7e8a9e".into(),

            red: "#E35043".into(),
            green: "#51A150".into(),
            blue: "#3D76F3".into(),
            cyan: "#00BFCF".into(),
            magenta: "#AB31A9".into(),
            yellow: "#FFCC00".into(),
            white: "#FFFFFF".into(),
            black: "#000000".into(),
        }
    }

    pub fn monokai() -> Self {
        CustomTheme {
            keyword: "#F92672".into(),
            keyword_bg: "#2D1A1F".into(),
            function: "#A6E22E".into(),
            string: "#E6DB74".into(),
            module: "#66D9EF".into(),
            constant: "#AE81FF".into(),
            comment: "#75715E".into(),
            foreground: "#F8F8F2".into(),
            guide: "#3E3D32".into(),
            background: "#272822".into(),
            surface: "#343429".into(),
            border: "#49483E".into(),

            red: "#F92672".into(),
            green: "#A6E22E".into(),
            blue: "#66D9EF".into(),
            cyan: "#66D9EF".into(),
            magenta: "#AE81FF".into(),
            yellow: "#E6DB74".into(),
            white: "#F8F8F2".into(),
            black: "#272822".into(),
        }
    }

    pub fn catppuccin() -> Self {
        CustomTheme {
            keyword: "#CBA6F7".into(),
            keyword_bg: "#2A1F33".into(),
            function: "#89B4FA".into(),
            string: "#A6E3A1".into(),
            module: "#89DCEB".into(),
            constant: "#F38BA8".into(),
            comment: "#7F849C".into(),
            foreground: "#CDD6F4".into(),
            guide: "#45475A".into(),
            background: "#1E1E2E".into(),
            surface: "#2A2A3A".into(),
            border: "#45475A".into(),

            red: "#F38BA8".into(),
            green: "#A6E3A1".into(),
            blue: "#89B4FA".into(),
            cyan: "#89DCEB".into(),
            magenta: "#CBA6F7".into(),
            yellow: "#F9E2AF".into(),
            white: "#CDD6F4".into(),
            black: "#1E1E2E".into(),
        }
    }

    pub fn tokyo_night() -> Self {
        CustomTheme {
            keyword: "#BB9AF7".into(),
            keyword_bg: "#261F2D".into(),
            function: "#7AA2F7".into(),
            string: "#9ECE6A".into(),
            module: "#2AC3DE".into(),
            constant: "#FF9E64".into(),
            comment: "#565F89".into(),
            foreground: "#C0CAF5".into(),
            guide: "#3B4261".into(),
            background: "#1A1B26".into(),
            surface: "#24283B".into(),
            border: "#414868".into(),

            red: "#F7768E".into(),
            green: "#9ECE6A".into(),
            blue: "#7AA2F7".into(),
            cyan: "#2AC3DE".into(),
            magenta: "#BB9AF7".into(),
            yellow: "#E0AF68".into(),
            white: "#C0CAF5".into(),
            black: "#1A1B26".into(),
        }
    }

    pub fn dracula() -> Self {
        CustomTheme {
            keyword: "#FF79C6".into(),
            keyword_bg: "#2D1B26".into(),
            function: "#50FA7B".into(),
            string: "#F1FA8C".into(),
            module: "#8BE9FD".into(),
            constant: "#BD93F9".into(),
            comment: "#6272A4".into(),
            foreground: "#F8F8F2".into(),
            guide: "#44475A".into(),
            background: "#282A36".into(),
            surface: "#353746".into(),
            border: "#44475A".into(),

            red: "#FF5555".into(),
            green: "#50FA7B".into(),
            blue: "#8BE9FD".into(),
            cyan: "#8BE9FD".into(),
            magenta: "#FF79C6".into(),
            yellow: "#F1FA8C".into(),
            white: "#F8F8F2".into(),
            black: "#282A36".into(),
        }
    }

    pub fn nord() -> Self {
        CustomTheme {
            keyword: "#81A1C1".into(),
            keyword_bg: "#1C2329".into(),
            function: "#88C0D0".into(),
            string: "#A3BE8C".into(),
            module: "#8FBCBB".into(),
            constant: "#B48EAD".into(),
            comment: "#616E88".into(),
            foreground: "#D8DEE9".into(),
            guide: "#434C5E".into(),
            background: "#272E37".into(),
            surface: "#323A47".into(),
            border: "#434C5E".into(),

            red: "#BF616A".into(),
            green: "#A3BE8C".into(),
            blue: "#81A1C1".into(),
            cyan: "#88C0D0".into(),
            magenta: "#B48EAD".into(),
            yellow: "#EBCB8B".into(),
            white: "#D8DEE9".into(),
            black: "#2E3440".into(),
        }
    }

    pub fn gruvbox() -> Self {
        CustomTheme {
            keyword: "#FB4934".into(),
            keyword_bg: "#2B1A18".into(),
            function: "#FABD2F".into(),
            string: "#B8BB26".into(),
            module: "#83A598".into(),
            constant: "#D3869B".into(),
            comment: "#928374".into(),
            foreground: "#EBDBB2".into(),
            guide: "#504945".into(),
            background: "#282828".into(),
            surface: "#3C3836".into(),
            border: "#665C54".into(),

            red: "#FB4934".into(),
            green: "#B8BB26".into(),
            blue: "#83A598".into(),
            cyan: "#8EC07C".into(),
            magenta: "#D3869B".into(),
            yellow: "#FABD2F".into(),
            white: "#EBDBB2".into(),
            black: "#282828".into(),
        }
    }

    pub fn solarized() -> Self {
        CustomTheme {
            keyword: "#268BD2".into(),
            keyword_bg: "#0A2935".into(),
            function: "#B58900".into(),
            string: "#2AA198".into(),
            module: "#859900".into(),
            constant: "#D33682".into(),
            comment: "#586E75".into(),
            foreground: "#839496".into(),
            guide: "#073642".into(),
            background: "#002B36".into(),
            surface: "#0E3A47".into(),
            border: "#586E75".into(),

            red: "#DC322F".into(),
            green: "#859900".into(),
            blue: "#268BD2".into(),
            cyan: "#2AA198".into(),
            magenta: "#D33682".into(),
            yellow: "#B58900".into(),
            white: "#EEE8D5".into(),
            black: "#002B36".into(),
        }
    }

    pub fn one_dark() -> Self {
        CustomTheme {
            keyword: "#C678DD".into(),
            keyword_bg: "#2A1F2D".into(),
            function: "#61AFEF".into(),
            string: "#98C379".into(),
            module: "#56B6C2".into(),
            constant: "#E06C75".into(),
            comment: "#5C6370".into(),
            foreground: "#ABB2BF".into(),
            guide: "#3E4451".into(),
            background: "#282C34".into(),
            surface: "#353B45".into(),
            border: "#3E4451".into(),

            red: "#E06C75".into(),
            green: "#98C379".into(),
            blue: "#61AFEF".into(),
            cyan: "#56B6C2".into(),
            magenta: "#C678DD".into(),
            yellow: "#E5C07B".into(),
            white: "#ABB2BF".into(),
            black: "#282C34".into(),
        }
    }

    pub fn github() -> Self {
        CustomTheme {
            keyword: "#FF7B72".into(),
            keyword_bg: "#2B1618".into(),
            function: "#D2A8FF".into(),
            string: "#A5D6FF".into(),
            module: "#FFA657".into(),
            constant: "#79C0FF".into(),
            comment: "#8B949E".into(),
            foreground: "#F0F6FC".into(),
            guide: "#30363D".into(),
            background: "#0D1117".into(),
            surface: "#1C2128".into(),
            border: "#30363D".into(),

            red: "#F85149".into(),
            green: "#56D364".into(),
            blue: "#58A6FF".into(),
            cyan: "#39D0D6".into(),
            magenta: "#BC8CFF".into(),
            yellow: "#E3B341".into(),
            white: "#F0F6FC".into(),
            black: "#0D1117".into(),
        }
    }

    pub fn material() -> Self {
        CustomTheme {
            keyword: "#C792EA".into(),
            keyword_bg: "#2F2A37".into(),
            function: "#82AAFF".into(),
            string: "#C3E88D".into(),
            module: "#FFCB6B".into(),
            constant: "#F78C6C".into(),
            comment: "#676E95".into(),
            foreground: "#A6ACCD".into(),
            guide: "#4E5579".into(),
            background: "#292D3E".into(),
            surface: "#32374D".into(),
            border: "#444267".into(),
            red: "#F07178".into(),
            green: "#C3E88D".into(),
            blue: "#82AAFF".into(),
            cyan: "#89DDFF".into(),
            magenta: "#C792EA".into(),
            yellow: "#FFCB6B".into(),
            white: "#FFFFFF".into(),
            black: "#292D3E".into(),
        }
    }

    pub fn ayu() -> Self {
        CustomTheme {
            keyword: "#FF8F40".into(),
            keyword_bg: "#1A1209".into(),
            function: "#FFB454".into(),
            string: "#AAD94C".into(),
            module: "#59C2FF".into(),
            constant: "#D2A6FF".into(),
            comment: "#ACB6BF8C".into(),
            foreground: "#BFBDB6".into(),
            guide: "#1F2430".into(),
            background: "#0A0E14".into(),
            surface: "#151A21".into(),
            border: "#1F2430".into(),

            red: "#F28779".into(),
            green: "#AAD94C".into(),
            blue: "#59C2FF".into(),
            cyan: "#95E6CB".into(),
            magenta: "#D2A6FF".into(),
            yellow: "#FFB454".into(),
            white: "#BFBDB6".into(),
            black: "#0A0E14".into(),
        }
    }

    pub fn ayu_mirage() -> Self {
        CustomTheme {
            keyword: "#FFA759".into(),
            keyword_bg: "#221A0D".into(),
            function: "#FFD580".into(),
            string: "#BAE67E".into(),
            module: "#73D0FF".into(),
            constant: "#D4BFFF".into(),
            comment: "#5C6773".into(),
            foreground: "#CBCCC6".into(),
            guide: "#242936".into(),
            background: "#1F2430".into(),
            surface: "#2A313F".into(),
            border: "#343B4C".into(),

            red: "#FF6666".into(),
            green: "#BAE67E".into(),
            blue: "#73D0FF".into(),
            cyan: "#95E6CB".into(),
            magenta: "#D4BFFF".into(),
            yellow: "#FFD580".into(),
            white: "#CBCCC6".into(),
            black: "#1F2430".into(),
        }
    }

    pub fn synthwave() -> Self {
        CustomTheme {
            keyword: "#FF7EDB".into(),
            keyword_bg: "#2B1929".into(),
            function: "#36F9F6".into(),
            string: "#E6DB74".into(),
            module: "#FE4450".into(),
            constant: "#FF8CC8".into(),
            comment: "#848077".into(),
            foreground: "#F8F8F2".into(),
            guide: "#2A2139".into(),
            background: "#262335".into(),
            surface: "#342949".into(),
            border: "#495495".into(),

            red: "#FE4450".into(),
            green: "#72F1B8".into(),
            blue: "#36F9F6".into(),
            cyan: "#36F9F6".into(),
            magenta: "#FF7EDB".into(),
            yellow: "#FEE715".into(),
            white: "#F8F8F2".into(),
            black: "#262335".into(),
        }
    }

    pub fn rose_pine() -> Self {
        CustomTheme {
            keyword: "#C4A7E7".into(),
            keyword_bg: "#24202E".into(),
            function: "#9CCFD8".into(),
            string: "#F6C177".into(),
            module: "#EBBCBA".into(),
            constant: "#EB6F92".into(),
            comment: "#6E6A86".into(),
            foreground: "#E0DEF4".into(),
            guide: "#26233A".into(),
            background: "#191724".into(),
            surface: "#21202E".into(),
            border: "#403D52".into(),

            red: "#EB6F92".into(),
            green: "#31748F".into(),
            blue: "#9CCFD8".into(),
            cyan: "#9CCFD8".into(),
            magenta: "#C4A7E7".into(),
            yellow: "#F6C177".into(),
            white: "#E0DEF4".into(),
            black: "#191724".into(),
        }
    }

    pub fn kanagawa() -> Self {
        CustomTheme {
            keyword: "#957FB8".into(),
            keyword_bg: "#1E1A22".into(),
            function: "#7AA89F".into(),
            string: "#98BB6C".into(),
            module: "#7FB4CA".into(),
            constant: "#D27E99".into(),
            comment: "#727169".into(),
            foreground: "#DCD7BA".into(),
            guide: "#2A2A37".into(),
            background: "#1F1F28".into(),
            surface: "#2A2A37".into(),
            border: "#54546D".into(),

            red: "#C34043".into(),
            green: "#76946A".into(),
            blue: "#7E9CD8".into(),
            cyan: "#6A9589".into(),
            magenta: "#938AA9".into(),
            yellow: "#C0A36E".into(),
            white: "#DCD7BA".into(),
            black: "#1F1F28".into(),
        }
    }

    pub fn everforest() -> Self {
        CustomTheme {
            keyword: "#E67E80".into(),
            keyword_bg: "#2B1F20".into(),
            function: "#A7C080".into(),
            string: "#DBBC7F".into(),
            module: "#7FBBB3".into(),
            constant: "#D699B6".into(),
            comment: "#7A8478".into(),
            foreground: "#D3C6AA".into(),
            guide: "#3D484D".into(),
            background: "#2D353B".into(),
            surface: "#384148".into(),
            border: "#504945".into(),

            red: "#E67E80".into(),
            green: "#A7C080".into(),
            blue: "#7FBBB3".into(),
            cyan: "#83C092".into(),
            magenta: "#D699B6".into(),
            yellow: "#DBBC7F".into(),
            white: "#D3C6AA".into(),
            black: "#2D353B".into(),
        }
    }

    pub fn vscode() -> Self {
        CustomTheme {
            keyword: "#569CD6".into(),
            keyword_bg: "#142129".into(),
            function: "#DCDCAA".into(),
            string: "#CE9178".into(),
            module: "#4EC9B0".into(),
            constant: "#B5CEA8".into(),
            comment: "#6A9955".into(),
            foreground: "#D4D4D4".into(),
            guide: "#404040".into(),
            background: "#1E1E1E".into(),
            surface: "#2D2D30".into(),
            border: "#3E3E42".into(),

            red: "#F44747".into(),
            green: "#6A9955".into(),
            blue: "#569CD6".into(),
            cyan: "#4EC9B0".into(),
            magenta: "#C586C0".into(),
            yellow: "#DCDCAA".into(),
            white: "#D4D4D4".into(),
            black: "#1E1E1E".into(),
        }
    }

    pub fn to_syntect_theme(&self) -> Theme {
        let mut settings = ThemeSettings::default();
        settings.foreground = Some(self.foreground.color);
        settings.background = Some(self.surface.color);
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
            scope: create_selectors(
                "module, struct, enum, generic, path, meta.path, entity.name.tag, support.type, meta.import-name",
            ),
            style: create_style(self.module.color),
        });

        theme.scopes.push(syntect::highlighting::ThemeItem {
            scope: create_selectors(
                "string, punctuation.string, constant.other.color, punctuation.definition.string",
            ),
            style: create_style(self.string.color),
        });

        theme.scopes.push(syntect::highlighting::ThemeItem {
            scope: create_selectors("constant, keyword.other.unit, support.constant"),
            style: create_style(self.constant.color),
        });

        theme.scopes.push(syntect::highlighting::ThemeItem {
            scope: create_selectors("comment, punctuation.comment, punctuation.definition.comment"),
            style: create_style(self.comment.color),
        });

        theme.scopes.push(syntect::highlighting::ThemeItem {
            scope: create_selectors(
                "variable, operator, punctuation, block, support.type.property-name, punctuation.definition, keyword.operator",
            ),
            style: create_style(self.foreground.color),
        });

        theme
    }

    pub fn to_html_style(&self) -> String {
        let root_css = format!(
            r#"
:root {{
  --keyword: {};
  --function: {};
  --type: {};
  --constant: {};
  --comment: {};
  --foreground: {};
  
  /* UI Colors */
  --background: {};
  --surface: {};
  --border: {};
}}
"#,
            self.keyword.value,
            self.function.value,
            self.module.value,
            self.constant.value,
            self.comment.value,
            self.foreground.value,
            self.background.value,
            self.surface.value,
            self.border.value
        );
        let full_css = include_str!("../assets/style.css");
        format!("{full_css}\n\n{root_css}")
    }
}
