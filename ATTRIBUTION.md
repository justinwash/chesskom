# Attribution

## Chess piece artwork — Cburnett set

The classic chess piece graphics embedded in `crates/chesskom-render/assets/pieces.bin`
are the **Cburnett** chess pieces, created by **Colin M.L. Burnett**. They are the
widely used "Wikipedia" / Lichess-default set.

- **Author:** Colin M.L. Burnett (user "Cburnett" on Wikimedia Commons)
- **License:** triple-licensed under the GNU GPL (v2 or later), the GNU Free
  Documentation License, and the BSD 3-clause license. This project uses them
  under the **BSD 3-clause license**, which requires the following attribution:

  > Copyright (c) Colin M.L. Burnett
  >
  > Redistribution and use in source and binary forms, with or without
  > modification, are permitted provided that the conditions of the BSD
  > 3-clause license are met, including retention of this copyright notice
  > and attribution to the author.

The SVG source for these pieces was obtained via the
[`python-chess`](https://pypi.org/project/chess/) package, which embeds the
Cburnett artwork in its `chess.svg` module. Our asset is a rasterized (grayscale
+ alpha) repacking of that artwork; see `scripts/gen-pieces.py` to regenerate it.

`python-chess` itself is authored by Niklas Fiekas and licensed GPL-3.0; only the
piece artwork (Cburnett, as above) is redistributed here, not the library.
