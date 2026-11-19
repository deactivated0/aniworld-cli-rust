//! Launch the user's mpv directly without a shell or an embedded runtime.
use crate::model::{Episode, Series, Stream};
use anyhow::{Context, Result, bail};
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(unix)]
fn runnable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.is_file()
        && path
            .metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn runnable(path: &Path) -> bool {
    path.is_file()
}

pub fn find_player(explicit: Option<&str>) -> Option<PathBuf> {
    if let Some(name) = explicit {
        let path = PathBuf::from(name);
        if runnable(&path) {
            return Some(path);
        }
        return from_path(name);
    }
    #[cfg(target_os = "windows")]
    if let Some(bundled) = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("mpv").join("mpv.exe")))
        .filter(|p| runnable(p))
    {
        return Some(bundled);
    }
    from_path("mpv")
}

fn from_path(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    if path.components().count() > 1 {
        return None;
    }
    let paths = env::var_os("PATH")?;
    for folder in env::split_paths(&paths) {
        let found = folder.join(name);
        if runnable(&found) {
            return Some(found);
        }
        #[cfg(target_os = "windows")]
        {
            let found = folder.join(format!("{name}.exe"));
            if runnable(&found) {
                return Some(found);
            }
        }
    }
    None
}

pub fn mpv_arguments(
    stream: &Stream,
    series: &Series,
    ep: &Episode,
    extra: &[String],
) -> Vec<String> {
    let mut args = vec![
        "--no-terminal".to_owned(),
        format!("--user-agent={}", stream.user_agent),
    ];
    if let Some(referrer) = &stream.referer {
        args.push(format!("--referrer={referrer}"));
    }
    let title = if ep.season == 0 {
        format!("{} - Movie {:02} - {}", series.title, ep.number, ep.title)
    } else {
        format!(
            "{} - S{:02}E{:02} - {}",
            series.title, ep.season, ep.number, ep.title
        )
    };
    args.push(format!("--force-media-title={title}"));
    args.extend_from_slice(extra);
    args.push("--".to_string());
    args.push(stream.url.clone());
    args
}

pub fn play(
    player: Option<&str>,
    stream: &Stream,
    series: &Series,
    ep: &Episode,
    extra: &[String],
) -> Result<()> {
    let program = find_player(player)
        .context("mpv not found: install mpv or specify --player /path/to/mpv")?;
    let code = Command::new(&program)
        .args(mpv_arguments(stream, series, ep, extra))
        .status()
        .with_context(|| format!("Cannot launch {}", program.display()))?;
    if !code.success() {
        bail!("mpv exited unsuccessfully: {code}")
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn argument_vector_never_uses_shell() {
        let s = Stream {
            url: "https://media.test/a.m3u8?x=1&y=2".into(),
            referer: Some("https://host.test/".into()),
            user_agent: "TestUA".into(),
        };
        let series = Series {
            title: "Demo".into(),
            slug: "demo".into(),
            url: "https://host.test/demo".into(),
        };
        let ep = Episode {
            season: 2,
            number: 3,
            title: "Hi".into(),
            url: "https://host.test/ep".into(),
        };
        let args = mpv_arguments(&s, &series, &ep, &[]);
        assert!(args.contains(&"--force-media-title=Demo - S02E03 - Hi".into()));
        assert_eq!(args.last().unwrap(), &s.url);
        assert_eq!(args[args.len() - 2], "--");
    }
}
