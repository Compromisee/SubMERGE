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
