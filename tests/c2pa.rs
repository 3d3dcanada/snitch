//! C2PA: five states, and three facts that must never be merged into one.

mod common;

use common::*;
use serde_json::json;
use snitch::{c2pa, inspect, sign};

fn have_c2patool() -> bool {
    sign::c2patool().is_some()
}

fn manifest_with(codes: &[&str], state: &str) -> serde_json::Value {
    json!({
        "validation_state": state,
        "active_manifest": "urn:x",
        "manifests": {"urn:x": {
            "title": "t",
            "signature_info": {"issuer": "snitch"},
        }},
        "validation_status": codes.iter().map(|c| json!({"code": c})).collect::<Vec<_>>(),
    })
}

#[test]
fn a_hash_mismatch_is_altered_and_an_untrusted_signer_is_not() {
    let altered = manifest_with(&["assertion.dataHash.mismatch"], "Invalid");
    let untrusted = manifest_with(&["signingCredential.untrusted"], "Valid");
    let both = manifest_with(
        &["signingCredential.untrusted", "assertion.dataHash.mismatch"],
        "Invalid",
    );

    assert!(c2pa::asset_altered(Some(&altered)));
    assert!(!c2pa::identity_untrusted(Some(&altered)));
    assert!(!c2pa::asset_altered(Some(&untrusted)));
    assert!(c2pa::identity_untrusted(Some(&untrusted)));
    assert!(c2pa::asset_altered(Some(&both)));
    assert!(c2pa::identity_untrusted(Some(&both)));
    assert!(!c2pa::asset_altered(None));
    assert!(!c2pa::identity_untrusted(None));
}

#[test]
fn every_hash_mismatch_code_counts_as_altered() {
    for code in c2pa::ALTERED_CODES {
        assert!(
            c2pa::asset_altered(Some(&manifest_with(&[code], "Invalid"))),
            "{code}"
        );
    }
}

#[test]
fn absent_unavailable_and_detected_are_three_different_answers() {
    let base = |status| inspect::Report {
        file: "x.jpg".into(),
        path: "/x.jpg".into(),
        bytes: 1,
        mime_type: Some("image/jpeg".into()),
        camera: vec![],
        gps: vec![],
        credit: vec![],
        png_text: vec![],
        generator_keywords: vec![],
        c2pa: None,
        c2pa_status: status,
        c2pa_error: String::new(),
        ai: None,
        ai_source: None,
        has_any_credit: false,
    };

    let absent = inspect::c2pa_summary(&base(c2pa::Status::Absent));
    let detected = inspect::c2pa_summary(&base(c2pa::Status::DetectedUnverified));
    let unavailable = inspect::c2pa_summary(&base(c2pa::Status::Unavailable));

    assert_eq!(absent.manifest, "absent");
    assert!(absent.means.contains("not evidence"), "{}", absent.means);
    assert_eq!(detected.manifest, "detected-unverified");
    assert!(detected.means.contains("c2patool"), "{}", detected.means);
    assert_eq!(unavailable.manifest, "unavailable");
    for summary in [&absent, &detected, &unavailable] {
        assert_eq!(summary.integrity, "unknown");
        assert_eq!(summary.signer_trusted, "unknown");
        assert_eq!(summary.signer, None);
    }
}

#[test]
fn the_status_names_are_the_ones_the_json_contract_promises() {
    assert_eq!(c2pa::Status::Present.as_str(), "present");
    assert_eq!(c2pa::Status::Absent.as_str(), "absent");
    assert_eq!(c2pa::Status::Unavailable.as_str(), "unavailable");
    assert_eq!(
        c2pa::Status::DetectedUnverified.as_str(),
        "detected-unverified"
    );
    assert_eq!(c2pa::Status::Error.as_str(), "error");
}

#[test]
fn a_manifest_that_asserts_a_camera_is_not_reported_as_generated() {
    let camera = json!({
        "active_manifest": "urn:x",
        "manifests": {"urn:x": {"assertions": [{
            "label": "c2pa.actions",
            "data": {"actions": [{"digitalSourceType": sign::CAMERA}]},
        }]}},
    });
    let generated = json!({
        "active_manifest": "urn:x",
        "manifests": {"urn:x": {"assertions": [{
            "label": "c2pa.actions",
            "data": {"actions": [{"digitalSourceType": sign::GENERATED}]},
        }]}},
    });

    assert_eq!(c2pa::digital_source(Some(&camera)), Some("camera"));
    assert_eq!(c2pa::digital_source(Some(&generated)), Some("generative"));
    assert_eq!(c2pa::digital_source(None), None);
}

