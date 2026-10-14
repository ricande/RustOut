//! Game rules and physics. No graphics, audio or window access lives here,
//! so everything in this module can be unit tested headlessly.

use macroquad::math::{vec2, Rect, Vec2};

use crate::levels::{Bonus, Layout};

/// Logical playfield size (portrait 9:16). The window never affects these.
pub const FIELD_W: f32 = 540.0;
pub const FIELD_H: f32 = 960.0;
/// The ceiling; the area above it is used for the HUD.
pub const WALL_TOP: f32 = 64.0;

/// Normal paddle width; bonuses scale it by `PADDLE_WIDTHS`.
pub const PADDLE_W: f32 = 96.0;
pub const PADDLE_H: f32 = 14.0;
pub const PADDLE_Y: f32 = 890.0;
/// Paddle width steps: normal, wider, widest.
pub const PADDLE_WIDTHS: [f32; 3] = [1.0, 1.5, 2.0];
const PADDLE_KEY_SPEED: f32 = 640.0;
const PADDLE_MOUSE_SPEED: f32 = 2400.0;

pub const BALL_R: f32 = 7.0;
pub const START_LIVES: u32 = 3;

const BASE_SPEED: f32 = 440.0;
const SPEED_PER_LEVEL: f32 = 40.0;
const SPEED_PER_BRICK: f32 = 3.0;
const MAX_SPEED: f32 = 820.0;
/// Ball speed steps: slow, normal, fast.
pub const SPEED_FACTORS: [f32; 3] = [0.75, 1.0, 1.25];
pub const NORMAL_SPEED_STEP: usize = 1;

/// Top edge of the bottom wall bonus.
pub const BOTTOM_WALL_Y: f32 = FIELD_H - 12.0;
pub const BOTTOM_WALL_HITS: u8 = 5;

pub const DROP_W: f32 = 40.0;
pub const DROP_H: f32 = 18.0;
const DROP_SPEED: f32 = 170.0;

/// Largest deflection from straight up when hitting the paddle edge.
const MAX_BOUNCE_ANGLE: f32 = 60.0 * std::f32::consts::PI / 180.0;
/// The ball never travels flatter than this (measured from horizontal).
pub const MIN_ANGLE: f32 = 18.0 * std::f32::consts::PI / 180.0;
const LAUNCH_ANGLE: f32 = 20.0 * std::f32::consts::PI / 180.0;
/// New multiball balls leave at this angle either side of the original.
pub const MULTIBALL_ANGLE: f32 = 20.0 * std::f32::consts::PI / 180.0;
/// The ball never moves further than this per collision check, which
/// keeps it from tunnelling through bricks or the paddle at any speed.
const MAX_SUBSTEP: f32 = BALL_R * 0.5;

const POINTS_PER_HIT: u32 = 10;
const POINTS_PER_HP: u32 = 50;
const POINTS_PER_BONUS: u32 = 50;
const LEVEL_BONUS: u32 = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Ball rests on the paddle, waiting to be launched.
    Serve,
    Playing,
    Paused,
    GameOver,
    Victory,
}

/// One-shot actions, triggered once per key press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Launch,
    TogglePause,
    Restart,
}

/// Continuous input, sampled every physics step.
#[derive(Clone, Copy, Debug, Default)]
pub struct Controls {
    /// -1 = left, 1 = right, 0 = none (keyboard).
    pub dir: f32,
    /// Desired paddle centre in field coordinates (mouse).
    pub target_x: Option<f32>,
}

/// Things that happened, for sound and visual effects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Launch,
    WallBounce,
    PaddleBounce,
    BrickHit { pos: Vec2 },
    BrickDestroyed { pos: Vec2, row: usize },
    StoneHit { pos: Vec2 },
    BonusCaught { bonus: Bonus, pos: Vec2 },
    BottomWallHit { remaining: u8 },
    BottomWallBroken,
    LifeLost,
    LevelCleared,
    GameOver,
    Victory,
}

#[derive(Clone, Debug)]
pub struct Paddle {
    /// Centre x.
    pub x: f32,
    pub vx: f32,
    /// Index into `PADDLE_WIDTHS`.
    pub width_step: usize,
}

impl Paddle {
    pub fn width(&self) -> f32 {
        PADDLE_W * PADDLE_WIDTHS[self.width_step]
    }

    pub fn rect(&self) -> Rect {
        let w = self.width();
        Rect::new(self.x - w / 2.0, PADDLE_Y, w, PADDLE_H)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub pos: Vec2,
    pub vel: Vec2,
}

#[derive(Clone, Debug)]
pub struct Brick {
    pub rect: Rect,
    pub hp: u8,
    pub max_hp: u8,
    pub row: usize,
    /// Indestructible stone: the ball bounces off, nothing else happens.
    pub stone: bool,
    /// Dropped when the brick is destroyed.
    pub bonus: Option<Bonus>,
}

/// A bonus capsule falling towards the paddle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drop {
    /// Centre.
    pub pos: Vec2,
    pub bonus: Bonus,
}

impl Drop {
    pub fn rect(&self) -> Rect {
        Rect::new(
            self.pos.x - DROP_W / 2.0,
            self.pos.y - DROP_H / 2.0,
            DROP_W,
            DROP_H,
        )
    }
}

enum BallStep {
    Alive,
    Lost,
    LevelCleared,
}

pub struct Game {
    /// The levels of this run, played in order.
    levels: Vec<Layout>,
    pub state: State,
    resume_state: State,
    pub score: u32,
    pub lives: u32,
    /// 0-based level index.
    pub level: usize,
    pub paddle: Paddle,
    pub balls: Vec<Ball>,
    pub bricks: Vec<Brick>,
    pub drops: Vec<Drop>,
    /// Base ball speed (grows per brick); the actual speed also depends
    /// on `speed_step`, see `ball_speed`.
    speed: f32,
    /// Index into `SPEED_FACTORS`.
    pub speed_step: usize,
    /// Hits left on the bottom wall; 0 = no wall.
    pub wall_hits: u8,
    events: Vec<Event>,
}

