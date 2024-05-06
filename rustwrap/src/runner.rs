use std::path::Path;

use crate::console::EnvConsole;
use crate::data::{Config, Session};
use crate::download::TargetsDownloader;
use crate::providers::npm;
use crate::providers::{brew, github};
use anyhow::{bail, Result};
use console::style;

/// Run a wrap workflow
///
/// # Errors
///
/// This function will return an error if an IO failed
pub fn run(version: Option<String>, config_file: &Path, out_path: &Path) -> Result<()> {
    let config = Config::load(config_file)?;
    let mut session = Session {
        config: &config,
        console: &mut EnvConsole {},
    };

    let target_v = if let Some(version) = version {
        semver::Version::parse(&version)?
    } else {
        session.console.say(&format!(
            "{} no tag given, discovering latest from github releases",
            crate::console::INFO
        ));
        let v = if let Some(repo) = config.repo.as_ref() {
            let discovered_v = github::latest(repo)?;
            session.console.say(&format!(
                "{} discovered: {discovered_v}",
                crate::console::INFO
            ));
            discovered_v
        } else {
            bail!("set tag with -t or repo in your configuration for auto discovery");
        };
        v
    };

    let releases_path = out_path.join("releases");
    let downloader = TargetsDownloader::new(&config.targets, &releases_path);
    let versioned_targets = downloader.download(&mut session, &target_v.to_string())?;

    if let Some(npm) = config.npm.as_ref() {
        let prefix = format!("{} {}", crate::console::PKG, style("brew").green());
        let latest_v = npm::latest(npm)?;
        if latest_v < target_v {
            session.console.say(&format!(
                "{prefix} current: {latest_v}, publishing: {target_v}..."
            ));
            npm::publish(
                &mut session,
                out_path,
                &target_v.to_string(),
                &versioned_targets,
                npm,
            )?;
        } else {
            session
                .console
                .say(&format!("{prefix} discovered version ({latest_v}) higher/equal to target version ({target_v}), skipping."));
        }
    }

    if let Some(brew) = config.brew.as_ref() {
        let prefix = format!("{} {}", crate::console::COFFEE, style("brew").green());
        if brew.publish {
            let latest_v = brew::latest(brew)?;
            if latest_v < target_v {
                bail!("current latest version is newer, aborting publish")
            }
            session.console.say(&format!(
                "{prefix} current: {latest_v}, publishing: {target_v}..."
            ));
        }
        brew::publish(
            &mut session,
            out_path,
            &target_v.to_string(),
            &versioned_targets,
            brew,
        )?;
    }
    Ok(())
}
