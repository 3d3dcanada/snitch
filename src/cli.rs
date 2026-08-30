//! Shared command-line plumbing: colour, argument parsing and the renderers.
//!
//! NO CLI FRAMEWORK. Argument parsing here is a few dozen lines over `std::env::args_os`, which is
//! the house rule and is also the only way to get the exact behaviour these tools need: `--` ends
//! options so a file genuinely named `--force.jpg` is a file, and an unknown flag is an error
//! rather than a positional.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use crate::{c2pa, inspect, survival};

pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const RED: &str = "\x1b[31m";
pub const GRN: &str = "\x1b[32m";
pub const YEL: &str = "\x1b[33m";
pub const OFF: &str = "\x1b[0m";

/// The separator the sourced-notes view puts between a citation's title and its URL. Held as a
/// constant so no source file in this repository has to contain the character itself.
const EM_DASH: char = '\u{2014}';

pub fn colour_enabled() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stdout().is_terminal()
}

/// Wrap in a colour, or do not, depending on where output is going.
pub fn c(text: &str, colour: &str) -> String {
    if colour_enabled() {
        format!("{colour}{text}{OFF}")
    } else {
        text.to_string()
    }
}

/// A path as it should be typed back at a shell. The `--` matters: without it a file named
/// `--force.jpg` reads as a flag, and the suggestion this tool prints would not work.
pub fn command_path(path: &Path) -> String {
    format!("-- {}", shell_quote(&path.display().to_string()))
}

