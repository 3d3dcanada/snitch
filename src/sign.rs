//! C2PA Content Credentials: signing.
//!
//! WHY THIS IS WORTH DOING AT ALL. A C2PA manifest carries tamper-evident provenance in the file.
//! LinkedIn documents a gradually deployed icon and metadata panel for inbound C2PA content, and
//! Meta and Google document reading some C2PA signals. Their handling varies by product and upload
//! path. LinkedIn does not document whether an untrusted self-signed credential from this tool
//! gets its UI.
//!
//! THE HONEST LIMIT, STATED HERE AND IN THE OUTPUT. A certificate this tool generates for you is
//! SELF-SIGNED. That is enough for a readable, tamper-evident development credential whose asset
//! binding can validate. It is NOT a conforming identity credential and is not on the C2PA trust
//! list, so a validator that checks issuers reports the signer as unknown rather than as you.
//! Being on that list means obtaining a certificate from a CA in the C2PA programme. Nothing here
//! can shortcut that, and a tool that implied otherwise would be lying to you.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

use crate::exif::which;

pub const CAMERA: &str = "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture";
pub const GENERATED: &str =
    "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia";

pub const DIGITAL_SOURCES: [(&str, &str); 8] = [
    ("camera", CAMERA),
    (
        "digital",
        "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCreation",
    ),
    (
        "screen",
        "http://cv.iptc.org/newscodes/digitalsourcetype/screenCapture",
    ),
    (
        "human-edited",
        "http://cv.iptc.org/newscodes/digitalsourcetype/humanEdits",
    ),
    ("generated", GENERATED),
    (
        "ai-edited",
        "http://cv.iptc.org/newscodes/digitalsourcetype/compositeWithTrainedAlgorithmicMedia",
    ),
    (
        "algorithmic",
        "http://cv.iptc.org/newscodes/digitalsourcetype/algorithmicMedia",
    ),
    (
        "data-driven",
        "http://cv.iptc.org/newscodes/digitalsourcetype/dataDrivenMedia",
    ),
];

pub fn digital_source_url(key: &str) -> Option<&'static str> {
    DIGITAL_SOURCES
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
}

pub fn digital_source_keys() -> String {
    DIGITAL_SOURCES
        .iter()
        .map(|(k, _)| *k)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Where the key and certificate live, per platform, matching the Python exactly so an existing
/// install keeps signing with the certificate it already made.
pub fn config_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    if cfg!(target_os = "windows") {
        let root = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Roaming"));
        return root.join("snitch");
    }
    if cfg!(target_os = "macos") {
        return home
            .join("Library")
            .join("Application Support")
            .join("snitch");
    }
    let root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    root.join("snitch")
}

pub fn default_key() -> PathBuf {
    config_dir().join("key.pem")
}

pub fn default_cert() -> PathBuf {
    config_dir().join("cert.pem")
}

pub fn c2patool() -> Option<PathBuf> {
    crate::c2pa::resolve_tool(None)
}

/// OpenSSL subject fields cannot carry a slash unescaped, and a studio name with one in it should
/// not silently become two fields.
fn subject_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('/', "\\/")
}

/// A P-256 signing certificate, made once. Returns true when it created one.
///
/// C2PA requires ES256 over prime256v1 for this algorithm, keyUsage digitalSignature and an
/// extendedKeyUsage of emailProtection. A certificate without those is refused by the signer
/// rather than producing a manifest that quietly fails to validate, which is the right way round.
pub fn ensure_cert(key: &Path, cert: &Path, org: &str) -> Result<bool, String> {
    if key.exists() && cert.exists() {
        return Ok(false);
    }
    if key.exists() || cert.exists() {
        let (present, missing) = if key.exists() {
            (key, cert)
        } else {
            (cert, key)
        };
        return Err(format!(
            "refusing to replace existing {}; matching file is missing: {}",
            present.display(),
            missing.display()
        ));
    }
    if which("openssl").is_none() {
        return Err("openssl is not installed, and generating a signing key needs it".into());
    }
    if let Some(parent) = key.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let key_tmp = crate::strip::temporary_sibling(key, false)?;
    let cert_tmp = crate::strip::temporary_sibling(cert, false)?;
    let cleanup = |a: &Path, b: &Path| {
        let _ = std::fs::remove_file(a);
        let _ = std::fs::remove_file(b);
    };

    let out = Command::new("openssl")
        .args([
            "ecparam",
            "-name",
            "prime256v1",
            "-genkey",
            "-noout",
            "-out",
        ])
        .arg(&key_tmp)
        .output()
        .map_err(|e| format!("could not run openssl: {e}"))?;
    if !out.status.success() {
        cleanup(&key_tmp, &cert_tmp);
        return Err(format!(
            "key generation failed: {}",
            String::from_utf8_lossy(&out.stderr)
                .chars()
                .take(300)
                .collect::<String>()
        ));
    }
    set_private_permissions(&key_tmp)?;

    let subject = subject_value(org);
    let out = Command::new("openssl")
        .args(["req", "-new", "-x509", "-utf8", "-key"])
        .arg(&key_tmp)
        .arg("-out")
        .arg(&cert_tmp)
        .args(["-days", "3650", "-sha256", "-subj"])
        .arg(format!("/CN={subject}/O={subject}"))
        .args([
            "-addext",
            "basicConstraints=critical,CA:FALSE",
            "-addext",
            "keyUsage=critical,digitalSignature",
            "-addext",
            "extendedKeyUsage=critical,emailProtection",
        ])
        .output()
        .map_err(|e| format!("could not run openssl: {e}"))?;
    if !out.status.success() {
        cleanup(&key_tmp, &cert_tmp);
        return Err(format!(
            "certificate generation failed: {}",
            String::from_utf8_lossy(&out.stderr)
                .chars()
                .take(300)
                .collect::<String>()
        ));
    }
    std::fs::rename(&key_tmp, key).map_err(|e| format!("{}: {e}", key.display()))?;
    std::fs::rename(&cert_tmp, cert).map_err(|e| format!("{}: {e}", cert.display()))?;
    Ok(true)
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> Result<(), String> {
    // Windows inherits the user's profile ACL, which is already private to the account.
    Ok(())
}

#[derive(Debug, Default, Clone)]
pub struct ManifestFields<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub creator: Option<&'a str>,
    pub org: Option<&'a str>,
    pub url: Option<&'a str>,
    pub contact: Option<&'a str>,
    pub licence: Option<&'a str>,
    pub digital_source: Option<&'a str>,
    pub generated: bool,
}

