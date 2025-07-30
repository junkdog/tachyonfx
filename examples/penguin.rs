use std::{
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
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
    Frame, Terminal,
};
use tachyonfx::{
    fx,
    Duration, Effect, EffectRenderer, Interpolation, IntoEffect,
    Motion, Shader, SimpleRng,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Clone, PartialEq)]
enum PenguinState {
    Idle,
    Walking,
    Jumping,
    Sliding,
    ReleasingBalloon,
    Dancing,
    Fishing,
}

#[derive(Debug, Clone, PartialEq)]
enum Direction {
    Up,
    Down, 
    Left,
    Right,
}

#[derive(Debug, Clone)]
struct Penguin {
    x: f32,
    y: f32,
    state: PenguinState,
    direction: Direction,
    animation_frame: usize,
    animation_timer: f32,
    jump_height: f32,
    slide_momentum: f32,
    happiness: u8, // 0-100
}

#[derive(Debug, Clone)]
struct Balloon {
    x: f32,
    y: f32,
    color: Color,
    age: f32,
    rising_speed: f32,
    sway: f32,
}

#[derive(Debug, Clone)]
struct Snowflake {
    x: f32,
    y: f32,
    speed: f32,
    size: char,
}

#[derive(Debug, Clone)]
struct Fish {
    x: f32,
    y: f32,
    caught: bool,
    animation_timer: f32,
}

#[derive(Debug, Clone)]
struct Iceberg {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    style: IcebergStyle,
}

#[derive(Debug, Clone)]
enum IcebergStyle {
    Small,
    Medium,
    Large,
    Platform,
}

struct World {
    width: u16,
    height: u16,
    icebergs: Vec<Iceberg>,
    water_level: u16,
    snow_depth: Vec<u8>, // Snow accumulation per column
}

struct Game {
    penguin: Penguin,
    world: World,
    balloons: Vec<Balloon>,
    snowflakes: Vec<Snowflake>,
    fish: Vec<Fish>,
    
    // Visual effects
    current_effects: Vec<Effect>,
    background_hue: f32,
    water_animation: f32,
    
    // Game state
    score: u32,
    time_played: f32,
    last_tick: Instant,
    
    // Input state
    keys_pressed: std::collections::HashSet<KeyCode>,
    
    // RNG
    rng: SimpleRng,
    
    // Camera
    camera_x: f32,
    camera_y: f32,
}

impl Penguin {
    fn new() -> Self {
        Self {
            x: 60.0,
            y: 20.0,
            state: PenguinState::Idle,
            direction: Direction::Right,
            animation_frame: 0,
            animation_timer: 0.0,
            jump_height: 0.0,
            slide_momentum: 0.0,
            happiness: 100,
        }
    }

    fn get_sprite(&self) -> (&str, Color) {
        match (&self.state, self.animation_frame % 4, &self.direction) {
            // Idle penguin - blinking occasionally
            (PenguinState::Idle, 0..=2, _) => ("🐧", Color::White),
            (PenguinState::Idle, 3, _) => ("😴", Color::White), // Sleepy blink
            
            // Walking animation
            (PenguinState::Walking, 0, Direction::Right) => ("🐧", Color::White),
            (PenguinState::Walking, 1, Direction::Right) => ("🚶", Color::White),
            (PenguinState::Walking, 2, Direction::Right) => ("🐧", Color::White),
            (PenguinState::Walking, 3, Direction::Right) => ("🏃", Color::White),
            
            (PenguinState::Walking, 0, Direction::Left) => ("🐧", Color::White),
            (PenguinState::Walking, 1, Direction::Left) => ("🚶", Color::White),
            (PenguinState::Walking, 2, Direction::Left) => ("🐧", Color::White),
            (PenguinState::Walking, 3, Direction::Left) => ("🏃", Color::White),
            
            // Jumping
            (PenguinState::Jumping, _, _) => {
                if self.jump_height > 0.5 {
                    ("🤸", Color::Yellow) // Mid-air
                } else {
                    ("🐧", Color::White) // Landing
                }
            }
            
            // Sliding
            (PenguinState::Sliding, _, _) => ("🛷", Color::Cyan),
            
            // Releasing balloon
            (PenguinState::ReleasingBalloon, _, _) => ("🎈", Color::Red),
            
            // Dancing
            (PenguinState::Dancing, 0, _) => ("💃", Color::Magenta),
            (PenguinState::Dancing, 1, _) => ("🕺", Color::Magenta),
            (PenguinState::Dancing, 2, _) => ("🐧", Color::Magenta),
            (PenguinState::Dancing, 3, _) => ("🎭", Color::Magenta),
            
            // Fishing
            (PenguinState::Fishing, _, _) => ("🎣", Color::Blue),
            
            _ => ("🐧", Color::White),
        }
    }

