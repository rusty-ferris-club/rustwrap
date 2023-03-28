#![allow(clippy::use_self)]
#![allow(clippy::module_name_repetitions)]
use crate::console::style;
use decompress::{decompress, ExtractOpts};
use itertools::Itertools;
use serde::Deserialize;

use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use fs_err as fs;
use serde_json::json;

use crate::data::{Session, Target};

#[derive(Deserialize, Debug)]
pub struct PackageInfo {
    name: Option<String>,
    manifest: String,
    readme: Option<String>,
}
#[derive(Deserialize, Debug)]
pub struct NpmOpts {
    pub org: String,
    pub name: String,
    pub publish: bool,
    pub bin: Option<String>,
    pub root: PackageInfo,
    pub sub: PackageInfo,
}
impl NpmOpts {
    pub fn shim_name(&self) -> String {
        self.bin.as_ref().unwrap_or(&self.name).to_string()
    }
    pub fn root_package_name(&self) -> String {
        self.root.name.as_ref().unwrap_or(&self.name).to_string()
    }
}

const BIN_SHIM: &str = include_str!("static/npm/bin-shim");
const POSTINSTALL: &str = include_str!("static/npm/postinstall.js");
const PACKAGE_JSON: &str = "package.json";
const POSTINSTALL_JS: &str = "postinstall.js";
const INFO_JSON: &str = "info.json";

pub fn latest(opts: &NpmOpts) -> Result<semver::Version> {
    #[cfg(not(target_os = "windows"))]
    let npm = "npm";
    #[cfg(target_os = "windows")]
    let npm = "npm.exe";

    let out = duct::cmd!(npm, "view", opts.root_package_name(), "version").read()?;

    semver::Version::parse(&out).context("cannot parse version")
}

fn subpkg_name(target: &Target, opts: &NpmOpts) -> String {
    format!(
        "{}/{}-bin-{}",
        opts.org,
        opts.sub.name.as_ref().unwrap_or(&opts.name),
        target.tuple_slug()
    )
}

fn edit_files(hash: &mut serde_json::Map<String, serde_json::Value>, entries: &[&str]) {
    let files = hash.get("files").and_then(|fs| {
        fs.as_array().map(|v| {
            v.iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
        })
    });

    let files = files.map_or_else(
        || entries.into(),
        |mut files| {
            entries.iter().for_each(|e| files.push(e));
            files
        },
    );

    hash.insert(
        "files".to_string(),
        json!(files.iter().unique().collect::<Vec<_>>()),
    );
}

fn edit_subpkg(
    pkg: &serde_json::Value,
    version: &str,
    target: &Target,
    opts: &NpmOpts,
) -> serde_json::Value {
    let mut new = pkg.clone();
    let res = new.as_object_mut().expect("malformed json");
    res.insert("name".to_string(), json!(subpkg_name(target, opts)));
    res.insert("version".to_string(), json!(version.to_string()));
    res.insert("os".to_string(), json!([target.platform.to_string()]));
    res.insert("cpu".to_string(), json!([target.arch.to_string()]));

    let bin_name = target.bin_name(&opts.shim_name()).to_string();
    edit_files(res, &[bin_name.as_str()]);

    new
}

fn edit_rootpkg(
    pkg: &serde_json::Value,
    version: &str,
    targets: &[Target],
    opts: &NpmOpts,
) -> serde_json::Value {
    let mut new = pkg.clone();
    let res = new.as_object_mut().expect("malformed json");
    res.insert("name".to_string(), json!(opts.root_package_name()));
    res.insert("version".to_string(), json!(version.to_string()));
    let mut hsh = HashMap::new();
    for target in targets {
        hsh.insert(subpkg_name(target, opts), version.to_string());
    }
    res.insert("optionalDependencies".to_string(), json!(hsh));

    #[allow(clippy::option_if_let_else)] // clippy FP
    if let Some(scripts) = res.get_mut("scripts") {
        scripts
            .as_object_mut()
            .expect("malformed json")
            .insert("postinstall".into(), "node postinstall.js".into());
    } else {
        res.insert(
            "scripts".into(),
            json!({ "postinstall": "node postinstall.js"}),
        );
    }

    //ensure we have the generated bin and postinstall in `files`
    let bin_name = format!("bin/{}", opts.shim_name());
    res.insert("bin".into(), json!(bin_name));

    edit_files(res, &[POSTINSTALL_JS, INFO_JSON, &bin_name]);
    new
}

fn copy_readme(pkg_path: &Path, pkg: &PackageInfo) -> Result<()> {
    if let Some(readme) = &pkg.readme {
        fs::copy(
            readme,
            pkg_path.join(
                Path::new(readme)
                    .file_name()
                    .map_or("README.md".to_owned(), |f| f.to_string_lossy().to_string()),
            ),
        )?;
    };
    Ok(())
}

