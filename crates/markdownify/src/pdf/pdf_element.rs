#[derive(Clone)]
pub enum PdfElement {
    Text(PdfText),
    Table(PdfTable),
}
pub enum PdfUnit {
    Text(PdfText),
    Line(PdfLine),
}

#[derive(Clone)]
pub struct PdfLine {
    pub from: (f32, f32),
    pub to: (f32, f32),
}

#[derive(Default, Clone)]
pub struct PdfText {
    pub text: String,
    pub italic: bool,
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
    fn lines_touch(a: &PdfLine, b: &PdfLine) -> bool {
        const LINE_PROXIMITY: f32 = 0.3;
        fn point_dist(p1: (f32, f32), p2: (f32, f32)) -> f32 {
            ((p1.0 - p2.0).powi(2) + (p1.1 - p2.1).powi(2)).sqrt()
        }

        [a.from, a.to].iter().any(|&p1| {
            [b.from, b.to]
                .iter()
                .any(|&p2| point_dist(p1, p2) < LINE_PROXIMITY)
        })
    }

    fn cluster_lines(lines: &[PdfLine]) -> Vec<Vec<PdfLine>> {
        let mut visited = vec![false; lines.len()];
        let mut clusters = vec![];

        for i in 0..lines.len() {
            if visited[i] {
                continue;
            }

            let mut stack = vec![i];
            let mut cluster = vec![];

            while let Some(idx) = stack.pop() {
                if visited[idx] {
                    continue;
                }

                visited[idx] = true;
                cluster.push(lines[idx].clone());

                for j in 0..lines.len() {
                    if !visited[j] && PdfTable::lines_touch(&lines[idx], &lines[j]) {
                        stack.push(j);
                    }
                }
            }

            if cluster.len() >= 4 {
                clusters.push(cluster);
            }
        }

        clusters
    }

    pub fn from_lines(lines: Vec<PdfLine>) -> Vec<PdfTable> {
        let clusters = PdfTable::cluster_lines(&lines);

        let mut tables = vec![];

        for cluster in clusters {
            let mut hlines = vec![];
            let mut vlines = vec![];

            for line in &cluster {
                if (line.from.1 - line.to.1).abs() < 1.0 {
                    hlines.push(line.clone());
                } else if (line.from.0 - line.to.0).abs() < 1.0 {
                    vlines.push(line.clone());
                }
            }

            if hlines.len() < 2 || vlines.len() < 2 {
                continue; // not enough to form a table
            }

            hlines.sort_by(|a, b| a.from.1.partial_cmp(&b.from.1).unwrap());
            vlines.sort_by(|a, b| a.from.0.partial_cmp(&b.from.0).unwrap());

            let mut boundaries = vec![];

            for y_pair in hlines.windows(2) {
                for x_pair in vlines.windows(2) {
                    let top = y_pair[0].from.1;
                    let bottom = y_pair[1].from.1;
                    let left = x_pair[0].from.0;
                    let right = x_pair[1].from.0;

                    if top != bottom && left != right {
                        boundaries.push(TableBoundary {
                            minx: left.min(right),
                            maxx: left.max(right),
                            miny: bottom.min(top),
                            maxy: bottom.max(top),
                            elements: vec![],
                        });
                    }
                }
            }

            if boundaries.is_empty() {
                continue;
            }

            let (sum_x, sum_y, count) = boundaries.iter().fold((0.0, 0.0, 0), |(sx, sy, c), b| {
                let cx = (b.minx + b.maxx) / 2.0;
                let cy = (b.miny + b.maxy) / 2.0;
                (sx + cx, sy + cy, c + 1)
            });

            tables.push(PdfTable {
                boundaries,
                x: sum_x / count as f32,
                y: sum_y / count as f32,
            });
        }

        tables
    }

    pub fn assign(&mut self, element: &PdfText) -> bool {
        for boundary in self.boundaries.iter_mut() {
            if boundary.assign(element) {
                return true;
            }
        }
        false
    }

    pub fn get_sorted_elements(&mut self) -> Vec<Vec<Vec<PdfText>>> {
        self.boundaries.sort_by(|a, b| {
            b.miny
                .partial_cmp(&a.miny)
                .unwrap()
                .then(a.minx.partial_cmp(&b.minx).unwrap())
        });
        const Y_THRESHOLD: f32 = 1.0;

        let mut rows: Vec<Vec<TableBoundary>> = Vec::new();
        for boundary in self.boundaries.iter() {
            let mut matched = false;
            for row in &mut rows {
                // Compare with the *row's reference y*, e.g. the first cell's miny
                if (row[0].miny - boundary.miny).abs() < Y_THRESHOLD {
                    row.push(boundary.clone());
                    matched = true;
                    break;
                }
            }
            if !matched {
                rows.push(vec![boundary.clone()]);
            }
        }

        for row in &mut rows {
            row.sort_by(|a, b| a.minx.partial_cmp(&b.minx).unwrap());
        }

        let result: Vec<Vec<Vec<PdfText>>> = rows
            .iter_mut()
            .map(|row| {
                row.iter_mut()
                    .map(|cell| cell.clone().get_sorted_elements())
                    .collect()
            })
            .collect();

        result
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
    let mut tables = PdfTable::from_lines(lines);
    // assign to the tables
    let texts = if !tables.is_empty() {
        let mut new_texts = Vec::new();
        for text in texts.iter() {
            for table in tables.iter_mut() {
                if !table.assign(text) {
                    new_texts.push(text.clone());
                }
            }
        }
        new_texts
    } else {
        texts
    };

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

    // Group into rows
    let mut result: Vec<Vec<PdfElement>> = Vec::new();
    let mut current_row: Vec<PdfElement> = Vec::new();
    let mut current_y = elements[0].get_y();
    for element in elements {
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

    // Compute row heights
    let row_heights: Vec<f32> = result
        .iter()
        .map(|row| {
            row.iter()
                .map(|text| text.get_y())
                .fold(f32::NEG_INFINITY, f32::max)
        })
        .collect();

    // Compute gaps
    let mut gaps = Vec::new();
    for i in 0..row_heights.len() - 1 {
        let gap = row_heights[i] - row_heights[i + 1];
        gaps.push(gap);
    }

    // Insert spacer *after* row[i+1] if gap[i+1] > gap[i] * 1.2
    let mut final_result = Vec::new();
    for i in 0..result.len() {
        final_result.push(result[i].clone());

        if i > 0 && i < gaps.len() {
            let prev_gap = gaps[i - 1];
            let curr_gap = gaps[i];

            if curr_gap > prev_gap * 1.3 {
                final_result.push(Vec::new()); // spacer
            }
        }
    }

    final_result
}
