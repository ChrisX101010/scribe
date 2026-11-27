# scribe

<p align="center">
  <img src="assets/demo.png" alt="scribe running in a terminal" width="100%">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-2024-cba6f7?style=for-the-badge&logo=rust&logoColor=white&labelColor=313244" alt="rust 2024">
  <img src="https://img.shields.io/badge/offline-first-94e2d5?style=for-the-badge&logo=protondrive&logoColor=white&labelColor=313244" alt="offline first">
  <img src="https://img.shields.io/badge/cpu-only-a6e3a1?style=for-the-badge&logo=amd&logoColor=white&labelColor=313244" alt="cpu only">
  <img src="https://img.shields.io/badge/whisper-powered-89b4fa?style=for-the-badge&logo=github&logoColor=white&labelColor=313244" alt="whisper powered">
  <img src="https://img.shields.io/badge/ratatui-tui-f5c2e7?style=for-the-badge&logo=rust&logoColor=white&labelColor=313244" alt="ratatui">
  <a href="https://crates.io/crates/scribe-tui"><img src="https://img.shields.io/crates/v/scribe-tui?style=for-the-badge&logo=rust&logoColor=white&labelColor=313244&color=f9e2af" alt="crates io"></a>
  <a href="https://aur.archlinux.org/packages/scribe-tui-bin"><img src="https://img.shields.io/badge/aur-scribe--tui--bin-fab387?style=for-the-badge&logo=archlinux&logoColor=white&labelColor=313244" alt="aur"></a>
  <img src="https://img.shields.io/badge/license-mit-f5e0dc?style=for-the-badge&logo=opensourceinitiative&logoColor=white&labelColor=313244" alt="mit license">
</p>

<p align="center">
  speech-to-text that stays in your terminal.
  <br>
  local inference · cpu-only · timestamped output
</p>

<p align="center">
  <a href="#get-started">get started</a> ·
  <a href="#features">features</a> ·
  <a href="#usage">usage</a> ·
  <a href="#keybindings">keybindings</a> ·
  <a href="#models">models</a> ·
  <a href="#privacy">privacy</a>
</p>

---

point scribe at an audio file or a supported url. it transcribes as the audio is processed, keeps word-level timing where whisper provides it, and lets you export the result without leaving the terminal.

no cloud account. no telemetry. no gpu requirement. no electron app.

## get started

### install

the crates.io package is named `scribe-tui`. the installed command is `scribe`.

```sh
cargo install scribe-tui
```

### run

```sh
scribe
```

open the source field, enter a local audio path or url, choose a model, and start transcription.

you can also pass a source directly:

```sh
scribe meeting.wav
scribe https://youtu.be/...
```

### requirements

- a supported operating system: linux, macos or windows
- a c/c++ toolchain and cmake when building from source
- ffmpeg for local audio decoding
- network access for the first model download and for url sources

scribe can fetch and manage its own yt-dlp and ffmpeg binaries for url workflows. local files use ffmpeg from your `PATH` when available.

## features

| | |
|:--|:--|
| **live transcription** | see transcript segments arrive while audio is processed |
| **word timestamps** | use whisper token timing in timestamped exports |
| **local inference** | whisper.cpp through whisper-rs, running on the cpu |
| **file and url input** | transcribe local media or supported urls |
| **model picker** | browse, download, pause, resume and remove cached models |
| **pause and resume** | pause between decode windows and continue from the last committed position |
| **partial results** | stop a job and keep the transcript already captured |
| **five export formats** | txt, srt, vtt, markdown and json |
| **22 themes** | switch palettes at runtime with a live preview |
| **search and navigation** | find transcript text and move between matches |
| **path completion** | fuzzy-ranked suggestions in the source field |
| **diagnostics** | inspect dependencies, model integrity, paths and permissions with `scribe --doctor` |
| **script-friendly mode** | write a plain transcript to stdout with `--no-tui` |

## usage

### command-line options

