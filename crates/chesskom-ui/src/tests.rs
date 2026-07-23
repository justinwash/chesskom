use super::*;
use crate::menu::MenuLayout;
use chess_core::{parse_fen, Game};
use std::cell::RefCell;
use std::rc::Rc;

fn sq(s: &str) -> Square {
    Square::from_algebraic(s).unwrap()
}

/// An app already in a fresh local game.
fn game_app() -> App {
    let mut app = App::clara_bw();
    app.start_new_local_game();
    app
}

fn targets(app: &App) -> Vec<Square> {
    app.render_options().targets
}

/// Tap the center of menu item `i`, given the item count on the current screen.
fn tap_menu(app: &mut App, count: usize, i: usize) -> bool {
    let ml = MenuLayout::new(1072, 1448, count);
    let r = ml.items[i];
    app.tap_pixel(r.x + r.w / 2, r.y + r.h / 2)
}

/// In-memory store so we can test load/save round-tripping.
struct MemStore(Rc<RefCell<History>>);
impl HistoryStore for MemStore {
    fn load(&self) -> History {
        self.0.borrow().clone()
    }
    fn save(&self, h: &History) {
        *self.0.borrow_mut() = h.clone();
    }
}

// ---- Game tap logic ------------------------------------------------------

#[test]
fn tapping_a_piece_selects_it_and_shows_targets() {
    let mut app = game_app();
    assert!(app.tap_square(sq("e2")));
    assert_eq!(app.selected(), Some(sq("e2")));
    let t = targets(&app);
    assert!(t.contains(&sq("e3")) && t.contains(&sq("e4")));
}

#[test]
fn select_then_tap_destination_makes_the_move() {
    let mut app = game_app();
    app.tap_square(sq("e2"));
    assert!(app.tap_square(sq("e4")));
    assert_eq!(app.selected(), None);
    assert_eq!(app.game().ply(), 1);
    assert_eq!(app.game().current().side_to_move, Color::Black);
}

#[test]
fn tapping_another_own_piece_switches_selection() {
    let mut app = game_app();
    app.tap_square(sq("e2"));
    assert!(app.tap_square(sq("d2")));
    assert_eq!(app.selected(), Some(sq("d2")));
    assert_eq!(app.game().ply(), 0);
}

#[test]
fn tapping_illegal_square_deselects() {
    let mut app = game_app();
    app.tap_square(sq("e2"));
    assert!(app.tap_square(sq("e5")));
    assert_eq!(app.selected(), None);
    assert_eq!(app.game().ply(), 0);
}

#[test]
fn prev_enters_review_and_board_tap_returns_to_live() {
    let mut app = game_app();
    app.tap_square(sq("e2"));
    app.tap_square(sq("e4"));
    assert!(app.press(Control::Prev));
    assert!(!app.game().is_at_live());
    app.tap_square(sq("d7"));
    assert!(app.game().is_at_live());
}

#[test]
fn flip_toggles_orientation() {
    let mut app = game_app();
    assert!(app.press(Control::Flip));
    assert_eq!(app.orientation(), Color::Black);
}

#[test]
fn promotion_defaults_to_queen() {
    let mut app = App::clara_bw();
    app.set_game(Game::from_position(
        parse_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap(),
    ));
    app.tap_square(sq("a7"));
    assert!(targets(&app).contains(&sq("a8")));
    app.tap_square(sq("a8"));
    let promoted = app.game().current().piece_at(sq("a8")).unwrap();
    assert_eq!(promoted.kind, PieceKind::Queen);
}

#[test]
fn tap_pixel_hits_control_buttons() {
    let app0 = game_app();
    let lay = app0.render_options().layout();
    let (_, rect) = lay
        .controls
        .iter()
        .find(|(c, _)| *c == Control::Flip)
        .copied()
        .unwrap();
    let mut app = game_app();
    assert!(app.tap_pixel(rect.x + rect.w / 2, rect.y + rect.h / 2));
    assert_eq!(app.orientation(), Color::Black);
}

// ---- Menu navigation -----------------------------------------------------

#[test]
fn app_starts_on_the_main_menu() {
    let app = App::clara_bw();
    assert!(!app.in_game());
}

