use super::*;
use chess_core::{parse_fen, Game};

fn sq(s: &str) -> Square {
    Square::from_algebraic(s).unwrap()
}

fn targets(app: &App) -> Vec<Square> {
    app.render_options().targets
}

#[test]
fn tapping_a_piece_selects_it_and_shows_targets() {
    let mut app = App::clara_bw();
    assert!(app.tap_square(sq("e2")));
    assert_eq!(app.selected(), Some(sq("e2")));
    let t = targets(&app);
    assert!(t.contains(&sq("e3")) && t.contains(&sq("e4")));
}

#[test]
fn tapping_selected_piece_again_deselects() {
    let mut app = App::clara_bw();
    app.tap_square(sq("e2"));
    assert!(app.tap_square(sq("e2")));
    assert_eq!(app.selected(), None);
}

#[test]
fn tapping_empty_own_piece_does_nothing_when_nothing_selected() {
    let mut app = App::clara_bw();
    // e4 is empty at the start; no selection -> no change/redraw.
    assert!(!app.tap_square(sq("e4")));
    assert_eq!(app.selected(), None);
    // Opponent's piece is likewise not selectable for White.
    assert!(!app.tap_square(sq("e7")));
}

#[test]
fn select_then_tap_destination_makes_the_move() {
    let mut app = App::clara_bw();
    app.tap_square(sq("e2"));
    assert!(app.tap_square(sq("e4")));
    assert_eq!(app.selected(), None);
    assert_eq!(app.game().ply(), 1);
    assert_eq!(app.game().current().side_to_move, Color::Black);
    // The move landed on e4.
    assert!(app.game().current().piece_at(sq("e4")).is_some());
}

#[test]
fn tapping_another_own_piece_switches_selection() {
    let mut app = App::clara_bw();
    app.tap_square(sq("e2"));
    assert!(app.tap_square(sq("d2")));
    assert_eq!(app.selected(), Some(sq("d2")));
    assert_eq!(app.game().ply(), 0);
}

#[test]
fn tapping_an_illegal_empty_square_deselects_without_moving() {
    let mut app = App::clara_bw();
    app.tap_square(sq("e2"));
    // e5 is not a legal destination for the e2 pawn.
    assert!(app.tap_square(sq("e5")));
    assert_eq!(app.selected(), None);
    assert_eq!(app.game().ply(), 0);
}

#[test]
fn prev_enters_review_and_board_tap_returns_to_live() {
    let mut app = App::clara_bw();
    app.tap_square(sq("e2"));
    app.tap_square(sq("e4"));
    assert!(app.press(Control::Prev));
    assert!(!app.game().is_at_live());
    // No targets while reviewing.
    app.tap_square(sq("d7")); // any board tap
    assert!(app.game().is_at_live());
}

#[test]
fn first_and_live_navigation() {
    let mut app = App::clara_bw();
    for (a, b) in [("e2", "e4"), ("e7", "e5"), ("g1", "f3")] {
        app.tap_square(sq(a));
        app.tap_square(sq(b));
    }
    assert_eq!(app.game().ply(), 3);
    assert!(app.press(Control::First));
    assert_eq!(app.game().viewed_ply(), 0);
    assert!(app.press(Control::Live));
    assert!(app.game().is_at_live());
    // Pressing Live again is a no-op (no redraw).
    assert!(!app.press(Control::Live));
}

#[test]
fn flip_toggles_orientation() {
    let mut app = App::clara_bw();
    assert_eq!(app.orientation(), Color::White);
    assert!(app.press(Control::Flip));
    assert_eq!(app.orientation(), Color::Black);
}

#[test]
fn new_resets_the_game() {
    let mut app = App::clara_bw();
    app.tap_square(sq("e2"));
    app.tap_square(sq("e4"));
    assert!(app.press(Control::New));
    assert_eq!(app.game().ply(), 0);
    assert_eq!(app.orientation(), Color::White);
    assert_eq!(app.selected(), None);
}

#[test]
fn quit_sets_flag() {
    let mut app = App::clara_bw();
    assert!(!app.should_quit());
    app.press(Control::Quit);
    assert!(app.should_quit());
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
    assert_eq!(promoted.color, Color::White);
}

#[test]
fn tap_pixel_maps_through_the_shared_layout() {
    // A pixel in the center of e2's square should select e2, matching tap_square.
    let app0 = App::clara_bw();
    let lay = app0.render_options().layout();
    let (x, y) = lay.square_origin(sq("e2").file(), sq("e2").rank());
    let (cx, cy) = (x + lay.square / 2, y + lay.square / 2);

    let mut app = App::clara_bw();
    assert!(app.tap_pixel(cx, cy));
    assert_eq!(app.selected(), Some(sq("e2")));
}

#[test]
fn tap_pixel_hits_control_buttons() {
    let app0 = App::clara_bw();
    let lay = app0.render_options().layout();
    let (_, rect) = lay
        .controls
        .iter()
        .find(|(c, _)| *c == Control::Flip)
        .copied()
        .unwrap();

    let mut app = App::clara_bw();
    assert!(app.tap_pixel(rect.x + rect.w / 2, rect.y + rect.h / 2));
    assert_eq!(app.orientation(), Color::Black);
}
