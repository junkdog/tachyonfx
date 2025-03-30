use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use std::{error::Error, io, time::{Duration as StdDuration, Instant}};
use tachyonfx::{dsl::EffectDsl, CenteredShrink, Duration, Effect, EffectRenderer, Shader};
use tui_textarea::TextArea;

const RATATUI_MASCOT: &str = indoc::indoc! {"
                   hhh
                 hhhhhh
                hhhhhhh
               hhhhhhhh
              hhhhhhhhh
             hhhhhhhhhh
            hhhhhhhhhhhh
            hhhhhhhhhhhhh
            hhhhhhhhhhhhh     ██████
             hhhhhhhhhhh    ████████
                  hhhhh ███████████
                   hhh ██ee████████
                    h █████████████
                ████ █████████████
               █████████████████
               ████████████████
               ████████████████
                ███ ██████████
              ▒▒    █████████
             ▒░░▒   █████████
            ▒░░░░▒ ██████████
           ▒░░▓░░░▒ █████████
          ▒░░▓▓░░░░▒ ████████
         ▒░░░░░░░░░░▒ ██████████
        ▒░░░░░░░░░░░░▒ ██████████
       ▒░░░░░░░▓▓░░░░░▒ █████████
      ▒░░░░░░░░░▓▓░░░░░▒ ████  ███
     ▒░░░░░░░░░░░░░░░░░░▒ ██   ███
    ▒░░░░░░░░░░░░░░░░░░░░▒ █   ███
    ▒░░░░░░░░░░░░░░░░░░░░░▒   ███
     ▒░░░░░░░░░░░░░░░░░░░░░▒ ███
      ▒░░░░░░░░░░░░░░░░░░░░░▒ █"
};

// import the gruvbox colors for consistent theming with other examples
#[path = "common/gruvbox.rs"]
mod gruvbox;
use crate::gruvbox::Gruvbox;

const DEFAULT_DSL_CODE: &str = r#"// Try these combinations:
// 1. Apply effect to both mascot and blake areas
fx::parallel(&[
    fx::fade_from_fg(Color::Gray, (1200, Interpolation::QuadOut))
        .with_area(mascot_area),
    fx::dissolve((1200, Interpolation::BounceOut))
        .with_area(blake_area)
])

// 2. Or create separate effects for each area:
/*
let t = (800, Linear);
let t2 = (1500, Linear);

let not_blake = CellFilter::Not(Box::new(CellFilter::Area(blake_area)));
let reset_filter = CellFilter::BgColor(Color::from_u32(0x000000));

parallel(&[
    // slide in mascot
    slide_in(UpToDown, 40, 0, Color::from_u32(0x101010), t)
        .with_filter(not_blake),

    // flicker blake quote
    prolong_start(1000,
        dissolve((500, BounceOut)).with_area(blake_area)),

    // explode the quote; fade into background (approx)
    prolong_start(t2, parallel(&[
        // fading colors to an approx of bg
        fade_to_fg(Color::from_u32(0x404040), t),

        // the delays avoid triggering effects prematurely
        delay(1, explode(20.0, 0.4, t)),

        // fade quote back in
        delay(800, fade_from(Color::Black, Color::Black, (800, SineOut))),
    ]).with_area(blake_area)),
])
*/
"#;


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
    pub compiled_effect: Option<Effect>,
    compilation_error: Option<String>,
    last_frame: Instant,
    full_effect_area: Rect,
    blake_area: Rect,
    mascot_area: Rect,
}

