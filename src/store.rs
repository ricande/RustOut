//! Reading and writing custom levels as human-readable JSON files.
//!
//! ```json
//! {
//!   "format": 3,
//!   "name": "My level",
//!   "grid": { "columns": 12, "rows": 24 },
//!   "rows": [
//!     "#3333333333#",
//!     "1.2.......21"
//!   ],
//!   "bonuses": [
//!     { "row": 2, "col": 3, "bonus": "multiball" }
//!   ]
//! }
//! ```
//!
//! Format 1 (the first release) had no stones and no bonuses; such files
//! load unchanged as levels without bonuses. Formats 1 and 2 had no `grid`;
//! they get the grid of that time (12 x 24), so their layout is unchanged.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::levels::{self, Bonus, Grid, Layout, MAX_NAME_LEN};
use crate::t;

/// Written by this version. Older formats that can still be read: 1, 2.
const FORMAT: u32 = 3;
/// Real level files are about 1 KB; refuse anything absurd.
const MAX_FILE_SIZE: u64 = 64 * 1024;

#[derive(Serialize, Deserialize)]
struct LevelFile {
    format: u32,
    name: String,
    /// Missing in formats 1 and 2: the default grid.
    #[serde(default)]
    grid: Option<GridEntry>,
    rows: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    bonuses: Vec<BonusEntry>,
}

#[derive(Serialize, Deserialize)]
struct GridEntry {
    columns: usize,
    rows: usize,
}

/// A bonus on the brick at `row`, `col` (1-based, like the error messages).
#[derive(Serialize, Deserialize)]
struct BonusEntry {
    row: usize,
    col: usize,
    bonus: String,
}

/// Checks a level name. Messages are shown to the player.
pub fn validate_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(t!("store.name_missing").into());
    }
    if trimmed.chars().count() > MAX_NAME_LEN {
        return Err(t!("store.name_long", max = MAX_NAME_LEN));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(t!("store.name_chars").into());
    }
    Ok(())
}

/// Checks everything a level needs before it can be saved or played.
pub fn validate(layout: &Layout) -> Result<(), String> {
    validate_name(&layout.name)?;
    if layout.brick_count() == 0 {
        return Err(t!("store.no_bricks").into());
    }
    Ok(())
}

pub fn to_json(layout: &Layout) -> String {
    let file = LevelFile {
        format: FORMAT,
        name: layout.name.trim().to_owned(),
        grid: Some(GridEntry {
            columns: layout.grid().cols,
            rows: layout.grid().rows,
        }),
        rows: layout.to_rows(),
        bonuses: layout
            .bonuses()
            .into_iter()
            .map(|(r, c, b)| BonusEntry {
                row: r + 1,
                col: c + 1,
                bonus: b.key().to_owned(),
            })
            .collect(),
    };
    serde_json::to_string_pretty(&file).expect("level serializes") + "\n"
}

pub fn from_json(text: &str) -> Result<Layout, String> {
    let file: LevelFile =
        serde_json::from_str(text).map_err(|e| t!("store.bad_json", error = e))?;
    if !(1..=FORMAT).contains(&file.format) {
        return Err(t!("store.bad_format", format = file.format, max = FORMAT));
    }
    let grid = match &file.grid {
        Some(g) => Grid::new(g.columns, g.rows)?,
        None => Grid::DEFAULT,
    };
    let mut layout = Layout::from_rows_in(file.name.trim(), grid, &file.rows)?;
    for entry in &file.bonuses {
        let bonus = Bonus::from_key(&entry.bonus).ok_or_else(|| {
            let known: Vec<&str> = Bonus::ALL.iter().map(|b| b.key()).collect();
            t!(
                "store.bad_bonus",
                name = entry.bonus,
                list = known.join(", ")
            )
        })?;
        if entry.row == 0 || entry.col == 0 {
            return Err(t!("store.bonus_from_1").into());
        }
        layout.add_bonus(entry.row - 1, entry.col - 1, bonus)?;
    }
    validate(&layout)?;
    Ok(layout)
}

