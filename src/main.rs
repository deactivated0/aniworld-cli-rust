mod catalog;
mod model;
mod net;
mod options;
mod player;
mod resolve;
mod state;

use std::{
    collections::HashMap,
    fmt::Write as _,
    io::{self, Write},
    sync::LazyLock,
};

use anyhow::{Context, Result, bail};
use clap::Parser;
use dialoguer::{Select, theme::ColorfulTheme};
use url::Url;

use crate::{
    model::{Episode, Host, Series},
    options::{Options, UiLanguage},
    state::State,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuRow {
    Header(&'static str),
    Action(&'static str),
    Blank,
}

pub const AFTER_PLAYBACK_ROWS: &[MenuRow] = &[
    MenuRow::Header("── CONTINUE ──────────────────────"),
    MenuRow::Action("Next episode"),
    MenuRow::Action("Previous episode"),
    MenuRow::Action("Replay episode"),
    MenuRow::Action("Resume playback"),
    MenuRow::Action("Next unwatched episode"),
    MenuRow::Action("Random episode"),
    MenuRow::Blank,
    MenuRow::Header("── NAVIGATION ────────────────────"),
    MenuRow::Action("Select episode"),
    MenuRow::Action("Select season"),
    MenuRow::Action("Jump to episode"),
    MenuRow::Action("Search another anime"),
    MenuRow::Action("Browse anime catalog"),
    MenuRow::Action("Browse by genre"),
    MenuRow::Action("Continue another anime"),
    MenuRow::Blank,
    MenuRow::Header("── STREAM & LANGUAGE ─────────────"),
    MenuRow::Action("Change language"),
    MenuRow::Action("Change hoster"),
    MenuRow::Action("Change stream quality"),
    MenuRow::Action("Find fastest hoster"),
    MenuRow::Action("Retry stream"),
    MenuRow::Action("Test all hosters"),
    MenuRow::Blank,
    MenuRow::Header("── PLAYBACK ──────────────────────"),
    MenuRow::Action("Change playback speed"),
    MenuRow::Action("Select audio track"),
    MenuRow::Action("Select subtitle track"),
    MenuRow::Action("Toggle auto-next"),
    MenuRow::Action("Toggle skip intro"),
    MenuRow::Action("Toggle skip outro"),
    MenuRow::Action("Player settings"),
    MenuRow::Blank,
    MenuRow::Header("── VIDEO ENHANCEMENTS ────────────"),
    MenuRow::Action("Toggle Anime4K"),
    MenuRow::Action("Select Anime4K preset"),
    MenuRow::Action("Video filters"),
    MenuRow::Action("Shader settings"),
    MenuRow::Blank,
    MenuRow::Header("── LIBRARY ───────────────────────"),
    MenuRow::Action("Open watchlist"),
    MenuRow::Action("Open favorites"),
    MenuRow::Action("View watch history"),
    MenuRow::Action("Add/remove favorite"),
    MenuRow::Action("Mark as watched"),
    MenuRow::Action("Mark as unwatched"),
    MenuRow::Blank,
    MenuRow::Header("── TOOLS ─────────────────────────"),
    MenuRow::Action("Episode information"),
    MenuRow::Action("Playback statistics"),
    MenuRow::Action("Copy episode URL"),
    MenuRow::Action("Copy stream URL"),
    MenuRow::Action("Open in browser"),
    MenuRow::Action("Refresh episode list"),
    MenuRow::Action("Refresh stream sources"),
    MenuRow::Action("Clear cache"),
    MenuRow::Blank,
    MenuRow::Header("── SYSTEM ────────────────────────"),
    MenuRow::Action("Settings"),
    MenuRow::Action("Quit"),
];

static SELECTABLE_ACTIONS: LazyLock<Vec<(usize, &'static str)>> = LazyLock::new(|| {
    AFTER_PLAYBACK_ROWS
        .iter()
        .enumerate()
        .filter_map(|(idx, row)| match row {
            MenuRow::Action(name) => Some((idx, *name)),
            _ => None,
        })
        .collect()
});

fn run_after_playback_menu(default_action: Option<&str>) -> Result<Option<&'static str>> {
    let term = console::Term::stderr();
    if !term.is_term() {
        return Ok(None);
    }

    let selectable = SELECTABLE_ACTIONS.as_slice();
    if selectable.is_empty() {
        return Ok(None);
    }

    let mut cursor_pos = default_action
        .and_then(|target| selectable.iter().position(|(_, name)| *name == target))
        .unwrap_or(0);

    let mut scroll_top = 0;
    let mut frame_buf = String::with_capacity(1024);
    term.hide_cursor()?;

    loop {
        let term_height = term.size().0 as usize;
        let max_visible = if term_height >= 12 {
            term_height.saturating_sub(4).clamp(8, 22)
        } else {
            8
        };

        let current_row = selectable[cursor_pos].0;
        if current_row < scroll_top {
            scroll_top = current_row;
        } else if current_row >= scroll_top + max_visible {
            scroll_top = current_row + 1 - max_visible;
        }

        let visible_end = (scroll_top + max_visible).min(AFTER_PLAYBACK_ROWS.len());

        frame_buf.clear();
        let mut lines_rendered = 0;
        frame_buf.push_str("? After playback ›\n");
        lines_rendered += 1;

        for (i, row) in AFTER_PLAYBACK_ROWS[scroll_top..visible_end]
            .iter()
            .enumerate()
        {
            let actual_idx = scroll_top + i;
            match row {
                MenuRow::Header(h) => {
                    let _ = writeln!(frame_buf, "  \x1b[38;5;244m{}\x1b[0m", h);
                }
                MenuRow::Blank => {
                    frame_buf.push('\n');
                }
                MenuRow::Action(label) => {
                    if actual_idx == current_row {
                        let _ = writeln!(frame_buf, "\x1b[1;36m❯ {}\x1b[0m", label);
                    } else {
                        let _ = writeln!(frame_buf, "  {}", label);
                    }
                }
            }
            lines_rendered += 1;
        }
        term.write_str(&frame_buf)?;
        term.flush()?;

        let key = term.read_key()?;
        term.clear_last_lines(lines_rendered)?;

        match key {
            console::Key::ArrowDown | console::Key::Char('j') | console::Key::Tab => {
                cursor_pos = (cursor_pos + 1) % selectable.len();
            }
            console::Key::ArrowUp | console::Key::Char('k') | console::Key::BackTab => {
                cursor_pos = (cursor_pos + selectable.len() - 1) % selectable.len();
            }
            console::Key::PageDown => {
                cursor_pos = (cursor_pos + 6).min(selectable.len() - 1);
            }
            console::Key::PageUp => {
                cursor_pos = cursor_pos.saturating_sub(6);
            }
            console::Key::Home => {
                cursor_pos = 0;
            }
            console::Key::End => {
                cursor_pos = selectable.len() - 1;
            }
            console::Key::Enter => {
                term.show_cursor()?;
                return Ok(Some(selectable[cursor_pos].1));
            }
            console::Key::Escape | console::Key::Char('q') => {
                term.show_cursor()?;
                return Ok(None);
            }
            _ => {}
        }
    }
}

fn copy_to_clipboard(text: &str) {
    let wayland = std::process::Command::new("wl-copy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()
        });
    if wayland.is_ok() {
        println!("✓ Copied to clipboard (Wayland): {text}");
        return;
    }

    let x11 = std::process::Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()
        });
    if x11.is_ok() {
        println!("✓ Copied to clipboard (X11): {text}");
        return;
    }

    println!("📋 URL: {text}");
}

fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    println!("🌐 Opened in browser: {url}");
}

struct App {
    opts: Options,
    net: net::Network,
    state: State,
    page_cache: HashMap<(String, u32), Vec<Episode>>,
    playback_speed: Option<String>,
    skip_intro: bool,
    skip_outro: bool,
    anime4k: bool,
    anime4k_preset: Option<String>,
    video_filter: Option<String>,
    custom_shader: Option<String>,
    stream_quality: Option<String>,
    audio_track: Option<String>,
    subtitle_track: Option<String>,
    fullscreen: bool,
    volume: Option<u32>,
}

impl App {
    fn new(opts: Options) -> Result<Self> {
        let net = net::Network::new(&opts.base_url, &opts.user_agent)?;
        Ok(Self {
            opts,
            net,
            state: state::load(),
            page_cache: HashMap::new(),
            playback_speed: None,
            skip_intro: false,
            skip_outro: false,
            anime4k: false,
            anime4k_preset: None,
            video_filter: None,
            custom_shader: None,
            stream_quality: None,
            audio_track: None,
            subtitle_track: None,
            fullscreen: false,
            volume: None,
        })
    }

    fn german(&self) -> bool {
        self.opts.ui_lang == UiLanguage::De
    }

    fn seasons(&self, series: &Series) -> Result<Vec<u32>> {
        let url = Url::parse(&series.url)?;
        let (html, _) = self.net.page(&url, None)?;
        let _observed_title = catalog::series_title(&html);
        let list = catalog::seasons(&html);
        anyhow::ensure!(
            !list.is_empty(),
            "No seasons found; the site layout may have changed"
        );
        Ok(list)
    }

    fn episodes(&mut self, series: &Series, season: u32) -> Result<Vec<Episode>> {
        let key = (series.slug.clone(), season);
        if let Some(list) = self.page_cache.get(&key) {
            return Ok(list.clone());
        }
        let base = Url::parse(&series.url)?;
        let path = if season == 0 {
            format!("{}/filme", series.url.trim_end_matches('/'))
        } else {
            format!("{}/staffel-{season}", series.url.trim_end_matches('/'))
        };
        let url = Url::parse(&path)?;
        let (html, _) = self.net.page(&url, None)?;
        let episodes = catalog::episodes(&html, &base, season);
        anyhow::ensure!(!episodes.is_empty(), "Season has no listed episodes");
        self.page_cache.insert(key, episodes.clone());
        Ok(episodes)
    }

    fn episode_hosts(&self, ep: &Episode) -> Result<Vec<Host>> {
        let url = Url::parse(&ep.url)?;
        let (html, _) = self.net.page(&url, None)?;
        let hosts = catalog::hosts(&html, &url);
        anyhow::ensure!(!hosts.is_empty(), "No stream hosts found on episode page");
        Ok(hosts)
    }

    fn next(
        &mut self,
        series: &Series,
        current: &Episode,
        auto_mode: bool,
    ) -> Result<Option<Episode>> {
        let eps = self.episodes(series, current.season)?;
        if let Some(ep) = eps.into_iter().find(|e| e.number > current.number) {
            return Ok(Some(ep));
        }
        if auto_mode {
            return Ok(None);
        }
        for season in self
            .seasons(series)?
            .into_iter()
            .filter(|s| *s > current.season)
        {
            if let Some(first) = self
                .episodes(series, season)
                .ok()
                .and_then(|eps| eps.into_iter().next())
            {
                return Ok(Some(first));
            }
        }
        Ok(None)
    }

    fn previous(&mut self, series: &Series, current: &Episode) -> Result<Option<Episode>> {
        let eps = self.episodes(series, current.season)?;
        if let Some(ep) = eps.into_iter().rev().find(|e| e.number < current.number) {
            return Ok(Some(ep));
        }
        let mut seasons = self.seasons(series)?;
        seasons.retain(|s| *s > 0 && *s < current.season);
        seasons.reverse();
        for season in seasons {
            if let Some(last) = self
                .episodes(series, season)
                .ok()
                .and_then(|eps| eps.into_iter().last())
            {
                return Ok(Some(last));
            }
        }
        Ok(None)
    }

    fn search_anime_interactive(&self) -> Result<Series> {
        print!(
            "{}: ",
            if self.german() {
                "Anime suchen"
            } else {
                "Search anime"
            }
        );
        io::stdout().flush()?;
        let mut q = String::new();
        io::stdin().read_line(&mut q)?;
        let q = q.trim();
        anyhow::ensure!(!q.is_empty(), "Search query cannot be empty");
        self.resolve_or_search(q)
    }

    fn resolve_or_search(&self, query: &str) -> Result<Series> {
        let query_trimmed = query.trim();

        // Direct URL detection (e.g. https://aniworld.to/anime/stream/frieren-beyond-journeys-end)
        let direct_url =
            if query_trimmed.starts_with("http://") || query_trimmed.starts_with("https://") {
                Url::parse(query_trimmed).ok()
            } else if let Some(stripped) = query_trimmed.strip_prefix("/anime/stream/") {
                self.net
                    .root()
                    .join(&format!("/anime/stream/{stripped}"))
                    .ok()
            } else {
                None
            };

        if let Some(url) = direct_url {
            if let Ok((html, final_url)) = self.net.page(&url, None) {
                let slug = final_url
                    .path()
                    .strip_prefix("/anime/stream/")
                    .and_then(|s| s.split('/').next())
                    .unwrap_or("")
                    .to_string();
                if !slug.is_empty() {
                    let title = catalog::series_title(&html).unwrap_or_else(|| slug.clone());
                    return Ok(Series {
                        title,
                        slug,
                        url: final_url.to_string(),
                    });
                }
            }
        }

        // Search API with direct slug fallback
        let results = self
            .net
            .get_search(query_trimmed)
            .ok()
            .and_then(|body| catalog::search_results(&body, self.net.root()).ok())
            .unwrap_or_default();

        if results.is_empty() {
            let slug = query_trimmed.trim_matches('/');
            if let Ok(direct) = self.net.root().join(&format!("/anime/stream/{slug}")) {
                if let Ok((html, final_url)) = self.net.page(&direct, None) {
                    if let Some(title) = catalog::series_title(&html) {
                        return Ok(Series {
                            title,
                            slug: slug.to_string(),
                            url: final_url.to_string(),
                        });
                    }
                }
            }
        }

        anyhow::ensure!(!results.is_empty(), "No series matched '{query}'");
        let labels = results.iter().map(|s| s.title.clone()).collect::<Vec<_>>();
        let index = self.pick_menu(
            if self.german() {
                "Anime auswählen"
            } else {
                "Choose series"
            },
            &labels,
            0,
        )?;
        Ok(results[index].clone())
    }

