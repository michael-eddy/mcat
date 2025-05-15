use std::{cmp::min, collections::HashMap, error::Error, io::Write, sync::atomic::Ordering};

use base64::{Engine, engine::general_purpose};
use flate2::{Compression, write::ZlibEncoder};

use crate::{
    Frame,
    term_misc::{self, EnvIdentifiers, image_to_base64, offset_to_terminal},
};

fn chunk_base64(
    base64: &str,
    mut out: impl Write,
    size: usize,
    first_opts: HashMap<String, String>,
    sub_opts: HashMap<String, String>,
) -> Result<(), std::io::Error> {
    // first block
    let mut first_opts_string = Vec::with_capacity(first_opts.len() * 8);
    for (key, value) in first_opts {
        if !first_opts_string.is_empty() {
            first_opts_string.push(b',');
        }
        write!(first_opts_string, "{}={}", key, value)?;
    }
    if !first_opts_string.is_empty() {
        first_opts_string.push(b',');
    }

    // all other blocks
    let mut sub_opts_string = Vec::with_capacity(sub_opts.len() * 8);
    for (key, value) in sub_opts {
        if !sub_opts_string.is_empty() {
            sub_opts_string.push(b',');
        }
        write!(sub_opts_string, "{}={}", key, value)?;
    }
    if !sub_opts_string.is_empty() {
        sub_opts_string.push(b',');
    }

    let total_bytes = base64.len();
    let mut start = 0;

    while start < total_bytes {
        let end = min(start + size, total_bytes);
        let chunk_data = &base64[start..end];
        let more_chunks = (end != total_bytes) as u8;

        let opts = if start == 0 {
            &first_opts_string
        } else {
            &sub_opts_string
        };

        out.write_all(b"\x1b_G")?;
        out.write_all(opts)?;
        write!(out, "m={};{}", more_chunks, chunk_data)?;
        out.write_all(b"\x1b\\")?;

        start = end;
    }

    Ok(())
}

/// encode an image bytes into inline image
/// should work with only png.
/// you can use crates like `image` to convert images into png
/// # example:
/// ```
/// use std::path::Path;
/// use rasteroid::kitty_encoder::encode_image;
/// use std::io::Write;
///
/// let path = Path::new("image.png");
/// let bytes = match std::fs::read(path) {
///     Ok(bytes) => bytes,
///     Err(e) => return,
/// };
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
    let center_string = offset_to_terminal(offset);
    let base64 = image_to_base64(img);

    out.write_all(center_string.as_bytes())?;
    chunk_base64(
        &base64,
        out,
        4096,
        HashMap::from([
            ("f".to_string(), "100".to_string()),
            ("a".to_string(), "T".to_string()),
        ]),
        HashMap::new(),
    )?;

    Ok(())
}

fn process_frame(
    data: &[u8],
    out: &mut impl Write,
    first_opts: HashMap<String, String>,
    sub_opts: HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;

    let base64 = general_purpose::STANDARD.encode(compressed);
    chunk_base64(&base64, out, 4096, first_opts, sub_opts)?;

    Ok(())
}

