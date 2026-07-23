//! Render the menu screens to PNGs for eyeballing on the desktop.
//! Usage: cargo run -p chesskom-ui --example menus -- out_dir

use chess_core::Square;
use chesskom_render::png;
use chesskom_render::Rect;
use chesskom_ui::{menu::MenuLayout, App};

fn save(app: &App, path: &str) {
    let c = app.render();
    std::fs::write(path, png::encode_grayscale(&c)).unwrap();
    println!("wrote {path}");
}

/// Tap a menu item by index (item count must match the current screen).
fn tap_menu(app: &mut App, count: usize, i: usize) {
    let ml = MenuLayout::new(1072, 1448, count);
    let Rect { x, y, w, h } = ml.items[i];
    app.tap_pixel(x + w / 2, y + h / 2);
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());

    // Main menu (the app boots here).
    let app = App::clara_bw();
    save(&app, &format!("{dir}/menu_main.png"));

    // Play and save two quick games so History has entries.
    let mut app = App::clara_bw();
    tap_menu(&mut app, 4, 0); // LOCAL
    tap_menu(&mut app, 3, 0); // NEW GAME
    for (a, b) in [("f2", "f3"), ("e7", "e5"), ("g2", "g4"), ("d8", "h4")] {
        app.tap_square(Square::from_algebraic(a).unwrap());
        app.tap_square(Square::from_algebraic(b).unwrap());
    }
    // MENU (saves the finished game), then back into Local -> History.
    app.tap_pixel(0, 0); // no-op-ish; ensure a defined state
    // Use the control bar MENU button via layout hit-test.
    let lay = app.render_options().layout();
    if let Some((_, r)) = lay
        .controls
        .iter()
        .find(|(c, _)| matches!(c, chesskom_render::Control::Menu))
        .copied()
    {
        app.tap_pixel(r.x + r.w / 2, r.y + r.h / 2);
    }
    tap_menu(&mut app, 3, 1); // GAME HISTORY
    save(&app, &format!("{dir}/menu_history.png"));
}
