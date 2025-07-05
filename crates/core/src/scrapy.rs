use futures::StreamExt;
use reqwest::Client;
use scraper::Html;
use tempfile::NamedTempFile;
use tokio::runtime::Builder;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::OnceLock;
use std::io::Write;
use std::time::Duration;

use crate::catter;

fn extension_from_mime(mime: &str) -> Option<&'static str> {
    if mime.contains("image/avif") {
        Some("avif")
    } else if mime.contains("image/jpeg") {
        Some("jpg")
    } else if mime.contains("image/png") {
        Some("png")
    } else if mime.contains("image/apng") {
        Some("apng")
    } else if mime.contains("image/gif") {
        Some("gif")
    } else if mime.contains("image/webp") {
        Some("webp")
    } else if mime.contains("image/tiff") {
        Some("tiff")
    } else if mime.contains("image/x-tga") {
        Some("tga")
    } else if mime.contains("image/vnd.ms-dds") {
        Some("dds")
    } else if mime.contains("image/bmp") {
        Some("bmp")
    } else if mime.contains("image/x-icon") || mime.contains("image/vnd.microsoft.icon") {
        Some("ico")
    } else if mime.contains("image/vnd.radiance") {
        Some("hdr")
    } else if mime.contains("image/aces") || mime.contains("image/exr") {
        Some("exr")
    } else if mime.contains("image/x-portable-bitmap") {
        Some("pbm")
    } else if mime.contains("image/x-portable-graymap") {
        Some("pgm")
    } else if mime.contains("image/x-portable-pixmap") {
        Some("ppm")
    } else if mime.contains("image/x-portable-anymap") {
        Some("pam")
    } else if mime.contains("image/x-farbfeld") {
        Some("ff")
    } else if mime.contains("image/x-qoi") {
        Some("qoi")
    } else if mime.contains("image/x-pcx") {
        Some("pcx")
    } else if mime.contains("image/svg+xml") {
        Some("svg")

    // Video
    } else if mime.contains("video/mp4") {
        Some("mp4")
    } else if mime.contains("video/webm") {
        Some("webm")
    } else if mime.contains("video/x-msvideo") {
        Some("avi")
    } else if mime.contains("video/x-matroska") {
        Some("mkv")
    } else if mime.contains("video/quicktime") {
        Some("mov")

    // Documents
    } else if mime.contains("application/pdf") {
        Some("pdf")
    } else if mime.contains("text/markdown") || mime.contains("text/x-markdown") {
        Some("md")
    } else if mime.contains("application/msword") {
        Some("doc")
    } else if mime
        .contains("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
    {
        Some("docx")
    } else if mime.contains("application/vnd.oasis.opendocument.text") {
        Some("odt")
    } else if mime.contains("application/vnd.ms-excel") {
        Some("xls")
    } else if mime.contains("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet") {
        Some("xlsx")
    } else if mime.contains("application/vnd.ms-excel.sheet.macroenabled.12") {
        Some("xlsm")
    } else if mime.contains("application/vnd.ms-excel.sheet.binary.macroenabled.12") {
        Some("xlsb")
    } else if mime.contains("application/vnd.ms-excel.addin.macroenabled.12") {
        Some("xlam")
    } else if mime.contains("application/vnd.ms-excel.addin") {
        Some("xla")
    } else if mime.contains("application/vnd.oasis.opendocument.spreadsheet") {
        Some("ods")
    } else if mime.contains("application/vnd.ms-powerpoint") {
        Some("ppt")
    } else if mime
        .contains("application/vnd.openxmlformats-officedocument.presentationml.presentation")
    {
        Some("pptx")
    } else if mime.contains("application/vnd.oasis.opendocument.presentation") {
        Some("odp")
    } else if mime.contains("text/csv") || mime.contains("application/csv") {
        Some("csv")

    // Archives
    } else if mime.contains("application/zip") {
        Some("zip")
    } else if mime.contains("application/x-rar-compressed") {
        Some("rar")
    } else if mime.contains("application/x-7z-compressed") {
        Some("7z")
    } else if mime.contains("application/x-tar") {
        Some("tar")
    } else if mime.contains("application/gzip") {
        Some("gz")
    } else if mime.contains("application/x-bzip2") {
        Some("bz2")
    } else {
        None
    }
}


static GLOBAL_MULTI_PROGRESS: OnceLock<MultiProgress> = OnceLock::new();

fn get_global_multi_progress() -> &'static MultiProgress {
    GLOBAL_MULTI_PROGRESS.get_or_init(|| MultiProgress::new())
}

