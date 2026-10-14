//! Level layouts, shared by the game, the editor and the level files.
//!
//! A layout is a grid of `cols` x `rows` cells that always fills the whole
//! build area; the grid size decides the block size. As text, each row is a
//! string of `.` (empty), `1`-`3` (a brick that needs that many hits) and `#`
//! (an indestructible stone). Breakable bricks can also carry a bonus.

use std::ops::RangeInclusive;

use macroquad::math::{Rect, Vec2};

use crate::game::{Brick, FIELD_W, PADDLE_Y};
use crate::i18n::{tr_in, Lang};
use crate::t;

/// Where blocks may be placed: the full field width, from below the HUD
/// down to well above the paddle (the paddle and ball start stay free).
pub const BUILD_AREA: Rect = Rect {
    x: 0.0,
    y: 130.0,
    w: FIELD_W,
    h: 576.0,
};
pub const COLS_RANGE: RangeInclusive<usize> = 4..=20;
pub const ROWS_RANGE: RangeInclusive<usize> = 8..=36;
/// Space drawn between blocks, inside each cell. Visual only: the collision
/// shape is the whole cell.
pub const GAP: f32 = 4.0;
pub const MAX_HP: u8 = 3;
pub const MAX_NAME_LEN: usize = 32;

/// Bonuses that fall from a destroyed brick and act when the paddle catches them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bonus {
    PaddlePlus,
    PaddleMinus,
    SpeedPlus,
    SpeedMinus,
    MultiBall,
    Wall,
}

impl Bonus {
    pub const ALL: [Bonus; 6] = [
        Bonus::PaddlePlus,
        Bonus::PaddleMinus,
        Bonus::SpeedPlus,
        Bonus::SpeedMinus,
        Bonus::MultiBall,
        Bonus::Wall,
    ];

    /// Name used in level files.
    pub fn key(self) -> &'static str {
        match self {
            Bonus::PaddlePlus => "paddle_plus",
            Bonus::PaddleMinus => "paddle_minus",
            Bonus::SpeedPlus => "speed_plus",
            Bonus::SpeedMinus => "speed_minus",
            Bonus::MultiBall => "multiball",
            Bonus::Wall => "wall",
        }
    }

    pub fn from_key(key: &str) -> Option<Bonus> {
        Bonus::ALL.into_iter().find(|b| b.key() == key)
    }

    /// Name shown to the player.
    pub fn name(self) -> &'static str {
        match self {
            Bonus::PaddlePlus => t!("bonus.paddle_plus"),
            Bonus::PaddleMinus => t!("bonus.paddle_minus"),
            Bonus::SpeedPlus => t!("bonus.speed_plus"),
            Bonus::SpeedMinus => t!("bonus.speed_minus"),
            Bonus::MultiBall => t!("bonus.multiball"),
            Bonus::Wall => t!("bonus.wall"),
        }
    }

    /// Short label drawn on capsules and bricks.
    pub fn label(self) -> &'static str {
        match self {
            Bonus::PaddlePlus => t!("bonus.label_paddle_plus"),
            Bonus::PaddleMinus => t!("bonus.label_paddle_minus"),
            Bonus::SpeedPlus => t!("bonus.label_speed_plus"),
            Bonus::SpeedMinus => t!("bonus.label_speed_minus"),
            Bonus::MultiBall => t!("bonus.label_multiball"),
            Bonus::Wall => t!("bonus.label_wall"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Block {
    #[default]
    Empty,
    /// Needs 1-3 hits.
    Brick(u8),
    /// Can never be destroyed.
    Stone,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub block: Block,
    /// Only breakable bricks carry a bonus.
    pub bonus: Option<Bonus>,
}

type BuiltinLevel = (
    &'static str,
    &'static [&'static str],
    &'static [(usize, usize, Bonus)],
);

const BUILTIN: [BuiltinLevel; 3] = [
    (
        "level.warmup",
        &[
            "222222222222",
            "222222222222",
            "111111111111",
            "111111111111",
            "111111111111",
            "111111111111",
            "111111111111",
            "111111111111",
        ],
        &[
            (2, 2, Bonus::PaddlePlus),
            (2, 9, Bonus::MultiBall),
            (4, 5, Bonus::SpeedMinus),
            (5, 1, Bonus::PaddleMinus),
            (6, 10, Bonus::Wall),
            (7, 6, Bonus::SpeedPlus),
        ],
    ),
    (
        "level.diamond",
        &[
            ".....22.....",
            "....2112....",
            "...211112...",
            "..21111112..",
            ".2111111112.",
            "21111##11112",
            ".2111111112.",
            "..21111112..",
            "...211112...",
            "....2112....",
            ".....22.....",
        ],
        &[
            (1, 6, Bonus::SpeedMinus),
            (3, 4, Bonus::MultiBall),
            (5, 1, Bonus::Wall),
            (5, 10, Bonus::PaddlePlus),
            (7, 6, Bonus::SpeedPlus),
            (9, 5, Bonus::PaddleMinus),
        ],
    ),
    (
        "level.fortress",
        &[
            "#3333333333#",
            "3..2....2..3",
            "3.111..111.3",
            "3.121..121.3",
            "3.111..111.3",
            "#..........#",
            "222......222",
            "111..##..111",
            "1.1.1..1.1.1",
            ".1.1.11.1.1.",
        ],
        &[
            (2, 8, Bonus::PaddleMinus),
            (3, 3, Bonus::MultiBall),
            (3, 8, Bonus::Wall),
            (6, 1, Bonus::PaddlePlus),
            (8, 4, Bonus::SpeedMinus),
            (9, 6, Bonus::SpeedPlus),
        ],
    ),
];

/// The block grid of a level. Cells tile the build area edge to edge, so a
/// block in an outer column closes the path along the side wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    pub cols: usize,
    pub rows: usize,
}

