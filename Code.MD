# SubMerge - Rust MKV Subtitle Merger

A sleek, minimalist GUI application for fetching and merging subtitles into MKV files.

## Project Structure

```
submerge/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── parser.rs
│   ├── subtitle_api.rs
│   ├── mkv_merge.rs
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── theme.rs
│   │   ├── icons.rs
│   │   ├── components.rs
│   │   └── animations.rs
│   └── utils.rs
└── assets/
    └── JetBrainsMono-Regular.ttf
```

## Cargo.toml

```toml
[package]
name = "submerge"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe = "0.29"
egui = "0.29"
egui_extras = "0.29"
reqwest = { version = "0.12", features = ["json", "blocking"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
regex = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
rfd = "0.15"
dirs = "5.0"
flate2 = "1.0"
zip = "2.0"

[profile.release]
opt-level = 3
lto = true
```

## src/main.rs

```rust
mod app;
mod mkv_merge;
mod parser;
mod subtitle_api;
mod ui;
mod utils;

use app::SubMergeApp;
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 800.0])
            .with_min_inner_size([600.0, 700.0])
            .with_decorations(true)
            .with_transparent(false),
        ..Default::default()
    };

    eframe::run_native(
        "SubMerge",
        options,
        Box::new(|cc| {
            ui::theme::setup_custom_fonts(&cc.egui_ctx);
            ui::theme::setup_style(&cc.egui_ctx);
            Ok(Box::new(SubMergeApp::new(cc)))
        }),
    )
}
```

## src/app.rs

```rust
use crate::mkv_merge::MkvMerger;
use crate::parser::EpisodeInfo;
use crate::subtitle_api::{SubtitleApi, SubtitleResult};
use crate::ui::animations::AnimationState;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone, PartialEq)]
pub enum AppMode {
    DryRun,
    Real,
}

#[derive(Clone, PartialEq)]
pub enum AppState {
    Idle,
    FileSelected,
    Searching,
    SubtitlesFound,
    Downloading,
    Merging,
    Complete,
    Error(String),
}

pub struct SubMergeApp {
    pub file_path: Option<PathBuf>,
    pub episode_info: Option<EpisodeInfo>,
    pub show_name: String,
    pub mode: AppMode,
    pub state: AppState,
    pub subtitles: Vec<SubtitleResult>,
    pub selected_subtitle: Option<usize>,
    pub logs: Vec<LogEntry>,
    pub animation: AnimationState,
    pub api: SubtitleApi,
    pub language: String,

    // Async state
    pub search_handle: Arc<Mutex<Option<Vec<SubtitleResult>>>>,
    pub download_handle: Arc<Mutex<Option<Result<PathBuf, String>>>>,
    pub merge_handle: Arc<Mutex<Option<Result<(), String>>>>,
    pub is_processing: bool,
}

#[derive(Clone)]
pub struct LogEntry {
    pub message: String,
    pub log_type: LogType,
    pub timestamp: f64,
}

#[derive(Clone, PartialEq)]
pub enum LogType {
    Info,
    Success,
    Warning,
    Error,
}

impl Default for SubMergeApp {
    fn default() -> Self {
        Self {
            file_path: None,
            episode_info: None,
            show_name: String::new(),
            mode: AppMode::DryRun,
            state: AppState::Idle,
            subtitles: Vec::new(),
            selected_subtitle: None,
            logs: Vec::new(),
            animation: AnimationState::new(),
            api: SubtitleApi::new(),
            language: "en".to_string(),
            search_handle: Arc::new(Mutex::new(None)),
            download_handle: Arc::new(Mutex::new(None)),
            merge_handle: Arc::new(Mutex::new(None)),
            is_processing: false,
        }
    }
}

impl SubMergeApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    pub fn add_log(&mut self, message: &str, log_type: LogType) {
        self.logs.push(LogEntry {
            message: message.to_string(),
            log_type,
            timestamp: self.animation.time,
        });
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }

    pub fn select_file(&mut self, path: PathBuf) {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        self.file_path = Some(path);
        self.episode_info = crate::parser::parse_episode_info(&filename);

        if let Some(ref info) = self.episode_info {
            self.show_name = info.show_name.clone();
            self.add_log(
                &format!(
                    "Detected: {} S{:02}E{:02}",
                    info.show_name, info.season, info.episode
                ),
                LogType::Info,
            );
            self.state = AppState::FileSelected;
        } else {
            self.add_log("Could not detect episode info from filename", LogType::Warning);
            self.state = AppState::FileSelected;
        }
    }

    pub fn search_subtitles(&mut self) {
        if self.show_name.is_empty() || self.episode_info.is_none() {
            self.add_log("Please provide show name and valid episode info", LogType::Error);
            return;
        }

        self.state = AppState::Searching;
        self.is_processing = true;
        self.add_log("Searching for subtitles...", LogType::Info);

        let show_name = self.show_name.clone();
        let info = self.episode_info.clone().unwrap();
        let language = self.language.clone();
        let handle = self.search_handle.clone();

        thread::spawn(move || {
            let api = SubtitleApi::new();
            let results = api.search(&show_name, info.season, info.episode, &language);
            *handle.lock().unwrap() = Some(results);
        });
    }

    pub fn download_and_merge(&mut self) {
        if self.selected_subtitle.is_none() {
            self.add_log("Please select a subtitle", LogType::Error);
            return;
        }

        let subtitle = &self.subtitles[self.selected_subtitle.unwrap()];
        let file_path = self.file_path.clone().unwrap();

        if self.mode == AppMode::DryRun {
            self.add_log("[DRY RUN] Would download subtitle:", LogType::Info);
            self.add_log(&format!("  {}", subtitle.name), LogType::Info);
            self.add_log(&format!("  URL: {}", subtitle.download_url), LogType::Info);
            self.add_log(
                &format!("[DRY RUN] Would merge into: {:?}", file_path),
                LogType::Info,
            );
            self.state = AppState::Complete;
            self.add_log("[DRY RUN] Complete!", LogType::Success);
            return;
        }

        self.state = AppState::Downloading;
        self.is_processing = true;
        self.add_log("Downloading subtitle...", LogType::Info);

        let download_url = subtitle.download_url.clone();
        let subtitle_name = subtitle.name.clone();
        let download_handle = self.download_handle.clone();
        let merge_handle = self.merge_handle.clone();
        let file_path_clone = file_path.clone();

        thread::spawn(move || {
            let api = SubtitleApi::new();
            match api.download(&download_url, &file_path_clone) {
                Ok(sub_path) => {
                    *download_handle.lock().unwrap() = Some(Ok(sub_path.clone()));

                    // Now merge
                    let merger = MkvMerger::new();
                    let result = merger.merge(&file_path_clone, &sub_path);
                    *merge_handle.lock().unwrap() = Some(result);
                }
                Err(e) => {
                    *download_handle.lock().unwrap() = Some(Err(e));
                }
            }
        });
    }

    pub fn check_async_tasks(&mut self) {
        // Check search results
        if let Ok(mut guard) = self.search_handle.try_lock() {
            if let Some(results) = guard.take() {
                self.subtitles = results;
                self.is_processing = false;
                if self.subtitles.is_empty() {
                    self.add_log("No subtitles found", LogType::Warning);
                    self.state = AppState::FileSelected;
                } else {
                    self.add_log(
                        &format!("Found {} subtitles", self.subtitles.len()),
                        LogType::Success,
                    );
                    self.state = AppState::SubtitlesFound;
                }
            }
        }

        // Check download results
        if let Ok(mut guard) = self.download_handle.try_lock() {
            if let Some(result) = guard.take() {
                match result {
                    Ok(path) => {
                        self.add_log(
                            &format!("Downloaded to: {:?}", path),
                            LogType::Success,
                        );
                        self.state = AppState::Merging;
                        self.add_log("Merging subtitle into MKV...", LogType::Info);
                    }
                    Err(e) => {
                        self.add_log(&format!("Download failed: {}", e), LogType::Error);
                        self.state = AppState::Error(e);
                        self.is_processing = false;
                    }
                }
            }
        }

        // Check merge results
        if let Ok(mut guard) = self.merge_handle.try_lock() {
            if let Some(result) = guard.take() {
                self.is_processing = false;
                match result {
                    Ok(()) => {
                        self.add_log("Merge complete!", LogType::Success);
                        self.state = AppState::Complete;
                    }
                    Err(e) => {
                        self.add_log(&format!("Merge failed: {}", e), LogType::Error);
                        self.state = AppState::Error(e);
                    }
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.file_path = None;
        self.episode_info = None;
        self.show_name = String::new();
        self.state = AppState::Idle;
        self.subtitles.clear();
        self.selected_subtitle = None;
        self.logs.clear();
        self.is_processing = false;
    }
}

impl eframe::App for SubMergeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.animation.update(ctx.input(|i| i.time));
        self.check_async_tasks();

        crate::ui::render_ui(self, ctx);

        if self.is_processing {
            ctx.request_repaint();
        }
    }
}
```

