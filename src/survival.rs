//! Sourced claims about what platforms do to image metadata.
//!
//! This is a research index, not a blanket verification claim. A platform can read metadata during
//! ingestion, retain it internally, show a label, and still omit it from the downloadable
//! derivative. Those behaviours are recorded separately in each note, and an inference is rendered
//! with a "?" so an expectation can never masquerade as a verified round trip.
//!
//! THE TABLE IS DATA, NOT CODE. It was exported verbatim from the Python that researched it, and
//! it is baked in with `include_str!` so the binary is self-contained. `SNITCH_SURVIVAL` points at
//! a replacement file for anyone who does the round trips and wants to publish better numbers
//! without a rebuild.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

const BUILTIN: &str = include_str!("../data/survival.json");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Layer {
    pub key: String,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cell {
    pub verdict: String,
    pub evidence: String,
    pub note: String,
    #[serde(default)]
    pub sources: Vec<Source>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Source {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Symbols {
    pub verdict: BTreeMap<String, String>,
    pub evidence: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Table {
    pub researched: String,
    pub advice: String,
    pub how_to_verify: String,
    pub legend: BTreeMap<String, String>,
    pub symbols: Symbols,
    pub layers: Vec<Layer>,
    /// serde_json preserves insertion order with the preserve_order feature off only for maps it
    /// controls, so platform order is taken from `platform_order` below instead of the map.
    pub platforms: BTreeMap<String, BTreeMap<String, Cell>>,
}

/// The order the platforms are printed in, which is the order the research put them in and is not
/// alphabetical. Kept beside the data rather than inferred from it.
pub const PLATFORM_ORDER: [&str; 7] = [
    "LinkedIn",
    "Instagram",
    "Facebook",
    "X / Twitter",
    "Reddit",
    "Printables",
    "Google Images",
];

impl Table {
    pub fn load() -> Table {
        // A replacement file that fails to parse is a mistake worth hearing about, but it must not
        // stop the tool: the built-in table is always a valid answer.
        if let Some(path) = std::env::var_os("SNITCH_SURVIVAL") {
            match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|s| serde_json::from_str::<Table>(&s).map_err(|e| e.to_string()))
            {
                Ok(table) => return table,
                Err(e) => eprintln!(
                    "  SNITCH_SURVIVAL at {} could not be read, using the built-in table: {e}",
                    std::path::Path::new(&path).display()
                ),
            }
        }
        serde_json::from_str(BUILTIN).expect("the built-in survival table is valid JSON")
    }

    pub fn ordered_platforms(&self) -> Vec<(&String, &BTreeMap<String, Cell>)> {
        let mut out: Vec<_> = PLATFORM_ORDER
            .iter()
            .filter_map(|name| self.platforms.get_key_value(*name))
            .collect();
        // Anything a replacement file adds still gets printed, just after the known ones.
        for (name, layers) in &self.platforms {
            if !PLATFORM_ORDER.contains(&name.as_str()) {
                out.push((name, layers));
            }
        }
        out
    }

    pub fn display_cell(&self, cell: &Cell) -> String {
        let evidence = self
            .symbols
            .evidence
            .get(&cell.evidence)
            .map(String::as_str)
            .unwrap_or("?");
        let verdict = self
            .symbols
            .verdict
            .get(&cell.verdict)
            .map(String::as_str)
            .unwrap_or(cell.verdict.as_str());
        format!("{evidence} {verdict}")
    }

    /// The machine-readable shape, matching what the Python's `as_dict` emitted.
    pub fn as_json(&self, include_notes: bool, include_check: bool) -> serde_json::Value {
        let mut platforms = serde_json::Map::new();
        for (name, layers) in self.ordered_platforms() {
            let mut entry = serde_json::Map::new();
            for layer in &self.layers {
                let Some(cell) = layers.get(&layer.key) else {
                    continue;
                };
                let mut item = serde_json::json!({
                    "verdict": cell.verdict,
                    "evidence": cell.evidence,
                    "live_tested": cell.evidence == "corroborated",
                });
                if include_notes {
                    item["note"] = serde_json::json!(cell.note);
                    item["sources"] = serde_json::to_value(&cell.sources).unwrap_or_default();
                }
                entry.insert(layer.key.clone(), item);
            }
            platforms.insert(name.clone(), serde_json::Value::Object(entry));
        }
        // Key order is the Python's insertion order, not alphabetical, because these reports get
        // diffed against each other across an upload round trip.
        let mut report = serde_json::json!({
            "researched": self.researched,
            "legend": {
                "D": self.legend.get("D"),
                "C": self.legend.get("C"),
                "?": self.legend.get("?"),
            },
            "layers": self.layers,
            "platforms": platforms,
            "advice": self.advice,
        });
        if include_check {
            report["how_to_verify"] = serde_json::json!(self.how_to_verify);
        }
        report
    }
}
