use icn_network::run_node;
use std::process;

#[tokio::main]
async fn main() {
    // Configure logging
    tracing_subscriber::fmt::init();

    // Run the node
    if let Err(e) = run_node().await {
        eprintln!("Error running node: {}", e);
        process::exit(1);
    }
} 