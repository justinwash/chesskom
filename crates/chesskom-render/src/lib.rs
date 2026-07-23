//! `chesskom-render`: dependency-free grayscale rendering of a chess position.
//!
//! The output is a plain 8-bit grayscale [`Canvas`]. On the desktop we encode it
//! to PNG for previews; on the Kobo the same canvas is blitted to the e-ink
//! framebuffer. Keeping the renderer output-agnostic means the board looks
//! identical in both places.

pub mod board;
pub mod canvas;
pub mod font;
pub mod piece_raster;
pub mod pieces;
pub mod png;

pub use board::{render, PieceStyle, RenderOptions};
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
    fn classic_piece_asset_is_present_and_well_formed() {
        assert!(piece_raster::available(), "embedded Cburnett asset must be valid");
    }

    #[test]
    fn both_piece_styles_render() {
        for style in [PieceStyle::Classic, PieceStyle::Vector] {
            let mut opts = RenderOptions::clara_bw();
            opts.piece_style = style;
            let c = render(&Position::start(), &opts);
            let first = c.pixels[0];
            assert!(c.pixels.iter().any(|&p| p != first));
        }
    }

    #[test]
    fn over_the_board_flips_only_the_far_side() {
        // In OTB mode the far side's pieces are rotated, so the top of the board
        // differs from the non-OTB render while the bottom (near side) does not.
        let pos = Position::start();
        let base = render(&pos, &RenderOptions::clara_bw());
        let mut otb_opts = RenderOptions::clara_bw();
        otb_opts.over_the_board = true;
        let otb = render(&pos, &otb_opts);

        let row = |c: &Canvas, y: u32| -> Vec<u8> {
            let s = (y * c.width) as usize;
            c.pixels[s..s + c.width as usize].to_vec()
        };
        // A row through Black's back rank (near the top) should change...
        assert_ne!(row(&base, 190), row(&otb, 190));
        // ...while a row through White's back rank (near the bottom) should not.
        assert_eq!(row(&base, 1030), row(&otb, 1030));
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
