//! Drawing for the menu, the custom level list and the editor. Uses the
//! same field coordinates (and button rectangles) as the input handling.

use macroquad::prelude::*;

use crate::app::{
    menu_buttons, select_buttons, video_dialog_buttons, video_field_rect, App, VideoField,
    VIDEO_BUTTON, VIDEO_PANEL,
};
use crate::editor::{dialog_buttons, palette, toolbar, Brush, Editor, Modal, GRID_ROWS_Y, PANEL};
use crate::game::{Brick, FIELD_H, FIELD_W, PADDLE_H, PADDLE_W, PADDLE_Y};
use crate::levels::{self, Block, BUILD_AREA};
use crate::render::{
    brick_face, dim, draw_brick, draw_capsule, draw_field, with_alpha, Effects, View, ACCENT,
    BACKDROP, NEON, TEXT,
};
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

pub fn draw_menu(app: &App, view: View, fx: &Effects) {
    clear_background(BACKDROP);
    draw_field(&view);
    for brick in levels::builtin()[0].bricks() {
        draw_brick(&view, &brick);
    }
    dim(&view, 0.6);
    let cx = FIELD_W / 2.0;
    view.text_centered("RUSTOUT", cx, 430.0, 96.0, ACCENT);
    view.text_centered(t!("menu.subtitle"), cx, 475.0, 26.0, NEON);
    for (i, b) in menu_buttons(app.can_save_video()).iter().enumerate() {
        let hot = i == app.menu_selected;
        draw_button(&view, b.rect, b.label, hot);
        if hot {
            view.text(">", b.rect.x - 30.0, b.rect.y + 37.0, 30.0, ACCENT);
        }
    }
    let help = [
        t!("menu.help_steer"),
        t!("menu.help_launch"),
        t!("menu.help_keys"),
    ];
    for (i, line) in help.iter().enumerate() {
        view.text_centered(line, cx, 830.0 + i as f32 * 28.0, 20.0, TEXT);
    }
    if let Some(e) = &app.settings_error {
        draw_wrapped(&view, e, 20.0, 790.0, 15.0, ERROR);
    }
    let prompt = with_alpha(TEXT, if fx.blink() { 0.9 } else { 0.3 });
    view.text_centered(t!("menu.help_choose"), cx, 930.0, 18.0, prompt);
}

pub fn draw_select(app: &App, view: View, pointer: Option<Vec2>) {
    clear_background(BACKDROP);
    draw_field(&view);
    let cx = FIELD_W / 2.0;
    view.text_centered(t!("select.title"), cx, 46.0, 34.0, ACCENT);
    if let Some(e) = &app.select_error {
        draw_wrapped(&view, e, 40.0, 180.0, 20.0, ERROR);
    } else if app.select.items.is_empty() {
        view.text_centered(t!("select.empty"), cx, 300.0, 26.0, TEXT);
        view.text_centered(t!("select.empty_hint"), cx, 340.0, 22.0, NEON);
    } else {
        draw_list(&view, &app.select, pointer);
        if let Some(e) = app.select.selected_item().and_then(|i| i.error.as_ref()) {
            draw_wrapped(
                &view,
                &t!("select.invalid", error = e),
                40.0,
                690.0,
                18.0,
                ERROR,
            );
        }
    }
    draw_buttons(&view, &select_buttons(), pointer);
    draw_wrapped(
        &view,
        &t!("select.folder", path = app.store.dir().display()),
        20.0,
        890.0,
        16.0,
        with_alpha(TEXT, 0.7),
    );
}

