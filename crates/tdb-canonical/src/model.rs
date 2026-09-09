//! Document types for the three canonical/adapter file kinds.
//!
//! Every struct is `deny_unknown_fields`: a misspelled constraint key is a **parse error**, never a
//! silently ignored line. That is the difference between "the format does not support this fact" and
//! "the format quietly dropped your fact", and only the first is acceptable in a registry that Build
//! generation reads.

use serde::Deserialize;
use std::collections::BTreeSet;

// ============================================================ canonical schema

/// `defs/canonical/project-schema.toml`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProjectSchema {
    pub schema: String,
    pub version: u32,
    #[serde(default, rename = "domain")]
    pub domains: Vec<Domain>,
    #[serde(default, rename = "entity")]
    pub entities: Vec<Entity>,
}

/// A value domain: what a property *means*, independently of any target encoding.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Domain {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub max: Option<i64>,
    #[serde(default)]
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub identity: Vec<String>,
    #[serde(default, rename = "property")]
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Property {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub references: Option<String>,
    #[serde(default)]
    pub description: String,
}

impl ProjectSchema {
    /// Every `<entity>.<property>` path the schema defines.
    pub fn property_paths(&self) -> BTreeSet<String> {
        self.entities
            .iter()
            .flat_map(|e| {
                e.properties
                    .iter()
                    .map(move |p| format!("{}.{}", e.id, p.id))
            })
            .collect()
    }

    /// The property at `<entity>.<property>`, if the schema defines it.
    pub fn property(&self, path: &str) -> Option<&Property> {
        let (entity, property) = path.split_once('.')?;
        self.entities
            .iter()
            .find(|e| e.id == entity)?
            .properties
            .iter()
            .find(|p| p.id == property)
    }

    /// The domain a property resolves to, if it declares one that exists.
    pub fn domain_of(&self, property: &Property) -> Option<&Domain> {
        let id = property.domain.as_deref()?;
        self.domains.iter().find(|d| d.id == id)
    }
}

// ======================================================= constraint envelope

/// Whether a constraint is **established from evidence**, **unknown**, or **not applicable** to this
/// target. These are three different things and the format keeps them apart:
///
/// * `Established` — a real fact, with a confidence and at least one provenance citation;
/// * `Unknown` — nothing in the repository establishes it. It carries no value, and export must
///   block rather than substitute one;
/// * `NotApplicable` — the target genuinely does not have this constraint (no such field, no such
///   sentinel, no such companion), with a `reason` recording how the absence was determined.
///
/// A missing block defaults to `Unknown`, so silence is never read as a fact.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum ConstraintState {
    Established,
    Unknown,
    NotApplicable,
}

impl ConstraintState {
    pub fn as_str(self) -> &'static str {
        match self {
            ConstraintState::Established => "established",
            ConstraintState::Unknown => "unknown",
            ConstraintState::NotApplicable => "not-applicable",
        }
    }
}

/// **Non-operative** candidate information for a constraint that is *not* established.
///
/// A candidate records a proposed value and its citation without letting anything act on it. It is
/// the place for a documented expectation that no evidence in this repository proves for *this*
/// target: a runtime-probed encoding, a foreign key inferred from a field name, a code mapping
/// confirmed on a neighbouring title. Nothing reads a candidate: [`Constraint::established`] never
/// returns it, so no derivation, default or export decision can be built on one.
///
/// A candidate may only accompany `state = "unknown"`. Attaching one to a decided constraint is a
/// contradiction and is refused.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, bound(deserialize = "T: Deserialize<'de>"))]
pub struct Candidate<T> {
    /// How well the *candidate* is known. `asserted` is legal here — that is the point of the
    /// channel — and is not legal on an established constraint.
    #[serde(default)]
    pub confidence: Option<String>,
    #[serde(default)]
    pub provenance: Vec<String>,
    /// What would have to be shown to promote this candidate to an established constraint.
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub value: Option<T>,
}

