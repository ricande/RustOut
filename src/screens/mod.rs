//! Drawing of everything except the game itself: the menu and level list
//! (`menu`), the editor (`editor`) and the video UI (`video`), plus the small
//! widgets they share. Uses the same field coordinates and button rectangles
//! as the input handling, so what is drawn is what is clicked.

mod editor;
mod menu;
mod video;

pub use editor::draw_editor;
pub use menu::{draw_menu, draw_select};
pub use video::draw_video_ui;

use macroquad::prelude::*;

use crate::game::FIELD_W;
use crate::render::{with_alpha, View, ACCENT, BACKDROP, NEON, TEXT};
use crate::t;
use crate::ui::{Button, ListState};

const ERROR: Color = Color::new(1.0, 0.4, 0.4, 1.0);
const BUTTON: Color = Color::new(0.08, 0.10, 0.22, 1.0);
const BUTTON_HOT: Color = Color::new(0.14, 0.20, 0.40, 1.0);
const PANEL_BG: Color = Color::new(0.04, 0.05, 0.12, 0.97);

fn hovered(rect: Rect, pointer: Option<Vec2>) -> bool {
    pointer.is_some_and(|p| rect.contains(p))
}

fn draw_button(v: &View, rect: Rect, label: &str, hot: bool) {
    v.rect(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if hot { BUTTON_HOT } else { BUTTON },
    );
    let border = with_alpha(NEON, if hot { 1.0 } else { 0.5 });
    v.rect_lines(rect.x, rect.y, rect.w, rect.h, 2.0, border);
    let size = (rect.h * 0.5).min(28.0);
    v.text_centered(
        label,
        rect.x + rect.w / 2.0,
        rect.y + rect.h / 2.0 + size * 0.32,
        size,
        TEXT,
    );
}

fn draw_buttons<A>(v: &View, buttons: &[Button<A>], pointer: Option<Vec2>) {
    for b in buttons {
        draw_button(v, b.rect, b.label, hovered(b.rect, pointer));
    }
}

/// Logical pixels, with one decimal only when needed (decimal comma in
/// Swedish, point in English).
fn px(v: f32) -> String {
    if (v - v.round()).abs() < 0.05 {
        format!("{:.0}", v.round())
    } else if crate::i18n::lang() == crate::i18n::Lang::Sv {
        format!("{v:.1}").replace('.', ",")
    } else {
        format!("{v:.1}")
    }
}

/// A one-line text box. Text that does not fit shows its end (where the
/// caret is), starting with "...".
fn draw_text_field(v: &View, r: Rect, text: &str, size: f32, caret: bool, border: Color) {
    v.rect(r.x, r.y, r.w, r.h, BACKDROP);
    v.rect_lines(r.x, r.y, r.w, r.h, 2.0, border);
    let fits = |s: &str| v.text_size(s, size).width / v.scale <= r.w - 24.0;
    let mut shown = format!("{text}_");
    let mut skip = 0;
    while !fits(&shown) && skip < text.chars().count() {
        skip += 1;
        shown = format!("...{}_", text.chars().skip(skip).collect::<String>());
    }
    shown.pop();
    if caret {
        shown.push('_');
    }
    v.text(&shown, r.x + 12.0, r.y + (r.h * 0.65).round(), size, TEXT);
}

/// Splits text into lines of at most `width` characters.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let mut word: Vec<char> = word.chars().collect();
            // Hard-break very long words such as file paths.
            while word.len() > width {
                if !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                lines.push(word.drain(..width).collect());
            }
            let word: String = word.into_iter().collect();
            if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(&word);
        }
        lines.push(line);
    }
    lines
}

fn draw_wrapped(v: &View, text: &str, x: f32, y: f32, size: f32, color: Color) -> f32 {
    let width = ((FIELD_W - 2.0 * x) / (size * 0.47)) as usize;
    let mut y = y;
    for line in wrap(text, width.max(10)) {
        v.text(&line, x, y, size, color);
        y += size * 1.3;
    }
    y
}

fn draw_list<T>(v: &View, list: &ListState<T>, pointer: Option<Vec2>) {
    for (i, item) in list.items.iter().enumerate() {
        let Some(r) = list.row_rect(i) else { continue };
        let bg = if i == list.selected {
            BUTTON_HOT
        } else if hovered(r, pointer) {
            BUTTON
        } else {
            with_alpha(BUTTON, 0.5)
        };
        v.rect(r.x, r.y, r.w, r.h, bg);
        if i == list.selected {
            v.rect_lines(r.x, r.y, r.w, r.h, 2.0, ACCENT);
        }
        let color = if item.error.is_some() { ERROR } else { TEXT };
        v.text(&item.label, r.x + 12.0, r.y + r.h / 2.0 + 7.0, 22.0, color);
    }
    let hidden_above = list.scroll > 0;
    let hidden_below = list.scroll + list.visible_rows() < list.items.len();
    let a = list.area;
    if hidden_above {
        v.text_centered(t!("list.more_up"), a.x + a.w / 2.0, a.y - 6.0, 16.0, NEON);
    }
    if hidden_below {
        let y = a.y + list.visible_rows() as f32 * crate::ui::LIST_ROW_H + 12.0;
        v.text_centered(t!("list.more_down"), a.x + a.w / 2.0, y, 16.0, NEON);
    }
}

#[cfg(test)]
mod tests {
    use super::wrap;

    #[test]
    fn pixel_sizes_are_formatted_readably() {
        assert_eq!(super::px(45.0), "45");
        assert_eq!(super::px(540.0 / 7.0), "77.1");
        std::thread::spawn(|| {
            crate::i18n::set_lang(crate::i18n::Lang::Sv);
            assert_eq!(super::px(540.0 / 7.0), "77,1");
        })
        .join()
        .unwrap();
        assert_eq!(super::px(24.0), "24");
    }

    #[test]
    fn wrap_breaks_words_and_long_paths() {
        assert_eq!(wrap("aa bb cc", 5), vec!["aa bb", "cc"]);
        assert_eq!(
            wrap("x /a/very/long/path", 6),
            vec!["x", "/a/ver", "y/long", "/path"]
        );
        assert_eq!(wrap("rad1\nrad2", 20), vec!["rad1", "rad2"]);
    }
}
