//! Compose a full-screen grayscale image of a chess position: shaded squares,
//! coordinate labels, optional header/footer text, last-move highlight, and
//! anti-aliased vector pieces.

use crate::canvas::Canvas;
use crate::font;
use crate::layout::Layout;
use crate::piece_raster;
use crate::pieces;
use chess_core::{Color, Position, Square};

/// Which piece artwork to draw.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PieceStyle {
    /// Classic Cburnett raster set (embedded asset). Falls back to `Vector` if
    /// the asset is unavailable.
    Classic,
    /// Dependency-free anti-aliased vector silhouettes (no asset needed).
    Vector,
}

/// Look and layout of a rendered board. Defaults target the Kobo Clara BW
/// (1072x1448 portrait, grayscale).
#[derive(Clone)]
pub struct RenderOptions {
    pub width: u32,
    pub height: u32,
    /// Which side sits at the bottom.
    pub orient: Color,
    /// Grayscale values for the two square colors and the page background.
    pub light_sq: u8,
    pub dark_sq: u8,
    pub bg: u8,
    /// Draw file/rank labels around the board.
    pub coords: bool,
    /// Optional lines of text above and below the board.
    pub header: Option<String>,
    pub footer: Option<String>,
    /// Highlight the from/to squares of the last move.
    pub highlight: Option<(Square, Square)>,
    /// A currently-selected square (drawn shaded).
    pub selected: Option<Square>,
    /// Legal destinations to mark (dots for quiet moves, rings for captures).
    pub targets: Vec<Square>,
    /// Draw the bottom control bar (and reserve space for it).
    pub controls: bool,
    /// Which piece artwork to draw.
    pub piece_style: PieceStyle,
    /// "Over the board" mode: rotate the far side's pieces 180° so two players
    /// facing each other across a flat device each read their own pieces upright.
    ///
    /// TODO (when we build actual gameplay): consider a "face the current player"
    /// variant — flip the *whole* board 180° between turns so the player on move
    /// always sees the board from their side. The per-piece rotation we already
    /// have is the building block; this just applies it to everything based on
    /// whose turn it is. Bonus: in that mode the orientation itself signals whose
    /// turn it is (the side seeing upright pieces is on move), so the explicit
    /// "White/Black to move" text becomes redundant and can be dropped. Deferred
    /// until the interactive play loop exists, since it needs turn state to drive it.
    pub over_the_board: bool,
}

impl RenderOptions {
    /// Full-screen defaults for the Clara BW.
    pub fn clara_bw() -> RenderOptions {
        RenderOptions {
            width: 1072,
            height: 1448,
            orient: Color::White,
            light_sq: 255,
            dark_sq: 168,
            bg: 255,
            coords: true,
            header: None,
            footer: None,
            highlight: None,
            selected: None,
            targets: Vec::new(),
            controls: false,
            piece_style: PieceStyle::Classic,
            over_the_board: false,
        }
    }

    /// The geometry this render will use — also what a touch layer hit-tests against.
    pub fn layout(&self) -> Layout {
        Layout::new(
            self.width,
            self.height,
            self.orient,
            self.coords,
            self.header.is_some(),
            self.controls,
        )
    }
}

const SS: u32 = 3; // supersampling factor for piece anti-aliasing.

