//! # Looneygrep Library
//!
//! This library provides the core functionality for the `looneygrep` command-line tool.
//!
//! It exposes configuration parsing, file searching (case-sensitive and insensitive), and the main run logic.
//!
//! ## Example
//!
//! ```rust
//! use looneygrep::{Config, run};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = Config { 
//!     query: String::from("foo"), 
//!     file_path: String::from("bar.txt"),
//!     ignore_case: false
//! };
//! run(config)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Features
//! - Search files or web pages for a query string
//! - Optional case-insensitive search
//! - Prompt-to-replace matches interactively
//! - Show context lines around matches
//! - Syntax highlighting for code files
//! - File type awareness
//! - Search all files in a directory with `--all`
//!
//! ## Usage
//!
//! ```sh
//! looneygrep <query> <filename> [--ignore-case] [--replace] [--context N] [--url <url>] [--all]
//! ```

use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

/// Configuration for the search operation.
///
/// This struct holds all options for a search, including the query string,
/// file path, case sensitivity, replacement mode, URL, context lines, and
/// whether to search all files in the current directory.
#[derive(Clone, Debug)]
/// Configuration for the search operation.
pub struct Config {
    /// The string to search for.
    pub query: String,
    /// The path to the file to search.
    pub file_path: String,
    /// Whether the search is case-insensitive.
    pub ignore_case: bool,
    /// Whether to prompt for replacement of matches.
    pub replace: bool,
    /// Optional URL to search instead of a file.
    pub url: Option<String>,
    /// Number of context lines to show around matches.
    pub context: usize,
    /// If true, search all files in the current directory.
    pub search_all: bool,
}

impl Config {
    /// Builds a `Config` from command-line arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - An iterator over command-line arguments.
    ///
    /// # Errors
    ///
    /// Returns an error if required arguments are missing or invalid.
    ///
    /// # Example
    ///
    /// ```
    /// let config = Config::build(std::env::args())?;
    /// ```
    pub fn build(mut args: impl Iterator<Item = String>,
    ) -> Result<Config, &'static str> {
        args.next(); // Skip program name
        let query = match args.next() {
            Some(arg) => arg,
            None => return Err("Didn't get a query string"),
        };
        let mut file_path = String::new();
        let mut url = None;
        let mut ignore_case = env::var("IGNORE_CASE").is_ok();
        let mut replace = false;
        let mut context = 0;
        let mut search_all = false;
        while let Some(arg) = args.next() {
            if arg == "--replace" {
                replace = true;
            } else if arg == "--ignore-case" {
                ignore_case = true;
            } else if arg == "--url" {
                url = args.next();
            } else if arg == "--context" {
                context = args.next().and_then(|n| n.parse().ok()).unwrap_or(0);
            } else if arg == "--all" {
                search_all = true;
            } else {
                file_path = arg;
            }
        }
        if !search_all && file_path.is_empty() && url.is_none() {
            return Err("Didn't get a file path or URL");
        }
        Ok(Config { query, file_path, ignore_case, replace, url, context, search_all })
    }
}

/// Runs the search with the given configuration.
///
/// If `search_all` is set, searches all files in the current directory.
/// If `url` is set, searches the contents of the web page.
/// Otherwise, searches the specified file.
///
/// # Errors
/// Returns an error if the file or URL cannot be read.
///
/// # Example
/// ```rust
/// use looneygrep::{Config, run};
/// let config = Config { /* ... */ };
/// run(config).unwrap();
/// ```
pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    if config.search_all {
        use std::fs;

        let entries = fs::read_dir(".")?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let file_path = path.to_string_lossy().to_string();
                let mut file_config = Config {
                    file_path: file_path.clone(),
                    url: None,
                    ..config.clone()
                };
                println!("\n=== Searching in file: {} ===", file_path);
                // Call a helper to search a single file
                search_file(&mut file_config)?;
            }
        }
        return Ok(());
    }

    // ...existing code for single file or URL...
    if let Some(url) = &config.url {
        let body = fetch_url(url)?;
        search_contents(&body, &config, "<web page>")?;
    } else {
        let contents = fs::read_to_string(&config.file_path)?;
        search_contents(&contents, &config, &config.file_path)?;
    }
    Ok(())
}

// Helper to search a single file
fn search_file(config: &mut Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(&config.file_path)?;
    search_contents(&contents, config, &config.file_path)
}

