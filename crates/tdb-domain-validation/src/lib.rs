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
//!   orphan-by-design values). Documented **secondary null sentinels** (e.g. `ZBac.ykFq`'s `16382`) are
//!   recognized as placeholders by [`is_known_null_sentinel`] and excluded entirely — they are not
//!   dangling keys, so surfacing them as "broken references" only cried wolf. Hard-failing on the
//!   remaining partial-subset orphans would refuse documented-clean states — the failure mode
//!   EDIT-MODEL rejects.
//!   Promoting a specific *total* FK to `Error` is an edit-time / entity-aware concern (later layer).
//! * [`enum_vocabulary`] — every decoded value of an enum-bound field is a labelled member of that
//!   enum. Advisory: some codes are intentionally reachable-but-unlabelled (e.g. captaincy raw `3`,
//!   nationality `127` "N/A"), so these surface as [`Severity::Info`], not errors.
//! * [`companion_presence`] — every `ubPc` player has the companion pair its `vjla` kind requires
//!   (`dSvy`+`xmja` for skaters, `mHuy`+`fMVl` for goalies, all sharing `wBIz`). A missing companion
//!   is a structurally-broken cluster, so this is the first [`Severity::Error`] predicate; it reuses
//!   the Layer 2 [`RosterModel`] invariant directly. The *equipment* half is required only when its
//!   table exists in this db — gen6 schemas (NHL07) ship no `xmja`/`fMVl` table at all, so their
//!   absence is a schema shape, not a broken cluster (the AI half `dSvy`/`mHuy` is always required).
//!
//! * [`single_captain`] — a team carries at most one Captain (`eGlu.mCcl == 1`). [`Severity::Error`];
//!   shared with the Layer 3 `set_captaincy` op so its auto-repair and this audit agree.
//! * [`towx_coherence`] — `eGlu` is partitioned into per-team blocks (by `dXSB`), each a contiguous
//!   record run, and the blocks tile the table with no overlap or gap (Spike S2,
//!   `docs/audits/towx-blocks.md`). [`Severity::Error`]; the invariant the occupant-relocation
//!   move/trade ops and the block-aware create/delete preserve by construction, and that the old
//!   `dXSB`-retag `move_player` (and swap-pop delete / append-at-end create) violated. Composed into
//!   both audit paths now that the structural ops are block-aware
//!   (`docs/audits/nhlviewng-record-model.md`).
//! * [`dup_on_team`] — within one team, no player occupies two slots and no two occupied slots share a
//!   non-sentinel jersey (`0` = unassigned, exempt). [`Severity::Error`]; promotes the op-time
//!   uniqueness `set_jersey_number` enforces to a whole-DB invariant and adds the same-player guard.
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
///
/// `message` is the precise, technical description (studio ids, numeric ids) kept for logs/telemetry
/// and power-user disclosure. `team_id` / `player_id` are the **structured** entity references the
/// finding is about, so a presentation layer can resolve them to real names without parsing prose
/// (the human-readable-surfaces rule; see `studio-domain`'s audit humanizer).
#[derive(Clone, Debug)]
pub struct Finding {
    pub severity: Severity,
    /// The predicate that produced it (stable identifier for grouping/telemetry).
    pub predicate: &'static str,
    pub message: String,
    pub table: String,
    pub field: String,
    pub record: Option<u32>,
    /// The team this finding concerns, if any (for name resolution).
    pub team_id: Option<u64>,
    /// The player this finding concerns, if any (for name resolution).
    pub player_id: Option<u64>,
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

/// Documented **secondary null sentinels** — per-field "none/unassigned" placeholders that are *not*
/// the generic `0` / all-ones value, so the generic filter misses them. A source value matching one of
/// these is a placeholder, not a dangling foreign key, and is excluded from referential integrity just
/// like `0`/all-ones (rather than surfaced as an alarming "broken reference").
///
/// Verified entries:
/// * `ZBac.ykFq = 16382` — the player-index-to-bio table's secondary "unassigned" code (its field is
///   14-bit, so all-ones is `16383`; `16382` is a distinct documented null). See the anchor audit.
fn is_known_null_sentinel(studio_table: &str, studio_field: &str, value: u64) -> bool {
    matches!((studio_table, studio_field, value), ("ZBac", "ykFq", 16382))
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
        if v == 0 || v == sentinel || is_known_null_sentinel(&b.source_table, &b.source_field, v) {
            continue; // documented null / "none" sentinels (generic 0/all-ones + per-field secondaries)
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
    // The player-index table (`ZBac`) carries a *free list*: pre-allocated index rows the roster does
    // not reference (NHL07 ships ~200 such rows, keyed to player ids below the roster's id base). Their
    // `ykFq` points at no bio row, but they are unallocated capacity, not dangling references — the game
    // never loads them. Recognize them by roster reachability (an index row whose `vfEq` no roster slot
    // points at) and exclude them from this edge, the same way the entity model excludes orphan bio
    // stubs. Built once; empty if the roster can't assemble (then nothing is excluded).
    let index_tbl = db.table_canonical(tdb_relations::entity::ids::INDEX);
    let index_key_field = index_tbl
        .and_then(|t| field_by_studio(db.endian, t, tdb_relations::entity::ids::INDEX_KEY));
    let referenced_index: HashSet<u64> = RosterModel::build(db)
        .map(|m| m.slots.iter().filter_map(|s| s.index_key).collect())
        .unwrap_or_default();

    let mut findings = Vec::new();
    for rel in relations.relationships() {
        for b in &rel.bindings {
            let Some(audit) = audit_edge(db, b) else {
                continue;
            };
            let is_index_edge = b.source_table == tdb_relations::entity::ids::INDEX;
            for (record, value) in audit.dangling {
                // Skip free-list index rows: a `ZBac` source row whose `vfEq` no roster slot references.
                if is_index_edge {
                    if let (Some(t), Some(kf)) = (index_tbl, index_key_field) {
                        if let Some(vfeq) = db.read_u64(t, record, kf) {
                            if !referenced_index.contains(&vfeq) {
                                continue;
                            }
                        }
                    }
                }
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
                    team_id: None,
                    player_id: None,
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
                        team_id: None,
                        player_id: None,
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
/// operation-time and save-time share one definition of "a Player is the whole cluster". The
/// equipment companion is required only when its table exists in this db (gen6 titles like NHL07
/// ship none), so this raises no findings on a schema that simply lacks equipment tables. Returns no
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
            team_id: None,
            player_id: Some(p.player_id),
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
                    team_id: Some(team_id),
                    player_id: None,
                });
            }
        }
    }
    findings
}

