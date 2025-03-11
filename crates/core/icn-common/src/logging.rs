use std::path::Path;
use tracing::{Level, Subscriber};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

use crate::error::Result;

pub fn init_logging(
    log_dir: impl AsRef<Path>,
    node_id: &str,
    log_level: &str,
) -> Result<()> {
    // Create file appender
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        log_dir.as_ref(),
        format!("{}.log", node_id),
    );

    // Parse log level
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    // Create console layer
    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(env_filter.clone());

    // Create file layer
    let file_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_ansi(false)
        .with_writer(file_appender)
        .with_filter(env_filter);

    // Combine layers and set as global default
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .try_init()
        .map_err(|e| format!("Failed to initialize logging: {}", e))?;

    Ok(())
} 