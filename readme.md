# scribe

![demo](assets/demo.png)

<p align="center">
  <img src="https://img.shields.io/badge/rust-2024-cba6f7?style=for-the-badge&logo=rust&logoColor=white&labelColor=313244" alt="Rust 2024">
  <img src="https://img.shields.io/badge/offline-first-94e2d5?style=for-the-badge&logo=protondrive&logoColor=white&labelColor=313244" alt="Offline first">
  <img src="https://img.shields.io/badge/cpu-only-a6e3a1?style=for-the-badge&logo=amd&logoColor=white&labelColor=313244" alt="CPU only">
  <img src="https://img.shields.io/badge/whisper-powered-89b4fa?style=for-the-badge&logo=github&logoColor=white&labelColor=313244" alt="Whisper powered">
  <img src="https://img.shields.io/badge/ratatui-tui-f5c2e7?style=for-the-badge&logo=rust&logoColor=white&labelColor=313244" alt="Ratatui">
  <img src="https://img.shields.io/badge/license-mit-f9e2af?style=for-the-badge&logo=opensourceinitiative&logoColor=white&labelColor=313244" alt="MIT license">
</p>

Speech to text in the terminal. Point it at a file or a URL, get a timestamped
transcript while the audio is still playing, press `e` when you want it out.
Runs locally on your CPU. No cloud, no account, no telemetry, no Electron.

## install

The crates.io name is `scribe-tui` because somebody else got plain `scribe`
first. The binary is still `scribe`, which is the part you actually type.

```sh
cargo install scribe-tui
```

From a checkout:

```sh
cargo build --release
./target/release/scribe
```

Prebuilt CPU binaries land on the releases page for Linux, macOS and Windows
when we cut a tag.

## setup, or the lack of it

Nothing to install by hand. On first URL use, the `yt-dlp` crate fetches its own
yt-dlp and ffmpeg binaries into `~/.cache/scribe/bin/yt-dlp-2` (or under
`$XDG_CACHE_HOME`). Local files use the ffmpeg on your `PATH`, falling back to
the managed copy if you have none.

Models live in `~/.cache/scribe/models`. Press `m` in the TUI to pick one and it
downloads itself from Hugging Face. Start with `small-q5_1`. Downloads are
checked against the upstream SHA-256 before they are accepted; a bad file gets
deleted and retried.

Settings made in the TUI persist in `~/.config/scribe/config` (or
`$XDG_CONFIG_HOME/scribe/config`). Command-line options win over saved values.
`--help` and error output borrow their colors from your saved theme when stdout
is a terminal. `NO_COLOR` and `TERM=dumb` turn that off.

## usage

```sh
scribe                            # just open it
scribe meeting.wav                # preload a file
scribe https://youtu.be/...       # preload a url
scribe a.wav --model base-q5_1    # pick a model by name
scribe a.wav --threads 4          # cap cpu threads
scribe a.wav --format srt         # export format
scribe a.wav --lang de            # language hint
scribe a.wav --translate          # translate to english
scribe a.wav --no-tui             # plain transcript to stdout, for scripts
scribe --help
scribe --version
```

`--no-tui` writes the transcript to stdout and keeps whisper's chatter on
stderr, so `scribe audio.wav --no-tui > out.txt` does what you expect.

## keybinds

| key | action |
| --- | --- |
| enter | start, confirm, accept a completion, or resume when paused |
| tab | switch focus; fill the shared prefix or accept a path completion |
| up / down | move selection, or move through completion suggestions |
| right | accept the highlighted completion |
| ctrl+space | force the completion list open |
| esc / space / p | pause the active job; press again to resume (`space` works in the transcript pane) |
| c | stop the active job, keeping the segments already captured |
| / | live search |
| n / N | next / previous match |
| j / k | move selection |
| page up / page down | scroll ten rows |
| esc | dismiss completion, clear search, or go back |
| e | export menu |
| y | copy selected segment (OSC 52, works over ssh) |
| m | model picker |
| s | settings |
| t | theme palette (live preview) |
| ? | help |
| ctrl+c | quit |

The first `esc` pauses an active transcription, not kills it. `enter`, `space`
or `p` picks it back up at the same audio position. Nothing restarts from zero.
A second `esc`, or `c`, stops it for good and leaves the partial transcript in
place so you can still export it.

Inside the **source field**, path completion behaves like you would expect from
an editor: fuzzy rank, inline ghost text, a suggestion list you can navigate,
shared-prefix fill on the first `tab`, and a full commit on the second.

Inside the **model picker**:

| key | action |
| --- | --- |
| enter | activate a ready model, or start / resume a download |
| p | pause the active download (partial data is kept) |
| c | cancel the active download and delete its partial file |
| x | remove a cached model |
| j / k or up / down | move through the table |
| page up / page down | jump ten rows |
| home / end | jump to the first or last row |
| esc | pause a download, or close |

Inside the **theme picker**, `j / k` or the arrow keys cycle with a live preview.
`enter` confirms; `esc` reverts the preview and closes.

## themes

22 palettes, switchable at runtime with `t`. The full set:

catppuccin mocha, catppuccin macchiato, catppuccin frappe, catppuccin latte,
tokyo night, tokyo night storm, nord, dracula, gruvbox dark, one dark,
solarized dark, solarized light, github dark, github light, everforest,
kanagawa, rose pine, rose pine dawn, material dark, palenight, cyberpunk,
high contrast.

`--theme <name>` sets the startup theme. Aliases are accepted (`dark`, `light`,
`contrast`, `nord`, `tokyo-night`, `rose-pine-dawn`, and so on). Unknown names
fall back to catppuccin mocha rather than making a scene.

