//! The level editor: palette, grid, toolbar, status line and dialogs.

use macroquad::prelude::*;

use super::{draw_buttons, draw_list, draw_text_field, draw_wrapped, hovered, px, ERROR, PANEL_BG};
use crate::editor::{dialog_buttons, palette, toolbar, Brush, Editor, Modal, GRID_ROWS_Y, PANEL};
use crate::game::{Brick, FIELD_H, FIELD_W, PADDLE_H, PADDLE_W, PADDLE_Y};
use crate::levels::{self, Block, Grid, BUILD_AREA};
use crate::render::{
    brick_face, draw_brick, draw_capsule, draw_field, with_alpha, Effects, View, ACCENT, BACKDROP,
    NEON, TEXT,
};
use crate::t;

pub fn draw_editor(editor: &Editor, view: View, fx: &Effects, pointer: Option<Vec2>) {
    let v = &view;
    clear_background(BACKDROP);
    draw_field(v);
    // Hover effects only while no dialog is open.
    let hover = pointer.filter(|_| matches!(editor.modal, Modal::None));

    draw_header(v, editor);
    draw_palette(v, editor, hover);
    draw_grid(v, editor, hover);
    draw_footer(v, editor, hover);
    draw_modal(editor, v, fx, pointer);
}

/// Level name, with `*` for unsaved changes.
fn draw_header(v: &View, editor: &Editor) {
    let title = format!(
        "{}{}",
        editor.layout.name,
        if editor.dirty { " *" } else { "" }
    );
    v.text_centered(&title, FIELD_W / 2.0, 42.0, 26.0, ACCENT);
}

/// Brush palette: block types above the grid, bonuses below it.
fn draw_palette(v: &View, editor: &Editor, hover: Option<Vec2>) {
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
        if selected || hovered(rect, hover) {
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
}

/// The build area with its grid, the blocks and the cell under the mouse.
fn draw_grid(v: &View, editor: &Editor, hover: Option<Vec2>) {
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
    let size = format!("{} × {}", grid.cols, grid.rows);
    v.text(&size, 436.0, 94.0, 18.0, TEXT);
    let cell = format!("{} × {} px", px(grid.cell_w()), px(grid.cell_h()));
    v.text(&cell, 436.0, 114.0, 15.0, with_alpha(TEXT, 0.75));
    for brick in editor.layout.bricks() {
        draw_brick(v, &brick);
        // Spell out which bonus a brick carries.
        if let Some(bonus) = brick.bonus {
            let r = brick.rect;
            let label = Rect::new(r.x + 5.0, r.y + 5.0, r.w - 10.0, r.h - 10.0);
            draw_capsule(v, label, bonus);
        }
    }
    if let Some((row, col)) = hover.and_then(|p| grid.cell_at(p)) {
        let r = brick_face(grid.cell_rect(row, col));
        v.rect(r.x, r.y, r.w, r.h, with_alpha(WHITE, 0.2));
        v.rect_lines(r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, 2.0, WHITE);
    }
}

/// Brush hint, where the paddle will be, the toolbar and the file status.
fn draw_footer(v: &View, editor: &Editor, hover: Option<Vec2>) {
    let erase = match editor.brush {
        Brush::Bonus(_) => t!("brush.erase_bonus"),
        Brush::Block(_) => t!("brush.erase_block"),
    };
    let hint = t!("brush.hint", brush = editor.brush.name(), erase = erase);
    v.text_centered(&hint, FIELD_W / 2.0, 764.0, 16.0, with_alpha(TEXT, 0.85));

    let paddle_x = FIELD_W / 2.0 - PADDLE_W / 2.0;
    v.rect(
        paddle_x,
        PADDLE_Y,
        PADDLE_W,
        PADDLE_H,
        with_alpha(WHITE, 0.15),
    );

    draw_buttons(v, &toolbar(), hover);
    let status = match &editor.file {
        Some(path) => t!(
            "editor.file",
            name = path.file_name().unwrap_or_default().to_string_lossy()
        ),
        None => t!("editor.not_saved").to_owned(),
    };
    v.text(&status, 20.0, 940.0, 15.0, with_alpha(TEXT, 0.6));
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
        Modal::Naming { field } => draw_naming(v, &field.text, fx),
        Modal::Browse { list } => {
            draw_list(v, list, pointer);
            if let Some(e) = list.selected_item().and_then(|i| i.error.as_ref()) {
                draw_wrapped(v, e, x, PANEL.bottom() - 98.0, 15.0, ERROR);
            }
        }
        Modal::Grid { cols, rows } => draw_grid_choice(v, editor, *cols, *rows),
        Modal::Confirm { text, .. } | Modal::Message { text, .. } => {
            draw_wrapped(v, text, x, PANEL.y + 110.0, 20.0, TEXT);
        }
    }
    draw_buttons(v, &dialog_buttons(&editor.modal), pointer);
}

/// The level name dialog's text box and hints.
fn draw_naming(v: &View, text: &str, fx: &Effects) {
    let x = PANEL.x + 24.0;
    let r = Rect::new(PANEL.x + 20.0, PANEL.y + 120.0, PANEL.w - 40.0, 52.0);
    draw_text_field(v, r, text, 24.0, fx.blink(), NEON);
    let hint = t!(
        "modal.name_hint",
        max = levels::MAX_NAME_LEN,
        file = crate::store::file_stem(text)
    );
    draw_wrapped(v, &hint, x, PANEL.y + 220.0, 18.0, with_alpha(TEXT, 0.8));
    let keys = t!("modal.name_keys");
    v.text(keys, x, PANEL.y + 330.0, 18.0, with_alpha(TEXT, 0.6));
}

/// The grid dialog: columns, rows, the resulting block size and a warning
/// if blocks would be lost.
fn draw_grid_choice(v: &View, editor: &Editor, cols: usize, rows: usize) {
    let x = PANEL.x + 24.0;
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
    let area = t!("grid.area", w = px(a.w), h = px(a.h));
    v.text(&area, x, y + 30.0, 18.0, with_alpha(TEXT, 0.8));
    if let Ok(grid) = Grid::new(cols, rows) {
        let lost = editor.layout.blocks_outside(grid);
        if lost > 0 {
            v.text(&t!("grid.lost", lost = lost), x, y + 60.0, 18.0, ERROR);
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
