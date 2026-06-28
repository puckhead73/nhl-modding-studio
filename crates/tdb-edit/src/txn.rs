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
use tdb_core::TdbFile;
use tdb_domain_validation::single_captain;
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
            // Share the save-time predicate so operation-time and save-time agree.
            let violations = single_captain(&db);
            if violations.is_empty() {
                Ok(outcome)
            } else {
                Err(EditError::AuditFailed {
                    predicate: "single_captain",
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

    /// Finalize: recompute all CRCs over the working bytes and return the new database.
    pub fn commit(mut self) -> Result<Vec<u8>, EditError> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view).map_err(|e| EditError::Parse(e.to_string()))?;
        db.recompute_crcs(&mut self.working);
        Ok(self.working)
    }
}
