mod browser;
mod config;

use std::sync::{Arc, Mutex};

use anyhow::Result;

fn main() -> Result<()> {
    let cfg = config::Config::load()?;

    cleanup_stale_profiles();

    let mut b = browser::new(&cfg);

    eprintln!("Setting up temporary profile...");
    b.setup(&cfg)?;

    // Wrap the browser in Arc<Mutex<>> so the signal handler can clean up
    let b = Arc::new(Mutex::new(b));
    let b_signal = Arc::clone(&b);

    ctrlc::set_handler(move || {
        if let Ok(browser) = b_signal.lock() {
            browser.cleanup();
        }
        std::process::exit(130); // 128 + SIGINT(2)
    })?;

    let args: Vec<String> = std::env::args().skip(1).collect();
    eprintln!("Starting {}", cfg.browser);

    let result = b.lock().unwrap().launch(&args);
    b.lock().unwrap().cleanup();

    result
}

/// Remove leftover ephemeral-browser-* temp directories from previous
/// sessions that weren't cleaned up (e.g. due to a crash or SIGKILL).
fn cleanup_stale_profiles() {
    let tmp = std::env::temp_dir();
    let Ok(entries) = std::fs::read_dir(&tmp) else {
        return;
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with("ephemeral-browser-") {
            continue;
        }
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let is_stale = match browser::common::read_pid_file(&path) {
            Some(pid) => !browser::common::is_process_alive(pid),
            // No PID file — old format or corrupted, use age heuristic
            None => std::fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age > std::time::Duration::from_secs(3600)),
        };

        if is_stale {
            eprintln!("Cleaning up stale profile: {}", path.display());
            let _ = std::fs::remove_dir_all(&path);
        }
    }
}
