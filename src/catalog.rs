//! AniWorld HTML and JSON interpretation. Pure functions make site changes testable.
use std::{
    collections::{BTreeMap, HashSet},
    sync::LazyLock,
};

use anyhow::{Context, Result};
use scraper::{Html, Selector};
use url::Url;

use crate::model::{Episode, Host, Series};

static SEL_SERIES_TITLE: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("h1[itemprop='name'], .series-title h1").expect("valid selector")
});
static SEL_SEASONS_LINKS: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("#stream > ul:first-of-type a[href]").expect("valid selector")
});
static SEL_EPISODES_ROWS: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("table.seasonEpisodesList tbody tr[data-episode-id]").expect("valid selector")
});
static SEL_EPISODES_LINKS: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("td a[href]").expect("valid selector"));
static SEL_EPISODES_TITLES: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("td.seasonEpisodeTitle").expect("valid selector"));
static SEL_EPISODES_NAV: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("#stream a[data-episode-id][href]").expect("valid selector"));
static SEL_HOSTS_ITEMS: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("li[data-link-target][data-lang-key]").expect("valid selector")
});
static SEL_HOSTS_LABELS: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("h4").expect("valid selector"));

fn unescape_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp_idx) = rest.find('&') {
        out.push_str(&rest[..amp_idx]);
        let after_amp = &rest[amp_idx + 1..];
        if let Some(semi_idx) = after_amp.find(';') {
            if semi_idx <= 8 {
                let entity = &after_amp[..semi_idx];
                match entity {
                    "amp" => out.push('&'),
                    "apos" | "#039" | "#39" => out.push('\''),
                    "quot" | "#034" | "#34" => out.push('"'),
                    "lt" => out.push('<'),
                    "gt" => out.push('>'),
                    "ndash" | "#8211" => out.push('–'),
                    "mdash" | "#8212" => out.push('—'),
                    _ => {
                        out.push('&');
                        out.push_str(&after_amp[..=semi_idx]);
                    }
                }
                rest = &after_amp[semi_idx + 1..];
                continue;
            }
        }
        out.push('&');
        rest = after_amp;
    }
    out.push_str(rest);
    out
}

fn stream_clean_text<'a>(iter: impl IntoIterator<Item = &'a str>) -> String {
    let mut out = String::new();
    let mut first = true;
    for chunk in iter {
        for word in chunk.split_whitespace() {
            if !first {
                out.push(' ');
            }
            out.push_str(word);
            first = false;
        }
    }
    if out.contains('&') {
        unescape_entities(&out)
    } else {
        out
    }
}

pub fn plain_text(html: &str) -> String {
    let fragment = Html::parse_fragment(html);
    stream_clean_text(fragment.root_element().text())
}

fn resolved_url(root: &Url, href: &str) -> Option<Url> {
    root.join(href)
        .ok()
        .filter(|u| matches!(u.scheme(), "http" | "https"))
}

pub fn search_results(body: &str, root: &Url) -> Result<Vec<Series>> {
    let data: serde_json::Value =
        serde_json::from_str(body).context("Search endpoint returned non-JSON data")?;
    let items = data
        .as_array()
        .or_else(|| data.get("results").and_then(|v| v.as_array()))
        .context("Search response is not an array")?;
    let mut seen = HashSet::with_capacity(items.len());
    let mut result = Vec::with_capacity(items.len());
    for item in items {
        let (Some(title), Some(href)) = (
            item.get("title").and_then(|v| v.as_str()),
            item.get("link").and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        let Some(url) = resolved_url(root, href) else {
            continue;
        };
        let Some(relative) = url.path().strip_prefix("/anime/stream/") else {
            continue;
        };
        let Some(slug) = relative.split('/').next() else {
            continue;
        };
        if slug.is_empty() || relative.contains('/') || !seen.insert(slug.to_owned()) {
            continue;
        }
        let title = plain_text(title);
        if title.is_empty() {
            continue;
        }
        result.push(Series {
            title,
            slug: slug.to_owned(),
            url: url.to_string(),
        });
    }
    Ok(result)
}

pub fn series_title(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    doc.select(&SEL_SERIES_TITLE)
        .next()
        .map(|n| stream_clean_text(n.text()))
        .filter(|s| !s.is_empty())
}

pub fn seasons(html: &str) -> Vec<u32> {
    let doc = Html::parse_document(html);
    let mut order = BTreeMap::new();
    for link in doc.select(&SEL_SEASONS_LINKS) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        if href.trim_end_matches('/').ends_with("/filme") {
            order.insert(0, ());
        } else if let Some(n) = href
            .rsplit_once("/staffel-")
            .and_then(|(_, tail)| tail.parse::<u32>().ok())
            .filter(|&n| n > 0)
        {
            order.insert(n, ());
        }
    }
    order.into_keys().collect()
}

