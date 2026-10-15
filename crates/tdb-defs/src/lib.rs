//! `tdb-defs` — the FieldMeta backbone (EDIT-MODEL Layer 1).
//!
//! Ingests EA's per-game `nhlng-meta.xml` into a typed [`FieldMetaIndex`] keyed by the on-disk
//! (raw) `(table, field)` ids — the same convention `tdb-core` reads. Each [`FieldMeta`] carries the
//! type, bit-depth, legal range, key/updatable flags and FK parent, and knows how to decode/encode
//! signed values (two's complement) and to **validate an edit at entry** (the "illegal states
//! unrepresentable" guardrail: reject non-updatable fields and out-of-`[range_low, range_high]`
//! values — the EA legal range, not merely the bit width).

use std::collections::HashMap;
use std::fmt;

/// Field value class, from the meta `type` (`FIELDTYPE_*` / `DBOFIELDTYPE_*`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldType {
    Integer,
    String,
    Real,
    Other(String),
}

impl FieldType {
    fn parse(s: &str) -> Self {
        match s.rsplit('_').next().unwrap_or(s) {
            "INTEGER" => FieldType::Integer,
            "STRING" => FieldType::String,
            "REAL" => FieldType::Real,
            other => FieldType::Other(other.to_string()),
        }
    }
}

/// Raw `(table, field)` sites verified to store **offset-binary** values — `raw = value - range_low`
/// — instead of the default two's-complement interpretation of a negative `range_low`.
///
/// Anchored via the prior app's real-data checks (Stastny 2014-15 `+5` stored as raw `69` on the
/// 8-bit `[-64..127]` season plus/minus; two's complement would read `+69`, impossible in the NHL).
/// Scoped to the anchored sites only — flipping every `range_low < 0` field would need its own
/// verification pass (see the format note in `docs/formats/tdb-v6-format.md`).
const OFFSET_BINARY_SITES: &[(&str, &str)] = &[
    ("ySst", "tmoU"), // stock career skater season plus/minus (Uomt), 8-bit [-64..127]
    ("OqAO", "tmoU"), // stock career skater playoffs plus/minus (Uomt), 6-bit [-32..31]
];

/// One declared field of a table, as the meta XML states it: raw field id, bit depth, legal range.
/// Used to pin a [`ScopedOffsetSite`] to an exact table declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeclaredField {
    pub field: &'static str,
    pub depth: u32,
    pub range_low: i64,
    pub range_high: i64,
}

/// How strongly an offset-binary reading is witnessed. Surfaces must keep this distinction
/// (P1-BS-R1 acceptance): a *structural* site is an inferred reading, not an externally confirmed one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffsetEvidence {
    /// Real-data anchor from the prior app's checks (the career plus/minus sites).
    Anchored,
    /// An independent compiled copy of the same table in the title's executable agrees with the
    /// offset reading (NHL 15 PS4 `stocksettings.id` 222/223, `stocksettings.min` 222/223) and the
    /// table's own size arithmetic (`size == max - min + 1`, 213/223) agrees.
    ExecutableWitnessed,
    /// The decoded values resolve to ids that a sibling title names by lineage (NHL 16
    /// `group.csv` parents 36/74/75 for `stocksettingcontext.subgroupparent`); no executable
    /// witness on this title.
    LineageInferred,
    /// Only the field's own value distribution supports the reading (raw 0 on every
    /// non-subgroup row, raw 2–7 on the six sub-groups for `stocksettingcontext.subgrouporder`);
    /// **unverified against any external witness**.
    StructuralOnly,
}

impl OffsetEvidence {
    /// Short user-facing wording for a disclosure pill.
    pub fn label(self) -> &'static str {
        match self {
            OffsetEvidence::Anchored => "anchored on real data",
            OffsetEvidence::ExecutableWitnessed => "witnessed by the executable's copy",
            OffsetEvidence::LineageInferred => "inferred by lineage",
            OffsetEvidence::StructuralOnly => "structural only (unverified externally)",
        }
    }
}

