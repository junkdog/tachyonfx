use std::{
    collections::VecDeque,
    error::Error,
    io,
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
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Widget},
    Frame, Terminal,
};
use tachyonfx::{
    color_from_hsl,
    fx::{self, never_complete, parallel, sequence, with_duration},
    Duration, Effect, EffectRenderer, Interpolation, IntoEffect,
    Motion, Shader, SimpleRng,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Clone, PartialEq)]
enum AppState {
    BootSequence,
    ExplosionTransition,
    MainMenu,
    EffectShowcase,
    InteractivePlayground,
    MatrixMode,
    GameMode,
}

#[derive(Debug, Clone)]
struct GameEntity {
    pos: (f32, f32),
    vel: (f32, f32),
    color: Color,
    symbol: String,
    effect_timer: f32,
}

struct App {
    state: AppState,
    next_state: Option<AppState>,
    
    // Boot sequence
    boot_progress: f64,
    boot_stage: usize,
    boot_messages: Vec<String>,
    
    // Effects
    current_effect: Option<Effect>,
    effect_repository: EffectsRepository,
    active_effect_idx: usize,
    
    // Animation
    entities: Vec<GameEntity>,
    background_hue: f32,
    
    // Interactive elements
    input_buffer: String,
    menu_selection: usize,
    
    // Timing
    last_tick: Instant,
    state_timer: f32,
    
    // RNG for effects
    rng: SimpleRng,
    
    // DSL playground
    dsl_code: String,
    dsl_error: Option<String>,
    
    // Messages
    messages: VecDeque<String>,
}