    fn update(&mut self, dt: f32, world: &World) {
        self.animation_timer += dt;
        
        // Update animation frame every 0.3 seconds
        if self.animation_timer >= 0.3 {
            self.animation_timer = 0.0;
            self.animation_frame = (self.animation_frame + 1) % 4;
        }

        match self.state {
            PenguinState::Jumping => {
                if self.jump_height > 0.0 {
                    self.jump_height -= dt * 8.0; // Gravity
                } else {
                    self.jump_height = 0.0;
                    self.state = PenguinState::Idle;
                }
            }
            PenguinState::Sliding => {
                if self.slide_momentum > 0.0 {
                    match self.direction {
                        Direction::Right => self.x += self.slide_momentum * dt * 20.0,
                        Direction::Left => self.x -= self.slide_momentum * dt * 20.0,
                        Direction::Down => self.y += self.slide_momentum * dt * 20.0,
                        Direction::Up => self.y -= self.slide_momentum * dt * 20.0,
                    }
                    self.slide_momentum -= dt * 2.0; // Friction
                } else {
                    self.slide_momentum = 0.0;
                    self.state = PenguinState::Idle;
                }
            }
            PenguinState::ReleasingBalloon => {
                // Auto-return to idle after balloon release
                if self.animation_timer > 0.5 {
                    self.state = PenguinState::Idle;
                }
            }
            PenguinState::Dancing => {
                // Dance for a bit then return to idle
                if self.animation_timer > 2.0 {
                    self.state = PenguinState::Idle;
                    self.happiness = (self.happiness + 10).min(100);
                }
            }
            _ => {}
        }

        // Keep penguin in world bounds
        self.x = self.x.max(1.0).min(world.width as f32 - 2.0);
        self.y = self.y.max(1.0).min(world.height as f32 - 2.0);

        // Check if on ice or in water
        let ground_y = self.get_ground_level(world);
        if self.y > ground_y && self.state != PenguinState::Jumping {
            self.y = ground_y;
        }
    }

    fn get_ground_level(&self, world: &World) -> f32 {
        // Check if penguin is on an iceberg
        for iceberg in &world.icebergs {
            if self.x >= iceberg.x as f32 && self.x <= (iceberg.x + iceberg.width) as f32 {
                if self.y >= iceberg.y as f32 && self.y <= (iceberg.y + iceberg.height) as f32 {
                    return iceberg.y as f32 - 1.0;
                }
            }
        }
        
        // Otherwise, use water level
        world.water_level as f32 - 1.0
    }

    fn move_penguin(&mut self, direction: Direction, dt: f32) {
        if matches!(self.state, PenguinState::Jumping | PenguinState::Sliding | PenguinState::ReleasingBalloon | PenguinState::Fishing) {
            return; // Can't move during these states
        }

        self.direction = direction.clone();
        self.state = PenguinState::Walking;
        
        let speed = 15.0;
        match direction {
            Direction::Up => self.y -= speed * dt,
            Direction::Down => self.y += speed * dt,
            Direction::Left => self.x -= speed * dt,
            Direction::Right => self.x += speed * dt,
        }
    }

    fn jump(&mut self) {
        if self.jump_height == 0.0 && !matches!(self.state, PenguinState::Jumping) {
            self.state = PenguinState::Jumping;
            self.jump_height = 3.0;
        }
    }

    fn slide(&mut self) {
        if !matches!(self.state, PenguinState::Sliding | PenguinState::Jumping) {
            self.state = PenguinState::Sliding;
            self.slide_momentum = 2.0;
        }
    }

    fn release_balloon(&mut self) -> Balloon {
        self.state = PenguinState::ReleasingBalloon;
        self.animation_timer = 0.0;
        self.happiness = (self.happiness + 5).min(100);
        
        let colors = [Color::Red, Color::Blue, Color::Yellow, Color::Green, Color::Magenta, Color::Cyan];
        let color = colors[self.animation_frame % colors.len()];
        
        Balloon {
            x: self.x,
            y: self.y - 1.0,
            color,
            age: 0.0,
            rising_speed: 2.0 + (self.animation_frame as f32 * 0.3),
            sway: 0.0,
        }
    }

