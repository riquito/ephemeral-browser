mod chromium;
pub mod common;
mod firefox;

use crate::config::Config;
use anyhow::Result;

pub use chromium::Chromium;
pub use firefox::Firefox;

pub trait Browser: Send {
    /// Create a temporary profile directory and configure it.
    fn setup(&mut self, cfg: &Config) -> Result<()>;

    /// Start the browser. Extra args are passed through (e.g. URLs).
    fn launch(&self, args: &[String]) -> Result<()>;

    /// Remove the temporary profile directory.
    fn cleanup(&self);
}

pub fn new(cfg: &Config) -> Box<dyn Browser> {
    match cfg.browser {
        crate::config::BrowserKind::Firefox => Box::new(Firefox::default()),
        kind @ (crate::config::BrowserKind::Chromium | crate::config::BrowserKind::Chrome) => {
            Box::new(Chromium::new(kind))
        }
    }
}
