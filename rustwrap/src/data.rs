#![allow(clippy::use_self)]
use anyhow::Result;
use fs_err as fs;
use serde::{Deserialize, Serialize};
use serde_variant::to_variant_name;
use std::{borrow::Cow, fmt::Display, path::Path};

use crate::{
    console::Console,
    providers::{brew::BrewOpts, npm::NpmOpts},
};

#[derive(Default, Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub enum Platform {
    #[default]
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "win32")]
    Win32,
    #[serde(rename = "darwin")]
    Darwin,
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(to_variant_name(self).unwrap())?;
        Ok(())
    }
}

#[derive(Default, Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub enum Architecture {
    #[default]
    #[serde(rename = "x64")]
    X64,
    #[serde(rename = "arm64")]
    ARM64,
}

impl Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(to_variant_name(self).unwrap())?;
        Ok(())
    }
}

#[derive(Default, Deserialize, Serialize, Clone, Debug)]
pub struct Target {
    pub platform: Platform,
    pub arch: Architecture,
    pub url_template: String,
    pub bin_name: Option<String>,
    pub archive: Option<String>,
}

impl Target {
    pub fn tuple_slug(&self) -> String {
        format!(
            "{}-{}",
            to_variant_name(&self.platform).unwrap(),
            to_variant_name(&self.arch).unwrap()
        )
    }
    pub fn bin_name<'a>(&self, name: &'a str) -> Cow<'a, str> {
        match self.platform {
            Platform::Win32 => format!("{}.exe", name).into(),
            _ => name.into(),
        }
    }
    pub fn url(&self, version: &str) -> String {
        self.url_template.replace("__VERSION__", version)
    }
}

#[derive(Deserialize, Default)]
pub struct Config {
    pub targets: Vec<Target>,
    pub npm: Option<NpmOpts>,
    pub brew: Option<BrewOpts>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(file: P) -> Result<Self> {
        let r: Config = serde_yaml::from_reader(fs::File::open(file.as_ref())?)?;
        Ok(r)
    }
}

pub struct Session<'a> {
    pub config: &'a Config,
    pub console: &'a mut dyn Console,
}
