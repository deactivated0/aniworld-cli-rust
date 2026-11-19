use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Series {
    pub title: String,
    pub slug: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Episode {
    pub season: u32,
    pub number: u32,
    pub title: String,
    pub url: String,
}

impl Episode {
    pub fn display(&self) -> String {
        if self.season == 0 {
            format!("Movie {:02}  {}", self.number, self.title)
        } else {
            format!("S{:02}E{:02}  {}", self.season, self.number, self.title)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Host {
    pub name: String,
    pub language: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Stream {
    pub url: String,
    pub referer: Option<String>,
    pub user_agent: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Progress {
    pub title: String,
    pub url: String,
    pub season: u32,
    pub number: u32,
    pub label: String,
    pub updated: String,
}
