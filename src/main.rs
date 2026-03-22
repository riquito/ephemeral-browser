mod browser;
mod config;

use anyhow::Result;

fn main() -> Result<()> {
    let cfg = config::Config::load()?;

    let mut b = browser::new(&cfg);

    eprintln!("Setting up temporary profile...");
    b.setup(&cfg)?;

    let args: Vec<String> = std::env::args().skip(1).collect();
    eprintln!("Starting {}", cfg.browser);

    let result = b.launch(&args);
    b.cleanup();

    result
}
