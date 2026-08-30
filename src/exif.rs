//! The ExifTool bridge, and the one piece of TIFF this tool writes by hand.
//!
//! WHY A SUBPROCESS AND NOT A CRATE. Nothing in any language matches ExifTool's tag database. The
//! Rust EXIF crates read a useful subset and none of them write IPTC Core, XMP-plus and
//! XMP-iptcCore the way a picture desk expects. Shelling out is what the Python did, it is what
//! every serious tool in this space does, and the alternative is a narrower tool that quietly
//! misses fields. The cost is one process per call and a stated dependency.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use serde_json::{Map, Value};

pub type Metadata = Map<String, Value>;

pub fn have(tool: &str) -> bool {
    which(tool).is_some()
}

/// `which`, without a crate. PATH lookup is a dozen lines and the alternative is a dependency.
pub fn which(tool: &str) -> Option<std::path::PathBuf> {
    if tool.contains('/') {
        let direct = std::path::PathBuf::from(tool);
        return direct.is_file().then_some(direct);
    }
    std::env::var_os("PATH")?
        .to_str()?
        .split(':')
        .filter(|dir| !dir.is_empty())
        .map(|dir| std::path::Path::new(dir).join(tool))
        .find(|candidate| candidate.is_file())
}

pub fn require(tool: &str, why: &str) -> Result<(), String> {
    if have(tool) {
        return Ok(());
    }
    let debian = if tool == "exiftool" {
        "libimage-exiftool-perl"
    } else {
        tool
    };
    Err(format!(
        "{tool} is not installed, and {why}.\n  \
         Debian/Ubuntu:  sudo apt install {debian}\n  \
         macOS:          brew install {tool}\n  \
         Windows:        choco install {tool}"
    ))
}

/// Everything ExifTool can see, as a map. The flags match the Python exactly: `-j` JSON, `-G`
/// group prefixes, `-n` numeric values so GPS is a number and not a rendered string, `-a`
/// duplicate tags, `-u` unknown tags.
///
/// THIS CALL IS THE TOOL'S MEMORY CEILING, and it is ExifTool's ceiling rather than ours. A 510 KB
/// PNG holding one zTXt chunk of compressed zeros costs ExifTool 1.6 GB and three seconds on its
/// own, measured, and `snitch` on the same file costs the same because everything above this line
/// is bounded: `png::read_text` handles that file in about 5 MB. Anything that shells out to
/// ExifTool inherits this, including the Python this was ported from, which cost 6.8 GB for the
/// same input. Bounding it would mean not asking ExifTool for everything, which changes what the
/// tool can report, so it is documented here rather than quietly traded away.
pub fn read_metadata(path: &Path) -> Result<Metadata, String> {
    let path = path
        .canonicalize()
        .map_err(|_| "not a regular file".to_string())?;
    if !path.is_file() {
        return Err("not a regular file".into());
    }
    require("exiftool", "reading metadata needs it")?;
    let out = Command::new("exiftool")
        .args(["-j", "-G", "-n", "-a", "-u"])
        .arg(&path)
        .output()
        .map_err(|e| format!("could not run exiftool: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.trim().is_empty() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "ExifTool could not read this file".into()
        } else {
            truncate(&err, 300)
        });
    }
    let reports: Vec<Value> =
        serde_json::from_str(&stdout).map_err(|e| format!("invalid ExifTool JSON: {e}"))?;
    let metadata = reports
        .into_iter()
        .next()
        .and_then(|v| match v {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .ok_or_else(|| "invalid ExifTool JSON: no report".to_string())?;
    if let Some(error) = metadata.get("ExifTool:Error") {
        return Err(render(error));
    }
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("ExifTool exited {}", out.status.code().unwrap_or(-1))
        } else {
            truncate(&err, 300)
        });
    }
    let mime = metadata
        .get("File:MIMEType")
        .map(render)
        .unwrap_or_default();
    if !mime.starts_with("image/") {
        let kind = metadata
            .get("File:FileType")
            .map(render)
            .unwrap_or_else(|| "unknown".into());
        return Err(format!("not an image (ExifTool identified {kind})"));
    }
    Ok(metadata)
}

