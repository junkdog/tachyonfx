
use ratatui::style::Color as RatatuiColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn opposite(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    pub fn to_ratatui_color(self) -> RatatuiColor {
        match self {
            Color::White => RatatuiColor::White,
            Color::Black => RatatuiColor::Gray,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceType {
    King,
    Queen,
    Rook,
    Bishop,
    Knight,
    Pawn,
}

impl PieceType {
    pub fn symbol(self, color: Color) -> &'static str {
        match (self, color) {
            (PieceType::King, Color::White) => "♔",
            (PieceType::King, Color::Black) => "♚",
            (PieceType::Queen, Color::White) => "♕",
            (PieceType::Queen, Color::Black) => "♛",
            (PieceType::Rook, Color::White) => "♖",
            (PieceType::Rook, Color::Black) => "♜",
            (PieceType::Bishop, Color::White) => "♗",
            (PieceType::Bishop, Color::Black) => "♝",
            (PieceType::Knight, Color::White) => "♘",
            (PieceType::Knight, Color::Black) => "♞",
            (PieceType::Pawn, Color::White) => "♙",
            (PieceType::Pawn, Color::Black) => "♟",
        }
    }

    pub fn value(self) -> i32 {
        match self {
            PieceType::Pawn => 1,
            PieceType::Knight | PieceType::Bishop => 3,
            PieceType::Rook => 5,
            PieceType::Queen => 9,
            PieceType::King => 1000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChessPiece {
    pub piece_type: PieceType,
    pub color: Color,
    pub has_moved: bool,
}

impl ChessPiece {
    pub fn new(piece_type: PieceType, color: Color) -> Self {
        Self {
            piece_type,
            color,
            has_moved: false,
        }
    }

    pub fn symbol(self) -> &'static str {
        self.piece_type.symbol(self.color)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChessMove {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub piece: ChessPiece,
    pub captured: Option<ChessPiece>,
    pub is_castling: bool,
    pub is_en_passant: bool,
    pub promotion: Option<PieceType>,
}

impl ChessMove {
    pub fn new(from: (usize, usize), to: (usize, usize), piece: ChessPiece) -> Self {
        Self {
            from,
            to,
            piece,
            captured: None,
            is_castling: false,
            is_en_passant: false,
            promotion: None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ChessBoard {
    pub squares: [[Option<ChessPiece>; 8]; 8],
    pub en_passant_target: Option<(usize, usize)>,
    pub white_king_pos: (usize, usize),
    pub black_king_pos: (usize, usize),
}

impl ChessBoard {
    pub fn new() -> Self {
        let mut board = Self {
            squares: [[None; 8]; 8],
            en_passant_target: None,
            white_king_pos: (4, 7),
            black_king_pos: (4, 0),
        };
        board.setup_initial_position();
        board
    }
    
    // Create a simple test position where white king is in check
    pub fn new_check_test() -> Self {
        let mut board = Self {
            squares: [[None; 8]; 8],
            en_passant_target: None,
            white_king_pos: (4, 4), // King in center
            black_king_pos: (0, 0), // Black king in corner
        };
        
        // Place white king in center
        board.squares[4][4] = Some(ChessPiece::new(PieceType::King, Color::White));
        
        // Place black rook attacking the king
        board.squares[4][0] = Some(ChessPiece::new(PieceType::Rook, Color::Black));
        
        // Place white rook that can block
        board.squares[1][1] = Some(ChessPiece::new(PieceType::Rook, Color::White));
        
        // Place black king
        board.squares[0][0] = Some(ChessPiece::new(PieceType::King, Color::Black));
        
        
        board
    }

    fn setup_initial_position(&mut self) {
        // Black pieces
        self.squares[0][0] = Some(ChessPiece::new(PieceType::Rook, Color::Black));
        self.squares[1][0] = Some(ChessPiece::new(PieceType::Knight, Color::Black));
        self.squares[2][0] = Some(ChessPiece::new(PieceType::Bishop, Color::Black));
        self.squares[3][0] = Some(ChessPiece::new(PieceType::Queen, Color::Black));
        self.squares[4][0] = Some(ChessPiece::new(PieceType::King, Color::Black));
        self.squares[5][0] = Some(ChessPiece::new(PieceType::Bishop, Color::Black));
        self.squares[6][0] = Some(ChessPiece::new(PieceType::Knight, Color::Black));
        self.squares[7][0] = Some(ChessPiece::new(PieceType::Rook, Color::Black));

        for x in 0..8 {
            self.squares[x][1] = Some(ChessPiece::new(PieceType::Pawn, Color::Black));
        }

        // White pieces
        self.squares[0][7] = Some(ChessPiece::new(PieceType::Rook, Color::White));
        self.squares[1][7] = Some(ChessPiece::new(PieceType::Knight, Color::White));
        self.squares[2][7] = Some(ChessPiece::new(PieceType::Bishop, Color::White));
        self.squares[3][7] = Some(ChessPiece::new(PieceType::Queen, Color::White));
        self.squares[4][7] = Some(ChessPiece::new(PieceType::King, Color::White));
        self.squares[5][7] = Some(ChessPiece::new(PieceType::Bishop, Color::White));
        self.squares[6][7] = Some(ChessPiece::new(PieceType::Knight, Color::White));
        self.squares[7][7] = Some(ChessPiece::new(PieceType::Rook, Color::White));

        for x in 0..8 {
            self.squares[x][6] = Some(ChessPiece::new(PieceType::Pawn, Color::White));
        }
    }

    pub fn get_piece(&self, pos: (usize, usize)) -> Option<ChessPiece> {
        if pos.0 < 8 && pos.1 < 8 {
            self.squares[pos.0][pos.1]
        } else {
            None
        }
    }

    pub fn set_piece(&mut self, pos: (usize, usize), piece: Option<ChessPiece>) {
        if pos.0 < 8 && pos.1 < 8 {
            self.squares[pos.0][pos.1] = piece;
        }
    }

    pub fn make_move(&mut self, chess_move: &ChessMove) -> Option<ChessPiece> {
        let captured = self.get_piece(chess_move.to);
        
        // Move the piece
        self.set_piece(chess_move.to, Some(chess_move.piece));
        self.set_piece(chess_move.from, None);

        // Update king positions
        if chess_move.piece.piece_type == PieceType::King {
            match chess_move.piece.color {
                Color::White => self.white_king_pos = chess_move.to,
                Color::Black => self.black_king_pos = chess_move.to,
            }
        }

        // Mark piece as moved
        if let Some(ref mut piece) = self.squares[chess_move.to.0][chess_move.to.1] {
            piece.has_moved = true;
        }

        captured
    }

    pub fn get_valid_moves(&self, pos: (usize, usize), color: Color) -> Vec<(usize, usize)> {
        let mut moves = Vec::new();
        
        if let Some(piece) = self.get_piece(pos) {
            if piece.color != color {
                return moves;
            }

            match piece.piece_type {
                PieceType::Pawn => moves.extend(self.get_pawn_moves(pos, piece.color)),
                PieceType::Rook => moves.extend(self.get_rook_moves(pos, piece.color)),
                PieceType::Bishop => moves.extend(self.get_bishop_moves(pos, piece.color)),
                PieceType::Queen => moves.extend(self.get_queen_moves(pos, piece.color)),
                PieceType::King => moves.extend(self.get_king_moves(pos, piece.color)),
                PieceType::Knight => moves.extend(self.get_knight_moves(pos, piece.color)),
            }
        }

        // Filter out moves that would leave king in check
        let valid_moves: Vec<(usize, usize)> = moves.into_iter()
            .filter(|&to| {
                // Simulate the move and check if king would still be in check
                let mut temp_board = *self;
                if let Some(piece) = temp_board.get_piece(pos) {
                    temp_board.set_piece(pos, None);
                    temp_board.set_piece(to, Some(piece));
                    
                    // Update king position if moving king
                    if piece.piece_type == PieceType::King {
                        match color {
                            Color::White => temp_board.white_king_pos = to,
                            Color::Black => temp_board.black_king_pos = to,
                        }
                    }
                    
                    // Move is valid if it doesn't leave king in check
                    !temp_board.is_in_check(color)
                } else {
                    false
                }
            })
            .collect();
                
        valid_moves
    }

    fn get_pawn_moves(&self, pos: (usize, usize), color: Color) -> Vec<(usize, usize)> {
        let mut moves = Vec::new();
        let (x, y) = pos;
        let direction = if color == Color::White { -1i32 } else { 1i32 };
        let start_row = if color == Color::White { 6 } else { 1 };

        // Forward move
        let new_y = (y as i32 + direction) as usize;
        if new_y < 8 && self.get_piece((x, new_y)).is_none() {
            moves.push((x, new_y));

            // Double move from start position
            if y == start_row {
                let double_y = (y as i32 + 2 * direction) as usize;
                if double_y < 8 && self.get_piece((x, double_y)).is_none() {
                    moves.push((x, double_y));
                }
            }
        }

        // Captures
        for dx in [-1, 1] {
            let new_x = x as i32 + dx;
            let new_y = y as i32 + direction;
            if new_x >= 0 && new_x < 8 && new_y >= 0 && new_y < 8 {
                let new_pos = (new_x as usize, new_y as usize);
                if let Some(piece) = self.get_piece(new_pos) {
                    if piece.color != color {
                        moves.push(new_pos);
                    }
                }
            }
        }

        moves
    }

    fn get_rook_moves(&self, pos: (usize, usize), color: Color) -> Vec<(usize, usize)> {
        let mut moves = Vec::new();
        let directions = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        
        for (dx, dy) in directions.iter() {
            moves.extend(self.get_sliding_moves(pos, *dx, *dy, color));
        }
        
        moves
    }

    fn get_bishop_moves(&self, pos: (usize, usize), color: Color) -> Vec<(usize, usize)> {
        let mut moves = Vec::new();
        let directions = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
        
        for (dx, dy) in directions.iter() {
            moves.extend(self.get_sliding_moves(pos, *dx, *dy, color));
        }
        
        moves
    }

    fn get_queen_moves(&self, pos: (usize, usize), color: Color) -> Vec<(usize, usize)> {
        let mut moves = self.get_rook_moves(pos, color);
        moves.extend(self.get_bishop_moves(pos, color));
        moves
    }

    fn get_king_moves(&self, pos: (usize, usize), color: Color) -> Vec<(usize, usize)> {
        let mut moves = Vec::new();
        let (x, y) = pos;
        
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 { continue; }
                
                let new_x = x as i32 + dx;
                let new_y = y as i32 + dy;
                
                if new_x >= 0 && new_x < 8 && new_y >= 0 && new_y < 8 {
                    let new_pos = (new_x as usize, new_y as usize);
                    
                    if let Some(piece) = self.get_piece(new_pos) {
                        if piece.color != color {
                            moves.push(new_pos);
                        }
                    } else {
                        moves.push(new_pos);
                    }
                }
            }
        }
        
        moves
    }

    fn get_knight_moves(&self, pos: (usize, usize), color: Color) -> Vec<(usize, usize)> {
        let mut moves = Vec::new();
        let (x, y) = pos;
        let knight_moves = [
            (2, 1), (2, -1), (-2, 1), (-2, -1),
            (1, 2), (1, -2), (-1, 2), (-1, -2)
        ];
        
        for (dx, dy) in knight_moves.iter() {
            let new_x = x as i32 + dx;
            let new_y = y as i32 + dy;
            
            if new_x >= 0 && new_x < 8 && new_y >= 0 && new_y < 8 {
                let new_pos = (new_x as usize, new_y as usize);
                
                if let Some(piece) = self.get_piece(new_pos) {
                    if piece.color != color {
                        moves.push(new_pos);
                    }
                } else {
                    moves.push(new_pos);
                }
            }
        }
        
        moves
    }

    fn get_sliding_moves(&self, pos: (usize, usize), dx: i32, dy: i32, color: Color) -> Vec<(usize, usize)> {
        let mut moves = Vec::new();
        let (mut x, mut y) = (pos.0 as i32, pos.1 as i32);
        
        loop {
            x += dx;
            y += dy;
            
            if x < 0 || x >= 8 || y < 0 || y >= 8 {
                break;
            }
            
            let new_pos = (x as usize, y as usize);
            
            if let Some(piece) = self.get_piece(new_pos) {
                if piece.color != color {
                    moves.push(new_pos);
                }
                break;
            } else {
                moves.push(new_pos);
            }
        }
        
        moves
    }

    fn would_be_in_check_after_move(&self, from: (usize, usize), to: (usize, usize), color: Color) -> bool {
        // Create a temporary board state to test the move
        let mut temp_board = *self;
        
        // Get the piece we're trying to move
        let piece = match temp_board.get_piece(from) {
            Some(p) => p,
            None => return false, // No piece to move
        };
        
        // Ensure the piece belongs to the current player
        if piece.color != color {
            return false;
        }
        
        // Make the move on the temporary board
        temp_board.set_piece(from, None);          // Remove piece from source
        temp_board.set_piece(to, Some(piece));     // Place piece at destination
        
        // Update king position if we're moving the king
        if piece.piece_type == PieceType::King {
            match color {
                Color::White => temp_board.white_king_pos = to,
                Color::Black => temp_board.black_king_pos = to,
            }
        }
        
        // Test if the king would still be in check after this move
        temp_board.is_in_check(color)
    }

    pub fn is_in_check(&self, color: Color) -> bool {
        let king_pos = match color {
            Color::White => self.white_king_pos,
            Color::Black => self.black_king_pos,
        };

        // Check if any enemy piece can attack the king
        for x in 0..8 {
            for y in 0..8 {
                if let Some(piece) = self.get_piece((x, y)) {
                    if piece.color != color {
                        let attacks = self.get_piece_attacks((x, y), piece.color);
                        if attacks.contains(&king_pos) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn get_piece_attacks(&self, pos: (usize, usize), _color: Color) -> Vec<(usize, usize)> {
        // Calculate raw attacks for check detection - use piece's actual color
        let mut attacks = Vec::new();
        
        if let Some(piece) = self.get_piece(pos) {
            match piece.piece_type {
                PieceType::Pawn => attacks.extend(self.get_pawn_attacks(pos, piece.color)),
                PieceType::Rook => attacks.extend(self.get_rook_moves(pos, piece.color)),
                PieceType::Bishop => attacks.extend(self.get_bishop_moves(pos, piece.color)),
                PieceType::Queen => attacks.extend(self.get_queen_moves(pos, piece.color)),
                PieceType::King => attacks.extend(self.get_king_attacks(pos, piece.color)),
                PieceType::Knight => attacks.extend(self.get_knight_moves(pos, piece.color)),
            }
        }

        attacks
    }

    fn get_pawn_attacks(&self, pos: (usize, usize), color: Color) -> Vec<(usize, usize)> {
        let mut attacks = Vec::new();
        let (x, y) = pos;
        let direction = if color == Color::White { -1i32 } else { 1i32 };

        // Pawn attacks diagonally
        for dx in [-1, 1] {
            let new_x = x as i32 + dx;
            let new_y = y as i32 + direction;
            if new_x >= 0 && new_x < 8 && new_y >= 0 && new_y < 8 {
                attacks.push((new_x as usize, new_y as usize));
            }
        }

        attacks
    }

    fn get_king_attacks(&self, pos: (usize, usize), _color: Color) -> Vec<(usize, usize)> {
        let mut attacks = Vec::new();
        let (x, y) = pos;
        
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 { continue; }
                
                let new_x = x as i32 + dx;
                let new_y = y as i32 + dy;
                
                if new_x >= 0 && new_x < 8 && new_y >= 0 && new_y < 8 {
                    attacks.push((new_x as usize, new_y as usize));
                }
            }
        }
        
        attacks
    }

    pub fn get_all_valid_moves(&self, color: Color) -> Vec<ChessMove> {
        let mut moves = Vec::new();
        
        for x in 0..8 {
            for y in 0..8 {
                if let Some(piece) = self.get_piece((x, y)) {
                    if piece.color == color {
                        let valid_moves = self.get_valid_moves((x, y), color);
                        for to in valid_moves {
                            moves.push(ChessMove::new((x, y), to, piece));
                        }
                    }
                }
            }
        }
        
        moves
    }
}

// Simple AI implementation
pub struct SimpleAI {
    pub color: Color,
}

impl SimpleAI {
    pub fn new(color: Color) -> Self {
        Self { color }
    }

    pub fn get_move(&mut self, board: &ChessBoard) -> Option<ChessMove> {
        let moves = board.get_all_valid_moves(self.color);
        if moves.is_empty() {
            return None;
        }

        // Simple evaluation: prioritize captures
        let mut best_move = &moves[0];
        let mut best_score = -1000;

        for chess_move in &moves {
            let mut score = 0;
            
            // Capture bonus
            if let Some(captured) = board.get_piece(chess_move.to) {
                score += captured.piece_type.value();
            }

            // Simple piece-square table bonus for center control
            let (x, y) = chess_move.to;
            if x >= 2 && x <= 5 && y >= 2 && y <= 5 {
                score += 1;
            }

            if score > best_score {
                best_score = score;
                best_move = chess_move;
            }
        }

        Some(*best_move)
    }
} 