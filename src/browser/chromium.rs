use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, ensure};

use crate::config::{BrowserKind, Config, Theme};

use super::Browser;
use super::common;

pub struct Chromium {
    kind: BrowserKind,
    profile_dir: Option<PathBuf>,
    ublock_dir: Option<PathBuf>,
    theme_dir: Option<PathBuf>,
    binary_override: Option<PathBuf>,
}

impl Chromium {
    pub fn new(kind: BrowserKind) -> Self {
        Self {
            kind,
            profile_dir: None,
            ublock_dir: None,
            theme_dir: None,
            binary_override: None,
        }
    }
}

impl Browser for Chromium {
    fn setup(&mut self, cfg: &Config) -> Result<()> {
        let dir: PathBuf = tempfile::tempdir().context("creating temp profile")?.keep();
        eprintln!("Profile directory: {}", dir.display());

        let default_dir = dir.join("Default");
        fs::create_dir_all(&default_dir).context("creating Default profile dir")?;

        self.profile_dir = Some(dir);
        self.binary_override = cfg.browser_path.clone();
        common::write_pid_file(self.profile_dir()?)?;

        self.install_ublock().context("installing ublock origin")?;
        self.install_theme().context("installing theme")?;
        self.write_preferences(cfg).context("writing preferences")?;

        if cfg.toolbar.should_show() {
            let path = default_dir.join("Bookmarks");
            write_chromium_bookmarks(&path, cfg).context("writing bookmarks")?;
        }

        Ok(())
    }

