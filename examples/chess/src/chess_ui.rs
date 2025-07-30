use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use tachyonfx::{EffectRenderer, Duration};

use crate::{ChessGame, GameState, GameMode};
use crate::chess_engine::{Color as ChessColor};
use crate::chess_effects::{EffectManager};

pub fn render_menu(f: &mut Frame, _game: &mut ChessGame) {
    let area = f.area();
    
    // Create menu background effects
    let menu_effects = EffectManager::create_menu_effects();
    for (i, mut effect) in menu_effects.into_iter().enumerate() {
        let effect_area = if i == 0 {
            // Matrix rain in background
            area
        } else {
            // Color shifting overlay
            area.inner(Margin::new(2, 1))
        };
        f.render_effect(&mut effect, effect_area, Duration::from_millis(16));
    }

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(area);

    // Title
    let title = create_title_widget();
    f.render_widget(title, chunks[0]);

    // Menu options
    let menu = create_menu_widget();
    f.render_widget(menu, chunks[1]);

    // Instructions
    let instructions = create_instructions_widget();
    f.render_widget(instructions, chunks[2]);
}

pub fn render_game(f: &mut Frame, game: &mut ChessGame) {
    let area = f.area();
    
    // Create main layout
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(75), // Game area
            Constraint::Percentage(25), // Side panel
        ])
        .split(area);

    let game_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Status bar
            Constraint::Min(16),        // Chess board
            Constraint::Length(3),      // Controls
        ])
        .split(main_chunks[0]);

    // Render background effects
    render_background_effects(f, &mut game.effects, area);

    // Render status bar
    render_status_bar(f, game, game_chunks[0]);

    // Render chess board
    render_chess_board(f, game, game_chunks[1]);

    // Render controls
    render_controls(f, game, game_chunks[2]);

    // Render side panel
    render_side_panel(f, game, main_chunks[1]);

    // Render game state effects (check, checkmate, etc.)
    render_game_state_effects(f, game, area);
}

fn render_background_effects(f: &mut Frame, effects: &mut EffectManager, area: Rect) {
    for mut effect in effects.get_background_effects().clone() {
        f.render_effect(&mut effect, area, Duration::from_millis(16));
    }
}

fn render_status_bar(f: &mut Frame, game: &ChessGame, area: Rect) {
    let current_player = match game.current_player {
        ChessColor::White => "White",
        ChessColor::Black => "Black",
    };

    let game_mode = match game.game_mode {
        GameMode::SinglePlayer => "vs AI",
        GameMode::Multiplayer => "Multiplayer",
        GameMode::Menu => "Menu",
    };

    let status_text = match &game.game_state {
        GameState::Playing => format!("{}'s Turn - {}", current_player, game_mode),
        GameState::Check(color) => format!("{:?} King in Check!", color),
        GameState::Checkmate(color) => format!("Checkmate! {:?} Wins!", color.opposite()),
        GameState::Stalemate => "Stalemate - Draw!".to_string(),
        GameState::Menu => "Main Menu".to_string(),
    };

    let status_color = match &game.game_state {
        GameState::Check(_) => Color::Red,
        GameState::Checkmate(_) => Color::Red,
        GameState::Stalemate => Color::Yellow,
        _ => match game.current_player {
            ChessColor::White => Color::White,
            ChessColor::Black => Color::Gray,
        }
    };

    let status_widget = Paragraph::new(status_text)
        .style(Style::default().fg(status_color).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title("Game Status")
        );

    f.render_widget(status_widget, area);
}

