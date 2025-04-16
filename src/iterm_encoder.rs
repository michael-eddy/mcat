use crate::{converter, term_misc::EnvIdentifiers};
use std::io::Write;

pub fn encode_image(
    img: &Vec<u8>,
    offset: Option<u16>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let base64_encoded = converter::image_to_base64(img);

    let mut buffer: Vec<u8> = Vec::with_capacity(base64_encoded.len() + 50);

    let center = converter::offset_to_terminal(offset);
    buffer.extend_from_slice(center.as_ref());

    buffer.extend_from_slice(b"\x1b]1337;File=inline=1;size=");
    write!(buffer, "{}", base64_encoded.len())?;
    buffer.push(b':');
    buffer.extend_from_slice(base64_encoded.as_bytes());
    buffer.push(b'\x07');

    Ok(buffer)
}

pub fn is_iterm_capable(env: &EnvIdentifiers) -> bool {
    env.term_contains("mintty")
        || env.term_contains("wezterm")
        || env.term_contains("iterm2")
        || env.term_contains("rio")
        || env.term_contains("warp")
        || env.has_key("KONSOLE_VERSION")
}
