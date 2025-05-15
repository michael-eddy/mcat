use std::{error, io::Cursor};

use fast_image_resize::{IntoImageView, Resizer, images::Image};
use image::{
    DynamicImage, GenericImageView, ImageEncoder, codecs::png::PngEncoder, imageops::crop_imm,
};

use crate::term_misc::dim_to_cells;

use super::term_misc::{self, dim_to_px};

pub trait InlineImage {
    /// fast image resizer, that takes logic units.
    /// # example:
    /// ```
    /// use std::path::Path;
    /// use rasteroid::image_extended::InlineImage;
    ///
    /// let path = Path::new("image.png");
    /// let buf = match std::fs::read(path) {
    ///     Ok(buf) => buf,
    ///     Err(e) => return,
    /// };
    /// let dyn_img = image::load_from_memory(&buf).unwrap();
    /// let (img_data, offset) = dyn_img.resize_plus(Some("80%"),Some("200c"), false).unwrap();
    /// ```
    /// * the offset is for centering the image
    /// * it accepts either `%` (percentage) / `c` (cells) / just a number
    /// * when resize for ascii is true it resizes to cells, if not it resizes to pixels
    fn resize_plus(
        &self,
        width: Option<&str>,
        height: Option<&str>,
        resize_for_ascii: bool,
    ) -> Result<(Vec<u8>, u16), Box<dyn error::Error>>;
    /// zoom into the image, and move around
    /// # example:
    /// ```
    /// use std::path::Path;
    /// use rasteroid::image_extended::InlineImage;
    ///
    /// let path = Path::new("image.png");
    /// let buf = match std::fs::read(path) {
    ///     Ok(buf) => buf,
    ///     Err(e) => return,
    /// };
    /// let dyn_img = image::load_from_memory(&buf).unwrap();
    /// let dyn_img = dyn_img.zoom_pan(Some(5), Some(3), Some(2));
    /// ```
    /// the above zooms 50%, moves right 15% and down 10%
    fn zoom_pan(self, zoom: Option<usize>, x: Option<i32>, y: Option<i32>) -> Self;
}

impl InlineImage for DynamicImage {
    fn resize_plus(
        &self,
        width: Option<&str>,
        height: Option<&str>,
        resize_for_ascii: bool,
    ) -> Result<(Vec<u8>, u16), Box<dyn error::Error>> {
        let (src_width, src_height) = self.dimensions();
        let width = match width {
            Some(w) => match resize_for_ascii {
                true => dim_to_cells(w, term_misc::SizeDirection::Width)?,
                false => dim_to_px(w, term_misc::SizeDirection::Width)?,
            },
            None => src_width,
        };
        let height = match height {
            Some(h) => match resize_for_ascii {
                true => dim_to_cells(h, term_misc::SizeDirection::Height)? * 2,
                false => dim_to_px(h, term_misc::SizeDirection::Height)?,
            },
            None => src_height,
        };

        let (new_width, new_height) = calc_fit(src_width, src_height, width, height);
        let center = term_misc::center_image(new_width as u16, resize_for_ascii);

        let mut dst_image = Image::new(
            new_width,
            new_height,
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

        Ok((buffer, center))
    }

    fn zoom_pan(self, zoom: Option<usize>, x: Option<i32>, y: Option<i32>) -> Self {
        if zoom.is_none() && x.is_none() && y.is_none() {
            return self;
        }

        let (width, height) = self.dimensions();
        let (width, height) = (width as f32, height as f32);
        let zoom = zoom.unwrap_or(1) as f32;
        let pan_x = x.unwrap_or(0) as f32;
        let pan_y = y.unwrap_or(0) as f32;

        // Calculate the zoom factor
        let zoom_factor = 1.0 - 0.1 * zoom;
        let zoomed_width = (width * zoom_factor).clamp(1.0, width);
        let zoomed_height = (height * zoom_factor).clamp(1.0, height);

        // Calculate pan offsets (5% per pan unit)
        let pan_offset_x = 0.05 * width * pan_x;
        let pan_offset_y = 0.05 * height * pan_y;

        let crop_x = ((width - zoomed_width) / 2.0 + pan_offset_x).clamp(0.0, width - zoomed_width);
        let crop_y =
            ((height - zoomed_height) / 2.0 + pan_offset_y).clamp(0.0, height - zoomed_height);

        let cropped = crop_imm(
            &self,
            crop_x.round() as u32,
            crop_y.round() as u32,
            zoomed_width.round() as u32,
            zoomed_height.round() as u32,
        );

        DynamicImage::ImageRgba8(cropped.to_image())
    }
}

/// calculates the dimensions of an image into fit bounding box
/// # example:
/// ```
/// use rasteroid::image_extended::calc_fit;
///
/// let (new_width, new_height) = calc_fit(1920, 1080, 800, 400);
/// ```
/// the above will return dimensions close to 800x400 that maintain the aspect ratio of 1920x1080
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
