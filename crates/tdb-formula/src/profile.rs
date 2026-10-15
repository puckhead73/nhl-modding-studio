//! The formula profile — the declarative FHM→EA mapping the operator edits and saves. Its JSON shape
//! is exactly `docs/reference/default_ea_formulas.json`; the packaged default is baked in here.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Denominator mode for FHM-attribute normalisation (see `docs/reference/fhm-ea-formula.md` §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NormalizationMode {
    /// Fixed FHM cap of 20 (Puckvault parity).
    #[default]
    Nominal,
    /// Corpus-observed per-(position, attribute) max.
    PositionMax,
}

/// One formula component: `(fhm_key_or_prebuilt, weight)`.
pub type Component = (String, f64);

/// A full formula profile: per-position `EA attribute → components`, plus the normalisation mode.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormulaProfile {
    #[serde(default)]
    pub forward: BTreeMap<String, Vec<Component>>,
    #[serde(default)]
    pub defenseman: BTreeMap<String, Vec<Component>>,
    #[serde(default)]
    pub goalie: BTreeMap<String, Vec<Component>>,
    /// Legacy files without this field deserialise to `Nominal`.
    #[serde(default, rename = "normalisation_mode")]
    pub normalisation_mode: NormalizationMode,
}

/// The packaged default preset (operator-curated), shipped so ratings compute out of the box.
const DEFAULT_JSON: &str = include_str!("../assets/default_ea_formulas.json");

impl FormulaProfile {
    /// The built-in default profile.
    pub fn default_preset() -> Self {
        serde_json::from_str(DEFAULT_JSON).unwrap_or_default()
    }

    /// The component list for one EA attribute in a position bucket (empty when unmapped).
    pub fn components(&self, position: &str, ea_attr: &str) -> &[Component] {
        let bucket = match position {
            "forward" => &self.forward,
            "defenseman" => &self.defenseman,
            "goalie" => &self.goalie,
            _ => return &[],
        };
        bucket.get(ea_attr).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preset_is_populated() {
        let p = FormulaProfile::default_preset();
        assert!(!p.forward.is_empty());
        assert!(!p.defenseman.is_empty());
        assert!(!p.goalie.is_empty());
        assert_eq!(p.normalisation_mode, NormalizationMode::Nominal);
        // Speed maps from FHM Speed.
        assert_eq!(
            p.components("forward", "Speed"),
            &[("Speed".to_string(), 1.0)]
        );
    }
}
