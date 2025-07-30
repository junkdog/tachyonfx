use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use std::{io, time::Instant};
use tachyonfx::{
    fx::{self, repeating, sequence},
    Duration, Effect, EffectRenderer, IntoEffect, Motion, Shader,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameState {
    Title,
    Playing,
    Dialogue,
    Combat,
    Victory,
    GameOver,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerrainType {
    Grass,
    Tree,
    Rock,
    Water,
    Path,
}

impl TerrainType {
    fn symbol(&self) -> &'static str {
        match self {
            TerrainType::Grass => ".",
            TerrainType::Tree => "T",
            TerrainType::Rock => "#",
            TerrainType::Water => "~",
            TerrainType::Path => "+",
        }
    }

    fn color(&self) -> Color {
        match self {
            TerrainType::Grass => Color::Green,
            TerrainType::Tree => Color::Rgb(34, 139, 34),
            TerrainType::Rock => Color::Gray,
            TerrainType::Water => Color::Blue,
            TerrainType::Path => Color::Yellow,
        }
    }

    fn walkable(&self) -> bool {
        match self {
            TerrainType::Grass | TerrainType::Path => true,
            _ => false,
        }
    }
}

struct Player {
    x: u16,
    y: u16,
    hp: i32,
    max_hp: i32,
    level: i32,
    exp: i32,
    exp_to_next: i32,
    move_effect: Effect,
    level_up_effect: Effect,
}

impl Player {
    fn new() -> Self {
        Self {
            x: 10,
            y: 10,
            hp: 100,
            max_hp: 100,
            level: 1,
            exp: 0,
            exp_to_next: 100,
            move_effect: fx::fade_from_fg(Color::White, Duration::from_millis(200)).into_effect(),
            level_up_effect: fx::fade_from_fg(Color::Yellow, Duration::from_millis(1000)).into_effect(),
        }
    }

    fn gain_exp(&mut self, amount: i32) {
        self.exp += amount;
        if self.exp >= self.exp_to_next {
            self.level += 1;
            self.exp = 0;
            self.exp_to_next = 100 * self.level;
            self.max_hp += 20;
            self.hp = self.max_hp;
            self.level_up_effect.reset();
        }
    }
}

struct Npc {
    x: u16,
    y: u16,
    dialogue: Vec<String>,
    current_dialogue: usize,
    symbol: &'static str,
    color: Color,
    bounce_effect: Effect,
}

impl Npc {
    fn new(x: u16, y: u16, dialogue: Vec<&str>, symbol: &'static str, color: Color) -> Self {
        Self {
            x,
            y,
            dialogue: dialogue.into_iter().map(|s| s.to_string()).collect(),
            current_dialogue: 0,
            symbol,
            color,
            bounce_effect: repeating(sequence(&[
                fx::fade_to_fg(color, Duration::from_millis(800)),
                fx::fade_from_fg(color, Duration::from_millis(800)),
            ])),
        }
    }

    fn get_current_dialogue(&self) -> &String {
        &self.dialogue[self.current_dialogue]
    }

    fn next_dialogue(&mut self) {
        self.current_dialogue = (self.current_dialogue + 1) % self.dialogue.len();
    }
}

struct Enemy {
    x: u16,
    y: u16,
    hp: i32,
    max_hp: i32,
    name: String,
    symbol: &'static str,
    color: Color,
    exp_reward: i32,
    idle_effect: Effect,
    damage_effect: Effect,
}

impl Enemy {
    fn new(x: u16, y: u16, name: &str, hp: i32, symbol: &'static str, color: Color, exp_reward: i32) -> Self {
        Self {
            x,
            y,
            hp,
            max_hp: hp,
            name: name.to_string(),
            symbol,
            color,
            exp_reward,
            idle_effect: repeating(sequence(&[
                fx::fade_to_fg(color, Duration::from_millis(1500)),
                fx::fade_from_fg(color, Duration::from_millis(1500)),
            ])),
            damage_effect: fx::fade_from_fg(Color::Red, Duration::from_millis(300)).into_effect(),
        }
    }
}

struct World {
    width: usize,
    height: usize,
    terrain: Vec<Vec<TerrainType>>,
    sparkle_effect: Effect,
}

impl World {
    fn new(width: usize, height: usize) -> Self {
        let mut terrain = vec![vec![TerrainType::Grass; width]; height];
        
        // Add some trees randomly
        for y in 0..height {
            for x in 0..width {
                if (x + y) % 7 == 0 && x > 5 && y > 5 {
                    terrain[y][x] = TerrainType::Tree;
                } else if (x * 3 + y * 2) % 11 == 0 && x > 8 && y > 8 {
                    terrain[y][x] = TerrainType::Rock;
                } else if y < 3 || x < 3 || y > height - 4 || x > width - 4 {
                    if (x + y) % 4 == 0 {
                        terrain[y][x] = TerrainType::Water;
                    }
                }
            }
        }

        // Create a path
        for i in 3..width - 3 {
            terrain[5][i] = TerrainType::Path;
            terrain[height - 6][i] = TerrainType::Path;
        }
        for i in 3..height - 3 {
            terrain[i][5] = TerrainType::Path;
            terrain[i][width - 6] = TerrainType::Path;
        }

        Self {
            width,
            height,
            terrain,
            sparkle_effect: repeating(sequence(&[
                fx::fade_to_fg(Color::White, Duration::from_millis(2000)),
                fx::fade_from_fg(Color::White, Duration::from_millis(2000)),
            ])),
        }
    }

    fn get_terrain(&self, x: usize, y: usize) -> TerrainType {
        if x < self.width && y < self.height {
            self.terrain[y][x]
        } else {
            TerrainType::Water
        }
    }

    fn can_walk(&self, x: u16, y: u16) -> bool {
        if x as usize >= self.width || y as usize >= self.height {
            return false;
        }
        self.terrain[y as usize][x as usize].walkable()
    }
}

struct App<'a> {
    game_state: GameState,
    last_tick: Duration,
    
    // Title screen effects
    title_effect: Effect,
    title_text: Vec<Line<'a>>,
    title_sparkles: Effect,
    
    // World and entities
    world: World,
    player: Player,
    npcs: Vec<Npc>,
    enemies: Vec<Enemy>,
    
    // Game state
    current_npc: Option<usize>,
    current_enemy: Option<usize>,
    
    // UI effects
    dialogue_effect: Effect,
    combat_ui_effect: Effect,
    victory_effect: Effect,
    screen_shake: Effect,
    
    // Combat state
    combat_turn: bool, // true = player turn, false = enemy turn
    combat_message: String,
    combat_message_timer: Duration,
}