    fn browse_catalog(&self) -> Result<Series> {
        if self.opts.no_menu || self.opts.json {
            return self.browse_popular();
        }

        let options = vec![
            if self.german() {
                "🔤 Alphabetisch durchsuchen (A-Z, 0-9)"
            } else {
                "🔤 Browse Alphabetically (A-Z, 0-9)"
            }
            .to_string(),
            if self.german() {
                "🔥 Beliebte Animes (Popular)"
            } else {
                "🔥 Popular Animes"
            }
            .to_string(),
            if self.german() {
                "🎭 Nach Genre durchsuchen"
            } else {
                "🎭 Browse by Genre"
            }
            .to_string(),
            if self.german() {
                "🔍 Anime suchen"
            } else {
                "🔍 Search anime by name"
            }
            .to_string(),
        ];

        let prompt = if self.german() {
            "Anime-Katalog"
        } else {
            "Anime Catalog"
        };
        let choice = self.pick_menu(prompt, &options, 0)?;

        match choice {
            0 => self.browse_alphabet(),
            1 => self.browse_popular(),
            2 => self.browse_genres(None),
            _ => self.search_anime_interactive(),
        }
    }

    fn browse_alphabet(&self) -> Result<Series> {
        let letters = vec![
            "0-9", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P",
            "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
        ];
        let labels = letters.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let prompt = if self.german() {
            "Buchstabe wählen"
        } else {
            "Select letter"
        };
        let idx = self.pick_menu(prompt, &labels, 1)?;
        let letter = letters[idx];

        let path = format!("/katalog/{letter}");
        let url = self.net.root().join(&path)?;
        println!("⏳ Loading catalog for '{letter}'...");
        let (html, _) = self.net.page(&url, None)?;
        let animes = catalog::parse_catalog_page(&html, self.net.root());
        anyhow::ensure!(!animes.is_empty(), "No anime found for letter '{letter}'");

        if self.opts.no_menu || self.opts.json {
            return Ok(animes[0].clone());
        }

        let mut titles = Vec::with_capacity(animes.len() + 1);
        titles.push(format!("🔍 Filter / Search within '{letter}'"));
        for s in &animes {
            titles.push(s.title.clone());
        }
        let sel = self.pick_menu(
            &format!("Animes ({letter}) - {} titles", animes.len()),
            &titles,
            1,
        )?;
        if sel == 0 {
            print!("Filter within '{letter}': ");
            io::stdout().flush()?;
            let mut filter_input = String::new();
            io::stdin().read_line(&mut filter_input)?;
            let fi = filter_input.trim().to_lowercase();
            let filtered: Vec<&Series> = animes
                .iter()
                .filter(|s| s.title.to_lowercase().contains(&fi))
                .collect();
            anyhow::ensure!(
                !filtered.is_empty(),
                "No anime matched '{fi}' under '{letter}'"
            );
            let filtered_titles = filtered.iter().map(|s| s.title.clone()).collect::<Vec<_>>();
            let f_sel = self.pick_menu(&format!("Matches for '{fi}'"), &filtered_titles, 0)?;
            Ok(filtered[f_sel].clone())
        } else {
            Ok(animes[sel - 1].clone())
        }
    }

    fn browse_popular(&self) -> Result<Series> {
        let url = self.net.root().join("/beliebte-animes")?;
        println!("⏳ Loading popular animes...");
        let (html, _) = self.net.page(&url, None)?;
        let animes = catalog::parse_catalog_page(&html, self.net.root());
        anyhow::ensure!(!animes.is_empty(), "No popular anime found");

        if self.opts.no_menu || self.opts.json {
            return Ok(animes[0].clone());
        }

        let titles = animes.iter().map(|s| s.title.clone()).collect::<Vec<_>>();
        let sel = self.pick_menu(
            &format!("Popular Animes - {} titles", animes.len()),
            &titles,
            0,
        )?;
        Ok(animes[sel].clone())
    }

    fn browse_genres(&self, direct_filter: Option<&str>) -> Result<Series> {
        let url = self.net.root().join("/animes")?;
        println!("⏳ Loading anime genres...");
        let (html, _) = self.net.page(&url, None)?;
        let genres = catalog::parse_genres(&html, self.net.root());
        anyhow::ensure!(!genres.is_empty(), "No genres found on AniWorld");

        let selected_genre = if let Some(filter) = direct_filter {
            let filter_lower = filter.to_lowercase();
            genres
                .into_iter()
                .find(|g| g.name.to_lowercase().contains(&filter_lower))
                .context(format!("Genre matching '{filter}' not found"))?
        } else {
            let labels = genres
                .iter()
                .map(|g| format!("{} ({} animes)", g.name, g.animes.len()))
                .collect::<Vec<_>>();
            let idx = self.pick_menu(
                if self.german() {
                    "Genre auswählen"
                } else {
                    "Select Genre"
                },
                &labels,
                0,
            )?;
            genres.into_iter().nth(idx).unwrap()
        };

        anyhow::ensure!(
            !selected_genre.animes.is_empty(),
            "No animes listed in genre '{}'",
            selected_genre.name
        );

        if self.opts.no_menu || self.opts.json {
            return Ok(selected_genre.animes[0].clone());
        }

        let mut titles = Vec::with_capacity(selected_genre.animes.len() + 1);
        titles.push(format!(
            "🔍 Filter / Search within '{}'",
            selected_genre.name
        ));
        for s in &selected_genre.animes {
            titles.push(s.title.clone());
        }

        let sel = self.pick_menu(
            &format!(
                "{} ({} titles)",
                selected_genre.name,
                selected_genre.animes.len()
            ),
            &titles,
            1,
        )?;

        if sel == 0 {
            print!("Filter {}: ", selected_genre.name);
            io::stdout().flush()?;
            let mut filter_input = String::new();
            io::stdin().read_line(&mut filter_input)?;
            let fi = filter_input.trim().to_lowercase();
            let filtered: Vec<&Series> = selected_genre
                .animes
                .iter()
                .filter(|s| s.title.to_lowercase().contains(&fi))
                .collect();
            anyhow::ensure!(
                !filtered.is_empty(),
                "No anime matched '{fi}' in genre '{}'",
                selected_genre.name
            );
            let filtered_titles = filtered.iter().map(|s| s.title.clone()).collect::<Vec<_>>();
            let f_sel = self.pick_menu(&format!("Matches for '{fi}'"), &filtered_titles, 0)?;
            Ok(filtered[f_sel].clone())
        } else {
            Ok(selected_genre.animes[sel - 1].clone())
        }
    }

