use std::io::Write;

pub mod ascii_encoder;
pub mod image_extended;
pub mod iterm_encoder;
pub mod kitty_encoder;
pub mod sixel_encoder;
pub mod term_misc;

#[macro_use]
extern crate lazy_static;

/// encode an image bytes into inline image using the given encoder
/// # example:
/// ```
/// use std::path::Path;
/// use rasteroid::InlineEncoder;
/// use rasteroid::inline_an_image;
/// use std::io::Write;
///
/// let path = Path::new("image.png");
/// let bytes = match std::fs::read(path) {
///     Ok(bytes) => bytes,
///     Err(e) => return,
/// };
/// let mut stdout = std::io::stdout();
/// let encoder = InlineEncoder::auto_detect(true, false, false, false); // force kitty as fallback
/// inline_an_image(&bytes, &stdout, None, &encoder).unwrap();
/// stdout.flush().unwrap();
/// ```
/// MENTION: it should work for Iterm Gifs too.
pub fn inline_an_image(
    img: &[u8],
    out: impl Write,
    offset: Option<u16>,
    inline_encoder: &InlineEncoder,
) -> Result<(), Box<dyn std::error::Error>> {
    match inline_encoder {
        InlineEncoder::Kitty => kitty_encoder::encode_image(img, out, offset),
        InlineEncoder::Iterm => iterm_encoder::encode_image(img, out, offset),
        InlineEncoder::Sixel => sixel_encoder::encode_image(img, out, offset),
        InlineEncoder::Ascii => ascii_encoder::encode_image(img, out, offset),
    }
}

pub enum InlineEncoder {
    Kitty,
    Iterm,
    Sixel,
    Ascii,
}
impl InlineEncoder {
    /// auto detect which Encoder works for the current terminal
    /// allows forcing certain encoders (sort of a fallback).
    pub fn auto_detect(
        force_kitty: bool,
        force_iterm: bool,
        force_sixel: bool,
        force_ascii: bool,
    ) -> Self {
        if force_kitty {
            return Self::Kitty;
        }
        if force_iterm {
            return Self::Iterm;
        }
        if force_sixel {
            return Self::Sixel;
        }
        if force_ascii {
            return Self::Ascii;
        }

        let env = term_misc::EnvIdentifiers::new();
        if kitty_encoder::is_kitty_capable(&env) {
            return Self::Kitty;
        }
        if iterm_encoder::is_iterm_capable(&env) {
            return Self::Iterm;
        }
        if sixel_encoder::is_sixel_capable(&env) {
            return Self::Sixel;
        }

        Self::Ascii
    }
}

pub trait Frame {
    fn timestamp(&self) -> f32;
    fn data(&self) -> &[u8];
    fn width(&self) -> u16;
    fn height(&self) -> u16;
}
