use std::{
    collections::VecDeque,
    io,
    process::{Command, Stdio},
    time::{Duration as StdDuration, Instant},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Widget},
    Frame, Terminal,
};
use serde_json::Value;
use tachyonfx::{
    fx::{self, never_complete},
    Duration, Effect, EffectRenderer, IntoEffect, Motion,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone, PartialEq)]
enum AppState {
    BootSequence,
    MatrixTransition,
    MainInterface,
}

#[derive(Debug, Clone)]
struct OllamaModel {
    name: String,
    size: String,
    modified: String,
}

#[derive(Debug, Clone)]
struct ChatMessage {
    role: String,
    content: String,
    timestamp: Instant,
}

struct App {
    state: AppState,
    boot_progress: f64,
    boot_stage: usize,
    boot_messages: Vec<String>,
    matrix_effect: Effect,
    typewriter_effect: Effect,
    wave_effect: Effect,
    fire_effect: Effect,
    avatar_pos: (f32, f32),
    avatar_direction: (f32, f32),
    avatar_color: Color,
    background_hue: f32,
    chat_messages: VecDeque<ChatMessage>,
    chat_input: String,
    selected_model: usize,
    available_models: Vec<OllamaModel>,
    last_tick: Instant,
    input_mode: bool,
    scroll_offset: usize,
}

impl App {
    fn new() -> Self {
        let boot_messages = vec![
            "INITIALIZING TACTICAL ESPIONAGE SYSTEM...".to_string(),
            "CODEC SYSTEM ONLINE".to_string(),
            "STEALTH CAMOUFLAGE CALIBRATED".to_string(),
            "SOLID EYE ACTIVATED".to_string(),
            "NANOMACHINE SYNCHRONIZATION... OK".to_string(),
            "SECURITY CLEARANCE: TOP SECRET".to_string(),
            "MISSION PARAMETERS LOADED".to_string(),
            "NEURAL INTERFACE ESTABLISHED".to_string(),
            "AI COMPANION SYSTEMS READY".to_string(),
            "WELCOME, AGENT. READY FOR OPERATION.".to_string(),
        ];

        Self {
            state: AppState::BootSequence,
            boot_progress: 0.0,
            boot_stage: 0,
            boot_messages,
            matrix_effect: never_complete(fx::fade_from_fg(Color::Green, Duration::from_millis(2000)).into_effect()),
            typewriter_effect: fx::slide_in(Motion::LeftToRight, 20, 0, Color::Green, Duration::from_millis(4000)).into_effect(),
            wave_effect: never_complete(fx::fade_to_fg(Color::Blue, Duration::from_millis(3000)).into_effect()),
            fire_effect: never_complete(fx::fade_from_fg(Color::Red, Duration::from_millis(2500)).into_effect()),
            avatar_pos: (10.0, 5.0),
            avatar_direction: (0.1, 0.05),
            avatar_color: Color::Cyan,
            background_hue: 0.0,
            chat_messages: VecDeque::new(),
            chat_input: String::new(),
            selected_model: 0,
            available_models: Vec::new(),
            last_tick: Instant::now(),
            input_mode: false,
            scroll_offset: 0,
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;

        match self.state {
            AppState::BootSequence => {
                self.boot_progress += dt as f64 * 0.15; // Adjust speed
                if self.boot_progress >= 1.0 {
                    self.boot_progress = 1.0;
                    // Transition to Matrix after 1 second delay
                    if self.boot_stage == 0 {
                        self.boot_stage = 1;
                    }
                }
                if self.boot_stage == 1 && self.boot_progress >= 1.0 {
                    self.state = AppState::MatrixTransition;
                    self.load_ollama_models();
                }
            }
            AppState::MatrixTransition => {
                // Auto-transition after Matrix effect
                if now.duration_since(self.last_tick) > StdDuration::from_secs(3) {
                    self.state = AppState::MainInterface;
                }
            }
            AppState::MainInterface => {
                // Update avatar animation
                self.avatar_pos.0 += self.avatar_direction.0 * dt * 20.0;
                self.avatar_pos.1 += self.avatar_direction.1 * dt * 20.0;

                // Bounce avatar off walls
                if self.avatar_pos.0 <= 1.0 || self.avatar_pos.0 >= 28.0 {
                    self.avatar_direction.0 *= -1.0;
                }
                if self.avatar_pos.1 <= 1.0 || self.avatar_pos.1 >= 18.0 {
                    self.avatar_direction.1 *= -1.0;
                }

                // Update background and avatar colors
                self.background_hue += dt * 30.0;
                if self.background_hue >= 360.0 {
                    self.background_hue = 0.0;
                }

                let hue = (self.background_hue + 180.0) % 360.0;
                self.avatar_color = Self::hsl_to_color(hue, 100.0, 50.0);
            }
        }
    }

    fn load_ollama_models(&mut self) {
        // Try to get Ollama models
        if let Ok(output) = Command::new("ollama")
            .args(&["list"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
        {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines().skip(1) {
                    // Skip header line
                    if !line.trim().is_empty() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 3 {
                            self.available_models.push(OllamaModel {
                                name: parts[0].to_string(),
                                size: parts[1].to_string(),
                                modified: parts[2..].join(" "),
                            });
                        }
                    }
                }
            }
        }

        // Add default models if none found
        if self.available_models.is_empty() {
            self.available_models = vec![
                OllamaModel {
                    name: "llama2".to_string(),
                    size: "7B".to_string(),
                    modified: "Available".to_string(),
                },
                OllamaModel {
                    name: "codellama".to_string(),
                    size: "13B".to_string(),
                    modified: "Available".to_string(),
                },
                OllamaModel {
                    name: "mistral".to_string(),
                    size: "7B".to_string(),
                    modified: "Available".to_string(),
                },
            ];
        }

        // Add welcome message
        self.chat_messages.push_back(ChatMessage {
            role: "System".to_string(),
            content: "🤖 TACTICAL AI ONLINE - Select a model and begin communication.".to_string(),
            timestamp: Instant::now(),
        });
    }

    fn send_message_to_ollama(&mut self, message: String) {
        if self.available_models.is_empty() {
            return;
        }

        let model = &self.available_models[self.selected_model];

        // Add user message
        self.chat_messages.push_back(ChatMessage {
            role: "User".to_string(),
            content: message.clone(),
            timestamp: Instant::now(),
        });

        // Send to Ollama (simplified - in real app you'd do this async)
        let response = self.call_ollama(&model.name, &message);
        
        self.chat_messages.push_back(ChatMessage {
            role: "AI".to_string(),
            content: response,
            timestamp: Instant::now(),
        });

        // Keep only last 50 messages
        while self.chat_messages.len() > 50 {
            self.chat_messages.pop_front();
        }
    }

    fn call_ollama(&self, model: &str, prompt: &str) -> String {
        // Try to call Ollama
        let payload = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false
        });