fn id_from_path(url: &str) -> Option<(u32, u32)> {
    let path = url.trim_end_matches('/');
    let (_, suffix) = path.rsplit_once('/')?;
    let (season, prefix) = if path.contains("/filme/") {
        (0, "film-")
    } else {
        (1, "episode-")
    };
    let number = suffix.strip_prefix(prefix)?.parse::<u32>().ok()?;
    if number == 0 {
        return None;
    }
    if season == 0 {
        return Some((0, number));
    }
    let (season_raw, _) = path.split_once("/staffel-")?.1.split_once('/')?;
    let s = season_raw.parse::<u32>().ok()?;
    Some((s, number))
}

pub fn episodes(html: &str, base: &Url, selected_season: u32) -> Vec<Episode> {
    let doc = Html::parse_document(html);
    let mut by_number = BTreeMap::<u32, Episode>::new();
    for row in doc.select(&SEL_EPISODES_ROWS) {
        let Some(link) = row.select(&SEL_EPISODES_LINKS).next() else {
            continue;
        };
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        let Some((season, number)) = id_from_path(href) else {
            continue;
        };
        if season != selected_season {
            continue;
        }
        let Some(url) = resolved_url(base, href) else {
            continue;
        };
        let title = row
            .select(&SEL_EPISODES_TITLES)
            .next()
            .map(|e| stream_clean_text(e.text()))
            .unwrap_or_default();
        let title = if title.is_empty() {
            format!("Episode {number}")
        } else {
            title
        };
        by_number.insert(
            number,
            Episode {
                season,
                number,
                title,
                url: url.to_string(),
            },
        );
    }
    if by_number.is_empty() {
        for link in doc.select(&SEL_EPISODES_NAV) {
            let Some(href) = link.value().attr("href") else {
                continue;
            };
            let Some((season, number)) = id_from_path(href) else {
                continue;
            };
            if season != selected_season {
                continue;
            }
            let Some(url) = resolved_url(base, href) else {
                continue;
            };
            by_number.insert(
                number,
                Episode {
                    season,
                    number,
                    title: format!("Episode {number}"),
                    url: url.to_string(),
                },
            );
        }
    }
    by_number.into_values().collect()
}

fn language_key(key: &str) -> String {
    match key {
        "1" => "ger-dub".to_owned(),
        "2" => "eng-sub".to_owned(),
        "3" => "ger-sub".to_owned(),
        _ => format!("unknown-{key}"),
    }
}

pub fn hosts(html: &str, base: &Url) -> Vec<Host> {
    let doc = Html::parse_document(html);
    let mut found = Vec::new();
    let mut seen = HashSet::new();
    for li in doc.select(&SEL_HOSTS_ITEMS) {
        let Some(href) = li.value().attr("data-link-target") else {
            continue;
        };
        let Some(lang) = li.value().attr("data-lang-key") else {
            continue;
        };
        let Some(url) = resolved_url(base, href) else {
            continue;
        };
        let name = li
            .select(&SEL_HOSTS_LABELS)
            .next()
            .map(|node| stream_clean_text(node.text()))
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        if seen.insert((name.clone(), lang.to_owned(), url.to_string())) {
            found.push(Host {
                name,
                language: language_key(lang),
                url: url.to_string(),
            });
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    fn base() -> Url {
        Url::parse("https://example.test").unwrap()
    }

    #[test]
    fn only_series_in_search() {
        let body = r#"[{"title":"<em>Naruto</em> &amp; Friends","link":"/anime/stream/naruto"},
        {"title":"Episode 1","link":"/anime/stream/naruto/staffel-1/episode-1"},
        {"title":"Naruto duplicate","link":"/anime/stream/naruto"}]"#;
        let got = search_results(body, &base()).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].title, "Naruto & Friends");
    }

    #[test]
    fn parse_series_seasons_and_episodes() {
        let html = r#"<div id="stream"><ul>
            <li><a href="/anime/stream/demo/filme">Filme</a></li>
            <li><a href="/anime/stream/demo/staffel-2">2</a></li>
            <li><a href="/anime/stream/demo/staffel-1">1</a></li>
        </ul></div><table class="seasonEpisodesList"><tbody><tr data-episode-id="12">
        <td><a href="/anime/stream/demo/staffel-1/episode-2">Folge 2</a></td>
        <td class="seasonEpisodeTitle"><a>First &amp; Last</a></td></tr></tbody></table>"#;
        assert_eq!(seasons(html), vec![0, 1, 2]);
        let eps = episodes(html, &base(), 1);
        assert_eq!(eps.len(), 1);
        assert_eq!(eps[0].number, 2);
        assert_eq!(eps[0].title, "First & Last");
    }

    #[test]
    fn parse_available_languages() {
        let html = r#"<ul><li data-link-target="/redirect/4" data-lang-key="3"><a><h4>VOE</h4></a></li>
        <li data-link-target="/redirect/9" data-lang-key="1"><a><h4>Vidmoly</h4></a></li></ul>"#;
        let got = hosts(html, &base());
        assert_eq!(got[0].language, "ger-sub");
        assert_eq!(got[1].name, "Vidmoly");
    }
}