## models

The picker carries all 33 pre-converted ggml models from the upstream
`ggerganov/whisper.cpp` repository, including quantized `q5`/`q8` builds and
English-only `.en` variants. The default is `small-q5_1`; `large-v3-turbo-q5_0`
is the quality pick. SHAs come from Hugging Face file metadata; the upstream
README table is stale and we do not trust it.

| name | disk | approx ram | speed |
| --- | --- | --- | --- |
| tiny / tiny.en | 75 MiB | 120 MiB | fastest |
| tiny-q5_1 / tiny.en-q5_1 | 31 MiB | 120 MiB | fastest |
| tiny-q8_0 / tiny.en-q8_0 | 42 MiB | 200 MiB | fastest |
| base / base.en | 142 MiB | 500 MiB | fast |
| base-q5_1 / base.en-q5_1 | 57 MiB | 200 MiB | fast |
| base-q8_0 / base.en-q8_0 | 78 MiB | 200 MiB | fast |
| small / small.en | 466 MiB | 1200 MiB | balanced |
| small-q5_1 / small.en-q5_1 | 181 MiB | 500 MiB | balanced |
| small-q8_0 / small.en-q8_0 | 252 MiB | 650 MiB | balanced |
| medium / medium.en | 1.5 GiB | 3200 MiB | accurate |
| medium-q5_0 / medium.en-q5_0 | 514 MiB | 1200 MiB | accurate |
| medium-q8_0 / medium.en-q8_0 | 785 MiB | 1600 MiB | accurate |
| large-v1 / large-v2 / large-v3 | 2.9 GiB | 3200 MiB | slow |
| large-v2-q5_0 / large-v3-q5_0 | 1.1 GiB | 2100 MiB | slow |
| large-v2-q8_0 | 1.5 GiB | 3200 MiB | slow |
| large-v3-turbo | 1.5 GiB | 3200 MiB | accurate |
| large-v3-turbo-q5_0 | 547 MiB | 1200 MiB | accurate |
| large-v3-turbo-q8_0 | 834 MiB | 1600 MiB | accurate |

Sizes are from the upstream table. A quantized model is smaller and usually
quicker on CPU. "Usually" is doing real work there; hardware differs, so
benchmark on your own box before calling anything the fastest. The picker shows
a ready/get badge and the first 12 characters of each SHA.

Custom ggml files work through `--model <path>` but do not show up in the picker.

## export

`txt`, `srt`, `vtt`, `md`, `json`. Files go next to the source, or into `--out`.
Existing files are not overwritten; move them yourself.

Word timings come from whisper's token data. `srt` and `vtt` get per-word cues,
`json` gets a `words` array per segment, and markdown turns each word into a
clickable timestamp link when the source is a YouTube URL.

## performance notes

- CPU only. No CUDA, Metal, Vulkan, HIPBLAS, CoreML or SYCL code is compiled in.
- Threads default to `min(logical cores, 8)`. `--threads` or the settings panel
  can override that.
- ffmpeg decodes incrementally and whisper runs 30-second windows with overlap
  handling, so words land while the tape is still rolling.
- completion, scroll and status transitions are eased over a few frames; the UI
  goes quiet when nothing moves.
- if the text comes out nonsense, it is probably the model, not the pipeline.
  Try turbo before filing an accuracy bug.

## the screen

Wide terminals get the transcript plus a side panel for session info, progress
and a level meter. Medium ones stack a compact status row under the transcript.
Narrow ones drop the side panel entirely. The layout does not try to squeeze a
30-column pane into a 24-column terminal and call that responsive. Below 20
columns you get a short message asking for more room, which is the honest
outcome.

The UI reads its colors from the selected palette, keeps the canvas transparent
so terminal blur shows through, and respects `NO_COLOR`, 16-colour ANSI and
truecolor detection. Pasting via bracketed paste never fires stray shortcuts.
Toasts sit in the bottom-right corner and fade on their own; they carry
download and pause news without interrupting the transcript.

## versions

Checked against crates.io and docs.rs, pinned in `Cargo.toml`.

| crate | version | note |
| --- | --- | --- |
| rust | stable, edition 2024 | MSRV 1.88 |
| ratatui | 0.30.2 | crossterm 0.29 re-export |
| crossterm | 0.29.0 | same major, no version skew |
| whisper-rs | 0.16.0 | cpu-only, all gpu features off |
| yt-dlp | 2.10.0 | cache-json + rustls features |
| tokio | 1.x | current-thread runtime for URL fetch only |
| ureq | 3.5.2 | model downloads |
| sha2 | 0.10 | model checksum verification |
| unicode-width | 0.2.3 | wrapping that does not cut wide glyphs |
| which | 8 | find system ffprobe |

Two quirks worth knowing: yt-dlp 2.10.0 needs a cache feature just to compile,
and whisper-rs 0.16's abort callback is unreliable mid-encode. Both are worked
around in code. If you touch those lines, leave the comments alone.

## limits, honestly

- some sites want cookies or a specific player client. yt-dlp covers 1800+, but
  authentication stays a site-by-site problem.
- word timings follow whisper token boundaries, not forced alignment. A token
  can cover several words in some languages.
- custom ggml models work through `--model` but are not in the picker.
- pausing a transcription parks the worker between decode windows, not in the
  middle of one. A 30-second window can finish after you hit pause; the UI
  ignores those stragglers and resumes at the last committed position.
- model downloads need network access once. After that, everything is local.

## build, test, run

```sh
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
cargo publish --dry-run --allow-dirty   # packaging check, nothing is uploaded
./target/release/scribe
```

## license

MIT
