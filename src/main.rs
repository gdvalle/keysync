use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process;

mod client;
mod config;
mod keyboard;
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
    // If rust log is not setup, set our default logging just for our library.
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", format!("{}=debug", env!("CARGO_CRATE_NAME")));
    }

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_target(false)
        .init();

    if let Err(e) = run() {
        tracing::error!("Application error: {}", e);
        // Print cause chain for better diagnostics
        let mut source = e.source();
        while let Some(cause) = source {
            tracing::error!("Caused by: {}", cause);
            source = cause.source();
        }
        process::exit(1);
    }
}
