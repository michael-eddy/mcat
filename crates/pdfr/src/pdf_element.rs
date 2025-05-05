#[derive(Clone)]
pub enum PdfElement {
    Text(PdfText),
    Table(PdfTable),
}
pub enum PdfUnit {
    Text(PdfText),
    Line(PdfLine),
}

pub struct PdfLine {
    pub from: (f32, f32),
    pub to: (f32, f32),
}

#[derive(Default, Clone)]
pub struct PdfText {
    pub text: String,
    pub font_name: Option<String>,
    pub italic_angle: Option<f32>,
    pub x: f32,
    pub y: f32,
    pub underlined: bool,
    pub color: Option<String>,
}

#[derive(Default, Clone)]
struct TableBoundary {
    minx: f32,
    maxx: f32,
    miny: f32,
    maxy: f32,
    elements: Vec<PdfText>,
}
#[derive(Default, Clone)]
pub struct PdfTable {
    boundaries: Vec<TableBoundary>,
    y: f32, // just the center of it
    x: f32, // just the center of it
}

impl PdfElement {
    pub fn get_y(&self) -> f32 {
        match self {
            PdfElement::Text(pdf_text) => pdf_text.y,
            PdfElement::Table(pdf_table) => pdf_table.y,
        }
    }
    pub fn get_x(&self) -> f32 {
        match self {
            PdfElement::Text(pdf_text) => pdf_text.x,
            PdfElement::Table(pdf_table) => pdf_table.x,
        }
    }
}
impl PdfTable {
    pub fn from_lines(lines: Vec<PdfLine>) -> Vec<PdfTable> {
        //TODO
        Vec::new()
    }

    pub fn assign(&mut self, element: &PdfText) -> bool {
        for boundary in self.boundaries.iter_mut() {
            if boundary.assign(element) {
                return true;
            }
        }
        false
    }

    pub fn get_sorted_elements(&self) -> Vec<Vec<PdfText>> {
        todo!()
    }
}

impl TableBoundary {
    pub fn assign(&mut self, element: &PdfText) -> bool {
        if element.x > self.minx
            && element.x < self.maxx
            && element.y < self.maxy
            && element.y > self.miny
        {
            self.elements.push(element.clone());
            return true;
        }

        false
    }

    pub fn get_sorted_elements(self) -> Vec<PdfText> {
        sort_transform_row(self.elements)
    }
}

pub fn units_to_elements(units: Vec<PdfUnit>) -> Vec<PdfElement> {
    let (texts, lines): (Vec<_>, Vec<_>) =
        units
            .into_iter()
            .fold((vec![], vec![]), |(mut texts, mut lines), unit| {
                match unit {
                    PdfUnit::Text(t) => texts.push(t),
                    PdfUnit::Line(l) => lines.push(l),
                }
                (texts, lines)
            });
    let tables = PdfTable::from_lines(lines);

    let mut elements: Vec<PdfElement> = Vec::new();
    let texts: Vec<PdfElement> = texts.into_iter().map(PdfElement::Text).collect();
    let tables: Vec<PdfElement> = tables.into_iter().map(PdfElement::Table).collect();
    elements.extend(texts);
    elements.extend(tables);
    elements
}

pub fn sort_transform_row(mut elements: Vec<PdfText>) -> Vec<PdfText> {
    elements.sort_by(|a, b| match b.y.partial_cmp(&a.y) {
        Some(std::cmp::Ordering::Equal) => {
            a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
        }
        Some(ordering) => ordering,
        None => std::cmp::Ordering::Equal,
    });

    elements
}

pub fn sort_transform_elements(elements: &mut Vec<PdfElement>) {
    elements.sort_by(|a, b| match b.get_y().partial_cmp(&a.get_y()) {
        Some(std::cmp::Ordering::Equal) => a
            .get_x()
            .partial_cmp(&b.get_x())
            .unwrap_or(std::cmp::Ordering::Equal),
        Some(ordering) => ordering,
        None => std::cmp::Ordering::Equal,
    });
}

pub fn elements_into_matrix(mut elements: Vec<PdfElement>) -> Vec<Vec<PdfElement>> {
    if elements.is_empty() {
        return Vec::new();
    }

    elements.sort_by(|a, b| {
        b.get_y()
            .partial_cmp(&a.get_y())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // making it into a matrix
    let mut result: Vec<Vec<PdfElement>> = Vec::new();
    let mut current_row: Vec<PdfElement> = Vec::new();
    let mut current_y = elements[0].get_y();
    for element in elements {
        // If y difference is more than 1, start a new row
        if (current_y - element.get_y()).abs() > 1.0 {
            if !current_row.is_empty() {
                result.push(current_row);
                current_row = Vec::new();
            }
            current_y = element.get_y();
        }
        current_row.push(element);
    }
    if !current_row.is_empty() {
        result.push(current_row);
    }

    // calculate gap and insert newlines between if needed.
    let row_heights: Vec<f32> = result
        .iter()
        .map(|row| {
            row.iter()
                .map(|text| text.get_y())
                .fold(f32::NEG_INFINITY, f32::max)
        })
        .collect();
    let mut gaps = Vec::new();
    for i in 0..row_heights.len() - 1 {
        let gap = row_heights[i] - row_heights[i + 1];
        gaps.push(gap);
    }
    if gaps.is_empty() {
        return result;
    }
    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let threshold_index = (gaps.len() as f32 * 0.8).floor() as usize;
    let threshold = gaps
        .get(threshold_index.min(gaps.len() - 1))
        .unwrap_or(&0.0); // top 80%

    // finally insert if the gap is bigger then expected
    let mut final_result = Vec::new();
    for i in 0..result.len() {
        final_result.push(result[i].clone());
        if i < result.len() - 1 && row_heights[i] - row_heights[i + 1] > *threshold {
            final_result.push(Vec::new());
        }
    }

    final_result
}