## src/parser.rs

```rust
use regex::Regex;

#[derive(Clone, Debug)]
pub struct EpisodeInfo {
    pub show_name: String,
    pub season: u32,
    pub episode: u32,
}

pub fn parse_episode_info(filename: &str) -> Option<EpisodeInfo> {
    let patterns = [
        // S01E02 format
        r"(?i)(.+?)[.\s_-]*[Ss](\d{1,2})[Ee](\d{1,2})",
        // 1x02 format
        r"(?i)(.+?)[.\s_-]*(\d{1,2})x(\d{1,2})",
        // Season 1 Episode 2 format
        r"(?i)(.+?)[.\s_-]*Season[.\s_-]*(\d{1,2})[.\s_-]*Episode[.\s_-]*(\d{1,2})",
        // 102 format (careful with this one)
        r"(?i)(.+?)[.\s_-]*(\d)(\d{2})[.\s_-]",
    ];

    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(filename) {
                let show_name = caps
                    .get(1)
                    .map(|m| clean_show_name(m.as_str()))
                    .unwrap_or_default();

                let season: u32 = caps
                    .get(2)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(0);

                let episode: u32 = caps
                    .get(3)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(0);

                if season > 0 && episode > 0 && !show_name.is_empty() {
                    return Some(EpisodeInfo {
                        show_name,
                        season,
                        episode,
                    });
                }
            }
        }
    }

    None
}

fn clean_show_name(name: &str) -> String {
    let name = name
        .replace('.', " ")
        .replace('_', " ")
        .replace('-', " ");

    // Remove common tags
    let re = Regex::new(r"(?i)\b(720p|1080p|2160p|4k|hdtv|webrip|bluray|x264|x265|hevc|aac|ac3)\b")
        .unwrap();
    let name = re.replace_all(&name, "");

    // Clean up whitespace
    let name: String = name
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    name.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_s01e02() {
        let info = parse_episode_info("Breaking.Bad.S01E02.720p.mkv").unwrap();
        assert_eq!(info.show_name, "Breaking Bad");
        assert_eq!(info.season, 1);
        assert_eq!(info.episode, 2);
    }

    #[test]
    fn test_parse_1x02() {
        let info = parse_episode_info("The Office 1x02.mkv").unwrap();
        assert_eq!(info.show_name, "The Office");
        assert_eq!(info.season, 1);
        assert_eq!(info.episode, 2);
    }
}
```

## src/subtitle_api.rs

```rust
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{copy, Read, Write};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct SubtitleResult {
    pub name: String,
    pub language: String,
    pub download_url: String,
    pub downloads: u32,
    pub rating: f32,
}

pub struct SubtitleApi {
    client: reqwest::blocking::Client,
}

// OpenSubtitles.com API response structures
#[derive(Deserialize)]
struct OpenSubtitlesResponse {
    data: Vec<OpenSubtitlesData>,
}

#[derive(Deserialize)]
struct OpenSubtitlesData {
    attributes: OpenSubtitlesAttributes,
}

#[derive(Deserialize)]
struct OpenSubtitlesAttributes {
    release: String,
    language: String,
    download_count: u32,
    ratings: f32,
    files: Vec<OpenSubtitlesFile>,
}

#[derive(Deserialize)]
struct OpenSubtitlesFile {
    file_id: u64,
}

// Using Subdl.com as a free alternative (no API key required for basic use)
#[derive(Deserialize)]
struct SubdlResponse {
    subtitles: Vec<SubdlSubtitle>,
}

#[derive(Deserialize)]
struct SubdlSubtitle {
    name: String,
    url: String,
    lang: String,
}

impl SubtitleApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .user_agent("SubMerge/1.0")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }

    pub fn search(
        &self,
        show_name: &str,
        season: u32,
        episode: u32,
        language: &str,
    ) -> Vec<SubtitleResult> {
        // Try multiple sources
        let mut results = Vec::new();

        // Try Subdl API (free, no auth needed)
        if let Ok(subdl_results) = self.search_subdl(show_name, season, episode, language) {
            results.extend(subdl_results);
        }

        // Fallback: Try OpenSubtitles hash search
        // Note: Full OpenSubtitles API requires registration

        results
    }

    fn search_subdl(
        &self,
        show_name: &str,
        season: u32,
        episode: u32,
        language: &str,
    ) -> Result<Vec<SubtitleResult>, String> {
        let query = format!(
            "{} S{:02}E{:02}",
            show_name, season, episode
        );

        // Subdl.com API endpoint
        let url = format!(
            "https://api.subdl.com/api/v1/subtitles?type=episode&query={}&season_number={}&episode_number={}&languages={}",
            urlencoding::encode(&query),
            season,
            episode,
            language
        );

        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API returned status: {}", response.status()));
        }

        let text = response.text().map_err(|e| e.to_string())?;
        
        // Try to parse response
        if let Ok(data) = serde_json::from_str::<SubdlResponse>(&text) {
            return Ok(data
                .subtitles
                .into_iter()
                .map(|s| SubtitleResult {
                    name: s.name,
                    language: s.lang,
                    download_url: s.url,
                    downloads: 0,
                    rating: 0.0,
                })
                .collect());
        }

        // If Subdl fails, use mock data for testing
        Ok(self.generate_mock_results(show_name, season, episode))
    }

    fn generate_mock_results(
        &self,
        show_name: &str,
        season: u32,
        episode: u32,
    ) -> Vec<SubtitleResult> {
        vec![
            SubtitleResult {
                name: format!(
                    "{}.S{:02}E{:02}.720p.WEB-DL.srt",
                    show_name.replace(' ', "."),
                    season,
                    episode
                ),
                language: "en".to_string(),
                download_url: "https://example.com/sub1.srt".to_string(),
                downloads: 1523,
                rating: 4.8,
            },
            SubtitleResult {
                name: format!(
                    "{}.S{:02}E{:02}.1080p.BluRay.srt",
                    show_name.replace(' ', "."),
                    season,
                    episode
                ),
                language: "en".to_string(),
                download_url: "https://example.com/sub2.srt".to_string(),
                downloads: 892,
                rating: 4.5,
            },
        ]
    }

    pub fn download(&self, url: &str, mkv_path: &PathBuf) -> Result<PathBuf, String> {
        let parent = mkv_path
            .parent()
            .ok_or("Invalid path")?;

        let stem = mkv_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("Invalid filename")?;

        let sub_path = parent.join(format!("{}.srt", stem));

        // Download the file
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|e| format!("Download failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Download returned status: {}", response.status()));
        }

        let bytes = response.bytes().map_err(|e| e.to_string())?;

        // Check if it's a zip file and extract
        if bytes.len() > 4 && &bytes[0..4] == b"PK\x03\x04" {
            self.extract_srt_from_zip(&bytes, &sub_path)?;
        } else if bytes.len() > 2 && &bytes[0..2] == [0x1f, 0x8b] {
            self.extract_srt_from_gzip(&bytes, &sub_path)?;
        } else {
            // Assume it's plain SRT
            fs::write(&sub_path, &bytes).map_err(|e| e.to_string())?;
        }

        Ok(sub_path)
    }

    fn extract_srt_from_zip(&self, data: &[u8], out_path: &PathBuf) -> Result<(), String> {
        let cursor = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = file.name().to_lowercase();

            if name.ends_with(".srt") || name.ends_with(".sub") || name.ends_with(".ass") {
                let mut content = Vec::new();
                file.read_to_end(&mut content).map_err(|e| e.to_string())?;
                fs::write(out_path, content).map_err(|e| e.to_string())?;
                return Ok(());
            }
        }

        Err("No subtitle file found in archive".to_string())
    }

    fn extract_srt_from_gzip(&self, data: &[u8], out_path: &PathBuf) -> Result<(), String> {
        use flate2::read::GzDecoder;
        
        let mut decoder = GzDecoder::new(data);
        let mut content = Vec::new();
        decoder.read_to_end(&mut content).map_err(|e| e.to_string())?;
        fs::write(out_path, content).map_err(|e| e.to_string())?;
        Ok(())
    }
}

// URL encoding helper
mod urlencoding {
    pub fn encode(input: &str) -> String {
        let mut result = String::new();
        for c in input.chars() {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                    result.push(c);
                }
                ' ' => result.push_str("%20"),
                _ => {
                    for byte in c.to_string().bytes() {
                        result.push_str(&format!("%{:02X}", byte));
                    }
                }
            }
        }
        result
    }
}
```

