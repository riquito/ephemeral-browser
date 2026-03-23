use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, ensure};

use crate::config::Config;

use super::Browser;
use super::common;

#[derive(Default)]
pub struct Firefox {
    profile_dir: Option<PathBuf>,
    binary_override: Option<PathBuf>,
}

impl Browser for Firefox {
    fn setup(&mut self, cfg: &Config) -> Result<()> {
        let dir: PathBuf = tempfile::tempdir().context("creating temp profile")?.keep();
        eprintln!("Profile directory: {}", dir.display());

        fs::create_dir_all(dir.join("extensions")).context("creating extensions dir")?;

        self.profile_dir = Some(dir);
        self.binary_override = cfg.browser_path.clone();
        common::write_pid_file(self.profile_dir()?)?;

        self.install_ublock().context("installing ublock origin")?;
        self.write_user_js(cfg).context("writing user.js")?;
        self.write_user_chrome_css()
            .context("writing userChrome.css")?;

        if cfg.toolbar.should_show() {
            let path = self.profile_dir()?.join("bookmarks.html");
            common::write_bookmarks_html(&path, cfg).context("writing bookmarks")?;
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
        if let Some(path) = &self.binary_override {
            return Ok(path.clone());
        }

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
        let cache_dir = common::cache_dir();
        let xpi_path = cache_dir.join("ublock-origin.xpi");
        let url = "https://addons.mozilla.org/firefox/downloads/latest/ublock-origin/latest.xpi";

        if common::needs_download(&xpi_path)? {
            eprintln!("Downloading uBlock Origin...");
            fs::create_dir_all(&cache_dir)?;
            common::download_file(url, &xpi_path)?;
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

// Enable userChrome.css customization
user_pref("toolkit.legacyUserProfileCustomizations.stylesheets", true);

// Set homepage
user_pref("browser.startup.homepage", "{homepage}");
user_pref("browser.startup.page", {startup_page});
"#,
        );

        let path = self.profile_dir()?.join("user.js");
        fs::write(path, prefs)?;
        Ok(())
    }

    fn write_user_chrome_css(&self) -> Result<()> {
        let chrome_dir = self.profile_dir()?.join("chrome");
        fs::create_dir_all(&chrome_dir)?;

        let css = r#"/* Dark purple toolbar to distinguish ephemeral-browser */
:root {
    --ephemeral-purple: #2d1b4e;
    --ephemeral-purple-light: #3d2b5e;
}

/* Navigation bar */
#nav-bar {
    background-color: var(--ephemeral-purple) !important;
}

/* Tab bar background */
#titlebar,
#TabsToolbar {
    background-color: var(--ephemeral-purple) !important;
}

/* Active tab */
.tabbrowser-tab[selected] .tab-background {
    background-color: var(--ephemeral-purple-light) !important;
}

/* Bookmarks toolbar */
#PersonalToolbar {
    background-color: var(--ephemeral-purple) !important;
}
"#;

        let path = chrome_dir.join("userChrome.css");
        fs::write(path, css)?;
        Ok(())
    }
}
