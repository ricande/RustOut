//! The level editor's logic. It works on abstract `Input` in field
//! coordinates and never touches the window, so it can be tested headlessly.

use std::path::PathBuf;

use macroquad::math::{Rect, Vec2};

use crate::levels::{
    self, Block, Bonus, Grid, Layout, COLS_RANGE, MAX_HP, MAX_NAME_LEN, ROWS_RANGE,
};
use crate::store::{self, LevelStore};
use crate::t;
use crate::ui::{
    button_row, clicked_button, Button, Input, Key, ListItem, ListResponse, ListState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    New,
    Rename,
    Grid,
    Save,
    Open,
    Test,
    Menu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogButton {
    Ok,
    Cancel,
    ColsMinus,
    ColsPlus,
    RowsMinus,
    RowsPlus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Builtin(usize),
    File(PathBuf),
}

/// An action waiting for the player to confirm it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pending {
    New,
    Open(Source),
    Overwrite,
    Leave,
    /// Change the grid, which removes blocks outside it.
    Resize(Grid),
}

pub enum Modal {
    None,
    Naming {
        text: String,
    },
    Browse {
        list: ListState<Source>,
    },
    Confirm {
        text: String,
        action: Pending,
    },
    Message {
        text: String,
        error: bool,
    },
    /// Choosing a new grid size; nothing changes until OK.
    Grid {
        cols: usize,
        rows: usize,
    },
}

/// What the editor asks the rest of the app to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Leave,
    TestPlay,
}

/// What the left button paints. The right button erases: the whole block,
/// or with a bonus brush only the bonus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Brush {
    Block(Block),
    Bonus(Bonus),
}

impl Brush {
    pub fn name(self) -> String {
        match self {
            Brush::Block(Block::Brick(hp)) => {
                if hp == 1 {
                    t!("brush.brick_one", hits = hp)
                } else {
                    t!("brush.brick_many", hits = hp)
                }
            }
            Brush::Block(Block::Stone) => t!("brush.stone").into(),
            Brush::Block(Block::Empty) => t!("brush.eraser").into(),
            Brush::Bonus(b) => t!("brush.bonus", name = b.name()),
        }
    }
}

struct Stroke {
    erase: bool,
    last: Vec2,
}

pub struct Editor {
    pub layout: Layout,
    /// What the left button places.
    pub brush: Brush,
    /// True if there are changes that are not saved to a file.
    pub dirty: bool,
    /// The file this layout was opened from or last saved to.
    pub file: Option<PathBuf>,
    pub modal: Modal,
    stroke: Option<Stroke>,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

/// Brush buttons: block types above the grid, bonuses below it.
pub fn palette() -> Vec<(Brush, Rect)> {
    let blocks = (1..=MAX_HP)
        .map(Block::Brick)
        .chain([Block::Stone])
        .enumerate()
        .map(|(i, b)| {
            let rect = Rect::new(96.0 + i as f32 * 84.0, 78.0, 76.0, 36.0);
            (Brush::Block(b), rect)
        });
    let bonuses = Bonus::ALL.into_iter().enumerate().map(|(i, b)| {
        let rect = Rect::new(96.0 + i as f32 * 72.0, 712.0, 64.0, 30.0);
        (Brush::Bonus(b), rect)
    });
    blocks.chain(bonuses).collect()
}

pub fn toolbar() -> Vec<Button<Tool>> {
    let mut buttons = button_row(
        Rect::new(20.0, 772.0, 500.0, 44.0),
        &[
            (t!("tool.new"), Tool::New),
            (t!("tool.name"), Tool::Rename),
            (t!("tool.grid"), Tool::Grid),
            (t!("common.save"), Tool::Save),
        ],
    );
    buttons.extend(button_row(
        Rect::new(20.0, 824.0, 500.0, 44.0),
        &[
            (t!("tool.open"), Tool::Open),
            (t!("tool.test"), Tool::Test),
            (t!("tool.menu"), Tool::Menu),
        ],
    ));
    buttons
}

pub const PANEL: Rect = Rect {
    x: 30.0,
    y: 240.0,
    w: 480.0,
    h: 470.0,
};

pub fn dialog_buttons(modal: &Modal) -> Vec<Button<DialogButton>> {
    let area = Rect::new(PANEL.x + 20.0, PANEL.bottom() - 70.0, PANEL.w - 40.0, 50.0);
    let (ok, cancel) = (DialogButton::Ok, DialogButton::Cancel);
    let items: Vec<(&'static str, DialogButton)> = match modal {
        Modal::None => vec![],
        Modal::Message { .. } => vec![(t!("common.ok"), ok)],
        Modal::Confirm { .. } => vec![(t!("common.yes"), ok), (t!("common.no"), cancel)],
        Modal::Naming { .. } | Modal::Grid { .. } => {
            vec![(t!("common.ok"), ok), (t!("common.cancel"), cancel)]
        }
        Modal::Browse { .. } => vec![(t!("tool.open"), ok), (t!("common.cancel"), cancel)],
    };
    let mut buttons = button_row(area, &items);
    if let Modal::Grid { .. } = modal {
        for (i, (minus, plus)) in [
            (DialogButton::ColsMinus, DialogButton::ColsPlus),
            (DialogButton::RowsMinus, DialogButton::RowsPlus),
        ]
        .into_iter()
        .enumerate()
        {
            let y = GRID_ROWS_Y + i as f32 * 70.0 - 34.0;
            buttons.push(Button {
                rect: Rect::new(PANEL.x + 290.0, y, 70.0, 48.0),
                label: "-",
                action: minus,
            });
            buttons.push(Button {
                rect: Rect::new(PANEL.x + 375.0, y, 70.0, 48.0),
                label: "+",
                action: plus,
            });
        }
    }
    buttons
}

/// Baseline of the "Columns" line in the grid dialog; "Rows" is 70 below.
pub const GRID_ROWS_Y: f32 = PANEL.y + 140.0;

fn browse_area() -> Rect {
    Rect::new(
        PANEL.x + 20.0,
        PANEL.y + 70.0,
        PANEL.w - 40.0,
        8.0 * crate::ui::LIST_ROW_H,
    )
}

impl Editor {
    pub fn new() -> Self {
        Editor {
            layout: Layout::empty(t!("editor.new_name")),
            brush: Brush::Block(Block::Brick(1)),
            dirty: false,
            file: None,
            modal: Modal::None,
            stroke: None,
        }
    }

