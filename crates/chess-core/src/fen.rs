//! FEN (Forsyth–Edwards Notation) parsing and serialization.
//!
//! We need this early: the chess.com Published-Data API returns ongoing daily
//! games as FEN + PGN, so the viewer milestone will lean on `parse_fen`.

use crate::position::{CastlingRights, Position};
use crate::types::{Color, Piece, Square};

/// Parse a full FEN string into a [`Position`].
pub fn parse_fen(fen: &str) -> Result<Position, FenError> {
    let mut fields = fen.split_whitespace();
    let placement = fields.next().ok_or(FenError::Missing("piece placement"))?;
    let active = fields.next().ok_or(FenError::Missing("active color"))?;
    let castling = fields.next().ok_or(FenError::Missing("castling"))?;
    let ep = fields.next().ok_or(FenError::Missing("en passant"))?;
    // Halfmove clock and fullmove number are optional (some sources omit them).
    let halfmove = fields.next().unwrap_or("0");
    let fullmove = fields.next().unwrap_or("1");

    let mut pos = Position::empty();

    // Piece placement is given rank 8 down to rank 1.
    let mut rank = 7i8;
    for row in placement.split('/') {
        let mut file = 0i8;
        for c in row.chars() {
            if let Some(skip) = c.to_digit(10) {
                file += skip as i8;
            } else {
                let piece = Piece::from_char(c).ok_or(FenError::BadPiece(c))?;
                let sq = Square::from_file_rank(file, rank).ok_or(FenError::OutOfRange)?;
                pos.set_piece(sq, Some(piece));
                file += 1;
            }
        }
        if file != 8 {
            return Err(FenError::BadRankWidth(rank));
        }
        rank -= 1;
    }
    if rank != -1 {
        return Err(FenError::BadRankCount);
    }

    pos.side_to_move = match active {
        "w" => Color::White,
        "b" => Color::Black,
        other => return Err(FenError::BadColor(other.to_string())),
    };

    pos.castling = CastlingRights {
        white_kingside: castling.contains('K'),
        white_queenside: castling.contains('Q'),
        black_kingside: castling.contains('k'),
        black_queenside: castling.contains('q'),
    };

    pos.en_passant = if ep == "-" {
        None
    } else {
        Some(Square::from_algebraic(ep).ok_or_else(|| FenError::BadSquare(ep.to_string()))?)
    };

    pos.halfmove_clock = halfmove.parse().unwrap_or(0);
    pos.fullmove_number = fullmove.parse().unwrap_or(1);

    Ok(pos)
}

/// Serialize a [`Position`] back to a FEN string.
pub fn to_fen(pos: &Position) -> String {
    let mut out = String::new();
    for rank in (0..8).rev() {
        let mut empty = 0;
        for file in 0..8 {
            let sq = Square::from_file_rank(file, rank).unwrap();
            match pos.piece_at(sq) {
                Some(p) => {
                    if empty > 0 {
                        out.push_str(&empty.to_string());
                        empty = 0;
                    }
                    out.push(p.to_char());
                }
                None => empty += 1,
            }
        }
        if empty > 0 {
            out.push_str(&empty.to_string());
        }
        if rank > 0 {
            out.push('/');
        }
    }

    out.push(' ');
    out.push(match pos.side_to_move {
        Color::White => 'w',
        Color::Black => 'b',
    });

    out.push(' ');
    let c = pos.castling;
    let mut rights = String::new();
    if c.white_kingside {
        rights.push('K');
    }
    if c.white_queenside {
        rights.push('Q');
    }
    if c.black_kingside {
        rights.push('k');
    }
    if c.black_queenside {
        rights.push('q');
    }
    if rights.is_empty() {
        out.push('-');
    } else {
        out.push_str(&rights);
    }

    out.push(' ');
    match pos.en_passant {
        Some(sq) => out.push_str(&sq.to_string()),
        None => out.push('-'),
    }

    out.push(' ');
    out.push_str(&pos.halfmove_clock.to_string());
    out.push(' ');
    out.push_str(&pos.fullmove_number.to_string());
    out
}

/// Reasons a FEN string can fail to parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FenError {
    Missing(&'static str),
    BadPiece(char),
    BadColor(String),
    BadSquare(String),
    BadRankWidth(i8),
    BadRankCount,
    OutOfRange,
}

impl core::fmt::Display for FenError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FenError::Missing(field) => write!(f, "missing FEN field: {field}"),
            FenError::BadPiece(c) => write!(f, "invalid piece char: {c}"),
            FenError::BadColor(s) => write!(f, "invalid active color: {s}"),
            FenError::BadSquare(s) => write!(f, "invalid square: {s}"),
            FenError::BadRankWidth(r) => write!(f, "rank {r} does not sum to 8 files"),
            FenError::BadRankCount => write!(f, "FEN must have exactly 8 ranks"),
            FenError::OutOfRange => write!(f, "square out of range"),
        }
    }
}

impl std::error::Error for FenError {}
