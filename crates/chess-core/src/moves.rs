//! Move representation.

use crate::types::{PieceKind, Square};
use core::fmt;

/// Extra semantics a move can carry that aren't obvious from `from`/`to` alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveFlag {
    Normal,
    /// Pawn advancing two squares (sets an en-passant target).
    DoublePawnPush,
    /// Pawn capturing en passant (the captured pawn is not on `to`).
    EnPassant,
    CastleKingside,
    CastleQueenside,
}

/// A single move. Promotions carry the promoted-to piece kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PieceKind>,
    pub flag: MoveFlag,
}

impl Move {
    pub fn normal(from: Square, to: Square) -> Move {
        Move {
            from,
            to,
            promotion: None,
            flag: MoveFlag::Normal,
        }
    }

    /// Long algebraic / UCI-style coordinate string, e.g. `e2e4`, `e7e8q`.
    pub fn to_uci(self) -> String {
        let mut s = format!("{}{}", self.from, self.to);
        if let Some(p) = self.promotion {
            s.push(p.to_char());
        }
        s
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_uci())
    }
}
