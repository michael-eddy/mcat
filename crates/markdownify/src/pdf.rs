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
                .and_then(|fd_obj| fd_obj.as_dict().ok())
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

#[derive(Debug, Clone, Copy)]
pub struct Matrix3x3 {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Matrix3x3 {
    pub fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    pub fn from_components(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self { a, b, c, d, e, f }
    }

    pub fn translate(&self, tx: f32, ty: f32) -> Self {
        self.multiply(&Matrix3x3 {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        })
    }

    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            e: self.a * other.e + self.c * other.f + self.e,
            f: self.b * other.e + self.d * other.f + self.f,
        }
    }

    pub fn apply_to_origin(&self) -> (f32, f32) {
        (self.e, self.f)
    }
}

#[derive(Debug, Clone)]
pub struct TextState {
    tm: Matrix3x3,
    tlm: Matrix3x3,
    leading: f32,
    ctm: Matrix3x3,
}

impl TextState {
    pub fn new() -> Self {
        Self {
            tm: Matrix3x3::identity(),
            tlm: Matrix3x3::identity(),
            ctm: Matrix3x3::identity(),
            leading: 0.0,
        }
    }

    pub fn bt(&mut self) {
        eprintln!("begin-------------------");
        self.tm = Matrix3x3::identity();
        self.tlm = Matrix3x3::identity();
    }

    pub fn et(&mut self) {
        eprintln!("end---------------------");
        // No-op
    }

    pub fn tl(&mut self, leading: f32) {
        eprintln!("setting lead to: {}", leading);
        self.leading = leading;
    }

    pub fn td(&mut self, tx: f32, ty: f32) {
        let translation = Matrix3x3::from_components(1.0, 0.0, 0.0, 1.0, tx, ty);
        self.tlm = translation.multiply(&self.tlm);
        eprintln!("Td: {:?}", self.tm);
        self.tm = self.tlm;
    }

    pub fn td_capital(&mut self, tx: f32, ty: f32) {
        self.leading = -ty;
        eprintln!("Starting TD------------");
        self.td(tx, ty);
    }

    pub fn tm(&mut self, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) {
        let m = Matrix3x3::from_components(a, b, c, d, e, f);
        self.tm = m;
        eprintln!("TM: {:?}", self.tm);
        self.tlm = m;
    }

    pub fn t_star(&mut self) {
        eprintln!("Starting T*------------");
        self.td(0.0, -self.leading);
    }

    pub fn cm(&mut self, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) {
        self.ctm = Matrix3x3::from_components(a, b, c, d, e, f);
        eprintln!("cm: {:?}", self.ctm);
    }

    pub fn current_position(&self) -> (f32, f32) {
        let combined = self.ctm.multiply(&self.tm);
        let f = combined.apply_to_origin();
        eprintln!("final: {:?}", f);
        f
    }
}