    pub fn handle(&mut self, input: &Input, store: &LevelStore) -> Option<Outcome> {
        if matches!(self.modal, Modal::None) {
            self.handle_editing(input, store)
        } else {
            self.handle_modal(input, store)
        }
    }

    fn handle_editing(&mut self, input: &Input, store: &LevelStore) -> Option<Outcome> {
        let brush_keys = [
            (Key::Num1, Block::Brick(1)),
            (Key::Num2, Block::Brick(2)),
            (Key::Num3, Block::Brick(3)),
            (Key::Num4, Block::Stone),
        ];
        for (key, block) in brush_keys {
            if input.key(key) {
                self.brush = Brush::Block(block);
            }
        }
        let shortcuts = [
            (Key::N, Tool::New),
            (Key::F2, Tool::Rename),
            (Key::G, Tool::Grid),
            (Key::S, Tool::Save),
            (Key::O, Tool::Open),
            (Key::T, Tool::Test),
            (Key::Escape, Tool::Menu),
        ];
        let tool = shortcuts
            .iter()
            .find(|(k, _)| input.key(*k))
            .map(|&(_, t)| t)
            .or_else(|| clicked_button(&toolbar(), input));
        if let Some(tool) = tool {
            self.stroke = None;
            return self.use_tool(tool, store);
        }
        if let Some(&(brush, _)) = palette().iter().find(|(_, r)| input.clicked(*r)) {
            self.brush = brush;
        }
        self.handle_painting(input);
        None
    }

    fn handle_painting(&mut self, input: &Input) {
        let Some(p) = input.pointer else {
            self.stroke = None;
            return;
        };
        if input.left_pressed || input.right_pressed {
            self.stroke = self.layout.grid().cell_at(p).map(|_| Stroke {
                erase: !input.left_pressed,
                last: p,
            });
        }
        let Some(stroke) = &self.stroke else { return };
        let held = if stroke.erase {
            input.right_down
        } else {
            input.left_down
        };
        if !held {
            self.stroke = None;
            return;
        }
        let (from, erase) = (stroke.last, stroke.erase);
        self.paint_line(from, p, erase);
        if let Some(stroke) = &mut self.stroke {
            stroke.last = p;
        }
    }

