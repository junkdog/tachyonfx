use std::{
    io,
    time::{Duration as StdDuration, Instant},
};

use ratatui::{
    crossterm::event::{self, Event},
    prelude::*,
    widgets::{Block, Clear},
};
use tachyonfx::{
    fx::{self, Direction as FxDirection, Glitch},
    CenteredShrink, Duration, Effect, EffectRenderer, EffectTimer, Interpolation, IntoEffect,
    Shader, SimpleRng,
};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let running_time = Instant::now();

    // You can construct effects using the builder pattern.
    let mut glitch: Effect = Glitch::builder()
        .rng(SimpleRng::default())
        .action_ms(200..400)
        .action_start_delay_ms(0..1)
        .cell_glitch_ratio(1.0)
        .build()
        .into_effect();

    // Or you can use the provided effect combinators.
    let mut effect = fx::sequence(&[
        // first we "sweep in" the text from the left, before reversing the effect
        fx::ping_pong(fx::sweep_in(
            FxDirection::LeftToRight,
            10,
            0,
            Color::DarkGray,
            EffectTimer::from_ms(2000, Interpolation::QuadIn),
        )),
        // then we coalesce the text back to its original state
        // (note that EffectTimers can be constructed from a tuple of duration and interpolation)
        fx::coalesce((800, Interpolation::SineOut))
    ]);

    loop {
        // Render the glitch effect after 10 seconds.
        let effect = if running_time.elapsed() > StdDuration::from_secs(10) {
            &mut glitch
        } else {
            &mut effect
        };
        terminal.draw(|f| ui(f, effect))?;
        if event::poll(StdDuration::from_millis(100))? {
            if let Event::Key(_) = event::read()? {
                break;
            }
        }
    }
    ratatui::restore();
    Ok(())
}

fn ui(f: &mut Frame<'_>, effect: &mut Effect) {
    Clear.render(f.area(), f.buffer_mut());
    Block::default()
        .style(Style::default().bg(Color::DarkGray))
        .render(f.area(), f.buffer_mut());
    let area = f.area().inner_centered(25, 2);
    let main_text = Text::from(vec![
        Line::from("Hello, TachyonFX!"),
        Line::from("Press any key to exit."),
    ]);
    f.render_widget(main_text.white().centered(), area);
    if effect.running() {
        f.render_effect(effect, area, Duration::from_millis(100));
    }
}
