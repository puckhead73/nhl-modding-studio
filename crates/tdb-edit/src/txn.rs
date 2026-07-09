//! Layer 3 — transactional entity operations over a **repairing transaction** (EDIT-MODEL Layers
//! 3–4).
//!
//! An [`EditTransaction`] owns a working copy of the database bytes and exposes operations that act
//! on **typed entities** (via the Layer 2 [`RosterModel`]) rather than bare cells. Each operation
//! runs inside [`EditTransaction::with_repairing_transaction`]:
//!
//! > snapshot → stage edits (+ auto-repair) → audit shared invariants → commit, or roll back on
//! > failure.
//!
//! Because operations mutate the working bytes in place, they **compose against pending state, not
//! on-disk** — a later operation sees the effects of earlier ones, closing the cross-command holes
//! the prior studio suffered. The post-condition audit reuses the *same* predicates as the save-time
//! audit ([`tdb_domain_validation`]), so operation-time and save-time can never disagree.
//!
//! ## Scope
//!
//! Attribute ops (`set_captaincy` with prior-captain auto-demotion, `set_jersey_number`,
//! `set_roster_status`, `set_field`), the occupant-relocation `move_player`, and the structural
//! `create_player`/`delete_player` clusters. The structural ops are **block-aware**: `eGlu` is
//! partitioned into contiguous per-team blocks (`docs/audits/towx-blocks.md`), so `create_player`
//! places the new slot *inside* the target team's block (reuse a vacant slot, else
//! `tdb_core::TdbFile::insert_record` at the block boundary) and `delete_player` removes the slot with
//! an *ordered* shift (`remove_record_ordered`), never swap-pop — keeping `towx_coherence` clean by
//! construction. Non-block cluster tables (bio/index/companions) still use swap-pop, where record
//! order is irrelevant. Still to come: `trade_slots`, `free_agent`, capacity growth past the ceiling.

use crate::EditError;
use std::collections::{BTreeMap, HashSet};
use tdb_core::TdbFile;
use tdb_domain_validation::{companion_presence, dup_on_team, single_captain, towx_coherence};
use tdb_relations::entity::ids;
use tdb_relations::{RosterModel, RosterSlot};

/// Captaincy roles — the confirmed `eGlu.mCcl` enum.
pub mod captaincy {
    pub const NONE: u64 = 0;
    pub const CAPTAIN: u64 = 1;
    pub const ALTERNATE: u64 = 2;
}

/// The per-team retired-jersey-number table (`wcSL` — "stockretirednumbertable", verified against the
/// anchor metadata and bytes: 322 rows on NHL Legacy, e.g. Montreal's eleven retired numbers). Each row
/// is a retired `(team, number)` pair; there is **no** retired-number field set on `kOtt` itself.
/// Studio ids: `dXSB` = team (→ `kOtt.vfEq`, 8-bit), `JFiY` = jersey number (7-bit).
pub mod retired {
    pub const TABLE: &str = "wcSL";
    pub const TEAM: &str = "dXSB";
    pub const NUMBER: &str = "JFiY";
}

/// The future-draft-pick table (`qHav` — "exhibitiondraftpick"). One row per pick; `Uwdb` is the
/// **current owner** (stored 1-indexed via its `range_low = -1` offset; 0 = unassigned) and
/// `GFcg` the original owner — the provenance that renders "via trade from …".
pub mod draft {
    pub const TABLE: &str = "qHav";
    pub const OWNER: &str = "Uwdb";
    pub const ORIGINAL_OWNER: &str = "GFcg";
}

/// Studio ids of the four stock career-stats tables (skater/goalie × regular season/playoffs).
/// Their rows key on `wBIz` stored **1-indexed** (raw `0` = "no player"; raw `N` → bio
/// `wBIz = N - 1` — the offset storage of the field's `range_low = -1`).
pub const CAREER_TABLES: &[&str] = &["tsSy", "OAqO", "USfF", "nCDp"];

/// Career-row identity fields shared by all four tables (studio ids).
const CAREER_YEAR: &str = "qFnd"; // season-END year, raw + 1900
const CAREER_LEAGUE: &str = "xMjj"; // 1-bit league flag: 0 = NHL, 1 = AHL
const CAREER_TEAMS: &str = "lyxL"; // fixed-width inline team-code string

fn career_table_guard(studio_table: &str) -> Result<(), EditError> {
    if CAREER_TABLES.contains(&studio_table) {
        Ok(())
    } else {
        Err(EditError::UnknownTable(format!(
            "{studio_table} is not a stock career-stats table"
        )))
    }
}

/// What an operation did: how many cell edits, and a human description of each auto-repair applied.
#[derive(Clone, Debug, Default)]
pub struct OpOutcome {
    pub edits: u32,
    pub repairs: Vec<String>,
}

/// One duplicate-jersey correction applied by [`EditTransaction::autocorrect_duplicate_jerseys`]: the
/// slot that was renumbered, its occupant, and the old → new numbers (structured so a presentation
/// layer can resolve `team_id`/`player_id` to real names).
#[derive(Clone, Debug)]
pub struct JerseyReassignment {
    pub team_id: u64,
    pub record: u32,
    pub player_id: Option<u64>,
    /// The duplicate number the slot wore.
    pub old: u64,
    /// The lowest free legal number it was reassigned to.
    pub new: u64,
}

/// Where `create_player` places its new `eGlu` slot to keep the team's record block contiguous.
enum SlotPlacement {
    /// Reuse an existing vacant slot already inside the team's block (record index).
    Reuse(u32),
    /// Insert a fresh record at this `eGlu` index (the team's block-end boundary).
    Insert(u32),
}

/// A staged, in-memory edit session over one database snapshot.
pub struct EditTransaction {
    working: Vec<u8>,
    /// When `true`, the post-condition whole-DB audit is skipped so edits proceed on a database that
    /// already carries invariant violations (the user-acknowledged "override / unsupported" mode for a
    /// corrupted DB). Op-level errors still roll back; only the whole-DB gate is lifted.
    allow_pre_existing_violations: bool,
}

impl EditTransaction {
    /// Begin a transaction over a parseable database snapshot.
    pub fn begin(db_bytes: &[u8]) -> Result<Self, EditError> {
        TdbFile::parse(db_bytes).map_err(|e| EditError::Parse(e.to_string()))?;
        Ok(EditTransaction {
            working: db_bytes.to_vec(),
            allow_pre_existing_violations: false,
        })
    }

    /// Enter (or leave) "override" mode: when enabled, [`with_repairing_transaction`] does **not** roll
    /// back on whole-DB invariant violations, letting edits proceed on an already-corrupt database.
    /// Unsupported — the caller has acknowledged that editing corrupt data may cause unforeseen
    /// in-game issues. Op-level errors (bad ids, out-of-range values) still roll back as usual.
    ///
    /// [`with_repairing_transaction`]: Self::with_repairing_transaction
    pub fn set_allow_pre_existing_violations(&mut self, allow: bool) {
        self.allow_pre_existing_violations = allow;
    }

    /// The current (pending) working bytes — CRCs are *not* refreshed until [`commit`](Self::commit).
    pub fn bytes(&self) -> &[u8] {
        &self.working
    }