impl Game {
    /// A new run through `levels`, waiting to serve on the first one.
    pub fn new(levels: Vec<Layout>) -> Self {
        assert!(!levels.is_empty(), "a game needs at least one level");
        let mut game = Game {
            bricks: levels[0].bricks(),
            levels,
            state: State::Serve,
            resume_state: State::Serve,
            score: 0,
            lives: START_LIVES,
            level: 0,
            paddle: Paddle {
                x: FIELD_W / 2.0,
                vx: 0.0,
                width_step: 0,
            },
            balls: Vec::new(),
            drops: Vec::new(),
            speed: BASE_SPEED,
            speed_step: NORMAL_SPEED_STEP,
            wall_hits: 0,
            events: Vec::new(),
        };
        game.reset_round();
        game
    }

    fn start_new_game(&mut self) {
        self.score = 0;
        self.lives = START_LIVES;
        self.load_level(0);
    }

    fn load_level(&mut self, level: usize) {
        self.level = level;
        self.bricks = self.levels[level].bricks();
        self.paddle.x = FIELD_W / 2.0;
        self.state = State::Serve;
        self.reset_round();
    }

    fn base_speed(&self) -> f32 {
        BASE_SPEED + SPEED_PER_LEVEL * self.level as f32
    }

    /// Back to one ball on a normal paddle at normal speed, no bonuses.
    fn reset_round(&mut self) {
        self.speed = self.base_speed();
        self.speed_step = NORMAL_SPEED_STEP;
        self.paddle.width_step = 0;
        self.clamp_paddle();
        self.wall_hits = 0;
        self.drops.clear();
        self.balls = vec![Ball {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
        }];
        self.stick_ball_to_paddle();
    }

    fn stick_ball_to_paddle(&mut self) {
        let pos = vec2(self.paddle.x, PADDLE_Y - BALL_R - 1.0);
        if let Some(ball) = self.balls.first_mut() {
            ball.pos = pos;
        }
    }