/// Position of edge `i` of `n` equal parts of `len`, starting at `start`.
/// Computing each edge directly keeps neighbours exactly edge to edge.
fn edge(start: f32, len: f32, i: usize, n: usize) -> f32 {
    start + len * i as f32 / n as f32
}

impl Grid {
    /// Used by built-in levels and by level files that do not specify a grid.
    pub const DEFAULT: Grid = Grid { cols: 12, rows: 24 };

    /// A grid within the allowed limits, or a player-readable error.
    pub fn new(cols: usize, rows: usize) -> Result<Grid, String> {
        if !COLS_RANGE.contains(&cols) {
            return Err(t!(
                "grid.bad_cols",
                min = COLS_RANGE.start(),
                max = COLS_RANGE.end(),
                value = cols
            ));
        }
        if !ROWS_RANGE.contains(&rows) {
            return Err(t!(
                "grid.bad_rows",
                min = ROWS_RANGE.start(),
                max = ROWS_RANGE.end(),
                value = rows
            ));
        }
        Ok(Grid { cols, rows })
    }

    /// Block width in logical pixels: build area width / columns.
    pub fn cell_w(self) -> f32 {
        BUILD_AREA.w / self.cols as f32
    }

    /// Block height in logical pixels: build area height / rows.
    pub fn cell_h(self) -> f32 {
        BUILD_AREA.h / self.rows as f32
    }

    /// A cell's rectangle, used for drawing, mouse placement and collisions.
    pub fn cell_rect(self, row: usize, col: usize) -> Rect {
        let a = BUILD_AREA;
        let (x0, x1) = (
            edge(a.x, a.w, col, self.cols),
            edge(a.x, a.w, col + 1, self.cols),
        );
        let (y0, y1) = (
            edge(a.y, a.h, row, self.rows),
            edge(a.y, a.h, row + 1, self.rows),
        );
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }

    /// The cell under a field position, if it is inside the build area.
    pub fn cell_at(self, pos: Vec2) -> Option<(usize, usize)> {
        let a = BUILD_AREA;
        if pos.x < a.x || pos.y < a.y || pos.x >= a.right() || pos.y >= a.bottom() {
            return None;
        }
        let col = ((pos.x - a.x) / a.w * self.cols as f32) as usize;
        let row = ((pos.y - a.y) / a.h * self.rows as f32) as usize;
        Some((row.min(self.rows - 1), col.min(self.cols - 1)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub name: String,
    grid: Grid,
    /// Row-major, `grid.rows * grid.cols` cells.
    cells: Vec<Cell>,
}

impl Layout {
    pub fn empty(name: &str) -> Self {
        Layout::with_grid(name, Grid::DEFAULT)
    }

    pub fn with_grid(name: &str, grid: Grid) -> Self {
        Layout {
            name: name.to_owned(),
            grid,
            cells: vec![Cell::default(); grid.cols * grid.rows],
        }
    }

    pub fn grid(&self) -> Grid {
        self.grid
    }

    fn index(&self, row: usize, col: usize) -> Option<usize> {
        (row < self.grid.rows && col < self.grid.cols).then(|| row * self.grid.cols + col)
    }

    /// Parses text rows for the default grid.
    pub fn from_rows<S: AsRef<str>>(name: &str, rows: &[S]) -> Result<Self, String> {
        Layout::from_rows_in(name, Grid::DEFAULT, rows)
    }

    /// Parses text rows. Fewer rows than the grid has is fine (the rest are
    /// empty). Error messages are meant to be shown to the player.
    pub fn from_rows_in<S: AsRef<str>>(name: &str, grid: Grid, rows: &[S]) -> Result<Self, String> {
        if rows.len() > grid.rows {
            return Err(t!("rows.too_many", count = rows.len(), rows = grid.rows));
        }
        let mut layout = Layout::with_grid(name, grid);
        for (r, line) in rows.iter().enumerate() {
            let line = line.as_ref();
            let len = line.chars().count();
            if len != grid.cols {
                return Err(t!(
                    "rows.wrong_length",
                    row = r + 1,
                    len = len,
                    cols = grid.cols
                ));
            }
            for (c, ch) in line.chars().enumerate() {
                let block = match ch {
                    '.' => Block::Empty,
                    '1'..='3' => Block::Brick(ch as u8 - b'0'),
                    '#' => Block::Stone,
                    _ => return Err(t!("rows.bad_char", row = r + 1, col = c + 1, ch = ch)),
                };
                layout.cells[r * grid.cols + c].block = block;
            }
        }
        Ok(layout)
    }

    /// Puts a bonus on a breakable brick (0-based position), with a
    /// player-readable error (1-based) if that is not possible.
    pub fn add_bonus(&mut self, row: usize, col: usize, bonus: Bonus) -> Result<(), String> {
        let place = t!("bonus.place", row = row + 1, col = col + 1);
        let Some(i) = self.index(row, col) else {
            return Err(t!("bonus.outside", place = place));
        };
        let cell = &mut self.cells[i];
        if !matches!(cell.block, Block::Brick(_)) {
            return Err(t!("bonus.not_brick", place = place));
        }
        if cell.bonus.is_some() {
            return Err(t!("bonus.taken", place = place));
        }
        cell.bonus = Some(bonus);
        Ok(())
    }

    fn rows_iter(&self) -> impl Iterator<Item = &[Cell]> {
        self.cells.chunks(self.grid.cols)
    }

    /// Text rows, without trailing empty rows.
    pub fn to_rows(&self) -> Vec<String> {
        let rows: Vec<&[Cell]> = self.rows_iter().collect();
        let used = rows
            .iter()
            .rposition(|row| row.iter().any(|c| c.block != Block::Empty))
            .map_or(0, |i| i + 1);
        rows[..used]
            .iter()
            .map(|row| {
                row.iter()
                    .map(|c| match c.block {
                        Block::Empty => '.',
                        Block::Brick(hp) => (b'0' + hp) as char,
                        Block::Stone => '#',
                    })
                    .collect()
            })
            .collect()
    }

    /// All bonuses as (row, col, bonus), 0-based, in reading order.
    pub fn bonuses(&self) -> Vec<(usize, usize, Bonus)> {
        let mut out = Vec::new();
        for (r, row) in self.rows_iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if let Some(b) = cell.bonus {
                    out.push((r, c, b));
                }
            }
        }
        out
    }

    #[cfg(test)]
    pub fn cell(&self, row: usize, col: usize) -> Cell {
        self.cells[self.index(row, col).expect("cell inside the grid")]
    }

    /// Sets a cell's block; returns true if anything changed. A bonus stays
    /// on a brick but is removed with the brick or when it becomes a stone.
    pub fn set_block(&mut self, row: usize, col: usize, block: Block) -> bool {
        let Some(i) = self.index(row, col) else {
            return false;
        };
        let block = match block {
            Block::Brick(hp) => Block::Brick(hp.clamp(1, MAX_HP)),
            other => other,
        };
        let cell = &mut self.cells[i];
        let before = *cell;
        cell.block = block;
        if !matches!(block, Block::Brick(_)) {
            cell.bonus = None;
        }
        *cell != before
    }

    /// Sets or clears a bonus; only breakable bricks can have one.
    /// Returns true if anything changed.
    pub fn set_bonus(&mut self, row: usize, col: usize, bonus: Option<Bonus>) -> bool {
        let Some(i) = self.index(row, col) else {
            return false;
        };
        let cell = &mut self.cells[i];
        if !matches!(cell.block, Block::Brick(_)) || cell.bonus == bonus {
            return false;
        }
        cell.bonus = bonus;
        true
    }

    /// Number of breakable bricks (stones do not count).
    pub fn brick_count(&self) -> usize {
        self.cells
            .iter()
            .filter(|c| matches!(c.block, Block::Brick(_)))
            .count()
    }

    /// Blocks (bricks and stones) that would fall outside `grid`.
    pub fn blocks_outside(&self, grid: Grid) -> usize {
        self.rows_iter()
            .enumerate()
            .flat_map(|(r, row)| row.iter().enumerate().map(move |(c, cell)| (r, c, cell)))
            .filter(|(r, c, cell)| {
                cell.block != Block::Empty && (*r >= grid.rows || *c >= grid.cols)
            })
            .count()
    }

    /// The same layout on another grid. Blocks keep their row and column
    /// (with type and bonus); those outside the new grid are dropped.
    pub fn resized(&self, grid: Grid) -> Layout {
        let mut out = Layout::with_grid(&self.name, grid);
        for (r, row) in self.rows_iter().enumerate().take(grid.rows) {
            for (c, &cell) in row.iter().enumerate().take(grid.cols) {
                out.cells[r * grid.cols + c] = cell;
            }
        }
        out
    }

    pub fn bricks(&self) -> Vec<Brick> {
        let mut bricks = Vec::new();
        for (row, cells) in self.rows_iter().enumerate() {
            for (col, cell) in cells.iter().enumerate() {
                let (hp, stone) = match cell.block {
                    Block::Empty => continue,
                    Block::Brick(hp) => (hp, false),
                    Block::Stone => (1, true),
                };
                bricks.push(Brick {
                    rect: self.grid.cell_rect(row, col),
                    hp,
                    max_hp: hp,
                    row,
                    stone,
                    bonus: cell.bonus,
                });
            }
        }
        bricks
    }
}

/// The built-in levels, in play order, named in the current language.
/// They live in code, never on disk, so they cannot be overwritten.
pub fn builtin() -> Vec<Layout> {
    BUILTIN
        .iter()
        .map(|(name_key, rows, bonuses)| {
            let name = crate::i18n::tr(name_key);
            let mut layout = Layout::from_rows(name, rows).expect("built-in level is valid");
            for &(r, c, b) in *bonuses {
                layout.add_bonus(r, c, b).expect("built-in bonus is valid");
            }
            layout
        })
        .collect()
}

/// True if `name` is a built-in level's name in any language.
pub fn is_builtin_name(name: &str) -> bool {
    let name = name.trim().to_lowercase();
    BUILTIN.iter().any(|(key, _, _)| {
        Lang::ALL
            .into_iter()
            .any(|lang| tr_in(lang, key).to_lowercase() == name)
    })
}

/// Keep the build area well above the paddle and the ball's start position.
const _: () = assert!(BUILD_AREA.y + BUILD_AREA.h < PADDLE_Y - 100.0);
const _: () = assert!(BUILD_AREA.x == 0.0 && BUILD_AREA.w == FIELD_W);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_levels_are_valid_and_inside_field() {
        let levels = builtin();
        assert_eq!(levels.len(), 3);
        for level in &levels {
            assert!(level.brick_count() > 0);
            assert_eq!(level.bonuses().len(), 6, "{}", level.name);
            for b in level.bricks() {
                assert!(b.rect.x >= 0.0 && b.rect.right() <= FIELD_W);
                assert!(b.rect.bottom() < PADDLE_Y - 100.0);
            }
        }
        // Every bonus type appears in every built-in level.
        for level in &levels {
            for kind in Bonus::ALL {
                assert!(level.bonuses().iter().any(|&(_, _, b)| b == kind));
            }
        }
        assert!(
            is_builtin_name(" diamanten "),
            "Swedish names are protected too"
        );
        assert!(is_builtin_name("the DIAMOND"));
        assert_eq!(levels[1].name, "The Diamond");
        assert!(!is_builtin_name("Min bana"));
    }

