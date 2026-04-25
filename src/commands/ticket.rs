use anyhow::{anyhow, Result};
use clap::Subcommand;
use futures::future::try_join_all;
use serde_json::{Map, Value};

use crate::client::ZammadClient;
use crate::output;
use crate::types::{Article, Ticket};
use crate::util::{build_search_query, insert_opt_str};

#[derive(Subcommand)]
pub enum TicketCmd {
    /// Search tickets by text query (Zammad search syntax accepted)
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// List tickets with filters
    List {
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        group: Option<String>,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        organization: Option<String>,
        #[arg(long)]
        customer: Option<String>,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long, default_value_t = 20)]
        per_page: u32,
    },
    /// Get ticket details by `#NUMBER` (ticket number) or plain integer (internal ID)
    Get { id: String },
    /// Get all articles/messages of a ticket (`#NUMBER` or internal ID)
    Articles { id: String },
    /// Create a new ticket
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: String,
        #[arg(long, default_value = "Destek Ekibi")]
        group: String,
        #[arg(long)]
        customer: Option<String>,
        #[arg(long, default_value = "2 normal")]
        priority: String,
        #[arg(long, default_value = "new")]
        state: String,
    },
    /// Update ticket (`#NUMBER` or internal ID)
    Update {
        id: String,
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        title: Option<String>,
    },
    /// Article subcommands
    Article {
        #[command(subcommand)]
        cmd: ArticleCmd,
    },
    /// Get summary of ticket counts by state
    Overview,
}

#[derive(Subcommand)]
pub enum ArticleCmd {
    /// Add a note/article to a ticket (`#NUMBER` or internal ID)
    Add {
        id: String,
        #[arg(long)]
        body: String,
        #[arg(long)]
        subject: Option<String>,
        /// Mark as public reply (default: internal note)
        #[arg(long)]
        public: bool,
    },
}

pub async fn run(cmd: TicketCmd, client: &ZammadClient, json: bool) -> Result<()> {
    match cmd {
        TicketCmd::Search { query, limit } => search(client, &query, limit, json).await,
        TicketCmd::List {
            state,
            group,
            owner,
            organization,
            customer,
            priority,
            page,
            per_page,
        } => {
            list(
                client,
                state,
                group,
                owner,
                organization,
                customer,
                priority,
                page,
                per_page,
                json,
            )
            .await
        }
        TicketCmd::Get { id } => get(client, &id, json).await,
        TicketCmd::Articles { id } => articles(client, &id, json).await,
        TicketCmd::Create {
            title,
            body,
            group,
            customer,
            priority,
            state,
        } => create(client, title, body, group, customer, priority, state, json).await,
        TicketCmd::Update {
            id,
            state,
            priority,
            owner,
            title,
        } => update(client, &id, state, priority, owner, title, json).await,
        TicketCmd::Article { cmd } => match cmd {
            ArticleCmd::Add {
                id,
                body,
                subject,
                public,
            } => article_add(client, &id, body, subject, !public, json).await,
        },
        TicketCmd::Overview => overview(client, json).await,
    }
}

async fn search(client: &ZammadClient, query: &str, limit: u32, json: bool) -> Result<()> {
    let query_params = vec![
        ("query", query.to_string()),
        ("per_page", limit.to_string()),
        ("expand", "true".to_string()),
    ];
    let value = client
        .get("/api/v1/tickets/search", Some(&query_params))
        .await?;
    let tickets: Vec<Ticket> = serde_json::from_value(value).unwrap_or_default();
    output::render(&tickets, json, |t| output::print_ticket_table(t))
}

#[allow(clippy::too_many_arguments)]
async fn list(
    client: &ZammadClient,
    state: Option<String>,
    group: Option<String>,
    owner: Option<String>,
    organization: Option<String>,
    customer: Option<String>,
    priority: Option<String>,
    page: u32,
    per_page: u32,
    json: bool,
) -> Result<()> {
    let mut parts: Vec<(&str, String)> = Vec::new();
    if let Some(v) = state {
        parts.push(("state.name", v));
    }
    if let Some(v) = group {
        parts.push(("group.name", v));
    }
    if let Some(v) = owner {
        parts.push(("owner.email", v));
    }
    if let Some(v) = organization {
        parts.push(("organization.name", v));
    }
    if let Some(v) = customer {
        parts.push(("customer.email", v));
    }
    if let Some(v) = priority {
        parts.push(("priority.name", v));
    }
    let query = build_search_query(&parts);

    let query_params = vec![
        ("query", query),
        ("page", page.to_string()),
        ("per_page", per_page.to_string()),
        ("expand", "true".to_string()),
    ];
    let value = client
        .get("/api/v1/tickets/search", Some(&query_params))
        .await?;
    let tickets: Vec<Ticket> = serde_json::from_value(value).unwrap_or_default();
    output::render(&tickets, json, |t| output::print_ticket_table(t))
}

async fn get(client: &ZammadClient, id_str: &str, json: bool) -> Result<()> {
    let resolved = resolve_ticket_id(client, id_str).await?;
    let value = client
        .get(
            &format!("/api/v1/tickets/{resolved}"),
            Some(&[("expand", "true")]),
        )
        .await?;
    if json {
        return output::emit_value(&value);
    }
    let ticket: Ticket = serde_json::from_value(value)?;
    output::print_ticket_detail(&ticket);
    Ok(())
}

