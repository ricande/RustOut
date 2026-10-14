//! Scripted runs, for checking the UI without touching the real mouse and
//! keyboard: `RUSTOUT_SCRIPT=check.txt cargo run`.
//!
//! A script is a text file with one step per line, `<frame> <command> …`.
//! Frames count from 1 and steps run in the frame they name; `#` starts a
//! comment. Positions are field coordinates (540 × 960). In a scripted run the
//! frame time is a fixed 1/60 s, so the same script gives the same pictures.
//!
//! | Command              | Effect                                              |
//! | -------------------- | --------------------------------------------------- |
//! | `key <name>`         | press a key (`enter`, `esc`, `space`, `up`, `f2`, `1` …) |
//! | `type <text>`        | type the rest of the line as characters             |
//! | `click <x> <y>`      | left click (press and release) at a position        |
//! | `rclick <x> <y>`     | right click                                         |
//! | `press <x> <y>`      | press and hold the left button                      |
//! | `rpress <x> <y>`     | press and hold the right button                     |
//! | `move <x> <y>`       | move the mouse (drags while a button is held)       |
//! | `release`            | release the held buttons                            |
//! | `wheel <amount>`     | scroll; positive is up                              |
//! | `autopilot on/off`   | let the paddle follow the lowest ball               |
//! | `shot <name>`        | save the frame as `<name>.png` in `RUSTOUT_SHOTS`   |
//! | `quit`               | end the program after this frame                    |

use macroquad::math::{vec2, Vec2};

use crate::ui::{Input, Key};

#[derive(Clone, Debug, PartialEq)]
enum Command {
    Key(Key),
    Type(String),
    Click { pos: Vec2, right: bool },
    Press { pos: Vec2, right: bool },
    Move(Vec2),
    Release,
    Wheel(f32),
    Autopilot(bool),
    Shot(String),
    Quit,
}

/// What a scripted frame asks for.
#[derive(Debug, Default)]
pub struct Frame {
    pub input: Input,
    /// Pictures to save after this frame is drawn.
    pub shots: Vec<String>,
    pub quit: bool,
    /// Steer the paddle towards the lowest ball.
    pub autopilot: bool,
}

#[derive(Debug, Default)]
pub struct Script {
    steps: Vec<(u32, Command)>,
    pointer: Option<Vec2>,
    left_held: bool,
    right_held: bool,
    autopilot: bool,
}

fn parse_pos(args: &[&str]) -> Result<Vec2, String> {
    match args {
        [x, y] => {
            let x: f32 = x.parse().map_err(|_| format!("bad x: {x}"))?;
            let y: f32 = y.parse().map_err(|_| format!("bad y: {y}"))?;
            Ok(vec2(x, y))
        }
        _ => Err("expected <x> <y>".into()),
    }
}

