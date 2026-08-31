//! A local stdio MCP server, hand-rolled, with no async runtime.
//!
//! WHY NOT THE OFFICIAL SDK. `rmcp` is 59 crates and brings tokio, measured with `cargo tree` on
//! 2026-08-30. MCP over stdio is newline-delimited JSON-RPC 2.0 read from one pipe and written to
//! another, which is a blocking read loop and about two hundred lines against serde_json, which is
//! already a dependency. The house rule is no async runtime for a loopback, single-operator
//! process, and this is exactly that: one client, one request at a time, over a pipe.
//!
//! STDOUT IS THE PROTOCOL WIRE. Nothing in this module may print to it except a response frame.
//! Every subprocess this tool spawns is captured with `Command::output()` rather than inherited,
//! and the CLI's colour and banner code is never linked in here. Diagnostics go to stderr, which
//! is where MCP hosts already look: Claude Desktop writes each server's stderr to
//! `mcp-server-NAME.log` precisely because stdio servers use it for all their logging.
//!
//! NOTHING IS MUTATED IN PLACE. `exif::write_credit` edits the file it is given, so the credit
//! tool copies to a new file first and edits the copy. A model calling a tool cannot be assumed to
//! have understood that its input was about to be overwritten, so it never is.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::{exif, inspect, strip, text};

/// The protocol revisions this server can speak, oldest first.
///
/// THE HANDSHAKE DOES NOT NEGOTIATE THE NEWEST VERSION. `2026-07-28` exists and is what the
/// official SDK calls its latest, but it is adopted AFTER initialize through a separate
/// mechanism; the `initialize` exchange itself only settles one of these. Answering the handshake
/// with `2026-07-28` makes a conforming client refuse the session outright with "unsupported
/// protocol version from the server", which is exactly what happened here before this list
/// existed and is why it is a list and not a constant.
pub const HANDSHAKE_VERSIONS: [&str; 4] = ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"];

/// What this server answers with when the client asks for something it does not know.
pub const PROTOCOL_VERSION: &str = "2025-11-25";

/// Echo the client's version when it is one we speak, otherwise state ours and let the client
/// decide whether it can proceed. That is what the specification asks for, in that order.
pub fn negotiate(requested: Option<&str>) -> &'static str {
    match requested {
        Some(asked) => HANDSHAKE_VERSIONS
            .iter()
            .find(|known| **known == asked)
            .copied()
            .unwrap_or(PROTOCOL_VERSION),
        None => PROTOCOL_VERSION,
    }
}

pub const SUPPORTED_STRIP: &str =
    "JPEG and PNG only. WebP, HEIC and AVIF are not supported for lossless stripping.";

/// A refusal the caller should see as a sentence, not as a crash.
#[derive(Debug)]
pub struct ToolError(pub String);

impl From<String> for ToolError {
    fn from(s: String) -> Self {
        ToolError(s)
    }
}

impl From<&str> for ToolError {
    fn from(s: &str) -> Self {
        ToolError(s.to_string())
    }
}

type ToolResult = Result<Value, ToolError>;

pub fn log(message: &str) {
    eprintln!("snitch-mcp: {message}");
    let _ = std::io::stderr().flush();
}

// ----------------------------------------------------------------------------------------------
// guards
// ----------------------------------------------------------------------------------------------

/// Resolve an input path, or refuse it. Ordinary readable files only.
pub fn source_path(raw: &str) -> Result<PathBuf, ToolError> {
    if raw.trim().is_empty() {
        return Err("no path given".into());
    }
    let expanded = expand_home(raw);
    if !expanded.exists() {
        return Err(format!("{}: not found", expanded.display()).into());
    }
    // Following a link to read is fine. Writing through one is how a caller destroys something it
    // did not name, so the whole class is refused rather than special-cased per tool.
    if expanded
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(format!(
            "{}: is a symbolic link. Pass the real file.",
            expanded.display()
        )
        .into());
    }
    if !expanded.is_file() {
        return Err(format!("{}: not a regular file", expanded.display()).into());
    }
    Ok(expanded)
}

