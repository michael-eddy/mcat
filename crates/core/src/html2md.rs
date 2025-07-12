use regex::Regex;
use scraper::{ElementRef, Html};
use std::collections::HashMap;

pub struct MarkdownHtmlPreprocessor {
    rules: HashMap<String, Box<dyn Fn(ElementRef, &MarkdownHtmlPreprocessor) -> String>>,
}

impl MarkdownHtmlPreprocessor {
    pub fn new() -> Self {
        let mut processor = Self {
            rules: HashMap::new(),
        };

        processor.add_div_rules();
        processor.add_details_rules();
        processor.add_quote_rules();
        processor.add_heading_rules();
        processor.add_formatting_rules();
        processor.add_link_rules();
        processor.add_img_rules();
        processor.add_code_rules();
        processor.add_block_rules();

        processor
    }

    fn add_img_rules(&mut self) {
        // img - images
        self.rules.insert(
            "img".to_string(),
            Box::new(|element, _processor| {
                let src = element.value().attr("src").unwrap_or("");
                let alt = element.value().attr("alt").unwrap_or("");
                let width = element.value().attr("width");
                let height = element.value().attr("height");

                // Build the src with dimensions if available
                let enhanced_src = match (width, height) {
                    (Some(w), Some(h)) => format!("{}#{}x{}", src, w, h),
                    (Some(w), None) => format!("{}#{}x", src, w),
                    (None, Some(h)) => format!("{}#x{}", src, h),
                    (None, None) => src.to_string(),
                };

                let mut result = format!("![{}]({})", alt, enhanced_src);

                if let Some(align) = element.value().attr("align") {
                    if align.trim().to_lowercase() == "center" {
                        result = format!("<center>{}</center>", result);
                    }
                }

                result
            }),
        );
    }

    fn add_code_rules(&mut self) {
        // pre - preformatted text
        self.rules.insert(
            "pre".to_string(),
            Box::new(|element, processor| {
                let content = processor.process_children(element);
                format!("```\n{}\n```\n\n", content)
            }),
        );

        // code - inline code
        self.rules.insert(
            "code".to_string(),
            Box::new(|element, processor| {
                let content = processor.process_children(element);
                format!("`{}`", content)
            }),
        );
    }

    fn add_link_rules(&mut self) {
        // a - links
        self.rules.insert(
            "a".to_string(),
            Box::new(|element, processor| {
                let content = processor.process_children(element);
                if let Some(href) = element.value().attr("href") {
                    format!("[{}]({})", content, href)
                } else {
                    content
                }
            }),
        );
    }

    fn add_block_rules(&mut self) {
        // blockquote
        self.rules.insert(
            "blockquote".to_string(),
            Box::new(|element, processor| {
                let content = processor.process_children(element);
                let mut result = String::new();

                for line in content.lines() {
                    result.push_str(&format!("> {}\n", line));
                }

                result
            }),
        );
    }

    fn add_formatting_rules(&mut self) {
        // br - line break
        self.rules.insert(
            "br".to_string(),
            Box::new(|_element, _processor| "\n".to_string()),
        );

        // var - italic
        self.rules.insert(
            "var".to_string(),
            Box::new(|element, processor| {
                let var_content = processor.process_children(element);
                format!("*{}*", var_content)
            }),
        );

        // hr - horizontal rule
        self.rules.insert(
            "hr".to_string(),
            Box::new(|_element, _processor| "---".to_string()),
        );

        // b - bold
        self.rules.insert(
            "b".to_string(),
            Box::new(|element, processor| {
                let content = processor.process_children(element);
                format!("**{}**", content)
            }),
        );

        // strong - bold
        self.rules.insert(
            "strong".to_string(),
            Box::new(|element, processor| {
                let content = processor.process_children(element);
                format!("**{}**", content)
            }),
        );

        // em - italic
        self.rules.insert(
            "em".to_string(),
            Box::new(|element, processor| {
                let content = processor.process_children(element);
                format!("*{}*", content)
            }),
        );

        // del - strike
        self.rules.insert(
            "del".to_string(),
            Box::new(|element, processor| {
                let del_content = processor.process_children(element);
                format!("~~{}~~", del_content)
            }),
        );

        // s - strikethrough
        self.rules.insert(
            "s".to_string(),
            Box::new(|element, processor| {
                let content = processor.process_children(element);
                format!("~~{}~~", content)
            }),
        );

        // strike - strikethrough
        self.rules.insert(
            "strike".to_string(),
            Box::new(|element, processor| {
                let content = processor.process_children(element);
                format!("~~{}~~", content)
            }),
        );
    }

