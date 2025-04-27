use crate::{converter, rasteroid::term_misc::EnvIdentifiers};
use std::io::Write;

pub fn encode_image(
    img: &[u8],
    mut out: impl Write,
    offset: Option<u16>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base64_encoded = converter::image_to_base64(img);

    let center = converter::offset_to_terminal(offset);
    out.write_all(center.as_ref())?;

    out.write_all(b"\x1b]1337;File=inline=1;size=")?;
    write!(out, "{}", base64_encoded.len())?;
    out.write_all(b":")?;
    out.write_all(base64_encoded.as_bytes())?;
    out.write_all(b"\x07")?;

    Ok(())
}

pub fn is_iterm_capable(env: &EnvIdentifiers) -> bool {
    env.term_contains("mintty")
        || env.term_contains("wezterm")
        || env.term_contains("iterm2")
        || env.term_contains("rio")
        || env.term_contains("warp")
        || env.has_key("KONSOLE_VERSION")
}