impl App {
    fn new() -> Self {
        let boot_messages = vec![
            "█ INITIALIZING TACHYONFX MEGA DEMO █".to_string(),
            "Loading quantum effect matrices...".to_string(),
            "Calibrating visual processors...".to_string(),
            "Synchronizing temporal buffers...".to_string(),
            "Activating particle accelerators...".to_string(),
            "Establishing neural link...".to_string(),
            "Loading explosive subroutines...".to_string(),
            "Preparing interactive playground...".to_string(),
            "Compiling reality shaders...".to_string(),
            "SYSTEM READY - PREPARE FOR AWESOMENESS!".to_string(),
        ];

        let mut entities = Vec::new();
        for i in 0..10 {
            entities.push(GameEntity {
                pos: (10.0 + i as f32 * 3.0, 5.0 + i as f32),
                vel: (0.1 - i as f32 * 0.02, 0.05 + i as f32 * 0.01),
                color: Color::Cyan,
                symbol: "★".to_string(),
                effect_timer: i as f32 * 0.1,
            });
        }

        Self {
            state: AppState::BootSequence,
            next_state: None,
            boot_progress: 0.0,
            boot_stage: 0,
            boot_messages,
            current_effect: None,
            effect_repository: EffectsRepository::new(),
            active_effect_idx: 0,
            entities,
            background_hue: 0.0,
            input_buffer: String::new(),
            menu_selection: 0,
            last_tick: Instant::now(),
            state_timer: 0.0,
            rng: SimpleRng::default(),
            dsl_code: "fx::parallel(&[\n    fx::fade_from_fg(Color::Red, (1000, Interpolation::BounceOut)),\n    fx::explode(15.0, 0.8, (1200, Interpolation::CircOut))\n])".to_string(),
            dsl_error: None,
            messages: VecDeque::new(),
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;
        self.state_timer += dt;

        // Update background animation
        self.background_hue += dt * 30.0;
        if self.background_hue >= 360.0 {
            self.background_hue = 0.0;
        }

        // Update entities
        for entity in &mut self.entities {
            entity.pos.0 += entity.vel.0 * dt * 50.0;
            entity.pos.1 += entity.vel.1 * dt * 50.0;
            entity.effect_timer += dt;

            // Bounce off edges
            if entity.pos.0 <= 0.0 || entity.pos.0 >= 120.0 {
                entity.vel.0 *= -1.0;
            }
            if entity.pos.1 <= 0.0 || entity.pos.1 >= 30.0 {
                entity.vel.1 *= -1.0;
            }

            // Color cycling
            let hue = (self.background_hue + entity.effect_timer * 100.0) % 360.0;
            entity.color = hsl_to_color(hue, 80.0, 60.0);
        }

        match self.state {
            AppState::BootSequence => {
                self.boot_progress += dt as f64 * 0.2;
                if self.boot_progress >= 1.0 {
                    self.boot_progress = 1.0;
                    if self.state_timer > 2.0 {
                        self.transition_to(AppState::ExplosionTransition);
                    }
                }
            }
            AppState::ExplosionTransition => {
                if self.state_timer > 3.0 {
                    self.transition_to(AppState::MainMenu);
                }
            }
            AppState::MainMenu => {
                // Menu is ready for interaction
            }
            AppState::EffectShowcase => {
                // Auto-cycle effects every 3 seconds
                if self.state_timer > 3.0 {
                    self.next_effect();
                    self.state_timer = 0.0;
                }
            }
            AppState::MatrixMode => {
                // Add random matrix entities
                if self.rng.gen() % 100 < 5 {
                    self.entities.push(GameEntity {
                        pos: (self.rng.gen() as f32 % 120.0, 0.0),
                        vel: (0.0, 0.5 + (self.rng.gen() as f32 % 100.0) / 100.0),
                        color: Color::Green,
                        symbol: "01".chars().nth((self.rng.gen() % 2) as usize).unwrap().to_string(),
                        effect_timer: 0.0,
                    });
                }
                // Remove old entities
                self.entities.retain(|e| e.pos.1 < 35.0);
            }
            AppState::GameMode => {
                // Spawn new entities occasionally
                if self.rng.gen() % 200 < 3 {
                    self.entities.push(GameEntity {
                        pos: (self.rng.gen() as f32 % 120.0, self.rng.gen() as f32 % 30.0),
                        vel: ((self.rng.gen() as f32 % 200.0 - 100.0) / 100.0, (self.rng.gen() as f32 % 200.0 - 100.0) / 100.0),
                        color: hsl_to_color(self.rng.gen() as f32 % 360.0, 100.0, 70.0),
                        symbol: "◆◇●○★☆▲△".chars().nth((self.rng.gen() % 8) as usize).unwrap().to_string(),
                        effect_timer: 0.0,
                    });
                }
                // Limit entities
                if self.entities.len() > 50 {
                    self.entities.remove(0);
                }
            }
            _ => {}
        }

        // Check for state transition
        if let Some(next_state) = self.next_state.take() {
            self.state = next_state;
            self.state_timer = 0.0;
            self.setup_state_effects();
        }
    }

    fn transition_to(&mut self, state: AppState) {
        self.next_state = Some(state);
    }

    fn setup_state_effects(&mut self) {
        match self.state {
            AppState::BootSequence => {
                self.current_effect = Some(
                    sequence(&[
                        fx::fade_from_fg(Color::Black, (1000, Interpolation::QuadOut)),
                        fx::coalesce((1000, Interpolation::BounceOut)),
                    ]).into_effect()
                );
            }
            AppState::ExplosionTransition => {
                self.current_effect = Some(
                    parallel(&[
                        fx::explode(25.0, 0.9, (2000, Interpolation::CircOut)),
                        fx::hsl_shift_fg([180.0, 50.0, 30.0], (2000, Interpolation::BounceOut)),
                        sequence(&[
                            fx::fade_to_fg(Color::Red, (500, Interpolation::QuadIn)),
                            fx::fade_from_fg(Color::Black, (1500, Interpolation::QuadOut)),
                        ]),
                    ]).into_effect()
                );
            }
            AppState::MainMenu => {
                self.current_effect = Some(
                    never_complete(
                        parallel(&[
                            fx::coalesce((2000, Interpolation::SineInOut)),
                            fx::hsl_shift_fg([360.0, 0.0, 0.0], Duration::from_millis(4000)),
                        ]).into_effect()
                    )
                );
            }
            AppState::EffectShowcase => {
                self.next_effect();
            }
            AppState::MatrixMode => {
                self.current_effect = Some(
                    never_complete(
                        parallel(&[
                            fx::sweep_in(Motion::UpToDown, 2, 10, Color::Black, (2000, Interpolation::Linear)),
                            fx::hsl_shift_fg([120.0, 0.0, 0.0], Duration::from_millis(3000)),
                        ]).into_effect()
                    )
                );
            }
            AppState::GameMode => {
                self.current_effect = Some(
                    never_complete(
                        parallel(&[
                            fx::explode(15.0, 0.3, (3000, Interpolation::CircOut)),
                            fx::hsl_shift_fg([180.0, 25.0, 0.0], (2500, Interpolation::SineInOut)),
                        ]).into_effect()
                    )
                );
            }
            _ => {
                self.current_effect = None;
            }
        }
    }

    fn next_effect(&mut self) {
        self.active_effect_idx = (self.active_effect_idx + 1) % self.effect_repository.len();
        let (name, effect) = self.effect_repository.get_effect(self.active_effect_idx);
        self.current_effect = Some(effect);
        self.messages.push_back(format!("🎨 Now showing: {}", name));
        if self.messages.len() > 10 {
            self.messages.pop_front();
        }
    }

    fn handle_input(&mut self, key: KeyCode) {
        match self.state {
            AppState::BootSequence | AppState::ExplosionTransition => {
                // Skip transitions with any key
                match key {
                    KeyCode::Esc => {} // Will be handled by main loop
                    _ => self.transition_to(AppState::MainMenu),
                }
            }
            AppState::MainMenu => {
                match key {
                    KeyCode::Up => {
                        if self.menu_selection > 0 {
                            self.menu_selection -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.menu_selection < 4 {
                            self.menu_selection += 1;
                        }
                    }
                    KeyCode::Enter => {
                        match self.menu_selection {
                            0 => self.transition_to(AppState::EffectShowcase),
                            1 => self.transition_to(AppState::InteractivePlayground),
                            2 => self.transition_to(AppState::MatrixMode),
                            3 => self.transition_to(AppState::GameMode),
                            4 => {}, // Exit - handled by main loop
                            _ => {}
                        }
                    }
                    KeyCode::Char('1') => self.transition_to(AppState::EffectShowcase),
                    KeyCode::Char('2') => self.transition_to(AppState::InteractivePlayground),
                    KeyCode::Char('3') => self.transition_to(AppState::MatrixMode),
                    KeyCode::Char('4') => self.transition_to(AppState::GameMode),
                    _ => {}
                }
            }
            AppState::EffectShowcase => {
                match key {
                    KeyCode::Left | KeyCode::Backspace => {
                        self.active_effect_idx = if self.active_effect_idx == 0 {
                            self.effect_repository.len() - 1
                        } else {
                            self.active_effect_idx - 1
                        };
                        self.next_effect();
                    }
                    KeyCode::Right | KeyCode::Enter => {
                        self.next_effect();
                    }
                    KeyCode::Char('r') => {
                        let fx_idx = (self.rng.gen() % self.effect_repository.len() as u32) as usize;
                        self.active_effect_idx = fx_idx;
                        self.next_effect();
                    }
                    KeyCode::Char('m') => self.transition_to(AppState::MainMenu),
                    _ => {}
                }
            }
            AppState::InteractivePlayground => {
                match key {
                    KeyCode::Char(c) => {
                        self.input_buffer.push(c);
                    }
                    KeyCode::Backspace => {
                        self.input_buffer.pop();
                    }
                    KeyCode::Enter => {
                        if !self.input_buffer.is_empty() {
                            self.messages.push_back(format!("💬 {}", self.input_buffer));
                            // Trigger random effect based on input
                            let hash = self.input_buffer.len() % self.effect_repository.len();
                            self.active_effect_idx = hash;
                            self.next_effect();
                            self.input_buffer.clear();
                        }
                    }
                    KeyCode::Char('m') => self.transition_to(AppState::MainMenu),
                    _ => {}
                }
            }
            AppState::MatrixMode => {
                match key {
                    KeyCode::Char(' ') => {
                        // Add explosion
                        self.current_effect = Some(
                            with_duration(
                                Duration::from_millis(2000),
                                parallel(&[
                                    fx::explode(20.0, 0.7, (1500, Interpolation::CircOut)),
                                    fx::sweep_in(Motion::UpToDown, 2, 10, Color::Black, (2000, Interpolation::Linear)),
                                ]).into_effect()
                            )
                        );
                    }
                    KeyCode::Char('m') => self.transition_to(AppState::MainMenu),
                    _ => {}
                }
            }
            AppState::GameMode => {
                match key {
                    KeyCode::Char(' ') => {
                        // Clear entities with explosion
                        self.entities.clear();
                        self.current_effect = Some(
                            with_duration(
                                Duration::from_millis(1500),
                                parallel(&[
                                    fx::explode(30.0, 1.0, (1000, Interpolation::BounceOut)),
                                    fx::hsl_shift_fg([360.0, 50.0, 25.0], (800, Interpolation::CircOut)),
                                    fx::fade_to_fg(Color::White, (500, Interpolation::QuadIn)),
                                ]).into_effect()
                            )
                        );
                    }
                    KeyCode::Char('m') => self.transition_to(AppState::MainMenu),
                    _ => {}
                }
            }
        }
    }
}

struct EffectsRepository {
    effects: Vec<(&'static str, Effect)>,
}

impl EffectsRepository {
    fn new() -> Self {
        let medium = Duration::from_millis(750);
        let fast = Duration::from_millis(500);

        let effects = vec![
            (
                "🌊 Wave Sweep",
                fx::sweep_in(Motion::LeftToRight, 20, 5, Color::Blue, (medium, Interpolation::QuadOut)),
            ),
            (
                "💥 Explosion",
                fx::explode(20.0, 0.8, (medium, Interpolation::CircOut)),
            ),
            (
                "🌈 Color Cycle",
                fx::hsl_shift_fg([360.0, 0.0, 0.0], medium),
            ),
            (
                "⚡ Dissolve Magic",
                fx::dissolve((fast, Interpolation::BounceOut)),
            ),
            (
                "🔥 Fade Fire",
                parallel(&[
                    fx::fade_to_fg(Color::Red, (fast, Interpolation::QuadIn)),
                    fx::fade_from_fg(Color::Yellow, (fast, Interpolation::QuadOut)),
                ]),
            ),
            (
                "📡 Sweep Matrix",
                parallel(&[
                    fx::sweep_out(Motion::UpToDown, 3, 8, Color::Green, (medium, Interpolation::Linear)),
                    fx::hsl_shift_fg([120.0, 0.0, 0.0], medium),
                ]),
            ),
            (
                "🌀 Coalesce",
                fx::coalesce((medium, Interpolation::BounceOut)),
            ),
            (
                "💫 Dissolve",
                fx::dissolve((medium, Interpolation::CircOut)),
            ),
            (
                "📈 Slide In",
                fx::slide_in(Motion::UpToDown, 15, 0, Color::Black, (medium, Interpolation::QuadOut)),
            ),
            (
                "🎭 Fade Magic",
                parallel(&[
                    fx::fade_to_fg(Color::Red, (fast, Interpolation::QuadIn)),
                    fx::fade_from_fg(Color::Green, (fast, Interpolation::QuadOut)),
                ]),
            ),
        ];

        Self { effects }
    }

    fn get_effect(&self, idx: usize) -> (&'static str, Effect) {
        self.effects[idx].clone()
    }

    fn len(&self) -> usize {
        self.effects.len()
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
    app.setup_state_effects();
    let mut last_frame = Instant::now();

    loop {
        let elapsed = last_frame.elapsed();
        last_frame = Instant::now();
        app.update();

        terminal.draw(|f| ui(f, &mut app, elapsed.into()))?;

        if event::poll(StdDuration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => break,
                        _ => app.handle_input(key.code),
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
        AppState::ExplosionTransition => render_explosion_transition(f, app, duration),
        AppState::MainMenu => render_main_menu(f, app, duration),
        AppState::EffectShowcase => render_effect_showcase(f, app, duration),
        AppState::InteractivePlayground => render_interactive_playground(f, app, duration),
        AppState::MatrixMode => render_matrix_mode(f, app, duration),
        AppState::GameMode => render_game_mode(f, app, duration),
    }
}

fn render_boot_sequence(f: &mut Frame, app: &mut App, duration: Duration) {
    let area = f.area();
    
    // Animated background
    let bg_color = hsl_to_color(app.background_hue, 20.0, 5.0);
    Clear.render(area, f.buffer_mut());
    Block::default()
        .style(Style::default().bg(bg_color))
        .render(area, f.buffer_mut());

    // Main container with glowing border
    let border_color = hsl_to_color(app.background_hue + 120.0, 80.0, 60.0);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
        .title("█▓▒░ TACHYONFX MEGA DEMO INITIALIZING ░▒▓█")
        .title_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
    
    f.render_widget(block, area);
    
    let inner = area.inner(Margin::new(3, 2));
    let chunks = Layout::vertical([
        Constraint::Length(5),   // Title art
        Constraint::Length(3),   // Progress bar
        Constraint::Min(8),      // Boot messages
        Constraint::Length(4),   // Status
    ]).split(inner);
    
    // ASCII art title
    let title_art = vec![
        Line::from("╔══════════════════════════════════════════════════════════════╗").style(Style::default().fg(Color::Cyan)),
        Line::from("║  ████████  █████   █████████ █████   █████ █████   █████    ║").style(Style::default().fg(Color::Yellow)),
        Line::from("║ ███░░░░███░░███   ███░░░░░███░░███   ░░███ ░░███   ░░███     ║").style(Style::default().fg(Color::Yellow)),
        Line::from("║░███   ░░░  ░███  ███     ░░░  ░███    ░███  ░███    ░███     ║").style(Style::default().fg(Color::Yellow)),
        Line::from("╚══════════════════════════════════════════════════════════════╝").style(Style::default().fg(Color::Cyan)),
    ];
    
    let title_widget = Paragraph::new(title_art)
        .alignment(Alignment::Center);
    f.render_widget(title_widget, chunks[0]);
    
    // Animated progress bar
    let progress_color = hsl_to_color((app.background_hue + 180.0) % 360.0, 90.0, 70.0);
    let progress = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("⚡ QUANTUM LOADING ⚡"))
        .gauge_style(Style::default().fg(progress_color).add_modifier(Modifier::BOLD))
        .ratio(app.boot_progress)
        .label(format!("{}% READY FOR AWESOMENESS", (app.boot_progress * 100.0) as u8));
    f.render_widget(progress, chunks[1]);
    
    // Boot messages with effects
    let message_count = (app.boot_progress * app.boot_messages.len() as f64) as usize;
    let messages: Vec<ListItem> = app.boot_messages
        .iter()
        .take(message_count + 1)
        .enumerate()
        .map(|(i, msg)| {
            let color = hsl_to_color((i as f32 * 30.0 + app.background_hue) % 360.0, 70.0, 80.0);
            ListItem::new(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(msg, Style::default().fg(color)),
            ]))
        })
        .collect();
    
    let messages_list = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("🚀 SYSTEM INITIALIZATION"));
    f.render_widget(messages_list, chunks[2]);
    
    // Status with animations
    let status_text = if app.boot_progress >= 1.0 {
        vec![
            Line::from("🎆 INITIALIZATION COMPLETE! 🎆").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::from("Press ANY KEY to begin the EPIC JOURNEY!").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from("Or just wait and enjoy the show...").style(Style::default().fg(Color::Gray)),
        ]
    } else {
        vec![
            Line::from("⚡ Charging particle accelerators...").style(Style::default().fg(Color::Blue)),
            Line::from("🌟 Calibrating reality matrices...").style(Style::default().fg(Color::Magenta)),
            Line::from("🔥 Igniting awesome engines...").style(Style::default().fg(Color::Red)),
        ]
    };
    
    let status_widget = Paragraph::new(status_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("🎮 STATUS"));
    f.render_widget(status_widget, chunks[3]);
    
    // Apply cool effects
    if let Some(effect) = &mut app.current_effect {
        if effect.running() {
            f.render_effect(effect, area, duration);
        }
    }
    
    // Render animated entities
    render_entities(f, &app.entities, area);
}

fn render_explosion_transition(f: &mut Frame, app: &mut App, duration: Duration) {
    let area = f.area();
    
    // Epic explosion background
    let explosion_colors = [Color::Red, Color::Yellow, Color::Rgb(255, 165, 0), Color::White];
    let color_idx = ((app.state_timer * 10.0) as usize) % explosion_colors.len();
    let bg_color = explosion_colors[color_idx];
    
    Clear.render(area, f.buffer_mut());
    Block::default()
        .style(Style::default().bg(bg_color))
        .render(area, f.buffer_mut());
    
    // Explosion text
    let explosion_text = vec![
        Line::from("").alignment(Alignment::Center),
        Line::from("██████╗  ██████╗  ██████╗ ███╗   ███╗").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)).alignment(Alignment::Center),
        Line::from("██╔══██╗██╔═══██╗██╔═══██╗████╗ ████║").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)).alignment(Alignment::Center),
        Line::from("██████╔╝██║   ██║██║   ██║██╔████╔██║").style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)).alignment(Alignment::Center),
        Line::from("██╔══██╗██║   ██║██║   ██║██║╚██╔╝██║").style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)).alignment(Alignment::Center),
        Line::from("██████╔╝╚██████╔╝╚██████╔╝██║ ╚═╝ ██║").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)).alignment(Alignment::Center),
        Line::from("╚═════╝  ╚═════╝  ╚═════╝ ╚═╝     ╚═╝").style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)).alignment(Alignment::Center),
        Line::from("").alignment(Alignment::Center),
        Line::from("💥💥💥 PREPARE FOR MAXIMUM AWESOME! 💥💥💥").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)).alignment(Alignment::Center),
    ];
    
    let explosion_widget = Paragraph::new(explosion_text)
        .alignment(Alignment::Center);
    f.render_widget(explosion_widget, area);
    
    // Apply explosion effect
    if let Some(effect) = &mut app.current_effect {
        if effect.running() {
            f.render_effect(effect, area, duration);
        }
    }
    
    // Render exploding entities
    render_entities(f, &app.entities, area);
}

