use crate::console::{style, DOWNLOAD, FINGER};
use anyhow::{bail, Context, Result};
use fs_err as fs;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header;
use std::io::{self, Read, Write};
use std::path::Path;

use crate::data::{Session, Target};

pub const DEFAULT_PROGRESS_TEMPLATE: &str = "   {prefix:} {bar:.green/red} {msg}";

#[derive(Debug)]
pub struct Download {
    show_progress: bool,
    url: String,
    headers: reqwest::header::HeaderMap,
}

#[allow(dead_code)]
impl Download {
    /// Specify download url
    pub fn from_url(url: &str) -> Self {
        Self {
            show_progress: false,
            url: url.to_owned(),
            headers: reqwest::header::HeaderMap::new(),
        }
    }

    pub fn show_progress(&mut self, b: bool) -> &mut Self {
        self.show_progress = b;
        self
    }

    pub fn set_headers(&mut self, headers: reqwest::header::HeaderMap) -> &mut Self {
        self.headers = headers;
        self
    }

    pub fn set_header(
        &mut self,
        name: reqwest::header::HeaderName,
        value: reqwest::header::HeaderValue,
    ) -> &mut Self {
        self.headers.insert(name, value);
        self
    }

    pub fn filename_from_content_disposition(
        value: &'_ reqwest::header::HeaderValue,
    ) -> Result<&'_ str> {
        let regex = regex::Regex::new(r#"filename="?([^"]*)"?"#)?;
        let capture = regex
            .captures(value.to_str()?)
            .context("Field 'filename' not present in the header value.")?
            .get(1)
            .context("Missing capture group from regex.")?;
        Ok(capture.as_str())
    }

    pub fn download_to(&self, out_dir: &Path) -> Result<String> {
        let mut headers = self.headers.clone();
        if !headers.contains_key(header::USER_AGENT) {
            headers.insert(
                header::USER_AGENT,
                "rust-reqwest/rustwrap".parse().expect("invalid user-agent"),
            );
        }

        #[cfg(target_os = "linux")]
        {
            if ::std::env::var_os("SSL_CERT_FILE").is_none() {
                ::std::env::set_var("SSL_CERT_FILE", "/etc/ssl/certs/ca-certificates.crt");
            }
            if ::std::env::var_os("SSL_CERT_DIR").is_none() {
                ::std::env::set_var("SSL_CERT_DIR", "/etc/ssl/certs");
            }
        }
        let mut resp = reqwest::blocking::Client::new()
            .get(&self.url)
            .headers(headers)
            .send()
            .with_context(|| format!("downloading {}", &self.url))?;
        let size = resp
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .map_or(0, |val| {
                val.to_str()
                    .map(|s| s.parse::<u64>().unwrap_or(0))
                    .unwrap_or(0)
            });
        if !resp.status().is_success() {
            bail!(
                "Downloading '{}' failed with status: {:?}",
                &self.url,
                resp.status()
            )
        }
        let show_progress = if size == 0 { false } else { self.show_progress };

        let file_name = resp
            .headers()
            .get(reqwest::header::CONTENT_DISPOSITION)
            .and_then(|value| Self::filename_from_content_disposition(value).ok())
            .unwrap_or("temp.bin")
            .to_string();

        if !out_dir.exists() {
            fs::create_dir_all(out_dir)?;
        }
        let out_file = out_dir.join(&file_name);
        let mut dest_file = fs::File::create(&out_file).unwrap();

        let bar = if show_progress {
            let pb = ProgressBar::new(size);
            pb.set_prefix(FINGER.to_string());
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(DEFAULT_PROGRESS_TEMPLATE)
                    .expect("set ProgressStyle template failed")
                    .progress_chars("━  ·"), // ━━╸━━━━━━━━━━━━━
            );
            pb
        } else {
            ProgressBar::hidden()
        };

        bar.set_message(style(&file_name).dim().to_string());
        copy_with_progress(&bar, &mut resp, &mut dest_file)?;

        bar.finish_with_message(file_name);
        Ok(out_file.to_string_lossy().to_string())
    }
}

pub fn copy_with_progress<R: ?Sized + Read, W: ?Sized + Write>(
    progress: &ProgressBar,
    reader: &mut R,
    writer: &mut W,
) -> io::Result<u64> {
    let mut buf = [0; 16384];
    let mut written = 0;
    loop {
        let len = match reader.read(&mut buf) {
            Ok(0) => return Ok(written),
            Ok(len) => len,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        writer.write_all(&buf[..len])?;
        written += len as u64;
        progress.inc(len as u64);
    }
}
pub struct TargetsDownloader<'a> {
    targets: &'a [Target],
    out_dir: &'a Path,
    show_progress: bool,
}

impl<'a> TargetsDownloader<'a> {
    pub fn new(targets: &'a [Target], out_dir: &'a Path) -> Self {
        Self {
            targets,
            out_dir,
            show_progress: true,
        }
    }
    pub fn download(&self, session: &mut Session<'_>, version: &str) -> Result<Vec<Target>> {
        session.console.say(&format!(
            "{} downloading {} target release(s) into {}",
            DOWNLOAD,
            self.targets.len(),
            style(&self.out_dir.to_string_lossy()).magenta(),
        ));
        self.targets
            .iter()
            .map(|t| {
                // if we have an archive and it exists on disk return it, otherwise download it
                if t.archive.is_some()
                    && t.archive
                        .as_ref()
                        .map(|a| Path::new(&self.out_dir.join(a)).exists())
                        .unwrap_or_default()
                {
                    Ok(t.clone())
                } else {
                    let url = t.url(version);
                    let mut d = Download::from_url(&url);
                    d.show_progress(self.show_progress);
                    d.download_to(self.out_dir).map(|archive| {
                        let mut updated = t.clone();
                        updated.archive = Some(archive);
                        updated
                    })
                }
            })
            .collect::<Result<Vec<_>>>()
    }
}
