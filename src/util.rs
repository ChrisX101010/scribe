//! the junk drawer: timestamps, widths, paths, and the osc 52 escape.
//!
//! deliberately boring. every function here takes a value and returns a value,
//! except clipboard encoding, which is just base64 with a fancy envelope.

use std::path::{Path, PathBuf};
use unicode_width::UnicodeWidthStr;

/// format centiseconds as `hh:mm:ss.mmm`. srt callers swap in a comma.
pub fn timestamp_cs(cs: i64) -> String {
    let cs = cs.max(0);
    let hours = cs / 360_000;
    let minutes = (cs / 6_000) % 60;
    let seconds = (cs / 100) % 60;
    let millis = (cs % 100) * 10;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

/// compact form for the transcript gutter: `mm:ss`.
pub fn timestamp_short(cs: i64) -> String {
    let cs = cs.max(0);
    let minutes = cs / 6_000;
    let seconds = (cs / 100) % 60;
    format!("{minutes:02}:{seconds:02}")
}

/// format a duration in seconds as `mm:ss` or `h:mm:ss`.
pub fn duration_short(seconds: f64) -> String {
    let s = seconds.max(0.0) as u64;
    let hours = s / 3600;
    let minutes = (s / 60) % 60;
    let secs = s % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes:02}:{secs:02}")
    }
}

/// bytes for humans. 1024-based, because nobody has ever been confused by
/// two different definitions of a megabyte. (everybody has.)
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["b", "kib", "mib", "gib", "tib"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} b")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// is this a url or a path? check the scheme, not the dots. a filename with a
/// dot in it is not a website, and a windows drive letter is definitely not a
/// protocol even if it looks like one.
///
pub fn is_url(s: &str) -> bool {
    let lower = s.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// is this a youtube link? those get clickable timestamps in markdown export,
/// because that is a nice trick and it costs us three string checks.
pub fn is_youtube_url(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower.contains("youtube.com/")
        || lower.contains("youtu.be/")
        || lower.contains("youtube-nocookie.com/")
}

/// dig the video id out of a youtube url. handles the three shapes people
/// actually paste: watch, youtu.be and embed/shorts.
pub fn youtube_id(s: &str) -> Option<String> {
    fn non_empty(value: &str) -> Option<String> {
        let value = value.split(['?', '#', '/']).next()?.trim();
        (!value.is_empty()).then(|| value.to_string())
    }
    let lower = s.to_ascii_lowercase();
    if lower.contains("youtu.be/") {
        return non_empty(s.split("youtu.be/").nth(1)?);
    }
    if lower.contains("youtube.com/watch") {
        let rest = s.split('?').nth(1)?;
        for pair in rest.split('&') {
            if let Some(v) = pair.strip_prefix("v=") {
                return non_empty(v);
            }
        }
    }
    if lower.contains("youtube.com/embed/") || lower.contains("youtube.com/shorts/") {
        let marker = if lower.contains("/embed/") {
            "/embed/"
        } else {
            "/shorts/"
        };
        return non_empty(s.split(marker).nth(1)?);
    }
    None
}

/// youtube timestamp link at a whole-second offset.
pub fn youtube_link(source: &str, cs: i64) -> Option<String> {
    let id = youtube_id(source)?;
    let secs = (cs.max(0) / 100) as u64;
    Some(format!("https://www.youtube.com/watch?v={id}&t={secs}s"))
}

/// expand `~` ourselves. the shell is not involved when a user pastes a path
/// into a text box, so somebody has to do it.
pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("~"));
    }
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}

/// one path suggestion, with everything the UI needs to render and rank it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionCandidate {
    /// the text that will replace the input when accepted.
    pub value: String,
    /// just the file/directory name, for display in the dropdown.
    pub label: String,
    /// the directory portion, for rendering a dimmed path prefix.
    pub dir: String,
    pub is_dir: bool,
    /// lower is better; drives the ordering in the dropdown.
    pub score: i32,
}