/// A raw `(table, field)` site that stores offset-binary values **only under one exact table
/// declaration**. Unlike [`OFFSET_BINARY_SITES`] the match is keyed by the table *and* the field
/// *and* the table's full declared field set (every raw field id with its depth and range), so
/// the reading applies to metadata declaring exactly this table shape — the NHL 15 PS4 base and
/// patch 01.06 `nhl_game-meta.xml` (identical bytes) and any unseen NHL 15 PS4 build with the same
/// declaration — and to nothing else: another title's metadata, a differently declared variant,
/// or a field-id collision such as `qbPU.CkjP` (`leaguesimpopups.id`, 14-bit `0..16383`) is
/// untouched. This is not a general negative-`rangelow` rule; `tdb-defs` keeps two's complement
/// for every other field.
#[derive(Clone, Copy, Debug)]
pub struct ScopedOffsetSite {
    pub table: &'static str,
    pub field: &'static str,
    /// The complete declared field set of `table` (order-independent) that the site is scoped to.
    pub table_shape: &'static [DeclaredField],
    pub evidence: OffsetEvidence,
    /// Which title/platform declaration the shape was measured on.
    pub measured_on: &'static str,
}

/// NHL 15 PS4 `stocksettings` (`fRvu`) — the eight declared fields (handoff 22 §1.1 / §1.9).
const NHL15_PS4_STOCKSETTINGS: &[DeclaredField] = &[
    DeclaredField { field: "jjHq", depth: 8, range_low: 0, range_high: 255 }, // legacyid (key)
    DeclaredField { field: "cVLR", depth: 2, range_low: 0, range_high: 3 },   // style
    DeclaredField { field: "npHF", depth: 3, range_low: 0, range_high: 7 },   // default
    DeclaredField { field: "sAhV", depth: 400, range_low: 0, range_high: 0 }, // stringid
    DeclaredField { field: "CkjP", depth: 10, range_low: -1, range_high: 511 }, // id
    DeclaredField { field: "SbEu", depth: 4, range_low: -8, range_high: 7 },  // min
    DeclaredField { field: "qrwM", depth: 5, range_low: 0, range_high: 31 },  // size
    DeclaredField { field: "KODr", depth: 5, range_low: 0, range_high: 31 },  // max
];

/// NHL 15 PS4 `stocksettingcontext` (`HwRX`) — the five declared fields.
const NHL15_PS4_STOCKSETTINGCONTEXT: &[DeclaredField] = &[
    DeclaredField { field: "mmiA", depth: 6, range_low: 0, range_high: 63 },   // context (key)
    DeclaredField { field: "OAja", depth: 7, range_low: 0, range_high: 127 },  // group
    DeclaredField { field: "QHRR", depth: 7, range_low: -1, range_high: 126 }, // subgroupparent
    DeclaredField { field: "ORkb", depth: 3, range_low: 0, range_high: 7 },    // order
    DeclaredField { field: "NVBd", depth: 5, range_low: -1, range_high: 15 },  // subgrouporder
];

const NHL15_PS4_META: &str = "NHL 15 PS4 nhl_game-meta.xml (base 01.00 anchor = patch 01.06 bytes)";

/// The declaration-scoped offset-binary sites (P1-BS-1). Evidence per site as recorded in
/// handoff 22 §1.2 / §1.9; the `subgrouporder` site is carried with its structural-only witness
/// deliberately marked, not promoted.
pub const SCOPED_OFFSET_BINARY_SITES: &[ScopedOffsetSite] = &[
    ScopedOffsetSite {
        table: "fRvu",
        field: "SbEu", // stocksettings.min, 4-bit [-8..7]: raw 8 = 0, raw 11 = 3, raw 1 = -7
        table_shape: NHL15_PS4_STOCKSETTINGS,
        evidence: OffsetEvidence::ExecutableWitnessed,
        measured_on: NHL15_PS4_META,
    },
    ScopedOffsetSite {
        table: "fRvu",
        field: "CkjP", // stocksettings.id, 10-bit [-1..511]: raw 153 = 152 (PlayoffSeriesLength)
        table_shape: NHL15_PS4_STOCKSETTINGS,
        evidence: OffsetEvidence::ExecutableWitnessed,
        measured_on: NHL15_PS4_META,
    },
    ScopedOffsetSite {
        table: "HwRX",
        field: "QHRR", // stocksettingcontext.subgroupparent, 7-bit [-1..126]: raw 37 = 36, raw 0 = -1
        table_shape: NHL15_PS4_STOCKSETTINGCONTEXT,
        evidence: OffsetEvidence::LineageInferred,
        measured_on: NHL15_PS4_META,
    },
    ScopedOffsetSite {
        table: "HwRX",
        field: "NVBd", // stocksettingcontext.subgrouporder, 5-bit [-1..15]: raw 0 = -1, raw 2..7 = 1..6
        table_shape: NHL15_PS4_STOCKSETTINGCONTEXT,
        evidence: OffsetEvidence::StructuralOnly,
        measured_on: NHL15_PS4_META,
    },
];

