//! Drawing and purely visual effects. Reads the game state, never changes it.

use macroquad::prelude::*;

use crate::game::{
    Event, Game, State, BALL_R, BOTTOM_WALL_HITS, BOTTOM_WALL_Y, FIELD_H, FIELD_W,
    NORMAL_SPEED_STEP, PADDLE_H, PADDLE_Y, WALL_TOP,
};
use crate::levels::{Bonus, GAP};
use crate::t;

pub(crate) const BACKDROP: Color = Color::new(0.02, 0.02, 0.05, 1.0);
const FIELD_TOP: Color = Color::new(0.06, 0.05, 0.16, 1.0);
const FIELD_BOTTOM: Color = Color::new(0.02, 0.07, 0.12, 1.0);
pub(crate) const NEON: Color = Color::new(0.35, 0.85, 1.0, 1.0);
pub(crate) const TEXT: Color = Color::new(0.95, 0.95, 1.0, 1.0);
pub(crate) const ACCENT: Color = Color::new(1.0, 0.8, 0.25, 1.0);

const ROW_COLORS: [Color; 9] = [
    Color::new(1.00, 0.30, 0.35, 1.0),
    Color::new(1.00, 0.55, 0.20, 1.0),
    Color::new(1.00, 0.85, 0.25, 1.0),
    Color::new(0.45, 0.90, 0.35, 1.0),
    Color::new(0.25, 0.85, 0.80, 1.0),
    Color::new(0.30, 0.60, 1.00, 1.0),
    Color::new(0.60, 0.40, 1.00, 1.0),
    Color::new(0.95, 0.40, 0.85, 1.0),
    Color::new(1.00, 0.45, 0.55, 1.0),
];

/// Stones are always dark grey, whatever row they are in.
pub(crate) const STONE: Color = Color::new(0.27, 0.28, 0.30, 1.0);

/// Green = helps the player, red/orange = makes it harder.
pub(crate) fn bonus_color(bonus: Bonus) -> Color {
    match bonus {
        Bonus::PaddlePlus => Color::new(0.35, 0.90, 0.40, 1.0),
        Bonus::PaddleMinus => Color::new(1.00, 0.35, 0.35, 1.0),
        Bonus::SpeedPlus => Color::new(1.00, 0.60, 0.20, 1.0),
        Bonus::SpeedMinus => Color::new(0.30, 0.80, 1.00, 1.0),
        Bonus::MultiBall => Color::new(0.85, 0.50, 1.00, 1.0),
        Bonus::Wall => Color::new(0.40, 0.60, 1.00, 1.0),
    }
}

pub(crate) fn row_color(row: usize) -> Color {
    ROW_COLORS[row % ROW_COLORS.len()]
}

pub(crate) fn with_alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

