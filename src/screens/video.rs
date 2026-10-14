//! The video UI drawn on top of every screen: the REC marker (window only,
//! never in the video), "Save video", the save dialog and the result message.

use macroquad::prelude::*;

use super::{draw_button, draw_buttons, draw_text_field, draw_wrapped, hovered, ERROR, PANEL_BG};
use crate::app::App;
use crate::game::{State, FIELD_H, FIELD_W};
use crate::render::{with_alpha, Effects, View, ACCENT, NEON, TEXT};
use crate::t;
use crate::video_dialog::{self, Field, SaveDialog, VIDEO_BUTTON};

pub fn draw_video_ui(app: &App, view: View, fx: &Effects, pointer: Option<Vec2>, recording: bool) {
    let v = &view;
    if recording && app.in_game() {
        draw_rec_marker(v, fx.blink() || app.game.state == State::Paused);
    }
    if app.in_game() && app.round_over() && app.video_dialog.is_none() {
        draw_round_end(v, app, pointer);
    }
    if app.video_saving {
        let cx = FIELD_W / 2.0;
        v.text_centered(t!("video.saving"), cx, FIELD_H - 24.0, 20.0, ACCENT);
    }
    if let Some(d) = &app.video_dialog {
        draw_save_dialog(v, d, fx, pointer);
    } else if let Some((text, error)) = &app.video_message {
        draw_message(v, text, *error);
    }
}

fn draw_rec_marker(v: &View, lit: bool) {
    let dot = if lit { ERROR } else { with_alpha(ERROR, 0.3) };
    v.circle(Vec2::new(478.0, 80.0), 6.0, dot);
    v.text("REC", 490.0, 87.0, 18.0, ERROR);
}

/// "Save video" after game over or a win, or why it is not possible.
fn draw_round_end(v: &View, app: &App, pointer: Option<Vec2>) {
    if app.can_save_video() {
        let hot = hovered(VIDEO_BUTTON, pointer);
        draw_button(v, VIDEO_BUTTON, t!("video.button"), hot);
    } else if let Some(why) = &app.video_unavailable {
        let y = VIDEO_BUTTON.y + 20.0;
        draw_wrapped(v, why, 40.0, y, 16.0, with_alpha(TEXT, 0.6));
    }
}

fn draw_save_dialog(v: &View, d: &SaveDialog, fx: &Effects, pointer: Option<Vec2>) {
    let cx = FIELD_W / 2.0;
    let p = video_dialog::PANEL;
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
            (Field::Name, t!("video.name_label"), &d.name.text),
            (Field::Dir, t!("video.folder_label"), &d.dir.text),
        ] {
            let r = video_dialog::field_rect(field);
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
    draw_buttons(v, &video_dialog::buttons(confirming), pointer);
}

/// The result of a save or a recording error.
fn draw_message(v: &View, text: &str, error: bool) {
    let cx = FIELD_W / 2.0;
    let p = Rect::new(30.0, 380.0, 480.0, 220.0);
    v.rect(0.0, 0.0, FIELD_W, FIELD_H, with_alpha(BLACK, 0.5));
    v.rect(p.x, p.y, p.w, p.h, PANEL_BG);
    let color = if error { ERROR } else { NEON };
    v.rect_lines(p.x, p.y, p.w, p.h, 2.0, color);
    let title = if error {
        t!("modal.error")
    } else {
        t!("modal.done")
    };
    v.text_centered(title, cx, p.y + 44.0, 30.0, color);
    draw_wrapped(v, text, p.x + 24.0, p.y + 90.0, 18.0, TEXT);
    let ok = t!("common.enter_ok");
    v.text_centered(ok, cx, p.bottom() - 20.0, 16.0, with_alpha(TEXT, 0.6));
}