    fn choose_series(&self) -> Result<Series> {
        if self.opts.watchlist {
            let entries = self.state.recent();
            anyhow::ensure!(
                !entries.is_empty(),
                "Watchlist empty: start watching a series first"
            );
            let names = entries
                .iter()
                .map(|(_, p)| format!("{} (S{:02}E{:02})", p.title, p.season, p.number))
                .collect::<Vec<_>>();
            let index = self.pick_menu(
                if self.german() {
                    "Merkliste"
                } else {
                    "Watchlist"
                },
                &names,
                0,
            )?;
            let (slug, saved) = &entries[index];
            return Ok(Series {
                title: saved.title.clone(),
                slug: slug.clone(),
                url: saved.url.clone(),
            });
        }

        if self.opts.catalog {
            return self.browse_catalog();
        }

        if let Some(genre) = &self.opts.genre {
            return self.browse_genres(if genre.is_empty() { None } else { Some(genre) });
        }

        if let Some(q) = &self.opts.query {
            return self.resolve_or_search(q);
        }

        anyhow::ensure!(
            !self.opts.no_menu && !self.opts.json,
            "Supply a query for noninteractive mode"
        );

        let main_choices = vec![
            if self.german() {
                "🔍 Anime suchen"
            } else {
                "🔍 Search animes"
            }
            .to_string(),
            if self.german() {
                "📚 Anime-Katalog (A-Z / Beliebt)"
            } else {
                "📚 Anime Catalog (A-Z / Popular)"
            }
            .to_string(),
            if self.german() {
                "🎭 Nach Genre durchsuchen"
            } else {
                "🎭 Browse by Genre"
            }
            .to_string(),
            if self.german() {
                "📑 Merkliste öffnen"
            } else {
                "📑 Watchlist"
            }
            .to_string(),
            if self.german() {
                "★ Favoriten öffnen"
            } else {
                "★ Favorites"
            }
            .to_string(),
            if self.german() {
                "📜 Verlauf anzeigen"
            } else {
                "📜 Watch History"
            }
            .to_string(),
        ];

        let prompt = if self.german() {
            "AniWorld Hauptmenü"
        } else {
            "AniWorld Menu"
        };
        let choice = self.pick_menu(prompt, &main_choices, 0)?;

        match choice {
            0 => self.search_anime_interactive(),
            1 => self.browse_catalog(),
            2 => self.browse_genres(None),
            3 => {
                let entries = self.state.recent();
                anyhow::ensure!(!entries.is_empty(), "Watchlist is empty");
                let names = entries
                    .iter()
                    .map(|(_, p)| format!("{} (S{:02}E{:02})", p.title, p.season, p.number))
                    .collect::<Vec<_>>();
                let idx = self.pick_menu("Watchlist", &names, 0)?;
                let (slug, saved) = &entries[idx];
                Ok(Series {
                    title: saved.title.clone(),
                    slug: slug.clone(),
                    url: saved.url.clone(),
                })
            }
            4 => {
                anyhow::ensure!(!self.state.favorites.is_empty(), "No favorites saved yet");
                let favs = self.state.favorites.clone();
                let idx = self.pick_menu("Favorites", &favs, 0)?;
                let slug = &favs[idx];
                let title = self
                    .state
                    .series
                    .get(slug)
                    .map(|p| p.title.clone())
                    .unwrap_or_else(|| slug.clone());
                let url = format!(
                    "{}/anime/stream/{slug}",
                    self.net.root().as_str().trim_end_matches('/')
                );
                Ok(Series {
                    title,
                    slug: slug.clone(),
                    url,
                })
            }
            _ => {
                let items = self.state.recent();
                if items.is_empty() {
                    println!("No watch history yet.");
                } else {
                    println!("\n📜 Watch History:");
                    for (_, p) in &items {
                        println!(
                            "  {}  S{:02}E{:02}  {}",
                            p.updated, p.season, p.number, p.title
                        );
                    }
                    println!();
                }
                self.search_anime_interactive()
            }
        }
    }

    fn pick_menu(&self, prompt: &str, options: &[String], default: usize) -> Result<usize> {
        anyhow::ensure!(!options.is_empty(), "No choices available for {prompt}");
        if options.len() == 1 || self.opts.no_menu || self.opts.json {
            return Ok(0);
        }
        let term_height = console::Term::stderr().size().0 as usize;
        let max_len = if term_height >= 12 {
            term_height.saturating_sub(4).clamp(6, 12)
        } else {
            8
        };
        let default_idx = default.min(options.len() - 1);
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .default(default_idx)
            .items(options)
            .max_length(max_len)
            .interact_opt()?
            .context("Selection cancelled")
    }

    fn choose_episode(&mut self, series: &Series) -> Result<Episode> {
        if let Some(ep) = &self.opts.episode {
            let (season, number) = options::parse_episode(ep)?;
            let all = self.episodes(series, season)?;
            return all
                .into_iter()
                .find(|e| e.number == number)
                .context("Requested episode not found");
        }
        if let Some(saved) = self.state.series.get(&series.slug).cloned() {
            let last = Episode {
                season: saved.season,
                number: saved.number,
                title: saved.label.clone(),
                url: String::new(),
            };
            if let Ok(Some(next)) = self.next(series, &last, false) {
                let continue_playing = if self.opts.no_menu || self.opts.json {
                    true
                } else {
                    let msg = format!(
                        "Continue {} after S{:02}E{:02}?",
                        series.title, saved.season, saved.number
                    );
                    Select::with_theme(&ColorfulTheme::default())
                        .with_prompt(msg)
                        .items(&["Continue", "Select episode"])
                        .default(0)
                        .interact()?
                        == 0
                };
                if continue_playing {
                    return Ok(next);
                }
            }
        }
        self.select_episode(series, None)
    }

    fn select_season(&mut self, series: &Series, current_season: Option<u32>) -> Result<u32> {
        let seasons = self.seasons(series)?;
        let choices = seasons
            .iter()
            .map(|s| {
                if *s == 0 {
                    "Movies".to_owned()
                } else {
                    format!("Season {s}")
                }
            })
            .collect::<Vec<_>>();
        let default_idx = current_season
            .and_then(|cur| seasons.iter().position(|s| *s == cur))
            .unwrap_or(0);
        let season_index = if self.opts.no_menu || self.opts.json {
            seasons.iter().position(|s| *s > 0).unwrap_or(0)
        } else {
            self.pick_menu("Season / movies", &choices, default_idx)?
        };
        Ok(seasons[season_index])
    }

    fn select_episode_in_season(
        &mut self,
        series: &Series,
        season: u32,
        current_ep: Option<&Episode>,
    ) -> Result<Episode> {
        let all = self.episodes(series, season)?;
        anyhow::ensure!(!all.is_empty(), "No episodes found in this season");

        let choices = all.iter().map(Episode::display).collect::<Vec<_>>();
        let default_idx = current_ep
            .and_then(|cur| {
                all.iter()
                    .position(|e| e.season == cur.season && e.number == cur.number)
            })
            .unwrap_or(0);

        let prompt = if season == 0 {
            "Select movie"
        } else {
            "Select episode"
        };
        let index = self.pick_menu(prompt, &choices, default_idx)?;
        Ok(all[index].clone())
    }

    fn select_episode(&mut self, series: &Series, current_ep: Option<&Episode>) -> Result<Episode> {
        let current_season = current_ep.map(|e| e.season);
        let season = self.select_season(series, current_season)?;
        self.select_episode_in_season(series, season, current_ep)
    }

