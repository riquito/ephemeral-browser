use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    Default,
}

impl Theme {
    pub fn firefox_theme_id(self) -> &'static str {
        match self {
            Theme::Dark => "firefox-compact-dark@mozilla.org",
            Theme::Light => "firefox-compact-light@mozilla.org",
            Theme::Default => "default-theme@mozilla.org",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BrowserKind {
    #[default]
    Firefox,
    Chromium,
    Chrome,
}

impl std::fmt::Display for BrowserKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserKind::Firefox => write!(f, "firefox"),
            BrowserKind::Chromium => write!(f, "chromium"),
            BrowserKind::Chrome => write!(f, "chrome"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ToolbarTab {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct Toolbar {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub tabs: Vec<ToolbarTab>,
}

impl Toolbar {
    pub fn should_show(&self) -> bool {
        self.enabled && !self.tabs.is_empty()
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub homepage: Option<String>,
    #[serde(default)]
    pub search_engine: SearchEngine,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub browser: BrowserKind,
    pub browser_path: Option<PathBuf>,
    #[serde(default)]
    pub toolbar: Toolbar,
}

#[derive(Debug, Deserialize)]
pub struct SearchEngine(String);

impl Default for SearchEngine {
    fn default() -> Self {
        Self("DuckDuckGo".into())
    }
}

impl std::fmt::Display for SearchEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Config {
    /// Returns the configured homepage URL.
    /// If homepage is not set in config, returns the default.
    /// If homepage is explicitly set to "", returns "".
    pub fn homepage_url(&self) -> &str {
        match &self.homepage {
            None => "https://duckduckgo.com",
            Some(url) => url,
        }
    }

    /// Load reads the config file, falling back to defaults when missing.
    pub fn load() -> Result<Self> {
        let Some(path) = find_config_file() else {
            return Ok(Self::default());
        };

        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Ok(Self::default()),
        };

        toml::from_str(&contents).context("parsing config.toml")
    }
}

fn find_config_file() -> Option<PathBuf> {
    // Next to the executable
    if let Ok(exe) = std::env::current_exe() {
        let p = exe.with_file_name("config.toml");
        if p.is_file() {
            return Some(p);
        }
    }

    // Current working directory
    let cwd = PathBuf::from("config.toml");
    if cwd.is_file() {
        return Some(cwd);
    }

    // OS config directory
    if let Some(config_dir) = dirs::config_dir() {
        let p = config_dir.join("ephemeral-browser").join("config.toml");
        if p.is_file() {
            return Some(p);
        }
    }

    None
}
