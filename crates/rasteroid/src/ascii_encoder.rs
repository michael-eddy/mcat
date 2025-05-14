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

    // Get dimensions
    let w = rgba_image.width() as usize;
    let h = rgba_image.height() as usize;

    // Ensure height is even for paired processing
    let h_adjusted = if h % 2 == 1 { h - 1 } else { h };

    // Process the image in pairs of rows to create half-block characters
    for y in (0..h_adjusted).step_by(2) {
        if offset.is_some() {
            let center = term_misc::offset_to_terminal(offset);
            out.write_all(center.as_ref())?;
        }

        for x in 0..w {
            // Get the upper and lower pixels
            let upper_pixel = rgba_image.get_pixel(x as u32, y as u32);
            let lower_pixel = rgba_image.get_pixel(x as u32, (y + 1) as u32);

            let (r_upper, g_upper, b_upper, a_upper) = (
                upper_pixel[0],
                upper_pixel[1],
                upper_pixel[2],
                upper_pixel[3],
            );
            let (r_lower, g_lower, b_lower, a_lower) = (
                lower_pixel[0],
                lower_pixel[1],
                lower_pixel[2],
                lower_pixel[3],
            );

            // Skip if both pixels are transparent
            if a_upper == 0 && a_lower == 0 {
                out.write_all(b" ")?;
                continue;
            }

            // Use half-block characters to represent two pixels vertically
            // ▀ (upper half block) for top pixel, ▄ (lower half block) for bottom pixel
            // Space for transparent pixels

            if a_upper == 0 && a_lower > 0 {
                // Only lower pixel is visible
                write!(
                    out,
                    "\x1b[38;2;{};{};{}m▄\x1b[0m",
                    r_lower, g_lower, b_lower
                )?;
            } else if a_upper > 0 && a_lower == 0 {
                // Only upper pixel is visible
                write!(
                    out,
                    "\x1b[38;2;{};{};{}m▀\x1b[0m",
                    r_upper, g_upper, b_upper
                )?;
            } else {
                // Both pixels are visible - use foreground/background colors
                write!(
                    out,
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀\x1b[0m",
                    r_upper, g_upper, b_upper, r_lower, g_lower, b_lower
                )?;
            }
        }
        out.write_all(b"\n")?;
    }

    // Handle odd height images
    if h % 2 == 1 && h > 0 {
        if offset.is_some() {
            let center = term_misc::offset_to_terminal(offset);
            out.write_all(center.as_ref())?;
        }

        for x in 0..w {
            let pixel = rgba_image.get_pixel(x as u32, (h - 1) as u32);
            let (r, g, b, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);

            if a == 0 {
                out.write_all(b" ")?;
            } else {
                write!(out, "\x1b[38;2;{};{};{}m▀\x1b[0m", r, g, b)?;
            }
        }
        out.write_all(b"\n")?;
    }

    // Reset terminal color
    out.write_all(b"\x1b[0m")?;
    Ok(())
}
