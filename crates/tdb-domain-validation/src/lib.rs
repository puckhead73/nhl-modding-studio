//! `tdb-domain-validation` — the whole-DB integrity engine (EDIT-MODEL Layer 4).
//!
//! A single set of **shared predicates** runs over a decoded [`TdbFile`] plus the confirmed
//! [`RelationRegistry`] / [`EnumRegistry`]. The same predicate functions are meant to back both
//! *operation-time* checks (inside a future repairing transaction) and the *save-time* whole-DB audit
//! — so the two can never drift. This first increment ships the two predicates that are directly
//! verifiable against real anchor data:
//!
//! * [`referential_integrity`] — every populated source value of a confirmed foreign key resolves to
//!   an existing target row (documented `0` / all-ones sentinels excluded). Unresolved values surface
//!   as [`Severity::Warning`], **not** `Error`: the relationship defs do not carry machine-readable FK
//!   *totality*, and real data legitimately holds partial-subset FKs (e.g. `team-links` keeps 30
//!   orphan-by-design values) and secondary null sentinels (e.g. `ZBac.ykFq`'s `16382`). Hard-failing
//!   on those would refuse documented-clean states — the precise failure mode EDIT-MODEL rejects.
//!   Promoting a specific *total* FK to `Error` is an edit-time / entity-aware concern (later layer).
//! * [`enum_vocabulary`] — every decoded value of an enum-bound field is a labelled member of that
//!   enum. Advisory: some codes are intentionally reachable-but-unlabelled (e.g. captaincy raw `3`,
//!   nationality `127` "N/A"), so these surface as [`Severity::Info`], not errors.
//! * [`companion_presence`] — every `ubPc` player has the companion pair its `vjla` kind requires
//!   (`dSvy`+`xmja` for skaters, `mHuy`+`fMVl` for goalies, all sharing `wBIz`). A missing companion
//!   is a structurally-broken cluster, so this is the first [`Severity::Error`] predicate; it reuses
//!   the Layer 2 [`RosterModel`] invariant directly.
//!
//! Deferred to later increments (they need the repairing-transaction harness): dup-on-team, `toWX`
//! roster-block coherence, and the single-captain rule. Each lands here as another shared predicate
//! so save-time and operation-time keep calling the same code.
//!
//! ## Id convention
//!
//! The registries speak **studio ids** (`ubPc`, `eGlu`, `kOtt`). [`TdbFile::table_canonical`] resolves
//! those across the big-endian on-disk reversal; field lookups use [`tdb_core::RawId::canonical`] the
//! same way. Tables/fields absent from a given db variant are skipped, not flagged.

use std::collections::{HashMap, HashSet};
use tdb_core::{Endian, FieldDescriptor, Table, TdbFile};
use tdb_enums::EnumRegistry;
use tdb_relations::{RelBinding, RelationRegistry, RosterModel};

/// Severity of an integrity finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// A broken invariant — illegal state that must not be written.
    Error,
    /// Suspicious but not provably illegal.
    Warning,
    /// Informational (e.g. a valid-but-unlabelled enum code).
    Info,
}

/// One integrity finding, located by studio `(table, field, record)` where applicable.
#[derive(Clone, Debug)]
pub struct Finding {
    pub severity: Severity,
    /// The predicate that produced it (stable identifier for grouping/telemetry).
    pub predicate: &'static str,
    pub message: String,
    pub table: String,
    pub field: String,
    pub record: Option<u32>,
}

/// The result of a whole-DB audit.
#[derive(Clone, Debug, Default)]
pub struct IntegrityReport {
    pub findings: Vec<Finding>,
}

impl IntegrityReport {
    pub fn errors(&self) -> impl Iterator<Item = &Finding> {
        self.findings.iter().filter(|f| f.severity == Severity::Error)
    }

    /// No `Error`-severity findings — the db is safe to write.
    pub fn is_clean(&self) -> bool {
        self.errors().next().is_none()
    }

    pub fn count(&self, sev: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == sev).count()
    }
}

/// Resolve a field on `table` by its **studio** id (handling the BE reversal).
fn field_by_studio<'a>(
    endian: Endian,
    table: &'a Table,
    studio_field: &str,
) -> Option<&'a FieldDescriptor> {
    table
        .fields
        .iter()
        .find(|f| f.id.canonical(endian) == studio_field)
}

