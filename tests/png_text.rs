//! What a PNG is saying in its text chunks, which is where generators put the whole prompt.
//!
//! The strip path always removed these. The read path did not report them, so the tool named for
//! telling you what your file says was silent about the most common carrier there is.

mod common;

use common::*;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use snitch::{inspect, png};
use std::io::Write;

fn deflate(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn reads_the_text_chunks_a_generator_wrote() {
    let dir = TempDir::new("gen");
    let source = dir.path("gen.png");
    generator_png(&source);

    let found = png::read_text(&source);

    let by_keyword: Vec<&str> = found.iter().map(|c| c.keyword.as_str()).collect();
    assert_eq!(by_keyword, vec!["parameters", "workflow"]);
    assert_eq!(found[0].chunk, "tEXt");
    assert!(found[0].text.contains("Sampler: Euler a"));
    assert_eq!(found[1].text, r#"{"nodes":[{"id":1,"type":"KSampler"}]}"#);
    assert!(found.iter().all(|c| c.is_generator()));
}

#[test]
fn reads_an_itxt_chunk_with_non_ascii_text() {
    let dir = TempDir::new("itxt");
    let source = dir.path("itxt.png");
    plain_png(&source, 8, 8);
    // keyword, NUL, compression flag 0, method 0, language NUL, translated keyword NUL, then UTF-8.
    let mut payload = b"Description\x00\x00\x00en\x00\x00".to_vec();
    payload.extend_from_slice("caf\u{e9} na\u{ef}ve \u{4e2d}\u{6587} \u{1F600}".as_bytes());
    insert_chunk(&source, &png::chunk(b"iTXt", &payload));

    let found = png::read_text(&source);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].chunk, "iTXt");
    assert_eq!(
        found[0].text,
        "caf\u{e9} na\u{ef}ve \u{4e2d}\u{6587} \u{1F600}"
    );
    assert!(!found[0].is_generator(), "a description is not a prompt");
}

#[test]
fn reads_compressed_ztxt_and_compressed_itxt() {
    let dir = TempDir::new("deflate");
    let source = dir.path("compressed.png");
    plain_png(&source, 8, 8);
    let long_prompt = "a very long prompt ".repeat(40);

    let mut ztxt = b"Comment\x00\x00".to_vec();
    ztxt.extend_from_slice(&deflate(long_prompt.as_bytes()));
    insert_chunk(&source, &png::chunk(b"zTXt", &ztxt));

    let mut itxt = b"prompt\x00\x01\x00en\x00\x00".to_vec();
    itxt.extend_from_slice(&deflate("g\u{e9}n\u{e9}ratif".as_bytes()));
    insert_chunk(&source, &png::chunk(b"iTXt", &itxt));

    let found = png::read_text(&source);
    let get = |k: &str| found.iter().find(|c| c.keyword == k).expect(k);

    assert_eq!(get("Comment").chunk, "zTXt");
    assert_eq!(get("Comment").text, long_prompt);
    assert_eq!(get("prompt").chunk, "iTXt");
    assert_eq!(get("prompt").text, "g\u{e9}n\u{e9}ratif");
}

#[test]
fn a_clean_png_and_a_jpeg_report_nothing_and_do_not_panic() {
    let dir = TempDir::new("clean");
    let clean = dir.path("clean.png");
    let jpeg = dir.path("photo.jpg");
    plain_png(&clean, 8, 8);
    plain_jpeg(&jpeg, 8, 8);

    assert!(png::read_text(&clean).is_empty());
    assert!(png::read_text(&jpeg).is_empty());
    assert!(png::read_text(&dir.path("missing.png")).is_empty());
}

#[test]
fn a_truncated_png_reports_what_survived_instead_of_panicking() {
    let dir = TempDir::new("cut");
    let source = dir.path("cut.png");
    generator_png(&source);
    let data = std::fs::read(&source).unwrap();
    std::fs::write(&source, &data[..data.len() - 9]).unwrap();

    let found = png::read_text(&source);

    assert_eq!(
        found.iter().map(|c| c.keyword.as_str()).collect::<Vec<_>>(),
        vec!["parameters", "workflow"]
    );
}

#[test]
fn inspect_names_the_generator_and_says_the_signal_is_not_a_credential() {
    if skipping(
        "exiftool",
        "inspect_names_the_generator_and_says_the_signal_is_not_a_credential",
    ) {
        return;
    }
    let dir = TempDir::new("inspect-gen");
    let source = dir.path("gen.png");
    generator_png(&source);

    let report = inspect::inspect(&source, None).expect("inspect");

    assert_eq!(report.generator_keywords, vec!["parameters", "workflow"]);
    assert_eq!(report.ai.as_deref(), Some("generative"));
    assert_eq!(report.ai_source.as_deref(), Some("png-text-chunk"));
}