    fn dance(&mut self) {
        if !matches!(self.state, PenguinState::Dancing) {
            self.state = PenguinState::Dancing;
            self.animation_timer = 0.0;
        }
    }

    fn start_fishing(&mut self) {
        if !matches!(self.state, PenguinState::Fishing) {
            self.state = PenguinState::Fishing;
            self.animation_timer = 0.0;
        }
    }
}

impl World {
    fn new(width: u16, height: u16) -> Self {
        let mut world = Self {
            width,
            height,
            icebergs: Vec::new(),
            water_level: height - 8,
            snow_depth: vec![0; width as usize],
        };
        
        world.generate_icebergs();
        world
    }

    fn generate_icebergs(&mut self) {
        // Generate a variety of icebergs for an interesting landscape
        let iceberg_data = [
            (10, 15, 8, 6, IcebergStyle::Medium),
            (25, 18, 12, 4, IcebergStyle::Large),
            (45, 20, 6, 3, IcebergStyle::Small),
            (60, 16, 15, 8, IcebergStyle::Large),
            (85, 19, 8, 5, IcebergStyle::Medium),
            (105, 17, 10, 6, IcebergStyle::Medium),
            (125, 21, 5, 2, IcebergStyle::Small),
            (140, 14, 20, 10, IcebergStyle::Large),
            (170, 19, 7, 4, IcebergStyle::Small),
            (190, 16, 14, 7, IcebergStyle::Large),
        ];

        for (x, y, width, height, style) in iceberg_data {
            if x + width < self.width {
                self.icebergs.push(Iceberg {
                    x,
                    y,
                    width,
                    height,
                    style,
                });
            }
        }
    }

    fn get_terrain_char(&self, x: u16, y: u16) -> (char, Color) {
        // Check if position is on an iceberg
        for iceberg in &self.icebergs {
            if x >= iceberg.x && x < iceberg.x + iceberg.width && 
               y >= iceberg.y && y < iceberg.y + iceberg.height {
                return match iceberg.style {
                    IcebergStyle::Small => ('▓', Color::Rgb(200, 230, 255)),
                    IcebergStyle::Medium => ('█', Color::Rgb(180, 220, 255)),
                    IcebergStyle::Large => ('█', Color::Rgb(160, 210, 255)),
                    IcebergStyle::Platform => ('▄', Color::Rgb(190, 225, 255)),
                };
            }
        }

        // Water
        if y >= self.water_level {
            let water_colors = [
                Color::Rgb(0, 100, 200),
                Color::Rgb(0, 120, 220),
                Color::Rgb(0, 110, 210),
                Color::Rgb(0, 90, 190),
            ];
            let color_index = ((x + y) % 4) as usize;
            return ('~', water_colors[color_index]);
        }

        // Sky with occasional clouds
        if (x + y) % 47 == 0 {
            ('☁', Color::Rgb(240, 240, 255))
        } else if (x + y) % 23 == 0 {
            ('❄', Color::White)
        } else {
            (' ', Color::Rgb(135, 206, 235)) // Sky blue
        }
    }
}