/// A metadata value as a person would read it. Numbers keep their shape, lists join with a comma.
pub fn render(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(render).collect::<Vec<_>>().join(", "),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// EXIF orientation, only when it is one of the values that actually rotates or mirrors.
/// 1 means "as stored", so reinserting it would add a segment that changes nothing.
pub fn orientation_value(meta: &Metadata) -> Option<u16> {
    let raw = meta.get("EXIF:Orientation")?;
    let n = match raw {
        Value::Number(n) => n.as_u64()?,
        Value::String(s) => s.parse().ok()?,
        _ => return None,
    };
    (2..=8).contains(&n).then_some(n as u16)
}

/// A minimal big-endian TIFF block carrying nothing but tag 274, prefixed `Exif\0\0`.
///
/// Byte-identical to what `PIL.Image.Exif().tobytes()` produced for the Python version, checked
/// against a captured sample, because a strip that silently changes the orientation payload
/// changes how the image displays.
pub fn orientation_payload(orientation: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(32);
    out.extend_from_slice(b"Exif\x00\x00");
    out.extend_from_slice(b"MM\x00\x2a"); // big endian, TIFF magic 42
    out.extend_from_slice(&8u32.to_be_bytes()); // offset of IFD0
    out.extend_from_slice(&1u16.to_be_bytes()); // one entry
    out.extend_from_slice(&0x0112u16.to_be_bytes()); // Orientation
    out.extend_from_slice(&3u16.to_be_bytes()); // SHORT
    out.extend_from_slice(&1u32.to_be_bytes()); // count
    out.extend_from_slice(&orientation.to_be_bytes()); // value, left aligned in its four bytes
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&0u32.to_be_bytes()); // no next IFD
    out
}

/// The credit fields a picture desk and Google actually read.
#[derive(Debug, Default, Clone)]
pub struct Credit {
    pub creator: Option<String>,
    pub credit: Option<String>,
    pub copyright: Option<String>,
    pub terms: Option<String>,
    pub rights_url: Option<String>,
    pub licensor: Option<String>,
    pub licensor_url: Option<String>,
    pub contact: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
}

impl Credit {
    pub fn is_empty(&self) -> bool {
        self.creator.is_none()
            && self.credit.is_none()
            && self.copyright.is_none()
            && self.terms.is_none()
            && self.rights_url.is_none()
            && self.licensor.is_none()
            && self.licensor_url.is_none()
            && self.contact.is_none()
            && self.title.is_none()
            && self.description.is_none()
            && self.keywords.is_empty()
    }

    /// The ExifTool arguments this credit becomes. Separated from running them so a test can
    /// assert the exact tag set without touching a file.
    pub fn args(&self, drop_gps: bool) -> Vec<String> {
        let mut a: Vec<String> = Vec::new();
        let mut add = |s: String| a.push(s);

        let writes_iptc = self.creator.is_some()
            || self.credit.is_some()
            || self.copyright.is_some()
            || self.title.is_some()
            || self.description.is_some()
            || !self.keywords.is_empty();
        if writes_iptc {
            add("-IPTC:CodedCharacterSet=UTF8".into());
        }
        if let Some(v) = &self.creator {
            add(format!("-XMP-dc:Creator={v}"));
            add(format!("-IPTC:By-line={v}"));
            add(format!("-EXIF:Artist={v}"));
        }
        if let Some(v) = &self.credit {
            add(format!("-XMP-photoshop:Credit={v}"));
            add(format!("-IPTC:Credit={v}"));
            add(format!("-XMP-photoshop:Source={v}"));
            add(format!("-IPTC:Source={v}"));
        }
        if let Some(v) = &self.copyright {
            add(format!("-XMP-dc:Rights={v}"));
            add(format!("-IPTC:CopyrightNotice={v}"));
            add(format!("-EXIF:Copyright={v}"));
            add("-XMP-xmpRights:Marked=True".into());
        }
        if let Some(v) = &self.terms {
            add(format!("-XMP-xmpRights:UsageTerms={v}"));
        }
        if let Some(v) = &self.rights_url {
            add(format!("-XMP-xmpRights:WebStatement={v}"));
        }
        if let Some(v) = &self.licensor {
            add(format!("-XMP-plus:LicensorName={v}"));
        }
        if let Some(v) = &self.licensor_url {
            add(format!("-XMP-plus:LicensorURL={v}"));
        }
        if let Some(v) = &self.contact {
            let key = if v.contains('@') {
                "CreatorWorkEmail"
            } else {
                "CreatorWorkURL"
            };
            add(format!("-XMP-iptcCore:{key}={v}"));
        }
        if let Some(v) = &self.title {
            add(format!("-XMP-dc:Title={v}"));
            add(format!("-IPTC:ObjectName={v}"));
            add(format!("-IPTC:Headline={v}"));
        }
        if let Some(v) = &self.description {
            add(format!("-XMP-dc:Description={v}"));
            add(format!("-IPTC:Caption-Abstract={v}"));
        }
        for k in &self.keywords {
            add(format!("-XMP-dc:Subject+={k}"));
            add(format!("-IPTC:Keywords+={k}"));
        }
        if drop_gps {
            add("-gps:all=".into());
        }
        a
    }
}

