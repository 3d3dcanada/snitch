//! The byte surgery, and the pixel proof that makes its claim honest.

mod common;

use common::*;
use snitch::{png, strip};

#[test]
fn stripping_a_jpeg_keeps_jfif_and_icc_and_drops_private_exif() {
    if !have("exiftool") {
        return;
    }
    let dir = TempDir::new("jpeg");
    let source = dir.path("source.jpg");
    let output = dir.path("clean.jpg");
    plain_jpeg(&source, 31, 19);
    write_metadata(
        &source,
        &[
            "-EXIF:Artist=Private Artist",
            "-EXIF:Orientation#=6",
            "-Comment=private comment",
        ],
    );

    let removed = strip::strip(&source, &output).expect("strip");
    let clean = std::fs::read(&output).expect("read");

    assert!(removed > 0);
    assert!(
        contains(&clean, b"JFIF\x00"),
        "JFIF must survive: it identifies the stream"
    );
    assert!(!contains(&clean, b"Private Artist"));
    assert!(!contains(&clean, b"private comment"));
    // Orientation is display, not metadata, and is put back as a minimal APP1.
    assert_eq!(read_tag(&output, "-Orientation#"), "6");
    assert_eq!(strip::pixels_identical(&source, &output), Some(true));
}

#[test]
fn stripping_a_png_keeps_critical_chunks_and_drops_the_rest() {
    let dir = TempDir::new("png");
    let source = dir.path("source.png");
    let output = dir.path("clean.png");
    generator_png(&source);
    // A C2PA box and a private ancillary chunk both have to go.
    insert_chunk(&source, &png::chunk(b"caBX", b"pretend manifest"));
    insert_chunk(
        &source,
        &png::chunk(b"pHYs", &[0, 0, 0x0B, 0x13, 0, 0, 0x0B, 0x13, 1]),
    );

    let removed = strip::strip(&source, &output).expect("strip");
    let names = chunk_names(&output);

    assert!(removed > 0);
    assert!(names.contains(&"IHDR".to_string()));
    assert!(names.contains(&"IDAT".to_string()));
    assert!(names.contains(&"IEND".to_string()));
    assert!(
        names.contains(&"pHYs".to_string()),
        "pHYs affects display and stays"
    );
    assert!(!names.contains(&"tEXt".to_string()));
    assert!(!names.contains(&"caBX".to_string()));
    assert_eq!(png::read_text(&output), vec![]);
    assert_eq!(strip::pixels_identical(&source, &output), Some(true));
}

#[test]
fn the_source_is_never_touched() {
    let dir = TempDir::new("source");
    let source = dir.path("source.png");
    generator_png(&source);
    let before = std::fs::read(&source).unwrap();

    strip::strip(&source, &dir.path("out.png")).expect("strip");

    assert_eq!(std::fs::read(&source).unwrap(), before);
}

#[test]
fn a_format_that_cannot_be_done_losslessly_is_refused_by_name() {
    let dir = TempDir::new("webp");
    let source = dir.path("photo.webp");
    std::fs::write(&source, b"RIFF....WEBP").unwrap();

    let err = strip::strip(&source, &dir.path("out.webp")).unwrap_err();

    assert_eq!(
        err,
        ".webp is not supported for lossless stripping. Only JPEG and PNG."
    );
}

#[test]
fn a_malformed_file_is_refused_rather_than_half_written() {
    let dir = TempDir::new("broken");
    for (name, expected) in [("broken.jpg", "not a JPEG"), ("broken.png", "not a PNG")] {
        let source = dir.path(name);
        std::fs::write(&source, b"not an image at all, just text").unwrap();
        let target = dir.path(&format!("out-{name}"));

        let err = strip::strip_atomic(&source, &target).unwrap_err();

        assert!(err.contains(expected), "{name}: {err}");
        assert!(
            !target.exists(),
            "{name}: a refused strip must leave no output"
        );
    }
}

#[test]
fn a_corrupted_png_crc_is_refused() {
    let dir = TempDir::new("crc");
    let source = dir.path("bad.png");
    generator_png(&source);
    let mut data = std::fs::read(&source).unwrap();
    let at = data.len() - 5; // inside the IEND chunk's CRC
    data[at] ^= 0xFF;
    std::fs::write(&source, &data).unwrap();

    let err = strip::strip(&source, &dir.path("out.png")).unwrap_err();

    assert!(err.contains("bad PNG CRC"), "{err}");
}

#[test]
fn atomic_strip_leaves_no_temporary_behind() {
    let dir = TempDir::new("atomic");
    let source = dir.path("source.png");
    let target = dir.path("out.png");
    generator_png(&source);

    strip::strip_atomic(&source, &target).expect("strip");

    let leftovers: Vec<_> = std::fs::read_dir(&dir.0)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("snitch-"))
        .collect();
    assert!(leftovers.is_empty(), "left behind: {leftovers:?}");
}

#[test]
fn a_changed_pixel_is_detected_by_the_proof() {
    let dir = TempDir::new("pixels");
    let a = dir.path("a.png");
    let b = dir.path("b.png");
    plain_png(&a, 20, 20);
    plain_png(&b, 20, 21); // one row taller, so the decoded pixels differ

    assert_eq!(strip::pixels_identical(&a, &a), Some(true));
    assert_eq!(strip::pixels_identical(&a, &b), Some(false));
    assert_eq!(strip::pixels_identical(&a, &dir.path("missing.png")), None);
}

#[test]
fn output_paths_refuse_to_collide_with_their_source() {
    let dir = TempDir::new("paths");
    let source = dir.path("photo.jpg");
    plain_jpeg(&source, 8, 8);

    assert_eq!(
        strip::outpath(&source, None, "-clean"),
        dir.path("photo-clean.jpg")
    );
    assert_eq!(
        strip::outpath(&source, Some(&dir.path("named.jpg")), "-clean"),
        dir.path("named.jpg")
    );
    assert!(strip::same_file(&source, &source));
    assert!(!strip::same_file(&source, &dir.path("other.jpg")));
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn chunk_names(path: &std::path::Path) -> Vec<String> {
    let data = std::fs::read(path).unwrap();
    let mut names = Vec::new();
    let mut i = 8usize;
    while i + 12 <= data.len() {
        let length = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        names.push(String::from_utf8_lossy(&data[i + 4..i + 8]).into_owned());
        i += length + 12;
    }
    names
}