impl Game {
    fn new() -> Self {
        let world = World::new(200, 30);
        
        Self {
            penguin: Penguin::new(),
            world,
            balloons: Vec::new(),
            snowflakes: Vec::new(),
            fish: Vec::new(),
            current_effects: Vec::new(),
            background_hue: 210.0, // Ice blue
            water_animation: 0.0,
            score: 0,
            time_played: 0.0,
            last_tick: Instant::now(),
            keys_pressed: std::collections::HashSet::new(),
            rng: SimpleRng::default(),
            camera_x: 0.0,
            camera_y: 0.0,
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;
        self.time_played += dt;

        // Update penguin
        self.penguin.update(dt, &self.world);

        // Update camera to follow penguin
        let target_camera_x = self.penguin.x - 40.0; // Center penguin horizontally
        self.camera_x += (target_camera_x - self.camera_x) * dt * 3.0; // Smooth follow
        self.camera_x = self.camera_x.max(0.0).min((self.world.width as f32 - 80.0).max(0.0));

        // Update background animations
        self.background_hue += dt * 10.0;
        if self.background_hue >= 360.0 {
            self.background_hue = 210.0; // Reset to ice blue
        }
        self.water_animation += dt * 2.0;

        // Update balloons
        for balloon in &mut self.balloons {
            balloon.y -= balloon.rising_speed * dt;
            balloon.age += dt;
            balloon.sway += dt * 3.0;
            balloon.x += (balloon.sway.sin() * 0.5) * dt;
        }
        self.balloons.retain(|b| b.age < 10.0 && b.y > -5.0);

        // Generate snowflakes occasionally
        if self.rng.gen() % 100 < 3 {
            self.snowflakes.push(Snowflake {
                x: self.camera_x + (self.rng.gen() % 80) as f32,
                y: -1.0,
                speed: 0.5 + (self.rng.gen() % 100) as f32 / 200.0,
                size: if self.rng.gen() % 3 == 0 { '❄' } else { '·' },
            });
        }

        // Update snowflakes
        for snowflake in &mut self.snowflakes {
            snowflake.y += snowflake.speed * dt * 5.0;
            snowflake.x += (self.time_played * 0.5).sin() * dt * 0.5; // Gentle sway
        }
        self.snowflakes.retain(|s| s.y < self.world.height as f32 + 2.0);

        // Generate fish occasionally
        if self.rng.gen() % 200 < 1 {
            self.fish.push(Fish {
                x: self.camera_x + (self.rng.gen() % 80) as f32,
                y: self.world.water_level as f32 + 1.0,
                caught: false,
                animation_timer: 0.0,
            });
        }

        // Update fish
        for fish in &mut self.fish {
            fish.animation_timer += dt;
            fish.x += (fish.animation_timer.sin() * 2.0) * dt; // Swimming motion
            
            // Check if penguin is fishing nearby
            if matches!(self.penguin.state, PenguinState::Fishing) {
                let distance = ((self.penguin.x - fish.x).powf(2.0) + (self.penguin.y - fish.y).powf(2.0)).sqrt();
                if distance < 3.0 && !fish.caught {
                    fish.caught = true;
                    self.score += 10;
                    self.penguin.happiness = (self.penguin.happiness + 15).min(100);
                    
                    // Add celebration effect
                    self.current_effects.push(
                        fx::explode(15.0, 0.6, (800, Interpolation::BounceOut))
                    );
                }
            }
        }
        self.fish.retain(|f| f.animation_timer < 30.0);

        // Clean up completed effects
        self.current_effects.retain(|effect| effect.running());
    }

    fn handle_input(&mut self, key: KeyCode) {
        self.keys_pressed.insert(key);
        
        let dt = 1.0 / 60.0; // Assume 60 FPS for input handling
        
        match key {
            // Movement
            KeyCode::Char('w') | KeyCode::Up => {
                self.penguin.move_penguin(Direction::Up, dt);
                self.add_movement_effect();
            }
            KeyCode::Char('s') | KeyCode::Down => {
                self.penguin.move_penguin(Direction::Down, dt);
                self.add_movement_effect();
            }
            KeyCode::Char('a') | KeyCode::Left => {
                self.penguin.move_penguin(Direction::Left, dt);
                self.add_movement_effect();
            }
            KeyCode::Char('d') | KeyCode::Right => {
                self.penguin.move_penguin(Direction::Right, dt);
                self.add_movement_effect();
            }
            
            // Actions
            KeyCode::Char(' ') => {
                self.penguin.jump();
                self.add_jump_effect();
            }
            KeyCode::Char('b') => {
                let balloon = self.penguin.release_balloon();
                self.balloons.push(balloon);
                self.add_balloon_effect();
            }
            KeyCode::Char('x') => {
                self.penguin.slide();
                self.add_slide_effect();
            }
            KeyCode::Char('z') => {
                self.penguin.dance();
                self.add_dance_effect();
            }
            KeyCode::Char('f') => {
                self.penguin.start_fishing();
                self.add_fishing_effect();
            }
            
            _ => {}
        }
    }

    fn add_movement_effect(&mut self) {
        if self.current_effects.len() < 3 {
            self.current_effects.push(
                fx::fade_from_fg(Color::Cyan, (200, Interpolation::QuadOut))
            );
        }
    }

    fn add_jump_effect(&mut self) {
        self.current_effects.push(
            fx::parallel(&[
                fx::explode(8.0, 0.4, (500, Interpolation::CircOut)),
                fx::hsl_shift_fg([60.0, 0.0, 20.0], (300, Interpolation::QuadOut)),
            ]).into_effect()
        );
    }

    fn add_balloon_effect(&mut self) {
        self.current_effects.push(
            fx::parallel(&[
                fx::fade_from_fg(Color::Magenta, (600, Interpolation::BounceOut)),
                fx::coalesce((400, Interpolation::SineOut)),
            ]).into_effect()
        );
    }

    fn add_slide_effect(&mut self) {
        self.current_effects.push(
            fx::sweep_in(Motion::LeftToRight, 10, 5, Color::Cyan, (800, Interpolation::CircOut))
        );
    }

    fn add_dance_effect(&mut self) {
        self.current_effects.push(
            fx::parallel(&[
                fx::hsl_shift_fg([360.0, 50.0, 0.0], (2000, Interpolation::SineInOut)),
                fx::dissolve((300, Interpolation::BounceOut)),
            ]).into_effect()
        );
    }

    fn add_fishing_effect(&mut self) {
        self.current_effects.push(
            fx::fade_to_fg(Color::Blue, (400, Interpolation::SineOut))
        );
    }

    fn release_input(&mut self, key: KeyCode) {
        self.keys_pressed.remove(&key);
        
        // Stop walking when movement keys are released
        if matches!(key, KeyCode::Char('w') | KeyCode::Char('a') | KeyCode::Char('s') | KeyCode::Char('d') |
                         KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right) {
            let movement_keys = [
                KeyCode::Char('w'), KeyCode::Char('a'), KeyCode::Char('s'), KeyCode::Char('d'),
                KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right
            ];
            
            let still_moving = movement_keys.iter().any(|k| self.keys_pressed.contains(k));
            if !still_moving && matches!(self.penguin.state, PenguinState::Walking) {
                self.penguin.state = PenguinState::Idle;
            }
        }
    }
}

fn main() -> Result<()> {
    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut game = Game::new();

    loop {
        game.update();
        
        terminal.draw(|f| ui(f, &mut game))?;

        if event::poll(StdDuration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match key.kind {
                    KeyEventKind::Press => {
                        match key.code {
                            KeyCode::Esc => break,
                            _ => game.handle_input(key.code),
                        }
                    }
                    KeyEventKind::Release => {
                        game.release_input(key.code);
                    }
                    _ => {}
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

fn ui(f: &mut Frame, game: &mut Game) {
    let area = f.area();
    
    // Clear screen with arctic sky color
    Clear.render(area, f.buffer_mut());
    
    // Main game area
    let chunks = Layout::vertical([
        Constraint::Min(25),     // Game world
        Constraint::Length(5),   // UI/Stats
    ]).split(area);

    render_world(f, game, chunks[0]);
    render_ui(f, game, chunks[1]);
    
    // Apply global effects
    for effect in &mut game.current_effects {
        if effect.running() {
            f.render_effect(effect, chunks[0], Duration::from_millis(16));
        }
    }
}

fn render_world(f: &mut Frame, game: &mut Game, area: Rect) {
    let world_width = area.width.min(80);
    let world_height = area.height.min(25);
    
    // Create world view centered on camera
    let start_x = game.camera_x as u16;
    let start_y = game.camera_y as u16;
    
    // Render terrain
    for y in 0..world_height {
        for x in 0..world_width {
            let world_x = start_x + x;
            let world_y = start_y + y;
            
            if world_x < game.world.width && world_y < game.world.height {
                let (terrain_char, terrain_color) = game.world.get_terrain_char(world_x, world_y);
                
                // Apply water animation to water tiles
                let color = if terrain_char == '~' {
                    let wave_offset = (game.water_animation + x as f32 * 0.2).sin();
                    let blue_intensity = (100.0 + wave_offset * 30.0) as u8;
                    Color::Rgb(0, blue_intensity, (blue_intensity as f32 * 1.8) as u8)
                } else {
                    terrain_color
                };
                
                let char_area = Rect::new(area.x + x, area.y + y, 1, 1);
                let terrain_widget = Paragraph::new(terrain_char.to_string())
                    .style(Style::default().fg(color));
                f.render_widget(terrain_widget, char_area);
            }
        }
    }
    
    // Render snowflakes
    for snowflake in &game.snowflakes {
        let screen_x = (snowflake.x - game.camera_x) as u16;
        let screen_y = snowflake.y as u16;
        
        if screen_x < world_width && screen_y < world_height {
            let snow_area = Rect::new(area.x + screen_x, area.y + screen_y, 1, 1);
            let snow_widget = Paragraph::new(snowflake.size.to_string())
                .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
            f.render_widget(snow_widget, snow_area);
        }
    }
    
    // Render fish
    for fish in &game.fish {
        if !fish.caught {
            let screen_x = (fish.x - game.camera_x) as u16;
            let screen_y = fish.y as u16;
            
            if screen_x < world_width && screen_y < world_height {
                let fish_char = if (fish.animation_timer * 4.0) as u32 % 2 == 0 { "🐟" } else { "🐠" };
                let fish_area = Rect::new(area.x + screen_x, area.y + screen_y, 1, 1);
                let fish_widget = Paragraph::new(fish_char)
                    .style(Style::default().fg(Color::Yellow));
                f.render_widget(fish_widget, fish_area);
            }
        }
    }
    
    // Render balloons
    for balloon in &game.balloons {
        let screen_x = (balloon.x - game.camera_x) as u16;
        let screen_y = balloon.y as u16;
        
        if screen_x < world_width && balloon.y >= 0.0 {
            let balloon_area = Rect::new(area.x + screen_x, area.y + screen_y, 1, 1);
            let balloon_widget = Paragraph::new("🎈")
                .style(Style::default().fg(balloon.color).add_modifier(Modifier::BOLD));
            f.render_widget(balloon_widget, balloon_area);
        }
    }
    
    // Render penguin (always centered when possible)
    let penguin_screen_x = (game.penguin.x - game.camera_x) as u16;
    let penguin_screen_y = (game.penguin.y - game.penguin.jump_height) as u16;
    
    if penguin_screen_x < world_width && penguin_screen_y < world_height {
        let (sprite, color) = game.penguin.get_sprite();
        let penguin_area = Rect::new(area.x + penguin_screen_x, area.y + penguin_screen_y, 2, 1);
        
        // Add glow effect based on happiness
        let happiness_color = if game.penguin.happiness > 80 {
            Color::Yellow
        } else if game.penguin.happiness > 50 {
            Color::Green
        } else {
            color
        };
        
        let penguin_widget = Paragraph::new(sprite)
            .style(Style::default().fg(happiness_color).add_modifier(Modifier::BOLD));
        f.render_widget(penguin_widget, penguin_area);
    }
}

fn render_ui(f: &mut Frame, game: &Game, area: Rect) {
    // Create UI layout
    let ui_chunks = Layout::horizontal([
        Constraint::Percentage(25), // Stats
        Constraint::Percentage(50), // Controls
        Constraint::Percentage(25), // Status
    ]).split(area);
    
    // Stats panel
    let stats_text = vec![
        Line::from(vec![
            Span::styled("🎯 Score: ", Style::default().fg(Color::Yellow)),
            Span::styled(game.score.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("⏰ Time: ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{:.0}s", game.time_played), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("🎈 Balloons: ", Style::default().fg(Color::Magenta)),
            Span::styled(game.balloons.len().to_string(), Style::default().fg(Color::White)),
        ]),
    ];
    
    let stats_widget = Paragraph::new(stats_text)
        .block(Block::default().borders(Borders::ALL).title("📊 Stats"))
        .alignment(Alignment::Left);
    f.render_widget(stats_widget, ui_chunks[0]);
    
    // Controls panel
    let controls_text = vec![
        Line::from("🎮 WASD/Arrow Keys: Move penguin").style(Style::default().fg(Color::Green)),
        Line::from("🦘 SPACE: Jump  🎈 B: Release balloon").style(Style::default().fg(Color::Yellow)),
        Line::from("🛷 X: Slide  💃 Z: Dance  🎣 F: Fish").style(Style::default().fg(Color::Cyan)),
    ];
    
    let controls_widget = Paragraph::new(controls_text)
        .block(Block::default().borders(Borders::ALL).title("🎮 Controls"))
        .alignment(Alignment::Center);
    f.render_widget(controls_widget, ui_chunks[1]);
    
    // Status panel
    let happiness_bar = "█".repeat((game.penguin.happiness / 10) as usize);
    let state_emoji = match game.penguin.state {
        PenguinState::Idle => "😴",
        PenguinState::Walking => "🚶",
        PenguinState::Jumping => "🦘",
        PenguinState::Sliding => "🛷",
        PenguinState::ReleasingBalloon => "🎈",
        PenguinState::Dancing => "💃",
        PenguinState::Fishing => "🎣",
    };
    
    let status_text = vec![
        Line::from(vec![
            Span::styled("State: ", Style::default().fg(Color::Blue)),
            Span::styled(state_emoji, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Happy: ", Style::default().fg(Color::Green)),
            Span::styled(happiness_bar, Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Pos: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("({:.0},{:.0})", game.penguin.x, game.penguin.y), Style::default().fg(Color::White)),
        ]),
    ];
    
    let status_widget = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title("🐧 Penguin"))
        .alignment(Alignment::Left);
    f.render_widget(status_widget, ui_chunks[2]);
}
