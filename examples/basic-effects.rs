use std::{io, panic};
use std::error::Error;
use std::io::Stdout;
use std::time::{Duration, Instant};

use crossterm::{event, execute};
use crossterm::event::{DisableMouseCapture, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use rand::prelude::{SeedableRng, SmallRng};
use ratatui::backend::CrosstermBackend;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Clear, Widget};

use Gruvbox::{Light3, Orange, OrangeBright};
use Interpolation::*;
use tachyonfx::{
    CellFilter,
    CenteredShrink,
    Effect,
    EffectRenderer,
    fx::{
        self,
        Direction,
        Glitch,
        never_complete,
        parallel,
        sequence,
        timed_never_complete,
        with_duration,
    },
    Interpolatable,
    Interpolation,
    Shader,
};

use crate::gruvbox::Gruvbox;
use crate::gruvbox::Gruvbox::{Dark0Hard, Dark0Soft, Light4};

#[path = "common/gruvbox.rs"]
mod gruvbox;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;


struct App {
    active_effect: (&'static str, Effect),
    active_effect_idx: usize,
    last_tick: Duration
}

impl App {
    fn new(effects: &EffectsRepository) -> Self {
        let active_effect = effects.get_effect(0);

        Self {
            active_effect,
            active_effect_idx: 0,
            last_tick: Duration::ZERO
        }
    }
}

fn main() -> Result<()> {
    let mut terminal = setup_terminal()?;

    // create app and run it
    let effects = EffectsRepository::new();
    let app = App::new(&effects);
    let res = run_app(&mut terminal, app, effects);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal,
    mut app: App,
    effects: EffectsRepository
) -> io::Result<()> {
    let mut last_frame_instant = std::time::Instant::now();
    loop {
        app.last_tick = last_frame_instant.elapsed();
        last_frame_instant = std::time::Instant::now();
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(Duration::from_millis(32))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc       => return Ok(()),
                        KeyCode::Char('r') => {
                            let fx_idx = rand::random::<usize>() % effects.len();
                            app.active_effect = effects.get_effect(fx_idx);
                            app.active_effect_idx = fx_idx;
                        },
                        KeyCode::Char(' ') => {
                            app.active_effect = effects.get_effect(app.active_effect_idx);
                        },
                        KeyCode::Char('s') => {
                            let duration = Duration::from_secs(7);
                            let rng = SmallRng::from_entropy();
                            app.active_effect = ("scramble", fx::with_duration(duration, Glitch::builder()
                                .cell_glitch_ratio(1f32)
                                .action_start_delay_ms(0..3000)
                                .rng(rng)
                                .action_ms(8000..10_000)
                                .into())
                            );
                        },
                        KeyCode::Enter     => {
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

                        _ => {}
                    }
                }
            }
        }
    }
}

