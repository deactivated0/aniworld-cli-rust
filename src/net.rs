//! Shared connection pool, bounded retries and CDN validation.
use std::{io::Read, thread, time::Duration};

use anyhow::{Context, Result, bail};
use reqwest::{
    StatusCode,
    blocking::Client,
    header::{ACCEPT_LANGUAGE, RANGE, REFERER},
};
use url::Url;

pub struct Network {
    pub client: Client,
    base: Url,
}

impl Network {
    pub fn new(base: &str, ua: &str) -> Result<Self> {
        let root = Url::parse(base).context("Invalid ANIWorld base URL")?;
        anyhow::ensure!(
            matches!(root.scheme(), "http" | "https"),
            "Only HTTP(S) URLs are accepted"
        );
        let client = Client::builder()
            .user_agent(ua)
            .default_headers({
                let mut h = reqwest::header::HeaderMap::new();
                h.insert(ACCEPT_LANGUAGE, "de-DE,de;q=0.9,en;q=0.6".parse()?);
                h
            })
            .timeout(Duration::from_secs(18))
            .connect_timeout(Duration::from_secs(8))
            .redirect(reqwest::redirect::Policy::limited(8))
            .pool_idle_timeout(Duration::from_secs(45))
            .build()?;
        Ok(Self { client, base: root })
    }

    pub fn root(&self) -> &Url {
        &self.base
    }

    pub fn page(&self, url: &Url, referer: Option<&str>) -> Result<(String, Url)> {
        for attempt in 0..3u64 {
            let mut request = self.client.get(url.clone());
            if let Some(v) = referer {
                request = request.header(REFERER, v);
            }
            match request.send() {
                Ok(response) => {
                    let status = response.status();
                    let final_url = response.url().clone();
                    if matches!(status.as_u16(), 429 | 500 | 502 | 503 | 504) && attempt < 2 {
                        thread::sleep(Duration::from_millis(350 * (attempt + 1)));
                        continue;
                    }
                    let data = response.text()?;
                    let maybe_challenge = (status == StatusCode::FORBIDDEN
                        || status == StatusCode::SERVICE_UNAVAILABLE)
                        && [
                            "just a moment",
                            "cf-chl",
                            "challenge-platform",
                            "cf-browser-verification",
                        ]
                        .iter()
                        .any(|tag| data.to_ascii_lowercase().contains(tag));
                    if maybe_challenge {
                        bail!(
                            "Cloudflare challenge detected at {final_url}; this program does not bypass CAPTCHA"
                        )
                    }
                    anyhow::ensure!(status.is_success(), "HTTP {status} from {final_url}");
                    return Ok((data, final_url));
                }
                Err(e) if (e.is_connect() || e.is_timeout()) && attempt < 2 => {
                    thread::sleep(Duration::from_millis(350 * (attempt + 1)));
                }
                Err(e) => return Err(e).context(format!("Cannot fetch {url}")),
            }
        }
        bail!("Request failed after retries: {url}")
    }

    pub fn get_search(&self, query: &str) -> Result<String> {
        let endpoint = self.base.join("/ajax/search")?;
        let response = self
            .client
            .post(endpoint.clone())
            .header(REFERER, self.base.as_str())
            .form(&[("keyword", query)])
            .send()
            .context("Site search request failed")?;
        let code = response.status();
        let text = response.text()?;
        if matches!(code.as_u16(), 403 | 503)
            && ["just a moment", "cf-chl", "challenge-platform"]
                .iter()
                .any(|tag| text.to_ascii_lowercase().contains(tag))
        {
            bail!(
                "AniWorld requested a Cloudflare challenge; automated CAPTCHA bypass is not supported"
            );
        }
        anyhow::ensure!(
            code.is_success(),
            "Search failed: HTTP {code} from {endpoint}"
        );
        Ok(text)
    }

    fn playlist(&self, stream_url: &Url, referer: Option<&str>) -> Result<String> {
        let mut request = self.client.get(stream_url.clone());
        if let Some(r) = referer {
            request = request.header(REFERER, r);
        }
        let mut res = request.send()?;
        anyhow::ensure!(res.status().is_success(), "Playlist HTTP {}", res.status());
        // Avoid accidentally loading unbounded bodies into memory.
        let mut out = Vec::new();
        res.by_ref().take(1_048_577).read_to_end(&mut out)?;
        anyhow::ensure!(
            out.len() <= 1_048_576,
            "Playlist response unexpectedly large"
        );
        String::from_utf8(out).context("Playlist is not UTF-8")
    }

    pub fn stream_works(&self, stream: &crate::model::Stream) -> bool {
        self.check_stream(stream).unwrap_or(false)
    }

    fn check_stream(&self, stream: &crate::model::Stream) -> Result<bool> {
        let mut current = Url::parse(&stream.url)?;
        if current.path().to_ascii_lowercase().contains(".m3u8") {
            for _ in 0..3 {
                let body = self.playlist(&current, stream.referer.as_deref())?;
                let Some(next) = first_media_entry(&body) else {
                    return Ok(true);
                };
                current = current.join(&next)?;
                if !current.path().to_ascii_lowercase().contains(".m3u8") {
                    break;
                }
            }
        }
        let mut request = self.client.get(current).header(RANGE, "bytes=0-1");
        if let Some(r) = &stream.referer {
            request = request.header(REFERER, r);
        }
        let response = request.send()?;
        Ok(matches!(response.status().as_u16(), 200 | 206))
    }
}

pub fn first_media_entry(playlist: &str) -> Option<String> {
    playlist
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn find_first_playlist_item() {
        assert_eq!(
            first_media_entry("#EXTM3U\r\n#comment\r\nvideo/480p.m3u8\r\n"),
            Some("video/480p.m3u8".into())
        );
        assert_eq!(first_media_entry("#EXTM3U\n"), None);
    }
}
