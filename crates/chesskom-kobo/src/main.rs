//! chesskom-kobo — draw a board on the Kobo e-ink screen.
//!
//! This is the first on-device milestone: prove we can render a position to the
//! panel. It does not yet handle touch input — that's the next step. It relies on
//! FBInk being installed on the device (see the crate README / project README).
//!
//! Usage on the device (via a terminal, NickelMenu action, or KFMon):
//!   chesskom-kobo                 # draw the starting position
//!   chesskom-kobo --flip          # draw with Black at the bottom
//!   chesskom-kobo --fen "<FEN>"   # draw an arbitrary position
//!
//! For desktop testing (no panel), pass --out to just write the PNG:
//!   chesskom-kobo --out /tmp/board.png

mod fbink;

use chess_core::{parse_fen, Color, Position};
use chesskom_render::{board::RenderOptions, png, render};
use std::process::exit;

fn main() {
    let mut fen: Option<String> = None;
    let mut orient = Color::White;
    let mut out: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--flip" => orient = Color::Black,
            "--fen" => fen = args.next(),
            "--out" => out = args.next(),
            "-h" | "--help" => {
                print_usage();
                return;
            }
            other => {
                eprintln!("unknown argument: {other}");
                print_usage();
                exit(2);
            }
        }
    }

    let pos = match &fen {
        Some(f) => match parse_fen(f) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("bad FEN: {e}");
                exit(1);
            }
        },
        None => Position::start(),
    };

    let mut opts = RenderOptions::clara_bw();
    opts.orient = orient;
    opts.header = Some("CHESSKOM".to_string());
    opts.footer = Some(status_text(&pos));
    let canvas = render(&pos, &opts);

    match out {
        // Desktop test mode: just write the PNG.
        Some(path) => {
            if let Err(e) = std::fs::write(&path, png::encode_grayscale(&canvas)) {
                eprintln!("failed to write {path}: {e}");
                exit(1);
            }
            println!("wrote {path} ({}x{})", canvas.width, canvas.height);
        }
        // Device mode: push to the e-ink panel via FBInk.
        None => {
            let display = fbink::Fbink::default();
            if let Err(e) = display.present(&canvas) {
                eprintln!("failed to present on panel: {e}");
                exit(1);
            }
        }
    }
}

fn status_text(pos: &Position) -> String {
    match pos.side_to_move {
        Color::White => "WHITE TO MOVE".to_string(),
        Color::Black => "BLACK TO MOVE".to_string(),
    }
}

fn print_usage() {
    eprintln!(
        "\
chesskom-kobo — draw a chess board on the Kobo e-ink screen

  --flip           orient with Black at the bottom
  --fen \"<FEN>\"    render a specific position (default: start position)
  --out <file>     write a PNG instead of drawing to the panel (desktop test)
  -h, --help       show this help

Device mode requires FBInk (https://github.com/NiLuJe/FBInk) on the Kobo."
    );
}
