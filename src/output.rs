use anyhow::Result;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, ContentArrangement, Table};
use serde::Serialize;
use serde_json::Value;

use crate::types::{Article, NamedItem, Organization, Ticket, User};
use crate::util::truncate;

pub fn emit_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn emit_value(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn render<T: Serialize>(value: &T, json: bool, human: impl FnOnce(&T)) -> Result<()> {
    if json {
        emit_json(value)
    } else {
        human(value);
        Ok(())
    }
}

pub fn print_message(msg: &str) {
    println!("{} {}", "→".bold().green(), msg);
}

pub fn print_ticket_table(tickets: &[Ticket]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            "ID", "Number", "Title", "State", "Priority", "Group", "Owner", "Customer",
        ]);
    for t in tickets {
        table.add_row(vec![
            Cell::new(t.id),
            Cell::new(&t.number),
            Cell::new(truncate(&t.title, 60)),
            Cell::new(t.state.as_deref().unwrap_or("-")),
            Cell::new(t.priority.as_deref().unwrap_or("-")),
            Cell::new(truncate(t.group.as_deref().unwrap_or("-"), 20)),
            Cell::new(truncate(t.owner.as_deref().unwrap_or("-"), 25)),
            Cell::new(truncate(t.customer.as_deref().unwrap_or("-"), 25)),
        ]);
    }
    println!("{table}");
    println!(
        "{} {}",
        tickets.len().to_string().bold(),
        "tickets".dimmed()
    );
}

pub fn print_ticket_detail(t: &Ticket) {
    let num = if t.number.is_empty() {
        "?"
    } else {
        t.number.as_str()
    };
    println!(
        "{} #{} {}",
        "Ticket".bold().underline(),
        num.cyan(),
        format!("(ID:{})", t.id).dimmed()
    );
    println!("  {} {}", "Title:".bold(), t.title);
    println!(
        "  {} {}",
        "State:".bold(),
        t.state.as_deref().unwrap_or("-")
    );
    println!(
        "  {} {}",
        "Priority:".bold(),
        t.priority.as_deref().unwrap_or("-")
    );
    println!(
        "  {} {}",
        "Group:".bold(),
        t.group.as_deref().unwrap_or("-")
    );
    println!(
        "  {} {}",
        "Owner:".bold(),
        t.owner.as_deref().unwrap_or("-")
    );
    println!(
        "  {} {}",
        "Customer:".bold(),
        t.customer.as_deref().unwrap_or("-")
    );
    println!(
        "  {} {}",
        "Organization:".bold(),
        t.organization.as_deref().unwrap_or("-")
    );
    if let Some(c) = &t.created_at {
        println!("  {} {}", "Created:".bold(), c);
    }
    if let Some(u) = &t.updated_at {
        println!("  {} {}", "Updated:".bold(), u);
    }
    if let Some(n) = t.article_count {
        println!("  {} {}", "Articles:".bold(), n);
    }
}

pub fn print_articles(articles: &[Article]) {
    if articles.is_empty() {
        print_message("No articles");
        return;
    }
    for a in articles {
        let header = format!(
            "[{}] (article {}) {} - {}",
            a.created_at.as_deref().unwrap_or("?"),
            a.id,
            a.sender.as_deref().unwrap_or("?"),
            a.from_addr.as_deref().unwrap_or("?")
        );
        let tag = if a.internal {
            "internal".yellow().to_string()
        } else {
            "public".green().to_string()
        };
        println!("{} [{}]", header.bold(), tag);
        if let Some(s) = a.subject.as_deref().filter(|s| !s.is_empty()) {
            println!("  {} {}", "Subject:".bold(), s);
        }
        for line in a.body.lines() {
            println!("  {line}");
        }
        if !a.attachments.is_empty() {
            println!("  {}", "Attachments:".bold());
            for att in &a.attachments {
                let size = att.size.as_deref().unwrap_or("?");
                let id = att.id.map(|i| i.to_string()).unwrap_or_else(|| "?".into());
                println!("    - [{id}] {} ({size} bytes)", att.filename);
            }
        }
        println!("{}", "---".dimmed());
    }
    println!(
        "{} {}",
        articles.len().to_string().bold(),
        "articles".dimmed()
    );
}

pub fn print_org_table(orgs: &[Organization]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec!["ID", "Name", "Active", "Members"]);
    for o in orgs {
        let members = o.member_ids.as_ref().map(|m| m.len()).unwrap_or(0);
        table.add_row(vec![
            Cell::new(o.id),
            Cell::new(&o.name),
            Cell::new(if o.active { "✓" } else { "" }),
            Cell::new(members),
        ]);
    }
    println!("{table}");
    println!("{} {}", orgs.len().to_string().bold(), "orgs".dimmed());
}

pub fn print_user_table(users: &[User]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["ID", "Name", "Email", "Phone", "Organization"]);
    for u in users {
        let name = format!("{} {}", u.firstname, u.lastname).trim().to_string();
        table.add_row(vec![
            Cell::new(u.id),
            Cell::new(if name.is_empty() { "-".into() } else { name }),
            Cell::new(&u.email),
            Cell::new(u.phone.as_deref().unwrap_or("-")),
            Cell::new(u.organization.as_deref().unwrap_or("-")),
        ]);
    }
    println!("{table}");
    println!("{} {}", users.len().to_string().bold(), "users".dimmed());
}

pub fn print_named_table(items: &[NamedItem], label: &str) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec!["ID", "Name", "Active"]);
    for i in items {
        table.add_row(vec![
            Cell::new(i.id),
            Cell::new(&i.name),
            Cell::new(if i.active { "✓" } else { "" }),
        ]);
    }
    println!("{table}");
    println!("{} {}", items.len().to_string().bold(), label.dimmed());
}