fn render_chess_board(f: &mut Frame, game: &mut ChessGame, area: Rect) {
    // Adaptive square size based on available space
    let max_square_width = (area.width.saturating_sub(10)) / 8; // Leave space for labels
    let max_square_height = (area.height.saturating_sub(6)) / 8; // Leave space for labels
    
    let square_width = std::cmp::min(5, max_square_width).max(3); // 3-5 chars wide
    let square_height = std::cmp::min(3, max_square_height).max(1); // 1-3 chars tall
    
    // Calculate board dimensions based on adaptive squares
    let board_width = 8 * square_width + 2; // +2 for border
    let board_height = 8 * square_height + 2; // +2 for border
    
    let board_area = Rect {
        x: area.x + (area.width.saturating_sub(board_width)) / 2,
        y: area.y + (area.height.saturating_sub(board_height)) / 2,
        width: board_width,
        height: board_height,
    };

    // Clear the board area with dark background
    let background = Block::default()
        .style(Style::default().bg(Color::Black));
    f.render_widget(background, board_area);

    // Render board squares and pieces
    for row in 0..8 {
        for col in 0..8 {
            let square_area = Rect {
                x: board_area.x + 1 + col as u16 * square_width, // +1 for border
                y: board_area.y + 1 + row as u16 * square_height, // +1 for border
                width: square_width,
                height: square_height,
            };

            render_board_square(f, game, (col, row), square_area, square_width, square_height);
        }
    }

    // Render board border with a thicker, more impressive border
    let border = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .title(" ♔ TACHYONFX CHESS ♚ ");
    f.render_widget(border, board_area);

    // Render coordinate labels
    render_board_coordinates(f, board_area, square_width, square_height);
}

fn render_board_square(f: &mut Frame, game: &mut ChessGame, pos: (usize, usize), area: Rect, square_width: u16, square_height: u16) {
    let (col, row) = pos;
    
    // Determine square color
    let is_light_square = (row + col) % 2 == 0;
    let base_bg_color = if is_light_square {
        Color::Rgb(240, 217, 181) // Light squares
    } else {
        Color::Rgb(181, 136, 99)  // Dark squares
    };

    let mut square_style = Style::default().bg(base_bg_color);
    
    // Apply selection highlighting
    if game.selected_square == Some(pos) {
        square_style = square_style.bg(Color::LightGreen);
    } else if game.valid_moves.contains(&pos) {
        square_style = square_style.bg(Color::LightBlue);
    } else if game.cursor_position == pos {
        square_style = square_style.bg(Color::Yellow);
    }

    // Get piece symbol
    let piece_symbol = game.board.get_piece(pos)
        .map(|piece| piece.symbol())
        .unwrap_or(" ");

    let piece_color = game.board.get_piece(pos)
        .map(|piece| match piece.color {
            ChessColor::White => Color::White,
            ChessColor::Black => Color::Black,
        })
        .unwrap_or(Color::Gray);

    // Create the square widget with adaptive formatting
    let square_content = if piece_symbol == " " {
        // Empty square - create pattern based on square size
        let empty_line = "·".repeat(square_width as usize);
        let mut lines = Vec::new();
        for _ in 0..square_height {
            lines.push(Line::from(empty_line.clone()));
        }
        Text::from(lines)
    } else if square_height >= 3 && square_width >= 5 {
        // Large squares - use border
        let top_line = "╔".to_string() + &"═".repeat(square_width as usize - 2) + "╗";
        let middle_padding = (square_width as usize - 3) / 2;
        let middle_line = format!("║{}{}{}║", 
            " ".repeat(middle_padding),
            piece_symbol,
            " ".repeat(square_width as usize - 2 - middle_padding));
        let bottom_line = "╚".to_string() + &"═".repeat(square_width as usize - 2) + "╝";
        
        let mut lines = vec![Line::from(top_line)];
        for i in 1..square_height - 1 {
            if i == square_height / 2 {
                lines.push(Line::from(middle_line.clone()));
            } else {
                let empty_border = format!("║{}║", " ".repeat(square_width as usize - 2));
                lines.push(Line::from(empty_border));
            }
        }
        lines.push(Line::from(bottom_line));
        Text::from(lines)
    } else {
        // Small squares - just center the piece
        let padding = (square_width as usize).saturating_sub(1) / 2;
        let content_line = format!("{}{}{}", 
            " ".repeat(padding),
            piece_symbol,
            " ".repeat(square_width as usize - padding - 1));
        
        let mut lines = Vec::new();
        for i in 0..square_height {
            if i == square_height / 2 {
                lines.push(Line::from(content_line.clone()));
            } else {
                lines.push(Line::from(" ".repeat(square_width as usize)));
            }
        }
        Text::from(lines)
    };
    
    let square_widget = Paragraph::new(square_content)
        .style(square_style.fg(piece_color).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);

    f.render_widget(square_widget, area);

    // Apply tachyonfx effects for this square
    render_square_effects(f, game, pos, area);
}

