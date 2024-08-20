use std::cell::RefCell;
use std::error::Error;
use std::io::Stdout;
use std::rc::Rc;
use std::time::Duration;
use std::{io, panic};

use crossterm::event::{DisableMouseCapture, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{event, execute};
use rand::prelude::SeedableRng;
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Offset, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Clear, StatefulWidget, Widget};
use ratatui::Frame;

use tachyonfx::fx::{never_complete, parallel, sequence, with_duration, Direction};
use tachyonfx::widget::{EffectTimeline, EffectTimelineRects};
use tachyonfx::CellFilter::{AllOf, Inner, Not, Outer, Text};
use tachyonfx::{fx, BufferRenderer, CenteredShrink, Effect, EffectRenderer, Interpolation, Shader};
use Interpolation::*;
use crate::effects::random_fx_in;

#[path = "common/gruvbox.rs"]
mod gruvbox;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;


struct App {
    last_tick: Duration,
    use_aux_buffer: bool,
    aux_buffer: Rc<RefCell<Buffer>>,
    inspected_effect: Rc<RefCell<Effect>>,
    screen_area: Rect,
    timeline: EffectTimeline,
}

#[derive(Default)]
struct Effects {
    pub aux_buf_fx: Option<Effect>,
    pub post_process: Vec<Effect>,
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
        let fx = random_fx_in(EffectTimelineRects::default());
        let layout = EffectTimeline::from(&fx)
            .layout(aux_buffer_area);
        let fx = random_fx_in(layout);

        Self {
            last_tick: Duration::ZERO,
            use_aux_buffer: true,
            aux_buffer: Rc::new(RefCell::new(Buffer::empty(aux_buffer_area))),
            timeline: EffectTimeline::from(&fx),
            screen_area: Rect::default(),
            inspected_effect: Rc::new(RefCell::new(fx)),
        }
    }

    fn refresh_aux_buffer(&self) {
        let effect = self.inspected_effect.clone();

        let mut buf = self.aux_buffer.borrow_mut();
        Block::new()
            .style(Style::default().bg(Color::Black))
            .render(buf.area, &mut buf);

        EffectTimeline::from(&effect.borrow())
            .render(buf.area, &mut buf);
    }
}

mod effects {
    use ratatui::buffer::Buffer;
    use ratatui::layout::{Offset, Rect};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;
    use rand::{Rng, SeedableRng};
    use ratatui::style::Color;
    use tachyonfx::fx::*;
    use tachyonfx::widget::EffectTimelineRects;
    use tachyonfx::Interpolation::{BounceInOut, BounceOut, CircIn, ExpoInOut, ExpoOut, QuadOut, SineIn};
    use tachyonfx::{CellFilter, Effect, Shader};

    pub(super) fn transition_fx(
        screen: Rect,
        fx_in: Effect,
    ) -> Effect {
        let out_fx = out_fx_1(screen);
        let out_duration = out_fx.timer().unwrap_or_default().duration();

        sequence(vec![
            timed_never_complete(out_duration + Duration::from_millis(500), out_fx),
            // todo: update_inspected_effect(),
            fx_in,
        ])
    }


    pub(super) fn random_fx_in(
        areas: EffectTimelineRects,
    ) -> Effect {
        let rng = &mut rand::rngs::SmallRng::from_entropy();
        match rng.gen_range(0..3) {
            0 => effect_in_1(areas),
            1 => effect_in_2(areas),
            _ => effect_in_3(areas),
        }
    }

    pub(super) fn effect_in_1(areas: EffectTimelineRects) -> Effect {
        parallel(vec![
            tree_fx_1(areas.tree),
            chart_fx_1(areas.chart),
            cell_filter_fx(areas.cell_filter, areas.cell_filter_legend),
        ])
    }

    pub(super) fn effect_in_2(areas: EffectTimelineRects) -> Effect {
        parallel(vec![
            tree_fx_1(areas.tree),
            chart_fx_2(areas.chart),
            cell_filter_fx(areas.cell_filter, areas.cell_filter_legend),
        ])
    }

    pub(super) fn effect_in_3(areas: EffectTimelineRects) -> Effect {
        parallel(vec![
            tree_fx_1(areas.tree),
            chart_fx_3(areas.chart),
            cell_filter_fx(areas.cell_filter, areas.cell_filter_legend),
        ])
    }

    pub(super) fn out_fx_1(area: Rect) -> Effect {
        let step = Duration::from_millis(100);
        let bg = Color::Black;

        with_duration(step * 15, parallel(vec![
            never_complete(dissolve(123, (step * 5, ExpoInOut))),
            never_complete(fade_to_fg(bg, (5 * step, BounceOut))),
        ]).with_area(area))
    }

