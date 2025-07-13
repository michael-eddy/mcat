use itertools::Itertools;
use regex::Regex;
use scraper::{ElementRef, Html};
use std::collections::HashMap;

pub struct ProcessingContext {
    output: String,
    rules: HashMap<String, Box<dyn Fn(ElementRef, &mut ProcessingContext)>>,
}

impl ProcessingContext {
    fn new() -> Self {
        let mut ctx = Self {
            output: String::new(),
            rules: HashMap::new(),
        };

        ctx.add_div_rules();
        ctx.add_details_rules();
        ctx.add_quote_rules();
        ctx.add_heading_rules();
        ctx.add_formatting_rules();
        ctx.add_link_rules();
        ctx.add_img_rules();
        ctx.add_code_rules();
        ctx.add_block_rules();

        ctx
    }

    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn collect(&mut self, element: ElementRef) -> String {
        let mut temp_output = String::new();
        let original_output = std::mem::replace(&mut self.output, temp_output);

        self.process_children(element);

        temp_output = std::mem::replace(&mut self.output, original_output);

        let start = self
            .output
            .lines()
            .last()
            .unwrap_or_default()
            .trim()
            .is_empty();
        if start {
            temp_output = temp_output.trim().into();
        }

        temp_output
    }

    fn add_img_rules(&mut self) {
        self.rules.insert(
            "img".to_string(),
            Box::new(|element, ctx| {
                let src = element.value().attr("src").unwrap_or("");
                let alt = element.value().attr("alt").unwrap_or("IMG");
                let width = element.value().attr("width");
                let height = element.value().attr("height");

                let enhanced_src = match (width, height) {
                    (Some(w), Some(h)) => format!("{}#{}x{}", src, w, h),
                    (Some(w), None) => format!("{}#{}x", src, w),
                    (None, Some(h)) => format!("{}#x{}", src, h),
                    (None, None) => src.to_string(),
                };

                ctx.write(&format!("![{}]({})", alt, enhanced_src));
            }),
        );
    }

