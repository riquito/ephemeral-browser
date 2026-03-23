use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::config::Config;

pub fn http_agent() -> ureq::Agent {
    ureq::config::Config::builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::NativeTls)
                .build(),
        )
        .build()
        .new_agent()
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn download_file(url: &str, dst: &Path) -> Result<()> {
    let response = http_agent()
        .get(url)
        .call()
        .context("downloading file")?;
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

pub const PID_FILE_NAME: &str = "ephemeral-browser.pid";

/// Write our PID to a file in the profile directory.
pub fn write_pid_file(profile_dir: &Path) -> Result<()> {
    let pid = std::process::id();
    fs::write(profile_dir.join(PID_FILE_NAME), pid.to_string())?;
    Ok(())
}

/// Read a PID from a pid file, returning None if the file doesn't exist or is invalid.
pub fn read_pid_file(profile_dir: &Path) -> Option<u32> {
    fs::read_to_string(profile_dir.join(PID_FILE_NAME))
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// Check if a process with the given PID is still running.
pub fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
    }
    #[cfg(not(unix))]
    {
        // On non-Unix, conservatively assume the process is alive
        // to avoid deleting an active profile.
        let _ = pid;
        true
    }
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
