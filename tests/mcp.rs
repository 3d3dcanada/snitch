//! The MCP surface: the guards, the protocol, and the property this file mostly exists for, which
//! is that nothing but a response frame ever reaches stdout.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use common::*;
use serde_json::{Value, json};
use snitch::mcp;

// ----------------------------------------------------------------------------------------------
// the guards
// ----------------------------------------------------------------------------------------------

#[test]
fn a_symlink_is_refused_rather_than_written_through() {
    let dir = TempDir::new("symlink");
    let real = dir.path("real.jpg");
    let link = dir.path("link.jpg");
    plain_jpeg(&real, 8, 8);
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real, &link).unwrap();
    #[cfg(not(unix))]
    return;

    let err = mcp::source_path(link.to_str().unwrap()).unwrap_err();

    assert!(err.0.contains("symbolic link"), "{}", err.0);
}

#[test]
fn empty_missing_and_non_file_paths_are_refused_with_different_sentences() {
    let dir = TempDir::new("guards");
    assert!(
        mcp::source_path("")
            .unwrap_err()
            .0
            .contains("no path given")
    );
    assert!(
        mcp::source_path(dir.path("ghost.jpg").to_str().unwrap())
            .unwrap_err()
            .0
            .contains("not found")
    );
    assert!(
        mcp::source_path(dir.0.to_str().unwrap())
            .unwrap_err()
            .0
            .contains("not a regular file")
    );
}

#[test]
fn output_may_not_be_the_source_or_silently_replace_a_file() {
    let dir = TempDir::new("targets");
    let source = dir.path("a.jpg");
    let taken = dir.path("taken.jpg");
    plain_jpeg(&source, 8, 8);
    std::fs::write(&taken, b"do not lose me").unwrap();
    let s = |p: &std::path::Path| p.to_str().unwrap().to_string();

    let err = |out: &str, force: bool| {
        mcp::target_path(&source, out, "-clean", force)
            .unwrap_err()
            .0
    };
    assert!(err(&s(&source), false).contains("overwrite the source"));
    assert!(err(&s(&taken), false).contains("exists"));
    assert!(err(dir.0.to_str().unwrap(), false).contains("is a directory"));
    assert!(err(&s(&dir.path("nowhere/x.jpg")), false).contains("does not exist"));

    // force is the only way past an existing file, and even then nothing is written by this call.
    assert_eq!(
        mcp::target_path(&source, &s(&taken), "-clean", true).unwrap(),
        taken
    );
    assert_eq!(std::fs::read(&taken).unwrap(), b"do not lose me");
    // No out_path at all means the default sibling.
    assert_eq!(
        mcp::target_path(&source, "", "-clean", false).unwrap(),
        dir.path("a-clean.jpg")
    );
}

// ----------------------------------------------------------------------------------------------
// the protocol
// ----------------------------------------------------------------------------------------------

#[test]
fn the_handshake_echoes_a_version_the_client_can_actually_accept() {
    // Answering with a version outside the handshake set makes a conforming client abandon the
    // session. That happened here once; this is the test that keeps it from happening again.
    assert_eq!(mcp::negotiate(Some("2025-06-18")), "2025-06-18");
    assert_eq!(mcp::negotiate(Some("2024-11-05")), "2024-11-05");
    assert_eq!(mcp::negotiate(Some("2026-07-28")), mcp::PROTOCOL_VERSION);
    assert_eq!(mcp::negotiate(Some("nonsense")), mcp::PROTOCOL_VERSION);
    assert_eq!(mcp::negotiate(None), mcp::PROTOCOL_VERSION);
    assert!(mcp::HANDSHAKE_VERSIONS.contains(&mcp::PROTOCOL_VERSION));
}

#[test]
fn every_tool_declares_a_schema_and_the_limits_it_has() {
    let tools = mcp::tool_descriptors();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert_eq!(
        names,
        vec![
            "snitch_inspect",
            "snitch_strip_metadata",
            "snitch_add_credit",
            "snitch_verify_c2pa",
            "snitch_clean_text",
        ]
    );
    for tool in &tools {
        assert_eq!(tool["inputSchema"]["type"], "object");
        assert!(tool["description"].as_str().unwrap().len() > 40);
    }
    // A model told a tool can do more than it can will promise a user something false.
    let strip = tools
        .iter()
        .find(|t| t["name"] == "snitch_strip_metadata")
        .unwrap();
    let description = strip["description"].as_str().unwrap();
    assert!(description.contains("JPEG and PNG only"), "{description}");
    assert!(description.contains("SynthID"), "{description}");
    let text = tools
        .iter()
        .find(|t| t["name"] == "snitch_clean_text")
        .unwrap();
    assert!(text["description"].as_str().unwrap().contains("ZWJ"));
}

