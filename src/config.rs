//! persistent user preferences. command-line arguments always take priority.

use crate::app::App;
use crate::cli::Args;
use crate::export::Format;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Preferences {
    pub model: Option<String>,
    pub threads: Option<i32>,
    pub lang: Option<String>,
    pub out: Option<String>,
    pub format: Option<Format>,
    pub theme: Option<String>,
    pub translate: Option<bool>,
}

pub fn path() -> PathBuf {
    if let Some(root) = std::env::var_os("XDG_CONFIG_HOME").filter(|p| !p.is_empty()) {
        return PathBuf::from(root).join("scribe").join("config");
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into()))
        .join(".config")
        .join("scribe")
        .join("config")
}

fn unescape(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('\\') => result.push('\\'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\n', "\\n")
}

impl Preferences {
    pub fn load() -> Self {
        let Ok(text) = fs::read_to_string(path()) else {
            return Self::default();
        };
        let mut prefs = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = unescape(value.trim());
            match key.trim() {
                "model" if !value.is_empty() => prefs.model = Some(value),
                "threads" => {
                    if let Ok(n) = value.parse::<i32>()
                        && (1..=256).contains(&n)
                    {
                        prefs.threads = Some(n);
                    }
                }
                "lang" if !value.is_empty() => prefs.lang = Some(value),
                "out" if !value.is_empty() => prefs.out = Some(value),
                "format" => prefs.format = Format::parse(&value),
                "theme" if !value.is_empty() => prefs.theme = Some(value),
                "translate" => prefs.translate = value.parse::<bool>().ok(),
                _ => {}
            }
        }
        prefs
    }

    pub fn from_app(app: &App) -> Self {
        Self {
            model: (!app.model_name.is_empty() && app.model_label != "-")
                .then(|| app.model_name.clone()),
            threads: Some(app.threads),
            lang: Some(app.lang.clone()),
            out: Some(app.out_dir.clone()),
            format: Some(app.format),
            theme: Some(app.theme.key().to_string()),
            translate: Some(app.translate),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = path();
        let parent = path.parent().ok_or("config path has no parent directory")?;
        fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create config directory {}: {e}", parent.display()))?;
        let mut contents =
            String::from("# scribe preferences; command-line options override these values\n");
        if let Some(v) = &self.model {
            contents.push_str(&format!("model={}\n", escape(v)));
        }
        if let Some(v) = self.threads {
            contents.push_str(&format!("threads={v}\n"));
        }
        if let Some(v) = &self.lang {
            contents.push_str(&format!("lang={}\n", escape(v)));
        }
        if let Some(v) = &self.out {
            contents.push_str(&format!("out={}\n", escape(v)));
        }
        if let Some(v) = self.format {
            contents.push_str(&format!("format={}\n", v.extension()));
        }
        if let Some(v) = &self.theme {
            contents.push_str(&format!("theme={}\n", escape(v)));
        }
        if let Some(v) = self.translate {
            contents.push_str(&format!("translate={v}\n"));
        }

        let temp = path.with_extension("config.tmp");
        fs::write(&temp, contents)
            .map_err(|e| format!("cannot write config {}: {e}", temp.display()))?;
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| format!("cannot replace config {}: {e}", path.display()))?;
        }
        fs::rename(&temp, &path)
            .map_err(|e| format!("cannot install config {}: {e}", path.display()))
    }
}

pub fn apply(args: &mut Args) {
    let prefs = Preferences::load();
    if args.model.is_none() {
        args.model = prefs.model.map(PathBuf::from);
    }
    if args.threads.is_none() {
        args.threads = prefs.threads;
    }
    if args.lang.is_none() {
        args.lang = prefs.lang;
    }
    if args.translate.is_none() {
        args.translate = prefs.translate;
    }
    if args.out.is_none() {
        args.out = prefs.out.map(PathBuf::from);
    }
    if !args.format_explicit
        && let Some(format) = prefs.format
    {
        args.format = format;
    }
    if args.theme.is_none() {
        args.theme = prefs.theme;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_round_trips_paths_and_newlines() {
        let value = "folder\\name\nsecond line";
        assert_eq!(unescape(&escape(value)), value);
    }
}
