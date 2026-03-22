use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::config::Config;

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn download_file(url: &str, dst: &Path) -> Result<()> {
    let response = ureq::get(url).call().context("downloading file")?;
    let mut reader = response.into_body().into_reader();
    let mut file = fs::File::create(dst)?;
    std::io::copy(&mut reader, &mut file)?;
    Ok(())
}

/// Check if a cached file needs to be re-downloaded (missing or older than 7 days).
pub fn needs_download(path: &Path) -> Result<bool> {
    match fs::metadata(path) {
        Ok(meta) => {
            let age = meta
                .modified()?
                .elapsed()
                .unwrap_or(Duration::from_secs(u64::MAX));
            Ok(age > Duration::from_secs(7 * 24 * 3600))
        }
        Err(_) => Ok(true),
    }
}

pub fn cache_dir() -> std::path::PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("ephemeral-browser")
}

pub fn write_bookmarks_html(path: &Path, cfg: &Config) -> Result<()> {
    let mut f = fs::File::create(path)?;

    write!(
        f,
        r#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<META HTTP-EQUIV="Content-Type" CONTENT="text/html; charset=UTF-8">
<TITLE>Bookmarks</TITLE>
<H1>Bookmarks Menu</H1>
<DL><p>
    <DT><H3 ADD_DATE="1" LAST_MODIFIED="1" PERSONAL_TOOLBAR_FOLDER="true">Bookmarks Toolbar</H3>
    <DL><p>
"#
    )?;

    for tab in &cfg.toolbar.tabs {
        writeln!(
            f,
            "        <DT><A HREF=\"{}\">{}</A>",
            html_escape(&tab.url),
            html_escape(&tab.label),
        )?;
    }

    write!(
        f,
        r#"    </DL><p>
</DL><p>
"#
    )?;

    Ok(())
}
