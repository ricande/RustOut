//! Screens and the transitions between them: main menu, custom level
//! selection, the game, the editor and test play from the editor. It also
//! opens the "save video" dialog (`video_dialog.rs`) and the language switch.
//! Like the game and the editor, this is driven by abstract `Input`;
//! recording and saving videos is done by `capture.rs`.

use std::path::PathBuf;

use macroquad::math::Rect;

use crate::editor::{Editor, Outcome};
use crate::game::{Command, Controls, Game, State};
use crate::levels::{self, Layout};
use crate::store::{self, LevelStore};
use crate::t;
use crate::ui::{
    button_row, clicked_button, Button, Input, Key, ListItem, ListResponse, ListState,
};
use crate::video_dialog::{DialogResult, SaveDialog, VIDEO_BUTTON};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Menu,
    /// Choosing a custom level to play.
    Select,
    Game,
    Editor,
    /// Playing the editor's current layout; returns to the editor.
    TestPlay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItem {
    Play,
    Custom,
    Editor,
    SaveVideo,
    /// Switches between English and Swedish.
    Language,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectButton {
    Play,
    Back,
}

/// The menu; "Save last video" appears while a recording can be saved.
pub fn menu_buttons(video: bool) -> Vec<Button<MenuItem>> {
    let mut items = vec![
        (t!("menu.play"), MenuItem::Play),
        (t!("menu.custom"), MenuItem::Custom),
        (t!("menu.editor"), MenuItem::Editor),
    ];
    if video {
        items.push((t!("menu.save_video"), MenuItem::SaveVideo));
    }
    items.push((t!("menu.language"), MenuItem::Language));
    items
        .into_iter()
        .enumerate()
        .map(|(i, (label, action))| Button {
            rect: Rect::new(130.0, 520.0 + i as f32 * 58.0, 280.0, 48.0),
            label,
            action,
        })
        .collect()
}

pub fn select_buttons() -> Vec<Button<SelectButton>> {
    button_row(
        Rect::new(40.0, 800.0, 460.0, 54.0),
        &[
            (t!("menu.play"), SelectButton::Play),
            (t!("common.back"), SelectButton::Back),
        ],
    )
}

pub fn select_area() -> Rect {
    Rect::new(40.0, 150.0, 460.0, 14.0 * crate::ui::LIST_ROW_H)
}

pub struct App {
    pub screen: Screen,
    pub game: Game,
    pub editor: Editor,
    pub store: LevelStore,
    pub menu_selected: usize,
    pub select: ListState<Option<Layout>>,
    /// Error shown on the selection screen (e.g. unreadable directory).
    pub select_error: Option<String>,
    /// Counts rounds; a new value tells `main` to start a new recording
    /// (and drop an unsaved one).
    pub round: u64,
    /// The last round's recording can be saved.
    pub video_ready: bool,
    /// A video is being written in the background.
    pub video_saving: bool,
    /// Why rounds cannot be recorded (e.g. ffmpeg is missing).
    pub video_unavailable: Option<String>,
    pub video_dialog: Option<SaveDialog>,
    /// Result of the last save, or a recording error: (text, is_error).
    pub video_message: Option<(String, bool)>,
    /// Folder offered in the save dialog; remembers the last choice.
    pub video_dir: String,
    /// Files `main` should save the recording to.
    pub video_requests: Vec<PathBuf>,
    /// Where the language choice is saved (`None`: not saved, e.g. in tests).
    pub settings_path: Option<PathBuf>,
    /// Problem saving the settings, shown in the menu.
    pub settings_error: Option<String>,
}

impl App {
    pub fn new(store: LevelStore) -> Self {
        App {
            screen: Screen::Menu,
            game: Game::new(levels::builtin()),
            editor: Editor::new(),
            store,
            menu_selected: 0,
            select: ListState::new(Vec::new(), select_area()),
            select_error: None,
            round: 0,
            video_ready: false,
            video_saving: false,
            video_unavailable: None,
            video_dialog: None,
            video_message: None,
            video_dir: crate::video::display_path(&crate::video::default_dir()),
            video_requests: Vec::new(),
            settings_path: None,
            settings_error: None,
        }
    }

    /// The round is over (or was left), so a recording can be offered.
    pub fn round_over(&self) -> bool {
        !self.in_game() || matches!(self.game.state, State::GameOver | State::Victory)
    }

    /// Whether "Save video" is offered right now.
    pub fn can_save_video(&self) -> bool {
        self.video_ready && !self.video_saving && self.round_over()
    }

    pub fn open_video_dialog(&mut self) {
        self.video_dialog = Some(SaveDialog::new(&self.video_dir));
    }

    /// Called by `main` when a save has finished.
    pub fn video_saved(&mut self, result: Result<PathBuf, String>) {
        self.video_saving = false;
        self.video_message = Some(match result {
            Ok(path) => (
                t!("video.saved", path = crate::video::display_path(&path)),
                false,
            ),
            // The recording is kept, so the player can try again.
            Err(e) => (t!("video.save_failed", error = e), true),
        });
    }

    fn handle_video_dialog(&mut self, input: &Input) {
        let Some(dialog) = &mut self.video_dialog else {
            return;
        };
        match dialog.handle(input) {
            DialogResult::Open => {}
            DialogResult::Cancelled => self.video_dialog = None,
            DialogResult::Save(path) => {
                // Remember the folder for the next video.
                self.video_dir = dialog.dir.text.trim().to_owned();
                self.video_dialog = None;
                self.video_saving = true;
                self.video_requests.push(path);
            }
        }
    }

    pub fn in_game(&self) -> bool {
        matches!(self.screen, Screen::Game | Screen::TestPlay)
    }

    /// Handles one frame of input.
    pub fn handle(&mut self, input: &Input) {
        if self.video_dialog.is_some() {
            self.handle_video_dialog(input);
            return;
        }
        if self.video_message.is_some() {
            let dismiss = [Key::Enter, Key::Escape, Key::Space]
                .iter()
                .any(|&k| input.key(k));
            if dismiss || input.left_pressed {
                self.video_message = None;
            }
            return;
        }
        match self.screen {
            Screen::Menu => self.handle_menu(input),
            Screen::Select => self.handle_select(input),
            Screen::Game | Screen::TestPlay => self.handle_game(input),
            Screen::Editor => match self.editor.handle(input, &self.store) {
                Some(Outcome::Leave) => self.screen = Screen::Menu,
                Some(Outcome::TestPlay) => self.start_test_play(),
                None => {}
            },
        }
    }

    /// Advances the physics by one fixed step (only while playing).
    pub fn update(&mut self, dt: f32, controls: &Controls) {
        if self.in_game() {
            self.game.update(dt, controls);
        }
    }

    fn handle_menu(&mut self, input: &Input) {
        let buttons = menu_buttons(self.can_save_video());
        self.menu_selected = self.menu_selected.min(buttons.len() - 1);
        if input.key(Key::Up) {
            self.menu_selected = self.menu_selected.saturating_sub(1);
        }
        if input.key(Key::Down) {
            self.menu_selected = (self.menu_selected + 1).min(buttons.len() - 1);
        }
        if let Some(p) = input.pointer {
            if let Some(i) = buttons.iter().position(|b| b.rect.contains(p)) {
                self.menu_selected = i;
            }
        }
        let chosen = clicked_button(&buttons, input).or_else(|| {
            (input.key(Key::Enter) || input.key(Key::Space))
                .then(|| buttons[self.menu_selected].action)
        });
        match chosen {
            Some(MenuItem::Play) => self.start_game(levels::builtin()),
            Some(MenuItem::Custom) => self.open_select(),
            Some(MenuItem::Editor) => self.screen = Screen::Editor,
            Some(MenuItem::SaveVideo) => self.open_video_dialog(),
            Some(MenuItem::Language) => self.switch_language(),
            None => {}
        }
    }

    /// Switches to the next language and remembers the choice.
    pub fn switch_language(&mut self) {
        let lang = crate::i18n::lang().next();
        crate::i18n::set_lang(lang);
        // The new-level name follows the language until the player edits.
        if !self.editor.dirty && self.editor.file.is_none() {
            self.editor = Editor::new();
        }
        self.settings_error = self
            .settings_path
            .as_ref()
            .and_then(|path| crate::settings::save_language(path, lang).err());
    }

    fn start_game(&mut self, levels: Vec<Layout>) {
        self.game = Game::new(levels);
        self.screen = Screen::Game;
        self.round += 1;
    }

    pub fn open_select(&mut self) {
        let (items, error) = match self.store.list() {
            Ok(stored) => (
                stored
                    .into_iter()
                    .map(|s| match s.level {
                        Ok(layout) => ListItem {
                            label: layout.name.clone(),
                            value: Some(layout),
                            error: None,
                        },
                        Err(e) => ListItem {
                            label: t!(
                                "list.broken",
                                file = s.path.file_name().unwrap_or_default().to_string_lossy()
                            ),
                            value: None,
                            error: Some(e),
                        },
                    })
                    .collect(),
                None,
            ),
            Err(e) => (Vec::new(), Some(e)),
        };
        self.select = ListState::new(items, select_area());
        self.select_error = error;
        self.screen = Screen::Select;
    }

    fn handle_select(&mut self, input: &Input) {
        let button = clicked_button(&select_buttons(), input);
        if button == Some(SelectButton::Back) || input.key(Key::Escape) {
            self.screen = Screen::Menu;
            return;
        }
        let activated = match self.select.handle(input) {
            ListResponse::Activate(i) => Some(i),
            ListResponse::None => {
                (button == Some(SelectButton::Play)).then_some(self.select.selected)
            }
        };
        // Broken files are listed with their error and simply cannot be played.
        if let Some(Some(layout)) = activated
            .and_then(|i| self.select.items.get(i))
            .map(|item| item.value.clone())
        {
            self.start_game(vec![layout]);
        }
    }

    fn handle_game(&mut self, input: &Input) {
        if self.can_save_video() && (input.key(Key::V) || input.clicked(VIDEO_BUTTON)) {
            self.open_video_dialog();
            return;
        }
        if input.key(Key::Escape) {
            self.screen = match self.screen {
                Screen::TestPlay => Screen::Editor,
                _ => Screen::Menu,
            };
            return;
        }
        let ended = matches!(self.game.state, State::GameOver | State::Victory);
        let launch = input.key(Key::Space) || input.left_pressed && input.pointer.is_some();
        if launch || input.key(Key::R) {
            // Restarting begins a new round (and a new recording).
            if input.key(Key::R) || ended {
                self.round += 1;
            }
        }
        if launch {
            self.game.command(Command::Launch);
        }
        if input.key(Key::P) {
            self.game.command(Command::TogglePause);
        }
        if input.key(Key::R) {
            self.game.command(Command::Restart);
        }
    }

    /// Plays a copy of the editor's layout. The editor itself, including
    /// unsaved changes, is left untouched until the player returns.
    pub fn start_test_play(&mut self) {
        if store::validate(&self.editor.layout).is_ok() {
            self.game = Game::new(vec![self.editor.layout.clone()]);
            self.screen = Screen::TestPlay;
            self.round += 1;
        }
    }

    /// Text for the "go back" hint in game overlays.
    pub fn back_hint(&self) -> &'static str {
        match self.screen {
            Screen::TestPlay => t!("game.back_editor"),
            _ => t!("game.back_menu"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Modal;
    use crate::levels::Grid;
    use crate::video_dialog;
    use macroquad::math::Rect;

    fn cell_rect(row: usize, col: usize) -> Rect {
        Grid::DEFAULT.cell_rect(row, col)
    }
    use crate::store::tests::temp_dir;
    use macroquad::math::Vec2;

    fn key(k: Key) -> Input {
        Input {
            keys: vec![k],
            ..Default::default()
        }
    }

    fn click(p: Vec2) -> Input {
        Input {
            pointer: Some(p),
            left_pressed: true,
            left_down: true,
            ..Default::default()
        }
    }

    fn button_center<A: PartialEq>(buttons: Vec<Button<A>>, action: A) -> Vec2 {
        buttons
            .into_iter()
            .find(|b| b.action == action)
            .unwrap()
            .rect
            .center()
    }

    #[test]
    fn menu_starts_builtin_game_and_escape_returns() {
        let mut app = App::new(LevelStore::new(temp_dir("app-menu")));
        app.handle(&key(Key::Space));
        assert_eq!(app.screen, Screen::Game);
        assert_eq!(app.game.level_count(), 3);
        app.handle(&key(Key::Escape));
        assert_eq!(app.screen, Screen::Menu);
    }

    #[test]
    fn editor_test_play_round_trip_keeps_unsaved_layout() {
        let mut app = App::new(LevelStore::new(temp_dir("app-test")));
        app.handle(&click(button_center(menu_buttons(false), MenuItem::Editor)));
        assert_eq!(app.screen, Screen::Editor);

        // Testing an empty level is refused with a message.
        app.handle(&key(Key::T));
        assert_eq!(app.screen, Screen::Editor);
        assert!(matches!(
            app.editor.modal,
            Modal::Message { error: true, .. }
        ));
        app.handle(&key(Key::Enter));

        app.handle(&click(cell_rect(2, 3).center()));
        app.handle(&Input::default());
        app.handle(&key(Key::Num3));
        app.handle(&click(cell_rect(4, 8).center()));
        app.handle(&Input::default());
        let before = app.editor.layout.clone();
        assert!(app.editor.dirty);

        app.handle(&key(Key::T));
        assert_eq!(app.screen, Screen::TestPlay);
        assert_eq!(app.game.level_count(), 1);
        let hp: Vec<u8> = app.game.bricks.iter().map(|b| b.hp).collect();
        assert_eq!(hp, vec![1, 3]);
        assert_eq!(app.game.bricks[0].rect, cell_rect(2, 3), "same coordinates");

        // Play a bit; the click that launches must not paint anything.
        app.handle(&key(Key::Space));
        for _ in 0..240 {
            app.update(1.0 / 120.0, &Controls::default());
        }
        app.handle(&click(cell_rect(0, 0).center()));

        app.handle(&key(Key::Escape));
        assert_eq!(app.screen, Screen::Editor);
        assert_eq!(app.editor.layout, before);
        assert!(app.editor.dirty, "unsaved changes survive test play");
        assert_eq!(app.back_hint(), "Esc: menu");
    }

    #[test]
    fn custom_levels_are_selectable_and_broken_ones_are_not() {
        let dir = temp_dir("app-select");
        let store = LevelStore::new(dir.clone());
        let layout = Layout::from_rows("Egen bana", &["3.3.3.3.3.3."]).unwrap();
        store.save(&layout, &store.path_for("Egen bana")).unwrap();
        std::fs::write(dir.join("zz-trasig.json"), "[]").unwrap();

        let mut app = App::new(store);
        app.handle(&key(Key::Down));
        app.handle(&key(Key::Enter));
        assert_eq!(app.screen, Screen::Select);
        assert_eq!(app.select.items.len(), 2);
        assert!(app.select.items[1].error.is_some());

        app.handle(&key(Key::Down));
        app.handle(&key(Key::Enter));
        assert_eq!(app.screen, Screen::Select, "broken file cannot be played");

        app.handle(&key(Key::Up));
        app.handle(&click(button_center(select_buttons(), SelectButton::Play)));
        assert_eq!(app.screen, Screen::Game);
        assert_eq!(app.game.level_name(), "Egen bana");
        assert_eq!(app.game.bricks.len(), 6);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn clicks_outside_the_view_do_nothing() {
        let mut app = App::new(LevelStore::new(temp_dir("app-outside")));
        let outside = Input {
            pointer: None,
            left_pressed: true,
            left_down: true,
            ..Default::default()
        };
        app.handle(&outside);
        assert_eq!(app.screen, Screen::Menu);
        app.handle(&key(Key::Space));
        app.handle(&outside);
        assert_eq!(app.game.state, State::Serve, "no launch from the side bars");
    }

    fn chars(text: &str) -> Input {
        Input {
            chars: text.chars().collect(),
            ..Default::default()
        }
    }

    /// An app whose game just ended, with a recording ready.
    fn finished_round(dir: &std::path::Path) -> App {
        let mut app = App::new(LevelStore::new(temp_dir("app-video")));
        app.handle(&key(Key::Space));
        app.video_dir = dir.display().to_string();
        app.game.state = State::GameOver;
        app.video_ready = true;
        app
    }

    #[test]
    fn rounds_are_counted_for_recordings() {
        let mut app = App::new(LevelStore::new(temp_dir("app-rounds")));
        assert_eq!(app.round, 0);
        app.handle(&key(Key::Space)); // menu: play
        assert_eq!(app.round, 1);
        app.handle(&key(Key::Space)); // launch: same round
        app.handle(&key(Key::P));
        app.handle(&key(Key::P));
        assert_eq!(app.round, 1);
        app.handle(&key(Key::R)); // restart: new round
        assert_eq!(app.round, 2);
        app.game.state = State::GameOver;
        app.handle(&key(Key::Space)); // play again: new round
        assert_eq!(app.round, 3);
        app.handle(&key(Key::Escape));
        assert!(app.round_over(), "leaving ends the round");
    }

    #[test]
    fn video_is_offered_only_after_the_round() {
        let dir = temp_dir("app-video-offer");
        let mut app = finished_round(&dir);
        app.game.state = State::Playing;
        app.handle(&key(Key::V));
        assert!(app.video_dialog.is_none(), "not during play");
        app.game.state = State::GameOver;
        app.video_ready = false;
        app.handle(&key(Key::V));
        assert!(app.video_dialog.is_none(), "nothing recorded");

        app.video_ready = true;
        // Clicking the button opens the dialog and does not restart the game.
        app.handle(&click(VIDEO_BUTTON.center()));
        assert_eq!(app.game.state, State::GameOver);
        let d = app.video_dialog.as_ref().unwrap();
        assert!(d.name.text.starts_with("rustout-"), "{}", d.name.text);
        assert_eq!(d.dir.text, dir.display().to_string());
        app.handle(&key(Key::Escape));
        assert!(app.video_dialog.is_none());

        // After leaving with Esc the menu offers it.
        app.handle(&key(Key::Escape));
        assert_eq!(app.screen, Screen::Menu);
        let buttons = menu_buttons(app.can_save_video());
        assert!(buttons.iter().any(|b| b.action == MenuItem::SaveVideo));
        app.handle(&click(button_center(buttons, MenuItem::SaveVideo)));
        assert!(app.video_dialog.is_some());
        assert!(!menu_buttons(false)
            .iter()
            .any(|b| b.action == MenuItem::SaveVideo));
    }

    #[test]
    fn save_dialog_names_the_file_and_asks_before_overwriting() {
        let dir = temp_dir("app-video-save");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("finns.mp4"), "old").unwrap();
        let mut app = finished_round(&dir);
        app.handle(&key(Key::V));

        // Type a name; Tab switches to the folder field.
        app.video_dialog.as_mut().unwrap().name.text.clear();
        app.handle(&chars("min/runda"));
        app.handle(&key(Key::Enter));
        let err = app.video_dialog.as_ref().unwrap().error.clone().unwrap();
        assert!(err.contains("/"), "{err}");
        for _ in 0..9 {
            app.handle(&key(Key::Backspace));
        }
        app.handle(&chars("finns"));
        app.handle(&key(Key::Tab));
        assert_eq!(
            app.video_dialog.as_ref().unwrap().focus,
            video_dialog::Field::Dir
        );
        app.handle(&key(Key::Enter));
        assert_eq!(
            app.video_dialog.as_ref().unwrap().overwrite,
            Some(dir.join("finns.mp4"))
        );
        app.handle(&key(Key::Escape)); // "Nej": back to editing the name
        assert!(app.video_dialog.as_ref().unwrap().overwrite.is_none());
        assert!(app.video_requests.is_empty());

        app.handle(&key(Key::Enter));
        app.handle(&key(Key::Enter)); // "Ja, skriv över"
        assert!(app.video_dialog.is_none());
        assert_eq!(app.video_requests, vec![dir.join("finns.mp4")]);
        assert!(app.video_saving);
        assert!(!app.can_save_video(), "no second save while saving");

        app.video_saved(Ok(dir.join("finns.mp4")));
        assert!(!app.video_saving);
        let (text, error) = app.video_message.clone().unwrap();
        assert!(!error && text.contains("finns.mp4"), "{text}");
        app.handle(&key(Key::Enter));
        assert!(app.video_message.is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn save_dialog_uses_and_remembers_the_chosen_folder() {
        let dir = temp_dir("app-video-folder");
        let mut app = finished_round(&dir);
        app.handle(&key(Key::V));
        app.handle(&click(
            video_dialog::field_rect(video_dialog::Field::Dir).center(),
        ));
        app.handle(&chars("/klipp"));
        app.handle(&click(video_dialog::buttons(false)[0].rect.center()));
        let expected_dir = dir.join("klipp");
        assert_eq!(app.video_requests.len(), 1);
        assert_eq!(app.video_requests[0].parent(), Some(expected_dir.as_path()));
        assert_eq!(app.video_dir, expected_dir.display().to_string());

        // A failed save keeps the recording so it can be retried.
        app.video_saved(Err("disken är full".into()));
        assert!(app.video_message.as_ref().unwrap().1);
        app.handle(&key(Key::Enter));
        assert!(app.can_save_video());
        app.handle(&key(Key::V));
        assert_eq!(
            app.video_dialog.as_ref().unwrap().dir.text,
            expected_dir.display().to_string()
        );
    }

    #[test]
    fn menu_switches_language_and_saves_the_choice() {
        use crate::i18n::{lang, Lang};
        let dir = temp_dir("app-language");
        let mut app = App::new(LevelStore::new(dir.join("levels")));
        app.settings_path = Some(dir.join("settings.json"));
        assert_eq!(lang(), Lang::En);
        let item = |app: &App| {
            menu_buttons(app.can_save_video())
                .into_iter()
                .find(|b| b.action == MenuItem::Language)
                .unwrap()
        };
        assert_eq!(item(&app).label, "Language: English");
        app.handle(&click(item(&app).rect.center()));
        assert_eq!(lang(), Lang::Sv);
        assert_eq!(item(&app).label, "Språk: Svenska");
        assert_eq!(menu_buttons(false)[0].label, "Spela");
        assert_eq!(app.editor.layout.name, "Ny bana");
        assert_eq!(levels::builtin()[1].name, "Diamanten");
        assert!(std::fs::read_to_string(dir.join("settings.json"))
            .unwrap()
            .contains("sv"));
        app.handle(&click(item(&app).rect.center()));
        assert_eq!(lang(), Lang::En);
        assert_eq!(app.back_hint(), "Esc: menu");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
