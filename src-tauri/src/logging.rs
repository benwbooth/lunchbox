//! Logging configuration with rolling file appender
//!
//! Sets up tracing with both console output and rolling log files.
//! Logs are stored in ~/.local/share/lunchbox/logs/

use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Get the logs directory path
pub fn logs_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.data_dir().join("lunchbox").join("logs"))
        .unwrap_or_else(|| PathBuf::from("logs"))
}

/// Initialize logging with both console and rolling file output.
///
/// Log files are stored in ~/.local/share/lunchbox/logs/ with daily rotation.
/// The default log level is INFO, with DEBUG for lunchbox modules.
/// Override with RUST_LOG environment variable.
pub fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let logs_dir = logs_dir();

    // Create logs directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&logs_dir) {
        eprintln!("Warning: Failed to create logs directory: {}", e);
    }

    // Rolling file appender - rotates daily, keeps logs with date suffix
    let file_appender = RollingFileAppender::new(Rotation::DAILY, &logs_dir, "lunchbox.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Console layer - pretty output for terminal
    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    // File layer - more detailed output for debugging
    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false) // No ANSI colors in file
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE);

    // Environment filter - default to info with debug for lunchbox
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,lunchbox=debug,lunchbox_lib=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    guard
}

/// Initialize logging for dev server (similar but slightly different defaults)
pub fn init_dev_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let logs_dir = logs_dir();

    // Create logs directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&logs_dir) {
        eprintln!("Warning: Failed to create logs directory: {}", e);
    }

    // Rolling file appender
    let file_appender = RollingFileAppender::new(Rotation::DAILY, &logs_dir, "lunchbox-dev.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Console layer
    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    // File layer - more detailed
    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE);

    // Environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,lunchbox=debug,lunchbox_lib=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    guard
}
