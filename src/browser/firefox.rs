use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, ensure};

use crate::config::Config;

use super::Browser;

#[derive(Default)]
pub struct Firefox {
    profile_dir: Option<PathBuf>,
}

impl Browser for Firefox {
    fn setup(&mut self, cfg: &Config) -> Result<()> {
        let dir: PathBuf = tempfile::tempdir().context("creating temp profile")?.keep();

        fs::create_dir_all(dir.join("extensions")).context("creating extensions dir")?;

        self.profile_dir = Some(dir);

        self.install_ublock().context("installing ublock origin")?;
        self.write_user_js(cfg).context("writing user.js")?;

        if cfg.toolbar.should_show() {
            self.write_bookmarks_html(cfg)
                .context("writing bookmarks")?;
        }

        Ok(())
    }

    fn launch(&self, args: &[String]) -> Result<()> {
        let bin = self.find_binary()?;
        let profile_dir = self.profile_dir()?;

        let status = Command::new(bin)
            .arg("-no-remote")
            .arg("-profile")
            .arg(profile_dir)
            .args(args)
            .status()
            .context("launching firefox")?;

        ensure!(status.success(), "firefox exited with {status}");
        Ok(())
    }

    fn cleanup(&self) {
        if let Some(dir) = &self.profile_dir
            && dir.exists()
        {
            let _ = fs::remove_dir_all(dir);
            eprintln!("Temporary profile deleted");
        }
    }
}

impl Firefox {
    fn profile_dir(&self) -> Result<&Path> {
        self.profile_dir
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("profile directory not set up"))
    }

    fn find_binary(&self) -> Result<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let candidates = [
                std::env::var("PROGRAMFILES")
                    .map(|p| PathBuf::from(p).join("Mozilla Firefox").join("firefox.exe"))
                    .ok(),
                std::env::var("PROGRAMFILES(X86)")
                    .map(|p| PathBuf::from(p).join("Mozilla Firefox").join("firefox.exe"))
                    .ok(),
            ];
            for candidate in candidates.into_iter().flatten() {
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            let app = PathBuf::from("/Applications/Firefox.app/Contents/MacOS/firefox");
            if app.is_file() {
                return Ok(app);
            }
        }

        which::which("firefox").context("firefox not found in PATH")
    }

    fn install_ublock(&self) -> Result<()> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("ephemeral-browser");

        let xpi_path = cache_dir.join("ublock-origin.xpi");
        let url = "https://addons.mozilla.org/firefox/downloads/latest/ublock-origin/latest.xpi";

        let needs_download = match fs::metadata(&xpi_path) {
            Ok(meta) => {
                let age = meta
                    .modified()?
                    .elapsed()
                    .unwrap_or(Duration::from_secs(u64::MAX));
                age > Duration::from_secs(7 * 24 * 3600)
            }
            Err(_) => true,
        };

        if needs_download {
            eprintln!("Downloading uBlock Origin...");
            fs::create_dir_all(&cache_dir)?;
            download_file(url, &xpi_path)?;
        }

        let dst = self
            .profile_dir()?
            .join("extensions")
            .join("uBlock0@raymondhill.net.xpi");
        fs::copy(&xpi_path, &dst)?;

        Ok(())
    }

    fn write_user_js(&self, cfg: &Config) -> Result<()> {
        let search_engine = &cfg.search_engine;
        let homepage = cfg.homepage_url();
        let startup_page: u8 = if homepage.is_empty() { 0 } else { 1 };
        let theme_id = cfg.theme.firefox_theme_id();
        let toolbar_visibility = if cfg.toolbar.should_show() {
            "always"
        } else {
            "never"
        };

        let prefs = format!(
            r#"// Set default search engine
user_pref("browser.search.defaultenginename", "{search_engine}");
user_pref("browser.search.selectedEngine", "{search_engine}");
user_pref("browser.newtabpage.activity-stream.improvesearch.handoffToAwesomebar", false);
user_pref("browser.urlbar.placeholderName", "{search_engine}");
user_pref("browser.urlbar.update2.engineAliasRefresh", true);

// Use theme
user_pref("extensions.activeThemeID", "{theme_id}");

// Auto-enable extensions without prompts
user_pref("extensions.autoDisableScopes", 0);
user_pref("extensions.enabledScopes", 15);

// Allow uBlock Origin in private windows
user_pref("extensions.webextensions.uBlock0@raymondhill.net.privateBrowsingAllowed", true);

// Disable privacy notice tab on first run
user_pref("datareporting.policy.firstRunURL", "");
user_pref("browser.startup.homepage_override.mstone", "ignore");

// Disable surveys
user_pref("app.shield.optoutstudies.enabled", false);
user_pref("browser.newtabpage.activity-stream.asrouter.userprefs.cfr.addons", false);
user_pref("browser.newtabpage.activity-stream.asrouter.userprefs.cfr.features", false);

// Disable new tab page clutter
user_pref("browser.newtabpage.activity-stream.feeds.section.topstories", false);
user_pref("browser.newtabpage.activity-stream.showSponsored", false);
user_pref("browser.newtabpage.activity-stream.showSponsoredTopSites", false);
user_pref("browser.newtabpage.activity-stream.feeds.topsites", false);
user_pref("browser.newtabpage.activity-stream.feeds.section.highlights", false);
user_pref("browser.newtabpage.activity-stream.feeds.section.recentActivity", false);
user_pref("browser.newtabpage.activity-stream.feeds.snippets", false);

// Disable telemetry and data collection
user_pref("datareporting.healthreport.uploadEnabled", false);
user_pref("datareporting.policy.dataSubmissionEnabled", false);
user_pref("toolkit.telemetry.enabled", false);
user_pref("toolkit.telemetry.unified", false);
user_pref("toolkit.telemetry.archive.enabled", false);

// Bookmarks toolbar
user_pref("browser.toolbars.bookmarks.visibility", "{toolbar_visibility}");

// Import bookmarks from HTML
user_pref("browser.places.importBookmarksHTML", true);

// Set homepage
user_pref("browser.startup.homepage", "{homepage}");
user_pref("browser.startup.page", {startup_page});
"#,
        );

        let path = self.profile_dir()?.join("user.js");
        fs::write(path, prefs)?;
        Ok(())
    }

    fn write_bookmarks_html(&self, cfg: &Config) -> Result<()> {
        let path = self.profile_dir()?.join("bookmarks.html");
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
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn download_file(url: &str, dst: &Path) -> Result<()> {
    let response = ureq::get(url).call().context("downloading file")?;
    let mut reader = response.into_body().into_reader();
    let mut file = fs::File::create(dst)?;
    std::io::copy(&mut reader, &mut file)?;
    Ok(())
}
