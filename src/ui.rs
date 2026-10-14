//! Window-independent input and small UI building blocks (buttons, lists).
//! Everything is in field coordinates, so menus, editor and game agree on
//! where things are no matter how the window is sized or letterboxed.

use macroquad::math::{Rect, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
    Space,
    Backspace,
    P,
    R,
    N,
    O,
    S,
    T,
    F2,
    G,
    V,
    Tab,
    Num1,
    Num2,
    Num3,
    Num4,
}

impl Key {
    /// A key by its name in scripts: `enter`, `esc`, `space`, `up`, `f2`, `1` …
    pub fn from_name(name: &str) -> Option<Key> {
        Some(match name.to_lowercase().as_str() {
            "up" => Key::Up,
            "down" => Key::Down,
            "left" => Key::Left,
            "right" => Key::Right,
            "enter" | "return" => Key::Enter,
            "esc" | "escape" => Key::Escape,
            "space" => Key::Space,
            "backspace" => Key::Backspace,
            "tab" => Key::Tab,
            "f2" => Key::F2,
            "p" => Key::P,
            "r" => Key::R,
            "n" => Key::N,
            "o" => Key::O,
            "s" => Key::S,
            "t" => Key::T,
            "g" => Key::G,
            "v" => Key::V,
            "1" => Key::Num1,
            "2" => Key::Num2,
            "3" => Key::Num3,
            "4" => Key::Num4,
            _ => return None,
        })
    }
}

/// One frame of input, already converted to field coordinates.
#[derive(Clone, Debug, Default)]
pub struct Input {
    /// Mouse position in field coordinates; `None` outside the view.
    pub pointer: Option<Vec2>,
    pub left_pressed: bool,
    pub left_down: bool,
    pub right_pressed: bool,
    pub right_down: bool,
    /// Mouse wheel, positive = up.
    pub wheel: f32,
    pub keys: Vec<Key>,
    pub chars: Vec<char>,
}

impl Input {
    pub fn key(&self, key: Key) -> bool {
        self.keys.contains(&key)
    }

    /// True if the left button was pressed on `rect` this frame.
    pub fn clicked(&self, rect: Rect) -> bool {
        self.left_pressed && self.pointer.is_some_and(|p| rect.contains(p))
    }
}

pub struct Button<A> {
    pub rect: Rect,
    pub label: &'static str,
    pub action: A,
}

pub fn clicked_button<A: Copy>(buttons: &[Button<A>], input: &Input) -> Option<A> {
    buttons
        .iter()
        .find(|b| input.clicked(b.rect))
        .map(|b| b.action)
}

/// Lays out `labels` as a row of equal buttons filling `area`.
pub fn button_row<A: Copy>(area: Rect, items: &[(&'static str, A)]) -> Vec<Button<A>> {
    const SPACING: f32 = 10.0;
    let n = items.len() as f32;
    let w = (area.w - SPACING * (n - 1.0)) / n;
    items
        .iter()
        .enumerate()
        .map(|(i, &(label, action))| Button {
            rect: Rect::new(area.x + i as f32 * (w + SPACING), area.y, w, area.h),
            label,
            action,
        })
        .collect()
}

pub const LIST_ROW_H: f32 = 36.0;

pub struct ListItem<T> {
    pub label: String,
    pub value: T,
    /// Why the item cannot be used, if it is broken.
    pub error: Option<String>,
}

/// A scrollable, selectable list shown inside `area`.
pub struct ListState<T> {
    pub items: Vec<ListItem<T>>,
    pub selected: usize,
    pub scroll: usize,
    pub area: Rect,
}

pub enum ListResponse {
    None,
    /// Enter, or a click on the already selected item.
    Activate(usize),
}

impl<T> ListState<T> {
    pub fn new(items: Vec<ListItem<T>>, area: Rect) -> Self {
        ListState {
            items,
            selected: 0,
            scroll: 0,
            area,
        }
    }

    pub fn visible_rows(&self) -> usize {
        ((self.area.h / LIST_ROW_H) as usize).max(1)
    }

    pub fn row_rect(&self, index: usize) -> Option<Rect> {
        let slot = index.checked_sub(self.scroll)?;
        (slot < self.visible_rows()).then(|| {
            Rect::new(
                self.area.x,
                self.area.y + slot as f32 * LIST_ROW_H,
                self.area.w,
                LIST_ROW_H - 4.0,
            )
        })
    }

    pub fn selected_item(&self) -> Option<&ListItem<T>> {
        self.items.get(self.selected)
    }

    fn select(&mut self, index: usize) {
        if self.items.is_empty() {
            return;
        }
        self.selected = index.min(self.items.len() - 1);
        let rows = self.visible_rows();
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + rows {
            self.scroll = self.selected + 1 - rows;
        }
    }

    pub fn handle(&mut self, input: &Input) -> ListResponse {
        if self.items.is_empty() {
            return ListResponse::None;
        }
        if input.key(Key::Up) {
            self.select(self.selected.saturating_sub(1));
        }
        if input.key(Key::Down) {
            self.select(self.selected + 1);
        }
        if input.wheel != 0.0 && input.pointer.is_some_and(|p| self.area.contains(p)) {
            let max_scroll = self.items.len().saturating_sub(self.visible_rows());
            self.scroll = if input.wheel > 0.0 {
                self.scroll.saturating_sub(1)
            } else {
                (self.scroll + 1).min(max_scroll)
            };
        }
        if input.key(Key::Enter) {
            return ListResponse::Activate(self.selected);
        }
        if input.left_pressed {
            for i in 0..self.items.len() {
                if self.row_rect(i).is_some_and(|r| input.clicked(r)) {
                    if i == self.selected {
                        return ListResponse::Activate(i);
                    }
                    self.select(i);
                    break;
                }
            }
        }
        ListResponse::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::math::vec2;

    fn list(n: usize) -> ListState<usize> {
        let items = (0..n)
            .map(|i| ListItem {
                label: format!("{i}"),
                value: i,
                error: None,
            })
            .collect();
        ListState::new(items, Rect::new(0.0, 100.0, 200.0, LIST_ROW_H * 3.0))
    }

    fn keys(k: Key) -> Input {
        Input {
            keys: vec![k],
            ..Default::default()
        }
    }

    #[test]
    fn keyboard_selection_scrolls_into_view() {
        let mut l = list(10);
        for _ in 0..5 {
            l.handle(&keys(Key::Down));
        }
        assert_eq!(l.selected, 5);
        assert_eq!(l.scroll, 3);
        assert!(matches!(
            l.handle(&keys(Key::Enter)),
            ListResponse::Activate(5)
        ));
        for _ in 0..20 {
            l.handle(&keys(Key::Up));
        }
        assert_eq!((l.selected, l.scroll), (0, 0));
    }

    #[test]
    fn click_selects_then_activates() {
        let mut l = list(3);
        let click = Input {
            pointer: Some(vec2(10.0, 100.0 + LIST_ROW_H * 1.5)),
            left_pressed: true,
            ..Default::default()
        };
        assert!(matches!(l.handle(&click), ListResponse::None));
        assert_eq!(l.selected, 1);
        assert!(matches!(l.handle(&click), ListResponse::Activate(1)));
    }

    #[test]
    fn button_row_fills_area() {
        let row = button_row(Rect::new(20.0, 0.0, 500.0, 40.0), &[("a", 1), ("b", 2)]);
        assert_eq!(row[0].rect.x, 20.0);
        assert!((row[1].rect.right() - 520.0).abs() < 0.01);
    }
}
