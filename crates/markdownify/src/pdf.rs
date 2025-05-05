use core::str;
use std::{collections::BTreeMap, mem::take, path::Path};

use lopdf::{Dictionary, Document, Encoding, Object};

use crate::sheets;

#[derive(Clone, Debug, Default)]
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

#[derive(Debug)]
struct TableBoundary {
    minx: f32,
    maxx: f32,
    miny: f32,
    maxy: f32,
    text: Option<Vec<StyledText>>,
}

impl TableBoundary {
    pub fn get_text(self) -> String {
        match self.text {
            Some(mut st) => {
                st.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal));
                let result: Vec<String> =
                    st.iter().map(|item| item.clone().to_markdown()).collect();
                result.join(" ")
            }
            None => "".to_string(),
        }
    }

    pub fn assign(&mut self, cont: &StyledText) {
        let x = cont.x + 3.0;
        let y = cont.y - 3.0;
        if x > self.minx && x < self.maxx && y < self.maxy && y > self.miny {
            match self.text.as_mut() {
                Some(texts) => texts.push(cont.clone()),
                None => self.text = Some(vec![cont.clone()]),
            };
        }
    }

    pub fn create_boundaries(coords: &[(f32, f32)]) -> Vec<TableBoundary> {
        let mut xs: Vec<f32> = Vec::new();
        let mut ys: Vec<f32> = Vec::new();
        let mut heights: Vec<f32> = coords.iter().map(|f| f.1).collect();
        let mut table_bondaries: Vec<TableBoundary> = Vec::new();
        let fair_gap = StyledText::fair_gap(&mut heights);

        // assigning xs (+-3) and ys (+-fair gap)
        for coord in coords {
            //xs
            let mut found_matching_x = false;
            for x in xs.iter() {
                if (x - coord.0).abs() < 3.0 {
                    found_matching_x = true;
                    break;
                }
            }
            if !found_matching_x {
                xs.push(coord.0);
            }
            //ys
            let mut found_matching_y = false;
            for y in ys.iter() {
                let sub: f32 = y - coord.1;
                if sub.abs() < fair_gap {
                    found_matching_y = true;
                    break;
                }
            }
            if !found_matching_y {
                ys.push(coord.1);
            }
        }

        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        ys.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let mut smallest_y = f32::INFINITY;
        for y_pair in ys.windows(2) {
            let miny = y_pair[1];
            let maxy = y_pair[0];
            if miny < smallest_y {
                smallest_y = miny;
            }

            let mut biggest_x = 0.0;
            for x_pair in xs.windows(2) {
                let minx = x_pair[0];
                let maxx = x_pair[1];
                if maxx > biggest_x {
                    biggest_x = maxx;
                }

                table_bondaries.push(TableBoundary {
                    minx,
                    maxx,
                    miny,
                    maxy,
                    text: None,
                });
            }
            table_bondaries.push(TableBoundary {
                minx: biggest_x,
                maxx: biggest_x * 2.0,
                miny,
                maxy,
                text: None,
            });
        }
        let mut biggest_x = 0.0;
        for x_pair in xs.windows(2) {
            let minx = x_pair[0];
            let maxx = x_pair[1];
            if maxx > biggest_x {
                biggest_x = maxx;
            }

            table_bondaries.push(TableBoundary {
                minx,
                maxx,
                miny: smallest_y - 500.0,
                maxy: smallest_y,
                text: None,
            });
        }
        table_bondaries.push(TableBoundary {
            minx: biggest_x,
            maxx: biggest_x * 2.0,
            miny: smallest_y - 500.0,
            maxy: smallest_y,
            text: None,
        });

        table_bondaries
    }
}

