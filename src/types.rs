use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub id: i64,
    #[serde(default)]
    pub number: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub customer: Option<String>,
    #[serde(default)]
    pub organization: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub article_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub id: i64,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default, rename = "from")]
    pub from_addr: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub internal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub member_ids: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    #[serde(default)]
    pub firstname: String,
    #[serde(default)]
    pub lastname: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub organization: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedItem {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub active: bool,
}