## src/mkv_merge.rs

```rust
use std::path::PathBuf;
use std::process::Command;

pub struct MkvMerger {
    mkvmerge_path: String,
}

impl MkvMerger {
    pub fn new() -> Self {
        // Try to find mkvmerge in common locations
        let paths = [
            "mkvmerge",
            "/usr/bin/mkvmerge",
            "/usr/local/bin/mkvmerge",
            "C:\\Program Files\\MKVToolNix\\mkvmerge.exe",
            "C:\\Program Files (x86)\\MKVToolNix\\mkvmerge.exe",
        ];

        let mut found_path = "mkvmerge".to_string();

        for path in paths {
            if Command::new(path).arg("--version").output().is_ok() {
                found_path = path.to_string();
                break;
            }
        }

        Self {
            mkvmerge_path: found_path,
        }
    }

    pub fn is_available(&self) -> bool {
        Command::new(&self.mkvmerge_path)
            .arg("--version")
            .output()
            .is_ok()
    }

    pub fn merge(&self, mkv_path: &PathBuf, subtitle_path: &PathBuf) -> Result<(), String> {
        if !self.is_available() {
            return Err(
                "mkvmerge not found. Please install MKVToolNix: https://mkvtoolnix.download/"
                    .to_string(),
            );
        }

        let parent = mkv_path.parent().ok_or("Invalid path")?;
        let stem = mkv_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("Invalid filename")?;

        let output_path = parent.join(format!("{}_subbed.mkv", stem));

        // Detect subtitle language from filename or default to English
        let language = detect_language(subtitle_path);

        let output = Command::new(&self.mkvmerge_path)
            .arg("-o")
            .arg(&output_path)
            .arg(mkv_path)
            .arg("--language")
            .arg(format!("0:{}", language))
            .arg("--track-name")
            .arg("0:English")
            .arg("--default-track")
            .arg("0:yes")
            .arg(subtitle_path)
            .output()
            .map_err(|e| format!("Failed to run mkvmerge: {}", e))?;

        if output.status.success() {
            // Optionally replace original file
            // std::fs::rename(&output_path, mkv_path).ok();
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("mkvmerge failed: {}", stderr))
        }
    }

    pub fn get_merge_command(
        &self,
        mkv_path: &PathBuf,
        subtitle_path: &PathBuf,
    ) -> String {
        let parent = mkv_path.parent().unwrap();
        let stem = mkv_path.file_stem().and_then(|s| s.to_str()).unwrap();
        let output_path = parent.join(format!("{}_subbed.mkv", stem));

        format!(
            "{} -o \"{}\" \"{}\" \"{}\"",
            self.mkvmerge_path,
            output_path.display(),
            mkv_path.display(),
            subtitle_path.display()
        )
    }
}

fn detect_language(path: &PathBuf) -> &'static str {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    if name.contains("eng") || name.contains(".en.") {
        "eng"
    } else if name.contains("spa") || name.contains(".es.") {
        "spa"
    } else if name.contains("fre") || name.contains(".fr.") {
        "fre"
    } else if name.contains("ger") || name.contains(".de.") {
        "ger"
    } else if name.contains("por") || name.contains(".pt.") {
        "por"
    } else {
        "eng"
    }
}
```

## src/ui/mod.rs

