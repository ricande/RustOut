# Architecture

RustOut is a single binary crate (`src/main.rs`) built on
[Macroquad](https://macroquad.rs). This document explains how the code is
organised, how a frame flows through it and how to extend it. For the rules of
the game itself, see [game_rules.md](game_rules.md).

## Design principles

1. **Logic does not touch the window.** Rules, physics, the editor, the menus'
   behaviour, level files, translations and video bookkeeping are plain Rust
   that work on abstract input. They are tested with `cargo test`, without a
   window, a GPU or a sound card.
2. **One coordinate system.** Everything — physics, block placement, buttons,
   mouse input, drawing — uses the logical 540 × 960 playfield. Only `View`
   (in `render.rs`) knows about the window.
3. **Fixed time step.** Physics advance in steps of 1/120 s, so behaviour does
   not depend on the frame rate.
4. **Events, not callbacks.** The game reports what happened (`game::Event`);
   sound, particles and the video recorder react to the same list.
5. **No asset files.** Graphics are drawn with shapes and text, and sounds are
   synthesized at startup.

## Modules

```
                 ┌────────────┐
   window ──────►│  main.rs   │  game loop, reads input, owns everything below
                 └─────┬──────┘
        Input (field   │                     Events
        coordinates)   ▼                       │
                 ┌────────────┐   ┌────────────┴───────────────┐
                 │   app.rs   │   │ audio.rs   render.rs (Effects)  capture.rs │
                 └─┬───┬────┬─┘   └────────────────────────────┘
                   │   │    │
          game.rs ◄┘   │    └► video dialog state (in app.rs)
                       ▼
                  editor.rs ──► store.rs (files) ──► levels.rs (Layout, Grid)
```

| Module        | Window-free | Responsibility |
| ------------- | :---------: | -------------- |
| `main.rs`     |             | Window setup, the frame loop, turning Macroquad input into `ui::Input`. |
| `app.rs`      | ✓ | Screens (menu, custom levels, game, editor, test play) and the transitions between them; the "save video" dialog; the language switch. |
| `game.rs`     | ✓ | Rules and physics: paddle, balls, bricks, bonuses, lives, levels, score. Emits `Event`s. |
| `levels.rs`   | ✓ | Level data: `Layout` (name + cells), `Grid` (columns × rows over the build area), `Block`, `Bonus`, the built-in levels. |
| `store.rs`    | ✓ | The JSON level format, validation, the level folder and the data folder. |
| `editor.rs`   | ✓ | The level editor's logic: brushes, painting, grid resizing, dialogs, save/open, unsaved-change guards. |
| `ui.rs`       | ✓ | `Input`, `Key`, buttons and the scrollable list widget, all in field coordinates. |
| `i18n.rs`     | ✓ | The translation catalog and the `t!` macro. |
| `settings.rs` | ✓ | `settings.json` (the chosen language). |
| `video.rs`    | ✓¹ | Recording via an `ffmpeg` process, the frame clock, sound mixing, saving, default folder and file names. |
| `screen.rs`   | ✓ | Picks the start window size from the screen's work area. |
| `script.rs`   | ✓ | Script mode: turns a text script into `ui::Input` per frame, for repeatable UI checks with screenshots. |
| `render.rs`   |   | `View` (field ↔ window), drawing of the game, particles and other effects. |
| `screens.rs`  |   | Drawing of the menu, level list, editor and the video UI. |
| `audio.rs`    |   | The synthesizer (window-free) and playback through Macroquad. |
| `capture.rs`  |   | Draws video frames off screen and drives `video::Recorder` for each round. |

¹ needs `ffmpeg` at run time; its tests skip themselves when ffmpeg is missing.

## The frame loop

`main.rs` runs this once per displayed frame:

1. **View.** `View::fit(screen_width, screen_height)` computes the largest 9:16
   area that fits the window (letterboxing).
2. **Input.** `read_input` collects key presses, typed characters, mouse
   buttons and the wheel. The mouse position is mapped to field coordinates;
   outside the view it is `None`, so clicks in the side bars do nothing.
3. **Handle.** `App::handle(&input)` runs the current screen's logic (menu
   choices, editor tools, launching the ball, dialogs …).
