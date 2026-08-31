//! What is actually in a piece of text.
//!
//! A port of the web tool's `app/snitch/lib/text.ts`, kept behaviourally identical on purpose so
//! the CLI, the MCP server and the website cannot disagree about what a piece of text contains.
//! Change one, change the other, or the tool starts telling two stories.
//!
//! Everything reported here is CHECKABLE. Each finding names the exact code points it found, so a
//! sceptical reader can verify it in any editor. Nothing is inferred from writing style: no "this
//! looks like AI" guessing, because that is not measurable and would be the same class of false
//! claim the image side has already been audited for.

pub const ZWSP: char = '\u{200B}';
pub const ZWNJ: char = '\u{200C}';
pub const ZWJ: char = '\u{200D}';
pub const BOM: char = '\u{FEFF}';
pub const WORD_JOINER: char = '\u{2060}';
pub const INVISIBLE_SEPARATOR: char = '\u{2063}';
pub const FUNCTION_APPLICATION: char = '\u{2061}';

/// ZWJ (U+200D) and ZWNJ (U+200C) are deliberately absent, and must stay absent. They are
/// load-bearing: ZWJ builds emoji sequences such as a family or a profession, and both are
/// standard orthography in Arabic, Persian, Kurdish and Indic scripts. Stripping them corrupts
/// real writing in four languages and turns a family into three strangers. The web tool's own
/// source records catching that in test. Only characters with no legitimate role in running text
/// belong here.
pub const SUSPICIOUS_ZERO_WIDTH: [char; 5] = [
    ZWSP,
    BOM,
    WORD_JOINER,
    INVISIBLE_SEPARATOR,
    FUNCTION_APPLICATION,
];

pub const RLO: char = '\u{202E}';
pub const LRO: char = '\u{202D}';

pub const BIDI_CONTROLS: [char; 9] = [
    '\u{202A}', // LRE
    '\u{202B}', // RLE
    '\u{202C}', // PDF, pop directional formatting
    LRO, RLO, '\u{2066}', // LRI
    '\u{2067}', // RLI
    '\u{2068}', // FSI
    '\u{2069}', // PDI
];

pub const ODD_SPACES: [(char, &str); 6] = [
    ('\u{00A0}', "non-breaking space"),
    ('\u{2007}', "figure space"),
    ('\u{2009}', "thin space"),
    ('\u{202F}', "narrow no-break space"),
    ('\u{3000}', "ideographic space"),
    ('\u{200A}', "hair space"),
];

