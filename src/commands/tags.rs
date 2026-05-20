use anyhow::Result;
use clap::Subcommand;

use crate::client::ZammadClient;
use crate::output;

pub const TICKET_OBJECT: &str = "Ticket";

#[derive(Subcommand)]
pub enum TagsCmd {
    /// List tags attached to an object (e.g. `tags list --object Ticket --id 42`)
    List {
        #[arg(long, default_value = TICKET_OBJECT)]
        object: String,
        #[arg(long)]
        id: i64,
    },
    /// Add a tag to an object
    Add {
        #[arg(long, default_value = TICKET_OBJECT)]
        object: String,
        #[arg(long)]
        id: i64,
        #[arg(long)]
        name: String,
    },
    /// Remove a tag from an object
    Remove {
        #[arg(long, default_value = TICKET_OBJECT)]
        object: String,
        #[arg(long)]
        id: i64,
        #[arg(long)]
        name: String,
    },
}

pub async fn run(cmd: TagsCmd, client: &ZammadClient, json: bool) -> Result<()> {
    match cmd {
        TagsCmd::List { object, id } => list(client, &object, id, json).await,
        TagsCmd::Add { object, id, name } => {
            tag_op(client, "add", &object, id, &name).await?;
            if json {
                output::emit_value(&serde_json::json!({"ok": true, "op": "add", "tag": name}))
            } else {
                output::print_message(&format!("Tag '{name}' added on {object} #{id}"));
                Ok(())
            }
        }
        TagsCmd::Remove { object, id, name } => {
            tag_op(client, "remove", &object, id, &name).await?;
            if json {
                output::emit_value(&serde_json::json!({"ok": true, "op": "remove", "tag": name}))
            } else {
                output::print_message(&format!("Tag '{name}' removed on {object} #{id}"));
                Ok(())
            }
        }
    }
}

/// Single tag add/remove call. Zammad takes one `item` per request and
/// requires DELETE (not POST) for `/tags/remove`.
pub async fn tag_op(
    client: &ZammadClient,
    op: &str,
    object: &str,
    id: i64,
    name: &str,
) -> Result<()> {
    let path = format!("/api/v1/tags/{op}");
    let payload = serde_json::json!({
        "object": object,
        "o_id": id,
        "item": name,
    });
    match op {
        "add" => {
            client.post(&path, Some(&payload)).await?;
        }
        "remove" => {
            client.delete(&path, Some(&payload)).await?;
        }
        _ => anyhow::bail!("Unknown tag op: {op}"),
    }
    Ok(())
}

async fn list(client: &ZammadClient, object: &str, id: i64, json: bool) -> Result<()> {
    let params = vec![("object", object.to_string()), ("o_id", id.to_string())];
    let value = client.get("/api/v1/tags", Some(&params)).await?;
    if json {
        return output::emit_value(&value);
    }
    let tags = value
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if tags.is_empty() {
        output::print_message(&format!("No tags on {object} #{id}"));
    } else {
        for t in &tags {
            println!("  {t}");
        }
        output::print_message(&format!("{} tags", tags.len()));
    }
    Ok(())
}