/// True iff `declared` (one table's fields as parsed from the meta XML) is exactly `shape`:
/// same raw field ids, same depth and range on each, nothing extra, nothing missing.
fn table_shape_matches(declared: &[(String, u32, Option<i64>, Option<i64>)], shape: &[DeclaredField]) -> bool {
    declared.len() == shape.len()
        && shape.iter().all(|d| {
            declared.iter().any(|(f, depth, lo, hi)| {
                f == d.field && *depth == d.depth && *lo == Some(d.range_low) && *hi == Some(d.range_high)
            })
        })
}

/// Per-field metadata.
#[derive(Clone, Debug)]
pub struct FieldMeta {
    pub table: String,
    pub field: String,
    pub name: String,
    pub kind: FieldType,
    pub bit_depth: u32,
    pub range_low: Option<i64>,
    pub range_high: Option<i64>,
    pub key: bool,
    pub updatable: bool,
    pub fk_parent: Option<String>,
    /// `true` for the [`OFFSET_BINARY_SITES`] and for a [`SCOPED_OFFSET_BINARY_SITES`] entry whose
    /// table declaration matched — decode/encode as `value = raw + range_low` rather than two's
    /// complement.
    pub offset_binary: bool,
    /// How the offset-binary reading is witnessed (`None` when `offset_binary` is false).
    pub offset_evidence: Option<OffsetEvidence>,
}

impl FieldMeta {
    /// True iff the field is **two's-complement signed** — i.e. its legal range goes below zero
    /// *and* fits in `bit_depth` signed bits (e.g. an `-50..=50` slider). A field whose `range_low`
    /// is negative but whose `range_high` exceeds the signed max is **not** two's complement; it is
    /// an unsigned field that uses a sentinel (see [`FieldMeta::neg1_sentinel`]).
    pub fn signed(&self) -> bool {
        if self.offset_binary {
            return false;
        }
        match (self.range_low, self.range_high) {
            (Some(lo), Some(hi)) if lo < 0 => {
                let w = self.bit_depth;
                (1..64).contains(&w) && hi < (1i64 << (w - 1)) && lo >= -(1i64 << (w - 1))
            }
            _ => false,
        }
    }

    /// True iff the field uses an all-ones `-1` sentinel ("none"): `range_low == -1` but it is not
    /// two's-complement signed (e.g. a team-id `-1..=254` at 8 bits, where raw `0xFF` means -1).
    pub fn neg1_sentinel(&self) -> bool {
        !self.offset_binary
            && self.range_low == Some(-1)
            && !self.signed()
            && (1..64).contains(&self.bit_depth)
    }

    /// Decode raw stored bits into the logical (sign-aware) value.
    pub fn decode_raw(&self, raw_bits: u64) -> i64 {
        let w = self.bit_depth;
        if self.offset_binary {
            return (raw_bits as i64) + self.range_low.unwrap_or(0);
        }
        if self.signed() {
            let sign = 1u64 << (w - 1);
            if raw_bits & sign != 0 {
                return (raw_bits as i64) - (1i64 << w);
            }
            raw_bits as i64
        } else if self.neg1_sentinel() && raw_bits == (1u64 << w) - 1 {
            -1
        } else {
            raw_bits as i64
        }
    }

    /// Encode a logical value into raw stored bits (two's complement — or `value - range_low` for
    /// the offset-binary sites — masked to `bit_depth`).
    pub fn encode(&self, value: i64) -> u64 {
        let v = if self.offset_binary {
            value - self.range_low.unwrap_or(0)
        } else {
            value
        };
        let w = self.bit_depth;
        if w >= 64 {
            return v as u64;
        }
        (v as u64) & ((1u64 << w) - 1)
    }

    /// Validate a logical value for an **edit at entry**: rejects non-updatable fields and
    /// values outside the EA legal range.
    pub fn validate(&self, value: i64) -> Result<(), ValidationError> {
        if !self.updatable {
            return Err(ValidationError::NotUpdatable {
                field: self.field.clone(),
            });
        }
        if let Some(lo) = self.range_low {
            if value < lo {
                return Err(ValidationError::OutOfRange {
                    field: self.field.clone(),
                    value,
                    low: lo,
                    high: self.range_high.unwrap_or(i64::MAX),
                });
            }
        }
        if let Some(hi) = self.range_high {
            if value > hi {
                return Err(ValidationError::OutOfRange {
                    field: self.field.clone(),
                    value,
                    low: self.range_low.unwrap_or(i64::MIN),
                    high: hi,
                });
            }
        }
        Ok(())
    }
}

