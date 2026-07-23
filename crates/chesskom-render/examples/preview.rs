//! Render a position to a PNG for eyeballing the board on the desktop.
//!
//! Usage:
//!   cargo run -p chesskom-render --example preview -- [out.png] [FEN]
//!
//! With no FEN it draws the starting position; pass a FEN (quote it) to draw any
//! position. This is purely a development aid for tuning how the board looks
//! before it goes to the e-ink panel.

use chess_core::{parse_fen, Color, Position};
use chesskom_render::{board::RenderOptions, png, render};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "board.png".to_string());
    let pos = match args.get(2) {
        Some(fen) => parse_fen(fen).unwrap_or_else(|e| {
            eprintln!("bad FEN: {e}");
            std::process::exit(1);
        }),
        None => Position::start(),
    };

    let mut opts = RenderOptions::clara_bw();
    opts.orient = Color::White;
    opts.header = Some("CHESSKOM".to_string());
    opts.footer = Some(status_text(&pos));

    let canvas = render(&pos, &opts);
    let bytes = png::encode_grayscale(&canvas);
    std::fs::write(&out, bytes).expect("write png");
    println!("wrote {out} ({}x{})", canvas.width, canvas.height);
}

fn status_text(pos: &Position) -> String {
    match pos.side_to_move {
        Color::White => "WHITE TO MOVE".to_string(),
        Color::Black => "BLACK TO MOVE".to_string(),
    }
}
