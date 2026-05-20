use anyhow::Result;
use clap::{Parser, Subcommand};

mod client;
mod commands;
mod config;
mod output;
mod types;
mod util;

use commands::{org, system, tags, ticket, user};

#[derive(Parser)]
#[command(name = "zammad-cli")]
#[command(version, about = "CLI for Zammad helpdesk system", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Ticket operations
    Ticket {
        #[command(subcommand)]
        cmd: ticket::TicketCmd,
    },
    /// Organization operations
    Org {
        #[command(subcommand)]
        cmd: org::OrgCmd,
    },
    /// User operations
    User {
        #[command(subcommand)]
        cmd: user::UserCmd,
    },
    /// System operations (groups, states, priorities)
    System {
        #[command(subcommand)]
        cmd: system::SystemCmd,
    },
    /// Tag operations (list/add/remove on any object)
    Tags {
        #[command(subcommand)]
        cmd: tags::TagsCmd,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = config::load()?;
    let client = client::ZammadClient::new(&cfg)?;

    match cli.command {
        Commands::Ticket { cmd } => ticket::run(cmd, &client, cli.json).await,
        Commands::Org { cmd } => org::run(cmd, &client, cli.json).await,
        Commands::User { cmd } => user::run(cmd, &client, cli.json).await,
        Commands::System { cmd } => system::run(cmd, &client, cli.json).await,
        Commands::Tags { cmd } => tags::run(cmd, &client, cli.json).await,
    }
}