/// `eGlu` roster-**block** coherence (Spike S2, `docs/audits/towx-blocks.md`). `eGlu` is partitioned
/// into per-team blocks (keyed by `dXSB`); each block is a **contiguous record run** and the blocks
/// **tile the table with no overlap or gap**. A violation is a [`Severity::Error`] — it is exactly the
/// corruption the old `dXSB`-retag `move_player` produced (a foreign team id dropped inside another
/// team's contiguous block, punching a hole in one block and a stray row into another) and the same
/// shape a swap-pop delete or append-at-end create would produce. Shared with the occupant-relocation
/// move/trade ops and the block-aware create/delete, whose contract is to preserve the tiling by
/// construction, so the same invariant is audited at operation- and save-time. Returns no findings if
/// `eGlu` is absent from this db variant.
///
/// **Not enforced: `toWX`-key contiguity.** Vanilla blocks *also* have contiguous per-team `toWX` key
/// runs, but that is a creation-order artifact, not a maintainable invariant: `toWX` is a globally
/// monotonic unique key with inter-block gaps, so a mid-block delete legitimately leaves a key hole and
/// a freshly-minted slot key is not block-adjacent. Enforcing it would cry wolf on every ordinary edit
/// (and force a risky global key renumber), so this predicate governs only the **record-block**
/// structure the game actually reads a roster by. (Verified against the anchor: `toWX` spans
/// `0..=8861` over 5532 records — globally monotonic, with gaps.)
pub fn towx_coherence(db: &TdbFile) -> Vec<Finding> {
    // The per-team contiguous-block layout is a property of the `toWX`-keyed roster (NHL 08→21).
    // Pre-`toWX` schemas (NHL07) store `eGlu` *interleaved* — each team's slots scattered across the
    // whole table rather than in one contiguous run — which is a valid layout this invariant does not
    // describe. Without the `toWX` key, skip the predicate rather than report the interleave as
    // corruption. (Verified: NHL07 `exhibitionrostertable` carries no `toWX` field.)
    let has_slot_key = db
        .table_canonical(tdb_relations::entity::ids::ROSTER_LINES)
        .map(|t| {
            t.fields
                .iter()
                .any(|f| f.id.canonical(db.endian) == tdb_relations::entity::ids::SLOT_KEY)
        })
        .unwrap_or(false);
    if !has_slot_key {
        return Vec::new();
    }
    let Ok(model) = RosterModel::build(db) else {
        return Vec::new();
    };

    // Group each slot's record by its resolved team (dXSB).
    let mut by_team: HashMap<u64, Vec<u32>> = HashMap::new();
    for slot in &model.slots {
        by_team.entry(slot.team_id).or_default().push(slot.record);
    }

    let block_finding = |record: u32, team: u64, message: String| Finding {
        severity: Severity::Error,
        predicate: "towx_coherence",
        message,
        table: tdb_relations::entity::ids::ROSTER_LINES.to_string(),
        field: tdb_relations::entity::ids::SLOT_TEAM.to_string(),
        record: Some(record),
        team_id: Some(team),
        player_id: None,
    };

    let mut findings = Vec::new();
    // (team_id, min_record, max_record) per populated team — sorted later for the tiling pass.
    let mut blocks: Vec<(u64, u32, u32)> = Vec::new();

    for (&team_id, records) in &by_team {
        let mut records = records.clone();
        records.sort_unstable();
        let count = records.len();
        let (min_r, max_r) = (records[0], records[count - 1]);

        // Contiguous record run — every adjacent record differs by exactly one.
        for w in records.windows(2) {
            if w[1] != w[0] + 1 {
                findings.push(block_finding(
                    w[1],
                    team_id,
                    format!(
                        "team {team_id} eGlu block is not a contiguous record run: gap between \
                         records {} and {} (block spans {min_r}..={max_r}, {count} slots)",
                        w[0], w[1]
                    ),
                ));
            }
        }

        blocks.push((team_id, min_r, max_r));
    }

    // (c) tiling — sorted by first record, adjacent blocks must abut (no overlap, no gap).
    blocks.sort_by_key(|&(_, min_r, _)| min_r);
    for pair in blocks.windows(2) {
        let (team_a, _, max_a) = pair[0];
        let (team_b, min_b, _) = pair[1];
        if min_b <= max_a {
            findings.push(block_finding(
                min_b,
                team_b,
                format!(
                    "team {team_b} eGlu block (from record {min_b}) overlaps team {team_a} block \
                     (ending at record {max_a})"
                ),
            ));
        } else if min_b != max_a + 1 {
            findings.push(block_finding(
                max_a,
                team_a,
                format!(
                    "gap in eGlu between team {team_a} block (ending record {max_a}) and team \
                     {team_b} block (starting record {min_b})"
                ),
            ));
        }
    }

    findings
}