fn render_main_menu(f: &mut Frame, app: &mut App, duration: Duration) {
    let area = f.area();
    
    // Animated background
    let bg_color = hsl_to_color(app.background_hue, 15.0, 8.0);
    Clear.render(area, f.buffer_mut());
    Block::default()
        .style(Style::default().bg(bg_color))
        .render(area, f.buffer_mut());
    
    let chunks = Layout::vertical([
        Constraint::Length(8),   // Title
        Constraint::Min(12),     // Menu
        Constraint::Length(6),   // Instructions
    ]).split(area.inner(Margin::new(5, 2)));
    
    // Animated title
    let title_color = hsl_to_color((app.background_hue + 60.0) % 360.0, 80.0, 70.0);
    let title = vec![
        Line::from("╔════════════════════════════════════════════════════════════════╗").style(Style::default().fg(title_color)),
        Line::from("║                    🎆 TACHYONFX MEGA DEMO 🎆                   ║").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Line::from("║                     ★ INTERACTIVE PLAYGROUND ★                ║").style(Style::default().fg(Color::Yellow)),
        Line::from("║                                                                ║").style(Style::default().fg(title_color)),
        Line::from("║           🚀 Choose your adventure, brave explorer! 🚀         ║").style(Style::default().fg(Color::Cyan)),
        Line::from("╚════════════════════════════════════════════════════════════════╝").style(Style::default().fg(title_color)),
    ];
    
    let title_widget = Paragraph::new(title)
        .alignment(Alignment::Center);
    f.render_widget(title_widget, chunks[0]);
    
    // Menu options
    let menu_items = vec![
        "🎨 Effect Showcase - Watch amazing visual effects!",
        "🛝 Interactive Playground - Type and create magic!",
        "🌧️ Matrix Mode - Enter the digital rain!",
        "🎮 Game Mode - Particle playground with explosions!",
        "🚪 Exit - Return to reality...",
    ];
    
    let menu_list: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.menu_selection {
                let highlight_color = hsl_to_color((app.background_hue + 180.0) % 360.0, 90.0, 80.0);
                Style::default().bg(highlight_color).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("{}. {}", i + 1, item)).style(style)
        })
        .collect();
    
    let menu_widget = List::new(menu_list)
        .block(Block::default()
            .borders(Borders::ALL)
            .title("🎯 CHOOSE YOUR ADVENTURE")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    f.render_widget(menu_widget, chunks[1]);
    
    // Instructions
    let instructions = vec![
        Line::from("🎹 CONTROLS:").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Line::from("   ↑↓ Navigate menu  |  Enter/1-5 Select  |  ESC Exit").style(Style::default().fg(Color::Gray)),
        Line::from(""),
        Line::from("💡 Each mode has its own special effects and interactions!").style(Style::default().fg(Color::Yellow)),
        Line::from("   Get ready for an awesome visual experience! 🌟").style(Style::default().fg(Color::Cyan)),
    ];
    
    let instructions_widget = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL).title("📖 GUIDE"))
        .alignment(Alignment::Left);
    f.render_widget(instructions_widget, chunks[2]);
    
    // Apply wave effect
    if let Some(effect) = &mut app.current_effect {
        if effect.running() {
            f.render_effect(effect, area, duration);
        }
    }
    
    // Render floating entities
    render_entities(f, &app.entities, area);
}

