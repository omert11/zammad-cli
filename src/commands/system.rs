use anyhow::Result;
use clap::Subcommand;

use crate::client::ZammadClient;
use crate::output;
use crate::types::NamedItem;

#[derive(Subcommand)]
pub enum SystemCmd {
    /// List all ticket groups
    Groups,
    /// List all ticket states
    States,
    /// List all ticket priorities
    Priorities,
}

pub async fn run(cmd: SystemCmd, client: &ZammadClient, json: bool) -> Result<()> {
    let (path, label) = match cmd {
        SystemCmd::Groups => ("/api/v1/groups", "groups"),
        SystemCmd::States => ("/api/v1/ticket_states", "states"),
        SystemCmd::Priorities => ("/api/v1/ticket_priorities", "priorities"),
    };
    let value = client.get::<()>(path, None).await?;
    let items: Vec<NamedItem> = serde_json::from_value(value).unwrap_or_default();
    output::render(&items, json, |i| output::print_named_table(i, label))
}
