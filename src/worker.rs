//! the pipeline: source -> ffmpeg -> 16 khz mono f32 -> whisper, in windows.
//!
//! this thread owns whisper. a second thread reads ffmpeg's output into a
//! bounded channel, so decoding never runs a mile ahead of inference and eats
//! your ram for breakfast. the ui only ever sees `Event`s, which is a polite
//! way of saying it cannot reach in here and break anything.

use crate::export::{Segment, Word};
use crate::util::is_url;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub const SAMPLE_RATE: usize = 16_000;
const WINDOW: usize = 30 * SAMPLE_RATE;
const CHUNK: usize = SAMPLE_RATE;
const MIN_TAIL: usize = SAMPLE_RATE / 2;
const MIN_WINDOW: usize = SAMPLE_RATE + SAMPLE_RATE / 10;
const EDGE_CS: i64 = 150;
pub const ENVELOPE_BUCKETS: usize = 64;

#[derive(Clone, Debug)]
pub struct Job {
    pub source: String,
    pub model: PathBuf,
    pub threads: i32,
    pub lang: Option<String>,
    pub translate: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Fetching,
    Loading,
    Transcribing,
}

#[derive(Debug)]
pub enum Event {
    Stage(Stage),
    Info {
        title: String,
        duration: Option<f64>,
    },
    Progress {
        done_s: f64,
    },
    Envelope(Vec<f32>),
    Segment(Segment),
    Done,
    Cancelled,
    Error(String),
}

type Children = Arc<Mutex<Vec<Child>>>;

pub struct Handle {
    cancel: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    children: Children,
}

impl Handle {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
        kill_all(&self.children);
    }

    /// suspend inference without killing the decoder. the worker parks itself
    /// between windows and resumes from the exact sample it was about to read.
    pub fn pause(&self) {
        self.paused.store(true, Ordering::Release);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::Release);
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub fn spawn(job: Job, tx: Sender<Event>) -> Handle {
    let cancel = Arc::new(AtomicBool::new(false));
    let paused = Arc::new(AtomicBool::new(false));
    let children: Children = Arc::new(Mutex::new(Vec::new()));
    {
        let (cancel, paused, children) = (cancel.clone(), paused.clone(), children.clone());
        let thread_tx = tx.clone();
        if let Err(e) = thread::Builder::new()
            .name("scribe-worker".into())
            .spawn(move || run(&job, &thread_tx, &cancel, &paused, &children))
        {
            // without this the ui would sit there waiting for an event that
            // never arrives. tell it what happened instead.
            let _ = tx.send(Event::Error(format!("cannot start worker thread: {e}")));
        }
    }
    Handle {
        cancel,
        paused,
        children,
    }
}

/// park the worker while paused, waking every 50 ms so cancel stays responsive.
fn wait_while_paused(cancel: &AtomicBool, paused: &AtomicBool) {
    while paused.load(Ordering::Acquire) && !cancel.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(50));
    }
}

/// Kill and reap each child. Calling `kill` alone leaves a zombie until the
/// process is waited on, so cancellation and shutdown both wait here.
fn kill_all(children: &Children) {
    if let Ok(mut list) = children.lock() {
        for child in list.iter_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn reap(children: &Children) {
    if let Ok(mut list) = children.lock() {
        for child in list.iter_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        list.clear();
    }
}

fn run(
    job: &Job,
    tx: &Sender<Event>,
    cancel: &Arc<AtomicBool>,
    paused: &Arc<AtomicBool>,
    children: &Children,
) {
    let result = pipeline(job, tx, cancel, paused, children);
    reap(children);
    let last = match result {
        _ if cancel.load(Ordering::Relaxed) => Event::Cancelled,
        Ok(()) => Event::Done,
        Err(e) => Event::Error(e),
    };
    let _ = tx.send(last);
}

/// system ffmpeg if there is one, otherwise the one the yt-dlp crate fetched.
/// reusing the managed binary for local files means one less thing to install.
pub fn ffmpeg_command() -> Command {
    Command::new(crate::fetch::ffmpeg_path().unwrap_or_else(|| PathBuf::from("ffmpeg")))
}

/// a friendly pre-flight check. ffmpeg is required for everything, so we look
/// for it here. yt-dlp is fetched on demand by the url path, so asking about it
/// this early would just produce a lie on first run.
pub fn check_tools(_need_ytdlp: bool) -> Result<(), String> {
    let ffmpeg = crate::fetch::ffmpeg_path().unwrap_or_else(|| PathBuf::from("ffmpeg"));
    if Command::new(&ffmpeg)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        return Err(
            "ffmpeg is missing and i could not fetch it. check your network, then try again".into(),
        );
    }
    Ok(())
}

fn first_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("ffmpeg reported no duration for this file")
        .to_string()
}

