mod app;
mod audio;
mod capture;
mod editor;
mod game;
mod i18n;
mod levels;
mod render;
mod screen;
mod screens;
mod script;
mod settings;
mod store;
mod ui;
mod video;

use macroquad::prelude::*;

use app::{App, Screen};
use game::Controls;
use store::LevelStore;
use ui::{Input, Key};

/// Fixed physics time step.
const DT: f32 = 1.0 / 120.0;
/// Upper bound on simulated time per frame, so a hitch never causes a
/// burst of catch-up steps.
const MAX_FRAME_TIME: f32 = 0.25;

const KEYS: [(KeyCode, Key); 22] = [
    (KeyCode::Up, Key::Up),
    (KeyCode::Down, Key::Down),
    (KeyCode::Left, Key::Left),
    (KeyCode::Right, Key::Right),
    (KeyCode::Enter, Key::Enter),
    (KeyCode::Escape, Key::Escape),
    (KeyCode::Space, Key::Space),
    (KeyCode::Backspace, Key::Backspace),
    (KeyCode::P, Key::P),
    (KeyCode::R, Key::R),
    (KeyCode::N, Key::N),
    (KeyCode::O, Key::O),
    (KeyCode::S, Key::S),
    (KeyCode::T, Key::T),
    (KeyCode::F2, Key::F2),
    (KeyCode::G, Key::G),
    (KeyCode::V, Key::V),
    (KeyCode::Tab, Key::Tab),
    (KeyCode::Key1, Key::Num1),
    (KeyCode::Key2, Key::Num2),
    (KeyCode::Key3, Key::Num3),
    (KeyCode::Key4, Key::Num4),
];

/// `RUSTOUT_WINDOW=900x500` forces a window size (e.g. to check letterboxing).
fn forced_window_size() -> Option<(i32, i32)> {
    let value = std::env::var("RUSTOUT_WINDOW").ok()?;
    let (w, h) = value.split_once('x')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

fn window_conf() -> Conf {
    let (width, height) = forced_window_size()
        .unwrap_or_else(|| screen::initial_window_size(screen::detect_work_area()));
    Conf {
        window_title: "RustOut".to_owned(),
        window_width: width,
        window_height: height,
        window_resizable: true,
        sample_count: 4,
        ..Default::default()
    }
}

/// Collects this frame's input, with the mouse mapped to field coordinates.
fn read_input(view: render::View) -> Input {
    let mouse = mouse_position();
    let mut keys: Vec<Key> = KEYS
        .iter()
        .filter(|(code, _)| is_key_pressed(*code))
        .map(|&(_, key)| key)
        .collect();
    if is_key_pressed(KeyCode::KpEnter) {
        keys.push(Key::Enter);
    }
    let mut chars = Vec::new();
    while let Some(ch) = get_char_pressed() {
        chars.push(ch);
    }
    Input {
        pointer: view.contains(mouse).then(|| view.to_field(mouse)),
        left_pressed: is_mouse_button_pressed(MouseButton::Left),
        left_down: is_mouse_button_down(MouseButton::Left),
        right_pressed: is_mouse_button_pressed(MouseButton::Right),
        right_down: is_mouse_button_down(MouseButton::Right),
        wheel: mouse_wheel().1,
        keys,
        chars,
    }
}

/// The script from `RUSTOUT_SCRIPT`, if any. A broken script stops the
/// program with a message rather than running unscripted.
fn load_script() -> Option<script::Script> {
    let path = std::env::var_os("RUSTOUT_SCRIPT")?;
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("cannot read script {}: {e}", path.to_string_lossy());
        std::process::exit(2)
    });
    match script::Script::parse(&text) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("script error: {e}");
            std::process::exit(2)
        }
    }
}

/// Saves the current frame as `<RUSTOUT_SHOTS or "shots">/<name>.png`.
fn save_shot(name: &str) {
    let dir = std::env::var("RUSTOUT_SHOTS").unwrap_or_else(|_| "shots".into());
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("cannot create {dir}: {e}");
        return;
    }
    let path = format!("{dir}/{name}.png");
    get_screen_data().export_png(&path);
    eprintln!("saved {path}");
}

/// The x of the lowest ball, for the scripted autopilot.
fn lowest_ball_x(game: &game::Game) -> Option<f32> {
    game.balls
        .iter()
        .max_by(|a, b| a.pos.y.total_cmp(&b.pos.y))
        .map(|b| b.pos.x)
}

#[macroquad::main(window_conf)]
async fn main() {
    let sounds = audio::Sounds::load().await;
    let settings_path = settings::default_path();
    i18n::set_lang(settings::load_language(&settings_path));
    let mut app = App::new(LevelStore::new(store::default_dir()));
    app.settings_path = Some(settings_path);
    let mut capture = capture::Capture::new(sounds.samples().to_vec(), &mut app);
    let mut fx = render::Effects::default();
    let mut accumulator = 0.0;
    let mut last_mouse = mouse_position();
    let mut mouse_steering = false;
    let mut script = load_script();
    let mut frame_number = 0u32;

    loop {
        frame_number += 1;
        let view = render::View::fit(screen_width(), screen_height());
        let scripted = script.as_mut().map(|s| s.frame(frame_number));
        let input = match &scripted {
            Some(f) => f.input.clone(),
            None => read_input(view),
        };

        // Whichever input was used most recently steers the paddle.
        // Mouse input only counts inside the playfield, not in the side bars.
        let mouse = mouse_position();
        if mouse != last_mouse {
            last_mouse = mouse;
            mouse_steering = input.pointer.is_some();
        }
        let mut dir = 0.0;
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            dir -= 1.0;
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            dir += 1.0;
        }
        if dir != 0.0 {
            mouse_steering = false;
        }
        let mut controls = Controls {
            dir,
            target_x: mouse_steering.then(|| view.to_field(mouse).x),
        };
        if let Some(f) = &scripted {
            controls = Controls {
                dir: 0.0,
                target_x: if f.autopilot {
                    lowest_ball_x(&app.game)
                } else {
                    input.pointer.map(|p| p.x)
                },
            };
        }

        app.handle(&input);

        // Scripted runs use a fixed frame time, so they are repeatable.
        let frame_time = if scripted.is_some() {
            1.0 / 60.0
        } else {
            get_frame_time().min(MAX_FRAME_TIME)
        };
        accumulator += frame_time;
        while accumulator >= DT {
            app.update(DT, &controls);
            accumulator -= DT;
        }

        let events = app.game.take_events();
        for event in &events {
            sounds.play(event);
            fx.on_event(event);
        }
        fx.update(frame_time, &app.game);
        // Draws this frame into the video too (before the window is drawn).
        capture.frame(&mut app, &fx, frame_time, &events);

        match app.screen {
            Screen::Menu => screens::draw_menu(&app, view, &fx),
            Screen::Select => screens::draw_select(&app, view, input.pointer),
            Screen::Editor => screens::draw_editor(&app.editor, view, &fx, input.pointer),
            Screen::Game | Screen::TestPlay => {
                render::draw(&app.game, &fx, view, app.back_hint(), true)
            }
        }
        screens::draw_video_ui(&app, view, &fx, input.pointer, capture.is_recording());
        capture.poll_saves(&mut app);

        if let Some(f) = &scripted {
            for name in &f.shots {
                save_shot(name);
            }
            let past_end = script
                .as_ref()
                .is_some_and(|s| frame_number > s.last_frame());
            if f.quit || past_end {
                break;
            }
        }
        next_frame().await;
    }
}