        if let Ok(output) = Command::new("curl")
            .args(&[
                "-X", "POST",
                "http://localhost:11434/api/generate",
                "-H", "Content-Type: application/json",
                "-d", &payload.to_string(),
                "--connect-timeout", "5",
                "--max-time", "30"
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
        {
            if output.status.success() {
                let response_str = String::from_utf8_lossy(&output.stdout);
                if let Ok(json) = serde_json::from_str::<Value>(&response_str) {
                    if let Some(response) = json.get("response").and_then(|r| r.as_str()) {
                        return response.to_string();
                    }
                }
            }
        }

        // Fallback responses if Ollama isn't available
        let responses = vec![
            "🔍 Analysis complete. What's your next move, Agent?",
            "⚡ Neural processing... I detect high tactical efficiency in your approach.",
            "🛡️ Stealth mode engaged. Your operational security is optimal.",
            "🎯 Target acquired. Shall we proceed with the mission?",
            "🔧 System diagnostics show all green. Ready for deployment.",
            "📡 Intercepted intel suggests multiple pathways. Choose wisely.",
            "🔥 Threat assessment: Minimal. You have the advantage.",
            "💭 My algorithms suggest a multi-vector approach to this challenge.",
        ];
        
        responses[prompt.len() % responses.len()].to_string()
    }

    fn hsl_to_color(h: f32, s: f32, l: f32) -> Color {
        let h = h / 360.0;
        let s = s / 100.0;
        let l = l / 100.0;

        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;

        let (r, g, b) = if h < 1.0 / 6.0 {
            (c, x, 0.0)
        } else if h < 2.0 / 6.0 {
            (x, c, 0.0)
        } else if h < 3.0 / 6.0 {
            (0.0, c, x)
        } else if h < 4.0 / 6.0 {
            (0.0, x, c)
        } else if h < 5.0 / 6.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Color::Rgb(
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
        )
    }
}

fn main() -> Result<()> {
    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut last_frame = Instant::now();

    loop {
        let elapsed = last_frame.elapsed();
        last_frame = Instant::now();
        app.update();

        terminal.draw(|f| ui(f, &mut app, elapsed.into()))?;

        if event::poll(StdDuration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.state {
                        AppState::BootSequence | AppState::MatrixTransition => {
                            if key.code == KeyCode::Esc {
                                break;
                            }
                            // Skip to main interface on any key
                            if key.code != KeyCode::Esc {
                                app.state = AppState::MainInterface;
                            }
                        }
                        AppState::MainInterface => {
                            if app.input_mode {
                                match key.code {
                                    KeyCode::Enter => {
                                        if !app.chat_input.trim().is_empty() {
                                            let message = app.chat_input.clone();
                                            app.chat_input.clear();
                                            app.send_message_to_ollama(message);
                                        }
                                    }
                                    KeyCode::Char(c) => {
                                        app.chat_input.push(c);
                                    }
                                    KeyCode::Backspace => {
                                        app.chat_input.pop();
                                    }
                                    KeyCode::Esc => {
                                        app.input_mode = false;
                                    }
                                    _ => {}
                                }
                            } else {
                                match key.code {
                                    KeyCode::Esc => break,
                                    KeyCode::Char('i') => app.input_mode = true,
                                    KeyCode::Up => {
                                        if app.selected_model > 0 {
                                            app.selected_model -= 1;
                                        }
                                    }
                                    KeyCode::Down => {
                                        if app.selected_model < app.available_models.len().saturating_sub(1) {
                                            app.selected_model += 1;
                                        }
                                    }
                                    KeyCode::PageUp => {
                                        app.scroll_offset = app.scroll_offset.saturating_sub(5);
                                    }
                                    KeyCode::PageDown => {
                                        app.scroll_offset = app.scroll_offset.saturating_add(5).min(app.chat_messages.len().saturating_sub(1));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, app: &mut App, duration: Duration) {
    match app.state {
        AppState::BootSequence => render_boot_sequence(f, app, duration),
        AppState::MatrixTransition => render_matrix_transition(f, app, duration),
        AppState::MainInterface => render_main_interface(f, app, duration),
    }
}

fn render_boot_sequence(f: &mut Frame, app: &mut App, duration: Duration) {
    let area = f.area();
    
    // Clear screen with black background
    Clear.render(area, f.buffer_mut());
    
    // Main container
    let block = Block::default()
        .style(Style::default().bg(Color::Black))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title("█ TACTICAL ESPIONAGE SYSTEM █")
        .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));
    
    f.render_widget(block, area);
    
    let inner = area.inner(Margin::new(2, 2));
    
    // Layout
    let chunks = Layout::vertical([
        Constraint::Length(3),  // Title
        Constraint::Length(3),  // Progress bar
        Constraint::Min(10),    // Messages
        Constraint::Length(3),  // Status
    ]).split(inner);
    
    // Classified header
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("█ CLASSIFIED █", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("MISSION: OPERATION DIGITAL SHADOW", Style::default().fg(Color::Yellow)),
        ]),
    ]).alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);
    
    // Progress bar
    let progress = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("BOOT PROGRESS"))
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(app.boot_progress)
        .label(format!("{}%", (app.boot_progress * 100.0) as u8));
    f.render_widget(progress, chunks[1]);
    
    // Boot messages
    let messages: Vec<ListItem> = app.boot_messages
        .iter()
        .enumerate()
        .take_while(|(i, _)| *i <= (app.boot_progress * app.boot_messages.len() as f64) as usize)
        .map(|(_, msg)| {
            ListItem::new(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(Color::Green)),
                Span::styled(msg, Style::default().fg(Color::White)),
            ]))
        })
        .collect();
    
    let messages_list = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("SYSTEM STATUS"));
    f.render_widget(messages_list, chunks[2]);
    
    // Status
    let status = if app.boot_progress >= 1.0 {
        "READY FOR DEPLOYMENT - PRESS ANY KEY TO CONTINUE"
    } else {
        "INITIALIZING SYSTEMS..."
    };
    
    let status_widget = Paragraph::new(status)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status_widget, chunks[3]);
    
    // Apply typewriter effect to messages
    if app.boot_progress < 1.0 {
        f.render_effect(&mut app.typewriter_effect, chunks[2], duration);
    }
}

