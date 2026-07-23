//! `chess-core`: dependency-free chess rules, move generation, game state, and rewind.
//!
//! This crate is intentionally free of external dependencies and platform
//! assumptions so it cross-compiles cleanly to constrained targets (e.g. a Kobo
//! e-reader). All I/O, UI, and networking live in other crates.

pub mod fen;
pub mod game;
pub mod moves;
pub mod position;
pub mod types;

pub use fen::{parse_fen, to_fen, FenError};
pub use game::{Game, IllegalMove, Status};
pub use moves::{Move, MoveFlag};
pub use position::{CastlingRights, Position};
pub use types::{Color, Piece, PieceKind, Square};

#[cfg(test)]
mod tests {
    use super::*;

    /// Count leaf nodes at the given depth — the standard move-generator correctness check.
    fn perft(pos: &Position, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        let moves = pos.legal_moves();
        if depth == 1 {
            return moves.len() as u64;
        }
        let mut nodes = 0;
        for mv in moves {
            let next = pos.apply_unchecked(mv);
            nodes += perft(&next, depth - 1);
        }
        nodes
    }

    // Known perft node counts for the initial position.
    // Source: https://www.chessprogramming.org/Perft_Results
    #[test]
    fn perft_startpos() {
        let pos = Position::start();
        assert_eq!(perft(&pos, 1), 20);
        assert_eq!(perft(&pos, 2), 400);
        assert_eq!(perft(&pos, 3), 8_902);
        assert_eq!(perft(&pos, 4), 197_281);
    }

    // "Kiwipete" — a dense middlegame position exercising castling, en passant,
    // pins, and promotions. The classic move-generator stress test.
    #[test]
    fn perft_kiwipete() {
        let pos =
            parse_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
                .unwrap();
        assert_eq!(perft(&pos, 1), 48);
        assert_eq!(perft(&pos, 2), 2_039);
        assert_eq!(perft(&pos, 3), 97_862);
    }

    // Position 3 from the CPW perft suite — endgame with tricky en-passant/pin cases.
    #[test]
    fn perft_position3() {
        let pos = parse_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
        assert_eq!(perft(&pos, 1), 14);
        assert_eq!(perft(&pos, 2), 191);
        assert_eq!(perft(&pos, 3), 2_812);
        assert_eq!(perft(&pos, 4), 43_238);
    }

    // Position 4 — promotions and a position where white is in check.
    #[test]
    fn perft_position4() {
        let pos =
            parse_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1")
                .unwrap();
        assert_eq!(perft(&pos, 1), 6);
        assert_eq!(perft(&pos, 2), 264);
        assert_eq!(perft(&pos, 3), 9_467);
    }

    #[test]
    fn fen_roundtrip_startpos() {
        let start = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let pos = parse_fen(start).unwrap();
        assert_eq!(to_fen(&pos), start);
    }

    #[test]
    fn fen_roundtrip_complex() {
        let f = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        assert_eq!(to_fen(&parse_fen(f).unwrap()), f);
    }

    #[test]
    fn detects_fools_mate() {
        // 1. f3 e5 2. g4 Qh4# — fastest checkmate.
        let mut game = Game::new();
        for uci in ["f2f3", "e7e5", "g2g4", "d8h4"] {
            let mv = parse_uci(uci);
            game.make_move(mv).expect("legal");
        }
        match game.status() {
            Status::Checkmate { winner } => assert_eq!(winner, Color::Black),
            other => panic!("expected checkmate, got {other:?}"),
        }
    }

    #[test]
    fn detects_stalemate() {
        // Classic K+Q stalemate: black king on a8, white king c6, white queen b6? No —
        // use a known stalemate: black to move, king h8 boxed. FEN below is a stalemate.
        let pos = parse_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
        let game = Game::from_position(pos);
        assert_eq!(game.status(), Status::Stalemate);
    }

    #[test]
    fn rewind_preserves_live_state() {
        let mut game = Game::new();
        for uci in ["e2e4", "e7e5", "g1f3"] {
            game.make_move(parse_uci(uci)).unwrap();
        }
        assert_eq!(game.ply(), 3);
        assert!(game.is_at_live());

        // Rewind to the start and walk forward.
        game.rewind_to_start();
        assert!(!game.is_at_live());
        assert_eq!(game.viewed(), &Position::start());
        assert!(game.viewed_last_move().is_none());

        game.step_forward();
        assert_eq!(game.viewed_last_move().unwrap().to, Square::from_algebraic("e4").unwrap());

        // Live position is untouched by viewing history.
        game.forward_to_live();
        assert_eq!(game.ply(), 3);

        // Playing resumes from live even if we were viewing an earlier ply.
        // At live it's Black to move (after e4 e5 Nf3), so play a Black move.
        game.rewind_to_start();
        assert!(!game.is_at_live());
        game.make_move(parse_uci("b8c6")).unwrap();
        assert_eq!(game.ply(), 4);
        assert!(game.is_at_live());
    }

    #[test]
    fn insufficient_material_king_vs_king() {
        let pos = parse_fen("8/8/4k3/8/8/4K3/8/8 w - - 0 1").unwrap();
        assert_eq!(Game::from_position(pos).status(), Status::InsufficientMaterial);
    }

    #[test]
    fn en_passant_capture_is_legal_and_removes_pawn() {
        // White pawn e5, black plays d7d5, white captures e5xd6 e.p.
        let mut game = Game::from_position(
            parse_fen("rnbqkbnr/pppp1ppp/8/4P3/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1").unwrap(),
        );
        game.make_move(parse_uci("d7d5")).unwrap();
        assert_eq!(game.current().en_passant, Square::from_algebraic("d6"));
        game.make_move(parse_uci("e5d6")).unwrap();
        // The black d5 pawn must be gone.
        assert!(game.current().piece_at(Square::from_algebraic("d5").unwrap()).is_none());
        assert!(game.current().piece_at(Square::from_algebraic("d6").unwrap()).is_some());
    }

    /// Minimal UCI parser for tests (from/to plus optional promotion char).
    fn parse_uci(s: &str) -> Move {
        let from = Square::from_algebraic(&s[0..2]).unwrap();
        let to = Square::from_algebraic(&s[2..4]).unwrap();
        let promotion = s.chars().nth(4).and_then(PieceKind::from_char);
        Move {
            from,
            to,
            promotion,
            flag: MoveFlag::Normal,
        }
    }
}
