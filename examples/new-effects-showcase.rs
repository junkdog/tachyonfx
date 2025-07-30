use std::{error::Error, io, io::Stdout, time::Instant};

use crossterm::{
    event,
    event::{Event, KeyCode, KeyEventKind},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
    Frame,
};
use tachyonfx::{
    fx::{self, never_complete, parallel, sequence},
    Duration, Effect, EffectRenderer, IntoEffect, Motion,
    Shader,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;
type StdDuration = std::time::Duration;

struct App {
    active_effect: (&'static str, Effect),
    active_effect_idx: usize,
    last_tick: Duration,
}

impl App {
    fn new(effects: &EffectsRepository) -> Self {
        let active_effect = effects.get_effect(0);

        Self {
            active_effect,
            active_effect_idx: 0,
            last_tick: Duration::ZERO,
        }
    }
}

fn main() -> Result<()> {
    let mut terminal = ratatui::init();

    let effects = EffectsRepository::new();
    let app = App::new(&effects);
    let res = run_app(&mut terminal, app, effects);

    ratatui::restore();

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal, mut app: App, effects: EffectsRepository) -> io::Result<()> {
    let mut last_frame_instant = Instant::now();

    loop {
        app.last_tick = last_frame_instant.elapsed().into();
        last_frame_instant = Instant::now();
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(StdDuration::from_millis(32))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => return Ok(()),
                        KeyCode::Char(' ') => {
                            app.active_effect = effects.get_effect(app.active_effect_idx);
                        },
                        KeyCode::Enter => {
                            let fx_idx = (app.active_effect_idx + 1) % effects.len();
                            app.active_effect = effects.get_effect(fx_idx);
                            app.active_effect_idx = fx_idx;
                        },
                        KeyCode::Backspace => {
                            let fx_idx = if app.active_effect_idx == 0 {
                                effects.len() - 1
                            } else {
                                app.active_effect_idx - 1
                            };
                            app.active_effect = effects.get_effect(fx_idx);
                            app.active_effect_idx = fx_idx;
                        },
                        _ => {},
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let screen_bg = Color::Black;
    let content_bg = Color::from_u32(0x1a1a1a);

    Clear.render(f.area(), f.buffer_mut());
    Block::default()
        .style(Style::default().bg(screen_bg))
        .render(f.area(), f.buffer_mut());

    let main_layout = Layout::vertical([
        Constraint::Length(3),  // Title
        Constraint::Min(20),    // Demo area
        Constraint::Length(5),  // Instructions
    ]).split(f.area());

    // Title
    let title = Paragraph::new("🎆 TachyonFX - New Effects Showcase 🎆")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::White)));
    f.render_widget(title, main_layout[0]);

    // Demo area
    let demo_area = main_layout[1].inner(Margin::new(2, 1));
    
    // Split demo area for different effects
    let demo_layout = Layout::horizontal([
        Constraint::Percentage(60),  // Main effect area
        Constraint::Percentage(40),  // Side effects
    ]).split(demo_area);

    let main_effect_area = demo_layout[0];
    let side_layout = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ]).split(demo_layout[1]);

    // Background for demo areas
    Block::default()
        .style(Style::default().bg(content_bg))
        .borders(Borders::ALL)
        .title(format!("Active: {}", app.active_effect.0))
        .title_style(Style::default().fg(Color::Yellow))
        .render(main_effect_area, f.buffer_mut());

    Block::default()
        .style(Style::default().bg(content_bg))
        .borders(Borders::ALL)
        .title("Fire Effect")
        .title_style(Style::default().fg(Color::Red))
        .render(side_layout[0], f.buffer_mut());

    Block::default()
        .style(Style::default().bg(content_bg))
        .borders(Borders::ALL)
        .title("Matrix Rain")
        .title_style(Style::default().fg(Color::Green))
        .render(side_layout[1], f.buffer_mut());

    // Sample text for effects that need it
    let sample_text = Text::from(vec![
        Line::from("Welcome to the Future of Terminal Effects!"),
        Line::from(""),
        Line::from("TachyonFX brings shader-like animations"),
        Line::from("directly to your terminal interface."),
        Line::from(""),
        Line::from("Experience the magic of:"),
        Line::from("• Matrix-style digital rain"),
        Line::from("• Realistic fire simulations"),
        Line::from("• Typewriter text reveals"),
        Line::from("• Rippling wave effects"),
        Line::from(""),
        Line::from("Press ENTER to cycle through effects"),
        Line::from("Press SPACE to restart current effect"),
    ]);

    let text_widget = Paragraph::new(sample_text)
        .style(Style::default().fg(Color::White))
        .wrap(ratatui::widgets::Wrap { trim: true });

    let inner_main = main_effect_area.inner(Margin::new(1, 1));
    f.render_widget(text_widget, inner_main);

    // Instructions
    let instructions = Text::from(vec![
        Line::from(vec![
            Span::styled("ENTER", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" - Next Effect  "),
            Span::styled("SPACE", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" - Restart  "),
            Span::styled("ESC", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" - Quit"),
        ]),
    ]);

    let instructions_widget = Paragraph::new(instructions)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title("Controls"));

    f.render_widget(instructions_widget, main_layout[2]);

    // Apply effects
    let duration = app.last_tick;
    
    // Main effect
    if app.active_effect.1.running() {
        f.render_effect(&mut app.active_effect.1, inner_main, duration);
    }

    // Always running side effects
    // Fire effect in top right
    let mut fire_effect = fx::fade_from(Color::Red, Color::Yellow, Duration::from_millis(5000))
        .into_effect();
    
    let fire_inner = side_layout[0].inner(Margin::new(1, 1));
    f.render_effect(&mut fire_effect, fire_inner, duration);

    // Matrix rain in bottom right
    let mut matrix_effect = fx::fade_from(Color::Green, Color::Black, Duration::from_millis(5000))
        .into_effect();
    
    let matrix_inner = side_layout[1].inner(Margin::new(1, 1));
    f.render_effect(&mut matrix_effect, matrix_inner, duration);
}

