use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process;

mod client;
mod config;
mod keyboard;
mod protocol;
mod reconnectable_stream;
mod server;
mod utils;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run in server mode
    Server {
        /// Address to listen on
        #[arg(short, long, default_value = "0.0.0.0:1234")]
        bind_address: String,
    },
    /// Run in client mode
    Client {
        /// Server address to connect to
        #[arg(short, long, default_value = "127.0.0.1:1234")]
        server_address: String,
    },
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Server {
            bind_address: listen_addr,
        } => {
            server::run(listen_addr)?;
        }
        Commands::Client {
            server_address: server_addr,
        } => {
            client::run(server_addr)?;
        }
    }

    Ok(())
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                // If RUST_LOG isn't set, set a default level.
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    if let Err(e) = run() {
        tracing::error!(cause = e.source(), "Application error: {}", e);
        process::exit(1);
    }
}
