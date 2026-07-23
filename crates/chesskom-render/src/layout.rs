//! Shared board/control geometry and hit-testing.
//!
//! The same `Layout` drives both drawing (where squares and buttons go) and input
//! (which square or button a tap landed on), so the two can never drift apart —
//! essential for a touch UI where a misaligned hitbox is invisible until you tap.

use chess_core::{Color, Square};

/// On-screen control buttons in the bottom bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    First, // jump view to the initial position
    Prev,  // step back one half-move
    Next,  // step forward one half-move
    Live,  // jump view to the current position
    Flip,  // flip board orientation
    New,   // start a new game
    Quit,  // exit
}

impl Control {
    pub fn label(self) -> &'static str {
        match self {
            Control::First => "FIRST",
            Control::Prev => "PREV",
            Control::Next => "NEXT",
            Control::Live => "LIVE",
            Control::Flip => "FLIP",
            Control::New => "NEW",
            Control::Quit => "QUIT",
        }
    }

    /// The bar's buttons, left to right.
    pub fn bar() -> [Control; 7] {
        [
            Control::First,
            Control::Prev,
            Control::Next,
            Control::Live,
            Control::Flip,
            Control::New,
            Control::Quit,
        ]
    }
}

/// A pixel rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// What a tap landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTarget {
    Square(Square),
    Button(Control),
}

/// Computed geometry for a given screen size and orientation.
pub struct Layout {
    pub width: u32,
    pub height: u32,
    pub orient: Color,
    pub margin: u32,
    pub label: u32,
    pub header_h: u32,
    pub board_x: u32,
    pub board_y: u32,
    pub square: u32,
    pub board_px: u32,
    /// Present only when a control bar is requested.
    pub controls: Vec<(Control, Rect)>,
}

impl Layout {
    pub fn new(
        width: u32,
        height: u32,
        orient: Color,
        coords: bool,
        header: bool,
        controls: bool,
    ) -> Layout {
        let margin = width / 24;
        let label = if coords { width / 18 } else { 0 };
        let header_h = if header { width / 12 } else { margin };

        let avail_w = width - 2 * margin - label;
        let square = avail_w / 8;
        let board_px = square * 8;
        let board_x = margin + label;
        let board_y = header_h + margin;

        let controls = if controls {
            Self::control_rects(width, height, margin)
        } else {
            Vec::new()
        };

        Layout {
            width,
            height,
            orient,
            margin,
            label,
            header_h,
            board_x,
            board_y,
            square,
            board_px,
            controls,
        }
    }

    fn control_rects(width: u32, height: u32, margin: u32) -> Vec<(Control, Rect)> {
        let bar = Control::bar();
        let n = bar.len() as u32;
        let bar_h = width / 8;
        let bar_y = height - margin - bar_h;
        let gap = width / 120;
        let usable = width - 2 * margin - gap * (n - 1);
        let bw = usable / n;
        let mut out = Vec::with_capacity(bar.len());
        for (i, c) in bar.iter().enumerate() {
            let x = margin + i as u32 * (bw + gap);
            out.push((*c, Rect { x, y: bar_y, w: bw, h: bar_h }));
        }
        out
    }

    /// Screen cell (col, row) for board (file, rank) under the orientation.
    pub fn cell_for(&self, file: i8, rank: i8) -> (u32, u32) {
        match self.orient {
            Color::White => (file as u32, (7 - rank) as u32),
            Color::Black => ((7 - file) as u32, rank as u32),
        }
    }

    /// Pixel top-left of a board square.
    pub fn square_origin(&self, file: i8, rank: i8) -> (u32, u32) {
        let (col, row) = self.cell_for(file, rank);
        (self.board_x + col * self.square, self.board_y + row * self.square)
    }

    /// Map a tap at (x, y) to a square or control button, if any.
    pub fn hit(&self, x: u32, y: u32) -> Option<HitTarget> {
        for (c, r) in &self.controls {
            if r.contains(x, y) {
                return Some(HitTarget::Button(*c));
            }
        }
        if x >= self.board_x
            && y >= self.board_y
            && x < self.board_x + self.board_px
            && y < self.board_y + self.board_px
        {
            let col = (x - self.board_x) / self.square;
            let row = (y - self.board_y) / self.square;
            // Invert cell_for.
            let (file, rank) = match self.orient {
                Color::White => (col as i8, 7 - row as i8),
                Color::Black => (7 - col as i8, row as i8),
            };
            return Square::from_file_rank(file, rank).map(HitTarget::Square);
        }
        None
    }
}