#[test]
fn the_manifest_refuses_to_guess_how_the_image_was_made() {
    let err = sign::manifest(&sign::ManifestFields {
        title: "x",
        ..Default::default()
    })
    .unwrap_err();
    assert!(err.contains("digital source is required"), "{err}");

    let camera = sign::manifest(&sign::ManifestFields {
        title: "x",
        digital_source: Some("camera"),
        ..Default::default()
    })
    .unwrap();
    let generated = sign::manifest(&sign::ManifestFields {
        title: "x",
        generated: true,
        ..Default::default()
    })
    .unwrap();

    let source = |m: &serde_json::Value| {
        m["assertions"][1]["data"]["actions"][0]["digitalSourceType"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert_eq!(source(&camera), sign::CAMERA);
    assert_eq!(source(&generated), sign::GENERATED);
    assert_eq!(camera["claim_generator_info"][0]["name"], "snitch");
    assert_eq!(
        camera["claim_generator_info"][0]["version"],
        snitch::VERSION
    );
}

#[test]
fn an_unknown_digital_source_is_refused_by_name() {
    assert!(sign::digital_source_url("camera").is_some());
    assert!(sign::digital_source_url("telepathy").is_none());
}

// ----------------------------------------------------------------------------------------------
// the real thing, when the tools are installed
// ----------------------------------------------------------------------------------------------

#[test]
fn a_real_signature_validates_and_a_tampered_one_says_it_was_altered() {
    if !have_c2patool() || !have("openssl") || !have("exiftool") {
        eprintln!("skipping: needs c2patool, openssl and exiftool");
        return;
    }
    let dir = TempDir::new("sign");
    let source = dir.path("signed.jpg");
    let key = dir.path("key.pem");
    let cert = dir.path("cert.pem");
    plain_jpeg(&source, 101, 67);

    assert!(sign::ensure_cert(&key, &cert, "Atelier \u{c9}toile").expect("cert"));
    assert!(!sign::ensure_cert(&key, &cert, "ignored").expect("second call is a no-op"));
    let manifest = sign::manifest(&sign::ManifestFields {
        title: "Signed work",
        creator: Some("Jos\u{e9} \u{c1}ngel"),
        org: Some("Atelier \u{c9}toile"),
        digital_source: Some("digital"),
        ..Default::default()
    })
    .unwrap();
    sign::sign_file(&source, &manifest, &key, &cert, &sign::c2patool().unwrap()).expect("sign");

    let report = inspect::inspect(&source, None).expect("inspect");
    assert_eq!(report.c2pa_status, c2pa::Status::Present);
    assert_eq!(c2pa::validation_state(report.c2pa.as_ref()), "Valid");
    assert_eq!(
        c2pa::signer(report.c2pa.as_ref()).as_deref(),
        Some("Atelier \u{c9}toile")
    );
    assert!(!c2pa::asset_altered(report.c2pa.as_ref()));
    // A self-signed certificate is never on a validator's trust list, and the tool says so.
    assert!(c2pa::identity_untrusted(report.c2pa.as_ref()));
    assert_eq!(
        report.ai, None,
        "a digitalCreation source is neither a camera nor a model"
    );

    // Flip one byte deep in the entropy-coded scan, well past the manifest.
    let mut data = std::fs::read(&source).unwrap();
    let at = data.len() - 200;
    data[at] ^= 0xFF;
    std::fs::write(&source, &data).unwrap();

    let report = inspect::inspect(&source, None).expect("inspect");
    assert_eq!(c2pa::validation_state(report.c2pa.as_ref()), "Invalid");
    assert!(
        c2pa::asset_altered(report.c2pa.as_ref()),
        "a changed pixel must read as altered"
    );
    let summary = inspect::c2pa_summary(&report);
    assert_eq!(summary.integrity, "altered");
    assert!(
        summary.means.contains("altered after signing"),
        "{}",
        summary.means
    );
}

#[test]
fn a_half_present_key_pair_is_refused_rather_than_replaced() {
    let dir = TempDir::new("halfcert");
    let key = dir.path("key.pem");
    let cert = dir.path("cert.pem");
    std::fs::write(&key, b"EXISTING PRIVATE KEY").unwrap();

    let err = sign::ensure_cert(&key, &cert, "x").unwrap_err();

    assert!(err.contains("refusing to replace existing"), "{err}");
    assert_eq!(std::fs::read(&key).unwrap(), b"EXISTING PRIVATE KEY");
    assert!(!cert.exists());
}

#[test]
fn a_missing_c2patool_reports_unavailable_and_not_absent() {
    let dir = TempDir::new("noc2pa");
    let source = dir.path("photo.jpg");
    plain_jpeg(&source, 8, 8);

    // Naming a tool that does not exist takes the same path as not having one installed.
    let report = c2pa::read_report(&source, Some("definitely-not-a-real-tool"));

    assert_eq!(report.status, c2pa::Status::Unavailable);
    assert_eq!(report.error, "c2patool is not installed");
    assert!(report.manifest.is_none());
}
