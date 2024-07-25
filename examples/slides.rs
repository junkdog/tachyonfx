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
use ratatui::layout::{Offset, Position, Rect};
use ratatui::style::Color;
use ratatui::text::Text;
use ratatui::widgets::{Clear, StatefulWidget, Widget};

use Interpolation::*;
use tachyonfx::{BufferRenderer, CenteredShrink, Effect, EffectRenderer, fx, Interpolation, Shader};
use tachyonfx::fx::effect_fn;
use crate::gruvbox::Gruvbox::{Dark0Hard, Dark0Soft};

#[path = "common/gruvbox.rs"]
mod gruvbox;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;


struct App {
    last_tick: Duration,
}

#[derive(Default)]
struct StatefulWidgets {
    slide_state: SlideState,
    effects: Vec<Effect>
}

impl App {
    fn new() -> Self {
        Self {
            last_tick: Duration::ZERO
        }
    }
}

fn main() -> Result<()> {
    let mut terminal = setup_terminal()?;

    // create app and run it
    let app = App::new();
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
    loop {
        app.last_tick = last_frame_instant.elapsed();
        last_frame_instant = std::time::Instant::now();
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(Duration::from_millis(32))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc       => return Ok(()),
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

    let offscreen_area = Rect::new(0, 0, 10, 1);
    let offscreen_buf = Rc::new(RefCell::new(Buffer::empty(offscreen_area)));

    let render_widget = fx::effect_fn_buf((), 0, |_state, context, buf| {
        Text::from("Hello, world!").render(context.area, buf);
    });
    let mut render_offscren_fx = fx::offscreen_buffer(render_widget, offscreen_buf.clone());
    f.render_effect(&mut render_offscren_fx, f.size(), Duration::from_millis(100));


    offscreen_buf.render_buffer(Offset { x: -1, y: 5 }, f.buffer_mut());
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

struct Slide;

#[derive(Default)]
struct SlideState {
    last_frame_ms: Duration,
    // buffer: Buffer
}

impl StatefulWidget for Slide {
    type State = SlideState;

    fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut Self::State
    ) {
        Text::from("Hello, world!")
            .render(area, buf);
    }
}