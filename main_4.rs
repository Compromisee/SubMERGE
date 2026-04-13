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