#[test]
fn main_menu_to_local_to_new_game() {
    let mut app = App::clara_bw();
    // MainMenu has 4 items; LOCAL is index 0.
    assert!(tap_menu(&mut app, 4, 0));
    assert!(!app.in_game());
    // LocalMenu has 3 items; NEW GAME is index 0.
    assert!(tap_menu(&mut app, 3, 0));
    assert!(app.in_game());
}

#[test]
fn main_menu_quit_sets_flag() {
    let mut app = App::clara_bw();
    assert!(!app.should_quit());
    tap_menu(&mut app, 4, 3); // QUIT
    assert!(app.should_quit());
}

#[test]
fn chesscom_and_lichess_show_coming_soon_then_back() {
    let mut app = App::clara_bw();
    tap_menu(&mut app, 4, 1); // CHESS.COM -> ComingSoon (2 items)
    assert!(!app.in_game());
    assert!(tap_menu(&mut app, 2, 1)); // BACK -> MainMenu
    // Back on main menu, LOCAL still works.
    assert!(tap_menu(&mut app, 4, 0));
}

#[test]
fn menu_control_returns_to_local_menu() {
    let mut app = game_app();
    app.tap_square(sq("e2"));
    app.tap_square(sq("e4"));
    assert!(app.press(Control::Menu));
    assert!(!app.in_game());
}

// ---- History -------------------------------------------------------------

#[test]
fn finished_game_is_saved_to_history_on_leaving() {
    let mut app = game_app();
    // Fool's mate: 1. f3 e5 2. g4 Qh4#
    for (a, b) in [("f2", "f3"), ("e7", "e5"), ("g2", "g4"), ("d8", "h4")] {
        app.tap_square(sq(a));
        app.tap_square(sq(b));
    }
    assert!(app.game().status().is_over());
    assert!(app.history().is_empty());
    app.press(Control::Menu); // leaving saves it
    assert_eq!(app.history().len(), 1);
    assert_eq!(app.history().games[0].move_count(), 4);
}

#[test]
fn empty_game_is_not_saved() {
    let mut app = game_app();
    app.press(Control::Menu);
    assert!(app.history().is_empty());
}

#[test]
fn history_replay_loads_the_saved_game() {
    let mut app = game_app();
    for (a, b) in [("e2", "e4"), ("e7", "e5"), ("g1", "f3")] {
        app.tap_square(sq(a));
        app.tap_square(sq(b));
    }
    app.press(Control::Menu); // LocalMenu, saved (3 plies)
    // LocalMenu: GAME HISTORY is index 1.
    assert!(tap_menu(&mut app, 3, 1));
    // History screen: 1 saved game + BACK => 2 items; replay index 0.
    assert!(tap_menu(&mut app, 2, 0));
    assert!(app.in_game());
    assert_eq!(app.game().ply(), 3);
    // Replay starts at the beginning of the game.
    assert_eq!(app.game().viewed_ply(), 0);
}

#[test]
fn history_persists_through_the_store() {
    let shared = Rc::new(RefCell::new(History::new()));
    {
        let mut app = App::with_store(1072, 1448, Box::new(MemStore(shared.clone())));
        app.start_new_local_game();
        app.tap_square(sq("e2"));
        app.tap_square(sq("e4"));
        app.press(Control::Menu); // saves through the store
    }
    // A fresh app with the same backing store loads the saved game.
    let app2 = App::with_store(1072, 1448, Box::new(MemStore(shared.clone())));
    assert_eq!(app2.history().len(), 1);
    assert_eq!(app2.history().games[0].move_count(), 1); // one ply: e2e4
}

#[test]
fn history_serialize_round_trips() {
    let mut app = game_app();
    for (a, b) in [("e2", "e4"), ("c7", "c5")] {
        app.tap_square(sq(a));
        app.tap_square(sq(b));
    }
    app.press(Control::New); // saves game 1
    app.tap_square(sq("d2"));
    app.tap_square(sq("d4"));
    app.press(Control::Menu); // saves game 2
    assert_eq!(app.history().len(), 2);

    let text = app.history().serialize();
    let parsed = History::parse(&text);
    assert_eq!(&parsed, app.history());
}