fn local_info(path: &str) -> (String, Option<f64>) {
    let title = Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    let ffprobe_bin = which::which("ffprobe")
        .ok()
        .or_else(|| crate::fetch::ffmpeg_path().and_then(|p| p.parent().map(|d| d.join("ffprobe"))))
        .unwrap_or_else(|| PathBuf::from("ffprobe"));
    let duration = Command::new(&ffprobe_bin)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            path,
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<f64>()
                .ok()
        })
        .filter(|d| *d > 0.0)
        .or_else(|| {
            // fallback: check ffmpeg -i stderr for "Duration: 00:00:00.00"
            let ffmpeg = crate::fetch::ffmpeg_path().unwrap_or_else(|| PathBuf::from("ffmpeg"));
            let output = Command::new(&ffmpeg)
                .args(["-i", path])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .output()
                .ok()?;
            let err_str = String::from_utf8_lossy(&output.stderr);
            for line in err_str.lines() {
                if let Some(pos) = line.find("Duration: ") {
                    let part = &line[pos + 10..];
                    let time_str = part.split(',').next()?.trim();
                    let parts: Vec<&str> = time_str.split(':').collect();
                    if parts.len() == 3 {
                        let h: f64 = parts[0].parse().ok()?;
                        let m: f64 = parts[1].parse().ok()?;
                        let s: f64 = parts[2].parse().ok()?;
                        let total = h * 3600.0 + m * 60.0 + s;
                        if total > 0.0 {
                            return Some(total);
                        }
                    }
                }
            }
            None
        });
    (title, duration)
}

enum Chunk {
    Samples(Vec<f32>),
    Failed(String),
}

