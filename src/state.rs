//! Compatible with the Python client's version-1 state.json format.
use std::{collections::BTreeMap, env, fs, path::PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::model::{Episode, Progress, Series};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    #[serde(default = "state_version")]
    pub version: u32,
    #[serde(default)]
    pub series: BTreeMap<String, Progress>,
}

const fn state_version() -> u32 {
    1
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: 1,
            series: BTreeMap::new(),
        }
    }
}

pub fn location() -> PathBuf {
    if let Some(dir) = env::var_os("ANIWORLD_STATE_DIR").filter(|s| !s.is_empty()) {
        return PathBuf::from(dir).join("state.json");
    }
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
}

pub fn load() -> State {
    fs::read_to_string(location())
        .ok()
        .and_then(|s| serde_json::from_str::<State>(&s).ok())
        .unwrap_or_default()
}

pub fn save(state: &State) -> Result<()> {
    let file = location();
    let directory = file.parent().context("Invalid state location")?;
    fs::create_dir_all(directory)?;
    let temp = file.with_extension("json.tmp");
    let serialized = serde_json::to_vec_pretty(state)?;
    fs::write(&temp, serialized).context("Could not write temporary state")?;
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

    pub fn recent(&self) -> Vec<(String, Progress)> {
        let mut entries = self
            .series
            .iter()
            .map(|(k, v)| (k.to_owned(), v.clone()))
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| b.1.updated.cmp(&a.1.updated));
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
