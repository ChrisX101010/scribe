//! argument parsing, by hand. clap is excellent, but a dependency that reads
//! twelve flags is a dependency i have to explain in the readme.

use crate::export::Format;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args {
    pub input: Option<String>,
    pub model: Option<PathBuf>,
    pub threads: Option<i32>,
    pub lang: Option<String>,
    pub translate: Option<bool>,
    pub out: Option<PathBuf>,
    pub format: Format,
    pub format_explicit: bool,
    pub no_tui: bool,
    pub theme: Option<String>,
    pub help: bool,
    pub version: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            input: None,
            model: None,
            threads: None,
            lang: None,
            translate: None,
            out: None,
            format: Format::Txt,
            format_explicit: false,
            no_tui: false,
            theme: None,
            help: false,
            version: false,
        }
    }
}

/// help is data, not a heredoc with hand-counted spaces. the renderer colors
/// each role from the active theme; piped output stays plain.
pub struct HelpLine {
    pub key: &'static str,
    pub desc: &'static str,
}

pub struct HelpSection {
    pub title: &'static str,
    pub lines: &'static [HelpLine],
}

pub const HELP_TITLE: &str = "scribe - local, offline, cpu-only speech to text";
pub const HELP_USAGE: &str = "usage: scribe [input] [options]";
pub const HELP_INTRO: &str = "input may be a local audio/video file or an http(s) url.";

pub const HELP_SECTIONS: &[HelpSection] = &[
    HelpSection {
        title: "options",
        lines: &[
            HelpLine {
                key: "--model <name|path>",
                desc: "ggml model name or path (default: small-q5_1; 33 models, see m in the tui)",
            },
            HelpLine {
                key: "--threads <n>",
                desc: "cpu threads for whisper (default: min(cores, 8))",
            },
            HelpLine {
                key: "--lang <code>",
                desc: "language code, or auto (default: auto)",
            },
            HelpLine {
                key: "--translate",
                desc: "translate speech to english",
            },
            HelpLine {
                key: "--out <dir>",
                desc: "directory for exported transcripts",
            },
            HelpLine {
                key: "--format <fmt>",
                desc: "txt, srt, vtt, md, json (default: txt)",
            },
            HelpLine {
                key: "--theme <name>",
                desc: "22 themes; try dark, light, nord, tokyo-night, gruvbox, dracula, contrast",
            },
            HelpLine {
                key: "--no-tui",
                desc: "transcript to stdout, for scripts",
            },
            HelpLine {
                key: "--version",
                desc: "print version and exit",
            },
            HelpLine {
                key: "--help",
                desc: "print this help and exit",
            },
        ],
    },
    HelpSection {
        title: "keybinds (tui)",
        lines: &[
            HelpLine {
                key: "enter",
                desc: "start transcription / accept completion / resume when paused",
            },
            HelpLine {
                key: "tab",
                desc: "switch focus; fill or accept a path completion",
            },
            HelpLine {
                key: "up / down",
                desc: "move through transcript or completion list",
            },
            HelpLine {
                key: "ctrl+space",
                desc: "force the completion list open",
            },
            HelpLine {
                key: "esc",
                desc: "pause active job; press again to stop",
            },
            HelpLine {
                key: "space / p",
                desc: "pause or resume the active job",
            },
            HelpLine {
                key: "c",
                desc: "stop active job, keeping saved segments",
            },
            HelpLine {
                key: "/",
                desc: "live search",
            },
            HelpLine {
                key: "n / N",
                desc: "next / previous match",
            },
            HelpLine {
                key: "j / k",
                desc: "move selection",
            },
            HelpLine {
                key: "page up/down",
                desc: "scroll ten rows",
            },
            HelpLine {
                key: "e",
                desc: "export menu",
            },
            HelpLine {
                key: "y",
                desc: "copy selected segment (osc 52, works over ssh)",
            },
            HelpLine {
                key: "s",
                desc: "settings overlay",
            },
            HelpLine {
                key: "m",
                desc: "model picker",
            },
            HelpLine {
                key: "t",
                desc: "theme picker (live preview)",
            },
            HelpLine {
                key: "?",
                desc: "help overlay",
            },
            HelpLine {
                key: "ctrl+c",
                desc: "quit",
            },
        ],
    },
];