#[tracing::instrument(level = "trace", skip(session), err)]
pub fn publish(
    session: &mut Session<'_>,
    out_dir: &Path,
    version: &str,
    targets: &[Target],
    opts: &NpmOpts,
) -> Result<()> {
    let out_dir = out_dir.join(format!("{}-{version}", opts.name)).join("npm");
    let prefix = format!("{} {}", crate::console::PKG, style("npm").green());
    session.console.say(&format!(
        "{} generating into {}",
        prefix,
        style(&out_dir.to_string_lossy()).magenta()
    ));

    tracing::trace!("npm: generating into {:?}", out_dir);
    if !out_dir.exists() {
        fs::create_dir_all(&out_dir)?;
    }

    let subpkg_json: serde_json::Value =
        serde_json::from_reader(fs::File::open(&opts.sub.manifest)?)
            .with_context(|| format!("could not read {}", &opts.sub.manifest))?;
    for target in targets {
        let pkg_name = subpkg_name(target, opts);
        let subpkg_path = out_dir.join(&pkg_name);
        tracing::trace!("npm: creating subpackage in {:?}", subpkg_path);
        if !subpkg_path.exists() {
            fs::create_dir_all(&subpkg_path)?;
        }

        let subpkg = edit_subpkg(&subpkg_json, version, target, opts);
        serde_json::to_writer_pretty(fs::File::create(subpkg_path.join(PACKAGE_JSON))?, &subpkg)?;

        // copy readme
        copy_readme(&subpkg_path, &opts.sub)?;

        if let Some(archive) = &target.archive {
            tracing::trace!("npm: decompressing archive into {:?}", subpkg_path);
            decompress(
                Path::new(archive),
                subpkg_path.as_path(),
                &ExtractOpts { strip: 1 },
            )?;
        }
        session.console.say(&format!(
            "   {} {}",
            style("subpackage").yellow(),
            &pkg_name,
        ));
        if opts.publish {
            let out = duct::cmd!("npm", "publish").dir(&subpkg_path).read()?;
            session.console.say(&format!(
                "   {} {} published:\n{}",
                style("subpackage").yellow(),
                &pkg_name,
                &out,
            ));
        }
        tracing::trace!("npm: done");
    }

    let rootpkg_path = out_dir.join(&opts.name);

    // package.json
    tracing::trace!("npm: creating root package in {:?}", rootpkg_path);
    if !rootpkg_path.exists() {
        fs::create_dir_all(&rootpkg_path)?;
    }
    let rootpkg_json: serde_json::Value =
        serde_json::from_reader(fs::File::open(&opts.root.manifest)?)?;
    let rootpkg = edit_rootpkg(&rootpkg_json, version, targets, opts);
    serde_json::to_writer_pretty(fs::File::create(rootpkg_path.join(PACKAGE_JSON))?, &rootpkg)?;

    // copy readme
    copy_readme(&rootpkg_path, &opts.root)?;

    session.console.say(&format!(
        "   {}    {}",
        style("package").yellow(),
        &opts.name
    ));
    tracing::trace!("npm: wrote package.");

    // create bin, +x it
    let bin_path = rootpkg_path.join("bin");
    if !bin_path.exists() {
        fs::create_dir_all(&bin_path)?;
    }
    let bin_fname = bin_path.join(opts.shim_name());
    fs::write(&bin_fname, BIN_SHIM)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&bin_fname, std::fs::Permissions::from_mode(0o755))?;
    }

    // postinstall.js
    fs::write(rootpkg_path.join(POSTINSTALL_JS), POSTINSTALL)?;
    tracing::trace!("npm: wrote binary and postinstall.");

    // info.json
    fs::write(
        rootpkg_path.join(INFO_JSON),
        serde_json::to_string_pretty(&json!({
            "platforms": &targets.iter().map(|t| json!({
              "platform": t.platform,
              "arch": t.arch,
              "bin":format!("{}/{}",  subpkg_name(t, opts),  t.bin_name(&opts.shim_name()))
            })).collect::<Vec<_>>(),
            "name": opts.name,
        }))?,
    )?;
    tracing::trace!("npm: wrote info.json.");

    if opts.publish {
        let out = duct::cmd!("npm", "publish").dir(&rootpkg_path).read()?;
        session.console.say(&format!(
            "   {}    {} published:\n{}",
            style("package").yellow(),
            &opts.name,
            &out,
        ));
    }
    session.console.say(&format!("{prefix} done."));
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        console::MemConsole,
        data::{Architecture, Config, Platform},
    };

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_tuples() {
        assert_eq!(
            "linux-arm64",
            Target {
                arch: Architecture::ARM64,
                platform: Platform::Linux,
                ..Default::default()
            }
            .tuple_slug()
        );
        assert_eq!(
            "recon",
            Target {
                arch: Architecture::ARM64,
                platform: Platform::Linux,
                ..Default::default()
            }
            .bin_name("recon")
        );
        assert_eq!(
            "recon.exe",
            Target {
                arch: Architecture::ARM64,
                platform: Platform::Win32,
                ..Default::default()
            }
            .bin_name("recon")
        );
    }

    #[test]
    fn test_generate() {
        let mut session = Session {
            config: &Config::default(),
            console: &mut MemConsole::default(),
        };
        publish(
            &mut session,
            Path::new("out/test_generate"),
            "1.0.1",
            &[
                Target {
                    platform: Platform::Darwin,
                    arch: Architecture::ARM64,
                    ..Default::default()
                },
                Target {
                    platform: Platform::Win32,
                    arch: Architecture::X64,
                    ..Default::default()
                },
            ],
            &NpmOpts {
                org: "@recontools".to_owned(),
                name: "recon".to_owned(),
                publish: false,
                bin: None,
                root: PackageInfo {
                    name: None,
                    manifest: "fixtures/config/recon-root.json".to_owned(),
                    readme: None,
                },
                sub: PackageInfo {
                    name: None,
                    manifest: "fixtures/config/recon-sub.json".to_owned(),
                    readme: None,
                },
            },
        )
        .unwrap();
    }

    #[test]
    fn test_latest_version() {
        let v = latest(&NpmOpts {
            org: "foo".to_string(),
            name: "react".to_string(),
            publish: false,
            bin: None,
            root: PackageInfo {
                name: None,
                manifest: String::new(),
                readme: None,
            },
            sub: PackageInfo {
                name: None,
                manifest: String::new(),
                readme: None,
            },
        })
        .unwrap();
        assert!(v > semver::Version::parse("18.0.0").unwrap());
    }
}
