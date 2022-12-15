#![allow(clippy::module_name_repetitions)]
use std::{
    env,
    io::{self, Read},
    path::Path,
};

use crate::console::style;
use anyhow::{bail, Context, Result};
use fs_err as fs;
use reqwest::header;
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
    let fname = opts
        .recipe_fname
        .clone()
        .unwrap_or_else(|| format!("{}.rb", opts.name));

    if opts.publish {
        let client = reqwest::blocking::Client::new();
        let mut res = client
            .put(format!(
                "https://api.github.com/repos/{}/contents/{fname}",
                opts.tap
            ))
            .header(header::USER_AGENT, "rust-reqwest/rustwrap")
            .json(&json!({"message": format!("rustwrap update: {fname}"), "content": base64::encode(&recipe)}))
            .bearer_auth(
                env::var("GITHUB_TOKEN").context("github token not found in 'GITHUB_TOKEN'")?,
            )
            .send()?;

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