fn ui(
    f: &mut Frame,
    app: &mut App,
) {
    let screen_bg: Color = Dark0Hard.into();
    let bg: Color = Dark0Soft.into();

    Clear.render(f.size(), f.buffer_mut());
    Block::default()
        .style(Style::default().bg(screen_bg))
        .render(f.size(), f.buffer_mut());

    let content_area = f.size().inner_centered(80, 17);
    Block::default()
        .style(Style::default().bg(bg))
        .render(content_area, f.buffer_mut());


    let anim_style = [
        Style::default().fg(Orange.into()),
        Style::default().fg(OrangeBright.into())
    ];
    let text_style = Style::default().fg(Light3.into());
    let shortcut_style = [
        Style::default().fg(Gruvbox::YellowBright.into()).add_modifier(Modifier::BOLD),
        Style::default().fg(Light4.into())
    ];

    let layout = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(7),
        Constraint::Length(6),
    ]).split(content_area.inner(Margin::new(1, 1)));

    let active_animation: Line = Line::from(vec![
        Span::from("Active animation: ").style(anim_style[0]),
        Span::from(app.active_effect.0).style(anim_style[1])
    ]);

    let main_text = Text::from(vec![
        Line::from("Many effects are composable, e.g. `parallel`, `sequence`, `repeating`."),
        Line::from("Most effects have a lifetime, after which they report done()."),
        Line::from("Effects such as `never_complete`, `temporary` influence or override this."),
        Line::from(""),
        Line::from("The text in this window will undergo a random transition"),
        Line::from("when any of the following keys are pressed:"),
    ]).style(text_style);

    let shortcut = |key: &'static str, desc: &'static str| {
        Line::from(vec![
            Span::from(key).style(shortcut_style[0]),
            Span::from(desc).style(shortcut_style[1])
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
    effects: Vec<(&'static str, Effect)>
}

impl EffectsRepository {
    fn new() -> Self {
        let screen_bg = Dark0Hard;
        let bg = Dark0Soft;

        let slow = Duration::from_millis(1250);
        let medium = Duration::from_millis(750);
        let short = Duration::from_millis(320);

        let glitch = Glitch::builder()
            .rng(SmallRng::from_entropy())
            .action_ms(200..400)
            .action_start_delay_ms(0..1)
            .cell_glitch_ratio(1.0)
            .into();

        // fx from lambdas
        let custom_color_cycle = fx::effect_fn(Instant::now(), slow, |state, _ctx, cell_iter| {
            let cycle: f64 = (state.elapsed().as_millis() % 3600) as f64;

            cell_iter
                .filter(|(_, cell)| cell.symbol() != " ")
                .enumerate()
                .for_each(|(i, (_pos, cell))| {
                    let hue = (2.0 * i as f64 + cycle * 0.2) % 360.0;
                    let color = Color::from_hsl(hue, 100.0, 50.0);
                    cell.set_fg(color);
                });
        }).with_cell_selection(CellFilter::FgColor(Light3.into()));

        let effects = vec![
            ("sweep in",
                fx::sweep_in(Direction::LeftToRight, 30, screen_bg, (slow, QuadOut))),
            ("sweep out/sweep in", sequence(vec![
                fx::sweep_out(Direction::DownToUp, 5, bg, (2000, QuadOut)),
                fx::sweep_in(Direction::UpToDown, 5, bg, (2000, QuadOut)),
                fx::sweep_out(Direction::UpToDown, 5, bg, (2000, QuadOut)),
                fx::sweep_in(Direction::DownToUp, 5, bg, (2000, QuadOut)),
            ])),
            ("coalesce",
                fx::coalesce(100, (medium, CubicOut))),
            ("glitchy coalesce", parallel(vec![
                fx::coalesce(100, (medium, CubicOut)),
                timed_never_complete(medium, glitch)
            ])),
            ("change hue, saturation and lightness" ,sequence(vec![
                fx::hsl_shift_fg([360.0, 0.0, 0.0], medium),
                fx::hsl_shift_fg([0.0, -100.0, 0.0], medium),
                fx::hsl_shift_fg([0.0, -100.0, 0.0], medium).reversed(),
                fx::hsl_shift_fg([0.0, 100.0, 0.0], medium),
                fx::hsl_shift_fg([0.0, 100.0, 0.0], medium).reversed(),
                fx::hsl_shift_fg([0.0, 0.0, -100.0], medium),
                fx::hsl_shift_fg([0.0, 0.0, -100.0], medium).reversed(),
                fx::hsl_shift_fg([0.0, 0.0, 100.0], medium),
                fx::hsl_shift_fg([0.0, 0.0, 100.0], medium).reversed(),
            ])),
            ("repeating fade in, dissolving fade out", fx::repeating(
                sequence(vec![
                    // fade in content area
                    with_duration(slow + short, parallel(vec![
                        never_complete(fx::dissolve(1, 0)),
                        never_complete(fx::fade_from(screen_bg, screen_bg, (slow, QuadOut))),
                    ])),
                    // fade in; all content visible
                    fx::fade_from_fg(bg, (slow, QuadOut)),
                    // do nothing for a while
                    fx::sleep(slow * 2),
                    // simultaneously dissolve and fade out content
                    parallel(vec![
                        fx::dissolve(100, (slow, CubicOut)),
                        fx::fade_to_fg(bg, (slow, QuadOut)),
                    ]),
                    // content hidden for some time
                    timed_never_complete(short, fx::dissolve(1, 0)),
                    // fade out content area
                    with_duration(slow + short, parallel(vec![
                        never_complete(fx::dissolve(1, 0)),
                        never_complete(fx::fade_to(screen_bg, screen_bg, (slow, QuadOut))),
                    ])),
                ])
            )),
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

fn setup_terminal() -> Result<Terminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let panic_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic| {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stderr(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );

        panic_hook(panic);
    }));

    Ok(terminal)
}