use std::{collections::BTreeMap, error::Error, mem::take};

use lopdf::{Dictionary, Document, Encoding, Object, ObjectId, Stream, xobject};

use crate::{
    pdf_element::{PdfLine, PdfText, PdfUnit},
    pdf_state::PdfState,
};

struct ChildFontsEncodings<'a> {
    pub fonts: BTreeMap<Vec<u8>, &'a Dictionary>,
    pub encodings: BTreeMap<Vec<u8>, Encoding<'a>>,
}
pub struct PdfPage<'a> {
    pub stream: Vec<u8>,
    id: ObjectId,
    document: &'a Document,
    fonts: BTreeMap<Vec<u8>, &'a Dictionary>,
    encodings: BTreeMap<Vec<u8>, Encoding<'a>>,
    current_font_alias: Vec<u8>,
    state: PdfState,
    state_stack: Vec<PdfState>,
    child_state: Option<ChildFontsEncodings<'a>>,
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
            id,
            document: doc,
            encodings,
            current_font_alias: Vec::new(),
            state: PdfState::new(),
            state_stack: Vec::new(),
            child_state: None,
        })
    }

    pub fn handle_stream(&mut self, stream: Vec<u8>) -> Result<Vec<PdfUnit>, Box<dyn Error>> {
        let mut elements: Vec<PdfUnit> = Vec::new();
        let mut current_element = PdfText::default();
        let content = lopdf::content::Content::decode(&stream)?;

        let _: Vec<Result<(), Box<dyn Error>>> = content
            .operations
            .iter()
            .map(|op| -> Result<(), Box<dyn Error>> {
                match op.operator.as_ref() {
                    "Tj" | "TJ" | "'" | "\"" => {
                        // ' is like TJ just with T* before it
                        // " is like as ' just with aw and ac as the first 2 operands
                        let r: &str = op.operator.as_ref();
                        if r == "'" || r == "\"" {
                            self.state.t_star();
                        }
                        let text =
                            extract_text_from_objs(&op.operands, self.get_current_encoding());
                        eprintln!("TJ: {}", text);
                        let (x, y) = self.state.current_position();
                        current_element.text = text;
                        current_element.x = x;
                        current_element.y = y;
                        elements.push(PdfUnit::Text(take(&mut current_element)));
                        Ok(())
                    }
                    "Do" => {
                        // may be alot of things, for now just check if steam and if so handle it
                        eprintln!("Starting Do----------------------------------");
                        let obj = op
                            .operands
                            .get(0)
                            .ok_or("failed to query xobject from 'Do' operator")?;
                        let resource = self.document.get_page_resources(self.id)?;
                        let page_dict = match resource.0 {
                            Some(v) => Some(v),
                            None => resource.1.iter().find_map(|obj_id| {
                                let obj = self.document.get_object(*obj_id).ok()?;
                                obj.as_dict().ok()
                            }),
                        }
                        .ok_or("failed to query resource from pdf")?;

                        let dict = page_dict.get(b"XObject")?.as_dict()?;
                        let id = dict.get(obj.as_name()?)?.as_reference()?;

                        let stream = self.document.get_object(id)?.as_stream()?;
                        let raw = stream.decompressed_content()?;

                        let units = self.handle_stream(raw)?;
                        elements.extend(units);
                        eprintln!(
                            "Ended Do---------------------------------- {}",
                            stream.content.len()
                        );
                        Ok(())
                    }
                    "BT" => {
                        // begin text
                        self.state.bt();
                        Ok(())
                    }
                    "q" => {
                        // push state to the stack
                        self.state_stack.push(self.state.clone());
                        Ok(())
                    }
                    "Q" => {
                        // pop state from the stack
                        self.state = self.state_stack.pop().unwrap_or_default();
                        Ok(())
                    }
                    "ET" => {
                        // end text
                        self.state.et();
                        Ok(())
                    }
                    "Tf" => {
                        // font info
                        let font_alias = op
                            .operands
                            .first()
                            .ok_or("failed to query current font in the pdf")?
                            .as_name()?;
                        self.current_font_alias = font_alias.to_owned();
                        eprintln!("setted tf with: {:?}", self.current_font_alias);

                        // styles realted to fonts~
                        let font_info = self
                            .fonts
                            .get(font_alias)
                            .ok_or("failed to get fonts for page")?;
                        let font_ref = font_info.get(b"FontDescriptor")?.as_reference()?;
                        let font_desc = self.document.get_object(font_ref)?.to_owned();
                        let fd = font_desc.as_dict()?;
                        let font_name = fd.get(b"FontName")?.as_name()?;
                        current_element.font_name = String::from_utf8(font_name.to_vec()).ok();
                        let italic_angle = fd.get(b"ItalicAngle")?;
                        current_element.italic_angle = italic_angle.as_float().ok();
                        Ok(())
                    }
                    "cm" => {
                        // current matrix
                        let items: Vec<f32> = op
                            .operands
                            .get(..6)
                            .ok_or("failed to get position for text in pdf")?
                            .iter()
                            .map(|f| f.as_float().unwrap())
                            .collect();
                        self.state
                            .cm(items[0], items[1], items[2], items[3], items[4], items[5]);
                        Ok(())
                    }
                    "Tm" => {
                        // text matrix
                        let items: Vec<f32> = op
                            .operands
                            .get(..6)
                            .ok_or("failed to get position for text in pdf")?
                            .iter()
                            .map(|f| f.as_float().unwrap())
                            .collect();
                        self.state
                            .tm(items[0], items[1], items[2], items[3], items[4], items[5]);
                        Ok(())
                    }
                    "Td" => {
                        // transforms tm
                        let items: Vec<f32> = op
                            .operands
                            .get(..2)
                            .ok_or("failed to get position for text in pdf")?
                            .iter()
                            .map(|f| f.as_float().unwrap())
                            .collect();
                        self.state.td(items[0], items[1]);
                        Ok(())
                    }
                    "TD" => {
                        // just like Td, just sets leading to -ty
                        let items: Vec<f32> = op
                            .operands
                            .get(..2)
                            .ok_or("failed to get position for text in pdf")?
                            .iter()
                            .map(|f| f.as_float().unwrap())
                            .collect();
                        self.state.td_capital(items[0], items[1]);
                        Ok(())
                    }
                    "TL" => {
                        // sets leading
                        self.state.tl(op.operands[0].as_float().unwrap());
                        Ok(())
                    }
                    "T*" => {
                        // applies leading
                        self.state.t_star();
                        Ok(())
                    }
                    "sc" | "rg" => {
                        // sets color
                        if let Some(items) = op.operands.get(..3) {
                            let r = items[0].as_float()?;
                            let g = items[1].as_float()?;
                            let b = items[2].as_float()?;
                            if r != 0.0 || g != 0.0 || b != 0.0 {
                                let color = rgb_to_hex(
                                    items[0].as_float()?,
                                    items[1].as_float()?,
                                    items[2].as_float()?,
                                );
                                current_element.color = Some(color);
                            }
                        }
                        Ok(())
                    }
                    "SC" | "RG" => {
                        // mostly happens after underlined
                        if let Some(last) = elements.last_mut() {
                            match last {
                                PdfUnit::Text(pdf_text) => pdf_text.underlined = true,
                                PdfUnit::Line(_) => {}
                            };
                        }
                        Ok(())
                    }
                    "m" => {
                        let items: Vec<f32> = op
                            .operands
                            .get(..2)
                            .ok_or("failed to get position for line in pdf")?
                            .iter()
                            .map(|f| f.as_float().unwrap())
                            .collect();
                        self.state.m(items[0], items[1]);
                        Ok(())
                    }
                    "l" => {
                        let items: Vec<f32> = op
                            .operands
                            .get(..2)
                            .ok_or("failed to get position for line in pdf")?
                            .iter()
                            .map(|f| f.as_float().unwrap())
                            .collect();
                        let (from, to) = self.state.l((items[0], items[1]));
                        let line = PdfLine { from, to };
                        elements.push(PdfUnit::Line(line));
                        Ok(())
                    }
                    // look for more styles, include things like line spacing, et..
                    _ => Ok(()),
                }
            })
            .collect();
        Ok(elements)
    }

    fn get_current_encoding(&self) -> &Encoding<'_> {
        self.encodings
            .get(&self.current_font_alias)
            .expect("couldn't get current encoding")
    }
}

fn rgb_to_hex(r: f32, g: f32, b: f32) -> String {
    let r = (r * 255.0).round() as u8;
    let g = (g * 255.0).round() as u8;
    let b = (b * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn extract_text_from_objs(objs: &[Object], encoding: &Encoding) -> String {
    let mut text = String::new();
    for obj in objs {
        text.push_str(&extract_text_from_obj(obj, encoding));
    }
    text
}

fn extract_text_from_obj(obj: &Object, encoding: &Encoding) -> String {
    let mut text = String::new();
    if obj.as_str().unwrap_or_default() == &[0xB7] {
        return "*".to_string();
    }
    match obj {
        Object::String(bytes, _) | Object::Name(bytes) => {
            if let Ok(s) = Document::decode_text(encoding, bytes) {
                text.push_str(&s);
            }
        }
        Object::Array(nested) => {
            text.push_str(&extract_text_from_objs(nested, encoding));
        }
        _ => {}
    }
    text
}
