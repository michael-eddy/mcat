use std::{error::Error, path::Path};

use lopdf::Document;
use pdf_element::{PdfElement, PdfUnit};
use pdf_page::PdfPage;

pub mod pdf_element;
pub mod pdf_page;
pub mod pdf_state;

pub struct Pdf {
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
