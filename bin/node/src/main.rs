// ICN Node entry point

use std::sync::Arc;
use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting ICN Node...");
    info!("Node initialized, press Ctrl+C to exit");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    info!("Shutting down ICN Node...");

    Ok(())
} 