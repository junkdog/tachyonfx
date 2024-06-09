use std::{io, panic, vec};
use std::error::Error;
use std::io::Stdout;
use std::time::{Duration, Instant};

use crossterm::{event, execute};
use crossterm::event::{DisableMouseCapture, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::layout::Constraint::Ratio;
use ratatui::prelude::Marker;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Axis, Block, Chart, Clear, Dataset, GraphType, LegendPosition, StatefulWidget, Widget};

use Gruvbox::OrangeBright;
use Interpolation::*;
use tachyonfx::{CellFilter, CenteredShrink, Effect, EffectRenderer, EffectTimer, fx, Interpolation, Shader};
use tachyonfx::fx::{parallel, repeating, sequence};

use crate::gruvbox::Gruvbox;
use crate::gruvbox::Gruvbox::{Dark0, Dark1, Light2};

#[path = "common/gruvbox.rs"]
mod gruvbox;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;

struct App {
    last_tick: Duration,
    tween_idx: usize,
    widget_states: Vec<InterpolationWidgetState>,
    shortcut_fx: Effect,
}

impl App {
    fn new() -> Self {
        let tween_idx = 0;
        let shortcut_fx = repeating(sequence(vec![
            fx::sweep_in(20, Dark0, EffectTimer::from_ms(1000, QuadIn)),
            fx::sleep(5_000),
            parallel(vec![
                fx::fade_to_fg(Dark0, EffectTimer::from_ms(500, BounceOut)),
                fx::dissolve(30, EffectTimer::from_ms(500, BounceOut)),
            ]),
        ]));

        let mut app = Self {
            last_tick: Duration::default(),
            tween_idx,
            widget_states: Vec::new(),
            shortcut_fx,
        };
        app.update_widget_states(1);
        app
    }

    fn update_widget_states(&mut self, widgets: usize) {
        let to_widget_state = |i| InterpolationWidgetState::new(self.tween_idx + i);

        self.widget_states = (0..widgets)
            .map(to_widget_state)
            .collect();
    }
}

fn main() -> Result<()> {
    let mut terminal = setup_terminal()?;

    // create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);

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
) -> io::Result<()> {
    let mut last_frame_instant = std::time::Instant::now();
    loop {
        app.last_tick = last_frame_instant.elapsed();
        terminal.draw(|f| ui(f, &mut app))?;
        last_frame_instant = Instant::now();

        while last_frame_instant.elapsed() < Duration::from_millis(32) {
            if event::poll(Duration::from_millis(5))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc => return Ok(()),
                            KeyCode::Up => {
                                app.tween_idx = app.tween_idx.checked_sub(1).unwrap_or(LAST_TWEEN_IDX);
                                app.widget_states.iter_mut().enumerate().for_each(|(i, state)| {
                                    state.update_interpolation(app.tween_idx + i);
                                });
                            },
                            KeyCode::Down => {
                                app.tween_idx = (app.tween_idx + 1) % TWEENS;
                                app.widget_states.iter_mut().enumerate().for_each(|(i, state)| {
                                    state.update_interpolation(app.tween_idx + i);
                                });
                            },
                            KeyCode::Char(n) if n.is_numeric() => {
                                let widgets = n.to_digit(10).unwrap_or(1) as usize;
                                app.update_widget_states(widgets);
                            },
                            _ => {}
                        }
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
    if f.size().height == 0 { return; }

    Clear.render(f.size(), f.buffer_mut());
    Block::default()
        .style(Style::default().fg(Light2.into()).bg(Dark0.into()))
        .render(f.size(), f.buffer_mut());

    let layout = Layout::vertical(
        vec![
            Constraint::Min(2),
            Constraint::Percentage(100),
        ]
    ).split(f.size());

    render_shortcuts(app, f, layout[0]);


    let widgets = app.widget_states.len();
    let constraint = Ratio(1, widgets as u32);
    let constraints = vec![constraint; widgets];
    Layout::horizontal(constraints)
        .margin(1)
        .split(layout[1])
        .iter()
        .zip(app.widget_states.iter_mut())
        .for_each(|(area, state)| {
            InterpolationWidget::new(app.last_tick)
                .render(*area, f.buffer_mut(), state);
        });
}

fn render_shortcuts(app: &mut App, f: &mut Frame, area: Rect) {
    let shortcut_key_style = Style::default()
        .fg(OrangeBright.into())
        .add_modifier(Modifier::BOLD);
    let shortcut_label_style = Style::default()
        .fg(Light2.into());

    let line = Line::from(vec![
        Span::from(" ↑/↓ ").style(shortcut_key_style),
        Span::from("interpolation ").style(shortcut_label_style),
        Span::from(" 1..9 ").style(shortcut_key_style),
        Span::from("widgets ").style(shortcut_label_style),
        Span::from(" ESC ").style(shortcut_key_style),
        Span::from("quit ").style(shortcut_label_style),
    ]);

    let content_area = area.inner_centered(line.width() as u16, 1);
    line.render(content_area, f.buffer_mut());
    f.render_effect(&mut app.shortcut_fx, content_area, app.last_tick)
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
        disable_raw_mode().expect("failed to disable raw mode");
        execute!(
            io::stderr(),
            LeaveAlternateScreen,
            DisableMouseCapture
        ).expect("failed to reset the terminal");

        panic_hook(panic);
    }));

    Ok(terminal)
}

