use std::path::Path;

/// convert `pdf` into markdown
/// # usage:
/// ```
/// use std::path::Path;
/// use markdownify::pdf::pdf_convert;
///
/// let path = Path::new("path/to/file.pdf");
/// match pdf_convert(&path) {
///     Ok(md) => println!("{}", md),
///     Err(e) => eprintln!("Error: {}", e)
/// }
/// ```
pub fn pdf_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let pdf = pdfr::Pdf::new(path)?;
    let mut result = String::new();
    let mut i = 0;

    for page in pdf.iter_pages() {
        i += 1;
        eprintln!("starting page");
        result.push_str(&format!("\n# Page {}\n\n", i));
        let mut page = page?;
        let units = page.handle_stream(page.stream.clone())?;
        let elements = pdfr::Pdf::pdf_units_to_elements(units);

        for row in elements {
            for e in row {
                match e {
                    pdfr::pdf_element::PdfElement::Text(pdf_text) => {
                        result.push_str(&format!("{} ", pdf_text.text))
                    }
                    pdfr::pdf_element::PdfElement::Table(pdf_table) => {
                        result.push_str("\ntable------------>\n")
                    }
                }
            }
            result.push_str("\n");
        }
    }

    Ok(result)
}