impl<'a> App<'a> {
    fn new() -> Self {
        let title_text = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "*** LEGENDARY QUEST ***",
                Style::default().fg(Color::Rgb(255, 215, 0)).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "A Nintendo-Style Terminal Adventure",
                Style::default().fg(Color::Cyan),
            )),
            Line::from(""),
            Line::from("Press Enter to Begin Your Journey!"),
            Line::from(""),
            Line::from(Span::styled("* * * * *", Style::default().fg(Color::Yellow))),
        ];

        let world = World::new(50, 30);
        
        let npcs = vec![
            Npc::new(
                15, 10,
                vec![
                    "Welcome, brave adventurer!",
                    "The evil creatures have invaded our peaceful land!",
                    "Please help us defeat them and restore peace!",
                    "I believe in you, hero!"
                ],
                "W", Color::Blue
            ),
            Npc::new(
                25, 8,
                vec![
                    "Thank you for helping us!",
                    "Here's some wisdom: Fight strategically!",
                    "Your level increases with each victory!",
                    "May the gods protect you!"
                ],
                "P", Color::Magenta
            ),
        ];

        let enemies = vec![
            Enemy::new(20, 15, "Goblin", 30, "G", Color::Red, 25),
            Enemy::new(30, 20, "Orc", 50, "O", Color::Rgb(139, 69, 19), 50),
            Enemy::new(35, 12, "Dragon", 100, "D", Color::Rgb(255, 0, 0), 100),
        ];

        Self {
            game_state: GameState::Title,
            last_tick: Duration::ZERO,
            
            title_effect: sequence(&[
                fx::sweep_in(Motion::LeftToRight, 30, 0, Color::Black, Duration::from_millis(1500)),
                fx::fade_to_fg(Color::Yellow, Duration::from_millis(1000)),
            ]),
            title_text,
            title_sparkles: repeating(sequence(&[
                fx::fade_to_fg(Color::Yellow, Duration::from_millis(800)),
                fx::fade_from_fg(Color::Yellow, Duration::from_millis(800)),
            ])),
            
            world,
            player: Player::new(),
            npcs,
            enemies,
            
            current_npc: None,
            current_enemy: None,
            
            dialogue_effect: fx::slide_in(Motion::LeftToRight, 20, 0, Color::White, Duration::from_millis(300)).into_effect(),
            combat_ui_effect: fx::sweep_in(Motion::UpToDown, 10, 0, Color::Black, Duration::from_millis(500)),
            victory_effect: sequence(&[
                fx::fade_to_fg(Color::Yellow, Duration::from_millis(500)),
                fx::fade_from_fg(Color::Yellow, Duration::from_millis(500)),
                fx::fade_to_fg(Color::Green, Duration::from_millis(500)),
            ]),
            screen_shake: fx::fade_from_fg(Color::Red, Duration::from_millis(200)).into_effect(),
            
            combat_turn: true,
            combat_message: String::new(),
            combat_message_timer: Duration::ZERO,
        }
    }

    fn on_tick(&mut self, duration: Duration) {
        self.last_tick = duration;
        if self.combat_message_timer > Duration::ZERO {
            if self.combat_message_timer > duration {
                self.combat_message_timer = self.combat_message_timer - duration;
            } else {
                self.combat_message_timer = Duration::ZERO;
                self.combat_message.clear();
            }
        }
    }

    fn move_player(&mut self, dx: i16, dy: i16) {
        let new_x = (self.player.x as i16 + dx).max(0) as u16;
        let new_y = (self.player.y as i16 + dy).max(0) as u16;
        
        if self.world.can_walk(new_x, new_y) {
            self.player.x = new_x;
            self.player.y = new_y;
            self.player.move_effect.reset();
        }
        
        self.check_interactions();
    }

    fn check_interactions(&mut self) {
        // Check for NPC interactions
        for (i, npc) in self.npcs.iter().enumerate() {
            if self.player.x.abs_diff(npc.x) <= 1 && self.player.y.abs_diff(npc.y) <= 1 {
                self.current_npc = Some(i);
                self.game_state = GameState::Dialogue;
                self.dialogue_effect.reset();
                return;
            }
        }

        // Check for enemy encounters
        for (i, enemy) in self.enemies.iter().enumerate() {
            if enemy.hp > 0 && self.player.x.abs_diff(enemy.x) <= 1 && self.player.y.abs_diff(enemy.y) <= 1 {
                self.current_enemy = Some(i);
                self.game_state = GameState::Combat;
                self.combat_ui_effect.reset();
                self.combat_turn = true;
                self.combat_message = format!("A wild {} appears!", enemy.name);
                self.combat_message_timer = Duration::from_millis(2000);
                return;
            }
        }
    }

    fn attack_enemy(&mut self) {
        if let Some(enemy_idx) = self.current_enemy {
            let damage = 20 + (self.player.level - 1) * 5;
            self.enemies[enemy_idx].hp -= damage;
            self.enemies[enemy_idx].damage_effect.reset();
            self.screen_shake.reset();
            
            self.combat_message = format!("You deal {} damage!", damage);
            self.combat_message_timer = Duration::from_millis(1500);

            if self.enemies[enemy_idx].hp <= 0 {
                let exp_reward = self.enemies[enemy_idx].exp_reward;
                self.player.gain_exp(exp_reward);
                self.combat_message = format!("Victory! You gained {} EXP!", exp_reward);
                self.combat_message_timer = Duration::from_millis(2000);
                self.game_state = GameState::Victory;
                self.victory_effect.reset();
            } else {
                self.combat_turn = false;
                self.enemy_attack();
            }
        }
    }

    fn enemy_attack(&mut self) {
        if let Some(enemy_idx) = self.current_enemy {
            let damage = 10 + (self.enemies[enemy_idx].max_hp / 10);
            self.player.hp -= damage;
            self.screen_shake.reset();
            
            self.combat_message = format!("{} attacks for {} damage!", self.enemies[enemy_idx].name, damage);
            self.combat_message_timer = Duration::from_millis(1500);

            if self.player.hp <= 0 {
                self.game_state = GameState::GameOver;
            } else {
                self.combat_turn = true;
            }
        }
    }

    fn end_combat(&mut self) {
        self.current_enemy = None;
        self.game_state = GameState::Playing;
        self.combat_message.clear();
    }
}

