use crossterm::event::{DisableMouseCapture, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{event, execute};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Offset, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Clear, Widget};
use ratatui::Frame;
use std::cell::RefCell;
use std::error::Error;
use std::io::Stdout;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;
use std::{io, panic, thread};

use crate::effects::{effect_in, transition_fx};
use tachyonfx::fx::Direction;
use tachyonfx::widget::{EffectTimeline, EffectTimelineRects};
use tachyonfx::{BufferRenderer, Effect, Shader};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;


struct App {
    sender: mpsc::Sender<AppEvent>,
    is_running: bool,
    last_tick: Duration,
    aux_buffer: Rc<RefCell<Buffer>>,
    inspected_effect_no: u8,
    screen_area: Rect,
}

#[derive(Default)]
struct Effects {
    pub aux_buf_fx: Option<Effect>,
    pub post_process: Option<Effect>,
}

impl Effects {
    fn push(&mut self, effect: Effect) {
        self.post_process = Some(effect);
    }

    fn process_active_fx(
        &mut self,
        duration: Duration,
        buf: &mut Buffer,
        area: Rect
    ) {
        self.post_process.iter_mut()
            .for_each(|effect| { effect.process(duration, buf, area); });

        if self.post_process.iter().all(Effect::done) {
            self.post_process = None;
        }
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
        sender: mpsc::Sender<AppEvent>,
        aux_buffer_area: Rect,
    ) -> Self {
        Self {
            sender,
            is_running: true,
            last_tick: Duration::ZERO,
            aux_buffer: Rc::new(RefCell::new(Buffer::empty(aux_buffer_area))),
            screen_area: Rect::default(),
            inspected_effect_no: 0,
        }
    }

    fn inspected_effect(&self) -> Effect {
        let idx = self.inspected_effect_no;
        let fx = effect_in(idx, EffectTimelineRects::default());
        let layout = EffectTimeline::from(&fx).layout(self.aux_buffer.borrow().area);

        effect_in(idx, layout)
    }

    fn refresh_aux_buffer(&self) {
        let effect = self.inspected_effect();

        let mut buf = self.aux_buffer.borrow_mut();
        Clear.render(buf.area, &mut buf);

        Block::new()
            .style(Style::default().bg(Color::Black))
            .render(buf.area, &mut buf);

        EffectTimeline::from(&effect)
            .render(buf.area, &mut buf);
    }

    fn apply_event(&mut self, effects: &mut Effects, e: AppEvent) {
        match e {
            AppEvent::Tick => (),
            AppEvent::KeyPressed(key) => match key {
                KeyCode::Esc => self.is_running = false,
                KeyCode::Char(' ') => {
                    effects.push(
                        // sends RefreshAufBuffer after transitioning out
                        transition_fx(self.screen_area, self.sender.clone(), self.inspected_effect())
                    );
                }
                KeyCode::Enter => {
                    self.inspected_effect_no = (self.inspected_effect_no + 1) % 3;
                    effects.push(
                        // sends RefreshAufBuffer after transitioning out
                        transition_fx(self.screen_area, self.sender.clone(), self.inspected_effect())
                    );
                },
                _ => (),
            },
            AppEvent::RefreshAufBuffer => {
                self.refresh_aux_buffer();
            },
            AppEvent::Resize(r) => self.screen_area = r
        }
    }
}

mod effects {
    use crate::AppEvent;
    use ratatui::buffer::Buffer;
    use ratatui::layout::{Offset, Rect};
    use ratatui::style::Color;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;
    use std::time::Duration;
    use tachyonfx::fx::*;
    use tachyonfx::widget::EffectTimelineRects;
    use tachyonfx::Interpolation::{BounceOut, CircIn, ExpoInOut, ExpoOut, QuadOut};
    use tachyonfx::{CellFilter, Effect};

    pub(super) fn transition_fx(
        screen: Rect,
        sender: mpsc::Sender<AppEvent>,
        fx_in: Effect,
    ) -> Effect {

        // refresh buffer after transitioning out
        let update_inspected_effect = effect_fn_buf((), 1, move |_, _, _| {
            sender.send(AppEvent::RefreshAufBuffer).unwrap();
        });

        sequence(vec![
            out_fx_1(screen),
            update_inspected_effect,
            fx_in,
        ])
    }

    pub(super) fn effect_in(idx: u8, areas: EffectTimelineRects) -> Effect {
        match idx {
            0 => effect_in_1(areas),
            1 => effect_in_2(areas),
            _ => effect_in_3(areas),
        }
    }

