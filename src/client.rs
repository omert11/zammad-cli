use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client, Method, Response, StatusCode};
use serde::Serialize;
use serde_json::Value;

use crate::config::Config;

const UA: &str = concat!("zammad-cli/", env!("CARGO_PKG_VERSION"));

pub struct ZammadClient {
    http: Client,
    base_url: String,
}

impl ZammadClient {
    pub fn new(cfg: &Config) -> Result<Self> {
        let mut headers = HeaderMap::new();
        let auth = format!("Token token={}", cfg.token);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth).context("Invalid token characters")?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        // Explicit UA — bypasses Cloudflare WAF default-UA blocks (error 1010)
        headers.insert(USER_AGENT, HeaderValue::from_static(UA));
        let http = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self {
            http,
            base_url: cfg.url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn request<Q: Serialize + ?Sized, B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        query: Option<&Q>,
        body: Option<&B>,
    ) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.request(method, &url);
        if let Some(q) = query {
            req = req.query(q);
        }
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await.context("Zammad request failed")?;
        handle_response(resp).await
    }

    pub async fn get<Q: Serialize + ?Sized>(&self, path: &str, query: Option<&Q>) -> Result<Value> {
        self.request::<Q, ()>(Method::GET, path, query, None).await
    }

    pub async fn post<B: Serialize + ?Sized>(&self, path: &str, body: Option<&B>) -> Result<Value> {
        self.request::<(), B>(Method::POST, path, None, body).await
    }

    pub async fn put<B: Serialize + ?Sized>(&self, path: &str, body: Option<&B>) -> Result<Value> {
        self.request::<(), B>(Method::PUT, path, None, body).await
    }

    pub async fn delete<B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: Option<&B>,
    ) -> Result<Value> {
        self.request::<(), B>(Method::DELETE, path, None, body)
            .await
    }
}

async fn handle_response(resp: Response) -> Result<Value> {
    let status = resp.status();
    if status.is_success() {
        if status == StatusCode::NO_CONTENT {
            return Ok(Value::Null);
        }
        let text = resp.text().await.context("Failed to read response body")?;
        if text.is_empty() {
            return Ok(Value::Null);
        }
        return serde_json::from_str(&text).context("Failed to parse response JSON");
    }

    let body = resp.text().await.unwrap_or_default();
    let parsed = serde_json::from_str::<Value>(&body).ok();
    let msg = parsed
        .as_ref()
        .and_then(|v| {
            v.get("error_human")
                .or_else(|| v.get("error"))
                .or_else(|| v.get("message"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| body.clone());

    let prefix = match status {
        StatusCode::NOT_FOUND => "Not found (404)",
        StatusCode::FORBIDDEN => "Permission denied (403)",
        StatusCode::BAD_REQUEST => "Bad request (400)",
        StatusCode::UNAUTHORIZED => "Unauthorized (401) — check ZAMMAD_TOKEN",
        StatusCode::UNPROCESSABLE_ENTITY => "Unprocessable (422)",
        _ => "Zammad API error",
    };
    if !body.is_empty() && body.trim() != msg.trim() {
        Err(anyhow!("{prefix}: {msg}\n--- raw: {body}"))
    } else {
        Err(anyhow!("{prefix}: {msg}"))
    }
}