    fn choose_language(&self, hosts: &[Host], remembered: Option<&str>) -> Result<String> {
        let pref = self.opts.preferred_languages();
        let available = resolve::ordered_languages(hosts, &pref);
        anyhow::ensure!(!available.is_empty(), "No languages offered");
        if let Some(lang) = remembered.filter(|lang| available.iter().any(|v| v == *lang)) {
            return Ok(lang.to_string());
        }
        let index = self.pick_menu(
            if self.german() {
                "Sprache"
            } else {
                "Audio / subtitles"
            },
            &available,
            0,
        )?;
        Ok(available[index].clone())
    }

    fn build_play_args(&self, extra: &[String]) -> Vec<String> {
        let mut args = self.opts.mpv_args.clone();
        args.extend_from_slice(extra);

        if !args
            .iter()
            .any(|a| a.starts_with("--save-position-on-quit"))
        {
            args.push("--save-position-on-quit".to_string());
        }
        if let Some(spd) = &self.playback_speed {
            args.push(format!("--speed={spd}"));
        }
        if self.skip_intro {
            args.push("--start=85".to_string());
        }
        if self.fullscreen && !args.iter().any(|a| a == "--fs" || a == "--fullscreen") {
            args.push("--fs".to_string());
        }
        if let Some(vol) = self.volume {
            args.push(format!("--volume={vol}"));
        }
        if let Some(q) = &self.stream_quality {
            args.push(format!("--ytdl-format={q}"));
        }
        if let Some(aid) = &self.audio_track {
            args.push(format!("--aid={aid}"));
        }
        if let Some(sid) = &self.subtitle_track {
            if sid == "no" {
                args.push("--sub-visibility=no".to_string());
            } else {
                args.push(format!("--sid={sid}"));
            }
        }
        if self.anime4k {
            let preset = self
                .anime4k_preset
                .as_deref()
                .unwrap_or("~~/shaders/Anime4K_Upscale_Denoise_CNN_x2_M.glsl");
            args.push(format!("--glsl-shaders={preset}"));
        } else if let Some(shader) = &self.custom_shader {
            args.push(format!("--glsl-shaders={shader}"));
        }
        if let Some(filter) = &self.video_filter {
            args.push(filter.clone());
        }
        args
    }