    #[test]
    fn rows_round_trip_and_trim_trailing_empty_rows() {
        let rows = [
            "1.2.3...#...",
            "............",
            "...3........",
            "............",
        ];
        let layout = Layout::from_rows("x", &rows).unwrap();
        assert_eq!(layout.to_rows(), &rows[..3]);
        assert_eq!(layout.cell(0, 2).block, Block::Brick(2));
        assert_eq!(layout.cell(0, 8).block, Block::Stone);
        assert_eq!(layout.brick_count(), 4, "stones are not counted");
        assert_eq!(Layout::empty("e").to_rows(), Vec::<String>::new());
    }

    #[test]
    fn invalid_rows_give_clear_errors() {
        let err = Layout::from_rows("x", &["111"]).unwrap_err();
        assert!(err.contains("Row 1") && err.contains("12"), "{err}");
        let err = Layout::from_rows("x", &["............", "....x......."]).unwrap_err();
        assert!(err.contains("Row 2, column 5"), "{err}");
        let err = Layout::from_rows("x", &["4..........."]).unwrap_err();
        assert!(err.contains("'4'"), "{err}");
        let too_many = vec!["............"; Grid::DEFAULT.rows + 1];
        assert!(Layout::from_rows("x", &too_many).is_err());
    }

    #[test]
    fn bonuses_only_on_breakable_bricks() {
        let mut l = Layout::from_rows("x", &["1#.........."]).unwrap();
        assert!(l.add_bonus(0, 0, Bonus::Wall).is_ok());
        assert!(l
            .add_bonus(0, 0, Bonus::Wall)
            .unwrap_err()
            .contains("already"));
        assert!(l
            .add_bonus(0, 1, Bonus::Wall)
            .unwrap_err()
            .contains("no breakable"));
        assert!(l.add_bonus(0, 2, Bonus::Wall).is_err());
        assert!(l
            .add_bonus(Grid::DEFAULT.rows, 0, Bonus::Wall)
            .unwrap_err()
            .contains("outside"));
        assert!(
            !l.set_bonus(0, 1, Some(Bonus::MultiBall)),
            "stone takes no bonus"
        );
        // Turning the brick into a stone or erasing it drops the bonus.
        assert!(l.set_block(0, 0, Block::Brick(3)));
        assert_eq!(l.cell(0, 0).bonus, Some(Bonus::Wall));
        assert!(l.set_block(0, 0, Block::Stone));
        assert_eq!(l.cell(0, 0).bonus, None);
        for b in Bonus::ALL {
            assert_eq!(Bonus::from_key(b.key()), Some(b));
        }
    }

