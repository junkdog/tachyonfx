use std::{
    io,
    time::{Duration as StdDuration, Instant},
};

use common::gruvbox::Gruvbox;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event},
    prelude::*,
    widgets::{Block, Borders},
};
use tachyonfx::{
    CellFilter, CenteredShrink, Duration, Effect, EffectRenderer, Interpolation, fx,
    fx::{consume_tick, never_complete, timed_never_complete, with_duration},
};

// these example effects are used for the tachyonfx::fx module documentation
mod examples {
    use ratatui::layout::Offset;
    use tachyonfx::{Motion, color_from_hsl};

    use super::*;

    pub fn coalesce() -> Effect {
        fx::coalesce((1000, Interpolation::BounceOut))
    }

    pub fn coalesce_from() -> Effect {
        let c = Theme::oob_color();
        let style = Style::default().bg(c);
        fx::coalesce_from(style, (1000, Interpolation::ExpoInOut))
    }

    pub fn dissolve() -> Effect {
        fx::dissolve(1000) // linear interpolation
    }

    pub fn dissolve_to() -> Effect {
        let c = Theme::oob_color();
        let style = Style::default().bg(c);
        fx::dissolve_to(style, (1000, Interpolation::CircOut))
    }

    pub fn fade_from_fg() -> Effect {
        // fade in content, excluding borders, from the bg color
        let c = Theme::quote().bg.expect("bg color to exist");
        let filter = CellFilter::Inner(Margin::new(1, 1));
        fx::fade_from_fg(c, (1000, Interpolation::QuadInOut)).with_filter(filter)
    }

    pub fn fade_to_fg() -> Effect {
        // fade out blake by targeting the author fg color
        let c = Theme::quote().bg.expect("bg color to exist");
        let filter = CellFilter::FgColor(Theme::author().fg.unwrap());
        fx::fade_to_fg(c, (1000, Interpolation::CircOut)).with_filter(filter)
    }

    pub fn fade_from() -> Effect {
        // fade in the entire area from the out-of-bounds color
        let c = Theme::oob_color();
        fx::fade_from(c, c, (1000, Interpolation::CircOut))
    }

    pub fn fade_to() -> Effect {
        // fade the entire area to the out-of-bounds color
        let c = Theme::oob_color();
        fx::fade_to(c, c, (1000, Interpolation::CircOut))
    }

    pub fn sweep_in() -> Effect {
        // sweep in from the left with a gradient length of 10 and no randomness
        let c = Theme::oob_color();
        let timer = (1000, Interpolation::Linear);
        fx::sweep_in(Motion::LeftToRight, 10, 0, c, timer)
    }

    pub fn sweep_out() -> Effect {
        // sweep out to the bottom-right corner
        let c = Theme::oob_color();
        let timer = (1000, Interpolation::Linear);
        fx::sweep_out(Motion::RightToLeft, 10, 0, c, timer)
    }

    pub fn slide_in() -> Effect {
        // slide in from the top, with some randomness
        let c = Theme::oob_color();
        let timer = (1000, Interpolation::Linear);
        fx::slide_in(Motion::UpToDown, 10, 0, c, timer)
    }

    pub fn slide_out() -> Effect {
        // slide out to the right, with some randomness
        let c = Theme::oob_color();
        let timer = (1000, Interpolation::Linear);
        fx::slide_out(Motion::LeftToRight, 24, 0, c, timer)
    }

    pub fn hsl_shift() -> Effect {
        // shift the hue of the entire area
        let timer = (1000, Interpolation::Linear);
        let fg_shift = [120.0, 25.0, 25.0];
        let bg_shift = [-40.0, -50.0, -50.0];
        fx::hsl_shift(Some(fg_shift), Some(bg_shift), timer)
    }

    pub fn hsl_shift_fg() -> Effect {
        // shift the hue of the entire area
        let timer = (1000, Interpolation::Linear);
        let fg_shift = [120.0, 25.0, 25.0];
        fx::hsl_shift(Some(fg_shift), None, timer)
    }

    pub fn parallel() -> Effect {
        // fade in the entire area from the out-of-bounds color
        let c = Theme::quote().bg.unwrap();
        let timer = (1000, Interpolation::CircOut);
        fx::parallel(&[fx::fade_from_fg(c, timer), fx::coalesce(timer)])
    }

