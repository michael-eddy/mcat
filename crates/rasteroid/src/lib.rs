use std::{io::Write, process::Command};

use term_misc::EnvIdentifiers;

pub mod ascii_encoder;
pub mod image_extended;
pub mod iterm_encoder;
pub mod kitty_encoder;
pub mod sixel_encoder;
pub mod term_misc;

/// encode an image bytes into inline image using the given encoder
/// # example:
/// ```
/// use std::path::Path;
/// use rasteroid::InlineEncoder;
/// use rasteroid::inline_an_image;
/// use std::io::Write;
/// use rasteroid::term_misc::EnvIdentifiers;
///
/// let path = Path::new("image.png");
/// let bytes = match std::fs::read(path) {
///     Ok(bytes) => bytes,
///     Err(e) => return,
/// };
/// let mut stdout = std::io::stdout();
/// let env = EnvIdentifiers::new();
/// let encoder = InlineEncoder::auto_detect(true, false, false, false, &env); // force kitty as fallback
/// inline_an_image(&bytes, &mut stdout, None, None, &encoder).unwrap();
/// stdout.flush().unwrap();
/// ```
/// MENTION: it should work for Iterm Gifs too.
pub fn inline_an_image(
    img: &[u8],
    out: &mut impl Write,
    offset: Option<u16>,
    print_at: Option<(u16, u16)>,
    inline_encoder: &InlineEncoder,
) -> Result<(), Box<dyn std::error::Error>> {
    match inline_encoder {
        InlineEncoder::Kitty => kitty_encoder::encode_image(img, out, offset, print_at),
        InlineEncoder::Iterm => iterm_encoder::encode_image(img, out, offset, print_at),
        InlineEncoder::Sixel => sixel_encoder::encode_image(img, out, offset, print_at),
        InlineEncoder::Ascii => ascii_encoder::encode_image(img, out, offset, print_at),
    }
}

#[derive(Clone, Copy)]
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
        env: &EnvIdentifiers,
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

/// checks if the current terminal is a tmux terminal
/// # example:
/// ```
///  use rasteroid::is_tmux;
///
/// let env = rasteroid::term_misc::EnvIdentifiers::new();
/// let tmux = is_tmux(&env);
/// println!("Tmux: {}", tmux);
/// ```
pub fn is_tmux(env: &EnvIdentifiers) -> bool {
    env.term_contains("tmux") || env.has_key("TMUX")
}

pub fn set_tmux_passthrough(enabled: bool) {
    let status = if enabled { "on" } else { "off" };
    if Command::new("tmux")
        .args(["set", "-g", "allow-passthrough", status])
        .status()
        .is_err()
    {
        // better ignored imo
        // eprintln!(
        //     "failed enabling tmux passthrough, even though the term is tmux. please enable manually with - `tmux set -g allow-passthrough`"
        // )
    }
}

pub trait Frame {
    fn timestamp(&self) -> f32;
    fn data(&self) -> &[u8];
    fn width(&self) -> u16;
    fn height(&self) -> u16;
}
