use std::io::Write;

use crate::term_misc;

///
/// # Example:
/// ```
/// use std::path::Path;
/// use rasteroid::ascii_encoder::{encode_image_ascii, AsciiStyle};
///
/// let path = Path::new("image.png");
/// let bytes = match std::fs::read(path) {
///     Ok(bytes) => bytes,
///     Err(e) => return,
/// };
/// let mut stdout = std::io::stdout();
/// encode_image_ascii(&bytes, &mut stdout, AsciiStyle::LowerHalfBlocks, Some(80), None).unwrap();
/// stdout.flush().unwrap();
/// ```
pub fn encode_image_ascii(
    img: &[u8],
    mut out: impl Write,
    offset: Option<u16>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the image from bytes using image crate
    let image = image::load_from_memory(img)?;

    // Convert image to RGBA for color processing
    let rgba_image = image.to_rgba8();

    // Write the center offset if specified
    if let Some(offset_value) = offset {
        let center = term_misc::offset_to_terminal(Some(offset_value));
        out.write_all(center.as_ref())?;
    }

    // Process the image and generate ASCII art
    let w = rgba_image.width() as usize;
    let h = rgba_image.height() as usize;

    // Select character set based on style
    let chars = vec![' ', '░', '▒', '▓', '█'];

    for y in 0..h {
        if offset.is_some() {
            let center = term_misc::offset_to_terminal(offset);
            out.write_all(center.as_ref())?;
        }

        for x in 0..w {
            let pixel = rgba_image.get_pixel(x as u32, y as u32);
            let (r, g, b, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);

            // Skip transparent pixels
            if a == 0 {
                out.write_all(b" ")?;
                continue;
            }

            // Calculate grayscale intensity
            let intensity = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as usize;
            let index = intensity * (chars.len() - 1) / 255;

            write!(out, "\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, chars[index])?;
        }
        out.write_all(b"\n")?;
    }

    // Reset terminal color
    out.write_all(b"\x1b[0m")?;

    Ok(())
}
