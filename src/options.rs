use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum UiLanguage {
    En,
    De,
}

#[derive(Parser, Debug)]
#[command(
    name = "aniworld",
    version,
    about = "AniWorld streaming in a fast, native Rust terminal client"
)]
pub struct Options {
    /// Anime title to search for
    pub query: Option<String>,

    /// Select a title from local watch history
    #[arg(short = 'w', long)]
    pub watchlist: bool,

    /// Play remaining episodes in the selected season consecutively
    #[arg(long)]
    pub auto: bool,

    /// Choose first matching items without opening menus
    #[arg(long)]
    pub no_menu: bool,

    /// Select a specific season and episode (1-3, or 0-1 for a movie)
    #[arg(long)]
    pub episode: Option<String>,

    /// Ordered preferred languages, e.g. ger-dub,ger-sub,eng-sub
    #[arg(long, env = "ANIWORLD_LANG", default_value = "ger-dub,ger-sub,eng-sub")]
    pub lang: String,

    /// Ordered host providers, e.g. VOE,Vidmoly
    #[arg(long, env = "ANIWORLD_HOSTER", default_value = "VOE,Vidmoly")]
    pub hoster: String,

    /// Absolute path to mpv, or executable on PATH
    #[arg(long)]
    pub player: Option<String>,

    /// Print resolved streaming URL and required headers; do not launch mpv
    #[arg(long)]
    pub debug: bool,

    /// JSON output for resolved stream; implies --no-menu and --debug
    #[arg(long)]
    pub json: bool,

    /// Bypass media segment validation (not recommended)
    #[arg(long)]
    pub no_verify: bool,

    /// Pass a single extra option to mpv; may be used multiple times
    #[arg(long = "mpv-arg", allow_hyphen_values = true)]
    pub mpv_args: Vec<String>,

    /// Check the player, saved state, and configured endpoint
    #[arg(long)]
    pub doctor: bool,

    /// Print saved watch history without starting playback
    #[arg(long)]
    pub history: bool,

    /// Output only site results as JSON; requires a search query
    #[arg(long)]
    pub search_json: bool,

    /// Site root (override for site migrations or testing)
    #[arg(long, env = "ANIWORLD_BASE_URL", default_value = "https://aniworld.to")]
    pub base_url: String,

    /// Browser-like User-Agent used for requests and mpv
    #[arg(
        long,
        env = "ANIWORLD_UA",
        default_value = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36"
    )]
    pub user_agent: String,

    /// Prompt language
    #[arg(long, value_enum, default_value = "en")]
    pub ui_lang: UiLanguage,
}

impl Options {
    pub fn preferred_languages(&self) -> Vec<String> {
        self.lang
            .split(',')
            .map(|x| x.trim().to_ascii_lowercase())
            .filter(|x| !x.is_empty())
            .collect()
    }
    pub fn preferred_hosters(&self) -> Vec<String> {
        self.hoster
            .split(',')
            .map(|x| x.trim().to_ascii_lowercase())
            .filter(|x| !x.is_empty())
            .collect()
    }
}

pub fn parse_episode(s: &str) -> anyhow::Result<(u32, u32)> {
    let (season, number) = s
        .split_once('-')
        .ok_or_else(|| anyhow::anyhow!("Invalid episode '{s}', expected S-E such as 1-3 or 0-1"))?;
    let season = season.parse::<u32>()?;
    let number = number.parse::<u32>()?;
    anyhow::ensure!(number > 0, "Episode number must be greater than zero");
    Ok((season, number))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn episode_parser() {
        assert_eq!(parse_episode("0-1").unwrap(), (0, 1));
        assert_eq!(parse_episode("12-345").unwrap(), (12, 345));
        assert!(parse_episode("4").is_err());
        assert!(parse_episode("1-0").is_err());
    }
}
