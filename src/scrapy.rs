use reqwest::Client;
use scraper::Html;
use tempfile::NamedTempFile;
use tokio::runtime::Builder;

use std::io::Write;

use crate::catter;

pub fn scrape_biggest_media(url: &str) -> Result<NamedTempFile, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .build()?;

    let rt = Builder::new_current_thread().enable_all().build()?;

    rt.block_on(async {
        let response = client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to retrieve URL: {}", response.status()).into());
        }

        let content_type = response
            .headers()
            .get("Content-Type")
            .and_then(|h| h.to_str().ok());

        if let Some(ct) = content_type {
            if ct.contains("image/svg+xml") {
                let svg_bytes = response.bytes().await?;
                let mut tmp_file = NamedTempFile::with_suffix(".svg")?;
                tmp_file.write_all(&svg_bytes)?;
                return Ok(tmp_file);
            }
        }

        let html_content = response.text().await?;
        let html_size = html_content.len();
        let document = Html::parse_document(&html_content);

        let mut potential_media: Vec<(String, String)> = Vec::new();

        // Find image tags
        let image_selector = scraper::Selector::parse("img[src]").unwrap();
        for element in document.select(&image_selector) {
            if let Some(src) = element.value().attr("src") {
                potential_media.push((src.to_string(), "image".to_string()));
            }
        }

        // Find video tags
        let video_selector = scraper::Selector::parse("video[src]").unwrap();
        for element in document.select(&video_selector) {
            if let Some(src) = element.value().attr("src") {
                potential_media.push((src.to_string(), "video".to_string()));
            }
        }

        // Find source tags within video tags
        let video_source_selector = scraper::Selector::parse("video source[src]").unwrap();
        for element in document.select(&video_source_selector) {
            if let Some(src) = element.value().attr("src") {
                potential_media.push((src.to_string(), "video".to_string()));
            }
        }

        // Find object and embed tags for SVGs
        let svg_selectors = vec![
            scraper::Selector::parse("object[type='image/svg+xml'][data]").unwrap(),
            scraper::Selector::parse("embed[type='image/svg+xml'][src]").unwrap(),
            scraper::Selector::parse("img[src$='.svg']").unwrap(),
        ];
        for selector in svg_selectors {
            for element in document.select(&selector) {
                if let Some(src) = element
                    .value()
                    .attr("data")
                    .or_else(|| element.value().attr("src"))
                {
                    potential_media.push((src.to_string(), "svg".to_string()));
                }
            }
        }

        let mut biggest_media: Option<(usize, Vec<u8>, String)> = None;

        for (media_url, media_type) in potential_media {
            if let Ok(resolved_url) =
                reqwest::Url::parse(url).and_then(|base| base.join(&media_url))
            {
                if let Ok(media_response) = client.get(resolved_url.as_str()).send().await {
                    if media_response.status().is_success() {
                        if let Ok(media_bytes) = media_response.bytes().await {
                            let media_size = media_bytes.len();
                            if media_size > (html_size as f64 * 0.3) as usize {
                                let extension = resolved_url
                                    .path_segments()
                                    .and_then(|segments| segments.last())
                                    .and_then(|filename| filename.split('.').last())
                                    .map(|ext| ext.to_lowercase())
                                    .unwrap_or_default();

                                let is_valid = match media_type.as_str() {
                                    "svg" => extension == "svg",
                                    "video" => catter::is_video(&extension),
                                    "image" => {
                                        image::ImageFormat::from_extension(&extension).is_some()
                                    }
                                    _ => false,
                                };

                                if is_valid {
                                    if biggest_media.is_none()
                                        || media_size > biggest_media.as_ref().unwrap().0
                                    {
                                        biggest_media =
                                            Some((media_size, media_bytes.to_vec(), extension));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        match biggest_media {
            Some((_, data, ext)) => {
                let mut tmp_file = NamedTempFile::with_suffix(format!(".{}", ext))?;
                tmp_file.write_all(&data)?;
                Ok(tmp_file)
            }
            None => Err("No significant and valid media found.".into()),
        }
    })
}
