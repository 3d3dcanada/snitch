//! credit: put your name on your work, in the places that survive.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use snitch::cli::{self, Args, Parsed};
use snitch::{c2pa, exif, inspect, sign, stamp, strip};

const FLAGS: [&str; 7] = [
    "keep-gps",
    "in-place",
    "force",
    "sign",
    "verify",
    "generated",
    "no-sign-notice",
];
const OPTIONS: [&str; 19] = [
    "creator",
    "credit",
    "copyright",
    "licence",
    "license",
    "terms",
    "rights-url",
    "url",
    "contact",
    "title",
    "description",
    "keyword",
    "stamp",
    "stamp-sub",
    "logo",
    "corner",
    "scale",
    "opacity",
    "font",
];
const MORE_OPTIONS: [&str; 5] = ["quality", "out", "o", "key", "cert"];

fn help() -> String {
    format!(
        "\
usage: credit [options] [--] FILE...

Put your name on your work, in the places that survive.

metadata:
  --creator NAME        the person who made it
  --credit NAME         the studio or organisation to credit
  --copyright TEXT      copyright notice
  --licence KEY         one of: {}
  --terms TEXT          usage terms, overrides the licence text
  --rights-url URL      --url URL          --contact EMAIL_OR_URL
  --title TEXT          --description TEXT --keyword WORD (repeatable)
  --keep-gps            keep location data. Off by default, because it doxxes people

visible stamp; portable but still subject to crop and re-encoding:
  --stamp TEXT          burn this text into the pixels
  --stamp-sub TEXT      second line under it
  --logo PNG            --corner {{bottom-right,bottom-left,top-right,top-left}}
  --scale N             --opacity N        --font PATH.ttf     --quality N

C2PA Content Credentials:
  --sign                sign with a local development certificate
  --digital-source KEY  one of: {}
  --generated           shorthand for --digital-source generated
  --verify              read and validate an existing credential
  --key PATH            --cert PATH

output:
  --in-place            atomically replace each input file
  -o, --out OUT         output file, or an existing directory for multiple inputs
  --force               replace an existing output file
  --version             -h, --help",
        snitch::licence_keys(),
        sign::digital_source_keys()
    )
}

/// The Python printed argparse's errors with a `credit: error:` prefix and printed its own two
/// validation messages bare. That inconsistency is ported rather than tidied, because a script
/// that greps for one of those lines should not have to care which release it is running.
enum Fault {
    Prefixed(String),
    Bare(String),
}

impl From<String> for Fault {
    fn from(s: String) -> Self {
        Fault::Prefixed(s)
    }
}

impl From<&str> for Fault {
    fn from(s: &str) -> Self {
        Fault::Prefixed(s.to_string())
    }
}

fn main() -> ExitCode {
    cli::quiet_on_closed_pipe();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut options: Vec<&str> = OPTIONS.to_vec();
    options.extend_from_slice(&MORE_OPTIONS);
    options.push("digital-source");
    let parsed = match Args::parse(argv, &FLAGS, &options) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("credit: {e}\n\n{}", help());
            return ExitCode::from(2);
        }
    };
    let a = match parsed {
        Parsed::Help => {
            println!("{}", help());
            return ExitCode::SUCCESS;
        }
        Parsed::Version => {
            println!("credit {}", snitch::VERSION);
            return ExitCode::SUCCESS;
        }
        Parsed::Args(a) => a,
    };
    match run(&a) {
        Ok(code) => code,
        Err(Fault::Prefixed(e)) => {
            eprintln!("credit: error: {e}");
            ExitCode::from(2)
        }
        Err(Fault::Bare(e)) => {
            println!("{e}");
            ExitCode::from(2)
        }
    }
}