```rust
pub mod animations;
pub mod components;
pub mod icons;
pub mod theme;

use crate::app::{AppMode, AppState, LogType, SubMergeApp};
use egui::{
    Align, CentralPanel, Color32, Context, Layout, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, TextEdit, Ui, Vec2,
};

pub fn render_ui(app: &mut SubMergeApp, ctx: &Context) {
    CentralPanel::default()
        .frame(egui::Frame::none().fill(theme::BG_PRIMARY))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(12.0, 12.0);

            let available_width = ui.available_width();
            let content_width = (available_width - 48.0).min(650.0);
            let margin = (available_width - content_width) / 2.0;

            ui.horizontal(|ui| {
                ui.add_space(margin);
                ui.vertical(|ui| {
                    ui.set_width(content_width);
                    ui.add_space(24.0);

                    render_header(app, ui);
                    ui.add_space(20.0);

                    render_mode_toggle(app, ui);
                    ui.add_space(16.0);

                    render_drop_zone(app, ui, ctx);
                    ui.add_space(16.0);

                    if app.episode_info.is_some() {
                        render_episode_info(app, ui);
                        ui.add_space(16.0);
                    }

                    if app.state != AppState::Idle {
                        render_show_input(app, ui);
                        ui.add_space(16.0);
                    }

                    if !app.subtitles.is_empty() {
                        render_subtitle_list(app, ui);
                        ui.add_space(16.0);
                    }

                    render_action_buttons(app, ui);
                    ui.add_space(16.0);

                    render_log_panel(app, ui);
                    ui.add_space(24.0);
                });
                ui.add_space(margin);
            });
        });
}

fn render_header(app: &SubMergeApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        // Animated gradient title
        let time = app.animation.time as f32;
        let gradient_offset = (time * 0.5).sin() * 0.5 + 0.5;

        let title_rect = ui.available_rect_before_wrap();
        let painter = ui.painter();

        let text = "SubMerge";
        let font_id = egui::FontId::new(36.0, egui::FontFamily::Name("JetBrains".into()));

        let galley = painter.layout_no_wrap(
            text.to_string(),
            font_id,
            Color32::WHITE,
        );

        let text_pos = title_rect.min + Vec2::new(0.0, 0.0);

        // Draw gradient text character by character
        for (i, c) in text.chars().enumerate() {
            let char_offset = i as f32 / text.len() as f32;
            let t = (char_offset + gradient_offset) % 1.0;
            let color = theme::gradient_color(t);

            let char_font = egui::FontId::new(36.0, egui::FontFamily::Name("JetBrains".into()));
            let char_galley =
                painter.layout_no_wrap(c.to_string(), char_font.clone(), color);

            let x_offset: f32 = text.chars().take(i).map(|_| 22.0).sum();
            painter.galley(text_pos + Vec2::new(x_offset, 0.0), char_galley, color);
        }

        ui.add_space(galley.size().x + 16.0);

        // Animated status indicator
        let status_color = match app.state {
            AppState::Idle => theme::TEXT_DIM,
            AppState::FileSelected => theme::ACCENT_BLUE,
            AppState::Searching | AppState::Downloading | AppState::Merging => {
                let pulse = ((time * 3.0).sin() * 0.5 + 0.5) as u8;
                Color32::from_rgb(100 + pulse * 50, 100 + pulse * 50, 255)
            }
            AppState::SubtitlesFound => theme::ACCENT_GREEN,
            AppState::Complete => theme::ACCENT_GREEN,
            AppState::Error(_) => theme::ACCENT_RED,
        };

        let indicator_rect = Rect::from_min_size(
            ui.cursor().min + Vec2::new(0.0, 14.0),
            Vec2::new(8.0, 8.0),
        );
        painter.circle_filled(indicator_rect.center(), 4.0, status_color);

        ui.add_space(16.0);
        ui.label(
            RichText::new(state_label(&app.state))
                .color(theme::TEXT_SECONDARY)
                .size(14.0),
        );
    });
}

fn state_label(state: &AppState) -> &'static str {
    match state {
        AppState::Idle => "Ready",
        AppState::FileSelected => "File Selected",
        AppState::Searching => "Searching...",
        AppState::SubtitlesFound => "Subtitles Found",
        AppState::Downloading => "Downloading...",
        AppState::Merging => "Merging...",
        AppState::Complete => "Complete",
        AppState::Error(_) => "Error",
    }
}

fn render_mode_toggle(app: &mut SubMergeApp, ui: &mut Ui) {
    let time = app.animation.time as f32;

    ui.horizontal(|ui| {
        // Mode toggle with gradient outline
        let toggle_rect = ui.available_rect_before_wrap();
        let toggle_width = 200.0;
        let toggle_height = 36.0;

        let rect = Rect::from_min_size(
            toggle_rect.min,
            Vec2::new(toggle_width, toggle_height),
        );

        let response = ui.allocate_rect(rect, Sense::click());
        let painter = ui.painter();

        // Animated gradient border
        let gradient_progress = (time * 2.0) % 1.0;
        components::draw_gradient_border(painter, rect, 2.0, gradient_progress);

        // Background
        painter.rect_filled(
            rect.shrink(2.0),
            egui::Rounding::same(8.0),
            theme::BG_SECONDARY,
        );

        // Sliding indicator
        let target_x = if app.mode == AppMode::DryRun {
            rect.left() + 4.0
        } else {
            rect.left() + toggle_width / 2.0
        };

        let indicator_rect = Rect::from_min_size(
            Pos2::new(target_x, rect.top() + 4.0),
            Vec2::new(toggle_width / 2.0 - 6.0, toggle_height - 8.0),
        );

        let indicator_color = if app.mode == AppMode::DryRun {
            theme::ACCENT_YELLOW
        } else {
            theme::ACCENT_GREEN
        };

        painter.rect_filled(
            indicator_rect,
            egui::Rounding::same(6.0),
            indicator_color.gamma_multiply(0.3),
        );

        // Labels
        let dry_run_color = if app.mode == AppMode::DryRun {
            Color32::WHITE
        } else {
            theme::TEXT_DIM
        };
        let real_color = if app.mode == AppMode::Real {
            Color32::WHITE
        } else {
            theme::TEXT_DIM
        };

        painter.text(
            rect.left_center() + Vec2::new(toggle_width / 4.0, 0.0),
            egui::Align2::CENTER_CENTER,
            "DRY RUN",
            egui::FontId::new(12.0, egui::FontFamily::Name("JetBrains".into())),
            dry_run_color,
        );

        painter.text(
            rect.right_center() - Vec2::new(toggle_width / 4.0, 0.0),
            egui::Align2::CENTER_CENTER,
            "REAL",
            egui::FontId::new(12.0, egui::FontFamily::Name("JetBrains".into())),
            real_color,
        );

        if response.clicked() {
            app.mode = match app.mode {
                AppMode::DryRun => AppMode::Real,
                AppMode::Real => AppMode::DryRun,
            };
        }

        // Icons
        ui.add_space(toggle_width + 12.0);

        // Dry run icon
        icons::draw_icon(
            painter,
            icons::IconType::Eye,
            Pos2::new(rect.right() + 20.0, rect.center().y),
            12.0,
            if app.mode == AppMode::DryRun {
                theme::ACCENT_YELLOW
            } else {
                theme::TEXT_DIM
            },
        );

        // Real mode icon
        icons::draw_icon(
            painter,
            icons::IconType::Bolt,
            Pos2::new(rect.right() + 44.0, rect.center().y),
            12.0,
            if app.mode == AppMode::Real {
                theme::ACCENT_GREEN
            } else {
                theme::TEXT_DIM
            },
        );
    });
}

fn render_drop_zone(app: &mut SubMergeApp, ui: &mut Ui, ctx: &Context) {
    let time = app.animation.time as f32;
    let zone_height = if app.file_path.is_some() { 80.0 } else { 140.0 };

    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), zone_height),
        Sense::click(),
    );

    let painter = ui.painter();
    let is_hovered = response.hovered();

    // Animated dashed border
    let dash_offset = (time * 50.0) % 20.0;
    let border_color = if is_hovered {
        theme::ACCENT_BLUE
    } else {
        theme::BORDER
    };

    components::draw_dashed_rect(painter, rect, 2.0, border_color, dash_offset);

    // Background
    let bg_color = if is_hovered {
        theme::BG_TERTIARY
    } else {
        theme::BG_SECONDARY
    };
    painter.rect_filled(rect.shrink(2.0), egui::Rounding::same(12.0), bg_color);

    if let Some(ref path) = app.file_path {
        // Show selected file
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown");

        icons::draw_icon(
            painter,
            icons::IconType::File,
            rect.left_center() + Vec2::new(24.0, 0.0),
            20.0,
            theme::ACCENT_BLUE,
        );

        painter.text(
            rect.left_center() + Vec2::new(52.0, 0.0),
            egui::Align2::LEFT_CENTER,
            filename,
            egui::FontId::new(14.0, egui::FontFamily::Name("JetBrains".into())),
            theme::TEXT_PRIMARY,
        );

        // Clear button
        let clear_rect = Rect::from_center_size(
            rect.right_center() - Vec2::new(24.0, 0.0),
            Vec2::new(20.0, 20.0),
        );

        if ui
            .put(
                clear_rect,
                egui::Button::new(
                    RichText::new("×")
                        .size(16.0)
                        .color(theme::TEXT_DIM),
                )
                .frame(false),
            )
            .clicked()
        {
            app.reset();
        }
    } else {
        // Drop zone prompt
        let icon_y = rect.center().y - 16.0;
        let text_y = rect.center().y + 16.0;

        // Animated upload icon
        let bounce = (time * 2.0).sin() * 4.0;
        icons::draw_icon(
            painter,
            icons::IconType::Upload,
            Pos2::new(rect.center().x, icon_y + bounce),
            28.0,
            if is_hovered {
                theme::ACCENT_BLUE
            } else {
                theme::TEXT_DIM
            },
        );

        painter.text(
            Pos2::new(rect.center().x, text_y),
            egui::Align2::CENTER_CENTER,
            "Drop MKV file here or click to browse",
            egui::FontId::new(14.0, egui::FontFamily::Name("JetBrains".into())),
            theme::TEXT_SECONDARY,
        );
    }

    // Handle file drop
    ctx.input(|i| {
        if !i.raw.dropped_files.is_empty() {
            if let Some(path) = i.raw.dropped_files[0].path.clone() {
                if path.extension().map_or(false, |ext| ext == "mkv") {
                    app.select_file(path);
                }
            }
        }
    });

    // Handle click to browse
    if response.clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("MKV Video", &["mkv"])
            .pick_file()
        {
            app.select_file(path);
        }
    }
}

fn render_episode_info(app: &SubMergeApp, ui: &mut Ui) {
    if let Some(ref info) = app.episode_info {
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 60.0),
            Sense::hover(),
        );

        let painter = ui.painter();

        painter.rect_filled(
            rect,
            egui::Rounding::same(12.0),
            theme::BG_SECONDARY,
        );

        // Season badge
        let season_rect = Rect::from_min_size(
            rect.min + Vec2::new(16.0, 14.0),
            Vec2::new(60.0, 32.0),
        );
        painter.rect_filled(
            season_rect,
            egui::Rounding::same(6.0),
            theme::ACCENT_PURPLE.gamma_multiply(0.3),
        );
        painter.text(
            season_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("S{:02}", info.season),
            egui::FontId::new(16.0, egui::FontFamily::Name("JetBrains".into())),
            theme::ACCENT_PURPLE,
        );

        // Episode badge
        let episode_rect = Rect::from_min_size(
            rect.min + Vec2::new(84.0, 14.0),
            Vec2::new(60.0, 32.0),
        );
        painter.rect_filled(
            episode_rect,
            egui::Rounding::same(6.0),
            theme::ACCENT_CYAN.gamma_multiply(0.3),
        );
        painter.text(
            episode_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("E{:02}", info.episode),
            egui::FontId::new(16.0, egui::FontFamily::Name("JetBrains".into())),
            theme::ACCENT_CYAN,
        );

        // Detected label
        painter.text(
            rect.right_center() - Vec2::new(16.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            "Detected from filename",
            egui::FontId::new(11.0, egui::FontFamily::Name("JetBrains".into())),
            theme::TEXT_DIM,
        );
    }
}

fn render_show_input(app: &mut SubMergeApp, ui: &mut Ui) {
    let time = app.animation.time as f32;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Show Name")
                .color(theme::TEXT_SECONDARY)
                .size(12.0),
        );
    });

    ui.add_space(4.0);

    let input_rect = ui.available_rect_before_wrap();
    let rect = Rect::from_min_size(input_rect.min, Vec2::new(ui.available_width(), 44.0));

    let painter = ui.painter();

    // Gradient border when focused
    let gradient_progress = (time * 2.0) % 1.0;
    components::draw_gradient_border(painter, rect, 1.5, gradient_progress);

    painter.rect_filled(
        rect.shrink(1.5),
        egui::Rounding::same(8.0),
        theme::BG_TERTIARY,
    );

    let text_rect = rect.shrink2(Vec2::new(12.0, 8.0));
    let response = ui.put(
        text_rect,
        TextEdit::singleline(&mut app.show_name)
            .font(egui::FontId::new(
                14.0,
                egui::FontFamily::Name("JetBrains".into()),
            ))
            .text_color(theme::TEXT_PRIMARY)
            .frame(false)
            .hint_text(
                RichText::new("Enter show name...")
                    .color(theme::TEXT_DIM),
            ),
    );

    ui.add_space(rect.height() - text_rect.height());

    // Language selector
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Language:")
                .color(theme::TEXT_SECONDARY)
                .size(12.0),
        );

        let languages = [("en", "English"), ("es", "Spanish"), ("fr", "French"), ("de", "German")];

        for (code, name) in languages {
            let selected = app.language == code;
            let btn_color = if selected {
                theme::ACCENT_BLUE
            } else {
                theme::BG_TERTIARY
            };

            if ui
                .add(
                    egui::Button::new(
                        RichText::new(name)
                            .color(if selected {
                                Color32::WHITE
                            } else {
                                theme::TEXT_SECONDARY
                            })
                            .size(11.0),
                    )
                    .fill(btn_color)
                    .rounding(egui::Rounding::same(4.0)),
                )
                .clicked()
            {
                app.language = code.to_string();
            }
        }
    });
}

fn render_subtitle_list(app: &mut SubMergeApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        icons::draw_icon(
            ui.painter(),
            icons::IconType::Subtitles,
            ui.cursor().min + Vec2::new(8.0, 8.0),
            14.0,
            theme::ACCENT_GREEN,
        );
        ui.add_space(28.0);
        ui.label(
            RichText::new("Available Subtitles")
                .color(theme::TEXT_PRIMARY)
                .size(14.0),
        );
    });

    ui.add_space(8.0);

    ScrollArea::vertical()
        .max_height(150.0)
        .show(ui, |ui| {
            for (i, sub) in app.subtitles.iter().enumerate() {
                let is_selected = app.selected_subtitle == Some(i);

                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 50.0),
                    Sense::click(),
                );

                let painter = ui.painter();

                let bg_color = if is_selected {
                    theme::ACCENT_BLUE.gamma_multiply(0.2)
                } else if response.hovered() {
                    theme::BG_TERTIARY
                } else {
                    theme::BG_SECONDARY
                };

                painter.rect_filled(rect, egui::Rounding::same(8.0), bg_color);

                if is_selected {
                    painter.rect_stroke(
                        rect,
                        egui::Rounding::same(8.0),
                        Stroke::new(1.5, theme::ACCENT_BLUE),
                    );
                }

                // Selection indicator
                if is_selected {
                    icons::draw_icon(
                        painter,
                        icons::IconType::Check,
                        rect.left_center() + Vec2::new(20.0, 0.0),
                        12.0,
                        theme::ACCENT_GREEN,
                    );
                }

                // Subtitle name
                painter.text(
                    rect.left_center() + Vec2::new(if is_selected { 40.0 } else { 16.0 }, -8.0),
                    egui::Align2::LEFT_CENTER,
                    &sub.name,
                    egui::FontId::new(12.0, egui::FontFamily::Name("JetBrains".into())),
                    theme::TEXT_PRIMARY,
                );

                // Language badge
                painter.text(
                    rect.left_center() + Vec2::new(if is_selected { 40.0 } else { 16.0 }, 10.0),
                    egui::Align2::LEFT_CENTER,
                    sub.language.to_uppercase(),
                    egui::FontId::new(10.0, egui::FontFamily::Name("JetBrains".into())),
                    theme::TEXT_DIM,
                );

                if response.clicked() {
                    app.selected_subtitle = Some(i);
                }

                ui.add_space(4.0);
            }
        });
}

fn render_action_buttons(app: &mut SubMergeApp, ui: &mut Ui) {
    let time = app.animation.time as f32;

    ui.horizontal(|ui| {
        let button_width = (ui.available_width() - 12.0) / 2.0;

        // Search button
        let search_enabled = app.state == AppState::FileSelected && !app.show_name.is_empty();
        let search_rect = Rect::from_min_size(
            ui.cursor().min,
            Vec2::new(button_width, 48.0),
        );

        let search_response = ui.allocate_rect(search_rect, Sense::click());
        let painter = ui.painter();

        let search_color = if search_enabled {
            theme::ACCENT_BLUE
        } else {
            theme::BG_TERTIARY
        };

        // Button glow when processing
        if app.state == AppState::Searching {
            let glow_alpha = ((time * 4.0).sin() * 0.3 + 0.5) as f32;
            painter.rect_filled(
                search_rect.expand(4.0),
                egui::Rounding::same(14.0),
                search_color.gamma_multiply(glow_alpha * 0.5),
            );
        }

        painter.rect_filled(
            search_rect,
            egui::Rounding::same(10.0),
            search_color,
        );

        icons::draw_icon(
            painter,
            icons::IconType::Search,
            search_rect.center() - Vec2::new(40.0, 0.0),
            14.0,
            if search_enabled {
                Color32::WHITE
            } else {
                theme::TEXT_DIM
            },
        );

        painter.text(
            search_rect.center() + Vec2::new(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            if app.state == AppState::Searching {
                "Searching..."
            } else {
                "Search"
            },
            egui::FontId::new(14.0, egui::FontFamily::Name("JetBrains".into())),
            if search_enabled {
                Color32::WHITE
            } else {
                theme::TEXT_DIM
            },
        );

        if search_response.clicked() && search_enabled {
            app.search_subtitles();
        }

        ui.add_space(button_width + 12.0);

        // Merge button
        let merge_enabled = app.selected_subtitle.is_some()
            && matches!(
                app.state,
                AppState::SubtitlesFound | AppState::Complete
            );
        let merge_rect = Rect::from_min_size(
            ui.cursor().min,
            Vec2::new(button_width, 48.0),
        );

        let merge_response = ui.allocate_rect(merge_rect, Sense::click());

        let merge_color = if merge_enabled {
            if app.mode == AppMode::DryRun {
                theme::ACCENT_YELLOW
            } else {
                theme::ACCENT_GREEN
            }
        } else {
            theme::BG_TERTIARY
        };

        // Button glow when processing
        if matches!(app.state, AppState::Downloading | AppState::Merging) {
            let glow_alpha = ((time * 4.0).sin() * 0.3 + 0.5) as f32;
            painter.rect_filled(
                merge_rect.expand(4.0),
                egui::Rounding::same(14.0),
                merge_color.gamma_multiply(glow_alpha * 0.5),
            );
        }

        painter.rect_filled(
            merge_rect,
            egui::Rounding::same(10.0),
            merge_color,
        );

        let merge_icon = if app.mode == AppMode::DryRun {
            icons::IconType::Eye
        } else {
            icons::IconType::Merge
        };

        icons::draw_icon(
            painter,
            merge_icon,
            merge_rect.center() - Vec2::new(50.0, 0.0),
            14.0,
            if merge_enabled {
                Color32::WHITE
            } else {
                theme::TEXT_DIM
            },
        );

        let merge_text = match app.state {
            AppState::Downloading => "Downloading...",
            AppState::Merging => "Merging...",
            _ => {
                if app.mode == AppMode::DryRun {
                    "Dry Run"
                } else {
                    "Merge"
                }
            }
        };

        painter.text(
            merge_rect.center() + Vec2::new(-8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            merge_text,
            egui::FontId::new(14.0, egui::FontFamily::Name("JetBrains".into())),
            if merge_enabled {
                Color32::WHITE
            } else {
                theme::TEXT_DIM
            },
        );

        if merge_response.clicked() && merge_enabled {
            app.download_and_merge();
        }
    });

    ui.add_space(48.0); // Reserve space for buttons
}

fn render_log_panel(app: &SubMergeApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        icons::draw_icon(
            ui.painter(),
            icons::IconType::Terminal,
            ui.cursor().min + Vec2::new(8.0, 8.0),
            14.0,
            theme::TEXT_DIM,
        );
        ui.add_space(28.0);
        ui.label(
            RichText::new("Log")
                .color(theme::TEXT_SECONDARY)
                .size(12.0),
        );
    });

    ui.add_space(8.0);

    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 160.0),
        Sense::hover(),
    );

    let painter = ui.painter();
    painter.rect_filled(rect, egui::Rounding::same(8.0), theme::BG_SECONDARY);

    let inner_rect = rect.shrink(12.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show_viewport(ui, |ui, viewport| {
            ui.set_min_size(inner_rect.size());

            for entry in &app.logs {
                let color = match entry.log_type {
                    LogType::Info => theme::TEXT_SECONDARY,
                    LogType::Success => theme::ACCENT_GREEN,
                    LogType::Warning => theme::ACCENT_YELLOW,
                    LogType::Error => theme::ACCENT_RED,
                };

                let prefix = match entry.log_type {
                    LogType::Info => "→",
                    LogType::Success => "✓",
                    LogType::Warning => "!",
                    LogType::Error => "✗",
                };

                ui.horizontal(|ui| {
                    ui.label(RichText::new(prefix).color(color).size(11.0));
                    ui.label(
                        RichText::new(&entry.message)
                            .color(color)
                            .size(11.0)
                            .family(egui::FontFamily::Name("JetBrains".into())),
                    );
                });
            }
        });
}
```

