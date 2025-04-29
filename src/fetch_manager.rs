use std::{error, fs, path::PathBuf};

use chromiumoxide::{BrowserConfig, BrowserFetcher, BrowserFetcherOptions};
use ffmpeg_sidecar::{
    command::FfmpegCommand,
    download::{download_ffmpeg_package, ffmpeg_download_url, unpack_ffmpeg},
};

pub fn fetch_chromium() -> Result<(), Box<dyn error::Error>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let cache_path = get_cache_path();
        eprintln!("downloading chromium, it may take a while..");
        let download_path = cache_path.join("chromium");
        if !download_path.exists() {
            fs::create_dir_all(download_path.clone())?;
        }
        let fetcher = BrowserFetcher::new(
            BrowserFetcherOptions::builder()
                .with_path(&download_path)
                .build()?,
        );
        let info = fetcher.fetch().await?;
        BrowserConfig::builder()
            .chrome_executable(info.executable_path)
            .new_headless_mode()
            .build()?;

        let ind = download_path.join("installed.txt");
        fs::File::create(ind)?;
        eprintln!("done!");
        Ok(())
    })
}

pub fn fetch_ffmpeg() -> Result<(), Box<dyn error::Error>> {
    let cache_path = get_cache_path();
    let des = cache_path.join("ffmpeg");
    if !des.exists() {
        fs::create_dir_all(des.clone())?;
    }
    let download_url = ffmpeg_download_url()?;
    eprintln!(
        "downloading ffmpeg into {}, it may take a while..",
        des.display()
    );
    let archive_path = download_ffmpeg_package(download_url, &des)?;
    eprintln!("unpacking ffmpeg");
    unpack_ffmpeg(&archive_path, &des)?;
    eprintln!("done!");
    Ok(())
}

pub fn clean() -> Result<(), Box<dyn error::Error>> {
    let cache_path = get_cache_path();
    eprintln!("deleting: {}", cache_path.display());
    if cache_path.exists() {
        fs::remove_dir_all(cache_path)?;
    }
    eprintln!("done!");
    Ok(())
}

pub fn get_cache_path() -> PathBuf {
    let base_dir = dirs::cache_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(std::env::temp_dir);

    base_dir.join("mcat")
}

pub fn get_ffmpeg() -> Option<FfmpegCommand> {
    if ffmpeg_sidecar::command::ffmpeg_is_installed() {
        return Some(FfmpegCommand::new());
    }
    let cache_path = get_cache_path();
    let mut ffmpeg_path = cache_path.join("ffmpeg").join("ffmpeg");
    if cfg!(windows) {
        ffmpeg_path.set_extension("exe");
    }
    if ffmpeg_path.exists() {
        return Some(FfmpegCommand::new_with_path(ffmpeg_path));
    }
    None
}
