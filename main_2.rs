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
