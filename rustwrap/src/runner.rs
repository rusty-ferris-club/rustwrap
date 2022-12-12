use std::path::Path;

use crate::console::EnvConsole;
use crate::data::{Config, Session};
use crate::download::TargetsDownloader;
use crate::providers::brew;
use crate::providers::npm;
use anyhow::Result;

/// Run a wrap workflow
///
/// # Errors
///
/// This function will return an error if an IO failed
pub fn run(version: &str, config_file: &Path, out_path: &Path) -> Result<()> {
    let config = Config::load(config_file)?;
    let mut session = Session {
        config: &config,
        console: &mut EnvConsole {},
    };

    let releases_path = out_path.join("releases");
    let downloader = TargetsDownloader::new(&config.targets, &releases_path);
    let versioned_targets = downloader.download(&mut session, version)?;

    if let Some(npm) = config.npm.as_ref() {
        npm::publish(&mut session, out_path, version, &versioned_targets, npm)?;
    }
    if let Some(brew) = config.brew.as_ref() {
        brew::publish(&mut session, out_path, version, &versioned_targets, brew)?;
    }
    Ok(())
}