/// The all-ones value for a `width`-bit field — the `-1`/"none" sentinel.
fn all_ones(width: u32) -> u64 {
    if width >= 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

/// Per-edge referential-integrity result. `None` from [`audit_edge`] means the edge's tables/fields
/// are not present in this db variant (skipped, not a violation).
#[derive(Clone, Debug)]
pub struct EdgeAudit {
    pub source_table: String,
    pub source_field: String,
    pub target_table: String,
    pub target_field: String,
    /// Source records whose value is neither `0` nor the all-ones sentinel.
    pub source_populated: usize,
    /// Of those, how many resolve to an existing target value.
    pub resolved: usize,
    /// The unresolved `(record, value)` pairs — dangling foreign keys.
    pub dangling: Vec<(u32, u64)>,
}

impl EdgeAudit {
    /// Fraction of populated source values that resolve (1.0 when there are none to resolve).
    pub fn resolution_rate(&self) -> f64 {
        if self.source_populated == 0 {
            1.0
        } else {
            self.resolved as f64 / self.source_populated as f64
        }
    }
}

/// Audit one confirmed FK binding against the decoded db. Returns `None` if either endpoint's
/// table/field is absent from this db, or the fields are not fixed-bit readable.
pub fn audit_edge(db: &TdbFile, b: &RelBinding) -> Option<EdgeAudit> {
    let src_t = db.table_canonical(&b.source_table)?;
    let tgt_t = db.table_canonical(&b.target_table)?;
    let src_f = field_by_studio(db.endian, src_t, &b.source_field)?;
    let tgt_f = field_by_studio(db.endian, tgt_t, &b.target_field)?;

    // Target value set.
    let mut targets: HashSet<u64> = HashSet::with_capacity(tgt_t.current_records as usize);
    for i in 0..tgt_t.current_records as u32 {
        if let Some(v) = db.read_u64(tgt_t, i, tgt_f) {
            targets.insert(v);
        }
    }

    let sentinel = all_ones(src_f.bit_width);
    let mut source_populated = 0;
    let mut resolved = 0;
    let mut dangling = Vec::new();
    for i in 0..src_t.current_records as u32 {
        let Some(v) = db.read_u64(src_t, i, src_f) else {
            continue;
        };
        if v == 0 || v == sentinel {
            continue; // documented null / "none" sentinels
        }
        source_populated += 1;
        if targets.contains(&v) {
            resolved += 1;
        } else {
            dangling.push((i, v));
        }
    }

    Some(EdgeAudit {
        source_table: b.source_table.clone(),
        source_field: b.source_field.clone(),
        target_table: b.target_table.clone(),
        target_field: b.target_field.clone(),
        source_populated,
        resolved,
        dangling,
    })
}

/// Referential-integrity predicate: a dangling-FK [`Finding`] per unresolved source value across
/// every confirmed relationship binding present in this db.
pub fn referential_integrity(db: &TdbFile, relations: &RelationRegistry) -> Vec<Finding> {
    let mut findings = Vec::new();
    for rel in relations.relationships() {
        for b in &rel.bindings {
            let Some(audit) = audit_edge(db, b) else {
                continue;
            };
            for (record, value) in audit.dangling {
                findings.push(Finding {
                    severity: Severity::Warning,
                    predicate: "referential_integrity",
                    message: format!(
                        "{}.{} = {} has no matching {}.{} ({})",
                        b.source_table,
                        b.source_field,
                        value,
                        b.target_table,
                        b.target_field,
                        rel.name
                    ),
                    table: b.source_table.clone(),
                    field: b.source_field.clone(),
                    record: Some(record),
                });
            }
        }
    }
    findings
}

/// Enum-vocabulary predicate: an [`Severity::Info`] [`Finding`] per decoded value that is not a
/// labelled member of its bound enum (reachable-but-unlabelled codes are expected, hence `Info`).
pub fn enum_vocabulary(db: &TdbFile, enums: &EnumRegistry) -> Vec<Finding> {
    let mut findings = Vec::new();
    for def in enums.defs() {
        for binding in &def.bindings {
            let Some(t) = db.table_canonical(&binding.table_raw_id) else {
                continue;
            };
            let Some(f) = field_by_studio(db.endian, t, &binding.field_raw_id) else {
                continue;
            };
            for i in 0..t.current_records as u32 {
                let Some(v) = db.read_u64(t, i, f) else {
                    continue;
                };
                if !def.contains(v as i64) {
                    findings.push(Finding {
                        severity: Severity::Info,
                        predicate: "enum_vocabulary",
                        message: format!(
                            "{}.{} = {} is not a labelled member of enum {:?}",
                            binding.table_raw_id, binding.field_raw_id, v, def.name
                        ),
                        table: binding.table_raw_id.clone(),
                        field: binding.field_raw_id.clone(),
                        record: Some(i),
                    });
                }
            }
        }
    }
    findings
}

/// Companion-presence predicate: every player must carry the companion pair its kind requires
/// (`dSvy`+`xmja` for skaters, `mHuy`+`fMVl` for goalies). A missing companion is a structurally
/// broken Player cluster — an [`Severity::Error`]. Reuses the Layer 2 [`RosterModel`] invariant so
/// operation-time and save-time share one definition of "a Player is the whole cluster". Returns no
/// findings if the cluster tables are absent from this db variant.
pub fn companion_presence(db: &TdbFile) -> Vec<Finding> {
    let Ok(model) = RosterModel::build(db) else {
        return Vec::new();
    };
    model
        .companion_violations()
        .iter()
        .map(|p| Finding {
            severity: Severity::Error,
            predicate: "companion_presence",
            message: format!(
                "player wBIz={} ({:?}, {:?}) is missing a required companion row",
                p.player_id, p.kind, p.position
            ),
            table: tdb_relations::entity::ids::BIO.to_string(),
            field: tdb_relations::entity::ids::PLAYER_ID.to_string(),
            record: Some(p.bio_record),
        })
        .collect()
}

/// The captaincy code that designates a team Captain (`eGlu.mCcl == 1`, confirmed enum).
pub const CAPTAIN: u64 = 1;

/// Single-captain predicate: a team must carry at most one Captain (`eGlu.mCcl == 1`). Each captain
/// beyond the first is an [`Severity::Error`]. Shared with the Layer 3 `set_captaincy` operation, so
/// its auto-repair and this audit agree by construction.
pub fn single_captain(db: &TdbFile) -> Vec<Finding> {
    let Ok(model) = RosterModel::build(db) else {
        return Vec::new();
    };
    let mut by_team: HashMap<u64, Vec<u32>> = HashMap::new();
    for slot in &model.slots {
        if slot.captaincy == Some(CAPTAIN) {
            by_team.entry(slot.team_id).or_default().push(slot.record);
        }
    }
    let mut findings = Vec::new();
    for (team_id, records) in by_team {
        if records.len() > 1 {
            // Flag every captain past the first.
            for &record in records.iter().skip(1) {
                findings.push(Finding {
                    severity: Severity::Error,
                    predicate: "single_captain",
                    message: format!(
                        "team {team_id} has {} captains (eGlu.mCcl == 1); only one is allowed",
                        records.len()
                    ),
                    table: tdb_relations::entity::ids::ROSTER_LINES.to_string(),
                    field: tdb_relations::entity::ids::SLOT_CAPTAINCY.to_string(),
                    record: Some(record),
                });
            }
        }
    }
    findings
}

/// Run every shared predicate and collect the findings into one report (the save-time audit entry).
pub fn validate_invariants(
    db: &TdbFile,
    relations: &RelationRegistry,
    enums: &EnumRegistry,
) -> IntegrityReport {
    let mut findings = referential_integrity(db, relations);
    findings.extend(enum_vocabulary(db, enums));
    findings.extend(companion_presence(db));
    findings.extend(single_captain(db));
    IntegrityReport { findings }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_clean_when_no_errors() {
        let mut r = IntegrityReport::default();
        assert!(r.is_clean());
        r.findings.push(Finding {
            severity: Severity::Info,
            predicate: "enum_vocabulary",
            message: "x".into(),
            table: "t".into(),
            field: "f".into(),
            record: Some(0),
        });
        assert!(r.is_clean(), "Info findings do not block a write");
        r.findings.push(Finding {
            severity: Severity::Error,
            predicate: "referential_integrity",
            message: "y".into(),
            table: "t".into(),
            field: "f".into(),
            record: Some(1),
        });
        assert!(!r.is_clean());
        assert_eq!(r.count(Severity::Info), 1);
        assert_eq!(r.count(Severity::Error), 1);
    }

    #[test]
    fn all_ones_sentinel() {
        assert_eq!(all_ones(8), 0xFF);
        assert_eq!(all_ones(1), 1);
        assert_eq!(all_ones(64), u64::MAX);
    }
}
