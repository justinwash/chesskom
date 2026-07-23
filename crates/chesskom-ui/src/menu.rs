//! A simple vertical list menu: a title and a stack of tappable buttons.
//!
//! Like the board's `Layout`, one `MenuLayout` drives both drawing and
//! hit-testing so a button is always exactly where a tap expects it.

use chesskom_render::board::draw_text;
use chesskom_render::font;
use chesskom_render::{Canvas, Rect};

pub struct MenuLayout {
    pub width: u32,
    pub height: u32,
    pub title_scale: u32,
    pub item_scale: u32,
    pub items: Vec<Rect>,
}

impl MenuLayout {
    pub fn new(width: u32, height: u32, n: usize) -> MenuLayout {
        let bw = width * 3 / 4;
        let bh = width / 9;
        let gap = width / 30;
        let x = (width - bw) / 2;
        let n_u = n as u32;
        let block = n_u * bh + n_u.saturating_sub(1) * gap;
        // Center the block in the area below the title.
        let title_area = height / 4;
        let avail = height.saturating_sub(title_area);
        let start = title_area + avail.saturating_sub(block) / 2;

        let mut items = Vec::with_capacity(n);
        for i in 0..n_u {
            items.push(Rect {
                x,
                y: start + i * (bh + gap),
                w: bw,
                h: bh,
            });
        }
        MenuLayout {
            width,
            height,
            title_scale: (width / 90).max(3),
            item_scale: (width / 260).max(2),
            items,
        }
    }

    /// Which item (if any) a tap at (x, y) landed on.
    pub fn hit(&self, x: u32, y: u32) -> Option<usize> {
        self.items.iter().position(|r| r.contains(x, y))
    }

    /// Render the menu with `title` and one label per item.
    pub fn render(&self, title: &str, labels: &[String]) -> Canvas {
        let mut c = Canvas::new(self.width, self.height, 255);

        // Title, centered near the top.
        let tw = font::text_width(title, self.title_scale);
        let tx = (self.width.saturating_sub(tw)) / 2;
        let ty = self.height / 8;
        draw_text(&mut c, tx, ty, title, self.title_scale, 0);

        for (r, label) in self.items.iter().zip(labels) {
            // Button face + inset border.
            c.fill_rect(r.x as i32, r.y as i32, r.w as i32, r.h as i32, 244);
            outline(&mut c, r, 3, 0);
            let lw = font::text_width(label, self.item_scale);
            let lh = font::GLYPH_H * self.item_scale;
            let lx = r.x + (r.w.saturating_sub(lw)) / 2;
            let ly = r.y + (r.h.saturating_sub(lh)) / 2;
            draw_text(&mut c, lx, ly, label, self.item_scale, 0);
        }
        c
    }
}

fn outline(c: &mut Canvas, r: &Rect, thick: i32, v: u8) {
    let (x, y, w, h) = (r.x as i32, r.y as i32, r.w as i32, r.h as i32);
    c.fill_rect(x, y, w, thick, v);
    c.fill_rect(x, y + h - thick, w, thick, v);
    c.fill_rect(x, y, thick, h, v);
    c.fill_rect(x + w - thick, y, thick, h, v);
}