/// encode a video into inline video.
/// recommended to use in conjunction with video parsing library
/// # example:
/// first make sure you can supply a iter of Frames (using ffmpeg-sidecar here)
/// ```
/// use ffmpeg_sidecar::command::FfmpegCommand;
/// use rasteroid::Frame;
/// use ffmpeg_sidecar::event::OutputVideoFrame;
/// use rasteroid::kitty_encoder::encode_frames;
/// use rasteroid::image_extended::calc_fit;
/// use ffmpeg_sidecar::event::FfmpegEvent;
/// use rasteroid::image_extended::InlineImage;
///
/// pub struct KittyFrames {
///     frame: OutputVideoFrame,
///     img: Vec<u8>,
/// }
/// impl Frame for KittyFrames {
///     fn timestamp(&self) -> f32 {
///         self.frame.timestamp
///     }
///     // needs to be something image crate can load.
///     fn data(&self) -> &[u8] {
///         &self.img
///     }
/// }
/// // next get the frames (taken from ffmpeg-sidecar)
///
/// let mut out = std::io::stdout();
/// let iter = match FfmpegCommand::new() // <- Builder API like `std::process::Command`
///   .testsrc()  // <- Discoverable aliases for FFmpeg args
///   .rawvideo() // <- Convenient argument presets
///   .spawn()    // <- Ordinary `std::process::Child`
///    {
///        Ok(res) => res,
///        Err(e) => return,
/// }.iter().unwrap();   // <- Blocking iterator over logs and output
/// // now convert to compatible frames
/// let width = Some("80%");
/// let height = Some("80%");
/// let center = true;
/// let mut kitty_frames = iter.filter_map(|event| {
///     if let FfmpegEvent::OutputFrame(f) = event {
///        let rgb_image = image::RgbImage::from_raw(f.width, f.height, f.data.clone())
///            .unwrap_or_default();
///        let img = image::DynamicImage::ImageRgb8(rgb_image);
///        let (img, _) = img.resize_plus(width, height, false).unwrap_or_default();
///        Some(KittyFrames { img, frame: f })
///     } else {
///        None
///     }
/// });
/// let id = rand::random::<u32>();
/// encode_frames(&mut kitty_frames, &mut out, id, center);
/// ```
pub fn encode_frames(
    frames: &mut dyn Iterator<Item = impl Frame>,
    out: &mut impl Write,
    id: u32,
    center: bool,
) -> Result<(), Box<dyn Error>> {
    let shutdown = term_misc::setup_signal_handler();
    let mut pre_timestamp = 0.0;
    let z = 100;

    for (c, frame) in frames.enumerate() {
        if c == 0 {
            let img = image::load_from_memory(frame.data())?;
            let offset = term_misc::center_image(img.width() as u16, false);
            if center {
                let center = offset_to_terminal(Some(offset));
                out.write_all(center.as_bytes())?;
            }

            // adding the root image
            let i = id.to_string();
            let s = img.width().to_string();
            let v = img.height().to_string();
            let f = "24".to_string();
            let o = "z".to_string();
            let q = "2".to_string();
            process_frame(
                &img.to_rgb8(),
                out,
                HashMap::from([
                    ("a".to_string(), "T".to_string()),
                    ("f".to_string(), f),
                    ("o".to_string(), o),
                    ("I".to_string(), i),
                    ("s".to_string(), s),
                    ("v".to_string(), v),
                    ("q".to_string(), q),
                ]),
                HashMap::new(),
            )?;

            // starting the animation
            write!(out, "\x1b_Ga=a,s=2,v=1,r=1,I={},z={}\x1b\\", id, z)?;
            continue;
        }

        let new_img = image::load_from_memory(frame.data())?;
        if shutdown.load(Ordering::SeqCst) {
            break; // clean exit
        }
        let s = new_img.width().to_string();
        let v = new_img.height().to_string();
        let i = id.to_string();
        let f = "24".to_string();
        let o = "z".to_string();
        let z = ((frame.timestamp() - pre_timestamp) * 1000.0) as u32;
        pre_timestamp = frame.timestamp();

        let first_opts = HashMap::from([
            ("a".to_string(), "f".to_string()),
            ("f".to_string(), f),
            ("o".to_string(), o),
            ("I".to_string(), i),
            ("c".to_string(), c.to_string()),
            ("s".to_string(), s),
            ("v".to_string(), v),
            ("z".to_string(), z.to_string()),
        ]);
        let sub_opts = HashMap::from([("a".to_string(), "f".to_string())]);

        process_frame(&new_img.to_rgb8(), out, first_opts, sub_opts)?;
    }

    write!(out, "\x1b_Ga=a,s=3,v=1,r=1,I={},z={}\x1b\\", id, z)?;
    Ok(())
}

/// checks if the current terminal supports Kitty's graphic protocol
/// # example:
/// ```
///  use rasteroid::kitty_encoder::is_kitty_capable;
///
/// let env = rasteroid::term_misc::EnvIdentifiers::new();
/// let is_capable = is_kitty_capable(&env);
/// println!("Kitty: {}", is_capable);
/// ```
pub fn is_kitty_capable(env: &EnvIdentifiers) -> bool {
    env.has_key("KITTY_WINDOW_ID") || env.term_contains("kitty") || env.term_contains("ghostty")
}
