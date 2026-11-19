//! AniWorld HTML and JSON interpretation. Pure functions make site changes testable.
use std::{
    collections::{BTreeMap, HashSet},
    sync::LazyLock,
};

use anyhow::{Context, Result};
use scraper::{Html, Selector};
use url::Url;

use crate::model::{Episode, Genre, Host, Series};

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
static SEL_CATALOG_ITEMS: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".seriesListContainer a[href*='/anime/stream/']").expect("valid selector")
});
static SEL_CATALOG_TITLE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("h3").expect("valid selector"));
static SEL_GENRE_BLOCKS: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.genre").expect("valid selector"));
static SEL_GENRE_TITLE: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".seriesGenreList h3, .genreTitle h3").expect("valid selector")
});
static SEL_GENRE_ANIME_LINKS: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("ul li a[href*='/anime/stream/']").expect("valid selector"));

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

pub fn parse_catalog_page(html: &str, base: &Url) -> Vec<Series> {
    let doc = Html::parse_document(html);
    let mut animes = Vec::new();
    let mut seen = HashSet::new();

    for link in doc.select(&SEL_CATALOG_ITEMS) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        let Some(url) = resolved_url(base, href) else {
            continue;
        };
        let Some(relative) = url.path().strip_prefix("/anime/stream/") else {
            continue;
        };
        let Some(slug) = relative.split('/').next() else {
            continue;
        };
        if slug.is_empty() || !seen.insert(slug.to_string()) {
            continue;
        }

        let title = link
            .select(&SEL_CATALOG_TITLE)
            .next()
            .map(|h3| stream_clean_text(h3.text()))
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| {
                link.value()
                    .attr("title")
                    .map(|t| {
                        t.split(" stream online")
                            .next()
                            .unwrap_or(t)
                            .trim()
                            .to_string()
                    })
                    .unwrap_or_else(|| slug.to_string())
            });

        animes.push(Series {
            title,
            slug: slug.to_string(),
            url: url.to_string(),
        });
    }

    animes
}

pub fn parse_genres(html: &str, base: &Url) -> Vec<Genre> {
    let doc = Html::parse_document(html);
    let mut genres = Vec::new();

    for block in doc.select(&SEL_GENRE_BLOCKS) {
        let Some(title_elem) = block.select(&SEL_GENRE_TITLE).next() else {
            continue;
        };
        let genre_name = stream_clean_text(title_elem.text());
        if genre_name.is_empty() {
            continue;
        }

        let mut animes = Vec::new();
        let mut seen = HashSet::new();

        for a in block.select(&SEL_GENRE_ANIME_LINKS) {
            let Some(href) = a.value().attr("href") else {
                continue;
            };
            let Some(url) = resolved_url(base, href) else {
                continue;
            };
            let Some(relative) = url.path().strip_prefix("/anime/stream/") else {
                continue;
            };
            let Some(slug) = relative.split('/').next() else {
                continue;
            };
            if slug.is_empty() || !seen.insert(slug.to_string()) {
                continue;
            }

            let title = stream_clean_text(a.text());
            let title = if title.is_empty() {
                a.value()
                    .attr("title")
                    .map(|t| t.trim_end_matches(" Stream anschauen").trim().to_string())
                    .unwrap_or_else(|| slug.to_string())
            } else {
                title
            };

            animes.push(Series {
                title,
                slug: slug.to_string(),
                url: url.to_string(),
            });
        }

        if !animes.is_empty() {
            genres.push(Genre {
                name: genre_name,
                animes,
            });
        }
    }

    genres
}

#[cfg(test)]
mod tests {
    use super::*;
    fn base() -> Url {
        Url::parse("https://example.test").unwrap()
    }

    #[test]
    fn parse_catalog_and_genres() {
        let catalog_html = r#"<div class="seriesListContainer row">
            <div><a href="/anime/stream/frieren" title="Frieren stream online"><h3>Frieren</h3><small>Fantasy</small></a></div>
            <div><a href="/anime/stream/fate" title="Fate stream online"><h3>Fate</h3><small>Action</small></a></div>
        </div>"#;
        let animes = parse_catalog_page(catalog_html, &base());
        assert_eq!(animes.len(), 2);
        assert_eq!(animes[0].title, "Frieren");
        assert_eq!(animes[1].slug, "fate");

        let genres_html = r#"<div class="genre">
            <div class="seriesGenreList"><h3>Fantasy</h3></div>
            <ul>
                <li><a href="/anime/stream/frieren">Frieren</a></li>
                <li><a href="/anime/stream/re-zero">Re:Zero</a></li>
            </ul>
        </div>
        <div class="genre">
            <div class="seriesGenreList"><h3>Action</h3></div>
            <ul>
                <li><a href="/anime/stream/bleach">Bleach</a></li>
            </ul>
        </div>"#;
        let genres = parse_genres(genres_html, &base());
        assert_eq!(genres.len(), 2);
        assert_eq!(genres[0].name, "Fantasy");
        assert_eq!(genres[0].animes.len(), 2);
        assert_eq!(genres[1].name, "Action");
        assert_eq!(genres[1].animes[0].title, "Bleach");
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
