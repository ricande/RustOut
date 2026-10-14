//! Translations. English is the default; Swedish can be chosen in the menu.
//!
//! Every text shown to the player lives in `CATALOG` as (key, English,
//! Swedish). Code refers to texts by key with the `t!` macro, which fills
//! in named placeholders: `t!("hud.score", score = 120)`.
//!
//! The language is kept per thread (the game runs on one thread); a worker
//! thread that produces texts copies the language when it is started.

use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
    Sv,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Sv];

    /// Code used in the settings file and `RUSTOUT_LANG`.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Sv => "sv",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        let code = code.trim().to_lowercase();
        Lang::ALL
            .into_iter()
            .find(|l| code == l.code() || code.starts_with(&format!("{}_", l.code())))
    }

    /// The next language, for the menu's language switch.
    pub fn next(self) -> Lang {
        match self {
            Lang::En => Lang::Sv,
            Lang::Sv => Lang::En,
        }
    }
}

thread_local! {
    static CURRENT: Cell<Lang> = const { Cell::new(Lang::En) };
}

pub fn lang() -> Lang {
    CURRENT.with(Cell::get)
}

pub fn set_lang(lang: Lang) {
    CURRENT.with(|c| c.set(lang));
}

/// The text for `key` in the current language.
pub fn tr(key: &str) -> &'static str {
    tr_in(lang(), key)
}

/// The text for `key` in `lang`. An unknown key is a programming error; in
/// release builds the key itself is shown rather than crashing the game.
pub fn tr_in(lang: Lang, key: &str) -> &'static str {
    static INDEX: OnceLock<HashMap<&'static str, (&'static str, &'static str)>> = OnceLock::new();
    let index = INDEX.get_or_init(|| CATALOG.iter().map(|&(k, en, sv)| (k, (en, sv))).collect());
    match index.get(key) {
        Some(&(en, sv)) => match lang {
            Lang::En => en,
            Lang::Sv => sv,
        },
        None => {
            debug_assert!(false, "missing translation key: {key}");
            CATALOG[0].1
        }
    }
}

/// Fills `{name}` placeholders in a translated text.
pub fn fill(text: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut out = text.to_owned();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), &value.to_string());
    }
    out
}

/// `t!("key")` gives the `&'static str` text; `t!("key", name = value, …)`
/// gives a `String` with the placeholders filled in.
#[macro_export]
macro_rules! t {
    ($key:literal) => {
        $crate::i18n::tr($key)
    };
    ($key:literal, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::fill(
            $crate::i18n::tr($key),
            &[$((stringify!($name), &$value as &dyn ::std::fmt::Display)),+],
        )
    };
}

