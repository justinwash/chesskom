//! Explicit regression tests for the esoteric / easy-to-get-wrong chess rules.
//!
//! The perft tests in the crate already validate move generation statistically
//! against known node counts (which implicitly covers all of the below), but
//! these spell out each tricky rule as a named, human-readable case so a future
//! change that breaks one fails loudly and obviously.

use chess_core::{parse_fen, Game, Move, MoveFlag, PieceKind, Position, Square, Status};

/// All legal moves from a FEN as sorted UCI strings.
fn legal_ucis(fen: &str) -> Vec<String> {
    let pos = parse_fen(fen).expect("valid FEN");
    let mut v: Vec<String> = pos.legal_moves().iter().map(|m| m.to_uci()).collect();
    v.sort();
    v
}

fn has_move(fen: &str, uci: &str) -> bool {
    legal_ucis(fen).iter().any(|m| m == uci)
}

/// Play a coordinate move (matched loosely by from/to/promotion) on a game.
fn play(game: &mut Game, uci: &str) {
    let from = Square::from_algebraic(&uci[0..2]).unwrap();
    let to = Square::from_algebraic(&uci[2..4]).unwrap();
    let promotion = uci.chars().nth(4).and_then(PieceKind::from_char);
    game.make_move(Move {
        from,
        to,
        promotion,
        flag: MoveFlag::Normal,
    })
    .unwrap_or_else(|_| panic!("expected {uci} to be legal"));
}

// ---- Castling ------------------------------------------------------------

#[test]
fn cannot_castle_through_an_attacked_square() {
    // Black rook on f8 attacks f1; the white king would pass through f1.
    let fen = "k4r2/8/8/8/8/8/8/4K2R w K - 0 1";
    assert!(!has_move(fen, "e1g1"), "castling through check must be illegal");
}

#[test]
fn cannot_castle_into_check() {
    // Black rook on g8 attacks the king's destination g1.
    let fen = "k5r1/8/8/8/8/8/8/4K2R w K - 0 1";
    assert!(!has_move(fen, "e1g1"), "castling into check must be illegal");
}

#[test]
fn cannot_castle_out_of_check() {
    // Black rook on e8 gives check along the e-file.
    let fen = "k3r3/8/8/8/8/8/8/4K2R w K - 0 1";
    let pos = parse_fen(fen).unwrap();
    assert!(pos.is_in_check(), "king should be in check in this setup");
    assert!(!has_move(fen, "e1g1"), "castling out of check must be illegal");
}

#[test]
fn can_castle_kingside_when_path_is_clear_and_safe() {
    let fen = "k7/8/8/8/8/8/8/4K2R w K - 0 1";
    assert!(has_move(fen, "e1g1"), "normal kingside castling must be legal");
}

#[test]
fn queenside_castling_allowed_even_when_b_file_is_attacked() {
    // The b1 square (the rook's path, not the king's) is attacked by the b8 rook.
    // Only the king's path (e1-d1-c1) must be safe, so this castling is legal.
    let fen = "1r5k/8/8/8/8/8/8/R3K3 w Q - 0 1";
    assert!(
        has_move(fen, "e1c1"),
        "queenside castling should be legal even when only the rook's path square is attacked"
    );
}

#[test]
fn castling_right_is_lost_after_the_king_moves_and_returns() {
    // King steps off e1 and back; the right must not come back.
    let mut game = Game::from_position(parse_fen("k7/8/8/8/8/8/8/4K2R w K - 0 1").unwrap());
    play(&mut game, "e1e2");
    play(&mut game, "a8a7"); // black tempo
    play(&mut game, "e2e1");
    play(&mut game, "a7a8");
    assert!(
        !game.current().legal_moves().iter().any(|m| m.flag == MoveFlag::CastleKingside),
        "castling right must stay revoked after the king has moved"
    );
}

#[test]
fn castling_not_offered_without_rights() {
    let fen = "k7/8/8/8/8/8/8/4K2R w - - 0 1";
    assert!(!has_move(fen, "e1g1"), "no castling without the right in the FEN");
}

// ---- En passant ----------------------------------------------------------

#[test]
fn en_passant_that_exposes_own_king_is_illegal() {
    // The classic horizontal-pin case: capturing en passant removes BOTH the
    // capturing pawn (b5) and the captured pawn (c5) from rank 5, exposing the
    // white king on a5 to the black rook on h5. So b5xc6 e.p. must be illegal.
    let fen = "7k/8/8/KPp4r/8/8/8/8 w - c6 0 1";
    assert!(
        !has_move(fen, "b5c6"),
        "en passant that discovers check on our own king must be illegal"
    );
}

#[test]
fn en_passant_is_legal_when_it_does_not_expose_the_king() {
    let fen = "4k3/8/8/KPp5/8/8/8/8 w - c6 0 1";
    assert!(has_move(fen, "b5c6"), "a safe en passant capture must be legal");
}

