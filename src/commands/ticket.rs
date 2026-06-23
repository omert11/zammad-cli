use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use futures::future::try_join_all;
use serde_json::{Map, Value};

use crate::client::ZammadClient;
use crate::commands::tags::{tag_op, TICKET_OBJECT};
use crate::output;
use crate::types::{Article, Ticket};
use crate::util::{
    build_attachments_opt, build_search_query, insert_opt_str, is_iso8601_datetime, split_csv,
};

#[derive(Subcommand)]
pub enum TicketCmd {
    /// Search tickets by text query (Zammad search syntax accepted)
    Search {
        query: String,
        #[arg(long, visible_alias = "per-page", default_value_t = 20)]
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
        #[arg(long, visible_alias = "limit", default_value_t = 20)]
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
        /// Assign owner (email or login)
        #[arg(long)]
        owner: Option<String>,
        /// Attach to organization (by name)
        #[arg(long)]
        organization: Option<String>,
        /// Comma-separated tags to add after creation (e.g. "billing,urgent")
        #[arg(long)]
        tags: Option<String>,
        /// Comma-separated file paths to attach to the initial article
        #[arg(long)]
        attachments: Option<String>,
    },
    /// Update ticket (`#NUMBER` or internal ID)
    Update {
        id: String,
        #[arg(long)]
        state: Option<String>,
        /// ISO 8601 timestamp for pending states (e.g. 2026-06-18T17:00:00Z).
        /// Required when --state is "pending close" or "pending reminder".
        #[arg(long = "pending-time")]
        pending_time: Option<String>,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        customer: Option<String>,
        #[arg(long)]
        organization: Option<String>,
        /// Comma-separated tags to add (e.g. "billing,urgent")
        #[arg(long = "tags-add")]
        tags_add: Option<String>,
        /// Comma-separated tags to remove
        #[arg(long = "tags-remove")]
        tags_remove: Option<String>,
        /// Comma-separated Jira ticket keys (e.g. "TIO-817, TIO-818")
        #[arg(long = "jira-tickets")]
        jira_tickets: Option<String>,
    },
    /// Article subcommands
    Article {
        #[command(subcommand)]
        cmd: ArticleCmd,
    },
    /// Attachment subcommands (list/download)
    Attachment {
        #[command(subcommand)]
        cmd: AttachmentCmd,
    },
    /// Get summary of ticket counts by state
    Overview,
    /// Create or update a shared draft (internal ID — see `ticket get` for the numeric ID)
    SharedDraft {
        id: String,
        #[arg(long)]
        body: String,
        /// Article type (email or note; default: email)
        #[arg(long, default_value = "email")]
        r#type: String,
        /// Mark as internal note (default: public reply)
        #[arg(long)]
        internal: bool,
    },
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
        /// Comma-separated file paths to attach
        #[arg(long)]
        attachments: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum AttachmentCmd {
    /// List all attachments of a ticket (`#NUMBER` or internal ID) with their IDs
    List { id: String },
    /// Download attachment(s) from a ticket (`#NUMBER` or internal ID)
    Download {
        id: String,
        /// Article ID owning the attachment (see `attachment list` or `articles`)
        #[arg(long, requires = "attachment")]
        article: Option<i64>,
        /// Attachment ID to download (requires --article)
        #[arg(long)]
        attachment: Option<i64>,
        /// Download every attachment on the ticket
        #[arg(long, conflicts_with_all = ["article", "attachment"])]
        all: bool,
        /// Output directory (created if missing; defaults to current dir)
        #[arg(long, short, default_value = ".")]
        out: String,
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
            owner,
            organization,
            tags,
            attachments,
        } => {
            create(
                client,
                title,
                body,
                group,
                customer,
                priority,
                state,
                owner,
                organization,
                tags,
                attachments,
                json,
            )
            .await
        }
        TicketCmd::Update {
            id,
            state,
            pending_time,
            priority,
            owner,
            title,
            customer,
            organization,
            tags_add,
            tags_remove,
            jira_tickets,
        } => {
            update(
                client,
                &id,
                state,
                pending_time,
                priority,
                owner,
                title,
                customer,
                organization,
                tags_add,
                tags_remove,
                jira_tickets,
                json,
            )
                .await
        }
        TicketCmd::Article { cmd } => match cmd {
            ArticleCmd::Add {
                id,
                body,
                subject,
                public,
                attachments,
            } => article_add(client, &id, body, subject, !public, attachments, json).await,
        },
        TicketCmd::Attachment { cmd } => match cmd {
            AttachmentCmd::List { id } => attachment_list(client, &id, json).await,
            AttachmentCmd::Download {
                id,
                article,
                attachment,
                all,
                out,
            } => attachment_download(client, &id, article, attachment, all, &out, json).await,
        },
        TicketCmd::Overview => overview(client, json).await,
        TicketCmd::SharedDraft {
            id,
            body,
            r#type,
            internal,
        } => shared_draft(client, &id, body, r#type, internal, json).await,
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

async fn fetch_articles(client: &ZammadClient, ticket_id: i64) -> Result<Vec<Article>> {
    let value = client
        .get::<()>(
            &format!("/api/v1/ticket_articles/by_ticket/{ticket_id}"),
            None,
        )
        .await?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

async fn articles(client: &ZammadClient, id_str: &str, json: bool) -> Result<()> {
    let resolved = resolve_ticket_id(client, id_str).await?;
    let articles = fetch_articles(client, resolved).await?;
    output::render(&articles, json, |a| output::print_articles(a))
}

/// List every attachment across a ticket's articles, with article + attachment IDs.
async fn attachment_list(client: &ZammadClient, id_str: &str, json: bool) -> Result<()> {
    let resolved = resolve_ticket_id(client, id_str).await?;
    let articles = fetch_articles(client, resolved).await?;

    let mut rows: Vec<Value> = Vec::new();
    for a in &articles {
        for att in &a.attachments {
            let Some(att_id) = att.id else { continue };
            rows.push(serde_json::json!({
                "ticket_id": resolved,
                "article_id": a.id,
                "attachment_id": att_id,
                "filename": att.filename,
                "size": att.size,
            }));
        }
    }

    if json {
        return output::emit_value(&Value::Array(rows));
    }

    if rows.is_empty() {
        println!("No attachments on ticket {resolved}");
        return Ok(());
    }
    println!("Attachments on ticket {resolved}:");
    for r in &rows {
        let article_id = r["article_id"].as_i64().unwrap_or_default();
        let att_id = r["attachment_id"].as_i64().unwrap_or_default();
        let filename = r["filename"].as_str().unwrap_or("attachment");
        let size = r["size"].as_str().unwrap_or("?");
        println!("  article {article_id} / attachment {att_id}  {filename} ({size} bytes)");
    }
    println!(
        "\nDownload: zammad-cli ticket attachment download {resolved} \
         --article <ARTICLE_ID> --attachment <ATTACHMENT_ID>"
    );
    Ok(())
}

/// Download one attachment (article+attachment IDs) or all of a ticket's attachments.
async fn attachment_download(
    client: &ZammadClient,
    id_str: &str,
    article: Option<i64>,
    attachment: Option<i64>,
    all: bool,
    out_dir: &str,
    json: bool,
) -> Result<()> {
    let resolved = resolve_ticket_id(client, id_str).await?;
    tokio::fs::create_dir_all(out_dir)
        .await
        .with_context(|| format!("Failed to create output directory: {out_dir}"))?;

    // Collect (article_id, attachment_id, filename) targets.
    let mut targets: Vec<(i64, i64, String)> = Vec::new();
    if all {
        let articles = fetch_articles(client, resolved).await?;
        for a in &articles {
            for att in &a.attachments {
                if let Some(att_id) = att.id {
                    targets.push((a.id, att_id, att.filename.clone()));
                }
            }
        }
        if targets.is_empty() {
            return Err(anyhow!("Ticket {resolved} has no attachments"));
        }
    } else {
        let article_id =
            article.ok_or_else(|| anyhow!("Provide --article and --attachment, or --all"))?;
        let att_id =
            attachment.ok_or_else(|| anyhow!("Provide --article and --attachment, or --all"))?;
        // Resolve the filename from the article metadata when available.
        let filename = fetch_articles(client, resolved)
            .await
            .ok()
            .and_then(|articles| {
                articles
                    .iter()
                    .find(|a| a.id == article_id)
                    .and_then(|a| a.attachments.iter().find(|att| att.id == Some(att_id)))
                    .map(|att| att.filename.clone())
            })
            .unwrap_or_else(|| format!("attachment-{att_id}"));
        targets.push((article_id, att_id, filename));
    }

    let mut saved: Vec<Value> = Vec::new();
    for (article_id, att_id, filename) in targets {
        let path = format!("/api/v1/ticket_attachment/{resolved}/{article_id}/{att_id}");
        let (bytes, _ct) = client.get_bytes(&path).await?;
        let dest = dedupe_path(out_dir, &filename);
        tokio::fs::write(&dest, &bytes)
            .await
            .with_context(|| format!("Failed to write {dest}"))?;
        if !json {
            println!("Saved {} ({} bytes)", dest, bytes.len());
        }
        saved.push(serde_json::json!({
            "article_id": article_id,
            "attachment_id": att_id,
            "path": dest,
            "bytes": bytes.len(),
        }));
    }

    if json {
        return output::emit_value(&Value::Array(saved));
    }
    Ok(())
}

/// Join `dir`/`filename`, appending a counter (`name (1).ext`) when the path exists,
/// so downloading multiple same-named attachments does not overwrite.
fn dedupe_path(dir: &str, filename: &str) -> String {
    let base = std::path::Path::new(dir).join(filename);
    if !base.exists() {
        return base.to_string_lossy().into_owned();
    }
    let path = std::path::Path::new(filename);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("attachment");
    let ext = path.extension().and_then(|s| s.to_str());
    for n in 1..10_000 {
        let candidate = match ext {
            Some(e) => format!("{stem} ({n}).{e}"),
            None => format!("{stem} ({n})"),
        };
        let p = std::path::Path::new(dir).join(&candidate);
        if !p.exists() {
            return p.to_string_lossy().into_owned();
        }
    }
    base.to_string_lossy().into_owned()
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
    owner: Option<String>,
    organization: Option<String>,
    tags: Option<String>,
    attachments: Option<String>,
    json: bool,
) -> Result<()> {
    let attachment_objs = build_attachments_opt(attachments.as_deref()).await?;

    let mut article = serde_json::json!({
        "subject": title,
        "body": body_text,
        "type": "note",
        "internal": false,
    });
    if !attachment_objs.is_empty() {
        article["attachments"] = Value::Array(attachment_objs);
    }

    let mut payload: Map<String, Value> = Map::new();
    payload.insert("title".into(), Value::String(title.clone()));
    payload.insert("group".into(), Value::String(group));
    // Plain name string. Wrapping in `{"name": ...}` triggers Zammad
    // ActiveSupport::HashWithIndifferentAccess cast error on POST.
    payload.insert("priority".into(), Value::String(priority));
    payload.insert("state".into(), Value::String(state));
    payload.insert("article".into(), article);
    insert_opt_str(&mut payload, "customer", customer);
    insert_opt_str(&mut payload, "owner", owner);
    insert_opt_str(&mut payload, "organization", organization);

    let value = client
        .post("/api/v1/tickets", Some(&Value::Object(payload)))
        .await?;

    // Tags require a separate call per item — Zammad does not accept tags in the create payload
    let tag_list = tags.as_deref().map(split_csv).unwrap_or_default();
    if !tag_list.is_empty() {
        if let Some(ticket_id) = value.get("id").and_then(|v| v.as_i64()) {
            apply_tags(client, ticket_id, &tag_list, true).await?;
        }
    }

    if json {
        return output::emit_value(&value);
    }
    let ticket: Ticket = serde_json::from_value(value)?;
    output::print_ticket_detail(&ticket);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn update(
    client: &ZammadClient,
    id_str: &str,
    state: Option<String>,
    pending_time: Option<String>,
    priority: Option<String>,
    owner: Option<String>,
    title: Option<String>,
    customer: Option<String>,
    organization: Option<String>,
    tags_add: Option<String>,
    tags_remove: Option<String>,
    jira_tickets: Option<String>,
    json: bool,
) -> Result<()> {
    // Pending states require a `pending_time`; fail early with a clear message
    // instead of surfacing Zammad's opaque 422 "Missing required value" error.
    // Comparison is case-insensitive so "Pending Close" is treated the same as
    // the canonical "pending close" and does not slip past validation.
    if let Some(s) = state.as_deref() {
        let is_pending = matches!(
            s.to_lowercase().as_str(),
            "pending close" | "pending reminder"
        );
        if is_pending && pending_time.is_none() {
            anyhow::bail!(
                "state \"{s}\" requires --pending-time <ISO8601> (e.g. 2026-06-18T17:00:00Z)"
            );
        }
        if !is_pending && pending_time.is_some() {
            anyhow::bail!(
                "--pending-time only applies to \"pending close\" or \"pending reminder\" states"
            );
        }
    }
    // When --state is omitted, --pending-time is still allowed: it reschedules a
    // ticket already in a pending state on the server (Zammad accepts a lone
    // `pending_time` PUT). We cannot know the server-side state here, so let
    // Zammad reject it if the ticket is not pending.

    // Validate the timestamp shape before sending, so an obvious typo surfaces a
    // clear error instead of Zammad's opaque 422. Kept minimal (no chrono dep):
    // require an ISO 8601 date+time prefix `YYYY-MM-DDTHH:MM`.
    if let Some(pt) = pending_time.as_deref() {
        if !is_iso8601_datetime(pt) {
            anyhow::bail!(
                "--pending-time \"{pt}\" is not a valid ISO 8601 timestamp \
                 (e.g. 2026-06-18T17:00:00Z)"
            );
        }
    }

    let mut body: Map<String, Value> = Map::new();
    insert_opt_str(&mut body, "state", state);
    insert_opt_str(&mut body, "pending_time", pending_time);
    insert_opt_str(&mut body, "priority", priority);
    insert_opt_str(&mut body, "owner", owner);
    insert_opt_str(&mut body, "title", title);
    insert_opt_str(&mut body, "customer", customer);
    insert_opt_str(&mut body, "organization", organization);
    insert_opt_str(&mut body, "jira_tickets", jira_tickets);

    let add_list = tags_add.as_deref().map(split_csv).unwrap_or_default();
    let remove_list = tags_remove.as_deref().map(split_csv).unwrap_or_default();

    if body.is_empty() && add_list.is_empty() && remove_list.is_empty() {
        anyhow::bail!("No fields provided to update");
    }

    let resolved = resolve_ticket_id(client, id_str).await?;

    let value = if body.is_empty() {
        // No PUT payload — just fetch current state for output after tag changes
        client
            .get(
                &format!("/api/v1/tickets/{resolved}"),
                Some(&[("expand", "true")]),
            )
            .await?
    } else {
        client
            .put(
                &format!("/api/v1/tickets/{resolved}"),
                Some(&Value::Object(body)),
            )
            .await?
    };

    if !add_list.is_empty() {
        apply_tags(client, resolved, &add_list, true).await?;
    }
    if !remove_list.is_empty() {
        apply_tags(client, resolved, &remove_list, false).await?;
    }

    if json {
        return output::emit_value(&value);
    }
    let ticket: Ticket = serde_json::from_value(value)?;
    output::print_ticket_detail(&ticket);
    Ok(())
}

/// Apply tag add/remove for a ticket. Zammad takes one item per request,
/// so calls are fired concurrently to amortize latency.
async fn apply_tags(
    client: &ZammadClient,
    ticket_id: i64,
    tags: &[String],
    add: bool,
) -> Result<()> {
    let op = if add { "add" } else { "remove" };
    let futs = tags
        .iter()
        .map(|t| tag_op(client, op, TICKET_OBJECT, ticket_id, t));
    try_join_all(futs).await?;
    Ok(())
}

async fn article_add(
    client: &ZammadClient,
    id_str: &str,
    body_text: String,
    subject: Option<String>,
    internal: bool,
    attachments: Option<String>,
    json: bool,
) -> Result<()> {
    let resolved = resolve_ticket_id(client, id_str).await?;
    let attachment_objs = build_attachments_opt(attachments.as_deref()).await?;

    let mut body: Map<String, Value> = Map::new();
    body.insert("ticket_id".into(), Value::Number(resolved.into()));
    body.insert("body".into(), Value::String(body_text));
    body.insert("type".into(), Value::String("note".into()));
    body.insert("internal".into(), Value::Bool(internal));
    insert_opt_str(&mut body, "subject", subject);
    if !attachment_objs.is_empty() {
        body.insert("attachments".into(), Value::Array(attachment_objs));
    }

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
    const PAGE_CAP: usize = 100;
    let states = ["new", "open", "pending reminder", "pending close", "closed"];

    let futs = states.iter().map(|s| async move {
        let params = vec![
            ("query", format!("state.name:{s}")),
            ("per_page", PAGE_CAP.to_string()),
        ];
        let value = client.get("/api/v1/tickets/search", Some(&params)).await?;
        let count = value.as_array().map(|a| a.len()).unwrap_or(0);
        Ok::<_, anyhow::Error>((s.to_string(), count))
    });
    let summary = try_join_all(futs).await?;
    let capped: Vec<&str> = summary
        .iter()
        .filter(|(_, n)| *n >= PAGE_CAP)
        .map(|(s, _)| s.as_str())
        .collect();

    if json {
        let mut map: Map<String, Value> = summary
            .iter()
            .map(|(s, n)| (s.clone(), Value::Number((*n as i64).into())))
            .collect();
        if !capped.is_empty() {
            map.insert(
                "_warning".into(),
                Value::String(format!(
                    "states at per_page cap ({PAGE_CAP}) — true count may be higher: {}",
                    capped.join(", ")
                )),
            );
        }
        return output::emit_value(&Value::Object(map));
    }

    use colored::Colorize;
    println!("{}", "Ticket Overview".bold().underline());
    for (s, n) in &summary {
        let suffix = if *n >= PAGE_CAP { "+" } else { "" };
        println!("  {:<20} {}{}", s, n.to_string().bold(), suffix.yellow());
    }
    if !capped.is_empty() {
        eprintln!(
            "{} states hit per_page cap ({}): {} — counts may undercount",
            "warning:".yellow().bold(),
            PAGE_CAP,
            capped.join(", ")
        );
    }
    Ok(())
}

/// Create or update a shared draft for a ticket.
///
/// PUT /api/v1/tickets/{ticket_id}/shared_draft
async fn shared_draft(
    client: &ZammadClient,
    id_str: &str,
    body_text: String,
    article_type: String,
    internal: bool,
    json: bool,
) -> Result<()> {
    let resolved = resolve_ticket_id(client, id_str).await?;
    let form_id = uuid::Uuid::new_v4().to_string();

    let payload = serde_json::json!({
        "form_id": form_id,
        "new_article": {
            "body": body_text,
            "content_type": "text/html",
            "type": article_type,
            "internal": internal,
        },
        "ticket_attributes": {},
    });

    let value = client
        .put(
            &format!("/api/v1/tickets/{resolved}/shared_draft"),
            Some(&payload),
        )
        .await?;

    if json {
        return output::emit_value(&value);
    }

    let draft_id = value
        .get("shared_draft_id")
        .and_then(|v| v.as_i64())
        .unwrap_or_default();
    let kind = if internal { "internal" } else { "public" };
    output::print_message(&format!(
        "Shared draft #{draft_id} saved on ticket {resolved} ({kind})"
    ));
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
