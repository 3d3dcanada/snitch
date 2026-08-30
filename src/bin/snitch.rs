//! snitch: what is this file telling people.
//!
//! There is deliberately no umbrella command with the other tools hidden underneath as
//! subcommands. Each tool is the thing it is called, because the name IS the interface: you
//! remember "snitch" long after you have forgotten which flag of which subcommand read GPS.

use std::process::ExitCode;

use snitch::cli::{self, Args, Parsed};
use snitch::{inspect, survival};

const HELP: &str = "\
usage: snitch [--platforms] [--notes] [--check] [--json] [--] FILE...

Your photo is telling on you. This says what it is telling.

options:
  --platforms   sourced platform metadata-handling research
  --notes       detail behind every cell
  --check       how to verify a row yourself
  --json        emit stable machine-readable JSON
  --version     print the version
  -h, --help    this

Then:  no-comment FILE   to make it stop
       credit FILE       to put your name on it instead";

fn main() -> ExitCode {
    cli::quiet_on_closed_pipe();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match Args::parse(
        argv,
        &["platforms", "notes", "check", "json"],
        &["c2patool"],
    ) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("snitch: {e}\n\n{HELP}");
            return ExitCode::from(2);
        }
    };
    let a = match parsed {
        Parsed::Help => {
            println!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Parsed::Version => {
            println!("snitch {}", snitch::VERSION);
            return ExitCode::SUCCESS;
        }
        Parsed::Args(a) => a,
    };

    let tool = a.get("c2patool");
    if a.has("json") {
        return json_output(&a, tool);
    }

    if a.has("platforms") || a.files.is_empty() {
        cli::print_platforms(a.has("notes"), a.has("check"));
        if a.files.is_empty() {
            return ExitCode::SUCCESS;
        }
    }

    let failed = cli::for_each_file(&a.files, |path| match inspect::inspect(path, tool) {
        Ok(report) => cli::print_report(&report, path),
        Err(e) => {
            eprintln!("  {}: {e}", path.display());
            true
        }
    });
    println!(
        "\n  {}",
        cli::c(
            "snitch --platforms   what survives an upload and what does not",
            cli::DIM
        )
    );
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn json_output(a: &Args, tool: Option<&str>) -> ExitCode {
    let mut report = serde_json::Map::new();
    let mut failed = false;
    if a.has("platforms") || a.files.is_empty() {
        report.insert(
            "platforms".into(),
            survival::Table::load().as_json(a.has("notes"), a.has("check")),
        );
    }
    if !a.files.is_empty() {
        let mut files = Vec::new();
        for path in &a.files {
            let absolute = path.canonicalize().unwrap_or_else(|_| path.clone());
            if !path.exists() {
                files.push(serde_json::json!({"path": absolute, "error": "not found"}));
                failed = true;
                continue;
            }
            if !path.is_file() {
                files.push(serde_json::json!({"path": absolute, "error": "not a regular file"}));
                failed = true;
                continue;
            }
            match inspect::inspect(path, tool) {
                Ok(one) => {
                    failed |= one.c2pa_status == snitch::c2pa::Status::Error;
                    files.push(serde_json::to_value(one).unwrap_or_default());
                }
                Err(e) => {
                    files.push(serde_json::json!({"path": absolute, "error": e}));
                    failed = true;
                }
            }
        }
        report.insert("files".into(), serde_json::Value::Array(files));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::Value::Object(report)).unwrap_or_default()
    );
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