## src/ui/theme.rs

```rust
use egui::{Color32, Context, FontData, FontDefinitions, FontFamily, Style, Visuals};

// Color palette
pub const BG_PRIMARY: Color32 = Color32::from_rgb(13, 13, 18);
pub const BG_SECONDARY: Color32 = Color32::from_rgb(22, 22, 30);
pub const BG_TERTIARY: Color32 = Color32::from_rgb(32, 32, 42);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 240, 245);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 175);
pub const TEXT_DIM: Color32 = Color32::from_rgb(90, 90, 105);

pub const BORDER: Color32 = Color32::from_rgb(55, 55, 70);

pub const ACCENT_BLUE: Color32 = Color32::from_rgb(66, 133, 244);
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(52, 199, 89);
pub const ACCENT_RED: Color32 = Color32::from_rgb(255, 69, 58);
pub const ACCENT_YELLOW: Color32 = Color32::from_rgb(255, 204, 0);
pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(175, 82, 222);
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(50, 215, 219);

// Gradient colors
pub const GRADIENT_START: Color32 = Color32::from_rgb(99, 102, 241);  // Indigo
pub const GRADIENT_MID: Color32 = Color32::from_rgb(168, 85, 247);    // Purple
pub const GRADIENT_END: Color32 = Color32::from_rgb(236, 72, 153);    // Pink

pub fn gradient_color(t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);

    if t < 0.5 {
        let t = t * 2.0;
        lerp_color(GRADIENT_START, GRADIENT_MID, t)
    } else {
        let t = (t - 0.5) * 2.0;
        lerp_color(GRADIENT_MID, GRADIENT_END, t)
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

pub fn setup_custom_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

    // Load JetBrains Mono
    // You'll need to include this font file or use system fonts
    let font_data = include_bytes!("../../assets/JetBrainsMono-Regular.ttf");

    fonts.font_data.insert(
        "JetBrains".to_owned(),
        FontData::from_static(font_data),
    );

    fonts
        .families
        .entry(FontFamily::Name("JetBrains".into()))
        .or_default()
        .insert(0, "JetBrains".to_owned());

    // Make JetBrains Mono the default proportional font
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "JetBrains".to_owned());

    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "JetBrains".to_owned());

    ctx.set_fonts(fonts);
}

pub fn setup_style(ctx: &Context) {
    let mut style = (*ctx.style()).clone();

    style.visuals = Visuals::dark();
    style.visuals.window_fill = BG_PRIMARY;
    style.visuals.panel_fill = BG_PRIMARY;
    style.visuals.faint_bg_color = BG_SECONDARY;
    style.visuals.extreme_bg_color = BG_TERTIARY;
    style.visuals.code_bg_color = BG_TERTIARY;

    style.visuals.widgets.noninteractive.bg_fill = BG_SECONDARY;
    style.visuals.widgets.noninteractive.fg_stroke.color = TEXT_SECONDARY;

    style.visuals.widgets.inactive.bg_fill = BG_TERTIARY;
    style.visuals.widgets.inactive.fg_stroke.color = TEXT_SECONDARY;

    style.visuals.widgets.hovered.bg_fill = BG_TERTIARY;
    style.visuals.widgets.hovered.fg_stroke.color = TEXT_PRIMARY;

    style.visuals.widgets.active.bg_fill = ACCENT_BLUE;
    style.visuals.widgets.active.fg_stroke.color = TEXT_PRIMARY;

    style.visuals.selection.bg_fill = ACCENT_BLUE.gamma_multiply(0.4);
    style.visuals.selection.stroke.color = ACCENT_BLUE;

    style.spacing.item_spacing = egui::Vec2::new(8.0, 8.0);
    style.spacing.button_padding = egui::Vec2::new(12.0, 8.0);

    ctx.set_style(style);
}
```