async fn articles(client: &ZammadClient, id_str: &str, json: bool) -> Result<()> {
    let resolved = resolve_ticket_id(client, id_str).await?;
    let value = client
        .get::<()>(
            &format!("/api/v1/ticket_articles/by_ticket/{resolved}"),
            None,
        )
        .await?;
    let articles: Vec<Article> = serde_json::from_value(value).unwrap_or_default();
    output::render(&articles, json, |a| output::print_articles(a))
}

#[allow(clippy::too_many_arguments)]
async fn create(
    client: &ZammadClient,
    title: String,
    body_text: String,
    group: String,
    customer: Option<String>,
    priority: String,
    state: String,
    json: bool,
) -> Result<()> {
    let mut payload: Map<String, Value> = Map::new();
    payload.insert("title".into(), Value::String(title.clone()));
    payload.insert("group".into(), Value::String(group));
    payload.insert("priority".into(), serde_json::json!({ "name": priority }));
    payload.insert("state".into(), serde_json::json!({ "name": state }));
    payload.insert(
        "article".into(),
        serde_json::json!({
            "subject": title,
            "body": body_text,
            "type": "note",
            "internal": false,
        }),
    );
    insert_opt_str(&mut payload, "customer", customer);

    let value = client
        .post("/api/v1/tickets", Some(&Value::Object(payload)))
        .await?;
    if json {
        return output::emit_value(&value);
    }
    let ticket: Ticket = serde_json::from_value(value)?;
    output::print_ticket_detail(&ticket);
    Ok(())
}

async fn update(
    client: &ZammadClient,
    id_str: &str,
    state: Option<String>,
    priority: Option<String>,
    owner: Option<String>,
    title: Option<String>,
    json: bool,
) -> Result<()> {
    let mut body: Map<String, Value> = Map::new();
    insert_opt_str(&mut body, "state", state);
    insert_opt_str(&mut body, "priority", priority);
    insert_opt_str(&mut body, "owner", owner);
    insert_opt_str(&mut body, "title", title);
    if body.is_empty() {
        anyhow::bail!("No fields provided to update");
    }

    let resolved = resolve_ticket_id(client, id_str).await?;
    let value = client
        .put(
            &format!("/api/v1/tickets/{resolved}"),
            Some(&Value::Object(body)),
        )
        .await?;
    if json {
        return output::emit_value(&value);
    }
    let ticket: Ticket = serde_json::from_value(value)?;
    output::print_ticket_detail(&ticket);
    Ok(())
}

async fn article_add(
    client: &ZammadClient,
    id_str: &str,
    body_text: String,
    subject: Option<String>,
    internal: bool,
    json: bool,
) -> Result<()> {
    let resolved = resolve_ticket_id(client, id_str).await?;
    let mut body: Map<String, Value> = Map::new();
    body.insert("ticket_id".into(), Value::Number(resolved.into()));
    body.insert("body".into(), Value::String(body_text));
    body.insert("type".into(), Value::String("note".into()));
    body.insert("internal".into(), Value::Bool(internal));
    insert_opt_str(&mut body, "subject", subject);

    let value = client
        .post("/api/v1/ticket_articles", Some(&Value::Object(body)))
        .await?;
    if json {
        return output::emit_value(&value);
    }
    let article_id = value.get("id").and_then(|v| v.as_i64()).unwrap_or_default();
    let kind = if internal { "internal" } else { "public" };
    output::print_message(&format!("Article {article_id} added ({kind})"));
    Ok(())
}

async fn overview(client: &ZammadClient, json: bool) -> Result<()> {
    let states = ["new", "open", "pending reminder", "pending close", "closed"];

    let futs = states.iter().map(|s| async move {
        let params = vec![
            ("query", format!("state.name:{s}")),
            ("per_page", "100".to_string()),
        ];
        let value = client.get("/api/v1/tickets/search", Some(&params)).await?;
        let count = value.as_array().map(|a| a.len()).unwrap_or(0);
        Ok::<_, anyhow::Error>((s.to_string(), count))
    });
    let summary = try_join_all(futs).await?;

    if json {
        let map: Map<String, Value> = summary
            .iter()
            .map(|(s, n)| (s.clone(), Value::Number((*n as i64).into())))
            .collect();
        return output::emit_value(&Value::Object(map));
    }

    use colored::Colorize;
    println!("{}", "Ticket Overview".bold().underline());
    for (s, n) in &summary {
        println!("  {:<20} {}", s, n.to_string().bold());
    }
    Ok(())
}

/// Resolve a user-supplied ticket reference to an internal Zammad ID.
///
/// Accepts:
/// - `#61234` → search Zammad for `number:61234`, return internal ID
/// - `42`     → already an internal ID, return as-is
///
/// Avoids the brittle "if id >= 60000 it's a number" heuristic from the
/// original Python source — Zammad instances start ticket numbers at 1
/// by default, so threshold-based detection misclassifies.
async fn resolve_ticket_id(client: &ZammadClient, id_str: &str) -> Result<i64> {
    if let Some(num_str) = id_str.strip_prefix('#') {
        let number: u64 = num_str
            .parse()
            .map_err(|_| anyhow!("Invalid ticket number after '#': {num_str}"))?;
        let params = vec![
            ("query", format!("number:{number}")),
            ("per_page", "1".to_string()),
        ];
        let value = client.get("/api/v1/tickets/search", Some(&params)).await?;
        if let Some(first) = value
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_i64())
        {
            return Ok(first);
        }
        anyhow::bail!("Ticket number #{number} not found");
    }

    id_str
        .parse::<i64>()
        .map_err(|_| anyhow!("Invalid ticket ID: {id_str} (use plain integer or '#NUMBER')"))
}
