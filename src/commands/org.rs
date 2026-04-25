use anyhow::Result;
use clap::Subcommand;

use crate::client::ZammadClient;
use crate::output;
use crate::types::Organization;

#[derive(Subcommand)]
pub enum OrgCmd {
    /// List all organizations
    List,
    /// Search organizations by name
    Search { query: String },
}

pub async fn run(cmd: OrgCmd, client: &ZammadClient, json: bool) -> Result<()> {
    match cmd {
        OrgCmd::List => list(client, json).await,
        OrgCmd::Search { query } => search(client, &query, json).await,
    }
}

async fn list(client: &ZammadClient, json: bool) -> Result<()> {
    let value = client.get::<()>("/api/v1/organizations", None).await?;
    let orgs: Vec<Organization> = serde_json::from_value(value).unwrap_or_default();
    output::render(&orgs, json, |o| output::print_org_table(o))
}

async fn search(client: &ZammadClient, query: &str, json: bool) -> Result<()> {
    let params = vec![("query", query.to_string()), ("expand", "true".to_string())];
    let value = client
        .get("/api/v1/organizations/search", Some(&params))
        .await?;
    let orgs: Vec<Organization> = serde_json::from_value(value).unwrap_or_default();
    output::render(&orgs, json, |o| output::print_org_table(o))
}
