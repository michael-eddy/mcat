#[derive(Clone, Debug)]
pub enum PdfUnit {
    Text(PdfText),
    Line(PdfLine),
}

#[derive(Clone, Copy, Debug)]
pub struct PdfLine {
    pub from: (f32, f32),
    pub to: (f32, f32),
}

#[derive(Default, Clone, Debug)]
pub struct PdfText {
    pub text: String,
    pub italic: bool,
    pub font_name: Option<String>,
    pub font_size: Option<f32>,
    pub x: f32,
    pub y: f32,
    pub underlined: bool,
    pub color: Option<String>,
}

pub fn is_fake_line(lines: &[PdfUnit]) -> bool {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    let only_lines = lines.iter().filter_map(|v| match v {
        PdfUnit::Text(_) => None,
        PdfUnit::Line(pdf_line) => Some(pdf_line),
    });

    for line in only_lines {
        for &(x, y) in &[line.from, line.to] {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }

    let width = max_x - min_x;
    let height = max_y - min_y;

    let threshold = 2.0;

    match (width < threshold, height < threshold) {
        // vertical line
        (true, false) => true,
        // horizontal line
        (false, true) => true,
        // dot
        (true, true) => false,
        // rect
        (false, false) => false,
    }
}