    fn tree_fx_1(area: Rect) -> Effect {
        let step = Duration::from_millis(100);
        let bg = Color::Black;

        parallel(vec![
            coalesce(100, (step * 5, ExpoInOut))
                .with_cell_selection(CellFilter::Text),
            sweep_in(Direction::UpToDown, 1, bg, step * 3),
        ]).with_area(area)
    }

    fn chart_fx_1(area: Rect) -> Effect {
        let step = Duration::from_millis(100);
        let bg = Color::Black;

        prolong_start(step * 4, sweep_in(Direction::RightToLeft, 5, bg, step * 3))
            .with_area(area)
    }

    fn chart_fx_2(area: Rect) -> Effect {
        let step = Duration::from_millis(100);
        let color1 = Color::from_u32(0x102020);
        let color2 = Color::from_u32(0x204040);

        parallel(vec![
            parallel(vec![
                timed_never_complete(step * 10, fade_to(Color::Black, Color::Black, 0)),
                timed_never_complete(step * 10, fade_to(color1, color1, (step * 5, QuadOut))),
            ]),
            sequence(vec![
                sleep(step * 10),
                parallel(vec![
                    slide_in(Direction::DownToUp, 15, color2, step * 5),
                    fade_from_fg(color1, (step * 10, ExpoOut)),
                ]),
            ])
        ]).with_area(area)
    }

    fn chart_fx_3(area: Rect) -> Effect {
        let step = Duration::from_millis(100);
        let bg = Color::Black;

        let hsl_shift = [0.0, -100.0, -50.0];

        parallel(vec![
            hsl_shift_fg(hsl_shift, (15 * step, CircIn)).reversed(),
            sweep_in(Direction::LeftToRight, 80, bg, step * 15),
        ]).with_area(area)
    }

    pub fn cell_filter_fx(column_area: Rect, legend_area: Rect) -> Effect {
        let base_delay = Duration::from_millis(500);

        parallel(vec![
            sweep_in(Direction::DownToUp, 1, Color::Black, (base_delay, QuadOut))
                .with_area(column_area),
            prolong_start(base_delay * 5, fade_from_fg(Color::Black, (700, QuadOut)))
                .with_area(legend_area),
        ])
    }

    pub(super) fn move_in_fx(direction: Direction, buf: Rc<RefCell<Buffer>>) -> Effect {
        let screen = buf.borrow().area;
        let offset: Offset = match direction {
            Direction::LeftToRight => Offset { x: -(screen.width as i32), y: 0 },
            Direction::RightToLeft => Offset { x: screen.width as i32, y: 0 },
            Direction::UpToDown    => Offset { x: 0, y: -(screen.height as i32) },
            Direction::DownToUp    => Offset { x: 0, y: screen.height as i32 },
        };

        translate_buf(offset, buf.clone(), (750, CircIn)).reversed()
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
    effects.aux_buf_fx = Some(effects::move_in_fx(Direction::LeftToRight, app.aux_buffer.clone()));
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
                        KeyCode::Char(' ') => {
                            let rects = app.timeline.layout(active_area());
                            let fx_in = effects::random_fx_in(rects);
                            let effect = effects::transition_fx(app.screen_area, fx_in);
                            effects.push(effect)
                        },
                        KeyCode::Char('1') => {
                            let inspected = app.inspected_effect.borrow();
                            let rects = EffectTimeline::from(&inspected).layout(active_area());
                            effects.push(effects::effect_in_1(rects))
                        },
                        KeyCode::Char('2') => {
                            let inspected = app.inspected_effect.borrow();
                            let rects = EffectTimeline::from(&inspected).layout(active_area());
                            effects.push(effects::effect_in_2(rects))
                        },
                        KeyCode::Char('3') => {
                            let inspected = app.inspected_effect.borrow();
                            let rects = EffectTimeline::from(&inspected).layout(active_area());
                            effects.push(effects::effect_in_3(rects))
                        },
                        KeyCode::Char('4') => {
                            effects.push(effects::out_fx_1(active_area()))
                        },
                        KeyCode::Char('5') => {
                            effects.aux_buf_fx = Some(effects::move_in_fx(Direction::LeftToRight, app.aux_buffer.clone()))
                        },
                        KeyCode::Tab       => app.use_aux_buffer = !app.use_aux_buffer,
                        _ => {}
                    }
                }
            }
        }
    }
}

fn  ui(
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