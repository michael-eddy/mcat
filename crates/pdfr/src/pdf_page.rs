use std::{collections::BTreeMap, error::Error};

use lopdf::{Dictionary, Document, Encoding, Object, ObjectId};

use crate::{
    pdf_element::{PdfElement, PdfText},
    pdf_state::PdfState,
};

pub struct PdfPage<'a> {
    pub stream: Vec<u8>,
    fonts: BTreeMap<Vec<u8>, &'a Dictionary>,
    encodings: BTreeMap<Vec<u8>, Encoding<'a>>,
    current_encoding: Option<Encoding<'a>>,
    state: PdfState,
    state_stack: Vec<PdfState>,
}

impl<'a> PdfPage<'a> {
    pub fn from_object_id(doc: &Document, id: ObjectId) -> Result<PdfPage, Box<dyn Error>> {
        let stream = doc.get_page_content(id)?;
        let fonts = doc.get_page_fonts(id)?;
        let encodings: BTreeMap<Vec<u8>, Encoding> = fonts
            .clone()
            .into_iter()
            .filter_map(|(name, font)| match font.get_font_encoding(&doc) {
                Ok(it) => Some((name, it)),
                Err(_) => None,
            })
            .collect();

        Ok(PdfPage {
            stream,
            fonts,
            encodings,
            current_encoding: None,
            state: PdfState::new(),
            state_stack: Vec::new(),
        })
    }

    pub fn handle_steam(&self, stream: Vec<u8>) -> Result<Vec<PdfElement>, Box<dyn Error>> {
        let elements: Vec<PdfElement> = Vec::new();
        let current_element = PdfText::default();
        let current_table = PdfText::default();
        let content = lopdf::content::Content::decode(&self.stream)?;

        for op in content.operations {
            match op.operator.as_ref() {
                "Tj" | "TJ" | "'" | "\"" => {
                    // ' is like TJ just with T* before it
                    // " is like as ' just with aw and ac as the first 2 operands
                }
                "Do" => {
                    // may be alot of things, for now just check if steam and if so handle it
                }
                "BT" => {
                    // begin text
                }
                "q" => {
                    // push state to the stack
                }
                "Q" => {
                    // pop state from the stack
                }
                "ET" => {
                    // end text
                }
                "Tf" => {
                    // font info
                }
                "cm" => {
                    // current matrix
                }
                "Tm" => {
                    // text matrix
                }
                "Td" => {
                    // transforms tm
                }
                "TD" => {
                    // just like Td, just sets leading to -ty
                }
                "TL" => {
                    // sets leading
                }
                "T*" => {
                    // applies leading
                }
                "sc" | "rg" => {
                    // sets color
                }
                "SC" | "RG" => {
                    // mostly happens after underlined
                }
                "l" => {
                    // line draw. only slight shot on catching a table, i think.
                }
                // look for more styles, include things like line spacing, et..
                _ => {}
            }
        }
        todo!()
    }

    fn handle_text(objs: Vec<Object>) -> PdfText {
        todo!()
    }
}