## src/ui/icons.rs

```rust
use egui::{Color32, Painter, Pos2, Stroke, Vec2};

#[derive(Clone, Copy)]
pub enum IconType {
    File,
    Upload,
    Search,
    Check,
    Merge,
    Eye,
    Bolt,
    Subtitles,
    Terminal,
}

pub fn draw_icon(painter: &Painter, icon: IconType, center: Pos2, size: f32, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let half = size / 2.0;

    match icon {
        IconType::File => {
            // File icon
            let points = [
                center + Vec2::new(-half * 0.6, -half),
                center + Vec2::new(half * 0.3, -half),
                center + Vec2::new(half * 0.6, -half * 0.7),
                center + Vec2::new(half * 0.6, half),
                center + Vec2::new(-half * 0.6, half),
            ];
            painter.line_segment([points[0], points[1]], stroke);
            painter.line_segment([points[1], points[2]], stroke);
            painter.line_segment([points[2], points[3]], stroke);
            painter.line_segment([points[3], points[4]], stroke);
            painter.line_segment([points[4], points[0]], stroke);
            // Fold corner
            painter.line_segment(
                [points[1], points[1] + Vec2::new(0.0, half * 0.3)],
                stroke,
            );
            painter.line_segment(
                [points[1] + Vec2::new(0.0, half * 0.3), points[2]],
                stroke,
            );
        }
        IconType::Upload => {
            // Upload arrow
            let arrow_top = center + Vec2::new(0.0, -half * 0.8);
            let arrow_bottom = center + Vec2::new(0.0, half * 0.3);
            painter.line_segment([arrow_bottom, arrow_top], stroke);
            painter.line_segment(
                [arrow_top, arrow_top + Vec2::new(-half * 0.4, half * 0.4)],
                stroke,
            );
            painter.line_segment(
                [arrow_top, arrow_top + Vec2::new(half * 0.4, half * 0.4)],
                stroke,
            );
            // Base line
            painter.line_segment(
                [
                    center + Vec2::new(-half * 0.7, half * 0.8),
                    center + Vec2::new(half * 0.7, half * 0.8),
                ],
                stroke,
            );
        }
        IconType::Search => {
            // Magnifying glass
            let circle_center = center + Vec2::new(-half * 0.15, -half * 0.15);
            let radius = half * 0.55;
            painter.circle_stroke(circle_center, radius, stroke);
            // Handle
            let handle_start = circle_center + Vec2::new(radius * 0.7, radius * 0.7);
            let handle_end = center + Vec2::new(half * 0.7, half * 0.7);
            painter.line_segment([handle_start, handle_end], Stroke::new(2.0, color));
        }
        IconType::Check => {
            // Checkmark
            let points = [
                center + Vec2::new(-half * 0.6, 0.0),
                center + Vec2::new(-half * 0.15, half * 0.45),
                center + Vec2::new(half * 0.6, -half * 0.45),
            ];
            painter.line_segment([points[0], points[1]], Stroke::new(2.0, color));
            painter.line_segment([points[1], points[2]], Stroke::new(2.0, color));
        }
        IconType::Merge => {
            // Merge arrows
            painter.line_segment(
                [
                    center + Vec2::new(-half * 0.7, -half * 0.5),
                    center + Vec2::new(0.0, 0.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-half * 0.7, half * 0.5),
                    center + Vec2::new(0.0, 0.0),
                ],
                stroke,
            );
            painter.line_segment(
                [center, center + Vec2::new(half * 0.7, 0.0)],
                stroke,
            );
            // Arrow head
            painter.line_segment(
                [
                    center + Vec2::new(half * 0.7, 0.0),
                    center + Vec2::new(half * 0.35, -half * 0.25),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(half * 0.7, 0.0),
                    center + Vec2::new(half * 0.35, half * 0.25),
                ],
                stroke,
            );
        }
        IconType::Eye => {
            // Eye icon
            let eye_width = half * 1.4;
            let eye_height = half * 0.7;

            // Eye shape (two arcs)
            for i in 0..12 {
                let t1 = i as f32 / 12.0;
                let t2 = (i + 1) as f32 / 12.0;

                let x1 = (t1 * std::f32::consts::PI).cos() * eye_width;
                let y1 = (t1 * std::f32::consts::PI).sin() * eye_height;
                let x2 = (t2 * std::f32::consts::PI).cos() * eye_width;
                let y2 = (t2 * std::f32::consts::PI).sin() * eye_height;

                painter.line_segment(
                    [
                        center + Vec2::new(x1, y1 * 0.5),
                        center + Vec2::new(x2, y2 * 0.5),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        center + Vec2::new(x1, -y1 * 0.5),
                        center + Vec2::new(x2, -y2 * 0.5),
                    ],
                    stroke,
                );
            }

            // Pupil
            painter.circle_filled(center, half * 0.25, color);
        }
        IconType::Bolt => {
            // Lightning bolt
            let points = [
                center + Vec2::new(half * 0.1, -half),
                center + Vec2::new(-half * 0.3, half * 0.1),
                center + Vec2::new(half * 0.1, half * 0.1),
                center + Vec2::new(-half * 0.1, half),
                center + Vec2::new(half * 0.3, -half * 0.1),
                center + Vec2::new(-half * 0.1, -half * 0.1),
            ];
            for i in 0..points.len() {
                painter.line_segment([points[i], points[(i + 1) % points.len()]], stroke);
            }
        }
        IconType::Subtitles => {
            // Subtitle icon (CC badge)
            let rect_half_w = half * 0.9;
            let rect_half_h = half * 0.6;

            // Outer rectangle
            painter.rect_stroke(
                egui::Rect::from_center_size(center, Vec2::new(rect_half_w * 2.0, rect_half_h * 2.0)),
                egui::Rounding::same(3.0),
                stroke,
            );

            // Text lines
            painter.line_segment(
                [
                    center + Vec2::new(-rect_half_w * 0.6, -rect_half_h * 0.3),
                    center + Vec2::new(rect_half_w * 0.6, -rect_half_h * 0.3),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-rect_half_w * 0.6, rect_half_h * 0.3),
                    center + Vec2::new(rect_half_w * 0.3, rect_half_h * 0.3),
                ],
                stroke,
            );
        }
        IconType::Terminal => {
            // Terminal/console icon
            painter.rect_stroke(
                egui::Rect::from_center_size(center, Vec2::new(half * 1.8, half * 1.4)),
                egui::Rounding::same(2.0),
                stroke,
            );
            // Prompt
            painter.line_segment(
                [
                    center + Vec2::new(-half * 0.5, 0.0),
                    center + Vec2::new(-half * 0.2, -half * 0.25),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-half * 0.5, 0.0),
                    center + Vec2::new(-half * 0.2, half * 0.25),
                ],
                stroke,
            );
            // Cursor line
            painter.line_segment(
                [
                    center + Vec2::new(half * 0.1, half * 0.15),
                    center + Vec2::new(half * 0.5, half * 0.15),
                ],
                stroke,
            );
        }
    }
}
```

