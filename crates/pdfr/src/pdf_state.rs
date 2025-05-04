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
pub struct PdfState {
    tm: Matrix3x3,
    tlm: Matrix3x3,
    leading: f32,
    ctm: Matrix3x3,
}

impl PdfState {
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
