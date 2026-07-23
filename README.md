# chesskom

A chess client for Kobo e-readers (primary target: **Kobo Clara BW**), written in Rust.

The goal, in order of milestones:

1. **Local two-player pass-and-play with rewind** ✅
2. **Board on the Kobo e-ink screen** ✅
3. **Interactive on-device play** ✅ *(current — tap to move + rewind controls;
   touch mapping pending on-device calibration)*
4. **chess.com viewer** — read-only browsing of your ongoing daily games
5. **Lichess play** — make and submit real moves over the internet

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

> **Picking this up as a developer?** See [HANDOFF.md](HANDOFF.md) for the status
> snapshot, dev setup, build/install steps, architecture notes, and what's next.

## Repository layout

```
crates/
  chess-core/       dependency-free rules, move generation, game state, rewind, FEN
  chesskom-render/  dependency-free grayscale renderer (pieces, overlays, layout/hit-test)
  chesskom-ui/      interactive controller: tap-to-move state machine + rewind controls
  chesskom-cli/     terminal front-end (dev harness for local play + rewind)
  chesskom-kobo/    Kobo e-ink device binary (touch input + FBInk output)
```

`chesskom-ui` holds the platform-agnostic app logic (selection, moves, navigation)
and is fully unit-tested; the Kobo binary is a thin loop feeding it touch taps and
painting its output.

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

**Over-the-board mode** (`RenderOptions::over_the_board`): rotate the far side's
pieces 180° so two players facing each other across a flat-lying Kobo each read
their own pieces upright — natural for local pass-and-play.

> **Later:** now that pieces can rotate, a "face the current player" variant
> becomes easy — flip the whole board 180° between turns so the side on move
> always sees it from their perspective. In that mode the orientation itself
> tells you whose turn it is (upright pieces = your move), so the explicit
> "White/Black to move" text can be dropped. Deferred until the interactive play
> loop exists (it needs turn state to drive it).

Preview a position as a PNG on your desktop:

```sh
cargo run -p chesskom-render --example preview -- board.png                   # start position
cargo run -p chesskom-render --example preview -- board.png "<FEN>"           # any position
cargo run -p chesskom-render --example preview -- board.png "<FEN>" vector    # vector pieces
cargo run -p chesskom-render --example preview -- board.png "<FEN>" otb       # over-the-board
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

`chesskom-kobo` draws to the e-ink panel via
[**FBInk**](https://github.com/NiLuJe/FBInk) (the maintained tool that handles
each Kobo model's framebuffer format and refresh waveform) and reads taps from the
touchscreen. Install FBInk on the Kobo, then:

```sh
chesskom-kobo                 # interactive local two-player game (default)
chesskom-kobo --calibrate     # print raw/mapped touch coords to tune the mapping
chesskom-kobo --fen "<FEN>"   # static render of a position (no touch)
chesskom-kobo --out board.png # desktop test: write a PNG instead of drawing
```

**Menus:** the app boots to a title screen — `LOCAL / CHESS.COM / LICHESS / QUIT`
(the last two are placeholders until milestones 4–5). `LOCAL` opens `NEW GAME`
and `GAME HISTORY`.

**Playing:** tap a piece to select it (legal destinations light up — dots for
quiet moves, rings for captures), tap a destination to move. The bottom control
bar has `FIRST / PREV / NEXT / LIVE` (rewind), `FLIP`, `NEW`, and `MENU` (back to
the menu). Promotion currently auto-queens (an under-promotion picker is a TODO).

**Game history:** the last 10 games (any with moves) are kept and can be replayed
from `LOCAL → GAME HISTORY` — pick one and step through it with the rewind
controls, or tap the board to resume playing from where it left off. On the
device, history is persisted to a small text file (path from `CHESSKOM_HISTORY`,
default `./chesskom-history.txt`); the format is just the starting FEN and move
list per game, so it's tiny and human-readable.

**Touch calibration (one-time, per model):** the raw→screen coordinate mapping
varies by Kobo model, so it's driven by `CHESSKOM_TOUCH_*` environment variables
rather than hard-coded. Run `chesskom-kobo --calibrate`, tap the four corners, and
adjust `CHESSKOM_TOUCH_SWAP`, `CHESSKOM_TOUCH_INVX`, `CHESSKOM_TOUCH_INVY`, and
`CHESSKOM_TOUCH_MAX_X/Y` until the mapped coordinates match where you tapped. See
`crates/chesskom-kobo/src/touch.rs` for the full list. This is the one piece that
needs the physical device to finalize — everything above it is unit-tested and
renders identically on the desktop.

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
