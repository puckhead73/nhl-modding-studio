//! [`EditSession`] — the live editing spine over one [`Project`].
//!
//! Holds the working inner-db bytes and an undo/redo history of snapshots. Every operation runs
//! through the Layer 3 repairing-transaction harness ([`EditTransaction`]): it begins a transaction
//! over the current working bytes, runs the op (which auto-repairs and audits the shared
//! invariants), commits (recomputing all CRCs), and pushes the prior state onto the undo stack. A
//! failed op leaves the working bytes untouched (the harness rolls itself back).

use std::path::Path;

use tdb_core::TdbFile;
use tdb_domain_validation::{validate_invariants, IntegrityReport};
use tdb_edit::txn::{EditTransaction, OpOutcome};
use tdb_edit::EditError;
use tdb_relations::RosterModel;

use crate::error::DomainError;
use crate::project::Project;
use crate::Result;

/// A live edit session: working bytes + undo/redo history over a loaded [`Project`].
pub struct EditSession {
    project: Project,
    working: Vec<u8>,
    undo: Vec<Vec<u8>>,
    redo: Vec<Vec<u8>>,
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
        }
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

    /// Run the full Layer 4 audit (every shared predicate) over the current working state.
    pub fn validate(&self) -> Result<IntegrityReport> {
        let db = TdbFile::parse(&self.working)?;
        Ok(validate_invariants(
            &db,
            self.project.relations(),
            self.project.enums(),
        ))
    }

    /// Run one Layer 3 operation through the repairing-transaction harness, recording undo history.
    /// On any error the working bytes are left exactly as they were.
    fn run<R>(
        &mut self,
        op: impl FnOnce(&mut EditTransaction) -> std::result::Result<R, EditError>,
    ) -> Result<R> {
        let mut tx = EditTransaction::begin(&self.working)?;
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

    // --- Layer 3 operations (the Phase 1 slice), each audited + undoable ---

    /// Set a player's captaincy on a team (single-captain invariant by construction).
    pub fn set_captaincy(&mut self, team: u64, player: u64, role: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.set_captaincy(team, player, role))
    }

    /// Create a new player cluster on a team; returns the new `wBIz` id and the op outcome.
    pub fn create_player(&mut self, team: u64, position: u64) -> Result<(u64, OpOutcome)> {
        self.run(|tx| tx.create_player(team, position))
    }

    /// Delete a player and its whole cluster.
    pub fn delete_player(&mut self, player: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.delete_player(player))
    }

    /// Move/trade a player to another team.
    pub fn move_player(&mut self, player: u64, to_team: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.move_player(player, to_team))
    }

    /// Set a player's jersey number on a team (unique within the team).
    pub fn set_jersey_number(&mut self, team: u64, player: u64, number: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.set_jersey_number(team, player, number))
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