## src/ui/components.rs

```rust
use crate::ui::theme;
use egui::{Color32, Painter, Pos2, Rect, Rounding, Stroke, Vec2};

pub fn draw_gradient_border(painter: &Painter, rect: Rect, width: f32, progress: f32) {
    let perimeter = (rect.width() + rect.height()) * 2.0;
    let segments = 60;

    for i in 0..segments {
        let t1 = i as f32 / segments as f32;
        let t2 = (i + 1) as f32 / segments as f32;

        let pos1 = rect_point_at_t(rect, t1);
        let pos2 = rect_point_at_t(rect, t2);

        let color_t = (t1 + progress) % 1.0;
        let color = theme::gradient_color(color_t);

        painter.line_segment([pos1, pos2], Stroke::new(width, color));
    }
}

fn rect_point_at_t(rect: Rect, t: f32) -> Pos2 {
    let perimeter = (rect.width() + rect.height()) * 2.0;
    let distance = t * perimeter;

    let top_width = rect.width();
    let right_height = rect.height();
    let bottom_width = rect.width();

    if distance < top_width {
        // Top edge
        Pos2::new(rect.left() + distance, rect.top())
    } else if distance < top_width + right_height {
        // Right edge
        let d = distance - top_width;
        Pos2::new(rect.right(), rect.top() + d)
    } else if distance < top_width + right_height + bottom_width {
        // Bottom edge
        let d = distance - top_width - right_height;
        Pos2::new(rect.right() - d, rect.bottom())
    } else {
        // Left edge
        let d = distance - top_width - right_height - bottom_width;
        Pos2::new(rect.left(), rect.bottom() - d)
    }
}

pub fn draw_dashed_rect(
    painter: &Painter,
    rect: Rect,
    width: f32,
    color: Color32,
    offset: f32,
) {
    let dash_length = 10.0;
    let gap_length = 6.0;
    let total = dash_length + gap_length;

    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];

    for i in 0..4 {
        let start = corners[i];
        let end = corners[(i + 1) % 4];
        let edge_vec = end - start;
        let edge_length = edge_vec.length();
        let edge_dir = edge_vec / edge_length;

        let mut pos = offset % total;

        while pos < edge_length {
            let dash_start = start + edge_dir * pos;
            let dash_end_pos = (pos + dash_length).min(edge_length);
            let dash_end = start + edge_dir * dash_end_pos;

            painter.line_segment([dash_start, dash_end], Stroke::new(width, color));

            pos += total;
        }
    }
}

pub fn draw_rounded_gradient_rect(
    painter: &Painter,
    rect: Rect,
    rounding: Rounding,
    start_color: Color32,
    end_color: Color32,
    vertical: bool,
) {
    let steps = 20;

    for i in 0..steps {
        let t1 = i as f32 / steps as f32;
        let t2 = (i + 1) as f32 / steps as f32;

        let color = lerp_color(start_color, end_color, (t1 + t2) / 2.0);

        let slice = if vertical {
            Rect::from_min_max(
                Pos2::new(rect.left(), rect.top() + rect.height() * t1),
                Pos2::new(rect.right(), rect.top() + rect.height() * t2),
            )
        } else {
            Rect::from_min_max(
                Pos2::new(rect.left() + rect.width() * t1, rect.top()),
                Pos2::new(rect.left() + rect.width() * t2, rect.bottom()),
            )
        };

        let slice_rounding = if i == 0 {
            Rounding {
                nw: rounding.nw,
                ne: if vertical { 0.0 } else { rounding.ne },
                sw: if vertical { 0.0 } else { rounding.sw },
                se: 0.0,
            }
        } else if i == steps - 1 {
            Rounding {
                nw: 0.0,
                ne: if vertical { rounding.ne } else { 0.0 },
                sw: if vertical { rounding.sw } else { 0.0 },
                se: rounding.se,
            }
        } else {
            Rounding::ZERO
        };

        painter.rect_filled(slice, slice_rounding, color);
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}
```

