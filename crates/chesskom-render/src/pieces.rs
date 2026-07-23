//! Vector chess-piece silhouettes.
//!
//! Each piece is drawn as a filled silhouette into a [`Mask`] whose dimensions are
//! the (supersampled) square size. Coordinates are given in normalized square-space
//! (0.0..1.0 on both axes, y pointing down) so the same shapes render crisply at any
//! board size — important because the e-ink panel is large (1072px wide) and we want
//! the pieces resolution-independent rather than pixel art tied to one scale.

use crate::canvas::Mask;
use chess_core::PieceKind;

/// A drawing pen over a mask that accepts normalized (0..1) coordinates.
struct Pen<'a> {
    m: &'a mut Mask,
    s: f32,
}

impl<'a> Pen<'a> {
    fn new(m: &'a mut Mask) -> Pen<'a> {
        let s = m.w as f32;
        Pen { m, s }
    }
    fn rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.m
            .fill_rect(x0 * self.s, y0 * self.s, x1 * self.s, y1 * self.s);
    }
    fn circle(&mut self, cx: f32, cy: f32, r: f32) {
        self.m.fill_circle(cx * self.s, cy * self.s, r * self.s);
    }
    fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32) {
        self.m
            .fill_ellipse(cx * self.s, cy * self.s, rx * self.s, ry * self.s);
    }
    fn poly(&mut self, pts: &[(f32, f32)]) {
        let scaled: Vec<(f32, f32)> = pts.iter().map(|&(x, y)| (x * self.s, y * self.s)).collect();
        self.m.fill_polygon(&scaled);
    }
    /// The wide flared foot shared by most pieces.
    fn base(&mut self, half_bottom: f32, half_top: f32) {
        let (l0, r0) = (0.5 - half_bottom, 0.5 + half_bottom);
        let (l1, r1) = (0.5 - half_top, 0.5 + half_top);
        self.poly(&[(l0, 0.90), (r0, 0.90), (r1, 0.80), (l1, 0.80)]);
    }
}

/// Rasterize a piece silhouette into a mask of side `size` pixels.
pub fn silhouette(kind: PieceKind, size: u32) -> Mask {
    let mut m = Mask::new(size, size);
    let mut p = Pen::new(&mut m);
    match kind {
        PieceKind::Pawn => pawn(&mut p),
        PieceKind::Rook => rook(&mut p),
        PieceKind::Bishop => bishop(&mut p),
        PieceKind::Knight => knight(&mut p),
        PieceKind::Queen => queen(&mut p),
        PieceKind::King => king(&mut p),
    }
    m
}

fn pawn(p: &mut Pen) {
    p.base(0.20, 0.14);
    // Neck/body flaring up to a collar.
    p.poly(&[(0.40, 0.80), (0.60, 0.80), (0.55, 0.52), (0.45, 0.52)]);
    p.rect(0.40, 0.50, 0.60, 0.55); // collar
    p.circle(0.5, 0.40, 0.135); // head
}

fn rook(p: &mut Pen) {
    p.base(0.24, 0.18);
    p.rect(0.34, 0.44, 0.66, 0.80); // body
    p.rect(0.30, 0.36, 0.70, 0.44); // top band
    // Three crenellations.
    p.rect(0.30, 0.28, 0.40, 0.36);
    p.rect(0.45, 0.28, 0.55, 0.36);
    p.rect(0.60, 0.28, 0.70, 0.36);
}

fn bishop(p: &mut Pen) {
    p.base(0.22, 0.16);
    p.rect(0.42, 0.72, 0.58, 0.80); // collar
    p.ellipse(0.5, 0.55, 0.15, 0.20); // body bulb
    p.poly(&[(0.5, 0.26), (0.63, 0.55), (0.37, 0.55)]); // mitre
    p.circle(0.5, 0.26, 0.05); // tip
}

fn knight(p: &mut Pen) {
    p.base(0.24, 0.18);
    // Horse head/neck facing left, traced around the profile.
    p.poly(&[
        (0.44, 0.80), // neck base, front
        (0.40, 0.62), // throat
        (0.30, 0.56), // jaw
        (0.21, 0.50), // muzzle tip
        (0.25, 0.43), // top of muzzle
        (0.40, 0.41), // cheek
        (0.42, 0.33), // up the face
        (0.50, 0.23), // forehead / top of head
        (0.57, 0.24), // between forehead and ear
        (0.60, 0.31), // ear front
        (0.64, 0.21), // ear tip
        (0.66, 0.34), // ear back
        (0.65, 0.54), // back of neck
        (0.62, 0.70),
        (0.62, 0.80), // neck base, back
    ]);
    // Mane suggested by a couple of notches down the back of the neck.
    p.poly(&[(0.62, 0.40), (0.70, 0.44), (0.63, 0.50)]);
    p.poly(&[(0.63, 0.54), (0.71, 0.58), (0.63, 0.64)]);
}

fn queen(p: &mut Pen) {
    p.base(0.26, 0.20);
    p.poly(&[(0.34, 0.80), (0.66, 0.80), (0.60, 0.46), (0.40, 0.46)]); // skirt
    p.rect(0.36, 0.42, 0.64, 0.47); // collar band
    // Crown zigzag.
    p.poly(&[
        (0.34, 0.42),
        (0.36, 0.24),
        (0.43, 0.36),
        (0.50, 0.22),
        (0.57, 0.36),
        (0.64, 0.24),
        (0.66, 0.42),
    ]);
    // Balls on the five points.
    for &(x, y) in &[(0.36, 0.24), (0.43, 0.34), (0.50, 0.22), (0.57, 0.34), (0.64, 0.24)] {
        p.circle(x, y, 0.037);
    }
}

fn king(p: &mut Pen) {
    p.base(0.26, 0.20);
    p.poly(&[(0.34, 0.80), (0.66, 0.80), (0.60, 0.46), (0.40, 0.46)]); // body
    p.rect(0.36, 0.42, 0.64, 0.47); // collar
    p.rect(0.38, 0.33, 0.62, 0.42); // crown band
    // Cross.
    p.rect(0.465, 0.15, 0.535, 0.33); // vertical
    p.rect(0.42, 0.21, 0.58, 0.27); // horizontal
}