/// File name for a level name: lowercase ASCII letters, digits and dashes.
pub fn file_stem(name: &str) -> String {
    let mut stem = String::new();
    for ch in name.trim().to_lowercase().chars() {
        let mapped = match ch {
            'å' | 'ä' | 'à' | 'á' | 'â' => 'a',
            'ö' | 'ø' | 'ó' | 'ò' | 'ô' => 'o',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'ü' | 'ú' => 'u',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if mapped != '-' || !stem.ends_with('-') {
            stem.push(mapped);
        }
    }
    let stem = stem.trim_matches('-');
    if stem.is_empty() {
        "level".into()
    } else {
        stem.into()
    }
}

/// The game's per-user data folder (levels and settings live here).
pub fn data_dir() -> PathBuf {
    let env = |key: &str| std::env::var_os(key).filter(|v| !v.is_empty());
    if cfg!(windows) {
        if let Some(appdata) = env("APPDATA") {
            return PathBuf::from(appdata).join("RustOut");
        }
    } else if cfg!(target_os = "macos") {
        if let Some(home) = env("HOME") {
            return PathBuf::from(home).join("Library/Application Support/RustOut");
        }
    } else if let Some(data) = env("XDG_DATA_HOME") {
        return PathBuf::from(data).join("rustout");
    } else if let Some(home) = env("HOME") {
        return PathBuf::from(home).join(".local/share/rustout");
    }
    PathBuf::from("rustout-data")
}

/// The documented per-user level directory (overridable with `RUSTOUT_LEVEL_DIR`).
pub fn default_dir() -> PathBuf {
    match std::env::var_os("RUSTOUT_LEVEL_DIR").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => data_dir().join("levels"),
    }
}

pub struct StoredLevel {
    pub path: PathBuf,
    /// The level, or why the file could not be used.
    pub level: Result<Layout, String>,
}

pub struct LevelStore {
    dir: PathBuf,
}

impl LevelStore {
    pub fn new(dir: PathBuf) -> Self {
        LevelStore { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn path_for(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{}.json", file_stem(name)))
    }

    /// All `.json` files in the level directory, sorted by file name.
    /// A missing directory just means there are no levels yet.
    pub fn list(&self) -> Result<Vec<StoredLevel>, String> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(t!("store.read_dir", path = self.dir.display(), error = e)),
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "json"))
            .collect();
        paths.sort();
        Ok(paths
            .into_iter()
            .map(|path| StoredLevel {
                level: load(&path),
                path,
            })
            .collect())
    }

    /// Writes the level atomically (temporary file + rename).
    pub fn save(&self, layout: &Layout, path: &Path) -> Result<(), String> {
        validate(layout)?;
        if levels::is_builtin_name(&layout.name) {
            return Err(t!("store.builtin_name", name = layout.name.trim()));
        }
        let fail = |e: std::io::Error| format!("{}: {e}", path.display());
        fs::create_dir_all(&self.dir)
            .map_err(|e| t!("store.create_dir", path = self.dir.display(), error = e))?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, to_json(layout)).map_err(fail)?;
        fs::rename(&tmp, path).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            fail(e)
        })
    }
}

