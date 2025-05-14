use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

use crate::{image_extended::InlineImage, kitty_encoder::Frame, term_misc};

/// Renders an image as colored ASCII in the terminal using upper/lower half-blocks.
///
/// This function converts an RGBA image into a terminal-friendly representation
/// using ANSI escape codes and colored Unicode half-block characters. It filters
/// out low-luminance, low-opacity pixels to suppress thin outlines and noise,
/// keeping only visually meaningful parts of the image.
///
/// # Arguments
/// - `img`: Image byte slice (any format supported by `image` crate, e.g., PNG, JPEG)
/// - `out`: A writer to send output to (e.g., `std::io::stdout`)
/// - `offset`: Optional horizontal offset in terminal columns (used for centering)
///
/// # Example
/// ```
/// use std::path::Path;
/// use std::io::{self, Write};
/// use rasteroid::ascii_encoder::encode_image;
///
/// let path = Path::new("image.png");
/// let bytes = match std::fs::read(path) {
///     Ok(bytes) => bytes,
///     Err(_) => return,
/// };
///
/// let mut stdout = io::stdout();
/// encode_image(&bytes, &mut stdout, Some(80)).unwrap();
/// stdout.flush().unwrap();
/// ```
pub fn encode_image(
    img: &[u8],
    mut out: impl Write,
    offset: Option<u16>,
) -> Result<(), Box<dyn std::error::Error>> {
    let image = image::load_from_memory(img)?;
    let rgba_image = image.to_rgba8();

    let w = rgba_image.width() as usize;
    let h = rgba_image.height() as usize;
    let h_adjusted = if h % 2 == 1 { h - 1 } else { h };

    // Luminance threshold: tweak this to suppress small sparkles
    const LUM_THRESHOLD: f32 = 35.0;

    for y in (0..h_adjusted).step_by(2) {
        if let Some(off) = offset {
            let center = term_misc::offset_to_terminal(Some(off));
            out.write_all(center.as_ref())?;
        }

        for x in 0..w {
            let upper = rgba_image.get_pixel(x as u32, y as u32);
            let lower = rgba_image.get_pixel(x as u32, (y + 1) as u32);

            let (ru, gu, bu, au) = (upper[0], upper[1], upper[2], upper[3]);
            let (rl, gl, bl, al) = (lower[0], lower[1], lower[2], lower[3]);

            const MIN_VISUAL_WEIGHT: f32 = 25.0; // tweak for strictness

            let upper_w = visual_weight(ru, gu, bu, au);
            let lower_w = visual_weight(rl, gl, bl, al);

            let upper_visible = upper_w > MIN_VISUAL_WEIGHT;
            let lower_visible = lower_w > MIN_VISUAL_WEIGHT;

            match (upper_visible, lower_visible) {
                (false, false) => {
                    out.write_all(b" ")?;
                }
                (true, false) => {
                    write!(out, "\x1b[38;2;{};{};{}m▀\x1b[0m", ru, gu, bu)?;
                }
                (false, true) => {
                    write!(out, "\x1b[38;2;{};{};{}m▄\x1b[0m", rl, gl, bl)?;
                }
                (true, true) => {
                    // Use dual-color upper/lower block
                    write!(
                        out,
                        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀\x1b[0m",
                        ru, gu, bu, rl, gl, bl
                    )?;
                }
            }
        }

        out.write_all(b"\n")?;
    }

    if h % 2 == 1 {
        if let Some(off) = offset {
            let center = term_misc::offset_to_terminal(Some(off));
            out.write_all(center.as_ref())?;
        }

        for x in 0..w {
            let p = rgba_image.get_pixel(x as u32, (h - 1) as u32);
            let (r, g, b, a) = (p[0], p[1], p[2], p[3]);
            let lum = luminance(r, g, b);

            if a == 0 || lum < LUM_THRESHOLD {
                out.write_all(b" ")?;
            } else {
                write!(out, "\x1b[38;2;{};{};{}m▀\x1b[0m", r, g, b)?;
            }
        }

        out.write_all(b"\n")?;
    }

    out.write_all(b"\x1b[0m")?;
    Ok(())
}

fn luminance(r: u8, g: u8, b: u8) -> f32 {
    0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32
}

fn visual_weight(r: u8, g: u8, b: u8, a: u8) -> f32 {
    if a == 0 {
        0.0
    } else {
        luminance(r, g, b) * (a as f32 / 255.0)
    }
}

pub fn encode_frames(
    frames: &mut dyn Iterator<Item = impl Frame>,
    mut out: impl Write,
    center: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut last_timestamp = None;

    for frame in frames {
        let rgb_image = image::RgbImage::from_raw(
            frame.width().into(),
            frame.height().into(),
            frame.data().to_vec(),
        )
        .ok_or("failed to load img1")?;
        let img = image::DynamicImage::ImageRgb8(rgb_image);
        let (img, offset) = img.resize_plus(Some("80%"), Some("80%"), true)?;
        // Optional: calculate delay
        let target_delay = match (frame.timestamp(), last_timestamp) {
            (ts, Some(last)) if ts > last => Duration::from_secs_f32(ts - last),
            _ => Duration::from_millis(33), // default ~30fps
        };
        last_timestamp = Some(frame.timestamp());

        // Clear terminal and move cursor to top-left
        write!(out, "\x1b[2J\x1b[H")?;
        out.flush()?; // Ensure clear happens before frame draw

        // Render frame
        encode_image(&img, &mut out, if center { Some(offset) } else { None })?;

        out.flush()?;

        // Sync timing
        std::thread::sleep(target_delay);
    }

    Ok(())
}
