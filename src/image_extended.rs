use std::{error, io::Cursor};

use base64::{Engine, engine::general_purpose};
use image::{DynamicImage, ImageResult};

use crate::term_misc::{self, dim_to_px};

pub trait InlineImage {
    fn encode_base64(&self) -> ImageResult<String>;
    fn resize_plus(
        &self,
        width: Option<&str>,
        height: Option<&str>,
    ) -> Result<DynamicImage, Box<dyn error::Error>>;
}

impl InlineImage for DynamicImage {
    fn encode_base64(&self) -> ImageResult<String> {
        let mut buf = Cursor::new(Vec::new());
        self.write_to(&mut buf, image::ImageFormat::Png)?;
        let bytes = buf.into_inner();

        Ok(general_purpose::STANDARD.encode(&bytes))
    }

    fn resize_plus(
        &self,
        width: Option<&str>,
        height: Option<&str>,
    ) -> Result<Self, Box<dyn error::Error>> {
        let width = match width {
            Some(w) => dim_to_px(w, term_misc::SizeDirection::Width)?,
            None => 0,
        };
        let height = match height {
            Some(h) => dim_to_px(h, term_misc::SizeDirection::Height)?,
            None => 0,
        };

        let img = self.resize(width, height, image::imageops::FilterType::Lanczos3);
        return Ok(img);
    }
}
