use std::{io, panic};
use std::cell::RefCell;
use std::error::Error;
use std::io::Stdout;
use std::rc::Rc;
use std::time::Duration;

use crossterm::{event, execute};
use crossterm::event::{DisableMouseCapture, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use rand::prelude::SeedableRng;
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::{Frame, widgets};
use ratatui::layout::{Margin, Offset, Position, Rect};
use ratatui::style::Color;
use ratatui::widgets::{Clear, StatefulWidget, Widget};

use Interpolation::*;
use tachyonfx::{BufferRenderer, CenteredShrink, Effect, EffectRenderer, fx, Interpolation, Shader, EffectTimer};
use tachyonfx::CellFilter::{AllOf, Inner, Not, Outer, Text};
use tachyonfx::fx::{effect_fn, never_complete, parallel, sequence, term256_colors, with_duration};
use tachyonfx::widget::EffectTimeline;
use crate::gruvbox::Gruvbox::{Dark0Hard, Dark0Soft};

#[path = "common/gruvbox.rs"]
mod gruvbox;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;


struct App {
    last_tick: Duration,
    use_aux_buffer: bool, /** demonstrates reusing aux_buffer contents for render */
    aux_buffer: Rc<RefCell<Buffer>>,
    inspected_effect: Effect,
    timeline: EffectTimeline,
}

#[derive(Default)]
struct StatefulWidgets {
    effects: Vec<Effect>
}

impl App {
    fn new(
        aux_buffer_area: Rect,
    ) -> Self {
        let fx = example_complex_fx();
        Self {
            last_tick: Duration::ZERO,
            use_aux_buffer: false,
            aux_buffer: Rc::new(RefCell::new(Buffer::empty(aux_buffer_area))),
            timeline: EffectTimeline::from(&fx),
            inspected_effect: fx,
        }
    }

    fn refresh_aux_buffer(&self) {
        let effect = self.inspected_effect.clone();

        let mut buf = self.aux_buffer.borrow_mut();
        EffectTimeline::from(&effect)
            .render(buf.area, &mut buf);
    }
}

fn main() -> Result<()> {
    let mut terminal = setup_terminal()?;

    // create app and run it
    let app = App::new(Rect::new(0, 0, 80, 40));
    let res = run_app(&mut terminal, app);

    // widgets
    let mut stateful_widgets = StatefulWidgets::default();

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

pub type OffscreenBuffer = Rc<RefCell<Buffer>>;

fn run_app(
    terminal: &mut Terminal,
    mut app: App,
) -> io::Result<()> {
    let mut last_frame_instant = std::time::Instant::now();

    app.refresh_aux_buffer();

    loop {
        app.last_tick = last_frame_instant.elapsed();
        last_frame_instant = std::time::Instant::now();
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(Duration::from_millis(32))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc       => return Ok(()),
                        KeyCode::Char(' ') => app.refresh_aux_buffer(),
                        KeyCode::Tab       => app.use_aux_buffer = !app.use_aux_buffer,
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
    if f.size().area() == 0 { return; }

    Clear.render(f.size(), f.buffer_mut());

    if app.use_aux_buffer {
        app.aux_buffer.render_buffer(Offset::default(), f.buffer_mut());
    } else {
        app.timeline.clone()
            .render(f.size(), f.buffer_mut());
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


fn example_complex_fx() -> Effect {
    let margin = Margin::new(1, 1);
    let border_text        = AllOf(vec![Outer(margin), Text]);
    let border_decorations = AllOf(vec![Outer(margin), Not(Text.into())]);

    let short = Duration::from_millis(220);
    let duration = Duration::from_millis(320);
    let time_scale = 2;

    let bg = Color::DarkGray;
    let gray = Color::Gray;

    fx::repeating(fx::parallel(vec![
        // window borders
        parallel(vec![
            sequence(vec![
                with_duration(short * time_scale, never_complete(fx::dissolve(1, 0))),
                fx::coalesce(111, (duration, BounceOut)),
            ]),
            fx::fade_from(gray, gray, duration * time_scale)
        ]).with_cell_selection(border_decorations),

        // window title and shortcuts
        sequence(vec![
            with_duration(duration * time_scale, never_complete(fx::fade_to(gray, gray, 0))),
            fx::fade_from(gray, gray, (320 * time_scale, QuadOut)),
        ]).with_cell_selection(border_text),

        // content area
        sequence(vec![
            with_duration(Duration::from_millis(270) * time_scale, parallel(vec![
                never_complete(fx::dissolve(1, 0)), // hiding icons/emoji
                never_complete(fx::fade_to(bg, bg, 0)),
            ])),
            parallel(vec![
                fx::coalesce(111, Duration::from_millis(220) * time_scale),
                fx::fade_from(bg, bg, (250 * time_scale, QuadOut))
            ]),
            fx::sleep(3000),
            parallel(vec![
                fx::fade_to(bg, bg, (250 * time_scale, BounceIn)),
                fx::dissolve(111, (Duration::from_millis(220) * time_scale, ElasticOut)),
            ]),
        ]).with_cell_selection(Inner(margin)),
    ]))
}