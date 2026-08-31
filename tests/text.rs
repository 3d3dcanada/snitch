//! The text scanner, and above all the characters it must never touch.
//!
//! Stripping U+200D and U+200C indiscriminately is the obvious way to write this and it is wrong.
//! ZWJ builds emoji sequences and both are standard orthography in Arabic, Persian, Kurdish and
//! Indic scripts, so a naive sweep silently corrupts real writing in four languages and turns a
//! family into three strangers. The web tool's own source records catching that in test. This file
//! is the reason it cannot come back.

use snitch::text::{self, Severity};

const ZWSP: char = '\u{200B}';
const ZWNJ: char = '\u{200C}';
const ZWJ: char = '\u{200D}';
const BOM: char = '\u{FEFF}';
const RLO: char = '\u{202E}';

fn keys(findings: &[text::Finding]) -> Vec<&str> {
    findings.iter().map(|f| f.key.as_str()).collect()
}

// ----------------------------------------------------------------------------------------------
// what must survive
// ----------------------------------------------------------------------------------------------

#[test]
fn emoji_zwj_sequences_are_left_exactly_alone() {
    let samples = [
        format!("\u{1F469}{ZWJ}\u{1F4BB}"), // woman technologist
        format!("\u{1F468}{ZWJ}\u{1F469}{ZWJ}\u{1F467}{ZWJ}\u{1F466}"), // family of four
        format!("\u{1F3F3}\u{FE0F}{ZWJ}\u{1F308}"), // rainbow flag
    ];
    for sample in samples {
        let text = format!("look {sample} here");
        let result = text::clean(&text);
        assert_eq!(result.text, text, "corrupted {sample:?}");
        assert_eq!(result.removed, 0);
    }
}

#[test]
fn arabic_persian_kurdish_and_indic_joiners_survive() {
    let samples = [
        format!("\u{645}\u{6CC}{ZWNJ}\u{62E}\u{648}\u{627}\u{647}\u{645}"), // Persian, required
        format!("\u{646}\u{645}\u{6CC}{ZWNJ}\u{62F}\u{627}\u{646}\u{645}"), // Persian
        format!("\u{62F}\u{6D5}\u{628}{ZWNJ}\u{6CE}\u{62A}"),               // Kurdish
        format!("\u{915}{ZWJ}\u{937}"),                                     // Devanagari conjunct
        format!("\u{915}{ZWNJ}\u{937}"),                                    // Devanagari blocked
        format!("\u{9AC}\u{9BE}{ZWNJ}\u{982}\u{9B2}\u{9BE}"),               // Bengali
    ];
    for sample in samples {
        let result = text::clean(&sample);
        assert_eq!(result.text, sample, "corrupted {sample:?}");
        assert_eq!(result.removed, 0, "removed something from {sample:?}");
    }
}

#[test]
fn ordinary_non_ascii_is_never_called_suspicious() {
    for sample in [
        "caf\u{e9} na\u{ef}ve",
        "\u{4e2d}\u{6587}\u{3067}\u{3059}",
        "\u{395}\u{3bb}\u{3bb}\u{3b7}\u{3bd}\u{3b9}\u{3ba}\u{3ac}",
        "\u{417}\u{434}\u{440}\u{430}\u{432}\u{441}\u{442}\u{432}\u{443}\u{439}\u{442}\u{435}",
        "\u{1F600} emoji",
    ] {
        assert_eq!(keys(&text::analyse(sample)), vec!["clean"], "{sample:?}");
    }
}

// ----------------------------------------------------------------------------------------------
// what must be caught
// ----------------------------------------------------------------------------------------------

#[test]
fn real_tracking_characters_are_found_and_removed() {
    let marked = format!("in{ZWSP}voice{BOM} number\u{2060} one\u{2063} here\u{2061}");

    let result = text::clean(&marked);

    assert_eq!(result.text, "invoice number one here");
    assert_eq!(result.removed, 5);
    let mut codes = result.removed_chars.clone();
    codes.sort();
    assert_eq!(codes, ["U+200B", "U+2060", "U+2061", "U+2063", "U+FEFF"]);
    let hidden = result.findings.iter().find(|f| f.key == "hidden").unwrap();
    assert_eq!(hidden.severity, Severity::Notable);
}

