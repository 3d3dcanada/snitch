//! snitch: read, strip and credit image metadata, and check text for hidden characters.
//!
//! Three separate jobs that people usually conflate:
//!
//!     snitch      read everything and say plainly what is there
//!     credit      write credit in, and optionally stamp it into the pixels
//!     no-comment  take metadata out
//!
//! STRIPPING IS SEGMENT SURGERY, NOT RE-ENCODING. Dropping JPEG APPn/COM markers and keeping only
//! the required PNG chunks leaves the decoded pixels byte-identical, and `strip::pixels_identical`
//! proves it per run rather than asserting it. Re-encoding to "clear" metadata throws away image
//! quality for no reason, and this tool will not do it.
//!
//! STAMPING DOES re-encode, because it has to: it changes pixels. That is stated at the call site
//! and in the CLI output, because a user deserves to know which operation costs them quality.
//!
//! HOUSE STYLE. No async, no web framework, no CLI framework, six direct dependencies, and the two
//! obvious crates for C2PA and MCP were measured and refused. See Cargo.toml for the numbers.

pub mod c2pa;
pub mod cli;
pub mod exif;
pub mod inspect;
pub mod jpeg;
pub mod mcp;
pub mod png;
pub mod sign;
pub mod stamp;
pub mod strip;
pub mod survival;
pub mod text;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Licence presets, exactly as the Python shipped them.
pub const LICENCES: [(&str, &str, Option<&str>); 8] = [
    (
        "cc-by",
        "CC BY 4.0",
        Some("https://creativecommons.org/licenses/by/4.0/"),
    ),
    (
        "cc-by-sa",
        "CC BY-SA 4.0",
        Some("https://creativecommons.org/licenses/by-sa/4.0/"),
    ),
    (
        "cc-by-nd",
        "CC BY-ND 4.0",
        Some("https://creativecommons.org/licenses/by-nd/4.0/"),
    ),
    (
        "cc-by-nc",
        "CC BY-NC 4.0",
        Some("https://creativecommons.org/licenses/by-nc/4.0/"),
    ),
    (
        "cc-by-nc-sa",
        "CC BY-NC-SA 4.0",
        Some("https://creativecommons.org/licenses/by-nc-sa/4.0/"),
    ),
    (
        "cc-by-nc-nd",
        "CC BY-NC-ND 4.0",
        Some("https://creativecommons.org/licenses/by-nc-nd/4.0/"),
    ),
    (
        "cc0",
        "CC0 1.0",
        Some("https://creativecommons.org/publicdomain/zero/1.0/"),
    ),
    ("arr", "All rights reserved", None),
];

pub fn licence(key: &str) -> Option<(&'static str, Option<&'static str>)> {
    LICENCES
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, name, url)| (*name, *url))
}

pub fn licence_keys() -> String {
    LICENCES
        .iter()
        .map(|(k, _, _)| *k)
        .collect::<Vec<_>>()
        .join(", ")
}
