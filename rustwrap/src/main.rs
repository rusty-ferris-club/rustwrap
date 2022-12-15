#![allow(clippy::must_use_candidate)]

use rustwrap::runner;

use anyhow::Result as AnyResult;
use clap::{crate_version, ArgAction};
use clap::{Arg, ArgMatches, Command};
use std::path::Path;
use std::process::exit;
use tracing::metadata::LevelFilter;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

pub const BANNER: &str = r#"
    B A N N E R
"#;

pub fn command() -> Command {
    Command::new("rustwrap")
        .version(crate_version!())
        .about("Wrap Rust releases for various package registries")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("CONFIG_FILE")
                .default_value("rustwrap.yaml")
                .help("Point to a configuration YAML"),
        )
        .arg(
            Arg::new("out")
                .short('o')
                .long("out")
                .value_name("OUT_DIR")
                .default_value("dist")
                .help("Output directory"),
        )
        .arg(
            Arg::new("tag")
                .short('t')
                .long("tag")
                .value_name("VERSION_TAG")
                .help("Version tag to package (e.g. '1.0.1')"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .help("Show details about interactions")
                .action(ArgAction::SetTrue),
        )
}

/// Run
///
/// # Errors
///
/// This function will return an error
fn run(matches: &ArgMatches) -> AnyResult<bool> {
    let out_path = matches.get_one::<String>("out");
    let config_file = matches.get_one::<String>("config");
    let version = matches
        .get_one::<String>("tag")
        .ok_or_else(|| anyhow::anyhow!("no version tag given. please supply one with `--tag`"))?;

    runner::run(
        version,
        Path::new(config_file.expect("no config")),
        Path::new(out_path.expect("no path")),
    )?;
    Ok(true)
}

fn main() {
    let app = command();

    let _v = app.render_version();
    let matches = app.get_matches();

    let level = if matches.get_flag("verbose") {
        LevelFilter::INFO
    } else {
        LevelFilter::OFF
    };

    Registry::default()
        .with(tracing_tree::HierarchicalLayer::new(2))
        .with(
            EnvFilter::builder()
                .with_default_directive(level.into())
                .with_env_var("LOG")
                .from_env_lossy(),
        )
        .init();

    // actual logic is in 'run'.
    // subcommand is an error, but you can swap it later if you bring in subcommands
    let res = match matches.subcommand() {
        None => run(&matches),
        _ => Ok(false),
    };

    match res {
        Ok(ok) => {
            exit(i32::from(!ok));
        }
        Err(err) => {
            eprintln!("error: {err}");
            exit(1)
        }
    }
}