pub const ORIGIN_TELLS: [(char, &str); 7] = [
    ('\u{2018}', "curly opening quote"),
    ('\u{2019}', "curly closing quote or apostrophe"),
    ('\u{201C}', "curly opening double quote"),
    ('\u{201D}', "curly closing double quote"),
    ('\u{2026}', "ellipsis character"),
    ('\u{2013}', "en dash"),
    ('\u{2014}', "em dash"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Alarming,
    Notable,
    Neutral,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Finding {
    pub key: String,
    pub label: String,
    pub value: String,
    pub severity: Severity,
    pub note: String,
    /// The exact characters found, for display and for the targeted strip.
    pub chars: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Cleaned {
    pub text: String,
    pub removed: usize,
    pub removed_chars: Vec<String>,
    pub findings: Vec<Finding>,
}

pub fn code_point(c: char) -> String {
    format!("U+{:04X}", c as u32)
}

fn lookup(table: &[(char, &'static str)], c: char) -> Option<&'static str> {
    table.iter().find(|(k, _)| *k == c).map(|(_, v)| *v)
}

/// Latin, Cyrillic or Greek, or None for anything else. Only these three confuse each other.
fn script_of(c: char) -> Option<&'static str> {
    let n = c as u32;
    if (0x41..=0x5A).contains(&n) || (0x61..=0x7A).contains(&n) {
        return Some("Latin");
    }
    if (0x0400..=0x04FF).contains(&n) {
        return Some("Cyrillic");
    }
    if (0x0370..=0x03FF).contains(&n) {
        return Some("Greek");
    }
    None
}

/// Words containing more than one alphabet.
///
/// This is the actual mechanism behind lookalike domains and impersonated names: a Cyrillic "а" is
/// a different character from a Latin "a" and renders identically in most fonts.
pub fn mixed_script_words(text: &str) -> Vec<(String, Vec<&'static str>)> {
    let mut out = Vec::new();
    for word in text.split_whitespace() {
        if word.chars().count() < 2 {
            continue;
        }
        let mut scripts: Vec<&'static str> = Vec::new();
        for c in word.chars() {
            if let Some(s) = script_of(c) {
                if !scripts.contains(&s) {
                    scripts.push(s);
                }
            }
        }
        if scripts.len() > 1 {
            scripts.sort_unstable();
            out.push((word.to_string(), scripts));
        }
    }
    out
}

fn unique(chars: &[char]) -> Vec<char> {
    let mut seen: Vec<char> = Vec::new();
    for &c in chars {
        if !seen.contains(&c) {
            seen.push(c);
        }
    }
    seen
}

fn joined(chars: &[char], f: impl Fn(char) -> String) -> String {
    chars.iter().map(|&c| f(c)).collect::<Vec<_>>().join(", ")
}

/// Findings, in the same order and with the same keys as the web tool.
pub fn analyse(text: &str) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    if text.is_empty() {
        return findings;
    }

    let (mut hidden, mut bidi, mut spaces, mut tells) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for c in text.chars() {
        if SUSPICIOUS_ZERO_WIDTH.contains(&c) {
            hidden.push(c);
        } else if BIDI_CONTROLS.contains(&c) {
            bidi.push(c);
        } else if lookup(&ODD_SPACES, c).is_some() {
            spaces.push(c);
        } else if lookup(&ORIGIN_TELLS, c).is_some() {
            tells.push(c);
        }
    }

    if !bidi.is_empty() {
        let uniq = unique(&bidi);
        let has_override = uniq.iter().any(|&c| c == RLO || c == LRO);
        findings.push(Finding {
            key: "bidi".into(),
            label: "Direction-control characters".into(),
            value: format!("{} found ({})", bidi.len(), joined(&uniq, code_point)),
            severity: if has_override {
                Severity::Alarming
            } else {
                Severity::Notable
            },
            note: if has_override {
                "This text contains a right-to-left override. That character can make text display \
                 in a different order than it is actually stored, which is the trick used to \
                 disguise what a filename or a link really says."
            } else {
                "Direction-control characters are legitimate in mixed right-to-left and \
                 left-to-right text, and are also used to disguise what text says. Worth knowing \
                 they are here."
            }
            .into(),
            chars: uniq.iter().map(|c| c.to_string()).collect(),
        });
    }

    let mixed = mixed_script_words(text);
    if !mixed.is_empty() {
        findings.push(Finding {
            key: "confusable".into(),
            label: "Words mixing alphabets".into(),
            value: mixed
                .iter()
                .take(4)
                .map(|(word, scripts)| format!("{word} ({})", scripts.join(" + ")))
                .collect::<Vec<_>>()
                .join(", "),
            severity: Severity::Alarming,
            note: "These words contain letters from more than one alphabet. A Cyrillic a and a \
                   Latin a look identical in almost every font, which is how lookalike domains \
                   and impersonated names work. If you did not write this in more than one \
                   language, treat it as deliberate."
                .into(),
            chars: Vec::new(),
        });
    }

    if !hidden.is_empty() {
        let uniq = unique(&hidden);
        findings.push(Finding {
            key: "hidden".into(),
            label: "Invisible characters".into(),
            value: format!("{} found ({})", hidden.len(), joined(&uniq, code_point)),
            severity: Severity::Notable,
            note:
                "Invisible characters. They survive copying and pasting, they are not visible in \
                   any editor, and they can be used to fingerprint a copy of a document. They are \
                   also occasionally harmless leftovers from a web page."
                    .into(),
            chars: uniq.iter().map(|c| c.to_string()).collect(),
        });
    }

    if !spaces.is_empty() {
        let uniq = unique(&spaces);
        findings.push(Finding {
            key: "space".into(),
            label: "Unusual spaces".into(),
            value: format!(
                "{} found ({})",
                spaces.len(),
                joined(&uniq, |c| lookup(&ODD_SPACES, c)
                    .unwrap_or("space")
                    .to_string())
            ),
            severity: Severity::Neutral,
            note: "Spaces that are not the ordinary space character. They survive copying, they \
                   break search and find, and they usually arrive from a web page or a word \
                   processor rather than being typed."
                .into(),
            chars: uniq.iter().map(|c| c.to_string()).collect(),
        });
    }

    if !tells.is_empty() {
        let uniq = unique(&tells);
        let plural = if tells.len() == 1 { "" } else { "s" };
        findings.push(Finding {
            key: "origin".into(),
            label: "Came through a word processor".into(),
            value: format!(
                "{} substituted character{plural} ({})",
                tells.len(),
                joined(&uniq, |c| lookup(&ORIGIN_TELLS, c)
                    .unwrap_or("mark")
                    .to_string())
            ),
            severity: Severity::Neutral,
            note:
                "Curly quotes, ellipsis characters and long dashes are inserted automatically by \
                   Word, Google Docs and most publishing tools. Plain typing in a code editor does \
                   not produce them. This is a weak hint about where the text came from, not proof \
                   of anything."
                    .into(),
            chars: uniq.iter().map(|c| c.to_string()).collect(),
        });
    }

    if findings.is_empty() {
        findings.push(Finding {
            key: "clean".into(),
            label: "Nothing hidden".into(),
            value: "Plain text".into(),
            severity: Severity::Neutral,
            note: "No invisible characters, no direction controls, no mixed alphabets and no \
                   word-processor substitutions. What you see is what is stored."
                .into(),
            chars: Vec::new(),
        });
    }

    findings
}

/// Remove exactly the characters given, and nothing else.
pub fn strip_chars(text: &str, unwanted: &[char]) -> String {
    if unwanted.is_empty() {
        return text.to_string();
    }
    text.chars().filter(|c| !unwanted.contains(c)).collect()
}

/// Remove the tracking characters and leave the writing alone.
///
/// Removes the invisible characters and the direction controls, because neither has a role in
/// ordinary running text. Leaves odd spaces and word-processor substitutions in place: they are
/// reported so a reader knows they are there, and removing them silently would edit someone's
/// prose. Never touches ZWJ or ZWNJ, which real writing depends on.
pub fn clean(text: &str) -> Cleaned {
    let findings = analyse(text);
    let removable: Vec<char> = findings
        .iter()
        .filter(|f| f.key == "hidden" || f.key == "bidi")
        .flat_map(|f| f.chars.iter().filter_map(|s| s.chars().next()))
        .collect();
    let cleaned = strip_chars(text, &removable);
    Cleaned {
        removed: text.chars().count() - cleaned.chars().count(),
        removed_chars: removable.iter().map(|&c| code_point(c)).collect(),
        text: cleaned,
        findings,
    }
}
