use std::io::Write;

pub mod image_extended;
pub mod iterm_encoder;
pub mod kitty_encoder;
pub mod sixel_encoder;
pub mod term_misc;

pub fn inline_an_image(
    img: &Vec<u8>,
    out: impl Write,
    offset: Option<u16>,
    inline_encoder: &InlineEncoder,
) -> Result<(), Box<dyn std::error::Error>> {
    match inline_encoder {
        InlineEncoder::Kitty => kitty_encoder::encode_image(img, out, offset),
        InlineEncoder::Iterm => iterm_encoder::encode_image(img, out, offset),
        InlineEncoder::Sixel => sixel_encoder::encode_image(img, out, offset),
    }
}

pub enum InlineEncoder {
    Kitty,
    Iterm,
    Sixel,
}
impl InlineEncoder {
    pub fn auto_detect(force_kitty: bool, force_iterm: bool, force_sixel: bool) -> Self {
        if force_kitty {
            return Self::Kitty;
        }
        if force_iterm {
            return Self::Iterm;
        }
        if force_sixel {
            return Self::Sixel;
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

        Self::Iterm
    }
}
