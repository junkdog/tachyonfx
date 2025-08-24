#![allow(
    clippy::std_instead_of_core,
    clippy::std_instead_of_alloc,
    clippy::alloc_instead_of_core
)]

use std::{error::Error, io, time::Instant};

use common::gruvbox::Gruvbox;
use crossterm::{
    event,
    event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind},
};
use ratatui::{
    Frame,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};
use tachyonfx::{
    Duration, EffectManager, Motion, RefRect,
    fx::{self, dynamic_area, parallel, sequence},
};

/// Effect identifiers for different UI components
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EffectId {
    #[default]
    Button1,
    Button2,
    Button3,
    StatusMessage,
}

/// Application events that can trigger effects
#[derive(Debug, Clone)]
enum AppEvent {
    ButtonClicked(u8),
    ButtonHovered(u8),
    ButtonUnhovered(u8),
    ShowMessage(String),
    ClearMessage,
}

/// Effect registry that manages all visual effects
struct EffectRegistry {
    effects: EffectManager<EffectId>,
    button_areas: [RefRect; 3],
    message_area: RefRect,
    screen_area: RefRect,
}

impl EffectRegistry {
    fn new() -> Self {
        Self {
            effects: EffectManager::default(),
            button_areas: [RefRect::default(), RefRect::default(), RefRect::default()],
            message_area: RefRect::default(),
            screen_area: RefRect::default(),
        }
    }

    fn handle_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::ButtonClicked(button_id) => {
                self.create_click_effect(*button_id as usize);
            },
            AppEvent::ButtonHovered(button_id) => {
                self.create_hover_effect(*button_id as usize);
            },
            AppEvent::ButtonUnhovered(button_id) => {
                self.create_unhover_effect(*button_id as usize);
            },
            AppEvent::ShowMessage(message) => {
                self.create_message_effect(message.clone());
            },
            AppEvent::ClearMessage => {
                self.clear_message_effect();
            },
        }
    }

    fn create_click_effect(&mut self, button_id: usize) {
        if button_id >= 3 {
            return;
        }

        let effect_id = match button_id {
            0 => EffectId::Button1,
            1 => EffectId::Button2,
            2 => EffectId::Button3,
            _ => return,
        };

        let button_area = self.button_areas[button_id].clone();

        // Create a dramatic click effect
        let click_effect = sequence(&[
            // Flash bright
            fx::fade_to(
                Gruvbox::YellowBright,
                Gruvbox::YellowBright,
                Duration::from_millis(100),
            ),
            // Pulse effect
            parallel(&[
                fx::fade_to(Gruvbox::Light0, Gruvbox::Light0, Duration::from_millis(150)),
                fx::fade_to(
                    Gruvbox::OrangeBright,
                    Gruvbox::OrangeBright,
                    Duration::from_millis(150),
                ),
            ]),
            // Return to normal with slight glow
            fx::fade_to(Gruvbox::Blue, Gruvbox::Blue, Duration::from_millis(200)),
            // Fade to normal
            fx::fade_to(Color::Reset, Color::Reset, Duration::from_millis(300)),
        ]);

        let effect = dynamic_area(button_area, click_effect);
        self.effects.add_unique_effect(effect_id, effect);
    }

    fn create_hover_effect(&mut self, button_id: usize) {
        if button_id >= 3 {
            return;
        }

        let effect_id = match button_id {
            0 => EffectId::Button1,
            1 => EffectId::Button2,
            2 => EffectId::Button3,
            _ => return,
        };

        let button_area = self.button_areas[button_id].clone();

        // Gentle hover effect
        let hover_effect = fx::fade_to(Gruvbox::Blue, Gruvbox::Blue, Duration::from_millis(200));
        let effect = dynamic_area(button_area, hover_effect);
        self.effects.add_unique_effect(effect_id, effect);
    }

    fn create_unhover_effect(&mut self, button_id: usize) {
        if button_id >= 3 {
            return;
        }

        let effect_id = match button_id {
            0 => EffectId::Button1,
            1 => EffectId::Button2,
            2 => EffectId::Button3,
            _ => return,
        };

        let button_area = self.button_areas[button_id].clone();

        // Fade back to normal
        let unhover_effect = fx::fade_to(Color::Reset, Color::Reset, Duration::from_millis(200));
        let effect = dynamic_area(button_area, unhover_effect);
        self.effects.add_unique_effect(effect_id, effect);
    }

    fn create_message_effect(&mut self, _message: String) {
        let message_area = self.message_area.clone();

        // Complex message effect with entrance, display, and exit
        let message_effect = sequence(&[
            // Slide in from left
            fx::slide_in(
                Motion::LeftToRight,
                10,
                0,
                Gruvbox::Green,
                Duration::from_millis(300),
            ),
            // Gentle pulse while displayed
            fx::ping_pong(fx::fade_to(
                Gruvbox::GreenBright,
                Gruvbox::GreenBright,
                Duration::from_millis(800),
            )),
            // Hold for 2 seconds
            fx::sleep(Duration::from_millis(2000)),
            // Fade out
            fx::fade_to(Color::Reset, Color::Reset, Duration::from_millis(400)),
        ]);

        let effect = dynamic_area(message_area, message_effect);
        self.effects
            .add_unique_effect(EffectId::StatusMessage, effect);
    }

    fn clear_message_effect(&mut self) {
        let message_area = self.message_area.clone();
        let clear_effect = fx::fade_to(Color::Reset, Color::Reset, Duration::from_millis(200));
        let effect = dynamic_area(message_area, clear_effect);
        self.effects
            .add_unique_effect(EffectId::StatusMessage, effect);
    }

    fn update_button_area(&mut self, button_id: usize, area: Rect) {
        if button_id < 3 {
            self.button_areas[button_id].set(area);
        }
    }

    fn update_message_area(&mut self, area: Rect) {
        self.message_area.set(area);
    }

    fn update_screen_area(&mut self, area: Rect) {
        self.screen_area.set(area);
    }

    fn process_effects(
        &mut self,
        duration: Duration,
        buf: &mut ratatui::buffer::Buffer,
        area: Rect,
    ) {
        self.effects.process_effects(duration, buf, area);
    }
}