    /// Paints every cell along the segment, so fast drags leave no holes.
    fn paint_line(&mut self, from: Vec2, to: Vec2, erase: bool) {
        let delta = to - from;
        let steps = (delta.length() / 6.0).ceil().max(1.0) as usize;
        for i in 0..=steps {
            let grid = self.layout.grid();
            if let Some((row, col)) = grid.cell_at(from + delta * (i as f32 / steps as f32)) {
                let changed = match (self.brush, erase) {
                    (Brush::Block(_), true) => self.layout.set_block(row, col, Block::Empty),
                    (Brush::Block(block), false) => self.layout.set_block(row, col, block),
                    (Brush::Bonus(_), true) => self.layout.set_bonus(row, col, None),
                    (Brush::Bonus(bonus), false) => self.layout.set_bonus(row, col, Some(bonus)),
                };
                self.dirty |= changed;
            }
        }
    }

    fn use_tool(&mut self, tool: Tool, store: &LevelStore) -> Option<Outcome> {
        match tool {
            Tool::New => return self.guard(Pending::New, t!("editor.ask_new"), store),
            Tool::Rename => {
                self.modal = Modal::Naming {
                    text: self.layout.name.clone(),
                }
            }
            Tool::Grid => {
                let grid = self.layout.grid();
                self.modal = Modal::Grid {
                    cols: grid.cols,
                    rows: grid.rows,
                }
            }
            Tool::Save => self.save(store, false),
            Tool::Open => self.open_browser(store),
            Tool::Test => {
                if let Err(e) = store::validate(&self.layout) {
                    self.error(t!("editor.cannot_test", error = e));
                } else {
                    return Some(Outcome::TestPlay);
                }
            }
            Tool::Menu => {
                if !self.dirty {
                    return Some(Outcome::Leave);
                }
                return self.guard(Pending::Leave, t!("editor.ask_menu"), store);
            }
        }
        None
    }

    /// Runs `action` directly, or asks first if unsaved changes would be lost.
    fn guard(&mut self, action: Pending, question: &str, store: &LevelStore) -> Option<Outcome> {
        if self.dirty {
            self.modal = Modal::Confirm {
                text: t!("editor.unsaved_lost", question = question),
                action,
            };
            None
        } else {
            self.run(action, store)
        }
    }

    fn run(&mut self, action: Pending, store: &LevelStore) -> Option<Outcome> {
        self.modal = Modal::None;
        match action {
            Pending::New => {
                *self = Editor {
                    brush: self.brush,
                    ..Editor::new()
                }
            }
            Pending::Open(source) => self.load(source),
            Pending::Overwrite => self.save(store, true),
            Pending::Resize(grid) => self.resize(grid),
            Pending::Leave => {
                *self = Editor {
                    brush: self.brush,
                    ..Editor::new()
                };
                return Some(Outcome::Leave);
            }
        }
        None
    }

    fn error(&mut self, text: String) {
        self.modal = Modal::Message { text, error: true };
    }

    fn save(&mut self, store: &LevelStore, overwrite: bool) {
        if let Err(e) = store::validate(&self.layout) {
            return self.error(t!("editor.cannot_save", error = e));
        }
        let path = store.path_for(&self.layout.name);
        let is_own_file = self.file.as_ref() == Some(&path);
        if !overwrite && !is_own_file && path.exists() {
            self.modal = Modal::Confirm {
                text: t!("common.file_exists", path = path.display()),
                action: Pending::Overwrite,
            };
            return;
        }
        match store.save(&self.layout, &path) {
            Ok(()) => {
                self.modal = Modal::Message {
                    text: t!("editor.saved_to", path = path.display()),
                    error: false,
                };
                self.file = Some(path);
                self.dirty = false;
            }
            Err(e) => self.error(t!("editor.save_failed", error = e)),
        }
    }

    fn open_browser(&mut self, store: &LevelStore) {
        let mut items: Vec<ListItem<Source>> = levels::builtin()
            .into_iter()
            .enumerate()
            .map(|(i, l)| ListItem {
                label: t!("list.builtin", name = l.name),
                value: Source::Builtin(i),
                error: None,
            })
            .collect();
        match store.list() {
            Ok(stored) => items.extend(stored.into_iter().map(|s| {
                let file_name = s.path.file_name().unwrap_or_default().to_string_lossy();
                let (label, error) = match s.level {
                    Ok(l) => (l.name, None),
                    Err(e) => (t!("list.broken", file = file_name), Some(e)),
                };
                ListItem {
                    label,
                    value: Source::File(s.path),
                    error,
                }
            })),
            Err(e) => return self.error(e),
        }
        self.modal = Modal::Browse {
            list: ListState::new(items, browse_area()),
        };
    }