pub fn draw_editor(editor: &Editor, view: View, fx: &Effects, pointer: Option<Vec2>) {
    let v = &view;
    clear_background(BACKDROP);
    draw_field(v);

    // Header: level name and unsaved marker.
    let title = format!(
        "{}{}",
        editor.layout.name,
        if editor.dirty { " *" } else { "" }
    );
    v.text_centered(&title, FIELD_W / 2.0, 42.0, 26.0, ACCENT);

    // Brush palette: block types above the grid, bonuses below it.
    v.text(t!("editor.blocks"), 20.0, 104.0, 20.0, TEXT);
    v.text(t!("editor.bonuses"), 20.0, 734.0, 20.0, TEXT);
    for (brush, rect) in palette() {
        match brush {
            Brush::Block(block) => {
                let (hp, stone) = match block {
                    Block::Brick(hp) => (hp, false),
                    _ => (1, true),
                };
                let brick = Brick {
                    rect,
                    hp,
                    max_hp: hp,
                    row: 2,
                    stone,
                    bonus: None,
                };
                draw_brick(v, &brick);
            }
            Brush::Bonus(bonus) => {
                let r = Rect::new(rect.x + 4.0, rect.y + 5.0, rect.w - 8.0, rect.h - 10.0);
                draw_capsule(v, r, bonus);
            }
        }
        let selected = brush == editor.brush;
        if selected || hovered(rect, pointer) && matches!(editor.modal, Modal::None) {
            let color = if selected {
                ACCENT
            } else {
                with_alpha(TEXT, 0.5)
            };
            v.rect_lines(
                rect.x - 4.0,
                rect.y - 4.0,
                rect.w + 8.0,
                rect.h + 8.0,
                3.0,
                color,
            );
        }
    }

    // Grid and build area.
    let area = BUILD_AREA;
    let grid = editor.layout.grid();
    v.rect(area.x, area.y, area.w, area.h, with_alpha(BLACK, 0.35));
    for row in 0..grid.rows {
        for col in 0..grid.cols {
            let r = brick_face(grid.cell_rect(row, col));
            v.rect_lines(r.x, r.y, r.w, r.h, 1.0, with_alpha(NEON, 0.18));
        }
    }
    v.rect_lines(area.x, area.y, area.w, area.h, 2.0, with_alpha(ACCENT, 0.6));
    // Grid size and the resulting block size, next to the block palette.
    v.text(
        &format!("{} × {}", grid.cols, grid.rows),
        436.0,
        94.0,
        18.0,
        TEXT,
    );
    v.text(
        &format!("{} × {} px", px(grid.cell_w()), px(grid.cell_h())),
        436.0,
        114.0,
        15.0,
        with_alpha(TEXT, 0.75),
    );
    for brick in editor.layout.bricks() {
        draw_brick(v, &brick);
        // Spell out which bonus a brick carries.
        if let Some(bonus) = brick.bonus {
            let r = brick.rect;
            draw_capsule(
                v,
                Rect::new(r.x + 5.0, r.y + 5.0, r.w - 10.0, r.h - 10.0),
                bonus,
            );
        }
    }
    if matches!(editor.modal, Modal::None) {
        if let Some((row, col)) = pointer.and_then(|p| grid.cell_at(p)) {
            let r = brick_face(grid.cell_rect(row, col));
            v.rect(r.x, r.y, r.w, r.h, with_alpha(WHITE, 0.2));
            v.rect_lines(r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, 2.0, WHITE);
        }
    }
    let erase = match editor.brush {
        Brush::Bonus(_) => t!("brush.erase_bonus"),
        Brush::Block(_) => t!("brush.erase_block"),
    };
    v.text_centered(
        &t!("brush.hint", brush = editor.brush.name(), erase = erase),
        FIELD_W / 2.0,
        764.0,
        16.0,
        with_alpha(TEXT, 0.85),
    );

    // Where the paddle will be, for reference.
    v.rect(
        FIELD_W / 2.0 - PADDLE_W / 2.0,
        PADDLE_Y,
        PADDLE_W,
        PADDLE_H,
        with_alpha(WHITE, 0.15),
    );

    draw_buttons(
        v,
        &toolbar(),
        pointer.filter(|_| matches!(editor.modal, Modal::None)),
    );
    let status = match &editor.file {
        Some(path) => t!(
            "editor.file",
            name = path.file_name().unwrap_or_default().to_string_lossy()
        ),
        None => t!("editor.not_saved").to_owned(),
    };
    v.text(&status, 20.0, 940.0, 15.0, with_alpha(TEXT, 0.6));

    draw_modal(editor, v, fx, pointer);
}

