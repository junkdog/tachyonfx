//! Ambient, noise-driven effects for tachyonfx.
//!
//! tachyonfx's built-in effects are mostly *transitions*: they run once and
//! finish. This demo covers the other case — effects meant to sit under a UI
//! indefinitely, giving it a little life without demanding attention.
//!
//! Each effect is compiled from its DSL string rather than constructed in Rust,
//! so what you see on screen is exactly what a user could write in a config
//! file, with no registration of its own.
//!
//! Run with `cargo run -p organic-effects`.

use std::{
    io,
    time::{Duration as StdDuration, Instant},
};

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    prelude::*,
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
};
use tachyonfx::{Duration, Effect, EffectRenderer, dsl::EffectDsl};

/// Starting frame rate, adjustable at runtime with `+` and `-`.
///
/// Nothing here is limited by rendering: the loop waits for a key with a
/// timeout, and that timeout sets the pace. Some cap is needed or the loop
/// spins at 100% CPU redrawing as fast as it can — and since an ambient effect
/// never finishes, that cost would be paid for as long as the UI is open.
/// 30fps is a starting point, not a ceiling.
const DEFAULT_FPS: u32 = 30;
const FPS_RANGE: core::ops::RangeInclusive<u32> = 5..=240;

fn frame_budget(fps: u32) -> StdDuration {
    StdDuration::from_micros(1_000_000 / u64::from(fps))
}

struct Demo {
    name: &'static str,
    kind: Kind,
    blurb: &'static str,
    dsl: &'static str,
}

enum Kind {
    /// Sets an absolute colour, so it fills blank cells and leaves text legible.
    Painting,
    /// Adjusts the colour a cell already has, so it acts on text.
    Modulating,
}

impl Kind {
    fn label(&self) -> &'static str {
        match self {
            Kind::Painting => "painting · targets blank cells",
            Kind::Modulating => "modulating · targets glyphs",
        }
    }
}

const DEMOS: &[Demo] = &[
    Demo {
        name: "noise_field",
        kind: Kind::Painting,
        blurb: "A drifting fractal-noise field. The workhorse ambient background: \
                unlike a sine-driven one it never visibly repeats.",
        dsl: "fx::noise_field(Color::Rgb(10, 12, 30), Color::Rgb(110, 60, 160), 0.2, 1.2, 3)",
    },
    Demo {
        name: "color_wave",
        kind: Kind::Painting,
        blurb: "A three-stop gradient sweeping sideways. Periodic and directional, \
                so it reads as a deliberate sweep rather than drift.",
        dsl: "fx::color_wave(Color::Rgb(40, 20, 70), Color::Rgb(20, 70, 80), \
              Color::Rgb(90, 35, 55), 1.2, 0.06)",
    },
    Demo {
        name: "glow",
        kind: Kind::Painting,
        blurb: "A radial bloom that breathes. Corrected for cell aspect ratio, \
                so it is round rather than a vertical ellipse.",
        dsl: "fx::glow(Color::Rgb(70, 90, 140), 22.0, 2.0, 3.0)",
    },
    Demo {
        name: "breathe",
        kind: Kind::Modulating,
        blurb: "A slow brightness swell. Asymmetric on purpose — a slow rise and \
                a quicker fall reads as breathing, where a sine reads as a machine.",
        dsl: "fx::breathe(0.55, 3.0)",
    },
    Demo {
        name: "flicker",
        kind: Kind::Modulating,
        blurb: "Stochastic per-column flicker, for a phosphor-tube feel. Each \
                column has its own path, so a line never pulses in unison.",
        dsl: "fx::flicker(0.45, 3.0)",
    },
    Demo {
        name: "shimmer",
        kind: Kind::Modulating,
        blurb: "A smooth highlight sliding across the text. Where flicker is \
                electrical noise, this is a moving gradient.",
        dsl: "fx::shimmer(0.7, 0.15, 0.8)",
    },
    Demo {
        name: "drift",
        kind: Kind::Modulating,
        blurb: "A slow noise-driven wander of the text colour, uniform across \
                the area rather than varying per cell.",
        dsl: "fx::drift(Color::Rgb(120, 170, 220), Color::Rgb(230, 140, 170), 1.0)",
    },
    Demo {
        name: "composed",
        kind: Kind::Painting,
        blurb: "Painting and modulating effects run together: the filters keep \
                them out of each other's way automatically.",
        dsl: "fx::parallel(&[\
                fx::noise_field(Color::Rgb(12, 16, 32), Color::Rgb(90, 50, 140), 0.22, 1.0, 3), \
                fx::shimmer(0.4, 0.15, 0.7)])",
    },
];

/// Rolling frame-timing stats.
///
/// Frame rate alone would be misleading here: the loop blocks on
/// `event::poll(FRAME)`, so it reads ~30fps whatever the effect costs. The draw
/// time is the number that actually reflects the effect's expense, and for an
/// ambient effect -- which runs for as long as the UI is open -- that is the
/// figure worth watching.
struct Stats {
    frames: [f32; Stats::WINDOW],
    draws: [f32; Stats::WINDOW],
    next: usize,
    filled: usize,
}

