# Looneygrep

**Looneygrep is executable as lg for those who wish to type less**

**Looneygrep** is a feature-rich, blazing-fast command-line text search tool created by Richie Looney as a modern alternative to `grep` and `ripgrep`.  
It supports searching files, web pages, and even interactive replacement of matches.

---

## Features

- 🔍 Search files or web pages for a query string
- 🅰️ Optional case-insensitive search
- 📝 Prompt-to-replace matches interactively
- 📄 Show context lines around matches
- 🎨 Syntax highlighting for code files
- 🧠 File type awareness
- 📂 Search all files in a directory with `--all`

---

## Installation

From [crates.io](https://crates.io/crates/looneygrep):

```sh
cargo install looneygrep
```

Or clone and build manually:

```sh
git clone https://github.com/looneyrichie/looneygrep.git
cd looneygrep
cargo build --release
```

---

## Usage

```sh
looneygrep <query> <filename> [--ignore-case] [--replace] [--context N] [--url <url>] [--all]
```

### Examples

**Search a file:**
```sh
looneygrep foo myfile.txt
```

**Search a web page:**
```sh
looneygrep Rust --url https://www.rust-lang.org
```

**Case-insensitive search with context:**
```sh
looneygrep error log.txt --ignore-case --context 2
```

**Prompt to replace matches:**
```sh
looneygrep oldword file.txt --replace
```

**Search all files in the current directory:**
```sh
looneygrep TODO --all
```

---

## Library Usage

You can also use Looneygrep as a library in your Rust code:

```rust
use looneygrep::{Config, run};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config {
        query: "foo".to_string(),
        file_path: "bar.txt".to_string(),
        ignore_case: false,
        replace: false,
        url: None,
        context: 0,
        search_all: false,
    };
    run(config)?;
    Ok(())
}
```

---

## License

Licensed under either of  
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

---

## Author

Richie Looney (<richieandkayla@gmail.com>)

---

**Contributions welcome!**