/// Build the manifest. Refuses to guess how the image was made, which is the point: a credential
/// that asserts camera capture for a generated image is worse than no credential.
pub fn manifest(fields: &ManifestFields) -> Result<Value, String> {
    let mut work = json!({
        "@context": "https://schema.org",
        "@type": "CreativeWork",
        "name": fields.title,
    });
    if let Some(v) = fields.description {
        work["description"] = json!(v);
    }
    if let Some(v) = fields.creator {
        work["author"] = json!([{"@type": "Person", "name": v}]);
    }
    if let Some(org) = fields.org {
        work["creator"] = json!([{"@type": "Organization", "name": org}]);
        work["copyrightHolder"] = json!([{"@type": "Organization", "name": org}]);
        work["creditText"] = json!(match fields.url {
            Some(url) => format!("{org} ({url})"),
            None => org.to_string(),
        });
    }
    if let Some(v) = fields.url {
        work["url"] = json!(v);
    }
    if let Some(contact) = fields.contact {
        if work.get("creator").is_none() {
            work["creator"] = json!([{}]);
        }
        work["creator"][0]["email"] = json!(contact);
    }
    if let Some(key) = fields.licence {
        if let Some((name, url)) = crate::licence(key) {
            if let Some(url) = url {
                work["license"] = json!(url);
            }
            work["usageInfo"] = json!(name);
        }
    }

    let source_key = if fields.generated {
        Some("generated")
    } else {
        fields.digital_source
    };
    let source = source_key
        .and_then(digital_source_url)
        .ok_or("a valid digital source is required for a C2PA created action")?;

    Ok(json!({
        "claim_generator_info": [{"name": "snitch", "version": crate::VERSION}],
        "title": fields.title,
        "assertions": [
            {"label": "stds.schema-org.CreativeWork", "data": work},
            {"label": "c2pa.actions", "data": {"actions": [{
                "action": "c2pa.created",
                "digitalSourceType": source,
                "softwareAgent": {"name": "snitch", "version": crate::VERSION},
            }]}},
        ],
    }))
}

/// Sign in place, atomically. The private key path goes in the manifest file, never the key
/// material, and the two C2PA environment variables are cleared so an ambient credential cannot
/// silently sign as somebody else.
pub fn sign_file(
    path: &Path,
    man: &Value,
    key: &Path,
    cert: &Path,
    tool: &Path,
) -> Result<(), String> {
    let source = path
        .canonicalize()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let manifest_path = crate::strip::temporary_sibling(&source.with_extension("json"), true)?;
    let output_path = crate::strip::temporary_sibling(&source, true)?;
    let cleanup = || {
        let _ = std::fs::remove_file(&manifest_path);
        let _ = std::fs::remove_file(&output_path);
    };

    let mut signing = man.clone();
    signing["alg"] = json!("es256");
    signing["private_key"] = json!(absolute(key).display().to_string());
    signing["sign_cert"] = json!(absolute(cert).display().to_string());
    if let Err(e) = std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&signing).unwrap_or_default(),
    ) {
        cleanup();
        return Err(format!("{}: {e}", manifest_path.display()));
    }

    let out = Command::new(tool)
        .arg(&source)
        .arg("-m")
        .arg(&manifest_path)
        .arg("-o")
        .arg(&output_path)
        .arg("-f")
        .env_remove("C2PA_PRIVATE_KEY")
        .env_remove("C2PA_SIGN_CERT")
        .output();
    let out = match out {
        Ok(out) => out,
        Err(e) => {
            cleanup();
            return Err(format!("could not run c2patool: {e}"));
        }
    };
    let wrote = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);
    if !out.status.success() || wrote == 0 {
        let mut text = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if text.is_empty() {
            text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        }
        cleanup();
        return Err(text.chars().take(400).collect());
    }
    if let Ok(metadata) = std::fs::metadata(&source) {
        let _ = std::fs::set_permissions(&output_path, metadata.permissions());
    }
    let result =
        std::fs::rename(&output_path, &source).map_err(|e| format!("{}: {e}", source.display()));
    let _ = std::fs::remove_file(&manifest_path);
    if result.is_err() {
        let _ = std::fs::remove_file(&output_path);
    }
    result
}

fn absolute(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| {
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(p)
        }
    })
}

/// The disclaimer that must accompany every signature this tool makes.
pub const SELF_SIGNED_NOTICE: &str = "  This is a development-grade self-signed credential. Its asset binding is\n  \
     tamper-evident, but it is not a conforming identity credential or on the\n  \
     C2PA trust list. Strict validators report the signer as unknown rather than you.\n  \
     Getting on that list requires a certificate from a CA in the C2PA programme.";