    fn add_heading_rules(&mut self) {
        for level in 1..=6 {
            let tag = format!("h{}", level);
            let prefix = "#".repeat(level);

            self.rules.insert(
                tag,
                Box::new(move |element, processor| {
                    let content = processor.process_children(element);
                    let mut result = format!("{} {}", prefix, content.trim());

                    // Check for align=center
                    if let Some(align) = element.value().attr("align") {
                        if align.trim().to_lowercase() == "center" {
                            result = format!("<center>{}</center>", result);
                        }
                    }

                    result
                }),
            );
        }
    }

    fn add_quote_rules(&mut self) {
        self.rules.insert(
            "q".to_string(),
            Box::new(|element, processor| {
                let quote_content = processor.process_children(element);
                let result = format!("\"{}\"", quote_content);

                // Check for align=center
                if let Some(align) = element.value().attr("align") {
                    if align.trim().to_lowercase() == "center" {
                        return format!("<center>{}</center>", result);
                    }
                }

                result
            }),
        );
    }

    fn add_div_rules(&mut self) {
        for item in ["div", "p"] {
            self.rules.insert(
                item.to_string(),
                Box::new(|element, processor| {
                    let inner_content = processor.process_children(element);

                    if let Some(align) = element.value().attr("align") {
                        if align.trim().to_lowercase() == "center" {
                            return format!("<center>{}</center>", inner_content);
                        }
                    }
                    inner_content
                }),
            );
        }
    }

    fn add_details_rules(&mut self) {
        self.rules.insert(
            "details".to_string(),
            Box::new(|element, processor| {
                let content = processor.process_children(element);
                let mut result = String::new();

                // Add content as blockquote, skip leading empty lines
                let mut start = true;
                for line in content.lines() {
                    if start && line.trim().is_empty() {
                        continue;
                    } else {
                        start = false;
                    }
                    result.push_str(&format!("> {}\n", line));
                }

                result
            }),
        );

        self.rules.insert(
            "summary".to_string(),
            Box::new(|element, processor| {
                let summary_content = processor.process_children(element);
                format!("▼ {}", summary_content.trim())
            }),
        );
    }

    fn escape_unknown_elements(&self, markdown: &str) -> String {
        // regex to match HTML tags
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

    pub fn process(&self, markdown: &str) -> String {
        let escaped_markdown = self.escape_unknown_elements(markdown);

        let document = Html::parse_fragment(&escaped_markdown);
        let result = self.process_children(document.root_element());

        self.unescape_text(&result)
    }

    fn process_children(&self, element: ElementRef) -> String {
        let mut result = String::new();
        for child in element.children() {
            match child.value() {
                scraper::node::Node::Element(_) => {
                    let child_element = ElementRef::wrap(child).unwrap();
                    let tag_name = child_element.value().name();

                    if let Some(rule) = self.rules.get(tag_name) {
                        result.push_str(&rule(child_element, self));
                    } else {
                        // For unknown elements, process their children
                        result.push_str(&self.process_children(child_element));
                    }
                }
                scraper::node::Node::Text(text) => {
                    result.push_str(text.text.as_ref());
                }
                _ => {}
            }
        }
        result
    }
}

impl Default for MarkdownHtmlPreprocessor {
    fn default() -> Self {
        Self::new()
    }
}
