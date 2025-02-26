use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{error::Error, io, time::{Duration as StdDuration, Instant}};
use tui_textarea::TextArea;
use tachyonfx::{dsl::{DslError, EffectDsl}, CenteredShrink, Duration, Effect, EffectRenderer, Shader};

// import the gruvbox colors for consistent theming with other examples
#[path = "common/gruvbox.rs"]
mod gruvbox;
use crate::gruvbox::Gruvbox;

const DEFAULT_DSL_CODE: &str = "fx::sequence(&[
    fx::fade_from_fg(Color::Gray, (500, Interpolation::QuadOut)),
    fx::dissolve((500, Interpolation::BounceOut))
])";


fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = ratatui::init();
    let mut app = App::new();
    app.update_effect(); // compile the initial effect

    // main application loop
    while tick_app(&mut terminal, &mut app)? {}

    ratatui::restore();
    Ok(())
}

struct App<'a> {
    editor: TextArea<'a>,
    dsl: EffectDsl,
    cursor_position: usize,
    compiled_effect: Option<Effect>,
    compilation_error: Option<String>,
    last_frame: Instant,
}

impl<'a> App<'a> {
    fn new() -> Self {
        let mut editor = TextArea::new(DEFAULT_DSL_CODE.lines().map(|s| s.to_string()).collect());
        editor.set_style(theme_editor_style());
        Self {
            editor,
            dsl: EffectDsl::new(),
            cursor_position: DEFAULT_DSL_CODE.char_indices().count(),
            compiled_effect: None,
            compilation_error: None,
            last_frame: Instant::now(),
        }
    }

    fn update_effect(&mut self) {
        // create a DSL compiler instance and bind our content and widget areas
        let dsl_result = self.dsl.compiler()
            // Bind variables that can be used in the DSL code
            .bind("content_area", Rect::new(0, 0, 40, 6)) // the "widget"
            .bind("widget_area", Rect::new(0, 0, 60, 15)) // the rest of the preview area
            .compile(&self.editor.lines().join("\n")); // try compiling the effect

        match dsl_result {
            Ok(effect) => {
                self.compiled_effect = Some(effect);
                self.compilation_error = None;
            }
            Err(err) => {
                self.compiled_effect = None;
                self.compilation_error = Some(format!("Error: {}", err));
            }
        }
    }

    fn update_timer(&mut self) -> Duration {
        let now = Instant::now();
        let elapsed = now - self.last_frame;
        self.last_frame = now;
        elapsed.into()
    }

    fn handle_event(&mut self, event: Event) -> bool {
        if let Event::Key(key) = event {
            // Only handle key presses (not releases)
            if key.kind != KeyEventKind::Press {
                return true;
            }

            match (key.code, key.modifiers) {
                // exit
                (KeyCode::Esc, _) => return false,

                // compile effect
                (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                    self.update_effect();
                }

                // forward key to the editor
                _ => {
                    self.editor.input(key);
                }
            }
        }
        true
    }
}

fn tick_app(terminal: &mut Terminal<impl Backend>, app: &mut App) -> io::Result<bool> {
    // elapsed time for animations
    let elapsed = app.update_timer();

    terminal.draw(|f| ui(f, app, elapsed))?;

    // poll for events with a short timeout (33 is maybe more prudent)
    if event::poll(StdDuration::from_millis(16))? {
        return Ok(app.handle_event(event::read()?));
    }

    Ok(true)
}

fn ui(f: &mut Frame, app: &mut App, elapsed: Duration) {
    // define layout with main areas
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // effect preview area
            Constraint::Min(1),     // editor area
        ])
        .split(f.size());

    // ---  preview area ---
    let preview_area = layout[0];
    let preview_block = Block::default()
        .title("Effect Preview (Ctrl+E to compile and run)")
        .borders(Borders::ALL)
        .border_style(theme_border_style())
        .bg(Gruvbox::Dark0Hard.color());

    f.render_widget(preview_block, preview_area);

    // inner area for the preview content
    let inner_preview = preview_area.inner(Margin::new(1, 1));
    let centered_area = inner_preview.inner_centered(40, 6);

    // setup the quote text to demonstrate the effect on
    let content = Text::from(vec![
        Line::from("You never know what is enough unless")
            .alignment(Alignment::Center),
        Line::from("you know what is more than enough")
            .alignment(Alignment::Center),
        Line::from(""),
        Line::from("— William Blake, Proverbs of Hell")
            .style(theme_author_style())
            .alignment(Alignment::Right),
    ]);

    // render background for the text area
    Block::default()
        .style(theme_quote_style())
        .render(centered_area, f.buffer_mut());

    // render the content
    let content_area = centered_area.inner(Margin::new(1, 1));
    f.render_widget(content, content_area);

    // apply the compiled effect, if any
    if let Some(effect) = &mut app.compiled_effect {
        if effect.running() {
            f.render_effect(effect, centered_area, elapsed);
        }
    }

    // --- editor area ---
    let editor_area = layout[1];
    let editor_block = Block::default()
        .title("Effect DSL Editor")
        .borders(Borders::ALL)
        .border_style(theme_border_style());

    f.render_widget(editor_block, editor_area);

    // render the editor
    let editor_inner = editor_area.inner(Margin::new(1, 1));
    f.render_widget(&app.editor, editor_inner);

    // --- display error message if compilation failed ---
    if let Some(error_msg) = &app.compilation_error {
        let error_area = Rect::new(
            editor_area.x + 2,
            editor_area.y + editor_area.height - 3,
            editor_area.width - 4,
            3,
        );

        let error_block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme_error_style());

        f.render_widget(Clear, error_area);
        f.render_widget(error_block, error_area);

        let paragraph = Paragraph::new(error_msg.as_str())
            .style(theme_error_style())
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, error_area.inner(Margin::new(1, 0)));
    }
}

fn theme_border_style() -> Style {
    Style::default()
        .bg(Gruvbox::Dark0Hard.color())
        .fg(Gruvbox::Orange.color())
}

fn theme_quote_style() -> Style {
    Style::default()
        .bg(Gruvbox::Dark2.color())
        .fg(Gruvbox::Light2.color())
}

fn theme_author_style() -> Style {
    Style::default()
        .fg(Gruvbox::YellowBright.color())
}

fn theme_editor_style() -> Style {
    Style::default()
        .fg(Gruvbox::Light2.color())
        .bg(Gruvbox::Dark0Hard.color())
}

fn theme_error_style() -> Style {
    Style::default()
        .fg(Gruvbox::Red.color())
}