/// One evidence-qualified constraint. The payload lives in a nested `value` table so that a value
/// present under a non-`established` state is structurally visible, and therefore refusable.
///
/// An **established** constraint is operative: derivations, defaults and export read it. It must
/// therefore be `measured` or `derived`. A merely `asserted` claim — a documented expectation, a
/// name-based inference, a fact confirmed on another title — is not operative and belongs in
/// [`Constraint::candidate`], where it keeps its citation without driving behaviour.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, bound(deserialize = "T: Deserialize<'de>"))]
pub struct Constraint<T> {
    pub state: ConstraintState,
    /// Required when `state = "established"`, and then restricted to
    /// [`crate::OPERATIVE_CONFIDENCE`].
    #[serde(default)]
    pub confidence: Option<String>,
    #[serde(default)]
    pub provenance: Vec<String>,
    /// Required when `state = "not-applicable"` — how the absence was determined.
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub note: String,
    /// Required when `confidence = "derived"`: the premises this conclusion rests on, why each is
    /// applicable to THIS target, and what the derivation does not establish.
    ///
    /// A confidence label and a citation string are not proof. `derived` means a conclusion
    /// supported by established premises applicable to this target; a cross-title analogy with no
    /// proven equivalence bridge is a [`Candidate`], however well the layouts or names match. This
    /// field is where that argument is written down and reviewable.
    #[serde(default)]
    pub derivation: String,
    #[serde(default)]
    pub value: Option<T>,
    /// A proposed value, recorded but not operative. Only legal under `state = "unknown"`.
    #[serde(default)]
    pub candidate: Option<Candidate<T>>,
}

impl<T> Default for Constraint<T> {
    fn default() -> Self {
        Constraint {
            state: ConstraintState::Unknown,
            confidence: None,
            provenance: Vec::new(),
            reason: String::new(),
            note: String::new(),
            derivation: String::new(),
            value: None,
            candidate: None,
        }
    }
}

impl<T> Constraint<T> {
    /// The payload, **only** when the constraint is actually established.
    ///
    /// This is the single accessor every derivation and every export decision goes through, and it
    /// deliberately cannot see a [`Candidate`]. A proposed value can therefore never leak into a
    /// human range, a Clear-to-Defaults value or a Build mapping by accident.
    pub fn established(&self) -> Option<&T> {
        match self.state {
            ConstraintState::Established => self.value.as_ref(),
            _ => None,
        }
    }

    /// The proposed, **non-operative** value, if one is recorded. Callers that use this are
    /// reporting it to a human, never acting on it.
    pub fn candidate_value(&self) -> Option<&T> {
        self.candidate.as_ref()?.value.as_ref()
    }
}

// ======================================================= constraint payloads

/// Physical storage of the target field: type, offset, width, signedness, range and endianness.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StorageValue {
    /// [`crate::STORAGE_KIND`].
    pub kind: String,
    pub bit_offset: u32,
    pub bit_width: u32,
    /// [`crate::SIGNEDNESS`].
    pub signedness: String,
    pub range_low: i64,
    pub range_high: i64,
    /// [`crate::ENDIAN`].
    pub endian: String,
}

/// Update / key / structural status of the target field.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StatusValue {
    /// The field is part of the table's primary key.
    pub key: bool,
    /// EA metadata marks the field updatable.
    pub updatable: bool,
    /// The field carries relational structure (slot ownership, companion links, line flags) and is
    /// therefore not a scalar edit even when `updatable`.
    pub structural: bool,
}

/// The scale / offset / index-origin semantics behind the transform names.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransformValue {
    /// [`crate::TRANSFORM_KIND`].
    pub kind: String,
    /// For `index-origin`: the first stored index. Canonical is always 1-origin.
    #[serde(default)]
    pub index_origin: Option<i64>,
    /// For `linear-offset`: `human = stored + offset`.
    #[serde(default)]
    pub offset: Option<i64>,
}