    pub fn sequence() -> Effect {
        // fade in the entire area from the out-of-bounds color
        let c = Theme::quote().bg.unwrap();
        let timer = (500, Interpolation::CircOut);
        fx::sequence(&[fx::fade_from_fg(c, timer), fx::dissolve(timer)])
    }

    pub fn delay() -> Effect {
        // wait 800ms before dissolving the content
        fx::delay(800, fx::dissolve(200))
    }

    pub fn never_complete() -> Effect {
        // immediately turns the content foreground color to white,
        // but never completes, effectively freezing the effect.
        // with_duration is used to prevent the effect from running
        // indefinitely.
        let zero_timer = 0;
        with_duration(
            Duration::from_millis(1000),
            fx::never_complete(fx::hsl_shift_fg([0.0, 0.0, 100.0], zero_timer)),
        )
    }

    pub fn ping_pong() -> Effect {
        let timer = (500, Interpolation::CircOut);
        fx::ping_pong(fx::coalesce(timer))
    }

    pub fn prolong_start() -> Effect {
        // hold the start state for 500ms before fading in the content
        let c = Theme::quote().bg.unwrap();
        let timer = (500, Interpolation::CircOut);
        fx::prolong_start(timer, fx::fade_from_fg(c, timer))
    }

    pub fn prolong_end() -> Effect {
        // hold the end state for 500ms after fading out the content
        let c = Theme::quote().bg.unwrap();
        let timer = (500, Interpolation::CircOut);
        fx::prolong_end(timer, fx::fade_to_fg(c, timer))
    }

    pub fn effect_fn() -> Effect {
        // This example creates an effect that runs indefinitely and cycles the color of each
        // foreground cell based on the elapsed time. Each cell's color is slightly offset by
        // the cell's position.

        fx::effect_fn(Instant::now(), 1000, |state, _ctx, cell_iter| {
            let cycle: f32 = (state.elapsed().as_millis() % 3600) as f32;
            cell_iter
                .filter(|(_, cell)| cell.symbol() != " ")
                .enumerate()
                .for_each(|(i, (_pos, cell))| {
                    let hue = (2.0 * i as f32 + cycle * 0.2) % 360.0;
                    let color = color_from_hsl(hue, 100.0, 50.0);
                    cell.set_fg(color);
                });
        })
    }

    pub fn effect_fn_buf() -> Effect {
        use ratatui::style::Color;
        use tachyonfx::*;

        let timer = EffectTimer::from_ms(1000, Interpolation::Linear);
        let no_state = (); // no state to keep track of

        fx::effect_fn_buf(no_state, timer, |_state, context, buf| {
            let offset = context.timer.remaining().as_millis() as usize / 30;

            let cell_pred = context.filter().map(|f| f.predicate(buf.area));
            for (i, pos) in buf.area.positions().enumerate() {
                let cell = &mut buf[pos];
                if cell_pred
                    .as_ref()
                    .is_some_and(|p| p.is_valid(pos, cell))
                {
                    cell.set_fg(Color::Indexed(((offset + i) % 256) as u8));
                }
            }
        })
        .with_filter(CellFilter::Text)
    }

    #[allow(dead_code)]
    pub fn translate_buf() -> Effect {
        use tachyonfx::*;

        let area = Rect::new(0, 0, 10, 10);
        let mut buf = Buffer::empty(area);
        Block::bordered()
            .title("translated")
            .render(area, &mut buf);

        let timer = EffectTimer::from_ms(1000, Interpolation::Linear);
        fx::translate_buf(Offset { x: -30, y: 0 }, ref_count(buf), timer)
    }
}

