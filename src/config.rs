use anyhow::{bail, Result};

pub struct Config {
    pub url: String,
    pub token: String,
}

pub fn load() -> Result<Config> {
    let url = std::env::var("ZAMMAD_URL").ok().filter(|s| !s.is_empty());
    let token = std::env::var("ZAMMAD_TOKEN").ok().filter(|s| !s.is_empty());
    match (url, token) {
        (Some(url), Some(token)) => Ok(Config { url, token }),
        _ => bail!(
            "ZAMMAD_URL and ZAMMAD_TOKEN must be set.\n\
             Example: export ZAMMAD_URL=https://support.example.com\n\
                      export ZAMMAD_TOKEN=your-api-token"
        ),
    }
}
