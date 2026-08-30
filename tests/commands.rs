//! The three commands, end to end, through the actual binaries.

mod common;

use std::path::Path;
use std::process::{Command, Output};

use common::*;

fn run(bin: &str, args: &[&str]) -> Output {
    Command::new(bin)
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("run binary")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

const SNITCH: &str = env!("CARGO_BIN_EXE_snitch");
const NO_COMMENT: &str = env!("CARGO_BIN_EXE_no-comment");
const CREDIT: &str = env!("CARGO_BIN_EXE_credit");

#[test]
fn every_command_reports_the_same_version() {
    for bin in [SNITCH, NO_COMMENT, CREDIT, env!("CARGO_BIN_EXE_snitch-mcp")] {
        let out = run(bin, &["--version"]);
        assert!(out.status.success());
        assert!(
            stdout(&out).trim().ends_with(snitch::VERSION),
            "{}",
            stdout(&out)
        );
    }
}

#[test]
fn snitch_names_the_location_and_the_command_that_removes_it() {
    if !have("exiftool") {
        return;
    }
    let dir = TempDir::new("cmd-gps");
    let source = dir.path("photo.jpg");
    plain_jpeg(&source, 32, 24);
    write_metadata(
        &source,
        &[
            "-GPSLatitude=50.2447",
            "-GPSLatitudeRef=N",
            "-GPSLongitude=-99.8433",
            "-GPSLongitudeRef=W",
            "-EXIF:Make=Canon",
        ],
    );

    let out = run(SNITCH, &[source.to_str().unwrap()]);
    let text = stdout(&out);

    assert!(out.status.success());
    assert!(text.contains("LOCATION IS IN THIS FILE"), "{text}");
    assert!(text.contains("50.2447"), "{text}");
    assert!(
        text.contains("no-comment --"),
        "the hint has to be paste-able: {text}"
    );
    assert!(text.contains("Canon"), "{text}");
}

#[test]
fn an_option_like_filename_is_a_file_and_not_a_flag() {
    if !have("exiftool") {
        return;
    }
    let dir = TempDir::new("dashes");
    let awkward = dir.path("--force.jpg");
    plain_jpeg(&awkward, 16, 16);

    let out = run(SNITCH, &["--", awkward.to_str().unwrap()]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("--force.jpg"), "{}", stdout(&out));
}

#[test]
fn an_unknown_flag_is_an_error_and_not_a_filename() {
    let out = run(SNITCH, &["--definitely-not-a-flag"]);

    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("unknown option"), "{}", stderr(&out));
}

#[test]
fn no_comment_strips_to_a_new_file_and_leaves_the_original_alone() {
    if !have("exiftool") {
        return;
    }
    let dir = TempDir::new("cmd-strip");
    let source = dir.path("photo.jpg");
    plain_jpeg(&source, 40, 30);
    write_metadata(&source, &["-EXIF:Artist=Somebody", "-Comment=private"]);
    let before = std::fs::read(&source).unwrap();

    let out = run(NO_COMMENT, &[source.to_str().unwrap()]);
    let text = stdout(&out);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(text.contains("pixels byte-identical"), "{text}");
    assert!(
        text.contains("In-pixel watermarks are not touched"),
        "{text}"
    );
    assert_eq!(
        std::fs::read(&source).unwrap(),
        before,
        "the input must not change"
    );
    let clean = dir.path("photo-clean.jpg");
    assert!(clean.is_file());
    assert_eq!(read_tag(&clean, "-Artist"), "");
}

