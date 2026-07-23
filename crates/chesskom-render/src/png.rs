//! A tiny grayscale PNG encoder (no dependencies).
//!
//! Uses uncompressed ("stored") DEFLATE blocks wrapped in a zlib stream, which is
//! valid PNG and needs no compressor. Files are larger than a real encoder would
//! produce, but this is only used for desktop previews / optional board export —
//! the Kobo path writes raw pixels to the framebuffer and never touches PNG.

use crate::canvas::Canvas;

pub fn encode_grayscale(c: &Canvas) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);

    // IHDR: width, height, bit depth 8, color type 0 (grayscale).
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&c.width.to_be_bytes());
    ihdr.extend_from_slice(&c.height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
    write_chunk(&mut out, b"IHDR", &ihdr);

    // Raw image data: each scanline prefixed with filter byte 0 (none).
    let mut raw = Vec::with_capacity(((c.width + 1) * c.height) as usize);
    for y in 0..c.height {
        raw.push(0);
        let start = (y * c.width) as usize;
        raw.extend_from_slice(&c.pixels[start..start + c.width as usize]);
    }

    let idat = zlib_store(&raw);
    write_chunk(&mut out, b"IDAT", &idat);
    write_chunk(&mut out, b"IEND", &[]);
    out
}

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = Crc32::new();
    crc.update(kind);
    crc.update(data);
    out.extend_from_slice(&crc.finalize().to_be_bytes());
}

/// Wrap `data` in a zlib stream using only stored (uncompressed) DEFLATE blocks.
fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(0x78); // CMF: deflate, 32K window
    out.push(0x01); // FLG: no dict, fastest
    let mut i = 0;
    while i < data.len() || (data.is_empty() && i == 0) {
        let remaining = data.len() - i;
        let block = remaining.min(0xFFFF);
        let final_block = i + block >= data.len();
        out.push(if final_block { 1 } else { 0 }); // BFINAL, BTYPE=00
        out.extend_from_slice(&(block as u16).to_le_bytes());
        out.extend_from_slice(&(!(block as u16)).to_le_bytes());
        out.extend_from_slice(&data[i..i + block]);
        i += block;
        if final_block {
            break;
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

struct Crc32 {
    crc: u32,
}

impl Crc32 {
    fn new() -> Crc32 {
        Crc32 { crc: 0xFFFF_FFFF }
    }
    fn update(&mut self, data: &[u8]) {
        for &byte in data {
            self.crc ^= byte as u32;
            for _ in 0..8 {
                let mask = (self.crc & 1).wrapping_neg();
                self.crc = (self.crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
    }
    fn finalize(self) -> u32 {
        self.crc ^ 0xFFFF_FFFF
    }
}
