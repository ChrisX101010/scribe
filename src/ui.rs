//! rendering. every color comes from the palette, every layout has three
//! shapes, and none of it knows where the data came from. that separation is
//! why resizing the terminal does not send the state machine into a spiral.

use crate::anim;
use crate::app::{App, DownloadState, Focus, Overlay, Status};
use crate::model;
use crate::theme::Palette;
use crate::util::{duration_short, is_url, pad_to, timestamp_short, truncate, wrap};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

pub struct Frame<'a> {
    pub buf: &'a mut Buffer,
    pub area: Rect,
    pub palette: Palette,
}

impl<'a> Frame<'a> {
    fn style(&self) -> Style {
        Style::default().fg(self.palette.text)
    }
}

pub fn render(buf: &mut Buffer, area: Rect, app: &mut App, palette: Palette) {
    let mut f = Frame { buf, area, palette };
    f.buf.set_style(f.area, f.style());

    // below this there is no room to be useful.
    if f.area.width < 20 || f.area.height < 6 {
        render_too_small(&mut f);
        return;
    }

    let wide = f.area.width >= 92 && f.area.height >= 16;
    let medium = f.area.width >= 40 && f.area.height >= 13;

    // always draw the full dashboard first — overlays render on top of it.
    if wide {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(7),
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(f.area);
        render_header(&mut f, chunks[0], app);
        let body =
            Layout::horizontal([Constraint::Min(44), Constraint::Length(44)]).split(chunks[2]);
        render_transcript(&mut f, body[0], app);
        render_side(&mut f, body[1], app);
        render_input(&mut f, shift_right(chunks[4], 3), app);
        render_footer(&mut f, shift_right(chunks[5], 2), app);
    } else if medium {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(f.area);
        render_header(&mut f, chunks[0], app);
        render_transcript(&mut f, chunks[2], app);
        render_compact_status(&mut f, chunks[3], app);
        render_input(&mut f, shift_right(chunks[5], 3), app);
        render_footer(&mut f, shift_right(chunks[6], 2), app);
    } else if f.area.height >= 8 {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(f.area);
        render_compact_header(&mut f, chunks[0], app);
        render_transcript(&mut f, chunks[2], app);
        render_input(&mut f, shift_right(chunks[3], 3), app);
        render_footer(&mut f, shift_right(chunks[4], 2), app);
    } else {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.area);
        render_compact_header(&mut f, chunks[0], app);
        render_transcript(&mut f, chunks[2], app);
        render_input(&mut f, shift_right(chunks[3], 3), app);
        render_footer(&mut f, shift_right(chunks[4], 2), app);
    }

    // overlays render on top — full-screen and opaque.
    match app.overlay {
        Overlay::Help => render_help(&mut f),
        Overlay::Settings => render_settings(&mut f, app),
        Overlay::ModelPicker => render_picker(&mut f, app),
        Overlay::ThemePicker => render_theme_picker(&mut f, app),
        Overlay::Export => render_export(&mut f, app),
        Overlay::Search => render_search(&mut f, app),
        Overlay::Message if !app.message.is_empty() => render_message(&mut f, app),
        _ => {}
    }

    // toasts paint last so they stay visible above every overlay.
    render_toasts(&mut f, app);
}

fn shift_right(area: Rect, cells: u16) -> Rect {
    let shift = cells.min(area.width);
    Rect::new(
        area.x + shift,
        area.y,
        area.width.saturating_sub(shift),
        area.height,
    )
}

fn render_compact_header(f: &mut Frame, area: Rect, app: &App) {
    let state = format!("{} {}", spinner(app), app.status.label());
    let line = Line::from(vec![
        Span::styled(
            truncate(
                &app.model_label,
                area.width.saturating_sub(state.len() as u16 + 8) as usize,
            ),
            Style::default().fg(f.palette.muted),
        ),
        Span::styled(format!(" {state}"), status_style(f, app)),
    ]);
    line.render(Rect::new(area.x, area.y, area.width, 1), f.buf);
}

fn render_compact_status(f: &mut Frame, area: Rect, app: &App) {
    render_progress(f, Rect::new(area.x, area.y, area.width, 1), app);
    if area.height > 1 {
        let elapsed = duration_short(app.elapsed_s());
        let eta = app
            .eta_s()
            .map(duration_short)
            .unwrap_or_else(|| "--:--".into());
        let speed = app
            .speed_multiplier()
            .map(|s| format!("{s:.1}x"))
            .unwrap_or_else(|| "-".into());
        let text = format!(
            "{elapsed} / {eta}  {speed}  {}",
            truncate(
                if app.title.is_empty() { "" } else { &app.title },
                area.width.saturating_sub(24) as usize
            )
        );
        Span::styled(
            truncate(&text, area.width as usize),
            Style::default().fg(f.palette.muted),
        )
        .render(Rect::new(area.x, area.y + 1, area.width, 1), f.buf);
    }
}

/// wipe glyphs but leave the background reset, so the terminal canvas shows
/// through the overlay on the alternate screen. borders and text are drawn
/// afterwards as usual.
fn clear_transparent(buf: &mut Buffer, area: Rect) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.set_char(' ');
            // reset, not default: Style::default() has bg=None, which merges
            // as "leave background alone" and leaves the footer's keycap
            // backgrounds behind as little grey boxes under the overlay.
            cell.set_style(Style::reset());
        }
    }
}

fn fill_blank(buf: &mut Buffer, area: Rect, style: Style) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.set_char(' ');
            cell.set_style(style);
        }
    }
}

fn render_too_small(f: &mut Frame) {
    let msg = "give me a little more room to breathe";
    let x = f.area.left() + (f.area.width.saturating_sub(msg.len() as u16)) / 2;
    let y = f.area.top() + f.area.height / 2;
    let text = Span::styled(msg, Style::default().fg(f.palette.warn));
    text.render(Rect::new(x, y, msg.len() as u16, 1), f.buf);
}

fn pane(f: &mut Frame, area: Rect, title: &str, focused: bool) -> Rect {
    let border_color = pane_color(f, title, focused);
    let title_style = Style::default()
        .fg(border_color)
        .add_modifier(Modifier::BOLD);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(format!(" {title} "), title_style));
    let inner = block.inner(area);
    block.render(area, f.buf);
    inner
}

fn pane_color(f: &Frame, title: &str, focused: bool) -> ratatui::style::Color {
    match title {
        "session" => f.palette.accent,
        "transcript" => f.palette.secondary_accent,
        "session telemetry" => f.palette.warn,
        "source" => f.palette.success,
        _ if focused => f.palette.accent,
        _ => f.palette.border,
    }
}

fn spinner(app: &App) -> String {
    if app.status.active() {
        format!("{} ", anim::spinner(app.tick * 16))
    } else {
        String::new()
    }
}

