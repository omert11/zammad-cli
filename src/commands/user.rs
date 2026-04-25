use anyhow::Result;
use clap::Subcommand;

use crate::client::ZammadClient;
use crate::output;
use crate::types::User;

#[derive(Subcommand)]
pub enum UserCmd {
    /// Search users by name, email, or phone
    Search { query: String },
}

pub async fn run(cmd: UserCmd, client: &ZammadClient, json: bool) -> Result<()> {
    match cmd {
        UserCmd::Search { query } => search(client, &query, json).await,
    }
}

async fn search(client: &ZammadClient, query: &str, json: bool) -> Result<()> {
    let params = vec![("query", query.to_string()), ("expand", "true".to_string())];
    let value = client.get("/api/v1/users/search", Some(&params)).await?;
    let users: Vec<User> = serde_json::from_value(value).unwrap_or_default();
    output::render(&users, json, |u| output::print_user_table(u))
}