/// Why an edit was rejected at entry.
#[derive(Debug, PartialEq, Eq)]
pub enum ValidationError {
    NotUpdatable {
        field: String,
    },
    OutOfRange {
        field: String,
        value: i64,
        low: i64,
        high: i64,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::NotUpdatable { field } => {
                write!(f, "field {field:?} is not updatable")
            }
            ValidationError::OutOfRange {
                field,
                value,
                low,
                high,
            } => write!(
                f,
                "value {value} out of range [{low}, {high}] for field {field:?}"
            ),
        }
    }
}
impl std::error::Error for ValidationError {}

/// Errors parsing a meta XML document.
#[derive(Debug)]
pub enum MetaError {
    Xml(roxmltree::Error),
}

impl fmt::Display for MetaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetaError::Xml(e) => write!(f, "meta XML parse error: {e}"),
        }
    }
}
impl std::error::Error for MetaError {}

/// A field-metadata index for one database, keyed by raw `(table, field)` ids.
#[derive(Clone, Debug, Default)]
pub struct SaveTableMeta {
    pub insert_id: Option<String>,
    pub delete_id: Option<String>,
    pub update_id: Option<String>,
    pub max_insert: Option<i64>,
    pub max_delete: Option<i64>,
    pub max_update: Option<i64>,
    pub groups: Vec<String>,
    /// Exact XML declarations, separate from the existing editor's editability policy.
    pub update_fields: Vec<String>,
    pub keys: Vec<String>,
}

#[derive(Debug, Default)]
pub struct FieldMetaIndex {
    fields: HashMap<(String, String), FieldMeta>,
    table_names: HashMap<String, String>,
    save_tables: HashMap<String, SaveTableMeta>,
}

