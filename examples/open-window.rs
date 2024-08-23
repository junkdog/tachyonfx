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
use ratatui::layout::{Constraint, Layout, Margin, Rect, Size};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Borders, BorderType, Clear, StatefulWidget, Widget};

use CellFilter::Text;
use Interpolation::*;
use tachyonfx::{CellFilter, CenteredShrink, Effect, EffectRenderer, fx, Interpolation, Shader};
use tachyonfx::CellFilter::{AllOf, Inner, Not, Outer};
use tachyonfx::fx::{Direction, never_complete, parallel, repeating, sequence, sleep, timed_never_complete, with_duration};

use crate::gruvbox::Gruvbox::{BlueBright, Dark0, Dark0Hard, Light2, YellowBright};
use crate::window::OpenWindow;

#[path = "common/gruvbox.rs"]
mod gruvbox;

#[path = "common/window.rs"]
mod window;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;

struct App {
    last_tick: Duration,
    pub win_left: HelloWorldPopupState,
    pub win_right: HelloWorldPopupState,
}

impl App {
    fn new() -> Self {
        Self {
            last_tick: Duration::ZERO,
            win_left: HelloWorldPopupState::new(
                fx::ping_pong( // pre-render fx
                    fx::translate(Some(fx::sleep(1300)), (0, 20), (1200, QuartInOut))
                ),
                glitchy_window_fx(Dark0Hard),
                Style::default()
                    .fg(YellowBright.into())
                    .bg(Dark0.into()),
                BorderType::Rounded,
                Borders::ALL,
                Style::default()
                    .fg(Light2.into())
                    .bg(Dark0.into()),
            ),
            win_right: HelloWorldPopupState::new(
                fx::repeating(
                    fx::resize_area(Some(fx::sleep(4000)), Size::new(35, 1), (500, SineInOut)),
                ),
                stylized_window_fx(),
                Style::default()
                    .fg(Dark0.into())
                    .bg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
                BorderType::QuadrantOutside,
                Borders::TOP,
                Style::default()
                    .fg(Dark0.into())
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
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
    if f.area().height == 0 { return; }

    Clear.render(f.area(), f.buffer_mut());

    let area = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(f.area());

    let mut popup_area_l = area[0].inner_centered(45, 7);
    popup_area_l.y = area[0].height / 2;
    HelloWorldPopup::new(app.last_tick)
        .render(popup_area_l, f.buffer_mut(), &mut app.win_left);

    let mut popup_area_r = area[1].inner_centered(45, 7);
    popup_area_r.y = area[1].height / 2;
    HelloWorldPopup::new(app.last_tick)
        .render(popup_area_r, f.buffer_mut(), &mut app.win_right);
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
    fn new(
        pre_render_fx: Effect,
        content_fx: Effect,
        border_style: Style,
        border_type: BorderType,
        borders: Borders,
        content_style: Style,
    ) -> Self {
        let styles = [
            Style::default()
                .fg(BlueBright.into())
                .add_modifier(Modifier::BOLD),
            Style::default()
                .fg(Light2.into())
        ];

        let text = text::Text::from(vec![
            Line::from("Hello, World!").style(styles[0]),
            Line::from("").style(content_style),
            Line::from("Lorem ipsum dolor sit amet, consectetur").style(content_style),
            Line::from("adipiscing elit. Sed imperdiet, turpis at").style(content_style),
            Line::from("tempor interdum, ligula est sagittis mauris,").style(content_style),
            Line::from("vitae semper eros nisl eget nisi. ").style(content_style),
        ]);

        let title_style = border_style.clone().add_modifier(Modifier::BOLD);

        
        
        let title = Line::from(vec![
            // Span::from("┫").style(border_style),
            Span::from(" ").style(title_style),
            Span::from("Hello, world!").style(title_style),
            Span::from(" ").style(title_style),
            // Span::from("┣").style(border_style),
        ]);


        let window_fx = OpenWindow::builder()
            .title(title)
            .border_style(border_style)
            .border_type(border_type)
            .borders(borders)
            .title_style(title_style)
            .pre_render_fx(pre_render_fx)
            .content_fx(content_fx)
            .background(content_style)
            .build();

        Self { text, window_fx }
    }
}

fn glitchy_window_fx<C: Into<Color>>(bg: C) -> Effect {
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
    repeating(parallel(&[
        // window borders
        parallel(&[
            sequence(&[
                with_duration(short * time_scale, never_complete(fx::dissolve(0))),
                fx::coalesce((duration, BounceOut)),
            ]),
            fx::fade_from(Dark0, Dark0, duration * time_scale)
        ]).with_cell_selection(border_decorations),

        // window title and shortcuts
        sequence(&[
            with_duration(duration * time_scale, never_complete(fx::fade_to(Dark0, Dark0, 0))),
            fx::fade_from(Dark0, Dark0, (320 * time_scale, QuadOut)),
        ]).with_cell_selection(border_text),

        // content area
        sequence(&[
            with_duration(Duration::from_millis(270) * time_scale, parallel(&[
                never_complete(fx::dissolve(0)), // hiding icons/emoji
                never_complete(fx::fade_to(bg, bg, 0)),
            ])),
            parallel(&[
                fx::coalesce(Duration::from_millis(220) * time_scale),
                fx::fade_from(bg, bg, (250 * time_scale, QuadOut))
            ]),
            sleep(3000),
            parallel(&[
                fx::fade_to(bg, bg, (250 * time_scale, BounceIn)),
                fx::dissolve((Duration::from_millis(220) * time_scale, ElasticOut)),
            ]),
        ]).with_cell_selection(Inner(margin)),
    ]))
}

fn stylized_window_fx() -> Effect {
    let layout = Layout::vertical([Constraint::Length(1), Constraint::Percentage(100)]);
    let content_area = CellFilter::Layout(layout, 1);

    let cyan = Color::from_hsl(180.0, 100.0, 50.0);
    repeating(parallel(&[
        // content area
        sequence(&[
            parallel(&[
                sequence(&[
                    with_duration(Duration::from_millis(1000), parallel(&[
                        never_complete(fx::fade_to(cyan, cyan, 0)),
                    ])),
                    timed_never_complete(Duration::from_millis(2500),
                        fx::fade_from(cyan, cyan, (400, QuadOut))
                    ),
                    fx::fade_to(Color::Black, Color::Black, (500, CircInOut)),
                ]),
                fx::slide_in(Direction::UpToDown, 10, Dark0, (900, QuadOut)),
            ]),
        ]).with_cell_selection(content_area),
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
        if state.window_fx.area().is_none() {
            return;
        }

        let content_area = state.window_fx.area()
            .unwrap_or(area)
            .inner(Margin::new(1, 1));

        state.text.clone().render(content_area, buf);
        state.window_fx.processing_content_fx(self.last_tick, buf, content_area);
    }
}

fn setup_terminal() -> Result<Terminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
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
