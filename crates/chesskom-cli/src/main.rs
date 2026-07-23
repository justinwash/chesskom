//! chesskom — terminal front-end.
//!
//! Milestone 1: local two-player pass-and-play with full rewind. This binary is
//! the development harness that lets us play real games against `chess-core` long
//! before the Kobo UI exists; the board layout and command model here map closely
//! onto what the e-ink renderer will do.

mod render;

use chess_core::{Color, Game, Move, PieceKind, Square};
use std::io::{self, Write};

fn main() {
    let mut game = Game::new();
    // Orientation follows the side to move by default so pass-and-play feels natural.
    let mut auto_flip = true;

    println!("chesskom — local two-player chess\n");
    print_help();

    loop {
        let orient = if auto_flip {
            game.current().side_to_move
        } else {
            Color::White
        };
        println!();
        print!("{}", render::board(game.viewed(), orient, game.viewed_last_move()));

        // Show whether we're viewing history or the live position.
        if game.is_at_live() {
            println!("\n{}", render::status_line(&game));
        } else {
            println!(
                "\n[viewing ply {}/{}] — 'end' to return to the live game",
                game.viewed_ply(),
                game.ply()
            );
        }
        let ml = render::move_list(&game);
        if !ml.is_empty() {
            println!("moves: {ml}");
        }

        if game.status().is_over() && game.is_at_live() {
            println!("\nGame over. You can still 'back'/'forward' to review, or 'new' to restart.");
        }

        print!("\n> ");
        io::stdout().flush().ok();

        let line = match read_line() {
            Some(l) => l,
            None => break, // EOF
        };
        let cmd = line.trim();
        if cmd.is_empty() {
            continue;
        }

        match cmd {
            "quit" | "q" | "exit" => break,
            "help" | "h" | "?" => print_help(),
            "new" => {
                game = Game::new();
                println!("New game started.");
            }
            "flip" => {
                auto_flip = !auto_flip;
                println!(
                    "Auto-flip {}.",
                    if auto_flip { "on (board faces mover)" } else { "off (white at bottom)" }
                );
            }
            "back" | "b" => {
                if !game.step_back() {
                    println!("Already at the start.");
                }
            }
            "forward" | "f" => {
                if !game.step_forward() {
                    println!("Already at the live position.");
                }
            }
            "start" => game.rewind_to_start(),
            "end" => game.forward_to_live(),
            "moves" => list_moves_from(&game),
            other => {
                // "moves <sq>" lists legal destinations from a square.
                if let Some(rest) = other.strip_prefix("moves ") {
                    list_moves_from_square(&game, rest.trim());
                } else {
                    try_move(&mut game, other, auto_flip);
                }
            }
        }
    }
    println!("bye.");
}

fn try_move(game: &mut Game, input: &str, _auto_flip: bool) {
    let mv = match parse_move(input) {
        Ok(m) => m,
        Err(msg) => {
            println!("{msg}");
            return;
        }
    };
    if !game.is_at_live() {
        println!("(resuming from the live position)");
    }
    match game.make_move(mv) {
        Ok(()) => {}
        Err(_) => println!("Illegal move: {input}"),
    }
}

fn list_moves_from(game: &Game) {
    let mut moves = game.legal_moves();
    moves.sort_by_key(|m| (m.from.0, m.to.0));
    let uci: Vec<String> = moves.iter().map(|m| m.to_uci()).collect();
    println!("legal moves ({}): {}", uci.len(), uci.join(" "));
}

fn list_moves_from_square(game: &Game, sq_str: &str) {
    let sq = match Square::from_algebraic(sq_str) {
        Some(s) => s,
        None => {
            println!("Not a square: {sq_str}");
            return;
        }
    };
    let dests: Vec<String> = game
        .legal_moves()
        .into_iter()
        .filter(|m| m.from == sq)
        .map(|m| m.to.to_string() + m.promotion.map(|p| p.to_char()).map(|c| c.to_string()).unwrap_or_default().as_str())
        .collect();
    if dests.is_empty() {
        println!("No legal moves from {sq}.");
    } else {
        println!("from {sq}: {}", dests.join(" "));
    }
}

/// Parse a move in coordinate form: `e2e4`, `e7e8q`, or with a separator `e2-e4`.
fn parse_move(input: &str) -> Result<Move, String> {
    let cleaned: String = input.chars().filter(|c| !matches!(c, '-' | ' ' | 'x')).collect();
    if cleaned.len() < 4 {
        return Err(format!(
            "Enter a move like 'e2e4' (or 'e7e8q' to promote). Got: {input}"
        ));
    }
    let from = Square::from_algebraic(&cleaned[0..2])
        .ok_or_else(|| format!("Bad from-square in '{input}'"))?;
    let to = Square::from_algebraic(&cleaned[2..4])
        .ok_or_else(|| format!("Bad to-square in '{input}'"))?;
    let promotion = cleaned.chars().nth(4).and_then(PieceKind::from_char);
    Ok(Move {
        from,
        to,
        promotion,
        flag: chess_core::MoveFlag::Normal,
    })
}

fn read_line() -> Option<String> {
    let mut s = String::new();
    match io::stdin().read_line(&mut s) {
        Ok(0) => None,
        Ok(_) => Some(s),
        Err(_) => None,
    }
}

fn print_help() {
    println!(
        "\
Commands:
  <move>        play a move in coordinate form, e.g. e2e4, g1f3, e7e8q (promote)
  moves         list all legal moves in the current position
  moves <sq>    list legal moves from a square, e.g. 'moves e2'
  back / b      rewind the view one half-move
  forward / f   advance the view one half-move
  start         jump the view to the initial position
  end           jump the view back to the live game
  flip          toggle auto-flip (board faces the side to move)
  new           start a new game
  help / h / ?  show this help
  quit / q      exit"
    );
}