    /// Current speed of every ball.
    pub fn ball_speed(&self) -> f32 {
        self.speed * SPEED_FACTORS[self.speed_step]
    }

    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    pub fn level_name(&self) -> &str {
        &self.levels[self.level].name
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    pub fn command(&mut self, cmd: Command) {
        match (cmd, self.state) {
            (Command::Restart, _) => self.start_new_game(),
            (Command::Launch, State::GameOver | State::Victory) => self.start_new_game(),
            (Command::Launch, State::Serve) => self.launch(),
            (Command::TogglePause, State::Serve | State::Playing) => {
                self.resume_state = self.state;
                self.state = State::Paused;
            }
            (Command::TogglePause, State::Paused) => self.state = self.resume_state,
            _ => {}
        }
    }

    fn launch(&mut self) {
        // Serve slightly in the direction the paddle is moving (right if still).
        let side = if self.paddle.vx < -1.0 { -1.0 } else { 1.0 };
        let vel = vec2(side * LAUNCH_ANGLE.sin(), -LAUNCH_ANGLE.cos()) * self.ball_speed();
        for ball in &mut self.balls {
            ball.vel = vel;
        }
        self.state = State::Playing;
        self.events.push(Event::Launch);
    }

    /// Advances the simulation by one fixed time step.
    pub fn update(&mut self, dt: f32, controls: &Controls) {
        match self.state {
            State::Serve => {
                self.move_paddle(dt, controls);
                self.stick_ball_to_paddle();
            }
            State::Playing => {
                self.move_paddle(dt, controls);
                self.step_balls(dt);
                if self.state == State::Playing {
                    self.update_drops(dt);
                }
            }
            _ => {}
        }
    }

    fn clamp_paddle(&mut self) {
        let half = self.paddle.width() / 2.0;
        self.paddle.x = self.paddle.x.clamp(half, FIELD_W - half);
    }

    fn move_paddle(&mut self, dt: f32, controls: &Controls) {
        let old = self.paddle.x;
        self.paddle.x = match controls.target_x {
            Some(target) => {
                let max = PADDLE_MOUSE_SPEED * dt;
                old + (target - old).clamp(-max, max)
            }
            None => old + controls.dir.clamp(-1.0, 1.0) * PADDLE_KEY_SPEED * dt,
        };
        self.clamp_paddle();
        self.paddle.vx = (self.paddle.x - old) / dt;
    }

    fn step_balls(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.balls.len() {
            let mut ball = self.balls[i];
            match self.step_ball(&mut ball, dt) {
                BallStep::Alive => {
                    self.balls[i] = ball;
                    i += 1;
                }
                // The last ball moves into slot `i` and is stepped next.
                BallStep::Lost => {
                    self.balls.swap_remove(i);
                }
                BallStep::LevelCleared => {
                    self.level_cleared();
                    return;
                }
            }
        }
        // A life is only lost once every ball is gone.
        if self.balls.is_empty() {
            self.lose_life();
        }
    }

    fn step_ball(&mut self, ball: &mut Ball, dt: f32) -> BallStep {
        let distance = ball.vel.length() * dt;
        let steps = (distance / MAX_SUBSTEP).ceil().max(1.0) as usize;
        let sub_dt = dt / steps as f32;
        for _ in 0..steps {
            ball.pos += ball.vel * sub_dt;
            self.collide_walls(ball);
            if ball.pos.y - BALL_R > FIELD_H {
                return BallStep::Lost;
            }
            self.collide_paddle(ball);
            if self.collide_bricks(ball) && self.bricks.iter().all(|b| b.stone) {
                return BallStep::LevelCleared;
            }
        }
        BallStep::Alive
    }

    fn collide_walls(&mut self, b: &mut Ball) {
        let mut bounced = false;
        if b.pos.x - BALL_R < 0.0 {
            b.pos.x = BALL_R;
            if b.vel.x < 0.0 {
                b.vel.x = -b.vel.x;
                bounced = true;
            }
        } else if b.pos.x + BALL_R > FIELD_W {
            b.pos.x = FIELD_W - BALL_R;
            if b.vel.x > 0.0 {
                b.vel.x = -b.vel.x;
                bounced = true;
            }
        }
        if b.pos.y - BALL_R < WALL_TOP {
            b.pos.y = WALL_TOP + BALL_R;
            if b.vel.y < 0.0 {
                b.vel.y = -b.vel.y;
                bounced = true;
            }
        }
        if bounced {
            b.vel = clamp_angle(b.vel, self.ball_speed());
            self.events.push(Event::WallBounce);
        }

        if self.wall_hits > 0 && b.pos.y + BALL_R > BOTTOM_WALL_Y && b.vel.y > 0.0 {
            b.pos.y = BOTTOM_WALL_Y - BALL_R;
            b.vel.y = -b.vel.y;
            b.vel = clamp_angle(b.vel, self.ball_speed());
            // The hits are shared by all balls; the last one still bounces.
            self.wall_hits -= 1;
            self.events.push(Event::BottomWallHit {
                remaining: self.wall_hits,
            });
            if self.wall_hits == 0 {
                self.events.push(Event::BottomWallBroken);
            }
        }
    }

    fn collide_paddle(&mut self, ball: &mut Ball) {
        let rect = self.paddle.rect();
        let Some((normal, depth)) = circle_rect_contact(ball.pos, BALL_R, rect) else {
            return;
        };
        if normal.y < 0.0 {
            // Top surface: the bounce angle depends on where the ball hits.
            if ball.vel.y > 0.0 {
                let half = self.paddle.width() / 2.0 + BALL_R;
                let offset = ((ball.pos.x - self.paddle.x) / half).clamp(-1.0, 1.0);
                let angle = offset * MAX_BOUNCE_ANGLE;
                ball.vel = vec2(angle.sin(), -angle.cos()) * self.ball_speed();
                self.events.push(Event::PaddleBounce);
            }
            ball.pos.y = rect.y - BALL_R;
        } else if normal.x != 0.0 {
            // Side of the paddle: knock the ball away; it is probably lost anyway.
            ball.pos.x += normal.x * depth;
            if ball.vel.x * normal.x < 0.0 {
                ball.vel.x = -ball.vel.x;
                ball.vel = clamp_angle(ball.vel, self.ball_speed());
                self.events.push(Event::PaddleBounce);
            }
        } else {
            ball.pos += normal * depth;
        }
    }

    /// Resolves at most one brick contact (the deepest). Returns true on a hit.
    fn collide_bricks(&mut self, ball: &mut Ball) -> bool {
        let mut best: Option<(usize, Vec2, f32)> = None;
        for (i, brick) in self.bricks.iter().enumerate() {
            if let Some((n, depth)) = circle_rect_contact(ball.pos, BALL_R, brick.rect) {
                if best.is_none_or(|(_, _, d)| depth > d) {
                    best = Some((i, n, depth));
                }
            }
        }
        let Some((i, normal, depth)) = best else {
            return false;
        };

        ball.pos += normal * depth;
        let into = ball.vel.dot(normal);
        if into < 0.0 {
            ball.vel -= 2.0 * into * normal;
        }

        let pos = self.bricks[i].rect.center();
        if self.bricks[i].stone {
            ball.vel = clamp_angle(ball.vel, self.ball_speed());
            self.events.push(Event::StoneHit { pos });
            return true;
        }

        self.speed = (self.speed + SPEED_PER_BRICK).min(MAX_SPEED);
        self.rescale_balls();
        ball.vel = clamp_angle(ball.vel, self.ball_speed());

        let brick = &mut self.bricks[i];
        brick.hp -= 1;
        if brick.hp == 0 {
            self.score += POINTS_PER_HP * brick.max_hp as u32;
            let row = brick.row;
            if let Some(bonus) = brick.bonus {
                self.drops.push(Drop { pos, bonus });
            }
            self.bricks.swap_remove(i);
            self.events.push(Event::BrickDestroyed { pos, row });
        } else {
            self.score += POINTS_PER_HIT;
            self.events.push(Event::BrickHit { pos });
        }
        true
    }

    /// Gives every ball the current speed, keeping its direction.
    fn rescale_balls(&mut self) {
        let speed = self.ball_speed();
        for ball in &mut self.balls {
            if let Some(dir) = ball.vel.try_normalize() {
                ball.vel = dir * speed;
            }
        }
    }

    fn update_drops(&mut self, dt: f32) {
        let paddle = self.paddle.rect();
        let mut caught = Vec::new();
        self.drops.retain_mut(|drop| {
            drop.pos.y += DROP_SPEED * dt;
            if drop.rect().overlaps(&paddle) {
                caught.push(*drop);
                return false;
            }
            // Capsules fall straight through the bottom wall.
            drop.pos.y - DROP_H / 2.0 <= FIELD_H
        });
        for drop in caught {
            self.apply_bonus(drop.bonus);
            self.score += POINTS_PER_BONUS;
            self.events.push(Event::BonusCaught {
                bonus: drop.bonus,
                pos: drop.pos,
            });
        }
    }

    pub fn apply_bonus(&mut self, bonus: Bonus) {
        let max_width = PADDLE_WIDTHS.len() - 1;
        let max_speed = SPEED_FACTORS.len() - 1;
        match bonus {
            Bonus::PaddlePlus => {
                self.paddle.width_step = (self.paddle.width_step + 1).min(max_width)
            }
            Bonus::PaddleMinus => self.paddle.width_step = self.paddle.width_step.saturating_sub(1),
            Bonus::SpeedPlus => {
                self.speed_step = (self.speed_step + 1).min(max_speed);
                self.rescale_balls();
            }
            Bonus::SpeedMinus => {
                self.speed_step = self.speed_step.saturating_sub(1);
                self.rescale_balls();
            }
            Bonus::MultiBall => {
                // Work from a snapshot, so new balls are not split again now.
                let snapshot = self.balls.clone();
                let speed = self.ball_speed();
                for ball in snapshot {
                    for angle in [MULTIBALL_ANGLE, -MULTIBALL_ANGLE] {
                        let vel = Vec2::from_angle(angle).rotate(ball.vel);
                        self.balls.push(Ball {
                            pos: ball.pos,
                            vel: clamp_angle(vel, speed),
                        });
                    }
                }
            }
            Bonus::Wall => self.wall_hits = BOTTOM_WALL_HITS,
        }
        self.clamp_paddle();
    }

    fn lose_life(&mut self) {
        self.lives = self.lives.saturating_sub(1);
        self.events.push(Event::LifeLost);
        if self.lives == 0 {
            self.state = State::GameOver;
            self.drops.clear();
            self.wall_hits = 0;
            self.events.push(Event::GameOver);
        } else {
            self.state = State::Serve;
            self.reset_round();
        }
    }

    fn level_cleared(&mut self) {
        self.score += LEVEL_BONUS;
        if self.level + 1 >= self.levels.len() {
            self.state = State::Victory;
            self.balls.clear();
            self.drops.clear();
            self.wall_hits = 0;
            self.events.push(Event::Victory);
        } else {
            self.load_level(self.level + 1);
            self.events.push(Event::LevelCleared);
        }
    }
}

/// Contact between a circle and a rectangle, as an axis-aligned outward
/// normal and the penetration depth along it. Corners snap to the dominant
/// axis, which gives classic arcade bounces and a clean push-out.
pub fn circle_rect_contact(c: Vec2, r: f32, rect: Rect) -> Option<(Vec2, f32)> {
    let closest = vec2(
        c.x.clamp(rect.x, rect.right()),
        c.y.clamp(rect.y, rect.bottom()),
    );
    let d = c - closest;
    if d.length_squared() >= r * r {
        return None;
    }
    if d == Vec2::ZERO {
        // Centre inside the rectangle: leave by the shortest way out.
        let exits = [
            (c.x - rect.x, vec2(-1.0, 0.0)),
            (rect.right() - c.x, vec2(1.0, 0.0)),
            (c.y - rect.y, vec2(0.0, -1.0)),
            (rect.bottom() - c.y, vec2(0.0, 1.0)),
        ];
        let (pen, n) = exits
            .into_iter()
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .unwrap();
        return Some((n, pen + r));
    }
    if d.x.abs() > d.y.abs() {
        Some((vec2(d.x.signum(), 0.0), r - d.x.abs()))
    } else {
        Some((vec2(0.0, d.y.signum()), r - d.y.abs()))
    }
}

/// Rescales `vel` to `speed` and keeps it from getting too close to horizontal.
pub fn clamp_angle(vel: Vec2, speed: f32) -> Vec2 {
    let dir = vel.try_normalize().unwrap_or(vec2(0.0, -1.0));
    let min_y = MIN_ANGLE.sin();
    if dir.y.abs() >= min_y {
        return dir * speed;
    }
    let sx = if dir.x < 0.0 { -1.0 } else { 1.0 };
    let sy = if dir.y < 0.0 { -1.0 } else { 1.0 };
    vec2(sx * MIN_ANGLE.cos(), sy * min_y) * speed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels;

    const DT: f32 = 1.0 / 120.0;

    fn playing_game() -> Game {
        let mut g = Game::new(levels::builtin());
        g.command(Command::Launch); // Serve -> Playing
        assert_eq!(g.state, State::Playing);
        g.take_events();
        g
    }

    /// A game with one plain brick far away, so nothing else interferes.
    fn quiet_game() -> Game {
        let mut g = playing_game();
        g.bricks = vec![brick(-1000.0, -1000.0, 1)];
        g
    }

    fn brick(x: f32, y: f32, hp: u8) -> Brick {
        Brick {
            rect: Rect::new(x, y, 58.0, 22.0),
            hp,
            max_hp: hp,
            row: 0,
            stone: false,
            bonus: None,
        }
    }

    fn stone(x: f32, y: f32) -> Brick {
        Brick {
            stone: true,
            ..brick(x, y, 1)
        }
    }

    fn idle() -> Controls {
        Controls::default()
    }

    fn set_ball(g: &mut Game, pos: Vec2, vel: Vec2) {
        g.balls = vec![Ball { pos, vel }];
    }

    fn ball(g: &Game) -> Ball {
        g.balls[0]
    }

    #[test]
    fn state_flow_serve_play_pause() {
        let mut g = Game::new(levels::builtin());
        assert_eq!(g.state, State::Serve);
        g.command(Command::Launch);
        assert_eq!(g.state, State::Playing);
        g.command(Command::TogglePause);
        assert_eq!(g.state, State::Paused);
        let before = ball(&g).pos;
        g.update(
            DT,
            &Controls {
                dir: 1.0,
                target_x: None,
            },
        );
        assert_eq!(ball(&g).pos, before, "nothing moves while paused");
        g.command(Command::Launch);
        assert_eq!(g.state, State::Paused, "launch ignored while paused");
        g.command(Command::TogglePause);
        assert_eq!(g.state, State::Playing);
    }

    #[test]
    fn pause_from_serve_resumes_to_serve() {
        let mut g = Game::new(levels::builtin());
        g.command(Command::TogglePause);
        g.command(Command::TogglePause);
        assert_eq!(g.state, State::Serve);
    }

    fn assert_normal_round(g: &Game) {
        assert_eq!(g.paddle.width_step, 0, "normal paddle");
        assert_eq!(g.speed_step, NORMAL_SPEED_STEP, "normal speed");
        assert_eq!(g.wall_hits, 0, "no bottom wall");
        assert_eq!(g.balls.len(), 1);
        assert!(g.drops.is_empty());
    }

    #[test]
    fn restart_resets_everything() {
        let mut g = playing_game();
        g.score = 1234;
        g.lives = 1;
        g.load_level(2);
        g.apply_bonus(Bonus::PaddlePlus);
        g.apply_bonus(Bonus::SpeedPlus);
        g.apply_bonus(Bonus::Wall);
        g.apply_bonus(Bonus::MultiBall);
        g.command(Command::Restart);
        assert_eq!(g.state, State::Serve);
        assert_eq!((g.score, g.lives, g.level), (0, START_LIVES, 0));
        assert_eq!(g.bricks.len(), levels::builtin()[0].brick_count());
        assert_normal_round(&g);
    }

    #[test]
    fn ball_follows_paddle_while_serving() {
        let mut g = Game::new(levels::builtin());
        for _ in 0..30 {
            g.update(
                DT,
                &Controls {
                    dir: 1.0,
                    target_x: None,
                },
            );
        }
        assert!(g.paddle.x > FIELD_W / 2.0);
        assert_eq!(ball(&g).pos.x, g.paddle.x);
    }

    #[test]
    fn paddle_stays_inside_field() {
        let mut g = Game::new(levels::builtin());
        for _ in 0..600 {
            g.update(
                DT,
                &Controls {
                    dir: -1.0,
                    target_x: None,
                },
            );
        }
        assert_eq!(g.paddle.rect().x, 0.0);
        // Growing at the edge keeps the paddle inside.
        g.apply_bonus(Bonus::PaddlePlus);
        g.apply_bonus(Bonus::PaddlePlus);
        assert_eq!(g.paddle.rect().x, 0.0);
        for _ in 0..600 {
            g.update(
                DT,
                &Controls {
                    dir: 0.0,
                    target_x: Some(5000.0),
                },
            );
        }
        assert_eq!(g.paddle.rect().right(), FIELD_W);
    }

    #[test]
    fn bounces_off_side_walls_and_ceiling() {
        let mut g = quiet_game();
        set_ball(&mut g, vec2(BALL_R + 1.0, 300.0), vec2(-300.0, -300.0));
        g.update(DT, &idle());
        assert!(ball(&g).vel.x > 0.0);
        assert!(ball(&g).pos.x >= BALL_R);

        set_ball(
            &mut g,
            vec2(FIELD_W - BALL_R - 1.0, 300.0),
            vec2(300.0, -300.0),
        );
        g.update(DT, &idle());
        assert!(ball(&g).vel.x < 0.0);

        set_ball(
            &mut g,
            vec2(400.0, WALL_TOP + BALL_R + 1.0),
            vec2(100.0, -400.0),
        );
        g.update(DT, &idle());
        assert!(ball(&g).vel.y > 0.0);
        assert!(g.take_events().contains(&Event::WallBounce));
    }

    fn paddle_bounce_at(g: &mut Game, offset: f32) -> Vec2 {
        let pos = vec2(g.paddle.x + offset, PADDLE_Y - BALL_R - 2.0);
        set_ball(g, pos, vec2(0.0, 400.0));
        g.update(DT, &idle());
        assert!(g.take_events().contains(&Event::PaddleBounce));
        assert!(ball(g).vel.y < 0.0, "ball must go up after paddle hit");
        ball(g).vel
    }

    #[test]
    fn paddle_bounce_angle_depends_on_hit_position() {
        let mut g = quiet_game();
        let left = paddle_bounce_at(&mut g, -45.0);
        let center = paddle_bounce_at(&mut g, 0.0);
        let right = paddle_bounce_at(&mut g, 45.0);
        let slight_right = paddle_bounce_at(&mut g, 15.0);
        assert!(left.x < 0.0);
        assert!(center.x.abs() < 1.0);
        assert!(right.x > 0.0);
        assert!(right.x > slight_right.x && slight_right.x > 0.0);
        // Even the edge never sends the ball flatter than the max angle.
        assert!(right.y.abs() >= right.length() * MAX_BOUNCE_ANGLE.cos() - 0.01);

        // On a wider paddle the same offset gives a smaller angle.
        g.apply_bonus(Bonus::PaddlePlus);
        let wide = paddle_bounce_at(&mut g, 45.0);
        assert!(wide.x > 0.0 && wide.x < right.x);
        // The ends of the widest paddle still catch the ball.
        g.apply_bonus(Bonus::PaddlePlus);
        let edge = paddle_bounce_at(&mut g, PADDLE_W - 2.0);
        assert!(edge.x > 0.0);
    }

    #[test]
    fn paddle_width_steps_and_limits() {
        let mut g = quiet_game();
        let widths = |g: &Game| g.paddle.width() / PADDLE_W;
        assert_eq!(widths(&g), 1.0);
        g.apply_bonus(Bonus::PaddleMinus);
        assert_eq!(widths(&g), 1.0, "already at the smallest step");
        g.apply_bonus(Bonus::PaddlePlus);
        assert_eq!(widths(&g), 1.5);
        g.apply_bonus(Bonus::PaddlePlus);
        assert_eq!(widths(&g), 2.0);
        g.apply_bonus(Bonus::PaddlePlus);
        assert_eq!(widths(&g), 2.0, "already at the largest step");
        g.apply_bonus(Bonus::PaddleMinus);
        assert_eq!(widths(&g), 1.5);
        assert_eq!(
            g.paddle.rect().w,
            PADDLE_W * 1.5,
            "collision uses the width"
        );
    }

    #[test]
    fn speed_steps_apply_to_all_balls_and_keep_direction() {
        let mut g = quiet_game();
        let normal = g.ball_speed();
        set_ball(&mut g, vec2(200.0, 400.0), vec2(0.6, -0.8) * normal);
        g.apply_bonus(Bonus::MultiBall);
        let dirs: Vec<Vec2> = g.balls.iter().map(|b| b.vel.normalize()).collect();

        g.apply_bonus(Bonus::SpeedPlus);
        g.apply_bonus(Bonus::SpeedPlus);
        assert!(
            (g.ball_speed() - normal * 1.25).abs() < 0.01,
            "stops at fast"
        );
        for (b, d) in g.balls.iter().zip(&dirs) {
            assert!((b.vel.length() - normal * 1.25).abs() < 0.01);
            assert!((b.vel.normalize() - *d).length() < 1e-4, "direction kept");
        }
        for _ in 0..3 {
            g.apply_bonus(Bonus::SpeedMinus);
        }
        assert!(
            (g.ball_speed() - normal * 0.75).abs() < 0.01,
            "stops at slow"
        );
        for b in &g.balls {
            assert!((b.vel.length() - normal * 0.75).abs() < 0.01);
        }
        // New balls get the current speed too.
        g.apply_bonus(Bonus::MultiBall);
        assert!(g
            .balls
            .iter()
            .all(|b| (b.vel.length() - normal * 0.75).abs() < 0.01));
    }

    #[test]
    fn multiball_triples_from_a_snapshot() {
        let mut g = quiet_game();
        let speed = g.ball_speed();
        let vel = vec2(0.0, -1.0) * speed;
        set_ball(&mut g, vec2(270.0, 500.0), vel);
        g.apply_bonus(Bonus::MultiBall);
        assert_eq!(g.balls.len(), 3);
        assert_eq!(g.balls[0].vel, vel, "the original keeps going");
        for b in &g.balls[1..] {
            assert_eq!(b.pos, vec2(270.0, 500.0));
            let angle = b.vel.angle_between(vel).abs();
            assert!((angle - MULTIBALL_ANGLE).abs() < 1e-4, "±20°: {angle}");
        }
        assert!(g.balls[1].vel.x * g.balls[2].vel.x < 0.0, "one each side");
        g.apply_bonus(Bonus::MultiBall);
        assert_eq!(g.balls.len(), 9);
        g.apply_bonus(Bonus::MultiBall);
        assert_eq!(g.balls.len(), 27, "no upper limit");
    }

    #[test]
    fn life_is_lost_only_when_all_balls_are_gone() {
        let mut g = quiet_game();
        g.balls = vec![
            Ball {
                pos: vec2(20.0, FIELD_H + 5.0),
                vel: vec2(0.0, 300.0),
            },
            Ball {
                pos: vec2(270.0, 400.0),
                vel: vec2(0.0, -300.0),
            },
        ];
        g.update(DT, &idle());
        assert_eq!(g.balls.len(), 1);
        assert_eq!((g.lives, g.state), (START_LIVES, State::Playing));
        g.balls[0].pos = vec2(20.0, FIELD_H + 5.0);
        g.balls[0].vel = vec2(0.0, 300.0);
        g.update(DT, &idle());
        assert_eq!((g.lives, g.state), (START_LIVES - 1, State::Serve));
    }

    #[test]
    fn bottom_wall_takes_five_shared_hits() {
        let mut g = quiet_game();
        g.apply_bonus(Bonus::Wall);
        assert_eq!(g.wall_hits, BOTTOM_WALL_HITS);
        let falling = |x: f32| Ball {
            pos: vec2(x, BOTTOM_WALL_Y - BALL_R - 1.0),
            vel: vec2(0.0, 900.0),
        };
        // Two balls hit at once: the hits are shared.
        g.balls = vec![falling(30.0), falling(500.0)];
        g.update(DT, &idle());
        assert_eq!(g.wall_hits, 3);
        assert!(g.balls.iter().all(|b| b.vel.y < 0.0));
        g.apply_bonus(Bonus::Wall);
        assert_eq!(g.wall_hits, 5, "a new wall bonus restores five hits");
        for expected in (0..5).rev() {
            g.balls = vec![falling(30.0)];
            g.update(DT, &idle());
            assert_eq!(g.wall_hits, expected);
            assert!(ball(&g).vel.y < 0.0, "even the fifth hit bounces");
        }
        assert!(g.take_events().contains(&Event::BottomWallBroken));
        // Without the wall the ball falls out.
        g.balls = vec![falling(30.0)];
        for _ in 0..10 {
            g.update(DT, &idle());
        }
        assert_eq!(g.lives, START_LIVES - 1);
    }

    #[test]
    fn bonus_drops_fall_and_are_caught_by_the_paddle() {
        let mut g = quiet_game();
        let mut b = brick(241.0, 300.0, 1);
        b.bonus = Some(Bonus::PaddlePlus);
        g.bricks.push(b);
        set_ball(&mut g, vec2(270.0, 330.0), vec2(0.0, -400.0));
        g.update(DT, &idle());
        assert_eq!(g.drops.len(), 1);
        assert_eq!(g.drops[0].bonus, Bonus::PaddlePlus);
        let c = Controls {
            dir: 0.0,
            target_x: Some(270.0),
        };
        for _ in 0..(120 * 5) {
            // Keep the ball out of the way while the capsule falls.
            set_ball(&mut g, vec2(30.0, 400.0), vec2(0.0, -10.0));
            g.update(DT, &c);
        }
        assert!(g.drops.is_empty());
        assert_eq!(g.paddle.width_step, 1);
        assert!(g.take_events().iter().any(|e| matches!(
            e,
            Event::BonusCaught {
                bonus: Bonus::PaddlePlus,
                ..
            }
        )));
    }

    #[test]
    fn missed_drops_pass_through_the_bottom_wall() {
        let mut g = quiet_game();
        g.apply_bonus(Bonus::Wall);
        g.drops.push(Drop {
            pos: vec2(20.0, BOTTOM_WALL_Y - 30.0),
            bonus: Bonus::MultiBall,
        });
        let c = Controls {
            dir: 0.0,
            target_x: Some(400.0),
        };
        for _ in 0..120 {
            set_ball(&mut g, vec2(400.0, 400.0), vec2(0.0, -10.0));
            g.update(DT, &c);
        }
        assert!(g.drops.is_empty(), "removed after leaving the field");
        assert_eq!(g.wall_hits, BOTTOM_WALL_HITS, "not hit by the capsule");
        assert_eq!(g.balls.len(), 1, "never caught");
    }

    #[test]
    fn life_loss_and_level_change_reset_bonuses() {
        let mut g = quiet_game();
        for b in [Bonus::PaddlePlus, Bonus::SpeedMinus, Bonus::Wall] {
            g.apply_bonus(b);
        }
        g.wall_hits = 0; // let the ball fall out
        g.drops.push(Drop {
            pos: vec2(100.0, 500.0),
            bonus: Bonus::Wall,
        });
        set_ball(&mut g, vec2(20.0, FIELD_H + 5.0), vec2(0.0, 300.0));
        g.update(DT, &idle());
        assert_eq!(g.state, State::Serve);
        assert_normal_round(&g);

        g.command(Command::Launch);
        for b in [Bonus::PaddlePlus, Bonus::SpeedPlus, Bonus::Wall] {
            g.apply_bonus(b);
        }
        clear_with_last_brick(&mut g);
        assert_eq!(g.level, 1);
        assert_normal_round(&g);
    }

    #[test]
    fn stones_cannot_be_destroyed_and_do_not_block_level_clear() {
        let mut g = playing_game();
        g.bricks = vec![stone(241.0, 300.0), brick(-1000.0, -1000.0, 1)];
        set_ball(&mut g, vec2(270.0, 330.0), vec2(0.0, -400.0));
        let score = g.score;
        for _ in 0..10 {
            g.update(DT, &idle());
        }
        assert_eq!(g.bricks.len(), 2, "stone survives");
        assert!(ball(&g).vel.y > 0.0, "ball bounces off the stone");
        assert_eq!(g.score, score);
        assert!(g
            .take_events()
            .iter()
            .any(|e| matches!(e, Event::StoneHit { .. })));
        // Only stones left after the last brick goes: level cleared.
        g.bricks = vec![stone(100.0, 300.0), brick(371.0, 300.0, 1)];
        set_ball(&mut g, vec2(400.0, 330.0), vec2(0.0, -400.0));
        for _ in 0..10 {
            g.update(DT, &idle());
        }
        assert_eq!(g.level, 1);
    }

    #[test]
    fn blocks_at_the_side_walls_leave_no_path() {
        // A stone on the left edge and a brick on the right edge.
        let layout = Layout::from_rows("x", &["#..........1", "......1....."]).unwrap();
        let mut g = Game::new(vec![layout]);
        g.command(Command::Launch);
        for x in [BALL_R, FIELD_W - BALL_R] {
            // Straight up, hugging the wall.
            set_ball(
                &mut g,
                vec2(x, levels::BUILD_AREA.y + 60.0),
                vec2(0.0, -400.0),
            );
            for _ in 0..30 {
                g.update(DT, &idle());
            }
            assert!(
                ball(&g).vel.y > 0.0,
                "x = {x}: the edge block must stop the ball"
            );
            assert!(ball(&g).pos.y > levels::BUILD_AREA.y + levels::Grid::DEFAULT.cell_h());
        }
    }

    #[test]
    fn collisions_follow_the_level_grid() {
        use crate::levels::{Block, Grid, BUILD_AREA};
        for (cols, rows) in [(4, 8), (20, 36), (7, 13), (16, 10)] {
            let grid = Grid::new(cols, rows).unwrap();
            let mut l = Layout::with_grid("x", grid);
            l.set_block(0, 0, Block::Stone);
            l.set_block(0, cols - 1, Block::Brick(3));
            let mid = cols / 2;
            l.set_block(2, mid, Block::Brick(3));
            let mut g = Game::new(vec![l]);
            g.command(Command::Launch);

            // The ball bounces exactly at the bottom edge of the cell.
            let cell = grid.cell_rect(2, mid);
            set_ball(
                &mut g,
                vec2(cell.center().x, cell.bottom() + 30.0),
                vec2(0.0, -400.0),
            );
            let mut top = f32::MAX;
            for _ in 0..20 {
                g.update(DT, &idle());
                top = top.min(ball(&g).pos.y);
            }
            assert!(
                (top - (cell.bottom() + BALL_R)).abs() < 0.5,
                "{cols}x{rows}: {top}"
            );
            let hit = g.bricks.iter().find(|b| b.rect == cell).unwrap();
            assert_eq!(
                hit.hp, 2,
                "{cols}x{rows}: the brick in that cell took the hit"
            );

            // Blocks in the outer columns close the side walls.
            for x in [BALL_R, FIELD_W - BALL_R] {
                let below = grid.cell_rect(0, 0).bottom() + 30.0;
                set_ball(&mut g, vec2(x, below), vec2(0.0, -400.0));
                for _ in 0..20 {
                    g.update(DT, &idle());
                }
                assert!(ball(&g).vel.y > 0.0, "{cols}x{rows} x={x}");
                assert!(ball(&g).pos.y > BUILD_AREA.y + grid.cell_h());
            }
        }
    }

    #[test]
    fn fast_ball_does_not_tunnel_through_paddle() {
        let mut g = quiet_game();
        let x = g.paddle.x;
        set_ball(&mut g, vec2(x, PADDLE_Y - 40.0), vec2(0.0, 5000.0));
        g.update(DT, &idle());
        assert!(ball(&g).vel.y < 0.0);
        assert!(ball(&g).pos.y < PADDLE_Y);
        assert_eq!(g.lives, START_LIVES);
    }

    #[test]
    fn fast_ball_does_not_tunnel_through_brick_or_bottom_wall() {
        let mut g = playing_game();
        g.bricks = vec![brick(371.0, 300.0, 1), brick(-1000.0, -1000.0, 1)];
        set_ball(&mut g, vec2(400.0, 350.0), vec2(0.0, -8000.0));
        g.update(DT, &idle());
        assert_eq!(g.bricks.len(), 1, "brick should be destroyed");
        assert!(ball(&g).vel.y > 0.0, "ball should bounce back down");
        assert!(ball(&g).pos.y > 322.0);

        g.apply_bonus(Bonus::Wall);
        set_ball(&mut g, vec2(20.0, BOTTOM_WALL_Y - 40.0), vec2(0.0, 8000.0));
        g.update(DT, &idle());
        assert!(ball(&g).vel.y < 0.0);
        assert!(ball(&g).pos.y <= BOTTOM_WALL_Y - BALL_R + 0.01);
    }

    #[test]
    fn only_one_brick_per_contact_and_multi_hit_bricks() {
        let mut g = playing_game();
        // Two bricks side by side; ball hits the seam between them.
        g.bricks = vec![brick(342.0, 300.0, 2), brick(404.0, 300.0, 2)];
        set_ball(&mut g, vec2(402.0, 335.0), vec2(0.0, -400.0));
        for _ in 0..10 {
            g.update(DT, &idle());
        }
        let total_hp: u8 = g.bricks.iter().map(|b| b.hp).sum();
        assert_eq!(total_hp, 3, "exactly one hit registered");
        assert_eq!(g.bricks.len(), 2);
        assert!(ball(&g).vel.y > 0.0);
        assert_eq!(g.score, POINTS_PER_HIT);
    }

    #[test]
    fn no_near_horizontal_bounces() {
        let v = clamp_angle(vec2(500.0, 1.0), 400.0);
        assert!((v.length() - 400.0).abs() < 0.01);
        assert!(v.y.abs() >= 400.0 * MIN_ANGLE.sin() - 0.01);
        assert!(v.x > 0.0 && v.y > 0.0);
        let v = clamp_angle(vec2(-500.0, -0.0), 400.0);
        assert!(v.x < 0.0 && v.y.abs() > 100.0);
        // Steep velocities are left alone apart from the speed.
        let v = clamp_angle(vec2(100.0, -300.0), 200.0);
        assert!((v.normalize() - vec2(100.0, -300.0).normalize()).length() < 1e-5);
    }

    #[test]
    fn losing_balls_costs_lives_then_game_over() {
        let mut g = quiet_game();
        for expected in (0..START_LIVES).rev() {
            g.state = State::Playing;
            set_ball(&mut g, vec2(20.0, FIELD_H + 5.0), vec2(0.0, 300.0));
            g.update(DT, &idle());
            assert_eq!(g.lives, expected);
        }
        assert_eq!(g.state, State::GameOver);
        let ev = g.take_events();
        assert!(ev.contains(&Event::GameOver));
        g.command(Command::Launch);
        assert_eq!(g.state, State::Serve);
        assert_eq!(g.lives, START_LIVES);
    }

    #[test]
    fn life_lost_returns_to_serve_on_paddle() {
        let mut g = quiet_game();
        set_ball(&mut g, vec2(20.0, FIELD_H + 5.0), vec2(0.0, 300.0));
        g.update(DT, &idle());
        assert_eq!(g.state, State::Serve);
        assert_eq!(ball(&g).vel, Vec2::ZERO);
        assert_eq!(ball(&g).pos.x, g.paddle.x);
    }

    fn clear_with_last_brick(g: &mut Game) {
        g.bricks = vec![brick(371.0, 300.0, 1)];
        set_ball(g, vec2(400.0, 330.0), vec2(0.0, -400.0));
        for _ in 0..10 {
            g.update(DT, &idle());
        }
    }

    #[test]
    fn clearing_levels_advances_and_ends_in_victory() {
        let mut g = playing_game();
        for level in 1..g.level_count() {
            clear_with_last_brick(&mut g);
            assert_eq!(g.level, level);
            assert_eq!(g.state, State::Serve);
            assert!(g.take_events().contains(&Event::LevelCleared));
            g.command(Command::Launch);
        }
        clear_with_last_brick(&mut g);
        assert_eq!(g.state, State::Victory);
        assert!(g.take_events().contains(&Event::Victory));
        assert!(g.score >= LEVEL_BONUS * g.level_count() as u32);
    }

    #[test]
    fn single_custom_level_ends_in_victory() {
        let layout = Layout::from_rows("Egen", &["..1........."]).unwrap();
        let mut g = Game::new(vec![layout]);
        assert_eq!((g.level_count(), g.level_name()), (1, "Egen"));
        assert_eq!(g.bricks.len(), 1);
        g.command(Command::Launch);
        clear_with_last_brick(&mut g);
        assert_eq!(g.state, State::Victory);
    }

    #[test]
    fn contact_normals() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert_eq!(
            circle_rect_contact(vec2(5.0, -3.0), 4.0, r).unwrap().0,
            vec2(0.0, -1.0)
        );
        assert_eq!(
            circle_rect_contact(vec2(13.0, 5.0), 4.0, r).unwrap().0,
            vec2(1.0, 0.0)
        );
        assert!(circle_rect_contact(vec2(5.0, -5.0), 4.0, r).is_none());
        let (n, depth) = circle_rect_contact(vec2(5.0, 9.0), 4.0, r).unwrap();
        assert_eq!(n, vec2(0.0, 1.0));
        assert!((depth - 5.0).abs() < 1e-5);
    }