/// The jersey value that means **no number assigned** — exempt from within-team uniqueness. Verified
/// against the anchor, where team 107 legitimately carries three occupied `0`-jersey slots (a
/// classic/all-star roster whose numbers are unset); every other team's occupied jerseys are unique.
pub const JERSEY_UNASSIGNED: u64 = 0;

/// Dup-on-team predicate: within a single team, (1) no player occupies two roster slots, and (2) no
/// two **occupied** slots wear the same non-sentinel jersey. Each duplicate past the first is a
/// [`Severity::Error`] — an ambiguous roster state with no unique auto-repair. This promotes the
/// op-time uniqueness `set_jersey_number` already enforces to a whole-DB invariant, and adds the
/// same-player guard, so operation-time and save-time agree. Slots that resolve to no player (vacant
/// `XSWT`) are ignored; a player legitimately on *several* teams is not flagged (different `team_id`).
/// Returns no findings if `eGlu` is absent from this db variant.
pub fn dup_on_team(db: &TdbFile) -> Vec<Finding> {
    let Ok(model) = RosterModel::build(db) else {
        return Vec::new();
    };
    let mut findings = Vec::new();

    // (1) The same player must not occupy two slots on one team.
    let mut first_player_slot: HashMap<(u64, u64), u32> = HashMap::new();
    for slot in &model.slots {
        let Some(player) = slot.player_id else {
            continue;
        };
        if let Some(&first) = first_player_slot.get(&(slot.team_id, player)) {
            findings.push(Finding {
                severity: Severity::Error,
                predicate: "dup_on_team",
                message: format!(
                    "player {player} occupies two slots on team {} (records {first} and {})",
                    slot.team_id, slot.record
                ),
                table: tdb_relations::entity::ids::ROSTER_LINES.to_string(),
                field: tdb_relations::entity::ids::SLOT_TO_INDEX.to_string(),
                record: Some(slot.record),
                team_id: Some(slot.team_id),
                player_id: Some(player),
            });
        } else {
            first_player_slot.insert((slot.team_id, player), slot.record);
        }
    }

    // (2) No two occupied slots on one team wear the same jersey (the `0` sentinel is exempt).
    let mut first_jersey_slot: HashMap<(u64, u64), u32> = HashMap::new();
    for slot in &model.slots {
        if slot.index_key.is_none() {
            continue; // a vacant slot wears no number
        }
        let Some(number) = slot.jersey else {
            continue;
        };
        if number == JERSEY_UNASSIGNED {
            continue;
        }
        if let Some(&first) = first_jersey_slot.get(&(slot.team_id, number)) {
            findings.push(Finding {
                severity: Severity::Error,
                predicate: "dup_on_team",
                message: format!(
                    "jersey {number} worn by two players on team {} (records {first} and {})",
                    slot.team_id, slot.record
                ),
                table: tdb_relations::entity::ids::ROSTER_LINES.to_string(),
                field: tdb_relations::entity::ids::SLOT_JERSEY.to_string(),
                record: Some(slot.record),
                team_id: Some(slot.team_id),
                player_id: None,
            });
        } else {
            first_jersey_slot.insert((slot.team_id, number), slot.record);
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
    findings.extend(towx_coherence(db));
    findings.extend(dup_on_team(db));
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
            team_id: None,
            player_id: None,
        });
        assert!(r.is_clean(), "Info findings do not block a write");
        r.findings.push(Finding {
            severity: Severity::Error,
            predicate: "referential_integrity",
            message: "y".into(),
            table: "t".into(),
            field: "f".into(),
            record: Some(1),
            team_id: None,
            player_id: None,
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
