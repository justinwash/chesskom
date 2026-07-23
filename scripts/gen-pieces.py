#!/usr/bin/env python3
"""Rasterize the Cburnett chess pieces into a compact binary asset.

The classic piece artwork (by Colin M.L. Burnett — the Wikipedia / Lichess-default
set, triple-licensed BSD/GPL/GFDL) ships embedded as SVG inside the `python-chess`
package, so we can generate our asset without fetching any external host.

Output: crates/chesskom-render/assets/pieces.bin

Format (little-endian):
    magic    4 bytes  b"CKP1"
    count    u8       number of pieces (12)
    size     u16      square edge in pixels (both W and H)
  then `count` pieces, each `size*size` L (luminance) bytes followed by
  `size*size` A (alpha) bytes. Piece order is White P,N,B,R,Q,K then Black
  P,N,B,R,Q,K — i.e. index = color*6 + kind, White=0/Black=1, kind P..K = 0..5.

Regenerate with:  pip install --only-binary=:all: chess cairosvg pillow
                  python3 scripts/gen-pieces.py
"""
import io
import os
import struct
import sys

import cairosvg
import chess.svg
from PIL import Image

SIZE = 128          # embedded square edge
SUPER = 512         # rasterize large, then downsample for clean edges
# White then black, each P,N,B,R,Q,K. python-chess: uppercase=white, lowercase=black.
ORDER = ["P", "N", "B", "R", "Q", "K", "p", "n", "b", "r", "q", "k"]


def render_piece(symbol: str) -> Image.Image:
    g = chess.svg.PIECES[symbol]
    svg = (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 45" '
        f'width="{SUPER}" height="{SUPER}">{g}</svg>'
    )
    png = cairosvg.svg2png(bytestring=svg.encode(), output_width=SUPER, output_height=SUPER)
    im = Image.open(io.BytesIO(png)).convert("RGBA")
    return im.resize((SIZE, SIZE), Image.LANCZOS)


def main() -> None:
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    out_path = os.path.join(root, "crates", "chesskom-render", "assets", "pieces.bin")
    os.makedirs(os.path.dirname(out_path), exist_ok=True)

    blob = bytearray()
    blob += b"CKP1"
    blob += struct.pack("<B", len(ORDER))
    blob += struct.pack("<H", SIZE)

    for symbol in ORDER:
        im = render_piece(symbol)
        px = im.load()
        lum = bytearray(SIZE * SIZE)
        alpha = bytearray(SIZE * SIZE)
        for y in range(SIZE):
            for x in range(SIZE):
                r, g, b, a = px[x, y]
                lum[y * SIZE + x] = round(0.299 * r + 0.587 * g + 0.114 * b)
                alpha[y * SIZE + x] = a
        blob += lum
        blob += alpha

    with open(out_path, "wb") as f:
        f.write(blob)
    print(f"wrote {out_path} ({len(blob)} bytes, {len(ORDER)} pieces @ {SIZE}px)", file=sys.stderr)


if __name__ == "__main__":
    main()