pub fn shell_quote(s: &str) -> String {
    let safe = !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "@%+=:,./-_".contains(c));
    if safe {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

/// A parsed command line: flags that were present, options with values, and the positionals.
pub struct Args {
    flags: Vec<String>,
    options: Vec<(String, String)>,
    pub files: Vec<PathBuf>,
}

pub enum Parsed {
    Args(Args),
    /// `--help` or `--version` was asked for; the caller prints and exits 0.
    Help,
    Version,
}

impl Args {
    /// `flag_names` take no value, `option_names` take the next argument or an `=` suffix.
    pub fn parse(
        argv: Vec<String>,
        flag_names: &[&str],
        option_names: &[&str],
    ) -> Result<Parsed, String> {
        let mut args = Args {
            flags: Vec::new(),
            options: Vec::new(),
            files: Vec::new(),
        };
        let mut iter = argv.into_iter();
        let mut only_files = false;
        while let Some(item) = iter.next() {
            if only_files {
                args.files.push(PathBuf::from(item));
                continue;
            }
            if item == "--" {
                only_files = true;
                continue;
            }
            if item == "--help" || item == "-h" {
                return Ok(Parsed::Help);
            }
            if item == "--version" {
                return Ok(Parsed::Version);
            }
            if let Some(rest) = item.strip_prefix("--") {
                let (name, inline) = match rest.split_once('=') {
                    Some((n, v)) => (n.to_string(), Some(v.to_string())),
                    None => (rest.to_string(), None),
                };
                if flag_names.contains(&name.as_str()) {
                    if inline.is_some() {
                        return Err(format!("--{name} does not take a value"));
                    }
                    args.flags.push(name);
                    continue;
                }
                if option_names.contains(&name.as_str()) {
                    let value = match inline {
                        Some(v) => v,
                        None => iter
                            .next()
                            .ok_or_else(|| format!("--{name} needs a value"))?,
                    };
                    args.options.push((name, value));
                    continue;
                }
                return Err(format!("unknown option --{name}"));
            }
            if let Some(short) = item
                .strip_prefix('-')
                .filter(|s| !s.is_empty() && *s != "-")
            {
                // A short option is the same name with one dash: `-o` is `--out`'s alias because
                // "o" is in the option list. Anything else with a leading dash is a mistake, not a
                // filename, and saying so beats silently treating `-0` as a file.
                let (name, inline) = match short.split_once('=') {
                    Some((n, v)) => (n.to_string(), Some(v.to_string())),
                    None => (short.to_string(), None),
                };
                if flag_names.contains(&name.as_str()) {
                    args.flags.push(name);
                    continue;
                }
                if option_names.contains(&name.as_str()) {
                    let value = match inline {
                        Some(v) => v,
                        None => iter
                            .next()
                            .ok_or_else(|| format!("-{name} needs a value"))?,
                    };
                    args.options.push((name, value));
                    continue;
                }
                return Err(format!("unknown option {item}"));
            }
            args.files.push(PathBuf::from(item));
        }
        Ok(Parsed::Args(args))
    }

    pub fn has(&self, name: &str) -> bool {
        self.flags.iter().any(|f| f == name)
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.options
            .iter()
            .rev()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// Every value given for a repeatable option, in the order they were typed.
    pub fn all(&self, name: &str) -> Vec<String> {
        self.options
            .iter()
            .filter(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .collect()
    }

    pub fn parse_number<T: std::str::FromStr>(&self, name: &str) -> Result<Option<T>, String> {
        match self.get(name) {
            None => Ok(None),
            Some(raw) => raw
                .parse::<T>()
                .map(Some)
                .map_err(|_| format!("--{name} needs a number, got {raw:?}")),
        }
    }
}

// ----------------------------------------------------------------------------------------------
// renderers
// ----------------------------------------------------------------------------------------------

/// Print one inspection report the way `snitch` prints it. Returns true when something went wrong
/// enough for a non-zero exit, which today means only a failed C2PA check.
pub fn print_report(report: &inspect::Report, path: &Path) -> bool {
    let mut failed = false;
    println!(
        "\n{}  {} bytes",
        c(&report.file, BOLD),
        thousands(report.bytes)
    );

    if !report.gps.is_empty() {
        println!("{}", c("  LOCATION IS IN THIS FILE", RED));
        for (key, value) in &report.gps {
            // The data carries `EXIF:GPSLatitude`; a person reading a terminal wants `GPSLatitude`.
            let short = key.rsplit(':').next().unwrap_or(key);
            println!("    {short:<16} {}", crate::exif::render(value));
        }
        println!(
            "{}",
            c(
                "    Anyone who downloads this can see where it was taken.",
                RED
            )
        );
        println!("    Make it stop:  no-comment {}", command_path(path));
    } else {
        println!("{}", c("  no location data", GRN));
    }

    if !report.camera.is_empty() {
        println!("  camera");
        for (key, value) in &report.camera {
            println!("    {key:<16} {}", crate::exif::render(value));
        }
    }

    if !report.credit.is_empty() {
        println!("{}", c("  credit", GRN));
        for (key, value) in &report.credit {
            println!(
                "    {key:<16} {}",
                ellipsis(&crate::exif::render(value), 90)
            );
        }
    } else {
        println!("{}", c("  NO CREDIT AT ALL", YEL));
        println!("    Nothing in this file says who made it.");
        println!(
            "    Fix it:  credit --creator \"Your Name\" {}",
            command_path(path)
        );
    }

    if !report.png_text.is_empty() {
        println!("{}", c("  TEXT IS EMBEDDED IN THIS PNG", YEL));
        for chunk in &report.png_text {
            let text: String = chunk.text.split_whitespace().collect::<Vec<_>>().join(" ");
            let keyword: String = chunk.keyword.chars().take(16).collect();
            println!("    {keyword:<16} {}", ellipsis(&text, 84));
        }
        if report.ai_source.as_deref() == Some("png-text-chunk") {
            println!(
                "{}",
                c(
                    "    A GENERATOR WROTE THIS. It names the tool, and usually the whole prompt.",
                    YEL
                )
            );
            println!(
                "    This is plain text, not a signed credential. It proves nothing on its own."
            );
        }
        println!("    Take it out:  no-comment {}", command_path(path));
    }

    match (&report.c2pa, report.c2pa_status) {
        (Some(manifest), _) => {
            let state = c2pa::validation_state(Some(manifest));
            println!("  C2PA Content Credential  [{state}]");
            println!(
                "    title            {}",
                c2pa::title(Some(manifest)).unwrap_or("?".into())
            );
            println!(
                "    certificate issuer {}",
                c2pa::signer(Some(manifest)).unwrap_or("?".into())
            );
            if c2pa::asset_altered(Some(manifest)) {
                println!(
                    "{}",
                    c(
                        "    ALTERED AFTER SIGNING: these pixels are not the ones that were signed",
                        RED
                    )
                );
            }
            if c2pa::identity_untrusted(Some(manifest)) {
                println!(
                    "{}",
                    c(
                        "    SIGNER IDENTITY UNTRUSTED (certificate not on validator trust list)",
                        YEL
                    )
                );
            }
            match (report.ai.as_deref(), report.ai_source.as_deref()) {
                (Some("generative"), Some("c2pa")) => println!(
                    "{}",
                    c("    THIS SAYS IT WAS MADE BY A GENERATIVE MODEL", YEL)
                ),
                (Some("camera"), _) => {
                    println!("{}", c("    asserts a camera made this, not a model", GRN))
                }
                _ => {}
            }
        }
        (None, c2pa::Status::Absent) => println!("  no C2PA Content Credential"),
        (None, c2pa::Status::DetectedUnverified) => {
            println!(
                "{}",
                c(
                    "  C2PA Content Credential detected; validation unavailable",
                    YEL
                )
            );
            println!("    Install c2patool to read and validate it.");
        }
        (None, c2pa::Status::Unavailable) => println!(
            "{}",
            c("  C2PA status unavailable: c2patool is not installed", YEL)
        ),
        (None, _) => {
            println!(
                "{}",
                c(&format!("  C2PA check failed: {}", report.c2pa_error), RED)
            );
            failed = true;
        }
    }
    failed
}

pub fn print_platforms(notes: bool, check: bool) {
    let table = survival::Table::load();
    println!(
        "\n{}   {}\n",
        c("Evidence on platform metadata handling", BOLD),
        c(&format!("researched {}", table.researched), DIM)
    );
    println!("  D documented by platform   C independently corroborated   ? unverified\n");
    let platforms = table.ordered_platforms();
    let width = platforms
        .iter()
        .map(|(n, _)| n.chars().count())
        .max()
        .unwrap_or(8)
        + 2;
    let head: String = table
        .layers
        .iter()
        .map(|l| format!("{:<26}", l.label))
        .collect();
    println!("  {:<width$}{head}", "", width = width);
    for (name, layers) in &platforms {
        let mut row = format!("  {name:<width$}", width = width);
        for layer in &table.layers {
            let cell = layers.get(&layer.key);
            let text = cell
                .map(|cell| table.display_cell(cell))
                .unwrap_or_default();
            let colour = match cell.map(|cell| (cell.evidence.as_str(), cell.verdict.as_str())) {
                Some(("inference", _)) => DIM,
                Some((_, "keeps")) | Some((_, "reads")) => GRN,
                Some((_, "strips")) => RED,
                Some((_, "partial")) => YEL,
                _ => DIM,
            };
            row.push_str(&c(&format!("{text:<26}"), colour));
        }
        println!("{row}");
    }
    println!("\n  {}\n", c(&table.advice, BOLD));
    if notes {
        for (name, layers) in &platforms {
            println!("  {}", c(name, BOLD));
            for layer in &table.layers {
                let Some(cell) = layers.get(&layer.key) else {
                    continue;
                };
                if cell.note.is_empty() {
                    continue;
                }
                println!("    {:<26} [{}] {}", layer.label, cell.evidence, cell.note);
                for source in &cell.sources {
                    println!("      source: {} {EM_DASH} {}", source.title, source.url);
                }
            }
            println!();
        }
    } else {
        println!("  {}", c("--notes for the detail on every cell", DIM));
    }
    if check {
        println!("\n{}", table.how_to_verify);
    }
}

fn ellipsis(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let kept: String = text.chars().take(limit.saturating_sub(1)).collect();
    format!("{kept}\u{2026}")
}

pub fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Walk the file list, refusing anything that is not an ordinary file, and hand each survivor to
/// `each`. Returns true if anything failed, which becomes the exit status.
pub fn for_each_file(files: &[PathBuf], mut each: impl FnMut(&Path) -> bool) -> bool {
    let mut failed = false;
    for path in files {
        if !path.exists() {
            eprintln!("  {}: not found", path.display());
            failed = true;
            continue;
        }
        if !path.is_file() {
            eprintln!("  {}: not a regular file", path.display());
            failed = true;
            continue;
        }
        failed |= each(path);
    }
    failed
}
