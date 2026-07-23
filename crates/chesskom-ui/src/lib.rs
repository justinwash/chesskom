//! `chesskom-ui`: the interactive game controller.
//!
//! This is the platform-agnostic brain of the on-device app. It owns a [`Game`],
//! a selection, and board orientation, and turns two kinds of input — a tap on a
//! board square and a press of a control button — into game-state changes and a
//! rendered [`Canvas`]. It performs no I/O, so it is fully unit-testable; the Kobo
//! binary is a thin loop that feeds it touch events and paints its output.
//!
//! Tap model (Lichess-style tap-tap, no dragging — ideal for slow e-ink):
//! - Tap your piece to select it; legal destinations light up.
//! - Tap a destination to move; tap the piece again to deselect; tap another of
//!   your pieces to switch selection.
//! - While reviewing history, a board tap returns to the live position first.

use chess_core::{Color, Game, Move, MoveFlag, PieceKind, Square, Status};
use chesskom_render::{
    render, Canvas, Control, HitTarget, PieceStyle, RenderOptions,
};

/// The interactive application state.
pub struct App {
    game: Game,
    selected: Option<Square>,
    orient: Color,
    style: PieceStyle,
    width: u32,
    height: u32,
    quit: bool,
}

impl App {
    pub fn new(width: u32, height: u32) -> App {
        App {
            game: Game::new(),
            selected: None,
            orient: Color::White,
            style: PieceStyle::Classic,
            width,
            height,
            quit: false,
        }
    }