fn draw_modal(editor: &Editor, v: &View, fx: &Effects, pointer: Option<Vec2>) {
    let (title, title_color) = match &editor.modal {
        Modal::None => return,
        Modal::Naming { .. } => (t!("modal.name"), ACCENT),
        Modal::Browse { .. } => (t!("modal.open"), ACCENT),
        Modal::Grid { .. } => (t!("modal.grid"), ACCENT),
        Modal::Confirm { .. } => (t!("modal.confirm"), ACCENT),
        Modal::Message { error: true, .. } => (t!("modal.error"), ERROR),
        Modal::Message { error: false, .. } => (t!("modal.done"), NEON),
    };
    v.rect(0.0, 0.0, FIELD_W, FIELD_H, with_alpha(BLACK, 0.6));
    v.rect(PANEL.x, PANEL.y, PANEL.w, PANEL.h, PANEL_BG);
    v.rect_lines(PANEL.x, PANEL.y, PANEL.w, PANEL.h, 2.0, title_color);
    v.text_centered(title, FIELD_W / 2.0, PANEL.y + 48.0, 32.0, title_color);
    let x = PANEL.x + 24.0;
    match &editor.modal {
        Modal::None => {}
        Modal::Naming { field } => {
            let text = &field.text;
            let r = Rect::new(PANEL.x + 20.0, PANEL.y + 120.0, PANEL.w - 40.0, 52.0);
            draw_text_field(v, r, text, 24.0, fx.blink(), NEON);
            draw_wrapped(
                v,
                &t!(
                    "modal.name_hint",
                    max = levels::MAX_NAME_LEN,
                    file = crate::store::file_stem(text)
                ),
                x,
                PANEL.y + 220.0,
                18.0,
                with_alpha(TEXT, 0.8),
            );
            v.text(
                t!("modal.name_keys"),
                x,
                PANEL.y + 330.0,
                18.0,
                with_alpha(TEXT, 0.6),
            );
        }
        Modal::Browse { list } => {
            draw_list(v, list, pointer);
            if let Some(e) = list.selected_item().and_then(|i| i.error.as_ref()) {
                draw_wrapped(v, e, x, PANEL.bottom() - 98.0, 15.0, ERROR);
            }
        }
        Modal::Grid { cols, rows } => {
            let (cols, rows) = (*cols, *rows);
            v.text(
                &t!("grid.columns", value = cols),
                x,
                GRID_ROWS_Y,
                26.0,
                TEXT,
            );
            v.text(
                &t!("grid.rows", value = rows),
                x,
                GRID_ROWS_Y + 70.0,
                26.0,
                TEXT,
            );
            let a = BUILD_AREA;
            let (w, h) = (a.w / cols as f32, a.h / rows as f32);
            let y = GRID_ROWS_Y + 140.0;
            v.text(
                &t!("grid.block_size", w = px(w), h = px(h)),
                x,
                y,
                22.0,
                ACCENT,
            );
            v.text(
                &t!("grid.area", w = px(a.w), h = px(a.h)),
                x,
                y + 30.0,
                18.0,
                with_alpha(TEXT, 0.8),
            );
            if let Ok(grid) = levels::Grid::new(cols, rows) {
                let lost = editor.layout.blocks_outside(grid);
                if lost > 0 {
                    let text = t!("grid.lost", lost = lost);
                    v.text(&text, x, y + 60.0, 18.0, ERROR);
                }
            }
            let (c, r) = (levels::COLS_RANGE, levels::ROWS_RANGE);
            let limits = t!(
                "grid.limits",
                cmin = c.start(),
                cmax = c.end(),
                rmin = r.start(),
                rmax = r.end()
            );
            draw_wrapped(v, &limits, x, y + 92.0, 15.0, with_alpha(TEXT, 0.6));
        }
        Modal::Confirm { text, .. } | Modal::Message { text, .. } => {
            draw_wrapped(v, text, x, PANEL.y + 110.0, 20.0, TEXT);
        }
    }
    draw_buttons(v, &dialog_buttons(&editor.modal), pointer);
}

