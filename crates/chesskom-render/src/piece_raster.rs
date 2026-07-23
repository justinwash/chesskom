//! Classic raster piece set (Cburnett), embedded and drawn with alpha
//! compositing. The asset is produced by `scripts/gen-pieces.py`; see that file
//! and `ATTRIBUTION.md` for provenance and licensing.
//!
//! Each piece is stored at a fixed square resolution as two 8-bit planes:
//! luminance (the ink color) and alpha (coverage). We box-downsample the plane to
//! the target square size and composite `out = bg*(1-a) + lum*a`, so the pieces
//! stay clean at any board size and blend correctly over either square shade.

use crate::canvas::Canvas;
use chess_core::{Color, PieceKind};

static ASSET: &[u8] = include_bytes!("../assets/pieces.bin");

/// Parsed header + a view into the embedded blob.
struct Atlas {
    size: usize,
    /// Byte offset of the first piece plane.
    data_start: usize,
}

impl Atlas {
    fn get() -> Atlas {
        // magic(4) count(1) size(2) then piece planes.
        debug_assert_eq!(&ASSET[0..4], b"CKP1");
        let size = u16::from_le_bytes([ASSET[5], ASSET[6]]) as usize;
        Atlas {
            size,
            data_start: 7,
        }
    }

    fn index(kind: PieceKind, color: Color) -> usize {
        let k = match kind {
            PieceKind::Pawn => 0,
            PieceKind::Knight => 1,
            PieceKind::Bishop => 2,
            PieceKind::Rook => 3,
            PieceKind::Queen => 4,
            PieceKind::King => 5,
        };
        let c = match color {
            Color::White => 0,
            Color::Black => 1,
        };
        c * 6 + k
    }

    /// (luminance plane, alpha plane) for a piece.
    fn planes(&self, kind: PieceKind, color: Color) -> (&[u8], &[u8]) {
        let plane = self.size * self.size;
        let base = self.data_start + Atlas::index(kind, color) * plane * 2;
        (
            &ASSET[base..base + plane],
            &ASSET[base + plane..base + plane * 2],
        )
    }
}

/// True if a usable embedded asset is present (lets callers fall back to vector
/// pieces if the asset were ever stripped).
pub fn available() -> bool {
    ASSET.len() > 7 && &ASSET[0..4] == b"CKP1"
}

/// Draw a classic piece into the square at (x, y) of side `square`, composited
/// over background value `bg`.
pub fn draw(c: &mut Canvas, kind: PieceKind, color: Color, x: u32, y: u32, square: u32, bg: u8) {
    let atlas = Atlas::get();
    let (lum, alpha) = atlas.planes(kind, color);
    let src = atlas.size as u32;

    for py in 0..square {
        // Source row span this output row maps to (box filter).
        let sy0 = (py * src) / square;
        let sy1 = (((py + 1) * src) / square).max(sy0 + 1).min(src);
        for px in 0..square {
            let sx0 = (px * src) / square;
            let sx1 = (((px + 1) * src) / square).max(sx0 + 1).min(src);

            let mut a_sum = 0u32; // sum of alpha over the block
            let mut la_sum = 0u32; // sum of lum*alpha over the block
            let mut n = 0u32; // pixels in the block
            for sy in sy0..sy1 {
                for sx in sx0..sx1 {
                    let idx = (sy * src + sx) as usize;
                    let a = alpha[idx] as u32;
                    a_sum += a;
                    la_sum += lum[idx] as u32 * a;
                    n += 1;
                }
            }
            if a_sum == 0 {
                continue; // fully transparent — leave the square background
            }
            let coverage = a_sum as f32 / (n as f32 * 255.0);
            let ink = la_sum as f32 / a_sum as f32; // alpha-weighted luminance
            let out = bg as f32 * (1.0 - coverage) + ink * coverage;
            c.set(x + px, y + py, out.round() as u8);
        }
    }
}