fn render_effect_showcase(f: &mut Frame, app: &mut App, duration: Duration) {
    let area = f.area();
    
    // Background
    let bg_color = hsl_to_color(app.background_hue, 25.0, 12.0);
    Clear.render(area, f.buffer_mut());
    Block::default()
        .style(Style::default().bg(bg_color))
        .render(area, f.buffer_mut());
    
    let chunks = Layout::vertical([
        Constraint::Length(4),   // Title and current effect
        Constraint::Min(15),     // Effect area
        Constraint::Length(8),   // Messages and controls
    ]).split(area.inner(Margin::new(2, 1)));
    
    // Title
    let (effect_name, _) = app.effect_repository.get_effect(app.active_effect_idx);
    let title = vec![
        Line::from("🎬 EFFECT SHOWCASE 🎬").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)).alignment(Alignment::Center),
        Line::from("").alignment(Alignment::Center),
        Line::from(format!("Currently showing: {}", effect_name)).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)).alignment(Alignment::Center),
    ];
    
    let title_widget = Paragraph::new(title)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title_widget, chunks[0]);
    
    // Effect demonstration area
    let demo_text = Text::from(vec![
        Line::from("You never know what is enough unless").alignment(Alignment::Center),
        Line::from("you know what is more than enough").alignment(Alignment::Center),
        Line::from("").alignment(Alignment::Center),
        Line::from("— William Blake, Proverbs of Hell").style(Style::default().fg(Color::Yellow)).alignment(Alignment::Right),
        Line::from("").alignment(Alignment::Center),
        Line::from("🌟 WATCH THE MAGIC HAPPEN! 🌟").style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)).alignment(Alignment::Center),
        Line::from("Effects are automatically cycling every 3 seconds").style(Style::default().fg(Color::Gray)).alignment(Alignment::Center),
        Line::from("Use arrow keys to navigate manually!").style(Style::default().fg(Color::Gray)).alignment(Alignment::Center),
    ]);
    
    let demo_area = chunks[1].inner(Margin::new(1, 1));
    f.render_widget(demo_text, demo_area);
    
    // Apply current effect to the demo area
    if let Some(effect) = &mut app.current_effect {
        if effect.running() {
            f.render_effect(effect, demo_area, duration);
        }
    }
    
    // Messages and controls
    let control_chunks = Layout::horizontal([
        Constraint::Percentage(60),  // Messages
        Constraint::Percentage(40),  // Controls
    ]).split(chunks[2]);
    
    // Recent messages
    let messages: Vec<ListItem> = app.messages
        .iter()
        .rev()
        .take(5)
        .map(|msg| ListItem::new(msg.clone()))
        .collect();
    
    let messages_widget = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("📢 MESSAGES"));
    f.render_widget(messages_widget, control_chunks[0]);
    
    // Controls
    let controls = vec![
        Line::from("🎮 CONTROLS:").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Line::from("←→ Change effect").style(Style::default().fg(Color::White)),
        Line::from("R  Random effect").style(Style::default().fg(Color::White)),
        Line::from("M  Back to menu").style(Style::default().fg(Color::White)),
        Line::from("ESC Exit").style(Style::default().fg(Color::White)),
    ];
    
    let controls_widget = Paragraph::new(controls)
        .block(Block::default().borders(Borders::ALL).title("🎹 KEYS"));
    f.render_widget(controls_widget, control_chunks[1]);
}