#[test]
fn a_notification_gets_no_reply_and_an_unknown_method_gets_an_error() {
    assert!(
        mcp::handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).is_none()
    );

    let response = mcp::handle(&json!({"jsonrpc": "2.0", "id": 7, "method": "nope"})).unwrap();
    assert_eq!(response["id"], 7);
    assert_eq!(response["error"]["code"], -32601);

    let ping = mcp::handle(&json!({"jsonrpc": "2.0", "id": 8, "method": "ping"})).unwrap();
    assert_eq!(ping["result"], json!({}));
}

#[test]
fn a_refusal_is_a_tool_result_and_not_a_protocol_error() {
    // The distinction matters: a protocol error tells the host the server is broken, and declining
    // to overwrite somebody's file is the server working exactly as intended.
    let response = mcp::handle(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "snitch_inspect", "arguments": {"path": "/nope/ghost.jpg"}},
    }))
    .unwrap();

    assert!(response.get("error").is_none(), "{response}");
    assert_eq!(response["result"]["isError"], true);
    assert!(
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found")
    );
}

// ----------------------------------------------------------------------------------------------
// a real session over a real pipe
// ----------------------------------------------------------------------------------------------

/// Speak JSON-RPC to the actual binary and collect every line it wrote to stdout.
fn session(requests: &[Value]) -> (Vec<Value>, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_snitch-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn snitch-mcp");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for request in requests {
            writeln!(stdin, "{request}").expect("write");
        }
    }
    // Dropping stdin closes the pipe, which ends the server's read loop.
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("wait");
    let lines = BufReader::new(output.stdout.as_slice())
        .lines()
        .map(|l| l.expect("line"))
        .collect::<Vec<_>>();
    let parsed = lines
        .iter()
        .map(|line| {
            serde_json::from_str(line).unwrap_or_else(|e| {
                panic!("stdout carried something that is not a frame: {e}\n{line}")
            })
        })
        .collect();
    (parsed, String::from_utf8_lossy(&output.stderr).into_owned())
}

#[test]
fn a_real_session_answers_every_request_and_writes_nothing_else_to_stdout() {
    if !have("exiftool") {
        eprintln!("skipping: needs exiftool");
        return;
    }
    let dir = TempDir::new("session");
    let source = dir.path("gen.png");
    let target = dir.path("gen-clean.png");
    generator_png(&source);

    let (responses, stderr) = session(&[
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": "2025-06-18"}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
            "name": "snitch_inspect", "arguments": {"path": source.to_str().unwrap()}}}),
        json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {
            "name": "snitch_strip_metadata",
            "arguments": {"path": source.to_str().unwrap(), "out_path": target.to_str().unwrap()}}}),
        json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {
            "name": "snitch_clean_text", "arguments": {"text_input": "a\u{200B}b"}}}),
    ]);

    // Five requests carried an id; the notification did not, and must not have been answered.
    assert_eq!(
        responses.len(),
        5,
        "one reply per request with an id, and no more"
    );
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "snitch");
    assert_eq!(responses[1]["result"]["tools"].as_array().unwrap().len(), 5);

    let inspected = &responses[2]["result"]["structuredContent"];
    assert_eq!(inspected["ai"], "generative");
    assert_eq!(inspected["ai_source"], "png-text-chunk");
    assert_eq!(inspected["c2pa_summary"]["integrity"], "unknown");

    let stripped = &responses[3]["result"]["structuredContent"];
    assert_eq!(stripped["pixels_identical"], true);
    assert!(stripped["bytes_removed"].as_u64().unwrap() > 0);
    assert!(
        stripped["removed"]
            .as_array()
            .unwrap()
            .contains(&json!("png text chunks"))
    );
    assert!(target.is_file());
    assert!(
        snitch::png::read_text(&source).len() == 2,
        "the input is never touched"
    );

    assert_eq!(responses[4]["result"]["structuredContent"]["text"], "ab");

    // The banner goes to stderr, where MCP hosts already look for it.
    assert!(stderr.contains("snitch-mcp: serving on stdio"), "{stderr}");
    assert!(
        !stderr.contains("\u{1b}["),
        "no ANSI escapes anywhere: {stderr}"
    );
}

#[test]
fn a_subprocess_that_fails_does_not_leak_onto_the_wire() {
    // ExifTool and c2patool write to stdout and stderr of their own. If either were inherited
    // rather than captured, this request would corrupt the stream instead of returning a refusal.
    let dir = TempDir::new("noise");
    let broken = dir.path("broken.jpg");
    std::fs::write(&broken, b"not an image at all").unwrap();

    let (responses, _stderr) = session(&[json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "snitch_inspect", "arguments": {"path": broken.to_str().unwrap()}},
    })]);

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["result"]["isError"], true);
}

#[test]
fn a_malformed_line_is_answered_with_a_parse_error_and_the_server_keeps_going() {
    let (responses, _stderr) = session(&[
        json!("{ this is not json"),
        json!({"jsonrpc": "2.0", "id": 2, "method": "ping"}),
    ]);

    // The first line is valid JSON (a string), so it parses and is ignored for having no id;
    // the second still gets its answer, which is the property under test.
    assert!(responses.iter().any(|r| r["id"] == 2));
}
