use core::str;
use std::{collections::BTreeMap, path::Path};

use lopdf::{Dictionary, Document, Encoding, Object};

#[derive(Clone, Debug)]
struct StyledText {
    x: f32,
    y: f32,
    is_line: bool,
    underlined: bool,
    text: Option<String>,
    font_descriptor: Option<Object>,
    table: Option<Table>,
    color: Option<String>,
}

#[derive(Clone, Debug)]
struct Table {
    items: Vec<StyledText>,
    boundaries: Vec<StyledText>,
}

impl Table {
    pub fn to_markdown(&self) -> String {
        todo!()
    }
}

impl StyledText {
    pub fn new() -> Self {
        StyledText {
            x: 0.0,
            y: 0.0,
            is_line: false,
            underlined: false,
            text: None,
            table: None,
            font_descriptor: None,
            color: None,
        }
    }

    fn extract_tables(items: &[StyledText]) -> Vec<StyledText> {
        if items.is_empty() {
            return Vec::new();
        }

        let mut result: Vec<StyledText> = Vec::new();
        let mut current_boundaries: Vec<StyledText> = Vec::new();
        let mut in_table = false;
        let mut table_items: Vec<StyledText> = Vec::new();
        //minx maxx, miny maxy
        let mut bounding_box: Option<(f32, f32, f32, f32)> = None;

        for item in items {
            if item.is_line {
                if !in_table {
                    // Starting a new table
                    in_table = true;
                    current_boundaries.push(item.clone());
                    bounding_box = Some((item.x, item.x, item.y, item.y));
                } else {
                    // Continue existing table - add line to boundaries
                    current_boundaries.push(item.clone());
                    if let Some((min_x, max_x, min_y, max_y)) = bounding_box {
                        bounding_box = Some((
                            min_x.min(item.x),
                            max_x.max(item.x),
                            min_y.min(item.y),
                            max_y.max(item.y),
                        ));
                    }
                }
            } else {
                // Found a text item
                if in_table {
                    let (min_x, max_x, min_y, max_y) = bounding_box.unwrap();
                    let padded_min_x = min_x - 10.0;
                    let padded_max_x = max_x + 10.0;
                    let padded_min_y = min_y - 10.0;
                    let padded_max_y = max_y + 10.0;

                    if item.x >= padded_min_x
                        && item.x <= padded_max_x
                        && item.y >= padded_min_y
                        && item.y <= padded_max_y
                    {
                        // This text is within the table - add to table items
                        table_items.push(item.clone());
                        continue;
                    } else {
                        // This text is outside the table - finish the current table
                        if !table_items.is_empty() && !current_boundaries.is_empty() {
                            let mut item = StyledText::new();
                            let bbox = bounding_box.unwrap();
                            //minx maxx, miny maxy
                            item.x = bbox.1 - ((bbox.1 - bbox.0) / 2.0);
                            item.y = bbox.3 - ((bbox.3 - bbox.2) / 2.0);
                            item.table = Some(Table {
                                items: table_items.clone(),
                                boundaries: current_boundaries.clone(),
                            });
                            result.push(item);
                        }

                        // Reset table state
                        in_table = false;
                        current_boundaries.clear();
                        table_items.clear();
                        bounding_box = None;
                    }
                }

                // Add the text item to the result (if not part of a table)
                if !in_table {
                    result.push(item.clone());
                }
            }
        }

        result
    }

    fn to_markdown(&self) -> String {
        if let Some(table) = self.table {
            return String::from("<<Table>>");
        }
        String::new()
    }