fn restore_terminal(mut terminal: Terminal) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

struct InterpolationWidget {
    last_frame: Duration,
}

impl InterpolationWidget {
    fn new(last_frame: Duration) -> Self {
        Self { last_frame }
    }

    fn render_transitions(
        self,
        area: Rect,
        buffer: &mut Buffer,
        state: &mut InterpolationWidgetState,
    ) {
        let layout = Layout::horizontal(
            vec![
                Constraint::Percentage(50),
                Constraint::Min(1),
                Constraint::Percentage(50),
            ]
        ).split(area.inner(&Margin::new(1, 0)));

        Text::from("coalesce/dissolve")
            .style(Style::default().fg(OrangeBright.into()))
            .render(layout[0], buffer);
        Text::from("fade in/fade out")
            .style(Style::default().fg(OrangeBright.into()))
            .render(layout[2], buffer);

        buffer.render_effect(&mut state.coalesce_fx, layout[0], self.last_frame);
        buffer.render_effect(&mut state.fade_fx, layout[2], self.last_frame);
    }
}

#[derive(Clone)]
struct InterpolationWidgetState {
    chart_fx: Effect,
    coalesce_fx: Effect,
    fade_fx: Effect,
    tween_idx: usize,
    dataset: Vec<(f64, f64)>
}

fn chart_fx() -> Effect {
    let duration = Duration::from_millis(300);
    let timer = EffectTimer::new(duration, QuadInOut);

    parallel(vec![
        // chart axis
        fx::sweep_in(15, Dark0, timer)
            .with_cell_selection(CellFilter::FgColor(Light2.into())),
        // chart data
        sequence(vec![
            fx::timed_never_complete(duration, fx::dissolve(1, 0))
                .with_cell_selection(CellFilter::FgColor(OrangeBright.into())),
            fx::sweep_in(15, Dark0, timer)
                .with_cell_selection(CellFilter::FgColor(OrangeBright.into())),
        ]),
    ])
}

impl InterpolationWidgetState {
    fn new(tween_idx: usize) -> Self {
        let mut s = Self {
            tween_idx,
            dataset: Vec::new(),
            chart_fx: chart_fx(),
            coalesce_fx: fx::sleep(0), // to-be-replaced
            fade_fx: fx::sleep(0),     // to-be-replaced
        };
        s.update_interpolation(tween_idx);
        s
    }


