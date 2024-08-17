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
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Clear, StatefulWidget, Widget};

use Interpolation::*;
use tachyonfx::{BufferRenderer, CenteredShrink, Effect, EffectRenderer, fx, Interpolation, Shader, EffectTimer};
use tachyonfx::CellFilter::{AllOf, Inner, Not, Outer, Text};
use tachyonfx::fx::{effect_fn, never_complete, parallel, sequence, term256_colors, with_duration, Direction};
use tachyonfx::widget::EffectTimeline;
use crate::gruvbox::Gruvbox::{Dark0Hard, Dark0Soft};

#[path = "common/gruvbox.rs"]
mod gruvbox;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;


struct App {
    last_tick: Duration,
    use_aux_buffer: bool,
    aux_buffer: Rc<RefCell<Buffer>>,
    inspected_effect: Effect,
    screen_area: Rect,
    timeline: EffectTimeline,
}

#[derive(Default)]
struct Effects {
    aux_buf_fx: Option<Effect>,
    post_process: Vec<Effect>,
}

impl Effects {
    fn push(&mut self, effect: Effect) {
        self.post_process.push(effect);
    }

    fn process_active_fx(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) {
        self.post_process.iter_mut()
            .for_each(|effect| { effect.process(duration, buf, area); });

        self.post_process.retain(|effect| !effect.done());
    }

    fn process_buf_fx(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) {
        if let Some(effect) = self.aux_buf_fx.as_mut() {
            effect.process(duration, buf, area);
            if effect.done() {
                self.aux_buf_fx = None;
            }
        }
    }
}

impl App {
    fn new(
        aux_buffer_area: Rect,
    ) -> Self {
        let fx = example_complex_fx();
        Self {
            last_tick: Duration::ZERO,
            use_aux_buffer: true,
            aux_buffer: Rc::new(RefCell::new(Buffer::empty(aux_buffer_area))),
            timeline: EffectTimeline::from(&fx),
            screen_area: Rect::default(),
            inspected_effect: fx,
        }
    }

    fn refresh_aux_buffer(&self) {
        let effect = self.inspected_effect.clone();

        let mut buf = self.aux_buffer.borrow_mut();
        Block::new()
            .style(Style::default().bg(Color::Black))
            .render(buf.area, &mut buf);

        EffectTimeline::from(&effect)
            .render(buf.area, &mut buf);
    }
}

mod effects {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;
    use crossterm::style::Color;
    use ratatui::buffer::Buffer;
    use ratatui::layout::{Offset, Rect};
    use tachyonfx::{fx, CellFilter, Effect, Interpolation};
    use tachyonfx::fx::*;
    use tachyonfx::Interpolation::{BounceIn, BounceInOut, BounceOut, CircIn, CircOut, ExpoIn, ExpoInOut, ExpoOut, QuadIn, QuadInOut, QuadOut, QuartIn, QuartOut, SineIn};

    pub(super) fn out_fx_1(area: Rect) -> Effect {
        let step = Duration::from_millis(100);
        let bg = Color::Black;

        with_duration(step * 15, parallel(vec![
            never_complete(dissolve(123, (step * 5, ExpoInOut))),
            never_complete(fade_to_fg(bg, (5 * step, BounceOut))),
        ]).with_area(area))
    }

    pub(super) fn tree_fx_1(area: Rect) -> Effect {
        let step = Duration::from_millis(100);
        let bg = Color::Black;

        parallel(vec![
            coalesce(100, (step * 5, ExpoInOut))
                .with_cell_selection(CellFilter::Text),
            sweep_in(Direction::UpToDown, 1, bg, step * 3),
        ]).with_area(area)
    }

    pub(super) fn chart_fx_1(area: Rect) -> Effect {
        let step = Duration::from_millis(100);
        let bg = Color::Black;

        sequence(vec![
            timed_never_complete(step * 3, fade_to_fg(bg, 0)),
            sweep_in(Direction::LeftToRight, 20, bg, step * 5),
        ]).with_area(area)
    }

    pub(super) fn chart_fx_2(area: Rect) -> Effect {
        let step = Duration::from_millis(100);
        let bg = Color::Black;

        sequence(vec![
            timed_never_complete(step * 3, fade_to_fg(bg, 0)),
            fade_from_fg(bg, (step * 7, ExpoOut)),
        ]).with_area(area)
    }

