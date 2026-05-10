use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "loom-daemon")]
#[command(version)]
#[command(about = "Local daemon for x07 Studio.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value = "127.0.0.1:7719")]
        addr: SocketAddr,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("loom_daemon=info,axum=info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Serve { root, addr } => {
            let state = loom_daemon::default_state(camino::Utf8PathBuf::from(root))?;
            loom_daemon::serve(addr, state).await?;
        }
    }

    Ok(())
}
