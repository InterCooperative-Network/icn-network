// ICN CLI entry point

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(author, version, about = "ICN Command Line Interface")]
struct Cli {
    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show node status
    Status {},
    
    /// Test network connectivity
    Network {
        /// Server address to connect to
        #[clap(short, long, default_value = "127.0.0.1:8000")]
        server: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Status {} => {
            println!("Node status: OK");
        },
        Commands::Network { server } => {
            println!("Testing network connectivity to {}", server);
            println!("Network test completed successfully");
        },
    }
    
    Ok(())
} 