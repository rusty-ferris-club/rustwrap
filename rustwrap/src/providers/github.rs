use std::env;

use anyhow::{bail, Context, Result};
use reqwest::header;
use tracing::info;

pub fn put(url: &str, val: &serde_json::Value) -> Result<reqwest::blocking::Response> {
    let client = reqwest::blocking::Client::new();
    let res = client.put(url).headers(api_headers()?).json(val).send()?;

    info!("put response: {}", res.status());

    Ok(res)
}

pub fn get(url: &str) -> Result<reqwest::blocking::Response> {
    let client = reqwest::blocking::Client::new();
    let res = client.get(url).headers(api_headers()?).send()?;

    info!("get response: {}", res.status());

    Ok(res)
}

pub fn api_headers() -> Result<header::HeaderMap> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        "rust-reqwest/self-update"
            .parse()
            .expect("github invalid user-agent"),
    );

    let auth_token = env::var("GITHUB_TOKEN").context("github token not found in 'GITHUB_TOKEN'");
    if let Ok(token) = auth_token {
        headers.insert(
            header::AUTHORIZATION,
            format!("token {token}")
                .parse()
                .map_err(|err| anyhow::format_err!("Failed to parse auth token: {}", err))?,
        );
    };

    Ok(headers)
}
pub fn latest(repo: &str) -> Result<semver::Version> {
    let api_url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let resp = get(&api_url)?;
    if !resp.status().is_success() {
        bail!(
            "api request failed with status: {:?} - for: {:?}",
            resp.status(),
            api_url
        )
    }
    let json = resp.json::<serde_json::Value>()?;
    let tag = json["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::format_err!("Release missing `tag_name`"))?;
    semver::Version::parse(tag.trim_start_matches('v')).context("cannot parse version")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latest_version() {
        let v = latest("jondot/makeme").unwrap();
        assert!(v > semver::Version::parse("0.0.1").unwrap());
    }
}