4. **Simulate.** The frame time (capped at 0.25 s) is added to an accumulator,
   and `App::update(1/120 s, &controls)` runs as many fixed steps as fit.
   `Controls` hold the paddle input: keyboard direction or a mouse target x.
5. **Events.** `Game::take_events()` returns what happened. Each event plays a
   sound (`audio::Sounds`) and may start an effect (`render::Effects`).
6. **Capture.** `Capture::frame` decides, via `video::plan_frame`, whether to
   start, record or finish the round's video, and draws a 1080 × 1920 frame
   off screen when one is due.
7. **Draw.** The current screen is drawn (`render::draw` or a function in
   `screens.rs`), then the video UI on top.
8. **Saves.** `Capture::poll_saves` starts requested video saves and reports
   finished ones back to `App`.

## Coordinates

- The field is 540 × 960 logical pixels; y grows downwards.
- `View` holds a scale and an offset. `View::to_field` converts a window
  position to the field, `View::contains` tells whether it is inside the view.
- Drawing helpers on `View` (`rect`, `circle`, `text`, …) take field
  coordinates and scale them. Text is shrunk if it would not fit the field.
- The video uses the same drawing code with `View::fit(1080, 1920)`, i.e. a
  scale of exactly 2.
- Level cells come from `Grid::cell_rect`, computed from the build area
  (`levels::BUILD_AREA`, x 0–540, y 130–706) so that the grid always fills it
  exactly. The same rectangle is used for drawing, mouse lookup
  (`Grid::cell_at`) and collisions.

## Game state

`Game` owns the paddle, a list of balls, the bricks, falling bonus capsules
(`drops`), score, lives, the speed and width steps and the bottom wall. Its
`State` is `Serve`, `Playing`, `Paused`, `GameOver` or `Victory`. Commands
(`Launch`, `TogglePause`, `Restart`) come from `App`; physics run in `update`.

Collisions are circle-vs-rectangle with an axis-aligned normal
(`circle_rect_contact`). Each step resolves at most the deepest brick contact,
pushes the ball out and reflects it only if it moves into the surface, so a
ball can never get stuck inside a block. `clamp_angle` keeps every ball at
least 18° from horizontal.

## Levels and files

- `Layout` is a name, a `Grid` and a row-major list of `Cell`s (a `Block` and
  an optional `Bonus`).
- Text rows (`.123#`) are the shared representation: built-in levels are
  written as rows in `levels.rs`, and level files store the same rows.
- `store.rs` reads and writes **format 3** JSON (`format`, `name`, `grid`,
  `rows`, `bonuses`). Formats 1 and 2 are still read; missing fields get the
  old defaults (no bonuses, a 12 × 24 grid). Saving is atomic (temp file +
  rename). Errors are translated, player-readable strings.
- Built-in levels are never files, so they cannot be overwritten; their names
  are reserved in every language.

## Screens and dialogs

`App.screen` is one of `Menu`, `Select`, `Game`, `Editor`, `TestPlay`.
`App::handle` dispatches to the screen; modal dialogs (the editor's `Modal`, the
video `SaveDialog`, the video message) take the input first while they are
open. Button positions are defined once, next to the logic, as functions that
return `ui::Button`s (for example `app::menu_buttons`, `editor::toolbar`); the
drawing code calls the same functions, so what is drawn is what is clicked.

Test play gives `Game` a clone of the editor's layout; the editor itself is left
untouched, so returning keeps unsaved changes.

## Video pipeline

1. `App.round` increases whenever a new round starts. `Capture` then drops an
   unsaved recording and *arms* a new one.
