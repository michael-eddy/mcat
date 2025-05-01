use core::str;
use std::{collections::BTreeMap, path::Path};

use lopdf::{Document, Encoding, Object};

pub fn pdf_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let doc = lopdf::Document::load(path)?;
    let mut result = String::new();

    for id in doc.page_iter() {
        let page = doc.get_page_content(id)?;
        let fonts = doc.get_page_fonts(id)?;
        let encodings: BTreeMap<Vec<u8>, Encoding> = fonts
            .clone()
            .into_iter()
            .filter_map(|(name, font)| match font.get_font_encoding(&doc) {
                Ok(it) => Some((name, it)),
                Err(_) => None,
            })
            .collect();
        let mut current_encoding = None;
        let operations = lopdf::content::Content::decode(&page)?;
        for op in operations.operations {
            match op.operator.as_ref() {
                "TJ" | "Tj" => {
                    let encoding =
                        current_encoding.expect("text didn't contain font encoding. Invalid pdf");
                    let text = extract_text_from_objs(&op.operands, encoding);
                    result.push_str(&text);
                    eprintln!("TJ: '{text}'");
                }
                "BT" => {} //start of text
                "ET" => {
                    eprintln!(
                        "new line:----------------------------------------------------------------------------------------------------"
                    );
                    eprintln!("{}", result.get(1..).unwrap());
                    result.push_str("\n");
                } //end of text
                "Q" => {}  //restore graphic state (pop the stack)
                "q" => {}  //save graphic state (insert to stack)
                "Tm" => {
                    //Tm: [150, 0, 0, 150, 0, 0]
                    //     six, x, y, siy, x, y
                }
                "Tf" => {
                    let font_alias = op
                        .operands
                        .first()
                        .expect("Syntax Error: Couldn't get font id")
                        .as_name()
                        .unwrap();
                    current_encoding = encodings.get(font_alias);
                    let font_info = fonts[font_alias];
                    let font =
                        str::from_utf8(font_info.get(b"BaseFont")?.as_name().unwrap()).unwrap();
                    let font_desc = font_info.get(b"FontDescriptor")?;
                    let font_desc_id = extract_ref_from_obj(font_desc)?;
                    // let font_desc_obj = doc.get_object(*font_desc_id)?;
                    eprintln!("Tf: {:?}, {:?}", font, font_desc_id);
                    // eprintln!("Font Desc Obj: {:?}", font_desc_obj);
                } //fonts, very imp
                "Td" => {}                   //may be indent (list items)
                "f*" | "S" | "l" | "w" => {} //may help finding underline / strikethrough
                "h" => {}                    //maybe can give highlights??
                "sc" | "cs" | "W" | "n" | "re" | "Tc" | "W*" | "rg" | "J" | "j" | "m" | "Do"
                | "cm" | "RG" => {} //colors, and shapes, spacing not imp
                _ => {
                    eprintln!("didn't handle: {}", op.operator)
                }
            };
        }
    }

    // Ok(result)
    Ok("".to_string())
}

fn extract_ref_from_obj(obj: &Object) -> Result<&(u32, u16), &str> {
    match obj {
        Object::Reference(id) => Ok(id),
        _ => Err("failed parsing ref from obj in pdf"),
    }
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
