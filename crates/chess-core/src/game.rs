//! A playable game: a sequence of positions plus a viewing cursor for rewind.
//!
//! We keep a full snapshot of every position rather than an undo stack. Snapshots
//! are ~70 bytes each, so even a thousand-move game is trivial, and rewind becomes
//! a simple index into history with zero chance of undo bugs — which matters on a
//! device where we'd rather not debug subtle state corruption.

use crate::moves::Move;
use crate::position::Position;
use crate::types::Color;

/// Outcome / status of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ongoing,
    /// The side to move is checkmated; the winner is the other color.
    Checkmate { winner: Color },
    Stalemate,
    /// 50-move rule (100 half-moves without pawn move or capture).
    FiftyMoveRule,
    /// Same position reached three times.
    ThreefoldRepetition,
    /// Neither side has sufficient material to checkmate.
    InsufficientMaterial,
}

impl Status {
    pub fn is_over(self) -> bool {
        !matches!(self, Status::Ongoing)
    }
}

/// A game in progress, with full move history and a rewind cursor.
#[derive(Debug, Clone)]
pub struct Game {
    /// `history[0]` is the initial position; `history[i]` is the state *after*
    /// `moves[i-1]`. Always non-empty.
    history: Vec<Position>,
    moves: Vec<Move>,
    /// Index into `history` currently being viewed. Equals `history.len() - 1`
    /// when viewing the live position.
    cursor: usize,
}

impl Game {
    /// New game from the standard starting position.
    pub fn new() -> Game {
        Game::from_position(Position::start())
    }

    /// New game from an arbitrary starting position (e.g. a FEN loaded from an API).
    pub fn from_position(pos: Position) -> Game {
        Game {
            history: vec![pos],
            moves: Vec::new(),
            cursor: 0,
        }
    }

    /// The initial position the game started from.
    pub fn initial(&self) -> &Position {
        &self.history[0]
    }

    /// The live (latest) position, regardless of where the rewind cursor sits.
    pub fn current(&self) -> &Position {
        self.history.last().expect("history is never empty")
    }

    /// The position currently being *viewed* (may be a rewound earlier state).
    pub fn viewed(&self) -> &Position {
        &self.history[self.cursor]
    }

    /// The move that led to the viewed position, if any.
    pub fn viewed_last_move(&self) -> Option<Move> {
        if self.cursor == 0 {
            None
        } else {
            Some(self.moves[self.cursor - 1])
        }
    }

    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// Number of half-moves played (the live ply).
    pub fn ply(&self) -> usize {
        self.moves.len()
    }

    /// Which ply the rewind cursor is currently viewing (0 = initial position).
    pub fn viewed_ply(&self) -> usize {
        self.cursor
    }

    /// True when the rewind cursor is on the live position.
    pub fn is_at_live(&self) -> bool {
        self.cursor + 1 == self.history.len()
    }

    /// Legal moves in the *live* position.
    pub fn legal_moves(&self) -> Vec<Move> {
        self.current().legal_moves()
    }

    /// Attempt to play a move against the live position. Rewinds the cursor to
    /// live first; playing while viewing history discards nothing (history is
    /// preserved up to live), it simply resumes from the true current state.
    ///
    /// Returns `Err` if the move is not legal.
    pub fn make_move(&mut self, mv: Move) -> Result<(), IllegalMove> {
        if self.status() != Status::Ongoing {
            return Err(IllegalMove);
        }
        let legal = self.current().legal_moves();
        // Match on from/to/promotion so callers don't need to supply exact flags.
        let chosen = legal.into_iter().find(|m| {
            m.from == mv.from && m.to == mv.to && m.promotion == mv.promotion
        });
        let chosen = match chosen {
            Some(m) => m,
            None => return Err(IllegalMove),
        };
        let next = self.current().apply_unchecked(chosen);
        self.history.push(next);
        self.moves.push(chosen);
        self.cursor = self.history.len() - 1;
        Ok(())
    }

    // ----- Rewind controls -------------------------------------------------

    /// Step the view back one half-move. Returns false if already at the start.
    pub fn step_back(&mut self) -> bool {
        if self.cursor > 0 {
            self.cursor -= 1;
            true
        } else {
            false
        }
    }

    /// Step the view forward one half-move. Returns false if already at live.
    pub fn step_forward(&mut self) -> bool {
        if self.cursor + 1 < self.history.len() {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    /// Jump the view to the initial position.
    pub fn rewind_to_start(&mut self) {
        self.cursor = 0;
    }

    /// Jump the view to the live position.
    pub fn forward_to_live(&mut self) {
        self.cursor = self.history.len() - 1;
    }

    /// Set the view to a specific ply (0 = initial). Clamped to valid range.
    pub fn seek(&mut self, ply: usize) {
        self.cursor = ply.min(self.history.len() - 1);
    }

    // ----- Status ----------------------------------------------------------

    /// Compute the status of the *live* position.
    pub fn status(&self) -> Status {
        let pos = self.current();
        let has_moves = !pos.legal_moves().is_empty();
        if !has_moves {
            return if pos.is_in_check() {
                Status::Checkmate {
                    winner: pos.side_to_move.opponent(),
                }
            } else {
                Status::Stalemate
            };
        }
        if pos.halfmove_clock >= 100 {
            return Status::FiftyMoveRule;
        }
        if self.is_threefold() {
            return Status::ThreefoldRepetition;
        }
        if insufficient_material(pos) {
            return Status::InsufficientMaterial;
        }
        Status::Ongoing
    }

    /// Threefold repetition: same board layout, side to move, castling rights,
    /// and en-passant target occurring three times across history.
    fn is_threefold(&self) -> bool {
        let cur = self.current();
        let count = self
            .history
            .iter()
            .filter(|p| {
                p.side_to_move == cur.side_to_move
                    && p.castling == cur.castling
                    && p.en_passant == cur.en_passant
                    && positions_same_board(p, cur)
            })
            .count();
        count >= 3
    }
}

impl Default for Game {
    fn default() -> Game {
        Game::new()
    }
}

fn positions_same_board(a: &Position, b: &Position) -> bool {
    for i in 0..64u8 {
        let sq = crate::types::Square(i);
        if a.piece_at(sq) != b.piece_at(sq) {
            return false;
        }
    }
    true
}

/// True if neither side has enough material to force checkmate:
/// K vs K, K+minor vs K, and K+bishop vs K+bishop with same-colored bishops.
fn insufficient_material(pos: &Position) -> bool {
    use crate::types::{PieceKind, Square};
    let mut minors = Vec::new();
    for i in 0..64u8 {
        let sq = Square(i);
        if let Some(p) = pos.piece_at(sq) {
            match p.kind {
                PieceKind::King => {}
                PieceKind::Knight | PieceKind::Bishop => minors.push((p, sq)),
                // Any pawn, rook, or queen means mate is possible.
                _ => return false,
            }
        }
    }
    match minors.len() {
        0 | 1 => true,
        2 => {
            // Two bishops on the same color square (one each side) can't mate.
            let (p0, s0) = minors[0];
            let (p1, s1) = minors[1];
            p0.kind == PieceKind::Bishop
                && p1.kind == PieceKind::Bishop
                && p0.color != p1.color
                && (s0.file() + s0.rank()) % 2 == (s1.file() + s1.rank()) % 2
        }
        _ => false,
    }
}

/// Returned when an illegal move is attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IllegalMove;

impl core::fmt::Display for IllegalMove {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "illegal move")
    }
}

impl std::error::Error for IllegalMove {}
