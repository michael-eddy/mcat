use core::str;
use std::{collections::BTreeMap, path::Path};

use lopdf::{Dictionary, Document, Encoding, Object};

use crate::markitdown::sheets;

#[derive(Clone, Debug)]
struct StyledText {
    x: f32,
    y: f32,
    is_line: bool,
    underlined: bool,
    italic: bool,
    is_spacer: bool,
    text: Option<String>,
    font_descriptor: Option<Object>,
    table: Option<Table>,
    color: Option<String>,
}

#[derive(Clone, Debug)]
struct Table {
    items: Vec<StyledText>,
}

impl Table {
    pub fn to_markdown(mut self) -> String {
        self.items
            .sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));

        let mut gaps = Vec::new();
        for i in 1..self.items.len() {
            let gap = self.items[i].y - self.items[i - 1].y;
            if gap > 0.0 {
                gaps.push(gap);
            }
        }

        let median_gap = if !gaps.is_empty() {
            gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mid = gaps.len() / 2;
            if gaps.len() % 2 == 0 {
                (gaps[mid - 1] + gaps[mid]) / 2.0
            } else {
                gaps[mid]
            }
        } else {
            0.0
        };

        // Step 2: Group into row clusters (by Y)
        let mut row_clusters: Vec<Vec<StyledText>> = Vec::new();
        for item in self.items {
            if let Some(last_row) = row_clusters.last_mut() {
                let y_avg = last_row.iter().map(|i| i.y).sum::<f32>() / last_row.len() as f32;
                if (item.y - y_avg).abs() <= 3.0 {
                    last_row.push(item);
                    continue;
                }
            }
            row_clusters.push(vec![item]);
        }

        // Step 3: Discover all global column positions
        let mut col_x_positions: Vec<f32> = Vec::new();
        for row in &row_clusters {
            for item in row {
                let mut found = false;
                for x in &mut col_x_positions {
                    if (item.x - *x).abs() <= 3.0 {
                        *x = (*x + item.x) / 2.0; // smooth merge
                        found = true;
                        break;
                    }
                }
                if !found {
                    col_x_positions.push(item.x);
                }
            }
        }
        col_x_positions.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Step 4: Build matrix
        let mut matrix: Vec<Vec<Option<StyledText>>> = Vec::new();
        for mut row in row_clusters {
            row.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());

            let mut row_vec: Vec<Option<StyledText>> = vec![None; col_x_positions.len()];

            for item in row {
                // Find the closest column index for this x
                if let Some((col_idx, _)) =
                    col_x_positions
                        .iter()
                        .enumerate()
                        .min_by(|(_, x1), (_, x2)| {
                            (item.x - **x1)
                                .abs()
                                .partial_cmp(&(item.x - **x2).abs())
                                .unwrap()
                        })
                {
                    // Optional: merge with previous if gap < median_gap
                    if let Some(Some(prev)) = row_vec.get_mut(col_idx) {
                        if (item.x - prev.x).abs() < median_gap {
                            // Merge text (basic example)
                            if let (Some(t1), Some(t2)) = (&prev.text, &item.text) {
                                prev.text = Some(format!("{} {}", t1, t2));
                            }
                        } else {
                            row_vec[col_idx] = Some(item);
                        }
                    } else {
                        row_vec[col_idx] = Some(item);
                    }
                }
            }

            matrix.push(row_vec);
        }

        let mds: Vec<Vec<String>> = matrix
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| {
                        if cell.is_some() {
                            cell.clone().unwrap().to_markdown()
                        } else {
                            "".to_string()
                        }
                    })
                    .collect()
            })
            .collect();
        let headers = mds[0].to_vec();
        let rest = mds[1..].to_vec();
        sheets::to_markdown_table(&headers, &rest)
    }
}