/// Resolve an output path that cannot be the source and cannot be silently clobbered.
pub fn target_path(
    source: &Path,
    out: &str,
    suffix: &str,
    force: bool,
) -> Result<PathBuf, ToolError> {
    let target = if out.trim().is_empty() {
        strip::outpath(source, None, suffix)
    } else {
        expand_home(out)
    };
    if target.is_dir() {
        return Err(format!("{}: is a directory", target.display()).into());
    }
    if strip::same_file(source, &target) {
        return Err("output would overwrite the source. Give a different out_path.".into());
    }
    if target.exists() && !force {
        return Err(format!(
            "{}: exists. Pass force=true to replace it.",
            target.display()
        )
        .into());
    }
    let parent = target.parent().filter(|p| !p.as_os_str().is_empty());
    match parent {
        Some(parent) if !parent.is_dir() => {
            Err(format!("{}: output directory does not exist", parent.display()).into())
        }
        _ => Ok(target),
    }
}

fn expand_home(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(raw)
}

/// Name what actually went, by comparing the two reports rather than by assuming.
fn removed_classes(before: &inspect::Report, after: &inspect::Report) -> Vec<&'static str> {
    let mut gone = Vec::new();
    if !before.gps.is_empty() && after.gps.is_empty() {
        gone.push("location");
    }
    if !before.camera.is_empty() && after.camera.is_empty() {
        gone.push("camera");
    }
    if !before.credit.is_empty() && after.credit.is_empty() {
        gone.push("credit");
    }
    if !before.png_text.is_empty() && after.png_text.is_empty() {
        gone.push("png text chunks");
    }
    let had = matches!(
        before.c2pa_status,
        crate::c2pa::Status::Present | crate::c2pa::Status::DetectedUnverified
    );
    if had && after.c2pa_status == crate::c2pa::Status::Absent {
        gone.push("C2PA content credential");
    }
    gone
}

// ----------------------------------------------------------------------------------------------
// the tools
// ----------------------------------------------------------------------------------------------