fn main() -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    Ok(result?)
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let mut last_frame_instant = Instant::now();
    loop {
        let frame_duration = last_frame_instant.elapsed();
        app.on_tick(frame_duration.into());
        last_frame_instant = Instant::now();

        terminal.draw(|f| ui(f, app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.game_state {
                        GameState::Title => match key.code {
                            KeyCode::Enter => app.game_state = GameState::Playing,
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            _ => {}
                        },
                        GameState::Playing => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Up | KeyCode::Char('w') => app.move_player(0, -1),
                            KeyCode::Down | KeyCode::Char('s') => app.move_player(0, 1),
                            KeyCode::Left | KeyCode::Char('a') => app.move_player(-1, 0),
                            KeyCode::Right | KeyCode::Char('d') => app.move_player(1, 0),
                            _ => {}
                        },
                        GameState::Dialogue => match key.code {
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                if let Some(npc_idx) = app.current_npc {
                                    app.npcs[npc_idx].next_dialogue();
                                }
                                app.dialogue_effect.reset();
                            },
                            KeyCode::Esc => {
                                app.current_npc = None;
                                app.game_state = GameState::Playing;
                            },
                            _ => {}
                        },
                        GameState::Combat => match key.code {
                            KeyCode::Char('a') | KeyCode::Enter => {
                                if app.combat_turn {
                                    app.attack_enemy();
                                }
                            },
                            KeyCode::Char('r') | KeyCode::Esc => app.end_combat(),
                            _ => {}
                        },
                        GameState::Victory => match key.code {
                            KeyCode::Enter | KeyCode::Char(' ') => app.end_combat(),
                            _ => {}
                        },
                        GameState::GameOver => match key.code {
                            KeyCode::Enter | KeyCode::Char('r') => {
                                *app = App::new();
                                app.game_state = GameState::Playing;
                            },
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            _ => {}
                        },
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    match app.game_state {
        GameState::Title => render_title_screen(f, app),
        GameState::Playing | GameState::Dialogue | GameState::Combat | GameState::Victory => render_game_world(f, app),
        GameState::GameOver => render_game_over(f, app),
    }
}

fn render_title_screen(f: &mut Frame, app: &mut App) {
    let area = f.area();
    
    let text = Text::from(app.title_text.clone()).alignment(Alignment::Center);
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Welcome to the Adventure!"));
    
    f.render_widget(paragraph, area);
    
    if app.title_effect.running() {
        f.render_effect(&mut app.title_effect, area, app.last_tick);
    }
    
    // Add sparkle effects in corners
    let sparkle_areas = [
        Rect::new(2, 2, 5, 3),
        Rect::new(area.width - 7, 2, 5, 3),
        Rect::new(2, area.height - 5, 5, 3),
        Rect::new(area.width - 7, area.height - 5, 5, 3),
    ];
    
    for &sparkle_area in &sparkle_areas {
        if app.title_sparkles.running() {
            f.render_effect(&mut app.title_sparkles, sparkle_area, app.last_tick);
        }
    }
}

fn render_game_world(f: &mut Frame, app: &mut App) {
    let area = f.area();
    
    // Main game area layout
    let chunks = Layout::default()
        .constraints([Constraint::Min(20), Constraint::Length(5)])
        .split(area);
    
    let game_area = chunks[0];
    let ui_area = chunks[1];
    
    // Render world background
    render_world(f, app, game_area);
    
    // Render entities
    render_entities(f, app, game_area);
    
    // Render UI
    render_ui(f, app, ui_area);
    
    // Render dialogue overlay
    if app.game_state == GameState::Dialogue {
        render_dialogue(f, app);
    }
    
    // Render combat overlay
    if app.game_state == GameState::Combat {
        render_combat(f, app);
    }
    
    // Render victory overlay
    if app.game_state == GameState::Victory {
        render_victory(f, app);
    }
    
    // Apply screen shake if active
    if app.screen_shake.running() {
        f.render_effect(&mut app.screen_shake, area, app.last_tick);
    }
}

fn render_world(f: &mut Frame, app: &mut App, area: Rect) {
    // Calculate visible area around player
    let view_width = area.width.min(40);
    let view_height = area.height.min(25);
    
    let start_x = app.player.x.saturating_sub(view_width / 2);
    let start_y = app.player.y.saturating_sub(view_height / 2);
    
    for dy in 0..view_height {
        for dx in 0..view_width {
            let world_x = start_x + dx;
            let world_y = start_y + dy;
            let screen_x = area.x + dx;
            let screen_y = area.y + dy;
            
            if screen_x < area.x + area.width && screen_y < area.y + area.height {
                let terrain = app.world.get_terrain(world_x as usize, world_y as usize);
                let terrain_char = Text::from(Span::styled(
                    terrain.symbol(),
                    Style::default().fg(terrain.color())
                ));
                let terrain_widget = Paragraph::new(terrain_char);
                let terrain_area = Rect::new(screen_x, screen_y, 1, 1);
                f.render_widget(terrain_widget, terrain_area);
                
                // Add sparkle effect to some grass
                if matches!(terrain, TerrainType::Grass) && (world_x + world_y) % 15 == 0 {
                    if app.world.sparkle_effect.running() {
                        f.render_effect(&mut app.world.sparkle_effect, terrain_area, app.last_tick);
                    }
                }
            }
        }
    }
}

fn render_entities(f: &mut Frame, app: &mut App, area: Rect) {
    let view_width = area.width.min(40);
    let view_height = area.height.min(25);
    let start_x = app.player.x.saturating_sub(view_width / 2);
    let start_y = app.player.y.saturating_sub(view_height / 2);
    
    // Render NPCs
    for npc in &mut app.npcs {
        if npc.x >= start_x && npc.x < start_x + view_width && 
           npc.y >= start_y && npc.y < start_y + view_height {
            let screen_x = area.x + (npc.x - start_x);
            let screen_y = area.y + (npc.y - start_y);
            
            let npc_char = Text::from(Span::styled(npc.symbol, Style::default().fg(npc.color)));
            let npc_widget = Paragraph::new(npc_char);
            let npc_area = Rect::new(screen_x, screen_y, 1, 1);
            f.render_widget(npc_widget, npc_area);
            
            if npc.bounce_effect.running() {
                f.render_effect(&mut npc.bounce_effect, npc_area, app.last_tick);
            }
        }
    }
    
    // Render enemies
    for enemy in &mut app.enemies {
        if enemy.hp > 0 && enemy.x >= start_x && enemy.x < start_x + view_width && 
           enemy.y >= start_y && enemy.y < start_y + view_height {
            let screen_x = area.x + (enemy.x - start_x);
            let screen_y = area.y + (enemy.y - start_y);
            
            let enemy_char = Text::from(Span::styled(enemy.symbol, Style::default().fg(enemy.color)));
            let enemy_widget = Paragraph::new(enemy_char);
            let enemy_area = Rect::new(screen_x, screen_y, 1, 1);
            f.render_widget(enemy_widget, enemy_area);
            
            if enemy.idle_effect.running() {
                f.render_effect(&mut enemy.idle_effect, enemy_area, app.last_tick);
            }
            
            if enemy.damage_effect.running() {
                f.render_effect(&mut enemy.damage_effect, enemy_area, app.last_tick);
            }
        }
    }
    
    // Render player
    let player_screen_x = area.x + (app.player.x - start_x);
    let player_screen_y = area.y + (app.player.y - start_y);
    
    let player_char = Text::from(Span::styled("@", Style::default().fg(Color::Cyan)));
    let player_widget = Paragraph::new(player_char);
    let player_area = Rect::new(player_screen_x, player_screen_y, 1, 1);
    f.render_widget(player_widget, player_area);
    
    if app.player.move_effect.running() {
        f.render_effect(&mut app.player.move_effect, player_area, app.last_tick);
    }
    
    if app.player.level_up_effect.running() {
        f.render_effect(&mut app.player.level_up_effect, player_area, app.last_tick);
    }
}

fn render_ui(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    
    // Player stats
    let _hp_ratio = app.player.hp as f64 / app.player.max_hp as f64;
    let _exp_ratio = app.player.exp as f64 / app.player.exp_to_next as f64;
    
    let stats_text = Text::from(vec![
        Line::from(vec![
            Span::styled("HP: ", Style::default().fg(Color::Red)),
            Span::styled(format!("{}/{}", app.player.hp, app.player.max_hp), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Level: ", Style::default().fg(Color::Yellow)),
            Span::styled(format!("{}", app.player.level), Style::default().fg(Color::White)),
            Span::styled(format!(" ({}/{})", app.player.exp, app.player.exp_to_next), Style::default().fg(Color::Gray)),
        ]),
    ]);
    
    let stats_widget = Paragraph::new(stats_text)
        .block(Block::default().borders(Borders::ALL).title("Stats"));
    
    f.render_widget(stats_widget, chunks[0]);
    
    // Controls
    let controls_text = match app.game_state {
        GameState::Playing => "WASD/Arrows: Move • ESC: Quit",
        GameState::Dialogue => "SPACE: Next • ESC: Exit",
        GameState::Combat => "A: Attack • R: Run",
        GameState::Victory => "SPACE: Continue",
        _ => "",
    };
    
    let controls_widget = Paragraph::new(controls_text)
        .block(Block::default().borders(Borders::ALL).title("Controls"))
        .alignment(Alignment::Center);
    
    f.render_widget(controls_widget, chunks[1]);
}

fn render_dialogue(f: &mut Frame, app: &mut App) {
    if let Some(npc_idx) = app.current_npc {
        let area = f.area();
        let dialogue_area = Rect::new(
            area.width / 6,
            area.height - 8,
            (area.width * 2) / 3,
            6
        );
        
        f.render_widget(Clear, dialogue_area);
        
        let dialogue_text = app.npcs[npc_idx].get_current_dialogue();
        let dialogue_widget = Paragraph::new(dialogue_text.as_str())
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("{} says:", app.npcs[npc_idx].symbol)))
            .alignment(Alignment::Left);
        
        f.render_widget(dialogue_widget, dialogue_area);
        
        if app.dialogue_effect.running() {
            f.render_effect(&mut app.dialogue_effect, dialogue_area, app.last_tick);
        }
    }
}

