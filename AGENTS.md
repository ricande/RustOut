# AGENTS.md

Guidance for AI coding agents (and humans) working on RustOut. Read this and
[docs/architecture.md](docs/architecture.md) before changing code; the game's
rules are in [docs/game_rules.md](docs/game_rules.md).

## Project in one paragraph

RustOut is a small Breakout/Arkanoid-style game in Rust with Macroquad: a
portrait 540 × 960 playfield scaled to the window, three built-in levels, a
level editor with JSON level files, bonuses, stones, MP4 recording of rounds
via ffmpeg, and English/Swedish translations. It is a single binary crate with
no asset files.

## Commands

Run all of these before you consider a change done:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

Run the game with `cargo run` (or `cargo run --release`). If `cargo` is not on
`PATH`, it is in `~/.cargo/bin`.

Useful environment variables while testing by hand, so you do not touch the
user's real data:

| Variable            | Effect                                   |
| ------------------- | ---------------------------------------- |
| `RUSTOUT_LEVEL_DIR` | folder for custom level files            |
| `RUSTOUT_VIDEO_DIR` | default folder for saved videos          |
| `RUSTOUT_LANG`      | `en` or `sv`, overrides the saved choice |
| `RUSTOUT_SCRIPT`    | run a UI script instead of real input (see below) |
| `RUSTOUT_SHOTS`     | folder for the script's screenshots (default `shots`) |
| `RUSTOUT_WINDOW`    | force a window size, e.g. `1000x640`     |

## Rules

1. **Keep logic window-free.** Game rules, the editor, screens' behaviour,
   files and translations must not call Macroquad's window, graphics, input or
   audio APIs. They take `ui::Input` / `game::Controls` and are tested headlessly.
   Only `main.rs`, `render.rs`, `screens/`, `capture.rs` and the playback
   part of `audio.rs` talk to Macroquad.
2. **Use field coordinates.** Everything is in the logical 540 × 960 field.
   Never use window pixels outside `render::View`. Define button rectangles once,
   in a function the input code and the drawing code both use.
3. **Every player-visible text goes through `t!`.** Add the key to `CATALOG`
   in `src/i18n.rs` with both English and Swedish, using the same named
   placeholders in both. Never put literal UI text in other modules. Code,
   comments, docs and commit messages are in English.
4. **Keep level files compatible.** Old files must keep loading. If the format
   changes, bump `FORMAT` in `store.rs`, default the new fields for old files,
   add a test with an old file, and document it in the README and CHANGELOG.
   Built-in levels must never be overwritable.
5. **Physics must stay stable.** Keep the fixed time step, the sub-stepping
   against tunnelling and the minimum bounce angle. Changes to collisions need
   tests (see the tunnelling and long-simulation tests in `game.rs`).
6. **Test what you add.** New rules, editor behaviour, file handling and screen
   transitions get unit tests in the same file. Tests that use files must use
   their own temp folder (`store::tests::temp_dir`), never the user's folders.
   Tests run in parallel threads; the language is thread-local, so set it only
   on a thread you spawn yourself.
7. **Match the surrounding code.** Small modules, short doc comments that say
   why, no new frameworks or heavy dependencies without a good reason.
8. **Update the docs with the code.** Rules and numbers → `docs/game_rules.md`;
   structure and data flow → `docs/architecture.md`; user-facing behaviour →
   `README.md`; every notable change → `CHANGELOG.md` under *Unreleased*.

## Verifying UI changes

Drawing code has no unit tests. When a change affects what is shown, run the
real game and look at it. Use the **script mode** rather than the real mouse
and keyboard (it does not disturb the user's desktop and is repeatable):

```bash
RUSTOUT_SCRIPT=scripts/ui_tour.txt RUSTOUT_SHOTS=/tmp/shots \
RUSTOUT_LEVEL_DIR=/tmp/levels RUSTOUT_VIDEO_DIR=/tmp/videos \
cargo run --release
```

- `scripts/ui_tour.txt` visits every screen and saves numbered screenshots.
  The command language is documented at the top of `src/script.rs`; write a
  new script for a specific check.
- A scripted run uses a fixed frame time, so the same script on the same code
  gives the same pictures. For a refactoring, take screenshots before and
  after and compare them (e.g. `compare -metric AE a.png b.png null:`).
  Expect differences only where the current time is shown (the video dialog)
  and, in non-integer window scales, at text edges.
- Check both languages (`RUSTOUT_LANG=en` / `sv`) and a wide window
  (`RUSTOUT_WINDOW=1000x640`), since the view is letterboxed.

Say clearly what you verified by running the game and what only by tests.

## Working with the maintainer

- The maintainer usually writes in Swedish; reply in the language they use.
- Ask before anything hard to undo or outward-facing: pushing, publishing,
  deleting files, or changing things outside the repository.
- Do not commit build output (`target/`), recordings or personal level files.
