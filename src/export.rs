//! the transcript record and the five ways to write it out.
//!
//! pure formatting, no file handles. subtitle cues are built from one ordered
//! list so segment-level and word-level output never overlap or duplicate.

use crate::util::{is_youtube_url, timestamp_cs, youtube_link};
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    pub start_cs: i64,
    pub end_cs: i64,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub start_cs: i64,
    pub end_cs: i64,
    pub text: String,
    pub words: Vec<Word>,
}

impl Segment {
    pub fn new(start_cs: i64, end_cs: i64, text: String) -> Self {
        Self {
            start_cs,
            end_cs,
            text,
            words: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Txt,
    Srt,
    Vtt,
    Md,
    Json,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Txt => "txt",
            Format::Srt => "srt",
            Format::Vtt => "vtt",
            Format::Md => "md",
            Format::Json => "json",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "txt" | "text" => Some(Self::Txt),
            "srt" => Some(Self::Srt),
            "vtt" | "webvtt" => Some(Self::Vtt),
            "md" | "markdown" => Some(Self::Md),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
}

fn srt_time(cs: i64) -> String {
    timestamp_cs(cs).replace('.', ",")
}

/// subtitle cue with centisecond timestamps and a single line of text.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Cue {
    start_cs: i64,
    end_cs: i64,
    text: String,
}

/// Build the cue list for subtitle formats. When a segment carries word
/// timings, those words become the cues; otherwise the segment itself is one
/// cue. The result is sorted by start time, free of zero-length cues, and
/// clamped so no cue overlaps the previous one.
fn cues(segments: &[Segment]) -> Vec<Cue> {
    let mut out: Vec<Cue> = Vec::new();
    for seg in segments {
        if !seg.words.is_empty() {
            for word in &seg.words {
                let text = word.text.trim();
                if !text.is_empty() && word.end_cs > word.start_cs {
                    out.push(Cue {
                        start_cs: word.start_cs,
                        end_cs: word.end_cs,
                        text: clean_cue_text(text),
                    });
                }
            }
        } else {
            let text = seg.text.trim();
            if !text.is_empty() && seg.end_cs > seg.start_cs {
                out.push(Cue {
                    start_cs: seg.start_cs,
                    end_cs: seg.end_cs,
                    text: clean_cue_text(text),
                });
            }
        }
    }
    out.sort_by(|a, b| a.start_cs.cmp(&b.start_cs).then(a.end_cs.cmp(&b.end_cs)));
    for i in 1..out.len() {
        if out[i].start_cs < out[i - 1].end_cs {
            out[i].start_cs = out[i - 1].end_cs;
        }
    }
    out.retain(|c| c.end_cs > c.start_cs);
    out
}

/// Subtitle text must stay on one line and cannot contain a blank line or a
/// timing arrow. Newlines inside a whisper segment are flattened to spaces.
fn clean_cue_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace("-->", "→")
}

pub fn to_txt(segments: &[Segment]) -> String {
    let mut text = String::new();
    for seg in segments {
        let _ = writeln!(
            text,
            "{} {}",
            crate::util::timestamp_short(seg.start_cs),
            seg.text
        );
    }
    text
}

pub fn to_srt(segments: &[Segment]) -> String {
    let cues = cues(segments);
    let mut text = String::new();
    for (i, cue) in cues.iter().enumerate() {
        let _ = writeln!(text, "{}", i + 1);
        let _ = writeln!(
            text,
            "{} --> {}",
            srt_time(cue.start_cs),
            srt_time(cue.end_cs)
        );
        let _ = writeln!(text, "{}", cue.text);
        let _ = writeln!(text);
    }
    text
}

pub fn to_vtt(segments: &[Segment]) -> String {
    let cues = cues(segments);
    let mut text = String::from("WEBVTT\n\n");
    for cue in &cues {
        let _ = writeln!(
            text,
            "{} --> {}",
            timestamp_cs(cue.start_cs),
            timestamp_cs(cue.end_cs)
        );
        let _ = writeln!(text, "{}", cue.text);
        let _ = writeln!(text);
    }
    text
}

pub fn to_md(segments: &[Segment], source: Option<&str>) -> String {
    let mut text = String::new();
    let link_for = |cs: i64| {
        source
            .filter(|s| is_youtube_url(s))
            .and_then(|s| youtube_link(s, cs))
    };
    for seg in segments {
        let stamp = crate::util::timestamp_short(seg.start_cs);
        if let Some(link) = link_for(seg.start_cs) {
            let _ = writeln!(text, "- [{stamp}]({link}) {}", seg.text);
        } else {
            let _ = writeln!(text, "- {stamp} {}", seg.text);
        }
        if !seg.words.is_empty() {
            let mut line = String::from("  ");
            for word in &seg.words {
                match link_for(word.start_cs) {
                    Some(link) => {
                        let _ = write!(line, "[{}]({link}) ", word.text);
                    }
                    None => {
                        let _ = write!(line, "{} ", word.text);
                    }
                }
            }
            let _ = writeln!(text, "{}", line.trim_end());
        }
    }
    text
}