fn run(a: &Args) -> Result<ExitCode, Fault> {
    if a.files.is_empty() {
        return Err("no files given. Run with --help.".into());
    }
    let stamping = a.get("stamp").is_some() || a.get("logo").is_some();
    let signing = a.has("sign");

    if a.has("verify") {
        let writing = stamping
            || signing
            || a.has("in-place")
            || a.has("force")
            || a.has("keep-gps")
            || a.get("out").is_some()
            || a.get("o").is_some()
            || a.get("key").is_some()
            || a.get("cert").is_some();
        if writing {
            return Err(
                "--verify cannot be combined with writing, stamping, or output options".into(),
            );
        }
        return Ok(verify(a));
    }

    let credit = build_credit(a)?;
    if credit.is_empty() && !stamping && !signing {
        return Err("nothing to do; provide credit fields, --stamp/--logo, or --sign".into());
    }
    if a.has("generated") && !signing {
        return Err("--generated only applies with --sign".into());
    }
    if a.get("digital-source").is_some() && !signing {
        return Err("--digital-source only applies with --sign".into());
    }
    if a.has("generated") && a.get("digital-source").is_some() {
        return Err("use --generated or --digital-source generated, not both".into());
    }
    if signing && !a.has("generated") && a.get("digital-source").is_none() {
        return Err("--sign requires --digital-source so the credential does not guess".into());
    }
    if let Some(key) = a.get("digital-source") {
        if sign::digital_source_url(key).is_none() {
            return Err(Fault::Bare(format!(
                "unknown digital source '{key}'. One of: {}",
                sign::digital_source_keys()
            )));
        }
    }
    if (a.get("key").is_some() || a.get("cert").is_some()) && !signing {
        return Err("--key and --cert only apply with --sign".into());
    }
    if a.get("key").is_some() != a.get("cert").is_some() {
        return Err("--key and --cert go together".into());
    }
    let in_place = a.has("in-place");
    let out = a.get("out").or_else(|| a.get("o")).map(PathBuf::from);
    if in_place && out.is_some() {
        return Err("--in-place and --out are alternatives, not a pair".into());
    }
    if in_place && a.has("force") {
        return Err("--force is only meaningful with copied output".into());
    }

    let targets = resolve_targets(&a.files, out.as_deref(), in_place).map_err(Fault::Prefixed)?;
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
        if !in_place && target.exists() && !a.has("force") {
            eprintln!(
                "  {}: output exists; pass --force to replace it",
                target.display()
            );
            failed = true;
            continue;
        }
        if let Err(e) = one_file(a, source, &target, &credit, stamping) {
            eprintln!(
                "  {}: FAILED {e}",
                source.file_name().unwrap_or_default().to_string_lossy()
            );
            failed = true;
            continue;
        }
        println!(
            "  {}  credit written  {}",
            target.display(),
            cli::c("ok", cli::GRN)
        );
        if signing {
            match do_sign(a, &target) {
                Ok(made_cert) => {
                    if let Some((key, cert)) = made_cert {
                        println!("  generated a signing certificate at {}", cert.display());
                        println!("  private key {} (chmod 600, keep it)", key.display());
                    }
                    println!(
                        "  {}  signed",
                        target.file_name().unwrap_or_default().to_string_lossy()
                    );
                    println!("\n{}", cli::c(sign::SELF_SIGNED_NOTICE, cli::DIM));
                }
                Err(e) => {
                    eprintln!("  {}: signing failed: {e}", target.display());
                    failed = true;
                }
            }
        }
    }
    println!(
        "\n{}",
        cli::c(
            "  Platform handling varies. --stamp puts credit in the pixels, where it\n  \
             survives metadata stripping but may still be cropped. snitch --platforms",
            cli::DIM
        )
    );
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn build_credit(a: &Args) -> Result<exif::Credit, Fault> {
    let licence_key = a.get("licence").or_else(|| a.get("license"));
    let (licence_name, licence_url) = match licence_key {
        None => (None, None),
        Some(key) => match snitch::licence(key) {
            Some((name, url)) => (Some(name), url),
            None => {
                return Err(Fault::Bare(format!(
                    "unknown licence '{key}'. One of: {}",
                    snitch::licence_keys()
                )));
            }
        },
    };
    let mut terms = a.get("terms").map(str::to_string);
    if terms.is_none() {
        terms = licence_name.map(|n| format!("{n}."));
    }
    if let (Some(t), Some(contact)) = (terms.clone(), a.get("contact")) {
        terms = Some(format!("{t} Commercial licensing: contact {contact}."));
    }
    Ok(exif::Credit {
        creator: a.get("creator").map(str::to_string),
        credit: a.get("credit").map(str::to_string),
        copyright: a.get("copyright").map(str::to_string),
        terms,
        rights_url: a
            .get("rights-url")
            .map(str::to_string)
            .or_else(|| licence_url.map(str::to_string)),
        licensor: a.get("credit").map(str::to_string),
        licensor_url: a.get("url").map(str::to_string),
        contact: a.get("contact").map(str::to_string),
        title: a.get("title").map(str::to_string),
        description: a.get("description").map(str::to_string),
        keywords: a.all("keyword"),
    })
}

