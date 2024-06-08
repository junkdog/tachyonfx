use std::{io, panic, vec};
use std::error::Error;
use std::io::Stdout;
use std::time::Duration;

use crossterm::{event, execute};
use crossterm::event::{DisableMouseCapture, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use rand::prelude::SeedableRng;
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::prelude::Marker;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Axis, BorderType, Chart, Clear, Dataset, GraphType, StatefulWidget, Widget};

use Interpolation::*;
use tachyonfx::{CenteredShrink, Effect, EffectRenderer, FilterMode, fx, Interpolation, IntoEffect, Shader};
use tachyonfx::FilterMode::{AllOf, Inner, Negate, Outer};
use tachyonfx::fx::{never_complete, parallel, sequence, temporary};
use crate::gruvbox::Gruvbox;

use crate::gruvbox::Gruvbox::{BlueBright, Dark0, Dark0Hard, Dark1, Light2, YellowBright};
use crate::window::OpenWindow;

#[path = "common/gruvbox.rs"]
mod gruvbox;

#[path = "common/window.rs"]
mod window;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;

struct App {
    last_tick: Duration,
    tween_idx: usize,
    widget_state: InterpolationWidgetState
}

impl App {
    fn new() -> Self {
        let tween_idx = 0;
        let widget_state = InterpolationWidgetState::new(tween_idx);
        Self { last_tick: Duration::default(), tween_idx, widget_state }
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
        last_frame_instant = std::time::Instant::now();

        while last_frame_instant.elapsed() < Duration::from_millis(32) {
            if event::poll(Duration::from_millis(5))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc => return Ok(()),
                            KeyCode::Up => {
                                if app.tween_idx == 0 {
                                    app.tween_idx = 31
                                } else {
                                    app.tween_idx -= 1;
                                }
                                app.widget_state.update_interpolation(app.tween_idx);
                            },
                            KeyCode::Down => {
                                app.tween_idx = (app.tween_idx + 1) % 32;
                                app.widget_state.update_interpolation(app.tween_idx);
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

    let popup_area = f.size().inner_centered(80, 40);
    InterpolationWidget::new(app.last_tick)
        .render(popup_area, f.buffer_mut(), &mut app.widget_state);
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
        // restore_terminal(terminal).expect("failed to reset the terminal");
        disable_raw_mode();
        execute!(
            io::stderr(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );

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
}

#[derive(Default)]
struct InterpolationWidgetState {
    title: Text<'static>,
    tween_idx: usize,
    dataset: Vec<(f64, f64)>
}

impl InterpolationWidgetState {
    fn new(tween_idx: usize) -> Self {
        let title = Text::from("Interpolation")
            .style(Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        );

        let mut s = Self { title, ..Self::default() };
        s.update_interpolation(tween_idx);
        s
    }

    fn update_interpolation(&mut self, tween_idx: usize) {
        self.tween_idx = tween_idx;
        let tween = idx_to_tween(tween_idx);

        self.dataset = (0..=100)
            .step_by(2)
            .map(|x| {
                let a = x as f64 / 100.0;
                (a, tween.alpha(a as f32) as f64)
            })
            .collect();
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
        let name = format!("Interpolation: {:?}", idx_to_tween(state.tween_idx));

        let data_0 = Dataset::default()
            .data(&state.dataset)
            .marker(Marker::Dot)
            .data(&[(0.0, 0.0), (1.0, 0.0)])
            .style(Style::default().fg(Gruvbox::Dark1.into()))
            .graph_type(GraphType::Line);
        let data_1 = Dataset::default()
            .data(&state.dataset)
            .marker(Marker::Dot)
            .data(&[(0.0, 1.0), (1.0, 1.0)])
            .style(Style::default().fg(Gruvbox::Dark1.into()))
            .graph_type(GraphType::Line);
        let data = Dataset::default()
            .name(name)
            .data(&state.dataset)
            .marker(Marker::Braille)
            .style(Style::default().fg(Gruvbox::OrangeBright.into()))
            .graph_type(GraphType::Line);

        let axis_x = Axis::default()
            .title("x")
            .bounds([0.0, 1.0])
            .labels(vec!["0.0".into(), "0.5".into(), "1.0".into()]);
        let axis_y = Axis::default()
            .title("y")
            .bounds([-0.2, 1.2])
            .labels(vec!["-0.2".into(), "0.5".into(), "1.2".into()]);

        Chart::new(vec![data_0, data_1, data])
            .x_axis(axis_x)
            .y_axis(axis_y)
            .render(area, buffer);
    }
}

fn idx_to_tween(idx: usize) -> Interpolation {
    match idx {
        0 => Linear,
        1 => Reverse,
        2 => QuadIn,
        3 => QuadOut,
        4 => QuartOut,
        5 => CubicIn,
        6 => CubicOut,
        7 => CubicInOut,
        8 => QuartIn,
        9 => QuartOut,
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
        _ => panic!("invalid tween index={idx}"),
    }
}