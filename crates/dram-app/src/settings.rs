//! Persistent recent-files list and user spellcheck dictionary storage.

use std::fs;
use std::path::{Path, PathBuf};

use dram_format::write_atomic_bytes;

use crate::app::DramApp;
use crate::document::read_text_bounded;

impl DramApp {
    pub(crate) fn push_recent_file(&mut self, path: &Path) {
        let pb = path.to_path_buf();
        self.recent_files.retain(|p| p != &pb);
        self.recent_files.insert(0, pb);
        self.recent_files.truncate(8);
        save_recent_files(&self.recent_files);
    }

    pub(crate) fn clear_recent_files(&mut self) {
        self.recent_files.clear();
        save_recent_files(&self.recent_files);
    }
}

pub(crate) fn recent_files_store_path() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("dram").join("recent.txt"));
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Some(PathBuf::from(appdata).join("dram").join("recent.txt"));
    }
    std::env::var_os("HOME").map(|h| {
        PathBuf::from(h)
            .join(".config")
            .join("dram")
            .join("recent.txt")
    })
}

pub(crate) fn load_recent_files() -> Vec<PathBuf> {
    let Some(store) = recent_files_store_path() else {
        return Vec::new();
    };
    let Ok(raw) = read_text_bounded(&store, 1024 * 1024) else {
        return Vec::new();
    };
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.contains('\0'))
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .take(8)
        .collect()
}

pub(crate) fn save_recent_files(paths: &[PathBuf]) {
    let Some(store) = recent_files_store_path() else {
        return;
    };
    if let Some(parent) = store.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("[config] failed to create config dir: {err}");
            return;
        }
    }
    let content = paths
        .iter()
        .filter_map(|p| p.to_str())
        .filter(|s| !s.contains('\n') && !s.contains('\r'))
        .collect::<Vec<_>>()
        .join("\n");
    if let Err(err) = write_atomic_bytes(&store, content.as_bytes()) {
        eprintln!("[config] failed to save recent files: {err}");
    }
}

pub(crate) fn user_dict_store_path() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("dram").join("user_dict.txt"));
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Some(PathBuf::from(appdata).join("dram").join("user_dict.txt"));
    }
    std::env::var_os("HOME").map(|h| {
        PathBuf::from(h)
            .join(".config")
            .join("dram")
            .join("user_dict.txt")
    })
}

pub(crate) fn load_user_dict() -> String {
    let Some(store) = user_dict_store_path() else {
        return String::new();
    };
    read_text_bounded(&store, 2 * 1024 * 1024).unwrap_or_default()
}

pub(crate) fn save_user_dict(words: &[String]) {
    let Some(store) = user_dict_store_path() else {
        return;
    };
    if let Some(parent) = store.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("[config] failed to create config dir: {err}");
            return;
        }
    }
    let cleaned: Vec<&str> = words
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|w| !w.is_empty() && !w.contains('\n') && !w.contains('\r'))
        .collect();
    if let Err(err) = write_atomic_bytes(&store, cleaned.join("\n").as_bytes()) {
        eprintln!("[config] failed to save user dict: {err}");
    }
}