#[test]
fn a_right_to_left_override_is_alarming_and_removed() {
    let result = text::clean(&format!("report{RLO}fdp.exe"));

    let bidi = result.findings.iter().find(|f| f.key == "bidi").unwrap();
    assert_eq!(bidi.severity, Severity::Alarming);
    assert!(bidi.note.contains("right-to-left override"));
    assert!(!result.text.contains(RLO));
}

#[test]
fn a_plain_direction_mark_is_notable_rather_than_alarming() {
    let findings = text::analyse("a\u{202A}bc\u{202C}");
    let bidi = findings.iter().find(|f| f.key == "bidi").unwrap();

    assert_eq!(bidi.severity, Severity::Notable);
}

#[test]
fn a_cyrillic_letter_hiding_in_a_latin_word_is_reported() {
    // The first "a" is Cyrillic U+0430, which renders identically to a Latin a.
    let findings = text::analyse("p\u{430}ypal is not paypal");

    let confusable = findings.iter().find(|f| f.key == "confusable").unwrap();
    assert_eq!(confusable.severity, Severity::Alarming);
    assert!(confusable.value.contains("Cyrillic"));
    assert!(confusable.value.contains("Latin"));
}

#[test]
fn a_wholly_cyrillic_word_is_not_a_confusable() {
    let sample = "\u{417}\u{434}\u{440}\u{430}\u{432}\u{441}\u{442}\u{432}\u{443}\u{439}\u{442}\u{435} \u{43C}\u{438}\u{440}";
    assert_eq!(keys(&text::analyse(sample)), vec!["clean"]);
}

// ----------------------------------------------------------------------------------------------
// reported, but not silently edited
// ----------------------------------------------------------------------------------------------

#[test]
fn odd_spaces_and_word_processor_marks_are_reported_but_left_in_the_prose() {
    let original = "one\u{00A0}two \u{201C}quoted\u{201D} and\u{2026} so on\u{2014}really";

    let result = text::clean(original);

    assert_eq!(
        result.text, original,
        "cleaning must not edit someone's prose"
    );
    assert_eq!(result.removed, 0);
    let mut found = keys(&result.findings);
    found.sort();
    assert_eq!(found, vec!["origin", "space"]);
    assert!(
        result
            .findings
            .iter()
            .all(|f| f.severity == Severity::Neutral)
    );
}

#[test]
fn clean_text_says_so_plainly() {
    let findings = text::analyse("Just some ordinary typed words.");

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].key, "clean");
    assert_eq!(findings[0].value, "Plain text");
}

#[test]
fn empty_text_reports_nothing_at_all() {
    assert!(text::analyse("").is_empty());
    assert_eq!(text::clean("").text, "");
    assert_eq!(text::clean("").removed, 0);
}

#[test]
fn strip_chars_removes_only_what_it_was_given() {
    assert_eq!(text::strip_chars("abc", &[]), "abc");
    assert_eq!(text::strip_chars("a-b-c", &['-']), "abc");
    assert_eq!(
        text::strip_chars(&format!("a{ZWJ}b"), &[ZWSP]),
        format!("a{ZWJ}b")
    );
}

#[test]
fn findings_name_the_exact_code_points_so_a_reader_can_check() {
    let findings = text::analyse(&format!("a{ZWSP}b"));
    let hidden = findings.iter().find(|f| f.key == "hidden").unwrap();

    assert!(hidden.value.contains("U+200B"));
    assert_eq!(hidden.chars, vec![ZWSP.to_string()]);
}

#[test]
fn the_suspicious_set_never_contains_a_joiner() {
    // The one invariant this whole module exists to hold.
    assert!(!text::SUSPICIOUS_ZERO_WIDTH.contains(&ZWJ));
    assert!(!text::SUSPICIOUS_ZERO_WIDTH.contains(&ZWNJ));
}