```sh
scribe                              # open the interface
scribe meeting.wav                  # load a local file
scribe https://youtu.be/...         # load a url
scribe audio.wav --model base-q5_1  # choose a model
scribe audio.wav --threads 4        # limit cpu threads
scribe audio.wav --format srt       # choose an export format
scribe audio.wav --lang de           # set a language hint
scribe audio.wav --translate         # translate output to english
scribe audio.wav --no-tui            # print transcript to stdout
scribe --doctor                      # inspect the installation
scribe --help
scribe --version
```

`--no-tui` sends the transcript to stdout and keeps whisper's diagnostic output on stderr:

```sh
scribe meeting.wav --no-tui > meeting.txt
```

language codes are checked against whisper's language table before inference. use `auto` for detection. english-only models always operate in english.

### exports

scribe supports:

- `txt` for plain text
- `srt` for subtitles
- `vtt` for web video captions
- `md` for markdown notes
- `json` for structured segments and word data

exports are written beside the source by default, or to the directory passed with `--out`. existing files are not overwritten. output creation is exclusive, and a failed write removes its own partial file.

srt and vtt use one cue per word when word timing is available. otherwise, they use one cue per segment. cues are sorted, clamped to avoid overlap and normalized to one line. json includes a `words` array per segment. markdown can include clickable timestamp links for youtube sources.

## keybindings

### main interface

| key | action |
|:--|:--|
| `enter` | start, confirm, accept a completion or resume |
| `tab` | switch focus or complete a path |
| `up` / `down` | move through selections and suggestions |
| `right` | accept the highlighted completion |
| `ctrl+space` | open the completion list |
| `esc` / `space` / `p` | pause an active job |
| `c` | stop the active job and keep captured segments |
| `/` | search the transcript |
| `n` / `N` | next / previous match |
| `j` / `k` | move selection |
| `page up` / `page down` | scroll ten rows |
| `e` | open export menu |
| `y` | copy the selected segment |
| `m` | open model picker |
| `s` | open settings |
| `t` | open theme palette |
| `?` | show help |
| `ctrl+c` | quit |

the first `esc` pauses an active transcription. use `enter`, `space` or `p` to resume. a second `esc`, or `c`, stops the job and keeps the partial transcript available for export.

### model picker

| key | action |
|:--|:--|
| `enter` | activate a ready model or start / resume its download |
| `p` | pause the active download |
| `c` | cancel the download and remove its partial file |
| `x` | remove a cached model |
| `j` / `k`, `up` / `down` | move through the model list |
| `page up` / `page down` | move ten rows |
| `home` / `end` | jump to the first / last row |
| `esc` | pause a download or close the picker |

### theme picker

use `j` / `k` or the arrow keys to preview palettes. press `enter` to keep a theme or `esc` to revert.

## models

the picker includes 33 pre-converted ggml models from the upstream whisper.cpp model collection, including quantized q5 and q8 variants and english-only models.

- **default:** `small-q5_1`
- **quality option:** `large-v3-turbo-q5_0`
- **custom models:** pass a local ggml path with `--model <path>`

| model family | download size | approximate ram | profile |
|:--|--:|--:|:--|
| tiny | 31–75 MiB | 120–200 MiB | fastest |
| base | 57–142 MiB | 200–500 MiB | fast |
| small | 181–466 MiB | 500–1,200 MiB | balanced |
| medium | 514 MiB–1.5 GiB | 1,200–3,200 MiB | higher accuracy |
| large | 1.1–2.9 GiB | 2,100–3,200 MiB | slower |
| large-v3-turbo | 547 MiB–1.5 GiB | 1,200–3,200 MiB | quality / speed balance |

these are approximate figures. actual memory use and speed depend on the model variant, audio and hardware. quantization reduces download size and memory requirements, but does not guarantee the same speedup on every cpu.

model downloads are checked against catalog size and sha-256 metadata before use. invalid catalog files are rejected. custom model files are checked for plausible size and a recognized ggml/gguf header, but cannot be checked against a catalog checksum.

## themes

22 palettes are available at runtime through `t`:

catppuccin mocha · catppuccin macchiato · catppuccin frappe · catppuccin latte · tokyo night · tokyo night storm · nord · dracula · gruvbox dark · one dark · solarized dark · solarized light · github dark · github light · everforest · kanagawa · rose pine · rose pine dawn · material dark · palenight · cyberpunk · high contrast

set the startup palette with `--theme <name>`. common aliases are accepted. unknown names fall back to catppuccin mocha.

the interface adapts to terminal width, supports 16-colour ansi and truecolor, respects `NO_COLOR`, and keeps the terminal background transparent. bracketed paste is supported. narrow terminals use a simpler layout instead of squeezing the side panel into unusable space.

## privacy

transcription and inference happen locally.

- audio is decoded on your machine.
- whisper.cpp runs locally on your cpu.
- transcripts and settings are stored locally.
- scribe does not require an account.
- scribe does not send audio or transcripts to a hosted transcription service.
- scribe does not include telemetry.

**network access is still needed** to download a model the first time and whenever you transcribe a url. after a model is cached, local-file transcription can run offline.

## storage and diagnostics

models are stored in the platform cache:

| platform | model cache |
|:--|:--|
| linux | `~/.cache/scribe/models` |
| macos | `~/Library/Caches/scribe/models` |
| windows | `%LOCALAPPDATA%\scribe\models` |

`$XDG_CACHE_HOME` is respected where applicable. settings are stored in the platform config directory, typically `~/.config/scribe/config` on linux or under `$XDG_CONFIG_HOME`.

run the diagnostic command for a read-only report:

```sh
scribe --doctor
```

it checks dependencies, model integrity, paths and permissions. it does not install packages or modify files.

## performance and implementation

- cpu-only build. cuda, metal, vulkan, hipblas, coreml and sycl backends are not compiled in.
- the default thread count is `min(logical cores, 8)`. override it with `--threads` or the settings panel.
- ffmpeg decodes incrementally. whisper processes 30-second windows with overlap handling.
- decoder stderr is drained separately with a bounded tail. decoder exit status is checked before partial audio is treated as a successful transcript.
- worker events carry a job id. events from stopped jobs are discarded, and workers are joined during shutdown.
- transcript wrapping is cached until segments or terminal width change. search checks new segments while the query stays unchanged.
- interface transitions are eased over a few frames. the ui becomes idle when nothing is changing.

## known limits

- url support depends on yt-dlp and the source site's access rules. some sites require cookies or a specific player client.
- word timestamps follow whisper token timing, not forced alignment. one token can represent multiple words in some languages.
- custom ggml models can be loaded by path but do not appear in the model picker.
- pausing takes effect between decode windows. an in-progress fetch or decode step is allowed to finish.
- stdin piping and batch transcription are not implemented in 0.3.0.
- model downloads require network access once. url transcription requires network access each time.
- ci release artifacts currently target linux x86_64, macos aarch64 and windows x86_64. other targets are not tested.

## build from source

### prerequisites

whisper-rs builds whisper.cpp through native bindings, so a c/c++ toolchain and cmake are required.

debian / ubuntu:

```sh
sudo apt-get install clang libclang-dev cmake pkg-config
```

macos requires xcode command line tools and cmake. windows requires the msvc build tools and cmake.

### compile and test

```sh
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
cargo publish --dry-run --allow-dirty
```

run the binary:

```sh
./target/release/scribe
```

## dependencies

| component | role |
|:--|:--|
| [whisper.cpp](https://github.com/ggml-org/whisper.cpp) | local speech recognition |
| [whisper-rs](https://github.com/tazz4843/whisper-rs) | rust bindings |
| [ratatui](https://github.com/ratatui/ratatui) | terminal interface |
| [crossterm](https://github.com/crossterm-rs/crossterm) | terminal input and rendering |
| [ffmpeg](https://ffmpeg.org/) | audio decoding |
| [yt-dlp](https://github.com/yt-dlp/yt-dlp) | url media extraction |

## contributing

issues and pull requests are welcome. please include the operating system, terminal, input format and relevant command when reporting a bug. for transcription problems, include the model name and language where possible.

run the formatting, test and clippy checks before opening a pull request.

## license

[mit](LICENSE)