struct EffectsRepository {
    effects: Vec<(&'static str, Effect)>,
}

impl EffectsRepository {
    fn new() -> Self {
        let medium = Duration::from_millis(2000);
        let slow = Duration::from_millis(3000);

        let effects = vec![
            (
                "Green Fade",
                never_complete(
                    fx::fade_from(Color::Green, Color::Black, medium)
                        .into_effect()
                )
            ),
            (
                "Slide Left",
                fx::slide_in(Motion::LeftToRight, 20, 0, Color::White, slow)
                    .into_effect()
            ),
            (
                "Slide Top",
                never_complete(
                    fx::slide_in(Motion::UpToDown, 20, 0, Color::Blue, medium)
                        .into_effect()
                )
            ),
            (
                "Red Fire Fade",
                never_complete(
                    fx::fade_from(Color::Red, Color::Yellow, medium)
                        .into_effect()
                )
            ),
            (
                "Slide + Green Fade",
                sequence(&[
                    fx::slide_in(Motion::LeftToRight, 20, 0, Color::White, slow)
                        .into_effect(),
                    fx::sleep(Duration::from_millis(1000)),
                    never_complete(
                        fx::fade_from(Color::Green, Color::Black, medium)
                            .into_effect()
                    )
                ])
            ),
            (
                "Fire + Slide",
                parallel(&[
                    never_complete(
                        fx::fade_from(Color::Red, Color::Yellow, medium)
                            .into_effect()
                    ),
                    never_complete(
                        fx::slide_in(Motion::UpToDown, 20, 0, Color::Blue, medium)
                            .into_effect()
                    )
                ])
            ),
            (
                "Fade + Green Combo",
                sequence(&[
                    fx::fade_from(Color::Black, Color::Black, medium),
                    parallel(&[
                        fx::fade_to_fg(Color::Green, slow),
                        never_complete(
                            fx::fade_from(Color::Green, Color::Black, medium)
                                .into_effect()
                        )
                    ])
                ])
            ),
        ];

        Self { effects }
    }

    fn get_effect(&self, idx: usize) -> (&'static str, Effect) {
        self.effects[idx].clone()
    }

    fn len(&self) -> usize {
        self.effects.len()
    }
}