/// match quality of `needle` against `haystack`, or None when it does not match.
/// lower scores are better. prefix beats substring beats fuzzy subsequence,
/// and a shorter haystack wins ties. this is the little bit of "intellisense
/// magic" that makes a partial name jump to the top of the list.
pub fn fuzzy_score(needle: &str, haystack: &str) -> Option<i32> {
    if needle.is_empty() {
        return Some(0);
    }
    let n = needle.to_ascii_lowercase();
    let h = haystack.to_ascii_lowercase();
    let hb = h.as_bytes();
    let nb = n.as_bytes();

    if h.starts_with(&n) {
        return Some(hb.len() as i32 - nb.len() as i32);
    }
    if let Some(pos) = h.find(&n) {
        return Some(100 + pos as i32 * 2 + hb.len() as i32 - nb.len() as i32);
    }

    // subsequence match: every char of needle appears in order.
    let mut ni = 0usize;
    let mut first = 0usize;
    let mut last = 0usize;
    let mut gaps = 0i32;
    for (i, &b) in hb.iter().enumerate() {
        if ni < nb.len() && b == nb[ni] {
            if ni == 0 {
                first = i;
            }
            if ni > 0 {
                gaps += (i - last - 1) as i32;
            }
            last = i;
            ni += 1;
        }
    }
    if ni == nb.len() {
        Some(300 + first as i32 * 2 + gaps + hb.len() as i32 - nb.len() as i32)
    } else {
        None
    }
}

/// media extensions we can actually feed to the decoder; give them a small
/// ranking nudge so `talk.wav` beats `talk.log` when both match.
fn extension_bonus(name: &str) -> i32 {
    let lower = name.to_ascii_lowercase();
    let media = [
        ".wav", ".mp3", ".m4a", ".flac", ".ogg", ".opus", ".webm", ".mp4", ".mkv", ".mov", ".aac",
        ".wma", ".aiff", ".aif", ".m4v", ".avi",
    ];
    if media.iter().any(|ext| lower.ends_with(ext)) {
        -20
    } else {
        0
    }
}

/// tab completion candidates, ranked. directories are kept even when the
/// prefix does not match exactly so you can keep drilling down. capped at 64
/// because nobody benefits from a list that fills the screen, and directory
/// reads are not free.
pub fn complete_path(input: &str) -> Vec<CompletionCandidate> {
    let raw = if input.starts_with('~') {
        expand_tilde(input)
    } else {
        PathBuf::from(input)
    };
    let text = raw.to_string_lossy().to_string();
    let (dir, prefix) = match text.rfind('/') {
        Some(i) => (&text[..=i], &text[i + 1..]),
        None => ("", text.as_str()),
    };
    let scan = if dir.is_empty() { "." } else { dir };
    let scan = expand_tilde(scan);
    let mut out: Vec<CompletionCandidate> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&scan) {
        let mut found: Vec<CompletionCandidate> = entries
            .filter_map(Result::ok)
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let score = match fuzzy_score(prefix, &name) {
                    Some(s) => s,
                    None => return None,
                };
                let mut value = if dir.is_empty() {
                    name.clone()
                } else {
                    format!("{dir}{name}")
                };
                if is_dir && !value.ends_with('/') {
                    value.push('/');
                }
                let bonus = if is_dir { -10 } else { 0 } + extension_bonus(&name);
                Some(CompletionCandidate {
                    value,
                    label: name,
                    dir: dir.to_string(),
                    is_dir,
                    score: score + bonus,
                })
            })
            .collect();
        found.sort_by(|a, b| {
            a.score.cmp(&b.score).then_with(|| {
                a.label
                    .to_ascii_lowercase()
                    .cmp(&b.label.to_ascii_lowercase())
            })
        });
        out.extend(found.into_iter().take(64));
    }
    out
}

/// the longest string shared by every candidate value. used for incremental
/// completion: if all matches start with `foo`, typing `f` + tab fills in `foo`
/// instead of blindly committing the first suggestion.
pub fn common_prefix(candidates: &[CompletionCandidate]) -> String {
    let Some(first) = candidates.first() else {
        return String::new();
    };
    let mut prefix = first.value.clone();
    for c in candidates.iter().skip(1) {
        let limit = prefix.len().min(c.value.len());
        let mut i = 0;
        while i < limit {
            let (a, b) = (prefix.as_bytes()[i], c.value.as_bytes()[i]);
            if a != b {
                break;
            }
            i += 1;
        }
        prefix.truncate(i);
    }
    prefix
}