pub fn scrape_biggest_media(url: &str, silent: bool) -> Result<NamedTempFile, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .build()?;

    let rt = Builder::new_current_thread().enable_all().build()?;

    rt.block_on(async {
        let initial_spinner = if !silent {
            let pb = get_global_multi_progress().add(ProgressBar::new_spinner());
            pb.set_style(ProgressStyle::default_spinner()
                .template(&format!("{{spinner:.green}} Fetching {url}..."))?
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "));
            Some(pb)
        } else {
            None
        };

        let request_future = client.get(url).send();
        tokio::pin!(request_future);

        let response = tokio::select! {
            result = &mut request_future => {
                result
            }
            _ = tokio::time::sleep(Duration::from_millis(300)) => {
                // Request is taking too long, start spinner and wait for it
                if let Some(ref spinner) = initial_spinner {
                    spinner.enable_steady_tick(Duration::from_millis(100));
                }
                request_future.await
            }
        }?;

        if let Some(spinner) = initial_spinner {
            spinner.finish_and_clear();
        }

        if !response.status().is_success() {
            return Err(format!("Failed to retrieve URL: {}", response.status()).into());
        }

        let content_type = response
            .headers()
            .get("Content-Type")
            .and_then(|h| h.to_str().ok());

        // Direct file download if mime type is recognized
        if let Some(ct) = content_type {
            if let Some(ext) = extension_from_mime(ct) {
                // Get content length for progress bar if available
                let content_length = response
                    .headers()
                    .get("content-length")
                    .and_then(|cl| cl.to_str().ok())
                    .and_then(|cl| cl.parse::<u64>().ok());
                
                // Setup progress bar if not silent and content length is known
                let progress_bar = if !silent && content_length.is_some() && content_length.unwrap() > 2_000_000 {
                    let pb = get_global_multi_progress().add(ProgressBar::new(content_length.unwrap()));
                    pb.set_style(ProgressStyle::default_bar()
                        .template("{spinner:.green} [{bar:50.blue/white}] {bytes}/{total_bytes} ({percent}%)")?
                        .progress_chars("█▓▒░"));
                    Some(pb)
                } else {
                    None
                };
                
                // Stream the response body
                let mut stream = response.bytes_stream();
                let mut file_data = Vec::new();
                
                while let Some(chunk_result) = stream.next().await {
                    let chunk = chunk_result?;
                    file_data.extend_from_slice(&chunk);
                    
                    // Update progress bar if we have one
                    if let Some(pb) = &progress_bar {
                        pb.set_position(file_data.len() as u64);
                    }
                }
                
                // Finish the progress bar if we have one
                if let Some(pb) = progress_bar {
                    pb.finish_and_clear();
                }
                
                // Write to temp file and return
                let mut tmp_file = NamedTempFile::with_suffix(&format!(".{}", ext))?;
                tmp_file.write_all(&file_data)?;
                return Ok(tmp_file);
            }
        }

        // Process HTML content for embedded media
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

        for (media_url, media_type) in potential_media.iter() {
            if let Ok(resolved_url) = reqwest::Url::parse(url).and_then(|base| base.join(&media_url)) {
                if let Ok(media_response) = client.get(resolved_url.as_str()).send().await {
                    if media_response.status().is_success() {
                        // Get content length for this media
                        let content_length = media_response
                            .headers()
                            .get("content-length")
                            .and_then(|cl| cl.to_str().ok())
                            .and_then(|cl| cl.parse::<u64>().ok());
                        
                        // Only create progress bar for media that's likely to be significant
                        let progress_bar = if !silent && content_length.is_some() && content_length.unwrap() > 2_000_000 {
                            let pb = get_global_multi_progress().add(ProgressBar::new(content_length.unwrap()));
                            pb.set_style(ProgressStyle::default_bar()
                                .template("{spinner:.green} [{bar:50.cyan/blue}] {bytes}/{total_bytes} ({percent}%)")?
                                .progress_chars("█▓▒░ "));
                            Some(pb)
                        } else {
                            None
                        };
                        
                        // Stream the response body for this media
                        let mut stream = media_response.bytes_stream();
                        let mut media_data = Vec::new();
                        
                        while let Some(chunk_result) = stream.next().await {
                            let chunk = chunk_result?;
                            media_data.extend_from_slice(&chunk);
                            
                            // Update progress bar if we have one
                            if let Some(pb) = &progress_bar {
                                pb.set_position(media_data.len() as u64);
                            }
                        }
                        
                        // Finish the progress bar
                        if let Some(pb) = progress_bar {
                            pb.finish_and_clear();
                        }
                        
                        let media_size = media_data.len();
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

                            if is_valid
                                && (biggest_media.is_none()
                                    || media_size > biggest_media.as_ref().unwrap().0)
                            {
                                biggest_media = Some((media_size, media_data, extension));
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
