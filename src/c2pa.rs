//! C2PA Content Credentials: reading, and keeping three different facts apart.
//!
//! PRESENCE, INTEGRITY AND IDENTITY ARE NOT THE SAME QUESTION, and merging them is how tools in
//! this space mislead people. A credential can be present and its asset binding valid while the
//! signer is completely unknown. A missing credential is not evidence an image is real: most
//! files never had one. And "we could not check" is a fourth answer that must never collapse into
//! "there is nothing there".
//!
//! WHY c2patool AND NOT THE c2pa CRATE. The library is 280 crates, measured with `cargo tree` on
//! 2026-08-30. c2patool is that library already compiled, it is what the Python called, and it is
//! a single binary a user installs with `cargo install c2patool`. Six times the dependency tree to
//! remove one subprocess is not a trade this codebase makes.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::exif::{Metadata, which};

/// What is known about a credential. These are five distinct answers and the fifth is the one
/// most tools get wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Read and validated.
    Present,
    /// The file genuinely has no credential.
    Absent,
    /// c2patool is not installed and nothing in the container suggested a credential.
    Unavailable,
    /// A credential IS in the container, seen through ExifTool, but c2patool is not installed so
    /// it could not be validated. Reporting this as `Absent` would be a lie.
    DetectedUnverified,
    /// The validator ran and failed.
    Error,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Present => "present",
            Status::Absent => "absent",
            Status::Unavailable => "unavailable",
            Status::DetectedUnverified => "detected-unverified",
            Status::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Report {
    pub status: Status,
    pub manifest: Option<Value>,
    pub error: String,
}

/// PATH first, then the place `cargo install` puts it, which is where it actually lives on a
/// developer machine that installed it the documented way.
pub fn resolve_tool(explicit: Option<&str>) -> Option<PathBuf> {
    if let Some(tool) = explicit {
        return which(tool);
    }
    if let Some(found) = which("c2patool") {
        return Some(found);
    }
    let cargo = home()?.join(".cargo/bin/c2patool");
    cargo.is_file().then_some(cargo)
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn available() -> bool {
    resolve_tool(None).is_some()
}

/// Read a report without conflating no tool, no claim, and failure.
pub fn read_report(path: &Path, tool: Option<&str>) -> Report {
    let Some(tool) = resolve_tool(tool) else {
        return Report {
            status: Status::Unavailable,
            manifest: None,
            error: "c2patool is not installed".into(),
        };
    };
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let out = match Command::new(&tool).arg(&path).output() {
        Ok(out) => out,
        Err(e) => {
            return Report {
                status: Status::Error,
                manifest: None,
                error: format!("could not run c2patool: {e}"),
            };
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() {
        let mut text = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if text.is_empty() {
            text = stdout.trim().to_string();
        }
        if text.contains("No claim found") {
            return Report {
                status: Status::Absent,
                manifest: None,
                error: String::new(),
            };
        }
        let error = if text.is_empty() {
            format!("c2patool exited {}", out.status.code().unwrap_or(-1))
        } else {
            text.chars().take(300).collect()
        };
        return Report {
            status: Status::Error,
            manifest: None,
            error,
        };
    }
    if stdout.trim().is_empty() {
        return Report {
            status: Status::Error,
            manifest: None,
            error: "c2patool returned no report".into(),
        };
    }
    match serde_json::from_str::<Value>(&stdout) {
        Ok(manifest) => Report {
            status: Status::Present,
            manifest: Some(manifest),
            error: String::new(),
        },
        Err(e) => Report {
            status: Status::Error,
            manifest: None,
            error: format!("invalid c2patool JSON: {e}"),
        },
    }
}

/// Upgrade `Unavailable` to `DetectedUnverified` when ExifTool saw a JUMBF box even though the
/// validator could not run. This is the whole reason the fifth state exists.
pub fn refine_with_metadata(report: &mut Report, meta: &Metadata) {
    if report.status == Status::Unavailable
        && meta.get("JUMBF:JUMDLabel").and_then(Value::as_str) == Some("c2pa")
    {
        report.status = Status::DetectedUnverified;
    }
}

fn status_codes(manifest: Option<&Value>) -> Vec<&str> {
    manifest
        .and_then(|m| m.get("validation_status"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|s| s.get("code").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default()
}

/// A hash mismatch is the validator saying the bytes it is looking at are not the bytes that were
/// signed. That is a different fact from an untrusted signer, and the two must never be merged:
/// one means the image changed, the other means we cannot say who signed it.
pub const ALTERED_CODES: [&str; 3] = [
    "assertion.dataHash.mismatch",
    "assertion.bmffHash.mismatch",
    "assertion.boxesHash.mismatch",
];

pub fn asset_altered(manifest: Option<&Value>) -> bool {
    status_codes(manifest)
        .iter()
        .any(|c| ALTERED_CODES.contains(c))
}

pub fn identity_untrusted(manifest: Option<&Value>) -> bool {
    status_codes(manifest).contains(&"signingCredential.untrusted")
}

pub fn validation_state(manifest: Option<&Value>) -> String {
    manifest
        .and_then(|m| m.get("validation_state"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string()
}

fn active_manifest(manifest: &Value) -> Option<&Value> {
    let active = manifest.get("active_manifest")?.as_str()?;
    manifest.get("manifests")?.get(active)
}

pub fn signer(manifest: Option<&Value>) -> Option<String> {
    let man = active_manifest(manifest?)?;
    man.get("signature_info")?
        .get("issuer")?
        .as_str()
        .map(str::to_string)
}

pub fn title(manifest: Option<&Value>) -> Option<String> {
    active_manifest(manifest?)?
        .get("title")?
        .as_str()
        .map(str::to_string)
}

/// What the credential says about how the image was made, read from the c2pa.actions assertion.
/// Returns "generative", "camera", or nothing.
pub fn digital_source(manifest: Option<&Value>) -> Option<&'static str> {
    let man = active_manifest(manifest?)?;
    let assertions = man.get("assertions")?.as_array()?;
    let mut answer: Option<&'static str> = None;
    for assertion in assertions {
        let label = assertion.get("label").and_then(Value::as_str).unwrap_or("");
        if !label.contains("action") {
            continue;
        }
        let Some(actions) = assertion
            .get("data")
            .and_then(|d| d.get("actions"))
            .and_then(Value::as_array)
        else {
            continue;
        };
        for action in actions {
            let src = action
                .get("digitalSourceType")
                .and_then(Value::as_str)
                .unwrap_or("");
            if src.contains("trainedAlgorithmicMedia")
                || src.contains("compositeWithTrainedAlgorithmic")
            {
                answer = Some("generative");
            } else if src.contains("digitalCapture") && answer.is_none() {
                answer = Some("camera");
            }
        }
    }
    answer
}