    pub(super) fn effect_in_1(areas: EffectTimelineRects) -> Effect {
        parallel(vec![
            tree_fx_1(areas.tree),
            chart_fx_1(areas.chart),
            cell_filter_and_area_fx(areas.cell_filter, areas.areas, areas.legend),
        ])
    }

    pub(super) fn effect_in_2(areas: EffectTimelineRects) -> Effect {
        parallel(vec![
            tree_fx_1(areas.tree),
            chart_fx_2(areas.chart),
            cell_filter_and_area_fx(areas.cell_filter, areas.areas, areas.legend),
        ])
    }

    pub(super) fn effect_in_3(areas: EffectTimelineRects) -> Effect {
        parallel(vec![
            tree_fx_1(areas.tree),
            chart_fx_3(areas.chart),
            cell_filter_and_area_fx(areas.cell_filter, areas.areas, areas.legend),
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

    pub fn cell_filter_and_area_fx(
        cell_filter_column: Rect,
        area_column: Rect,
        legend: Rect
    ) -> Effect {
        let d = Duration::from_millis(500);

        parallel(vec![
            prolong_start(d, sweep_in(Direction::DownToUp, 1, Color::Black, (d, QuadOut)))
                .with_area(cell_filter_column),
            prolong_start(d * 2, sweep_in(Direction::UpToDown, 1, Color::Black, (d, QuadOut)))
                .with_area(area_column),
            prolong_start(d * 3,  fade_from_fg(Color::Black, (700, QuadOut)))
                .with_area(legend),
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

    // event handler
    let event_handler = EventHandler::new(Duration::from_millis(33));
    let sender = event_handler.sender();

    // create app and run it
    let app = App::new(sender, Rect::new(0, 0, 100, 40));
    let res = run_app(&mut terminal, app, event_handler);

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
    event_handler: EventHandler,
) -> io::Result<()> {
    let mut last_frame_instant = std::time::Instant::now();

    let mut effects = Effects::default();
    effects.aux_buf_fx = Some(effects::move_in_fx(Direction::LeftToRight, app.aux_buffer.clone()));
    app.refresh_aux_buffer();

    while app.is_running {
        event_handler.receive_events(|e| app.apply_event(&mut effects, e));

        app.last_tick = last_frame_instant.elapsed();
        last_frame_instant = std::time::Instant::now();
        terminal.draw(|f| {
            app.screen_area = f.area();
            ui(f, &app, &mut effects)
        })?;
    }

    Ok(())
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

    if effects.aux_buf_fx.is_some() {
        effects.process_buf_fx(app.last_tick, buf, rect);
    } else {
        app.aux_buffer.render_buffer(Offset::default(), buf);
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

enum AppEvent {
    Tick,
    KeyPressed(KeyCode),
    Resize(Rect),
    RefreshAufBuffer,
}

pub struct EventHandler {
    sender: mpsc::Sender<AppEvent>,
    receiver: mpsc::Receiver<AppEvent>,
    _handler: thread::JoinHandle<()>
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::channel();

        let handler = {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut last_tick = std::time::Instant::now();
                loop {
                    let timeout = tick_rate
                        .checked_sub(last_tick.elapsed())
                        .unwrap_or(tick_rate);

                    if event::poll(timeout).expect("unable to poll for events") {
                        Self::apply_event(&sender);
                    }

                    if last_tick.elapsed() >= tick_rate {
                        sender.send(AppEvent::Tick)
                            .expect("failed to send tick event");

                        last_tick = std::time::Instant::now();
                    }
                }
            })
        };

        Self { sender, receiver, _handler: handler }
    }

    pub(crate) fn sender(&self) -> mpsc::Sender<AppEvent> {
        self.sender.clone()
    }

    fn next(&self) -> std::result::Result<AppEvent, mpsc::RecvError> {
        self.receiver.recv()
    }

    fn try_next(&self) -> Option<AppEvent> {
        match self.receiver.try_recv() {
            Ok(e) => Some(e),
            Err(_) => None
        }
    }

    pub(crate) fn receive_events<F>(&self, mut f: F)
    where F: FnMut(AppEvent)
    {
        f(self.next().unwrap());
        while let Some(event) = self.try_next() { f(event) }
    }

    fn apply_event(sender: &mpsc::Sender<AppEvent>) {
        match event::read().expect("unable to read event") {
            Event::Key(e) if e.kind == KeyEventKind::Press =>
                sender.send(AppEvent::KeyPressed(e.code)),
            Event::Resize(w, h) =>
                sender.send(AppEvent::Resize(Rect::new(0, 0, w, h))),
            _ => Ok(())
        }.expect("failed to send event")
    }
}