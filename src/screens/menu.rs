//! The main menu and the custom level list.

use macroquad::prelude::*;

use super::{draw_button, draw_buttons, draw_list, draw_wrapped, ERROR};
use crate::app::{menu_buttons, select_buttons, App};
use crate::game::FIELD_W;
use crate::levels;
use crate::render::{
    dim, draw_brick, draw_field, with_alpha, Effects, View, ACCENT, BACKDROP, NEON, TEXT,
};
use crate::t;

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
            let text = t!("select.invalid", error = e);
            draw_wrapped(&view, &text, 40.0, 690.0, 18.0, ERROR);
        }
    }
    draw_buttons(&view, &select_buttons(), pointer);
    let folder = t!("select.folder", path = app.store.dir().display());
    draw_wrapped(&view, &folder, 20.0, 890.0, 16.0, with_alpha(TEXT, 0.7));
}