fn arg_str(args: &Map<String, Value>, key: &str) -> String {
    args.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn arg_bool(args: &Map<String, Value>, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn optional(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

fn snitch_inspect(args: &Map<String, Value>) -> ToolResult {
    let path = source_path(&arg_str(args, "path"))?;
    let report = inspect::inspect(&path, None).map_err(ToolError)?;
    let summary = inspect::c2pa_summary(&report);
    let mut value = serde_json::to_value(&report).map_err(|e| ToolError(e.to_string()))?;
    value["c2pa_summary"] = serde_json::to_value(summary).map_err(|e| ToolError(e.to_string()))?;
    Ok(value)
}

fn snitch_strip_metadata(args: &Map<String, Value>) -> ToolResult {
    let path = source_path(&arg_str(args, "path"))?;
    let before = inspect::inspect(&path, None).map_err(ToolError)?;
    let target = target_path(
        &path,
        &arg_str(args, "out_path"),
        "-clean",
        arg_bool(args, "force"),
    )?;
    let (removed, identical) = strip::strip_atomic(&path, &target).map_err(ToolError)?;
    let after = inspect::inspect(&target, None).map_err(ToolError)?;
    Ok(json!({
        "output_path": target.display().to_string(),
        "format": after.mime_type,
        "bytes_before": before.bytes,
        "bytes_after": after.bytes,
        "bytes_removed": removed,
        "removed": removed_classes(&before, &after),
        "pixels_identical": identical,
        "proof": "Decoded pixels compared by SHA-256 before and after. This is a per-run check, \
                  not a claim about the algorithm.",
        "watermarks": "In-pixel watermarks are untouched. Metadata stripping cannot reach them.",
    }))
}

fn snitch_add_credit(args: &Map<String, Value>) -> ToolResult {
    let path = source_path(&arg_str(args, "path"))?;
    let credit = exif::Credit {
        creator: optional(arg_str(args, "creator")),
        credit: optional(arg_str(args, "credit")),
        copyright: optional(arg_str(args, "copyright")),
        terms: optional(arg_str(args, "terms")),
        rights_url: optional(arg_str(args, "rights_url")),
        contact: optional(arg_str(args, "contact")),
        title: optional(arg_str(args, "title")),
        description: optional(arg_str(args, "description")),
        ..Default::default()
    };
    if credit.is_empty() {
        return Err(
            "nothing to write: give at least one of creator, credit, copyright, terms, \
                    rights_url, contact, title or description"
                .into(),
        );
    }
    let keep_gps = arg_bool(args, "keep_gps");
    let target = target_path(
        &path,
        &arg_str(args, "out_path"),
        "-credited",
        arg_bool(args, "force"),
    )?;
    let bytes_before = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    // Copy first, then edit the copy. write_credit mutates whatever file it is handed, and the
    // caller's input is not ours to change.
    let temporary = strip::temporary_sibling(&target, true).map_err(ToolError)?;
    let result = (|| -> Result<(), String> {
        std::fs::copy(&path, &temporary).map_err(|e| format!("{}: {e}", temporary.display()))?;
        exif::write_credit(&temporary, &credit, !keep_gps)?;
        std::fs::rename(&temporary, &target).map_err(|e| format!("{}: {e}", target.display()))
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&temporary);
        return Err(ToolError(e));
    }

    let after = inspect::inspect(&target, None).map_err(ToolError)?;
    Ok(json!({
        "output_path": target.display().to_string(),
        "format": after.mime_type,
        "bytes_before": bytes_before,
        "bytes_after": after.bytes,
        "written": after.credit,
        "gps_kept": !after.gps.is_empty(),
        "note": "Written into metadata, which survives ordinary copying but is removed by many \
                 upload paths. A visible stamp is the version that survives stripping.",
    }))
}

fn snitch_verify_c2pa(args: &Map<String, Value>) -> ToolResult {
    let path = source_path(&arg_str(args, "path"))?;
    let report = inspect::inspect(&path, None).map_err(ToolError)?;
    let summary = inspect::c2pa_summary(&report);
    let mut value = serde_json::to_value(summary).map_err(|e| ToolError(e.to_string()))?;
    value["file"] = json!(report.file);
    Ok(value)
}

fn snitch_clean_text(args: &Map<String, Value>) -> ToolResult {
    let input = arg_str(args, "text_input");
    serde_json::to_value(text::clean(&input)).map_err(|e| ToolError(e.to_string()))
}

struct Tool {
    name: &'static str,
    description: &'static str,
    schema: fn() -> Value,
    run: fn(&Map<String, Value>) -> ToolResult,
}

fn path_only() -> Value {
    json!({
        "type": "object",
        "properties": {"path": {"type": "string", "description": "Absolute path to the image file."}},
        "required": ["path"],
    })
}

fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "snitch_inspect",
            description: "Report everything an image file is saying about its owner: GPS location, \
                          camera, credit and copyright, PNG text chunks a generator wrote, and any \
                          C2PA content credential. Reads the file, never changes it.",
            schema: path_only,
            run: snitch_inspect,
        },
        Tool {
            name: "snitch_strip_metadata",
            description: concat!(
                "Remove metadata from an image without re-encoding it, so the decoded pixels come \
                 out byte-identical. ",
                "JPEG and PNG only. WebP, HEIC and AVIF are not supported for lossless stripping. ",
                "Writes a new file and leaves the input untouched. Does not remove in-pixel \
                 watermarks such as SynthID: those live in the image data and no metadata tool can \
                 reach them.",
            ),
            schema: || {
                json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Absolute path to the image file."},
                        "out_path": {"type": "string", "description": "Where to write. Defaults to the input with a -clean suffix."},
                        "force": {"type": "boolean", "description": "Replace out_path if it already exists."},
                    },
                    "required": ["path"],
                })
            },
            run: snitch_strip_metadata,
        },
        Tool {
            name: "snitch_add_credit",
            description: "Write creator, credit, copyright and licence into an image's IPTC and \
                          XMP fields, into a new file. Location data is dropped unless keep_gps is \
                          true, because publishing coordinates deanonymises people. Does not sign \
                          a C2PA credential and does not stamp the pixels.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Absolute path to the image file."},
                        "creator": {"type": "string", "description": "The person who made it."},
                        "credit": {"type": "string", "description": "The studio or organisation to credit."},
                        "copyright": {"type": "string"},
                        "terms": {"type": "string", "description": "Usage terms."},
                        "rights_url": {"type": "string"},
                        "contact": {"type": "string", "description": "Email or URL for licensing enquiries."},
                        "title": {"type": "string"},
                        "description": {"type": "string"},
                        "keep_gps": {"type": "boolean", "description": "Keep location data. Off by default, because it doxxes people."},
                        "out_path": {"type": "string"},
                        "force": {"type": "boolean"},
                    },
                    "required": ["path"],
                })
            },
            run: snitch_add_credit,
        },
        Tool {
            name: "snitch_verify_c2pa",
            description: "Check an image's C2PA content credential. Reports presence, \
                          asset-binding integrity and signer trust as three separate answers, \
                          because they are three different facts. A missing credential is not \
                          evidence that an image is real, and a self-signed certificate is not \
                          proof of who signed it.",
            schema: path_only,
            run: snitch_verify_c2pa,
        },
        Tool {
            name: "snitch_clean_text",
            description: "Find hidden characters in text: invisible zero-width marks, \
                          right-to-left overrides, letters from mixed alphabets, unusual spaces \
                          and word-processor substitutions. Returns the text with the tracking \
                          characters removed. Never touches ZWJ or ZWNJ, which emoji sequences and \
                          Arabic, Persian, Kurdish and Indic writing depend on.",
            schema: || {
                json!({
                    "type": "object",
                    "properties": {"text_input": {"type": "string", "description": "The text to check."}},
                    "required": ["text_input"],
                })
            },
            run: snitch_clean_text,
        },
    ]
}

