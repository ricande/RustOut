//! The player's settings, stored as `settings.json` in the game's data
//! folder. Today that is only the language.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::i18n::Lang;
use crate::t;

#[derive(Default, Serialize, Deserialize)]
struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    language: Option<String>,
}

pub fn default_path() -> PathBuf {
    crate::store::data_dir().join("settings.json")
}

/// The language to start in: `RUSTOUT_LANG` if set, else the saved choice,
/// else English.
pub fn load_language(path: &Path) -> Lang {
    if let Some(lang) = std::env::var("RUSTOUT_LANG")
        .ok()
        .and_then(|code| Lang::from_code(&code))
    {
        return lang;
    }
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
        .and_then(|s| s.language)
        .and_then(|code| Lang::from_code(&code))
        .unwrap_or_default()
}

pub fn save_language(path: &Path, lang: Lang) -> Result<(), String> {
    let settings = Settings {
        language: Some(lang.code().to_owned()),
    };
    let text = serde_json::to_string_pretty(&settings).expect("settings serialize") + "\n";
    let write = || -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, text)
    };
    write().map_err(|e| t!("settings.save_failed", error = e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_is_saved_and_loaded() {
        let dir = crate::store::tests::temp_dir("settings");
        let path = dir.join("sub/settings.json");
        fs::create_dir_all(&dir).unwrap();
        if std::env::var_os("RUSTOUT_LANG").is_none() {
            assert_eq!(load_language(&path), Lang::En, "default is English");
            save_language(&path, Lang::Sv).unwrap();
            assert_eq!(load_language(&path), Lang::Sv);
            assert!(fs::read_to_string(&path)
                .unwrap()
                .contains("\"language\": \"sv\""));
            fs::write(&path, "{ broken").unwrap();
            assert_eq!(load_language(&path), Lang::En, "a broken file is ignored");
        }
        // A folder that cannot be created gives an error, not a crash.
        fs::write(dir.join("file"), "x").unwrap();
        assert!(save_language(&dir.join("file/settings.json"), Lang::Sv).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