fn one_file(
    a: &Args,
    source: &Path,
    target: &Path,
    credit: &exif::Credit,
    stamping: bool,
) -> Result<(), String> {
    // Everything happens on a sibling temporary and lands with one rename, so an interrupted run
    // never leaves a half-written file where a finished one is expected.
    let temporary = strip::temporary_sibling(target, true)?;
    let result = (|| -> Result<(), String> {
        if stamping {
            let options = stamp::Options {
                text: a.get("stamp").unwrap_or(""),
                subtext: a.get("stamp-sub"),
                logo: a.get("logo").map(Path::new),
                corner: a.get("corner").unwrap_or("bottom-right"),
                scale: a.parse_number("scale")?.unwrap_or(0.05),
                opacity: a.parse_number("opacity")?.unwrap_or(0.85),
                font: a.get("font").map(Path::new),
                quality: a.parse_number("quality")?.unwrap_or(94),
            };
            // The encoder picks its format from the extension, so the temporary has to keep the
            // target's, which `temporary_sibling(.., true)` guarantees.
            stamp::stamp(source, &temporary, &options)?;
        } else {
            std::fs::copy(source, &temporary)
                .map_err(|e| format!("{}: {e}", temporary.display()))?;
        }
        if !credit.is_empty() {
            exif::write_credit(&temporary, credit, !a.has("keep-gps"))?;
        } else if !a.has("keep-gps") {
            // Stamp-only still drops location, because that is the promise the flag makes.
            let empty = exif::Credit::default();
            let _ = exif::write_credit(&temporary, &empty, true);
        }
        std::fs::rename(&temporary, target).map_err(|e| format!("{}: {e}", target.display()))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

type MadeCert = Option<(PathBuf, PathBuf)>;

fn do_sign(a: &Args, target: &Path) -> Result<MadeCert, String> {
    let tool = sign::c2patool()
        .ok_or("c2patool is not installed, and signing needs it.\n  cargo install c2patool")?;
    let key = a
        .get("key")
        .map(PathBuf::from)
        .unwrap_or_else(sign::default_key);
    let cert = a
        .get("cert")
        .map(PathBuf::from)
        .unwrap_or_else(sign::default_cert);
    let org = a.get("credit").unwrap_or("snitch");
    let created = sign::ensure_cert(&key, &cert, org)?;
    let title = a.get("title").map(str::to_string).unwrap_or_else(|| {
        target
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    });
    let manifest = sign::manifest(&sign::ManifestFields {
        title: &title,
        description: a.get("description"),
        creator: a.get("creator"),
        org: a.get("credit"),
        url: a.get("url"),
        contact: a.get("contact"),
        licence: a.get("licence").or_else(|| a.get("license")),
        digital_source: a.get("digital-source"),
        generated: a.has("generated"),
    })?;
    sign::sign_file(target, &manifest, &key, &cert, &tool)?;
    Ok(created.then_some((key, cert)))
}

fn verify(a: &Args) -> ExitCode {
    let failed = cli::for_each_file(&a.files, |path| {
        let report = match inspect::inspect(path, None) {
            Ok(report) => report,
            Err(e) => {
                eprintln!("  {}: {e}", path.display());
                return true;
            }
        };
        let name = report.file.clone();
        match (&report.c2pa, report.c2pa_status) {
            (None, c2pa::Status::Absent) => {
                println!("  {name}: no Content Credential");
                true
            }
            (None, _) => {
                eprintln!("  {name}: C2PA check failed: {}", report.c2pa_error);
                true
            }
            (Some(manifest), _) => {
                let state = c2pa::validation_state(Some(manifest));
                let colour = if state == "Valid" { cli::GRN } else { cli::YEL };
                println!(
                    "  {name}  {}  certificate issuer {}  {}",
                    cli::c(&state, colour),
                    c2pa::signer(Some(manifest)).unwrap_or("?".into()),
                    c2pa::title(Some(manifest)).unwrap_or_default()
                );
                if c2pa::asset_altered(Some(manifest)) {
                    println!(
                        "{}",
                        cli::c(
                            "    ALTERED AFTER SIGNING: these pixels are not the ones that were signed",
                            cli::RED
                        )
                    );
                }
                if c2pa::identity_untrusted(Some(manifest)) {
                    println!(
                        "{}",
                        cli::c(
                            "    SIGNER IDENTITY UNTRUSTED (certificate not on validator trust list)",
                            cli::YEL
                        )
                    );
                }
                state != "Valid"
            }
        }
    });
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn resolve_targets(
    files: &[PathBuf],
    out: Option<&Path>,
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
                strip::outpath(&out.join(name), None, "-credited")
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
            .map(|f| strip::outpath(f, None, "-credited"))
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