impl FieldMetaIndex {
    /// Build the index from an `nhlng-meta.xml` document.
    pub fn from_xml(xml: &str) -> Result<Self, MetaError> {
        let doc = roxmltree::Document::parse(xml).map_err(MetaError::Xml)?;
        let mut idx = FieldMetaIndex::default();
        for table in doc.descendants().filter(|n| n.has_tag_name("table")) {
            let Some(tsn) = table.attribute("shortname") else {
                continue;
            };
            if let Some(name) = table.attribute("name") {
                idx.table_names.insert(tsn.to_string(), name.to_string());
            }
            idx.save_tables.insert(tsn.into(), SaveTableMeta {
                insert_id: table.attribute("insertshortname").map(str::to_owned),
                delete_id: table.attribute("deleteshortname").map(str::to_owned),
                update_id: table.attribute("updateshortname").map(str::to_owned),
                max_insert: table.attribute("maxinsert").and_then(|v| v.parse().ok()),
                max_delete: table.attribute("maxdelete").and_then(|v| v.parse().ok()),
                max_update: table.attribute("maxupdate").and_then(|v| v.parse().ok()),
                groups: table.attribute("savegroups").unwrap_or_default().split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect(),
                update_fields: table.descendants().filter(|f| f.has_tag_name("field") && f.attribute("update") == Some("True")).filter_map(|f| f.attribute("shortname").map(str::to_owned)).collect(),
                keys: table.descendants().filter(|f| f.has_tag_name("field") && f.attribute("key") == Some("True")).filter_map(|f| f.attribute("shortname").map(str::to_owned)).collect(),
            });
            let mut metas = Vec::new();
            for field in table.descendants().filter(|n| n.has_tag_name("field")) {
                let Some(fsn) = field.attribute("shortname") else {
                    continue;
                };
                let anchored = OFFSET_BINARY_SITES.contains(&(tsn, fsn));
                metas.push(FieldMeta {
                    table: tsn.to_string(),
                    field: fsn.to_string(),
                    name: field.attribute("name").unwrap_or_default().to_string(),
                    kind: FieldType::parse(field.attribute("type").unwrap_or_default()),
                    bit_depth: field
                        .attribute("depth")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0),
                    range_low: field.attribute("rangelow").and_then(|v| v.parse().ok()),
                    range_high: field.attribute("rangehigh").and_then(|v| v.parse().ok()),
                    key: field.attribute("key") == Some("True"),
                    // EA marks read-only fields with update="False"; default is updatable.
                    updatable: field.attribute("update") != Some("False"),
                    fk_parent: field.attribute("fkparenttable").map(|s| s.to_string()),
                    offset_binary: anchored,
                    offset_evidence: anchored.then_some(OffsetEvidence::Anchored),
                });
            }
            // Declaration-scoped sites: the whole table's declared shape must match before any of
            // its fields takes the offset reading (table + field + shape, never field id alone).
            let declared: Vec<(String, u32, Option<i64>, Option<i64>)> = metas
                .iter()
                .map(|m| (m.field.clone(), m.bit_depth, m.range_low, m.range_high))
                .collect();
            for site in SCOPED_OFFSET_BINARY_SITES.iter().filter(|s| s.table == tsn) {
                if !table_shape_matches(&declared, site.table_shape) {
                    continue;
                }
                if let Some(m) = metas.iter_mut().find(|m| m.field == site.field) {
                    m.offset_binary = true;
                    m.offset_evidence = Some(site.evidence);
                }
            }
            for meta in metas {
                idx.fields.insert((meta.table.clone(), meta.field.clone()), meta);
            }
        }
        Ok(idx)
    }

    /// Look up a field by raw `(table, field)` ids.
    pub fn field(&self, table: &str, field: &str) -> Option<&FieldMeta> {
        self.fields.get(&(table.to_string(), field.to_string()))
    }

    /// The human table name for a raw table id, if known.
    pub fn table_name(&self, table: &str) -> Option<&str> {
        self.table_names.get(table).map(String::as_str)
    }

    pub fn save_tables(&self) -> impl Iterator<Item = (&str, &SaveTableMeta)> {
        self.save_tables.iter().map(|(id, meta)| (id.as_str(), meta))
    }

    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    pub fn table_count(&self) -> usize {
        self.table_names.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_encode_signed() {
        let m = FieldMeta {
            table: "t".into(),
            field: "f".into(),
            name: "slider".into(),
            kind: FieldType::Integer,
            bit_depth: 8,
            range_low: Some(-50),
            range_high: Some(50),
            key: false,
            updatable: true,
            fk_parent: None,
            offset_binary: false,
            offset_evidence: None,
        };
        assert!(m.signed());
        assert_eq!(m.decode_raw(246), -10);
        assert_eq!(m.decode_raw(206), -50);
        assert_eq!(m.decode_raw(50), 50);
        assert_eq!(m.encode(-10) & 0xFF, 246);
        assert_eq!(m.encode(-50) & 0xFF, 206);
        // Round-trip.
        for v in [-50i64, -1, 0, 1, 50] {
            assert_eq!(m.decode_raw(m.encode(v)), v);
        }
    }

    /// The anchored offset-binary sites: career plus/minus stores `value - range_low` (Stastny
    /// 2014-15 `+5` = raw `69` on the 8-bit season field), not two's complement (`+69`).
    #[test]
    fn decode_encode_offset_binary() {
        let m = FieldMeta {
            table: "ySst".into(),
            field: "tmoU".into(),
            name: "plusminus".into(),
            kind: FieldType::Integer,
            bit_depth: 8,
            range_low: Some(-64),
            range_high: Some(127),
            key: false,
            updatable: true,
            fk_parent: None,
            offset_binary: true,
            offset_evidence: Some(OffsetEvidence::Anchored),
        };
        assert!(!m.signed());
        assert_eq!(m.decode_raw(69), 5);
        assert_eq!(m.decode_raw(64), 0);
        assert_eq!(m.decode_raw(0), -64);
        assert_eq!(m.encode(5), 69);
        assert_eq!(m.encode(-64), 0);
        for v in [-64i64, -5, 0, 5, 63] {
            assert_eq!(m.decode_raw(m.encode(v)), v);
        }
    }

    #[test]
    fn validation() {
        let mut m = FieldMeta {
            table: "t".into(),
            field: "f".into(),
            name: "x".into(),
            kind: FieldType::Integer,
            bit_depth: 8,
            range_low: Some(0),
            range_high: Some(99),
            key: false,
            updatable: true,
            fk_parent: None,
            offset_binary: false,
            offset_evidence: None,
        };
        assert!(m.validate(50).is_ok());
        assert!(matches!(
            m.validate(100),
            Err(ValidationError::OutOfRange { .. })
        ));
        assert!(matches!(
            m.validate(-1),
            Err(ValidationError::OutOfRange { .. })
        ));
        m.updatable = false;
        assert!(matches!(
            m.validate(50),
            Err(ValidationError::NotUpdatable { .. })
        ));
    }
}
