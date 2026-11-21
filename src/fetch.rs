//! url ingestion. the yt-dlp crate is async, so we spin up a tiny current-
//! thread tokio runtime inside a plain std thread and keep the rest of scribe
//! refreshingly synchronous. no runtime spaghetti, no callback pyramid.
//!
//! the crate fetches yt-dlp and ffmpeg on demand. you do not install anything,
//! which is the way tools should work in 2026.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct FetchedAudio {
    pub path: PathBuf,
    pub title: String,
    pub duration: Option<f64>,
}

/// where the managed binaries live. versioned directory so a future yt-dlp
/// upgrade cannot trip over yesterday's binary and make a mess of things.
pub fn libs_dir() -> PathBuf {
    let base = if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            PathBuf::from(xdg).join("scribe")
        } else {
            default_home()
        }
    } else {
        default_home()
    };
    base.join("bin").join("yt-dlp-2")
}

fn default_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".cache")
        .join("scribe")
}

fn output_dir() -> PathBuf {
    libs_dir().join("audio")
}

/// fetch audio and its metadata. blocks until finished, with `cancel` checked
/// between the async steps. yes, this is a blocking call in an async wrapper;
/// the alternative was dragging tokio through the entire tui, and i like my
/// event loop boring.
pub fn fetch(url: &str, cancel: &Arc<AtomicBool>) -> Result<FetchedAudio, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("could not start the network runtime: {e}"))?;
    rt.block_on(async move { fetch_async(url, cancel).await })
}

async fn fetch_async(url: &str, cancel: &Arc<AtomicBool>) -> Result<FetchedAudio, String> {
    use yt_dlp::Downloader;
    use yt_dlp::client::deps::{Libraries, LibraryInstaller};

    let dir = libs_dir();
    let out = output_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create tools directory: {e}"))?;
    std::fs::create_dir_all(&out).map_err(|e| format!("cannot create audio directory: {e}"))?;

    let ytdlp = dir.join(if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    });
    let ffmpeg = dir.join(if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    });

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }

    // install what is missing. the crate knows which release asset belongs to
    // which os/arch, which is one less lookup table i have to maintain.
    if !ytdlp.is_file() || !ffmpeg.is_file() {
        let installer = LibraryInstaller::new(dir.clone());
        if !ytdlp.is_file() {
            let installed = installer
                .install_youtube(None)
                .await
                .map_err(|e| format!("could not install yt-dlp: {e}"))?;
            if installed != ytdlp {
                std::fs::copy(&installed, &ytdlp)
                    .map_err(|e| format!("could not place yt-dlp binary: {e}"))?;
            }
        }
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        if !ffmpeg.is_file() {
            let installed = installer
                .install_ffmpeg(None)
                .await
                .map_err(|e| format!("could not install ffmpeg: {e}"))?;
            if installed != ffmpeg {
                std::fs::copy(&installed, &ffmpeg)
                    .map_err(|e| format!("could not place ffmpeg binary: {e}"))?;
            }
        }
    }

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }

    let libs = Libraries::new(ytdlp.clone(), ffmpeg.clone());
    let downloader = Downloader::builder(libs, out.clone())
        .build()
        .await
        .map_err(|e| format!("could not start the downloader: {e}"))?;

    let video = downloader
        .fetch_video_infos(url)
        .await
        .map_err(|e| format!("could not read that url: {e}"))?;
    let title = if video.title.trim().is_empty() {
        "web audio".to_string()
    } else {
        video.title.clone()
    };
    let duration = video
        .duration
        .and_then(|d| if d > 0 { Some(d as f64) } else { None });

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }

    // unique name so two runs cannot fight over the same file. i have seen
    // that movie and it does not end well.
    let name = format!("scribe-{}-{}.m4a", std::process::id(), now_millis());
    let path = match downloader.download_audio_stream(&video, &name).await {
        Ok(p) => {
            if p.is_absolute() {
                p
            } else {
                out.join(p)
            }
        }
        Err(crate_err) => {
            // youtube's cdn sometimes says 403 to direct http downloads. the
            // managed yt-dlp binary knows about player clients and signatures,
            // so we fall back to it. we tried being clever first. it was not.
            let fallback = out.join(format!(
                "scribe-{}-{}.webm",
                std::process::id(),
                now_millis()
            ));
            let status = tokio::process::Command::new(&ytdlp)
                .args([
                    "--no-playlist",
                    "--no-warnings",
                    "-f",
                    "bestaudio/best",
                    "-o",
                    fallback.to_str().unwrap_or("audio.webm"),
                    url,
                ])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await
                .map_err(|e| format!("could not download audio: {e}"))?;
            if !status.success() || !fallback.is_file() {
                return Err(format!(
                    "no audio came out. the http path said: {crate_err}"
                ));
            }
            fallback
        }
    };

    Ok(FetchedAudio {
        path,
        title,
        duration,
    })
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// have the managed tools been installed yet? used for the startup hint.
pub fn tools_ready() -> bool {
    let dir = libs_dir();
    dir.join(if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    })
    .is_file()
        && dir
            .join(if cfg!(windows) {
                "ffmpeg.exe"
            } else {
                "ffmpeg"
            })
            .is_file()
}

/// path to the managed ffmpeg binary, or none until it is installed.
pub fn ffmpeg_path() -> Option<PathBuf> {
    let p = libs_dir().join(if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    });
    p.is_file().then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_under_cache() {
        let d = libs_dir();
        assert!(d.to_string_lossy().contains("scribe"));
        assert!(d.ends_with("yt-dlp-2"));
    }

    #[test]
    fn ffmpeg_lookup_is_safe() {
        let _ = ffmpeg_path();
        let _ = tools_ready();
    }
}
