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
//! ## Scope (this increment)
//!
//! Ships the harness plus [`EditTransaction::set_captaincy`] — the single-captain rule with
//! auto-demotion of the prior captain. Structural operations that **allocate or remove records**
//! (`create_player`, `delete_player`, `trade`) are deferred: they need record-allocation primitives
//! (table growth, the swap-pop tail-row hazard) that the engine does not yet provide. They will land
//! here as further operations on the same harness.

use crate::EditError;
use std::collections::HashSet;
use tdb_core::TdbFile;
use tdb_domain_validation::{companion_presence, single_captain};
use tdb_relations::entity::ids;
use tdb_relations::RosterModel;

/// Captaincy roles — the confirmed `eGlu.mCcl` enum.
pub mod captaincy {
    pub const NONE: u64 = 0;
    pub const CAPTAIN: u64 = 1;
    pub const ALTERNATE: u64 = 2;
}

/// What an operation did: how many cell edits, and a human description of each auto-repair applied.
#[derive(Clone, Debug, Default)]
pub struct OpOutcome {
    pub edits: u32,
    pub repairs: Vec<String>,
}

/// A staged, in-memory edit session over one database snapshot.
pub struct EditTransaction {
    working: Vec<u8>,
}

impl EditTransaction {
    /// Begin a transaction over a parseable database snapshot.
    pub fn begin(db_bytes: &[u8]) -> Result<Self, EditError> {
        TdbFile::parse(db_bytes).map_err(|e| EditError::Parse(e.to_string()))?;
        Ok(EditTransaction {
            working: db_bytes.to_vec(),
        })
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

    /// Swap-pop every record of a studio table whose studio field value is in `wanted`. Re-scans from
    /// scratch after each removal (swap-pop relocates the tail). Cluster references are by value, so
    /// no repointing is needed. Returns how many rows were removed.
    fn remove_matching(
        &mut self,
        studio_table: &str,
        studio_field: &str,
        wanted: &HashSet<u64>,
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
            let (new_bytes, _moved) = db
                .remove_record(&raw, index)
                .map_err(|e| EditError::Alloc(e.to_string()))?;
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
            let view = self.working.clone();
            let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
            // Share the save-time predicates so operation-time and save-time agree. (Predicates that
            // need the def registries — referential/enum — are verified at save time; the db-only
            // invariants run here.)
            let mut violations = single_captain(&db);
            let predicate = if violations.is_empty() {
                let comp = companion_presence(&db);
                let p = if comp.is_empty() { "" } else { "companion_presence" };
                violations = comp;
                p
            } else {
                "single_captain"
            };
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

    /// Create a new player as a complete, audit-clean **cluster** wired onto a team: the `ubPc` bio
    /// (fresh unique `wBIz`, `vjla` position, `dXSB` team), the kind-correct companion pair
    /// (`dSvy`+`xmja` for skaters, `mHuy`+`fMVl` for goalies — all keyed on the new `wBIz`), a `ZBac`
    /// index row (`ykFq` → bio, fresh `vfEq`), and an `eGlu` roster slot (`XSWT` → the index, `dXSB`
    /// team). All rows are allocated atomically inside a repairing transaction; the post-condition
    /// audit guarantees the companion invariant holds. Returns the new `wBIz`.
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

            // Roster slot (eGlu): wire onto the team via the index.
            let slot = tx.append(ids::ROSTER_LINES)?;
            tx.set_field(ids::ROSTER_LINES, slot, ids::SLOT_KEY, new_slot_key)?;
            tx.set_field(ids::ROSTER_LINES, slot, ids::SLOT_TO_INDEX, new_vfeq)?;
            tx.set_field(ids::ROSTER_LINES, slot, ids::SLOT_TEAM, team_id)?;

            Ok(OpOutcome {
                edits: 10,
                repairs: vec![format!(
                    "created {} cluster wBIz={new_pid}: bio + {ai}/{equip} + index + slot on team {team_id}",
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
            // 1. Roster slots that bridge to this player's index rows.
            edits += tx.remove_matching(ids::ROSTER_LINES, ids::SLOT_TO_INDEX, &index_keys)?;
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

    /// Copy a slot's **occupant** — every `eGlu` field except the slot-identity fields `toWX` (key)
    /// and `dXSB` (team) — from record `src` to record `dst`. This relocates a player's full slot
    /// content (the `XSWT` index pointer, jersey, style, captaincy, status, and the line-assignment
    /// flags) into another slot **without touching either slot's team or key**, so the per-team
    /// contiguous-block layout is preserved by construction. Returns the number of fields copied.
    fn transplant_slot(&mut self, src: u32, dst: u32) -> Result<u32, EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        let table = db
            .table_canonical(ids::ROSTER_LINES)
            .ok_or_else(|| EditError::UnknownTable(ids::ROSTER_LINES.to_string()))?;
        let mut copies: Vec<(String, u64)> = Vec::new();
        for f in &table.fields {
            let sid = f.id.canonical(db.endian);
            if sid == ids::SLOT_KEY || sid == ids::SLOT_TEAM {
                continue; // preserve the destination slot's own identity (team, key)
            }
            if let Some(v) = db.read_u64(table, src, f) {
                copies.push((sid, v));
            }
        }
        drop(db);
        let n = copies.len() as u32;
        for (sid, v) in copies {
            self.set_field(ids::ROSTER_LINES, dst, &sid, v)?;
        }
        Ok(n)
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
    /// captaincy** on arrival (single-captain repair).
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

            // Relocate occupancy into the free destination slot, then vacate the source slot.
            let mut edits = tx.transplant_slot(source_record, dest_record)?;
            tx.set_field(ids::ROSTER_LINES, source_record, ids::SLOT_TO_INDEX, 0)?; // XSWT = empty
            edits += 1;

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

            Ok(OpOutcome { edits, repairs })
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
            if model
                .slots
                .iter()
                .any(|s| s.team_id == team_id && s.record != target_record && s.jersey == Some(number))
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
