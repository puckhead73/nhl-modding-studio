//! [`Workspace`] — the multi-file edit spine: a canonical `nhlng.db` [`EditSession`] plus an optional
//! mirrored roster-save session. Reads come from the canonical session; every **write fans out** to
//! the roster too so the two files never desync (the user's rule — some player data, e.g. career
//! stats, lives only in `nhlng.db`, so editing one file without the other corrupts the pairing).
//!
//! The roster save's inner TDB is a verified record-index-aligned **subset** of `nhlng.db` (memory
//! `roster-save-mirrors-nhlng`): for the shared tables a `(table, record, field)` cell edit applies
//! identically to both. The alignment is verified for nhl14 PS3 only, so [`Workspace`] checks it at
//! open (`aligned`) and per structural op, and **degrades to `nhlng`-only** if it doesn't hold rather
//! than risk a silent desync.

use std::collections::HashSet;

use tdb_core::TdbFile;
use tdb_edit::txn::{JerseyReassignment, OpOutcome};

use crate::error::DomainError;
use crate::session::{EditSession, FieldValue};
use crate::Result;

/// The mirrored roster-save session plus what we know about its relationship to `nhlng.db`.
struct Mirror {
    session: EditSession,
    path: String,
    /// Canonical table ids present in **both** files (gates generic `set_field` mirroring).
    shared_tables: HashSet<String>,
    /// `true` when every shared table has equal `current_records` in both files (so record indices
    /// line up and a cell edit maps 1:1). When `false`, generic field edits are not mirrored.
    aligned: bool,
}

/// A loaded editing workspace: the canonical `nhlng.db` session plus an optional roster mirror.
pub struct Workspace {
    nhlng: EditSession,
    nhlng_path: String,
    roster: Option<Mirror>,
    /// Per committed op: was it mirrored into the roster? Drives coordinated undo/redo so a
    /// `nhlng`-only edit (career stats) doesn't pop an unrelated roster state.
    history: Vec<bool>,
    redo: Vec<bool>,
}

