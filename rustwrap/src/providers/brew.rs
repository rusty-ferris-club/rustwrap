#![allow(clippy::module_name_repetitions)]
use regex::Regex;
use std::{
    io::{self, Read},
    path::Path,
};

use crate::{
    console::style,
    providers::github::{get, put},
};
use anyhow::{anyhow, bail, Result};
use fs_err as fs;
use serde::Deserialize;
use serde_json::json;
use sha2::Digest;

use crate::data::{Architecture, Platform, Session, Target};

const VAR_URL: &str = "__URL__";
const VAR_SHA: &str = "__SHA__";
const VAR_VERSION: &str = "__VERSION__";

#[derive(Deserialize)]
pub struct BrewOpts {
    pub name: String,
    pub tap: String,
    pub recipe_fname: Option<String>,
    pub recipe_template: String,
    pub publish: bool,
}

impl BrewOpts {
    fn validate(&self) -> Result<()> {
        match (
            self.recipe_template.contains(VAR_URL),
            self.recipe_template.contains(VAR_SHA),
            self.recipe_template.contains(VAR_VERSION),
        ) {
            (false, _, _) => bail!("missing URL variable"),
            (_, false, _) => bail!("missing SHA variable"),
            (_, _, false) => bail!("missing VERSION variable"),
            (_, _, _) => {}
        }
        Ok(())
    }

    fn recipe(&self, version: &str, url: &str, sha: &str) -> String {
        self.recipe_template
            .replace(VAR_VERSION, version)
            .replace(VAR_URL, url)
            .replace(VAR_SHA, sha)
    }

    fn recipe_file(&self) -> String {
        self.recipe_fname
            .clone()
            .unwrap_or_else(|| format!("{}.rb", self.name))
    }
}

pub fn latest(opts: &BrewOpts) -> Result<semver::Version> {
    let remote_file = format!(
        "https://api.github.com/repos/{}/contents/{}",
        opts.tap,
        opts.recipe_file()
    );
    let resp: serde_json::Value = get(&remote_file)?.json()?;
    let content = resp
        .pointer("/content")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| base64::decode(s.replace('\n', "")).ok())
        .and_then(|d| String::from_utf8(d).ok());
    let v = match content {
        Some(c) => {
            let re = Regex::new("version \"(.*)\"").unwrap();
            let caps = re.captures(c.as_ref());
            caps.and_then(|cs| cs.get(1))
                .map(|cap| cap.as_str().to_string())
        }
        None => {
            bail!("no content found at {remote_file}")
        }
    };

    v.and_then(|v| semver::Version::parse(&v).ok())
        .ok_or_else(|| anyhow::format_err!("cannot find version at {remote_file}"))
}

pub fn publish(
    session: &mut Session<'_>,
    out_dir: &Path,
    version: &str,
    targets: &[Target],
    opts: &BrewOpts,
) -> Result<()> {
    let out_dir = out_dir
        .join(format!("{}-{version}", opts.name))
        .join("brew");
    fs::create_dir_all(&out_dir)?;

    let prefix = format!("{} {}", crate::console::COFFEE, style("brew").green());
    session.console.say(&format!(
        "{} generating into {}",
        prefix,
        style(&out_dir.to_string_lossy()).magenta()
    ));

    opts.validate()?;

    let target = targets
        .iter()
        .find(|t| t.arch == Architecture::X64 && t.platform == Platform::Darwin)
        .ok_or_else(|| anyhow::anyhow!("no Intel macOS compatible target found"))?;

    //
    // prep: file name, template, hash, and render the url incl. version
    // then, render the recipe file (ruby source)
    //
    let fname = target
        .archive
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("archive '{:?}' was not found", target))?;

    let mut file = fs::File::open(fname)?;

    let mut hasher = sha2::Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    let hash = hasher.finalize();
    let sha = format!("{hash:x}");

    let recipe = opts.recipe(version, &target.url(version), &sha);
    tracing::info!(recipe, "rendered recipe");

    //
    // post the rendered file to github
    //
    let fname = opts.recipe_file();

    if opts.publish {
        let remote_file = format!("https://api.github.com/repos/{}/contents/{fname}", opts.tap);
        let resp = get(&remote_file)?;

        let sha = match resp.status() {
            reqwest::StatusCode::OK => match resp.json::<serde_json::Value>() {
                Ok(parsed) => parsed
                    .pointer("/sha")
                    .ok_or_else(|| anyhow!("no `sha` in response"))?
                    .as_str()
                    .map(std::string::ToString::to_string),
                Err(_) => None,
            },
            _ => None,
        };

        let mut res = put(
            &remote_file,
            &json!({"message": format!("rustwrap update: {fname}"), "content": base64::encode(&recipe), "sha": sha}),
        )?;

        if !res.status().is_success() {
            let mut response_body = String::new();
            res.read_to_string(&mut response_body)?;
            tracing::info!(response_body, "response");
            bail!("{} publishing with status: {:?}", prefix, res.status());
        }

        //
        // we're done
        //
        session.console.say(&format!(
            "{} published '{}' in '{}'",
            prefix,
            style(&fname).magenta(),
            style(&opts.tap).magenta()
        ));
    }

    //
    // save rendered file to disk
    //
    let dest_file = out_dir.join(&fname);
    fs::write(&dest_file, recipe)?;
    session.console.say(&format!(
        "{} saved recipe to '{}'",
        prefix,
        style(&dest_file.to_string_lossy()).magenta(),
    ));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latest_version() {
        let v = latest(&BrewOpts {
            name: "mm".to_string(),
            tap: "jondot/homebrew-tap".to_string(),
            recipe_fname: None,
            recipe_template: String::new(),
            publish: false,
        })
        .unwrap();
        assert!(v > semver::Version::parse("0.0.1").unwrap());
    }
}