2. `video::plan_frame` starts the recording on the round's first `Launch` event,
   captures one frame per displayed frame (skipping pauses) and finishes on the
   frame the round ends, drawing that last frame without overlays.
3. `video::FrameClock` turns game time into an exact 60 fps frame count; a
   picture is repeated when the display is slower than 60 Hz and frames are
   skipped when it is faster.
4. Frames are read back from a render target and sent, through a small bounded
   queue and a writer thread, to `ffmpeg` (raw RGBA → H.264) writing a temporary
   file. `capture.rs` detects at startup whether the GPU returns rows bottom-up
   and asks ffmpeg to flip if so.
5. Every sound event is stored with its time. When the player saves, a thread
   mixes the sound track (`video::mix`), writes it as WAV and lets ffmpeg combine
   it with the video (AAC stereo) into the chosen file, via a `.partial.mp4`
   and a rename.

## Translations

- `i18n.rs` holds `CATALOG`: `(key, English, Swedish)` for every text.
- `t!("key")` returns the text in the current language; `t!("key", name = v)`
  fills `{name}` placeholders.
- The language is thread-local. `main.rs` sets it from `settings.json` (or
  `RUSTOUT_LANG`); the video save thread copies it when it starts.
- Tests check that every `t!` key used in the source exists, that keys are
  unique and that both languages have the same placeholders.

## Persistence

| What            | Where                                   | Module        |
| --------------- | --------------------------------------- | ------------- |
| Custom levels   | `<data folder>/levels/*.json`           | `store.rs`    |
| Language        | `<data folder>/settings.json`           | `settings.rs` |
| Videos          | chosen by the player (default `~/Videos/RustOut`) | `video.rs` |
| Recording temp  | the system temp folder (`rustout-rec-*`) | `video.rs`   |

The data folder is `~/.local/share/rustout` on Linux (see the README for other
systems). `RUSTOUT_LEVEL_DIR`, `RUSTOUT_VIDEO_DIR` and `RUSTOUT_LANG` override
the defaults.

## Testing

- `cargo test` runs about 90 tests without a window: physics (including long
  simulated games), level parsing and grids, files and validation, the editor,
  screen transitions, dialogs, translations, settings and the real video
  pipeline (when ffmpeg is installed).
- Tests that use files create their own folders under the system temp folder.
- Tests run on many threads; since the language is thread-local, a test that
  needs Swedish sets it on its own thread.
- Drawing code is not unit tested. It is checked by running the real binary
  in **script mode** (`RUSTOUT_SCRIPT`, see `src/script.rs`): a text script
  replaces mouse and keyboard, the frame time is fixed so runs repeat exactly,
  and `shot` steps save screenshots. `scripts/ui_tour.txt` visits every screen.

## How to …

**Add a text.** Add `("area.name", "English", "Svenska")` to `CATALOG` in
`i18n.rs` and use `t!("area.name")`. Use named placeholders (`{count}`) in both
languages. `cargo test` fails if the key is missing.

**Add a bonus.** Add a variant to `levels::Bonus` (with `ALL`, `key`, `name`,
`label`), its catalog entries, its effect in `Game::apply_bonus`, a colour in
`render::bonus_color` and tests in `game.rs`. The editor palette and the file
format pick it up from `Bonus::ALL` and `Bonus::key`. Document it in
`game_rules.md`.

**Add a built-in level.** Add an entry to `levels::BUILTIN` (a catalog key for
the name, the text rows, the bonuses) and the name to the catalog. The test
`builtin_levels_are_valid_and_inside_field` checks it.

**Change the level file format.** Bump `store::FORMAT`, keep reading the older
formats with sensible defaults, add a test that an old file still loads, and
describe the change in the README and the changelog.

**Add a screen.** Add a `Screen` variant, handle its input in `App::handle`,
draw it from `main.rs` (usually a new function in `screens.rs`) and define its
buttons as a function that both sides use.