    fn launch(&self, args: &[String]) -> Result<()> {
        let bin = self.find_binary()?;
        let profile_dir = self.profile_dir()?;

        let mut cmd = Command::new(bin);
        cmd.arg(format!("--user-data-dir={}", profile_dir.display()))
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-background-networking")
            .arg("--disable-component-update")
            .arg("--disable-sync")
            .arg("--disable-domain-reliability")
            .arg("--disable-breakpad")
            .arg("--disable-search-engine-choice-screen")
            .arg("--disable-field-trial-config")
            .arg("--metrics-recording-only")
            .arg("--disable-features=OptimizationHints");

        let mut extensions: Vec<&Path> = Vec::new();
        if let Some(d) = &self.ublock_dir {
            extensions.push(d);
        }
        if let Some(d) = &self.theme_dir {
            extensions.push(d);
        }
        if !extensions.is_empty() {
            let paths: Vec<_> = extensions.iter().map(|p| p.display().to_string()).collect();
            cmd.arg(format!("--load-extension={}", paths.join(",")));
        }

        cmd.args(args);

        let status = cmd.status().context("launching browser")?;

        ensure!(status.success(), "{} exited with {status}", self.kind);
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

impl Chromium {
    fn profile_dir(&self) -> Result<&Path> {
        self.profile_dir
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("profile directory not set up"))
    }

    fn find_binary(&self) -> Result<PathBuf> {
        if let Some(path) = &self.binary_override {
            return Ok(path.clone());
        }

        let names = match self.kind {
            BrowserKind::Chrome => {
                #[cfg(target_os = "windows")]
                {
                    let candidates = [
                        std::env::var("PROGRAMFILES")
                            .map(|p| {
                                PathBuf::from(p)
                                    .join("Google")
                                    .join("Chrome")
                                    .join("Application")
                                    .join("chrome.exe")
                            })
                            .ok(),
                        std::env::var("PROGRAMFILES(X86)")
                            .map(|p| {
                                PathBuf::from(p)
                                    .join("Google")
                                    .join("Chrome")
                                    .join("Application")
                                    .join("chrome.exe")
                            })
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
                    let app = PathBuf::from(
                        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                    );
                    if app.is_file() {
                        return Ok(app);
                    }
                }

                &["google-chrome-stable", "google-chrome"][..]
            }
            BrowserKind::Chromium => {
                #[cfg(target_os = "macos")]
                {
                    let app = PathBuf::from("/Applications/Chromium.app/Contents/MacOS/Chromium");
                    if app.is_file() {
                        return Ok(app);
                    }
                }

                &["chromium", "chromium-browser"][..]
            }
            BrowserKind::Firefox => unreachable!(),
        };

        for name in names {
            if let Ok(path) = which::which(name) {
                return Ok(path);
            }
        }

        anyhow::bail!("{} not found in PATH", self.kind)
    }

    fn install_ublock(&mut self) -> Result<()> {
        let cache_dir = common::cache_dir();
        let zip_path = cache_dir.join("ublock-origin-lite-chromium.zip");
        let unpack_dir = cache_dir.join("ublock-origin-lite-chromium");

        // Re-download if zip is stale
        if common::needs_download(&zip_path)? {
            eprintln!("Downloading uBlock Origin Lite for Chromium...");
            fs::create_dir_all(&cache_dir)?;

            let url = get_ublock_chromium_url().context("fetching uBlock Origin download URL")?;
            common::download_file(&url, &zip_path)?;

            // Remove old unpacked dir so it gets re-extracted
            if unpack_dir.exists() {
                fs::remove_dir_all(&unpack_dir)?;
            }
        }

        // Unpack if needed
        if !unpack_dir.exists() {
            eprintln!("Unpacking uBlock Origin...");
            unpack_zip(&zip_path, &unpack_dir)?;
        }

        // The zip contains a top-level directory (e.g. uBlock0.chromium/)
        // that holds the manifest — point --load-extension there.
        let extension_dir = find_extension_root(&unpack_dir)?;
        self.ublock_dir = Some(extension_dir);
        Ok(())
    }

    fn install_theme(&mut self) -> Result<()> {
        let theme_dir = self.profile_dir()?.join("ephemeral-theme");
        fs::create_dir_all(&theme_dir)?;

        // RGB color values used below:
        //   [45, 27, 78]    = #2d1b4e  dark purple (frame, title bar)
        //   [35, 17, 68]    = #231144  darker purple (inactive window frame)
        //   [61, 43, 94]    = #3d2b5e  lighter purple (toolbar, active tab)
        //   [255, 255, 255] = #ffffff  white (all text)
        let manifest = r#"{
    "manifest_version": 2,
    "name": "Ephemeral Browser Theme",
    "version": "1.0",
    "theme": {
        "colors": {
            "frame": [45, 27, 78],
            "frame_inactive": [35, 17, 68],
            "toolbar": [61, 43, 94],
            "tab_background_text": [255, 255, 255],
            "bookmark_text": [255, 255, 255],
            "tab_text": [255, 255, 255]
        }
    }
}"#;

        fs::write(theme_dir.join("manifest.json"), manifest)?;
        self.theme_dir = Some(theme_dir);
        Ok(())
    }

    fn write_preferences(&self, cfg: &Config) -> Result<()> {
        let homepage = cfg.homepage_url();
        let restore_on_startup = if homepage.is_empty() { 5 } else { 4 };

        let dark_mode = matches!(cfg.theme, Theme::Dark);

        let startup_urls = if homepage.is_empty() {
            String::new()
        } else {
            format!(r#""startup_urls": ["{homepage}"],"#)
        };

        // Pre-register the theme extension ID so Chromium treats it as already
        // active at startup, suppressing the "Installed theme" infobar.
        // For Chrome (where the extension doesn't load), we also set
        // user_color2/color_variant2 which Chrome uses for native theme colors.
        let theme_ext_id = self
            .theme_dir
            .as_ref()
            .and_then(|d| compute_extension_id(d));

        let extensions_theme_id = theme_ext_id.as_deref().unwrap_or("user_color_theme_id");

        // #9200ff as signed ARGB int: 0xFF9200FF = -7208705
        // color_scheme2: 2 = dark mode, color_variant2: 1 = neutral
        const PURPLE_ARGB: i32 = -7208705;

        // Chromium preferences are a JSON file in <profile>/Default/Preferences
        let prefs = format!(
            r#"{{
    "bookmark_bar": {{
        "show_on_all_tabs": {show_bookmarks}
    }},
    "browser": {{
        "check_default_browser": false,
        "has_seen_welcome_page": true,
        "theme": {{
            "color_scheme2": 2,
            "color_variant2": 1,
            "user_color2": {PURPLE_ARGB}
        }}
    }},
    "extensions": {{
        "theme": {{
            "id": "{extensions_theme_id}",
            "system_theme": 0
        }}
    }},
    "session": {{
        "restore_on_startup": {restore_on_startup},
        {startup_urls}
        "dummy": true
    }},
    "sync_promo": {{
        "user_skipped": true
    }},
    "first_run_tabs": [],
    "homepage": "{homepage}",
    "homepage_is_newtabpage": false,
    "ntp": {{
        "shortcut_visible": false
    }},
    "webkit": {{
        "webprefs": {{
            "dark_mode_enabled": {dark_mode}
        }}
    }}
}}"#,
            show_bookmarks = cfg.toolbar.should_show(),
        );

        let path = self.profile_dir()?.join("Default").join("Preferences");
        fs::write(path, prefs)?;
        Ok(())
    }
}

