mod catalog;
mod model;
mod net;
mod options;
mod player;
mod resolve;
mod state;

use std::{
    collections::HashMap,
    io::{self, Write},
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

struct App {
    opts: Options,
    net: net::Network,
    state: State,
    page_cache: HashMap<(String, u32), Vec<Episode>>,
}

impl App {
    fn new(opts: Options) -> Result<Self> {
        let net = net::Network::new(&opts.base_url, &opts.user_agent)?;
        Ok(Self {
            opts,
            net,
            state: state::load(),
            page_cache: HashMap::new(),
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
        anyhow::ensure!(!episodes.is_empty(), "No episodes found in season {season}");
        self.page_cache.insert(key, episodes.clone());
        Ok(episodes)
    }

    fn episode_hosts(&self, ep: &Episode) -> Result<Vec<Host>> {
        let url = Url::parse(&ep.url)?;
        let (html, _) = self.net.page(&url, None)?;
        let hosters = catalog::hosts(&html, &url);
        anyhow::ensure!(
            !hosters.is_empty(),
            "No supported episode links were found in the site HTML"
        );
        Ok(hosters)
    }

    fn next(
        &mut self,
        series: &Series,
        current: &Episode,
        within_season: bool,
    ) -> Result<Option<Episode>> {
        let eps = self.episodes(series, current.season)?;
        if let Some(ep) = eps.into_iter().find(|e| e.number > current.number) {
            return Ok(Some(ep));
        }
        if within_season {
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
            let index = self.pick(
                if self.german() {
                    "Merkliste"
                } else {
                    "Watchlist"
                },
                &names,
            )?;
            let (slug, saved) = &entries[index];
            return Ok(Series {
                title: saved.title.clone(),
                slug: slug.clone(),
                url: saved.url.clone(),
            });
        }
        let query = if let Some(q) = &self.opts.query {
            q.clone()
        } else {
            anyhow::ensure!(
                !self.opts.no_menu && !self.opts.json,
                "Supply a query for noninteractive mode"
            );
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
            q.trim().to_owned()
        };
        anyhow::ensure!(!query.trim().is_empty(), "Search query cannot be empty");
        let results = catalog::search_results(&self.net.get_search(&query)?, self.net.root())?;
        anyhow::ensure!(!results.is_empty(), "No series matched '{query}'");
        let labels = results.iter().map(|s| s.title.clone()).collect::<Vec<_>>();
        let index = self.pick(
            if self.german() {
                "Anime"
            } else {
                "Choose series"
            },
            &labels,
        )?;
        Ok(results[index].clone())
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

    fn pick(&self, prompt: &str, options: &[String]) -> Result<usize> {
        self.pick_menu(prompt, options, 0)
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
            println!("Resolved via {} ({})", chosen.name, chosen.language);

            let mut play_args = self.opts.mpv_args.clone();
            play_args.append(&mut extra_mpv_flags);
            if !play_args
                .iter()
                .any(|a| a.starts_with("--save-position-on-quit"))
            {
                play_args.push("--save-position-on-quit".to_string());
            }

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

            const MENU_OPTIONS: [&str; 12] = [
                "Next episode",
                "Previous episode",
                "Replay",
                "Resume playback",
                "Select episode",
                "Select season",
                "Change language",
                "Change hoster",
                "Search another anime",
                "Add to watchlist",
                "Toggle auto-next",
                "Quit",
            ];

            let mut menu_default = 0;
            let action = loop {
                let choice = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("After playback")
                    .items(&MENU_OPTIONS)
                    .default(menu_default)
                    .max_length(15)
                    .interact_opt()?;

                match choice {
                    Some(0) => {
                        // Next episode
                        if let Some(next) = self.next(&series, &current, false)? {
                            break Some(next);
                        } else {
                            println!("No later episode found in this series.");
                            menu_default = 0;
                        }
                    }
                    Some(1) => {
                        // Previous episode
                        if let Some(prev) = self.previous(&series, &current)? {
                            break Some(prev);
                        } else {
                            println!("No earlier episode found in this series.");
                            menu_default = 1;
                        }
                    }
                    Some(2) => {
                        // Replay
                        extra_mpv_flags.push("--start=0".to_string());
                        println!("Replaying {} from beginning...", current.display());
                        break Some(current.clone());
                    }
                    Some(3) => {
                        // Resume playback
                        extra_mpv_flags.push("--save-position-on-quit".to_string());
                        println!("Resuming playback of {}...", current.display());
                        break Some(current.clone());
                    }
                    Some(4) => {
                        // Select episode (in current season)
                        match self.select_episode_in_season(&series, current.season, Some(&current))
                        {
                            Ok(ep) => break Some(ep),
                            Err(e) => {
                                eprintln!("Episode selection error: {e}");
                                menu_default = 4;
                            }
                        }
                    }
                    Some(5) => {
                        // Select season (and then episode)
                        match self.select_episode(&series, Some(&current)) {
                            Ok(ep) => break Some(ep),
                            Err(e) => {
                                eprintln!("Season/episode selection error: {e}");
                                menu_default = 5;
                            }
                        }
                    }
                    Some(6) => {
                        // Change language
                        match self.episode_hosts(&current) {
                            Ok(hosts) => {
                                let available = resolve::ordered_languages(
                                    &hosts,
                                    &self.opts.preferred_languages(),
                                );
                                if available.is_empty() {
                                    println!("No languages offered for this episode.");
                                    menu_default = 6;
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
                                        break Some(current.clone());
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Could not load episode languages: {e}");
                                menu_default = 6;
                            }
                        }
                    }
                    Some(7) => {
                        // Change hoster
                        match self.episode_hosts(&current) {
                            Ok(hosts) => {
                                let host_labels = hosts
                                    .iter()
                                    .map(|h| format!("{} ({})", h.name, h.language))
                                    .collect::<Vec<_>>();
                                if host_labels.is_empty() {
                                    println!("No hosters found for this episode.");
                                    menu_default = 7;
                                } else {
                                    if let Ok(idx) =
                                        self.pick_menu("Choose hoster", &host_labels, 0)
                                    {
                                        let h = &hosts[idx];
                                        forced_hoster = Some(h.name.clone());
                                        forced_language = Some(h.language.clone());
                                        remembered_language = Some(h.language.clone());
                                        println!("Hoster set to: {} ({})", h.name, h.language);
                                        break Some(current.clone());
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Could not load hosters: {e}");
                                menu_default = 7;
                            }
                        }
                    }
                    Some(8) => {
                        // Search another anime
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
                        if !q.is_empty() {
                            match self.net.get_search(q) {
                                Ok(resp) => match catalog::search_results(&resp, self.net.root()) {
                                    Ok(results) if !results.is_empty() => {
                                        let labels = results
                                            .iter()
                                            .map(|s| s.title.clone())
                                            .collect::<Vec<_>>();
                                        if let Ok(idx) = self.pick_menu("Choose series", &labels, 0)
                                        {
                                            series = results[idx].clone();
                                            match self.select_episode(&series, None) {
                                                Ok(ep) => {
                                                    remembered_language = None;
                                                    forced_hoster = None;
                                                    forced_language = None;
                                                    break Some(ep);
                                                }
                                                Err(e) => {
                                                    eprintln!("Could not select episode: {e}");
                                                    menu_default = 8;
                                                }
                                            }
                                        }
                                    }
                                    Ok(_) => {
                                        println!("No anime series found matching '{q}'.");
                                        menu_default = 8;
                                    }
                                    Err(e) => {
                                        eprintln!("Search parse error: {e}");
                                        menu_default = 8;
                                    }
                                },
                                Err(e) => {
                                    eprintln!("Search request failed: {e}");
                                    menu_default = 8;
                                }
                            }
                        }
                    }
                    Some(9) => {
                        // Add to watchlist
                        self.state.record(&series, &current);
                        if let Err(e) = state::save(&self.state) {
                            eprintln!("Warning: could not save watchlist: {e}");
                        } else {
                            println!(
                                "✓ Saved \"{}\" ({}) to watchlist",
                                series.title,
                                current.display()
                            );
                        }
                        menu_default = 9;
                    }
                    Some(10) => {
                        // Toggle auto-next
                        self.opts.auto = !self.opts.auto;
                        println!(
                            "Auto-next is now {}",
                            if self.opts.auto {
                                "ON (Episodes will autoplay consecutively)"
                            } else {
                                "OFF"
                            }
                        );
                        menu_default = 10;
                    }
                    _ => {
                        // Quit (Some(11) or None/Esc)
                        break None;
                    }
                }
            };

            match action {
                Some(next_ep) => {
                    current = next_ep;
                }
                None => {
                    break;
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
    if opts.no_menu && opts.query.is_none() && !opts.watchlist {
        bail!("--no-menu requires a title or --watchlist");
    }
    App::new(opts)?.start()
}
