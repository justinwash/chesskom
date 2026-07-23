//! Compose a full-screen grayscale image of a chess position: shaded squares,
//! coordinate labels, optional header/footer text, last-move highlight, and
//! anti-aliased vector pieces.

use crate::canvas::Canvas;
use crate::font;
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
            piece_style: PieceStyle::Classic,
            over_the_board: false,
        }
    }
}

const SS: u32 = 3; // supersampling factor for piece anti-aliasing.

/// Render `pos` into a new [`Canvas`] using `opts`.
pub fn render(pos: &Position, opts: &RenderOptions) -> Canvas {
    let mut c = Canvas::new(opts.width, opts.height, opts.bg);

    // ---- Layout -----------------------------------------------------------
    let margin = opts.width / 24;
    let label = if opts.coords { opts.width / 18 } else { 0 };
    let header_h = if opts.header.is_some() {
        opts.width / 12
    } else {
        margin
    };

    // Board is square, fit within width (accounting for labels on the left).
    let avail_w = opts.width - 2 * margin - label;
    let square = avail_w / 8;
    let board_px = square * 8;
    let board_x = margin + label;
    let board_y = header_h + margin;

    // ---- Header -----------------------------------------------------------
    if let Some(h) = &opts.header {
        let scale = (opts.width / 200).max(2);
        let w = font::text_width(h, scale);
        let x = (opts.width - w) / 2;
        draw_text(&mut c, x, margin, h, scale, 0);
    }

    // ---- Squares + highlight ---------------------------------------------
    for rank in 0..8i8 {
        for file in 0..8i8 {
            let sq = Square::from_file_rank(file, rank).unwrap();
            let (col, row) = to_screen(file, rank, opts.orient);
            let x = board_x + col * square;
            let y = board_y + row * square;
            let mut v = if (file + rank) % 2 == 0 {
                opts.dark_sq
            } else {
                opts.light_sq
            };
            // Darken highlighted squares a notch so they read on e-ink.
            if let Some((from, to)) = opts.highlight {
                if sq == from || sq == to {
                    v = v.saturating_sub(48);
                }
            }
            c.fill_rect(x as i32, y as i32, square as i32, square as i32, v);
        }
    }

    // ---- Board frame ------------------------------------------------------
    draw_frame(&mut c, board_x as i32, board_y as i32, board_px as i32, 2, 0);

    // ---- Coordinate labels ------------------------------------------------
    if opts.coords {
        let scale = (square / 24).max(2);
        for i in 0..8i8 {
            // Files along the bottom.
            let file = i;
            let (col, _) = to_screen(file, 0, opts.orient);
            let ch = (b'a' + file as u8) as char;
            let gx = board_x + col * square + square / 2 - (font::GLYPH_W * scale) / 2;
            let gy = board_y + board_px + square / 8;
            draw_text(&mut c, gx, gy, &ch.to_string(), scale, 0);

            // Ranks along the left.
            let rank = i;
            let (_, row) = to_screen(0, rank, opts.orient);
            let ch = (b'1' + rank as u8) as char;
            let ly = board_y + row * square + square / 2 - (font::GLYPH_H * scale) / 2;
            let lx = board_x - label + (label - font::GLYPH_W * scale) / 2;
            draw_text(&mut c, lx, ly, &ch.to_string(), scale, 0);
        }
    }

    // ---- Pieces -----------------------------------------------------------
    for rank in 0..8i8 {
        for file in 0..8i8 {
            let sq = Square::from_file_rank(file, rank).unwrap();
            if let Some(piece) = pos.piece_at(sq) {
                let (col, row) = to_screen(file, rank, opts.orient);
                let x = board_x + col * square;
                let y = board_y + row * square;
                let sq_bg = if (file + rank) % 2 == 0 {
                    opts.dark_sq
                } else {
                    opts.light_sq
                };
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

    // ---- Footer -----------------------------------------------------------
    if let Some(f) = &opts.footer {
        let scale = (opts.width / 220).max(2);
        let w = font::text_width(f, scale);
        let x = (opts.width - w) / 2;
        let y = board_y + board_px + square / 2;
        draw_text(&mut c, x, y, f, scale, 0);
    }

    c
}

/// Map (file, rank) board coordinates to (col, row) screen cells for an orientation.
fn to_screen(file: i8, rank: i8, orient: Color) -> (u32, u32) {
    match orient {
        Color::White => (file as u32, (7 - rank) as u32),
        Color::Black => ((7 - file) as u32, rank as u32),
    }
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
