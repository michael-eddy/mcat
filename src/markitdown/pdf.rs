use std::path::Path;

pub fn pdf_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let doc = lopdf::Document::load(path)?;
    let mut result = String::new();

    let num_pages = doc.get_pages().len();
    for i in 1..=num_pages {
        let page_text = doc.extract_text(&[i as u32])?.replace("  ", " ");

        let mut output = String::with_capacity(page_text.len());
        for line in page_text.lines() {
            output.push_str(line.trim());
            output.push('\n');
        }
        let page_text = output;
        let page_text = page_text.replace("\n\n\n", "\0");
        let page_text = page_text.replace("\n\n", " ");
        let page_text = page_text.replace("\n", " ");
        let page_text = page_text.replace("\0", "\n\n\n");

        result.push_str(&format!("## Page {}\n\n", i));

        result.push_str(&page_text);
        result.push_str("\n\n");
    }

    Ok(result)
}
