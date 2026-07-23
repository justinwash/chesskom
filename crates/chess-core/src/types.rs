//! Fundamental chess value types: colors, pieces, and squares.

use core::fmt;

/// The two sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Black,
}

impl Color {
    #[inline]
    pub fn opponent(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    /// Direction a pawn of this color advances along the rank axis (+1 for white, -1 for black).
    #[inline]
    pub fn pawn_dir(self) -> i8 {
        match self {
            Color::White => 1,
            Color::Black => -1,
        }
    }
}

/// The six piece kinds, colorless.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceKind {
    /// Lowercase letter used in FEN / algebraic notation (`p`, `n`, `b`, `r`, `q`, `k`).
    pub fn to_char(self) -> char {
        match self {
            PieceKind::Pawn => 'p',
            PieceKind::Knight => 'n',
            PieceKind::Bishop => 'b',
            PieceKind::Rook => 'r',
            PieceKind::Queen => 'q',
            PieceKind::King => 'k',
        }
    }

    pub fn from_char(c: char) -> Option<PieceKind> {
        Some(match c.to_ascii_lowercase() {
            'p' => PieceKind::Pawn,
            'n' => PieceKind::Knight,
            'b' => PieceKind::Bishop,
            'r' => PieceKind::Rook,
            'q' => PieceKind::Queen,
            'k' => PieceKind::King,
            _ => return None,
        })
    }
}

/// A colored piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceKind,
}

impl Piece {
    pub fn new(color: Color, kind: PieceKind) -> Piece {
        Piece { color, kind }
    }

    /// FEN letter: uppercase for white, lowercase for black.
    pub fn to_char(self) -> char {
        let c = self.kind.to_char();
        match self.color {
            Color::White => c.to_ascii_uppercase(),
            Color::Black => c,
        }
    }

    pub fn from_char(c: char) -> Option<Piece> {
        let kind = PieceKind::from_char(c)?;
        let color = if c.is_ascii_uppercase() {
            Color::White
        } else {
            Color::Black
        };
        Some(Piece { color, kind })
    }
}

/// A board square, indexed 0..=63 with `a1 = 0`, `b1 = 1`, ..., `h8 = 63`.
///
/// `rank = index / 8` (0 = rank 1), `file = index % 8` (0 = file a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Square(pub u8);

impl Square {
    /// Build from file (0=a..7=h) and rank (0=rank1..7=rank8). Returns `None` if out of range.
    #[inline]
    pub fn from_file_rank(file: i8, rank: i8) -> Option<Square> {
        if (0..8).contains(&file) && (0..8).contains(&rank) {
            Some(Square((rank * 8 + file) as u8))
        } else {
            None
        }
    }

    #[inline]
    pub fn file(self) -> i8 {
        (self.0 % 8) as i8
    }

    #[inline]
    pub fn rank(self) -> i8 {
        (self.0 / 8) as i8
    }

    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// Parse algebraic square coordinate like `e4`.
    pub fn from_algebraic(s: &str) -> Option<Square> {
        let mut chars = s.chars();
        let file_c = chars.next()?;
        let rank_c = chars.next()?;
        if chars.next().is_some() {
            return None;
        }
        let file = (file_c as i32) - ('a' as i32);
        let rank = (rank_c as i32) - ('1' as i32);
        Square::from_file_rank(file as i8, rank as i8)
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = (b'a' + self.file() as u8) as char;
        let rank = (b'1' + self.rank() as u8) as char;
        write!(f, "{}{}", file, rank)
    }
}
