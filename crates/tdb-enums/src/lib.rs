//! `tdb-enums` — confirmed value↔label enum registry (EDIT-MODEL Layer 1, enum validation).
//!
//! Ingests the operator-curated enum definitions under the repo's `defs/enums/*.toml` into a typed
//! [`EnumRegistry`]. Each [`EnumDef`] carries a vocabulary of raw-code → label [`EnumEntry`]s plus the
//! `(table, field)` [`EnumBinding`]s the enum applies to, so the edit layer can both **validate enum
//! membership at entry** (an `eGlu.mCcl` captaincy edit must be one of `None/Captain/Alternate`) and
//! **render human labels** on read.
//!
//! ## Id convention (read this)
//!
//! These defs use the **studio / NHLViewNG id convention** (`ubPc`, `vjla`, `eGlu`, `mCcl`, …): the
//! same 4-char ids the metadata XML *names* are derived from. The **on-disk TDB binary** (and the EA
//! `*-meta.xml`) use the **character-reversed** form of each id (`ubPc`↔`cPbu`, `vjla`↔`aljv`,
//! `eGlu`↔`ulGe`). This crate keys the registry by the **studio ids verbatim** — it does not silently
//! transform them. Crossing to the binary id space is the integration layer's job; [`reverse_id`] is
//! provided for that, and the cross-check test in `tests/enums.rs` uses it to verify each enum's
//! field really exists in a real anchor database with a bit-width wide enough to hold its codes.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Provenance block (`[source]`) — how a def was confirmed.
#[derive(Clone, Debug, Deserialize)]
pub struct EnumSource {
    pub kind: String,
    #[serde(default)]
    pub description: String,
}

/// A `(table, field)` site an enum applies to. Keyed by the **studio** ids (see module docs).
#[derive(Clone, Debug, Deserialize)]
pub struct EnumBinding {
    pub field_name: String,
    pub field_raw_id: String,
    pub table_raw_id: String,
}

/// One raw-code → label mapping.
#[derive(Clone, Debug, Deserialize)]
pub struct EnumEntry {
    pub raw: i64,
    pub label: String,
}

/// A single confirmed enum definition (one `defs/enums/*.toml` file).
#[derive(Clone, Debug, Deserialize)]
pub struct EnumDef {
    pub name: String,
    #[serde(default)]
    pub confidence: String,
    #[serde(default)]
    pub notes: String,
    pub source: Option<EnumSource>,
    #[serde(default)]
    pub bindings: Vec<EnumBinding>,
    #[serde(default)]
    pub entries: Vec<EnumEntry>,
}

impl EnumDef {
    /// Parse one enum def from TOML text.
    pub fn from_toml(text: &str) -> Result<Self, EnumError> {
        toml::from_str(text).map_err(EnumError::Parse)
    }

    /// The label for a raw code, if listed.
    pub fn label(&self, raw: i64) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.raw == raw)
            .map(|e| e.label.as_str())
    }

    /// The raw code for an exact label, if present.
    pub fn raw_of(&self, label: &str) -> Option<i64> {
        self.entries
            .iter()
            .find(|e| e.label == label)
            .map(|e| e.raw)
    }

    /// Whether a raw code is a labelled member of this enum.
    pub fn contains(&self, raw: i64) -> bool {
        self.entries.iter().any(|e| e.raw == raw)
    }

    /// The largest labelled raw code (used to sanity-check storage bit-width).
    pub fn max_raw(&self) -> Option<i64> {
        self.entries.iter().map(|e| e.raw).max()
    }
}

/// Errors loading or validating against the enum registry.
#[derive(Debug)]
pub enum EnumError {
    /// A `*.toml` def failed to parse.
    Parse(toml::de::Error),
    /// A directory could not be read.
    Io(std::io::Error),
    /// A value was edited to a code that is not a labelled member of the bound enum.
    NotAMember {
        table: String,
        field: String,
        enum_name: String,
        value: i64,
    },
}

impl std::fmt::Display for EnumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnumError::Parse(e) => write!(f, "enum def parse error: {e}"),
            EnumError::Io(e) => write!(f, "enum def io error: {e}"),
            EnumError::NotAMember {
                table,
                field,
                enum_name,
                value,
            } => write!(
                f,
                "value {value} is not a member of enum {enum_name:?} bound to {table}.{field}"
            ),
        }
    }
}
impl std::error::Error for EnumError {}

