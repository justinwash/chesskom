# chesskom

A chess client for Kobo e-readers (primary target: **Kobo Clara BW**), written in Rust.

The goal, in order of milestones:

1. **Local two-player pass-and-play with rewind** ✅ *(current)*
2. **chess.com viewer** — read-only browsing of your ongoing daily games
3. **Lichess play** — make and submit real moves over the internet

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
  chess-core/     dependency-free rules, move generation, game state, rewind, FEN
  chesskom-cli/   terminal front-end (dev harness for local play + rewind)
```

`chess-core` has **no dependencies** and makes no platform assumptions, so it
cross-compiles to the Kobo untouched. Everything device-specific (UI, input,
networking) lives outside it.

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

## Kobo target (planned)

The device is `armv7` Linux. The intended cross-compile target is static musl to
sidestep the device's older glibc:

```sh
rustup target add armv7-unknown-linux-musleabihf
cargo build --release --target armv7-unknown-linux-musleabihf -p chesskom-cli
```

On-device UI will draw to the framebuffer (`/dev/fb0`) and read touch input
(`/dev/input/*`); a chessboard is a natural fit for e-ink (static, high-contrast,
no animation required). Install will be via NickelMenu/KFMon launcher entries.
This layer is not built yet — milestone 1 is the portable core plus a desktop
harness to validate the rules and rewind before touching hardware.

## License

MIT — see [LICENSE](LICENSE).
