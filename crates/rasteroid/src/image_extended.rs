use std::{error, io::Cursor};

use fast_image_resize::{IntoImageView, Resizer, images::Image};
use image::{DynamicImage, GenericImage, GenericImageView, ImageEncoder, codecs::png::PngEncoder};

use crate::term_misc::{SizeDirection, dim_to_cells};

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
    /// let (img_data, offset, width, height) = dyn_img.resize_plus(Some("80%"),Some("200c"), false, false).unwrap();
    /// ```
    /// * the offset is for centering the image
    /// * it accepts either `%` (percentage) / `c` (cells) / just a number
    /// * when resize for ascii is true it resizes to cells, if not it resizes to pixels
    /// * pad adds empty pixels so the image will be the exact dimensions specified, while still
    /// maintaining aspect ratio
    fn resize_plus(
        &self,
        width: Option<&str>,
        height: Option<&str>,
        resize_for_ascii: bool,
        pad: bool,
    ) -> Result<(Vec<u8>, u16, u32, u32), Box<dyn error::Error>>;
}

impl InlineImage for DynamicImage {
    fn resize_plus(
        &self,
        width: Option<&str>,
        height: Option<&str>,
        resize_for_ascii: bool,
        pad: bool,
    ) -> Result<(Vec<u8>, u16, u32, u32), Box<dyn error::Error>> {
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

        if pad && (new_width != width || new_height != height) {
            let img = image::load_from_memory(&buffer)?;
            let mut new_img = DynamicImage::new_rgba8(width, height);
            let x_offset = if width == new_width {
                0
            } else {
                (width - new_width) / 2
            };
            let y_offset = if height == new_height {
                0
            } else {
                (height - new_height) / 2
            };
            new_img.copy_from(&img, x_offset, y_offset)?;
            let mut cursor = Cursor::new(Vec::new());
            new_img.write_to(&mut cursor, image::ImageFormat::Png)?;
            return Ok((cursor.into_inner(), center, width, height));
        }

        Ok((buffer, center, new_width, new_height))
    }
}

pub struct Viewport {
    zoom: usize,
    x: i32,
    y: i32,
    image_width: u32,
    image_height: u32,
    term_width: u32,
    term_height: u32,
}

impl Viewport {
    pub fn new(image: &DynamicImage) -> Self {
        let (img_width, img_height) = image.dimensions();
        let tinfo = term_misc::get_wininfo();

        Viewport {
            zoom: 1,
            x: 0,
            y: 0,
            image_width: img_width,
            image_height: img_height,
            term_width: tinfo.sc_width as u32,
            term_height: tinfo.sc_height as u32,
        }
    }

    pub fn pan(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
        self.clamp_viewport();
    }

    pub fn zoom(&mut self, factor: f32, center_x: Option<f32>, center_y: Option<f32>) {
        let old_zoom = self.zoom as f32;
        let new_zoom = (self.zoom as f32 * factor).max(1.0).round() as usize;

        if new_zoom == self.zoom {
            return;
        }

        // Get center coordinates (either provided or current center)
        let (cx, cy) = match (center_x, center_y) {
            (Some(x), Some(y)) => (x, y),
            _ => {
                let center_x = self.x as f32 + (self.term_width as f32 / 2.0) / old_zoom;
                let center_y = self.y as f32 + (self.term_height as f32 / 2.0) / old_zoom;
                (center_x, center_y)
            }
        };

        // Calculate new position to keep the center point stable
        self.zoom = new_zoom;
        let new_x = cx - (self.term_width as f32 / 2.0) / self.zoom as f32;
        let new_y = cy - (self.term_height as f32 / 2.0) / self.zoom as f32;

        self.x = new_x.round() as i32;
        self.y = new_y.round() as i32;
        self.clamp_viewport();
    }

    pub fn apply(&self, image: &DynamicImage) -> DynamicImage {
        if self.zoom == 1 && self.x == 0 && self.y == 0 {
            return image.clone();
        }

        // Calculate visible rectangle in image coordinates
        let view_x = (self.x as f32 / self.zoom as f32).max(0.0);
        let view_y = (self.y as f32 / self.zoom as f32).max(0.0);
        let view_width = self.term_width as f32 / self.zoom as f32;
        let view_height = self.term_height as f32 / self.zoom as f32;

        // Convert to pixel coordinates with bounds checking
        let x1 = view_x.round() as u32;
        let y1 = view_y.round() as u32;
        let x2 = (view_x + view_width).round() as u32;
        let y2 = (view_y + view_height).round() as u32;

        // Clamp coordinates to image bounds
        let x1 = x1.min(self.image_width - 1);
        let y1 = y1.min(self.image_height - 1);
        let x2 = x2.min(self.image_width);
        let y2 = y2.min(self.image_height);

        // Extract the visible region
        let x1 = dim_to_px(&format!("{x1}c"), SizeDirection::Width).unwrap();
        let x2 = dim_to_px(&format!("{x2}c"), SizeDirection::Width).unwrap();
        let y1 = dim_to_px(&format!("{y1}c"), SizeDirection::Height).unwrap();
        let y2 = dim_to_px(&format!("{y2}c"), SizeDirection::Height).unwrap();

        image.crop_imm(x1, y1, x2 - x1, y2 - y1)
    }

    fn clamp_viewport(&mut self) {
        let max_x =
            ((self.image_width as f32 * self.zoom as f32) - self.term_width as f32).max(0.0) as i32;
        let max_y = ((self.image_height as f32 * self.zoom as f32) - self.term_height as f32)
            .max(0.0) as i32;

        self.x = self.x.clamp(0, max_x);
        self.y = self.y.clamp(0, max_y);
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
