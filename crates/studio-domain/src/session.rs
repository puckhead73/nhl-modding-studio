//! [`EditSession`] — the live editing spine over one [`Project`].
//!
//! Holds the working inner-db bytes and an undo/redo history of snapshots. Every operation runs
//! through the Layer 3 repairing-transaction harness ([`EditTransaction`]): it begins a transaction
//! over the current working bytes, runs the op (which auto-repairs and audits the shared
//! invariants), commits (recomputing all CRCs), and pushes the prior state onto the undo stack. A
//! failed op leaves the working bytes untouched (the harness rolls itself back).

use std::path::Path;

use tdb_core::TdbFile;
use tdb_defs::ValidationError;
use tdb_domain_validation::{validate_invariants, IntegrityReport};
use tdb_edit::txn::{EditTransaction, JerseyReassignment, OpOutcome};
use tdb_edit::EditError;
use tdb_relations::RosterModel;

use crate::error::DomainError;
use crate::project::Project;
use crate::Result;

/// A logical value for a generic field edit — an integer (sign-aware) or inline-string text.
#[derive(Clone, Debug)]
pub enum FieldValue {
    Int(i64),
    Text(String),
}

/// A validated, encoded edit ready to write transactionally (produced by `prepare_field_edit`).
struct PreparedEdit {
    table: String,
    field: String,
    record: u32,
    payload: PreparedPayload,
}

enum PreparedPayload {
    Int(u64),
    Text(String),
}

/// A live edit session: working bytes + undo/redo history over a loaded [`Project`].
pub struct EditSession {
    project: Project,
    working: Vec<u8>,
    undo: Vec<Vec<u8>>,
    redo: Vec<Vec<u8>>,
    /// Acknowledged "override" mode — edits proceed even though the DB fails the structural audit
    /// (see [`EditTransaction::set_allow_pre_existing_violations`]). Off by default.
    allow_pre_existing_violations: bool,
}

impl EditSession {
    /// Begin a session over a loaded project. The working copy starts as the project's inner db.
    pub fn begin(project: Project) -> Self {
        let working = project.db_bytes().to_vec();
        EditSession {
            project,
            working,
            undo: Vec::new(),
            redo: Vec::new(),
            allow_pre_existing_violations: false,
        }
    }

    /// Enter/leave the acknowledged corruption-override edit mode. When on, the per-op post-condition
    /// audit no longer blocks edits on a DB that already fails the structural invariants (unsupported).
    pub fn set_allow_pre_existing_violations(&mut self, allow: bool) {
        self.allow_pre_existing_violations = allow;
    }

    /// The project this session edits.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The current working inner-db bytes (CRC-valid after every committed op).
    pub fn bytes(&self) -> &[u8] {
        &self.working
    }

    /// `true` once the working bytes differ from the loaded baseline.
    pub fn is_dirty(&self) -> bool {
        self.working != self.project.db_bytes()
    }

    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    /// Grow one table's **allocation** (`max_records`) up to `new_max`, leaving its population
    /// (`current_records`) untouched — the Phase 4.4 capacity-normalization primitive
    /// (docs/audits/roster-capacity.md). Runs **outside** the undo history: it is a load-time
    /// repair of a packed file back to the vanilla pre-allocated shape, not a user edit.
    ///
    /// Returns `Some((from, to))` when the table grew, `None` when it is absent from this file's
    /// subset or already at/above `new_max` (idempotent).
    pub fn grow_table_max(&mut self, canonical_table: &str, new_max: u16) -> Result<Option<(u16, u16)>> {
        let db = TdbFile::parse(&self.working)?;
        let Some(t) = db.table_canonical(canonical_table) else {
            return Ok(None); // not in this file's table subset — nothing to normalize
        };
        let from = t.max_records;
        if from >= new_max {
            return Ok(None);
        }
        let raw = t.id.raw();
        let grown = db
            .grow_table_max(&raw, new_max)
            .map_err(|e| DomainError::Edit(EditError::Alloc(e.to_string())))?;
        self.working = grown;
        Ok(Some((from, new_max)))
    }