fn render_interactive_playground(f: &mut Frame, app: &mut App, duration: Duration) {
    let area = f.area();
    
    // Dynamic background
    let bg_color = hsl_to_color(app.background_hue, 30.0, 10.0);
    Clear.render(area, f.buffer_mut());
    Block::default()
        .style(Style::default().bg(bg_color))
        .render(area, f.buffer_mut());
    
    let chunks = Layout::vertical([
        Constraint::Length(3),   // Title
        Constraint::Min(12),     // Interactive area
        Constraint::Length(3),   // Input
        Constraint::Length(6),   // Messages and controls
    ]).split(area.inner(Margin::new(2, 1)));
    
    // Title
    let title = Paragraph::new("🛝 INTERACTIVE PLAYGROUND - Type anything to trigger effects! 🛝")
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);
    
    // Interactive content area
    let content = Text::from(vec![
        Line::from("╔══════════════════════════════════════════════════════════════╗").style(Style::default().fg(Color::Cyan)),
        Line::from("║                    🌟 MAGIC HAPPENS HERE 🌟                 ║").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Line::from("║                                                              ║").style(Style::default().fg(Color::Cyan)),
        Line::from("║  Type anything in the input box below and press Enter!      ║").style(Style::default().fg(Color::Yellow)),
        Line::from("║  Your words will trigger amazing visual effects!            ║").style(Style::default().fg(Color::Yellow)),
        Line::from("║                                                              ║").style(Style::default().fg(Color::Cyan)),
        Line::from("║              Different words = Different magic! ✨           ║").style(Style::default().fg(Color::Magenta)),
        Line::from("║                                                              ║").style(Style::default().fg(Color::Cyan)),
        Line::from("╚══════════════════════════════════════════════════════════════╝").style(Style::default().fg(Color::Cyan)),
    ]);
    
    let content_area = chunks[1].inner(Margin::new(1, 1));
    f.render_widget(content, content_area);
    
    // Input box
    let input_text = format!("💬 Type here: {}_", app.input_buffer);
    let input_widget = Paragraph::new(input_text)
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("✏️ INPUT"));
    f.render_widget(input_widget, chunks[2]);
    
    // Messages and controls split
    let bottom_chunks = Layout::horizontal([
        Constraint::Percentage(60),  // Messages
        Constraint::Percentage(40),  // Controls
    ]).split(chunks[3]);
    
    // Messages
    let messages: Vec<ListItem> = app.messages
        .iter()
        .rev()
        .take(4)
        .map(|msg| ListItem::new(msg.clone()))
        .collect();
    
    let messages_widget = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("💭 CHAT"));
    f.render_widget(messages_widget, bottom_chunks[0]);
    
    // Controls
    let controls = vec![
        Line::from("🎮 CONTROLS:").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Line::from("Type + Enter = Magic!").style(Style::default().fg(Color::White)),
        Line::from("M = Back to menu").style(Style::default().fg(Color::White)),
        Line::from("ESC = Exit").style(Style::default().fg(Color::White)),
    ];
    
    let controls_widget = Paragraph::new(controls)
        .block(Block::default().borders(Borders::ALL).title("🎹 KEYS"));
    f.render_widget(controls_widget, bottom_chunks[1]);
    
    // Apply current effect
    if let Some(effect) = &mut app.current_effect {
        if effect.running() {
            f.render_effect(effect, content_area, duration);
        }
    }
    
    // Render interactive entities
    render_entities(f, &app.entities, area);
}

