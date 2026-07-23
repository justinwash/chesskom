//! Board rendering for the terminal. Kept separate from input handling so the
//! layout logic can be reused/adapted when we build the e-ink renderer later.

use chess_core::{Color, Game, Move, Piece, PieceKind, Position, Square, Status};

/// Unicode glyph for a piece. White pieces use the "white" chess glyphs; on a
/// dark terminal these read as outlines, which is fine and matches e-ink later.
fn glyph(piece: Piece) -> char {
    match (piece.color, piece.kind) {
        (Color::White, PieceKind::King) => '\u{2654}',
        (Color::White, PieceKind::Queen) => '\u{2655}',
        (Color::White, PieceKind::Rook) => '\u{2656}',
        (Color::White, PieceKind::Bishop) => '\u{2657}',
        (Color::White, PieceKind::Knight) => '\u{2658}',
        (Color::White, PieceKind::Pawn) => '\u{2659}',
        (Color::Black, PieceKind::King) => '\u{265A}',
        (Color::Black, PieceKind::Queen) => '\u{265B}',
        (Color::Black, PieceKind::Rook) => '\u{265C}',
        (Color::Black, PieceKind::Bishop) => '\u{265D}',
        (Color::Black, PieceKind::Knight) => '\u{265E}',
        (Color::Black, PieceKind::Pawn) => '\u{265F}',
    }
}

/// Render the viewed position as an 8x8 grid, oriented for the given viewpoint.
/// Highlights the squares of the last move that produced the viewed position.
pub fn board(pos: &Position, orient: Color, last: Option<Move>) -> String {
    let (from_hl, to_hl) = match last {
        Some(m) => (Some(m.from), Some(m.to)),
        None => (None, None),
    };

    let ranks: Vec<i8> = match orient {
        Color::White => (0..8).rev().collect(),
        Color::Black => (0..8).collect(),
    };
    let files: Vec<i8> = match orient {
        Color::White => (0..8).collect(),
        Color::Black => (0..8).rev().collect(),
    };

    let mut out = String::new();
    out.push_str("  +------------------------+\n");
    for &r in &ranks {
        out.push((b'1' + r as u8) as char);
        out.push(' ');
        out.push('|');
        for &f in &files {
            let sq = Square::from_file_rank(f, r).unwrap();
            let ch = match pos.piece_at(sq) {
                Some(p) => glyph(p),
                None => {
                    // Checkerboard using middot / space for empty squares.
                    if (f + r) % 2 == 0 {
                        '\u{00B7}'
                    } else {
                        ' '
                    }
                }
            };
            let marked = Some(sq) == from_hl || Some(sq) == to_hl;
            if marked {
                out.push('[');
                out.push(ch);
                out.push(']');
            } else {
                out.push(' ');
                out.push(ch);
                out.push(' ');
            }
        }
        out.push('|');
        out.push('\n');
    }
    out.push_str("  +------------------------+\n");
    out.push_str("   ");
    for &f in &files {
        out.push(' ');
        out.push((b'a' + f as u8) as char);
        out.push(' ');
    }
    out.push('\n');
    out
}

/// A one-line status/prompt describing whose turn it is and any game result.
pub fn status_line(game: &Game) -> String {
    match game.status() {
        Status::Ongoing => {
            let side = match game.current().side_to_move {
                Color::White => "White",
                Color::Black => "Black",
            };
            let check = if game.current().is_in_check() {
                "  (check!)"
            } else {
                ""
            };
            format!("{side} to move{check}")
        }
        Status::Checkmate { winner } => {
            let w = match winner {
                Color::White => "White",
                Color::Black => "Black",
            };
            format!("Checkmate — {w} wins")
        }
        Status::Stalemate => "Draw — stalemate".to_string(),
        Status::FiftyMoveRule => "Draw — fifty-move rule".to_string(),
        Status::ThreefoldRepetition => "Draw — threefold repetition".to_string(),
        Status::InsufficientMaterial => "Draw — insufficient material".to_string(),
    }
}

/// A compact move list (pairs of half-moves), for the sidebar/footer.
pub fn move_list(game: &Game) -> String {
    let mut out = String::new();
    let moves = game.moves();
    for (i, mv) in moves.iter().enumerate() {
        if i % 2 == 0 {
            out.push_str(&format!("{}. ", i / 2 + 1));
        }
        out.push_str(&mv.to_uci());
        out.push(' ');
    }
    out
}
