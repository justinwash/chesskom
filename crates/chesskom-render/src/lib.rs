//! `chesskom-render`: dependency-free grayscale rendering of a chess position.
//!
//! The output is a plain 8-bit grayscale [`Canvas`]. On the desktop we encode it
//! to PNG for previews; on the Kobo the same canvas is blitted to the e-ink
//! framebuffer. Keeping the renderer output-agnostic means the board looks
//! identical in both places.

pub mod board;
pub mod canvas;
pub mod font;
pub mod pieces;
pub mod png;

pub use board::{render, RenderOptions};
pub use canvas::Canvas;

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::Position;

    #[test]
    fn renders_start_position_without_panicking() {
        let opts = RenderOptions::clara_bw();
        let c = render(&Position::start(), &opts);
        assert_eq!(c.width, 1072);
        assert_eq!(c.height, 1448);
        // Sanity: the image is not a single flat color.
        let first = c.pixels[0];
        assert!(c.pixels.iter().any(|&p| p != first));
    }

    #[test]
    fn png_roundtrip_header_is_valid() {
        let c = Canvas::new(4, 4, 128);
        let bytes = png::encode_grayscale(&c);
        assert_eq!(&bytes[0..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        // IHDR length is 13 and immediately follows the signature.
        assert_eq!(&bytes[8..12], &[0, 0, 0, 13]);
        assert_eq!(&bytes[12..16], b"IHDR");
    }
}