/// A button widget that integrates with the effect registry
#[derive(Clone)]
struct Button {
    id: u8,
    text: String,
    area: RefRect,
    is_hovered: bool,
}

impl Button {
    fn new(id: u8, text: String) -> Self {
        Self {
            id,
            text,
            area: RefRect::default(),
            is_hovered: false,
        }
    }

    fn handle_click(&mut self, registry: &mut EffectRegistry) {
        registry.handle_event(&AppEvent::ButtonClicked(self.id));

        // Also show a status message
        let message = format!("Button {} clicked!", self.id + 1);
        registry.handle_event(&AppEvent::ShowMessage(message));
    }

    fn handle_hover(&mut self, registry: &mut EffectRegistry, hovered: bool) {
        if hovered && !self.is_hovered {
            registry.handle_event(&AppEvent::ButtonHovered(self.id));
            self.is_hovered = true;
        } else if !hovered && self.is_hovered {
            registry.handle_event(&AppEvent::ButtonUnhovered(self.id));
            self.is_hovered = false;
        }
    }

    fn area(&self) -> Rect {
        self.area.get()
    }
}

impl Widget for Button {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        // Update the RefRect - this is crucial for dynamic effects!
        self.area.set(area);

        // Render the button
        let button = Paragraph::new(self.text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Button {}", self.id + 1))
                    .style(Style::default().fg(Gruvbox::Light4.into())),
            )
            .alignment(Alignment::Center)
            .style(Style::default().fg(Gruvbox::Light2.into()));

        button.render(area, buf);
    }
}

/// Main application state
struct App {
    registry: EffectRegistry,
    buttons: [Button; 3],
    is_running: bool,
    message: String,
}