pub const HELP_FOOTER: &str = "no manual setup: yt-dlp and ffmpeg install themselves on first url use.\nmodels download from hugging face, verified by sha256, when you pick them.";

fn write_styled(
    out: &mut impl std::io::Write,
    text: &str,
    color: crossterm::style::Color,
    bold: bool,
    colorize: bool,
) -> std::io::Result<()> {
    use crossterm::style::{Attribute, ResetColor, SetAttribute, SetForegroundColor};
    if colorize {
        crossterm::queue!(out, SetForegroundColor(color))?;
        if bold {
            crossterm::queue!(out, SetAttribute(Attribute::Bold))?;
        }
    }
    write!(out, "{text}")?;
    if colorize {
        crossterm::queue!(out, ResetColor, SetAttribute(Attribute::Reset))?;
    }
    Ok(())
}

/// print help in the configured palette when stdout is a terminal. the plain
/// path is byte-for-byte stable for scripts and `NO_COLOR`.
pub fn render_help(w: &mut impl std::io::Write, palette: &crate::theme::Palette, colorize: bool) {
    use crossterm::style::Color as Ct;
    let ct = |c: ratatui::style::Color| crate::theme::to_crossterm(c);
    let title_color = if colorize {
        ct(palette.accent)
    } else {
        Ct::Reset
    };
    let head_color = if colorize {
        ct(palette.secondary_accent)
    } else {
        Ct::Reset
    };
    let key_color = if colorize {
        ct(palette.success)
    } else {
        Ct::Reset
    };
    let desc_color = if colorize {
        ct(palette.subtext0)
    } else {
        Ct::Reset
    };
    let muted_color = if colorize {
        ct(palette.muted)
    } else {
        Ct::Reset
    };

    let _ = write_styled(w, HELP_TITLE, title_color, true, colorize);
    let _ = writeln!(w);
    let _ = writeln!(w);
    let _ = write_styled(w, HELP_USAGE, head_color, false, colorize);
    let _ = writeln!(w);
    let _ = writeln!(w);
    let _ = write_styled(w, HELP_INTRO, muted_color, false, colorize);
    let _ = writeln!(w);
    let _ = writeln!(w);

    let key_width = HELP_SECTIONS
        .iter()
        .flat_map(|s| s.lines)
        .map(|l| l.key.len())
        .max()
        .unwrap_or(0)
        + 2;

    for section in HELP_SECTIONS {
        let _ = write_styled(
            w,
            &format!("{}:", section.title),
            head_color,
            true,
            colorize,
        );
        let _ = writeln!(w);
        for line in section.lines {
            let _ = write!(w, "  ");
            let _ = write_styled(
                w,
                &format!("{:<key_width$}", line.key),
                key_color,
                false,
                colorize,
            );
            let _ = write_styled(w, line.desc, desc_color, false, colorize);
            let _ = writeln!(w);
        }
        let _ = writeln!(w);
    }

    let _ = write_styled(w, HELP_FOOTER, muted_color, false, colorize);
    let _ = writeln!(w);
}

/// compact, one-line help for the parse-error path.
pub fn hint() -> &'static str {
    "run scribe --help for the full option list"
}

fn need_value(args: &[String], i: &mut usize, flag: &str) -> Result<String, String> {
    *i += 1;
    args.get(*i)
        .cloned()
        .ok_or_else(|| format!("{flag} is missing its value"))
}