pub(crate) fn shade(c: Color, f: f32) -> Color {
    Color::new(c.r * f, c.g * f, c.b * f, c.a)
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Maps the fixed 540x960 field onto the window, letterboxed so the
/// proportions never change when the window is resized.
#[derive(Clone, Copy)]
pub struct View {
    pub(crate) scale: f32,
    ox: f32,
    oy: f32,
}

impl View {
    pub fn fit(screen_w: f32, screen_h: f32) -> Self {
        let scale = (screen_w / FIELD_W).min(screen_h / FIELD_H).max(0.01);
        View {
            scale,
            ox: (screen_w - FIELD_W * scale) / 2.0,
            oy: (screen_h - FIELD_H * scale) / 2.0,
        }
    }

    /// True if a screen position (e.g. a click) lies inside the playfield view.
    pub fn contains(self, screen: (f32, f32)) -> bool {
        let p = self.to_field(screen);
        (0.0..=FIELD_W).contains(&p.x) && (0.0..=FIELD_H).contains(&p.y)
    }

    /// Converts a screen position (e.g. the mouse) to field coordinates.
    pub fn to_field(self, screen: (f32, f32)) -> Vec2 {
        vec2(
            (screen.0 - self.ox) / self.scale,
            (screen.1 - self.oy) / self.scale,
        )
    }

    fn shifted(self, offset: Vec2) -> Self {
        View {
            ox: self.ox + offset.x * self.scale,
            oy: self.oy + offset.y * self.scale,
            ..self
        }
    }

    pub(crate) fn rect(&self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let s = self.scale;
        draw_rectangle(self.ox + x * s, self.oy + y * s, w * s, h * s, color);
    }

    pub(crate) fn rect_lines(&self, x: f32, y: f32, w: f32, h: f32, thick: f32, color: Color) {
        let s = self.scale;
        draw_rectangle_lines(
            self.ox + x * s,
            self.oy + y * s,
            w * s,
            h * s,
            thick * s,
            color,
        );
    }

    pub(crate) fn circle(&self, p: Vec2, r: f32, color: Color) {
        draw_circle(
            self.ox + p.x * self.scale,
            self.oy + p.y * self.scale,
            r * self.scale,
            color,
        );
    }

    pub(crate) fn line(&self, a: Vec2, b: Vec2, thick: f32, color: Color) {
        let s = self.scale;
        draw_line(
            self.ox + a.x * s,
            self.oy + a.y * s,
            self.ox + b.x * s,
            self.oy + b.y * s,
            thick * s,
            color,
        );
    }

    pub(crate) fn text_size(&self, text: &str, size: f32) -> TextDimensions {
        measure_text(text, None, (size * self.scale).round().max(1.0) as u16, 1.0)
    }

    /// Draws text with a drop shadow; `y` is the baseline in field units.
    pub(crate) fn text(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        let px = (size * self.scale).round().max(1.0);
        let sx = self.ox + x * self.scale;
        let sy = self.oy + y * self.scale;
        let off = (size * self.scale * 0.06).max(1.0);
        draw_text(
            text,
            sx + off,
            sy + off,
            px,
            with_alpha(BLACK, 0.7 * color.a),
        );
        draw_text(text, sx, sy, px, color);
    }

    /// Centred text, shrunk if needed so it never spills outside the field.
    pub(crate) fn text_centered(&self, text: &str, cx: f32, y: f32, size: f32, color: Color) {
        const MAX_W: f32 = FIELD_W - 32.0;
        let mut size = size;
        let mut w = self.text_size(text, size).width / self.scale;
        if w > MAX_W {
            size *= MAX_W / w;
            w = self.text_size(text, size).width / self.scale;
        }
        self.text(text, cx - w / 2.0, y, size, color);
    }
}

struct Particle {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    max_life: f32,
    size: f32,
    color: Color,
}

/// Visual-only effects driven by game events.
#[derive(Default)]
pub struct Effects {
    particles: Vec<Particle>,
    trail: Vec<Vec2>,
    shake: f32,
    flash: f32,
    paddle_glow: f32,
    time: f32,
}

impl Effects {
    pub fn on_event(&mut self, event: &Event) {
        match *event {
            Event::BrickDestroyed { pos, row } => self.burst(pos, row_color(row), 18, 260.0),
            Event::BrickHit { pos } => self.burst(pos, Color::new(0.9, 0.9, 1.0, 1.0), 6, 140.0),
            Event::PaddleBounce => self.paddle_glow = 1.0,
            Event::LifeLost => {
                self.shake = 0.35;
                self.flash = 0.35;
                self.trail.clear();
            }
            Event::LevelCleared | Event::Victory => self.flash = 0.25,
            Event::BonusCaught { bonus, pos } => {
                self.burst(pos, bonus_color(bonus), 14, 220.0);
                self.paddle_glow = 1.0;
            }
            Event::StoneHit { pos } => self.burst(pos, Color::new(0.7, 0.7, 0.72, 1.0), 4, 120.0),
            Event::BottomWallHit { .. } => self.shake = self.shake.max(0.06),
            Event::BottomWallBroken => {
                for i in 0..12 {
                    let pos = vec2(20.0 + i as f32 * 45.0, BOTTOM_WALL_Y + 6.0);
                    self.burst(pos, NEON, 3, 200.0);
                }
            }
            _ => {}
        }
    }

    fn burst(&mut self, pos: Vec2, color: Color, count: usize, speed: f32) {
        for _ in 0..count {
            let angle = rand::gen_range(0.0, std::f32::consts::TAU);
            let v = rand::gen_range(0.3, 1.0) * speed;
            let life = rand::gen_range(0.35, 0.7);
            self.particles.push(Particle {
                pos: pos + vec2(rand::gen_range(-14.0, 14.0), rand::gen_range(-6.0, 6.0)),
                vel: vec2(angle.cos(), angle.sin()) * v,
                life,
                max_life: life,
                size: rand::gen_range(2.0, 4.5),
                color,
            });
        }
    }

    pub fn update(&mut self, dt: f32, game: &Game) {
        self.time += dt;
        self.shake = (self.shake - dt).max(0.0);
        self.flash = (self.flash - dt).max(0.0);
        self.paddle_glow = (self.paddle_glow - dt * 4.0).max(0.0);
        for p in &mut self.particles {
            p.vel.y += 500.0 * dt;
            p.pos += p.vel * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);

        // The trail only follows a single ball; with multiball it would be noise.
        if game.state == State::Playing && game.balls.len() == 1 {
            self.trail.push(game.balls[0].pos);
            if self.trail.len() > 10 {
                self.trail.remove(0);
            }
        } else if game.state != State::Paused {
            self.trail.clear();
        }
    }

    pub(crate) fn blink(&self) -> bool {
        (self.time * 2.0).fract() < 0.65
    }
}

/// Draws the game; `back_hint` tells how to leave (menu or editor).
/// Without `overlays` only the playfield and HUD are drawn (used for the
/// last frame of a video, so it does not end on a "game over" card).
pub fn draw(game: &Game, fx: &Effects, view: View, back_hint: &str, overlays: bool) {
    clear_background(BACKDROP);

    let shake = if fx.shake > 0.0 {
        let a = fx.shake * 14.0;
        vec2(rand::gen_range(-a, a), rand::gen_range(-a, a))
    } else {
        Vec2::ZERO
    };
    let v = view.shifted(shake);

    draw_field(&v);
    for brick in &game.bricks {
        draw_brick(&v, brick);
    }
    for drop in &game.drops {
        draw_capsule(&v, drop.rect(), drop.bonus);
    }
    draw_bottom_wall(&v, game);
    draw_paddle(&v, game, fx);
    if !matches!(game.state, State::GameOver | State::Victory) {
        draw_balls(&v, game, fx);
    }
    for p in &fx.particles {
        let a = (p.life / p.max_life).clamp(0.0, 1.0);
        v.rect(
            p.pos.x - p.size / 2.0,
            p.pos.y - p.size / 2.0,
            p.size,
            p.size,
            with_alpha(p.color, a),
        );
    }
    draw_hud(&view, game);
    if overlays {
        draw_overlay(&view, game, fx, back_hint);
    }

    if fx.flash > 0.0 {
        view.rect(
            0.0,
            0.0,
            FIELD_W,
            FIELD_H,
            Color::new(1.0, 0.3, 0.3, fx.flash * 0.5),
        );
    }
}

pub(crate) fn draw_field(v: &View) {
    const BANDS: usize = 24;
    let band_h = FIELD_H / BANDS as f32;
    for i in 0..BANDS {
        let t = i as f32 / (BANDS - 1) as f32;
        v.rect(
            0.0,
            i as f32 * band_h,
            FIELD_W,
            band_h + 0.5,
            lerp_color(FIELD_TOP, FIELD_BOTTOM, t),
        );
    }
    let grid = with_alpha(NEON, 0.05);
    let mut x = 0.0;
    while x <= FIELD_W {
        v.line(vec2(x, WALL_TOP), vec2(x, FIELD_H), 1.0, grid);
        x += 40.0;
    }
    let mut y = WALL_TOP;
    while y <= FIELD_H {
        v.line(vec2(0.0, y), vec2(FIELD_W, y), 1.0, grid);
        y += 40.0;
    }
    // HUD band and glowing walls.
    v.rect(
        0.0,
        0.0,
        FIELD_W,
        WALL_TOP,
        Color::new(0.03, 0.03, 0.08, 1.0),
    );
    for (w, a) in [(6.0, 0.15), (3.0, 0.35), (1.5, 1.0)] {
        let c = with_alpha(NEON, a);
        v.line(vec2(0.0, WALL_TOP), vec2(FIELD_W, WALL_TOP), w, c);
        v.line(vec2(0.0, WALL_TOP), vec2(0.0, FIELD_H), w, c);
        v.line(vec2(FIELD_W, WALL_TOP), vec2(FIELD_W, FIELD_H), w, c);
    }
}

/// The visible face of a block: its cell shrunk by half the visual gap.
pub(crate) fn brick_face(cell: Rect) -> Rect {
    let g = GAP / 2.0;
    Rect::new(cell.x + g, cell.y + g, cell.w - GAP, cell.h - GAP)
}

pub(crate) fn draw_brick(v: &View, brick: &crate::game::Brick) {
    let r = brick_face(brick.rect);
    if brick.stone {
        v.rect(r.x + 2.0, r.y + 3.0, r.w, r.h, with_alpha(BLACK, 0.35));
        v.rect(r.x, r.y, r.w, r.h, STONE);
        v.rect(r.x, r.y, r.w, 3.0, shade(STONE, 1.35));
        v.rect(r.x, r.y + r.h - 3.0, r.w, 3.0, shade(STONE, 0.6));
        v.rect_lines(r.x, r.y, r.w, r.h, 1.0, shade(STONE, 0.5));
        // Rivets, so a stone reads as metal rather than a dim brick.
        for x in [r.x + 6.0, r.right() - 6.0] {
            v.circle(vec2(x, r.y + r.h / 2.0), 2.0, shade(STONE, 1.6));
        }
        return;
    }
    let base = row_color(brick.row);
    let health = brick.hp as f32 / brick.max_hp as f32;
    let body = shade(base, 0.55 + 0.45 * health);
    v.rect(r.x + 2.0, r.y + 3.0, r.w, r.h, with_alpha(BLACK, 0.35));
    v.rect(r.x, r.y, r.w, r.h, body);
    v.rect(r.x, r.y, r.w, 4.0, with_alpha(WHITE, 0.35));
    v.rect(r.x, r.y + r.h - 4.0, r.w, 4.0, with_alpha(BLACK, 0.25));
    if brick.max_hp > 1 {
        let metal = Color::new(0.85, 0.88, 0.95, 0.9);
        v.rect_lines(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0, 2.0, metal);
        // One stud per remaining hit.
        let cy = r.y + r.h / 2.0;
        let spacing = 9.0;
        let start = r.x + r.w / 2.0 - (brick.hp as f32 - 1.0) * spacing / 2.0;
        for i in 0..brick.hp {
            v.circle(vec2(start + i as f32 * spacing, cy), 2.5, metal);
        }
    }
    if let Some(bonus) = brick.bonus {
        // A small gem in the corner hints that something is inside.
        let c = vec2(r.right() - 5.0, r.y + 5.0);
        v.circle(c, 4.0, BLACK);
        v.circle(c, 3.0, bonus_color(bonus));
    }
}

/// A falling bonus, or a palette button in the editor.
pub(crate) fn draw_capsule(v: &View, r: Rect, bonus: Bonus) {
    let color = bonus_color(bonus);
    let rad = r.h / 2.0;
    v.rect(
        r.x - 2.0,
        r.y - 2.0,
        r.w + 4.0,
        r.h + 4.0,
        with_alpha(color, 0.25),
    );
    v.rect(r.x + rad, r.y, r.w - r.h, r.h, color);
    v.circle(vec2(r.x + rad, r.y + rad), rad, color);
    v.circle(vec2(r.right() - rad, r.y + rad), rad, color);
    v.rect(
        r.x + rad,
        r.y + 2.0,
        r.w - r.h,
        3.0,
        with_alpha(WHITE, 0.45),
    );
    v.text_centered(
        bonus.label(),
        r.x + r.w / 2.0,
        r.y + r.h * 0.78,
        r.h * 0.85,
        BLACK,
    );
}

fn draw_bottom_wall(v: &View, game: &Game) {
    if game.wall_hits == 0 {
        return;
    }
    // One segment per remaining hit, so the strength is visible at a glance.
    let total = BOTTOM_WALL_HITS as f32;
    let seg = FIELD_W / total;
    let h = FIELD_H - BOTTOM_WALL_Y;
    v.rect(
        0.0,
        BOTTOM_WALL_Y - 3.0,
        FIELD_W,
        h + 3.0,
        with_alpha(NEON, 0.15),
    );
    for i in 0..BOTTOM_WALL_HITS {
        let x = i as f32 * seg + 2.0;
        let lit = i < game.wall_hits;
        let color = if lit { NEON } else { with_alpha(NEON, 0.12) };
        v.rect(x, BOTTOM_WALL_Y, seg - 4.0, h, color);
        if lit {
            v.rect(x, BOTTOM_WALL_Y, seg - 4.0, 3.0, with_alpha(WHITE, 0.6));
        }
    }
    v.text(
        &t!("hud.wall", left = game.wall_hits, total = BOTTOM_WALL_HITS),
        FIELD_W - 112.0,
        BOTTOM_WALL_Y - 8.0,
        18.0,
        NEON,
    );
}

fn draw_paddle(v: &View, game: &Game, fx: &Effects) {
    let w = game.paddle.width();
    let x = game.paddle.x - w / 2.0;
    let y = PADDLE_Y;
    let h = PADDLE_H;
    let r = h / 2.0;
    let glow = with_alpha(NEON, 0.15 + 0.35 * fx.paddle_glow);
    v.rect(x - 4.0, y - 4.0, w + 8.0, h + 8.0, glow);
    let body = Color::new(0.75, 0.82, 0.92, 1.0);
    v.rect(x + r, y, w - h, h, body);
    v.circle(vec2(x + r, y + r), r, Color::new(1.0, 0.35, 0.3, 1.0));
    v.circle(vec2(x + w - r, y + r), r, Color::new(1.0, 0.35, 0.3, 1.0));
    v.rect(x + r, y + 2.0, w - h, 3.0, with_alpha(WHITE, 0.6));
    v.rect(x + r, y + h - 4.0, w - h, 3.0, shade(body, 0.6));
}

fn draw_balls(v: &View, game: &Game, fx: &Effects) {
    let n = fx.trail.len();
    for (i, p) in fx.trail.iter().enumerate() {
        let t = (i + 1) as f32 / (n + 1) as f32;
        v.circle(*p, BALL_R * (0.4 + 0.5 * t), with_alpha(ACCENT, 0.25 * t));
    }
    for ball in &game.balls {
        let p = ball.pos;
        v.circle(p, BALL_R * 2.4, with_alpha(ACCENT, 0.10));
        v.circle(p, BALL_R * 1.6, with_alpha(ACCENT, 0.25));
        v.circle(p, BALL_R, Color::new(1.0, 0.97, 0.85, 1.0));
        v.circle(p + vec2(-2.0, -2.0), BALL_R * 0.35, WHITE);
    }
    draw_status(v, game);
}

/// Paddle and speed steps, shown while they differ from normal.
fn draw_status(v: &View, game: &Game) {
    let mut parts = Vec::new();
    if game.paddle.width_step > 0 {
        parts.push(["", t!("hud.paddle_wider"), t!("hud.paddle_widest")][game.paddle.width_step]);
    }
    if game.speed_step != NORMAL_SPEED_STEP {
        parts.push([t!("hud.speed_slow"), "", t!("hud.speed_fast")][game.speed_step]);
    }
    if game.balls.len() > 1 {
        v.text(
            &t!("hud.balls", count = game.balls.len()),
            12.0,
            84.0,
            18.0,
            TEXT,
        );
    }
    if !parts.is_empty() {
        let y = if game.wall_hits > 0 {
            BOTTOM_WALL_Y - 8.0
        } else {
            FIELD_H - 10.0
        };
        v.text(&parts.join("   "), 12.0, y, 18.0, with_alpha(TEXT, 0.85));
    }
}

fn draw_hud(v: &View, game: &Game) {
    let y = 42.0;
    let size = 24.0;
    let score = format!("{:06}", game.score);
    v.text(&t!("hud.score", score = score), 14.0, y, size, TEXT);
    v.text_centered(
        &t!(
            "hud.level",
            level = game.level + 1,
            count = game.level_count()
        ),
        FIELD_W / 2.0 + 30.0,
        y,
        size,
        ACCENT,
    );
    let icons_start = FIELD_W - 14.0 - game.lives as f32 * 18.0;
    for i in 0..game.lives {
        let c = vec2(icons_start + 9.0 + i as f32 * 18.0, y - 7.0);
        v.circle(c, 7.0, with_alpha(ACCENT, 0.3));
        v.circle(c, 5.0, Color::new(1.0, 0.97, 0.85, 1.0));
    }
}

pub(crate) fn dim(v: &View, alpha: f32) {
    v.rect(
        0.0,
        WALL_TOP,
        FIELD_W,
        FIELD_H - WALL_TOP,
        with_alpha(BLACK, alpha),
    );
}

fn draw_overlay(v: &View, game: &Game, fx: &Effects, back_hint: &str) {
    let cx = FIELD_W / 2.0;
    let prompt = with_alpha(TEXT, if fx.blink() { 1.0 } else { 0.25 });
    let replay = t!("game.replay_hint");
    match game.state {
        State::Serve => {
            let y = 680.0;
            let level = t!("game.level", level = game.level + 1);
            v.text_centered(&level, cx, y, 48.0, ACCENT);
            v.text_centered(game.level_name(), cx, y + 36.0, 26.0, NEON);
            v.text_centered(t!("game.launch_hint"), cx, y + 76.0, 24.0, prompt);
        }
        State::Paused => {
            dim(v, 0.6);
            v.text_centered(t!("game.paused"), cx, 460.0, 80.0, ACCENT);
            v.text_centered(t!("game.resume_hint"), cx, 510.0, 26.0, TEXT);
            v.text_centered(back_hint, cx, 550.0, 22.0, TEXT);
        }
        State::GameOver => {
            dim(v, 0.65);
            v.text_centered(
                t!("game.over"),
                cx,
                440.0,
                80.0,
                Color::new(1.0, 0.35, 0.35, 1.0),
            );
            let score = t!("game.score", score = game.score);
            v.text_centered(&score, cx, 495.0, 30.0, TEXT);
            v.text_centered(replay, cx, 580.0, 24.0, prompt);
            v.text_centered(back_hint, cx, 620.0, 22.0, TEXT);
        }
        State::Victory => {
            dim(v, 0.6);
            let done = if game.level_count() > 1 {
                t!("game.all_cleared")
            } else {
                t!("game.level_cleared")
            };
            v.text_centered(t!("game.won"), cx, 420.0, 80.0, ACCENT);
            v.text_centered(done, cx, 475.0, 30.0, TEXT);
            let score = t!("game.score", score = game.score);
            v.text_centered(&score, cx, 515.0, 30.0, TEXT);
            v.text_centered(replay, cx, 600.0, 24.0, prompt);
            v.text_centered(back_hint, cx, 640.0, 22.0, TEXT);
        }
        State::Playing => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_keeps_9_16_and_centres_in_any_window() {
        for (sw, sh) in [
            (540.0, 960.0),
            (1920.0, 1080.0),
            (300.0, 900.0),
            (800.0, 600.0),
        ] {
            let v = View::fit(sw, sh);
            let (w, h) = (FIELD_W * v.scale, FIELD_H * v.scale);
            assert!((w / h - 9.0 / 16.0).abs() < 1e-4);
            assert!(
                w <= sw + 0.01 && h <= sh + 0.01,
                "view must fit, never crop"
            );
            assert!((v.ox * 2.0 + w - sw).abs() < 0.01, "centred horizontally");
            assert!((v.oy * 2.0 + h - sh).abs() < 0.01, "centred vertically");
            assert!(v.ox.abs() < 0.01 || v.oy.abs() < 0.01, "fills one axis");
        }
    }

    #[test]
    fn mouse_maps_to_field_and_outside_clicks_are_rejected() {
        // 1920x1080: field is 607.5 x 1080, starting at x = 656.25.
        let v = View::fit(1920.0, 1080.0);
        let centre = v.to_field((960.0, 540.0));
        assert!((centre - vec2(FIELD_W / 2.0, FIELD_H / 2.0)).length() < 0.01);
        let corner = v.to_field((656.25, 0.0));
        assert!(corner.length() < 0.01);
        assert!(v.contains((960.0, 540.0)));
        assert!(!v.contains((100.0, 540.0)), "left letterbox");
        assert!(!v.contains((1800.0, 540.0)), "right letterbox");
    }
}
