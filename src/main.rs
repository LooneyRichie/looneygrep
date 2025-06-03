//! # Looneygrep
//!
//! Looneygrep is a feature-rich command-line text search tool created by Richie Looney as an alternative to `grep`.
//!
//! ## Features
//! - Search files or web pages for a query string
//! - Optional case-insensitive search
//! - Prompt-to-replace matches interactively
//! - Show context lines around matches
//! - Syntax highlighting for code files
//! - File type awareness
//!
//! ## Usage
//!
//! ```sh
//! looneygrep <query> <filename> [--ignore-case] [--replace] [--context N] [--url <url>] [--all]
//! ```
//!
//! ## Example (Rust)
//! ```rust
//! use looneygrep::{Config, run};
//! let config = Config { /* ... */ };
//! run(config).unwrap();
//! ```

use std::env;
use std::process;

use looneygrep::Config;

/// The main entry point for the Looneygrep application.
///
/// Parses command-line arguments, builds the configuration,
/// and runs the search. Exits with an error code if something fails.
fn main() {
    let config = Config::build(env::args())
        .unwrap_or_else(|err| {
            eprintln!("Problem parsing arguments: {}", err);
            process::exit(1);
        });
    if let Err(e) = looneygrep::run(config) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
    println!("Search completed successfully.");
}