pub fn parse(argv: &[String]) -> Result<Args, String> {
    let mut out = Args::default();
    let mut i = 0;
    while i < argv.len() {
        let arg = &argv[i];
        match arg.as_str() {
            "--help" | "-h" => out.help = true,
            "--version" | "-V" => out.version = true,
            "--no-tui" => out.no_tui = true,
            "--translate" => out.translate = Some(true),
            "--model" | "-m" => {
                let value = need_value(argv, &mut i, arg)?;
                if value.starts_with('-') {
                    return Err(format!("{arg} is missing its value"));
                }
                out.model = Some(PathBuf::from(value));
            }
            "--threads" | "-t" => {
                let v = need_value(argv, &mut i, arg)?;
                if v.starts_with('-') {
                    return Err(format!("{arg} is missing its value"));
                }
                let threads = v
                    .parse::<i32>()
                    .map_err(|_| format!("threads must be a number, not '{v}'"))?;
                out.threads = Some(threads);
            }
            "--lang" | "-l" => {
                let value = need_value(argv, &mut i, arg)?;
                if value.starts_with('-') {
                    return Err(format!("{arg} is missing its value"));
                }
                out.lang = Some(value);
            }
            "--out" | "-o" => {
                let value = need_value(argv, &mut i, arg)?;
                if value.starts_with('-') {
                    return Err(format!("{arg} is missing its value"));
                }
                out.out = Some(PathBuf::from(value));
            }
            "--format" | "-f" => {
                let v = need_value(argv, &mut i, arg)?;
                if v.starts_with('-') {
                    return Err(format!("{arg} is missing its value"));
                }
                out.format = Format::parse(&v).ok_or_else(|| {
                    format!("unknown format '{v}'. try txt, srt, vtt, md or json")
                })?;
                out.format_explicit = true;
            }
            "--theme" => {
                let value = need_value(argv, &mut i, arg)?;
                if value.starts_with('-') {
                    return Err(format!("{arg} is missing its value"));
                }
                out.theme = Some(value);
            }
            "--" => {
                for rest in &argv[i + 1..] {
                    if out.input.is_none() {
                        out.input = Some(rest.clone());
                    }
                }
                break;
            }
            other if other.starts_with('-') && other != "-" => {
                return Err(format!("unknown option '{other}'. try --help"));
            }
            other => {
                if out.input.is_none() {
                    out.input = Some(other.to_string());
                } else {
                    return Err(format!("i only take one input. '{other}' is one too many"));
                }
            }
        }
        i += 1;
    }
    if let Some(n) = out.threads
        && !(1..=256).contains(&n)
    {
        return Err("threads must be between 1 and 256. pick something sane".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(v: &[&str]) -> Result<Args, String> {
        parse(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn basic() {
        let args = a(&["/tmp/a.wav", "--threads", "4", "--format", "srt"]).unwrap();
        assert_eq!(args.input.as_deref(), Some("/tmp/a.wav"));
        assert_eq!(args.threads, Some(4));
        assert_eq!(args.format, Format::Srt);
    }

    #[test]
    fn flags() {
        let args = a(&["--no-tui", "--version", "https://example.com/a"]).unwrap();
        assert!(args.no_tui && args.version);
        assert_eq!(args.input.as_deref(), Some("https://example.com/a"));
    }

    #[test]
    fn errors() {
        assert!(a(&["--threads"]).is_err());
        assert!(a(&["--threads", "x"]).is_err());
        assert!(a(&["--threads", "0"]).is_err());
        assert!(a(&["--threads", "257"]).is_err());
        assert!(a(&["--format", "nope"]).is_err());
        assert!(a(&["--unknown"]).is_err());
        assert!(a(&["a", "b"]).is_err());
        assert!(a(&["--model"]).is_err());
        assert!(a(&["--lang"]).is_err());
        assert!(a(&["--out"]).is_err());
        assert!(a(&["--theme"]).is_err());
    }

    #[test]
    fn all_options_parse() {
        let args = a(&[
            "audio.wav",
            "--model",
            "small-q5_1",
            "--threads",
            "8",
            "--lang",
            "de",
            "--translate",
            "--out",
            "/tmp/out",
            "--format",
            "vtt",
            "--theme",
            "nord",
            "--no-tui",
        ])
        .unwrap();
        assert_eq!(args.input.as_deref(), Some("audio.wav"));
        assert_eq!(args.model.unwrap().to_str(), Some("small-q5_1"));
        assert_eq!(args.threads, Some(8));
        assert_eq!(args.lang.as_deref(), Some("de"));
        assert!(args.translate.unwrap());
        assert_eq!(args.out.unwrap().to_str(), Some("/tmp/out"));
        assert_eq!(args.format, Format::Vtt);
        assert!(args.format_explicit);
        assert_eq!(args.theme.as_deref(), Some("nord"));
        assert!(args.no_tui);
    }

    #[test]
    fn separator_accepts_dashed_input() {
        let args = a(&["--", "-weird-name.wav"]).unwrap();
        assert_eq!(args.input.as_deref(), Some("-weird-name.wav"));
    }
}
