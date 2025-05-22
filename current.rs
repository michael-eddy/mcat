use std::{
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
};

use comrak::{
    Arena, ComrakOptions, ComrakPlugins, markdown_to_html_with_plugins,
    nodes::{AstNode, NodeValue},
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

pub fn md_to_ansi(md: &str) -> String {
    let arena = Arena::new();
    let opts = comrak_options();
    let root = comrak::parse_document(&arena, md, &opts);
    eprintln!("{:?} ", root);
    let mut result = String::new();

    let ps = SyntaxSet::load_defaults_newlines();
    let theme = CustomTheme::dark();
    format_ast_node(
        root,
        &mut result,
        &ps,
        &theme.to_syntect_theme(),
        &theme,
        &AtomicUsize::new(1),
    );

    result
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
    options.extension.footnotes = true;
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

pub fn format_ast_node<'a>(
    node: &'a AstNode<'a>,
    output: &mut String,
    ps: &SyntaxSet,
    ts: &Theme,
    theme: &CustomTheme,
    line: &AtomicUsize,
) {
    let data = node.data.borrow();
    let sps = data.sourcepos;
    let current_line = line.load(Ordering::SeqCst);

    if current_line < sps.start.line {
        let offset = sps.start.line - current_line;
        if offset != 0 {
            line.store(sps.start.line, Ordering::SeqCst);
            output.push_str(&"\n".repeat(offset));
        }
    }

    match &data.value {
        NodeValue::Document => {
            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }
            return;
        }
        NodeValue::FrontMatter(content) => {
            // I dont know what that is..
            let surface = theme.surface.bg.clone();
            output.push_str(&format!(
                "{surface}--- Front Matter ---\n{}\n---{RESET}\n\n",
                content
            ));
        }
        NodeValue::BlockQuote => {
            for child in node.children() {
                let mut block_content = String::new();
                format_ast_node(child, &mut block_content, ps, ts, theme, line);

                // Add blockquote formatting
                for line in block_content.lines() {
                    output.push_str(&format!("{FG_BLUE}│ {RESET}{}\n", line));
                }
            }
            output.push('\n');
        }
        NodeValue::List(node_list) => {
            //CHECKED
            let list_type = &node_list.list_type;

            let mut index = match list_type {
                comrak::nodes::ListType::Bullet => 0,
                comrak::nodes::ListType::Ordered => node_list.start,
            };

            for child in node.children() {
                let mut item_content = String::new();
                format_ast_node(child, &mut item_content, ps, ts, theme, line);

                // Create bullet or number
                let prefix = if node_list.is_task_list {
                    "  ".to_owned()
                } else {
                    match list_type {
                        comrak::nodes::ListType::Bullet => format!("{FG_YELLOW}•{RESET} "),
                        comrak::nodes::ListType::Ordered => {
                            let result = format!("{FG_YELLOW}{}.{RESET} ", index);
                            index += 1;
                            result
                        }
                    }
                };

                let lines: Vec<&str> = item_content.lines().collect();

                if let Some(first_line) = lines.first() {
                    output.push_str(&format!("{}{}", prefix, first_line));

                    let continuation_indent = if node_list.is_task_list {
                        "  ".to_owned()
                    } else {
                        " ".repeat(prefix.chars().count() - FG_YELLOW.len() - RESET.len())
                    };
                    for line in &lines[1..] {
                        output.push_str(&format!("{}{}\n", continuation_indent, line));
                    }
                }
            }
        }
        NodeValue::Item(_) => {
            // Process children directly - the List handler manages item formatting
            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }
        }
        NodeValue::DescriptionList => {
            for child in node.children() {
                format_ast_node(child, &mut *output, ps, ts, theme, line);
            }
        }
        NodeValue::DescriptionItem(_) => {
            let mut has_term = false;
            let mut term_content = String::new();
            let mut details_content = String::new();

            for child in node.children() {
                match &child.data.borrow().value {
                    NodeValue::DescriptionTerm => {
                        has_term = true;
                        format_ast_node(child, &mut term_content, ps, ts, theme, line);
                    }
                    NodeValue::DescriptionDetails => {
                        format_ast_node(child, &mut details_content, ps, ts, theme, line);
                    }
                    _ => {}
                }
            }

            if has_term {
                output.push_str(&format!("{BOLD}{}{RESET}\n", term_content));
                output.push_str(&details_content);
            }
        }
        NodeValue::DescriptionTerm => {
            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }
        }
        NodeValue::DescriptionDetails => {
            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }
        }
        NodeValue::CodeBlock(node_code_block) => {
            //CHECKED
            let code = &node_code_block.literal;
            let lang = &node_code_block.info;
            let lang = if lang.is_empty() {
                &"txt".to_string()
            } else {
                lang
            };

            let br = br();
            let surface = theme.surface.bg.clone();
            output.push_str(&format!(
                "{surface}{FG_WHITE} {} {RESET}\n{FG_BRIGHT_BLACK}{br}{RESET}\n",
                lang
            ));

            let syntax = ps
                .find_syntax_by_token(lang)
                .unwrap_or_else(|| ps.find_syntax_plain_text());
            let mut highlighter = HighlightLines::new(syntax, &ts);

            for line in LinesWithEndings::from(code) {
                let ranges: Vec<(Style, &str)> = highlighter.highlight_line(line, &ps).unwrap();
                let highlighted = as_24_bit_terminal_escaped(&ranges[..], false);
                output.push_str(&format!("{}", highlighted));
            }
            output.push_str(&format!("{FG_BRIGHT_BLACK}{br}{RESET}\n\n"));
        }
        NodeValue::HtmlBlock(node_html_block) => {
            output.push_str(&format!("{FG_MAGENTA}<HTML>{RESET}\n",));
            for line in node_html_block.literal.lines() {
                output.push_str(&format!("{FG_MAGENTA}{}{RESET}\n", line));
            }
            output.push_str(&format!("{FG_MAGENTA}</HTML>{RESET}\n\n",));
        }
        NodeValue::Paragraph => {
            let mut paragraph = String::new();
            for child in node.children() {
                format_ast_node(child, &mut paragraph, ps, ts, theme, line);
            }

            output.push_str(&paragraph);
            output.push('\n');
        }
        NodeValue::Heading(node_heading) => {
            //CHECKED
            let mut heading_content = String::new();
            let prefix = match node_heading.level {
                1 => "㊀",
                2 => "㊁",
                3 => "㊂",
                4 => "㊃",
                5 => "㊄",
                6 => "㊅",
                _ => "",
            };

            for child in node.children() {
                format_ast_node(child, &mut heading_content, ps, ts, theme, line);
            }

            output.push_str(&format!(
                "{BOLD}{FG_BRIGHT_MAGENTA}{prefix} {}{RESET}\n",
                heading_content
            ))
        }
        NodeValue::ThematicBreak => {
            //CHECKED
            let br = br();
            output.push_str(&format!("{FG_BRIGHT_BLACK}{br}{RESET}\n\n",));
        }
        NodeValue::FootnoteDefinition(node_footnote_definition) => {
            //CHECKED
            let label = &node_footnote_definition.name;
            let mut content = String::new();

            for child in node.children() {
                format_ast_node(child, &mut content, ps, ts, theme, line);
            }

            output.push_str(&format!(
                "{FG_BRIGHT_BLACK}^{}{FG_BRIGHT_BLACK}: {RESET}{}",
                label, content
            ));
        }
        NodeValue::Table(table) => {
            //CHECKED
            let alignments = &table.alignments;
            let mut rows: Vec<Vec<String>> = Vec::new();

            // Process all rows
            for child in node.children() {
                let mut row_cells: Vec<String> = Vec::new();

                // Process cells in this row
                for cell_node in child.children() {
                    let mut cell_content = String::new();
                    format_ast_node(cell_node, &mut cell_content, ps, ts, theme, line);
                    row_cells.push(cell_content.to_string());
                }

                rows.push(row_cells);
            }

            // Find the maximum width for each column
            let mut column_widths: Vec<usize> = vec![0; alignments.len()];
            for row in &rows {
                for (i, cell) in row.iter().enumerate() {
                    if i < column_widths.len() && cell.len() > column_widths[i] {
                        column_widths[i] = cell.len();
                    }
                }
            }

            let color = FG_BLUE;
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
                    line.push('\n');
                    line
                };

                let top_border = build_line("╭", "┬", "╮", "─");
                let middle_border = build_line("├", "┼", "┤", "─");
                let bottom_border = build_line("╰", "┴", "╯", "─");
                output.push_str(&top_border);

                for (i, row) in rows.iter().enumerate() {
                    // Print the row content
                    output.push_str(&format!("{color}│{RESET}"));
                    for (j, cell) in row.iter().enumerate() {
                        let width = column_widths[j];
                        let padding = width - cell.len();
                        let (left_pad, right_pad) = match alignments[j] {
                            comrak::nodes::TableAlignment::Center => {
                                (padding / 2, padding - (padding / 2))
                            }
                            comrak::nodes::TableAlignment::Right => (padding, 0),
                            _ => (0, padding),
                        };
                        output.push_str(&format!(
                            " {}{}{} {color}│{RESET}",
                            " ".repeat(left_pad),
                            cell,
                            " ".repeat(right_pad)
                        ));
                    }
                    output.push('\n');

                    if i == 0 {
                        output.push_str(&middle_border);
                    }
                }
                output.push_str(&bottom_border);
            }
        }
        NodeValue::TableRow(_) => {
            // Handled by the Table node
        }
        NodeValue::TableCell => {
            // Process children directly
            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }
        }
        NodeValue::Text(text) => {
            output.push_str(text);
        }
        NodeValue::TaskItem(task) => {
            //CHECKED
            let checked = task.unwrap_or_default() == 'x';
            let checkbox = if checked {
                format!("{FG_GREEN}[x]{RESET} ")
            } else {
                format!("{FG_RED}[ ]{RESET} ")
            };

            // Format the task item content
            let mut content = String::new();
            for child in node.children() {
                format_ast_node(child, &mut content, ps, ts, theme, line);
            }

            output.push_str(&format!("{}{}", checkbox, content));
        }
        NodeValue::SoftBreak => {
            output.push(' ');
        }
        NodeValue::LineBreak => {
            output.push_str("\n");
        }
        NodeValue::Code(node_code) => {
            let surface = theme.surface.bg.clone();
            output.push_str(&format!(
                "{surface}{FG_GREEN} {} {RESET}",
                node_code.literal
            ));
        }
        NodeValue::HtmlInline(html) => {
            output.push_str(&format!("{FG_MAGENTA}{}{RESET}", html));
        }
        NodeValue::Raw(raw_text) => {
            output.push_str(raw_text);
        }
        NodeValue::Emph => {
            output.push_str(ITALIC);
            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }
            output.push_str(RESET);
        }
        NodeValue::Strong => {
            output.push_str(BOLD);
            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }
            output.push_str(RESET);
        }
        NodeValue::Strikethrough => {
            output.push_str(STRIKETHROUGH);
            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }
            output.push_str(RESET);
        }
        NodeValue::Superscript => {
            output.push_str(&format!("{FG_BRIGHT_CYAN}^{RESET}"));
            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }
        }
        NodeValue::Link(node_link) => {
            let mut text = String::new();
            for child in node.children() {
                format_ast_node(child, &mut text, ps, ts, theme, line);
            }

            output.push_str(&format!(
                "{UNDERLINE}{FG_BLUE}{}{RESET} {FG_BRIGHT_BLACK}({}){RESET}",
                text, node_link.url
            ));
        }
        NodeValue::Image(node_link) => {
            let mut alt_text = String::new();
            for child in node.children() {
                format_ast_node(child, &mut alt_text, ps, ts, theme, line);
            }

            output.push_str(&format!(
                "{FG_BRIGHT_MAGENTA}[Image: {}{FG_BRIGHT_MAGENTA}]{RESET} {FG_BRIGHT_BLACK}({}){RESET}",
                alt_text, node_link.url
            ));
        }
        NodeValue::FootnoteReference(node_footnote_reference) => {
            output.push_str(&format!(
                "{FG_BRIGHT_BLACK}[^{}]{RESET}",
                node_footnote_reference.name
            ));
        }
        NodeValue::Math(node_math) => {
            if node_math.display_math {
                output.push_str(&format!(
                    "{FG_BRIGHT_YELLOW}${}{FG_BRIGHT_YELLOW}${RESET}",
                    node_math.literal
                ));
            } else {
                output.push_str(&format!("\n{FG_BRIGHT_YELLOW}$${RESET}\n{FG_BRIGHT_YELLOW}{}{RESET}\n{FG_BRIGHT_YELLOW}$${RESET}\n", 
                    node_math.literal));
            }
        }
        NodeValue::MultilineBlockQuote(multiline_quote) => {
            output.push_str(&format!("{FG_BLUE}❝{RESET} "));

            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }

            output.push_str(&format!(" {FG_BLUE}❞{RESET}"));
        }
        NodeValue::Escaped => {
            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }
        }
        NodeValue::WikiLink(node_wiki_link) => {
            output.push_str(&format!(
                "{FG_BRIGHT_GREEN}[[{}{FG_BRIGHT_GREEN}]]{RESET}",
                node_wiki_link.url
            ));
        }
        NodeValue::Underline => {
            output.push_str(UNDERLINE);
            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }
            output.push_str(RESET);
        }
        NodeValue::Subscript => {
            output.push_str(&format!("{FG_BRIGHT_CYAN}_{RESET}"));
            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }
        }
        NodeValue::SpoileredText => {
            output.push_str(&format!("{BG_BLACK}{FG_BLACK}"));
            for child in node.children() {
                format_ast_node(child, output, ps, ts, theme, line);
            }
            output.push_str(RESET);
        }
        NodeValue::EscapedTag(tag) => {
            output.push_str(&format!("{FG_BRIGHT_BLACK}\\{}{RESET}", tag));
        }
        NodeValue::Alert(node_alert) => {
            let kind = &node_alert.alert_type;

            // Choose color based on alert type
            let (prefix, color) = match kind {
                comrak::nodes::AlertType::Note => ("ℹ️ NOTE", FG_BRIGHT_BLUE),
                comrak::nodes::AlertType::Tip => ("💡 TIP", FG_BRIGHT_GREEN),
                comrak::nodes::AlertType::Important => ("ℹ️ INFO", FG_BRIGHT_CYAN),
                comrak::nodes::AlertType::Warning => ("⚠️ WARNING", FG_BRIGHT_YELLOW),
                comrak::nodes::AlertType::Caution => ("🚨 DANGER", FG_BRIGHT_RED),
            };

            output.push_str(&format!("\n{}│ {BOLD}{}{RESET}\n", color, prefix));

            for child in node.children() {
                let mut alert_content = String::new();
                format_ast_node(child, &mut alert_content, ps, ts, theme, line);

                for line in alert_content.lines() {
                    output.push_str(&format!("{}│ {}\n", color, line));
                }
            }
        }
    }
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