    /// A structural summary of the current working database (table list, sizes, CRC/size checks).
    pub fn overview(&self) -> Result<crate::DbOverview> {
        let db = TdbFile::parse(&self.working)?;
        Ok(crate::DbOverview::of(&db))
    }

    /// Probe which optional schema features this title's database carries (drives the UI's
    /// hide/disable gating for surfaces the game doesn't have). Evidence-based: read from the
    /// working file's own descriptors and the title's own metadata, never assumed from the title.
    pub fn capabilities(&self) -> Result<crate::Capabilities> {
        let db = TdbFile::parse(&self.working)?;
        Ok(crate::Capabilities::probe(&db, self.project.meta()))
    }

    /// Assemble the Layer 2 entity model over the current working state.
    pub fn model(&self) -> Result<RosterModel> {
        let db = TdbFile::parse(&self.working)?;
        RosterModel::build(&db).map_err(|e| DomainError::Edit(EditError::Entity(e.to_string())))
    }

    /// The player master list (view-models) over the current working state.
    pub fn player_summaries(&self) -> Result<Vec<crate::PlayerSummary>> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        Ok(crate::view::player_summaries(
            &db,
            self.project.meta(),
            self.project.enums(),
            &model,
        ))
    }

    /// The full Player detail view for one `wBIz`, or `None` if no such player.
    pub fn player_view(&self, player_id: u64) -> Result<Option<crate::PlayerView>> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        Ok(crate::view::player_view(
            &db,
            self.project.meta(),
            self.project.enums(),
            &model,
            player_id,
        ))
    }

    /// The team master list over the current working state.
    pub fn team_summaries(&self) -> Result<Vec<crate::TeamSummary>> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        Ok(crate::view::team_summaries(
            &db,
            self.project.meta(),
            self.project.enums(),
            &model,
        ))
    }

    /// The full Team detail for one `kOtt.vfEq`, or `None` if absent.
    pub fn team_view(&self, team_id: u64) -> Result<Option<crate::TeamView>> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        Ok(crate::view::team_view(
            &db,
            self.project.meta(),
            self.project.enums(),
            &model,
            team_id,
        ))
    }

    /// The arena master list over the current working state.
    pub fn arena_summaries(&self) -> Result<Vec<crate::ArenaSummary>> {
        let db = TdbFile::parse(&self.working)?;
        Ok(crate::view::arena_summaries(&db, self.project.meta()))
    }

    /// The full Arena detail for one `StEO.vfEq`, or `None` if absent.
    pub fn arena_view(&self, arena_id: u64) -> Result<Option<crate::ArenaView>> {
        let db = TdbFile::parse(&self.working)?;
        Ok(crate::view::arena_view(
            &db,
            self.project.meta(),
            self.project.enums(),
            arena_id,
        ))
    }

    /// A player's read-only stock career stats over the current working state.
    pub fn career_stats(&self, player_id: u64) -> Result<crate::CareerStats> {
        let db = TdbFile::parse(&self.working)?;
        Ok(crate::view::career_stats(
            &db,
            self.project.meta(),
            self.project.enums(),
            player_id,
        ))
    }

    /// A team's strategy sheet (`EeYo`), or `None` when the title/database lacks it.
    pub fn team_strategies(&self, team_id: u64) -> Result<Option<crate::TeamStrategies>> {
        let db = TdbFile::parse(&self.working)?;
        Ok(crate::team_tabs::team_strategies(
            &db,
            self.project.meta(),
            self.project.enums(),
            team_id,
        ))
    }

    /// A team's future draft picks (`qHav`), possibly empty.
    pub fn team_draft_picks(&self, team_id: u64) -> Result<Vec<crate::DraftPick>> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        Ok(crate::team_tabs::team_draft_picks(
            &db,
            self.project.meta(),
            &model,
            team_id,
        ))
    }

    /// A team's registered jerseys (`xjgw`), possibly empty.
    pub fn team_jerseys(&self, team_id: u64) -> Result<Vec<crate::TeamJersey>> {
        let db = TdbFile::parse(&self.working)?;
        Ok(crate::team_tabs::team_jerseys(&db, team_id))
    }

    /// A team's stock equipment rows (`hHbv`), possibly empty.
    pub fn team_equipment(&self, team_id: u64) -> Result<Vec<crate::TeamEquipment>> {
        let db = TdbFile::parse(&self.working)?;
        Ok(crate::team_tabs::team_equipment(&db, team_id))
    }

    /// The depth chart (lines) for one team over the current working state.
    pub fn lines_view(&self, team_id: u64) -> Result<Option<crate::LinesView>> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        Ok(crate::view::lines_view(
            &db,
            self.project.meta(),
            self.project.enums(),
            &model,
            team_id,
        ))
    }

    fn build_model<'a>(&self, db: &TdbFile<'a>) -> Result<RosterModel> {
        RosterModel::build(db).map_err(|e| DomainError::Edit(EditError::Entity(e.to_string())))
    }

    /// Run the full Layer 4 audit (every shared predicate) over the current working state.
    pub fn validate(&self) -> Result<IntegrityReport> {
        let db = TdbFile::parse(&self.working)?;
        Ok(validate_invariants(
            &db,
            self.project.relations(),
            self.project.enums(),
        ))
    }

    /// Run the Layer 4 audit and resolve its findings into display-ready text (real team/player
    /// names, metadata field labels) — the human-readable-surfaces rule. Raw ids ride along for
    /// power-user disclosure. This is what the UI should call; [`Self::validate`] stays the
    /// engine-facing, id-precise form.
    pub fn audit_report(&self) -> Result<crate::audit::HumanReport> {
        let db = TdbFile::parse(&self.working)?;
        let raw = validate_invariants(&db, self.project.relations(), self.project.enums());
        Ok(crate::audit::humanize(
            &db,
            self.project.meta(),
            self.project.enums(),
            raw.findings,
        ))
    }

    /// Run one Layer 3 operation through the repairing-transaction harness, recording undo history.
    /// On any error the working bytes are left exactly as they were.
    fn run<R>(
        &mut self,
        op: impl FnOnce(&mut EditTransaction) -> std::result::Result<R, EditError>,
    ) -> Result<R> {
        let mut tx = EditTransaction::begin(&self.working)?;
        tx.set_allow_pre_existing_violations(self.allow_pre_existing_violations);
        let result = op(&mut tx)?;
        let new_bytes = tx.commit()?;
        self.undo.push(std::mem::replace(&mut self.working, new_bytes));
        self.redo.clear();
        Ok(result)
    }

    /// Revert the last committed operation. Returns `false` if there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(std::mem::replace(&mut self.working, prev));
                true
            }
            None => false,
        }
    }

    /// Re-apply the last undone operation. Returns `false` if there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(std::mem::replace(&mut self.working, next));
                true
            }
            None => false,
        }
    }

    /// Validate and stage a generic field edit (studio ids): integer fields are range/updatable
    /// checked and two's-complement encoded against the metadata; string fields are length-checked.
    /// The write runs through the repairing transaction (audited + undoable). Rejected edits leave
    /// the working bytes untouched and return the reason.
    ///
    /// Strategy edits fan out: an `EeYo` (exhibition strategies) cell write is mirrored onto the
    /// `JUfQ` (career-mode strategies) row with the same `vfEq` in the **same transaction**, so
    /// changing a team's strategy lands in both game modes. `JUfQ` ships with 0 rows in stock
    /// data — the mirror silently no-ops until a dynasty save populates it.
    pub fn set_field(
        &mut self,
        studio_table: &str,
        record: u32,
        studio_field: &str,
        value: FieldValue,
    ) -> Result<OpOutcome> {
        let prepared = self.prepare_field_edit(studio_table, record, studio_field, value.clone())?;
        let mirror = if studio_table == crate::team_tabs::STRATEGIES_TABLE {
            self.prepare_strategy_mirror(record, studio_field, value)
        } else {
            None
        };
        self.run(|tx| {
            let mut total = match &prepared.payload {
                PreparedPayload::Int(v) => {
                    tx.set_cell_int(&prepared.table, prepared.record, &prepared.field, *v)?
                }
                PreparedPayload::Text(s) => {
                    tx.set_cell_str(&prepared.table, prepared.record, &prepared.field, s)?
                }
            };
            if let Some(m) = &mirror {
                let o = match &m.payload {
                    PreparedPayload::Int(v) => tx.set_cell_int(&m.table, m.record, &m.field, *v)?,
                    PreparedPayload::Text(s) => tx.set_cell_str(&m.table, m.record, &m.field, s)?,
                };
                total.edits += o.edits;
                total.repairs.extend(o.repairs);
                total
                    .repairs
                    .push("mirrored the strategy edit to the career-mode table (JUfQ)".into());
            }
            Ok(total)
        })
    }

    /// Prepare the `JUfQ` twin of an `EeYo` cell edit (same studio field, row matched on `vfEq`).
    /// `None` whenever the mirror can't or shouldn't fire — table absent, no matching row, the
    /// key field itself, or a validation failure on the mirror side — so the primary edit always
    /// proceeds alone rather than being blocked by career-mode state.
    fn prepare_strategy_mirror(
        &self,
        record: u32,
        studio_field: &str,
        value: FieldValue,
    ) -> Option<PreparedEdit> {
        if studio_field == "vfEq" {
            return None;
        }
        let view = self.working.clone();
        let db = TdbFile::parse(&view).ok()?;
        let eeyo = db.table_canonical(crate::team_tabs::STRATEGIES_TABLE)?;
        let team = crate::view::read_u64_by_studio(&db, eeyo, record, "vfEq")?;
        let jufq = db.table_canonical(crate::team_tabs::STRATEGIES_LEAGUE_TABLE)?;
        let mirror_rec = (0..jufq.current_records as u32)
            .find(|&r| crate::view::read_u64_by_studio(&db, jufq, r, "vfEq") == Some(team))?;
        self.prepare_field_edit(
            crate::team_tabs::STRATEGIES_LEAGUE_TABLE,
            mirror_rec,
            studio_field,
            value,
        )
        .ok()
    }

    /// Stage several integer cell edits on one record as a **single transaction** (one undo entry).
    /// Each `(studio_field, value)` is validated + encoded exactly like [`Self::set_field`]. Used for
    /// set-membership edits such as trait activation, where one logical change touches several slots.
    /// A rejected edit leaves the working bytes untouched.
    pub fn set_cells_int(
        &mut self,
        studio_table: &str,
        record: u32,
        writes: &[(String, i64)],
    ) -> Result<OpOutcome> {
        // Prepare (validate + encode) every write before taking `&mut self` for `run`.
        let prepared: Vec<PreparedEdit> = writes
            .iter()
            .map(|(field, value)| {
                self.prepare_field_edit(studio_table, record, field, FieldValue::Int(*value))
            })
            .collect::<Result<_>>()?;
        self.run(|tx| {
            let mut total = OpOutcome::default();
            for p in &prepared {
                let outcome = match &p.payload {
                    PreparedPayload::Int(v) => {
                        tx.set_cell_int(&p.table, p.record, &p.field, *v)?
                    }
                    PreparedPayload::Text(s) => {
                        tx.set_cell_str(&p.table, p.record, &p.field, s)?
                    }
                };
                total.edits += outcome.edits;
                total.repairs.extend(outcome.repairs);
            }
            Ok(total)
        })
    }

    /// The current `(slot_field_id, value)` for all 32 `Trait N` slots on a ratings record — the
    /// input the trait activate/deactivate planner needs. Missing slots (absent from this title's
    /// schema) are skipped.
    pub fn trait_slot_values(&self, studio_table: &str, record: u32) -> Result<Vec<(String, i64)>> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view)?;
        let table = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?;
        let mut out = Vec::new();
        for slot_id in crate::attributes::TRAIT_SLOT_ORDER {
            if let Some(field) = table
                .fields
                .iter()
                .find(|f| f.id.canonical(db.endian) == *slot_id)
            {
                let raw = db.read_u64(table, record, field).unwrap_or(0);
                let value = self
                    .project
                    .meta()
                    .field(&table.id.raw(), &field.id.raw())
                    .map(|m| m.decode_raw(raw))
                    .unwrap_or(raw as i64);
                out.push(((*slot_id).to_string(), value));
            }
        }
        Ok(out)
    }

    /// Resolve the field descriptor + metadata and validate `value` at entry, returning an owned
    /// encoded edit. All borrows of `self` end with the returned value, so the caller can then take
    /// `&mut self` for `run`.
    fn prepare_field_edit(
        &self,
        studio_table: &str,
        record: u32,
        studio_field: &str,
        value: FieldValue,
    ) -> Result<PreparedEdit> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view)?;
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
        // Metadata is keyed by the on-disk id (`raw()` — endian-correct).
        let meta = self
            .project
            .meta()
            .field(&table.id.raw(), &field.id.raw())
            .ok_or_else(|| EditError::NoMeta {
                table: studio_table.to_string(),
                field: studio_field.to_string(),
            })?;

        let payload = match value {
            FieldValue::Int(i) => {
                meta.validate(i).map_err(EditError::Invalid)?;
                PreparedPayload::Int(meta.encode(i))
            }
            FieldValue::Text(s) => {
                if !meta.updatable {
                    return Err(EditError::Invalid(ValidationError::NotUpdatable {
                        field: studio_field.to_string(),
                    })
                    .into());
                }
                let width_bytes = (field.bit_width / 8) as usize;
                if s.len() > width_bytes {
                    return Err(EditError::InvalidValue {
                        what: "string length",
                        value: s.len() as i64,
                    }
                    .into());
                }
                PreparedPayload::Text(s)
            }
        };

        Ok(PreparedEdit {
            table: studio_table.to_string(),
            field: studio_field.to_string(),
            record,
            payload,
        })
    }

    // --- Layer 3 operations (the Phase 1 slice), each audited + undoable ---

    /// Set a player's captaincy on a team (single-captain invariant by construction).
    pub fn set_captaincy(&mut self, team: u64, player: u64, role: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.set_captaincy(team, player, role))
    }

    /// Create a new player cluster on a team; returns the new `wBIz` id and the op outcome.
    pub fn create_player(&mut self, team: u64, position: u64) -> Result<(u64, OpOutcome)> {
        self.run(|tx| tx.create_player(team, position))
    }

    /// Clone an existing player onto a team (deep-copy of bio + companions); returns the new `wBIz`.
    pub fn clone_player(&mut self, source_player: u64, to_team: u64) -> Result<(u64, OpOutcome)> {
        self.run(|tx| tx.clone_player(source_player, to_team))
    }

    /// Delete a player and its whole cluster.
    pub fn delete_player(&mut self, player: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.delete_player(player))
    }

    /// Move a player's roster membership from `from_team` to another team.
    pub fn move_player(&mut self, player: u64, from_team: u64, to_team: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.move_player(player, from_team, to_team))
    }

    /// Swap two players' roster memberships between teams (the block-full alternative to a move).
    pub fn trade_slots(
        &mut self,
        a_team: u64,
        a_player: u64,
        b_team: u64,
        b_player: u64,
    ) -> Result<OpOutcome> {
        self.run(|tx| tx.trade_slots(a_team, a_player, b_team, b_player))
    }

    /// Release a player from a team to free agency (vacate their roster slot).
    pub fn release_to_fa(&mut self, from_team: u64, player: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.release_to_fa(from_team, player))
    }

    /// Sign an off-roster free agent onto a team's free block slot.
    pub fn sign_from_fa(&mut self, player: u64, to_team: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.sign_from_fa(player, to_team))
    }

    /// Add an extra roster membership (national / all-star / alternate roster) on `to_team`,
    /// leaving the player's primary team untouched.
    pub fn add_membership(&mut self, player: u64, to_team: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.add_membership(player, to_team))
    }

    /// Remove one extra roster membership (vacate the player's slot on `from_team`); the primary
    /// team pointer and all other memberships stay untouched.
    pub fn remove_membership(&mut self, from_team: u64, player: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.remove_membership(from_team, player))
    }

    /// Add a stock career season row (one of the four career tables) for a player; returns the new
    /// record index. The row's identity (join key, year, league, team code) is set; stat cells start
    /// zero and are edited through [`Self::set_field`].
    pub fn add_career_season(
        &mut self,
        studio_table: &str,
        player: u64,
        year_end: u64,
        league: u64,
        team: &str,
    ) -> Result<(u32, OpOutcome)> {
        self.run(|tx| tx.add_career_season(studio_table, player, year_end, league, team))
    }

    /// Delete one stock career row by record index (ordered shift — stored row order preserved).
    pub fn delete_career_row(&mut self, studio_table: &str, record: u32) -> Result<OpOutcome> {
        self.run(|tx| tx.delete_career_row(studio_table, record))
    }

    /// Retire a jersey number for a team (add it to the retired-number table).
    pub fn retire_jersey(&mut self, team: u64, number: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.retire_jersey(team, number))
    }

    /// Un-retire a jersey number for a team (remove it from the retired-number table).
    pub fn unretire_jersey(&mut self, team: u64, number: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.unretire_jersey(team, number))
    }

    /// Set a player's jersey number on a team (unique within the team).
    pub fn set_jersey_number(&mut self, team: u64, player: u64, number: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.set_jersey_number(team, player, number))
    }

    /// Trade a future draft pick (`qHav` row) to another team.
    pub fn trade_draft_pick(&mut self, record: u32, to_team: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.trade_draft_pick(record, to_team))
    }

    /// Auto-correct every duplicate jersey number in the DB (the default reassignment logic), returning
    /// the corrections applied. A no-op when there are none — and, in that case, it deliberately does
    /// **not** run a transaction, so it leaves no undo entry and doesn't mark the project dirty.
    pub fn autocorrect_duplicate_jerseys(&mut self) -> Result<(OpOutcome, Vec<JerseyReassignment>)> {
        // Peek read-only: nothing to fix → return without touching the working bytes or undo history.
        let peek = EditTransaction::begin(&self.working)?;
        if peek.plan_jersey_corrections()?.is_empty() {
            return Ok((OpOutcome::default(), Vec::new()));
        }
        self.run(|tx| tx.autocorrect_duplicate_jerseys())
    }

    /// Set a player's lineup status (dressed vs. scratched) on their slot in `team`'s block — the
    /// per-slot `eGlu.hSZj` boolean (Spike S4). Not `ZBac.RREB` (that is player type).
    pub fn set_dressed(&mut self, team: u64, player: u64, dressed: bool) -> Result<OpOutcome> {
        self.run(|tx| tx.set_dressed(team, player, dressed))
    }

    // --- Save spine ---

    /// Package the current working db into the project's on-disk form (raw db, or repacked
    /// container) — the bytes to write back.
    pub fn save(&self) -> Result<Vec<u8>> {
        self.project.package(&self.working)
    }

    /// Save to a path on disk.
    pub fn save_to(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let bytes = self.save()?;
        std::fs::write(path, bytes).map_err(|source| DomainError::Io {
            path: path.display().to_string(),
            source,
        })
    }
}
