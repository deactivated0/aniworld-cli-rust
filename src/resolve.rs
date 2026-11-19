//! Provider extraction and stream selection, implemented independently in Rust.
//! A provider changing its embedding format should fail closed and allow fallback.
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;
use std::sync::LazyLock;
use url::Url;

use crate::{
    model::{Host, Stream},
    net::Network,
};

static VIDMOLY_SOURCES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)\bsources\s*:\s*\[\s*\{\s*file\s*:\s*["']([^"']+)["']"#).expect("valid regex")
});
static JS_BOUNCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"window\.location\.href\s*=\s*["']([^"']+)["']"#).expect("valid regex")
});

fn rot13(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'a'..='z' => char::from_u32(((c as u32 - 97 + 13) % 26) + 97).unwrap_or(c),
            'A'..='Z' => char::from_u32(((c as u32 - 65 + 13) % 26) + 65).unwrap_or(c),
            _ => c,
        })
        .collect()
}

fn decode_voe(blob: &str) -> Option<String> {
    let envelope: Value = serde_json::from_str(blob).ok()?;
    let encoded = match envelope {
        Value::Array(items) => items.first()?.as_str()?.to_owned(),
        Value::String(s) => s,
        _ => return None,
    };
    let mut transformed = rot13(&encoded);
    for junk in ["@$", "^^", "~@", "%?", "*~", "!!", "#&"] {
        transformed = transformed.replace(junk, "");
    }
    let shifted = STANDARD.decode(transformed).ok()?;
    // Host markup encodes base64 ASCII after a reversible character shift.
    let reversed = shifted
        .iter()
        .map(|b| b.checked_sub(3))
        .collect::<Option<Vec<_>>>()?;
    let reversed = String::from_utf8(reversed).ok()?;
    let inner = reversed.chars().rev().collect::<String>();
    let decoded = STANDARD.decode(inner).ok()?;
    let payload: Value = serde_json::from_slice(&decoded).ok()?;
    payload
        .get("source")
        .or_else(|| payload.get("direct_access_url"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

pub fn voe_url(markup: &str) -> Option<String> {
    // Parse actual script nodes rather than trying to tokenize HTML with a regex.
    // This only reads publicly delivered page data and does not run JavaScript.
    let selector = Selector::parse("script[type='application/json']").ok()?;
    let document = Html::parse_document(markup);
    document.select(&selector).find_map(|node| {
        let payload = node.text().collect::<String>();
        decode_voe(payload.trim())
    })
}

pub fn vidmoly_url(markup: &str) -> Option<String> {
    VIDMOLY_SOURCES
        .captures(markup)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_owned()))
}

fn bounce_url(markup: &str) -> Option<String> {
    JS_BOUNCE
        .captures(markup)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_owned()))
}

fn origin(url: &Url) -> String {
    format!("{}/", url.origin().ascii_serialization())
}

fn extract(host: &Host, network: &Network, ua: &str) -> Result<Option<Stream>> {
    let url = Url::parse(&host.url)?;
    let (mut body, mut location) = network.page(&url, Some(network.root().as_str()))?;
    let name = host.name.to_ascii_lowercase();
    if name == "voe" {
        for _ in 0..4 {
            let Some(next) = bounce_url(&body) else { break };
            let address = location.join(&next)?;
            let (page, final_url) = network.page(&address, Some(&origin(&location)))?;
            body = page;
            location = final_url;
        }
        return Ok(voe_url(&body).map(|url| Stream {
            url,
            referer: Some(origin(&location)),
            user_agent: ua.to_owned(),
        }));
    }
    if name == "vidmoly" {
        return Ok(vidmoly_url(&body).map(|url| Stream {
            url,
            referer: Some(origin(&location)),
            user_agent: ua.to_owned(),
        }));
    }
    Ok(None)
}

pub fn known_host(host: &Host) -> bool {
    matches!(host.name.to_ascii_lowercase().as_str(), "voe" | "vidmoly")
}