    /// Full-screen Clara BW app.
    pub fn clara_bw() -> App {
        App::new(1072, 1448)
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// Replace the current game (e.g. to load a position). Clears any selection.
    pub fn set_game(&mut self, game: Game) {
        self.game = game;
        self.selected = None;
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn selected(&self) -> Option<Square> {
        self.selected
    }

    pub fn orientation(&self) -> Color {
        self.orient
    }

    // ---- Input ------------------------------------------------------------

    /// Handle a tap at pixel (x, y). Returns true if the display should redraw.
    pub fn tap_pixel(&mut self, x: u32, y: u32) -> bool {
        match self.render_options().layout().hit(x, y) {
            Some(HitTarget::Square(sq)) => self.tap_square(sq),
            Some(HitTarget::Button(c)) => self.press(c),
            None => false,
        }
    }

    /// Handle a tap on a board square. Returns true if the display should redraw.
    pub fn tap_square(&mut self, sq: Square) -> bool {
        // Reviewing history: a board tap just returns to the live game.
        if !self.game.is_at_live() {
            self.game.forward_to_live();
            self.selected = None;
            return true;
        }
        if self.game.status().is_over() {
            return false;
        }

        let pos = self.game.current();
        let side = pos.side_to_move;

        match self.selected {
            None => {
                if self.is_selectable(sq) {
                    self.selected = Some(sq);
                    true
                } else {
                    false
                }
            }
            Some(from) => {
                if sq == from {
                    self.selected = None;
                    return true;
                }
                // A legal destination from the selected piece?
                if let Some(promo) = self.move_to(from, sq) {
                    let mv = Move {
                        from,
                        to: sq,
                        promotion: promo,
                        flag: MoveFlag::Normal,
                    };
                    let _ = self.game.make_move(mv);
                    self.selected = None;
                    return true;
                }
                // Switching to another of our own pieces?
                if pos.piece_at(sq).map(|p| p.color) == Some(side) && self.is_selectable(sq) {
                    self.selected = Some(sq);
                    return true;
                }
                // Anywhere else: deselect.
                self.selected = None;
                true
            }
        }
    }

    /// Handle a control-button press. Returns true if the display should redraw.
    pub fn press(&mut self, ctrl: Control) -> bool {
        match ctrl {
            Control::First => {
                self.game.rewind_to_start();
                self.selected = None;
                true
            }
            Control::Prev => {
                let moved = self.game.step_back();
                if moved {
                    self.selected = None;
                }
                moved
            }
            Control::Next => {
                let moved = self.game.step_forward();
                if moved {
                    self.selected = None;
                }
                moved
            }
            Control::Live => {
                if self.game.is_at_live() {
                    false
                } else {
                    self.game.forward_to_live();
                    self.selected = None;
                    true
                }
            }
            Control::Flip => {
                self.orient = self.orient.opponent();
                true
            }
            Control::New => {
                self.game = Game::new();
                self.selected = None;
                self.orient = Color::White;
                true
            }
            Control::Quit => {
                self.quit = true;
                false
            }
        }
    }

    // ---- Rendering --------------------------------------------------------

    /// Build the render options that reflect the current state.
    pub fn render_options(&self) -> RenderOptions {
        let mut opts = RenderOptions::clara_bw();
        opts.width = self.width;
        opts.height = self.height;
        opts.orient = self.orient;
        opts.piece_style = self.style;
        opts.controls = true;
        opts.header = Some("CHESSKOM".to_string());
        opts.footer = Some(self.status_text());
        opts.highlight = self.game.viewed_last_move().map(|m| (m.from, m.to));
        opts.selected = self.selected;
        opts.targets = self.target_squares();
        opts
    }

    /// Render the current state to a canvas.
    pub fn render(&self) -> Canvas {
        render(self.game.viewed(), &self.render_options())
    }

    // ---- Helpers ----------------------------------------------------------

    /// Is there at least one legal move from `sq` for the side to move?
    fn is_selectable(&self, sq: Square) -> bool {
        let pos = self.game.current();
        match pos.piece_at(sq) {
            Some(p) if p.color == pos.side_to_move => {
                pos.legal_moves().iter().any(|m| m.from == sq)
            }
            _ => false,
        }
    }

    /// If moving the selected piece from `from` to `to` is legal, return the
    /// promotion piece to use (`Some(None)` = legal non-promotion; `Some(Some(k))`
    /// = promotion, defaulting to queen). Returns `None` if the move is illegal.
    fn move_to(&self, from: Square, to: Square) -> Option<Option<PieceKind>> {
        let pos = self.game.current();
        let mut legal_here = pos
            .legal_moves()
            .into_iter()
            .filter(|m| m.from == from && m.to == to)
            .peekable();
        legal_here.peek()?;
        // If any legal move here is a promotion, default to queen.
        let is_promo = pos
            .legal_moves()
            .iter()
            .any(|m| m.from == from && m.to == to && m.promotion.is_some());
        Some(if is_promo { Some(PieceKind::Queen) } else { None })
    }

    /// Destination squares of the selected piece (deduplicated).
    fn target_squares(&self) -> Vec<Square> {
        let Some(from) = self.selected else {
            return Vec::new();
        };
        if !self.game.is_at_live() {
            return Vec::new();
        }
        let mut seen = Vec::new();
        for m in self.game.current().legal_moves() {
            if m.from == from && !seen.contains(&m.to) {
                seen.push(m.to);
            }
        }
        seen
    }

    fn status_text(&self) -> String {
        if !self.game.is_at_live() {
            return format!("VIEWING {}/{}", self.game.viewed_ply(), self.game.ply());
        }
        match self.game.status() {
            Status::Ongoing => {
                let side = match self.game.current().side_to_move {
                    Color::White => "WHITE",
                    Color::Black => "BLACK",
                };
                let check = if self.game.current().is_in_check() {
                    " - CHECK"
                } else {
                    ""
                };
                format!("{side} TO MOVE{check}")
            }
            Status::Checkmate { winner } => {
                let w = match winner {
                    Color::White => "WHITE",
                    Color::Black => "BLACK",
                };
                format!("CHECKMATE - {w} WINS")
            }
            Status::Stalemate => "DRAW - STALEMATE".to_string(),
            Status::FiftyMoveRule => "DRAW - 50 MOVE RULE".to_string(),
            Status::ThreefoldRepetition => "DRAW - REPETITION".to_string(),
            Status::InsufficientMaterial => "DRAW - INSUFFICIENT".to_string(),
        }
    }
}

#[cfg(test)]
mod tests;