    pub fn normalize(items: &[StyledText]) -> Vec<Vec<String>> {
        if items.is_empty() {
            return Vec::new();
        }

        let items = StyledText::extract_tables(items);
        let mut rows: Vec<Vec<&StyledText>> = Vec::new();

        //grouping into rows of within +-3 y
        for item in items.iter() {
            let mut found_row = false;

            for row in &mut rows {
                if let Some(first_item) = row.first() {
                    if (item.y - first_item.y).abs() <= 3.0 {
                        row.push(item);
                        found_row = true;
                        break;
                    }
                }
            }

            if !found_row {
                rows.push(vec![item]);
            }
        }

        //sort each row by x.
        for row in &mut rows {
            row.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        }

        //sort rows by y
        rows.sort_by(|a, b| {
            if let (Some(a_first), Some(b_first)) = (a.first(), b.first()) {
                a_first
                    .y
                    .partial_cmp(&b_first.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                std::cmp::Ordering::Equal
            }
        });

        let matrix: Vec<Vec<String>> = rows
            .iter()
            .map(|row| row.iter().map(|cell| cell.to_markdown()).collect())
            .collect();

        matrix
    }
}

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
        let mut styled_text = StyledText::new();
        let mut text_list = Vec::new();
        let operations = lopdf::content::Content::decode(&page)?;
        for op in operations.operations {
            match op.operator.as_ref() {
                "TJ" | "Tj" => {
                    let encoding =
                        current_encoding.expect("text didn't contain font encoding. Invalid pdf");
                    let text = extract_text_from_objs(&op.operands, encoding);
                    styled_text.text = Some(text);
                    text_list.push(styled_text.clone());
                    styled_text = StyledText::new();
                }
                "BT" => {} //start of text
                "ET" => {} //end of text
                "Tm" => {
                    //Tm: [150, 0, 0, 150, 0, 0]
                    //     six, x, y, siy, x, y
                }
                "Tf" => {
                    // when it says symbol. likely a list item (worth looking out for)
                    let font_alias = op
                        .operands
                        .first()
                        .expect("Syntax Error: Couldn't get font id")
                        .as_name()
                        .unwrap();
                    current_encoding = encodings.get(font_alias);
                    let font_info = fonts[font_alias];
                    let font_desc = font_info.get(b"FontDescriptor")?;
                    let font_desc_id = extract_ref_from_obj(font_desc)?;
                    let font_desc_obj = doc.get_object(*font_desc_id)?;
                    styled_text.font_descriptor = Some(font_desc_obj.to_owned());
                }
                "Td" => {
                    let items = op
                        .operands
                        .get(..2)
                        .ok_or("failed to position for text in pdf")?;
                    styled_text.x = items[0].as_float()?;
                    styled_text.y = items[1].as_float()?;
                }
                "cm" => {
                    let items = op
                        .operands
                        .get(4..6)
                        .ok_or("failed to position for text in pdf")?;
                    styled_text.x = items[0].as_float()?;
                    styled_text.y = items[1].as_float()?;
                }
                "l" => {
                    let items = op
                        .operands
                        .get(..2)
                        .ok_or("failed to get position for lines in pdf")?;
                    styled_text.x = items[0].as_float()?;
                    styled_text.y = items[1].as_float()?;
                    styled_text.is_line = true;
                    text_list.push(styled_text.clone());
                    styled_text = StyledText::new();
                }
                "sc" | "rg" => {
                    let items = op
                        .operands
                        .get(..3)
                        .ok_or("failed to get color from color operand in pdf")?;
                    let r = items[0].as_float()?;
                    let g = items[1].as_float()?;
                    let b = items[2].as_float()?;
                    if r != 0.0 || g != 0.0 || b != 0.0 {
                        let color = rgb_to_hex(
                            items[0].as_float()?,
                            items[1].as_float()?,
                            items[2].as_float()?,
                        );
                        styled_text.color = Some(color);
                    }
                }
                "SC" | "RG" => {
                    text_list.last_mut().unwrap().underlined = true;
                }
                "h" | "re" | "cs" | "f*" | "S" | "w" | "W" | "n" | "Tc" | "W*" | "J" | "j"
                | "m" | "Do" | "q" | "Q" => {} //colors, and shapes, spacing not imp
                _ => {
                    eprintln!("didn't handle: {} - {:?}", op.operator, op.operands)
                }
            };
        }
        let matrix = StyledText::normalize(&text_list);
        for row in matrix {
            for cell in row {
                result.push_str(&cell);
            }
            result.push_str("\n");
        }
    }

    // Ok(result)
    Ok("".to_string())
}

fn rgb_to_hex(r: f32, g: f32, b: f32) -> String {
    let r = (r * 255.0).round() as u8;
    let g = (g * 255.0).round() as u8;
    let b = (b * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn extract_ref_from_obj(obj: &Object) -> Result<&(u32, u16), &str> {
    match obj {
        Object::Reference(id) => Ok(id),
        _ => Err("failed parsing ref from obj in pdf"),
    }
}

fn extract_dict_from_obj(obj: &Object) -> Result<&Dictionary, &str> {
    match obj {
        Object::Dictionary(dict) => Ok(dict),
        _ => Err("failed parsing dict from obj in pdf"),
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