fn render_square_effects(f: &mut Frame, game: &mut ChessGame, pos: (usize, usize), area: Rect) {
    // Render selection effect
    if game.selected_square == Some(pos) {
        if let Some(mut effect) = game.effects.get_selection_effect() {
            f.render_effect(&mut effect, area, Duration::from_millis(16));
        }
    }

    // Render valid move hints
    if game.valid_moves.contains(&pos) {
        let valid_move_effects = game.effects.get_valid_move_effects();
        for mut effect in valid_move_effects {
            f.render_effect(&mut effect, area, Duration::from_millis(16));
        }
    }

    // Render check warning
    if let GameState::Check(color) = &game.game_state {
        let king_pos = match color {
            ChessColor::White => game.board.white_king_pos,
            ChessColor::Black => game.board.black_king_pos,
        };
        if pos == king_pos {
            if let Some(mut effect) = game.effects.get_check_effect() {
                f.render_effect(&mut effect, area, Duration::from_millis(16));
            }
        }
    }

    // Render active effects (explosions, trails, etc.)
    for effect_instance in game.effects.get_active_effects() {
        if effect_instance.position == pos {
            let mut effect = effect_instance.effect.clone();
            f.render_effect(&mut effect, area, Duration::from_millis(16));
        }
    }
}

fn render_board_coordinates(f: &mut Frame, board_area: Rect, square_width: u16, square_height: u16) {
    // Only render coordinates if there's space
    let screen_area = f.area();
    
    // Render file labels (a-h) - positioned below the board
    if board_area.y + board_area.height < screen_area.height {
        for col in 0..8 {
            let file_label = ((b'a' + col as u8) as char).to_string();
            let x_pos = board_area.x + 1 + col as u16 * square_width + square_width / 2;
            if x_pos < screen_area.width.saturating_sub(2) {
                let label_area = Rect {
                    x: x_pos.saturating_sub(1),
                    y: board_area.y + board_area.height,
                    width: 2,
                    height: 1,
                };
                if label_area.y < screen_area.height {
                    let label_widget = Paragraph::new(file_label)
                        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                        .alignment(Alignment::Center);
                    f.render_widget(label_widget, label_area);
                }
            }
        }
    }

    // Render rank labels (1-8) - positioned to the left of the board
    if board_area.x >= 3 {
        for row in 0..8 {
            let rank_label = (8 - row).to_string();
            let y_pos = board_area.y + 1 + row as u16 * square_height + square_height / 2;
            if y_pos < screen_area.height {
                let label_area = Rect {
                    x: board_area.x.saturating_sub(3),
                    y: y_pos,
                    width: 2,
                    height: 1,
                };
                let label_widget = Paragraph::new(rank_label)
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    .alignment(Alignment::Center);
                f.render_widget(label_widget, label_area);
            }
        }
    }
}

fn render_controls(f: &mut Frame, game: &ChessGame, area: Rect) {
    let controls_text = match game.game_state {
        GameState::Menu => "Press 1 for Single Player, 2 for Multiplayer, ESC to quit",
        GameState::Playing => "Arrow keys: Move cursor, ENTER/Space: Select, R: Restart, ESC: Menu",
        _ => "Press R to restart, ENTER to return to menu",
    };

    let controls_widget = Paragraph::new(controls_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
                .title("Controls")
        );

    f.render_widget(controls_widget, area);
}

fn render_side_panel(f: &mut Frame, game: &ChessGame, area: Rect) {
    let side_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // Captured pieces
            Constraint::Percentage(40), // Move history
            Constraint::Percentage(20), // Game info
        ])
        .split(area);

    // Render captured pieces
    render_captured_pieces(f, game, side_chunks[0]);

    // Render move history
    render_move_history(f, game, side_chunks[1]);

    // Render game info
    render_game_info(f, game, side_chunks[2]);
}