/// The tool list as the protocol wants it. Public so a test can assert the descriptions without
/// opening a session.
pub fn tool_descriptors() -> Vec<Value> {
    tools()
        .into_iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": (t.schema)(),
            })
        })
        .collect()
}

pub fn call_tool(name: &str, args: &Map<String, Value>) -> ToolResult {
    match tools().into_iter().find(|t| t.name == name) {
        Some(tool) => (tool.run)(args),
        None => Err(format!("unknown tool: {name}").into()),
    }
}

// ----------------------------------------------------------------------------------------------
// JSON-RPC 2.0 over stdio
// ----------------------------------------------------------------------------------------------

fn result(id: Value, value: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": value})
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// Handle one parsed request. `None` means it was a notification and gets no reply.
pub fn handle(request: &Value) -> Option<Value> {
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let id = request.get("id").cloned();
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

    // A notification has no id. Answering one is a protocol violation, so it never happens here.
    let id = id?;

    match method {
        "initialize" => Some(result(
            id,
            json!({
                "protocolVersion": negotiate(
                    params.get("protocolVersion").and_then(Value::as_str)
                ),
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "snitch", "version": crate::VERSION},
                "instructions": "Read, strip and credit image metadata, and check text for hidden \
                                 characters. Everything runs locally. Nothing is uploaded and no \
                                 network call is made.",
            }),
        )),
        "ping" => Some(result(id, json!({}))),
        "tools/list" => Some(result(id, json!({"tools": tool_descriptors()}))),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let empty = Map::new();
            let args = params
                .get("arguments")
                .and_then(Value::as_object)
                .unwrap_or(&empty);
            match call_tool(name, args) {
                Ok(value) => {
                    let text = serde_json::to_string_pretty(&value).unwrap_or_default();
                    Some(result(
                        id,
                        json!({
                            "content": [{"type": "text", "text": text}],
                            "structuredContent": value,
                            "isError": false,
                        }),
                    ))
                }
                // A refusal is a tool result with isError, not a protocol error. The distinction
                // matters: a protocol error tells the host the server is broken, and declining to
                // overwrite somebody's file is the server working exactly as intended.
                Err(ToolError(message)) => Some(result(
                    id,
                    json!({
                        "content": [{"type": "text", "text": message}],
                        "isError": true,
                    }),
                )),
            }
        }
        other => Some(error(id, -32601, &format!("method not found: {other}"))),
    }
}

/// The blocking read loop. One line in, at most one line out, until stdin closes.
pub fn serve() -> std::io::Result<()> {
    log(&format!("serving on stdio, snitch {}", crate::VERSION));
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => handle(&request),
            Err(e) => Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
        };
        if let Some(response) = response {
            writeln!(
                stdout,
                "{}",
                serde_json::to_string(&response).unwrap_or_default()
            )?;
            stdout.flush()?;
        }
    }
    Ok(())
}
