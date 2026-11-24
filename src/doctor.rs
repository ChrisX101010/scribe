//! read-only readiness check. no installs, no writes, no surprises.

use crate::{config, fetch, model, util, worker};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    Warn,
    Fail,
}

impl Verdict {
    fn label(self) -> &'static str {
        match self {
            Verdict::Ok => "ok",
            Verdict::Warn => "warn",
            Verdict::Fail => "fail",
        }
    }
}

struct Row {
    label: &'static str,
    verdict: Verdict,
    detail: String,
}

fn version(binary: &std::path::Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(binary)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .next()
        .map(|line| line.trim().chars().take(80).collect())
}

pub fn run(selected_model: Option<&str>) -> i32 {
    let mut rows = Vec::new();
    rows.push(Row {
        label: "scribe",
        verdict: Verdict::Ok,
        detail: env!("CARGO_PKG_VERSION").into(),
    });
    rows.push(Row {
        label: "os / arch",
        verdict: Verdict::Ok,
        detail: format!("{} / {}", std::env::consts::OS, std::env::consts::ARCH),
    });

    match worker::resolve_ffmpeg() {
        Ok(path) => {
            let found = version(&path, &["-version"]);
            let verdict = if found.is_some() {
                Verdict::Ok
            } else {
                Verdict::Fail
            };
            let detail = found
                .map(|v| format!("{} · {}", path.display(), v))
                .unwrap_or_else(|| format!("{} (self check failed)", path.display()));
            rows.push(Row {
                label: "ffmpeg (required)",
                verdict,
                detail,
            });
        }
        Err(e) => rows.push(Row {
            label: "ffmpeg (required)",
            verdict: Verdict::Fail,
            detail: e,
        }),
    }

    let managed = fetch::libs_dir();
    let ytdlp = managed.join(if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    });
    if ytdlp.is_file() {
        let detail =
            version(&ytdlp, &["--version"]).unwrap_or_else(|| "installed, version unknown".into());
        rows.push(Row {
            label: "yt-dlp (url only)",
            verdict: Verdict::Ok,
            detail,
        });
    } else {
        rows.push(Row {
            label: "yt-dlp (url only)",
            verdict: Verdict::Warn,
            detail: "not installed; scribe fetches it on first url use".into(),
        });
    }
    rows.push(Row {
        label: "managed tools",
        verdict: if fetch::tools_ready() {
            Verdict::Ok
        } else {
            Verdict::Warn
        },
        detail: managed.display().to_string(),
    });

    let cache = model::cache_dir();
    let cache_ok = cache
        .parent()
        .map(|p| p.is_dir())
        .unwrap_or_else(|| cache.is_dir());
    rows.push(Row {
        label: "model cache",
        verdict: if cache_ok { Verdict::Ok } else { Verdict::Fail },
        detail: format!(
            "{} ({})",
            cache.display(),
            if cache_ok { "writable" } else { "not writable" }
        ),
    });

    let chosen = selected_model.map(str::to_string).or_else(|| {
        model::resolve_cached()
            .ok()
            .map(|(_, path)| path.display().to_string())
    });
    match chosen {
        Some(name) => {
            let catalog_hit = model::CATALOG
                .iter()
                .find(|info| name == info.file || name.ends_with(&format!("/{}", info.file)));
            if let Some(info) = catalog_hit {
                let path = cache.join(info.file);
                let verdict = if !path.is_file() {
                    Verdict::Warn
                } else if model::validate_cached(info, &path).is_ok() {
                    Verdict::Ok
                } else {
                    Verdict::Fail
                };
                let detail = if !path.is_file() {
                    format!("{} not downloaded ({} MiB)", info.key, info.size_mib)
                } else {
                    format!("{} · {}", info.label, verdict.label())
                };
                rows.push(Row {
                    label: "selected model",
                    verdict,
                    detail,
                });
            } else {
                match model::classify(&name) {
                    Ok(model::ModelRef::Catalog(info)) => {
                        let path = cache.join(info.file);
                        if path.is_file() {
                            match model::validate_cached(info, &path) {
                                Ok(()) => rows.push(Row {
                                    label: "selected model",
                                    verdict: Verdict::Ok,
                                    detail: format!("{} · verified", info.label),
                                }),
                                Err(e) => rows.push(Row {
                                    label: "selected model",
                                    verdict: Verdict::Fail,
                                    detail: e,
                                }),
                            }
                        } else {
                            rows.push(Row {
                                label: "selected model",
                                verdict: Verdict::Warn,
                                detail: format!(
                                    "{} not downloaded ({} MiB)",
                                    info.key, info.size_mib
                                ),
                            });
                        }
                    }
                    Ok(model::ModelRef::Path(path)) => match model::validate_custom(&path) {
                        Ok(()) => rows.push(Row {
                            label: "selected model",
                            verdict: Verdict::Ok,
                            detail: format!("{} · header ok", path.display()),
                        }),
                        Err(e) => rows.push(Row {
                            label: "selected model",
                            verdict: Verdict::Fail,
                            detail: e,
                        }),
                    },
                    Err(e) => rows.push(Row {
                        label: "selected model",
                        verdict: Verdict::Fail,
                        detail: e,
                    }),
                }
            }
        }
        None => rows.push(Row {
            label: "selected model",
            verdict: Verdict::Warn,
            detail: format!(
                "none cached; open the picker with m and grab {}",
                model::default_model().label
            ),
        }),
    }

    let cfg = config::path();
    let readable = std::fs::read_to_string(&cfg).is_ok();
    rows.push(Row {
        label: "config",
        verdict: if readable || !cfg.exists() {
            Verdict::Ok
        } else {
            Verdict::Warn
        },
        detail: format!(
            "{} ({})",
            cfg.display(),
            if readable {
                "readable"
            } else if cfg.exists() {
                "unreadable"
            } else {
                "not created yet"
            }
        ),
    });
    rows.push(Row {
        label: "user dirs",
        verdict: Verdict::Ok,
        detail: format!(
            "cache base {} · config base {}",
            util::cache_home().display(),
            util::config_home().display()
        ),
    });

    let width = rows
        .iter()
        .map(|r| r.label.len())
        .max()
        .unwrap_or(0)
        .max(18);
    println!(
        "scribe doctor — {} {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    );
    println!();
    for row in &rows {
        println!(
            "{:<width$}  {:<4}  {}",
            row.label,
            row.verdict.label(),
            row.detail,
            width = width
        );
    }
    let failures = rows.iter().filter(|r| r.verdict == Verdict::Fail).count();
    if failures == 0 {
        println!("\nready. missing url tools are fetched on first use.");
        0
    } else {
        println!("\n{failures} problem(s) need attention before transcription will work.");
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_labels_are_short() {
        assert_eq!(Verdict::Ok.label(), "ok");
        assert_eq!(Verdict::Warn.label(), "warn");
        assert_eq!(Verdict::Fail.label(), "fail");
    }
}