fn main() -> io::Result<()> {
    let wait_for_input = || never_complete(consume_tick());
    let mut terminal = ratatui::init();

    run_with_effect(&mut terminal, wait_for_input)?;
    run_example(&mut terminal, examples::fade_from)?;
    run_example(&mut terminal, examples::fade_to)?;
    run_example(&mut terminal, examples::fade_from_fg)?;
    run_example(&mut terminal, examples::fade_to_fg)?;
    run_example(&mut terminal, examples::coalesce)?;
    run_example(&mut terminal, examples::coalesce_from)?;
    run_example(&mut terminal, examples::dissolve)?;
    run_example(&mut terminal, examples::dissolve_to)?;
    run_example(&mut terminal, examples::sweep_in)?;
    run_example(&mut terminal, examples::sweep_out)?;
    run_example(&mut terminal, examples::slide_in)?;
    run_example(&mut terminal, examples::slide_out)?;
    run_example(&mut terminal, examples::hsl_shift)?;
    run_example(&mut terminal, examples::hsl_shift_fg)?;
    run_example(&mut terminal, examples::parallel)?;
    run_example(&mut terminal, examples::sequence)?;
    run_example(&mut terminal, examples::delay)?;
    run_example(&mut terminal, examples::never_complete)?;
    run_example(&mut terminal, examples::ping_pong)?;
    run_example(&mut terminal, examples::prolong_start)?;
    run_example(&mut terminal, examples::prolong_end)?;
    run_example(&mut terminal, examples::effect_fn)?;
    run_example(&mut terminal, examples::effect_fn_buf)?;
    ratatui::restore();

    Ok(())
}

fn run_example<F: FnOnce() -> Effect>(terminal: &mut DefaultTerminal, effect: F) -> io::Result<()> {
    let started = Instant::now();

    let reset = |duration| timed_never_complete(Duration::from_millis(duration), consume_tick());

    run_with_effect(terminal, || reset(1000))?;
    run_with_effect(terminal, effect)?;

    // in order to easily split the recording into individual files per effect,
    // we try to keep the total duration of each effect to 2 seconds.
    let remaining = 3000u32.saturating_sub(started.elapsed().as_millis() as u32);
    run_with_effect(terminal, || reset(remaining as _))?;

    Ok(())
}

fn run_with_effect<F: FnOnce() -> Effect>(
    terminal: &mut DefaultTerminal,
    effect: F,
) -> io::Result<()> {
    let mut app = App::new();
    let mut effect = effect();

    let label = effect.name();
    let label = if ["with_duration", "never_complete"].contains(&label) { "" } else { label };

    // Main render loop:
    // - Continues until a key is pressed or the effect completes
    // - Updates timer to track animation progress
    // - Renders UI and applies the current effect
    // - Effect is applied to a centered 40x6 area
    while poll_for_events(16) && effect.running() {
        // ~60 FPS; for faster transitions
        let elapsed: Duration = app.update_timer();

        terminal.draw(|f| {
            let area = f.area().inner_centered(40, 6);
            render_ui(f, area, label);
            f.render_effect(&mut effect, area, elapsed);
        })?;
    }

    Ok(())
}

fn poll_for_events(poll_timeout_ms: u64) -> bool {
    !(event::poll(StdDuration::from_millis(poll_timeout_ms)).expect("poll to work")
        && matches!(event::read().expect("read to work"), Event::Key(_)))
}

fn render_ui(f: &mut Frame<'_>, area: Rect, label: &str) {
    // clear the area with the out-of-bounds color
    Block::default()
        .style(Style::default().bg(Theme::oob_color()))
        .render(f.area(), f.buffer_mut());

    // render the content area and border
    Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border_style())
        .title(label)
        .style(Theme::quote())
        .render(area, f.buffer_mut());

    // the marriage of heaven and hell
    let content = Text::from(vec![
        Line::from("You never know what is enough unless").alignment(Alignment::Center),
        Line::from("you know what is more than enough").alignment(Alignment::Center),
        Line::from(""),
        Line::from("— William Blake, Proverbs of Hell")
            .style(Theme::author())
            .alignment(Alignment::Right),
    ]);

    // render the content area
    let content_area = area.inner(Margin::new(1, 1));
    f.render_widget(content, content_area);
}

struct App {
    last_frame: Instant,
}

impl App {
    fn new() -> Self {
        Self { last_frame: Instant::now() }
    }

    fn update_timer(&mut self) -> Duration {
        let now = Instant::now();
        let elapsed = now - self.last_frame;
        self.last_frame = now;
        elapsed.into()
    }
}

struct Theme;

impl Theme {
    const fn oob_color() -> Color {
        Gruvbox::Dark0Hard.color()
    }

    fn border_style() -> Style {
        Style::default()
            .bg(Gruvbox::Dark2.color())
            .fg(Gruvbox::Orange.color())
    }

    fn quote() -> Style {
        Style::default()
            .bg(Gruvbox::Dark2.color())
            .fg(Gruvbox::Light2.color())
    }

    fn author() -> Style {
        Style::default().fg(Gruvbox::YellowBright.color())
    }
}