impl StyledText {
    pub fn new() -> Self {
        StyledText {
            x: 0.0,
            y: 0.0,
            is_line: false,
            underlined: false,
            is_spacer: false,
            italic: false,
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
        let mut in_table = false;
        let mut table_items: Vec<StyledText> = Vec::new();
        //minx maxx, miny maxy
        let mut bounding_box: Option<(f32, f32, f32, f32)> = None;

        for item in items {
            if item.is_line {
                if !in_table {
                    // Starting a new table
                    in_table = true;
                    bounding_box = Some((item.x, item.x, item.y, item.y));
                } else {
                    // Continue existing table - add line to boundaries
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
                        if !table_items.is_empty() {
                            let mut item = StyledText::new();
                            let bbox = bounding_box.unwrap();
                            //minx maxx, miny maxy
                            item.x = bbox.1 - ((bbox.1 - bbox.0) / 2.0);
                            item.y = bbox.3 - ((bbox.3 - bbox.2) / 2.0);
                            item.table = Some(Table {
                                items: table_items.clone(),
                            });
                            result.push(item);
                        }

                        // Reset table state
                        in_table = false;
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

    fn to_markdown(mut self) -> String {
        if let Some(table) = self.table {
            return table.to_markdown();
        } else {
            if self.is_line {
                //bad
                return "".to_string();
            }
            if self.is_spacer {
                return "".to_string();
            }
            let mut text = self.text.unwrap();
            let fd = self.font_descriptor.unwrap();
            let fd = extract_dict_from_obj(&fd).unwrap();
            let font_name = fd.get(b"FontName").unwrap().as_name().unwrap();
            let mut bold = false;
            if let Ok(font_name) = str::from_utf8(font_name) {
                let font_name = font_name.to_lowercase();
                if font_name.contains("bold") {
                    bold = true;
                }
                if font_name.contains("italic") {
                    self.italic = true;
                }
            }
            let italic_angle = fd.get(b"ItalicAngle").unwrap().as_float().unwrap();
            if italic_angle != 0.0 {
                self.italic = true;
            }

            //format
            // sadly there isn't a nice way to maintain the color
            if let Some(color) = self.color {
                if color != "#FFFFFF" {
                    text = format!("`{}` ", text);
                }
            }
            if bold {
                text = format!("**{}** ", text.trim());
            }
            if self.italic {
                text = format!("*{}* ", text.trim());
            }
            if self.underlined {
                text = format!("<u>{}</u> ", text.trim());
            }
            return text;
        }
    }

    fn fair_gap(numbers: &mut [f32]) -> f32 {
        numbers.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let gaps: Vec<f32> = numbers.windows(2).map(|pair| pair[1] - pair[0]).collect();
        if gaps.is_empty() {
            return 0.0;
        }

        let mut sorted_gaps = gaps;
        sorted_gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let idx = ((sorted_gaps.len() as f32) * 0.8).floor() as usize;
        sorted_gaps[idx.min(sorted_gaps.len() - 1)]
    }

    fn insert_spacers(items: Vec<Vec<&StyledText>>, valid_gap: f32) -> Vec<Vec<StyledText>> {
        let mut result: Vec<Vec<StyledText>> = Vec::new();
        let len = items.len();

        for i in 0..len {
            let current_row: Vec<StyledText> = items[i].iter().map(|f| (*f).clone()).collect();
            let mut spacer = None;
            let first = current_row.first().map(|f| f.clone()).unwrap();

            if i + 1 < len {
                let next_row = items[i + 1].to_owned();
                let next = next_row.first().unwrap();
                let gap = first.y - next.y;
                if gap > valid_gap * 1.5 {
                    let mut spacer_temp = StyledText::new();
                    spacer_temp.is_spacer = true;
                    spacer = Some(spacer_temp);
                }
            }
            result.push(current_row);
            if let Some(spacer) = spacer {
                result.push(vec![spacer]);
            }
        }

        result
    }

    pub fn normalize(items: &[StyledText]) -> Vec<Vec<String>> {
        if items.is_empty() {
            return Vec::new();
        }

        let items = StyledText::extract_tables(items);
        let mut rows: Vec<Vec<&StyledText>> = Vec::new();
        // for figuring out a fair gap (so we can have better seperated paragraphs)
        let mut heights: Vec<f32> = Vec::new();

        //grouping into rows of within +-3 y
        for item in items.iter() {
            let mut found_row = false;

            for row in &mut rows {
                if let Some(first_item) = row.first() {
                    if (item.y - first_item.y).abs() <= 3.0 {
                        heights.push(first_item.y);
                        row.push(item);
                        found_row = true;
                        break;
                    }
                }
            }

            if !found_row {
                heights.push(item.y);
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
                b_first
                    .y
                    .partial_cmp(&a_first.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                std::cmp::Ordering::Equal
            }
        });

        let median_gap = Self::fair_gap(&mut heights);
        let rows = Self::insert_spacers(rows, median_gap);

        let matrix: Vec<Vec<String>> = rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| cell.to_owned().to_markdown())
                    .collect()
            })
            .collect();

        matrix
    }
}

pub fn pdf_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let doc = lopdf::Document::load(path)?;
    let mut result = String::new();

    let mut page_nm = 0;
    for id in doc.page_iter() {
        let page = doc.get_page_content(id)?;
        let fonts = doc.get_page_fonts(id)?;
        page_nm += 1;
        result.push_str(&format!("\n# Page {}\n\n", page_nm));
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
        let mut text_list: Vec<StyledText> = Vec::new();
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
                "Tm" => {
                    let items = op
                        .operands
                        .get(..6)
                        .ok_or("failed to position for text in pdf")?;
                    let x = items[4].as_float()?;
                    let y = items[5].as_float()?;
                    if x != 0.0 {
                        styled_text.x = x;
                    }
                    if y != 0.0 {
                        styled_text.y = y;
                    }
                    styled_text.italic = items[1].as_float()? != 0.0 || items[2].as_float()? != 0.0
                }
                "Td" => {
                    let items = op
                        .operands
                        .get(..2)
                        .ok_or("failed to position for text in pdf")?;
                    let x = items[0].as_float()?;
                    let y = items[1].as_float()?;
                    if x != 0.0 {
                        styled_text.x = x;
                    }
                    if y != 0.0 {
                        styled_text.y = y;
                    }
                }
                "cm" => {
                    let items = op
                        .operands
                        .get(4..6)
                        .ok_or("failed to position for text in pdf")?;
                    let x = items[0].as_float()?;
                    let y = items[1].as_float()?;
                    if x != 0.0 {
                        styled_text.x = x;
                    }
                    if y != 0.0 {
                        styled_text.y = y;
                    }
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
                result.push_str(&format!("{} ", cell));
            }
            result.push_str("  \n");
        }
    }

    Ok(result)
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