    /// Grids of many proportions, including ones that do not divide evenly.
    const GRIDS: [(usize, usize); 8] = [
        (4, 8),
        (4, 36),
        (20, 8),
        (20, 36),
        (12, 24),
        (7, 13),
        (13, 17),
        (9, 30),
    ];

    #[test]
    fn grid_fills_the_build_area_exactly() {
        let a = BUILD_AREA;
        for (cols, rows) in GRIDS {
            let g = Grid::new(cols, rows).unwrap();
            assert!((g.cell_w() - a.w / cols as f32).abs() < 1e-4);
            assert!((g.cell_h() - a.h / rows as f32).abs() < 1e-4);
            // Outer edges match the build area exactly.
            assert_eq!(g.cell_rect(0, 0).x, a.x);
            assert_eq!(g.cell_rect(0, 0).y, a.y);
            assert!((g.cell_rect(rows - 1, cols - 1).right() - a.right()).abs() < 1e-3);
            assert!((g.cell_rect(rows - 1, cols - 1).bottom() - a.bottom()).abs() < 1e-3);
            // Neighbours share edges: no strips, no overlaps.
            for c in 1..cols {
                let (l, r) = (g.cell_rect(rows / 2, c - 1), g.cell_rect(rows / 2, c));
                assert!((l.right() - r.x).abs() < 1e-3, "{cols}x{rows} col {c}");
            }
            for r in 1..rows {
                let (t, b) = (g.cell_rect(r - 1, cols / 2), g.cell_rect(r, cols / 2));
                assert!((t.bottom() - b.y).abs() < 1e-3, "{cols}x{rows} row {r}");
            }
            let total: f32 = (0..cols).map(|c| g.cell_rect(0, c).w).sum();
            assert!((total - a.w).abs() < 1e-2);
        }
        assert_eq!(Grid::DEFAULT.cell_w(), 45.0);
        assert_eq!(Grid::DEFAULT.cell_h(), 24.0);
    }

