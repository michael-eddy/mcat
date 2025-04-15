use std::path::Path;

use crate::term_misc;

struct Imager {
    buffer: Vec<u8>,
}

pub enum Encoder {
    Kitty,
    Iterm,
    Sixel,
}

impl Encoder {
    pub fn auto_detect() -> Self {
        todo!()
    }
}

impl Imager {
    /// must be bytes of a png
    pub fn from_raw(bytes: Vec<u8>) -> Self {
        Imager { buffer: bytes }
    }

    /// opens local file for a image (can be other then png)
    pub fn open<P: AsRef<Path>>(p: P) -> Self {
        todo!()
    }

    pub fn is_image<P: AsRef<Path>>(p: P) -> bool {
        todo!()
    }

    pub fn inline(&self, encoder: &Encoder) -> Vec<u8> {
        todo!()
    }

    pub fn resize(&self, fit: bool, dim: term_misc::Size) -> Self {
        todo!()
    }
}

fn encode_kitty(bytes: Vec<u8>) {
    todo!()
}
fn encode_iterm(bytes: Vec<u8>) {
    todo!()
}
fn encode_sixel(bytes: Vec<u8>) {
    todo!()
}

fn is_kitty_capable() -> bool {
    todo!()
}
fn is_iterm_capable() -> bool {
    todo!()
}
fn is_sixel_capable() -> bool {
    todo!()
}
