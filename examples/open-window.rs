use std::{io, panic, vec};
use std::error::Error;
use std::io::Stdout;
use std::time::Duration;

use crossterm::{event, execute};
use crossterm::event::{DisableMouseCapture, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{Frame, text};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{BorderType, Clear, StatefulWidget, Widget};

use CellFilter::Text;
use Interpolation::*;
use tachyonfx::{CellFilter, CenteredShrink, Effect, EffectRenderer, fx, Interpolation, Shader};
use tachyonfx::CellFilter::{AllOf, Inner, Not, Outer};
use tachyonfx::fx::{never_complete, parallel, repeating, sequence, sleep, with_duration};

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
    pub popup_state: HelloWorldPopupState,
}

impl App {
    fn new() -> Self {
        Self {
            last_tick: Duration::ZERO,
            popup_state: HelloWorldPopupState::new(),
        }
    }
}

fn main() -> Result<()> {
    let mut terminal = setup_terminal()?;

    // create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // restore terminal
    let _ = disable_raw_mode().expect("failed to disable raw mode");
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    ).expect("failed to leave alternate screen");
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

    let mut popup_area = f.size().inner_centered(58, 7);
    popup_area.y = f.size().height - 8;

    Clear.render(f.size(), f.buffer_mut());
    HelloWorldPopup::new(app.last_tick)
        .render(popup_area, f.buffer_mut(), &mut app.popup_state);
}


struct HelloWorldPopup {
    last_tick: Duration,
}

impl HelloWorldPopup {
    fn new(last_tick: Duration) -> Self {
        Self { last_tick }
    }
}

#[derive(Clone)]
struct HelloWorldPopupState {
    text: text::Text<'static>,
    window_fx: OpenWindow,
}

impl HelloWorldPopupState {
    fn new() -> Self {
        let styles = [
            Style::default()
                .fg(BlueBright.into())
                .add_modifier(Modifier::BOLD),
            Style::default()
                .fg(Light2.into())
        ];

        let text = text::Text::from(vec![
            Line::from("Hello, World!").style(styles[0]),
            Line::from("").style(styles[1]),
            Line::from("Lorem ipsum dolor sit amet, consectetur adipiscing elit.").style(styles[1]),
            Line::from("Sed imperdiet, turpis at tempor interdum, ligula est").style(styles[1]),
            Line::from("sagittis mauris, vitae semper eros nisl eget nisi.").style(styles[1]),
        ]);

        let border_style = Style::default()
            .fg(YellowBright.into());
        let title_style = Style::default()
            .fg(Dark0.into())
            .bg(YellowBright.into())
            .add_modifier(Modifier::BOLD);

        let title = Line::from(vec![
            Span::from("┫").style(border_style),
            Span::from(" ").style(title_style),
            Span::from("Hello, world!").style(title_style),
            Span::from(" ").style(title_style),
            Span::from("┣").style(border_style),
        ]);

        let move_in_window = fx::ping_pong(
            fx::translate(None, (0, -25), (1200, QuartInOut)).reversed()
        );

        let window_fx = OpenWindow::builder()
            .title(title)
            .border_style(border_style)
            .border_type(BorderType::Rounded)
            .title_style(title_style)
            .pre_render_fx(move_in_window)
            .content_fx(open_window_fx(Dark0Hard))
            .background(Style::default().bg(Dark1.into()))
            .build()
            .unwrap();

        Self { text, window_fx }
    }

}

fn open_window_fx<C: Into<Color>>(bg: C) -> Effect {
    let margin = Margin::new(1, 1);
    let border_text        = AllOf(vec![Outer(margin), Text]);
    let border_decorations = AllOf(vec![Outer(margin), Not(Text.into())]);

    let bg = bg.into();

    // window open effect; effects run in parallel for:
    // - window borders
    // - window title and shortcuts
    // - content area
    let short = Duration::from_millis(220);
    let duration = Duration::from_millis(320);
    let time_scale = 2;
    repeating(parallel(vec![
        // window borders
        parallel(vec![
            sequence(vec![
                with_duration(short * time_scale, never_complete(fx::dissolve(1, 0))),
                fx::coalesce(111, (duration, BounceOut)),
            ]),
            fx::fade_from(Dark0, Dark0, duration * time_scale)
                .with_cell_selection(border_decorations),
        ]),

        // window title and shortcuts
        sequence(vec![
            with_duration(duration * time_scale, never_complete(fx::fade_to(Dark0, Dark0, 0))),
            fx::fade_from(Dark0, Dark0, (320 * time_scale, QuadOut)),
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
            sleep(3000),
            parallel(vec![
                fx::fade_to(bg, bg, (250 * time_scale, BounceIn)),
                fx::dissolve(111, (Duration::from_millis(220) * time_scale, ElasticOut)),
            ]),
        ]).with_cell_selection(Inner(margin)),
    ]))
}

impl StatefulWidget for HelloWorldPopup {
    type State = HelloWorldPopupState;

    fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut Self::State
    ) {
        buf.render_effect(&mut state.window_fx, area, self.last_tick);
        let content_area = state.window_fx.area()
            .unwrap_or(area)
            .inner(&Margin::new(1, 1));

        state.text.clone().render(content_area, buf);

        state.window_fx.processing_content_fx(self.last_tick, buf, content_area);
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
