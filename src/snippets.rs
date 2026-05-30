use std::fs;
use std::path::PathBuf;

use chrono::Local;

fn base_dir() -> PathBuf {
    let home = dirs::home_dir().expect("No home directory");
    home.join(".rust-pad")
}

fn snippets_dir() -> PathBuf {
    let d = base_dir().join("snippets");
    fs::create_dir_all(&d).ok();
    d
}

fn history_dir() -> PathBuf {
    let d = base_dir().join("history");
    fs::create_dir_all(&d).ok();
    d
}

pub fn save_snippet(name: &str, content: &str) -> Result<PathBuf, String> {
    let dir = snippets_dir();
    let filename = if name.ends_with(".rs") {
        name.to_string()
    } else {
        format!("{name}.rs")
    };
    let path = dir.join(&filename);
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn save_history(content: &str) -> Result<(), String> {
    if content.trim().is_empty() {
        return Ok(());
    }

    let dir = history_dir();

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{timestamp}.rs");
    let path = dir.join(&filename);
    fs::write(&path, content).map_err(|e| e.to_string())?;

    // Prune to last 50
    prune_history(50);

    Ok(())
}

fn prune_history(max: usize) {
    let dir = history_dir();
    let mut entries: Vec<_> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
            .collect(),
        Err(_) => return,
    };

    if entries.len() <= max {
        return;
    }

    entries.sort_by_key(|e| e.file_name());
    let to_remove = entries.len() - max;
    for entry in entries.into_iter().take(to_remove) {
        fs::remove_file(entry.path()).ok();
    }
}

pub fn list_snippets() -> Vec<(String, PathBuf)> {
    list_dir_rs(&snippets_dir())
}

pub fn list_history() -> Vec<(String, PathBuf)> {
    let mut items = list_dir_rs(&history_dir());
    items.reverse(); // Most recent first
    items
}

fn list_dir_rs(dir: &PathBuf) -> Vec<(String, PathBuf)> {
    let mut items: Vec<(String, PathBuf)> = match fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                (name, e.path())
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    items.sort_by(|a, b| a.0.cmp(&b.0));
    items
}

pub fn load_file(path: &PathBuf) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
}

pub fn delete_file(path: &PathBuf) -> Result<(), String> {
    fs::remove_file(path).map_err(|e| e.to_string())
}