/// Render `pos` into a new [`Canvas`] using `opts`.
pub fn render(pos: &Position, opts: &RenderOptions) -> Canvas {
    let mut c = Canvas::new(opts.width, opts.height, opts.bg);
    let lay = opts.layout();
    let square = lay.square;
    let board_px = lay.board_px;

    // ---- Header -----------------------------------------------------------
    if let Some(h) = &opts.header {
        let scale = (opts.width / 200).max(2);
        let w = font::text_width(h, scale);
        let x = (opts.width - w) / 2;
        draw_text(&mut c, x, lay.margin, h, scale, 0);
    }

    // ---- Squares (with last-move highlight + selection shading) -----------
    for rank in 0..8i8 {
        for file in 0..8i8 {
            let sq = Square::from_file_rank(file, rank).unwrap();
            let (x, y) = lay.square_origin(file, rank);
            let mut v = if (file + rank) % 2 == 0 {
                opts.dark_sq
            } else {
                opts.light_sq
            };
            if let Some((from, to)) = opts.highlight {
                if sq == from || sq == to {
                    v = v.saturating_sub(40);
                }
            }
            if opts.selected == Some(sq) {
                v = v.saturating_sub(72); // stronger shade for the picked square
            }
            c.fill_rect(x as i32, y as i32, square as i32, square as i32, v);
        }
    }

    // ---- Board frame ------------------------------------------------------
    draw_frame(&mut c, lay.board_x as i32, lay.board_y as i32, board_px as i32, 2, 0);

    // ---- Coordinate labels ------------------------------------------------
    if opts.coords {
        let scale = (square / 24).max(2);
        for i in 0..8i8 {
            let (col, _) = lay.cell_for(i, 0);
            let ch = (b'a' + i as u8) as char;
            let gx = lay.board_x + col * square + square / 2 - (font::GLYPH_W * scale) / 2;
            let gy = lay.board_y + board_px + square / 8;
            draw_text(&mut c, gx, gy, &ch.to_string(), scale, 0);

            let (_, row) = lay.cell_for(0, i);
            let ch = (b'1' + i as u8) as char;
            let ly = lay.board_y + row * square + square / 2 - (font::GLYPH_H * scale) / 2;
            let lx = lay.board_x - lay.label + (lay.label - font::GLYPH_W * scale) / 2;
            draw_text(&mut c, lx, ly, &ch.to_string(), scale, 0);
        }
    }

    // ---- Pieces -----------------------------------------------------------
    for rank in 0..8i8 {
        for file in 0..8i8 {
            let sq = Square::from_file_rank(file, rank).unwrap();
            if let Some(piece) = pos.piece_at(sq) {
                let (x, y) = lay.square_origin(file, rank);
                let sq_bg = square_shade(opts, sq, file, rank);
                // In OTB mode, rotate the pieces of the side that isn't at the
                // bottom so the opposing player reads them upright.
                let flip = opts.over_the_board && piece.color == opts.orient.opponent();
                let use_classic =
                    opts.piece_style == PieceStyle::Classic && piece_raster::available();
                if use_classic {
                    piece_raster::draw(&mut c, piece.kind, piece.color, x, y, square, sq_bg, flip);
                } else {
                    draw_piece(&mut c, piece.kind, piece.color, x, y, square, sq_bg, flip);
                }
            }
        }
    }

    // ---- Move-target markers (over the pieces) ---------------------------
    for &sq in &opts.targets {
        let (x, y) = lay.square_origin(sq.file(), sq.rank());
        let cx = x as f32 + square as f32 / 2.0;
        let cy = y as f32 + square as f32 / 2.0;
        if pos.piece_at(sq).is_some() {
            // Capture: a ring around the occupied square.
            let r = square as f32 * 0.46;
            c.ring(cx, cy, r, square as f32 * 0.09, 64);
        } else {
            // Quiet move: a centered dot.
            c.disc(cx, cy, square as f32 * 0.15, 96);
        }
    }

    // ---- Footer / status --------------------------------------------------
    if let Some(f) = &opts.footer {
        let scale = (opts.width / 220).max(2);
        let w = font::text_width(f, scale);
        let x = (opts.width - w) / 2;
        let y = lay.board_y + board_px + square / 2;
        draw_text(&mut c, x, y, f, scale, 0);
    }

    // ---- Control bar ------------------------------------------------------
    if opts.controls {
        draw_controls(&mut c, &lay);
    }

    c
}

/// The background shade of a square (accounting for selection/highlight), used so
/// pieces anti-alias over the exact color behind them.
fn square_shade(opts: &RenderOptions, sq: Square, file: i8, rank: i8) -> u8 {
    let mut v = if (file + rank) % 2 == 0 {
        opts.dark_sq
    } else {
        opts.light_sq
    };
    if let Some((from, to)) = opts.highlight {
        if sq == from || sq == to {
            v = v.saturating_sub(40);
        }
    }
    if opts.selected == Some(sq) {
        v = v.saturating_sub(72);
    }
    v
}

