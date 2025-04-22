use std::{error, io::Cursor};

use fast_image_resize::{IntoImageView, Resizer, images::Image};
use image::{DynamicImage, GenericImageView, ImageEncoder, codecs::png::PngEncoder};

use super::term_misc::{self, dim_to_px};

pub trait InlineImage {
    fn resize_plus(
        &self,
        width: Option<&str>,
        height: Option<&str>,
    ) -> Result<(Vec<u8>, u16), Box<dyn error::Error>>;
}

impl InlineImage for DynamicImage {
    fn resize_plus(
        &self,
        width: Option<&str>,
        height: Option<&str>,
    ) -> Result<(Vec<u8>, u16), Box<dyn error::Error>> {
        let (src_width, src_height) = self.dimensions();
        let width = match width {
            Some(w) => dim_to_px(w, term_misc::SizeDirection::Width)?,
            None => src_width,
        };
        let height = match height {
            Some(h) => dim_to_px(h, term_misc::SizeDirection::Height)?,
            None => src_height,
        };

        let (new_width, new_height) = calc_fit(src_width, src_height, width, height);
        let center = term_misc::center_image(new_width as u16);

        let mut dst_image = Image::new(
            new_width.into(),
            new_height.into(),
            self.pixel_type().ok_or("image is invalid")?,
        );
        let mut resizer = Resizer::new();
        resizer.resize(self, &mut dst_image, None)?;

        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        let encoder = PngEncoder::new(&mut cursor);
        encoder.write_image(
            dst_image.buffer(),
            dst_image.width(),
            dst_image.height(),
            self.color().into(),
        )?;

        return Ok((buffer, center));
    }
}

pub fn calc_fit(src_width: u32, src_height: u32, dst_width: u32, dst_height: u32) -> (u32, u32) {
    let src_ar = src_width as f32 / src_height as f32;
    let dst_ar = dst_width as f32 / dst_height as f32;

    if src_ar > dst_ar {
        // Image is wider than target: scale by width
        let scaled_height = (dst_width as f32 / src_ar).round() as u32;
        (dst_width, scaled_height)
    } else {
        // Image is taller than target: scale by height
        let scaled_width = (dst_height as f32 * src_ar).round() as u32;
        (scaled_width, dst_height)
    }
}