    pub(super) fn chart_fx_3(area: Rect) -> Effect {
        let step = Duration::from_millis(100);
        let bg = Color::Black;

        let hsl_shift = [0.0, -100.0, -50.0];

        parallel(vec![
            // initially desaturated
            sequence(vec![
                timed_never_complete(9 * step, hsl_shift_fg(hsl_shift, 0)),
                hsl_shift_fg(hsl_shift, (12 * step, SineIn)).reversed()
            ]),
            sequence(vec![
                timed_never_complete(step * 3, fade_to_fg(bg, 0)),
                sweep_in(Direction::LeftToRight, 20, bg, step * 5),
            ]),
        ]).with_area(area)
    }

    pub(super) fn move_in_fx(direction: Direction, buf: Rc<RefCell<Buffer>>) -> Effect {
        let screen = buf.borrow().area;
        let offset: Offset = match direction {
            Direction::LeftToRight => Offset { x: -(screen.width as i32), y: 0 },
            Direction::RightToLeft => Offset { x: screen.width as i32, y: 0 },
            Direction::UpToDown    => Offset { x: 0, y: -(screen.height as i32) },
            Direction::DownToUp    => Offset { x: 0, y: screen.height as i32 },
        };

        translate_buf(offset, buf.clone(), (250, CircIn)).reversed()
    }
}

fn main() -> Result<()> {
    let mut terminal = setup_terminal()?;

    // create app and run it
    let app = App::new(Rect::new(0, 0, 100, 40));
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

pub type AuxBuffer = Rc<RefCell<Buffer>>;

fn run_app(
    terminal: &mut Terminal,
    mut app: App,
) -> io::Result<()> {
    let mut last_frame_instant = std::time::Instant::now();

    let mut effects = Effects::default();
    // effects.add(effects::tree_fx_1(&app.inspected_effect, terminal.size()));
    app.refresh_aux_buffer();

    loop {
        app.last_tick = last_frame_instant.elapsed();
        last_frame_instant = std::time::Instant::now();
        terminal.draw(|f| {
            app.screen_area = f.area();
            ui(f, &app, &mut effects)
        })?;

        if event::poll(Duration::from_millis(32))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {

                    let active_area = || {
                        if app.use_aux_buffer {
                            app.aux_buffer.borrow().area
                        } else {
                            app.screen_area
                        }
                    };

                    match key.code {
                        KeyCode::Esc       => return Ok(()),
                        KeyCode::Char(' ') => app.refresh_aux_buffer(),
                        KeyCode::Char('1') => {
                            let inspected = &app.inspected_effect;
                            let rects = EffectTimeline::from(inspected).layout(active_area());
                            effects.push(effects::tree_fx_1(rects.tree))
                        },
                        KeyCode::Char('2') => {
                            let inspected = &app.inspected_effect;
                            let rects = EffectTimeline::from(inspected).layout(active_area());
                            effects.push(effects::chart_fx_1(rects.chart))
                        },
                        KeyCode::Char('3') => {
                            let inspected = &app.inspected_effect;
                            let rects = EffectTimeline::from(inspected).layout(active_area());
                            effects.push(effects::chart_fx_2(rects.chart))
                        },
                        KeyCode::Char('4') => {
                            effects.aux_buf_fx = Some(effects::move_in_fx(Direction::LeftToRight, app.aux_buffer.clone()))
                        },
                        KeyCode::Char('5') => {
                            effects.aux_buf_fx = Some(effects::move_in_fx(Direction::UpToDown, app.aux_buffer.clone()))
                        },
                        KeyCode::Char('6') => {
                            effects.push(effects::out_fx_1(active_area()))
                        },
                        KeyCode::Char('7') => {
                            let inspected = &app.inspected_effect;
                            let rects = EffectTimeline::from(inspected).layout(active_area());
                            effects.push(effects::chart_fx_3(rects.chart))
                        },
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
    app: &App,
    effects: &mut Effects
) {
    let rect = f.area();
    if rect.area() == 0 { return; }

    let buf: &mut Buffer = f.buffer_mut();
    Clear.render(rect, buf);

    if app.use_aux_buffer {
        if effects.aux_buf_fx.is_some() {
            effects.process_buf_fx(app.last_tick, buf, rect);
        } else {
            app.aux_buffer.render_buffer(Offset::default(), buf);
        }
    } else {
        Block::new()
            .style(Style::default().bg(Color::from_u32(0x201416)))
            .render(rect, buf);

        app.timeline.clone()
            .render(rect, buf);
    }

    effects.process_active_fx(app.last_tick, buf, rect);
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