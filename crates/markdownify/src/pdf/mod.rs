use std::{error::Error, path::Path};

use lopdf::Document;
use pdf_element::{PdfElement, PdfText, PdfUnit};
use pdf_page::PdfPage;

use crate::sheets;

mod pdf_element;
mod pdf_page;
mod pdf_state;

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
    let pdf = Pdf::new(path)?;
    let mut result = String::new();
    let mut i = 0;

    for page in pdf.iter_pages() {
        i += 1;
        result.push_str(&format!("\n# Page {}\n\n", i));
        let mut page = page?;
        let units = page.handle_stream(page.stream.clone())?;
        let elements = Pdf::pdf_units_to_elements(units);

        for row in elements {
            for e in row {
                match e {
                    pdf_element::PdfElement::Text(pdf_text) => {
                        let text = pdftext_to_md(pdf_text);
                        result.push_str(&format!("{} ", text))
                    }
                    pdf_element::PdfElement::Table(mut pdf_table) => {
                        let elements = pdf_table.get_sorted_elements();
                        let elements: Vec<Vec<String>> = elements
                            .iter()
                            .map(|row| {
                                row.iter()
                                    .map(|cell| {
                                        cell.iter()
                                            .map(|item| pdftext_to_md(item.clone()))
                                            .collect::<Vec<String>>()
                                            .join(" ")
                                    })
                                    .collect()
                            })
                            .collect();

                        let headers = elements[0].to_vec();
                        let rows = elements[1..].to_vec();
                        let md = sheets::to_markdown_table(&headers, &rows);
                        result.push_str(&md);
                    }
                }
            }
            result.push_str("\n");
        }
    }

    Ok(result)
}

fn pdftext_to_md(mut unit: PdfText) -> String {
    let mut text = unit.text;

    if let Some(color) = unit.color {
        if color != "#FFFFFF" {
            text = format!("`{}` ", text);
        }
    }
    if let Some(name) = unit.font_name {
        let lwc = name.to_lowercase();
        if lwc.contains("bold") {
            text = format!("**{}** ", text.trim());
        }
        if lwc.contains("italic") {
            unit.italic = true;
        }
    }
    if unit.italic || unit.italic_angle.unwrap_or_default() != 0.0 {
        text = format!("*{}* ", text.trim());
    }
    if unit.underlined {
        text = format!("<u>{}</u> ", text.trim());
    }

    return text;
}

struct Pdf {
    doc: Document,
}

impl Pdf {
    pub fn new(path: &Path) -> Result<Pdf, Box<dyn Error>> {
        let doc = lopdf::Document::load(path)?;
        let pdf = Pdf { doc };

        Ok(pdf)
    }

    pub fn iter_pages(&self) -> impl Iterator<Item = Result<PdfPage, Box<dyn Error>>> {
        self.doc
            .page_iter()
            .map(|id| PdfPage::from_object_id(&self.doc, id))
    }

    pub fn pdf_units_to_elements(units: Vec<PdfUnit>) -> Vec<Vec<PdfElement>> {
        let elements = pdf_element::units_to_elements(units);
        let mut matrix = pdf_element::elements_into_matrix(elements);
        for row in matrix.iter_mut() {
            pdf_element::sort_transform_elements(row);
        }
        matrix
    }
}
