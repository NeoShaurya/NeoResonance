use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordItem {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricLine {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub words: Vec<WordItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackLyrics {
    pub track_key: String,
    pub is_synced: bool,
    pub lines: Vec<LyricLine>,
}

pub struct LyricsCache(pub Arc<Mutex<HashMap<String, TrackLyrics>>>);

impl Default for LyricsCache {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }
}

#[derive(Deserialize)]
struct LrcLibRecord {
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
}

fn parse_timestamp(ts: &str) -> Option<u64> {
    // Format: "mm:ss.xx" or "mm:ss.xxx" or "mm:ss"
    let parts: Vec<&str> = ts.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let mins: u64 = parts[0].trim().parse().ok()?;
    let sec_parts: Vec<&str> = parts[1].split('.').collect();
    let secs: u64 = sec_parts[0].trim().parse().ok()?;
    let ms: u64 = if sec_parts.len() > 1 {
        let ms_str = sec_parts[1].trim();
        if ms_str.len() == 2 {
            ms_str.parse::<u64>().unwrap_or(0) * 10
        } else if ms_str.len() >= 3 {
            ms_str[..3].parse::<u64>().unwrap_or(0)
        } else {
            ms_str.parse::<u64>().unwrap_or(0) * 100
        }
    } else {
        0
    };

    Some(mins * 60_000 + secs * 1000 + ms)
}

fn parse_word_level_line(text: &str, line_start_ms: u64, line_end_ms: u64) -> (String, Vec<WordItem>) {
    let mut clean_text = String::new();
    let mut words = Vec::new();

    // Check if line contains <mm:ss.xx> word tags
    let has_word_tags = text.contains('<') && text.contains('>');

    if has_word_tags {
        // Parse word tags
        let mut pos = 0;
        let chars: Vec<char> = text.chars().collect();
        let mut current_start = line_start_ms;

        while pos < chars.len() {
            if chars[pos] == '<' {
                if let Some(close_idx) = chars[pos..].iter().position(|&c| c == '>') {
                    let tag: String = chars[pos + 1..pos + close_idx].iter().collect();
                    if let Some(ts) = parse_timestamp(&tag) {
                        current_start = ts;
                    }
                    pos += close_idx + 1;
                    continue;
                }
            }

            // Word text until next '<' or end
            let next_tag = chars[pos..].iter().position(|&c| c == '<');
            let word_end_idx = next_tag.map(|idx| pos + idx).unwrap_or(chars.len());
            let raw_word: String = chars[pos..word_end_idx].iter().collect();
            let trimmed = raw_word.trim();

            if !trimmed.is_empty() {
                clean_text.push_str(trimmed);
                clean_text.push(' ');
                words.push(WordItem {
                    text: trimmed.to_string(),
                    start_ms: current_start,
                    end_ms: current_start + 500, // Provisional until adjusted
                });
            }

            pos = word_end_idx;
        }

        // Adjust word end times to match next word's start time
        for i in 0..words.len() {
            if i + 1 < words.len() {
                words[i].end_ms = words[i + 1].start_ms;
            } else {
                words[i].end_ms = line_end_ms.max(words[i].start_ms + 400);
            }
        }
    } else {
        // Standard line without inline word tags:
        // Distribute words smoothly across line duration weighted by character length
        clean_text = text.trim().to_string();
        let word_tokens: Vec<&str> = clean_text.split_whitespace().collect();

        if !word_tokens.is_empty() {
            let total_chars: usize = word_tokens.iter().map(|w| w.chars().count().max(1)).sum();
            let duration = if line_end_ms > line_start_ms {
                line_end_ms - line_start_ms
            } else {
                (word_tokens.len() as u64) * 500
            };

            let mut running_ms = line_start_ms;
            for token in word_tokens {
                let char_len = token.chars().count().max(1);
                let word_duration = (duration * char_len as u64) / (total_chars as u64).max(1);
                let end = running_ms + word_duration;
                words.push(WordItem {
                    text: token.to_string(),
                    start_ms: running_ms,
                    end_ms: end,
                });
                running_ms = end;
            }
        }
    }

    (clean_text.trim().to_string(), words)
}

pub fn parse_lrc(lrc_text: &str, total_duration_ms: Option<u64>) -> Vec<LyricLine> {
    let mut raw_lines = Vec::new();

    for line in lrc_text.lines() {
        let line = line.trim();
        if !line.starts_with('[') {
            continue;
        }

        if let Some(close_bracket) = line.find(']') {
            let ts_str = &line[1..close_bracket];
            if let Some(start_ms) = parse_timestamp(ts_str) {
                let content = line[close_bracket + 1..].trim().to_string();
                if !content.is_empty() {
                    raw_lines.push((start_ms, content));
                }
            }
        }
    }

    raw_lines.sort_by_key(|(start, _)| *start);

    let mut result = Vec::new();
    let count = raw_lines.len();

    for i in 0..count {
        let start_ms = raw_lines[i].0;
        let line_end_ms = if i + 1 < count {
            raw_lines[i + 1].0
        } else {
            total_duration_ms.unwrap_or(start_ms + 4000).max(start_ms + 1000)
        };

        let (text, words) = parse_word_level_line(&raw_lines[i].1, start_ms, line_end_ms);
        if !text.is_empty() {
            result.push(LyricLine {
                start_ms,
                end_ms: line_end_ms,
                text,
                words,
            });
        }
    }

    result
}

#[tauri::command]
pub async fn get_lyrics(
    cache: tauri::State<'_, LyricsCache>,
    artist: String,
    title: String,
    album: Option<String>,
    duration_sec: Option<u64>,
) -> Result<TrackLyrics, String> {
    let track_key = format!("{} - {}", artist.trim().to_lowercase(), title.trim().to_lowercase());

    // Check cache
    {
        let lock = cache.0.lock().unwrap();
        if let Some(cached) = lock.get(&track_key) {
            return Ok(cached.clone());
        }
    }

    if artist.is_empty() || title.is_empty() || title == "No Media Playing" {
        return Ok(TrackLyrics {
            track_key,
            is_synced: false,
            lines: Vec::new(),
        });
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    // Primary lookup
    let mut url = format!(
        "https://lrclib.net/api/get?artist_name={}&track_name={}",
        urlencoding::encode(&artist),
        urlencoding::encode(&title)
    );
    if let Some(ref alb) = album {
        if !alb.is_empty() {
            url.push_str(&format!("&album_name={}", urlencoding::encode(alb)));
        }
    }
    if let Some(dur) = duration_sec {
        if dur > 0 {
            url.push_str(&format!("&duration={}", dur));
        }
    }

    let mut lrc_content: Option<String> = None;

    if let Ok(resp) = client.get(&url).send().await {
        if resp.status().is_success() {
            if let Ok(record) = resp.json::<LrcLibRecord>().await {
                lrc_content = record.synced_lyrics.or(record.plain_lyrics);
            }
        }
    }

    // Fallback search if primary returned nothing
    if lrc_content.is_none() {
        let search_url = format!(
            "https://lrclib.net/api/search?q={}+{}",
            urlencoding::encode(&artist),
            urlencoding::encode(&title)
        );

        if let Ok(resp) = client.get(&search_url).send().await {
            if resp.status().is_success() {
                if let Ok(records) = resp.json::<Vec<LrcLibRecord>>().await {
                    for r in records {
                        if r.synced_lyrics.is_some() {
                            lrc_content = r.synced_lyrics;
                            break;
                        } else if lrc_content.is_none() && r.plain_lyrics.is_some() {
                            lrc_content = r.plain_lyrics;
                        }
                    }
                }
            }
        }
    }

    let parsed = if let Some(content) = lrc_content {
        let is_synced = content.contains('[');
        let lines = if is_synced {
            parse_lrc(&content, duration_sec.map(|d| d * 1000))
        } else {
            content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .enumerate()
                .map(|(i, l)| LyricLine {
                    start_ms: (i as u64) * 3000,
                    end_ms: (i as u64 + 1) * 3000,
                    text: l.trim().to_string(),
                    words: Vec::new(),
                })
                .collect()
        };

        TrackLyrics {
            track_key: track_key.clone(),
            is_synced,
            lines,
        }
    } else {
        TrackLyrics {
            track_key: track_key.clone(),
            is_synced: false,
            lines: Vec::new(),
        }
    };

    // Store in cache
    {
        let mut lock = cache.0.lock().unwrap();
        lock.insert(track_key, parsed.clone());
    }

    Ok(parsed)
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for byte in s.bytes() {
            match byte {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(byte as char);
                }
                _ => {
                    encoded.push_str(&format!("%{:02X}", byte));
                }
            }
        }
        encoded
    }
}