/// (key, English, Swedish)
const CATALOG: &[(&str, &str, &str)] = &[
    ("missing", "?", "?"),
    // Common words and buttons
    ("common.ok", "OK", "OK"),
    ("common.yes", "Yes", "Ja"),
    ("common.no", "No", "Nej"),
    ("common.save", "Save", "Spara"),
    ("common.cancel", "Cancel", "Avbryt"),
    ("common.back", "Back", "Tillbaka"),
    ("common.enter_ok", "Enter: OK", "Enter: OK"),
    ("common.file_exists", "The file {path} already exists. Do you want to overwrite it?", "Filen {path} finns redan. Vill du skriva över den?"),
    // Main menu
    ("menu.subtitle", "A small arcade game in Rust", "Ett litet arkadspel i Rust"),
    ("menu.play", "Play", "Spela"),
    ("menu.custom", "Custom levels", "Egna banor"),
    ("menu.editor", "Level editor", "Baneditor"),
    ("menu.save_video", "Save last video", "Spara senaste video"),
    ("menu.language", "Language: English", "Språk: Svenska"),
    ("menu.help_steer", "Steer: mouse or left/right arrow", "Styr: mus eller vänster/höger pil"),
    ("menu.help_launch", "Launch: space or click", "Skjut: mellanslag eller klick"),
    ("menu.help_keys", "P: pause   R: restart   Esc: menu", "P: paus   R: börja om   Esc: meny"),
    ("menu.help_choose", "Choose with the arrow keys and Enter", "Välj med piltangenter och Enter"),
    // Custom level list
    ("select.title", "CUSTOM LEVELS", "EGNA BANOR"),
    ("select.empty", "No custom levels yet.", "Inga egna banor ännu."),
    ("select.empty_hint", "Create one in the level editor.", "Skapa en i baneditorn."),
    ("select.invalid", "Invalid file: {error}", "Ogiltig fil: {error}"),
    ("select.folder", "Folder: {path}", "Mapp: {path}"),
    ("list.broken", "{file} (broken)", "{file} (trasig)"),
    ("list.builtin", "{name} (built-in)", "{name} (inbyggd)"),
    ("list.more_up", "^ more ^", "^ fler ^"),
    ("list.more_down", "v more v", "v fler v"),
    // Game
    ("hud.score", "SCORE {score}", "POÄNG {score}"),
    ("hud.level", "LEVEL {level}/{count}", "BANA {level}/{count}"),
    ("hud.wall", "WALL {left}/{total}", "VÄGG {left}/{total}"),
    ("hud.balls", "Balls: {count}", "Bollar: {count}"),
    ("hud.paddle_wider", "Paddle: wider", "Paddel: bredare"),
    ("hud.paddle_widest", "Paddle: widest", "Paddel: bredast"),
    ("hud.speed_slow", "Speed: slow", "Fart: långsam"),
    ("hud.speed_fast", "Speed: fast", "Fart: snabb"),
    ("game.level", "LEVEL {level}", "BANA {level}"),
    ("game.launch_hint", "Space or click: launch", "Mellanslag eller klick: skjut"),
    ("game.paused", "PAUSED", "PAUS"),
    ("game.resume_hint", "Press P to continue", "Tryck P för att fortsätta"),
    ("game.over", "GAME OVER", "GAME OVER"),
    ("game.score", "Score: {score}", "Poäng: {score}"),
    ("game.replay_hint", "Space or R: play again", "Mellanslag eller R: spela igen"),
    ("game.won", "YOU WIN!", "DU VANN!"),
    ("game.all_cleared", "All levels cleared!", "Alla banor klara!"),
    ("game.level_cleared", "Level cleared!", "Banan klar!"),
    ("game.back_menu", "Esc: menu", "Esc: meny"),
    ("game.back_editor", "Esc: back to the editor", "Esc: tillbaka till editorn"),
    // Bonuses
    ("bonus.paddle_plus", "Paddle +", "Paddel +"),
    ("bonus.paddle_minus", "Paddle -", "Paddel -"),
    ("bonus.speed_plus", "Speed +", "Hastighet +"),
    ("bonus.speed_minus", "Speed -", "Hastighet -"),
    ("bonus.multiball", "Multiball", "Multiboll"),
    ("bonus.wall", "Bottom wall", "Bottenvägg"),
    ("bonus.label_paddle_plus", "P+", "P+"),
    ("bonus.label_paddle_minus", "P-", "P-"),
    ("bonus.label_speed_plus", "S+", "H+"),
    ("bonus.label_speed_minus", "S-", "H-"),
    ("bonus.label_multiball", "x3", "x3"),
    ("bonus.label_wall", "W", "V"),
    // Built-in levels
    ("level.warmup", "Warm-up", "Uppvärmning"),
    ("level.diamond", "The Diamond", "Diamanten"),
    ("level.fortress", "The Fortress", "Fästningen"),
    // Level data errors
    ("grid.bad_cols", "Columns must be {min}-{max} (was {value}).", "Antal kolumner måste vara {min}-{max} (var {value})."),
    ("grid.bad_rows", "Rows must be {min}-{max} (was {value}).", "Antal rader måste vara {min}-{max} (var {value})."),
    ("rows.too_many", "Too many rows: {count} (the grid has {rows}).", "För många rader: {count} (rutnätet har {rows})."),
    ("rows.wrong_length", "Row {row} has {len} characters, it should have {cols} (one per column).", "Rad {row} har {len} tecken, ska ha {cols} (en per kolumn)."),
    ("rows.bad_char", "Row {row}, column {col}: invalid character '{ch}' (allowed: . 1 2 3 #).", "Rad {row}, kolumn {col}: ogiltigt tecken '{ch}' (tillåtna: . 1 2 3 #)."),
    ("bonus.place", "Bonus at row {row}, column {col}", "Bonus på rad {row}, kolumn {col}"),
    ("bonus.outside", "{place}: outside the grid.", "{place}: utanför rutnätet."),
    ("bonus.not_brick", "{place}: there is no breakable block there.", "{place}: där finns inget förstörbart block."),
    ("bonus.taken", "{place}: the cell already has a bonus.", "{place}: rutan har redan en bonus."),
    // Level files
    ("store.name_missing", "The level must have a name.", "Banan måste ha ett namn."),
    ("store.name_long", "The name can be at most {max} characters.", "Namnet får vara högst {max} tecken."),
    ("store.name_chars", "The name contains invalid characters.", "Namnet innehåller otillåtna tecken."),
    ("store.no_bricks", "The level needs at least one breakable block.", "Banan måste ha minst ett block som går att förstöra."),
    ("store.bad_json", "Invalid JSON: {error}", "Ogiltig JSON: {error}"),
    ("store.bad_format", "Unknown file format {format} (supported: 1-{max}).", "Okänt filformat {format} (stöds: 1-{max})."),
    ("store.bad_bonus", "Unknown bonus type \"{name}\" (allowed: {list}).", "Okänd bonustyp \"{name}\" (tillåtna: {list})."),
    ("store.bonus_from_1", "Bonus: rows and columns are counted from 1.", "Bonus: rad och kolumn räknas från 1."),
    ("store.read_dir", "Could not read the folder {path}: {error}", "Kunde inte läsa katalogen {path}: {error}"),
    ("store.builtin_name", "\"{name}\" is a built-in level and cannot be overwritten. Choose another name.", "\"{name}\" är en inbyggd bana och kan inte skrivas över. Välj ett annat namn."),
    ("store.create_dir", "Could not create the folder {path}: {error}", "Kunde inte skapa katalogen {path}: {error}"),
    ("store.read_file", "Could not read the file: {error}", "Kunde inte läsa filen: {error}"),
    ("store.too_big", "The file is too large ({size} bytes).", "Filen är för stor ({size} byte)."),
    // Editor
    ("editor.new_name", "New level", "Ny bana"),
    ("editor.copy_name", "{name} copy", "{name} kopia"),
    ("editor.blocks", "Blocks:", "Block:"),
    ("editor.bonuses", "Bonus:", "Bonus:"),
    ("brush.brick_one", "Block, {hits} hit", "Block, {hits} träff"),
    ("brush.brick_many", "Block, {hits} hits", "Block, {hits} träffar"),
    ("brush.stone", "Stone (indestructible)", "Sten (oförstörbar)"),
    ("brush.eraser", "Eraser", "Sudd"),
    ("brush.bonus", "Bonus: {name}", "Bonus: {name}"),
    ("brush.hint", "{brush}  |  left: place, {erase}", "{brush}  |  vänster: placera, {erase}"),
    ("brush.erase_block", "right: erase", "höger: sudda"),
    ("brush.erase_bonus", "right: remove bonus", "höger: ta bort bonus"),
    ("tool.new", "New", "Ny"),
    ("tool.name", "Name", "Namn"),
    ("tool.grid", "Grid", "Rutnät"),
    ("tool.open", "Open", "Öppna"),
    ("tool.test", "Test", "Testa"),
    ("tool.menu", "Menu", "Meny"),
    ("editor.file", "File: {name} (in the level folder)", "Fil: {name} (i banmappen)"),
    ("editor.not_saved", "Not saved yet", "Inte sparad ännu"),
    ("editor.ask_new", "Create a new level?", "Skapa en ny bana?"),
    ("editor.ask_menu", "Go to the menu?", "Gå till menyn?"),
    ("editor.ask_open", "Open another level?", "Öppna en annan bana?"),
    ("editor.unsaved_lost", "{question} Unsaved changes will be lost.", "{question} Osparade ändringar går förlorade."),
    ("editor.cannot_test", "Cannot test play: {error}", "Kan inte testspela: {error}"),
    ("editor.cannot_save", "Cannot save: {error}", "Kan inte spara: {error}"),
    ("editor.saved_to", "Saved to {path}", "Sparad till {path}"),
    ("editor.save_failed", "Saving failed: {error}", "Sparningen misslyckades: {error}"),
    ("editor.open_failed", "Could not open {path}:\n{error}", "Kunde inte öppna {path}:\n{error}"),
    ("editor.invalid_file", "The file is invalid and cannot be opened:\n{error}", "Filen är ogiltig och kan inte öppnas:\n{error}"),
    ("editor.grid_loses", "The {cols} × {rows} grid cannot hold all blocks. Blocks that will be removed: {lost}. Continue?", "Rutnätet {cols} × {rows} får inte plats med alla block: {lost} block utanför tas bort. Fortsätta?"),
    ("modal.name", "Level name", "Banans namn"),
    ("modal.open", "Open level", "Öppna bana"),
    ("modal.grid", "Grid", "Rutnät"),
    ("modal.confirm", "Are you sure?", "Är du säker?"),
    ("modal.error", "Error", "Fel"),
    ("modal.done", "Done", "Klart"),
    ("modal.name_hint", "Type a name (at most {max} characters). The file is saved as {file}.json.", "Skriv ett namn (högst {max} tecken). Filen sparas som {file}.json."),
    ("modal.name_keys", "Enter: OK   Esc: cancel", "Enter: OK   Esc: avbryt"),
    ("grid.columns", "Columns: {value}", "Kolumner: {value}"),
    ("grid.rows", "Rows: {value}", "Rader: {value}"),
    ("grid.block_size", "Block size: {w} × {h} px", "Blockstorlek: {w} × {h} px"),
    ("grid.area", "Build area: {w} × {h} px", "Byggområde: {w} × {h} px"),
    ("grid.lost", "Blocks outside the new grid (removed): {lost}", "{lost} block hamnar utanför och tas bort."),
    ("grid.limits", "Columns {cmin}-{cmax}, rows {rmin}-{rmax}. Arrow keys: left/right = columns, up/down = rows.", "Kolumner {cmin}-{cmax}, rader {rmin}-{rmax}. Piltangenter: vänster/höger = kolumner, upp/ner = rader."),
    // Video
    ("video.title", "Save video", "Spara video"),
    ("video.button", "Save video (V)", "Spara video (V)"),
    ("video.saving", "Saving video...", "Sparar video..."),
    ("video.name_label", "File name (.mp4 is added)", "Filnamn (.mp4 läggs till)"),
    ("video.folder_label", "Folder", "Mapp"),
    ("video.info", "1080 × 1920, 60 fps, with sound. Tab: switch field  Enter: save  Esc: cancel", "1080 × 1920, 60 fps, med ljud. Tab: byt fält  Enter: spara  Esc: avbryt"),
    ("video.overwrite_yes", "Yes, overwrite", "Ja, skriv över"),
    ("video.saved", "The video was saved as {path}", "Videon sparades som {path}"),
    ("video.save_failed", "Could not save the video: {error}", "Kunde inte spara videon: {error}"),
    ("video.no_ffmpeg", "ffmpeg is missing, so rounds cannot be recorded. Install ffmpeg to save videos.", "ffmpeg saknas, så omgångar kan inte spelas in. Installera ffmpeg för att kunna spara video."),
    ("video.start_failed", "Recording could not start: {error}", "Inspelningen kunde inte starta: {error}"),
    ("video.record_failed", "Recording failed: {error}", "Inspelningen misslyckades: {error}"),
    ("video.nothing", "there is no recording to save", "det finns ingen inspelning att spara"),
    ("video.busy", "a video is already being saved", "en video sparas redan"),
    ("video.crashed", "saving crashed", "sparningen kraschade"),
    ("video.ffmpeg_start", "could not start ffmpeg: {error}", "kunde inte starta ffmpeg: {error}"),
    ("video.ffmpeg_failed", "ffmpeg failed: {error}", "ffmpeg misslyckades: {error}"),
    ("video.pipe", "could not send frames to ffmpeg: {error}", "kunde inte skicka bilder till ffmpeg: {error}"),
    ("video.thread", "the recording thread crashed", "inspelningstråden kraschade"),
    ("video.empty", "the recording is empty", "inspelningen är tom"),
    ("video.mkdir", "could not create the folder {path}: {error}", "kunde inte skapa mappen {path}: {error}"),
    ("video.write_sound", "could not write the sound: {error}", "kunde inte skriva ljudet: {error}"),
    ("video.write_file", "could not save {path}: {error}", "kunde inte spara {path}: {error}"),
    ("video.need_name", "Enter a file name.", "Ange ett filnamn."),
    ("video.bad_chars", "The file name cannot contain / or \\.", "Filnamnet får inte innehålla / eller \\."),
    ("video.name_long", "The file name can be at most {max} characters.", "Filnamnet får vara högst {max} tecken."),
    ("video.bad_name", "Invalid file name.", "Ogiltigt filnamn."),
    ("video.need_folder", "Enter a folder.", "Ange en mapp."),
    ("video.home_unknown", "The home folder is unknown; type the full path.", "Hemmappen är okänd; skriv hela sökvägen."),
    ("video.need_absolute", "Enter the folder as a full path (e.g. ~/Videos/RustOut).", "Ange mappen som en fullständig sökväg (t.ex. ~/Videos/RustOut)."),
    // Settings
    ("settings.save_failed", "Could not save the settings: {error}", "Kunde inte spara inställningarna: {error}"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn placeholders(text: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut rest = text;
        while let Some(start) = rest.find('{') {
            let Some(len) = rest[start..].find('}') else {
                break;
            };
            let name = &rest[start + 1..start + len];
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                out.insert(name.to_owned());
            }
            rest = &rest[start + len..];
        }
        out
    }

    #[test]
    fn catalog_is_complete_and_consistent() {
        let mut keys = BTreeSet::new();
        for &(key, en, sv) in CATALOG {
            assert!(keys.insert(key), "duplicate key {key}");
            assert!(!en.is_empty() && !sv.is_empty(), "{key} is empty");
            assert_eq!(
                placeholders(en),
                placeholders(sv),
                "{key}: placeholders differ"
            );
        }
    }

    /// Every `t!("…")` in the source must exist in the catalog, so a typo
    /// is caught here rather than shown to the player.
    #[test]
    fn every_key_used_in_the_code_exists() {
        let sources = [
            include_str!("app.rs"),
            include_str!("audio.rs"),
            include_str!("capture.rs"),
            include_str!("editor.rs"),
            include_str!("game.rs"),
            include_str!("levels.rs"),
            include_str!("main.rs"),
            include_str!("render.rs"),
            include_str!("screens.rs"),
            include_str!("settings.rs"),
            include_str!("store.rs"),
            include_str!("ui.rs"),
            include_str!("video.rs"),
        ];
        let known: BTreeSet<&str> = CATALOG.iter().map(|&(k, _, _)| k).collect();
        let mut used = 0;
        for src in sources {
            let mut rest = src;
            while let Some(at) = rest.find("t!(\"") {
                // Skip `format!(` and other macros that merely end in `t!(`.
                let standalone = rest[..at]
                    .chars()
                    .last()
                    .is_none_or(|c| !c.is_alphanumeric() && c != '_');
                rest = &rest[at + 4..];
                if standalone {
                    let key = &rest[..rest.find('"').unwrap()];
                    assert!(known.contains(key), "unknown translation key: {key}");
                    used += 1;
                }
            }
        }
        assert!(used > 100, "found only {used} uses");
    }

    #[test]
    fn placeholders_are_filled_in_both_languages() {
        let en = fill(
            tr_in(Lang::En, "hud.level"),
            &[("level", &2), ("count", &3)],
        );
        let sv = fill(
            tr_in(Lang::Sv, "hud.level"),
            &[("level", &2), ("count", &3)],
        );
        assert_eq!(en, "LEVEL 2/3");
        assert_eq!(sv, "BANA 2/3");
    }

    #[test]
    fn language_is_per_thread_and_defaults_to_english() {
        assert_eq!(lang(), Lang::En);
        assert_eq!(t!("menu.play"), "Play");
        std::thread::spawn(|| {
            set_lang(Lang::Sv);
            assert_eq!(t!("menu.play"), "Spela");
            assert_eq!(t!("game.score", score = 7), "Poäng: 7");
        })
        .join()
        .unwrap();
        assert_eq!(t!("menu.play"), "Play", "other threads are not affected");
        assert_eq!(Lang::from_code("sv_SE.UTF-8"), Some(Lang::Sv));
        assert_eq!(Lang::from_code("EN"), Some(Lang::En));
        assert_eq!(Lang::from_code("de"), None);
        assert_eq!(Lang::En.next(), Lang::Sv);
    }
}
