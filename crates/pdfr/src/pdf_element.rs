pub enum PdfElement {
    Text(PdfText),
    Table(PdfTable),
}

#[derive(Default)]
pub struct PdfText {
    pub is_line: bool,
}

#[derive(Default)]
struct TableBoundary {
    minx: f32,
    maxx: f32,
    miny: f32,
    maxy: f32,
    text: Option<Vec<PdfText>>,
}
#[derive(Default)]
pub struct PdfTable {
    minx: f32,
    maxx: f32,
    miny: f32,
    maxy: f32,
    elements: Vec<PdfText>,
}
impl PdfTable {
    pub fn get_sorted_elements(&self) -> Vec<Vec<PdfText>> {
        todo!()
    }

    pub fn assign(&mut self, element: PdfText) {
        todo!()
    }

    fn create_boundaries(coords: &[(f32, f32)]) -> Vec<TableBoundary> {
        todo!()
    }
}
