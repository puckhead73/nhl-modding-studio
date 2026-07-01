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
use tdb_edit::txn::OpOutcome;

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

    pub fn set_jersey_number(&mut self, team: u64, player: u64, number: u64) -> Result<OpOutcome> {
        self.fan_out_entity(move |s| s.set_jersey_number(team, player, number))
    }

    pub fn set_roster_status(&mut self, team: u64, player: u64, status: u64) -> Result<OpOutcome> {
        self.fan_out_entity(move |s| s.set_roster_status(team, player, status))
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