impl Stats {
    /// Two seconds at ~30fps: long enough to be steady, short enough to react
    /// when switching effects.
    const WINDOW: usize = 60;

    fn new() -> Self {
        Self {
            frames: [0.0; Self::WINDOW],
            draws: [0.0; Self::WINDOW],
            next: 0,
            filled: 0,
        }
    }

    fn record(&mut self, frame: StdDuration, draw: StdDuration) {
        self.frames[self.next] = frame.as_secs_f32();
        self.draws[self.next] = draw.as_secs_f32();
        self.next = (self.next + 1) % Self::WINDOW;
        self.filled = (self.filled + 1).min(Self::WINDOW);
    }

    fn mean(values: &[f32], n: usize) -> f32 {
        if n == 0 {
            return 0.0;
        }
        values[..n].iter().sum::<f32>() / n as f32
    }

    fn fps(&self) -> f32 {
        let mean = Self::mean(&self.frames, self.filled);
        if mean > 0.0 { 1.0 / mean } else { 0.0 }
    }

    /// Mean milliseconds spent building and flushing a frame.
    fn draw_ms(&self) -> f32 {
        Self::mean(&self.draws, self.filled) * 1000.0
    }

    /// Slowest draw in the window, in milliseconds. The mean hides the spikes
    /// that are actually felt as stutter.
    fn peak_draw_ms(&self) -> f32 {
        self.draws[..self.filled]
            .iter()
            .copied()
            .fold(0.0, f32::max)
            * 1000.0
    }
}

/// Steps the target frame rate through a set of round values, so `+` and `-`
/// make a visible difference rather than nudging it by one.
fn step_fps(current: u32, up: bool) -> u32 {
    const STOPS: [u32; 7] = [5, 10, 15, 30, 60, 120, 240];
    let position = STOPS.iter().position(|&s| s == current).unwrap_or(3);
    let next = if up {
        (position + 1).min(STOPS.len() - 1)
    } else {
        position.saturating_sub(1)
    };
    STOPS[next].clamp(*FPS_RANGE.start(), *FPS_RANGE.end())
}

fn compile(index: usize) -> Effect {
    // These are built-ins, so a default DSL already knows them.
    let dsl = EffectDsl::new();
    dsl.compiler()
        .compile(DEMOS[index].dsl)
        .expect("demo expressions are checked by the unit tests")
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut index = 0usize;
    let mut effect = compile(index);
    let mut last = Instant::now();
    let mut stats = Stats::new();
    let mut target_fps = DEFAULT_FPS;

    loop {
        let delta = last.elapsed();
        last = Instant::now();

        let draw_start = Instant::now();
        terminal.draw(|f| ui(f, index, &mut effect, delta.into(), &stats, target_fps))?;
        let draw = draw_start.elapsed();
        stats.record(delta, draw);

        if event::poll(frame_budget(target_fps))?
            && let Event::Key(key) = event::read()?
            // Terminals may report releases too; without this every press
            // would advance twice and skip an effect.
            && key.kind == KeyEventKind::Press
        {
            let previous = index;
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Right | KeyCode::Char('l' | ' ') => {
                    index = (index + 1) % DEMOS.len();
                },
                KeyCode::Left | KeyCode::Char('h') => {
                    index = (index + DEMOS.len() - 1) % DEMOS.len();
                },
                KeyCode::Char('+' | '=') => {
                    target_fps = step_fps(target_fps, true);
                    stats = Stats::new();
                },
                KeyCode::Char('-' | '_') => {
                    target_fps = step_fps(target_fps, false);
                    stats = Stats::new();
                },
                _ => {},
            }
            if index != previous {
                effect = compile(index);
                // Otherwise the previous effect's timings linger in the window.
                stats = Stats::new();
            }
        }
    }

    ratatui::restore();
    Ok(())
}