fn render_matrix_mode(f: &mut Frame, app: &mut App, duration: Duration) {
    let area = f.area();
    
    // Matrix background
    Clear.render(area, f.buffer_mut());
    Block::default()
        .style(Style::default().bg(Color::Black))
        .render(area, f.buffer_mut());
    
    // Matrix title
    let title = vec![
        Line::from("🌧️ WELCOME TO THE MATRIX 🌧️").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)).alignment(Alignment::Center),
        Line::from("There is no spoon... only awesome effects!").style(Style::default().fg(Color::Gray)).alignment(Alignment::Center),
        Line::from("Press SPACE for explosions, M for menu").style(Style::default().fg(Color::Gray)).alignment(Alignment::Center),
    ];
    
    let title_area = Rect::new(0, 0, area.width, 4);
    let title_widget = Paragraph::new(title);
    f.render_widget(title_widget, title_area);
    
    // Apply matrix effect to the whole area
    if let Some(effect) = &mut app.current_effect {
        if effect.running() {
            f.render_effect(effect, area, duration);
        }
    }
    
    // Render matrix entities (falling characters)
    for entity in &app.entities {
        if entity.pos.0 >= 0.0 && entity.pos.0 < area.width as f32 
            && entity.pos.1 >= 0.0 && entity.pos.1 < area.height as f32 {
            let x = entity.pos.0 as u16;
            let y = entity.pos.1 as u16;
            
            if x < area.width && y < area.height {
                let char_area = Rect::new(x, y, 1, 1);
                let char_widget = Paragraph::new(entity.symbol.as_str())
                    .style(Style::default().fg(entity.color));
                f.render_widget(char_widget, char_area);
            }
        }
    }
}