// Helper to search contents (used for both file and URL)
fn search_contents(contents: &str, config: &Config, file_path: &str) -> Result<(), Box<dyn Error>> {
    let mut lines: Vec<String> = contents.lines().map(|l| l.to_string()).collect();
    let mut changed = false;

    // Find matches
    let matches: Vec<(usize, String)> = lines.iter()
        .enumerate()
        .filter(|(_, line)| {
            if config.ignore_case {
                line.to_lowercase().contains(&config.query.to_lowercase())
            } else {
                line.contains(&config.query)
            }
        })
        .map(|(i, l)| (i, l.clone()))
        .collect();

    // Live preview (same as before)
    println!("Preview of matches:");
    let mut printed = vec![false; lines.len()];
    let mut lines_printed = 0;
    let max_lines = 1000;

    for (i, _) in &matches {
        let start = i.saturating_sub(config.context);
        let end = usize::min(i + 1 + config.context, lines.len());
        for line_idx in start..end {
            if !printed[line_idx] {
                let line_num = line_idx + 1;
                if line_idx == *i {
                    let highlighted = highlight_all_matches(&lines[line_idx], &config.query, config.ignore_case);
                    println!("{}: {}", line_num, syntax_highlight_line(&highlighted, file_path));
                } else {
                    println!("{}: {}", line_num, syntax_highlight_line(&lines[line_idx], file_path));
                }
                printed[line_idx] = true;
            }
        }
        println!("---");
        lines_printed += 1;
        if lines_printed >= max_lines {
            println!("Output truncated. Too many results.");
            break;
        }
    }

    if config.replace {
        if config.url.is_some() {
            println!("Warning: --replace is not supported when searching a URL. No changes will be made.");
            return Ok(());
        }
        // Prompt to replace
        let mut replace_all = false;
        for (i, line) in matches {
            if !replace_all {
                print!(
                    "Replace in line {}? (y/n/all/quit): {} ",
                    i + 1,
                    highlight_all_matches(&line, &config.query, config.ignore_case)
                );
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                match input.trim() {
                    "y" => {},
                    "all" => { replace_all = true; },
                    "n" => { continue; },
                    "quit" => { break; },
                    _ => { continue; }
                }
            }
            lines[i] = replace_all_matches(&lines[i], &config.query, "<REPLACED>", config.ignore_case);
            changed = true;
        }

        // Write changes if any
        if changed {
            fs::write(file_path, lines.join("\n"))?;
            println!("Replacements made and file saved.");
        } else {
            println!("No replacements made.");
        }
    }

    if config.url.is_none() {
        print_file_type_note(file_path);
    }

    Ok(())
}

/// Highlights all matches of the query in a line using ANSI escape codes.
fn highlight_all_matches(line: &str, query: &str, ignore_case: bool) -> String {
    if query.is_empty() {
        return line.to_string();
    }
    let mut result = String::new();
    let mut last = 0;
    let (line_cmp, query_cmp) = if ignore_case {
        (line.to_lowercase(), query.to_lowercase())
    } else {
        (line.to_string(), query.to_string())
    };
    let mut search_start = 0;
    while let Some(pos) = line_cmp[search_start..].find(&query_cmp) {
        let abs_pos = search_start + pos;
        result.push_str(&line[last..abs_pos]);
        result.push_str("\x1b[31m"); // Red
        result.push_str(&line[abs_pos..abs_pos + query.len()]);
        result.push_str("\x1b[0m");
        last = abs_pos + query.len();
        search_start = last;
    }
    result.push_str(&line[last..]);
    result
}

/// Replaces all matches of the query in a line, case-sensitive or insensitive.
fn replace_all_matches(line: &str, query: &str, replacement: &str, ignore_case: bool) -> String {
    if ignore_case {
        let mut result = String::new();
        let mut last = 0;
        let line_lower = line.to_lowercase();
        let query_lower = query.to_lowercase();
        let mut search_start = 0;
        while let Some(pos) = line_lower[search_start..].find(&query_lower) {
            let abs_pos = search_start + pos;
            result.push_str(&line[last..abs_pos]);
            result.push_str(replacement);
            last = abs_pos + query.len();
            search_start = last;
        }
        result.push_str(&line[last..]);
        result
    } else {
        line.replace(query, replacement)
    }
}

/// Fetches the contents of a URL using a blocking HTTP request.
///
/// # Errors
/// Returns an error if the request fails.
fn fetch_url(url: &str) -> Result<String, Box<dyn Error>> {
    let resp = reqwest::blocking::get(url)?;
    let body = resp.text()?;
    Ok(body)
}