fn ui(
    f: &mut Frame<'_>,
    index: usize,
    effect: &mut Effect,
    delta: Duration,
    stats: &Stats,
    target_fps: u32,
) {
    let demo = &DEMOS[index];

    Clear.render(f.area(), f.buffer_mut());
    Block::default()
        .style(Style::default().bg(Color::Rgb(16, 16, 22)))
        .render(f.area(), f.buffer_mut());

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(6),
        Constraint::Length(1),
    ])
    .split(f.area());

    let heading = Line::from(vec![
        Span::styled(
            format!(" {} ", demo.name),
            Style::default()
                .fg(Color::Rgb(240, 240, 250))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("· {} ", demo.kind.label()),
            Style::default().fg(Color::Rgb(130, 140, 165)),
        ),
        Span::styled(
            format!("· {}/{}", index + 1, DEMOS.len()),
            Style::default().fg(Color::Rgb(90, 100, 125)),
        ),
    ]);
    f.render_widget(
        Paragraph::new(heading).block(Block::default().borders(Borders::BOTTOM)),
        chunks[0],
    );

    // Deliberately mixes text with generous blank space, so painting effects
    // (which fill the blanks) and modulating ones (which act on the glyphs) are
    // both visible on the same screen.
    let body = Paragraph::new(vec![
        Line::from(""),
        Line::from(demo.blurb).style(Style::default().fg(Color::Rgb(205, 210, 230))),
        Line::from(""),
        Line::from(""),
        Line::from("      The quick brown fox jumps over the lazy dog")
            .style(Style::default().fg(Color::Rgb(150, 190, 220))),
        Line::from("      0123456789  ──────────  ▲ ▼ ◆ ●")
            .style(Style::default().fg(Color::Rgb(190, 160, 210))),
        Line::from(""),
        Line::from(""),
    ])
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(60, 66, 86)))
            .padding(Padding::horizontal(2)),
    );
    f.render_widget(body, chunks[1]);

    let source = Paragraph::new(demo.dsl)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::Rgb(150, 200, 170)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 56, 72)))
                .title(" compiled from this DSL expression ")
                .title_style(Style::default().fg(Color::Rgb(110, 120, 145)))
                .padding(Padding::horizontal(1)),
        );
    f.render_widget(source, chunks[2]);

    let footer = Line::from(vec![
        Span::styled(
            " ←/→ effect   ·   +/- frame rate   ·   q quit",
            Style::default().fg(Color::Rgb(90, 100, 125)),
        ),
        Span::styled(
            format!(
                "   ·   {:.0}/{target_fps} fps   ·   draw {:.2}ms (peak {:.2}ms)",
                stats.fps(),
                stats.draw_ms(),
                stats.peak_draw_ms(),
            ),
            Style::default().fg(Color::Rgb(120, 145, 120)),
        ),
    ]);
    f.render_widget(Paragraph::new(footer), chunks[3]);

    // Ambient effects are applied last, over everything already rendered: the
    // cell filters need the glyphs to be on screen to tell them from blanks.
    f.render_effect(effect, f.area(), delta);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(v: u64) -> StdDuration {
        StdDuration::from_millis(v)
    }

    #[test]
    fn empty_stats_report_zero_rather_than_dividing_by_zero() {
        let stats = Stats::new();
        assert_eq!(stats.fps(), 0.0);
        assert_eq!(stats.draw_ms(), 0.0);
        assert_eq!(stats.peak_draw_ms(), 0.0);
    }

    #[test]
    fn steady_frames_report_the_expected_rate() {
        let mut stats = Stats::new();
        for _ in 0..10 {
            stats.record(ms(33), ms(1));
        }
        assert!(
            (stats.fps() - 30.3).abs() < 0.5,
            "33ms frames should read ~30fps, got {}",
            stats.fps()
        );
        assert!((stats.draw_ms() - 1.0).abs() < 0.01);
    }

    /// The mean smooths away exactly the spikes that are felt as stutter, which
    /// is why the peak is reported alongside it.
    #[test]
    fn peak_survives_a_window_of_fast_frames() {
        let mut stats = Stats::new();
        stats.record(ms(33), ms(20));
        for _ in 0..20 {
            stats.record(ms(33), ms(1));
        }
        assert!(stats.draw_ms() < 3.0, "mean should stay low");
        assert!(
            (stats.peak_draw_ms() - 20.0).abs() < 0.01,
            "peak must still show the spike"
        );
    }

    #[test]
    fn the_window_forgets_old_frames() {
        let mut stats = Stats::new();
        for _ in 0..Stats::WINDOW {
            stats.record(ms(100), ms(50));
        }
        for _ in 0..Stats::WINDOW {
            stats.record(ms(10), ms(1));
        }
        assert!(
            (stats.fps() - 100.0).abs() < 1.0,
            "old slow frames should have rolled out, got {} fps",
            stats.fps()
        );
        assert!((stats.peak_draw_ms() - 1.0).abs() < 0.01);
    }
}

#[cfg(test)]
mod fps_tests {
    use super::*;

    #[test]
    fn stepping_walks_the_stops_and_saturates() {
        assert_eq!(step_fps(30, true), 60);
        assert_eq!(step_fps(30, false), 15);
        assert_eq!(step_fps(240, true), 240, "must not run past the top stop");
        assert_eq!(step_fps(5, false), 5, "must not run past the bottom stop");
    }

    #[test]
    fn every_stop_stays_within_the_supported_range() {
        let mut fps = 5;
        for _ in 0..10 {
            fps = step_fps(fps, true);
            assert!(FPS_RANGE.contains(&fps), "{fps} escaped the range");
        }
    }

    #[test]
    fn the_budget_matches_the_requested_rate() {
        assert_eq!(frame_budget(30).as_micros(), 33_333);
        assert_eq!(frame_budget(60).as_micros(), 16_666);
        // A zero budget would spin the loop; the range makes that unreachable.
        assert!(frame_budget(*FPS_RANGE.end()) > StdDuration::ZERO);
    }
}
