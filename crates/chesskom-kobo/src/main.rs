//! chesskom-kobo — the on-device app.
//!
//! Modes:
//!   (default)            interactive local two-player game on the e-ink panel:
//!                        tap a piece, tap a destination; use the bottom control
//!                        bar for rewind / flip / new / quit. Requires FBInk and a
//!                        readable touch device.
//!   --calibrate          print raw + mapped touch coordinates (to tune the
//!                        CHESSKOM_TOUCH_* env vars) instead of playing.
//!   --fen "<FEN>" / --out <file>
//!                        one-shot static render (no touch); --out writes a PNG
//!                        for desktop testing, otherwise draws once to the panel.
//!
//! See touch.rs for the touch calibration environment variables.

mod fbink;
mod touch;

use chess_core::{parse_fen, Color, Position};
use chesskom_render::{png, render, RenderOptions};
use chesskom_ui::App;
use std::process::exit;

fn main() {
    let mut fen: Option<String> = None;
    let mut orient = Color::White;
    let mut out: Option<String> = None;
    let mut otb = false;
    let mut calibrate = false;
    let mut interactive = true;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--flip" => orient = Color::Black,
            "--otb" => otb = true,
            "--calibrate" => calibrate = true,
            "--fen" => {
                fen = args.next();
                interactive = false;
            }
            "--out" => {
                out = args.next();
                interactive = false;
            }
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

    if calibrate {
        run_calibrate();
        return;
    }
    if interactive {
        run_interactive();
        return;
    }
    run_static(fen, orient, otb, out);
}

/// Interactive game loop: present a frame, wait for a tap, update, repeat.
fn run_interactive() {
    let mut app = App::clara_bw();
    let display = fbink::Fbink::default();

    // Initial full refresh.
    if let Err(e) = display.present(&app.render(), true) {
        eprintln!("failed to present on panel: {e}\n(is FBInk installed?)");
        exit(1);
    }

    let mut touch = match touch::TouchReader::open() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("failed to open touch device: {e}");
            eprintln!("set CHESSKOM_TOUCH_DEV to your touchscreen event node.");
            exit(1);
        }
    };
    let cal = touch::Calibration::from_env(1072, 1448);

    loop {
        let (rx, ry) = match touch.next_tap() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("touch read error: {e}");
                break;
            }
        };
        let (x, y) = cal.map(rx, ry);
        let ply_before = app.game().ply();
        let orient_before = app.orientation();
        if app.tap_pixel(x, y) {
            // A wholesale board change (a reset back to ply 0, or a flip) warrants
            // a full flashing refresh to clear ghosting; incremental taps don't.
            let reset = app.game().ply() == 0 && ply_before > 0;
            let flipped = app.orientation() != orient_before;
            if let Err(e) = display.present(&app.render(), reset || flipped) {
                eprintln!("present error: {e}");
                break;
            }
        }
        if app.should_quit() {
            break;
        }
    }
}

/// Print raw and mapped touch coordinates so the CHESSKOM_TOUCH_* vars can be tuned.
fn run_calibrate() {
    let cal = touch::Calibration::from_env(1072, 1448);
    let mut touch = match touch::TouchReader::open() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("failed to open touch device: {e}");
            exit(1);
        }
    };
    eprintln!("calibrate: tap the screen; Ctrl-C to stop.");
    eprintln!("target screen is 1072x1448. Tap the four corners and check the mapping.");
    loop {
        match touch.next_tap() {
            Ok((rx, ry)) => {
                let (sx, sy) = cal.map(rx, ry);
                println!("raw=({rx},{ry})  ->  screen=({sx},{sy})");
            }
            Err(e) => {
                eprintln!("touch read error: {e}");
                break;
            }
        }
    }
}

/// One-shot static render (no touch).
fn run_static(fen: Option<String>, orient: Color, otb: bool, out: Option<String>) {
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
    opts.over_the_board = otb;
    opts.header = Some("CHESSKOM".to_string());
    opts.footer = Some(status_text(&pos));
    let canvas = render(&pos, &opts);

    match out {
        Some(path) => {
            if let Err(e) = std::fs::write(&path, png::encode_grayscale(&canvas)) {
                eprintln!("failed to write {path}: {e}");
                exit(1);
            }
            println!("wrote {path} ({}x{})", canvas.width, canvas.height);
        }
        None => {
            let display = fbink::Fbink::default();
            if let Err(e) = display.present(&canvas, true) {
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
chesskom-kobo — chess on the Kobo e-ink screen

  (no args)        interactive local two-player game (needs FBInk + touch)
  --calibrate      print raw/mapped touch coordinates to tune CHESSKOM_TOUCH_*
  --flip           static render oriented with Black at the bottom
  --otb            static render in over-the-board mode
  --fen \"<FEN>\"    static render of a specific position
  --out <file>     write a PNG instead of drawing (desktop test)
  -h, --help       show this help

Requires FBInk (https://github.com/NiLuJe/FBInk) on the Kobo.
Touch mapping is calibrated via CHESSKOM_TOUCH_* env vars (see --calibrate)."
    );
}