impl Workspace {
    /// A single-file workspace (no roster mirror) — the legacy / PS4 / Recomp path.
    pub fn single(nhlng: EditSession, nhlng_path: String) -> Self {
        Workspace {
            nhlng,
            nhlng_path,
            roster: None,
            history: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// A workspace mirroring edits into a roster save. Computes the shared-table set and whether the
    /// two files are record-index-aligned (so generic field edits can be mirrored safely).
    pub fn with_roster(
        nhlng: EditSession,
        nhlng_path: String,
        roster: EditSession,
        roster_path: String,
    ) -> Result<Self> {
        let (shared_tables, aligned) = analyze(&nhlng, &roster)?;
        Ok(Workspace {
            nhlng,
            nhlng_path,
            roster: Some(Mirror {
                session: roster,
                path: roster_path,
                shared_tables,
                aligned,
            }),
            history: Vec::new(),
            redo: Vec::new(),
        })
    }

    // --- Status (for the project summary DTO) ---

    /// The canonical session — all reads (views, validation, summaries) go through here.
    pub fn primary(&self) -> &EditSession {
        &self.nhlng
    }

    pub fn roster_attached(&self) -> bool {
        self.roster.is_some()
    }

    /// `true` when a roster is attached **and** the two files are record-index-aligned (so generic
    /// field edits mirror). A `false` here with `roster_attached() == true` means degraded
    /// (`nhlng`-only for field edits).
    pub fn roster_aligned(&self) -> bool {
        self.roster.as_ref().map(|m| m.aligned).unwrap_or(false)
    }

    pub fn is_dirty(&self) -> bool {
        self.nhlng.is_dirty() || self.roster.as_ref().map(|m| m.session.is_dirty()).unwrap_or(false)
    }

    pub fn undo_depth(&self) -> usize {
        self.history.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    // --- Coordinated history ---

    fn record(&mut self, mirrored: bool) {
        self.history.push(mirrored);
        self.redo.clear();
    }

    /// Revert the last committed op across both files (the roster too iff that op was mirrored).
    pub fn undo(&mut self) -> bool {
        match self.history.pop() {
            Some(mirrored) => {
                let did = self.nhlng.undo();
                if mirrored {
                    if let Some(m) = self.roster.as_mut() {
                        m.session.undo();
                    }
                }
                self.redo.push(mirrored);
                did
            }
            None => false,
        }
    }

    /// Re-apply the last undone op across both files.
    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(mirrored) => {
                let did = self.nhlng.redo();
                if mirrored {
                    if let Some(m) = self.roster.as_mut() {
                        m.session.redo();
                    }
                }
                self.history.push(mirrored);
                did
            }
            None => false,
        }
    }

    /// Enter/leave the acknowledged corruption-override edit mode across both files. When on, edits
    /// are allowed on a DB that fails the structural audit (the user has acknowledged this is
    /// unsupported and may cause unforeseen in-game issues).
    pub fn set_edit_override(&mut self, allow: bool) {
        self.nhlng.set_allow_pre_existing_violations(allow);
        if let Some(m) = self.roster.as_mut() {
            m.session.set_allow_pre_existing_violations(allow);
        }
    }

    // --- Generic field edit (mirrored only for shared, aligned tables) ---

    pub fn set_field(
        &mut self,
        studio_table: &str,
        record: u32,
        studio_field: &str,
        value: FieldValue,
    ) -> Result<OpOutcome> {
        let outcome = self
            .nhlng
            .set_field(studio_table, record, studio_field, value.clone())?;
        let do_mirror = matches!(
            self.roster.as_ref(),
            Some(m) if m.aligned && m.shared_tables.contains(studio_table)
        );
        // `.map` returns an owned Result, so the roster borrow ends before we touch `self` again.
        let roster_result = if do_mirror {
            self.roster
                .as_mut()
                .map(|m| m.session.set_field(studio_table, record, studio_field, value))
        } else {
            None
        };
        let mirror = match roster_result {
            None => false,
            Some(Ok(_)) => true,
            Some(Err(e)) => {
                self.nhlng.undo();
                return Err(e);
            }
        };
        self.record(mirror);
        Ok(outcome)
    }

    // --- Entity-id attribute ops (mirrored whenever a roster is attached) ---

    pub fn set_captaincy(&mut self, team: u64, player: u64, role: u64) -> Result<OpOutcome> {
        self.fan_out_entity(move |s| s.set_captaincy(team, player, role))
    }

    pub fn move_player(&mut self, player: u64, from_team: u64, to_team: u64) -> Result<OpOutcome> {
        self.fan_out_entity(move |s| s.move_player(player, from_team, to_team))
    }

    pub fn trade_slots(
        &mut self,
        a_team: u64,
        a_player: u64,
        b_team: u64,
        b_player: u64,
    ) -> Result<OpOutcome> {
        self.fan_out_entity(move |s| s.trade_slots(a_team, a_player, b_team, b_player))
    }

    pub fn release_to_fa(&mut self, from_team: u64, player: u64) -> Result<OpOutcome> {
        self.fan_out_entity(move |s| s.release_to_fa(from_team, player))
    }

    pub fn sign_from_fa(&mut self, player: u64, to_team: u64) -> Result<OpOutcome> {
        self.fan_out_entity(move |s| s.sign_from_fa(player, to_team))
    }

    pub fn set_jersey_number(&mut self, team: u64, player: u64, number: u64) -> Result<OpOutcome> {
        self.fan_out_entity(move |s| s.set_jersey_number(team, player, number))
    }

    pub fn set_roster_status(&mut self, team: u64, player: u64, status: u64) -> Result<OpOutcome> {
        self.fan_out_entity(move |s| s.set_roster_status(team, player, status))
    }

    /// Auto-correct duplicate jersey numbers across the DB (the default reassignment logic), mirroring
    /// into the roster when attached, aligned, and `eGlu` is shared. Returns the corrections applied.
    /// A no-op leaves both files (and the undo history) untouched.
    pub fn autocorrect_duplicate_jerseys(&mut self) -> Result<(OpOutcome, Vec<JerseyReassignment>)> {
        let (outcome, corrections) = self.nhlng.autocorrect_duplicate_jerseys()?;
        if corrections.is_empty() {
            return Ok((outcome, corrections)); // nothing changed on nhlng — no mirror, no history entry
        }
        // `eGlu` (roster slots) is a shared table; mirror the same reassignments when aligned.
        let do_mirror = matches!(
            self.roster.as_ref(),
            Some(m) if m.aligned && m.shared_tables.contains(tdb_relations::entity::ids::ROSTER_LINES)
        );
        let roster_result = if do_mirror {
            self.roster.as_mut().map(|m| m.session.autocorrect_duplicate_jerseys())
        } else {
            None
        };
        let mirror = match roster_result {
            None => false,
            Some(Ok(_)) => true,
            Some(Err(e)) => {
                self.nhlng.undo();
                return Err(e);
            }
        };
        self.record(mirror);
        Ok((outcome, corrections))
    }

    /// Retire a jersey number for a team (add it to the `wcSL` retired-number table).
    pub fn retire_jersey(&mut self, team: u64, number: u64) -> Result<OpOutcome> {
        self.fan_out_shared(tdb_edit::txn::retired::TABLE, move |s| s.retire_jersey(team, number))
    }

    /// Un-retire a jersey number for a team (remove it from `wcSL`).
    pub fn unretire_jersey(&mut self, team: u64, number: u64) -> Result<OpOutcome> {
        self.fan_out_shared(tdb_edit::txn::retired::TABLE, move |s| {
            s.unretire_jersey(team, number)
        })
    }

    /// Apply an op that touches a single `studio_table`, mirroring to the roster **only when that table
    /// is shared and aligned** (otherwise `nhlng`-only) — the degrade-gracefully pattern of
    /// [`Workspace::set_field`], for structural ops on standalone tables (e.g. `wcSL` retired numbers)
    /// that may not be present in the roster save. Rolls `nhlng` back on a mirror error.
    fn fan_out_shared(
        &mut self,
        studio_table: &str,
        op: impl Fn(&mut EditSession) -> Result<OpOutcome>,
    ) -> Result<OpOutcome> {
        let outcome = op(&mut self.nhlng)?;
        let do_mirror = matches!(
            self.roster.as_ref(),
            Some(m) if m.aligned && m.shared_tables.contains(studio_table)
        );
        let roster_result = if do_mirror {
            self.roster.as_mut().map(|m| op(&mut m.session))
        } else {
            None
        };
        let mirror = match roster_result {
            None => false,
            Some(Ok(_)) => true,
            Some(Err(e)) => {
                self.nhlng.undo();
                return Err(e);
            }
        };
        self.record(mirror);
        Ok(outcome)
    }

    /// Apply an entity-id op to `nhlng`, then (if attached) the same op to the roster. The op is
    /// keyed by entity id (shared across files), so no record remapping is needed. On a mirror
    /// failure, `nhlng` is rolled back so the two stay consistent.
    fn fan_out_entity(
        &mut self,
        op: impl Fn(&mut EditSession) -> Result<OpOutcome>,
    ) -> Result<OpOutcome> {
        let outcome = op(&mut self.nhlng)?;
        let roster_result = self.roster.as_mut().map(|m| op(&mut m.session));
        let mirror = match roster_result {
            None => false,
            Some(Ok(_)) => true,
            Some(Err(e)) => {
                self.nhlng.undo();
                return Err(e);
            }
        };
        self.record(mirror);
        Ok(outcome)
    }

    // --- Structural ops (strict guard: roll both back on any divergence) ---

    /// Create a player cluster on a team. With a roster attached, creates in both and **verifies the
    /// new `wBIz` and full record alignment match** across files; any divergence rolls both back.
    pub fn create_player(&mut self, team: u64, position: u64) -> Result<(u64, OpOutcome)> {
        let (nid, outcome) = self.nhlng.create_player(team, position)?;
        let roster_result = self.roster.as_mut().map(|m| m.session.create_player(team, position));
        match roster_result {
            None => {
                self.record(false);
                Ok((nid, outcome))
            }
            Some(Err(e)) => {
                self.nhlng.undo();
                Err(e)
            }
            Some(Ok((rid, _))) => {
                if rid != nid || !self.fully_aligned()? {
                    self.roll_back_both();
                    return Err(DomainError::Sync(format!(
                        "create_player desync: nhlng allocated wBIz {nid}, roster {rid} \
                         (or record indices diverged); both files rolled back"
                    )));
                }
                self.record(true);
                Ok((nid, outcome))
            }
        }
    }

    /// Clone a player onto a team (deep-copy cluster). Same structural dual-create + `fully_aligned`
    /// guard as [`Workspace::create_player`]: both files clone independently and must agree on the new
    /// `wBIz` and full record alignment, else both roll back.
    pub fn clone_player(&mut self, source_player: u64, to_team: u64) -> Result<(u64, OpOutcome)> {
        let (nid, outcome) = self.nhlng.clone_player(source_player, to_team)?;
        let roster_result = self
            .roster
            .as_mut()
            .map(|m| m.session.clone_player(source_player, to_team));
        match roster_result {
            None => {
                self.record(false);
                Ok((nid, outcome))
            }
            Some(Err(e)) => {
                self.nhlng.undo();
                Err(e)
            }
            Some(Ok((rid, _))) => {
                if rid != nid || !self.fully_aligned()? {
                    self.roll_back_both();
                    return Err(DomainError::Sync(format!(
                        "clone_player desync: nhlng allocated wBIz {nid}, roster {rid} \
                         (or record indices diverged); both files rolled back"
                    )));
                }
                self.record(true);
                Ok((nid, outcome))
            }
        }
    }

    /// Delete a player and its cluster. With a roster attached, deletes from both and **re-verifies
    /// full record alignment**; any divergence rolls both back.
    pub fn delete_player(&mut self, player: u64) -> Result<OpOutcome> {
        let outcome = self.nhlng.delete_player(player)?;
        let roster_result = self.roster.as_mut().map(|m| m.session.delete_player(player));
        match roster_result {
            None => {
                self.record(false);
                Ok(outcome)
            }
            Some(Err(e)) => {
                self.nhlng.undo();
                Err(e)
            }
            Some(Ok(_)) => {
                if !self.fully_aligned()? {
                    self.roll_back_both();
                    return Err(DomainError::Sync(
                        "delete_player desynced the two files; both rolled back".into(),
                    ));
                }
                self.record(true);
                Ok(outcome)
            }
        }
    }

    /// Revert the most recent op on both sessions (used by the structural-op guard on divergence).
    fn roll_back_both(&mut self) {
        if let Some(m) = self.roster.as_mut() {
            m.session.undo();
        }
        self.nhlng.undo();
    }

    /// Strict post-structural-op check: both files have the same player set, each shared player at
    /// the same `bio_record` index. (`true` trivially when no roster is attached.)
    fn fully_aligned(&self) -> Result<bool> {
        let Some(m) = self.roster.as_ref() else {
            return Ok(true);
        };
        let nm = self.nhlng.model()?;
        let rm = m.session.model()?;
        if nm.players.len() != rm.players.len() {
            return Ok(false);
        }
        for p in &nm.players {
            match rm.player(p.player_id) {
                Some(rp) if rp.bio_record == p.bio_record => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    // --- Save ---

    /// In-place dual save: write `nhlng.db` and (if attached) the roster save back to their original
    /// paths, repacking the roster's container with recomputed checksums via `Project::package`.
    pub fn save_in_place(&self) -> Result<u64> {
        self.nhlng.save_to(&self.nhlng_path)?;
        let mut files = 1u64;
        if let Some(m) = &self.roster {
            m.session.save_to(&m.path)?;
            files += 1;
        }
        Ok(files)
    }

    /// Write the canonical session to an explicit path (legacy single-file "Save As").
    pub fn save_primary_to(&self, out: &str) -> Result<u64> {
        let bytes = self.nhlng.save()?;
        let len = bytes.len() as u64;
        std::fs::write(out, &bytes).map_err(|source| DomainError::Io {
            path: out.to_string(),
            source,
        })?;
        Ok(len)
    }
}

/// Compute the shared canonical table set and whether the two files are record-index-aligned.
fn analyze(nhlng: &EditSession, roster: &EditSession) -> Result<(HashSet<String>, bool)> {
    let ndb = TdbFile::parse(nhlng.bytes())?;
    let rdb = TdbFile::parse(roster.bytes())?;
    let nset: HashSet<String> = ndb
        .tables
        .iter()
        .map(|t| t.id.canonical(ndb.endian))
        .collect();

    let mut shared = HashSet::new();
    let mut aligned = true;
    for t in &rdb.tables {
        let cid = t.id.canonical(rdb.endian);
        if nset.contains(&cid) {
            let ncount = ndb.table_canonical(&cid).map(|x| x.current_records);
            if ncount != Some(t.current_records) {
                aligned = false;
            }
            shared.insert(cid);
        }
    }
    Ok((shared, aligned))
}
