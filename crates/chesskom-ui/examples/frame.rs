//! Render a single interactive frame to a PNG for eyeballing the overlays
//! (selection shading, move targets, control bar) on the desktop.
//!
//! Usage: cargo run -p chesskom-ui --example frame -- out.png

use chess_core::Square;
use chesskom_render::png;
use chesskom_ui::App;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "frame.png".to_string());

    let mut app = App::clara_bw();
    app.start_new_local_game();
    // 1. e4 d5 — now the e4 pawn can capture on d5 (ring) or push to e5 (dot).
    for (a, b) in [("e2", "e4"), ("d7", "d5")] {
        app.tap_square(Square::from_algebraic(a).unwrap());
        app.tap_square(Square::from_algebraic(b).unwrap());
    }
    // Select the e4 pawn to show both marker kinds: a quiet-move dot and a
    // capture ring around the black pawn on d5.
    app.tap_square(Square::from_algebraic("e4").unwrap());

    let canvas = app.render();
    std::fs::write(&out, png::encode_grayscale(&canvas)).expect("write png");
    println!("wrote {out} ({}x{})", canvas.width, canvas.height);
}