/// Enum vocabulary **and its stored codes** on this target.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnumerationValue {
    #[serde(default, rename = "entry")]
    pub entries: Vec<EnumEntry>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnumEntry {
    /// The value actually stored in the field's bits.
    pub code: i64,
    /// The canonical domain value this code means.
    pub canonical: String,
}

/// The target's null / "none" encoding for this field.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NullSentinelValue {
    /// [`crate::SENTINEL_ENCODING`].
    pub encoding: String,
    /// The stored code that means "absent". Required unless `encoding = "none"`.
    #[serde(default)]
    pub raw: Option<i64>,
    /// [`crate::SENTINEL_MEANING`] — what the canonical model reads the sentinel as.
    #[serde(default)]
    pub canonical_meaning: Option<String>,
}

/// String encoding and pool behaviour.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StringStorageValue {
    /// [`crate::STRING_STORAGE`].
    pub storage: String,
    /// Byte budget for `inline-fixed`; meaningless for a pool.
    #[serde(default)]
    pub byte_capacity: Option<u32>,
    /// [`crate::STRING_ENCODING`].
    pub encoding: String,
    /// [`crate::STRING_PADDING`].
    pub padding: String,
    /// [`crate::STRING_OVERFLOW`] — what happens to a value longer than the budget.
    pub over_capacity: String,
}

/// A foreign-key relationship on this target.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ForeignKeyValue {
    pub parent_table: String,
    pub parent_key_field: String,
    /// [`crate::FK_DECLARATION`] — declared by EA metadata, or inferred in this repository.
    pub declaration: String,
    #[serde(default)]
    pub index_origin: Option<i64>,
}

/// Presence / applicability: which records this mapping applies to at all.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ApplicabilityValue {
    /// [`crate::APPLICABILITY_KIND`].
    pub kind: String,
    /// For `canonical-predicate`: the canonical property the predicate reads.
    #[serde(default)]
    pub property: Option<String>,
    /// For `target-predicate`: the discriminating target field.
    #[serde(default)]
    pub table: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
    /// The stored codes for which the mapping applies.
    #[serde(default)]
    pub codes: Vec<i64>,
}

/// Per-table record capacity.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CapacityValue {
    /// Allocated slots — the ceiling an edit may grow into.
    pub ceiling_records: u32,
    /// Vanilla population, for headroom.
    pub population_records: u32,
    /// [`crate::CAPACITY_SOURCE`] — **which file** the ceiling was read from. On gen7 the main DB is
    /// packed and does not expose the ceiling, so the source is load-bearing, not decorative.
    pub source: String,
}

/// Companion-record cardinality for an entity's primary table.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompanionValue {
    /// The key companions share with the primary table.
    pub shared_key_field: String,
    #[serde(default, rename = "companion")]
    pub companions: Vec<CompanionEntry>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompanionEntry {
    pub table: String,
    /// [`crate::CARDINALITY`].
    pub cardinality: String,
    /// [`crate::APPLIES_WHEN`].
    pub applies_when: String,
}

/// How skater and goalie records are told apart — required before any `position-*` companion rule
/// means anything.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PositionDiscriminatorValue {
    pub table: String,
    pub field: String,
    pub skater_codes: Vec<i64>,
    pub goalie_codes: Vec<i64>,
}

// ============================================================ target adapter