fn render_game_mode(f: &mut Frame, app: &mut App, duration: Duration) {
    let area = f.area();
    
    // Game background with fire effects
    let bg_color = hsl_to_color(app.background_hue, 40.0, 15.0);
    Clear.render(area, f.buffer_mut());
    Block::default()
        .style(Style::default().bg(bg_color))
        .render(area, f.buffer_mut());
    
    // Game title and instructions
    let title = vec![
        Line::from("🎮 PARTICLE PLAYGROUND 🎮").style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)).alignment(Alignment::Center),
        Line::from("Watch the particles dance! Press SPACE to clear with explosions!").style(Style::default().fg(Color::Yellow)).alignment(Alignment::Center),
        Line::from("M = Menu | ESC = Exit").style(Style::default().fg(Color::Gray)).alignment(Alignment::Center),
    ];
    
    let title_area = Rect::new(0, 0, area.width, 4);
    let title_widget = Paragraph::new(title);
    f.render_widget(title_widget, title_area);
    
    // Apply fire and lightning effects
    if let Some(effect) = &mut app.current_effect {
        if effect.running() {
            f.render_effect(effect, area, duration);
        }
    }
    
    // Render all the game entities
    render_entities(f, &app.entities, area);
}

fn render_entities(f: &mut Frame, entities: &[GameEntity], area: Rect) {
    for entity in entities {
        if entity.pos.0 >= 0.0 && entity.pos.0 < area.width as f32 
            && entity.pos.1 >= 0.0 && entity.pos.1 < area.height as f32 {
            let x = entity.pos.0 as u16;
            let y = entity.pos.1 as u16;
            
            if x < area.width && y < area.height {
                let entity_area = Rect::new(area.x + x, area.y + y, 1, 1);
                let entity_widget = Paragraph::new(entity.symbol.as_str())
                    .style(Style::default().fg(entity.color).add_modifier(Modifier::BOLD));
                f.render_widget(entity_widget, entity_area);
            }
        }
    }
}

fn hsl_to_color(h: f32, s: f32, l: f32) -> Color {
    color_from_hsl(h, s, l)
}