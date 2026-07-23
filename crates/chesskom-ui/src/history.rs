//! Saved-game history: keep the last N games' moves so they can be replayed.
//!
//! We store only the starting FEN and the move list per game — tiny, and enough
//! to reconstruct the full game (and its result) by replaying through `chess-core`.
//! Persistence is abstracted behind [`HistoryStore`] so the app logic stays
//! I/O-free and testable; the device binary provides a file-backed store.

use chess_core::{parse_fen, to_fen, Game, Move, MoveFlag, PieceKind, Square, Status};

/// How many games to retain (most-recent first).
pub const MAX_GAMES: usize = 10;

/// A replayable game: where it started and the moves played.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedGame {
    pub start_fen: String,
    pub moves: Vec<Move>,
}

impl SavedGame {
    pub fn from_game(game: &Game) -> SavedGame {
        // Store canonical coordinate moves (from/to/promotion). The internal
        // `flag` (double-push, en passant, castle) is re-derived on replay by
        // matching against generated legal moves, so we don't persist it — this
        // also makes the serialized form and in-memory form identical.
        let moves = game
            .moves()
            .iter()
            .map(|m| Move {
                from: m.from,
                to: m.to,
                promotion: m.promotion,
                flag: MoveFlag::Normal,
            })
            .collect();
        SavedGame {
            start_fen: to_fen(game.initial()),
            moves,
        }
    }

    /// Reconstruct a playable/reviewable `Game`.
    pub fn to_game(&self) -> Game {
        let pos = parse_fen(&self.start_fen).unwrap_or_else(|_| chess_core::Position::start());
        let mut game = Game::from_position(pos);
        for mv in &self.moves {
            // Saved sequences are legal; ignore any that somehow don't apply.
            let _ = game.make_move(*mv);
        }
        game
    }

    pub fn move_count(&self) -> usize {
        self.moves.len()
    }

    /// A short human label for the history list, e.g. "24 MOVES  1-0".
    pub fn label(&self, index: usize) -> String {
        let game = self.to_game();
        let result = match game.status() {
            Status::Checkmate {
                winner: chess_core::Color::White,
            } => "1-0",
            Status::Checkmate {
                winner: chess_core::Color::Black,
            } => "0-1",
            Status::Stalemate
            | Status::FiftyMoveRule
            | Status::ThreefoldRepetition
            | Status::InsufficientMaterial => "DRAW",
            Status::Ongoing => "UNFINISHED",
        };
        // Full moves = ceil(plies / 2).
        let full = self.moves.len().div_ceil(2);
        format!("{}. {} MOVES  {}", index + 1, full, result)
    }
}

/// The retained set of games, most-recent first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct History {
    pub games: Vec<SavedGame>,
}

impl History {
    pub fn new() -> History {
        History { games: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.games.is_empty()
    }

    pub fn len(&self) -> usize {
        self.games.len()
    }

    /// Record a game at the front, dropping the oldest beyond [`MAX_GAMES`].
    /// A no-op for games with no moves, or a duplicate of the most-recent entry.
    pub fn add(&mut self, game: SavedGame) {
        if game.moves.is_empty() {
            return;
        }
        if self.games.first() == Some(&game) {
            return;
        }
        self.games.insert(0, game);
        self.games.truncate(MAX_GAMES);
    }

    // ---- Serialization ----------------------------------------------------

    const HEADER: &'static str = "chesskom-history v1";

    /// Serialize to a small text format: a header line, then one line per game
    /// as `<start-fen>|<uci moves space-separated>`.
    pub fn serialize(&self) -> String {
        let mut out = String::from(Self::HEADER);
        out.push('\n');
        for g in &self.games {
            let ucis: Vec<String> = g.moves.iter().map(|m| m.to_uci()).collect();
            out.push_str(&g.start_fen);
            out.push('|');
            out.push_str(&ucis.join(" "));
            out.push('\n');
        }
        out
    }

    /// Parse the text format; unknown/empty input yields an empty history.
    pub fn parse(text: &str) -> History {
        let mut history = History::new();
        let mut lines = text.lines();
        match lines.next() {
            Some(h) if h.trim() == Self::HEADER => {}
            _ => return history, // missing/unknown header
        }
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Some((fen, moves_str)) = line.split_once('|') else {
                continue;
            };
            let moves: Vec<Move> = moves_str
                .split_whitespace()
                .filter_map(parse_uci)
                .collect();
            history.games.push(SavedGame {
                start_fen: fen.to_string(),
                moves,
            });
        }
        history.games.truncate(MAX_GAMES);
        history
    }
}

/// Parse a UCI coordinate move (`e2e4`, `e7e8q`). Flag is left `Normal`; the
/// game applies it by matching from/to/promotion.
fn parse_uci(s: &str) -> Option<Move> {
    if s.len() < 4 {
        return None;
    }
    let from = Square::from_algebraic(&s[0..2])?;
    let to = Square::from_algebraic(&s[2..4])?;
    let promotion = s.chars().nth(4).and_then(PieceKind::from_char);
    Some(Move {
        from,
        to,
        promotion,
        flag: MoveFlag::Normal,
    })
}

/// Where history is loaded from / saved to. Kept as a trait so the app logic
/// never touches the filesystem directly (the device binary supplies a file
/// implementation; tests use an in-memory one or the no-op store).
pub trait HistoryStore {
    fn load(&self) -> History;
    fn save(&self, history: &History);
}

/// A store that persists nothing (desktop/testing default).
pub struct NullStore;

impl HistoryStore for NullStore {
    fn load(&self) -> History {
        History::new()
    }
    fn save(&self, _history: &History) {}
}