impl Table {
    pub fn to_markdown(self) -> String {
        if self.items.is_empty() {
            return "".to_string();
        }
        let coords: Vec<(f32, f32)> = self.items.iter().map(|f| (f.x, f.y)).collect();
        let mut boundaries = TableBoundary::create_boundaries(&coords);

        for item in self.items {
            for boundary in boundaries.iter_mut() {
                boundary.assign(&item);
            }
        }

        let mut pre_height = 0.0;
        let mut matrix: Vec<Vec<String>> = Vec::new();
        let mut current_row: Vec<String> = Vec::new();
        let mut start = true;
        for boundary in boundaries {
            if start {
                start = false;
                pre_height = boundary.maxy;
            }
            if boundary.maxy < pre_height {
                pre_height = boundary.maxy;
                matrix.push(std::mem::take(&mut current_row));
            }
            current_row.push(boundary.get_text());
        }
        matrix.push(current_row);

        if matrix.is_empty() {
            return "".to_string();
        }
        let headers = matrix.get(0).unwrap();
        let rows = matrix.get(1..).unwrap_or_default();
        sheets::to_markdown_table(&headers, &rows)
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
                    let bbox = bounding_box.unwrap_or_default();
                    let (min_x, max_x, min_y, max_y) = bbox;
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

            // gathering styles but not a must.
            let mut text = self.text.unwrap_or_default();
            let mut bold = false;
            if let Some(fd) = self
                .font_descriptor
                .as_ref()
                .and_then(|fd_obj| extract_dict_from_obj(fd_obj).ok())
            {
                if let Ok(font_name) = fd.get(b"FontName").and_then(|f| f.as_name()) {
                    if let Ok(font_name) = str::from_utf8(font_name) {
                        let font_name = font_name.to_lowercase();
                        if font_name.contains("bold") {
                            bold = true;
                        }
                        if font_name.contains("italic") {
                            self.italic = true;
                        }
                    }
                }
                if let Ok(italic_angle) = fd.get(b"ItalicAngle").and_then(|f| f.as_float()) {
                    if italic_angle != 0.0 {
                        self.italic = true;
                    }
                }
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
        numbers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let gaps: Vec<f32> = numbers.windows(2).map(|pair| pair[1] - pair[0]).collect();
        if gaps.is_empty() {
            return 0.0;
        }

        let mut sorted_gaps = gaps;
        sorted_gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let idx = ((sorted_gaps.len() as f32) * 0.8).floor() as usize;
        sorted_gaps[idx.min(sorted_gaps.len() - 1)]
    }

    fn insert_spacers(items: Vec<Vec<&StyledText>>, valid_gap: f32) -> Vec<Vec<StyledText>> {
        let mut result: Vec<Vec<StyledText>> = Vec::new();
        let len = items.len();

        for i in 0..len {
            let current_row: Vec<StyledText> = items[i].iter().map(|f| (*f).clone()).collect();
            let mut spacer = None;
            let first = current_row.first().map(|f| f.clone()).unwrap_or_default();

            if i + 1 < len {
                let next_row = items[i + 1].to_owned();
                if next_row.is_empty() {
                    continue;
                }
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
        // for figuring out a fair gap (so we can have better separated paragraphs)
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

fn compute_pdf_position(
    cm: Option<Vec<f32>>,
    tm: Option<Vec<f32>>,
    td: Option<Vec<f32>>,
) -> (f32, f32) {
    if let Some(td) = td {
        return (td[0], td[1]);
    }
    let cm = cm.unwrap_or(vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    let tm = tm.unwrap_or(vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    let rel_x = tm[4];
    let rel_y = tm[5];
    let x_scale = cm[0];
    let y_scale = cm[3];
    let x_origin = cm[4];
    let y_origin = cm[5];

    let final_x = rel_x * x_scale + x_origin;
    let final_y = rel_y * y_scale + y_origin;

    (final_x, final_y)
}

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
        let mut cm = None;
        let mut tm = None;
        let mut td = None;
        let operations = lopdf::content::Content::decode(&page)?;
        for op in operations.operations {
            match op.operator.as_ref() {
                "TJ" | "Tj" => {
                    let encoding =
                        current_encoding.expect("text didn't contain font encoding. Invalid pdf");
                    let text = extract_text_from_objs(&op.operands, encoding);
                    styled_text.text = Some(text);
                    let (x, y) = compute_pdf_position(take(&mut cm), take(&mut tm), take(&mut td));
                    styled_text.x = x;
                    styled_text.y = y;
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
                        .as_name()?;
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
                    tm = Some(items.iter().map(|f| f.as_float().unwrap()).collect());
                    styled_text.italic = items[1].as_float()? != 0.0 || items[2].as_float()? != 0.0
                }
                "Td" => {
                    let items = op
                        .operands
                        .get(..2)
                        .ok_or("failed to position for text in pdf")?;
                    td = Some(items.iter().map(|f| f.as_float().unwrap()).collect());
                }
                "cm" => {
                    let items = op
                        .operands
                        .get(..6)
                        .ok_or("failed to position for text in pdf")?;
                    cm = Some(items.iter().map(|f| f.as_float().unwrap()).collect());
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
                            styled_text.color = Some(color);
                        }
                    }
                }
                "SC" | "RG" => {
                    if let Some(last) = text_list.last_mut() {
                        last.underlined = true;
                    }
                }
                "h" | "re" | "cs" | "f*" | "S" | "w" | "W" | "n" | "Tc" | "W*" | "J" | "j"
                | "m" | "Do" | "q" | "Q" => {} //colors, and shapes, spacing not imp
                _ => {}
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
