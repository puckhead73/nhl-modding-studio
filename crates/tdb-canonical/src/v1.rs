//! `target-adapter/1` — the previous adapter schema, kept only so existing documents have a
//! **tested transition** into `target-adapter/2` rather than being silently reinterpreted.
//!
//! The transition is deliberately **conservative**: it never invents a v2 constraint from a v1
//! field. Every v2 constraint block on an upgraded mapping comes out `unknown`, and every v1 field
//! with no v2 destination is reported in [`Upgrade::notes`] rather than dropped. An upgraded adapter
//! is therefore a faithful, lossless-by-report translation — not a promotion.

use serde::Deserialize;

use crate::model::{Adapter, Constraint, Coverage, Mapping, Omission};

/// `defs/adapters/<target>.toml` as it was written under `target-adapter/1`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AdapterV1 {
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
    #[serde(default)]
    pub coverage_declared: Option<Coverage>,
    #[serde(default, rename = "mapping")]
    pub mappings: Vec<MappingV1>,
    #[serde(default, rename = "omission")]
    pub omissions: Vec<OmissionV1>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MappingV1 {
    pub canonical: String,
    #[serde(default)]
    pub table: String,
    #[serde(default)]
    pub field: String,
    pub read_transform: String,
    pub write_transform: String,
    #[serde(default)]
    pub inverse_verified: bool,
    #[serde(default)]
    pub storage_bits: Option<u32>,
    #[serde(default)]
    pub storage_range: Option<[i64; 2]>,
    #[serde(default)]
    pub human_range: Option<[i64; 2]>,
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
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OmissionV1 {
    pub canonical: String,
    pub reason: String,
    #[serde(default)]
    pub detail: String,
}

/// The result of upgrading a v1 document: the v2 adapter plus an explicit account of everything the
/// transition could not carry across as a constraint.
#[derive(Debug, Clone, PartialEq)]
pub struct Upgrade {
    pub adapter: Adapter,
    pub notes: Vec<UpgradeNote>,
}

/// One v1 fact with no v2 destination, recorded rather than discarded.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UpgradeNote {
    /// `<target>/<canonical>`.
    pub subject: String,
    /// The v1 key.
    pub field: String,
    /// Its v1 value, rendered.
    pub value: String,
    /// Why it did not become a v2 constraint.
    pub disposition: String,
}

/// The literal placeholder v1 used for an unestablished identity. In v2 an unestablished identity is
/// an **absent** key, never this string.
pub const V1_UNKNOWN: &str = "unknown";

/// Parse a `target-adapter/1` document.
pub fn parse_adapter_v1(text: &str) -> Result<AdapterV1, toml::de::Error> {
    toml::from_str(text)
}

fn identity(raw: &str) -> Option<String> {
    match raw.trim() {
        "" | V1_UNKNOWN => None,
        other => Some(other.to_string()),
    }
}

/// Translate a `target-adapter/1` document into `target-adapter/2`.
///
/// Every constraint block comes out `unknown`. `storage_bits`, `storage_range` and `human_range` are
/// **not** promoted into `[mapping.storage]`: a bit width without a bit offset, a signedness and an
/// endianness is not an established storage fact, and a derived human range is not a declarable one
/// at all (v2 computes it). They are reported in [`Upgrade::notes`] instead.
pub fn upgrade_v1(source: &AdapterV1) -> Upgrade {
    let mut notes = Vec::new();
    let mut mappings = Vec::new();

    for m in &source.mappings {
        let subject = format!("{}/{}", source.target, m.canonical);
        let mut note = |field: &str, value: String, disposition: &str| {
            notes.push(UpgradeNote {
                subject: subject.clone(),
                field: field.to_string(),
                value,
                disposition: disposition.to_string(),
            });
        };
        if m.table.trim() == V1_UNKNOWN {
            note(
                "table",
                V1_UNKNOWN.to_string(),
                "v1 placeholder string; v2 represents an unestablished identity as an absent key",
            );
        }
        if m.field.trim() == V1_UNKNOWN {
            note(
                "field",
                V1_UNKNOWN.to_string(),
                "v1 placeholder string; v2 represents an unestablished identity as an absent key",
            );
        }
        if let Some(bits) = m.storage_bits {
            note(
                "storage_bits",
                bits.to_string(),
                "a width alone is not an established [mapping.storage]; re-author against the schema snapshot",
            );
        }
        if let Some(range) = m.storage_range {
            note(
                "storage_range",
                format!("[{}, {}]", range[0], range[1]),
                "a range alone is not an established [mapping.storage]; re-author against the schema snapshot",
            );
        }
        if let Some(range) = m.human_range {
            note(
                "human_range",
                format!("[{}, {}]", range[0], range[1]),
                "v2 derives the human range from [mapping.transform] + [mapping.storage] instead of declaring it",
            );
        }

        mappings.push(Mapping {
            canonical: m.canonical.clone(),
            table: identity(&m.table),
            field: identity(&m.field),
            components: Vec::new(),
            read_transform: m.read_transform.clone(),
            write_transform: m.write_transform.clone(),
            inverse_verified: m.inverse_verified,
            default: m.default.clone(),
            confidence: m.confidence.clone(),
            qualification: m.qualification.clone(),
            automated_case: m.automated_case.clone(),
            provenance: m.provenance.clone(),
            note: m.note.clone(),
            storage: Constraint::default(),
            status: Constraint::default(),
            transform: Constraint::default(),
            enumeration: Constraint::default(),
            null_sentinel: Constraint::default(),
            string_storage: Constraint::default(),
            foreign_key: Constraint::default(),
            applicability: Constraint::default(),
        });
    }

    let omissions = source
        .omissions
        .iter()
        .map(|o| Omission {
            canonical: o.canonical.clone(),
            reason: o.reason.clone(),
            detail: o.detail.clone(),
            provenance: Vec::new(),
        })
        .collect();

    // A v1 file declares no tables, so nothing it mentions can be table-resolved. The upgraded
    // document therefore carries the mappings' table ids as declarations with no provenance, which
    // the validator reports rather than accepting — an upgrade produces a *correct translation*, not
    // a passing document.
    let adapter = Adapter {
        schema: crate::ADAPTER_SCHEMA.to_string(),
        adapter_version: source.adapter_version,
        canonical_schema_version: source.canonical_schema_version,
        target: source.target.clone(),
        generation: source.generation.clone(),
        endian: source.endian.clone(),
        tdb_version: source.tdb_version,
        physical_snapshot: source.physical_snapshot.clone(),
        synthetic: false,
        coverage_declared: source.coverage_declared.clone(),
        tables: Vec::new(),
        mappings,
        omissions,
        capacities: Vec::new(),
        companion_rules: Vec::new(),
        position_discriminator: None,
    };

    notes.sort();
    Upgrade { adapter, notes }
}