    fn update_interpolation(&mut self, tween_idx: usize) {
        self.tween_idx = tween_idx;
        let tween = idx_to_tween(tween_idx);

        let timer = EffectTimer::from_ms(1000, tween);

        self.chart_fx = chart_fx();
        self.coalesce_fx = repeating(sequence(vec![
           fx::coalesce(20, timer),
           fx::dissolve(20, timer),
        ]));
        self.fade_fx = repeating(sequence(vec![
            fx::fade_from(Dark0, Dark0, timer),
            fx::fade_to_fg(Dark0, timer),
        ]));

        self.dataset = (0..=100)
            .step_by(2)
            .map(|x| {
                let a = x as f64 / 100.0;
                (a, tween.alpha(a as f32) as f64)
            })
            .collect();
    }

    fn dataset(&self) -> Vec<Dataset> {
        let name = format!("{:?}", idx_to_tween(self.tween_idx));

        let data_0 = Dataset::default()
            .marker(Marker::Dot)
            .data(&[(0.0, 0.0), (1.0, 0.0)])
            .style(Style::default().fg(Dark1.into()))
            .graph_type(GraphType::Line);
        let data_1 = Dataset::default()
            .marker(Marker::Dot)
            .data(&[(0.0, 1.0), (1.0, 1.0)])
            .style(Style::default().fg(Dark1.into()))
            .graph_type(GraphType::Line);
        let data = Dataset::default()
            .name(name)
            .data(&self.dataset)
            .marker(Marker::Braille)
            .style(Style::default().fg(OrangeBright.into()))
            .graph_type(GraphType::Line);

        vec![data_0, data_1, data]
    }
}

impl StatefulWidget for InterpolationWidget {
    type State = InterpolationWidgetState;

    fn render(
        self,
        area: Rect,
        buffer: &mut Buffer,
        state: &mut Self::State,
    ) {
        Clear.render(area, buffer);
        Block::new().style(Style::default().bg(Dark0.into()))
            .render(area, buffer);

        let layout = Layout::vertical(
            vec![
                Constraint::Percentage(100), // chart
                Constraint::Min(1),          // separator
                Constraint::Min(1),          // fx
            ]
        ).split(area);

        let axis_x = Axis::default()
            .title("x")
            .bounds([0.0, 1.0])
            .labels(vec!["0.0".into(), "0.5".into(), "1.0".into()]);
        let axis_y = Axis::default()
            .title("y")
            .bounds([-0.2, 1.2])
            .labels(vec!["-0.2".into(), "0.5".into(), "1.2".into()]);

        let chart = Chart::new(state.dataset())
            .x_axis(axis_x)
            .y_axis(axis_y)
            .style(Style::default().fg(Light2.into()).bg(Dark0.into()))
            .legend_position(Some(LegendPosition::BottomRight))
            .hidden_legend_constraints((Constraint::Min(0), Ratio(1, 4)));

        chart.render(layout[0], buffer);

        if state.chart_fx.running() {
            state.chart_fx.process(self.last_frame, buffer, layout[0]);
        }

        self.render_transitions(layout[2], buffer, state);
    }
}

fn idx_to_tween(idx: usize) -> Interpolation {
    let idx = idx % TWEENS;
    match idx {
        0  => Linear,
        1  => Reverse,
        2  => QuadIn,
        3  => QuadOut,
        4  => QuartOut,
        5  => CubicIn,
        6  => CubicOut,
        7  => CubicInOut,
        8  => QuartIn,
        9  => QuartOut,
        10 => QuartInOut,
        11 => QuintIn,
        12 => QuintOut,
        13 => QuintInOut,
        14 => SineIn,
        15 => SineOut,
        16 => SineInOut,
        17 => ExpoIn,
        18 => ExpoOut,
        19 => ExpoInOut,
        20 => CircIn,
        21 => CircOut,
        22 => CircInOut,
        23 => ElasticIn,
        24 => ElasticOut,
        25 => ElasticInOut,
        26 => BackIn,
        27 => BackOut,
        28 => BackInOut,
        29 => BounceIn,
        30 => BounceOut,
        31 => BounceInOut,
        _ => panic!("should never happen"),
    }
}

const TWEENS: usize = 32;
const LAST_TWEEN_IDX: usize = TWEENS -1;