fn render_combat(f: &mut Frame, app: &mut App) {
    if let Some(enemy_idx) = app.current_enemy {
        let area = f.area();
        let combat_area = Rect::new(
            area.width / 4,
            area.height / 3,
            area.width / 2,
            area.height / 3
        );
        
        f.render_widget(Clear, combat_area);
        
        let enemy = &app.enemies[enemy_idx];
        let _hp_ratio = enemy.hp as f64 / enemy.max_hp as f64;
        
        let combat_text = Text::from(vec![
            Line::from(vec![
                Span::styled(format!("Fighting: {}", enemy.name), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Enemy HP: ", Style::default().fg(Color::White)),
                Span::styled(format!("{}/{}", enemy.hp, enemy.max_hp), Style::default().fg(Color::Red)),
            ]),
            Line::from(""),
            Line::from(if app.combat_turn {
                "Your turn! Press A to attack!"
            } else {
                "Enemy's turn..."
            }),
            Line::from(""),
            Line::from(app.combat_message.as_str()),
        ]);
        
        let combat_widget = Paragraph::new(combat_text)
            .block(Block::default().borders(Borders::ALL).title("Combat!"))
            .alignment(Alignment::Center);
        
        f.render_widget(combat_widget, combat_area);
        
        if app.combat_ui_effect.running() {
            f.render_effect(&mut app.combat_ui_effect, combat_area, app.last_tick);
        }
    }
}

fn render_victory(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let victory_area = Rect::new(
        area.width / 4,
        area.height / 3,
        area.width / 2,
        area.height / 4
    );
    
    f.render_widget(Clear, victory_area);
    
    let victory_text = Text::from(vec![
        Line::from(vec![
            Span::styled("*** VICTORY! ***", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(app.combat_message.as_str()),
        Line::from(""),
        Line::from("Press SPACE to continue"),
    ]);
    
    let victory_widget = Paragraph::new(victory_text)
        .block(Block::default().borders(Borders::ALL).title("Victory!"))
        .alignment(Alignment::Center);
    
    f.render_widget(victory_widget, victory_area);
    
    if app.victory_effect.running() {
        f.render_effect(&mut app.victory_effect, victory_area, app.last_tick);
    }
}

fn render_game_over(f: &mut Frame, _app: &mut App) {
    let area = f.area();
    
    let game_over_text = Text::from(vec![
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("*** GAME OVER ***", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from("Your journey ends here..."),
        Line::from(""),
        Line::from("Press ENTER to try again"),
        Line::from("Press Q to quit"),
    ]).alignment(Alignment::Center);
    
    let game_over_widget = Paragraph::new(game_over_text)
        .block(Block::default().borders(Borders::ALL).title("The End"));
    
    f.render_widget(game_over_widget, area);
}