// ---- Pins ----------------------------------------------------------------

#[test]
fn pinned_piece_may_not_leave_the_pin_line_but_may_capture_the_pinner() {
    // White rook e2 is pinned to the king on e1 by the black rook on e8.
    let fen = "4r2k/8/8/8/8/8/4R3/4K3 w - - 0 1";
    assert!(!has_move(fen, "e2d2"), "pinned rook cannot move off the file");
    assert!(has_move(fen, "e2e5"), "pinned rook may still slide along the pin line");
    assert!(has_move(fen, "e2e8"), "pinned rook may capture the pinner");
}

// ---- King safety along a slider ray -------------------------------------

#[test]
fn king_cannot_step_further_along_the_checking_ray() {
    // Black rook a1 checks the king on e1 along rank 1. Moving to f1 stays on the
    // ray (once the king vacates e1 the rook sees f1), so it must be illegal —
    // the bug that naive generators hit by not vacating the king's square.
    let fen = "4k3/8/8/8/8/8/8/r3K3 w - - 0 1";
    assert!(!has_move(fen, "e1f1"), "king may not walk along the check ray");
    assert!(!has_move(fen, "e1d1"), "d1 is still on the rook's ray");
    assert!(has_move(fen, "e1e2"), "stepping off the rank escapes the check");
}

// ---- Promotion -----------------------------------------------------------

#[test]
fn promotion_offers_all_four_pieces_and_no_plain_push() {
    let fen = "4k3/P7/8/8/8/8/8/4K3 w - - 0 1";
    let moves = legal_ucis(fen);
    for promo in ["a7a8q", "a7a8r", "a7a8b", "a7a8n"] {
        assert!(moves.contains(&promo.to_string()), "missing promotion {promo}");
    }
    assert!(
        !moves.contains(&"a7a8".to_string()),
        "a promotion push must always specify a piece"
    );
}

#[test]
fn promotion_capture_with_underpromotion_is_generated() {
    // White pawn b7 can capture the rook on a8 or c8, or push to b8, each promoting.
    let fen = "r1r1k3/1P6/8/8/8/8/8/4K3 w - - 0 1";
    let moves = legal_ucis(fen);
    assert!(moves.contains(&"b7a8q".to_string()), "capture-promotion to queen");
    assert!(moves.contains(&"b7a8n".to_string()), "capture-underpromotion to knight");
    assert!(moves.contains(&"b7c8q".to_string()), "capture the other rook, promoting");
}

// ---- Two kings never adjacent -------------------------------------------

#[test]
fn king_cannot_move_adjacent_to_the_enemy_king() {
    // Kings on e1 and e3; white king may not step to d2/e2/f2 (all adjacent to e3).
    let fen = "8/8/8/8/8/4k3/8/4K3 w - - 0 1";
    for illegal in ["e1d2", "e1e2", "e1f2"] {
        assert!(!has_move(fen, illegal), "{illegal} would put the kings adjacent");
    }
    assert!(has_move(fen, "e1d1"), "moving away from the enemy king is fine");
}

// ---- Check must be resolved ---------------------------------------------

#[test]
fn in_check_only_check_evasions_are_legal() {
    // White king e1 in check from the rook on e8. Every legal move must end the check.
    let fen = "4r3/8/8/8/8/8/8/4K3 w - - 0 1";
    let pos = parse_fen(fen).unwrap();
    assert!(pos.is_in_check());
    for mv in pos.legal_moves() {
        let after = pos.apply_unchecked(mv);
        // After our move, our (white) king must not be attacked.
        let k = after.king_square(chess_core::Color::White).unwrap();
        assert!(
            !after.is_attacked(k, chess_core::Color::Black),
            "generated move {} leaves the king in check",
            mv.to_uci()
        );
    }
}

// ---- Draw conditions -----------------------------------------------------

#[test]
fn fifty_move_rule_triggers_at_100_halfmoves() {
    // Halfmove clock already at 100 → drawn regardless of pieces.
    let pos = parse_fen("4k3/8/8/8/8/8/8/R3K3 w - - 100 60").unwrap();
    assert_eq!(Game::from_position(pos).status(), Status::FiftyMoveRule);
}

#[test]
fn bishops_of_opposite_color_on_same_square_color_is_insufficient() {
    // White bishop on a light square, black bishop on a light square → dead draw.
    let pos = parse_fen("4k3/8/8/8/8/8/5B2/4K1b1 w - - 0 1").unwrap();
    assert_eq!(
        Game::from_position(pos).status(),
        Status::InsufficientMaterial
    );
}

#[test]
fn a_lone_rook_is_sufficient_material() {
    let pos = parse_fen("4k3/8/8/8/8/8/8/R3K3 w - - 0 1").unwrap();
    assert_eq!(Game::from_position(pos).status(), Status::Ongoing);
}

#[test]
fn start_position_has_twenty_legal_moves() {
    assert_eq!(Position::start().legal_moves().len(), 20);
}