fn status_style(f: &Frame, app: &App) -> Style {
    let color = match app.status {
        Status::Error => f.palette.error,
        Status::Ready => f.palette.success,
        Status::Cancelled | Status::Paused => f.palette.warn,
        Status::Idle => f.palette.muted,
        _ => f.palette.accent,
    };
    let t = anim::ease_out_cubic((app.stage_started.elapsed().as_secs_f32() / 0.24).min(1.0));
    let color = crate::theme::mix(f.palette.muted, color, t);
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let spin = spinner(app);
    let status_text = format!("{spin}{}", app.status.label());

    let wordmark = "scribe";

    let duration_tag = app
        .source_duration
        .filter(|d| *d > 0.0)
        .map(|d| format!(" ({})", duration_short(d)))
        .unwrap_or_default();

    let source = if app.title.is_empty() {
        "awaiting audio source".to_string()
    } else {
        format!("{}{duration_tag}", app.title)
    };

    let dl = if app.download == DownloadState::Downloading {
        format!("  ↓{}%", (app.download_frac * 100.0) as u32)
    } else {
        String::new()
    };

    let used = wordmark.len() as u16 + status_text.len() as u16 + dl.len() as u16 + 5;
    let mid_w = area.width.saturating_sub(used);

    let line = Line::from(vec![
        Span::styled(
            wordmark,
            Style::default()
                .fg(f.palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            truncate(&source, mid_w as usize),
            Style::default().fg(f.palette.text),
        ),
        Span::styled(
            dl,
            Style::default()
                .fg(f.palette.secondary_accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("   {status_text} "), status_style(f, app)),
    ]);
    line.render(
        Rect::new(area.x + 1, area.y, area.width.saturating_sub(1), 1),
        f.buf,
    );
}

// transcript pane
fn render_transcript(f: &mut Frame, area: Rect, app: &mut App) {
    let inner = pane(f, area, "transcript", app.focus == Focus::Transcript);
    let width = inner.width.saturating_sub(2) as usize;
    let gutter = 7usize;
    let text_w = width.saturating_sub(gutter).max(1);

    if app.segments.is_empty() {
        render_playing_cat(f, inner, app);
        if app.status.active() {
            Span::styled(
                "whisper is listening and decoding speech segments...",
                Style::default()
                    .fg(f.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .render(
                Rect::new(inner.x + 3, inner.y + 6, inner.width.saturating_sub(6), 1),
                f.buf,
            );
            return;
        }

        let steps = [
            ("start here", f.palette.secondary_accent, true),
            (
                "▸ 1. paste a file path or url in the source bar below",
                f.palette.text,
                false,
            ),
            (
                "▸ 2. hit enter and whisper gets to work on your cpu",
                f.palette.text,
                false,
            ),
            (
                "▸ 3. m picks a model, s is settings, e exports when you are done",
                f.palette.subtext0,
                false,
            ),
            ("", f.palette.muted, false),
            ("takes these", f.palette.secondary_accent, true),
            (
                "wav · mp3 · m4a · flac · ogg · webm · mp4 · mkv · youtube links",
                f.palette.muted,
                false,
            ),
        ];

        for (index, (text, color, bold)) in steps.into_iter().enumerate() {
            let row = inner.y + 6 + index as u16;
            if row >= inner.y + inner.height {
                break;
            }
            if !text.is_empty() {
                let mut style = Style::default().fg(color);
                if bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                Span::styled(text, style).render(
                    Rect::new(inner.x + 3, row, inner.width.saturating_sub(6), 1),
                    f.buf,
                );
            }
        }
        return;
    }

    let mut visual_lines: Vec<(usize, String, usize)> = Vec::new();
    let mut selected_line = 0usize;
    for (idx, seg) in app.segments.iter().enumerate() {
        let wrapped = wrap(&seg.text, text_w);
        if idx == app.selected {
            selected_line = visual_lines.len();
        }
        for (li, line) in wrapped.into_iter().enumerate() {
            visual_lines.push((idx, line, li));
        }
    }

    let visible = inner.height as usize;
    let max_scroll = visual_lines.len().saturating_sub(visible);

    // strict anti-overscroll: the first visible line is always in [0, max_scroll].
    // the last line of the transcript is reachable, but never past it.
    let first = app.scroll.min(max_scroll);

    if app.scroll_home {
        // completion / cancellation: glide back to the top and stay there.
        // park the selection at the first segment too, otherwise the manual
        // "keep selection visible" branch below yanks us straight back down.
        app.scroll = 0;
        app.settle_scroll_home();
    } else if app.follow {
        // live transcription: stay pinned to the newest line.
        app.scroll = max_scroll;
    } else if visible > 0 {
        // manual navigation: keep the selected segment inside the viewport,
        // scrolling in both directions instead of pinning it to the top row.
        if selected_line < first {
            app.scroll = selected_line;
        } else if selected_line >= first + visible {
            app.scroll = selected_line + 1 - visible;
        }
    }

    // the smoothed position is what actually renders, so scrolling animates.
    let render_first = (app.scroll_smooth.round() as usize).min(max_scroll);

    for (row, (idx, text, li)) in visual_lines
        .into_iter()
        .skip(render_first)
        .take(visible)
        .enumerate()
    {
        let y = inner.y + row as u16;
        if y >= inner.y + inner.height {
            break;
        }
        let seg = &app.segments[idx];
        let selected = idx == app.selected;
        let mut line_style = Style::default().fg(f.palette.text);
        if selected {
            line_style = line_style.fg(f.palette.accent);
        }

        let bar = if selected { "▌" } else { " " };
        let bar_color = if selected {
            f.palette.accent
        } else {
            f.palette.surface1
        };
        Span::styled(
            bar,
            Style::default().fg(bar_color).add_modifier(Modifier::BOLD),
        )
        .render(Rect::new(inner.x, y, 1, 1), f.buf);

        let stamp = if li == 0 {
            timestamp_short(seg.start_cs)
        } else {
            "     ".to_string()
        };
        let stamp_color = if selected {
            f.palette.secondary_accent
        } else {
            f.palette.muted
        };
        Span::styled(format!("{stamp} "), Style::default().fg(stamp_color))
            .render(Rect::new(inner.x + 1, y, gutter as u16, 1), f.buf);

        let rendered = highlight_line(f, &text, &app.query);
        rendered.render(
            Rect::new(inner.x + 1 + gutter as u16, y, text_w as u16, 1),
            f.buf,
        );
    }
}

fn render_playing_cat(f: &mut Frame, area: Rect, app: &App) {
    if area.width < 20 || area.height < 5 {
        return;
    }
    let phase = ((app.animation_time / 0.42) as usize) % 4;
    let x = area.x + 3;
    let y = area.y + 1;
    let ear = Style::default()
        .fg(f.palette.accent)
        .add_modifier(Modifier::BOLD);
    let face = Style::default()
        .fg(f.palette.secondary_accent)
        .add_modifier(Modifier::BOLD);
    let paws = Style::default()
        .fg(f.palette.success)
        .add_modifier(Modifier::BOLD);
    let ball = Style::default()
        .fg(f.palette.warn)
        .add_modifier(Modifier::BOLD);

    Line::from(vec![Span::styled(" /\\_/\\ ", ear)]).render(Rect::new(x, y, 12, 1), f.buf);
    let eyes = match phase {
        1 => "( ^.^ )",
        3 => "( o.o )",
        _ => "( •.• )",
    };
    Line::from(vec![Span::styled(eyes, face)]).render(Rect::new(x, y + 1, 12, 1), f.buf);

    let (paw, ball_offset) = match phase {
        0 => ("> ^ <", 9usize),
        1 => ("> ^/", 7usize),
        2 => ("> ^ <", 5usize),
        _ => ("/^ <", 7usize),
    };
    let spaces = ball_offset.saturating_sub(paw.len() + 1);
    Line::from(vec![
        Span::raw(" "),
        Span::styled(paw, paws),
        Span::raw(" ".repeat(spaces)),
        Span::styled("●", ball),
    ])
    .render(Rect::new(x, y + 2, area.width.saturating_sub(3), 1), f.buf);
}

fn highlight_line(f: &Frame, text: &str, query: &str) -> Line<'static> {
    if query.is_empty() {
        return Line::from(Span::styled(
            text.to_string(),
            Style::default().fg(f.palette.text),
        ));
    }
    let lower = text.to_ascii_lowercase();
    let q = query.to_ascii_lowercase();
    let mut spans = Vec::new();
    let mut pos = 0usize;
    while let Some(at) = lower[pos..].find(&q) {
        let start = pos + at;
        if start > pos {
            spans.push(Span::styled(
                text[pos..start].to_string(),
                Style::default().fg(f.palette.text),
            ));
        }
        spans.push(Span::styled(
            text[start..start + q.len()].to_string(),
            Style::default()
                .fg(f.palette.bg)
                .bg(f.palette.accent)
                .add_modifier(Modifier::BOLD),
        ));
        pos = start + q.len();
    }
    if pos < text.len() {
        spans.push(Span::styled(
            text[pos..].to_string(),
            Style::default().fg(f.palette.text),
        ));
    }
    Line::from(spans)
}

// side panel
fn render_side(f: &mut Frame, area: Rect, app: &App) {
    let inner = pane(f, area, "session telemetry", false);
    if inner.width < 4 || inner.height < 2 {
        return;
    }
    let x = inner.x + 1;
    let w = inner.width.saturating_sub(2);
    let mut y = inner.y;

    let lbl = Style::default().fg(f.palette.muted);
    let val = Style::default().fg(f.palette.text);
    let acc = Style::default()
        .fg(f.palette.accent)
        .add_modifier(Modifier::BOLD);

    // config
    let rows: &[(&str, String)] = &[
        ("model", truncate(&app.model_label, 32)),
        ("threads", format!("{} cores", app.threads)),
        ("lang", app.lang.clone()),
        ("format", app.format.extension().to_string()),
    ];
    for (k, v) in rows {
        if y >= inner.y + inner.height {
            return;
        }
        Line::from(vec![
            Span::styled(pad_to(k, 10), lbl),
            Span::styled(v.clone(), val),
        ])
        .render(Rect::new(x, y, w, 1), f.buf);
        y += 1;
    }

    // divider
    if y < inner.y + inner.height {
        Span::styled(
            "─".repeat(w as usize),
            Style::default().fg(f.palette.surface1),
        )
        .render(Rect::new(x, y, w, 1), f.buf);
        y += 1;
    }

    // progress
    let pct = progress_percent(app.progress);
    let state_color = match app.status {
        Status::Ready => f.palette.success,
        Status::Error => f.palette.error,
        Status::Cancelled => f.palette.warn,
        _ if app.status.active() => f.palette.accent,
        _ => f.palette.text,
    };

    if y < inner.y + inner.height {
        let processed_str = if let Some(tot) = app.source_duration.filter(|d| *d > 0.0) {
            format!(
                "{} / {}",
                duration_short(app.done_seconds),
                duration_short(tot)
            )
        } else if app.done_seconds > 0.0 {
            duration_short(app.done_seconds)
        } else {
            "00:00".to_string()
        };

        Line::from(vec![
            Span::styled(pad_to("progress", 10), lbl),
            Span::styled(
                format!("{pct}% "),
                Style::default()
                    .fg(state_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(processed_str, Style::default().fg(f.palette.subtext0)),
        ])
        .render(Rect::new(x, y, w, 1), f.buf);
        y += 1;
    }
    if y < inner.y + inner.height {
        let value_x = x + 10;
        render_progress(f, Rect::new(value_x, y, w.saturating_sub(10), 1), app);
        y += 1;
    }

    // timing
    let elapsed = duration_short(app.elapsed_s());
    let eta = app
        .eta_s()
        .map(duration_short)
        .unwrap_or_else(|| "--:--".into());
    let speed = app
        .speed_multiplier()
        .map(|s| format!("{s:.1}x"))
        .unwrap_or_else(|| "—".into());
    let speed_ok = app.speed_multiplier().map(|s| s >= 1.0).unwrap_or(false);

    let timing: &[(&str, String, Style)] = &[
        ("elapsed", elapsed, val),
        (
            "eta",
            eta,
            if app.eta_s().map(|e| e > 30.0).unwrap_or(false) {
                Style::default().fg(f.palette.warn)
            } else {
                val
            },
        ),
        ("speed", speed, if speed_ok { acc } else { val }),
    ];
    for (k, v, style) in timing {
        if y >= inner.y + inner.height {
            return;
        }
        Line::from(vec![
            Span::styled(pad_to(k, 10), lbl),
            Span::styled(v.clone(), *style),
        ])
        .render(Rect::new(x, y, w, 1), f.buf);
        y += 1;
    }

    // transcript stats
    let total_words: usize = app
        .segments
        .iter()
        .map(|s| s.text.split_whitespace().count())
        .sum();
    let stats: &[(&str, String)] = &[
        ("segments", app.segments.len().to_string()),
        ("words", total_words.to_string()),
    ];
    for (k, v) in stats {
        if y >= inner.y + inner.height {
            return;
        }
        Line::from(vec![
            Span::styled(pad_to(k, 10), lbl),
            Span::styled(v.clone(), val),
        ])
        .render(Rect::new(x, y, w, 1), f.buf);
        y += 1;
    }

    // separator before meter
    if y < inner.y + inner.height {
        Span::styled(
            "─".repeat(w as usize),
            Style::default().fg(f.palette.surface1),
        )
        .render(Rect::new(x, y, w, 1), f.buf);
        y += 1;
    }

    // audio spectrum
    if y < inner.y + inner.height {
        Line::from(vec![
            Span::styled("audio ", lbl),
            Span::styled(
                if app.status.active() {
                    "live spectrum"
                } else {
                    "ready"
                },
                Style::default().fg(f.palette.secondary_accent),
            ),
        ])
        .render(Rect::new(x, y, w, 1), f.buf);
        y += 1;
    }

    // give the spectrum whatever vertical space is left.
    if y < inner.y + inner.height {
        let meter_h = inner.y + inner.height - y;
        render_meter(f, Rect::new(x, y, w, meter_h), app);
    }
}

fn progress_percent(progress: f64) -> u32 {
    (progress.clamp(0.0, 1.0) * 100.0).round() as u32
}

// progress bar
const BAR_FULL: [&str; 9] = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];

fn render_progress(f: &mut Frame, area: Rect, app: &App) {
    let w = area.width as usize;
    if w == 0 {
        return;
    }
    let frac = app.progress_smooth.clamp(0.0, 1.0) as f64;
    let filled = frac * w as f64;
    for i in 0..w {
        let local = (filled - i as f64).clamp(0.0, 1.0);
        let idx = (local * 8.0).round() as usize;
        let ch = BAR_FULL[idx.min(8)];
        let color = if i as f64 <= filled {
            let t = anim::ease_out_cubic(i as f32 / w.max(1) as f32);
            let base = crate::theme::mix(f.palette.accent, f.palette.success, t);
            let head_dist = (filled - i as f64).abs();
            if head_dist < 3.0 && app.status.active() {
                let pulse =
                    (1.0 - head_dist / 3.0) as f32 * (0.5 + 0.5 * (app.tick as f32 * 0.12).sin());
                crate::theme::mix(base, f.palette.text, pulse * 0.5)
            } else {
                base
            }
        } else {
            f.palette.border
        };
        Span::styled(ch.to_string(), Style::default().fg(color))
            .render(Rect::new(area.x + i as u16, area.y, 1, 1), f.buf);
    }
}

// audio equalizer
const CAVA_BARS: [&str; 8] = [" ", " ", "▂", "▃", "▄", "▅", "▆", "▇"];

fn render_meter(f: &mut Frame, area: Rect, app: &App) {
    let w = area.width as usize;
    let h = area.height as usize;
    if w == 0 || h == 0 {
        return;
    }

    let n = app.envelope_smooth.len();
    let is_active = app.status.active();

    for r in 0..h {
        let row_y = area.y + r as u16;
        let inv_r = h.saturating_sub(1).saturating_sub(r) as f32;
        let row_min = inv_r / h as f32;
        let row_max = (inv_r + 1.0) / h as f32;

        for i in 0..w {
            let bi = (i * n / w).min(n.saturating_sub(1));
            let v = if is_active {
                app.envelope_smooth
                    .get(bi)
                    .copied()
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0)
            } else {
                // smooth, offset sine bands make a cava-like idle spectrum.
                // animation_time advances only from elapsed loop time, so key
                // repeats and redraws cannot change its speed.
                let x = i as f32 / w.max(1) as f32;
                let t = app.animation_time;
                let primary = ((x * 9.0 - t * 1.35).sin() * 0.5 + 0.5).powf(1.7);
                let secondary = ((x * 17.0 + t * 0.82 + 1.4).sin() * 0.5 + 0.5).powf(2.2);
                (0.06 + primary * 0.48 + secondary * 0.27).clamp(0.0, 0.92)
            };

            let ch = if v >= row_max {
                "█"
            } else if v <= row_min {
                " "
            } else {
                let frac = ((v - row_min) / (row_max - row_min)).clamp(0.0, 1.0);
                let level = (frac * 7.0).round() as usize;
                CAVA_BARS[level.min(7)]
            };

            let height_ratio = (h - r) as f32 / h as f32;
            let color = spectrum_color(f, height_ratio);

            Span::styled(ch, Style::default().fg(color))
                .render(Rect::new(area.x + i as u16, row_y, 1, 1), f.buf);
        }
    }
}

fn spectrum_color(f: &Frame, t: f32) -> ratatui::style::Color {
    // Fixed vertical gradient from the selected palette: only bar heights
    // move; the colors never drift with time or horizontal position.
    let t = t.clamp(0.0, 1.0);
    if t < 0.34 {
        crate::theme::mix(f.palette.accent, f.palette.secondary_accent, t / 0.34)
    } else if t < 0.72 {
        crate::theme::mix(
            f.palette.secondary_accent,
            f.palette.success,
            (t - 0.34) / 0.38,
        )
    } else {
        crate::theme::mix(f.palette.success, f.palette.warn, (t - 0.72) / 0.28)
    }
}

/// a transparent input strip: just a label and a text row, no box or fill, so
/// the terminal canvas shows straight through.
fn floating_dock(f: &mut Frame, area: Rect, title: &str, focused: bool) -> Rect {
    let label = format!(" {title} ");
    Span::styled(
        label.clone(),
        Style::default()
            .fg(pane_color(f, title, focused))
            .add_modifier(Modifier::BOLD),
    )
    .render(Rect::new(area.x, area.y, label.len() as u16, 1), f.buf);

    Rect::new(
        area.x,
        area.y + 1,
        area.width,
        area.height.saturating_sub(1),
    )
}

/// the intellisense-style suggestion list. floats above the source strip,
/// lists ranked candidates with type icons, and highlights the active row.
fn render_completion_popup(f: &mut Frame, input_area: Rect, app: &App) {
    let c = &app.completion;
    if c.dismissed || c.candidates.is_empty() || app.focus != Focus::Input {
        return;
    }

    const MAX_ROWS: usize = 8;
    let total = c.candidates.len();
    let window = total.min(MAX_ROWS);
    let top = (c.selected + 1).saturating_sub(window);
    let rows = window as u16;
    let popup_h = rows + 2;
    let available = input_area.y.min(f.area.bottom().saturating_sub(f.area.y));
    if available < popup_h {
        return;
    }

    let popup_w = input_area.width.clamp(24, 72);
    let x = input_area.x;
    let y = input_area.y.saturating_sub(popup_h);
    let popup = Rect::new(x, y, popup_w, popup_h);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(f.palette.border))
        .title(Span::styled(
            format!(" suggestions · {total} "),
            Style::default()
                .fg(f.palette.secondary_accent)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup);
    block.render(popup, f.buf);
    clear_transparent(f.buf, inner);

    for (i, cand) in c.candidates.iter().skip(top).take(window).enumerate() {
        let row_y = inner.y + i as u16;
        if row_y >= inner.bottom() {
            break;
        }
        let selected = top + i == c.selected;
        let icon = if cand.is_dir { "▸" } else { "·" };
        let icon_color = if cand.is_dir {
            f.palette.accent
        } else {
            f.palette.subtext0
        };
        let text = if selected {
            cand.value.as_str()
        } else {
            cand.label.as_str()
        };
        let max = inner.width.saturating_sub(4) as usize;
        Line::from(vec![
            Span::styled(
                if selected { "❯ " } else { "  " },
                Style::default()
                    .fg(f.palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(icon, Style::default().fg(icon_color)),
            Span::raw(" "),
            Span::styled(
                truncate(text, max),
                Style::default().fg(if selected {
                    f.palette.text
                } else {
                    f.palette.subtext0
                }),
            ),
        ])
        .render(
            Rect::new(inner.x + 1, row_y, inner.width.saturating_sub(2), 1),
            f.buf,
        );
    }

    if total > window {
        let more = format!(" {} more ", total - window);
        Span::styled(more.clone(), Style::default().fg(f.palette.muted)).render(
            Rect::new(
                popup.right().saturating_sub(more.len() as u16 + 1),
                popup.y,
                more.len() as u16,
                1,
            ),
            f.buf,
        );
    }
}

// source / input pane
fn render_input(f: &mut Frame, area: Rect, app: &App) {
    let inner = floating_dock(f, area, "source", app.focus == Focus::Input);
    let (text, color) = if app.input.is_empty() {
        (
            "paste file path or http(s) url here…  (tab: complete  enter: start)",
            f.palette.muted,
        )
    } else {
        (app.input.as_str(), f.palette.text)
    };

    let cursor = if app.focus == Focus::Input {
        "> "
    } else {
        "  "
    };

    let badge = if app.input.is_empty() {
        Span::raw("")
    } else if is_url(&app.input) {
        if app.input.contains("youtu") {
            Span::styled(
                " [youtube] ",
                Style::default()
                    .fg(f.palette.accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                " [url] ",
                Style::default()
                    .fg(f.palette.secondary_accent)
                    .add_modifier(Modifier::BOLD),
            )
        }
    } else if std::path::Path::new(&app.input).is_file() {
        Span::styled(
            " [ready] ",
            Style::default()
                .fg(f.palette.success)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(" [path?] ", Style::default().fg(f.palette.muted))
    };

    let badge_w = badge.content.chars().count() as u16;
    let text_max_w = inner.width.saturating_sub(4 + badge_w) as usize;

    let ghost = app.ghost_suffix();
    let mut spans = vec![
        Span::styled(
            cursor,
            Style::default()
                .fg(f.palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(truncate(text, text_max_w), Style::default().fg(color)),
    ];
    if !ghost.is_empty() {
        spans.push(Span::styled(
            truncate(
                &ghost,
                text_max_w.saturating_sub(truncate(text, text_max_w).chars().count()),
            ),
            Style::default()
                .fg(f.palette.muted)
                .add_modifier(Modifier::DIM),
        ));
    }
    Line::from(spans).render(
        Rect::new(
            inner.x + 1,
            inner.y,
            inner.width.saturating_sub(2 + badge_w),
            1,
        ),
        f.buf,
    );

    if badge_w > 0 {
        badge.render(
            Rect::new(
                inner.x + inner.width.saturating_sub(badge_w + 1),
                inner.y,
                badge_w,
                1,
            ),
            f.buf,
        );
    }

    if !app.completion.candidates.is_empty() && !app.completion.dismissed {
        let current = app
            .completion
            .current()
            .map(|c| format!("❯ {}  ", c.value))
            .unwrap_or_default();
        let hint = format!("{current}↑↓ select  ·  tab accept  ·  esc dismiss");
        Span::styled(
            truncate(&hint, inner.width.saturating_sub(2) as usize),
            Style::default().fg(f.palette.muted),
        )
        .render(
            Rect::new(inner.x + 1, inner.y + 1, inner.width.saturating_sub(2), 1),
            f.buf,
        );
    }

    render_completion_popup(f, area, app);
}

// footer
fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let mut spans: Vec<Span> = Vec::new();
    let show_pulse = app.status.active();

    let items: &[(&str, &str)] = if app.overlay != Overlay::None {
        &[("esc", "back / close")]
    } else if app.status == Status::Paused {
        &[
            ("space / p / enter", "resume"),
            ("c / esc", "stop"),
            ("tab", "switch pane"),
            ("e", "export"),
            ("?", "help"),
        ]
    } else if app.status.active() {
        &[
            ("esc / space / p", "pause"),
            ("c", "stop"),
            ("tab", "switch pane"),
            ("/", "search"),
            ("e", "export"),
            ("?", "help"),
        ]
    } else if app.focus == Focus::Input {
        &[
            ("enter", "transcribe"),
            ("tab", "browse transcript"),
            ("m", "models"),
            ("t", "themes"),
            ("s", "settings"),
            ("e", "export"),
            ("?", "help"),
            ("ctrl+c", "quit"),
        ]
    } else {
        &[
            ("j/k", "navigate"),
            ("y", "copy osc52"),
            ("e", "export"),
            ("t", "themes"),
            ("/", "search"),
            ("tab", "input source"),
            ("?", "help"),
        ]
    };

    spans.push(Span::raw(" "));
    for (key, desc) in items {
        let pulse = show_pulse && (*desc == "pause" || *desc == "stop");
        let mut key_style = Style::default()
            .bg(f.palette.surface1)
            .fg(f.palette.accent)
            .add_modifier(Modifier::BOLD);
        if pulse {
            let t = anim::ease_out_cubic((app.tick as f32 * 0.06).sin().mul_add(0.5, 0.5));
            key_style = key_style.bg(crate::theme::mix(f.palette.surface1, f.palette.surface, t));
        }
        spans.push(Span::styled(format!(" {key} "), key_style));
        spans.push(Span::styled(
            format!(" {desc}  "),
            Style::default().fg(f.palette.subtext0),
        ));
    }

    Line::from(spans).render(Rect::new(area.x, area.y, area.width, 1), f.buf);
}

// fullscreen overlay helpers
/// how many model rows fit in the picker for a given terminal height. shared
/// with the key handler so navigation and rendering agree on the viewport.
pub fn picker_visible_rows(screen_height: u16, downloading: bool) -> usize {
    let inner_h = screen_height.saturating_sub(2);
    let footer = if downloading { 3 } else { 2 };
    let rows = inner_h.saturating_sub(2 + footer);
    rows as usize
}

fn fullscreen(f: &mut Frame, title: &str, hint: &str) -> Rect {
    let area = f.area;

    clear_transparent(f.buf, area);

    let title_line = Line::from(vec![
        Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(f.palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("· {hint} "), Style::default().fg(f.palette.muted)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(f.palette.accent))
        .title(title_line);

    let inner = block.inner(area);
    block.render(area, f.buf);
    clear_transparent(f.buf, inner);

    if inner.height > 1 {
        // keep the hint inside the frame, not on the bottom border where it
        // used to leave a little grey box stuck to the corner.
        let foot_y = inner.y + inner.height.saturating_sub(1);
        let close_hint = "esc: close";
        let hx = inner.x + inner.width.saturating_sub(close_hint.len() as u16 + 1);
        Span::styled(
            close_hint,
            Style::default()
                .fg(f.palette.accent)
                .add_modifier(Modifier::BOLD),
        )
        .render(Rect::new(hx, foot_y, close_hint.len() as u16, 1), f.buf);
    }

    Rect::new(
        inner.x + 1,
        inner.y + 1,
        inner.width.saturating_sub(2),
        inner.height.saturating_sub(2),
    )
}

// help
fn render_help(f: &mut Frame) {
    let inner = fullscreen(f, "keyboard shortcuts", "? to toggle");

    let sections: &[(&str, &[(&str, &str)])] = &[
        (
            "autocomplete",
            &[
                ("tab", "accept suggestion  (or fill shared prefix)"),
                ("↑ / ↓", "move through suggestions"),
                ("→", "accept the highlighted suggestion"),
                ("ctrl+space", "force the suggestion list open"),
                ("esc", "dismiss the suggestion list"),
            ],
        ),
        (
            "transcription",
            &[
                (
                    "enter",
                    "start  (or resume when paused / accept completion)",
                ),
                ("esc / space / p", "pause active job  (again resumes)"),
                ("c", "stop active job  (keeps saved segments)"),
                ("m", "open model picker"),
                ("s", "open settings"),
            ],
        ),
        (
            "navigation",
            &[
                ("j / k", "move selection down / up"),
                ("↑ / ↓", "same as j / k"),
                ("page dn/up", "scroll by 10 lines"),
                ("tab", "switch between source bar and transcript"),
            ],
        ),
        (
            "transcript",
            &[
                ("/", "open live search"),
                ("n / N", "next / previous match"),
                ("y", "copy selected line via osc 52  (works over ssh)"),
                ("e", "export  (txt  srt  vtt  md  json)"),
                ("enter", "open export menu when transcript has content"),
            ],
        ),
        (
            "theming & models",
            &[
                ("t", "open theme palette picker (live preview)"),
                ("m", "open whisper model catalog"),
                ("p", "pause download (keeps partial file)"),
                ("c", "cancel download and delete partial file"),
                ("x", "remove a cached model"),
                ("home / end", "jump to first / last model"),
                ("s", "settings menu (threads, language, output, format)"),
            ],
        ),
        (
            "general",
            &[
                ("?", "toggle this help screen"),
                ("ctrl+c", "graceful quit"),
            ],
        ),
    ];

    let mut y = inner.y;
    for (section, keys) in sections {
        if y >= inner.y + inner.height {
            break;
        }
        Span::styled(
            *section,
            Style::default()
                .fg(f.palette.secondary_accent)
                .add_modifier(Modifier::BOLD),
        )
        .render(Rect::new(inner.x, y, inner.width, 1), f.buf);
        y += 1;

        for (key, desc) in *keys {
            if y >= inner.y + inner.height {
                break;
            }
            Line::from(vec![
                Span::styled(
                    format!("  {}", pad_to(key, 12)),
                    Style::default()
                        .fg(f.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(*desc, Style::default().fg(f.palette.text)),
            ])
            .render(Rect::new(inner.x, y, inner.width, 1), f.buf);
            y += 1;
        }
        y += 1;
    }
}

fn render_theme_picker(f: &mut Frame, app: &App) {
    let inner = fullscreen(f, "themes", "j/k preview  enter keep  esc cancel");

    let themes = crate::theme::ThemeKind::ALL;
    let selected = app.theme_index.min(themes.len().saturating_sub(1));
    let page_rows = inner.height as usize;
    let start = selected.saturating_sub(page_rows.saturating_sub(1) / 2);
    let end = (start + page_rows).min(themes.len());

    for (row, i) in (start..end).enumerate() {
        let theme = themes[i];
        let is_selected = i == selected;
        let is_active = theme == app.theme;
        let p = crate::theme::palette(theme, crate::theme::ColorDepth::Truecolor);
        let name_style = if is_selected {
            Style::default()
                .fg(f.palette.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(f.palette.subtext0)
        };
        let mut spans = vec![
            Span::styled(
                if is_selected { "› " } else { "  " },
                Style::default()
                    .fg(if is_selected {
                        p.accent
                    } else {
                        f.palette.muted
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(pad_to(theme.name(), 25), name_style),
            Span::styled(
                format!("{:<16} ", theme.key()),
                Style::default().fg(f.palette.muted),
            ),
            Span::styled("● ", Style::default().fg(p.accent)),
            Span::styled("● ", Style::default().fg(p.secondary_accent)),
            Span::styled("● ", Style::default().fg(p.success)),
            Span::styled("● ", Style::default().fg(p.warn)),
            Span::styled("● ", Style::default().fg(p.error)),
        ];
        if is_active {
            spans.push(Span::styled(
                "  active",
                Style::default().fg(p.success).add_modifier(Modifier::BOLD),
            ));
        }
        Line::from(spans).render(
            Rect::new(inner.x, inner.y + row as u16, inner.width, 1),
            f.buf,
        );
    }
    if inner.height > 1 {
        let caption_y = inner.y + inner.height - 1;
        let caption = themes[selected].description();
        Span::styled(
            truncate(caption, inner.width as usize),
            Style::default().fg(f.palette.muted),
        )
        .render(Rect::new(inner.x, caption_y, inner.width, 1), f.buf);
    }
}

fn render_settings(f: &mut Frame, app: &App) {
    let inner = fullscreen(
        f,
        "settings",
        "j/k select  ←/→ change  enter choose  esc close",
    );

    let rows = [
        (
            "threads",
            app.threads.to_string(),
            "parallel cpu workers for whisper",
        ),
        (
            "language",
            app.lang.clone(),
            "target language (auto = detect)",
        ),
        (
            "translate",
            if app.translate {
                "on".into()
            } else {
                "off".into()
            },
            "translate output to english",
        ),
        (
            "output dir",
            if app.out_dir.is_empty() {
                "current directory".into()
            } else {
                app.out_dir.clone()
            },
            "left: parent  right/enter: first child",
        ),
        (
            "theme",
            app.theme.name().into(),
            "color palette  (dark / light / contrast)",
        ),
        (
            "format",
            app.format.extension().into(),
            "default export format",
        ),
    ];

    let start_y = inner.y;
    for (i, (key, val, hint)) in rows.iter().enumerate() {
        let y = start_y + i as u16;
        if y >= inner.y + inner.height {
            break;
        }
        let selected = i == app.settings_index;

        let key_style = if selected {
            Style::default()
                .fg(f.palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(f.palette.muted)
        };
        let val_style = if selected {
            Style::default()
                .fg(f.palette.success)
                .add_modifier(Modifier::BOLD)
        } else {
            match *key {
                "threads" => Style::default().fg(f.palette.accent),
                _ => Style::default().fg(f.palette.text),
            }
        };
        let marker = if selected { "▸ " } else { "  " };
        Line::from(vec![
            Span::styled(marker, key_style),
            Span::styled(pad_to(key, 14), key_style),
            Span::styled(pad_to(val, 18), val_style),
            Span::styled(
                hint.to_string(),
                Style::default().fg(if selected {
                    f.palette.subtext0
                } else {
                    f.palette.muted
                }),
            ),
        ])
        .render(Rect::new(inner.x, y, inner.width, 1), f.buf);
    }
}

// model picker
fn render_picker(f: &mut Frame, app: &App) {
    let is_downloading = app.download == DownloadState::Downloading;

    // reserve bottom 3 rows: separator + progress bar + hint
    let progress_rows: u16 = if is_downloading || app.download == DownloadState::Done {
        3
    } else {
        2
    };

    let inner = fullscreen(
        f,
        "model picker",
        if is_downloading {
            "j/k select  p pause  c cancel  x remove  esc close"
        } else {
            "j/k select  enter select/get/resume  x remove  esc close"
        },
    );

    // compact, fixed-width columns. the model name is capped so short names
    // never leave a canyon before the disk column; any spare width is appended
    // after the status badge instead of being stuffed into one column.
    let avail = inner.width as usize;
    let marker = 2usize;
    let tiny = avail < 56;
    // widths hug real content; extra columns are dropped when space is tight.
    let full = avail >= 100;
    let col_model = if tiny {
        6
    } else if avail < 100 {
        12
    } else {
        19
    };
    let col_num = 6;
    let col_lang = if full { 5 } else { 4 };
    let col_status = if tiny { 5 } else { 7 };
    let col_sha = if full { 12 } else { 0 };
    let col_speed = if tiny { 7 } else { 8 };
    let col_note = if full { 12 } else { 0 };
    // two blank cells after every column keep the table readable.
    let gap = if avail < 52 { 1 } else { 2 };
    let columns = 7 - usize::from(col_note == 0) - usize::from(col_sha == 0);
    let used = marker
        + col_model
        + col_num * 2
        + col_speed
        + col_note
        + col_lang
        + col_sha
        + col_status
        + gap * columns;
    let trailing = avail.saturating_sub(used);

    // emit a value padded to its column width, followed by the gap.
    let cell = |text: &str, width: usize| -> String {
        format!("{}{}", pad_to(text, width), " ".repeat(gap))
    };

    let header_spans = |bold: bool| {
        let h = Style::default().fg(f.palette.muted);
        let h = if bold {
            h.add_modifier(Modifier::BOLD)
        } else {
            h
        };
        let mut v = vec![
            Span::raw("  "),
            Span::styled(cell("model", col_model), h),
            Span::styled(cell("disk", col_num), h),
            Span::styled(cell("ram", col_num), h),
            Span::styled(cell("speed", col_speed), h),
        ];
        if col_note > 0 {
            v.push(Span::styled(cell("note", col_note), h));
        }
        v.push(Span::styled(cell("lang", col_lang), h));
        if col_sha > 0 {
            v.push(Span::styled(cell("sha", col_sha), h));
        }
        v.push(Span::styled(cell("status", col_status), h));
        if trailing > 0 {
            v.push(Span::raw(" ".repeat(trailing)));
        }
        v
    };

    if inner.height > 1 {
        Line::from(header_spans(true)).render(Rect::new(inner.x, inner.y, inner.width, 1), f.buf);
    }

    if inner.height > 2 {
        Span::styled(
            "─".repeat(inner.width as usize),
            Style::default().fg(f.palette.surface1),
        )
        .render(Rect::new(inner.x, inner.y + 1, inner.width, 1), f.buf);
    }

    // list area (leave space for the progress footer)
    let list_y = inner.y + 2;
    let list_max_y = inner.y + inner.height.saturating_sub(progress_rows);
    let visible_rows = list_max_y.saturating_sub(list_y) as usize;

    // the key handler already keeps picker_top in sync with the selection;
    // clamp here as a safety net for terminal resizes.
    let top = app
        .picker_top
        .min(app.download_models.len().saturating_sub(1));

    for (i, m) in app
        .download_models
        .iter()
        .enumerate()
        .skip(top)
        .take(visible_rows)
    {
        let y = list_y + (i - top) as u16;
        if y >= list_max_y {
            break;
        }
        let cached = model::cache_dir().join(m.file).is_file();
        let part_bytes = model::partial_size(m);
        let selected = i == app.download_index;

        // per-row status badge
        let is_this_downloading = is_downloading && m.file == app.download_file;
        let (mark, mark_color) = if cached {
            ("ready".to_string(), f.palette.success)
        } else if is_this_downloading {
            (
                format!("↓ {:.0}%", app.download_frac * 100.0),
                f.palette.accent,
            )
        } else if part_bytes.is_some() && !cached {
            let pct = part_bytes.unwrap_or(0) * 100 / (m.size_mib * 1024 * 1024).max(1);
            (format!("⏸ {pct}%"), f.palette.warn)
        } else {
            ("get".to_string(), f.palette.muted)
        };

        let lang = if m.multilingual { "multi" } else { "en" };
        let label_style = if selected {
            Style::default()
                .fg(f.palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(f.palette.text)
        };
        let dim = Style::default().fg(if selected {
            f.palette.subtext0
        } else {
            f.palette.muted
        });

        if selected {
            fill_blank(
                f.buf,
                Rect::new(inner.x, y, inner.width, 1),
                Style::default().bg(f.palette.surface),
            );
        }
        let mut spans = vec![
            Span::styled(if selected { "▸ " } else { "  " }, label_style),
            Span::styled(cell(&truncate(m.label, col_model), col_model), label_style),
            Span::styled(cell(&format!("{}m", m.size_mib), col_num), dim),
            Span::styled(cell(&format!("{}m", m.ram_mib), col_num), dim),
            Span::styled(cell(&truncate(m.speed, col_speed), col_speed), dim),
        ];
        if col_note > 0 {
            spans.push(Span::styled(
                cell(&truncate(m.note, col_note), col_note),
                dim,
            ));
        }
        spans.push(Span::styled(
            cell(lang, col_lang),
            Style::default().fg(f.palette.secondary_accent),
        ));
        if col_sha > 0 {
            spans.push(Span::styled(
                cell(&m.sha[..col_sha.min(m.sha.len())], col_sha),
                Style::default().fg(f.palette.muted),
            ));
        }
        spans.push(Span::styled(
            cell(&truncate(&mark, col_status), col_status),
            Style::default().fg(mark_color).add_modifier(Modifier::BOLD),
        ));
        if trailing > 0 {
            spans.push(Span::raw(" ".repeat(trailing)));
        }
        Line::from(spans).render(Rect::new(inner.x, y, inner.width, 1), f.buf);
    }

    // scroll indicator on the right of the header when the list overflows.
    if app.download_models.len() > visible_rows && visible_rows > 0 {
        let page = format!(" {}/{} ", top + 1, app.download_models.len());
        Span::styled(
            page.clone(),
            Style::default().fg(f.palette.accent).bg(f.palette.surface1),
        )
        .render(
            Rect::new(
                inner.right().saturating_sub(page.len() as u16),
                inner.y,
                page.len() as u16,
                1,
            ),
            f.buf,
        );
    }

    // progress footer
    let foot_sep_y = inner.y + inner.height.saturating_sub(progress_rows);
    let foot_bar_y = foot_sep_y + 1;
    let foot_hint_y = foot_bar_y + 1;

    if is_downloading && foot_sep_y < inner.y + inner.height {
        // separator
        Span::styled(
            "─".repeat(inner.width as usize),
            Style::default().fg(f.palette.surface1),
        )
        .render(Rect::new(inner.x, foot_sep_y, inner.width, 1), f.buf);

        // progress bar
        if foot_bar_y < inner.y + inner.height {
            let w = inner.width as usize;
            let frac = app.download_frac.clamp(0.0, 1.0);
            let pct = (frac * 100.0) as u32;
            let info_w = w.saturating_sub(20).min(app.download_info.len()).min(40);
            let bar_w = w.saturating_sub(info_w + 8).max(4);
            let filled = (frac * bar_w as f64).round() as usize;
            let bar: String = (0..bar_w)
                .map(|i| if i < filled { '█' } else { '░' })
                .collect();
            Line::from(vec![
                Span::styled(bar, Style::default().fg(f.palette.accent)),
                Span::styled(
                    format!(" {:>3}%  {}", pct, truncate(&app.download_info, info_w)),
                    Style::default().fg(f.palette.subtext0),
                ),
            ])
            .render(Rect::new(inner.x, foot_bar_y, inner.width, 1), f.buf);
        }

        // hint
        if foot_hint_y < inner.y + inner.height {
            Line::from(vec![
                Span::styled(
                    " p",
                    Style::default()
                        .fg(f.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" / ", Style::default().fg(f.palette.muted)),
                Span::styled(
                    "esc",
                    Style::default()
                        .fg(f.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" pause  ", Style::default().fg(f.palette.muted)),
                Span::styled(
                    "c",
                    Style::default()
                        .fg(f.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" cancel  ", Style::default().fg(f.palette.muted)),
                Span::styled(
                    "enter",
                    Style::default()
                        .fg(f.palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" select when ready", Style::default().fg(f.palette.muted)),
            ])
            .render(Rect::new(inner.x, foot_hint_y, inner.width, 1), f.buf);
        }
    } else if !is_downloading {
        // idle/done/failed status line
        let status = match app.download {
            DownloadState::Idle => {
                if app.download_info.is_empty() {
                    "small-q5_1 is a good starting point.".to_string()
                } else {
                    app.download_info.clone()
                }
            }
            DownloadState::Done => "download complete — press enter to activate".into(),
            DownloadState::Failed => format!("✕ {}", app.download_info),
            DownloadState::Downloading => String::new(),
        };
        let status_color = match app.download {
            DownloadState::Done => f.palette.success,
            DownloadState::Failed => f.palette.error,
            _ => f.palette.muted,
        };
        let sep_y = inner.y + inner.height.saturating_sub(2);
        if sep_y > list_max_y && sep_y < inner.y + inner.height {
            Span::styled(
                "─".repeat(inner.width as usize),
                Style::default().fg(f.palette.surface1),
            )
            .render(Rect::new(inner.x, sep_y, inner.width, 1), f.buf);
        }
        let hint_y = inner.y + inner.height.saturating_sub(1);
        if hint_y < inner.y + inner.height && !status.is_empty() {
            Span::styled(
                truncate(&status, inner.width as usize),
                Style::default().fg(status_color),
            )
            .render(Rect::new(inner.x, hint_y, inner.width, 1), f.buf);
        }
    }
}

// export
fn render_export(f: &mut Frame, app: &App) {
    let inner = fullscreen(f, "export transcript", "j/k select  enter save  esc close");

    let formats = App::export_formats();

    Span::styled(
        "choose a format",
        Style::default()
            .fg(f.palette.secondary_accent)
            .add_modifier(Modifier::BOLD),
    )
    .render(Rect::new(inner.x, inner.y, inner.width, 1), f.buf);

    let start_y = inner.y + 2;
    for (i, fmt) in formats.iter().enumerate() {
        let y = start_y + i as u16;
        if y >= inner.y + inner.height {
            break;
        }
        let path = crate::util::output_path(
            &app.input,
            std::path::Path::new(&app.out_dir)
                .to_str()
                .filter(|s| !s.is_empty())
                .map(std::path::Path::new),
            fmt.extension(),
        );
        let selected = i == app.export_index;

        if selected {
            fill_blank(
                f.buf,
                Rect::new(inner.x, y, inner.width, 1),
                Style::default().bg(f.palette.surface),
            );
        }

        let fmt_color = match fmt.extension() {
            "txt" => f.palette.accent,
            "srt" => f.palette.warn,
            "vtt" => f.palette.success,
            "md" => f.palette.secondary_accent,
            "json" => f.palette.muted,
            _ => f.palette.accent,
        };
        let ext_style = if selected {
            Style::default()
                .fg(f.palette.bg)
                .bg(fmt_color)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(fmt_color).add_modifier(Modifier::BOLD)
        };

        Line::from(vec![
            Span::styled(
                if selected { "▸ " } else { "  " },
                Style::default().fg(f.palette.accent),
            ),
            Span::styled(format!("{:<5}", fmt.extension()), ext_style),
            Span::styled("  →  ", Style::default().fg(f.palette.muted)),
            Span::styled(
                truncate(
                    &path.display().to_string(),
                    inner.width.saturating_sub(14) as usize,
                ),
                Style::default().fg(if selected {
                    f.palette.text
                } else {
                    f.palette.subtext0
                }),
            ),
        ])
        .render(Rect::new(inner.x, y, inner.width, 1), f.buf);
    }
}

// search
fn render_search(f: &mut Frame, app: &App) {
    let popup_w = f.area.width.saturating_sub(4).min(64);
    let popup_h = 3u16;
    let px = f.area.x + (f.area.width.saturating_sub(popup_w)) / 2;
    let py = f.area.y + f.area.height.saturating_sub(5);
    let popup_area = Rect::new(px, py, popup_w, popup_h);

    clear_transparent(f.buf, popup_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(f.palette.accent))
        .title(Span::styled(
            " search ",
            Style::default()
                .fg(f.palette.accent)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup_area);
    block.render(popup_area, f.buf);
    clear_transparent(f.buf, inner);

    let match_hint = if app.matches.is_empty() && !app.query.is_empty() {
        " [0 matches]"
    } else if !app.matches.is_empty() {
        " [n/N jump]"
    } else {
        ""
    };

    Line::from(vec![
        Span::styled(
            "▸ ",
            Style::default()
                .fg(f.palette.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.query.clone(),
            Style::default()
                .fg(f.palette.text)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(match_hint, Style::default().fg(f.palette.muted)),
    ])
    .render(
        Rect::new(inner.x + 1, inner.y, inner.width.saturating_sub(2), 1),
        f.buf,
    );
}

/// transient notices stacked in the bottom-right corner. newest sits at the
/// bottom so the stack grows upward like a proper notification tray.
fn render_toasts(f: &mut Frame, app: &App) {
    if app.toasts.is_empty() {
        return;
    }
    let max_w = (f.area.width / 2).clamp(28, 56) as usize;
    let shown = app.toasts.iter().rev().take(4).collect::<Vec<_>>();
    let mut rows: Vec<(Rect, &crate::app::Toast)> = Vec::new();
    let mut cursor = f.area.bottom().saturating_sub(1);

    for toast in shown {
        let lines = wrap(&toast.text, max_w.saturating_sub(4));
        let h = (lines.len() as u16 + 2).min(6);
        if h + 1 > cursor.saturating_sub(f.area.y) {
            break;
        }
        let y = cursor.saturating_sub(h);
        let w = (max_w as u16).min(f.area.width);
        let x = f.area.right().saturating_sub(w + 1);
        rows.push((Rect::new(x, y, w, h), toast));
        cursor = y.saturating_sub(1);
    }

    for (area, toast) in rows.iter().rev() {
        let age = toast.created.elapsed().as_secs_f64();
        // fade during the final second by dropping the left border accent.
        let (color, icon) = match toast.kind {
            crate::app::ToastKind::Success => (f.palette.success, "✓"),
            crate::app::ToastKind::Warn => (f.palette.warn, "!"),
            crate::app::ToastKind::Error => (f.palette.error, "✕"),
            crate::app::ToastKind::Info => (f.palette.accent, "·"),
        };
        let border = if age > 5.0 { f.palette.border } else { color };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border))
            .title(Span::styled(
                format!(" {icon} "),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(*area);
        block.render(*area, f.buf);
        clear_transparent(f.buf, inner);
        let lines = wrap(&toast.text, inner.width.saturating_sub(1) as usize);
        for (row, line) in lines.iter().take(inner.height as usize).enumerate() {
            Span::styled(line.clone(), Style::default().fg(f.palette.text)).render(
                Rect::new(inner.x, inner.y + row as u16, inner.width, 1),
                f.buf,
            );
        }
    }
}

// notice / error message
fn render_message(f: &mut Frame, app: &App) {
    let popup_w = f.area.width.min(66);
    let text = wrap(&app.message, popup_w.saturating_sub(6) as usize);
    let popup_h = (text.len() as u16 + 4).min(12);
    let px = f.area.x + (f.area.width.saturating_sub(popup_w)) / 2;
    let py = f.area.y + (f.area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(px, py, popup_w, popup_h);

    clear_transparent(f.buf, popup_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if app.status == Status::Error {
            f.palette.error
        } else {
            f.palette.accent
        }))
        .title(Span::styled(
            " notice ",
            Style::default()
                .fg(if app.status == Status::Error {
                    f.palette.error
                } else {
                    f.palette.accent
                })
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup_area);
    block.render(popup_area, f.buf);
    clear_transparent(f.buf, inner);

    for (i, line) in text
        .iter()
        .take(inner.height.saturating_sub(1) as usize)
        .enumerate()
    {
        Span::styled(line.clone(), Style::default().fg(f.palette.text)).render(
            Rect::new(
                inner.x + 1,
                inner.y + i as u16,
                inner.width.saturating_sub(2),
                1,
            ),
            f.buf,
        );
    }
    Span::styled(
        "press esc, enter or space to dismiss",
        Style::default().fg(f.palette.muted),
    )
    .render(
        Rect::new(
            inner.x + 1,
            inner.y + inner.height.saturating_sub(1),
            inner.width.saturating_sub(2),
            1,
        ),
        f.buf,
    );
}
