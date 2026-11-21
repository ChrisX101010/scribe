//! the transcript record and the five ways to write it out.
//!
//! pure formatting, no file handles. this is why i can test subtitle timing
//! without creating a single temp file, and why export bugs stay in this file.

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
    let mut text = String::new();
    let mut n = 0;
    for seg in segments {
        n += 1;
        let _ = writeln!(text, "{n}");
        let _ = writeln!(
            text,
            "{} --> {}",
            srt_time(seg.start_cs),
            srt_time(seg.end_cs)
        );
        let _ = writeln!(text, "{}", seg.text);
        let _ = writeln!(text);
        for word in &seg.words {
            if word.end_cs > word.start_cs {
                n += 1;
                let _ = writeln!(text, "{n}");
                let _ = writeln!(
                    text,
                    "{} --> {}",
                    srt_time(word.start_cs),
                    srt_time(word.end_cs)
                );
                let _ = writeln!(text, "{}", word.text);
                let _ = writeln!(text);
            }
        }
    }
    text
}

pub fn to_vtt(segments: &[Segment]) -> String {
    let mut text = String::from("webvtt\n\n");
    for seg in segments {
        let _ = writeln!(
            text,
            "{} --> {}",
            timestamp_cs(seg.start_cs),
            timestamp_cs(seg.end_cs)
        );
        let _ = writeln!(text, "{}", seg.text);
        let _ = writeln!(text);
        if !seg.words.is_empty() {
            text.push_str("note word timings\n\n");
            for word in &seg.words {
                if word.end_cs > word.start_cs {
                    let _ = writeln!(
                        text,
                        "{} --> {}",
                        timestamp_cs(word.start_cs),
                        timestamp_cs(word.end_cs)
                    );
                    let _ = writeln!(text, "{}", word.text);
                    let _ = writeln!(text);
                }
            }
        }
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

    #[test]
    fn srt_format() {
        let s = to_srt(&sample());
        assert!(s.contains("00:00:00,000 --> 00:00:02,500"));
        assert!(s.contains("2\n00:00:02,500 --> 00:00:04,150"));
    }

    #[test]
    fn vtt_format() {
        let s = to_vtt(&sample());
        assert!(s.starts_with("webvtt\n\n"));
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
        seg.words = vec![
            Word {
                start_cs: 0,
                end_cs: 40,
                text: "hello".into(),
            },
            Word {
                start_cs: 45,
                end_cs: 100,
                text: "world".into(),
            },
        ];
        let srt = to_srt(&[seg.clone()]);
        assert!(srt.contains("00:00:00,000 --> 00:00:00,400\nhello"));
        assert!(srt.contains("00:00:00,450 --> 00:00:01,000\nworld"));
        let vtt = to_vtt(&[seg.clone()]);
        assert!(vtt.contains("note word timings"));
        let json = to_json(&[seg.clone()]);
        assert!(json.contains("\"words\": [{\"start\": 0, \"end\": 40, \"text\": \"hello\"}"));
        let md = to_md(&[seg], Some("https://youtu.be/abc123"));
        assert!(md.contains("[hello](https://www.youtube.com/watch?v=abc123&t=0s)"));
        assert!(md.contains("[world](https://www.youtube.com/watch?v=abc123&t=0s)"));
    }
}
