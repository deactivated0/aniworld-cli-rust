//! Compatible with the Python client's version-1 state.json format.
use std::{
    collections::BTreeMap,
    env, fs,
    io::{BufReader, BufWriter},
    path::PathBuf,
    sync::LazyLock,
};

use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::model::{Episode, Progress, Series};

static DEFAULT_STATE_FILE: LazyLock<PathBuf> = LazyLock::new(|| {
    #[cfg(target_os = "windows")]
    {
        let home = env::var_os("APPDATA")
            .or_else(|| env::var_os("USERPROFILE"))
            .unwrap_or_default();
        PathBuf::from(home).join("aniworld-cli").join("state.json")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = env::var_os("HOME").unwrap_or_default();
        let xdg = env::var_os("XDG_CONFIG_HOME")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(home).join(".config"));
        xdg.join("aniworld-cli").join("state.json")
    }
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    #[serde(default = "state_version")]
    pub version: u32,
    #[serde(default)]
    pub series: BTreeMap<String, Progress>,
    #[serde(default)]
    pub favorites: Vec<String>,
}

const fn state_version() -> u32 {
    1
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: 1,
            series: BTreeMap::new(),
            favorites: Vec::new(),
        }
    }
}

pub fn location() -> PathBuf {
    if let Some(dir) = env::var_os("ANIWORLD_STATE_DIR").filter(|s| !s.is_empty()) {
        return PathBuf::from(dir).join("state.json");
    }
    DEFAULT_STATE_FILE.clone()
}

pub fn config_dir() -> PathBuf {
    location()
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn load() -> State {
    let path = location();
    let Ok(file) = fs::File::open(&path) else {
        return State::default();
    };
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).unwrap_or_default()
}

pub fn save(state: &State) -> Result<()> {
    let file = location();
    let directory = file.parent().context("Invalid state location")?;
    fs::create_dir_all(directory)?;
    let temp = file.with_extension("json.tmp");
    {
        let f = fs::File::create(&temp).context("Could not create temporary state file")?;
        let writer = BufWriter::new(f);
        serde_json::to_writer_pretty(writer, state).context("Could not serialize state")?;
    }
    fs::rename(temp, file).context("Could not atomically replace state")?;
    Ok(())
}

impl State {
    pub fn record(&mut self, series: &Series, ep: &Episode) {
        self.series.insert(
            series.slug.clone(),
            Progress {
                title: series.title.clone(),
                url: series.url.clone(),
                season: ep.season,
                number: ep.number,
                label: ep.title.clone(),
                updated: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            },
        );
    }

    pub fn is_favorite(&self, slug: &str) -> bool {
        self.favorites.iter().any(|s| s == slug)
    }

    pub fn toggle_favorite(&mut self, slug: &str) -> bool {
        if let Some(pos) = self.favorites.iter().position(|s| s == slug) {
            self.favorites.remove(pos);
            false
        } else {
            self.favorites.push(slug.to_string());
            true
        }
    }

    pub fn recent(&self) -> Vec<(String, Progress)> {
        let mut entries = self
            .series
            .iter()
            .map(|(k, v)| (k.to_owned(), v.clone()))
            .collect::<Vec<_>>();
        entries.sort_unstable_by(|a, b| b.1.updated.cmp(&a.1.updated));
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backward_compatible() {
        let original = r#"{"version":1,"series":{"demo":{"title":"Demo","url":"https://example.test/anime/stream/demo","season":2,"number":3,"label":"Episode title","updated":"2026-10-18T19:12:00"}}}"#;
        let state: State = serde_json::from_str(original).unwrap();
        assert_eq!(state.series["demo"].number, 3);
        let reconstructed = serde_json::to_string(&state).unwrap();
        assert!(reconstructed.contains("Episode title"));
    }
}
