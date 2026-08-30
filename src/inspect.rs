//! The structured report. The CLI formats it, the MCP server serialises it, and the data lives
//! here so neither of them has to reimplement it.

use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::{c2pa, exif, png};

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub file: String,
    pub path: String,
    pub bytes: u64,
    pub mime_type: Option<String>,
    /// Held as ordered pairs because the CLI prints them in the order the field table declares,
    /// and serialised as JSON objects because that is the shape the Python emitted and other
    /// things already parse.
    #[serde(serialize_with = "as_object")]
    pub camera: Vec<(String, Value)>,
    /// Full ExifTool keys, `EXIF:GPSLatitude` and friends. The renderer trims the group prefix;
    /// the data keeps it, because that is what the JSON contract says and what a caller can look
    /// up in the ExifTool tag directory.
    #[serde(serialize_with = "as_object")]
    pub gps: Vec<(String, Value)>,
    #[serde(serialize_with = "as_object")]
    pub credit: Vec<(String, Value)>,
    pub png_text: Vec<png::TextChunk>,
    pub generator_keywords: Vec<String>,
    pub c2pa: Option<Value>,
    pub c2pa_status: c2pa::Status,
    pub c2pa_error: String,
    /// "generative", "camera", or nothing.
    pub ai: Option<String>,
    /// Which signal answered: "c2pa" or "png-text-chunk". Never absent when `ai` is present.
    pub ai_source: Option<String>,
    pub has_any_credit: bool,
}

fn as_object<S: serde::Serializer>(pairs: &[(String, Value)], s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    let mut map = s.serialize_map(Some(pairs.len()))?;
    for (k, v) in pairs {
        map.serialize_entry(k, v)?;
    }
    map.end()
}

pub fn inspect(path: &Path, tool: Option<&str>) -> Result<Report, String> {
    let meta = exif::read_metadata(path)?;
    let mut report = c2pa::read_report(path, tool);
    c2pa::refine_with_metadata(&mut report, &meta);

    // The raw value is kept, not a rendered string: a latitude is a number in the Python's JSON
    // and a consumer that starts parsing strings back into floats is a contract this broke.
    let gps: Vec<(String, Value)> = exif::GPS_FIELDS
        .iter()
        .filter_map(|key| meta.get(*key).map(|v| ((*key).to_string(), v.clone())))
        .collect();
    let credit = exif::ordered_group(&meta, &exif::CREDIT_FIELDS);
    let camera = exif::ordered_group(&meta, &exif::CAMERA_FIELDS);

    let png_text = png::read_text(path);
    let mut generator_keywords: Vec<String> = png_text
        .iter()
        .filter(|c| c.is_generator())
        .map(|c| c.keyword.clone())
        .collect();
    generator_keywords.sort();
    generator_keywords.dedup();

    // A C2PA claim is cryptographically bound to the asset. A text chunk is plain text that anyone
    // can write and that survives being copied around. Both are worth reporting, they are not
    // worth the same, and the caller is told which one answered so a chunk can never pass for a
    // credential.
    let mut ai: Option<String> = None;
    let mut ai_source: Option<String> = None;
    if let Some(source) = c2pa::digital_source(report.manifest.as_ref()) {
        ai = Some(source.to_string());
        ai_source = Some("c2pa".into());
    }
    if ai.is_none() && !generator_keywords.is_empty() {
        ai = Some("generative".into());
        ai_source = Some("png-text-chunk".into());
    }

    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    Ok(Report {
        file: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string(),
        path: absolute.display().to_string(),
        bytes: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        mime_type: meta.get("File:MIMEType").map(exif::render),
        camera,
        gps,
        credit: credit.clone(),
        png_text,
        generator_keywords,
        c2pa: report.manifest,
        c2pa_status: report.status,
        c2pa_error: report.error,
        ai,
        ai_source,
        has_any_credit: !credit.is_empty(),
    })
}

/// Presence, integrity and signer trust as three separate answers, because they are three facts.
#[derive(Debug, Clone, Serialize)]
pub struct C2paSummary {
    pub manifest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_state: Option<String>,
    pub integrity: String,
    pub signer_trusted: String,
    pub signer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_status: Option<Value>,
    pub means: String,
}

pub fn c2pa_summary(report: &Report) -> C2paSummary {
    if report.c2pa_status != c2pa::Status::Present {
        let means = match report.c2pa_status {
            c2pa::Status::Absent => {
                "No content credential in this file. That is not evidence about how the image was \
                 made: most files have never had one."
                    .to_string()
            }
            c2pa::Status::DetectedUnverified => {
                "A credential is in this file but c2patool is not installed, so it could not be \
                 validated. Install c2patool to read it."
                    .to_string()
            }
            c2pa::Status::Unavailable => {
                "c2patool is not installed, so no credential could be read.".to_string()
            }
            _ => {
                if report.c2pa_error.is_empty() {
                    "the validator failed".to_string()
                } else {
                    report.c2pa_error.clone()
                }
            }
        };
        return C2paSummary {
            manifest: report.c2pa_status.as_str().to_string(),
            validation_state: None,
            integrity: "unknown".into(),
            signer_trusted: "unknown".into(),
            signer: None,
            title: None,
            validation_status: None,
            means,
        };
    }
    let manifest = report.c2pa.as_ref();
    let state = c2pa::validation_state(manifest);
    let altered = c2pa::asset_altered(manifest);
    let untrusted = c2pa::identity_untrusted(manifest);
    let means = if altered {
        "The pixels are not the ones that were signed: this image was altered after signing."
            .to_string()
    } else {
        let trust = if untrusted {
            "The signing certificate is not on the validator trust list, so this says nothing \
             about who signed it."
        } else {
            "The signer is on the validator trust list."
        };
        format!("The asset binding holds, so the image matches its credential. {trust}")
    };
    C2paSummary {
        manifest: "present".into(),
        integrity: if altered {
            "altered".into()
        } else if state == "Valid" {
            "valid".into()
        } else {
            "unknown".into()
        },
        signer_trusted: if untrusted {
            "no".into()
        } else if state == "Valid" {
            "yes".into()
        } else {
            "unknown".into()
        },
        validation_state: Some(state),
        signer: c2pa::signer(manifest),
        title: c2pa::title(manifest),
        validation_status: manifest.and_then(|m| m.get("validation_status")).cloned(),
        means,
    }
}
