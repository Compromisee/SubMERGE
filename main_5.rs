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
