use std::{error::Error, path::Path};

use lopdf::Document;
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
}
