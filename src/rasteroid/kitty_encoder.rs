use std::{cmp::min, collections::HashMap, error::Error, io::Write, sync::atomic::Ordering};

use base64::{Engine, engine::general_purpose};
use ffmpeg_sidecar::event::OutputVideoFrame;
use flate2::{Compression, write::ZlibEncoder};

use crate::{
    converter,
    rasteroid::term_misc::{self, EnvIdentifiers},
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

pub fn encode_image(
    img: &[u8],
    mut out: impl Write,
    offset: Option<u16>,
) -> Result<(), Box<dyn std::error::Error>> {
    let center_string = converter::offset_to_terminal(offset);
    let base64 = converter::image_to_base64(img);

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
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;

    let base64 = general_purpose::STANDARD.encode(compressed);
    chunk_base64(&base64, out, 4096, first_opts, sub_opts)?;

    Ok(())
}

pub fn encode_frames(
    frames: Box<dyn Iterator<Item = OutputVideoFrame>>,
    out: &mut impl Write,
    id: u32,
    center: bool,
) -> Result<(), Box<dyn Error>> {
    let mut frames = frames.into_iter();

    // getting the first frame
    let first = frames.next().ok_or("video doesn't contain any frames")?;
    let offset = term_misc::center_image(first.width as u16);
    if center {
        let center = converter::offset_to_terminal(Some(offset));
        out.write_all(center.as_bytes())?;
    }
    let mut pre_timestamp = 0.0;

    // adding the root image
    let i = id.to_string();
    let s = first.width.to_string();
    let v = first.height.to_string();
    let f = "24".to_string();
    let o = "z".to_string();
    let q = "2".to_string();
    process_frame(
        &first.data,
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
    let z = 100;
    write!(out, "\x1b_Ga=a,s=2,v=1,r=1,I={},z={}\x1b\\", id, z)?;

    let shutdown = term_misc::setup_signal_handler();

    for (c, frame) in frames.enumerate() {
        if shutdown.load(Ordering::SeqCst) {
            break; // clean exit
        }
        let s = frame.width.to_string();
        let v = frame.height.to_string();
        let i = id.to_string();
        let f = "24".to_string();
        let o = "z".to_string();
        let z = ((frame.timestamp - pre_timestamp) * 1000.0) as u32;
        pre_timestamp = frame.timestamp;

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

        process_frame(&frame.data, out, first_opts, sub_opts)?;
    }

    write!(out, "\x1b_Ga=a,s=3,v=1,r=1,I={},z={}\x1b\\", id, z)?;
    Ok(())
}

pub fn is_kitty_capable(env: &EnvIdentifiers) -> bool {
    env.has_key("KITTY_WINDOW_ID") || env.term_contains("kitty") || env.term_contains("ghostty")
}