/// The video parts drawn on top of every screen: the REC marker (window
/// only, never in the video), "Save video", the save dialog and messages.
pub fn draw_video_ui(app: &App, view: View, fx: &Effects, pointer: Option<Vec2>, recording: bool) {
    let v = &view;
    let cx = FIELD_W / 2.0;
    if recording && app.in_game() {
        let on = fx.blink() || app.game.state == crate::game::State::Paused;
        v.circle(
            Vec2::new(478.0, 80.0),
            6.0,
            if on { ERROR } else { with_alpha(ERROR, 0.3) },
        );
        v.text("REC", 490.0, 87.0, 18.0, ERROR);
    }
    let game_over = app.in_game() && app.round_over();
    if game_over && app.video_dialog.is_none() {
        if app.can_save_video() {
            draw_button(
                v,
                VIDEO_BUTTON,
                t!("video.button"),
                hovered(VIDEO_BUTTON, pointer),
            );
        } else if let Some(why) = &app.video_unavailable {
            draw_wrapped(
                v,
                why,
                40.0,
                VIDEO_BUTTON.y + 20.0,
                16.0,
                with_alpha(TEXT, 0.6),
            );
        }
    }
    if app.video_saving {
        v.text_centered(t!("video.saving"), cx, FIELD_H - 24.0, 20.0, ACCENT);
    }

    if let Some(d) = &app.video_dialog {
        let p = VIDEO_PANEL;
        v.rect(0.0, 0.0, FIELD_W, FIELD_H, with_alpha(BLACK, 0.6));
        v.rect(p.x, p.y, p.w, p.h, PANEL_BG);
        v.rect_lines(p.x, p.y, p.w, p.h, 2.0, ACCENT);
        v.text_centered(t!("video.title"), cx, p.y + 46.0, 32.0, ACCENT);
        let x = p.x + 24.0;
        if let Some(path) = &d.overwrite {
            let text = t!(
                "common.file_exists",
                path = crate::video::display_path(path)
            );
            draw_wrapped(v, &text, x, p.y + 110.0, 20.0, TEXT);
        } else {
            for (field, label, text) in [
                (VideoField::Name, t!("video.name_label"), &d.name.text),
                (VideoField::Dir, t!("video.folder_label"), &d.dir.text),
            ] {
                let r = video_field_rect(field);
                v.text(label, r.x, r.y - 10.0, 18.0, with_alpha(TEXT, 0.8));
                let focused = d.focus == field;
                let border = if focused {
                    ACCENT
                } else {
                    with_alpha(NEON, 0.5)
                };
                draw_text_field(v, r, text, 20.0, focused && fx.blink(), border);
            }
            let info = t!("video.info");
            draw_wrapped(v, info, x, p.y + 290.0, 15.0, with_alpha(TEXT, 0.6));
            if let Some(e) = &d.error {
                draw_wrapped(v, e, x, p.y + 335.0, 17.0, ERROR);
            }
        }
        let confirming = d.overwrite.is_some();
        draw_buttons(v, &video_dialog_buttons(confirming), pointer);
    } else if let Some((text, error)) = &app.video_message {
        let p = Rect::new(30.0, 380.0, 480.0, 220.0);
        v.rect(0.0, 0.0, FIELD_W, FIELD_H, with_alpha(BLACK, 0.5));
        v.rect(p.x, p.y, p.w, p.h, PANEL_BG);
        let color = if *error { ERROR } else { NEON };
        v.rect_lines(p.x, p.y, p.w, p.h, 2.0, color);
        let title = if *error {
            t!("modal.error")
        } else {
            t!("modal.done")
        };
        v.text_centered(title, cx, p.y + 44.0, 30.0, color);
        draw_wrapped(v, text, p.x + 24.0, p.y + 90.0, 18.0, TEXT);
        v.text_centered(
            t!("common.enter_ok"),
            cx,
            p.bottom() - 20.0,
            16.0,
            with_alpha(TEXT, 0.6),
        );
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