/// wrap to a cell width without splitting wide glyphs. a single word longer
/// than the line is split by character, because losing text is worse than an
/// ugly break.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut result = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        let mut line_width = 0usize;
        for word in paragraph.split_whitespace() {
            let word_width = UnicodeWidthStr::width(word);
            if line_width > 0 && line_width + 1 + word_width <= width {
                line.push(' ');
                line.push_str(word);
                line_width += 1 + word_width;
                continue;
            }
            if line_width > 0 {
                result.push(std::mem::take(&mut line));
                line_width = 0;
            }
            if word_width <= width {
                line.push_str(word);
                line_width = word_width;
            } else {
                for ch in word.chars() {
                    let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                    if line_width + cw > width && !line.is_empty() {
                        result.push(std::mem::take(&mut line));
                        line_width = 0;
                    }
                    line.push(ch);
                    line_width += cw;
                }
            }
        }
        result.push(line);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

/// truncate or pad to an exact cell width. alignment is the entire point;
/// without this the columns jitter every time a word changes length.
pub fn pad_to(s: &str, width: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w >= width {
        return truncate(s, width);
    }
    format!("{s}{}", " ".repeat(width - w))
}

/// truncate to at most `width` cells, adding a single ellipsis if needed.
pub fn truncate(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(s) <= width {
        return s.to_string();
    }
    let target = width.saturating_sub(1);
    let mut out = String::new();
    let mut used = 0usize;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > target {
            break;
        }
        out.push(ch);
        used += cw;
    }
    out.push('…');
    out
}