/// Write credit into the file given. This MUTATES the target, so every caller that must not touch
/// a user's input copies first and edits the copy.
pub fn write_credit(path: &Path, credit: &Credit, drop_gps: bool) -> Result<(), String> {
    require("exiftool", "writing metadata needs it")?;
    let fields = credit.args(drop_gps);
    if fields.is_empty() {
        return Err("nothing to write".into());
    }
    let out = Command::new("exiftool")
        .args(["-overwrite_original", "-q", "-P"])
        .args(&fields)
        .arg(path)
        .output()
        .map_err(|e| format!("could not run exiftool: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(if err.is_empty() {
        "exiftool failed".into()
    } else {
        truncate(&err, 300)
    })
}

/// Field groups, ported unchanged. The order inside each list is the order of preference: the
/// first tag that has a value wins, so XMP beats IPTC beats EXIF where they disagree.
pub const CREDIT_FIELDS: [(&str, &[&str]); 10] = [
    ("Creator", &["XMP:Creator", "IPTC:By-line", "EXIF:Artist"]),
    ("Credit", &["XMP:Credit", "IPTC:Credit"]),
    (
        "Copyright",
        &["XMP:Rights", "IPTC:CopyrightNotice", "EXIF:Copyright"],
    ),
    ("Usage terms", &["XMP:UsageTerms"]),
    ("Rights URL", &["XMP:WebStatement"]),
    ("Licensor", &["XMP:LicensorName", "XMP:LicensorURL"]),
    ("Contact", &["XMP:CreatorWorkEmail", "XMP:CreatorWorkURL"]),
    ("Title", &["XMP:Title", "IPTC:ObjectName"]),
    ("Description", &["XMP:Description", "IPTC:Caption-Abstract"]),
    ("Keywords", &["XMP:Subject", "IPTC:Keywords"]),
];

pub const GPS_FIELDS: [&str; 4] = [
    "EXIF:GPSLatitude",
    "EXIF:GPSLongitude",
    "Composite:GPSPosition",
    "EXIF:GPSAltitude",
];

pub const CAMERA_FIELDS: [(&str, &[&str]); 4] = [
    ("Camera", &["EXIF:Make", "EXIF:Model"]),
    ("Lens", &["EXIF:LensModel", "EXIF:LensInfo"]),
    ("Taken", &["EXIF:DateTimeOriginal", "EXIF:CreateDate"]),
    ("Software", &["EXIF:Software", "XMP:CreatorTool"]),
];

/// The first key in `keys` that has a non-empty value.
pub fn first(meta: &Metadata, keys: &[&str]) -> Option<String> {
    for key in keys {
        match meta.get(*key) {
            None | Some(Value::Null) => continue,
            Some(Value::String(s)) if s.is_empty() => continue,
            Some(Value::Array(a)) if a.is_empty() => continue,
            Some(v) => return Some(render(v)),
        }
    }
    None
}

/// Pull a labelled group out of a metadata map, preserving the declared field order.
pub fn group(meta: &Metadata, fields: &[(&str, &[&str])]) -> BTreeMap<String, String> {
    // BTreeMap would reorder the labels, so the caller gets an ordered vector instead and this
    // stays available for the cases where a plain lookup is what is wanted.
    fields
        .iter()
        .filter_map(|(label, keys)| first(meta, keys).map(|v| (label.to_string(), v)))
        .collect()
}

/// The same thing, in declaration order, which is what the CLI prints. Values stay as they came
/// out of ExifTool so a number is still a number in the JSON.
pub fn ordered_group(meta: &Metadata, fields: &[(&str, &[&str])]) -> Vec<(String, Value)> {
    fields
        .iter()
        .filter_map(|(label, keys)| first_value(meta, keys).map(|v| (label.to_string(), v)))
        .collect()
}

pub fn first_value(meta: &Metadata, keys: &[&str]) -> Option<Value> {
    for key in keys {
        match meta.get(*key) {
            None | Some(Value::Null) => continue,
            Some(Value::String(s)) if s.is_empty() => continue,
            Some(Value::Array(a)) if a.is_empty() => continue,
            Some(v) => return Some(v.clone()),
        }
    }
    None
}
