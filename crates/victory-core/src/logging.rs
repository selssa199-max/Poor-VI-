//! One tracing setup for every binary.
//!
//! Filter with `VICTORY_LOG` (falls back to `RUST_LOG`), e.g.
//! `VICTORY_LOG=victory_loader=trace,info`. Debug and trace output exist
//! only in builds with the `diagnostics` feature; a plain release build
//! logs info and above whatever the filter asks for.
//! Set `VICTORY_LOG_FILE` to append the same messages to a plain-text file.

use std::io::Write;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// The most verbose level this build can log.
const CAP: LevelFilter = if cfg!(feature = "diagnostics") {
    LevelFilter::TRACE
} else {
    LevelFilter::INFO
};

pub fn init(default_level: &str) {
    let filter = std::env::var("VICTORY_LOG")
        .or_else(|_| std::env::var("RUST_LOG"))
        .ok()
        .and_then(|v| EnvFilter::try_new(v).ok())
        .unwrap_or_else(|| EnvFilter::new(default_level));
    let file_layer = std::env::var_os("VICTORY_LOG_FILE")
        .filter(|p| !p.is_empty())
        .and_then(|path| match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(file) => Some(file),
            Err(error) => {
                let path = std::path::Path::new(&path).display();
                let _ = writeln!(std::io::stderr(), "Could not open VICTORY_LOG_FILE {path}: {error}");
                None
            }
        })
        .map(|f| {
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(f))
        });
    let _ = tracing_subscriber::registry()
        .with(CAP)
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_writer(std::io::stderr),
        )
        .with(file_layer)
        .try_init();
}
