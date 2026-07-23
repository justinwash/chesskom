# chesskom

A chess client for Kobo e-readers (primary target: **Kobo Clara BW**), written in Rust.

The goal, in order of milestones:

1. **Local two-player pass-and-play with rewind** ✅
2. **Board on the Kobo e-ink screen** ✅ *(current — static render; touch input next)*
3. **chess.com viewer** — read-only browsing of your ongoing daily games
4. **Lichess play** — make and submit real moves over the internet

## Why this shape? (feasibility & the ban question)

Some research went into scoping this. The short version:

- **The Kobo Clara BW is very moddable.** It's a 1 GHz ARM Linux device with a
  mature hacking ecosystem (KOReader, NickelMenu, KFMon, and the `koxtoolchain`
  cross-compiler). A native Rust app is well within what this hardware supports.

- **chess.com has no public "make a move" API.** Their
  [Published-Data API](https://www.chess.com/announcements/view/published-data-api)
  is **read-only** — you can fetch profiles, game archives, and *ongoing daily
  games as FEN/PGN with whose-turn-it-is*, but you cannot submit moves. An
  "Interactive API" is perpetually "in development" behind a partner form.

- **Submitting moves to chess.com from a custom client would violate their
  [User Agreement](https://www.chess.com/legal/user-agreement)** (no automation
  / no reverse-engineering the service) and risk an account ban. So for
  chess.com we stay **read-only** — fully within the public API and the ToS.

- **Lichess is the clean way to actually play.** Its official
  [Board API](https://lichess.org/api) is *built for* third-party clients and
  physical boards, supports correspondence time controls, and explicitly permits
  streaming games and submitting moves. That's milestone 3.

- **Running the chess.com website in a browser on the Kobo** is *not* a ToS
  problem (it's just their real front-end), but it's a poor fit for e-ink: heavy
  SPA, slow refresh, and a big lift to get a modern browser engine running on
  the device. Parked as a future curiosity.

## Repository layout

```
crates/
  chess-core/       dependency-free rules, move generation, game state, rewind, FEN
  chesskom-render/  dependency-free grayscale board renderer (vector pieces, PNG export)
  chesskom-cli/     terminal front-end (dev harness for local play + rewind)
  chesskom-kobo/    Kobo e-ink device binary (framebuffer output via FBInk)
```

`chess-core` and `chesskom-render` have **no dependencies** and make no platform
assumptions, so they cross-compile to the Kobo untouched. Everything
device-specific (input, networking) lives outside them.

### `chess-core` highlights

- Full legal move generation: castling, en passant, promotion, pins, check.
- Correctness proven by **perft** against known node counts (startpos, Kiwipete,
  and CPW positions 3 & 4) — see `src/lib.rs` tests.
- Game state keeps a full position snapshot per ply, so **rewind** is a simple,
  bug-proof cursor into history.
- FEN parse/serialize (needed for the chess.com viewer milestone).
- End-state detection: checkmate, stalemate, 50-move, threefold, insufficient
  material.

## Build & run (desktop)

```sh
cargo test            # runs the perft + rules test suite
cargo run -p chesskom-cli
```

The CLI is a local two-player board. Commands:

| command        | effect                                             |
| -------------- | -------------------------------------------------- |
| `e2e4`         | play a move (coordinate form; `e7e8q` to promote)  |
| `moves`        | list all legal moves                               |
| `moves e2`     | list legal moves from a square                     |
| `back` / `b`   | rewind the view one half-move                      |
| `forward` / `f`| advance the view one half-move                     |
| `start` / `end`| jump the view to the initial / live position       |
| `flip`         | toggle auto-flip (board faces the side to move)    |
| `new`          | new game                                           |
| `quit` / `q`   | exit                                               |

## Rendering & the Kobo screen

The board is drawn by `chesskom-render` into a plain 8-bit grayscale image:
alternating shaded squares, coordinate labels, a header/footer text line, an
optional last-move highlight, and the pieces. The same image feeds both the
desktop PNG preview and the device.

Two piece styles are available (`RenderOptions::piece_style`):

- **Classic** (default) — the **Cburnett** set (the Wikipedia / Lichess-default
  artwork), rasterized to a compact embedded grayscale+alpha asset and composited
  over the squares. See [ATTRIBUTION.md](ATTRIBUTION.md) for licensing; regenerate
  the asset with `python3 scripts/gen-pieces.py`.
- **Vector** — dependency-free anti-aliased silhouettes drawn from primitives, no
  asset required. A fallback / lightweight option.

Preview a position as a PNG on your desktop:

```sh
cargo run -p chesskom-render --example preview -- board.png                   # start position
cargo run -p chesskom-render --example preview -- board.png "<FEN>"           # any position
cargo run -p chesskom-render --example preview -- board.png "<FEN>" vector    # vector pieces
```

### Building for the Kobo

The device is `armv7` Linux with an older glibc, so we target **static musl** —
one self-contained ~370 KB binary that runs regardless of the device libc.

```sh
rustup target add armv7-unknown-linux-musleabihf
./scripts/build-kobo.sh          # -> dist/chesskom-kobo (static ARM binary)
```

Cross-linking uses clang + LLD (no target toolchain package needed); see
[`.cargo/config.toml`](.cargo/config.toml) for the GNU-toolchain alternative.

### Running on the device

`chesskom-kobo` draws the board to the e-ink panel via
[**FBInk**](https://github.com/NiLuJe/FBInk), the maintained tool that handles
each Kobo model's framebuffer format and refresh waveform. Install FBInk on the
Kobo, then:

```sh
chesskom-kobo                 # draw the starting position
chesskom-kobo --flip          # Black at the bottom
chesskom-kobo --fen "<FEN>"   # draw a specific position
chesskom-kobo --out board.png # desktop test: write a PNG instead of drawing
```

**Install path:** copy `dist/chesskom-kobo` to `/mnt/onboard/.adds/chesskom/` on
the device (USB mass storage), `chmod +x` it, and either run it from a terminal
or add a [NickelMenu](https://github.com/pgaskin/NickelMenu) entry to launch it
from the stock reader UI.

> **Why FBInk rather than writing `/dev/fb0` directly?** Kobo models span two SoC
> families (i.MX6 and Allwinner/sunxi) with different e-ink refresh interfaces.
> FBInk already abstracts them, so it's the reliable way to get correct pixels
> and a clean refresh without model-specific guesswork. A native, dependency-free
> direct-framebuffer backend is a planned follow-up that needs on-device tuning.

### Not built yet

Touch input (`/dev/input/*`) and the interactive on-device loop are the next
step — this milestone gets a correct board onto the panel. After that comes the
chess.com viewer, then Lichess play.

## License

MIT — see [LICENSE](LICENSE).
