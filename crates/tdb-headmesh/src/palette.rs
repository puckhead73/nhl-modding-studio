//! Named bone palettes + the hand-override layer.
//!
//! An RW skin palette stores unnamed inverse-bind matrices; names come from correlating
//! bind translations against a *named* reference skeleton (`tdb_rx2::skeleton::
//! correlate_palette`) and are persisted with per-bone confidence in
//! `defs/appearance/legacy-bone-names.toml`. Correlation alone is not trustworthy —
//! coincident bind origins produce ambiguous or plain wrong names (the eyeball register-0
//! lesson) — so remap targeting is governed by three explicit rules layered on top:
//!
//! 1. `exclude` — never a remap target. Slots no vanilla asset of the class references
//!    hold garbage matrices; weighting them shreds the mesh in-game. Validated against
//!    the donor-usage census ([`crate::census`]).
//! 2. `prefer` — wins over distance when duplicate names exist (Neck must land on the
//!    donor-used slot 6, never the excluded 5).
//! 3. otherwise smallest bind distance wins among same-named slots.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct PaletteBone {
    pub name: String,
    #[serde(default)]
    pub dist: f64,
    #[serde(default)]
    pub confidence: String,
    #[serde(default)]
    pub candidates: Vec<String>,
    /// This slot wins name→palette resolution over any same-named slot.
    #[serde(default)]
    pub prefer: bool,
    /// This slot is never a remap target (garbage / unused matrix).
    #[serde(default)]
    pub exclude: bool,
}

#[derive(Debug, Deserialize)]
struct PaletteFile {
    bones: HashMap<String, PaletteBone>,
}

/// A named target palette (e.g. the Legacy 122-slot player rig).
#[derive(Debug, Clone)]
pub struct NamedPalette {
    /// Dense palette index → bone entry.
    pub bones: Vec<(u32, PaletteBone)>,
}

#[derive(Debug, thiserror::Error)]
pub enum PaletteError {
    #[error("failed to read palette defs: {0}")]
    Io(#[from] std::io::Error),
    #[error("palette defs are not valid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("palette key {0:?} is not a palette index")]
    BadKey(String),
}

impl NamedPalette {
    pub fn load(path: &Path) -> Result<NamedPalette, PaletteError> {
        Self::from_toml_str(&std::fs::read_to_string(path)?)
    }

    pub fn from_toml_str(text: &str) -> Result<NamedPalette, PaletteError> {
        let file: PaletteFile = toml::from_str(text)?;
        let mut bones = Vec::with_capacity(file.bones.len());
        for (key, bone) in file.bones {
            let idx: u32 = key.parse().map_err(|_| PaletteError::BadKey(key.clone()))?;
            bones.push((idx, bone));
        }
        bones.sort_by_key(|(idx, _)| *idx);
        Ok(NamedPalette { bones })
    }

    pub fn len(&self) -> usize {
        self.bones.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bones.is_empty()
    }

    pub fn get(&self, palette_index: u32) -> Option<&PaletteBone> {
        self.bones
            .binary_search_by_key(&palette_index, |(idx, _)| *idx)
            .ok()
            .map(|pos| &self.bones[pos].1)
    }

    /// Build the bone-name → palette-index inversion under the three targeting rules.
    pub fn name_to_palette(&self) -> HashMap<&str, u32> {
        let mut map: HashMap<&str, u32> = HashMap::new();
        for (idx, bone) in &self.bones {
            if bone.exclude {
                continue;
            }
            match map.get(bone.name.as_str()) {
                None => {
                    map.insert(&bone.name, *idx);
                }
                Some(&held) => {
                    let held_bone = self.get(held).expect("held index came from this palette");
                    let replace = if bone.prefer != held_bone.prefer {
                        bone.prefer
                    } else {
                        bone.dist < held_bone.dist
                    };
                    if replace {
                        map.insert(&bone.name, *idx);
                    }
                }
            }
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[bones.5]
name = "Neck"
dist = 0.0217
confidence = "near"
exclude = true

[bones.6]
name = "Neck"
dist = 0.0301
confidence = "near"
prefer = true

[bones.8]
name = "Head"
dist = 0.001
confidence = "low"

[bones.9]
name = "Head1"
dist = 0.002
confidence = "exact"

[bones.10]
name = "Head1"
dist = 0.004
confidence = "exact"
"#;

    #[test]
    fn inversion_honors_exclude_prefer_and_distance() {
        let p = NamedPalette::from_toml_str(SAMPLE).unwrap();
        assert_eq!(p.len(), 5);
        let inv = p.name_to_palette();
        // exclude removes slot 5 even though its dist beats slot 6; prefer keeps 6 stable
        // regardless of insertion order.
        assert_eq!(inv["Neck"], 6);
        assert_eq!(inv["Head"], 8);
        // duplicate names without prefer: smallest bind distance wins.
        assert_eq!(inv["Head1"], 9);
        assert!(p.get(5).unwrap().exclude);
    }
}