fn start_decoder(
    source: &Path,
    children: &Children,
) -> Result<(ChildStdout, Option<ChildStderr>), String> {
    let mut ffmpeg = ffmpeg_command();
    ffmpeg
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(source)
        .args(["-vn", "-ac", "1", "-ar", "16000", "-f", "f32le", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = ffmpeg
        .spawn()
        .map_err(|e| format!("ffmpeg will not start: {e}"))?;
    let out = child
        .stdout
        .take()
        .ok_or("ffmpeg gave me no stdout to read")?;
    let err = child.stderr.take();
    children
        .lock()
        .map_err(|_| "mutex poisoned; this should never happen")?
        .push(child);
    Ok((out, err))
}

/// read until the buffer is full or the stream ends. os reads are allowed to
/// come back short, and some of them do it just to keep you on your toes.
fn read_full<R: Read>(reader: &mut R, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

/// little-endian f32, the format ffmpeg was asked to produce. trailing bytes
/// that do not make a whole sample are dropped, not trusted.
pub fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
    let (full, _) = bytes.as_chunks::<4>();
    full.iter()
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

fn ingest(
    mut out: ChildStdout,
    mut err: Option<ChildStderr>,
    tx: &SyncSender<Chunk>,
    cancel: &AtomicBool,
) {
    let mut bytes = vec![0u8; CHUNK * 4];
    let mut total = 0usize;
    while !cancel.load(Ordering::Relaxed) {
        let n = match read_full(&mut out, &mut bytes) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let samples = bytes_to_f32(&bytes[..n - n % 4]);
        total += samples.len();
        if tx.send(Chunk::Samples(samples)).is_err() {
            return;
        }
        if n < bytes.len() {
            break;
        }
    }
    if total == 0 && !cancel.load(Ordering::Relaxed) {
        let mut text = Vec::new();
        if let Some(e) = err.as_mut() {
            let _ = e.read_to_end(&mut text);
        }
        let detail = first_line(&text);
        let msg = format!("could not read audio from that file: {detail}");
        let _ = tx.send(Chunk::Failed(msg));
    }
}

/// youtube audio lands in a temp file. this makes sure it disappears when we
/// are done, including on the error paths i forgot about at 2am.
struct TempAudio(PathBuf, bool);

impl Drop for TempAudio {
    fn drop(&mut self) {
        if self.1 {
            crate::fetch::cleanup_audio(&self.0);
        }
    }
}

fn pipeline(
    job: &Job,
    tx: &Sender<Event>,
    cancel: &Arc<AtomicBool>,
    paused: &Arc<AtomicBool>,
    children: &Children,
) -> Result<(), String> {
    let url = is_url(&job.source);
    check_tools(url)?;

    let _ = tx.send(Event::Stage(Stage::Fetching));
    let decoded_source: PathBuf;
    let (title, duration) = if url {
        // the crate installs its binaries and gives us real metadata from
        // yt-dlp --dump-single-json, so progress is exact from the first frame.
        let fetched = crate::fetch::fetch(&job.source, cancel)?;
        decoded_source = fetched.path;
        let d = fetched
            .duration
            .or_else(|| local_info(&decoded_source.to_string_lossy()).1);
        (fetched.title, d)
    } else {
        if !Path::new(&job.source).is_file() {
            return Err(format!("no such file: {}", job.source));
        }
        decoded_source = PathBuf::from(&job.source);
        local_info(&job.source)
    };
    let _temp = TempAudio(decoded_source.clone(), url);
    let _ = tx.send(Event::Info { title, duration });
    wait_while_paused(cancel, paused);
    if cancel.load(Ordering::Relaxed) {
        return Ok(());
    }

    // redirect all whisper.cpp and ggml logs away from stdout/stderr so they
    // do not clobber the alternate screen buffer.
    whisper_rs::install_logging_hooks();

    let _ = tx.send(Event::Stage(Stage::Loading));
    let mut cparams = WhisperContextParameters::default();
    cparams.use_gpu(false);
    let ctx = WhisperContext::new_with_params(&job.model, cparams)
        .map_err(|e| format!("cannot load model {}: {e}", job.model.display()))?;
    let mut state = ctx
        .create_state()
        .map_err(|e| format!("could not create a whisper inference state: {e}"))?;
    let multilingual = ctx.is_multilingual();
    wait_while_paused(cancel, paused);
    if cancel.load(Ordering::Relaxed) {
        return Ok(());
    }

    let (out, err) = start_decoder(&decoded_source, children)?;
    let (chunk_tx, chunk_rx) = mpsc::sync_channel::<Chunk>(48);
    {
        let cancel = cancel.clone();
        thread::Builder::new()
            .name("scribe-ingest".into())
            .spawn(move || ingest(out, err, &chunk_tx, &cancel))
            .map_err(|e| format!("could not start the ingest thread: {e}"))?;
    }

    let _ = tx.send(Event::Stage(Stage::Transcribing));
    // borrow a local String into FullParams instead of leaking it: repeated
    // transcription jobs used to allocate a permanent 'static str per job.
    let lang: String = if multilingual {
        job.lang.clone().unwrap_or_else(|| "auto".into())
    } else {
        "en".into()
    };

    let mut buf: Vec<f32> = Vec::new();
    let mut base = 0usize;
    let mut cursor = 0usize;
    let mut eof = false;
    // only the text and end of the last emitted cue are needed to suppress
    // overlap duplicates; keeping a whole Segment clone per window was wasteful.
    let mut last_emitted: Option<(String, i64)> = None;

    loop {
        wait_while_paused(cancel, paused);
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        while !eof && buf.len() - (cursor - base) < WINDOW {
            if cancel.load(Ordering::Relaxed) {
                return Ok(());
            }
            wait_while_paused(cancel, paused);
            match chunk_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Chunk::Samples(s)) => buf.extend_from_slice(&s),
                Ok(Chunk::Failed(msg)) => return Err(msg),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => eof = true,
            }
        }
        // a pause can arrive while the window was filling; park again before
        // handing anything to whisper so inference truly stops.
        wait_while_paused(cancel, paused);
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let avail = buf.len() - (cursor - base);
        if avail == 0 || (eof && avail < MIN_TAIL) {
            break;
        }
        let n = avail.min(WINDOW);
        let is_final = eof && avail <= WINDOW;
        let start = cursor - base;
        let window: std::borrow::Cow<[f32]> = if n < MIN_WINDOW {
            let mut padded = buf[start..start + n].to_vec();
            padded.resize(MIN_WINDOW, 0.0);
            padded.into()
        } else {
            (&buf[start..start + n]).into()
        };

        let _ = tx.send(Event::Envelope(envelope(&window[..n], ENVELOPE_BUCKETS)));
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(job.threads);
        params.set_language(Some(lang.as_str()));
        params.set_translate(job.translate && multilingual);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_no_context(true);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        params.set_token_timestamps(true);
        // we tried whisper's abort callback. it was not trustworthy: on some
        // builds it fires mid-encode and corrupts the run. so esc is checked
        // between windows instead. a window is ~30 s, which is close enough
        // to "prompt" for a terminal tool, and the transcript survives.
        let tx_prog = tx.clone();
        let base_cursor = cursor;
        let win_n = n;
        params.set_progress_callback_safe(move |pct: i32| {
            let cur = base_cursor + ((pct.clamp(0, 100) as f64 / 100.0) * win_n as f64) as usize;
            let _ = tx_prog.send(Event::Progress {
                done_s: cur as f64 / SAMPLE_RATE as f64,
            });
        });

        let window_segments: Vec<Segment> = {
            let seg_tx: Arc<Mutex<Vec<Segment>>> = Arc::new(Mutex::new(Vec::new()));
            let collect = seg_tx.clone();
            params.set_segment_callback_safe(move |data: whisper_rs::SegmentCallbackData| {
                let text = data.text.trim().to_string();
                if is_speech_text(&text)
                    && let Ok(mut v) = collect.lock()
                {
                    v.push(Segment::new(data.start_timestamp, data.end_timestamp, text));
                }
            });
            if let Err(e) = state.full(params, &window) {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(());
                }
                return Err(format!("whisper failed on this window: {e}"));
            }
            let mut segments = seg_tx.lock().map(|v| v.clone()).unwrap_or_default();
            // attach word timings from the token data, matched to their segment.
            for seg in &mut segments {
                let idx = (0..state.full_n_segments()).find(|i| {
                    state
                        .get_segment(*i)
                        .is_some_and(|s| s.start_timestamp() == seg.start_cs)
                });
                if let Some(i) = idx
                    && let Some(segment) = state.get_segment(i)
                {
                    for t in 0..segment.n_tokens() {
                        if let Some(token) = segment.get_token(t) {
                            let data = token.token_data();
                            if let Ok(text) = token.to_str() {
                                let trimmed = text.trim();
                                if !trimmed.is_empty() && data.t1 > data.t0 {
                                    seg.words.push(Word {
                                        start_cs: data.t0,
                                        end_cs: data.t1,
                                        text: trimmed.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            segments
        };

        let window_cs = (n / (SAMPLE_RATE / 100)) as i64;
        let spans: Vec<(i64, i64)> = window_segments
            .iter()
            .map(|s| (s.start_cs, s.end_cs))
            .collect();
        let (keep, advance_cs) = plan_advance(window_cs, &spans, is_final);
        let offset_cs = (cursor / (SAMPLE_RATE / 100)) as i64;
        for mut seg in window_segments.into_iter().take(keep) {
            seg.start_cs += offset_cs;
            seg.end_cs += offset_cs;
            for word in &mut seg.words {
                word.start_cs += offset_cs;
                word.end_cs += offset_cs;
            }
            let duplicate = last_emitted
                .as_ref()
                .is_some_and(|(text, end)| *text == seg.text && seg.start_cs < *end);
            if !duplicate && seg.end_cs > seg.start_cs {
                last_emitted = Some((seg.text.clone(), seg.end_cs));
                let _ = tx.send(Event::Segment(seg));
            }
        }

        let advance = if advance_cs >= window_cs {
            n
        } else {
            advance_cs as usize * (SAMPLE_RATE / 100)
        };
        cursor += advance.max(1);
        let _ = tx.send(Event::Progress {
            done_s: cursor as f64 / SAMPLE_RATE as f64,
        });

        if cursor - base >= WINDOW {
            buf.drain(..cursor - base);
            base = cursor;
        }
    }
    Ok(())
}

/// whisper occasionally emits `[ Silence ]` and friends. those are not words,
/// and i refuse to put them in your transcript.
pub fn is_speech_text(text: &str) -> bool {
    let t = text.trim();
    !t.is_empty()
        && !matches!(
            t,
            "[BLANK_AUDIO]" | "[ Silence ]" | "[SILENCE]" | "(silence)" | "[ Inaudible ]"
        )
}

/// the overlap bookkeeping. if the last segment touches the window edge it
/// probably got cut mid-word, so we hold it back and let the next window
/// transcribe it properly. duplicate text is worse than a little extra work.
pub fn plan_advance(window_cs: i64, segs: &[(i64, i64)], is_final: bool) -> (usize, i64) {
    if is_final || segs.is_empty() {
        return (segs.len(), window_cs);
    }
    let (last_start, last_end) = segs[segs.len() - 1];
    let touches_edge = last_end >= window_cs - EDGE_CS;
    if touches_edge && last_start >= 100 {
        (segs.len() - 1, last_start)
    } else {
        (segs.len(), window_cs)
    }
}

/// rms amplitude per bucket, squashed into 0..1 so quiet speech still shows up
/// on the meter. pure math, no audio crate, no drama.
pub fn envelope(samples: &[f32], buckets: usize) -> Vec<f32> {
    if samples.is_empty() || buckets == 0 {
        return vec![0.0; buckets];
    }
    (0..buckets)
        .map(|b| {
            let lo = (b * samples.len() / buckets).min(samples.len() - 1);
            let hi = ((b + 1) * samples.len() / buckets)
                .max(lo + 1)
                .min(samples.len());
            let slice = &samples[lo..hi];
            let rms = (slice.iter().map(|s| s * s).sum::<f32>() / slice.len() as f32).sqrt();
            (rms * 6.0).min(1.0).powf(0.6)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn final_window_keeps_everything() {
        assert_eq!(
            plan_advance(3000, &[(0, 500), (500, 3000)], true),
            (2, 3000)
        );
    }

    #[test]
    fn silent_window_advances_fully() {
        assert_eq!(plan_advance(3000, &[], false), (0, 3000));
    }

    #[test]
    fn edge_segment_is_held_back() {
        let segs = [(0, 800), (800, 1900), (1900, 3000)];
        assert_eq!(plan_advance(3000, &segs, false), (2, 1900));
    }

    #[test]
    fn single_long_segment_cannot_stall() {
        assert_eq!(plan_advance(3000, &[(0, 3000)], false), (1, 3000));
        assert_eq!(plan_advance(3000, &[(50, 3000)], false), (1, 3000));
    }

    #[test]
    fn envelope_shape_and_range() {
        let loud = vec![0.5f32; 6400];
        let env = envelope(&loud, 64);
        assert_eq!(env.len(), 64);
        assert!(env.iter().all(|v| (0.0..=1.0).contains(v)));
        assert!(envelope(&vec![0.0; 6400], 8).iter().all(|v| *v == 0.0));
    }

    #[test]
    fn pcm_bytes_decode() {
        let mut bytes = Vec::new();
        for v in [0.5f32, -1.0, 0.25] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes.push(9);
        assert_eq!(bytes_to_f32(&bytes), vec![0.5, -1.0, 0.25]);
    }

    #[test]
    fn read_full_handles_short_reads() {
        struct Dribble(Cursor<Vec<u8>>);
        impl Read for Dribble {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let n = buf.len().min(3);
                self.0.read(&mut buf[..n])
            }
        }
        let mut r = Dribble(Cursor::new((0u8..20).collect()));
        let mut buf = [0u8; 8];
        assert_eq!(read_full(&mut r, &mut buf).unwrap(), 8);
    }

    #[test]
    fn placeholders_are_not_speech() {
        assert!(!is_speech_text("  "));
        assert!(!is_speech_text("[BLANK_AUDIO]"));
        assert!(is_speech_text("hello"));
    }

    #[test]
    fn worker_reports_missing_model_without_hanging() {
        let (tx, rx) = mpsc::channel();
        let job = Job {
            source: "/definitely/missing.wav".into(),
            model: PathBuf::from("/definitely/missing-model.bin"),
            threads: 1,
            lang: None,
            translate: false,
        };
        let handle = spawn(job, tx);
        // drain stage events until the terminal one. a hang here would be a
        // stuck worker thread that never reports success, failure or cancel.
        let mut terminal = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while std::time::Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Event::Error(_) | Event::Cancelled | Event::Done) => {
                    terminal = true;
                    break;
                }
                Ok(_) => {}
                Err(_) => {}
            }
        }
        assert!(terminal, "worker never produced a terminal event");
        // dropping the handle must not panic or double-kill
        drop(handle);
    }

    #[test]
    fn handle_cancel_is_idempotent() {
        let (tx, _rx) = mpsc::channel();
        let job = Job {
            source: "/definitely/missing.wav".into(),
            model: PathBuf::from("/definitely/missing-model.bin"),
            threads: 1,
            lang: None,
            translate: false,
        };
        let handle = spawn(job, tx);
        handle.cancel();
        handle.cancel();
        drop(handle);
    }

    #[test]
    fn decoder_reads_a_real_file() {
        if crate::fetch::ffmpeg_path().is_none()
            && std::process::Command::new("ffmpeg")
                .arg("-version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_err()
        {
            return;
        }
        let dir = std::env::temp_dir().join(format!("scribe-wk-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav = dir.join("tone.wav");
        let made = Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=2",
                "-ar",
                "44100",
            ])
            .arg(&wav)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if made {
            let children: Children = Arc::new(Mutex::new(Vec::new()));
            let (mut out, _err) = start_decoder(&wav, &children).unwrap();
            let mut bytes = Vec::new();
            out.read_to_end(&mut bytes).unwrap();
            let samples = bytes_to_f32(&bytes);
            assert!(
                (31_000..=33_000).contains(&samples.len()),
                "got {}",
                samples.len()
            );
            reap(&children);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
