//! The "save video" dialog: choosing a file name and a folder for the last
//! round's recording, with a question before an existing file is replaced.
//! Window-independent like the rest of the UI logic; `screens.rs` draws it.

use std::path::PathBuf;

use macroquad::math::Rect;

use crate::t;
use crate::ui::{button_row, clicked_button, Button, Input, Key, TextField};

/// "Save video" on the game over and victory screens.
pub const VIDEO_BUTTON: Rect = Rect {
    x: 140.0,
    y: 690.0,
    w: 260.0,
    h: 52.0,
};

pub const PANEL: Rect = Rect {
    x: 30.0,
    y: 280.0,
    w: 480.0,
    h: 430.0,
};

const MAX_DIR_LEN: usize = 300;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Dir,
}

/// Text boxes of the dialog: file name and folder.
pub fn field_rect(field: Field) -> Rect {
    let y = match field {
        Field::Name => PANEL.y + 110.0,
        Field::Dir => PANEL.y + 205.0,
    };
    Rect::new(PANEL.x + 20.0, y, PANEL.w - 40.0, 46.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Save,
    Cancel,
}

/// "Save"/"Cancel", or "Yes, overwrite"/"No" while confirming.
pub fn buttons(confirming: bool) -> Vec<Button<Action>> {
    let area = Rect::new(PANEL.x + 20.0, PANEL.bottom() - 70.0, PANEL.w - 40.0, 50.0);
    let items = if confirming {
        [
            (t!("video.overwrite_yes"), Action::Save),
            (t!("common.no"), Action::Cancel),
        ]
    } else {
        [
            (t!("common.save"), Action::Save),
            (t!("common.cancel"), Action::Cancel),
        ]
    };
    button_row(area, &items)
}

/// What the dialog wants after a frame of input.
#[derive(Debug, PartialEq, Eq)]
pub enum DialogResult {
    Open,
    Cancelled,
    /// Save the video to this file (overwriting is already confirmed).
    Save(PathBuf),
}

pub struct SaveDialog {
    pub name: TextField,
    pub dir: TextField,
    pub focus: Field,
    pub error: Option<String>,
    /// The chosen file exists; waiting for "overwrite?" to be answered.
    pub overwrite: Option<PathBuf>,
}

impl SaveDialog {
    /// A dialog with a name from the current time and the given folder.
    pub fn new(dir: &str) -> Self {
        let name = crate::video::default_name(std::time::SystemTime::now());
        SaveDialog {
            name: TextField::new(&name, crate::video::MAX_NAME_LEN),
            dir: TextField::new(dir, MAX_DIR_LEN),
            focus: Field::Name,
            error: None,
            overwrite: None,
        }
    }

    pub fn handle(&mut self, input: &Input) -> DialogResult {
        let button = clicked_button(&buttons(self.overwrite.is_some()), input);
        let ok = button == Some(Action::Save) || input.key(Key::Enter);
        let cancel = button == Some(Action::Cancel) || input.key(Key::Escape);

        if let Some(path) = &self.overwrite {
            if ok {
                return DialogResult::Save(path.clone());
            }
            if cancel {
                // "No": back to editing the name.
                self.overwrite = None;
            }
            return DialogResult::Open;
        }
        if cancel {
            return DialogResult::Cancelled;
        }
        if input.key(Key::Tab) {
            self.focus = match self.focus {
                Field::Name => Field::Dir,
                Field::Dir => Field::Name,
            };
        }
        for field in [Field::Name, Field::Dir] {
            if input.clicked(field_rect(field)) {
                self.focus = field;
            }
        }
        match self.focus {
            Field::Name => self.name.handle(input),
            Field::Dir => self.dir.handle(input),
        };
        if ok {
            match crate::video::output_path(&self.dir.text, &self.name.text) {
                Err(e) => self.error = Some(e),
                Ok(path) if path.exists() => {
                    self.error = None;
                    self.overwrite = Some(path);
                }
                Ok(path) => return DialogResult::Save(path),
            }
        }
        DialogResult::Open
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::temp_dir;

    fn key(k: Key) -> Input {
        Input {
            keys: vec![k],
            ..Default::default()
        }
    }

    fn typed(text: &str) -> Input {
        Input {
            chars: text.chars().collect(),
            ..Default::default()
        }
    }

    #[test]
    fn typing_goes_to_the_focused_field() {
        let mut d = SaveDialog::new("/videos");
        assert!(d.name.text.starts_with("rustout-"));
        d.name.text.clear();
        d.handle(&typed("klipp"));
        d.handle(&key(Key::Tab));
        d.handle(&typed("/x"));
        assert_eq!(
            (d.name.text.as_str(), d.dir.text.as_str()),
            ("klipp", "/videos/x")
        );
        let click = Input {
            pointer: Some(field_rect(Field::Name).center()),
            left_pressed: true,
            left_down: true,
            ..Default::default()
        };
        d.handle(&click);
        assert_eq!(d.focus, Field::Name);
        assert_eq!(d.handle(&key(Key::Escape)), DialogResult::Cancelled);
    }

    #[test]
    fn saving_validates_and_confirms_overwrites() {
        let dir = temp_dir("video-dialog");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("old.mp4"), "x").unwrap();
        let mut d = SaveDialog::new(&dir.display().to_string());

        d.name.text = "a/b".into();
        assert_eq!(d.handle(&key(Key::Enter)), DialogResult::Open);
        assert!(d.error.is_some(), "invalid name");

        d.name.text = "new".into();
        assert_eq!(
            d.handle(&key(Key::Enter)),
            DialogResult::Save(dir.join("new.mp4"))
        );

        d.name.text = "old".into();
        assert_eq!(d.handle(&key(Key::Enter)), DialogResult::Open);
        assert_eq!(d.overwrite, Some(dir.join("old.mp4")));
        assert_eq!(d.handle(&key(Key::Escape)), DialogResult::Open, "No");
        assert!(d.overwrite.is_none());
        d.handle(&key(Key::Enter));
        let yes = Input {
            pointer: Some(buttons(true)[0].rect.center()),
            left_pressed: true,
            left_down: true,
            ..Default::default()
        };
        assert_eq!(d.handle(&yes), DialogResult::Save(dir.join("old.mp4")));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