    fn add_code_rules(&mut self) {
        self.rules.insert(
            "pre".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);
                ctx.write(&format!("```\n{}\n```\n\n", content));
            }),
        );

        self.rules.insert(
            "code".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);
                ctx.write(&format!("`{}`", content));
            }),
        );
    }

    fn add_link_rules(&mut self) {
        self.rules.insert(
            "a".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);
                if let Some(href) = element.value().attr("href") {
                    ctx.write(&format!("[{}]({})", content.trim(), href.trim()));
                } else {
                    ctx.write(&content);
                }
            }),
        );
    }

    fn add_block_rules(&mut self) {
        self.rules.insert(
            "blockquote".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);
                for line in content.lines() {
                    ctx.write(&format!("> {}\n", line));
                }
            }),
        );
    }

    fn add_formatting_rules(&mut self) {
        self.rules.insert(
            "br".to_string(),
            Box::new(|_element, _ctx| {
                // Line breaks are handled by not writing anything
            }),
        );

        self.rules.insert(
            "var".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);
                ctx.write(&format!("*{}*", content.trim()));
            }),
        );

        self.rules.insert(
            "hr".to_string(),
            Box::new(|_element, ctx| {
                if !ctx
                    .output
                    .lines()
                    .last()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                {
                    ctx.write("\n");
                }
                ctx.write("<!--HR-->");
            }),
        );

        self.rules.insert(
            "b".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);
                ctx.write(&format!("**{}**", content.trim()));
            }),
        );

        self.rules.insert(
            "strong".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);
                ctx.write(&format!("**{}**", content.trim()));
            }),
        );

        self.rules.insert(
            "em".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);
                ctx.write(&format!("*{}*", content.trim()));
            }),
        );

        self.rules.insert(
            "del".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);
                ctx.write(&format!("~~{}~~", content.trim()));
            }),
        );

        self.rules.insert(
            "s".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);
                ctx.write(&format!("~~{}~~", content.trim()));
            }),
        );

        self.rules.insert(
            "strike".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);
                ctx.write(&format!("~~{}~~", content.trim()));
            }),
        );
    }

    fn add_heading_rules(&mut self) {
        for level in 1..=6 {
            let tag = format!("h{}", level);
            let prefix = "#".repeat(level);

            self.rules.insert(
                tag,
                Box::new(move |element, ctx| {
                    let content = ctx.collect(element);
                    let mut result = format!("{} {}", prefix, content.trim());

                    if let Some(align) = element.value().attr("align") {
                        if align.trim().to_lowercase() == "center" {
                            result = format!("{result}<!--CENTER-->");
                        }
                    }

                    ctx.write(&result);
                }),
            );
        }
    }

    fn add_quote_rules(&mut self) {
        self.rules.insert(
            "q".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);
                ctx.write(&format!("\"{}\"", content));
            }),
        );
    }

    fn add_div_rules(&mut self) {
        for item in ["div", "p"] {
            self.rules.insert(
                item.to_string(),
                Box::new(|element, ctx| {
                    let mut content = ctx.collect(element);

                    if let Some(align) = element.value().attr("align") {
                        if align.trim().to_lowercase() == "center" {
                            content = content
                                .lines()
                                .map(|v| {
                                    if v.trim().is_empty() {
                                        v.into()
                                    } else {
                                        format!("{v}<!--CENTER-->")
                                    }
                                })
                                .join("\n");
                        }
                    }
                    ctx.write(&content);
                }),
            );
        }
    }

    fn add_details_rules(&mut self) {
        self.rules.insert(
            "details".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);

                for line in content.lines() {
                    ctx.write(&format!("> {}\n", line));
                }
            }),
        );

        self.rules.insert(
            "summary".to_string(),
            Box::new(|element, ctx| {
                let content = ctx.collect(element);
                ctx.write(&format!("▼ {}", content.trim()));
            }),
        );
    }

    fn escape_unknown_elements(&self, markdown: &str) -> String {
        let tag_regex = Regex::new(r"</?([a-zA-Z][a-zA-Z0-9]*)[^>]*>").unwrap();

        tag_regex
            .replace_all(markdown, |caps: &regex::Captures| {
                let tag_name = caps.get(1).unwrap().as_str().to_lowercase();

                if self.rules.contains_key(&tag_name) {
                    caps.get(0).unwrap().as_str().to_string()
                } else {
                    caps.get(0)
                        .unwrap()
                        .as_str()
                        .replace("<", "___ESCAPED_LT___")
                        .replace(">", "___ESCAPED_GT___")
                }
            })
            .to_string()
    }

    fn unescape_text(&self, text: &str) -> String {
        text.replace("___ESCAPED_LT___", "<")
            .replace("___ESCAPED_GT___", ">")
    }

    fn process_children(&mut self, element: ElementRef) {
        for child in element.children() {
            match child.value() {
                scraper::node::Node::Element(_) => {
                    let child_element = ElementRef::wrap(child).unwrap();
                    let tag_name = child_element.value().name();

                    if let Some(rule) = self.rules.get(tag_name) {
                        // We need to clone the rule to avoid borrow checker issues
                        let rule_clone = unsafe {
                            std::mem::transmute::<
                                &Box<dyn Fn(ElementRef, &mut ProcessingContext)>,
                                &Box<dyn Fn(ElementRef, &mut ProcessingContext)>,
                            >(rule)
                        };
                        rule_clone(child_element, self);
                    } else {
                        // For unknown elements, process their children
                        self.process_children(child_element);
                    }
                }
                scraper::node::Node::Text(text) => {
                    self.write(text.text.as_ref());
                }
                _ => {}
            }
        }
    }
}

// Global process function
pub fn process(markdown: &str) -> String {
    let mut ctx = ProcessingContext::new();

    let escaped_markdown = ctx.escape_unknown_elements(markdown);
    let document = Html::parse_fragment(&escaped_markdown);

    ctx.process_children(document.root_element());

    ctx.unescape_text(&ctx.output)
}
