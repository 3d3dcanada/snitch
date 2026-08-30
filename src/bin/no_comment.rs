//! no-comment: take the metadata out, losslessly.

use std::path::PathBuf;
use std::process::ExitCode;

use snitch::cli::{self, Args, Parsed};
use snitch::strip;

const HELP: &str = "\
usage: no-comment [-o OUT] [--in-place] [--force] [--] FILE...

Strip metadata out of an image. Losslessly: the pixels do not change.

JPEG and PNG only. Nothing is re-encoded, and every write is checked by comparing the decoded
pixels before and after, so \"pixels byte-identical\" is a result and not a promise.

options:
  -o, --out OUT   output file, or an existing directory for multiple inputs
  --in-place      atomically replace each input file
  --force         replace an existing output file
  --version       print the version
  -h, --help      this

In-pixel watermarks are not touched by metadata stripping.";

const WATERMARK_NOTE: &str = "
  In-pixel watermarks are not touched by metadata stripping.
  SynthID and its relatives live in image data; removal attempts repaint pixels.";

fn main() -> ExitCode {
    cli::quiet_on_closed_pipe();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match Args::parse(argv, &["in-place", "force"], &["out", "o"]) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("no-comment: {e}\n\n{HELP}");
            return ExitCode::from(2);
        }
    };
    let a = match parsed {
        Parsed::Help => {
            println!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Parsed::Version => {
            println!("no-comment {}", snitch::VERSION);
            return ExitCode::SUCCESS;
        }
        Parsed::Args(a) => a,
    };
    if a.files.is_empty() {
        eprintln!("{HELP}");
        return ExitCode::from(2);
    }
    let out = a.get("out").or_else(|| a.get("o")).map(PathBuf::from);
    if a.has("in-place") && out.is_some() {
        eprintln!("no-comment: --in-place and --out are alternatives, not a pair");
        return ExitCode::from(2);
    }
    if a.has("in-place") && a.has("force") {
        eprintln!("no-comment: --force is only meaningful with copied output");
        return ExitCode::from(2);
    }

    let targets = match resolve_targets(&a.files, out.as_deref(), a.has("in-place")) {
        Ok(targets) => targets,
        Err(e) => {
            eprintln!("no-comment: {e}");
            return ExitCode::from(2);
        }
    };

    let mut failed = false;
    for (source, target) in a.files.iter().zip(targets) {
        if !source.exists() {
            eprintln!("  {}: not found", source.display());
            failed = true;
            continue;
        }
        if !source.is_file() {
            eprintln!("  {}: not a regular file", source.display());
            failed = true;
            continue;
        }
        let in_place = a.has("in-place");
        if !in_place && target.exists() && !a.has("force") {
            eprintln!(
                "  {}: output exists; pass --force to replace it",
                target.display()
            );
            failed = true;
            continue;
        }
        match strip::strip_atomic(source, &target) {
            Ok((removed, identical)) => {
                let proof = if identical {
                    "pixels byte-identical"
                } else {
                    "pixels CHANGED"
                };
                let name = if in_place { source } else { &target };
                if removed == 0 {
                    println!("  {}  already had nothing to remove", name.display());
                } else {
                    println!(
                        "  {}  removed {} bytes of metadata  {}",
                        name.display(),
                        cli::thousands(removed),
                        cli::c(proof, cli::GRN)
                    );
                }
            }
            Err(e) => {
                eprintln!("  {}: {e}", source.display());
                failed = true;
            }
        }
    }
    println!("{}", cli::c(WATERMARK_NOTE, cli::DIM));
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Where each input's output goes, refusing a set that would write two inputs to one name.
fn resolve_targets(
    files: &[PathBuf],
    out: Option<&std::path::Path>,
    in_place: bool,
) -> Result<Vec<PathBuf>, String> {
    if in_place {
        return Ok(files.to_vec());
    }
    let targets: Vec<PathBuf> = match out {
        Some(out) if out.is_dir() => files
            .iter()
            .map(|f| {
                let name = f.file_name().unwrap_or_default();
                strip::outpath(&out.join(name), None, "-clean")
            })
            .collect(),
        Some(_) if files.len() > 1 => {
            return Err(
                "--out must be an existing directory when processing multiple files".into(),
            );
        }
        Some(out) => vec![out.to_path_buf()],
        None => files
            .iter()
            .map(|f| strip::outpath(f, None, "-clean"))
            .collect(),
    };
    let mut seen: Vec<PathBuf> = Vec::new();
    for target in &targets {
        let key = target.canonicalize().unwrap_or_else(|_| target.clone());
        if seen.contains(&key) {
            return Err("multiple inputs would write the same output filename".into());
        }
        seen.push(key);
    }
    Ok(targets)
}
