# Handoff & development guide

Everything you need to pick `chesskom` up on your own machine and keep going.

## Status snapshot

| Milestone | State |
| --- | --- |
| 1. Local two-player + rewind (chess engine, rules) | ✅ done |
| 2. Board on the Kobo e-ink screen (renderer, classic pieces, OTB) | ✅ done |
| 3. Interactive on-device play (touch, control bar) | ✅ done* |
| 3b. Title screen / menu + replayable game history | ✅ done |
| 4. chess.com read-only viewer | ⬜ next |
| 5. Lichess play (make/submit moves) | ⬜ later |

\* Everything **except the touchscreen coordinate mapping** is done and tested.
That one layer can only be finalized on the physical device — see
[Touch calibration](#touch-calibration-the-one-on-device-step) below.

Tests: **53** across the workspace, all passing. `cargo clippy` is clean.

## Prerequisites

- **Rust** stable (developed on 1.94; anything recent works). `rustup` recommended.
- For the Kobo cross-build: the `armv7-unknown-linux-musleabihf` target plus a
  cross-linker (`clang` + `lld`, already wired in `.cargo/config.toml`; a GNU
  `arm-linux-gnueabihf-gcc` is the documented alternative).
- Only to *regenerate* piece art (you won't normally need this): Python with
  `pip install --only-binary=:all: chess cairosvg pillow`.

## Everyday commands

```sh
cargo test                      # full suite (engine perft, rules, UI, render)
cargo clippy --all-targets      # lints (kept clean)

# Play locally in the terminal (dev harness for the engine):
cargo run -p chesskom-cli

# Preview the e-ink board as a PNG (what the device draws):
cargo run -p chesskom-render --example preview -- board.png                 # start pos
cargo run -p chesskom-render --example preview -- board.png "<FEN>"         # any position
cargo run -p chesskom-render --example preview -- board.png "<FEN>" vector  # vector pieces
cargo run -p chesskom-render --example preview -- board.png "<FEN>" otb     # over-the-board

# Preview the interactive UI (selection/targets/control bar) and menus:
cargo run -p chesskom-ui --example frame -- frame.png
cargo run -p chesskom-ui --example menus -- .        # writes menu_main.png, menu_history.png
```

## Building & installing on the Kobo

```sh
rustup target add armv7-unknown-linux-musleabihf
./scripts/build-kobo.sh          # -> dist/chesskom-kobo (static ARM, ~800 KB)
```

Then on the device (needs [FBInk](https://github.com/NiLuJe/FBInk) installed):

1. Copy `dist/chesskom-kobo` to e.g. `/mnt/onboard/.adds/chesskom/` over USB.
2. `chmod +x` it.
3. Run it from a terminal, or add a
   [NickelMenu](https://github.com/pgaskin/NickelMenu) entry to launch from the
   stock reader UI.
4. Set `CHESSKOM_HISTORY=/mnt/onboard/.adds/chesskom/history.txt` (via the
   launcher/env) so saved games persist across runs.

### Touch calibration (the one on-device step)

The raw-touch → screen-pixel transform varies per Kobo model, so it's driven by
env vars instead of hard-coded values. Everything upstream of it (game logic,
rendering) is unit-tested and renders identically on desktop, so this should be a
quick dial-in, not a debugging slog:

```sh
chesskom-kobo --calibrate        # tap the four corners; compare printed coords
```

Adjust until the mapped coordinates match where you tapped:
`CHESSKOM_TOUCH_SWAP`, `CHESSKOM_TOUCH_INVX`, `CHESSKOM_TOUCH_INVY`,
`CHESSKOM_TOUCH_MAX_X`, `CHESSKOM_TOUCH_MAX_Y`, and `CHESSKOM_TOUCH_DEV` /
`CHESSKOM_INPUT_STRUCT` if the device node or event size differ. Full list and
rationale in `crates/chesskom-kobo/src/touch.rs`.

## Architecture (where things live)

```
crates/
  chess-core/       rules, legal move gen (perft-validated), Game + rewind, FEN. No deps.
  chesskom-render/  grayscale Canvas, classic + vector pieces, overlays, Layout/hit-test,
                    PNG encoder. No deps. Asset: assets/pieces.bin (Cburnett set).
  chesskom-ui/      App = screen state machine + tap-to-move controller + game History.
                    No I/O (HistoryStore trait); fully unit-tested.
  chesskom-cli/     terminal front-end (engine dev harness).
  chesskom-kobo/    device binary: touch input (evdev) + FBInk output + FileStore.
```

Key decisions worth knowing before you change things:

- **Rewind = full position snapshots per ply** (not an undo stack). Simple and
  bug-proof; memory is trivial for correspondence-length games.
- **One `Layout` drives both drawing and hit-testing** so buttons/squares can't
  drift from where taps land. Same idea for `MenuLayout`.
- **`chess-core` and `chesskom-render` have zero dependencies** and no platform
  assumptions, so they cross-compile to the Kobo untouched.
- **FBInk for output, not raw `/dev/fb0`**: Kobo models split across i.MX and
  Allwinner SoCs with different refresh interfaces; FBInk abstracts them. A native
  framebuffer backend is a possible future optimization.
- **History persistence is behind `HistoryStore`** so the app stays testable; the
  device supplies a file-backed store. Saved games are start-FEN + canonical
  coordinate moves (move flags are re-derived on replay).

## Open TODOs / deferred ideas

- **Under-promotion picker** — promotion currently auto-queens
  (`chesskom-ui/src/lib.rs`, `move_to`).
- **"Face the current player" mode** — auto-flip the whole board between turns;
  the per-piece rotation is already built (see the note on
  `RenderOptions::over_the_board`). In that mode the orientation replaces the
  "to move" text.
- **Native direct-framebuffer backend** (optional; FBInk works fine).

## Next up: milestone 4 — chess.com read-only viewer

The menu already has a `CHESS.COM` entry (currently "coming soon"), so this slots
in without restructuring. Plan:

- New crate (e.g. `chesskom-net`) with a small client for the **public
  Published-Data API** (`https://api.chess.com/pub/...`) — read-only, no auth
  needed, no ToS risk. Fetch the player's ongoing daily games, parse their
  FEN/PGN, and feed them into the existing board + rewind viewer.
- Wire the `CHESS.COM` menu entry to a games-list screen (reuse `MenuLayout`),
  then reuse the game/replay screen for viewing.
- **Read-only only** — the public API cannot submit moves, and doing so via a
  custom client violates chess.com's User Agreement. Real play comes in
  milestone 5 via Lichess's sanctioned Board API.

To start it, two things are needed:
1. Your **chess.com username** (the public API keys off it; no login).
2. Confirm the build/dev environment can reach `api.chess.com` over HTTPS.

See [ATTRIBUTION.md](ATTRIBUTION.md) for piece-art licensing and
[README.md](README.md) for the user-facing overview and the feasibility/ToS
findings.
