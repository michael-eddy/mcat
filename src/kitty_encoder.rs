use std::{cmp::min, collections::HashMap, error::Error, io::Write};

use image::DynamicImage;

use crate::{
    image_extended::InlineImage,
    term_misc::{self, EnvIdentifiers},
};

fn chunk_base64(
    base64: &str,
    buffer: &mut Vec<u8>,
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

        buffer.extend_from_slice(b"\x1b_G");
        buffer.extend_from_slice(opts);
        write!(buffer, "m={};{}", more_chunks, chunk_data)?;
        buffer.extend_from_slice(b"\x1b\\");

        start = end;
    }

    Ok(())
}

pub fn encode_image(img: &DynamicImage) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let center_string = term_misc::center_image(img.width() as u16);
    let base64 = img.encode_base64()?;
    let mut buffer = Vec::with_capacity(base64.len() + 10);
    buffer.extend_from_slice(center_string.as_bytes());
    chunk_base64(
        &base64,
        &mut buffer,
        4096,
        HashMap::from([
            ("f".to_string(), "100".to_string()),
            ("a".to_string(), "T".to_string()),
        ]),
        HashMap::new(),
    )?;

    Ok(buffer)
}

pub fn is_kitty_capable(env: &EnvIdentifiers) -> bool {
    env.has_key("KITTY_WINDOW_ID") || env.term_contains("kitty") || env.term_contains("ghostty")
}
