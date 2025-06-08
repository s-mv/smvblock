mod cli;
mod http;

use anyhow::Result;
use clap::{Parser, Subcommand};
use url::Url;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long)]
    gui: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Status {
        #[arg(long)]
        node: Url,
    },

    SendTx {
        #[arg(long)]
        node: Url,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.gui {
        if cli.command.is_some() {
            eprintln!("Cannot use both GUI mode and CLI commands");
            std::process::exit(1);
        }

        println!("GUI mode not yet implemented!");
        return Ok(());
    }

    match cli.command {
        Some(Commands::Status { node }) => {
            cli::handle_status(node).await?;
        }
        Some(Commands::SendTx { node, to, amount }) => {
            cli::handle_send_tx(node, to, amount).await?;
        }
        None => {
            println!("No command specified. Use --help for usage information.");
        }
    }

    Ok(())
}
