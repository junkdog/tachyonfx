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
    Frame, Terminal,
};
use tachyonfx::{
    Duration,
};

mod chess_engine;
mod chess_ui;
mod chess_effects;

use chess_engine::{ChessBoard, ChessMove, ChessPiece, Color as ChessColor, SimpleAI};
use chess_ui::*;
use chess_effects::*;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Clone, PartialEq)]
pub enum GameMode {
    SinglePlayer,
    Multiplayer,
    Menu,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Playing,
    Check(ChessColor),
    Checkmate(ChessColor),
    Stalemate,
    Menu,
}

pub struct ChessGame {
    board: ChessBoard,
    game_state: GameState,
    current_player: ChessColor,
    selected_square: Option<(usize, usize)>,
    cursor_position: (usize, usize),
    valid_moves: Vec<(usize, usize)>,
    move_history: Vec<ChessMove>,
    captured_pieces: Vec<ChessPiece>,
    game_mode: GameMode,
    effects: EffectManager,
    last_frame: Instant,
    ai: Option<SimpleAI>,
}

impl ChessGame {
    pub fn new() -> Self {
        Self {
            board: ChessBoard::new(),
            game_state: GameState::Menu,
            current_player: ChessColor::White,
            selected_square: None,
            cursor_position: (0, 0),
            valid_moves: Vec::new(),
            move_history: Vec::new(),
            captured_pieces: Vec::new(),
            game_mode: GameMode::Menu,
            effects: EffectManager::new(),
            last_frame: Instant::now(),
            ai: None,
        }
    }

    pub fn start_single_player(&mut self) {
        self.game_mode = GameMode::SinglePlayer;
        self.game_state = GameState::Playing;
        self.ai = Some(SimpleAI::new(ChessColor::Black));
        self.board = ChessBoard::new(); // Back to normal chess position
        self.current_player = ChessColor::White;
        self.selected_square = None;
        self.cursor_position = (0, 0);
        self.valid_moves.clear();
        self.move_history.clear();
        self.captured_pieces.clear();
    }

    pub fn start_multiplayer(&mut self) {
        self.game_mode = GameMode::Multiplayer;
        self.game_state = GameState::Playing;
        self.ai = None;
        self.board = ChessBoard::new();
        self.current_player = ChessColor::White;
        self.selected_square = None;
        self.cursor_position = (0, 0);
        self.valid_moves.clear();
        self.move_history.clear();
        self.captured_pieces.clear();
    }

    pub fn handle_input(&mut self, key: KeyCode) {
        match self.game_state {
            GameState::Menu => self.handle_menu_input(key),
            GameState::Playing => self.handle_game_input(key),
            _ => {
                if matches!(key, KeyCode::Char('r') | KeyCode::Enter) {
                    self.game_state = GameState::Menu;
                }
            }
        }
    }

    fn handle_menu_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('1') => self.start_single_player(),
            KeyCode::Char('2') => self.start_multiplayer(),
            _ => {}
        }
    }

    fn handle_game_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.game_state = GameState::Menu,
            KeyCode::Char('r') => {
                if self.game_mode == GameMode::SinglePlayer {
                    self.start_single_player();
                } else {
                    self.start_multiplayer();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(0, -1),
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(0, 1),
            KeyCode::Left | KeyCode::Char('h') => self.move_cursor(-1, 0),
            KeyCode::Right | KeyCode::Char('l') => self.move_cursor(1, 0),
            KeyCode::Enter | KeyCode::Char(' ') => self.select_square(),
            _ => {}
        }
    }

    fn move_cursor(&mut self, dx: i32, dy: i32) {
        let new_x = (self.cursor_position.0 as i32 + dx).clamp(0, 7) as usize;
        let new_y = (self.cursor_position.1 as i32 + dy).clamp(0, 7) as usize;
        self.cursor_position = (new_x, new_y);
    }

    fn select_square(&mut self) {
        let pos = self.cursor_position;
        
        if let Some(selected_pos) = self.selected_square {
            // If we already have a square selected
            if selected_pos == pos {
                // Deselect if clicking the same square
                self.selected_square = None;
                self.valid_moves.clear();
                self.effects.set_selected_position(None);
                self.effects.set_valid_moves(Vec::new());
            } else if self.valid_moves.contains(&pos) {
                // Make the move if it's valid
                if let Some(piece) = self.board.get_piece(selected_pos) {
                    let chess_move = ChessMove::new(selected_pos, pos, piece);
                    self.make_move(chess_move);
                    self.selected_square = None;
                    self.valid_moves.clear();
                    self.effects.set_selected_position(None);
                    self.effects.set_valid_moves(Vec::new());
                }
            } else {
                // Select a different piece
                self.try_select_piece(pos);
            }
        } else {
            // No piece currently selected
            self.try_select_piece(pos);
        }
    }

    fn try_select_piece(&mut self, pos: (usize, usize)) {
        if let Some(piece) = self.board.get_piece(pos) {
            if piece.color == self.current_player {
                self.selected_square = Some(pos);
                self.valid_moves = self.board.get_valid_moves(pos, self.current_player);
                self.effects.set_selected_position(Some(pos));
                self.effects.set_valid_moves(self.valid_moves.clone());
            }
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let elapsed = now - self.last_frame;
        self.last_frame = now;
        
        self.effects.update(elapsed.into());

        // Handle AI moves
        if let Some(ref mut ai) = self.ai {
            if self.current_player == ai.color && self.game_state == GameState::Playing {
                if let Some(chess_move) = ai.get_move(&self.board) {
                    self.make_move(chess_move);
                }
            }
        }
    }

    fn make_move(&mut self, chess_move: ChessMove) {
        // Apply the move to the board
        if let Some(captured) = self.board.make_move(&chess_move) {
            self.captured_pieces.push(captured);
            // Add explosion effect for captured piece
            self.effects.add_explosion(chess_move.to);
        }

        self.move_history.push(chess_move);
        self.current_player = self.current_player.opposite();
        
        // Add move trail effect
        self.effects.add_move_trail(chess_move.from, chess_move.to);
        
        // Check game state
        self.update_game_state();
    }

    fn update_game_state(&mut self) {
        let is_in_check = self.board.is_in_check(self.current_player);
        let valid_moves = self.board.get_all_valid_moves(self.current_player);
        
        if valid_moves.is_empty() {
            if is_in_check {
                self.game_state = GameState::Checkmate(self.current_player);
            } else {
                self.game_state = GameState::Stalemate;
            }
        } else if is_in_check {
            self.game_state = GameState::Check(self.current_player);
            self.effects.set_check_position(Some(match self.current_player {
                ChessColor::White => self.board.white_king_pos,
                ChessColor::Black => self.board.black_king_pos,
            }));
        } else {
            self.game_state = GameState::Playing;
            self.effects.set_check_position(None);
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

    let mut game = ChessGame::new();

    loop {
        game.update();
        
        terminal.draw(|f| render_ui(f, &mut game))?;

        if event::poll(StdDuration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc if game.game_state == GameState::Menu => break,
                        _ => game.handle_input(key.code),
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

fn render_ui(f: &mut Frame, game: &mut ChessGame) {
    match game.game_state {
        GameState::Menu => render_menu(f, game),
        _ => render_game(f, game),
    }
} 