impl Script {
    /// Parses a script; errors name the line.
    pub fn parse(text: &str) -> Result<Script, String> {
        let mut steps = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let fail = |msg: String| format!("line {}: {msg}: {raw}", i + 1);
            let mut words = line.split_whitespace();
            let frame: u32 = words
                .next()
                .and_then(|w| w.parse().ok())
                .filter(|&f| f > 0)
                .ok_or_else(|| fail("expected a frame number from 1".into()))?;
            let name = words.next().ok_or_else(|| fail("missing command".into()))?;
            let args: Vec<&str> = words.collect();
            let command = match (name, args.as_slice()) {
                ("key", [k]) => {
                    Command::Key(Key::from_name(k).ok_or_else(|| fail(format!("unknown key {k}")))?)
                }
                ("type", _) => {
                    // The rest of the line after "type ", spaces included.
                    let rest = line.split_once("type").map_or("", |(_, r)| r);
                    Command::Type(rest.strip_prefix(' ').unwrap_or(rest).to_owned())
                }
                ("click" | "rclick", pos) => Command::Click {
                    pos: parse_pos(pos).map_err(fail)?,
                    right: name == "rclick",
                },
                ("press" | "rpress", pos) => Command::Press {
                    pos: parse_pos(pos).map_err(fail)?,
                    right: name == "rpress",
                },
                ("move", pos) => Command::Move(parse_pos(pos).map_err(fail)?),
                ("release", []) => Command::Release,
                ("wheel", [a]) => {
                    Command::Wheel(a.parse().map_err(|_| fail(format!("bad amount {a}")))?)
                }
                ("autopilot", ["on"]) => Command::Autopilot(true),
                ("autopilot", ["off"]) => Command::Autopilot(false),
                ("shot", [n]) => Command::Shot((*n).to_owned()),
                ("quit", []) => Command::Quit,
                _ => return Err(fail("unknown command or wrong arguments".into())),
            };
            steps.push((frame, command));
        }
        Ok(Script {
            steps,
            ..Default::default()
        })
    }

    /// The input and actions for frame `n` (counting from 1).
    pub fn frame(&mut self, n: u32) -> Frame {
        let mut f = Frame::default();
        let mut pressed = (false, false);
        let commands: Vec<Command> = self
            .steps
            .iter()
            .filter(|(at, _)| *at == n)
            .map(|(_, c)| c.clone())
            .collect();
        let mut released_after = false;
        for command in commands {
            match command {
                Command::Key(k) => f.input.keys.push(k),
                Command::Type(text) => f.input.chars.extend(text.chars()),
                Command::Click { pos, right } => {
                    self.pointer = Some(pos);
                    if right {
                        pressed.1 = true;
                        self.right_held = true;
                    } else {
                        pressed.0 = true;
                        self.left_held = true;
                    }
                    released_after = true;
                }
                Command::Press { pos, right } => {
                    self.pointer = Some(pos);
                    if right {
                        pressed.1 = true;
                        self.right_held = true;
                    } else {
                        pressed.0 = true;
                        self.left_held = true;
                    }
                }
                Command::Move(pos) => self.pointer = Some(pos),
                Command::Release => {
                    self.left_held = false;
                    self.right_held = false;
                }
                Command::Wheel(a) => f.input.wheel += a,
                Command::Autopilot(on) => self.autopilot = on,
                Command::Shot(name) => f.shots.push(name),
                Command::Quit => f.quit = true,
            }
        }
        f.input.pointer = self.pointer;
        f.input.left_pressed = pressed.0;
        f.input.right_pressed = pressed.1;
        f.input.left_down = self.left_held;
        f.input.right_down = self.right_held;
        f.autopilot = self.autopilot;
        if released_after {
            // A click is held for this frame only.
            self.left_held = false;
            self.right_held = false;
        }
        f
    }

    /// The last frame any step refers to.
    pub fn last_frame(&self) -> u32 {
        self.steps.iter().map(|(n, _)| *n).max().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_become_input_in_their_frame() {
        let mut s = Script::parse(
            "# a comment\n\
             2 key enter\n\
             2 key f2   # same frame\n\
             3 type Min bana\n\
             4 click 100 200\n\
             6 shot menu\n\
             7 quit\n",
        )
        .unwrap();
        assert_eq!(s.last_frame(), 7);
        assert!(s.frame(1).input.keys.is_empty());
        assert_eq!(s.frame(2).input.keys, vec![Key::Enter, Key::F2]);
        assert_eq!(
            s.frame(3).input.chars.iter().collect::<String>(),
            "Min bana"
        );
        let click = s.frame(4).input;
        assert!(click.left_pressed && click.left_down);
        assert_eq!(click.pointer, Some(vec2(100.0, 200.0)));
        let after = s.frame(5).input;
        assert!(
            !after.left_pressed && !after.left_down,
            "a click is released"
        );
        assert_eq!(after.pointer, Some(vec2(100.0, 200.0)), "the mouse stays");
        assert_eq!(s.frame(6).shots, vec!["menu"]);
        assert!(s.frame(7).quit);
    }

    #[test]
    fn press_move_release_drags() {
        let mut s =
            Script::parse("1 rpress 10 20\n2 move 30 40\n3 release\n4 autopilot on\n5 wheel -1")
                .unwrap();
        let f = s.frame(1).input;
        assert!(f.right_pressed && f.right_down && !f.left_down);
        let f = s.frame(2).input;
        assert!(!f.right_pressed && f.right_down);
        assert_eq!(f.pointer, Some(vec2(30.0, 40.0)));
        assert!(!s.frame(3).input.right_down);
        assert!(s.frame(4).autopilot);
        assert_eq!(s.frame(5).input.wheel, -1.0);
        assert!(s.frame(5).autopilot, "autopilot stays on");
    }

    #[test]
    fn errors_name_the_line() {
        for (text, expected) in [
            ("0 quit", "line 1"),
            ("x quit", "frame number"),
            ("1 quit\n2 jump", "line 2"),
            ("1 key hyper", "unknown key"),
            ("1 click 10", "<x> <y>"),
            ("1 click a 10", "bad x"),
            ("1 quit now", "wrong arguments"),
        ] {
            let err = Script::parse(text).unwrap_err();
            assert!(err.contains(expected), "{text}: {err}");
        }
    }
}