/// small json writer. i hand-rolled it because dragging serde in for five
/// fields would be like renting a truck to move a chair. escape codes below
/// are the whole reason this function exists and is tested.
pub fn to_json(segments: &[Segment]) -> String {
    let mut text = String::from("[\n");
    for (i, seg) in segments.iter().enumerate() {
        let words = seg
            .words
            .iter()
            .map(|w| {
                format!(
                    "{{\"start\": {}, \"end\": {}, \"text\": \"{}\"}}",
                    w.start_cs,
                    w.end_cs,
                    json_escape(&w.text)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(
            text,
            "  {{\"start\": {}, \"end\": {}, \"text\": \"{}\", \"words\": [{}]}}",
            seg.start_cs,
            seg.end_cs,
            json_escape(&seg.text),
            words
        );
        if i + 1 < segments.len() {
            text.push(',');
        }
        text.push('\n');
    }
    text.push_str("]\n");
    text
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

pub fn export(format: Format, segments: &[Segment], source: Option<&str>) -> String {
    match format {
        Format::Txt => to_txt(segments),
        Format::Srt => to_srt(segments),
        Format::Vtt => to_vtt(segments),
        Format::Md => to_md(segments, source),
        Format::Json => to_json(segments),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Segment> {
        vec![
            Segment::new(0, 250, "hello world".into()),
            Segment::new(250, 415, "a \"quote\" and\ttab".into()),
        ]
    }

    fn word(start_cs: i64, end_cs: i64, text: &str) -> Word {
        Word {
            start_cs,
            end_cs,
            text: text.into(),
        }
    }

    #[test]
    fn srt_format() {
        let s = to_srt(&sample());
        assert!(s.contains("00:00:00,000 --> 00:00:02,500"));
        assert!(s.contains("2\n00:00:02,500 --> 00:00:04,150"));
    }

    #[test]
    fn vtt_format() {
        let s = to_vtt(&sample());
        assert!(s.starts_with("WEBVTT\n\n"));
        assert!(s.contains("00:00:00.000 --> 00:00:02.500"));
    }

    #[test]
    fn md_links_youtube_only() {
        let s = to_md(&sample(), Some("https://youtu.be/abc123"));
        assert!(s.contains("[00:00](https://www.youtube.com/watch?v=abc123&t=0s)"));
        assert!(s.contains("&t=2s"));
        let plain = to_md(&sample(), Some("/tmp/a.wav"));
        assert!(!plain.contains("]("));
    }

    #[test]
    fn json_escapes() {
        let s = to_json(&sample());
        assert!(s.contains("a \\\"quote\\\" and\\ttab"));
        assert!(s.starts_with("[\n"));
    }

    #[test]
    fn format_parse() {
        assert_eq!(Format::parse("SRT"), Some(Format::Srt));
        assert_eq!(Format::parse("webvtt"), Some(Format::Vtt));
        assert_eq!(Format::parse("nope"), None);
    }

    #[test]
    fn word_timestamps_export() {
        let mut seg = Segment::new(0, 100, "hello world".into());
        seg.words = vec![word(0, 40, "hello"), word(45, 100, "world")];
        let srt = to_srt(&[seg.clone()]);
        assert!(srt.contains("00:00:00,000 --> 00:00:00,400\nhello"));
        assert!(srt.contains("00:00:00,450 --> 00:00:01,000\nworld"));
        // only word cues: no overlapping segment cue is emitted
        assert!(!srt.contains("00:00:01,000\nhello world"));
        let vtt = to_vtt(&[seg.clone()]);
        assert!(!vtt.contains("note word timings"));
        assert!(vtt.contains("00:00:00.000 --> 00:00:00.400\nhello"));
        let json = to_json(&[seg.clone()]);
        assert!(json.contains("\"words\": [{\"start\": 0, \"end\": 40, \"text\": \"hello\"}"));
        let md = to_md(&[seg], Some("https://youtu.be/abc123"));
        assert!(md.contains("[hello](https://www.youtube.com/watch?v=abc123&t=0s)"));
        assert!(md.contains("[world](https://www.youtube.com/watch?v=abc123&t=0s)"));
    }

    #[test]
    fn cues_are_sorted_clamped_and_deduplicated() {
        let segments = vec![
            Segment {
                start_cs: 300,
                end_cs: 500,
                text: "third".into(),
                words: vec![],
            },
            Segment {
                start_cs: 0,
                end_cs: 200,
                text: "first\nline".into(),
                words: vec![],
            },
            Segment {
                start_cs: 150,
                end_cs: 350,
                text: "second".into(),
                words: vec![],
            },
        ];
        let c = cues(&segments);
        assert_eq!(c.len(), 3);
        assert_eq!(c[0].start_cs, 0);
        assert!(c[0].text == "first line", "text should be one line");
        assert!(c[1].start_cs >= c[0].end_cs, "no overlap");
        assert!(c[2].start_cs >= c[1].end_cs, "no overlap");
    }

    #[test]
    fn empty_and_invalid_cues_are_skipped() {
        let segments = vec![
            Segment::new(0, 100, "".into()),
            Segment::new(100, 100, "zero length".into()),
            Segment::new(200, 150, "backwards".into()),
        ];
        assert!(cues(&segments).is_empty());
        assert_eq!(to_srt(&segments), "");
        assert_eq!(to_vtt(&segments), "WEBVTT\n\n");
    }

    #[test]
    fn subtitle_text_escapes_arrows() {
        let segments = vec![Segment::new(0, 100, "a --> b".into())];
        let srt = to_srt(&segments);
        assert!(srt.contains("a → b"));
        assert!(
            !srt.contains("a --> b\n"),
            "arrow in text must be sanitized"
        );
    }

    #[test]
    fn long_recording_timestamps_have_hours() {
        let s = to_srt(&[Segment::new(3_661_234, 3_661_334, "late".into())]);
        assert!(s.contains("10:10:12,340 --> 10:10:13,340"));
    }
}