/// Compute the Chromium extension ID for an unpacked extension at the given path.
///
/// Chromium computes extension IDs as:
/// SHA-256(canonical_path) → first 16 bytes → each hex nibble mapped to \[a-p\].
///
/// Returns `None` if the computation fails (e.g. `sha256sum` not available).
fn compute_extension_id(dir: &Path) -> Option<String> {
    let canonical = dir.canonicalize().ok()?;
    let path_str = canonical.to_str()?;

    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    child
        .stdin
        .take()
        .unwrap()
        .write_all(path_str.as_bytes())
        .ok()?;

    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = std::str::from_utf8(&output.stdout).ok()?;
    let hex = stdout.get(..32)?;

    Some(
        hex.chars()
            .map(|c| {
                let n = c.to_digit(16).unwrap_or(0) as u8;
                (b'a' + n) as char
            })
            .collect(),
    )
}

/// Fetch the latest uBlock Origin Lite (MV3) Chromium zip URL from GitHub releases.
/// Full uBlock Origin uses Manifest V2 which is no longer supported by modern Chromium.
fn get_ublock_chromium_url() -> Result<String> {
    let response = common::http_agent()
        .get("https://api.github.com/repos/uBlockOrigin/uBOL-home/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "ephemeral-browser")
        .call()
        .context("fetching uBlock Origin Lite releases")?;

    let body = response.into_body().read_to_string()?;
    let json: serde_json::Value = serde_json::from_str(&body).context("parsing release JSON")?;

    let assets = json["assets"].as_array().context("no assets in release")?;

    for asset in assets {
        if let Some(name) = asset["name"].as_str()
            && name.contains("chromium")
            && name.ends_with(".zip")
            && let Some(url) = asset["browser_download_url"].as_str()
        {
            return Ok(url.to_string());
        }
    }

    anyhow::bail!("chromium zip not found in uBlock Origin Lite release assets")
}

/// Find the subdirectory containing manifest.json inside the unpacked extension.
fn find_extension_root(dir: &Path) -> Result<PathBuf> {
    // Check if manifest.json is directly in the dir
    if dir.join("manifest.json").is_file() {
        return Ok(dir.to_path_buf());
    }

    // Otherwise look one level deep (zip typically has a top-level folder)
    let entries = fs::read_dir(dir).context("reading unpacked extension dir")?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join("manifest.json").is_file() {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "manifest.json not found in unpacked extension at {}",
        dir.display()
    )
}

fn unpack_zip(zip_path: &Path, dst: &Path) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file).context("reading zip archive")?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let out_path = dst.join(entry.mangled_name());

        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out_file = fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }

    Ok(())
}

fn write_chromium_bookmarks(path: &Path, cfg: &Config) -> Result<()> {
    // Chromium uses a JSON bookmarks format
    let mut children = String::new();
    for (i, tab) in cfg.toolbar.tabs.iter().enumerate() {
        if i > 0 {
            children.push_str(",\n");
        }
        children.push_str(&format!(
            r#"            {{
                "name": "{}",
                "type": "url",
                "url": "{}"
            }}"#,
            tab.label, tab.url,
        ));
    }

    let bookmarks = format!(
        r#"{{
    "roots": {{
        "bookmark_bar": {{
            "children": [
{children}
            ],
            "name": "Bookmarks bar",
            "type": "folder"
        }},
        "other": {{
            "children": [],
            "name": "Other bookmarks",
            "type": "folder"
        }},
        "synced": {{
            "children": [],
            "name": "Mobile bookmarks",
            "type": "folder"
        }}
    }},
    "version": 1
}}"#,
    );

    fs::write(path, bookmarks)?;
    Ok(())
}
