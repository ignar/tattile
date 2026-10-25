//! Saves all app data as one JSON file in ~/Library/Application Support.

use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;

use crate::model::AppData;

/// Set `TATTILE_DATA_DIR` to keep data somewhere else, for example for tests or a demo.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("TATTILE_DATA_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    ProjectDirs::from("com", "tattile", "Tattile")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn data_file() -> PathBuf {
    data_dir().join("data.json")
}

pub fn load() -> AppData {
    let path = data_file();
    let Ok(text) = fs::read_to_string(&path) else {
        return AppData::default();
    };
    match serde_json::from_str(&text) {
        Ok(data) => data,
        Err(err) => {
            // Keep the damaged file, so no writing is lost.
            let backup = path.with_extension(format!("broken-{}.json", chrono::Local::now().format("%Y%m%d-%H%M%S")));
            let _ = fs::rename(&path, &backup);
            eprintln!("tattile: could not read {}: {err}. Moved it to {}", path.display(), backup.display());
            AppData::default()
        }
    }
}

pub fn save(data: &AppData) -> Result<(), String> {
    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    // Write to a temporary file first, so a crash cannot leave a half-written file.
    let tmp = dir.join("data.json.tmp");
    fs::write(&tmp, json).map_err(|e| format!("Could not write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, data_file()).map_err(|e| format!("Could not save data: {e}"))
}