/// base64 for osc 52. works over ssh, unlike a clipboard crate that expects
/// a local x server. that is the whole reason this helper exists.
pub fn base64(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let bits = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((bits >> 18) & 63) as usize] as char);
        out.push(TABLE[((bits >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((bits >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(bits & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// the osc 52 escape itself. it goes to the terminal, not the os clipboard,
/// which is why it keeps working when you are ssh-ed into a box with no gui.
pub fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

/// where an export lands: beside the source, or in `--out` if you set it.
pub fn output_path(source: &str, out_dir: Option<&Path>, ext: &str) -> PathBuf {
    let stem = if is_url(source) {
        "transcript".to_string()
    } else {
        Path::new(source)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "transcript".to_string())
    };
    let file = format!("{stem}.{ext}");
    match out_dir {
        Some(dir) if !dir.as_os_str().is_empty() => dir.join(file),
        _ if is_url(source) => PathBuf::from(file),
        _ => Path::new(source).with_file_name(file),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps() {
        assert_eq!(timestamp_cs(0), "00:00:00.000");
        assert_eq!(timestamp_cs(1234), "00:00:12.340");
        assert_eq!(timestamp_cs(3_661_234), "10:10:12.340");
        assert_eq!(timestamp_short(1234), "00:12");
        assert_eq!(timestamp_short(3_661_234), "610:12");
    }

    #[test]
    fn duration() {
        assert_eq!(duration_short(0.0), "00:00");
        assert_eq!(duration_short(65.0), "01:05");
        assert_eq!(duration_short(3661.0), "1:01:01");
    }

    #[test]
    fn byte_units() {
        assert_eq!(human_bytes(512), "512 b");
        assert_eq!(human_bytes(2048), "2.0 kib");
    }

    #[test]
    fn url_detection() {
        assert!(is_url("https://example.com/a"));
        assert!(is_url("HTTP://EXAMPLE.COM"));
        assert!(!is_url("/tmp/a.wav"));
        assert!(!is_url(r"c:\audio\file.wav"));
        // schemes that look like urls but are not http(s) are paths
        assert!(!is_url("ftp://example.com/a"));
        assert!(!is_url("file:///tmp/a.wav"));
        assert!(!is_url("not a url"));
    }

    #[test]
    fn invalid_youtube_urls_do_not_produce_links() {
        assert!(youtube_id("https://youtu.be/").is_none());
        assert!(youtube_id("https://youtube.com/watch?x=1").is_none());
        assert!(youtube_link("https://example.com/a", 0).is_none());
    }

    #[test]
    fn fuzzy_ranking() {
        let exact = fuzzy_score("foo", "foobar.wav").unwrap();
        let sub = fuzzy_score("foo", "xfoobar.wav").unwrap();
        let fuzzy = fuzzy_score("foo", "f_a_o_o.wav").unwrap();
        assert!(exact < sub && sub < fuzzy);
        assert!(fuzzy_score("ab", "abc").unwrap() < fuzzy_score("ab", "abcdef").unwrap());
        assert_eq!(fuzzy_score("zzz", "foobar.wav"), None);
        assert_eq!(fuzzy_score("", "anything"), Some(0));
    }

    #[test]
    fn shared_prefix() {
        use super::CompletionCandidate as C;
        let list = vec![
            C {
                value: "foobar.wav".into(),
                label: "a".into(),
                dir: "".into(),
                is_dir: false,
                score: 0,
            },
            C {
                value: "foobaz.wav".into(),
                label: "b".into(),
                dir: "".into(),
                is_dir: false,
                score: 0,
            },
        ];
        assert_eq!(common_prefix(&list), "fooba");
        assert_eq!(common_prefix(&[]), "");
    }

    #[test]
    fn complete_ranks_and_marks_dirs() {
        let dir = std::env::temp_dir().join(format!("scribe_cmpl_{}", std::process::id()));
        let _ = std::fs::create_dir_all(dir.join("subdir"));
        let _ = std::fs::write(dir.join("talk.wav"), b"x");
        let _ = std::fs::write(dir.join("talk.log"), b"x");

        let input = format!("{}/tal", dir.display());
        let list = complete_path(&input);
        assert!(!list.is_empty());
        let wav = list
            .iter()
            .position(|c| c.value.ends_with("talk.wav"))
            .unwrap();
        let log = list
            .iter()
            .position(|c| c.value.ends_with("talk.log"))
            .unwrap();
        assert!(wav < log);

        let dirs = complete_path(&format!("{}/sub", dir.display()));
        assert!(dirs[0].is_dir);
        assert!(dirs[0].value.ends_with('/'));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn youtube_links() {
        assert_eq!(youtube_id("https://youtu.be/abc123"), Some("abc123".into()));
        assert_eq!(
            youtube_id("https://youtube.com/watch?v=xyz&t=4"),
            Some("xyz".into())
        );
        assert!(is_youtube_url("https://youtu.be/abc123"));
        assert_eq!(
            youtube_link("https://youtu.be/abc123", 6500),
            Some("https://www.youtube.com/watch?v=abc123&t=65s".into())
        );
        assert_eq!(youtube_link("https://example.com/a", 0), None);
    }

    #[test]
    fn wrapping_unicode() {
        let lines = wrap("你好 世界 test", 8);
        assert!(
            lines
                .iter()
                .all(|l| UnicodeWidthStr::width(l.as_str()) <= 8)
        );
        assert!(wrap("", 5) == vec![""]);
    }

    #[test]
    fn truncate_and_pad() {
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(pad_to("ab", 5), "ab   ");
        assert_eq!(truncate("ab", 5), "ab");
    }

    #[test]
    fn base64_osc52() {
        assert_eq!(base64(b"hello"), "aGVsbG8=");
        assert_eq!(base64(b""), "");
        assert!(osc52("hi").starts_with("\x1b]52;c;aGk=\x07"));
    }

    #[test]
    fn output_naming() {
        let out = output_path("/tmp/meeting.wav", None, "srt");
        assert_eq!(out, PathBuf::from("/tmp/meeting.srt"));
        let out = output_path("https://x/y", None, "txt");
        assert_eq!(out, PathBuf::from("transcript.txt"));
    }
}