    /// Long automated games with an auto-pilot paddle that also catches
    /// bonuses: every ball must stay inside the field with a sane speed and
    /// angle, and bricks must keep breaking.
    #[test]
    fn long_simulation_stays_stable() {
        for seed in 0..12 {
            let mut g = playing_game();
            let angle = (seed as f32 - 6.0) * 0.12;
            g.balls[0].vel = vec2(angle.sin(), -angle.cos()) * g.ball_speed();
            let start_bricks = g.bricks.len();
            let mut max_balls = 1;
            // Like a player, hit the ball at a different spot on every return
            // (a fixed spot would replay the same path forever).
            let mut aim: u32 = seed;
            for _ in 0..(120 * 180) {
                // Follow the lowest ball.
                let lowest = g
                    .balls
                    .iter()
                    .max_by(|a, b| a.pos.y.total_cmp(&b.pos.y))
                    .map_or(FIELD_W / 2.0, |b| b.pos.x);
                let target = lowest + (((aim >> 16) % 9) as f32 - 4.0) * 10.0;
                let c = Controls {
                    dir: 0.0,
                    target_x: Some(target),
                };
                g.update(DT, &c);
                if g.take_events().contains(&Event::PaddleBounce) {
                    aim = aim.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                }
                if g.state == State::Serve {
                    g.command(Command::Launch);
                }
                if g.state != State::Playing {
                    break;
                }
                max_balls = max_balls.max(g.balls.len());
                for b in &g.balls {
                    let p = b.pos;
                    assert!(p.is_finite());
                    assert!(p.x >= BALL_R - 0.01 && p.x <= FIELD_W - BALL_R + 0.01);
                    assert!(p.y >= WALL_TOP + BALL_R - 0.01);
                    let v = b.vel;
                    assert!((v.length() - g.ball_speed()).abs() < 1.0);
                    assert!(v.y.abs() >= v.length() * MIN_ANGLE.sin() - 0.5);
                }
            }
            assert!(
                g.bricks.len() < start_bricks / 2 || g.level > 0,
                "seed {seed}: too little progress ({} of {start_bricks} left)",
                g.bricks.len()
            );
            assert!(max_balls < 5000, "seed {seed}: runaway ball count");
        }
    }
}