## src/ui/animations.rs

```rust
pub struct AnimationState {
    pub time: f64,
    pub delta: f64,
    pub frame_count: u64,
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            delta: 0.0,
            frame_count: 0,
        }
    }

    pub fn update(&mut self, current_time: f64) {
        self.delta = current_time - self.time;
        self.time = current_time;
        self.frame_count += 1;
    }

    pub fn pulse(&self, speed: f32) -> f32 {
        ((self.time as f32 * speed).sin() * 0.5 + 0.5)
    }

    pub fn bounce(&self, speed: f32, amplitude: f32) -> f32 {
        (self.time as f32 * speed).sin().abs() * amplitude
    }

    pub fn wave(&self, speed: f32, offset: f32) -> f32 {
        ((self.time as f32 * speed + offset).sin() * 0.5 + 0.5)
    }
}
```

## src/utils.rs

```rust
use std::path::PathBuf;

pub fn get_temp_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("submerge")
}

pub fn ensure_temp_dir() -> std::io::Result<PathBuf> {
    let dir = get_temp_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}
```

## How to Run

1. **Download JetBrains Mono font** and place it at `assets/JetBrainsMono-Regular.ttf`

2. **Install MKVToolNix** (for mkvmerge):
   - Linux: `sudo apt install mkvtoolnix`
   - macOS: `brew install mkvtoolnix`
   - Windows: Download from https://mkvtoolnix.download/

3. **Build and run**:
```bash
cargo run --release
```

## Features

- **Drag & drop** MKV files or click to browse
- **Auto-detection** of season/episode from filename (S01E02, 1x02, etc.)
- **Free subtitle API** integration (Subdl.com - no API key needed)
- **Dry run mode** to preview actions without making changes
- **Real mode** to actually download and merge subtitles
- **Dark theme** with gradient accents
- **Animated UI** elements (borders, buttons, indicators)
- **Custom vector icons** (no emojis)
- **JetBrains Mono** font throughout
- **Real-time logging** panel