impl App {
    fn new() -> Self {
        Self {
            registry: EffectRegistry::new(),
            buttons: [
                Button::new(0, "Effect 1".to_string()),
                Button::new(1, "Effect 2".to_string()),
                Button::new(2, "Effect 3".to_string()),
            ],
            is_running: true,
            message: "Click buttons to see effects! Mouse support enabled.".to_string(),
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            self.is_running = false;
                        },
                        KeyCode::Char('1') => {
                            self.buttons[0].handle_click(&mut self.registry);
                        },
                        KeyCode::Char('2') => {
                            self.buttons[1].handle_click(&mut self.registry);
                        },
                        KeyCode::Char('3') => {
                            self.buttons[2].handle_click(&mut self.registry);
                        },
                        KeyCode::Char('c') => {
                            self.registry
                                .handle_event(&AppEvent::ClearMessage);
                        },
                        _ => {},
                    }
                }
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    let pos = (mouse.column, mouse.row);
                    for button in &mut self.buttons {
                        if button.area().contains(pos.into()) {
                            button.handle_click(&mut self.registry);
                        }
                    }
                },
                MouseEventKind::Moved => {
                    let pos = (mouse.column, mouse.row);
                    for button in &mut self.buttons {
                        let hovered = button.area().contains(pos.into());
                        button.handle_hover(&mut self.registry, hovered);
                    }
                },
                _ => {},
            },
            _ => {},
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.registry.update_screen_area(area);

        // Clear the screen
        frame.render_widget(Clear, area);

        // Main layout
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(5), // Buttons
                Constraint::Length(3), // Message area
                Constraint::Min(0),    // Help
            ])
            .split(area);

        // Title
        let title = Paragraph::new("EffectRegistry Demo")
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Gruvbox::Yellow.into())
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(title, main_layout[0]);

        // Buttons layout
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(main_layout[1]);

        // Render buttons and update their areas in the registry
        for (i, button_area) in button_layout.iter().enumerate() {
            self.registry.update_button_area(i, *button_area);
            frame.render_widget(self.buttons[i].clone(), *button_area);
        }

        // Message area
        self.registry.update_message_area(main_layout[2]);
        let message_widget = Paragraph::new(self.message.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Status"),
            )
            .alignment(Alignment::Center)
            .style(Style::default().fg(Gruvbox::Light3.into()));
        frame.render_widget(message_widget, main_layout[2]);

        // Help text
        let help_text = vec![
            Line::from(vec![
                Span::styled("Controls: ", Style::default().fg(Gruvbox::Blue.into())),
                Span::raw("1/2/3 - Click buttons, "),
                Span::raw("c - Clear message, "),
                Span::raw("q/Esc - Quit"),
            ]),
            Line::from(vec![
                Span::styled("Mouse: ", Style::default().fg(Gruvbox::Green.into())),
                Span::raw("Click buttons, move to hover"),
            ]),
        ];

        let help = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Help"),
            )
            .style(Style::default().fg(Gruvbox::Light4.into()));
        frame.render_widget(help, main_layout[3]);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture,
    )?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Create app
    let mut app = App::new();
    let mut last_tick = Instant::now();

    // Main loop
    while app.is_running {
        let timeout = std::time::Duration::from_millis(16); // ~60 FPS
        let now = Instant::now();
        let elapsed = now.duration_since(last_tick);
        last_tick = now;

        // Handle events
        if event::poll(timeout)? {
            let event = event::read()?;
            app.handle_event(event);
        }

        // Render
        terminal.draw(|frame| {
            // Render UI
            app.render(frame);

            // Process effects
            let tachyon_duration = Duration::from_millis(elapsed.as_millis() as u32);
            let frame_area = frame.area();
            app.registry
                .process_effects(tachyon_duration, frame.buffer_mut(), frame_area);
        })?;
    }

    // Cleanup
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
    )?;

    Ok(())
}