    fn load(&mut self, source: Source) {
        let brush = self.brush;
        match source {
            Source::Builtin(i) => {
                let mut layout = levels::builtin().swap_remove(i);
                // A copy: the built-in level itself can never be overwritten.
                layout.name = t!("editor.copy_name", name = layout.name);
                layout.name.truncate(MAX_NAME_LEN);
                *self = Editor {
                    layout,
                    brush,
                    ..Editor::new()
                };
            }
            Source::File(path) => match store::load(&path) {
                Ok(layout) => {
                    *self = Editor {
                        layout,
                        brush,
                        file: Some(path),
                        ..Editor::new()
                    }
                }
                Err(e) => self.error(t!("editor.open_failed", path = path.display(), error = e)),
            },
        }
    }

    /// Puts the layout on a new grid, keeping blocks at their row and column.
    fn resize(&mut self, grid: Grid) {
        if grid != self.layout.grid() {
            self.layout = self.layout.resized(grid);
            self.dirty = true;
        }
        self.stroke = None;
    }

    fn handle_modal(&mut self, input: &Input, store: &LevelStore) -> Option<Outcome> {
        let button = clicked_button(&dialog_buttons(&self.modal), input);
        let ok = button == Some(DialogButton::Ok) || input.key(Key::Enter);
        let cancel = button == Some(DialogButton::Cancel) || input.key(Key::Escape);

        match &mut self.modal {
            Modal::None => {}
            Modal::Grid { cols, rows } => {
                let step = |v: &mut usize, d: isize, range: &std::ops::RangeInclusive<usize>| {
                    *v = v
                        .saturating_add_signed(d)
                        .clamp(*range.start(), *range.end());
                };
                let pressed = |b: DialogButton, k: Key| button == Some(b) || input.key(k);
                if pressed(DialogButton::ColsMinus, Key::Left) {
                    step(cols, -1, &COLS_RANGE);
                }
                if pressed(DialogButton::ColsPlus, Key::Right) {
                    step(cols, 1, &COLS_RANGE);
                }
                if pressed(DialogButton::RowsMinus, Key::Down) {
                    step(rows, -1, &ROWS_RANGE);
                }
                if pressed(DialogButton::RowsPlus, Key::Up) {
                    step(rows, 1, &ROWS_RANGE);
                }
                if cancel {
                    self.modal = Modal::None;
                } else if ok {
                    let grid = match Grid::new(*cols, *rows) {
                        Ok(grid) => grid,
                        Err(e) => {
                            self.error(e);
                            return None;
                        }
                    };
                    let lost = self.layout.blocks_outside(grid);
                    if lost > 0 {
                        // Ask first; "No" leaves the level exactly as it was.
                        self.modal = Modal::Confirm {
                            text: t!(
                                "editor.grid_loses",
                                cols = grid.cols,
                                rows = grid.rows,
                                lost = lost
                            ),
                            action: Pending::Resize(grid),
                        };
                    } else {
                        self.modal = Modal::None;
                        self.resize(grid);
                    }
                }
            }
            Modal::Message { .. } => {
                if ok || cancel {
                    self.modal = Modal::None;
                }
            }
            Modal::Confirm { action, .. } => {
                if ok {
                    let action = action.clone();
                    return self.run(action, store);
                }
                if cancel {
                    self.modal = Modal::None;
                }
            }
            Modal::Naming { text } => {
                for &ch in &input.chars {
                    if !ch.is_control() && text.chars().count() < MAX_NAME_LEN {
                        text.push(ch);
                    }
                }
                if input.key(Key::Backspace) {
                    text.pop();
                }
                if cancel {
                    self.modal = Modal::None;
                } else if ok {
                    let name = text.trim().to_owned();
                    match store::validate_name(&name) {
                        Err(e) => self.error(e),
                        Ok(()) => {
                            if name != self.layout.name {
                                self.layout.name = name;
                                self.dirty = true;
                            }
                            self.modal = Modal::None;
                        }
                    }
                }
            }
            Modal::Browse { list } => {
                let activated = match list.handle(input) {
                    ListResponse::Activate(i) => Some(i),
                    ListResponse::None => ok.then_some(list.selected),
                };
                let chosen = activated
                    .and_then(|i| list.items.get(i))
                    .map(|item| (item.value.clone(), item.error.clone()));
                if cancel {
                    self.modal = Modal::None;
                } else if let Some((source, error)) = chosen {
                    match error {
                        Some(e) => self.error(t!("editor.invalid_file", error = e)),
                        None => {
                            self.modal = Modal::None;
                            return self.guard(Pending::Open(source), t!("editor.ask_open"), store);
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell_rect(row: usize, col: usize) -> Rect {
        Grid::DEFAULT.cell_rect(row, col)
    }
    use crate::store::tests::temp_dir;

    fn key(k: Key) -> Input {
        Input {
            keys: vec![k],
            ..Default::default()
        }
    }

    fn press_left(p: Vec2) -> Input {
        Input {
            pointer: Some(p),
            left_pressed: true,
            left_down: true,
            ..Default::default()
        }
    }

    fn drag_left(p: Vec2) -> Input {
        Input {
            pointer: Some(p),
            left_down: true,
            ..Default::default()
        }
    }

    fn typed(s: &str) -> Input {
        Input {
            chars: s.chars().collect(),
            ..Default::default()
        }
    }

    fn rename(ed: &mut Editor, store: &LevelStore, name: &str) {
        ed.handle(&key(Key::F2), store);
        if let Modal::Naming { text } = &mut ed.modal {
            text.clear();
        }
        ed.handle(&typed(name), store);
        ed.handle(&key(Key::Enter), store);
        assert!(matches!(ed.modal, Modal::None));
    }

    fn message(ed: &Editor) -> (&str, bool) {
        match &ed.modal {
            Modal::Message { text, error } => (text, *error),
            _ => panic!("expected a message"),
        }
    }

    #[test]
    fn click_and_drag_paints_right_drag_erases() {
        let store = LevelStore::new(temp_dir("paint"));
        let mut ed = Editor::new();
        ed.handle(&key(Key::Num2), &store);
        ed.handle(&press_left(cell_rect(0, 0).center()), &store);
        // A fast drag across the whole row must not skip cells.
        ed.handle(&drag_left(cell_rect(0, 11).center()), &store);
        assert!((0..12).all(|c| ed.layout.cell(0, c).block == Block::Brick(2)));
        assert!(ed.dirty);

        ed.handle(&Input::default(), &store); // release
        let erase = Input {
            pointer: Some(cell_rect(0, 3).center()),
            right_pressed: true,
            right_down: true,
            ..Default::default()
        };
        ed.handle(&erase, &store);
        ed.handle(
            &Input {
                pointer: Some(cell_rect(0, 5).center()),
                right_down: true,
                ..Default::default()
            },
            &store,
        );
        assert_eq!(ed.layout.cell(0, 2).block, Block::Brick(2));
        assert!((3..=5).all(|c| ed.layout.cell(0, c).block == Block::Empty));
        assert_eq!(ed.layout.cell(0, 6).block, Block::Brick(2));
    }

    #[test]
    fn painting_is_limited_to_the_build_area() {
        let store = LevelStore::new(temp_dir("limit"));
        let mut ed = Editor::new();
        let area = levels::BUILD_AREA;
        // Starting outside the grid (e.g. near the paddle) paints nothing.
        ed.handle(&press_left(Vec2::new(270.0, 880.0)), &store);
        ed.handle(&drag_left(Vec2::new(270.0, 860.0)), &store);
        assert_eq!(ed.layout.brick_count(), 0);
        // Dragging from the grid down past its edge stops at the last row.
        ed.handle(
            &press_left(Vec2::new(area.x + 5.0, area.bottom() - 5.0)),
            &store,
        );
        ed.handle(&drag_left(Vec2::new(area.x + 5.0, 900.0)), &store);
        assert_eq!(ed.layout.brick_count(), 1);
        // Pointer outside the window view ends the stroke.
        ed.handle(
            &Input {
                left_down: true,
                ..Default::default()
            },
            &store,
        );
        assert!(ed.stroke.is_none());
    }

    #[test]
    fn palette_click_selects_brush() {
        let store = LevelStore::new(temp_dir("palette"));
        let mut ed = Editor::new();
        for (brush, rect) in palette() {
            ed.handle(&press_left(rect.center()), &store);
            ed.handle(&Input::default(), &store);
            assert_eq!(ed.brush, brush);
        }
        assert_eq!(ed.brush, Brush::Bonus(Bonus::Wall));
        assert_eq!(palette().len(), 4 + 6, "1-3 hits, stone and six bonuses");
        assert_eq!(ed.layout, Layout::empty("New level"), "nothing painted");
        // No palette button overlaps the grid or the toolbar.
        for (_, r) in palette() {
            assert!(!r.overlaps(&levels::BUILD_AREA));
            assert!(toolbar().iter().all(|b| !b.rect.overlaps(&r)));
        }
    }

    #[test]
    fn stones_and_bonuses_are_painted() {
        let dir = temp_dir("editor-bonus");
        let store = LevelStore::new(dir.clone());
        let mut ed = Editor::new();
        let click = |ed: &mut Editor, r: usize, c: usize, right: bool| {
            let p = cell_rect(r, c).center();
            let input = Input {
                pointer: Some(p),
                left_pressed: !right,
                left_down: !right,
                right_pressed: right,
                right_down: right,
                ..Default::default()
            };
            ed.handle(&input, &store);
            ed.handle(&Input::default(), &store);
        };
        click(&mut ed, 0, 0, false);
        click(&mut ed, 0, 1, false);
        ed.handle(&key(Key::Num4), &store);
        click(&mut ed, 0, 2, false);
        assert_eq!(ed.layout.cell(0, 2).block, Block::Stone);

        ed.brush = Brush::Bonus(Bonus::MultiBall);
        click(&mut ed, 0, 0, false);
        click(&mut ed, 0, 2, false); // stone: no bonus
        click(&mut ed, 0, 5, false); // empty: no bonus
        assert_eq!(ed.layout.bonuses(), vec![(0, 0, Bonus::MultiBall)]);
        ed.brush = Brush::Bonus(Bonus::Wall);
        click(&mut ed, 0, 1, false);
        click(&mut ed, 0, 0, true); // right click removes only the bonus
        assert_eq!(ed.layout.cell(0, 0).block, Block::Brick(1));
        assert_eq!(ed.layout.bonuses(), vec![(0, 1, Bonus::Wall)]);

        // Saved and reopened, everything is still there.
        rename(&mut ed, &store, "Bonusbana");
        ed.handle(&key(Key::S), &store);
        assert!(!message(&ed).1);
        let saved = ed.layout.clone();
        assert_eq!(store::load(&dir.join("bonusbana.json")).unwrap(), saved);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn save_reopen_and_overwrite_warning() {
        let dir = temp_dir("editor-save");
        let store = LevelStore::new(dir.clone());
        let mut ed = Editor::new();

        ed.handle(&key(Key::S), &store);
        assert!(message(&ed).1, "empty level cannot be saved");
        ed.handle(&key(Key::Enter), &store);

        ed.handle(&press_left(cell_rect(3, 4).center()), &store);
        rename(&mut ed, &store, "Min bana");
        ed.handle(&key(Key::S), &store);
        let (text, error) = message(&ed);
        assert!(!error && text.contains("min-bana.json"), "{text}");
        assert!(!ed.dirty);
        ed.handle(&key(Key::Enter), &store);

        // Saving the same file again needs no confirmation.
        ed.handle(&press_left(cell_rect(3, 5).center()), &store);
        ed.handle(&key(Key::S), &store);
        assert!(!message(&ed).1);
        ed.handle(&key(Key::Enter), &store);

        // A new level with the same name would overwrite: ask first.
        ed.handle(&key(Key::N), &store);
        ed.handle(&press_left(cell_rect(0, 0).center()), &store);
        rename(&mut ed, &store, "MIN BANA");
        ed.handle(&key(Key::S), &store);
        assert!(matches!(
            ed.modal,
            Modal::Confirm {
                action: Pending::Overwrite,
                ..
            }
        ));
        ed.handle(&key(Key::Escape), &store);
        assert_eq!(
            store::load(&dir.join("min-bana.json"))
                .unwrap()
                .brick_count(),
            2
        );
        ed.handle(&key(Key::S), &store);
        ed.handle(&key(Key::Enter), &store); // confirm overwrite
        assert!(!message(&ed).1);
        assert_eq!(
            store::load(&dir.join("min-bana.json"))
                .unwrap()
                .brick_count(),
            1
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn builtin_levels_open_as_copies_and_cannot_be_overwritten() {
        let dir = temp_dir("editor-builtin");
        let store = LevelStore::new(dir.clone());
        let mut ed = Editor::new();
        ed.handle(&key(Key::O), &store);
        ed.handle(&key(Key::Down), &store); // "Diamanten"
        ed.handle(&key(Key::Enter), &store);
        assert!(matches!(ed.modal, Modal::None));
        assert_eq!(ed.layout.name, "The Diamond copy");
        assert_eq!(ed.layout.brick_count(), levels::builtin()[1].brick_count());

        rename(&mut ed, &store, "the diamond");
        ed.handle(&key(Key::S), &store);
        let (text, error) = message(&ed);
        assert!(error && text.contains("built-in"), "{text}");
        assert!(!dir.exists());
    }

    #[test]
    fn unsaved_changes_are_protected() {
        let store = LevelStore::new(temp_dir("editor-guard"));
        let mut ed = Editor::new();
        assert_eq!(ed.handle(&key(Key::Escape), &store), Some(Outcome::Leave));

        ed.handle(&press_left(cell_rect(1, 1).center()), &store);
        ed.handle(&Input::default(), &store);
        assert_eq!(ed.handle(&key(Key::Escape), &store), None);
        assert!(matches!(
            ed.modal,
            Modal::Confirm {
                action: Pending::Leave,
                ..
            }
        ));
        ed.handle(&key(Key::Escape), &store); // "Nej"
        assert_eq!(ed.layout.brick_count(), 1);

        ed.handle(&key(Key::N), &store);
        assert!(matches!(
            ed.modal,
            Modal::Confirm {
                action: Pending::New,
                ..
            }
        ));
        ed.handle(&key(Key::Escape), &store);
        assert_eq!(ed.layout.brick_count(), 1);

        ed.handle(&key(Key::O), &store);
        ed.handle(&key(Key::Enter), &store);
        assert!(matches!(
            ed.modal,
            Modal::Confirm {
                action: Pending::Open(_),
                ..
            }
        ));
        ed.handle(&key(Key::Escape), &store);
        assert_eq!(ed.layout.brick_count(), 1);

        ed.handle(&key(Key::Escape), &store);
        assert_eq!(ed.handle(&key(Key::Enter), &store), Some(Outcome::Leave));
        assert_eq!(ed.layout.brick_count(), 0, "discarded after confirming");
    }

    #[test]
    fn invalid_files_show_errors() {
        let dir = temp_dir("editor-invalid");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("trasig.json"),
            r#"{"format":1,"name":"x","rows":["12"]}"#,
        )
        .unwrap();
        let store = LevelStore::new(dir.clone());
        let mut ed = Editor::new();
        ed.handle(&key(Key::O), &store);
        for _ in 0..3 {
            ed.handle(&key(Key::Down), &store);
        }
        ed.handle(&key(Key::Enter), &store);
        let (text, error) = message(&ed);
        assert!(error && text.contains("Row 1"), "{text}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_save_is_reported() {
        let path = temp_dir("editor-fail");
        std::fs::write(&path, "a file, not a directory").unwrap();
        let store = LevelStore::new(path.clone());
        let mut ed = Editor::new();
        ed.handle(&press_left(cell_rect(0, 0).center()), &store);
        ed.handle(&key(Key::S), &store);
        let (text, error) = message(&ed);
        assert!(error && text.contains("failed"), "{text}");
        assert!(ed.dirty, "still unsaved");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn naming_accepts_text_and_backspace() {
        let store = LevelStore::new(temp_dir("naming"));
        let mut ed = Editor::new();
        ed.handle(&key(Key::F2), &store);
        ed.handle(&typed("!x"), &store);
        ed.handle(&key(Key::Backspace), &store);
        ed.handle(&key(Key::Enter), &store);
        assert_eq!(ed.layout.name, "New level!");
        assert!(ed.dirty);
        let long = "a".repeat(100);
        ed.handle(&key(Key::F2), &store);
        ed.handle(&typed(&long), &store);
        ed.handle(&key(Key::Enter), &store);
        assert_eq!(ed.layout.name.chars().count(), MAX_NAME_LEN);
    }

    fn open_grid_dialog(ed: &mut Editor, store: &LevelStore) {
        ed.handle(&key(Key::G), store);
        assert!(matches!(ed.modal, Modal::Grid { .. }));
    }

    fn click_dialog(ed: &mut Editor, store: &LevelStore, which: DialogButton) {
        let rect = dialog_buttons(&ed.modal)
            .into_iter()
            .find(|b| b.action == which)
            .unwrap()
            .rect;
        ed.handle(&press_left(rect.center()), store);
        ed.handle(&Input::default(), store);
    }

    #[test]
    fn grid_dialog_changes_columns_and_rows_independently() {
        let store = LevelStore::new(temp_dir("grid-dialog"));
        let mut ed = Editor::new();
        open_grid_dialog(&mut ed, &store);
        for _ in 0..4 {
            click_dialog(&mut ed, &store, DialogButton::ColsMinus);
        }
        ed.handle(&key(Key::Up), &store);
        assert!(matches!(ed.modal, Modal::Grid { cols: 8, rows: 25 }));
        // Limits hold.
        for _ in 0..30 {
            ed.handle(&key(Key::Left), &store);
            ed.handle(&key(Key::Down), &store);
        }
        assert!(matches!(ed.modal, Modal::Grid { cols: 4, rows: 8 }));
        // Cancel changes nothing.
        ed.handle(&key(Key::Escape), &store);
        assert_eq!(ed.layout.grid(), Grid::DEFAULT);
        assert!(!ed.dirty);

        open_grid_dialog(&mut ed, &store);
        click_dialog(&mut ed, &store, DialogButton::ColsPlus);
        click_dialog(&mut ed, &store, DialogButton::RowsMinus);
        click_dialog(&mut ed, &store, DialogButton::Ok);
        assert_eq!(ed.layout.grid(), Grid::new(13, 23).unwrap());
        assert!(ed.dirty, "an empty level resizes without asking");
        assert!(matches!(ed.modal, Modal::None));
    }

    #[test]
    fn shrinking_warns_before_removing_blocks() {
        let store = LevelStore::new(temp_dir("grid-shrink"));
        let mut ed = Editor::new();
        ed.layout =
            Layout::from_rows("x", &["2..........#", "............", "1..........."]).unwrap();
        ed.layout.add_bonus(0, 0, Bonus::Wall).unwrap();
        let before = ed.layout.clone();

        open_grid_dialog(&mut ed, &store);
        ed.handle(&key(Key::Left), &store); // 11 columns: the stone is outside
        ed.handle(&key(Key::Enter), &store);
        match &ed.modal {
            Modal::Confirm { text, action } => {
                assert!(text.contains("removed: 1."), "{text}");
                assert_eq!(*action, Pending::Resize(Grid::new(11, 24).unwrap()));
            }
            _ => panic!("expected a warning"),
        }
        ed.handle(&key(Key::Escape), &store); // "Nej"
        assert_eq!(ed.layout, before, "cancel changes nothing");
        assert!(!ed.dirty);

        open_grid_dialog(&mut ed, &store);
        ed.handle(&key(Key::Left), &store);
        ed.handle(&key(Key::Enter), &store);
        ed.handle(&key(Key::Enter), &store); // "Ja"
        assert_eq!(ed.layout.grid(), Grid::new(11, 24).unwrap());
        assert_eq!(
            ed.layout.to_rows(),
            ["2..........", "...........", "1.........."]
        );
        assert_eq!(ed.layout.bonuses(), vec![(0, 0, Bonus::Wall)], "bonus kept");
        assert!(ed.dirty);

        // Growing never asks.
        open_grid_dialog(&mut ed, &store);
        ed.handle(&key(Key::Right), &store);
        ed.handle(&key(Key::Right), &store);
        ed.handle(&key(Key::Enter), &store);
        assert!(matches!(ed.modal, Modal::None));
        assert_eq!(ed.layout.grid().cols, 13);
    }

    #[test]
    fn painting_uses_the_current_grid() {
        let store = LevelStore::new(temp_dir("grid-paint"));
        let mut ed = Editor::new();
        let grid = Grid::new(5, 9).unwrap();
        ed.layout = Layout::with_grid("x", grid);
        // Click the lower right part of the last cell: it must land there.
        let r = grid.cell_rect(8, 4);
        ed.handle(&press_left(r.point() + r.size() * 0.9), &store);
        ed.handle(&drag_left(grid.cell_rect(8, 0).center()), &store);
        assert!((0..5).all(|c| ed.layout.cell(8, c).block == Block::Brick(1)));
        assert_eq!(ed.layout.bricks().len(), 5);
        assert_eq!(ed.layout.bricks()[0].rect, grid.cell_rect(8, 0));
    }
}