/// Draw the bottom control bar: a labeled, outlined button per control.
fn draw_controls(c: &mut Canvas, lay: &Layout) {
    for (ctrl, r) in &lay.controls {
        // Button face + inset border.
        c.fill_rect(r.x as i32, r.y as i32, r.w as i32, r.h as i32, 244);
        outline_rect(c, r.x as i32, r.y as i32, r.w as i32, r.h as i32, 2, 0);
        let label = ctrl.label();
        let scale = (lay.width / 320).max(2);
        let tw = font::text_width(label, scale);
        let th = font::GLYPH_H * scale;
        let tx = r.x + (r.w.saturating_sub(tw)) / 2;
        let ty = r.y + (r.h.saturating_sub(th)) / 2;
        draw_text(c, tx, ty, label, scale, 0);
    }
}

/// Draw a border just inside the given rectangle.
fn outline_rect(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, thick: i32, v: u8) {
    c.fill_rect(x, y, w, thick, v); // top
    c.fill_rect(x, y + h - thick, w, thick, v); // bottom
    c.fill_rect(x, y, thick, h, v); // left
    c.fill_rect(x + w - thick, y, thick, h, v); // right
}

/// Draw a piece into the square at (x,y) of side `square`, anti-aliased over `bg`.
/// When `flip` is set the piece is rotated 180° (OTB mode).
#[allow(clippy::too_many_arguments)] // low-level blit; a struct would only obscure it
fn draw_piece(c: &mut Canvas, kind: chess_core::PieceKind, color: Color, x: u32, y: u32, square: u32, bg: u8, flip: bool) {
    let msize = square * SS;
    let mask = pieces::silhouette(kind, msize);
    // Outline thickness scales with size; ~2px at final resolution.
    let outline = ((msize / 44).max(SS)) as i32;
    let interior = mask.erode(outline);
    let fill: i32 = if color == Color::White { 255 } else { 0 };
    let last = (msize - 1) as i32;

    for py in 0..square {
        for px in 0..square {
            let mut cov = 0u32;
            let mut ink = 0u32;
            for sy in 0..SS {
                for sx in 0..SS {
                    let mut mx = (px * SS + sx) as i32;
                    let mut my = (py * SS + sy) as i32;
                    if flip {
                        mx = last - mx;
                        my = last - my;
                    }
                    if mask.get(mx, my) {
                        cov += 1;
                        // Border pixels are black; interior takes the fill color.
                        ink += if interior.get(mx, my) { fill as u32 } else { 0 };
                    }
                }
            }
            if cov == 0 {
                continue; // leave the square background
            }
            let total = SS * SS;
            let ink_avg = (ink / cov) as f32;
            let coverage = cov as f32 / total as f32;
            let out = bg as f32 * (1.0 - coverage) + ink_avg * coverage;
            c.set(x + px, y + py, out.round() as u8);
        }
    }
}

/// Draw a hollow rectangle border of the given thickness.
fn draw_frame(c: &mut Canvas, x: i32, y: i32, size: i32, thick: i32, v: u8) {
    c.fill_rect(x - thick, y - thick, size + 2 * thick, thick, v); // top
    c.fill_rect(x - thick, y + size, size + 2 * thick, thick, v); // bottom
    c.fill_rect(x - thick, y, thick, size, v); // left
    c.fill_rect(x + size, y, thick, size, v); // right
}

/// Draw `text` with the built-in 5x7 font at integer `scale`.
pub fn draw_text(c: &mut Canvas, x: u32, y: u32, text: &str, scale: u32, v: u8) {
    let mut cx = x;
    for ch in text.chars() {
        let g = font::glyph(ch);
        for (row, bits) in g.iter().enumerate() {
            for col in 0..font::GLYPH_W {
                // Bit 0x10 is the leftmost column.
                if bits & (0x10 >> col) != 0 {
                    let px = cx + col * scale;
                    let py = y + row as u32 * scale;
                    c.fill_rect(px as i32, py as i32, scale as i32, scale as i32, v);
                }
            }
        }
        cx += (font::GLYPH_W + 1) * scale;
    }
}