pub fn ordered_languages(hosts: &[Host], priority: &[String]) -> Vec<String> {
    let mut langs = Vec::<String>::new();
    for lang in priority.iter().chain(hosts.iter().map(|h| &h.language)) {
        if hosts.iter().any(|h| &h.language == lang) && !langs.contains(lang) {
            langs.push(lang.to_owned());
        }
    }
    langs
}

pub fn choose_hosts(
    hosts: &[Host],
    langs: &[String],
    priority: &[String],
    selected: Option<&str>,
) -> Vec<Host> {
    let mut ordered: Vec<_> = hosts
        .iter()
        .filter(|h| known_host(h))
        .filter(|h| selected.is_none_or(|lang| lang == h.language.as_str()))
        .filter(|h| selected.is_some() || langs.contains(&h.language))
        .cloned()
        .collect();
    ordered.sort_by_key(|host| {
        let l = langs
            .iter()
            .position(|s| s == &host.language)
            .unwrap_or(usize::MAX);
        let n = priority
            .iter()
            .position(|s| s == &host.name.to_ascii_lowercase())
            .unwrap_or(usize::MAX);
        (l, n)
    });
    ordered
}

#[allow(clippy::too_many_arguments)]
pub fn resolve(
    hosts: &[Host],
    network: &Network,
    ua: &str,
    language: Option<&str>,
    language_priority: &[String],
    host_priority: &[String],
    verify: bool,
    quiet: bool,
) -> Result<(Stream, Host)> {
    let ordered = choose_hosts(hosts, language_priority, host_priority, language);
    let mut errors = Vec::new();
    for host in ordered {
        if !quiet {
            eprintln!("Trying {} ({})...", host.name, host.language);
        }
        match extract(&host, network, ua) {
            Ok(Some(stream)) => {
                let valid = Url::parse(&stream.url)
                    .is_ok_and(|url| matches!(url.scheme(), "http" | "https"));
                if !valid {
                    errors.push(format!("{} returned an invalid URL", host.name));
                    continue;
                }
                if !verify || network.stream_works(&stream) {
                    return Ok((stream, host));
                }
                errors.push(format!(
                    "{}: stream or media segment inaccessible",
                    host.name
                ));
            }
            Ok(None) => errors.push(format!("{}: no supported media found", host.name)),
            Err(e) => errors.push(format!("{}: {e}", host.name)),
        }
    }
    anyhow::bail!(
        "No working stream: {}",
        if errors.is_empty() {
            "no compatible hosts were found".to_owned()
        } else {
            errors.join("; ")
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_generated_test_vector() {
        let obj = serde_json::json!({"source":"https://media.example/test.m3u8"}).to_string();
        let inner = STANDARD.encode(obj.as_bytes());
        let backwards: Vec<u8> = inner.chars().rev().map(|c| c as u8 + 3).collect();
        let outer = rot13(&STANDARD.encode(backwards));
        let page = format!(
            r#"<script type="application/json">{}</script>"#,
            serde_json::json!([outer])
        );
        assert_eq!(voe_url(&page).unwrap(), "https://media.example/test.m3u8");
    }

    #[test]
    fn vidmoly_sample() {
        assert_eq!(
            vidmoly_url(r#"jwplayer.setup({sources: [{ file: 'https://foo.test/master.m3u8'}]})"#),
            Some("https://foo.test/master.m3u8".into())
        );
    }

    #[test]
    fn host_language_selection() {
        let hosts = vec![
            Host {
                name: "Vidmoly".into(),
                language: "ger-dub".into(),
                url: "https://x".into(),
            },
            Host {
                name: "VOE".into(),
                language: "ger-sub".into(),
                url: "https://y".into(),
            },
            Host {
                name: "VOE".into(),
                language: "ger-dub".into(),
                url: "https://z".into(),
            },
        ];
        let langs = vec!["ger-dub".into(), "ger-sub".into()];
        let priorities = vec!["voe".into(), "vidmoly".into()];
        let ordered = choose_hosts(&hosts, &langs, &priorities, None);
        assert_eq!(&ordered[0].url, "https://z");
    }
}
