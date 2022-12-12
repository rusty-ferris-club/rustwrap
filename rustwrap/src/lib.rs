//!
//!
//!
//! Rustwrap is a tool that helps wrap binary releases for easy distribution. Currently supporting:
//!* **npm** - `npm install -g your-tool` will make your binary `your-tool` available via the CLI. `rustwrap` creates the necessary binary packages and root package with a Node.js shim that delegates running to your platform-specific bin.
//!* **Homebrew** - creates a recipe and saves or publishes it to your tap.
//!
//!
#![warn(missing_docs)] // uncomment for docs
#![allow(clippy::missing_const_for_fn)]
mod console;
mod data;
mod download;
mod providers;

/// run the main workflow
pub mod runner;
