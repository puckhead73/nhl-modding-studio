//! `tdb-relations` — confirmed inter-table relationship registry (EDIT-MODEL Layer 2 join model).
//!
//! Ingests the operator-curated relationship definitions under the repo's `defs/relationships/*.toml`
//! into a typed [`RelationRegistry`]. Each [`Relationship`] is an **explicit, audited** foreign-key
//! promotion (no wildcards, no silent inference) carrying one or more `(source → target)` field
//! [`RelBinding`]s plus the evidence that confirmed it. These edges are what let the entity layer join
//! a roster slot back to a player (`eGlu.XSWT → ZBac.vfEq → ubPc.wBIz`), a player to a team, an
//! attribute row to its bio, and a team to its arena — by construction rather than by heuristic.
//!
//! ## Id convention
//!
//! Like [`tdb_enums`](../tdb_enums), these defs use the **studio / NHLViewNG id convention**
//! (`ubPc`, `eGlu`, `kOtt`, …). The on-disk TDB binary uses the **character-reversed** form
//! (`ubPc`↔`cPbu`). The registry keys by the studio ids verbatim; cross to the binary space at the
//! integration layer.

pub mod entity;
pub use entity::{EntityError, Player, PlayerKind, Position, RosterModel, RosterSlot, Team};

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// The relationship class. EA's confirmed inter-table edges are all foreign keys today; the variant
/// keeps room for future kinds without reshaping the registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum RelKind {
    ForeignKey,
}

/// Join cardinality, as confirmed by the value-overlap audit behind each def.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Cardinality {
    OneToOne,
    ManyToOne,
    ManyToMany,
}

/// Provenance block (`[source]`).
#[derive(Clone, Debug, Deserialize)]
pub struct RelSource {
    pub kind: String,
    #[serde(default)]
    pub description: String,
}

/// One confirmed `source.(table.field) → target.(table.field)` edge.
#[derive(Clone, Debug, Deserialize)]
pub struct RelBinding {
    pub source_table: String,
    pub source_field: String,
    pub target_table: String,
    pub target_field: String,
    #[serde(default)]
    pub evidence_notes: String,
}

/// A single confirmed relationship def (one `defs/relationships/*.toml` file). May carry several
/// bindings under one name (e.g. `team-links` confirms both `PuTc` and `puoR` → `Teix`).
#[derive(Clone, Debug, Deserialize)]
pub struct Relationship {
    pub name: String,
    pub kind: RelKind,
    pub cardinality: Cardinality,
    #[serde(default)]
    pub confidence: String,
    #[serde(default)]
    pub notes: String,
    pub source: Option<RelSource>,
    #[serde(default)]
    pub bindings: Vec<RelBinding>,
}

impl Relationship {
    /// Parse one relationship def from TOML text.
    pub fn from_toml(text: &str) -> Result<Self, RelationError> {
        toml::from_str(text).map_err(RelationError::Parse)
    }
}

/// Errors loading the relationship registry.
#[derive(Debug)]
pub enum RelationError {
    Parse(toml::de::Error),
    Io(std::io::Error),
}

impl std::fmt::Display for RelationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationError::Parse(e) => write!(f, "relationship def parse error: {e}"),
            RelationError::Io(e) => write!(f, "relationship def io error: {e}"),
        }
    }
}
impl std::error::Error for RelationError {}

/// A resolved edge: a binding plus the name/kind/cardinality of the def it came from.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedEdge<'a> {
    pub relationship: &'a Relationship,
    pub binding: &'a RelBinding,
}

/// All confirmed relationships, indexed by their source `(table, field)`.
#[derive(Debug, Default)]
pub struct RelationRegistry {
    relationships: Vec<Relationship>,
    // (source_table, source_field) -> (relationship index, binding index)
    by_source: HashMap<(String, String), (usize, usize)>,
}

impl RelationRegistry {
    /// Load every `*.toml` relationship def from a directory (e.g. `defs/relationships`).
    pub fn from_dir(dir: impl AsRef<Path>) -> Result<Self, RelationError> {
        let mut reg = RelationRegistry::default();
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .map_err(RelationError::Io)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        paths.sort();
        for path in paths {
            let text = std::fs::read_to_string(&path).map_err(RelationError::Io)?;
            reg.insert(Relationship::from_toml(&text)?);
        }
        Ok(reg)
    }

    /// Add one def, registering each of its source `(table, field)` edges.
    pub fn insert(&mut self, rel: Relationship) {
        let ri = self.relationships.len();
        for (bi, b) in rel.bindings.iter().enumerate() {
            self.by_source
                .insert((b.source_table.clone(), b.source_field.clone()), (ri, bi));
        }
        self.relationships.push(rel);
    }

    /// Resolve the edge a source `(table, field)` points along, if it is a confirmed FK.
    pub fn resolve(&self, table: &str, field: &str) -> Option<ResolvedEdge<'_>> {
        let &(ri, bi) = self.by_source.get(&(table.to_string(), field.to_string()))?;
        let relationship = &self.relationships[ri];
        Some(ResolvedEdge {
            relationship,
            binding: &relationship.bindings[bi],
        })
    }

    /// Every confirmed outgoing edge whose source table is `table`.
    pub fn outgoing(&self, table: &str) -> Vec<ResolvedEdge<'_>> {
        self.relationships
            .iter()
            .flat_map(|r| r.bindings.iter().map(move |b| (r, b)))
            .filter(|(_, b)| b.source_table == table)
            .map(|(relationship, binding)| ResolvedEdge {
                relationship,
                binding,
            })
            .collect()
    }

    /// Every loaded relationship def.
    pub fn relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    /// Total confirmed edges across all defs.
    pub fn edge_count(&self) -> usize {
        self.by_source.len()
    }

    pub fn len(&self) -> usize {
        self.relationships.len()
    }

    pub fn is_empty(&self) -> bool {
        self.relationships.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROSTER_INDEX: &str = r#"
name = "roster-player-index"
kind = "ForeignKey"
cardinality = "OneToOne"
confidence = "Confirmed"
[[bindings]]
source_table = "eGlu"
source_field = "XSWT"
target_table = "ZBac"
target_field = "vfEq"
evidence_notes = "1:1"
"#;

    const TEAM_LINKS: &str = r#"
name = "team-links"
kind = "ForeignKey"
cardinality = "ManyToOne"
[[bindings]]
source_table = "PuTc"
source_field = "vfEq"
target_table = "Teix"
target_field = "vfEq"
[[bindings]]
source_table = "puoR"
source_field = "vfEq"
target_table = "Teix"
target_field = "vfEq"
"#;

    #[test]
    fn parses_and_resolves() {
        let r = Relationship::from_toml(ROSTER_INDEX).unwrap();
        assert_eq!(r.kind, RelKind::ForeignKey);
        assert_eq!(r.cardinality, Cardinality::OneToOne);

        let mut reg = RelationRegistry::default();
        reg.insert(r);
        let edge = reg.resolve("eGlu", "XSWT").unwrap();
        assert_eq!(edge.binding.target_table, "ZBac");
        assert_eq!(edge.binding.target_field, "vfEq");
        assert!(reg.resolve("eGlu", "nope").is_none());
    }

    #[test]
    fn multi_binding_def_registers_each_edge() {
        let mut reg = RelationRegistry::default();
        reg.insert(Relationship::from_toml(TEAM_LINKS).unwrap());
        assert_eq!(reg.edge_count(), 2);
        assert!(reg.resolve("PuTc", "vfEq").is_some());
        assert!(reg.resolve("puoR", "vfEq").is_some());
        assert_eq!(reg.outgoing("PuTc").len(), 1);
    }
}
