# Changelog

All notable changes to RustOut are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-10-14

The first version, built in one go.

### Added

- **Game:** Breakout/Arkanoid-style play with a paddle controlled by mouse or
  arrow keys, a bounce angle that depends on where the ball hits the paddle,
  score, three lives and three built-in levels (Warm-up, The Diamond,
  The Fortress). States for serve, play, pause (`P`), game over and win;
  restart with `R`.
- **Stable physics:** fixed 120 Hz time step, sub-stepping so the ball never
  tunnels through blocks, the paddle or walls, at most one block per contact,
  and a minimum bounce angle against near-horizontal loops.
- **Portrait playfield:** logical 540 × 960 (9:16) scaled proportionally to the
  window with letterboxing; the start window is fitted to the screen's work
  area; mouse input is mapped to the field and clicks outside it are ignored.
- **Blocks:** 1-, 2- and 3-hit blocks and indestructible dark grey stones;
  blocks lie edge to edge with each other and the side walls.
- **Bonuses:** Paddle +/− (100/150/200 %), Speed +/− (75/100/125 %), Multiball
  (every ball splits in three, no limit) and Bottom wall (five shared hits).
  Bonuses drop as capsules from destroyed blocks and reset when a life or level
  ends.
- **Level editor:** paint and erase blocks, stones and bonuses with the mouse,
  adjustable grid (4–20 columns × 8–36 rows filling the build area), name, save,
  open, test play with unsaved changes kept, warnings before losing changes or
  overwriting, and clear error messages.
- **Level files:** readable JSON (format 3) in the user's data folder, with
  support for older formats 1 and 2; custom levels can be played from the menu.
- **Video recording:** each round is recorded from the first launch until it
  ends and can be saved afterwards as a 1080 × 1920, 60 fps MP4 with stereo
  sound (needs ffmpeg), with a chosen file name and folder.
- **Translations:** English by default, Swedish selectable in the menu; the
  choice is saved in `settings.json`.
- **Sound and graphics:** synthesized sound effects and code-drawn arcade
  graphics with particles; no asset files.
- **Documentation:** README, game rules, architecture, AGENTS.md, this changelog
  and the MIT license.

[Unreleased]: https://github.com/ricande/RustOut/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ricande/RustOut/releases/tag/v0.1.0
