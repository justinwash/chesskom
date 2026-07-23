//! An 8-bit grayscale drawing surface with simple filled primitives.
//!
//! e-ink is grayscale, so we work in a single 8-bit channel throughout (0 = black,
//! 255 = white). Pieces are drawn as silhouette *masks* at a supersampled scale and
//! downsampled for anti-aliasing (see `pieces` / `board`), which keeps the primitive
//! set here tiny: we only need hard-edged fills into a boolean mask plus solid-gray
//! fills into the final canvas.

/// A grayscale image, row-major, one byte per pixel.
#[derive(Clone)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(width: u32, height: u32, fill: u8) -> Canvas {
        Canvas {
            width,
            height,
            pixels: vec![fill; (width * height) as usize],
        }
    }

    #[inline]
    pub fn set(&mut self, x: u32, y: u32, v: u8) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = v;
        }
    }

    #[inline]
    pub fn get(&self, x: u32, y: u32) -> u8 {
        self.pixels[(y * self.width + x) as usize]
    }

    /// Alpha-blend value `v` onto pixel (x,y) with coverage in 0.0..=1.0.
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, v: u8, coverage: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = (y as u32 * self.width + x as u32) as usize;
        let bg = self.pixels[idx] as f32;
        let out = bg * (1.0 - coverage) + v as f32 * coverage;
        self.pixels[idx] = out.round() as u8;
    }

    /// Filled anti-aliased disc centered at (cx, cy).
    pub fn disc(&mut self, cx: f32, cy: f32, r: f32, v: u8) {
        let x0 = (cx - r - 1.0).floor() as i32;
        let x1 = (cx + r + 1.0).ceil() as i32;
        let y0 = (cy - r - 1.0).floor() as i32;
        let y1 = (cy + r + 1.0).ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                let cov = (r + 0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, v, cov);
                }
            }
        }
    }

    /// Anti-aliased ring (annulus) centered at (cx, cy) between radii r-thick and r.
    pub fn ring(&mut self, cx: f32, cy: f32, r: f32, thick: f32, v: u8) {
        let inner = r - thick;
        let x0 = (cx - r - 1.0).floor() as i32;
        let x1 = (cx + r + 1.0).ceil() as i32;
        let y0 = (cy - r - 1.0).floor() as i32;
        let y1 = (cy + r + 1.0).ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                // Coverage of the band [inner, r].
                let outer_cov = (r + 0.5 - d).clamp(0.0, 1.0);
                let inner_cov = (d - (inner - 0.5)).clamp(0.0, 1.0);
                let cov = outer_cov.min(inner_cov);
                if cov > 0.0 {
                    self.blend(x, y, v, cov);
                }
            }
        }
    }

    /// Fill an axis-aligned rectangle (clipped to bounds) with a solid value.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, v: u8) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.width as i32);
        let y1 = (y + h).min(self.height as i32);
        for yy in y0..y1 {
            for xx in x0..x1 {
                self.pixels[(yy as u32 * self.width + xx as u32) as usize] = v;
            }
        }
    }
}

/// A boolean coverage mask used to build piece silhouettes at supersampled scale.
pub struct Mask {
    pub w: u32,
    pub h: u32,
    pub bits: Vec<bool>,
}

impl Mask {
    pub fn new(w: u32, h: u32) -> Mask {
        Mask {
            w,
            h,
            bits: vec![false; (w * h) as usize],
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            false
        } else {
            self.bits[(y as u32 * self.w + x as u32) as usize]
        }
    }

    #[inline]
    fn set(&mut self, x: i32, y: i32) {
        if x >= 0 && y >= 0 && x < self.w as i32 && y < self.h as i32 {
            self.bits[(y as u32 * self.w + x as u32) as usize] = true;
        }
    }

    pub fn fill_rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        for y in y0.floor() as i32..=y1.ceil() as i32 {
            for x in x0.floor() as i32..=x1.ceil() as i32 {
                let cx = x as f32 + 0.5;
                let cy = y as f32 + 0.5;
                if cx >= x0 && cx <= x1 && cy >= y0 && cy <= y1 {
                    self.set(x, y);
                }
            }
        }
    }

    /// Filled circle centered at (cx, cy).
    pub fn fill_circle(&mut self, cx: f32, cy: f32, r: f32) {
        let r2 = r * r;
        for y in (cy - r).floor() as i32..=(cy + r).ceil() as i32 {
            for x in (cx - r).floor() as i32..=(cx + r).ceil() as i32 {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                if dx * dx + dy * dy <= r2 {
                    self.set(x, y);
                }
            }
        }
    }

    /// Filled ellipse (axis-aligned) centered at (cx, cy) with radii rx, ry.
    pub fn fill_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32) {
        for y in (cy - ry).floor() as i32..=(cy + ry).ceil() as i32 {
            for x in (cx - rx).floor() as i32..=(cx + rx).ceil() as i32 {
                let dx = (x as f32 + 0.5 - cx) / rx;
                let dy = (y as f32 + 0.5 - cy) / ry;
                if dx * dx + dy * dy <= 1.0 {
                    self.set(x, y);
                }
            }
        }
    }

    /// Filled convex/concave polygon via even-odd scanline rule.
    pub fn fill_polygon(&mut self, pts: &[(f32, f32)]) {
        if pts.len() < 3 {
            return;
        }
        let (mut min_y, mut max_y) = (f32::MAX, f32::MIN);
        for &(_, y) in pts {
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let y0 = min_y.floor().max(0.0) as i32;
        let y1 = (max_y.ceil() as i32).min(self.h as i32 - 1);
        for y in y0..=y1 {
            let yc = y as f32 + 0.5;
            let mut xs: Vec<f32> = Vec::new();
            for i in 0..pts.len() {
                let (ax, ay) = pts[i];
                let (bx, by) = pts[(i + 1) % pts.len()];
                if (ay <= yc && by > yc) || (by <= yc && ay > yc) {
                    let t = (yc - ay) / (by - ay);
                    xs.push(ax + t * (bx - ax));
                }
            }
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut i = 0;
            while i + 1 < xs.len() {
                let sx = xs[i].ceil() as i32;
                let ex = xs[i + 1].floor() as i32;
                for x in sx..=ex {
                    self.set(x, y);
                }
                i += 2;
            }
        }
    }

    /// Erode the mask by `r` pixels (Chebyshev). A pixel survives only if every
    /// pixel within the square neighbourhood of radius `r` is also set. Used to
    /// carve the interior out of a silhouette so we can draw a clean outline.
    pub fn erode(&self, r: i32) -> Mask {
        let mut out = Mask::new(self.w, self.h);
        for y in 0..self.h as i32 {
            for x in 0..self.w as i32 {
                if !self.get(x, y) {
                    continue;
                }
                let mut keep = true;
                'scan: for dy in -r..=r {
                    for dx in -r..=r {
                        if !self.get(x + dx, y + dy) {
                            keep = false;
                            break 'scan;
                        }
                    }
                }
                if keep {
                    out.set(x, y);
                }
            }
        }
        out
    }
}