fn render_captured_pieces(f: &mut Frame, game: &ChessGame, area: Rect) {
    let white_captured: Vec<String> = game.captured_pieces
        .iter()
        .filter(|piece| piece.color == ChessColor::White)
        .map(|piece| piece.symbol().to_string())
        .collect();

    let black_captured: Vec<String> = game.captured_pieces
        .iter()
        .filter(|piece| piece.color == ChessColor::Black)
        .map(|piece| piece.symbol().to_string())
        .collect();

    let captured_text = Text::from(vec![
        Line::from(vec![
            Span::styled("White: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw(white_captured.join(" ")),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Black: ", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
            Span::raw(black_captured.join(" ")),
        ]),
    ]);

    let captured_widget = Paragraph::new(captured_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title("Captured Pieces")
        );

    f.render_widget(captured_widget, area);
}

fn render_move_history(f: &mut Frame, game: &ChessGame, area: Rect) {
    let moves: Vec<ListItem> = game.move_history
        .iter()
        .enumerate()
        .map(|(i, chess_move)| {
            let move_number = (i / 2) + 1;
            let color_indicator = if i % 2 == 0 { "♔" } else { "♚" };
            let from_square = format!("{}{}", 
                ((b'a' + chess_move.from.0 as u8) as char),
                8 - chess_move.from.1
            );
            let to_square = format!("{}{}", 
                ((b'a' + chess_move.to.0 as u8) as char),
                8 - chess_move.to.1
            );
            
            let move_text = if i % 2 == 0 {
                format!("{}. {} {}-{}", move_number, color_indicator, from_square, to_square)
            } else {
                format!("   {} {}-{}", color_indicator, from_square, to_square)
            };

            ListItem::new(move_text)
                .style(Style::default().fg(if i % 2 == 0 { Color::White } else { Color::Gray }))
        })
        .collect();

    let moves_widget = List::new(moves)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title("Move History")
        );

    f.render_widget(moves_widget, area);
}

fn render_game_info(f: &mut Frame, game: &ChessGame, area: Rect) {
    let move_count = game.move_history.len();
    let game_mode_text = match game.game_mode {
        GameMode::SinglePlayer => "Single Player",
        GameMode::Multiplayer => "Multiplayer",
        GameMode::Menu => "Menu",
    };

    let info_text = Text::from(vec![
        Line::from(vec![
            Span::styled("Mode: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(game_mode_text),
        ]),
        Line::from(vec![
            Span::styled("Moves: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(move_count.to_string()),
        ]),
    ]);

    let info_widget = Paragraph::new(info_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta))
                .title("Game Info")
        );

    f.render_widget(info_widget, area);
}

fn render_game_state_effects(f: &mut Frame, game: &mut ChessGame, area: Rect) {
    match &game.game_state {
        GameState::Checkmate(_) => {
            let victory_effects = EffectManager::create_victory_effects();
            for mut effect in victory_effects {
                f.render_effect(&mut effect, area, Duration::from_millis(16));
            }
        },
        GameState::Stalemate => {
            let game_over_effects = EffectManager::create_game_over_effects();
            for mut effect in game_over_effects {
                f.render_effect(&mut effect, area, Duration::from_millis(16));
            }
        },
        _ => {}
    }
}

fn create_title_widget() -> Paragraph<'static> {
    let title_text = Text::from(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("♔ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("TACHYON", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled("FX", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::styled(" CHESS ♚", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]).alignment(Alignment::Center),
        Line::from(""),
        Line::from(Span::styled(
            "Epic Animated Terminal Chess", 
            Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC)
        )).alignment(Alignment::Center),
    ]);

    Paragraph::new(title_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta))
                .title("Welcome")
        )
}

fn create_menu_widget() -> Paragraph<'static> {
    let menu_text = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            "1. Single Player (vs AI)", 
            Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)
        )).alignment(Alignment::Center),
        Line::from(""),
        Line::from(Span::styled(
            "2. Multiplayer (Local)", 
            Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD)
        )).alignment(Alignment::Center),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "Choose your adventure!", 
            Style::default().fg(Color::LightYellow).add_modifier(Modifier::ITALIC)
        )).alignment(Alignment::Center),
    ]);

    Paragraph::new(menu_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title("Game Mode")
        )
}

fn create_instructions_widget() -> Paragraph<'static> {
    let instructions_text = Text::from(vec![
        Line::from(Span::styled(
            "ESC: Quit Game", 
            Style::default().fg(Color::Red)
        )).alignment(Alignment::Center),
        Line::from(Span::styled(
            "Arrow Keys: Navigate • ENTER/Space: Select", 
            Style::default().fg(Color::Cyan)
        )).alignment(Alignment::Center),
    ]);

    Paragraph::new(instructions_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
                .title("Controls")
        )
} 