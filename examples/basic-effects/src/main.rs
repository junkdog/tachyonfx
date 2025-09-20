use std::{error::Error, io, io::Stdout, time::Instant};

use Gruvbox::{Light3, Orange, OrangeBright};
use Interpolation::*;
use common::gruvbox::{
    Gruvbox,
    Gruvbox::{Dark0Hard, Dark0Soft, Light4},
};
use crossterm::{
    event,
    event::{Event, KeyCode, KeyEventKind},
};
use ratatui::{
    Frame,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Clear, Widget},
};
use tachyonfx::{
    CellFilter, CenteredShrink, Duration, Effect, EffectRenderer, Interpolation, IntoEffect,
    Motion, SimpleRng, color_from_hsl,
    fx::{self, ExpandDirection, Glitch, never_complete, parallel, sequence},
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

    // create app and run it
    let effects = EffectsRepository::new();
    let app = App::new(&effects);
    let res = run_app(&mut terminal, app, effects);

    // restore terminal
    ratatui::restore();

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal, mut app: App, effects: EffectsRepository) -> io::Result<()> {
    let mut last_frame_instant = Instant::now();
    let mut rng = SimpleRng::default();

    loop {
        app.last_tick = last_frame_instant.elapsed().into();
        last_frame_instant = Instant::now();
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(StdDuration::from_millis(32))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => return Ok(()),
                        KeyCode::Char('r') => {
                            let fx_idx = (rng.r#gen() % effects.len() as u32) as usize;
                            app.active_effect = effects.get_effect(fx_idx);
                            app.active_effect_idx = fx_idx;
                        },
                        KeyCode::Char(' ') => {
                            app.active_effect = effects.get_effect(app.active_effect_idx);
                        },
                        KeyCode::Char('s') => {
                            let duration = Duration::from_secs(7);
                            app.active_effect = (
                                "scramble",
                                fx::with_duration(
                                    duration,
                                    Glitch::builder()
                                        .cell_glitch_ratio(1f32)
                                        .action_start_delay_ms(0..3000)
                                        .action_ms(8000..10_000)
                                        .build()
                                        .into_effect(),
                                ),
                            );
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
    let screen_bg: Color = Dark0Hard.into();
    let bg: Color = Dark0Soft.into();

    Clear.render(f.area(), f.buffer_mut());
    Block::default()
        .style(Style::default().bg(screen_bg))
        .render(f.area(), f.buffer_mut());

    let content_area = f.area().inner_centered(80, 17);
    Block::default()
        .style(Style::default().bg(bg))
        .render(content_area, f.buffer_mut());

    let anim_style = [Style::default().fg(Orange.into()), Style::default().fg(OrangeBright.into())];
    let text_style = Style::default().fg(Light3.into());
    let shortcut_style = [
        Style::default()
            .fg(Gruvbox::YellowBright.into())
            .add_modifier(Modifier::BOLD),
        Style::default().fg(Light4.into()),
    ];

    let layout =
        Layout::vertical([Constraint::Length(2), Constraint::Length(7), Constraint::Length(6)])
            .split(content_area.inner(Margin::new(1, 1)));

    let active_animation: Line = Line::from(vec![
        Span::from("Active animation: ").style(anim_style[0]),
        Span::from(app.active_effect.0).style(anim_style[1]),
    ]);

    let main_text = Text::from(vec![
        Line::from("Many effects are composable, e.g. `parallel`, `sequence`, `repeating`."),
        Line::from("Most effects have a lifetime, after which they report done()."),
        Line::from("Effects such as `never_complete`, `temporary` influence or override this."),
        Line::from(""),
        Line::from("The text in this window will undergo a random transition"),
        Line::from("when any of the following keys are pressed:"),
    ])
    .style(text_style);

    let shortcut = |key: &'static str, desc: &'static str| {
        Line::from(vec![
            Span::from(key).style(shortcut_style[0]),
            Span::from(desc).style(shortcut_style[1]),
        ])
    };

    let shortcuts = Text::from(vec![
        shortcut("↵   ", "next transition"),
        shortcut("⌫   ", "previous transition"),
        shortcut("␣   ", "restart transition"),
        shortcut("r   ", "random transition"),
        shortcut("s   ", "scramble text toggle"),
        shortcut("ESC ", "quit"),
    ]);

    f.render_widget(active_animation, layout[0]);
    f.render_widget(main_text, layout[1]);
    f.render_widget(shortcuts, layout[2]);

    let duration = app.last_tick;
    if app.active_effect.1.running() {
        f.render_effect(&mut app.active_effect.1, content_area, duration);
    }
}

struct EffectsRepository {
    effects: Vec<(&'static str, Effect)>,
}

impl EffectsRepository {
    fn new() -> Self {
        let screen_bg = Dark0Hard.into();
        let bg = Dark0Soft.into();

        let slow = Duration::from_millis(1250);
        let medium = Duration::from_millis(750);

        let _glitch: Effect = Glitch::builder()
            .rng(SimpleRng::default())
            .action_ms(200..400)
            .action_start_delay_ms(0..1)
            .cell_glitch_ratio(1.0)
            .build()
            .into_effect();

        // fx from lambdas
        let custom_color_cycle = fx::effect_fn(Instant::now(), slow, |state, _ctx, cell_iter| {
            let cycle: f32 = (state.elapsed().as_millis() % 3600) as f32;

            cell_iter
                .filter(|(_, cell)| cell.symbol() != " ")
                .enumerate()
                .for_each(|(i, (_pos, cell))| {
                    let hue = (2.0 * i as f32 + cycle * 0.2) % 360.0;
                    let color = color_from_hsl(hue, 100.0, 50.0);
                    cell.set_fg(color);
                });
        })
        .with_filter(CellFilter::FgColor(Light3.into()));

        let effects = vec![
            (
                "sweep in",
                fx::sweep_in(Motion::LeftToRight, 30, 0, screen_bg, (slow, QuadOut)),
            ),
            (
                "smooth expand and reversed",
                sequence(&[
                    fx::expand(
                        ExpandDirection::Vertical,
                        Style::new().fg(bg).bg(screen_bg),
                        1200,
                    ),
                    fx::sleep(slow),
                    fx::expand(
                        ExpandDirection::Horizontal,
                        Style::new().fg(bg).bg(screen_bg),
                        1200,
                    )
                    .reversed(),
                ]),
            ),
            (
                "irregular sweep out/sweep in",
                sequence(&[
                    fx::sweep_out(Motion::DownToUp, 5, 20, bg, (2000, QuadOut)),
                    fx::sweep_in(Motion::UpToDown, 5, 20, bg, (2000, QuadOut)),
                    fx::sweep_out(Motion::UpToDown, 5, 20, bg, (2000, QuadOut)),
                    fx::sweep_in(Motion::DownToUp, 5, 20, bg, (2000, QuadOut)),
                ]),
            ),
            (
                "coalesce",
                fx::sequence(&[
                    fx::coalesce((medium, CubicOut)),
                    fx::sleep(medium),
                    fx::prolong_end(
                        medium,
                        fx::dissolve_to(Style::default().bg(screen_bg), medium),
                    ),
                ]),
            ),
            (
                "slide in/out",
                fx::repeating(sequence(&[
                    parallel(&[
                        fx::fade_from_fg(bg, (2000, ExpoInOut)),
                        fx::slide_in(Motion::UpToDown, 20, 0, Dark0Hard, medium),
                    ]),
                    fx::sleep(medium),
                    fx::prolong_end(
                        medium,
                        fx::slide_out(Motion::LeftToRight, 80, 0, Dark0Hard, medium),
                    ),
                ])),
            ),
            (
                "change hue, saturation and lightness",
                sequence(&[
                    fx::hsl_shift_fg([360.0, 0.0, 0.0], medium),
                    fx::hsl_shift_fg([0.0, -100.0, 0.0], medium),
                    fx::hsl_shift_fg([0.0, -100.0, 0.0], medium).reversed(),
                    fx::hsl_shift_fg([0.0, 100.0, 0.0], medium),
                    fx::hsl_shift_fg([0.0, 100.0, 0.0], medium).reversed(),
                    fx::hsl_shift_fg([0.0, 0.0, -100.0], medium),
                    fx::hsl_shift_fg([0.0, 0.0, -100.0], medium).reversed(),
                    fx::hsl_shift_fg([0.0, 0.0, 100.0], medium),
                    fx::hsl_shift_fg([0.0, 0.0, 100.0], medium).reversed(),
                ]),
            ),
            ("custom color cycle", never_complete(custom_color_cycle)),
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