/// `defs/adapters/<target>.toml`, schema `target-adapter/2`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Adapter {
    pub schema: String,
    pub adapter_version: u32,
    pub canonical_schema_version: u32,
    pub target: String,
    #[serde(default)]
    pub generation: String,
    #[serde(default)]
    pub endian: String,
    #[serde(default)]
    pub tdb_version: Option<u32>,
    #[serde(default)]
    pub physical_snapshot: Option<String>,
    /// A **labelled synthetic fixture**: it exercises representations for which no real fact is
    /// established. It is never a target profile and is excluded from registry coverage.
    #[serde(default)]
    pub synthetic: bool,
    #[serde(default)]
    pub coverage_declared: Option<Coverage>,
    /// The target tables this adapter refers to. Every `table` reference elsewhere in the file must
    /// resolve here.
    #[serde(default, rename = "table")]
    pub tables: Vec<TableDecl>,
    #[serde(default, rename = "mapping")]
    pub mappings: Vec<Mapping>,
    #[serde(default, rename = "omission")]
    pub omissions: Vec<Omission>,
    #[serde(default, rename = "capacity")]
    pub capacities: Vec<CapacityDecl>,
    #[serde(default, rename = "companion_rule")]
    pub companion_rules: Vec<CompanionRuleDecl>,
    #[serde(default)]
    pub position_discriminator: Option<Constraint<PositionDiscriminatorValue>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    #[serde(default)]
    pub tables_total: u32,
    #[serde(default)]
    pub fields_total: u32,
    #[serde(default)]
    pub fields_mapped: u32,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TableDecl {
    /// Studio (canonical-order) table id, e.g. `ubPc`.
    pub id: String,
    /// EA metadata name, e.g. `exhibitionplayerbiotable`.
    pub name: String,
    #[serde(default)]
    pub provenance: Vec<String>,
    #[serde(default)]
    pub note: String,
}

/// One target field that carries part of a composite canonical value (a date's year/month/day, a
/// colour's channels). Each component carries its own physical storage, because they are separate
/// fields at separate offsets.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FieldComponent {
    /// What this field contributes, e.g. `year`. Unique within the mapping.
    pub role: String,
    pub field: String,
    #[serde(default)]
    pub storage: Constraint<StorageValue>,
    #[serde(default)]
    pub note: String,
}

/// One canonical property's target encoding.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub canonical: String,
    /// Absent means the table identity is **not established**. The literal string `"unknown"` is
    /// refused: a placeholder is not a value.
    #[serde(default)]
    pub table: Option<String>,
    /// The single target field, when the canonical value maps to exactly one. Mutually exclusive
    /// with `[[mapping.component]]`.
    #[serde(default)]
    pub field: Option<String>,
    /// The target fields that jointly carry a composite canonical value. Mutually exclusive with
    /// `field`; when used, the mapping-level `[mapping.storage]` does not apply, because each
    /// component has its own offset and width.
    #[serde(default, rename = "component")]
    pub components: Vec<FieldComponent>,
    pub read_transform: String,
    pub write_transform: String,
    /// Whether a **named case** has proven the write is the inverse of the read.
    #[serde(default)]
    pub inverse_verified: bool,
    /// An evidence-backed target default. The literal `"unknown"` is refused: an unknown default
    /// must block, never substitute.
    #[serde(default)]
    pub default: Option<String>,
    pub confidence: String,
    pub qualification: String,
    #[serde(default)]
    pub automated_case: Option<String>,
    #[serde(default)]
    pub provenance: Vec<String>,
    #[serde(default)]
    pub note: String,

    // -- the constraint blocks. Each defaults to `unknown`, so an absent block is never a fact.
    #[serde(default)]
    pub storage: Constraint<StorageValue>,
    #[serde(default)]
    pub status: Constraint<StatusValue>,
    #[serde(default)]
    pub transform: Constraint<TransformValue>,
    #[serde(default)]
    pub enumeration: Constraint<EnumerationValue>,
    #[serde(default)]
    pub null_sentinel: Constraint<NullSentinelValue>,
    #[serde(default)]
    pub string_storage: Constraint<StringStorageValue>,
    #[serde(default)]
    pub foreign_key: Constraint<ForeignKeyValue>,
    #[serde(default)]
    pub applicability: Constraint<ApplicabilityValue>,
}

/// A canonical property this target does not carry. Reported for approval, never dropped.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Omission {
    pub canonical: String,
    /// [`crate::OMISSION_REASON`] — a closed vocabulary, because "the target has no such field" and
    /// "we have not found the field" are different states and must not share a spelling.
    pub reason: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CapacityDecl {
    pub table: String,
    pub declared: Constraint<CapacityValue>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompanionRuleDecl {
    pub primary_table: String,
    pub declared: Constraint<CompanionValue>,
}
