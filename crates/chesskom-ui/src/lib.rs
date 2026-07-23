//! `chesskom-ui`: the interactive app — screens, the game controller, and history.
//!
//! The app is a small screen state machine:
//!   MainMenu ──▶ LocalMenu ──▶ Game (play)
//!      │             └───────▶ History ──▶ Game (replay)
//!      ├──▶ ComingSoon("CHESS.COM")   (milestone 4)
//!      └──▶ ComingSoon("LICHESS")     (milestone 5)
//!
//! It performs no I/O of its own: touch comes in as pixel taps, output is a
//! rendered [`Canvas`], and history persistence goes through a [`HistoryStore`]
//! the caller supplies. That keeps the whole thing unit-testable; the Kobo binary
//! is a thin loop that feeds taps, paints frames, and provides a file-backed store.

pub mod history;
pub mod menu;

use chess_core::{Color, Game, Move, MoveFlag, PieceKind, Square, Status};
use chesskom_render::{render, Canvas, Control, HitTarget, PieceStyle, RenderOptions};

pub use history::{History, HistoryStore, NullStore, SavedGame};
use menu::MenuLayout;

/// Which screen is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    MainMenu,
    LocalMenu,
    History,
    ComingSoon(&'static str),
    Game,
}

/// The interactive application state.
pub struct App {
    screen: Screen,
    game: Game,
    selected: Option<Square>,
    orient: Color,
    style: PieceStyle,
    width: u32,
    height: u32,
    quit: bool,
    history: History,
    store: Box<dyn HistoryStore>,
}

impl App {
    /// Build an app with a specific screen size and history store.
    pub fn with_store(width: u32, height: u32, store: Box<dyn HistoryStore>) -> App {
        let history = store.load();
        App {
            screen: Screen::MainMenu,
            game: Game::new(),
            selected: None,
            orient: Color::White,
            style: PieceStyle::Classic,
            width,
            height,
            quit: false,
            history,
            store,
        }
    }

    /// Full-screen Clara BW app with no persistence (desktop/testing default).
    pub fn clara_bw() -> App {
        App::with_store(1072, 1448, Box::new(NullStore))
    }