pub fn load(path: &Path) -> Result<Layout, String> {
    let size = fs::metadata(path)
        .map_err(|e| t!("store.read_file", error = e))?
        .len();
    if size > MAX_FILE_SIZE {
        return Err(t!("store.too_big", size = size));
    }
    let text = fs::read_to_string(path).map_err(|e| t!("store.read_file", error = e))?;
    from_json(&text)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A fresh, empty directory for one test.
    pub fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rustout-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn sample() -> Layout {
        let rows = ["1.2.3...#...", "............", "333........."];
        let mut l = Layout::from_rows("Min bana", &rows).unwrap();
        l.add_bonus(0, 2, Bonus::MultiBall).unwrap();
        l.add_bonus(2, 0, Bonus::Wall).unwrap();
        l
    }

    #[test]
    fn json_round_trip_is_readable() {
        let json = to_json(&sample());
        assert!(json.contains("\"name\": \"Min bana\""));
        assert!(
            json.contains("\"1.2.3...#...\""),
            "rows stay readable:\n{json}"
        );
        assert!(json.contains("\"format\": 3"));
        assert!(json.contains("\"columns\": 12"), "{json}");
        assert!(json.contains("\"bonus\": \"multiball\""), "{json}");
        assert_eq!(from_json(&json).unwrap(), sample());
    }

    #[test]
    fn invalid_json_is_rejected_with_reasons() {
        let cases = [
            ("not json", "Invalid JSON"),
            (r#"{"format":1,"name":"x"}"#, "Invalid JSON"),
            (
                r#"{"format":4,"name":"x","rows":["1..........."]}"#,
                "file format",
            ),
            (
                r#"{"format":3,"name":"x","grid":{"columns":2,"rows":24},"rows":["1."]}"#,
                "Columns",
            ),
            (
                r#"{"format":3,"name":"x","grid":{"columns":12,"rows":99},"rows":["1..........."]}"#,
                "Rows",
            ),
            (
                r#"{"format":3,"name":"x","grid":{"columns":6,"rows":8},"rows":["1..........."]}"#,
                "should have 6",
            ),
            (
                r#"{"format":3,"name":"x","grid":{"columns":6},"rows":["1....."]}"#,
                "Invalid JSON",
            ),
            (
                r##"{"format":2,"name":"x","rows":["#..........."]}"##,
                "breakable",
            ),
            (
                r#"{"format":2,"name":"x","rows":["1..........."],"bonuses":[{"row":1,"col":1,"bonus":"reverse"}]}"#,
                "Unknown bonus type",
            ),
            (
                r#"{"format":2,"name":"x","rows":["1#.........."],"bonuses":[{"row":1,"col":2,"bonus":"wall"}]}"#,
                "no breakable",
            ),
            (
                r#"{"format":2,"name":"x","rows":["1..........."],"bonuses":[{"row":0,"col":1,"bonus":"wall"}]}"#,
                "from 1",
            ),
            (
                r#"{"format":1,"name":"  ","rows":["1..........."]}"#,
                "name",
            ),
            (r#"{"format":1,"name":"x","rows":["1.."]}"#, "Row 1"),
            (
                r#"{"format":1,"name":"x","rows":["1.........z."]}"#,
                "invalid character",
            ),
            (
                r#"{"format":1,"name":"x","rows":["............"]}"#,
                "breakable block",
            ),
        ];
        for (json, expected) in cases {
            let err = from_json(json).unwrap_err();
            assert!(err.contains(expected), "{json} -> {err}");
        }
    }

    #[test]
    fn format_1_files_still_load() {
        let old = r#"{
  "format": 1,
  "name": "Gammal bana",
  "rows": [
    "222222222222",
    "1.1.1.1.1.1."
  ]
}"#;
        let layout = from_json(old).unwrap();
        assert_eq!(layout.name, "Gammal bana");
        assert_eq!(layout.brick_count(), 18);
        assert!(layout.bonuses().is_empty());
        // Saving upgrades it to format 2 with the same content.
        assert_eq!(from_json(&to_json(&layout)).unwrap(), layout);
    }

    #[test]
    fn format_2_files_get_the_old_grid() {
        let old = r#"{
  "format": 2,
  "name": "Bonus",
  "rows": ["1.2.#......3"],
  "bonuses": [{ "row": 1, "col": 12, "bonus": "wall" }]
}"#;
        let layout = from_json(old).unwrap();
        assert_eq!(layout.grid(), Grid::DEFAULT);
        assert_eq!(layout.bonuses(), vec![(0, 11, Bonus::Wall)]);
        assert_eq!(layout.bricks()[3].rect, Grid::DEFAULT.cell_rect(0, 11));
    }

    #[test]
    fn grid_size_is_saved_and_loaded() {
        for (cols, rows) in [(4, 8), (7, 13), (20, 36), (16, 9)] {
            let grid = Grid::new(cols, rows).unwrap();
            let mut l = Layout::with_grid("Rutnät", grid);
            l.set_block(0, 0, levels::Block::Brick(2));
            l.set_block(rows - 1, cols - 1, levels::Block::Stone);
            l.set_bonus(0, 0, Some(Bonus::MultiBall));
            let json = to_json(&l);
            assert!(json.contains(&format!("\"columns\": {cols}")), "{json}");
            let back = from_json(&json).unwrap();
            assert_eq!(back, l);
            assert_eq!(back.grid(), grid);
        }
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(file_stem("Min Bana!"), "min-bana");
        assert_eq!(file_stem("Örnens näste 2"), "ornens-naste-2");
        assert_eq!(file_stem("../../etc"), "etc");
        assert_eq!(file_stem("???"), "level");
    }

    #[test]
    fn save_list_and_load() {
        let dir = temp_dir("save");
        let store = LevelStore::new(dir.clone());
        assert!(store.list().unwrap().is_empty(), "missing dir = no levels");

        let path = store.path_for("Min bana");
        store.save(&sample(), &path).unwrap();
        assert_eq!(path, dir.join("min-bana.json"));
        assert_eq!(load(&path).unwrap(), sample());

        fs::write(dir.join("trasig.json"), "{ nope").unwrap();
        fs::write(dir.join("ignored.txt"), "hello").unwrap();
        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].level.as_ref().unwrap().name, "Min bana");
        assert!(listed[1]
            .level
            .as_ref()
            .unwrap_err()
            .contains("Invalid JSON"));
        assert!(!dir.join("min-bana.json.tmp").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn save_refuses_builtin_names_and_reports_io_errors() {
        let dir = temp_dir("refuse");
        let store = LevelStore::new(dir.clone());
        let mut layout = sample();
        layout.name = "Diamanten".into();
        let err = store
            .save(&layout, &store.path_for("Diamanten"))
            .unwrap_err();
        assert!(err.contains("built-in"), "{err}");

        // The "directory" is a file, so saving must fail cleanly.
        fs::write(&dir, "not a dir").unwrap();
        let err = store
            .save(&sample(), &store.path_for("Min bana"))
            .unwrap_err();
        assert!(err.contains("Could not create the folder"), "{err}");
        assert!(store.list().is_err());
        fs::remove_file(dir).unwrap();
    }
}
