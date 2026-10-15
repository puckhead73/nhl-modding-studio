//! C5 glyph gate, static half (21 Sep 2026) — research 2026-09-21 handoff 09
//! (`research-r34-fonts-encoding`): dictionary text is UTF-8 on every title, and the shipped
//! fonts' `cmap` unions are measured per title tree (`defs/fonts/glyph-coverage.toml`). A new or
//! edited string is checked character by character against the target tree: "in the shipped
//! fonts" / "not in any shipped font (system-font fallback unknown)". Which font a screen uses,
//! the console's `arial.ttf` fallback and the rendering of format characters are **not**
//! established — the runtime gate stays a card. C1 controls (`U+0080–009F`) exist in shipped data
//! (two defects on PS4) and are preserved on round-trip but refused on input.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{DomainError, Platform, Result};

/// `defs/fonts/glyph-coverage.toml`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GlyphCoverageDefs {
    #[serde(default)]
    pub source_sha256: String,
    #[serde(default)]
    pub trees: Vec<TreeCoverage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TreeCoverage {
    pub tree: String,
    pub measured: bool,
    #[serde(default)]
    pub fonts: Vec<String>,
    #[serde(default)]
    pub union_count: usize,
    /// `U+XXXX` or `U+XXXX-U+YYYY`, inclusive.
    #[serde(default)]
    pub ranges: Vec<String>,
}