impl App<'_> {
    fn new() -> Self {
        let mut editor = TextArea::new(DEFAULT_DSL_CODE.lines().map(|s| s.to_string()).collect());
        editor.set_style(theme_editor_style());
        Self {
            editor,
            dsl: EffectDsl::new(),
            compiled_effect: None,
            compilation_error: None,
            last_frame: Instant::now(),
            full_effect_area: Rect::default(),
            blake_area: Rect::default(),
            mascot_area: Rect::default(),
        }
    }

    fn update_effect(&mut self) {
        // create a DSL compiler instance and bind the content and preview areas
        let dsl_result = self.dsl.compiler()
            // Bind variables that can be used in the DSL code
            .bind("blake_area", self.blake_area)          // the Blake quote widget
            .bind("mascot_area", self.mascot_area)        // the Ratatui mascot area
            .bind("full_area", self.full_effect_area)     // the entire preview area
            .compile(&self.editor.lines().join("\n"));    // compile the effect

        match dsl_result {
            Ok(effect) => {
                self.compiled_effect = Some(effect);
                self.compilation_error = None;
            }
            Err(err) => {
                self.compiled_effect = None;
                self.compilation_error = Some(format!("{}", err));
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

    terminal.draw(|f| {
        // define layout with main areas; we persist the areas
        // so that we can bind them to the DSL compiler instance
        let layout = Layout::vertical([
                Constraint::Length(35), // increased height for effect preview area
                Constraint::Min(1),     // editor area
            ])
            .split(f.area());

        app.full_effect_area = layout[0];
        ui(f, app, &layout, elapsed)
    })?;

    // poll for events at ~30hz
    if event::poll(StdDuration::from_millis(33))? {
        return Ok(app.handle_event(event::read()?));
    }

    Ok(true)
}

fn ui(f: &mut Frame, app: &mut App, layout: &[Rect], elapsed: Duration) {
    // ---  preview area ---
    let preview_area = layout[0];
    let preview_block = Block::default()
        .title("Effect Preview (Ctrl+E to compile and run, ESC to exit)")
        .borders(Borders::ALL)
        .border_style(theme_border_style())
        .bg(Gruvbox::Dark0Hard.color());

    f.render_widget(preview_block, preview_area);

    // inner area for the preview content
    let inner_preview = preview_area.inner(Margin::new(1, 1));

    // update areas for binding to DSL
    app.mascot_area = inner_preview;

    // Set up a centered area for the Blake quote
    let blake_container = Rect::new(40, 4, 40, 6);
    app.blake_area = blake_container.inner_centered(40, 6);

    // --- render the Ratatui mascot ---
    let mascot_block = Block::default()
        .style(theme_mascot_style())
        .title(Line::from(vec![
            Span::from(" area bound as "),
            Span::from("mascot_area ").style(theme_mascot_style().add_modifier(Modifier::BOLD)),
        ]))
        .borders(Borders::ALL);

    let mascot_text = Paragraph::new(RATATUI_MASCOT)
        .style(theme_mascot_text_style())
        .block(mascot_block)
        .alignment(Alignment::Left);

    f.render_widget(mascot_text, app.mascot_area);

    // --- setup the Blake quote text ---
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

    // render background for the Blake text area
    Block::default()
        .style(theme_quote_style())
        .title(Line::from(vec![
            Span::from(" area bound as "),
            Span::from("blake_area ").style(theme_quote_style().add_modifier(Modifier::BOLD)),
        ]))
        .borders(Borders::ALL)
        .render(app.blake_area, f.buffer_mut());

    // render the Blake content
    let content_area = app.blake_area.inner(Margin::new(1, 1));
    f.render_widget(content, content_area);

    // apply the compiled effect, if any
    if let Some(effect) = &mut app.compiled_effect {
        if effect.running() {
            f.render_effect(effect, inner_preview, elapsed);
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
        let message = error_msg;

        let error_area = Rect::new(
            editor_area.x + 1,
            editor_area.y + editor_area.height - 5,
            editor_area.width - 2,
            4,
        );

        let error_block = Block::default()
            .style(theme_error_style());

        f.render_widget(Clear, error_area);
        f.render_widget(error_block, error_area);

        Text::from_iter(message.lines())
            .alignment(Alignment::Left)
            .render(error_area, f.buffer_mut());
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

fn theme_mascot_style() -> Style {
    Style::default()
        .bg(Gruvbox::Dark0Soft.color())
}

fn theme_mascot_text_style() -> Style {
    Style::default()
        .fg(Gruvbox::YellowBright.color())
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
        .bg(Gruvbox::Dark1.color())
        .fg(Gruvbox::Light3.color())
}