#[test]
fn no_comment_refuses_to_replace_an_existing_output_without_force() {
    if !have("exiftool") {
        return;
    }
    let dir = TempDir::new("cmd-force");
    let source = dir.path("photo.jpg");
    let taken = dir.path("photo-clean.jpg");
    plain_jpeg(&source, 16, 16);
    std::fs::write(&taken, b"do not lose me").unwrap();

    let out = run(NO_COMMENT, &[source.to_str().unwrap()]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("pass --force"), "{}", stderr(&out));
    assert_eq!(std::fs::read(&taken).unwrap(), b"do not lose me");

    let out = run(NO_COMMENT, &["--force", source.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_ne!(std::fs::read(&taken).unwrap(), b"do not lose me");
}

#[test]
fn no_comment_refuses_two_inputs_that_would_write_one_output() {
    if !have("exiftool") {
        return;
    }
    let dir = TempDir::new("cmd-collide");
    let a = dir.path("a.jpg");
    let b = dir.path("b.jpg");
    plain_jpeg(&a, 8, 8);
    plain_jpeg(&b, 8, 8);

    let out = run(
        NO_COMMENT,
        &[
            "-o",
            dir.path("one.jpg").to_str().unwrap(),
            a.to_str().unwrap(),
            b.to_str().unwrap(),
        ],
    );

    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("existing directory"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn credit_writes_the_fields_and_drops_location_by_default() {
    if !have("exiftool") {
        return;
    }
    let dir = TempDir::new("cmd-credit");
    let source = dir.path("photo.jpg");
    plain_jpeg(&source, 40, 30);
    write_metadata(&source, &["-GPSLatitude=50.2447", "-GPSLatitudeRef=N"]);

    let out = run(
        CREDIT,
        &[
            source.to_str().unwrap(),
            "--creator",
            "Ren\u{e9}e \u{c5}berg",
            "--copyright",
            "\u{a9} 2026",
            "--licence",
            "cc-by",
        ],
    );
    let target = dir.path("photo-credited.jpg");

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(target.is_file());
    assert_eq!(
        read_tag(&target, "-XMP-dc:Creator"),
        "Ren\u{e9}e \u{c5}berg"
    );
    assert_eq!(read_tag(&target, "-IPTC:By-line"), "Ren\u{e9}e \u{c5}berg");
    assert!(read_tag(&target, "-XMP-xmpRights:UsageTerms").contains("CC BY 4.0"));
    assert_eq!(
        read_tag(&target, "-GPSLatitude"),
        "",
        "location goes unless asked to stay"
    );
    assert_eq!(
        read_tag(&source, "-XMP-dc:Creator"),
        "",
        "the input is never touched"
    );
}

#[test]
fn credit_keeps_location_only_when_told_to() {
    if !have("exiftool") {
        return;
    }
    let dir = TempDir::new("cmd-keepgps");
    let source = dir.path("photo.jpg");
    plain_jpeg(&source, 24, 24);
    write_metadata(&source, &["-GPSLatitude=50.2447", "-GPSLatitudeRef=N"]);

    let out = run(
        CREDIT,
        &[source.to_str().unwrap(), "--creator", "X", "--keep-gps"],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!read_tag(&dir.path("photo-credited.jpg"), "-GPSLatitude").is_empty());
}

#[test]
fn credit_refuses_to_sign_without_being_told_how_the_image_was_made() {
    let dir = TempDir::new("cmd-sign");
    let source = dir.path("photo.jpg");
    plain_jpeg(&source, 8, 8);

    let out = run(
        CREDIT,
        &[source.to_str().unwrap(), "--creator", "X", "--sign"],
    );

    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("--sign requires --digital-source so the credential does not guess"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn credit_with_nothing_to_write_says_so() {
    let dir = TempDir::new("cmd-nothing");
    let source = dir.path("photo.jpg");
    plain_jpeg(&source, 8, 8);

    let out = run(CREDIT, &[source.to_str().unwrap()]);

    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("nothing to do"), "{}", stderr(&out));
}

#[test]
fn a_stamp_lands_in_the_pixels_and_carries_the_camera_block_across_the_re_encode() {
    if !have("exiftool") {
        return;
    }
    let dir = TempDir::new("cmd-stamp");
    let source = dir.path("photo.jpg");
    let target = dir.path("stamped.jpg");
    plain_jpeg(&source, 640, 480);
    write_metadata(
        &source,
        &[
            "-EXIF:Make=Canon",
            "-GPSLatitude=50.2447",
            "-GPSLatitudeRef=N",
        ],
    );

    let out = run(
        CREDIT,
        &[
            source.to_str().unwrap(),
            "--creator",
            "X",
            "--stamp",
            "3D3D",
            "--stamp-sub",
            "3d3d.ca",
            "--keep-gps",
            "-o",
            target.to_str().unwrap(),
        ],
    );

    if !out.status.success() && stderr(&out).contains("no system font") {
        eprintln!("skipping: no system font for the stamp");
        return;
    }
    assert!(out.status.success(), "{}", stderr(&out));
    // THE RE-ENCODE MUST CARRY THE METADATA. Without it, --stamp --keep-gps kept nothing, because
    // the stamp had already thrown the EXIF away before the metadata step ran.
    assert_eq!(read_tag(&target, "-EXIF:Make"), "Canon");
    assert!(
        !read_tag(&target, "-GPSLatitude").is_empty(),
        "--keep-gps has to mean it"
    );
    assert_eq!(read_tag(&target, "-XMP-dc:Creator"), "X");
    assert!(
        pixels_differ(&source, &target),
        "the stamp has to actually be in the pixels"
    );
}

#[test]
fn the_platform_table_marks_every_unverified_row_as_unverified() {
    let out = run(SNITCH, &["--platforms"]);
    let text = stdout(&out);

    assert!(out.status.success());
    assert!(text.contains("researched"), "{text}");
    assert!(
        text.contains("? unverified"),
        "the legend has to be there: {text}"
    );
    assert!(text.contains("LinkedIn"));
    assert!(text.contains("Test your exact upload path."), "{text}");
}

#[test]
fn the_json_report_is_stable_enough_to_diff_across_a_round_trip() {
    if !have("exiftool") {
        return;
    }
    let dir = TempDir::new("cmd-json");
    let source = dir.path("photo.jpg");
    plain_jpeg(&source, 16, 16);

    let first = stdout(&run(SNITCH, &["--json", source.to_str().unwrap()]));
    let second = stdout(&run(SNITCH, &["--json", source.to_str().unwrap()]));

    assert_eq!(
        first, second,
        "two runs on one file must produce identical bytes"
    );
    let parsed: serde_json::Value = serde_json::from_str(&first).expect("valid JSON");
    let file = &parsed["files"][0];
    assert_eq!(file["mime_type"], "image/jpeg");
    assert_eq!(
        file["c2pa_status"],
        if snitch::c2pa::available() {
            "absent"
        } else {
            "unavailable"
        }
    );
    assert_eq!(file["has_any_credit"], false);
}

#[test]
fn a_missing_file_exits_one_and_says_which_one() {
    for bin in [SNITCH, NO_COMMENT] {
        let out = run(bin, &["definitely-missing.jpg"]);
        assert_eq!(out.status.code(), Some(1), "{bin}");
        assert!(
            stderr(&out).contains("not found"),
            "{bin}: {}",
            stderr(&out)
        );
    }
}

fn pixels_differ(a: &Path, b: &Path) -> bool {
    let (a, b) = (
        image::open(a).unwrap().to_rgb8(),
        image::open(b).unwrap().to_rgb8(),
    );
    a.as_raw() != b.as_raw()
}