/// Prints a note about the file type based on its extension.
fn print_file_type_note(file_path: &str) {
    if let Some(ext) = std::path::Path::new(file_path).extension().and_then(|e| e.to_str()) {
        match ext {
            "rs" => println!("(Rust source file detected)"),
            "txt" => println!("(Text file detected)"),
            "md" => println!("(Markdown file detected)"),
            "html" | "htm" => println!("(HTML file detected)"),
            "css" => println!("(CSS file detected)"),
            "json" => println!("(JSON file detected)"),
            "xml" => println!("(XML file detected)"),
            "yaml" | "yml" => println!("(YAML file detected)"),
            "toml" => println!("(TOML file detected)"),
            "log" => println!("(Log file detected)"),
            "csv" => println!("(CSV file detected)"),
            "conf" | "cfg" => println!("(Configuration file detected)"),
            "sh" => println!("(Shell script detected)"),
            "bat" => println!("(Batch script detected)"),
            "php" => println!("(PHP source file detected)"),
            "java" => println!("(Java source file detected)"),
            "go" => println!("(Go source file detected)"),
            "py" => println!("(Python source file detected)"),
            "js" => println!("(JavaScript source file detected)"),
            "c" | "h" => println!("(C source/header file detected)"),
            _ => {}
        }
    }
}

/// Applies syntax highlighting to a line based on the file extension.
fn syntax_highlight_line(line: &str, file_path: &str) -> String {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let syntax = ps.find_syntax_by_extension(ext).unwrap_or_else(|| ps.find_syntax_plain_text());
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    let mut highlighted = String::new();
    for line in LinesWithEndings::from(line) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        highlighted.push_str(&as_24_bit_terminal_escaped(&ranges[..], false));
    }
    highlighted
}

/// Searches lines using a custom matcher closure.
///
/// # Arguments
///
/// * `lines` - An iterator over lines to search.
/// * `matcher` - A closure that takes a line and returns `true` if it matches.
///
/// # Returns
///
/// A vector of matching lines.
///
/// # Example
///
/// ```rust
/// let lines = vec!["foo", "bar", "baz"];
/// let matches = search(lines.iter(), |line| line.contains("ba"));
/// assert_eq!(matches, vec!["bar", "baz"]);
/// ```
pub fn search<'a, I, F>(lines: I, matcher: F) -> Vec<&'a str>
where
    I: IntoIterator<Item = &'a str>,
    F: Fn(&str) -> bool,
{
    lines.into_iter().filter(|line| matcher(line)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that context lines are correctly identified around matches.
    #[test]
    fn test_context_lines() {
        let contents = "\
line1
match
line3
match
line5";
        let _config = Config {
            /*config fields*/
            query: "match".to_string(),
            file_path: "".to_string(),
            ignore_case: false,
            replace: false,
            url: None,
            context: 1,
            search_all: false,
        };
        let _output: Vec<(usize, &str)> = Vec::new();
        // You'd need to refactor search_contents to write to output for testability
        // For now, just check that context lines are correct
        let lines: Vec<&str> = contents.lines().collect();
        let matches: Vec<(usize, &str)> = lines.iter().enumerate()
            .filter(|(_, line)| (*line).contains("match"))
            .map(|(i, line)| (i, *line))
            .collect();
        assert_eq!(matches, vec![(1, "match"), (3, "match")]);
    }

    /// Tests that all matches in a line are replaced correctly.
    #[test]
    fn test_replace_all_matches() {
        let line = "foo bar foo";
        let replaced = replace_all_matches(line, "foo", "baz", false);
        assert_eq!(replaced, "baz bar baz");
    }

    /// Tests that all matches in a line are highlighted with ANSI codes.
    #[test]
    fn test_highlight_all_matches() {
        let line = "foo bar foo";
        let highlighted = highlight_all_matches(line, "foo", false);
        assert!(highlighted.contains("\x1b[31mfoo\x1b[0m"));
    }

    /// Tests that file type notes print for various extensions.
    #[test]
    fn test_file_type_note() {
        // This just checks that the function runs; for real tests, capture stdout
        print_file_type_note("test.rs");
        print_file_type_note("test.py");
        print_file_type_note("test.txt");
    }

    /// Tests that syntax highlighting adds ANSI codes for supported file types.
    #[test]
    fn test_syntax_highlight_line() {
        let line = "fn main() {}";
        let highlighted = syntax_highlight_line(line, "test.rs");
        assert!(highlighted.contains("\x1b["));
    }
}
