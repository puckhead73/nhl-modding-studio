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

    /// A structural summary of the current working database (table list, sizes, CRC/size checks).
    pub fn overview(&self) -> Result<crate::DbOverview> {
        let db = TdbFile::parse(&self.working)?;
        Ok(crate::DbOverview::of(&db))
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
        Ok(crate::view::team_summaries(&db, self.project.meta(), &model))
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
    pub fn set_field(
        &mut self,
        studio_table: &str,
        record: u32,
        studio_field: &str,
        value: FieldValue,
    ) -> Result<OpOutcome> {
        let prepared = self.prepare_field_edit(studio_table, record, studio_field, value)?;
        self.run(|tx| match &prepared.payload {
            PreparedPayload::Int(v) => tx.set_cell_int(&prepared.table, prepared.record, &prepared.field, *v),
            PreparedPayload::Text(s) => tx.set_cell_str(&prepared.table, prepared.record, &prepared.field, s),
        })
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

    /// Set a player's roster status (2-bit `ZBac.RREB`).
    pub fn set_roster_status(&mut self, team: u64, player: u64, status: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.set_roster_status(team, player, status))
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
