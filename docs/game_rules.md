# Game rules

This document describes how RustOut plays: the field, the ball and paddle,
scoring, lives, blocks and bonuses. The numbers are the values used in the code
(mostly constants at the top of `src/game.rs` and `src/levels.rs`); if you
change one there, update it here.

All sizes are in **logical pixels** on the 540 × 960 playfield. The window is
only a scaled view of it, so window size never changes the rules.

## The field

| Part           | Where / size                                                   |
| -------------- | -------------------------------------------------------------- |
| Playfield      | 540 × 960, portrait 9:16                                       |
| HUD band       | y 0–64 (score, level, lives); the ceiling is at y 64           |
| Build area     | x 0–540, y 130–706: the only place blocks can be               |
| Safety area    | below the build area: free space for the paddle and the serve  |
| Paddle         | top edge at y 890, 14 px tall                                  |
| Bottom         | y 960: a ball that falls below it is lost                      |

The ball bounces off the left wall, the right wall and the ceiling. There is no
wall at the bottom, unless the *bottom wall* bonus is active.

## Rounds, lives and levels

- A game is a run through a list of levels: the three built-in levels in order,
  or a single custom level (also when test playing from the editor).
- You start with **3 lives**.
- Each life starts with the ball resting on the paddle (**serve**). Space or a
  click launches it, angled 20° from vertical towards the side the paddle is
  moving to (right if it is still).
- A life is lost when **every** ball has fallen out of the field. With lives
  left, a new serve follows; with none left it is **game over**.
- A level is cleared when every **breakable** block is gone (stones do not
  count). The next level starts with a serve; after the last level you **win**.
- `P` pauses and resumes (nothing moves while paused). `R` restarts the whole
  game. `Esc` leaves to the menu (or back to the editor when test playing).

## Ball and paddle

- The paddle is 96 px wide at normal size. It follows the mouse (at most
  2400 px/s) or moves with the arrow keys / A and D (640 px/s), and never
  leaves the field.
- **Bounce angle:** where the ball hits the top of the paddle decides its new
  direction: straight up in the middle, up to 60° from vertical at the ends.
  The offset is measured relative to the current paddle width.
- A ball that hits the side of the paddle is knocked sideways.
- **Never flat:** after every bounce the ball travels at least 18° from
  horizontal, so it cannot get stuck going back and forth between the walls.
- **Speed:** a level starts at 440 px/s + 40 px/s per level (level 1 = 440).
  Every hit on a breakable block adds 3 px/s, up to 820 px/s. All balls always
  have the same speed. The *speed* bonuses multiply this (see below).
- **No tunnelling:** the ball moves in steps of at most half its radius
  (3.5 px), so it cannot pass through a block, the paddle or the bottom wall at
  any speed. It resolves at most one block per step, so it cannot destroy two
  blocks with one touch of the seam between them.
- Physics run at a fixed 120 steps per second, independent of the frame rate.

## Blocks

Levels are grids of cells that fill the whole build area; each cell is empty or
holds one block. The default grid is 12 columns × 24 rows (blocks of 45 × 24 px);
custom levels can use 4–20 columns and 8–36 rows. Blocks lie edge to edge with
each other and with the side walls (the gaps you see are only drawn), so the
ball can never slip between two blocks or between a block and a wall.

| Block | Hits | Looks like                                      |
| ----- | ---- | ----------------------------------------------- |
| `1`   | 1    | plain coloured block                            |
| `2`   | 2    | metal rim, one dot per hit left                 |
| `3`   | 3    | metal rim, one dot per hit left                 |
| `#`   | –    | **stone**: always dark grey with rivets; indestructible |

The colour of a breakable block comes from its row. A damaged block gets darker.
A stone only bounces the ball: it gives no points, does not speed the ball up
and does not have to be destroyed to clear the level.

## Scoring

| Event                               | Points              |
| ----------------------------------- | ------------------- |
| Hit a block without destroying it   | 10                  |
| Destroy a block                     | 50 × its hits (50 / 100 / 150) |
| Catch a bonus capsule               | 50                  |
| Clear a level                       | 1000                |

## Bonuses

Some breakable blocks carry a bonus (a small coloured dot in the corner of the
block; in the editor its label is shown). When such a block is destroyed, a
capsule with the bonus falls straight down at 170 px/s. Catch it with the paddle
to apply it; a capsule that leaves the bottom of the field is lost.

| Capsule (en / sv) | Bonus       | Effect |
| ----------------- | ----------- | ------ |
| `P+`              | Paddle +    | paddle one step wider |
| `P-`              | Paddle −    | paddle one step narrower |
| `S+` / `H+`       | Speed +     | all balls one step faster |
| `S-` / `H-`       | Speed −     | all balls one step slower |
| `x3`              | Multiball   | every ball gets two new balls |
| `W` / `V`         | Bottom wall | a wall along the bottom edge |

**Paddle width** has three steps: 100 % (normal), 150 % and 200 %. *Paddle +*
and *Paddle −* move one step and stop at the smallest and largest step. The
paddle stays inside the field when it grows next to a wall.

**Ball speed** has three steps: 75 % (slow), 100 % (normal) and 125 % (fast).
*Speed +* and *Speed −* move one step and stop at the ends. The step applies to
every ball, also balls created later, and balls keep their direction.

**Multiball:** every ball that exists when the capsule is caught gets two new
balls at the same position and speed, turned 20° to either side of its
direction. The originals keep going: 1 → 3 → 9 → 27 … There is no upper limit.
New balls are made from a snapshot of the ball list, so balls created by the
bonus are not split again in the same activation.

**Bottom wall:** a wall along the whole bottom edge (top at y 948). Balls bounce
off it; capsules fall straight through it. It takes **five ball hits in total**,
shared by all balls. The fifth hit still bounces the ball and then removes the
wall. Catching another wall bonus resets it to five hits. It is shown as five
segments and the text `WALL n/5`.

**No timers.** Width and speed last until the life or the level ends. When a
life is lost, a new level starts or the game restarts, everything goes back to
normal: one ball, normal paddle, normal speed, no bottom wall, and capsules that
are still falling disappear.

## Built-in levels

| # | English name | Swedish name | Notes |
| - | ------------ | ------------ | ----- |
| 1 | Warm-up      | Uppvärmning  | eight full rows; two rows of 2-hit blocks on top |
| 2 | The Diamond  | Diamanten    | a diamond of 1- and 2-hit blocks with two stones in the middle |
| 3 | The Fortress | Fästningen   | 3-hit walls with stone corners around an inner keep |

Each built-in level has one of each of the six bonuses. Built-in levels live in
the code and cannot be overwritten; their names are reserved in both languages.

## Video recording rules

(Not gameplay, but part of a round.) A round is recorded from the moment its
**first ball is launched** until it ends: win, game over, `Esc` or `R`. Paused
time is not recorded, and the last frame is the field at the moment the round
ends, without the "YOU WIN!" / "GAME OVER" card. See the README for how videos
are saved.