fn render_matrix_transition(f: &mut Frame, app: &mut App, duration: Duration) {
    let area = f.area();
    
    // Fill with matrix effect
    let text = Paragraph::new("ENTERING THE MATRIX...\n\nWELCOME TO THE DIGITAL REALM\n\nPREPARING TACTICAL INTERFACE...")
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("NEURAL LINK ESTABLISHED"));
    
    f.render_widget(text, area);
    
    // Apply matrix rain effect
    f.render_effect(&mut app.matrix_effect, area, duration);
}

fn render_main_interface(f: &mut Frame, app: &mut App, duration: Duration) {
    let area = f.area();
    
    // Main layout: Left sidebar and right chat
    let main_chunks = Layout::horizontal([
        Constraint::Percentage(30),  // Avatar sidebar
        Constraint::Percentage(70),  // Chat area
    ]).split(area);
    
    render_avatar_sidebar(f, app, main_chunks[0], duration);
    render_chat_area(f, app, main_chunks[1], duration);
}

fn render_avatar_sidebar(f: &mut Frame, app: &mut App, area: Rect, duration: Duration) {
    // Background with changing colors
    let bg_color = App::hsl_to_color(app.background_hue, 30.0, 10.0);
    let block = Block::default()
        .style(Style::default().bg(bg_color))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title("█ AGENT STATUS █")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    
    f.render_widget(block, area);
    
    let inner = area.inner(Margin::new(1, 1));
    
    // Layout for avatar area and status
    let chunks = Layout::vertical([
        Constraint::Percentage(70),  // Avatar area
        Constraint::Percentage(30),  // Status info
    ]).split(inner);
    
    // Avatar area background
    let avatar_bg = Block::default()
        .style(Style::default().bg(App::hsl_to_color(app.background_hue + 60.0, 40.0, 15.0)))
        .borders(Borders::ALL)
        .title("OPERATIVE")
        .title_style(Style::default().fg(app.avatar_color));
    
    f.render_widget(avatar_bg, chunks[0]);
    
    // Draw moving avatar
    let avatar_inner = chunks[0].inner(Margin::new(1, 1));
    let avatar_x = (app.avatar_pos.0 as u16).min(avatar_inner.width.saturating_sub(3));
    let avatar_y = (app.avatar_pos.1 as u16).min(avatar_inner.height.saturating_sub(2));
    
    let avatar_area = Rect {
        x: avatar_inner.x + avatar_x,
        y: avatar_inner.y + avatar_y,
        width: 3,
        height: 2,
    };
    
    let avatar = Paragraph::new("█▀█\n█▄█")
        .style(Style::default().fg(app.avatar_color).add_modifier(Modifier::BOLD));
    f.render_widget(avatar, avatar_area);
    
    // Status info
    let status_text = vec![
        Line::from(vec![
            Span::styled("STATUS: ", Style::default().fg(Color::Yellow)),
            Span::styled("OPERATIONAL", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("HEALTH: ", Style::default().fg(Color::Yellow)),
            Span::styled("█████████░", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("STEALTH: ", Style::default().fg(Color::Yellow)),
            Span::styled("███████░░░", Style::default().fg(Color::Blue)),
        ]),
        Line::from(vec![
            Span::styled("ALERT: ", Style::default().fg(Color::Yellow)),
            Span::styled("CLEAR", Style::default().fg(Color::Green)),
        ]),
    ];
    
    let status_widget = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title("VITALS"));
    f.render_widget(status_widget, chunks[1]);
    
    // Apply wave effect to the sidebar
    f.render_effect(&mut app.wave_effect, area, duration);
}

fn render_chat_area(f: &mut Frame, app: &mut App, area: Rect, duration: Duration) {
    let chunks = Layout::vertical([
        Constraint::Length(3),   // Model selection
        Constraint::Min(10),     // Chat messages
        Constraint::Length(3),   // Input area
        Constraint::Length(2),   // Controls
    ]).split(area);
    
    // Model selection
    let models: Vec<ListItem> = app.available_models
        .iter()
        .enumerate()
        .map(|(i, model)| {
            let style = if i == app.selected_model {
                Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("🤖 {} ({})", model.name, model.size)).style(style)
        })
        .collect();
    
    let models_list = List::new(models)
        .block(Block::default().borders(Borders::ALL).title("AI MODELS - Use ↑↓ to select"));
    f.render_widget(models_list, chunks[0]);
    
    // Chat messages
    let messages: Vec<ListItem> = app.chat_messages
        .iter()
        .skip(app.scroll_offset)
        .map(|msg| {
            let (prefix, style) = match msg.role.as_str() {
                "User" => ("👤 YOU:", Style::default().fg(Color::Cyan)),
                "AI" => ("🤖 AI:", Style::default().fg(Color::Green)),
                _ => ("🔧 SYS:", Style::default().fg(Color::Yellow)),
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(&msg.content, Style::default().fg(Color::White)),
            ]))
        })
        .collect();
    
    let messages_list = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("TACTICAL COMMUNICATIONS"));
    f.render_widget(messages_list, chunks[1]);
    
    // Input area
    let input_style = if app.input_mode {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    
    let input_text = if app.input_mode {
        format!("▶ {}_", app.chat_input)
    } else {
        "Press 'i' to enter message...".to_string()
    };
    
    let input_widget = Paragraph::new(input_text)
        .style(input_style)
        .block(Block::default().borders(Borders::ALL).title("MESSAGE INPUT"));
    f.render_widget(input_widget, chunks[2]);
    
    // Controls
    let controls = Paragraph::new("ESC: Quit | i: Input | ↑↓: Select Model | PgUp/PgDn: Scroll | Enter: Send")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(controls, chunks[3]);
    
    // Apply subtle fire effect to chat area
    f.render_effect(&mut app.fire_effect, chunks[1], duration);
}