impl GlyphCoverageDefs {
    pub fn from_toml(text: &str) -> Result<Self> {
        let d: Self = toml::from_str(text).map_err(|e| DomainError::Validation(format!("glyph coverage: {e}")))?;
        for t in &d.trees {
            for r in &t.ranges {
                parse_range(r).ok_or_else(|| DomainError::Validation(format!("glyph coverage: tree {}: bad range {r:?}", t.tree)))?;
            }
        }
        Ok(d)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_toml(&text).map_err(|e| DomainError::Validation(format!("{}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(DomainError::Io { path: path.display().to_string(), source }),
        }
    }

    /// The tree measured for a title: `<platform>-<game>`; Recompiled shares Legacy's Xbox 360
    /// tree by identity. `None` when the defs have no row.
    pub fn tree_for(&self, game_id: &str, platform: Platform) -> Option<&TreeCoverage> {
        let (plat, game) = match platform {
            Platform::Xbox => ("xbox360", game_id),
            Platform::Recomp => ("xbox360", "nhllegacy"),
            Platform::Ps3 => ("ps3", game_id),
            Platform::Ps4 => ("ps4", game_id),
        };
        let name = format!("{plat}-{game}");
        self.trees.iter().find(|t| t.tree == name)
    }

    /// The static verdict for `text` on a title.
    pub fn check(&self, game_id: &str, platform: Platform, text: &str) -> GlyphVerdict {
        let controls: Vec<char> = text.chars().filter(|c| ('\u{80}'..='\u{9F}').contains(c)).collect();
        let Some(tree) = self.tree_for(game_id, platform) else {
            return GlyphVerdict { tree: None, measured: false, missing: Vec::new(), controls, status: GlyphStatus::NoTree };
        };
        if !tree.measured {
            return GlyphVerdict { tree: Some(tree.tree.clone()), measured: false, missing: Vec::new(), controls, status: GlyphStatus::Unmeasured };
        }
        let ranges: Vec<(u32, u32)> = tree.ranges.iter().filter_map(|r| parse_range(r)).collect();
        let mut missing: Vec<char> = Vec::new();
        for c in text.chars() {
            if c.is_control() {
                continue; // C0 (tab / newline / carriage return) are layout, not glyphs; C1 are reported separately
            }
            let cp = c as u32;
            if !ranges.iter().any(|(a, b)| cp >= *a && cp <= *b) && !missing.contains(&c) {
                missing.push(c);
            }
        }
        let status = if missing.is_empty() { GlyphStatus::Covered } else { GlyphStatus::Missing };
        GlyphVerdict { tree: Some(tree.tree.clone()), measured: true, missing, controls, status }
    }
}

fn parse_cp(s: &str) -> Option<u32> {
    u32::from_str_radix(s.strip_prefix("U+")?, 16).ok()
}

fn parse_range(r: &str) -> Option<(u32, u32)> {
    match r.split_once('-') {
        Some((a, b)) => Some((parse_cp(a)?, parse_cp(b)?)),
        None => parse_cp(r).map(|c| (c, c)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GlyphStatus {
    /// Every character is inside the shipped fonts' union.
    Covered,
    /// `missing` names the characters no shipped font maps.
    Missing,
    /// The tree is known but its fonts sit inside archives the research did not open.
    Unmeasured,
    /// No row for this title / platform.
    NoTree,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlyphVerdict {
    pub tree: Option<String>,
    pub measured: bool,
    pub missing: Vec<char>,
    /// C1 control characters in the text (refused on input by the writers).
    pub controls: Vec<char>,
    pub status: GlyphStatus,
}

impl GlyphVerdict {
    /// The one-line warning the dictionary previews carry.
    pub fn warning(&self) -> String {
        let fmt = |cs: &[char]| cs.iter().map(|c| format!("{c:?} U+{:04X}", *c as u32)).collect::<Vec<_>>().join(", ");
        match self.status {
            GlyphStatus::Covered => format!(
                "glyph gate (static): every character of the new text is in the shipped fonts of {} (research 2026-09-21 handoff 09); which font a screen uses is runtime",
                self.tree.as_deref().unwrap_or("?")
            ),
            GlyphStatus::Missing => format!(
                "glyph gate (static): {} not in any shipped font of {} — a system-font fallback is unknown, the character may render blank (research 2026-09-21 handoff 09)",
                fmt(&self.missing),
                self.tree.as_deref().unwrap_or("?")
            ),
            GlyphStatus::Unmeasured => format!("glyph gate: the fonts of {} sit inside archives the research did not open — no static coverage; only a run shows a glyph", self.tree.as_deref().unwrap_or("?")),
            GlyphStatus::NoTree => "glyph gate: no measured font set for this title / platform; only a run shows a glyph".to_string(),
        }
    }
}

/// Refuse text a dictionary writer must not take (handoff 09 §4): C1 controls.
pub fn refuse_input_controls(text: &str) -> Result<()> {
    let controls: Vec<String> = text.chars().filter(|c| ('\u{80}'..='\u{9F}').contains(c)).map(|c| format!("U+{:04X}", c as u32)).collect();
    if controls.is_empty() {
        Ok(())
    } else {
        Err(DomainError::Validation(format!("the new text carries C1 control character(s) {} — refused on input (shipped data carries two such defects, which round-trip untouched)", controls.join(", "))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defs() -> GlyphCoverageDefs {
        GlyphCoverageDefs::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../defs/fonts/glyph-coverage.toml")).unwrap()
    }

    /// Handoff 09 §3 + the measured ranges of its JSON: on gen7 Legacy the Polish / Hungarian
    /// capitals and the CJK language names are outside the union (the JSON's union also lacks
    /// the Polish lowercase forms — the data, not the prose, is what the gate uses) while the
    /// Czech letters, Cyrillic and Latin-1 are inside; on PS4 21 the capitals are covered;
    /// Recompiled reads Legacy's Xbox 360 tree; PS3 12 is unmeasured; NHL 07 has no row.
    #[test]
    fn verdicts_follow_the_research_findings() {
        let d = defs();
        assert_eq!(d.trees.len(), 24);
        let v = d.check("nhllegacy", Platform::Xbox, "Łukasz Ąć čřšžěůďťň Ярослав é");
        assert_eq!(v.status, GlyphStatus::Missing);
        assert_eq!(v.missing, vec!['Ł', 'Ą', 'ć']);
        assert!(v.controls.is_empty());
        assert_eq!(d.check("nhllegacy", Platform::Recomp, "Ł").missing, vec!['Ł']);
        assert_eq!(d.check("nhllegacy", Platform::Xbox, "中文").missing, vec!['中', '文']);
        assert_eq!(d.check("nhllegacy", Platform::Ps3, "Maple Leafs – Canadiens €").status, GlyphStatus::Covered);
        assert_eq!(d.check("nhl21", Platform::Ps4, "Łukasz Ąć Ő Ű Ż").status, GlyphStatus::Covered);
        assert_eq!(d.check("nhl12", Platform::Ps3, "x").status, GlyphStatus::Unmeasured);
        assert_eq!(d.check("nhl07", Platform::Xbox, "x").status, GlyphStatus::NoTree);
        let c = d.check("nhl21", Platform::Ps4, "a\u{81}b");
        assert_eq!(c.controls, vec!['\u{81}']);
        assert!(refuse_input_controls("a\u{81}b").is_err());
        assert!(refuse_input_controls("plain é").is_ok());
        assert!(c.warning().contains("every character"));
        assert!(v.warning().contains("'Ł' U+0141"));
    }
}