#[test]
fn a_caption_mentioning_a_model_is_not_treated_as_generated() {
    if skipping(
        "exiftool",
        "a_caption_mentioning_a_model_is_not_treated_as_generated",
    ) {
        return;
    }
    let dir = TempDir::new("caption");
    let source = dir.path("photo.png");
    plain_png(&source, 8, 8);
    insert_chunk(
        &source,
        &png::chunk(
            b"tEXt",
            &text_payload(
                "Description",
                "shot on film, not Stable Diffusion, no prompt involved",
            ),
        ),
    );

    let report = inspect::inspect(&source, None).expect("inspect");

    assert_eq!(report.png_text[0].keyword, "Description");
    assert!(report.generator_keywords.is_empty());
    assert_eq!(report.ai, None);
    assert_eq!(report.ai_source, None);
}

#[test]
fn a_clean_png_reports_no_embedded_text() {
    if skipping("exiftool", "a_clean_png_reports_no_embedded_text") {
        return;
    }
    let dir = TempDir::new("inspect-clean");
    let source = dir.path("clean.png");
    plain_png(&source, 8, 8);

    let report = inspect::inspect(&source, None).expect("inspect");

    assert!(report.png_text.is_empty());
    assert!(report.generator_keywords.is_empty());
    assert_eq!(report.ai_source, None);
}

#[test]
fn stripping_removes_the_chunks_without_touching_the_pixels() {
    if skipping(
        "exiftool",
        "stripping_removes_the_chunks_without_touching_the_pixels",
    ) {
        return;
    }
    let dir = TempDir::new("strip-gen");
    let source = dir.path("gen.png");
    let output = dir.path("gen-clean.png");
    generator_png(&source);

    let (removed, identical) = snitch::strip::strip_atomic(&source, &output).expect("strip");

    assert!(removed > 0);
    assert!(identical);
    assert!(png::read_text(&output).is_empty());
    assert_eq!(inspect::inspect(&output, None).expect("inspect").ai, None);
}

#[test]
fn every_generator_keyword_is_lowercase_so_the_match_is_case_insensitive() {
    // The lookup lowercases the keyword it found; a mixed-case entry in the table would never hit.
    for keyword in png::GENERATOR_KEYWORDS {
        assert_eq!(keyword, &keyword.to_ascii_lowercase(), "{keyword}");
    }
}

#[test]
fn a_chunk_that_inflates_enormously_is_capped_and_says_so() {
    // A 510 KB file with one zTXt chunk of compressed zeros drove the reader to 3.6 GB before this
    // cap existed. The cap is not a guess: no legitimate generator payload is anywhere near it.
    let dir = TempDir::new("bomb");
    let source = dir.path("bomb.png");
    plain_png(&source, 8, 8);
    let mut payload = b"Comment\x00\x00".to_vec();
    payload.extend_from_slice(&deflate(&vec![0u8; 64 * 1024 * 1024]));
    insert_chunk(&source, &png::chunk(b"zTXt", &payload));

    let found = png::read_text(&source);

    assert_eq!(found.len(), 1);
    assert!(
        found[0].truncated,
        "a chunk that hit the cap has to admit it"
    );
    assert!(
        found[0].text.len() <= 1 << 20,
        "held {} bytes",
        found[0].text.len()
    );
}

#[test]
fn an_uncompressed_chunk_is_capped_the_same_way() {
    let dir = TempDir::new("bigtext");
    let source = dir.path("big.png");
    plain_png(&source, 8, 8);
    insert_chunk(
        &source,
        &png::chunk(b"tEXt", &text_payload("parameters", &"x".repeat(3 << 20))),
    );

    let found = png::read_text(&source);

    assert!(found[0].truncated);
    assert!(found[0].text.len() <= 1 << 20);
    assert!(
        found[0].is_generator(),
        "capping must not change what the keyword means"
    );
}

#[test]
fn a_file_stuffed_with_chunks_stops_at_a_bounded_number_of_them() {
    let dir = TempDir::new("flood");
    let source = dir.path("flood.png");
    plain_png(&source, 8, 8);
    for i in 0..2000 {
        insert_chunk(
            &source,
            &png::chunk(b"tEXt", &text_payload(&format!("k{i}"), "v")),
        );
    }

    let found = png::read_text(&source);

    assert_eq!(found.len(), 256, "the cap is 256 and it holds");
}

#[test]
fn an_ordinary_chunk_carries_no_truncated_flag_into_the_json() {
    // The flag is skipped when false, so a normal file's report is byte-identical to what it was
    // before the cap existed, and the parity harness still passes.
    let dir = TempDir::new("normal");
    let source = dir.path("gen.png");
    generator_png(&source);

    let found = png::read_text(&source);
    let json = serde_json::to_string(&found[0]).unwrap();

    assert!(!found[0].truncated);
    assert!(!json.contains("truncated"), "{json}");
}