    fn start(&mut self) -> Result<()> {
        let mut series = self.choose_series()?;
        let mut current = self.choose_episode(&series)?;
        let mut remembered_language: Option<String> = None;
        let mut forced_language: Option<String> = None;
        let mut forced_hoster: Option<String> = None;
        let mut extra_mpv_flags: Vec<String> = Vec::new();

        loop {
            if !self.opts.json {
                println!("\n▶ {}  {}", series.title, current.display());
            }
            let hosts = self.episode_hosts(&current)?;
            let chosen_lang = if let Some(fl) = forced_language.take() {
                Some(fl)
            } else if self.opts.no_menu || self.opts.json {
                None
            } else {
                let lang = self.choose_language(&hosts, remembered_language.as_deref())?;
                Some(lang)
            };

            let mut host_priority = self.opts.preferred_hosters();
            if let Some(fh) = forced_hoster.take() {
                host_priority.retain(|h| !h.eq_ignore_ascii_case(&fh));
                host_priority.insert(0, fh);
            }

            let (stream, chosen) = resolve::resolve(
                &hosts,
                &self.net,
                &self.opts.user_agent,
                chosen_lang.as_deref(),
                &self.opts.preferred_languages(),
                &host_priority,
                !self.opts.no_verify,
                self.opts.json,
            )?;
            remembered_language = Some(chosen.language.clone());
            if self.opts.json {
                let output = serde_json::json!({"series":series, "episode":current,
                    "host":chosen.name, "language":chosen.language,
                    "url":stream.url, "headers":{"User-Agent":stream.user_agent,"Referer":stream.referer}});
                println!("{}", serde_json::to_string_pretty(&output)?);
                return Ok(());
            }
            if self.opts.debug {
                println!("Stream URL: {}", stream.url);
                println!("User-Agent: {}", stream.user_agent);
                println!("Referer: {}", stream.referer.as_deref().unwrap_or(""));
                return Ok(());
            }
            println!("Resolved via {} ({})\n", chosen.name, chosen.language);

            let play_args = self.build_play_args(&extra_mpv_flags);
            extra_mpv_flags.clear();

            player::play(
                self.opts.player.as_deref(),
                &stream,
                &series,
                &current,
                &play_args,
            )?;
            self.state.record(&series, &current);
            if let Err(e) = state::save(&self.state) {
                eprintln!("Warning: could not save progress: {e}");
            }
            if self.opts.auto {
                if let Some(next) = self.next(&series, &current, true)? {
                    current = next;
                    continue;
                }
                println!("Season complete.");
                break;
            }
            if self.opts.no_menu {
                break;
            }

            let mut last_selected_action = Some("Next episode");
            let mut break_to_play = false;

            while !break_to_play {
                let choice = run_after_playback_menu(last_selected_action)?;
                last_selected_action = choice;

                match choice {
                    Some("Next episode") => {
                        if let Some(next) = self.next(&series, &current, false)? {
                            current = next;
                            break_to_play = true;
                        } else {
                            println!("No later episode found in this series.");
                        }
                    }
                    Some("Previous episode") => {
                        if let Some(prev) = self.previous(&series, &current)? {
                            current = prev;
                            break_to_play = true;
                        } else {
                            println!("No earlier episode found in this series.");
                        }
                    }
                    Some("Replay episode") => {
                        extra_mpv_flags.push("--start=0".to_string());
                        println!("Replaying {} from beginning...", current.display());
                        break_to_play = true;
                    }
                    Some("Resume playback") => {
                        extra_mpv_flags.push("--save-position-on-quit".to_string());
                        println!("Resuming playback of {}...", current.display());
                        break_to_play = true;
                    }
                    Some("Next unwatched episode") => {
                        let seasons = self.seasons(&series)?;
                        let mut found = None;
                        for season in seasons {
                            let eps = self.episodes(&series, season)?;
                            for ep in eps {
                                if ep.season > current.season
                                    || (ep.season == current.season && ep.number > current.number)
                                {
                                    found = Some(ep);
                                    break;
                                }
                            }
                            if found.is_some() {
                                break;
                            }
                        }
                        if let Some(ep) = found {
                            current = ep;
                            break_to_play = true;
                        } else {
                            println!("All caught up! No later unwatched episodes found.");
                        }
                    }
                    Some("Random episode") => {
                        let seasons = self.seasons(&series)?;
                        if !seasons.is_empty() {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_nanos() as usize;
                            let s_idx = now % seasons.len();
                            let season = seasons[s_idx];
                            let eps = self.episodes(&series, season)?;
                            if !eps.is_empty() {
                                let e_idx = (now / 7) % eps.len();
                                current = eps[e_idx].clone();
                                println!("🎲 Selected random episode: {}", current.display());
                                break_to_play = true;
                            }
                        }
                    }
                    Some("Select episode") => {
                        match self.select_episode_in_season(&series, current.season, Some(&current))
                        {
                            Ok(ep) => {
                                current = ep;
                                break_to_play = true;
                            }
                            Err(e) => eprintln!("Episode selection error: {e}"),
                        }
                    }
                    Some("Select season") => match self.select_episode(&series, Some(&current)) {
                        Ok(ep) => {
                            current = ep;
                            break_to_play = true;
                        }
                        Err(e) => eprintln!("Season/episode selection error: {e}"),
                    },
                    Some("Jump to episode") => {
                        let eps = self.episodes(&series, current.season)?;
                        print!("Enter episode number (1-{}): ", eps.len());
                        io::stdout().flush()?;
                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;
                        if let Ok(num) = input.trim().parse::<u32>() {
                            if let Some(ep) = eps.into_iter().find(|e| e.number == num) {
                                current = ep;
                                break_to_play = true;
                            } else {
                                println!("Episode {num} not found in this season.");
                            }
                        } else {
                            println!("Invalid episode number.");
                        }
                    }
                    Some("Search another anime") => match self.search_anime_interactive() {
                        Ok(s) => match self.select_episode(&s, None) {
                            Ok(ep) => {
                                series = s;
                                remembered_language = None;
                                forced_hoster = None;
                                forced_language = None;
                                current = ep;
                                break_to_play = true;
                            }
                            Err(e) => eprintln!("Could not select episode: {e}"),
                        },
                        Err(e) => eprintln!("Search error: {e}"),
                    },
                    Some("Browse anime catalog") => match self.browse_catalog() {
                        Ok(s) => match self.select_episode(&s, None) {
                            Ok(ep) => {
                                series = s;
                                remembered_language = None;
                                forced_hoster = None;
                                forced_language = None;
                                current = ep;
                                break_to_play = true;
                            }
                            Err(e) => eprintln!("Could not select episode: {e}"),
                        },
                        Err(e) => eprintln!("Catalog error: {e}"),
                    },
                    Some("Browse by genre") => match self.browse_genres(None) {
                        Ok(s) => match self.select_episode(&s, None) {
                            Ok(ep) => {
                                series = s;
                                remembered_language = None;
                                forced_hoster = None;
                                forced_language = None;
                                current = ep;
                                break_to_play = true;
                            }
                            Err(e) => eprintln!("Could not select episode: {e}"),
                        },
                        Err(e) => eprintln!("Genre browse error: {e}"),
                    },
                    Some("Continue another anime") => {
                        let recent = self.state.recent();
                        if recent.is_empty() {
                            println!("Watchlist is empty.");
                        } else {
                            let choices: Vec<String> = recent
                                .iter()
                                .map(|(_, p)| {
                                    format!("{} (S{:02}E{:02})", p.title, p.season, p.number)
                                })
                                .collect();
                            if let Ok(idx) =
                                self.pick_menu("Select series from watchlist", &choices, 0)
                            {
                                let (slug, p) = &recent[idx];
                                series = Series {
                                    title: p.title.clone(),
                                    slug: slug.clone(),
                                    url: p.url.clone(),
                                };
                                current = self.select_episode(&series, None)?;
                                remembered_language = None;
                                forced_hoster = None;
                                forced_language = None;
                                break_to_play = true;
                            }
                        }
                    }
                    Some("Change language") => match self.episode_hosts(&current) {
                        Ok(hosts) => {
                            let available = resolve::ordered_languages(
                                &hosts,
                                &self.opts.preferred_languages(),
                            );
                            if available.is_empty() {
                                println!("No languages offered for this episode.");
                            } else {
                                let default_idx = remembered_language
                                    .as_deref()
                                    .and_then(|l| available.iter().position(|v| v == l))
                                    .unwrap_or(0);
                                if let Ok(idx) =
                                    self.pick_menu("Audio / subtitles", &available, default_idx)
                                {
                                    remembered_language = Some(available[idx].clone());
                                    forced_language = Some(available[idx].clone());
                                    println!("Language set to: {}", available[idx]);
                                    break_to_play = true;
                                }
                            }
                        }
                        Err(e) => eprintln!("Could not load episode languages: {e}"),
                    },
                    Some("Change hoster") => match self.episode_hosts(&current) {
                        Ok(hosts) => {
                            let host_labels = hosts
                                .iter()
                                .map(|h| format!("{} ({})", h.name, h.language))
                                .collect::<Vec<_>>();
                            if host_labels.is_empty() {
                                println!("No hosters found for this episode.");
                            } else if let Ok(idx) = self.pick_menu("Choose hoster", &host_labels, 0)
                            {
                                let h = &hosts[idx];
                                forced_hoster = Some(h.name.clone());
                                forced_language = Some(h.language.clone());
                                remembered_language = Some(h.language.clone());
                                println!("Hoster set to: {} ({})", h.name, h.language);
                                break_to_play = true;
                            }
                        }
                        Err(e) => eprintln!("Could not load hosters: {e}"),
                    },
                    Some("Change stream quality") => {
                        let qualities = vec![
                            "1080p (Best / Default)".to_string(),
                            "720p".to_string(),
                            "480p".to_string(),
                            "360p".to_string(),
                        ];
                        if let Ok(idx) = self.pick_menu("Select stream quality", &qualities, 0) {
                            self.stream_quality = match idx {
                                1 => Some(
                                    "bestvideo[height<=720]+bestaudio/best[height<=720]".into(),
                                ),
                                2 => Some(
                                    "bestvideo[height<=480]+bestaudio/best[height<=480]".into(),
                                ),
                                3 => Some(
                                    "bestvideo[height<=360]+bestaudio/best[height<=360]".into(),
                                ),
                                _ => None,
                            };
                            println!("Stream quality set to: {}", qualities[idx]);
                        }
                    }
                    Some("Find fastest hoster") => match self.episode_hosts(&current) {
                        Ok(hosts) => {
                            println!("Testing hoster response times...");
                            let mut fastest: Option<(&Host, std::time::Duration)> = None;
                            for h in &hosts {
                                let start = std::time::Instant::now();
                                if let Ok(res) = self
                                    .net
                                    .client
                                    .get(&h.url)
                                    .timeout(std::time::Duration::from_secs(3))
                                    .send()
                                    && res.status().is_success()
                                {
                                    let elapsed = start.elapsed();
                                    println!("  {} ({}) -> {:.2?}", h.name, h.language, elapsed);
                                    if fastest.as_ref().is_none_or(|(_, b)| elapsed < *b) {
                                        fastest = Some((h, elapsed));
                                    }
                                }
                            }
                            if let Some((h, dur)) = fastest {
                                println!(
                                    "⚡ Fastest hoster: {} ({} in {:.2?})",
                                    h.name, h.language, dur
                                );
                                forced_hoster = Some(h.name.clone());
                                forced_language = Some(h.language.clone());
                                remembered_language = Some(h.language.clone());
                                break_to_play = true;
                            } else {
                                println!("Could not benchmark hosters.");
                            }
                        }
                        Err(e) => eprintln!("Could not load hosters: {e}"),
                    },
                    Some("Retry stream") => {
                        println!("Retrying stream resolution...");
                        break_to_play = true;
                    }
                    Some("Test all hosters") => match self.episode_hosts(&current) {
                        Ok(hosts) => {
                            println!("\n🔍 Hoster Status Check:");
                            for h in &hosts {
                                match self
                                    .net
                                    .client
                                    .head(&h.url)
                                    .timeout(std::time::Duration::from_secs(4))
                                    .send()
                                {
                                    Ok(res) => println!(
                                        "  [{}] {} -> HTTP {}",
                                        h.name,
                                        h.language,
                                        res.status()
                                    ),
                                    Err(e) => {
                                        println!("  [{}] {} -> Failed: {e}", h.name, h.language)
                                    }
                                }
                            }
                            println!();
                        }
                        Err(e) => eprintln!("Could not load hosters: {e}"),
                    },
                    Some("Change playback speed") => {
                        let speeds = vec![
                            "0.75x".to_string(),
                            "1.0x (Normal)".to_string(),
                            "1.25x".to_string(),
                            "1.5x".to_string(),
                            "1.75x".to_string(),
                            "2.0x".to_string(),
                        ];
                        if let Ok(idx) = self.pick_menu("Playback speed", &speeds, 1) {
                            let val = match idx {
                                0 => "0.75",
                                1 => "1.0",
                                2 => "1.25",
                                3 => "1.5",
                                4 => "1.75",
                                5 => "2.0",
                                _ => "1.0",
                            };
                            self.playback_speed = Some(val.to_string());
                            println!("Playback speed set to: {val}x");
                        }
                    }
                    Some("Select audio track") => {
                        let tracks = vec![
                            "Track 1 (Default)".to_string(),
                            "Track 2".to_string(),
                            "German Track (deu)".to_string(),
                            "Japanese Track (jpn)".to_string(),
                            "English Track (eng)".to_string(),
                        ];
                        if let Ok(idx) = self.pick_menu("Audio Track", &tracks, 0) {
                            self.audio_track = match idx {
                                1 => Some("2".to_string()),
                                2 => Some("deu".to_string()),
                                3 => Some("jpn".to_string()),
                                4 => Some("eng".to_string()),
                                _ => Some("1".to_string()),
                            };
                            println!("Audio track set to: {}", tracks[idx]);
                        }
                    }
                    Some("Select subtitle track") => {
                        let subs = vec![
                            "Subtitles ON (Default / Track 1)".to_string(),
                            "Subtitles ON (German)".to_string(),
                            "Subtitles ON (English)".to_string(),
                            "Subtitles OFF".to_string(),
                        ];
                        if let Ok(idx) = self.pick_menu("Subtitle Track", &subs, 0) {
                            self.subtitle_track = match idx {
                                1 => Some("ger".to_string()),
                                2 => Some("eng".to_string()),
                                3 => Some("no".to_string()),
                                _ => Some("1".to_string()),
                            };
                            println!("Subtitle track set to: {}", subs[idx]);
                        }
                    }
                    Some("Toggle auto-next") => {
                        self.opts.auto = !self.opts.auto;
                        println!(
                            "Auto-next is now {}",
                            if self.opts.auto {
                                "ON (Episodes will autoplay consecutively)"
                            } else {
                                "OFF"
                            }
                        );
                    }
                    Some("Toggle skip intro") => {
                        self.skip_intro = !self.skip_intro;
                        println!(
                            "Skip intro (85s offset) is now {}",
                            if self.skip_intro { "ON" } else { "OFF" }
                        );
                    }
                    Some("Toggle skip outro") => {
                        self.skip_outro = !self.skip_outro;
                        println!(
                            "Skip outro is now {}",
                            if self.skip_outro { "ON" } else { "OFF" }
                        );
                    }
                    Some("Player settings") => {
                        let settings_choices = vec![
                            format!(
                                "Toggle fullscreen (currently: {})",
                                if self.fullscreen { "ON" } else { "OFF" }
                            ),
                            format!("Set volume (currently: {}%)", self.volume.unwrap_or(100)),
                        ];
                        if let Ok(idx) = self.pick_menu("Player settings", &settings_choices, 0) {
                            if idx == 0 {
                                self.fullscreen = !self.fullscreen;
                                println!(
                                    "Fullscreen: {}",
                                    if self.fullscreen { "ON" } else { "OFF" }
                                );
                            } else {
                                print!("Enter volume (0-150): ");
                                io::stdout().flush()?;
                                let mut input = String::new();
                                io::stdin().read_line(&mut input)?;
                                if let Ok(vol) = input.trim().parse::<u32>() {
                                    self.volume = Some(vol.min(150));
                                    println!("Volume set to: {}%", vol.min(150));
                                }
                            }
                        }
                    }
                    Some("Toggle Anime4K") => {
                        self.anime4k = !self.anime4k;
                        println!(
                            "Anime4K upscaling shader: {}",
                            if self.anime4k { "ENABLED" } else { "DISABLED" }
                        );
                    }
                    Some("Select Anime4K preset") => {
                        let presets = vec![
                            "Anime4K Mode A (Fast Upscale)".to_string(),
                            "Anime4K Mode B (Reconstruction + Upscale)".to_string(),
                            "Anime4K Mode C (Denoise + Deblur + Upscale)".to_string(),
                            "Anime4K Ultra Fast".to_string(),
                        ];
                        if let Ok(idx) = self.pick_menu("Anime4K Preset", &presets, 0) {
                            let shader = match idx {
                                0 => "~~/shaders/Anime4K_ModeA.glsl",
                                1 => "~~/shaders/Anime4K_ModeB.glsl",
                                2 => "~~/shaders/Anime4K_ModeC.glsl",
                                _ => "~~/shaders/Anime4K_Upscale_Denoise_CNN_x2_M.glsl",
                            };
                            self.anime4k_preset = Some(shader.to_string());
                            self.anime4k = true;
                            println!("Anime4K preset set to: {}", presets[idx]);
                        }
                    }
                    Some("Video filters") => {
                        let filters = vec![
                            "None (Default)".to_string(),
                            "Anime Contrast Boost (--contrast=10)".to_string(),
                            "Vivid Saturation (--saturation=15)".to_string(),
                            "Cinematic Warmth (--gamma=5)".to_string(),
                        ];
                        if let Ok(idx) = self.pick_menu("Video filters", &filters, 0) {
                            self.video_filter = match idx {
                                1 => Some("--contrast=10".to_string()),
                                2 => Some("--saturation=15".to_string()),
                                3 => Some("--gamma=5".to_string()),
                                _ => None,
                            };
                            println!("Video filter set to: {}", filters[idx]);
                        }
                    }
                    Some("Shader settings") => {
                        print!("Enter custom GLSL shader path: ");
                        io::stdout().flush()?;
                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;
                        let path = input.trim();
                        if !path.is_empty() {
                            self.custom_shader = Some(path.to_string());
                            self.anime4k = false;
                            println!("Custom shader set: {path}");
                        }
                    }
                    Some("Open watchlist") => {
                        let recent = self.state.recent();
                        if recent.is_empty() {
                            println!("Watchlist is empty.");
                        } else {
                            let choices: Vec<String> = recent
                                .iter()
                                .map(|(_, p)| {
                                    format!("{} (S{:02}E{:02})", p.title, p.season, p.number)
                                })
                                .collect();
                            if let Ok(idx) = self.pick_menu("Watchlist", &choices, 0) {
                                let (slug, p) = &recent[idx];
                                series = Series {
                                    title: p.title.clone(),
                                    slug: slug.clone(),
                                    url: p.url.clone(),
                                };
                                current = self.select_episode(&series, None)?;
                                break_to_play = true;
                            }
                        }
                    }
                    Some("Open favorites") => {
                        if self.state.favorites.is_empty() {
                            println!(
                                "No favorites saved yet. Use 'Add/remove favorite' to star an anime."
                            );
                        } else {
                            let favs = self.state.favorites.clone();
                            if let Ok(idx) = self.pick_menu("Favorites", &favs, 0) {
                                let slug = &favs[idx];
                                let title = self
                                    .state
                                    .series
                                    .get(slug)
                                    .map(|p| p.title.clone())
                                    .unwrap_or_else(|| slug.clone());
                                let url = format!(
                                    "{}/anime/stream/{slug}",
                                    self.net.root().as_str().trim_end_matches('/')
                                );
                                series = Series {
                                    title,
                                    slug: slug.clone(),
                                    url,
                                };
                                current = self.select_episode(&series, None)?;
                                break_to_play = true;
                            }
                        }
                    }
                    Some("View watch history") => {
                        let items = self.state.recent();
                        if items.is_empty() {
                            println!("No watch history yet.");
                        } else {
                            println!("\n📜 Watch History:");
                            for (_, p) in items {
                                println!(
                                    "  {}  S{:02}E{:02}  {}",
                                    p.updated, p.season, p.number, p.title
                                );
                            }
                            println!();
                        }
                    }
                    Some("Add/remove favorite") => {
                        let is_fav = self.state.toggle_favorite(&series.slug);
                        let _ = state::save(&self.state);
                        if is_fav {
                            println!("★ Added '{}' to favorites!", series.title);
                        } else {
                            println!("☆ Removed '{}' from favorites.", series.title);
                        }
                    }
                    Some("Mark as watched") => {
                        self.state.record(&series, &current);
                        let _ = state::save(&self.state);
                        println!("✓ Marked as watched: {}", current.display());
                    }
                    Some("Mark as unwatched") => {
                        self.state.series.remove(&series.slug);
                        let _ = state::save(&self.state);
                        println!("✓ Removed from watch history: {}", series.title);
                    }
                    Some("Episode information") => {
                        println!("\nℹ️ Episode Information:");
                        println!("  Series: {}", series.title);
                        println!("  Slug: {}", series.slug);
                        println!("  Season: {}", current.season);
                        println!("  Episode: {}", current.number);
                        println!("  Title: {}", current.title);
                        println!(
                            "  Favorite: {}",
                            if self.state.is_favorite(&series.slug) {
                                "Yes ★"
                            } else {
                                "No ☆"
                            }
                        );
                        println!("  Episode URL: {}", current.url);
                        if let Ok(hosts) = self.episode_hosts(&current) {
                            let host_str = hosts
                                .iter()
                                .map(|h| format!("{} ({})", h.name, h.language))
                                .collect::<Vec<_>>()
                                .join(", ");
                            println!("  Available Streams: {host_str}");
                        }
                        println!();
                    }
                    Some("Playback statistics") => {
                        println!("\n📊 Playback Statistics:");
                        println!(
                            "  Player: {}",
                            player::find_player(self.opts.player.as_deref())
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| "Not found".into())
                        );
                        println!("  State File: {}", state::location().display());
                        println!("  Cached Seasons/Episodes: {}", self.page_cache.len());
                        println!("  Base Endpoint: {}", self.net.root());
                        println!();
                    }
                    Some("Copy episode URL") => {
                        copy_to_clipboard(&current.url);
                    }
                    Some("Copy stream URL") => {
                        copy_to_clipboard(&stream.url);
                    }
                    Some("Open in browser") => {
                        open_browser(&current.url);
                    }
                    Some("Refresh episode list") => {
                        self.page_cache.retain(|(slug, _), _| slug != &series.slug);
                        println!("✓ Refreshed episode list cache for {}", series.title);
                    }
                    Some("Refresh stream sources") => {
                        println!("✓ Refreshed stream sources for {}", current.display());
                        break_to_play = true;
                    }
                    Some("Clear cache") => {
                        self.page_cache.clear();
                        println!("✓ In-memory cache cleared.");
                    }
                    Some("Settings") => {
                        println!("\n⚙️ Settings & Configuration:");
                        println!("  Base URL: {}", self.opts.base_url);
                        println!("  User-Agent: {}", self.opts.user_agent);
                        println!("  UI Language: {:?}", self.opts.ui_lang);
                        println!(
                            "  Preferred Languages: {:?}",
                            self.opts.preferred_languages()
                        );
                        println!("  Preferred Hosters: {:?}", self.opts.preferred_hosters());
                        println!("  Auto-play Binge: {}", self.opts.auto);
                        println!("  Verify Segments: {}", !self.opts.no_verify);
                        println!();
                    }
                    _ => {
                        println!("Goodbye!");
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }
}

fn main() {
    let options = Options::parse();
    if let Err(e) = run(options) {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

fn run(opts: Options) -> Result<()> {
    if opts.doctor {
        println!(
            "aniworld-cli-rust {} | Rust 2024 | MIT",
            env!("CARGO_PKG_VERSION")
        );
        println!("State: {}", state::location().display());
        println!(
            "mpv: {}",
            player::find_player(opts.player.as_deref())
                .map(|s| s.display().to_string())
                .unwrap_or("NOT FOUND".into())
        );
        let net = net::Network::new(&opts.base_url, &opts.user_agent)?;
        match net.page(net.root(), None) {
            Ok((_, final_url)) => println!("Endpoint: reachable ({final_url})"),
            Err(e) => println!("Endpoint: unavailable ({e:#})"),
        }
        return Ok(());
    }
    if opts.history {
        let items = state::load().recent();
        if items.is_empty() {
            println!("No watch history yet.");
        }
        for (_, p) in items {
            println!(
                "{}  S{:02}E{:02}  {}",
                p.updated, p.season, p.number, p.title
            );
        }
        return Ok(());
    }
    if opts.search_json {
        let q = opts
            .query
            .as_deref()
            .context("--search-json needs a search query")?;
        let net = net::Network::new(&opts.base_url, &opts.user_agent)?;
        let data = catalog::search_results(&net.get_search(q)?, net.root())?;
        println!("{}", serde_json::to_string_pretty(&data)?);
        return Ok(());
    }
    if opts.no_menu
        && opts.query.is_none()
        && !opts.watchlist
        && !opts.catalog
        && opts.genre.is_none()
    {
        bail!("--no-menu requires a title, --watchlist, --catalog, or --genre");
    }
    App::new(opts)?.start()
}