/// convert `pdf` into markdown
/// # usuage:
/// ```
/// let path = Path::new("path/to/file.pdf");
/// let md = pdf_convert(&path).unwrap();
/// println!("{}", md);
/// ```
pub fn pdf_convert(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let doc = lopdf::Document::load(path)?;
    let mut result = String::new();

    let mut page_nm = 0;
    for id in doc.page_iter() {
        let page = doc.get_page_content(id)?;
        let fonts = doc.get_page_fonts(id).unwrap_or_default();
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
        let mut text_state = TextState::new();
        let mut state_stack = Vec::new();
        let operations = lopdf::content::Content::decode(&page)?;
        for op in operations.operations {
            match op.operator.as_ref() {
                "TJ" | "Tj" => {
                    let encoding =
                        current_encoding.expect("text didn't contain font encoding. Invalid pdf");
                    let text = extract_text_from_objs(&op.operands, encoding);
                    eprintln!("TJ: {}", text);
                    styled_text.text = Some(text);
                    let (x, y) = text_state.current_position();
                    styled_text.x = x;
                    styled_text.y = y;
                    text_list.push(styled_text.clone());
                    styled_text = StyledText::new();
                }
                "'" => {
                    //same as T* and right after TJ, contains string
                    text_state.t_star();
                    let encoding =
                        current_encoding.expect("text didn't contain font encoding. Invalid pdf");
                    let text = extract_text_from_objs(&op.operands, encoding);
                    eprintln!("': {}", text);
                    styled_text.text = Some(text);
                    let (x, y) = text_state.current_position();
                    styled_text.x = x;
                    styled_text.y = y;
                    text_list.push(styled_text.clone());
                    styled_text = StyledText::new();
                }
                "\"" => {
                    text_state.t_star();
                    let encoding =
                        current_encoding.expect("text didn't contain font encoding. Invalid pdf");
                    let obj = op.operands.get(2).unwrap();
                    let text = extract_text_from_obj(&obj, encoding);
                    eprintln!("\": {}", text);
                    styled_text.text = Some(text);
                    let (x, y) = text_state.current_position();
                    styled_text.x = x;
                    styled_text.y = y;
                    text_list.push(styled_text.clone());
                    styled_text = StyledText::new();
                    // same as the above just aw ac string in the operands
                }
                "Do" => {
                    let obj = op.operands.get(0).unwrap();
                    let res = doc.get_page_resources(id).unwrap();
                    let dict = res.0.unwrap().get(b"XObject").unwrap().as_dict().unwrap();
                    let id = dict
                        .get(obj.as_name().unwrap())
                        .unwrap()
                        .as_reference()
                        .unwrap();
                    let obj = doc.get_object(id).unwrap().as_stream().unwrap();
                    eprintln!("Do :{:?}", obj);
                    // if its a steam it can be lit everything else at once.. need to make things
                    // way more modular / slick so we can reuse parts, current impl will just
                    // require re doing everything here.
                }
                "BT" => {
                    text_state.bt();
                } //start of text
                "ET" => {
                    text_state.et();
                } //end of text
                "Tf" => {
                    // when it says symbol. likely a list item (worth looking out for)
                    if let Some(font_alias) = op.operands.first().and_then(|f| f.as_name().ok()) {
                        current_encoding = encodings.get(font_alias);
                        let font_info = fonts[font_alias];
                        styled_text.font_descriptor = get_font_descriptor(&doc, font_info);
                    }
                }
                "Tm" => {
                    let items = op
                        .operands
                        .get(..6)
                        .ok_or("failed to position for text in pdf")?;
                    text_state.tm(
                        items[0].as_float().unwrap(),
                        items[1].as_float().unwrap(),
                        items[2].as_float().unwrap(),
                        items[3].as_float().unwrap(),
                        items[4].as_float().unwrap(),
                        items[5].as_float().unwrap(),
                    );
                    styled_text.italic = items[1].as_float()? != 0.0 || items[2].as_float()? != 0.0
                }
                "Td" => {
                    let items = op
                        .operands
                        .get(..2)
                        .ok_or("failed to position for text in pdf")?;
                    text_state.td(items[0].as_float().unwrap(), items[1].as_float().unwrap());
                }
                "cm" => {
                    let items = op
                        .operands
                        .get(..6)
                        .ok_or("failed to position for text in pdf")?;
                    text_state.cm(
                        items[0].as_float().unwrap(),
                        items[1].as_float().unwrap(),
                        items[2].as_float().unwrap(),
                        items[3].as_float().unwrap(),
                        items[4].as_float().unwrap(),
                        items[5].as_float().unwrap(),
                    );
                }
                "l" => {
                    let items = op
                        .operands
                        .get(..2)
                        .ok_or("failed to get position for lines in pdf")?;
                    eprintln!("L: ---------");
                    styled_text.x = items[0].as_float()?;
                    styled_text.y = items[1].as_float()?;
                    styled_text.is_line = true;
                    text_list.push(styled_text.clone());
                    styled_text = StyledText::new();
                }
                "TL" => {
                    let item = op
                        .operands
                        .get(0)
                        .ok_or("failed it get leading op in pdf")?;
                    text_state.tl(item.as_float().unwrap());
                }
                "TD" => {
                    let items = op
                        .operands
                        .get(..2)
                        .ok_or("failed to position for text in pdf")?;
                    text_state
                        .td_capital(items[0].as_float().unwrap(), items[1].as_float().unwrap());
                }
                "T*" => {
                    text_state.t_star();
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
                "q" => {
                    state_stack.push(text_state.clone());
                    eprintln!("q: pushing state------------");
                }
                "Q" => {
                    if let Some(saved) = state_stack.pop() {
                        text_state = saved;
                    }
                    eprintln!("Q: poping state-------------");
                }
                // "h" | "re" | "cs" | "f*" | "S" | "w" | "W" | "n" | "Tc" | "W*" | "J" | "j"
                // | "m" | "Do" | "Tw" | "i" | "f" => {} // irrelevant
                _ => {
                    eprintln!("didn't handle: {}, op: {:?}", op.operator, op.operands);
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

fn get_font_descriptor(doc: &Document, font_info: &Dictionary) -> Option<Object> {
    let font_desc = font_info.get(b"FontDescriptor").ok()?;
    let font_desc_id = font_desc.as_reference().ok()?;
    let font_desc_obj = doc.get_object(font_desc_id).ok()?;

    Some(font_desc_obj.to_owned())
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
