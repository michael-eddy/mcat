use crate::term_misc::{self, EnvIdentifiers};
use std::io::Write;

/// encode an image bytes into inline image
/// should work with all formats Iterm, which include but not limited to GIF,PNG,JPEG..
/// # example:
/// ```
/// let path = Path::new("image.png");
/// let bytes = std::fs::read(path).unwrap();
/// let mut stdout = std::io::stdout();
/// encode_image(&bytes, &stdout, None).unwrap();
/// stdout.flush().unwrap();
/// ```
/// the option offset just offsets the image to the right by the amount of cells you specify
pub fn encode_image(
    img: &[u8],
    mut out: impl Write,
    offset: Option<u16>,
) -> Result<(), Box<dyn std::error::Error>> {
    let base64_encoded = term_misc::image_to_base64(img);

    let center = term_misc::offset_to_terminal(offset);
    out.write_all(center.as_ref())?;

    out.write_all(b"\x1b]1337;File=inline=1;size=")?;
    write!(out, "{}", base64_encoded.len())?;
    out.write_all(b":")?;
    out.write_all(base64_encoded.as_bytes())?;
    out.write_all(b"\x07")?;

    Ok(())
}

/// checks if the current terminal supports Iterm graphic protocol
/// # example:
/// ```
/// let env = rasteroid::term_misc::EnvIdentifiers::new();
/// let is_capable = is_iterm_capable(&env);
/// println!("Iterm: {}", is_capable);
/// ```
pub fn is_iterm_capable(env: &EnvIdentifiers) -> bool {
    env.term_contains("mintty")
        || env.term_contains("wezterm")
        || env.term_contains("iterm2")
        || env.term_contains("rio")
        || env.term_contains("warp")
        || env.has_key("KONSOLE_VERSION")
}