    #[test]
    fn grid_limits_are_validated() {
        assert!(Grid::new(3, 24).unwrap_err().contains("Columns"));
        assert!(Grid::new(21, 24).unwrap_err().contains("Columns"));
        assert!(Grid::new(12, 7).unwrap_err().contains("Rows"));
        assert!(Grid::new(12, 37).unwrap_err().contains("Rows"));
        // Even the smallest cells are larger than the ball.
        let tiny = Grid::new(*COLS_RANGE.end(), *ROWS_RANGE.end()).unwrap();
        assert!(tiny.cell_w() > 2.0 * crate::game::BALL_R);
        assert!(tiny.cell_h() > 2.0 * crate::game::BALL_R);
    }

    #[test]
    fn cell_lookup_matches_cell_rects() {
        for (cols, rows) in GRIDS {
            let g = Grid::new(cols, rows).unwrap();
            for (row, col) in [(0, 0), (rows / 2, cols / 3), (rows - 1, cols - 1)] {
                let r = g.cell_rect(row, col);
                assert_eq!(g.cell_at(r.center()), Some((row, col)));
                assert_eq!(g.cell_at(r.point() + Vec2::splat(0.01)), Some((row, col)));
                let inner = r.point() + r.size() - Vec2::splat(0.01);
                assert_eq!(g.cell_at(inner), Some((row, col)));
            }
            let a = BUILD_AREA;
            assert_eq!(g.cell_at(Vec2::new(a.x + 5.0, a.y - 1.0)), None);
            assert_eq!(g.cell_at(Vec2::new(a.x + 5.0, a.bottom() + 1.0)), None);
            assert_eq!(g.cell_at(Vec2::new(-1.0, a.y + 5.0)), None);
            assert_eq!(g.cell_at(Vec2::new(FIELD_W + 1.0, a.y + 5.0)), None);
            assert_eq!(
                g.cell_at(Vec2::new(270.0, PADDLE_Y)),
                None,
                "never near the paddle"
            );
        }
    }

    #[test]
    fn resizing_keeps_positions_and_properties() {
        let mut l =
            Layout::from_rows("x", &["1#.........3", "............", "2..........."]).unwrap();
        l.add_bonus(0, 0, Bonus::MultiBall).unwrap();
        l.add_bonus(2, 0, Bonus::Wall).unwrap();
        let small = Grid::new(6, 8).unwrap();
        assert_eq!(l.blocks_outside(small), 1, "the 3 in column 12");
        let r = l.resized(small);
        assert_eq!(r.grid(), small);
        assert_eq!(r.cell(0, 0).block, Block::Brick(1));
        assert_eq!(r.cell(0, 0).bonus, Some(Bonus::MultiBall));
        assert_eq!(r.cell(0, 1).block, Block::Stone);
        assert_eq!(r.cell(2, 0).bonus, Some(Bonus::Wall));
        assert_eq!(r.to_rows(), ["1#....", "......", "2....."]);
        // Growing again keeps everything that was left.
        let big = r.resized(Grid::new(20, 36).unwrap());
        assert_eq!(big.blocks_outside(small), 0);
        assert_eq!(big.bonuses(), r.bonuses());
        assert_eq!(big.cell(0, 1).block, Block::Stone);
        assert_eq!(l.resized(l.grid()), l);
        // Painting outside the grid is ignored rather than panicking.
        let mut s = r.clone();
        assert!(!s.set_block(0, 6, Block::Stone));
        assert!(!s.set_bonus(8, 0, Some(Bonus::Wall)));
    }

    #[test]
    fn parsing_uses_the_given_grid() {
        let g = Grid::new(5, 10).unwrap();
        let l = Layout::from_rows_in("x", g, &["1.2.#", "....3"]).unwrap();
        assert_eq!(l.grid(), g);
        assert_eq!(l.bricks().len(), 4);
        assert_eq!(l.bricks()[0].rect, g.cell_rect(0, 0));
        let err = Layout::from_rows_in("x", g, &["1..........."]).unwrap_err();
        assert!(err.contains("should have 5"), "{err}");
    }
}
