# RustOut

A small Breakout/Arkanoid-inspired arcade game written in Rust with
[Macroquad](https://macroquad.rs). All graphics are drawn in code and all sound
effects are synthesized at startup – the project has no image or sound files.

- Portrait 9:16 playfield that scales to any window
- Three built-in levels, breakable blocks and indestructible stones
- Six bonuses: paddle +/−, speed +/−, multiball and a bottom wall
- A level editor with adjustable grid and readable JSON level files
- Save any round as a 1080 × 1920, 60 fps MP4 with sound
- English and Swedish

## Documentation

| Document | Contents |
| -------- | -------- |
| [docs/game_rules.md](docs/game_rules.md) | How the game plays: ball, paddle, scoring, lives, blocks, bonuses |
| [docs/architecture.md](docs/architecture.md) | Modules, the frame loop, coordinates, video pipeline, how to extend |
| [AGENTS.md](AGENTS.md) | Rules and commands for contributors and AI agents |
| [CHANGELOG.md](CHANGELOG.md) | What changed in each version |
| [LICENSE](LICENSE) | MIT |

## Language

The game is in English by default; Swedish is available. Switch with
**Language: English / Språk: Svenska** in the main menu. The choice is saved
in `settings.json` in the game's data folder (see [Where files are
stored](#where-files-are-stored)) and used the next time the game starts.
The environment variable `RUSTOUT_LANG=en` or `RUSTOUT_LANG=sv` overrides the
saved choice.

All texts live in one translation catalog (`src/i18n.rs`) with an English and
a Swedish version of each. Error messages, the built-in level names and number
formatting (decimal point or comma) follow the language too.

## Playfield

The playfield is portrait, 9:16, with a logical resolution of 540 × 960. The
whole view (menus included) is scaled to fit the window, centred and
surrounded by background – it is never cropped or stretched. Physics and block
placement always use logical coordinates, so the window size does not affect
the game. The mouse position is converted to logical coordinates, and clicks
outside the playfield are ignored.

The start window is fitted to the screen's free work area. On Linux/X11
(including XWayland) it is read with `xprop` and `xrandr`; otherwise, or if
that fails, the game starts at 360 × 640. The window can always be resized.

## Main menu

Choose with the mouse or with the arrow keys and Enter:

- **Play** – the three built-in levels in order.
- **Custom levels** – pick a saved custom level to play. Broken files are
  shown in red with the reason and cannot be started.
- **Level editor** – build your own levels.
- **Save last video** – shown when the last round can be saved as a video
  (see below).
- **Language** – switch between English and Swedish.

## Controls

| Key / mouse                   | Action                                        |
| ----------------------------- | --------------------------------------------- |
| Mouse, ←/→ (or A/D)           | Move the paddle                               |
| Space / left click            | Launch the ball                               |
| P                             | Pause / continue                              |
| R                             | Restart                                       |
| Esc                           | To the menu (when test playing: to the editor) |
| V / the Save video button     | Save the round as a video (after game over/win) |

Where the ball hits the paddle decides the bounce angle: the middle sends it
straight up, the ends out to the side. You have three lives and the game has
three levels.

## Blocks and bonuses

- **Blocks** need 1, 2 or 3 hits; **stones** (dark grey) cannot be destroyed.
  A level is cleared when every breakable block is gone.
- Some blocks hold a **bonus**: a capsule falls when the block breaks; catch it
  with the paddle. Paddle +/− and Speed +/− move one step at a time, Multiball
  splits every ball in three, and the Bottom wall stops five balls.
- Bonuses last until the life or level ends; a life is lost only when every
  ball is gone.

The full rules, with all numbers, are in [docs/game_rules.md](docs/game_rules.md).

## Saving videos

Every round is recorded in the background, and afterwards you can choose to
save it as an MP4 file:

- **Format:** 1080 × 1920 (9:16) at 60 fps, H.264 with stereo AAC sound. The
  video is drawn separately at full resolution, so the window size and side
  bars do not affect it.
- **Sound:** the same sound effects as in the game, mixed by the game's own
  clock so sound and picture stay in sync.
- **Saving:** at game over and on winning, **Save video (V)** is shown. If you
  leave the round with Esc, **Save last video** is in the main menu.
- **File name and folder:** in the dialog you choose a file name (pre-filled
  with the date and time in UTC; `.mp4` is added) and a folder. Tab switches
  field, Enter saves. If the file exists the game asks before overwriting it.
- **Default folder:** your video folder + `RustOut`: `~/Videos/RustOut` on
  Linux (following `XDG_VIDEOS_DIR`), `~/Movies/RustOut` on macOS and
  `%USERPROFILE%\Videos\RustOut` on Windows. `RUSTOUT_VIDEO_DIR` sets another
  default. The folder you chose last is offered again while the game runs.
- **What is recorded:** recording starts when the round's first ball is
  launched; the time before that is left out and the video starts with the
  launch sound. Level changes and lost lives are recorded.
- **End:** recording stops on the very frame you win (or at game over, Esc or
  a restart with R). The last picture is the field at the winning moment,
  without the "YOU WIN!" card. A sound cut off at the end fades out over 20 ms.
- Paused time is left out. The red REC marker is only shown in the window.
  A round where no ball was launched gives no video.
- **Unsaved videos** are deleted when the next round starts. The recording is
  kept in the system's temp folder meanwhile; leftovers from interrupted runs
  are cleaned up after a day.
- **ffmpeg is required** for recording (see Requirements). Without it the game
  works as usual, and the game over screen explains why videos cannot be saved.

## Level editor

The editor shows the level's grid. Blocks can only be placed in the build
area, which is the full width (540 px) and 576 px tall (y 130–706). Below it
is a free safety area for the paddle and the ball's start. The grid always
fills the whole build area, without unused strips:

- Block width = 540 / columns, block height = 576 / rows.
- The default is 12 × 24, i.e. blocks of 45 × 24 px.
- Limits: 4–20 columns and 8–36 rows. The smallest block is 27 × 16 px,
  larger than the ball.

Blocks sit edge to edge with each other and with the side walls. The visible
gaps are drawn inside the cells and do not change the sizes, so a block or a
stone in the outer column closes the path along the wall. Drawing, mouse
placement and collisions use the same cell sizes, and the editor uses the same
level data as the game.

**Grid** (the button or G) opens a dialog where columns and rows are changed
independently, with −/+ or the arrow keys (left/right = columns, up/down =
rows). The dialog shows the resulting block size; the current grid and block
size are also shown at the top right of the editor. When the grid changes,
blocks keep their row and column, with their type and bonus. If blocks would
end up outside the new grid, the editor says how many and asks first; answering
no changes nothing.

| Key / mouse                    | Action                                          |
| ------------------------------ | ----------------------------------------------- |
| Left click / drag              | Place the chosen brush (hold to paint)          |
| Right click / drag             | Remove blocks – with a bonus brush only the bonus |
| 1, 2, 3 / 4 or the palette     | Block with 1, 2 or 3 hits / stone               |
| Bonus palette (below the grid) | Choose a bonus; left click puts it on a breakable block |
| N / New                        | New empty level                                 |
| F2 / Name                      | Name the level                                  |
| G / Grid                       | Change the number of columns and rows           |
| S / Save                       | Save                                            |
| O / Open                       | Open a custom or built-in level                 |
| T / Test                       | Test play; Esc returns to the editor            |
| Esc / Menu                     | Back to the main menu                           |

- **Save** writes a file named after the level (e.g. `My level` →
  `my-level.json`). If another file with that name exists, the editor asks
  before overwriting it.
- **Unsaved changes** are marked with `*`. The editor asks before throwing
  them away (New, Open, Menu). Test play does not touch the editor: you come
  back to exactly the same layout, unsaved changes included.
- **Bonuses** can only sit on breakable blocks. If a block becomes a stone or
  is erased, its bonus goes too. In the editor the bonus label is shown on the
  block.
- **Built-in levels** open as a copy ("The Diamond copy") and cannot be
  overwritten; their names (in both languages) are reserved.
- **Errors** – invalid files, an empty level, a missing name or a failed save –
  are shown in a box with the reason.

## Where files are stored

The game's data folder:

| System  | Folder                                                   |
| ------- | -------------------------------------------------------- |
| Linux   | `$XDG_DATA_HOME/rustout`, normally `~/.local/share/rustout` |
| macOS   | `~/Library/Application Support/RustOut`                  |
| Windows | `%APPDATA%\RustOut`                                      |

- `levels/` – custom levels as readable JSON files. `RUSTOUT_LEVEL_DIR` chooses
  another folder. It is created on the first save and is also shown on the
  Custom levels screen.
- `settings.json` – the chosen language.

### Level file format

Format 3. `grid` gives the number of columns and rows. `rows` are the rows from
the top, one character per column (`.` = empty, `1`–`3` = number of hits,
`#` = stone); empty rows at the end can be left out. `bonuses` give row and
column counted from 1 and can be left out:

```json
{
  "format": 3,
  "name": "My level",
  "grid": { "columns": 12, "rows": 24 },
  "rows": [
    "#3333333333#",
    "1.2.2..2.2.1"
  ],
  "bonuses": [
    { "row": 2, "col": 3, "bonus": "multiball" }
  ]
}
```

Bonus names in the file: `paddle_plus`, `paddle_minus`, `speed_plus`,
`speed_minus`, `multiball`, `wall`. The file format is the same whatever
language the game is in.

**Compatibility:**

- Format 1 files (before bonuses) become levels without bonuses and stones.
  Format 1 had no bonus types, so there are no old bonus names to translate.
- Formats 1 and 2 have no `grid` and get the earlier 12 × 24 grid, so the
  layout is exactly as before.
- When an older level is saved again it is written as format 3.
- A grid outside the limits, rows of the wrong length, unknown bonus names,
  bonuses on stones or empty cells and unknown format numbers give a clear
  error message.

## Requirements

- Rust (stable, 1.82 or later) via [rustup](https://rustup.rs)
- Linux: development packages for X11, ALSA and OpenGL, e.g. on Debian/Ubuntu:
  `sudo apt install libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev`
- Windows and macOS need nothing extra to build.
- To save videos: [ffmpeg](https://ffmpeg.org) in `PATH`, with libx264 and AAC
  (Debian/Ubuntu: `sudo apt install ffmpeg`). It is only needed when the game
  runs, not to build it.

## Commands

```bash
cargo run
```

```bash
cargo run --release
```

Tests (game logic, editor, files, translations and screen transitions run
without a window):

```bash
cargo test
```

## Structure

The code is split into window-free logic (game, levels, editor, files,
translations, video bookkeeping) and a thin Macroquad layer for input, drawing
and sound. See [docs/architecture.md](docs/architecture.md) for the module map
and how a frame flows through the code.

## License

RustOut is released under the [MIT License](LICENSE).