/// Reverse a 4-char (or any) raw id between the studio and on-disk/binary conventions.
///
/// `ubPc` ↔ `cPbu`, `vjla` ↔ `aljv`, `eGlu` ↔ `ulGe`. The mapping is its own inverse.
pub fn reverse_id(id: &str) -> String {
    id.chars().rev().collect()
}

/// All confirmed enums, indexed by the studio `(table, field)` they bind to.
#[derive(Debug, Default)]
pub struct EnumRegistry {
    defs: Vec<EnumDef>,
    by_field: HashMap<(String, String), usize>,
}

impl EnumRegistry {
    /// Load every `*.toml` enum def from a directory (e.g. the repo's `defs/enums`).
    pub fn from_dir(dir: impl AsRef<Path>) -> Result<Self, EnumError> {
        let mut reg = EnumRegistry::default();
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .map_err(EnumError::Io)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        // Deterministic load order regardless of filesystem enumeration.
        paths.sort();
        for path in paths {
            let text = std::fs::read_to_string(&path).map_err(EnumError::Io)?;
            reg.insert(EnumDef::from_toml(&text)?);
        }
        Ok(reg)
    }

    /// Add one def, registering each of its `(table, field)` bindings.
    pub fn insert(&mut self, def: EnumDef) {
        let i = self.defs.len();
        for b in &def.bindings {
            self.by_field
                .insert((b.table_raw_id.clone(), b.field_raw_id.clone()), i);
        }
        self.defs.push(def);
    }

    /// The enum bound to a studio `(table, field)`, if any.
    pub fn for_field(&self, table: &str, field: &str) -> Option<&EnumDef> {
        self.by_field
            .get(&(table.to_string(), field.to_string()))
            .map(|&i| &self.defs[i])
    }

    /// The label for a raw code at a bound `(table, field)`, if both the binding and code are known.
    pub fn label(&self, table: &str, field: &str, raw: i64) -> Option<&str> {
        self.for_field(table, field).and_then(|d| d.label(raw))
    }

    /// Validate an edit's enum membership. Fields with **no** bound enum are unconstrained here
    /// (range/`updatable` checks live in `tdb-defs`); fields **with** one must take a labelled code.
    pub fn validate(&self, table: &str, field: &str, value: i64) -> Result<(), EnumError> {
        match self.for_field(table, field) {
            Some(def) if !def.contains(value) => Err(EnumError::NotAMember {
                table: table.to_string(),
                field: field.to_string(),
                enum_name: def.name.clone(),
                value,
            }),
            _ => Ok(()),
        }
    }

    /// Every loaded def.
    pub fn defs(&self) -> &[EnumDef] {
        &self.defs
    }

    pub fn len(&self) -> usize {
        self.defs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POSITION: &str = r#"
name = "Position"
confidence = "Confirmed"
[[bindings]]
field_name = "Position"
field_raw_id = "vjla"
table_raw_id = "ubPc"
[[entries]]
raw = 0
label = "C"
[[entries]]
raw = 4
label = "G"
"#;

    #[test]
    fn parses_and_maps() {
        let d = EnumDef::from_toml(POSITION).unwrap();
        assert_eq!(d.name, "Position");
        assert_eq!(d.label(0), Some("C"));
        assert_eq!(d.label(4), Some("G"));
        assert_eq!(d.raw_of("G"), Some(4));
        assert!(d.contains(0) && !d.contains(2));
        assert_eq!(d.max_raw(), Some(4));
    }

    #[test]
    fn registry_validates_membership() {
        let mut reg = EnumRegistry::default();
        reg.insert(EnumDef::from_toml(POSITION).unwrap());
        assert_eq!(reg.label("ubPc", "vjla", 4), Some("G"));
        assert!(reg.validate("ubPc", "vjla", 0).is_ok());
        assert!(matches!(
            reg.validate("ubPc", "vjla", 7),
            Err(EnumError::NotAMember { .. })
        ));
        // Unbound field is unconstrained by the enum layer.
        assert!(reg.validate("ubPc", "zzzz", 999).is_ok());
    }

    #[test]
    fn reverse_id_is_involution() {
        for id in ["ubPc", "vjla", "eGlu", "mCcl"] {
            assert_eq!(reverse_id(&reverse_id(id)), id);
        }
        assert_eq!(reverse_id("ubPc"), "cPbu");
        assert_eq!(reverse_id("vjla"), "aljv");
    }
}