    pub fn should_quit(&self) -> bool {
        self.quit
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

    pub fn history(&self) -> &History {
        &self.history
    }

    /// True when a board (play/replay) is showing rather than a menu.
    pub fn in_game(&self) -> bool {
        self.screen == Screen::Game
    }

    /// Replace the current game and show the board (e.g. to load a position).
    pub fn set_game(&mut self, game: Game) {
        self.game = game;
        self.selected = None;
        self.screen = Screen::Game;
    }

    /// Start a fresh local game and switch to the board.
    pub fn start_new_local_game(&mut self) {
        self.save_current_game();
        self.game = Game::new();
        self.selected = None;
        self.orient = Color::White;
        self.screen = Screen::Game;
    }

    // ---- Input ------------------------------------------------------------

    /// Handle a tap at pixel (x, y). Returns true if the display should redraw.
    pub fn tap_pixel(&mut self, x: u32, y: u32) -> bool {
        match self.screen {
            Screen::Game => match self.render_options().layout().hit(x, y) {
                Some(HitTarget::Square(sq)) => self.tap_square(sq),
                Some(HitTarget::Button(c)) => self.press(c),
                None => false,
            },
            _ => {
                let (_, labels) = self.menu_spec();
                let ml = MenuLayout::new(self.width, self.height, labels.len());
                match ml.hit(x, y) {
                    Some(i) => self.on_menu_select(i),
                    None => false,
                }
            }
        }
    }

    /// Handle a tap on a board square (game screen only).
    pub fn tap_square(&mut self, sq: Square) -> bool {
        if self.screen != Screen::Game {
            return false;
        }
        // Reviewing history: a board tap returns to the live game.
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
                if pos.piece_at(sq).map(|p| p.color) == Some(side) && self.is_selectable(sq) {
                    self.selected = Some(sq);
                    return true;
                }
                self.selected = None;
                true
            }
        }
    }

    /// Handle a control-button press (game screen).
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
                self.save_current_game();
                self.game = Game::new();
                self.selected = None;
                self.orient = Color::White;
                true
            }
            Control::Menu => {
                self.save_current_game();
                self.selected = None;
                self.screen = Screen::LocalMenu;
                true
            }
        }
    }

    /// Handle selecting item `i` on the current menu screen.
    fn on_menu_select(&mut self, i: usize) -> bool {
        match self.screen {
            Screen::MainMenu => {
                match i {
                    0 => self.screen = Screen::LocalMenu,
                    1 => self.screen = Screen::ComingSoon("CHESS.COM"),
                    2 => self.screen = Screen::ComingSoon("LICHESS"),
                    3 => self.quit = true,
                    _ => return false,
                }
                true
            }
            Screen::LocalMenu => {
                match i {
                    0 => self.start_new_local_game(),
                    1 => self.screen = Screen::History,
                    2 => self.screen = Screen::MainMenu,
                    _ => return false,
                }
                true
            }
            Screen::History => {
                if self.history.is_empty() {
                    // Items: ["(NO SAVED GAMES)", "BACK"].
                    if i == 1 {
                        self.screen = Screen::LocalMenu;
                        return true;
                    }
                    return false;
                }
                let n = self.history.len();
                if i < n {
                    // Replay: load the game and start at its beginning.
                    self.game = self.history.games[i].to_game();
                    self.game.rewind_to_start();
                    self.selected = None;
                    self.orient = Color::White;
                    self.screen = Screen::Game;
                    true
                } else if i == n {
                    self.screen = Screen::LocalMenu;
                    true
                } else {
                    false
                }
            }
            Screen::ComingSoon(_) => {
                // Items: ["COMING SOON", "BACK"].
                if i == 1 {
                    self.screen = Screen::MainMenu;
                    return true;
                }
                false
            }
            Screen::Game => false,
        }
    }

    // ---- Rendering --------------------------------------------------------

    /// Render options for the game screen (also what the touch layer hit-tests).
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

    /// Render the current screen to a canvas.
    pub fn render(&self) -> Canvas {
        match self.screen {
            Screen::Game => render(self.game.viewed(), &self.render_options()),
            _ => {
                let (title, labels) = self.menu_spec();
                MenuLayout::new(self.width, self.height, labels.len()).render(&title, &labels)
            }
        }
    }

    /// Title and item labels for the current menu screen.
    fn menu_spec(&self) -> (String, Vec<String>) {
        match self.screen {
            Screen::MainMenu => (
                "CHESSKOM".to_string(),
                vec![
                    "LOCAL".to_string(),
                    "CHESS.COM".to_string(),
                    "LICHESS".to_string(),
                    "QUIT".to_string(),
                ],
            ),
            Screen::LocalMenu => (
                "LOCAL".to_string(),
                vec![
                    "NEW GAME".to_string(),
                    "GAME HISTORY".to_string(),
                    "BACK".to_string(),
                ],
            ),
            Screen::History => {
                let title = "HISTORY".to_string();
                if self.history.is_empty() {
                    (title, vec!["(NO SAVED GAMES)".to_string(), "BACK".to_string()])
                } else {
                    let mut labels: Vec<String> = self
                        .history
                        .games
                        .iter()
                        .enumerate()
                        .map(|(i, g)| g.label(i))
                        .collect();
                    labels.push("BACK".to_string());
                    (title, labels)
                }
            }
            Screen::ComingSoon(name) => (
                name.to_string(),
                vec!["COMING SOON".to_string(), "BACK".to_string()],
            ),
            Screen::Game => ("CHESSKOM".to_string(), Vec::new()),
        }
    }

    // ---- Helpers ----------------------------------------------------------

    /// Push the current game into history (if it has moves) and persist.
    fn save_current_game(&mut self) {
        if self.game.ply() > 0 {
            self.history.add(SavedGame::from_game(&self.game));
            self.store.save(&self.history);
        }
    }

    fn is_selectable(&self, sq: Square) -> bool {
        let pos = self.game.current();
        match pos.piece_at(sq) {
            Some(p) if p.color == pos.side_to_move => {
                pos.legal_moves().iter().any(|m| m.from == sq)
            }
            _ => false,
        }
    }

    /// If moving `from`→`to` is legal, return the promotion to use (queen by
    /// default). `None` if illegal.
    fn move_to(&self, from: Square, to: Square) -> Option<Option<PieceKind>> {
        let pos = self.game.current();
        let legal: Vec<Move> = pos
            .legal_moves()
            .into_iter()
            .filter(|m| m.from == from && m.to == to)
            .collect();
        if legal.is_empty() {
            return None;
        }
        let is_promo = legal.iter().any(|m| m.promotion.is_some());
        Some(if is_promo { Some(PieceKind::Queen) } else { None })
    }

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