    /// Assemble the Layer 2 entity model over the current pending state.
    pub fn model(&self) -> Result<RosterModel, EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        RosterModel::build(&db).map_err(|e| EditError::Entity(e.to_string()))
    }

    /// Write a fixed-bit field on one record, addressed by **studio** `(table, field)` ids.
    fn set_field(
        &mut self,
        studio_table: &str,
        record: u32,
        studio_field: &str,
        value: u64,
    ) -> Result<(), EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let table = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?;
        let field = table
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == studio_field)
            .ok_or_else(|| EditError::UnknownField {
                table: studio_table.to_string(),
                field: studio_field.to_string(),
            })?;
        if !db.set_u64(&mut self.working, table, record, field, value) {
            return Err(EditError::OutOfRange {
                table: studio_table.to_string(),
                record,
            });
        }
        Ok(())
    }

    /// Write an inline-string field on one record, addressed by **studio** `(table, field)` ids.
    fn set_field_str(
        &mut self,
        studio_table: &str,
        record: u32,
        studio_field: &str,
        text: &str,
    ) -> Result<(), EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let table = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?;
        let field = table
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == studio_field)
            .ok_or_else(|| EditError::UnknownField {
                table: studio_table.to_string(),
                field: studio_field.to_string(),
            })?;
        if !db.set_inline_str(&mut self.working, table, record, field, text) {
            return Err(EditError::OutOfRange {
                table: studio_table.to_string(),
                record,
            });
        }
        Ok(())
    }

    /// Set a single fixed-bit cell (studio ids) inside a repairing transaction. `value` is the
    /// **already-validated, encoded** raw value — entry-time validation (range / updatable / enum) is
    /// the caller's job (`studio-domain` does it against the metadata). The post-condition audit still
    /// runs, so a cell edit that would break a shared invariant is rolled back.
    pub fn set_cell_int(
        &mut self,
        studio_table: &str,
        record: u32,
        studio_field: &str,
        value: u64,
    ) -> Result<OpOutcome, EditError> {
        self.with_repairing_transaction(|tx| {
            tx.set_field(studio_table, record, studio_field, value)?;
            Ok(OpOutcome {
                edits: 1,
                repairs: Vec::new(),
            })
        })
    }

    /// Set a single inline-string cell (studio ids) inside a repairing transaction. `text` must
    /// already fit the field width (the caller validates against metadata).
    pub fn set_cell_str(
        &mut self,
        studio_table: &str,
        record: u32,
        studio_field: &str,
        text: &str,
    ) -> Result<OpOutcome, EditError> {
        self.with_repairing_transaction(|tx| {
            tx.set_field_str(studio_table, record, studio_field, text)?;
            Ok(OpOutcome {
                edits: 1,
                repairs: Vec::new(),
            })
        })
    }

    /// Append a zeroed record to a studio-named table, updating the working bytes; returns its index.
    fn append(&mut self, studio_table: &str) -> Result<u32, EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let raw = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?
            .id
            .raw();
        let (new_bytes, index) = db
            .append_record(&raw)
            .map_err(|e| EditError::Alloc(e.to_string()))?;
        self.working = new_bytes;
        Ok(index)
    }

    /// Mint a fresh unique id for a key field: the **smallest free value** in `1..sentinel` not already
    /// used by any record. (`max + 1` is wrong here — these 14-bit id spaces are nearly full and their
    /// top value is the all-ones "none" sentinel, so `max + 1` would alias it; the spaces are sparse,
    /// so a low free value exists.)
    fn mint_id(&self, studio_table: &str, studio_field: &str) -> Result<u64, EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let table = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?;
        let field = table
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == studio_field)
            .ok_or_else(|| EditError::UnknownField {
                table: studio_table.to_string(),
                field: studio_field.to_string(),
            })?;
        let sentinel = if field.bit_width >= 64 {
            u64::MAX
        } else {
            (1u64 << field.bit_width) - 1
        };
        let mut used = HashSet::new();
        for i in 0..table.current_records as u32 {
            if let Some(v) = db.read_u64(table, i, field) {
                used.insert(v);
            }
        }
        (1..sentinel)
            .find(|c| !used.contains(c))
            .ok_or_else(|| EditError::Alloc(format!("no free id for {studio_table}.{studio_field}")))
    }

    /// Insert a zeroed record **at `index`** of a studio-named table (order-preserving), updating the
    /// working bytes. Use for block-structured tables (`eGlu`), where appending at the end would drop
    /// the new slot outside its team's contiguous block; the caller then populates the slot in place.
    fn insert_at(&mut self, studio_table: &str, index: u32) -> Result<(), EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let raw = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?
            .id
            .raw();
        let new_bytes = db
            .insert_record(&raw, index)
            .map_err(|e| EditError::Alloc(e.to_string()))?;
        self.working = new_bytes;
        Ok(())
    }

    /// Swap-pop every record of a studio table whose studio field value is in `wanted`. Re-scans from
    /// scratch after each removal (swap-pop relocates the tail). Cluster references are by value, so
    /// no repointing is needed. Returns how many rows were removed. Order-**not**-preserving — only for
    /// tables whose record order is irrelevant (value-keyed: bio, index, companions).
    fn remove_matching(
        &mut self,
        studio_table: &str,
        studio_field: &str,
        wanted: &HashSet<u64>,
    ) -> Result<u32, EditError> {
        self.remove_matching_impl(studio_table, studio_field, wanted, false)
    }

    /// Order-preserving variant of [`Self::remove_matching`]: closes each hole by sliding the tail up
    /// (never drops a far row into it). Required for block-structured tables (`eGlu`) so per-team
    /// blocks stay contiguous and keep tiling.
    fn remove_matching_ordered(
        &mut self,
        studio_table: &str,
        studio_field: &str,
        wanted: &HashSet<u64>,
    ) -> Result<u32, EditError> {
        self.remove_matching_impl(studio_table, studio_field, wanted, true)
    }

    fn remove_matching_impl(
        &mut self,
        studio_table: &str,
        studio_field: &str,
        wanted: &HashSet<u64>,
        ordered: bool,
    ) -> Result<u32, EditError> {
        let mut removed = 0;
        loop {
            let view = self.working.clone();
            let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
            let table = db
                .table_canonical(studio_table)
                .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?;
            let field = table
                .fields
                .iter()
                .find(|f| f.id.canonical(db.endian) == studio_field)
                .ok_or_else(|| EditError::UnknownField {
                    table: studio_table.to_string(),
                    field: studio_field.to_string(),
                })?;
            let hit = (0..table.current_records as u32)
                .find(|&i| db.read_u64(table, i, field).is_some_and(|v| wanted.contains(&v)));
            let Some(index) = hit else { break };
            let raw = table.id.raw();
            let new_bytes = if ordered {
                db.remove_record_ordered(&raw, index)
                    .map_err(|e| EditError::Alloc(e.to_string()))?
            } else {
                db.remove_record(&raw, index)
                    .map_err(|e| EditError::Alloc(e.to_string()))?
                    .0
            };
            self.working = new_bytes;
            removed += 1;
        }
        Ok(removed)
    }

    /// Run `op` inside a repairing transaction: snapshot, stage, then audit the shared invariants;
    /// roll the working bytes back to the snapshot if the op errors or the audit finds violations.
    pub fn with_repairing_transaction<F>(&mut self, op: F) -> Result<OpOutcome, EditError>
    where
        F: FnOnce(&mut Self) -> Result<OpOutcome, EditError>,
    {
        let snapshot = self.working.clone();
        let result = op(self).and_then(|outcome| {
            // Override mode: the operator has acknowledged working on a corrupt DB, so skip the
            // whole-DB gate. The op's own errors still propagate (and roll back) below.
            if self.allow_pre_existing_violations {
                return Ok(outcome);
            }
            let view = self.working.clone();
            let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
            // Share the save-time predicates so operation-time and save-time agree. (Predicates that
            // need the def registries — referential/enum — are verified at save time; the db-only
            // invariants run here.)
            // Run each db-only predicate in turn; the first to find a violation names the failure.
            let mut violations = single_captain(&db);
            let mut predicate = "single_captain";
            if violations.is_empty() {
                violations = companion_presence(&db);
                predicate = "companion_presence";
            }
            if violations.is_empty() {
                violations = towx_coherence(&db);
                predicate = "towx_coherence";
            }
            if violations.is_empty() {
                violations = dup_on_team(&db);
                predicate = "dup_on_team";
            }
            if violations.is_empty() {
                Ok(outcome)
            } else {
                Err(EditError::AuditFailed {
                    predicate,
                    violations: violations.len(),
                })
            }
        });
        if result.is_err() {
            self.working = snapshot;
        }
        result
    }

    /// Set a player's captaincy on a team, maintaining the **single-captain** invariant by
    /// construction: promoting a Captain auto-demotes any prior Captain on that team (a repair, not a
    /// refusal). `role` must be one of [`captaincy`].
    pub fn set_captaincy(
        &mut self,
        team_id: u64,
        player_id: u64,
        role: u64,
    ) -> Result<OpOutcome, EditError> {
        if !matches!(role, captaincy::NONE | captaincy::CAPTAIN | captaincy::ALTERNATE) {
            return Err(EditError::InvalidValue {
                what: "captaincy role",
                value: role as i64,
            });
        }
        self.with_repairing_transaction(|tx| {
            let model = tx.model()?;
            let target = model
                .slots
                .iter()
                .find(|s| s.team_id == team_id && s.player_id == Some(player_id))
                .ok_or(EditError::NoSuchSlot { team_id, player_id })?;
            let target_record = target.record;

            let mut repairs = Vec::new();
            if role == captaincy::CAPTAIN {
                // Auto-repair: demote any existing captain(s) on this team.
                for prior in model.slots.iter().filter(|s| {
                    s.team_id == team_id
                        && s.captaincy == Some(captaincy::CAPTAIN)
                        && s.record != target_record
                }) {
                    tx.set_field(ids::ROSTER_LINES, prior.record, ids::SLOT_CAPTAINCY, captaincy::NONE)?;
                    repairs.push(format!(
                        "demoted prior captain (player {:?}) on team {team_id}",
                        prior.player_id
                    ));
                }
            }
            tx.set_field(ids::ROSTER_LINES, target_record, ids::SLOT_CAPTAINCY, role)?;
            Ok(OpOutcome {
                edits: 1 + repairs.len() as u32,
                repairs,
            })
        })
    }

    /// Where a new `eGlu` slot for a team should go, keeping the team's record block contiguous
    /// (`towx_coherence`). Computed from the current model *before* the create transaction stages any
    /// rows (bio/index/companion appends are in other tables and don't move `eGlu`).
    fn eglu_placement(&self, team_id: u64) -> Result<SlotPlacement, EditError> {
        let model = self.model()?;
        let team_slots: Vec<_> = model.slots.iter().filter(|s| s.team_id == team_id).collect();
        // Prefer reusing a vacant slot already inside the team's block (an unoccupied `XSWT`): no
        // allocation, no shift, and the slot keeps its in-block `toWX` key.
        if let Some(vacant) = team_slots.iter().find(|s| s.index_key.is_none()) {
            return Ok(SlotPlacement::Reuse(vacant.record));
        }
        // Otherwise insert at the block's end boundary so the new record extends this team's block
        // (later blocks shift down by one but stay contiguous and keep tiling). A team with no slots
        // yet starts a fresh one-slot block at the table end (`insert_record` treats index == current
        // as an append).
        let at = match team_slots.iter().map(|s| s.record).max() {
            Some(max_rec) => max_rec + 1,
            None => {
                let view = self.working.clone();
                let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
                db.table_canonical(ids::ROSTER_LINES)
                    .map(|t| t.current_records as u32)
                    .ok_or_else(|| EditError::UnknownTable(ids::ROSTER_LINES.to_string()))?
            }
        };
        Ok(SlotPlacement::Insert(at))
    }

    /// Create a new player as a complete, audit-clean **cluster** wired onto a team: the `ubPc` bio
    /// (fresh unique `wBIz`, `vjla` position, `dXSB` team), the kind-correct companion pair
    /// (`dSvy`+`xmja` for skaters, `mHuy`+`fMVl` for goalies — all keyed on the new `wBIz`), a `ZBac`
    /// index row (`ykFq` → bio, fresh `vfEq`), and an `eGlu` roster slot (`XSWT` → the index, `dXSB`
    /// team). The slot is placed **inside the team's contiguous `eGlu` block** — reusing a vacant
    /// in-block slot, else inserting at the block boundary (never appended at the table end, which
    /// would break `towx_coherence`). All rows are staged atomically inside a repairing transaction;
    /// the post-condition audit guarantees the companion + block invariants hold. Returns the new
    /// `wBIz`.
    pub fn create_player(
        &mut self,
        team_id: u64,
        position: u64,
    ) -> Result<(u64, OpOutcome), EditError> {
        if position > 4 {
            return Err(EditError::InvalidValue {
                what: "position",
                value: position as i64,
            });
        }
        if self.model()?.team(team_id).is_none() {
            return Err(EditError::InvalidValue {
                what: "team_id (no such team)",
                value: team_id as i64,
            });
        }
        let new_pid = self.mint_id(ids::BIO, ids::PLAYER_ID)?;
        let new_vfeq = self.mint_id(ids::INDEX, ids::INDEX_KEY)?;
        let new_slot_key = self.mint_id(ids::ROSTER_LINES, ids::SLOT_KEY)?;
        let goalie = position == 4;
        let placement = self.eglu_placement(team_id)?;

        let outcome = self.with_repairing_transaction(|tx| {
            // Bio (ubPc): identity + position + team affiliation.
            let bio = tx.append(ids::BIO)?;
            tx.set_field(ids::BIO, bio, ids::PLAYER_ID, new_pid)?;
            tx.set_field(ids::BIO, bio, ids::POSITION, position)?;
            tx.set_field(ids::BIO, bio, ids::SLOT_TEAM, team_id)?; // ubPc.dXSB

            // Kind-correct companion pair, keyed on the new wBIz.
            let (ai, equip) = if goalie {
                (ids::GOALIE_AI, ids::GOALIE_EQUIP)
            } else {
                (ids::SKATER_AI, ids::SKATER_EQUIP)
            };
            let ai_rec = tx.append(ai)?;
            tx.set_field(ai, ai_rec, ids::PLAYER_ID, new_pid)?;
            let eq_rec = tx.append(equip)?;
            tx.set_field(equip, eq_rec, ids::PLAYER_ID, new_pid)?;

            // Player index (ZBac): bridge the bio.
            let zb = tx.append(ids::INDEX)?;
            tx.set_field(ids::INDEX, zb, ids::INDEX_KEY, new_vfeq)?;
            tx.set_field(ids::INDEX, zb, ids::INDEX_TO_BIO, new_pid)?;

            // Roster slot (eGlu): install the occupant *inside the team's block* (block-coherent).
            let (_slot, slot_edits) = match placement {
                // Reuse a vacant in-block slot: it already carries this team's `dXSB` and an in-block
                // `toWX`, so only the (clean) occupant is installed — no key mint, no shift.
                SlotPlacement::Reuse(rec) => {
                    let n = tx.occupy_slot(rec, new_vfeq)?;
                    (rec, n)
                }
                // Grow the block: insert a fresh record at the boundary, then wire key + occupant + team.
                SlotPlacement::Insert(at) => {
                    tx.insert_at(ids::ROSTER_LINES, at)?;
                    tx.set_field(ids::ROSTER_LINES, at, ids::SLOT_KEY, new_slot_key)?;
                    tx.set_field(ids::ROSTER_LINES, at, ids::SLOT_TO_INDEX, new_vfeq)?;
                    tx.set_field(ids::ROSTER_LINES, at, ids::SLOT_TEAM, team_id)?;
                    (at, 3)
                }
            };

            Ok(OpOutcome {
                edits: 7 + slot_edits,
                repairs: vec![format!(
                    "created {} cluster wBIz={new_pid}: bio + {ai}/{equip} + index + slot on team {team_id}",
                    if goalie { "goalie" } else { "skater" }
                )],
            })
        })?;
        Ok((new_pid, outcome))
    }

    /// Clone an existing player onto `to_team` — a full duplicate cluster with a fresh identity. Backs
    /// the Transactions "copy". Like [`Self::create_player`], but instead of default fields it
    /// **deep-copies** the source's `ubPc` bio and kind-correct companion pair (all ratings, name,
    /// appearance, equipment) into the new records, then overrides just the identity: a freshly minted
    /// `wBIz` (re-keying bio + companions), a fresh `ZBac` index (`vfEq` → new `wBIz`), the bio's team
    /// set to `to_team`, and a block-aware `eGlu` slot wired to the new index. The source may be
    /// rostered or a free agent; its slot state (jersey/captaincy) is **not** copied — the clone gets a
    /// clean slot. Returns the new `wBIz`.
    ///
    /// Structural op (allocates a cluster) → the workspace uses the dual-create + `fully_aligned` guard.
    pub fn clone_player(
        &mut self,
        source_player: u64,
        to_team: u64,
    ) -> Result<(u64, OpOutcome), EditError> {
        // Resolve the source's cluster record indices (stable across the appends below) up front.
        let (src_bio, src_ai, src_equip, goalie) = {
            let model = self.model()?;
            if model.team(to_team).is_none() {
                return Err(EditError::InvalidValue {
                    what: "to_team (no such team)",
                    value: to_team as i64,
                });
            }
            let src = model
                .player(source_player)
                .ok_or(EditError::NoSuchPlayer(source_player))?;
            let (ai, equip, goalie) = if let (Some(ai), Some(eq)) = (src.goalie_ai, src.goalie_equip)
            {
                (ai, eq, true)
            } else if let (Some(ai), Some(eq)) = (src.skater_ai, src.skater_equip) {
                (ai, eq, false)
            } else {
                return Err(EditError::Entity(format!(
                    "source player {source_player} is missing a companion pair to clone"
                )));
            };
            (src.bio_record, ai, equip, goalie)
        };
        let new_pid = self.mint_id(ids::BIO, ids::PLAYER_ID)?;
        let new_vfeq = self.mint_id(ids::INDEX, ids::INDEX_KEY)?;
        let new_slot_key = self.mint_id(ids::ROSTER_LINES, ids::SLOT_KEY)?;
        let placement = self.eglu_placement(to_team)?;

        let outcome = self.with_repairing_transaction(|tx| {
            // Bio: deep-copy the source, then override identity (fresh wBIz) + team.
            let bio = tx.append(ids::BIO)?;
            tx.copy_record(ids::BIO, src_bio, bio)?;
            tx.set_field(ids::BIO, bio, ids::PLAYER_ID, new_pid)?;
            tx.set_field(ids::BIO, bio, ids::SLOT_TEAM, to_team)?; // ubPc.dXSB → to_team

            // Kind-correct companion pair: deep-copy each, re-keyed to the new wBIz.
            let (ai, equip) = if goalie {
                (ids::GOALIE_AI, ids::GOALIE_EQUIP)
            } else {
                (ids::SKATER_AI, ids::SKATER_EQUIP)
            };
            let ai_rec = tx.append(ai)?;
            tx.copy_record(ai, src_ai, ai_rec)?;
            tx.set_field(ai, ai_rec, ids::PLAYER_ID, new_pid)?;
            let eq_rec = tx.append(equip)?;
            tx.copy_record(equip, src_equip, eq_rec)?;
            tx.set_field(equip, eq_rec, ids::PLAYER_ID, new_pid)?;

            // Player index (ZBac): a fresh bridge to the new bio (not copied).
            let zb = tx.append(ids::INDEX)?;
            tx.set_field(ids::INDEX, zb, ids::INDEX_KEY, new_vfeq)?;
            tx.set_field(ids::INDEX, zb, ids::INDEX_TO_BIO, new_pid)?;

            // Roster slot (eGlu): block-aware placement, clean occupant (as create_player).
            let slot_edits = match placement {
                SlotPlacement::Reuse(rec) => tx.occupy_slot(rec, new_vfeq)?,
                SlotPlacement::Insert(at) => {
                    tx.insert_at(ids::ROSTER_LINES, at)?;
                    tx.set_field(ids::ROSTER_LINES, at, ids::SLOT_KEY, new_slot_key)?;
                    tx.set_field(ids::ROSTER_LINES, at, ids::SLOT_TO_INDEX, new_vfeq)?;
                    tx.set_field(ids::ROSTER_LINES, at, ids::SLOT_TEAM, to_team)?;
                    3
                }
            };

            Ok(OpOutcome {
                // bio(copy+wBIz+team) + ai(copy+wBIz) + equip(copy+wBIz) + index(key+bio) = 9, + slot.
                edits: 9 + slot_edits,
                repairs: vec![format!(
                    "cloned player {source_player} → new {} wBIz={new_pid} on team {to_team}",
                    if goalie { "goalie" } else { "skater" }
                )],
            })
        })?;
        Ok((new_pid, outcome))
    }

    /// Delete a player and its **whole cluster** without orphans: the `eGlu` slots that bridge to it,
    /// its `ZBac` index rows, every companion row (any of `dSvy`/`xmja`/`mHuy`/`fMVl`) keyed on its
    /// `wBIz`, and the `ubPc` bio. Removal is swap-pop; cluster references are by value (`wBIz`/`vfEq`)
    /// so the relocated tail rows need no repointing. Atomic + audited.
    pub fn delete_player(&mut self, player_id: u64) -> Result<OpOutcome, EditError> {
        // Resolve the player and the set of ZBac index keys (vfEq) it owns, before any removal.
        let model = self.model()?;
        if model.player(player_id).is_none() {
            return Err(EditError::NoSuchPlayer(player_id));
        }
        let index_keys: HashSet<u64> = self.player_index_keys(player_id)?;
        let id_set = HashSet::from([player_id]);

        self.with_repairing_transaction(|tx| {
            let mut edits = 0;
            // 1. Roster slots that bridge to this player's index rows. `eGlu` is block-structured, so
            //    this MUST be an ordered remove (slide later slots up, keeping each team's block
            //    contiguous and the table tiling) — swap-pop would drop a foreign team's tail slot into
            //    the hole and violate `towx_coherence` (docs/audits/towx-blocks.md).
            edits += tx.remove_matching_ordered(ids::ROSTER_LINES, ids::SLOT_TO_INDEX, &index_keys)?;
            // 2. The player-index rows themselves.
            edits += tx.remove_matching(ids::INDEX, ids::INDEX_KEY, &index_keys)?;
            // 3. Every companion row sharing the wBIz (kind-agnostic, robust to stray rows).
            for companion in [
                ids::SKATER_AI,
                ids::SKATER_EQUIP,
                ids::GOALIE_AI,
                ids::GOALIE_EQUIP,
            ] {
                edits += tx.remove_matching(companion, ids::PLAYER_ID, &id_set)?;
            }
            // 4. The bio last.
            edits += tx.remove_matching(ids::BIO, ids::PLAYER_ID, &id_set)?;

            Ok(OpOutcome {
                edits,
                repairs: vec![format!("deleted full cluster for wBIz={player_id}")],
            })
        })
    }

    // -----------------------------------------------------------------------
    // Stock career-stats rows (the four per-player stat tables)
    // -----------------------------------------------------------------------

    /// Add one stock career season row for a player. `studio_table` must be one of
    /// [`CAREER_TABLES`]; the appended record starts zeroed (stat cells are edited afterwards
    /// through the normal validated field path) with its identity set: the 1-indexed `wBIz` join
    /// key (raw `0` is the "no player" sentinel — the tables store `bio wBIz + 1`), the season-end
    /// year (`raw + 1900`), the 1-bit league flag, and the inline team-code string. Returns the new
    /// record index. Audited + undoable.
    pub fn add_career_season(
        &mut self,
        studio_table: &str,
        player_id: u64,
        year_end: u64,
        league: u64,
        team: &str,
    ) -> Result<(u32, OpOutcome), EditError> {
        career_table_guard(studio_table)?;
        if self.model()?.player(player_id).is_none() {
            return Err(EditError::NoSuchPlayer(player_id));
        }
        if !(1901..=2155).contains(&year_end) {
            return Err(EditError::InvalidValue {
                what: "season-end year (1901..=2155)",
                value: year_end as i64,
            });
        }
        if league > 1 {
            return Err(EditError::InvalidValue {
                what: "league (0 = NHL, 1 = AHL)",
                value: league as i64,
            });
        }
        // The team-code slot is an inline string of fixed byte width — reject an over-long code
        // up front rather than let the writer truncate it silently.
        let width = self.field_byte_width(studio_table, CAREER_TEAMS)?;
        if team.len() > width {
            return Err(EditError::InvalidValue {
                what: "team code length (exceeds the field's byte width)",
                value: team.len() as i64,
            });
        }

        let mut record = 0;
        let outcome = self.with_repairing_transaction(|tx| {
            let rec = tx.append(studio_table)?;
            tx.set_field(studio_table, rec, ids::PLAYER_ID, player_id + 1)?;
            tx.set_field(studio_table, rec, CAREER_YEAR, year_end - 1900)?;
            tx.set_field(studio_table, rec, CAREER_LEAGUE, league)?;
            tx.set_field_str(studio_table, rec, CAREER_TEAMS, team)?;
            record = rec;
            Ok(OpOutcome {
                edits: 4,
                repairs: vec![format!(
                    "added {studio_table} season {year_end} for wBIz={player_id}"
                )],
            })
        })?;
        Ok((record, outcome))
    }

    /// Delete one stock career row by record index — an **ordered** remove (later rows slide up),
    /// matching the reference tools' shift semantics so stored row order is preserved. Audited +
    /// undoable.
    pub fn delete_career_row(
        &mut self,
        studio_table: &str,
        record: u32,
    ) -> Result<OpOutcome, EditError> {
        career_table_guard(studio_table)?;
        self.with_repairing_transaction(|tx| {
            tx.remove_at_ordered(studio_table, record)?;
            Ok(OpOutcome {
                edits: 1,
                repairs: vec![format!(
                    "removed {studio_table} career row {record} (ordered shift)"
                )],
            })
        })
    }

    /// Remove one record by index with the order-preserving shift.
    fn remove_at_ordered(&mut self, studio_table: &str, index: u32) -> Result<(), EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let raw = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?
            .id
            .raw();
        let new_bytes = db
            .remove_record_ordered(&raw, index)
            .map_err(|e| EditError::Alloc(e.to_string()))?;
        self.working = new_bytes;
        Ok(())
    }

    /// The byte width of a fixed-width inline-string field (bit width / 8).
    fn field_byte_width(&self, studio_table: &str, studio_field: &str) -> Result<usize, EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let table = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?;
        let field = table
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == studio_field)
            .ok_or_else(|| EditError::UnknownField {
                table: studio_table.to_string(),
                field: studio_field.to_string(),
            })?;
        Ok((field.bit_width / 8) as usize)
    }

    /// The set of `ZBac.vfEq` index keys whose `ykFq` points at `player_id`.
    fn player_index_keys(&self, player_id: u64) -> Result<HashSet<u64>, EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let t = db
            .table_canonical(ids::INDEX)
            .ok_or_else(|| EditError::UnknownTable(ids::INDEX.to_string()))?;
        let find = |name: &str| {
            t.fields
                .iter()
                .find(|f| f.id.canonical(db.endian) == name)
                .cloned()
        };
        let (key, to_bio) = (
            find(ids::INDEX_KEY).ok_or_else(|| EditError::UnknownField {
                table: ids::INDEX.to_string(),
                field: ids::INDEX_KEY.to_string(),
            })?,
            find(ids::INDEX_TO_BIO).ok_or_else(|| EditError::UnknownField {
                table: ids::INDEX.to_string(),
                field: ids::INDEX_TO_BIO.to_string(),
            })?,
        );
        let mut keys = HashSet::new();
        for i in 0..t.current_records as u32 {
            if db.read_u64(t, i, &to_bio) == Some(player_id) {
                if let Some(v) = db.read_u64(t, i, &key) {
                    keys.insert(v);
                }
            }
        }
        Ok(keys)
    }

    /// The index of the first record whose studio field equals `value`, if any.
    fn find_record(
        &self,
        studio_table: &str,
        studio_field: &str,
        value: u64,
    ) -> Result<Option<u32>, EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let table = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?;
        let field = table
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == studio_field)
            .ok_or_else(|| EditError::UnknownField {
                table: studio_table.to_string(),
                field: studio_field.to_string(),
            })?;
        Ok((0..table.current_records as u32)
            .find(|&i| db.read_u64(table, i, field) == Some(value)))
    }

    /// The storage bit-width of a studio field.
    fn field_width(&self, studio_table: &str, studio_field: &str) -> Result<u32, EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let table = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?;
        table
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == studio_field)
            .map(|f| f.bit_width)
            .ok_or_else(|| EditError::UnknownField {
                table: studio_table.to_string(),
                field: studio_field.to_string(),
            })
    }

    /// Read a fixed-bit field on one record, addressed by **studio** `(table, field)` ids.
    fn read_field(
        &self,
        studio_table: &str,
        record: u32,
        studio_field: &str,
    ) -> Result<Option<u64>, EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let table = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?;
        let field = table
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == studio_field)
            .ok_or_else(|| EditError::UnknownField {
                table: studio_table.to_string(),
                field: studio_field.to_string(),
            })?;
        Ok(db.read_u64(table, record, field))
    }

    /// The `wcSL` record retiring `number` for `team`, if one exists (matched on **both** fields).
    fn retired_number_record(&self, team: u64, number: u64) -> Result<Option<u32>, EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let t = db
            .table_canonical(retired::TABLE)
            .ok_or_else(|| EditError::UnknownTable(retired::TABLE.to_string()))?;
        let field = |name: &str| {
            t.fields
                .iter()
                .find(|f| f.id.canonical(db.endian) == name)
                .ok_or_else(|| EditError::UnknownField {
                    table: retired::TABLE.to_string(),
                    field: name.to_string(),
                })
        };
        let (team_f, num_f) = (field(retired::TEAM)?, field(retired::NUMBER)?);
        for i in 0..t.current_records as u32 {
            if db.read_u64(t, i, team_f) == Some(team) && db.read_u64(t, i, num_f) == Some(number) {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }

    /// Remove a record by index via swap-pop, addressed by studio table id. For tables whose record
    /// order is irrelevant and that are referenced by value, not index (e.g. `wcSL`).
    fn remove_at_swap(&mut self, studio_table: &str, index: u32) -> Result<(), EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let raw = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?
            .id
            .raw();
        let (new_bytes, _moved) = db
            .remove_record(&raw, index)
            .map_err(|e| EditError::Alloc(e.to_string()))?;
        self.working = new_bytes;
        Ok(())
    }

    /// Capture a slot's **occupant** — every `eGlu` field except the slot-identity fields `toWX` (key)
    /// and `dXSB` (team) — as `(studio_field, value)` pairs. The read half of a relocation: the
    /// occupant is the `XSWT` index pointer, jersey, style, captaincy, status, and the line-assignment
    /// flags; the slot's own team/key are left out so block layout is preserved when it's re-applied.
    fn capture_occupant(&self, record: u32) -> Result<Vec<(String, u64)>, EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let table = db
            .table_canonical(ids::ROSTER_LINES)
            .ok_or_else(|| EditError::UnknownTable(ids::ROSTER_LINES.to_string()))?;
        let mut out = Vec::new();
        for f in &table.fields {
            let sid = f.id.canonical(db.endian);
            if sid == ids::SLOT_KEY || sid == ids::SLOT_TEAM {
                continue; // preserve the target slot's own identity (team, key)
            }
            if let Some(v) = db.read_u64(table, record, f) {
                out.push((sid, v));
            }
        }
        Ok(out)
    }

    /// Write a captured occupant (see [`Self::capture_occupant`]) onto a slot. Returns the field count.
    fn apply_occupant(&mut self, record: u32, fields: &[(String, u64)]) -> Result<u32, EditError> {
        for (sid, v) in fields {
            self.set_field(ids::ROSTER_LINES, record, sid, *v)?;
        }
        Ok(fields.len() as u32)
    }

    /// Relocate a slot's occupant from `src` to `dst` without touching either slot's team/key (so the
    /// per-team contiguous-block layout is preserved by construction). Returns the field count.
    fn transplant_slot(&mut self, src: u32, dst: u32) -> Result<u32, EditError> {
        let fields = self.capture_occupant(src)?;
        self.apply_occupant(dst, &fields)
    }

    /// Mark a slot **empty**: zero its `XSWT` (the game's own empty-slot convention, so no `ZBac` index
    /// row is referenced) and clear any captaincy so the vacated slot can't count as a phantom captain.
    /// The slot keeps its `dXSB`/`toWX` (it stays in its team's block, now unoccupied) and its other
    /// per-slot fields are left as-is — matching the stale-but-harmless state EA's own empty slots carry
    /// (the occupied-only predicates ignore them). Returns the field count. Used by release-to-FA and
    /// the source side of a move.
    fn vacate_slot(&mut self, record: u32) -> Result<u32, EditError> {
        self.set_field(ids::ROSTER_LINES, record, ids::SLOT_TO_INDEX, 0)?;
        self.set_field(ids::ROSTER_LINES, record, ids::SLOT_CAPTAINCY, captaincy::NONE)?;
        Ok(2)
    }

    /// Install a player index key into a slot as a **clean occupant**: set `XSWT`, clear captaincy, and
    /// unassign the jersey. For *reusing* a vacant in-block slot (create/clone/sign), whose stale
    /// per-slot fields must not leak onto the arriving player. Returns the field count.
    fn occupy_slot(&mut self, record: u32, vfeq: u64) -> Result<u32, EditError> {
        self.set_field(ids::ROSTER_LINES, record, ids::SLOT_TO_INDEX, vfeq)?;
        self.set_field(ids::ROSTER_LINES, record, ids::SLOT_CAPTAINCY, captaincy::NONE)?;
        self.set_field(ids::ROSTER_LINES, record, ids::SLOT_JERSEY, 0)?;
        Ok(3)
    }

    /// Deep-copy the whole record block from `src` to `dst` of a studio-named table (every field at
    /// once — fixed-bit, inline strings, all bytes), updating the working bytes. The caller then
    /// overrides the key fields. `src`/`dst` must be existing records (e.g. `dst` just appended).
    fn copy_record(&mut self, studio_table: &str, src: u32, dst: u32) -> Result<(), EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let table = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?;
        let mut buf = self.working.clone();
        if !db.copy_record(&mut buf, table, src, dst) {
            return Err(EditError::Alloc(format!(
                "copy_record out of range in {studio_table}: {src} -> {dst}"
            )));
        }
        db.recompute_crcs(&mut buf); // structure unchanged → the current parse's layout is valid
        self.working = buf;
        Ok(())
    }

    /// If the slot at `record` wears a **non-zero** jersey already worn by another *occupied* slot on
    /// `team`, **reassign** it to the lowest free legal number on that team and return `(old, new)` —
    /// the dup-on-team repair a relocation (move/trade) applies on arrival. Keeping the staying player's
    /// number and renumbering the newcomer is the smart default (a later phase adds an interactive
    /// prompt to choose which player is renumbered and to what). Falls back to clearing to the
    /// unassigned sentinel (`new == 0`) only if the team already wears every number (never happens with
    /// a 7-bit field and <128 slots). `0` is exempt from the within-team uniqueness invariant, so the
    /// result is always legal. (Retired numbers aren't consulted yet — a later refinement.)
    fn resolve_colliding_jersey(
        &mut self,
        record: u32,
        team: u64,
    ) -> Result<Option<(u64, u64)>, EditError> {
        let model = self.model()?;
        let current = model
            .slots
            .iter()
            .find(|s| s.record == record)
            .and_then(|m| m.jersey)
            .filter(|&n| n != 0);
        let Some(number) = current else {
            return Ok(None);
        };
        // The numbers worn by the team's *other* occupied slots — what the newcomer must avoid.
        let worn: HashSet<u64> = model
            .slots
            .iter()
            .filter(|s| s.team_id == team && s.record != record && s.index_key.is_some())
            .filter_map(|s| s.jersey)
            .collect();
        drop(model);
        if !worn.contains(&number) {
            return Ok(None); // no collision — the newcomer's number is free here
        }
        let max = (1u64 << self.field_width(ids::ROSTER_LINES, ids::SLOT_JERSEY)?) - 1;
        let new = (1..=max).find(|n| !worn.contains(n)).unwrap_or(0);
        self.set_field(ids::ROSTER_LINES, record, ids::SLOT_JERSEY, new)?;
        Ok(Some((number, new)))
    }

    /// Move a player's **roster membership** from `from_team` to `to_team`. A player can hold several
    /// memberships at once (e.g. an NHL team *and* an all-star/national team — verified: ~6 % of
    /// players occupy >1 slot), so the move is keyed by `from_team` to pick the right one.
    ///
    /// `eGlu` slots are fixed per-team containers laid out as contiguous, gap-free per-team blocks
    /// (verified — see `docs/audits/towx-blocks.md`), so a move is **not** a team retag: it relocates
    /// the player's occupancy into a free slot in the destination team's block (`XSWT` index pointer +
    /// per-slot fields) and **vacates the source slot** by zeroing its `XSWT` (the game's own
    /// "empty slot" convention). Neither slot's `dXSB`/`toWX` changes, so the block tiling is preserved.
    /// The bio's team (`ubPc.dXSB`) follows only when it matched `from_team` (the player's primary
    /// affiliation); companions key on `wBIz` and follow automatically. A moved membership **loses any
    /// captaincy** on arrival (single-captain repair); if the player's jersey is already worn on the
    /// destination team the newcomer is **reassigned the lowest free number** (dup-on-team repair).
    ///
    /// Errors with [`EditError::TeamRosterFull`] when the destination block has no free slot — growing
    /// the block (capacity normalization) is a later phase; until then use a two-player trade instead.
    pub fn move_player(
        &mut self,
        player_id: u64,
        from_team: u64,
        to_team: u64,
    ) -> Result<OpOutcome, EditError> {
        {
            let model = self.model()?;
            if model.team(to_team).is_none() {
                return Err(EditError::InvalidValue {
                    what: "to_team (no such team)",
                    value: to_team as i64,
                });
            }
            if model.player(player_id).is_none() {
                return Err(EditError::NoSuchPlayer(player_id));
            }
        }
        self.with_repairing_transaction(|tx| {
            let model = tx.model()?;
            let bio_record = model.player(player_id).unwrap().bio_record;

            // The membership being moved: the player's slot on `from_team`.
            let source = model
                .slots
                .iter()
                .find(|s| s.team_id == from_team && s.player_id == Some(player_id))
                .ok_or(EditError::NoSuchSlot {
                    team_id: from_team,
                    player_id,
                })?;
            let source_record = source.record;
            let had_captaincy = source.captaincy.is_some_and(|c| c != captaincy::NONE);

            // Already on the destination team → nothing to do (avoids consuming a free slot).
            if from_team == to_team {
                return Ok(OpOutcome {
                    edits: 0,
                    repairs: vec![format!("player {player_id} already on team {to_team}")],
                });
            }

            // A free (unoccupied) slot in the destination team's block.
            let dest_record = model
                .slots
                .iter()
                .find(|s| s.team_id == to_team && s.player_id.is_none())
                .map(|s| s.record)
                .ok_or(EditError::TeamRosterFull { team: to_team })?;
            drop(model);

            // Relocate occupancy into the free destination slot, then vacate the source slot (zero its
            // XSWT and clear captaincy so the emptied slot leaves no phantom captain behind).
            let mut edits = tx.transplant_slot(source_record, dest_record)?;
            edits += tx.vacate_slot(source_record)?;

            // Update the bio's team only if it was this (primary) membership.
            if tx.read_field(ids::BIO, bio_record, ids::SLOT_TEAM)? == Some(from_team) {
                tx.set_field(ids::BIO, bio_record, ids::SLOT_TEAM, to_team)?; // ubPc.dXSB
                edits += 1;
            }

            // Captaincy does not carry to the new team (single-captain repair).
            let mut repairs = Vec::new();
            tx.set_field(ids::ROSTER_LINES, dest_record, ids::SLOT_CAPTAINCY, captaincy::NONE)?;
            edits += 1;
            if had_captaincy {
                repairs.push(format!("cleared captaincy on move to team {to_team}"));
            }

            // The player brings their jersey; if it's already worn on the destination team, reassign
            // the newcomer the lowest free number (a dup-on-team repair, mirroring the captaincy one).
            if let Some((old, new)) = tx.resolve_colliding_jersey(dest_record, to_team)? {
                edits += 1;
                repairs.push(if new == 0 {
                    format!("cleared jersey #{old} on move to team {to_team} (already worn, no free number)")
                } else {
                    format!("reassigned jersey #{old} to #{new} on move to team {to_team} (#{old} already worn there)")
                });
            }

            Ok(OpOutcome { edits, repairs })
        })
    }

    /// Swap two players' roster memberships between teams — a straight **exchange of occupants**. This
    /// is the general relocation to reach for when neither block has a free slot (so [`Self::move_player`]
    /// would return [`EditError::TeamRosterFull`]): rather than needing a spare slot, each player takes
    /// the other's.
    ///
    /// Mechanics mirror `move_player`, doubled and without allocation: each occupant (`XSWT` + per-slot
    /// fields) is captured and written into the *other* slot, leaving both slots' `dXSB`/`toWX`
    /// untouched — so the per-team contiguous blocks are preserved by construction and no free slot is
    /// consumed. On each arrival, **captaincy is cleared** (single-captain repair) and a **jersey
    /// already worn on the new team is reassigned to the lowest free number** (dup-on-team repair); each player's bio
    /// team (`ubPc.dXSB`) follows when it matched their old team. Attribute op (no records
    /// allocated/removed) → the workspace mirrors it via `fan_out_entity`.
    ///
    /// Errors with [`EditError::NoSuchSlot`] if either player is not on the named team, or
    /// [`EditError::InvalidValue`] for an unknown team or a trade of a slot with itself.
    pub fn trade_slots(
        &mut self,
        a_team: u64,
        a_player: u64,
        b_team: u64,
        b_player: u64,
    ) -> Result<OpOutcome, EditError> {
        {
            let model = self.model()?;
            for (team, player) in [(a_team, a_player), (b_team, b_player)] {
                if model.team(team).is_none() {
                    return Err(EditError::InvalidValue {
                        what: "team (no such team)",
                        value: team as i64,
                    });
                }
                if model.player(player).is_none() {
                    return Err(EditError::NoSuchPlayer(player));
                }
            }
        }
        self.with_repairing_transaction(|tx| {
            let model = tx.model()?;
            let slot_of = |team: u64, player: u64| {
                model
                    .slots
                    .iter()
                    .find(|s| s.team_id == team && s.player_id == Some(player))
                    .map(|s| s.record)
            };
            let rec_a = slot_of(a_team, a_player).ok_or(EditError::NoSuchSlot {
                team_id: a_team,
                player_id: a_player,
            })?;
            let rec_b = slot_of(b_team, b_player).ok_or(EditError::NoSuchSlot {
                team_id: b_team,
                player_id: b_player,
            })?;
            if rec_a == rec_b {
                return Err(EditError::InvalidValue {
                    what: "trade of a slot with itself",
                    value: rec_a as i64,
                });
            }
            let a_bio = model.player(a_player).unwrap().bio_record;
            let b_bio = model.player(b_player).unwrap().bio_record;
            drop(model);

            // Exchange the two occupants (capture both before writing either).
            let a_fields = tx.capture_occupant(rec_a)?;
            let b_fields = tx.capture_occupant(rec_b)?;
            let mut edits = tx.apply_occupant(rec_a, &b_fields)?; // b_player now sits in a_team's slot
            edits += tx.apply_occupant(rec_b, &a_fields)?; // a_player now sits in b_team's slot

            let mut repairs = Vec::new();
            // Captaincy does not carry across a trade (single-captain repair, both arrivals).
            tx.set_field(ids::ROSTER_LINES, rec_a, ids::SLOT_CAPTAINCY, captaincy::NONE)?;
            tx.set_field(ids::ROSTER_LINES, rec_b, ids::SLOT_CAPTAINCY, captaincy::NONE)?;
            edits += 2;

            // Each bio's team follows only if it matched that player's (primary) old team.
            if tx.read_field(ids::BIO, a_bio, ids::SLOT_TEAM)? == Some(a_team) {
                tx.set_field(ids::BIO, a_bio, ids::SLOT_TEAM, b_team)?;
                edits += 1;
            }
            if tx.read_field(ids::BIO, b_bio, ids::SLOT_TEAM)? == Some(b_team) {
                tx.set_field(ids::BIO, b_bio, ids::SLOT_TEAM, a_team)?;
                edits += 1;
            }

            // Resolve any jersey collision each arrival creates on its new team.
            if let Some((old, new)) = tx.resolve_colliding_jersey(rec_a, a_team)? {
                edits += 1;
                repairs.push(if new == 0 {
                    format!("cleared jersey #{old} for the player arriving on team {a_team} (already worn, no free number)")
                } else {
                    format!("reassigned jersey #{old} to #{new} for the player arriving on team {a_team} (#{old} already worn there)")
                });
            }
            if let Some((old, new)) = tx.resolve_colliding_jersey(rec_b, b_team)? {
                edits += 1;
                repairs.push(if new == 0 {
                    format!("cleared jersey #{old} for the player arriving on team {b_team} (already worn, no free number)")
                } else {
                    format!("reassigned jersey #{old} to #{new} for the player arriving on team {b_team} (#{old} already worn there)")
                });
            }

            Ok(OpOutcome { edits, repairs })
        })
    }

    /// Release a player to **free agency**: vacate their slot on `from_team` so no `eGlu` slot
    /// references their `ZBac` index row — the verified definition of off-roster (Spike S1,
    /// `docs/audits/roster-pools.md`: pool membership == slot presence at the index level; there is no
    /// sentinel team). The slot stays in the team's block (now empty), so block tiling is preserved;
    /// its `XSWT` is zeroed and captaincy cleared. The player's bio + index rows are untouched — a free
    /// agent keeps their identity, just no roster slot. Attribute op → the workspace mirrors it via
    /// `fan_out_entity`.
    ///
    /// Errors with [`EditError::NoSuchSlot`] if the player is not on `from_team`.
    pub fn release_to_fa(&mut self, from_team: u64, player_id: u64) -> Result<OpOutcome, EditError> {
        {
            let model = self.model()?;
            if model.team(from_team).is_none() {
                return Err(EditError::InvalidValue {
                    what: "from_team (no such team)",
                    value: from_team as i64,
                });
            }
            if model.player(player_id).is_none() {
                return Err(EditError::NoSuchPlayer(player_id));
            }
        }
        self.with_repairing_transaction(|tx| {
            let model = tx.model()?;
            let record = model
                .slots
                .iter()
                .find(|s| s.team_id == from_team && s.player_id == Some(player_id))
                .ok_or(EditError::NoSuchSlot {
                    team_id: from_team,
                    player_id,
                })?
                .record;
            drop(model);
            let edits = tx.vacate_slot(record)?;
            Ok(OpOutcome {
                edits,
                repairs: vec![format!(
                    "released player {player_id} from team {from_team} to free agency"
                )],
            })
        })
    }

    /// Sign a **free agent** — an off-roster player (indexed but occupying no slot) — onto `to_team` by
    /// installing their `ZBac` index key (`vfEq`) into a **free slot in the team's block**. The slot's
    /// team and key are untouched, so the block stays coherent and no record is allocated; the arriving
    /// slot is given a clean state (captaincy cleared, jersey unassigned — a free agent carries no
    /// number, since jerseys live on slots) and the player's bio team follows to `to_team`. Attribute
    /// op → `fan_out_entity`.
    ///
    /// Errors with [`EditError::AlreadyRostered`] if the player already occupies a slot (use
    /// move/trade), or [`EditError::TeamRosterFull`] when the destination block has no free slot —
    /// growing the block is capacity work (Phase 4.4); until then, free a slot first (release or trade).
    pub fn sign_from_fa(&mut self, player_id: u64, to_team: u64) -> Result<OpOutcome, EditError> {
        let vfeq = {
            let model = self.model()?;
            if model.team(to_team).is_none() {
                return Err(EditError::InvalidValue {
                    what: "to_team (no such team)",
                    value: to_team as i64,
                });
            }
            if model.player(player_id).is_none() {
                return Err(EditError::NoSuchPlayer(player_id));
            }
            if model.slots.iter().any(|s| s.player_id == Some(player_id)) {
                return Err(EditError::AlreadyRostered { player: player_id });
            }
            // The player's index key (`ZBac.vfEq`); smallest if somehow several, for determinism.
            self.player_index_keys(player_id)?
                .into_iter()
                .min()
                .ok_or(EditError::NoSuchPlayer(player_id))?
        };
        self.with_repairing_transaction(|tx| {
            let model = tx.model()?;
            let bio_record = model.player(player_id).unwrap().bio_record;
            let dest_record = model
                .slots
                .iter()
                .find(|s| s.team_id == to_team && s.player_id.is_none())
                .map(|s| s.record)
                .ok_or(EditError::TeamRosterFull { team: to_team })?;
            drop(model);

            // Install the occupant into the free slot with a clean arrival state.
            let mut edits = tx.occupy_slot(dest_record, vfeq)?;
            // The bio's primary team follows to the signing team.
            if tx.read_field(ids::BIO, bio_record, ids::SLOT_TEAM)? != Some(to_team) {
                tx.set_field(ids::BIO, bio_record, ids::SLOT_TEAM, to_team)?;
                edits += 1;
            }
            Ok(OpOutcome {
                edits,
                repairs: vec![format!("signed free agent {player_id} to team {to_team}")],
            })
        })
    }

    /// Add an **additional roster membership**: install the player's `ZBac` index key into a free
    /// slot in `to_team`'s block, leaving every other membership *and* the bio's primary team
    /// (`ubPc.dXSB`) untouched. This is the additive counterpart to [`Self::sign_from_fa`] for
    /// players who are already rostered somewhere — national teams, all-star / Winter Classic
    /// rosters, alternate dynasty rosters — where the main club stays canonical. The arriving slot
    /// gets the same clean state (captaincy cleared, jersey unassigned).
    ///
    /// Errors with [`EditError::InvalidValue`] if the player already holds a slot on `to_team`
    /// (one membership per team — a duplicate is an integrity violation), or
    /// [`EditError::TeamRosterFull`] when the destination block has no free slot.
    pub fn add_membership(&mut self, player_id: u64, to_team: u64) -> Result<OpOutcome, EditError> {
        let vfeq = {
            let model = self.model()?;
            if model.team(to_team).is_none() {
                return Err(EditError::InvalidValue {
                    what: "to_team (no such team)",
                    value: to_team as i64,
                });
            }
            if model.player(player_id).is_none() {
                return Err(EditError::NoSuchPlayer(player_id));
            }
            if model
                .slots
                .iter()
                .any(|s| s.team_id == to_team && s.player_id == Some(player_id))
            {
                return Err(EditError::InvalidValue {
                    what: "to_team (player already on that team)",
                    value: to_team as i64,
                });
            }
            self.player_index_keys(player_id)?
                .into_iter()
                .min()
                .ok_or(EditError::NoSuchPlayer(player_id))?
        };
        self.with_repairing_transaction(|tx| {
            let model = tx.model()?;
            let dest_record = model
                .slots
                .iter()
                .find(|s| s.team_id == to_team && s.player_id.is_none())
                .map(|s| s.record)
                .ok_or(EditError::TeamRosterFull { team: to_team })?;
            drop(model);

            let edits = tx.occupy_slot(dest_record, vfeq)?;
            Ok(OpOutcome {
                edits,
                repairs: vec![format!(
                    "added roster membership for player {player_id} on team {to_team}"
                )],
            })
        })
    }

    /// Remove one **additional roster membership**: vacate the player's slot on `from_team` (zero
    /// its `XSWT`, clear captaincy), leaving the bio's primary team and every other membership
    /// untouched. Mechanically identical to [`Self::release_to_fa`]; the distinct op exists so the
    /// caller's intent ("drop the extra row", not "make a free agent") reads correctly in the
    /// outcome. Callers gate which memberships count as additional — the canonical Main Team row
    /// is not offered for removal here.
    ///
    /// Errors with [`EditError::NoSuchSlot`] if the player is not on `from_team`.
    pub fn remove_membership(
        &mut self,
        from_team: u64,
        player_id: u64,
    ) -> Result<OpOutcome, EditError> {
        {
            let model = self.model()?;
            if model.team(from_team).is_none() {
                return Err(EditError::InvalidValue {
                    what: "from_team (no such team)",
                    value: from_team as i64,
                });
            }
            if model.player(player_id).is_none() {
                return Err(EditError::NoSuchPlayer(player_id));
            }
        }
        self.with_repairing_transaction(|tx| {
            let model = tx.model()?;
            let record = model
                .slots
                .iter()
                .find(|s| s.team_id == from_team && s.player_id == Some(player_id))
                .ok_or(EditError::NoSuchSlot {
                    team_id: from_team,
                    player_id,
                })?
                .record;
            drop(model);
            let edits = tx.vacate_slot(record)?;
            Ok(OpOutcome {
                edits,
                repairs: vec![format!(
                    "removed roster membership for player {player_id} on team {from_team}"
                )],
            })
        })
    }

    /// Set a player's jersey number on a team (`eGlu.sVRt`). Validated against the field width and
    /// **unique within the team**: a number already worn by another player is rejected (a jersey
    /// collision has no unambiguous auto-repair, so it surfaces a structured error).
    pub fn set_jersey_number(
        &mut self,
        team_id: u64,
        player_id: u64,
        number: u64,
    ) -> Result<OpOutcome, EditError> {
        let max = (1u64 << self.field_width(ids::ROSTER_LINES, ids::SLOT_JERSEY)?) - 1;
        if number > max {
            return Err(EditError::InvalidValue {
                what: "jersey number",
                value: number as i64,
            });
        }
        self.with_repairing_transaction(|tx| {
            let model = tx.model()?;
            let target_record = model
                .slots
                .iter()
                .find(|s| s.team_id == team_id && s.player_id == Some(player_id))
                .ok_or(EditError::NoSuchSlot { team_id, player_id })?
                .record;
            // 0 = "no number" — always assignable (it's how a number is cleared), so it's exempt
            // from the uniqueness check (matching `plan_jersey_corrections`, which ignores 0s).
            if number != 0
                && model.slots.iter().any(|s| {
                    s.team_id == team_id && s.record != target_record && s.jersey == Some(number)
                })
            {
                return Err(EditError::JerseyTaken {
                    team: team_id,
                    number,
                });
            }
            tx.set_field(ids::ROSTER_LINES, target_record, ids::SLOT_JERSEY, number)?;
            Ok(OpOutcome {
                edits: 1,
                repairs: Vec::new(),
            })
        })
    }

    /// Plan the whole-DB duplicate-jersey corrections **without applying them** (read-only). Within each
    /// team, the first occupied slot (in record order) to wear a given non-zero number keeps it; every
    /// later slot wearing that same number is reassigned the **lowest free legal number** on that team —
    /// the same default logic move/trade apply on arrival. Deterministic (teams and slots processed in
    /// id/record order). Numbers assigned earlier in the pass are reserved so two corrections on one team
    /// never collide. A slot is left untouched only if the team already wears every legal number.
    pub fn plan_jersey_corrections(&self) -> Result<Vec<JerseyReassignment>, EditError> {
        let max = (1u64 << self.field_width(ids::ROSTER_LINES, ids::SLOT_JERSEY)?) - 1;
        let model = self.model()?;

        let mut by_team: BTreeMap<u64, Vec<&RosterSlot>> = BTreeMap::new();
        for s in model.slots.iter().filter(|s| s.index_key.is_some()) {
            by_team.entry(s.team_id).or_default().push(s);
        }

        let mut plan = Vec::new();
        for (team, mut slots) in by_team {
            slots.sort_by_key(|s| s.record);
            // Numbers currently in use on the team (kept ones stay reserved) plus any we newly assign.
            let mut worn: HashSet<u64> =
                slots.iter().filter_map(|s| s.jersey).filter(|&n| n != 0).collect();
            let mut kept: HashSet<u64> = HashSet::new();
            for s in slots {
                let Some(number) = s.jersey.filter(|&n| n != 0) else {
                    continue;
                };
                if kept.insert(number) {
                    continue; // first wearer keeps the number
                }
                // Duplicate: take the lowest free legal number (skip if the team wears them all).
                let Some(new) = (1..=max).find(|n| !worn.contains(n)) else {
                    continue;
                };
                worn.insert(new);
                plan.push(JerseyReassignment {
                    team_id: team,
                    record: s.record,
                    player_id: s.player_id,
                    old: number,
                    new,
                });
            }
        }
        Ok(plan)
    }

    /// Auto-correct every duplicate jersey number in the DB using [`Self::plan_jersey_corrections`]'s
    /// default logic, inside the repairing transaction (audited + rolled back on failure). Returns the
    /// op outcome and the list of corrections applied — empty (and a no-op) when there are no duplicates.
    pub fn autocorrect_duplicate_jerseys(
        &mut self,
    ) -> Result<(OpOutcome, Vec<JerseyReassignment>), EditError> {
        let plan = self.plan_jersey_corrections()?;
        if plan.is_empty() {
            return Ok((OpOutcome::default(), Vec::new()));
        }
        let applied = plan.clone();
        let outcome = self.with_repairing_transaction(move |tx| {
            let mut repairs = Vec::with_capacity(applied.len());
            for r in &applied {
                tx.set_field(ids::ROSTER_LINES, r.record, ids::SLOT_JERSEY, r.new)?;
                repairs.push(format!(
                    "reassigned duplicate jersey #{} to #{} on team {}",
                    r.old, r.new, r.team_id
                ));
            }
            Ok(OpOutcome {
                edits: applied.len() as u32,
                repairs,
            })
        })?;
        Ok((outcome, plan))
    }

    /// Retire a jersey `number` for a `team` — add the `(team, number)` pair to the retired-number
    /// table (`wcSL`; see [`retired`]). **Idempotent:** a no-op if that number is already retired for
    /// the team. Structural op (appends a `wcSL` row; the table is value-keyed, so append is safe).
    /// The number is range-checked against the field width; the team must exist.
    ///
    /// Errors with [`EditError::InvalidValue`] for an unknown team or an out-of-range number.
    pub fn retire_jersey(&mut self, team: u64, number: u64) -> Result<OpOutcome, EditError> {
        if self.model()?.team(team).is_none() {
            return Err(EditError::InvalidValue {
                what: "team (no such team)",
                value: team as i64,
            });
        }
        let max = (1u64 << self.field_width(retired::TABLE, retired::NUMBER)?) - 1;
        if number > max {
            return Err(EditError::InvalidValue {
                what: "retired jersey number",
                value: number as i64,
            });
        }
        self.with_repairing_transaction(|tx| {
            if tx.retired_number_record(team, number)?.is_some() {
                return Ok(OpOutcome {
                    edits: 0,
                    repairs: vec![format!("jersey #{number} already retired for team {team}")],
                });
            }
            let rec = tx.append(retired::TABLE)?;
            tx.set_field(retired::TABLE, rec, retired::TEAM, team)?;
            tx.set_field(retired::TABLE, rec, retired::NUMBER, number)?;
            Ok(OpOutcome {
                edits: 2,
                repairs: vec![format!("retired jersey #{number} for team {team}")],
            })
        })
    }

    /// Un-retire a jersey `number` for a `team` — remove its `(team, number)` row from `wcSL`.
    /// **Idempotent:** a no-op if the number was not retired for the team. Structural op (swap-pop; the
    /// table is value-keyed so relocating the tail row needs no repointing).
    pub fn unretire_jersey(&mut self, team: u64, number: u64) -> Result<OpOutcome, EditError> {
        self.with_repairing_transaction(|tx| match tx.retired_number_record(team, number)? {
            None => Ok(OpOutcome {
                edits: 0,
                repairs: vec![format!("jersey #{number} was not retired for team {team}")],
            }),
            Some(rec) => {
                tx.remove_at_swap(retired::TABLE, rec)?;
                Ok(OpOutcome {
                    edits: 1,
                    repairs: vec![format!("un-retired jersey #{number} for team {team}")],
                })
            }
        })
    }

    /// Trade a future draft pick to another team: rewrite the pick's current owner
    /// (`qHav.Uwdb`, stored 1-indexed). The original owner (`GFcg`) is deliberately untouched —
    /// it's the provenance that renders "via trade from …" on the Draft Picks tab. Errors on an
    /// unknown team or an out-of-range/unpopulated pick record. Audited + undoable.
    pub fn trade_draft_pick(&mut self, record: u32, to_team: u64) -> Result<OpOutcome, EditError> {
        if self.model()?.team(to_team).is_none() {
            return Err(EditError::InvalidValue {
                what: "to_team (no such team)",
                value: to_team as i64,
            });
        }
        {
            let view = self.working.clone();
            let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
            let t = db
                .table_canonical(draft::TABLE)
                .ok_or_else(|| EditError::UnknownTable(draft::TABLE.to_string()))?;
            if record >= t.current_records as u32 {
                return Err(EditError::InvalidValue {
                    what: "draft-pick record (out of range)",
                    value: record as i64,
                });
            }
        }
        self.with_repairing_transaction(|tx| {
            tx.set_field(draft::TABLE, record, draft::OWNER, to_team + 1)?;
            Ok(OpOutcome {
                edits: 1,
                repairs: vec![format!(
                    "traded draft pick (qHav row {record}) to team {to_team}"
                )],
            })
        })
    }

    /// Set a player's roster status on the bridged player-index row (`ZBac.RREB`, 2-bit). The
    /// label→value mapping (Scratched / Dressed / In-Minors) is **candidate**; this writes the raw
    /// 2-bit code (`0..=3`).
    pub fn set_roster_status(
        &mut self,
        team_id: u64,
        player_id: u64,
        status: u64,
    ) -> Result<OpOutcome, EditError> {
        if status > 3 {
            return Err(EditError::InvalidValue {
                what: "roster status (2-bit)",
                value: status as i64,
            });
        }
        self.with_repairing_transaction(|tx| {
            let model = tx.model()?;
            let index_key = model
                .slots
                .iter()
                .find(|s| s.team_id == team_id && s.player_id == Some(player_id))
                .ok_or(EditError::NoSuchSlot { team_id, player_id })?
                .index_key
                .ok_or(EditError::NoSuchSlot { team_id, player_id })?;
            drop(model);
            let zbac_record = tx
                .find_record(ids::INDEX, ids::INDEX_KEY, index_key)?
                .ok_or_else(|| EditError::Entity(format!("no ZBac row with vfEq {index_key}")))?;
            tx.set_field(ids::INDEX, zbac_record, ids::INDEX_STATUS, status)?;
            Ok(OpOutcome {
                edits: 1,
                repairs: Vec::new(),
            })
        })
    }

    /// Finalize: recompute all CRCs over the working bytes and return the new database.
    pub fn commit(mut self) -> Result<Vec<u8>, EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        db.recompute_crcs(&mut self.working);
        Ok(self.working)
    }
}
