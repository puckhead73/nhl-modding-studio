//! [`Workspace`] — the multi-file edit spine: a canonical `nhlng.db` [`EditSession`] plus an optional
//! mirrored roster-save session. Reads come from the display session ([`Workspace::display`] —
//! canonical by default, switchable to the roster mirror for users who only care about the roster
//! save); every **write fans out** to the roster too so the two files never desync (the user's rule
//! — some player data, e.g. career stats, lives only in `nhlng.db`, so editing one file without the
//! other corrupts the pairing).
//!
//! When the displayed roster is **not** record-index-aligned with `nhlng.db`, the pairing is already
//! desynced and record indices don't map across files — so the workspace enters *roster-only* mode
//! ([`Workspace::roster_only`]): every edit applies strictly to the roster session and `nhlng.db` is
//! left untouched (surfaced to the user as "not all areas are editable without an aligned
//! nhlng.db"). Undo/redo tracks which file(s) each op touched so history stays coherent across mode
//! switches.
//!
//! The roster save's inner TDB is a verified record-index-aligned **subset** of `nhlng.db` (memory
//! `roster-save-mirrors-nhlng`): for the shared tables a `(table, record, field)` cell edit applies
//! identically to both. The alignment is verified for nhl14 PS3 only, so [`Workspace`] checks it at
//! open (`aligned`) and per structural op, and **degrades to `nhlng`-only** if it doesn't hold rather
//! than risk a silent desync.

use std::collections::HashSet;

use tdb_core::TdbFile;
use tdb_edit::txn::{BlockRepairSummary, JerseyReassignment, OpOutcome};

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
    /// The studio game id (`nhl10`…`nhllegacy`) this workspace was opened as — set by the onboarding
    /// path; `None` on the legacy file-pick path (where the user never named a title).
    game_id: Option<String>,
    /// Per committed op: which file(s) it touched. Drives coordinated undo/redo so a `nhlng`-only
    /// edit (career stats) or a roster-only edit (misaligned display mode) doesn't pop an unrelated
    /// state on the other file.
    history: Vec<OpTarget>,
    redo: Vec<OpTarget>,
    /// When `true`, reads are served from the roster mirror instead of `nhlng.db` (for users whose
    /// game only loads the roster save). On an **aligned** pair the write path is unchanged (writes
    /// fan out from `nhlng` so the pairing stays in sync); on a misaligned pair this flag puts the
    /// workspace in roster-only mode instead ([`Workspace::roster_only`]).
    display_roster: bool,
}

/// Which file(s) a committed op landed in (the undo/redo unit).
#[derive(Clone, Copy)]
enum OpTarget {
    /// `nhlng.db` only (e.g. a table the roster save doesn't carry).
    Canonical,
    /// Mirrored into both files.
    Both,
    /// The roster save only (roster-only mode on a misaligned pair).
    Roster,
}

impl Workspace {
    /// A single-file workspace (no roster mirror) — the legacy / PS4 / Recomp path.
    pub fn single(nhlng: EditSession, nhlng_path: String) -> Self {
        Workspace {
            nhlng,
            nhlng_path,
            roster: None,
            game_id: None,
            history: Vec::new(),
            redo: Vec::new(),
            display_roster: false,
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
            game_id: None,
            history: Vec::new(),
            redo: Vec::new(),
            display_roster: false,
        })
    }

    /// Tag the workspace with the studio game id it was opened as (onboarding path).
    pub fn set_game_id(&mut self, game_id: impl Into<String>) {
        self.game_id = Some(game_id.into());
    }

    // --- Status (for the project summary DTO) ---

    /// The studio game id (`nhl10`…`nhllegacy`), when known.
    pub fn game_id(&self) -> Option<&str> {
        self.game_id.as_deref()
    }

    /// The canonical `nhlng.db` session (the write spine and save source — not necessarily what the
    /// user is looking at; reads go through [`Workspace::display`]).
    pub fn primary(&self) -> &EditSession {
        &self.nhlng
    }

    /// The session reads (views, validation, summaries) are served from: the roster mirror when the
    /// display toggle is on, else the canonical `nhlng.db`.
    pub fn display(&self) -> &EditSession {
        match &self.roster {
            Some(m) if self.display_roster => &m.session,
            _ => &self.nhlng,
        }
    }

    /// The on-disk write source of the attached roster mirror, if any — lets callers flag an
    /// experimental write path (e.g. a v1 container) that lives in the mirror rather than the
    /// canonical `nhlng.db`.
    pub fn roster_source(&self) -> Option<&crate::Source> {
        self.roster.as_ref().map(|m| m.session.project().source())
    }

    /// Whether reads are currently served from the roster mirror (`false` when no mirror is
    /// attached, even if the toggle was set).
    pub fn display_roster(&self) -> bool {
        self.display_roster && self.roster.is_some()
    }

    /// `true` when the workspace is in roster-only mode: the roster mirror is displayed but is
    /// **not** record-index-aligned with `nhlng.db`, so record indices don't map across files and
    /// every edit applies strictly to the roster session (`nhlng.db` untouched).
    pub fn roster_only(&self) -> bool {
        self.display_roster && matches!(&self.roster, Some(m) if !m.aligned)
    }

    /// Switch reads between the canonical `nhlng.db` and the roster mirror. Turning the roster view
    /// on only requires an attached mirror; if the pair is misaligned the workspace runs in
    /// roster-only mode ([`Workspace::roster_only`]) rather than refusing.
    pub fn set_display_roster(&mut self, on: bool) -> Result<()> {
        if on && self.roster.is_none() {
            return Err(DomainError::Sync(
                "no roster save is attached to this workspace".into(),
            ));
        }
        self.display_roster = on;
        Ok(())
    }

    /// The roster session as the sole edit target (roster-only mode). Panics if called without an
    /// attached mirror — gate on [`Workspace::roster_only`] first.
    fn roster_mut(&mut self) -> &mut EditSession {
        &mut self.roster.as_mut().expect("roster_only implies an attached mirror").session
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

    fn record(&mut self, target: OpTarget) {
        self.history.push(target);
        self.redo.clear();
    }

    /// Record a canonical-first op: `Both` when it was mirrored into the roster, else `Canonical`.
    fn record_mirrored(&mut self, mirrored: bool) {
        self.record(if mirrored { OpTarget::Both } else { OpTarget::Canonical });
    }

    /// Revert the last committed op on exactly the file(s) it touched.
    pub fn undo(&mut self) -> bool {
        match self.history.pop() {
            Some(target) => {
                let did = match target {
                    OpTarget::Canonical => self.nhlng.undo(),
                    OpTarget::Both => {
                        let did = self.nhlng.undo();
                        if let Some(m) = self.roster.as_mut() {
                            m.session.undo();
                        }
                        did
                    }
                    OpTarget::Roster => {
                        self.roster.as_mut().map(|m| m.session.undo()).unwrap_or(false)
                    }
                };
                self.redo.push(target);
                did
            }
            None => false,
        }
    }

    /// Re-apply the last undone op on exactly the file(s) it touched.
    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(target) => {
                let did = match target {
                    OpTarget::Canonical => self.nhlng.redo(),
                    OpTarget::Both => {
                        let did = self.nhlng.redo();
                        if let Some(m) = self.roster.as_mut() {
                            m.session.redo();
                        }
                        did
                    }
                    OpTarget::Roster => {
                        self.roster.as_mut().map(|m| m.session.redo()).unwrap_or(false)
                    }
                };
                self.history.push(target);
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

    // --- Roster capacity normalization (Phase 4.4, docs/audits/roster-capacity.md) ---

    /// Normalize the attached **roster save's** allocation back to the vanilla ceilings: for each
    /// roster-bearing table whose `max_records` was packed below the ceiling, pre-allocate slack up
    /// to it (`current_records` untouched), so edits fill slack the way the vanilla file does.
    /// Run once at workspace open, after [`Workspace::set_game_id`].
    ///
    /// Ceiling per table = the larger of the GEN7 constant for this `game_id`
    /// ([`crate::capacity::gen7_ceilings`], the audited vanilla-ROSTER reads — the packed GEN7 main
    /// DB cannot supply it) and the attached main DB's own `max_records` (exact on PS4-style mains
    /// that carry the slack; a safe no-op floor on packed GEN7 mains). Never touches the main DB,
    /// idempotent, and a no-op on every vanilla-shaped file. Not undoable — it is a load-time
    /// repair, so it lives outside the op history.
    pub fn normalize_roster_capacity(&mut self) -> Result<Vec<crate::capacity::CapacityGrowth>> {
        let Some(m) = self.roster.as_mut() else {
            return Ok(Vec::new()); // single-file workspace — nothing to normalize
        };
        let main = TdbFile::parse(self.nhlng.bytes())?;
        let consts = self.game_id.as_deref().and_then(crate::capacity::gen7_ceilings);
        let mut grown = Vec::new();
        for (i, cid) in crate::capacity::ROSTER_TABLES.iter().enumerate() {
            let main_max = main.table_canonical(cid).map(|t| t.max_records).unwrap_or(0);
            let ceiling = consts.map(|c| c[i]).unwrap_or(0).max(main_max);
            if ceiling == 0 {
                continue; // no ceiling source for this table — leave it alone
            }
            if let Some((from, to)) = m.session.grow_table_max(cid, ceiling)? {
                grown.push(crate::capacity::CapacityGrowth { table: cid.to_string(), from, to });
            }
        }
        if !grown.is_empty() {
            // Re-derive the mirror relationship over the grown shape (growth leaves
            // `current_records` alone, so alignment should be unchanged — but never assume).
            let (shared_tables, aligned) = analyze(&self.nhlng, &m.session)?;
            m.shared_tables = shared_tables;
            m.aligned = aligned;
        }
        Ok(grown)
    }

    // --- Generic field edit (mirrored only for shared, aligned tables) ---

    pub fn set_field(
        &mut self,
        studio_table: &str,
        record: u32,
        studio_field: &str,
        value: FieldValue,
    ) -> Result<OpOutcome> {
        if self.roster_only() {
            // Misaligned display mode: the record index came from a roster view and only maps
            // there — apply strictly to the roster file.
            let outcome = self.roster_mut().set_field(studio_table, record, studio_field, value)?;
            self.record(OpTarget::Roster);
            return Ok(outcome);
        }
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
        self.record_mirrored(mirror);
        Ok(outcome)
    }

    /// Activate or deactivate a trait on a player's ratings record (`dSvy`/`mHuy`). Traits live in 32
    /// `Trait N` slots; activation writes the trait number into the first free slot, deactivation
    /// zeroes every slot holding it. Both cases apply as a **single** transaction/undo entry. Mirrored
    /// into the roster save only when the ratings table is shared and aligned (as [`Self::set_field`]).
    /// Returns an unchanged no-op when the trait is already in the desired state.
    pub fn set_player_trait(
        &mut self,
        studio_table: &str,
        record: u32,
        value: i64,
        active: bool,
    ) -> Result<OpOutcome> {
        if self.roster_only() {
            // Plan against the roster's own slot state — the record index only maps there.
            let m = self.roster_mut();
            let slots = m.trait_slot_values(studio_table, record)?;
            let writes = crate::attributes::plan_trait_toggle(&slots, value, active).map_err(|()| {
                DomainError::Edit(tdb_edit::EditError::InvalidValue {
                    what: "no free trait slot (all 32 occupied)",
                    value,
                })
            })?;
            if writes.is_empty() {
                return Ok(OpOutcome::default());
            }
            let outcome = self.roster_mut().set_cells_int(studio_table, record, &writes)?;
            self.record(OpTarget::Roster);
            return Ok(outcome);
        }
        let slots = self.nhlng.trait_slot_values(studio_table, record)?;
        let writes = crate::attributes::plan_trait_toggle(&slots, value, active).map_err(|()| {
            DomainError::Edit(tdb_edit::EditError::InvalidValue {
                what: "no free trait slot (all 32 occupied)",
                value,
            })
        })?;
        if writes.is_empty() {
            return Ok(OpOutcome::default()); // already in the desired state — no edit, no history
        }
        let outcome = self.nhlng.set_cells_int(studio_table, record, &writes)?;
        // Mirror the identical writes when the ratings table is shared + aligned.
        let do_mirror = matches!(
            self.roster.as_ref(),
            Some(m) if m.aligned && m.shared_tables.contains(studio_table)
        );
        let roster_result = if do_mirror {
            self.roster
                .as_mut()
                .map(|m| m.session.set_cells_int(studio_table, record, &writes))
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
        self.record_mirrored(mirror);
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

    pub fn add_membership(&mut self, player: u64, to_team: u64) -> Result<OpOutcome> {
        self.fan_out_entity(move |s| s.add_membership(player, to_team))
    }

    pub fn remove_membership(&mut self, from_team: u64, player: u64) -> Result<OpOutcome> {
        self.fan_out_entity(move |s| s.remove_membership(from_team, player))
    }

    pub fn set_jersey_number(&mut self, team: u64, player: u64, number: u64) -> Result<OpOutcome> {
        self.fan_out_entity(move |s| s.set_jersey_number(team, player, number))
    }

    pub fn set_dressed(&mut self, team: u64, player: u64, dressed: bool) -> Result<OpOutcome> {
        self.fan_out_entity(move |s| s.set_dressed(team, player, dressed))
    }

    /// Auto-correct duplicate jersey numbers across the DB (the default reassignment logic), mirroring
    /// into the roster when attached, aligned, and `eGlu` is shared. Returns the corrections applied.
    /// A no-op leaves both files (and the undo history) untouched.
    pub fn autocorrect_duplicate_jerseys(&mut self) -> Result<(OpOutcome, Vec<JerseyReassignment>)> {
        if self.roster_only() {
            let (outcome, corrections) = self.roster_mut().autocorrect_duplicate_jerseys()?;
            if !corrections.is_empty() {
                self.record(OpTarget::Roster);
            }
            return Ok((outcome, corrections));
        }
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
        self.record_mirrored(mirror);
        Ok((outcome, corrections))
    }

    /// Repair a corrupted roster-block layout (`towx_coherence` errors) by permuting `eGlu` back
    /// into contiguous per-team blocks — an order-only structural repair that cannot change any
    /// membership (see `EditTransaction::repair_roster_blocks`). **Both files are checked and
    /// repaired independently**: each carries its own `eGlu` copy and either can be the corrupt
    /// one, and each repair is computed from that file's own bytes (an aligned identical pair
    /// yields the identical permutation, so alignment is preserved). A fully clean pair leaves
    /// both files (and the undo history) untouched.
    pub fn repair_roster_blocks(&mut self) -> Result<(OpOutcome, BlockRepairSummary)> {
        if self.roster_only() {
            let (outcome, summary) = self.roster_mut().repair_roster_blocks()?;
            if summary.moved_slots > 0 {
                self.record(OpTarget::Roster);
            }
            return Ok((outcome, summary));
        }
        let (outcome, summary) = self.nhlng.repair_roster_blocks()?;
        // The roster save is repaired from its own bytes even when the main DB was clean — the
        // corruption can live in either file. A clean roster is a no-op (no session undo entry).
        let roster_repair = match self.roster.as_mut().map(|m| m.session.repair_roster_blocks()) {
            None => None,
            Some(Ok(r)) => Some(r),
            Some(Err(e)) => {
                if summary.moved_slots > 0 {
                    self.nhlng.undo(); // keep the pair consistent
                }
                return Err(e);
            }
        };
        let roster_moved = roster_repair.as_ref().map(|(_, rs)| rs.moved_slots).unwrap_or(0);
        match (summary.moved_slots > 0, roster_moved > 0) {
            (false, false) => return Ok((outcome, summary)), // both coherent — no history entry
            (true, mirrored) => self.record_mirrored(mirrored),
            (false, true) => self.record(OpTarget::Roster),
        }
        // Merge for display: per-file permutations of the same corruption move the same rows, so
        // report the larger count (not the sum) and the union of flagged teams. When only the
        // roster moved, its outcome is the meaningful one.
        let (mut outcome, mut merged) = (outcome, summary);
        if let Some((ro, rs)) = roster_repair {
            if merged.moved_slots == 0 {
                outcome = ro;
            }
            merged.moved_slots = merged.moved_slots.max(rs.moved_slots);
            merged.affected_teams.extend(rs.affected_teams);
            merged.affected_teams.sort_unstable();
            merged.affected_teams.dedup();
        }
        Ok((outcome, merged))
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

    /// Trade a future draft pick to another team; mirrored into the roster save when `qHav`
    /// is shared and aligned.
    pub fn trade_draft_pick(&mut self, record: u32, to_team: u64) -> Result<OpOutcome> {
        self.fan_out_shared(tdb_edit::txn::draft::TABLE, move |s| {
            s.trade_draft_pick(record, to_team)
        })
    }

    /// Add a stock career season row for a player; returns the new record index. Mirrored into the
    /// roster save when the career table is shared and aligned (both files append at the same
    /// index by construction); a diverging mirror index rolls both back.
    pub fn add_career_season(
        &mut self,
        studio_table: &str,
        player: u64,
        year_end: u64,
        league: u64,
        team: &str,
    ) -> Result<(u32, OpOutcome)> {
        if self.roster_only() {
            // Roster saves usually lack the career tables — the op then fails naturally, which is
            // the honest "not editable without an aligned nhlng.db" surface.
            let (record, outcome) =
                self.roster_mut().add_career_season(studio_table, player, year_end, league, team)?;
            self.record(OpTarget::Roster);
            return Ok((record, outcome));
        }
        let (record, outcome) =
            self.nhlng
                .add_career_season(studio_table, player, year_end, league, team)?;
        let do_mirror = matches!(
            self.roster.as_ref(),
            Some(m) if m.aligned && m.shared_tables.contains(studio_table)
        );
        let roster_result = if do_mirror {
            self.roster
                .as_mut()
                .map(|m| m.session.add_career_season(studio_table, player, year_end, league, team))
        } else {
            None
        };
        let mirror = match roster_result {
            None => false,
            Some(Ok((r, _))) if r == record => true,
            Some(Ok((r, _))) => {
                // Divergent append index — the files were not as aligned as advertised; undo both.
                self.nhlng.undo();
                if let Some(m) = self.roster.as_mut() {
                    m.session.undo();
                }
                return Err(DomainError::Sync(format!(
                    "career append diverged: nhlng record {record} vs roster record {r}; both rolled back"
                )));
            }
            Some(Err(e)) => {
                self.nhlng.undo();
                return Err(e);
            }
        };
        self.record_mirrored(mirror);
        Ok((record, outcome))
    }

    /// Delete one stock career row (ordered shift), mirrored when the table is shared and aligned.
    pub fn delete_career_row(&mut self, studio_table: &str, record: u32) -> Result<OpOutcome> {
        self.fan_out_shared(studio_table, move |s| s.delete_career_row(studio_table, record))
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
        if self.roster_only() {
            let outcome = op(self.roster_mut())?;
            self.record(OpTarget::Roster);
            return Ok(outcome);
        }
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
        self.record_mirrored(mirror);
        Ok(outcome)
    }

    /// Apply an entity-id op to `nhlng`, then (if attached) the same op to the roster. The op is
    /// keyed by entity id (shared across files), so no record remapping is needed. On a mirror
    /// failure, `nhlng` is rolled back so the two stay consistent.
    fn fan_out_entity(
        &mut self,
        op: impl Fn(&mut EditSession) -> Result<OpOutcome>,
    ) -> Result<OpOutcome> {
        if self.roster_only() {
            // Entity-id keyed, so it *could* apply to both — but on a misaligned pair the files
            // already disagree, so honour the mode's contract: touch the roster file only.
            let outcome = op(self.roster_mut())?;
            self.record(OpTarget::Roster);
            return Ok(outcome);
        }
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
        self.record_mirrored(mirror);
        Ok(outcome)
    }

    // --- Structural ops (strict guard: roll both back on any divergence) ---

    /// Create a player cluster on a team. With a roster attached, creates in both and **verifies the
    /// new `wBIz` and full record alignment match** across files; any divergence rolls both back.
    pub fn create_player(&mut self, team: u64, position: u64) -> Result<(u64, OpOutcome)> {
        if self.roster_only() {
            // Single-file create in the roster save — no cross-file guard, nothing to diverge from.
            let (rid, outcome) = self.roster_mut().create_player(team, position)?;
            self.record(OpTarget::Roster);
            return Ok((rid, outcome));
        }
        let (nid, outcome) = self.nhlng.create_player(team, position)?;
        let roster_result = self.roster.as_mut().map(|m| m.session.create_player(team, position));
        match roster_result {
            None => {
                self.record_mirrored(false);
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
                self.record_mirrored(true);
                Ok((nid, outcome))
            }
        }
    }

    /// Clone a player onto a team (deep-copy cluster). Same structural dual-create + `fully_aligned`
    /// guard as [`Workspace::create_player`]: both files clone independently and must agree on the new
    /// `wBIz` and full record alignment, else both roll back.
    pub fn clone_player(&mut self, source_player: u64, to_team: u64) -> Result<(u64, OpOutcome)> {
        if self.roster_only() {
            let (rid, outcome) = self.roster_mut().clone_player(source_player, to_team)?;
            self.record(OpTarget::Roster);
            return Ok((rid, outcome));
        }
        let (nid, outcome) = self.nhlng.clone_player(source_player, to_team)?;
        let roster_result = self
            .roster
            .as_mut()
            .map(|m| m.session.clone_player(source_player, to_team));
        match roster_result {
            None => {
                self.record_mirrored(false);
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
                self.record_mirrored(true);
                Ok((nid, outcome))
            }
        }
    }

    /// Delete a player and its cluster. With a roster attached, deletes from both and **re-verifies
    /// full record alignment**; any divergence rolls both back.
    pub fn delete_player(&mut self, player: u64) -> Result<OpOutcome> {
        if self.roster_only() {
            let outcome = self.roster_mut().delete_player(player)?;
            self.record(OpTarget::Roster);
            return Ok(outcome);
        }
        let outcome = self.nhlng.delete_player(player)?;
        let roster_result = self.roster.as_mut().map(|m| m.session.delete_player(player));
        match roster_result {
            None => {
                self.record_mirrored(false);
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
                self.record_mirrored(true);
                Ok(outcome)
            }
        }
    }

    // --- Roster Builder import (Phase 5.4) ---
    //
    // Reads (match/lower/preview) run against the **canonical** `nhlng.db` — the write spine a commit
    // targets — so a preview reflects exactly what commit will do.

    /// The canonical `nhlng.db`'s on-disk path — a stable identity for the EA target (the import
    /// ledger keys on it).
    pub fn primary_path(&self) -> &str {
        &self.nhlng_path
    }

    /// Auto-detect EA identity matches for a corpus person (the cart's Match/Create toggle).
    pub fn match_ea_player(
        &self,
        corpus: &crate::historical::FhmCorpus,
        person_id: i64,
    ) -> Result<Vec<crate::import::MatchCandidate>> {
        self.nhlng.match_ea_player(corpus, person_id)
    }

    /// Lower a whole cart against the canonical db (the ops a preview/commit consume).
    pub fn lower_change_set(
        &self,
        corpus: &crate::historical::FhmCorpus,
        items: &[crate::import::ImportItem],
        profile: &tdb_formula::FormulaProfile,
        maxima: &tdb_fhm_store::AttributeMaxima,
    ) -> Result<crate::import::LoweredChangeSet> {
        self.nhlng.lower_change_set(corpus, items, profile, maxima)
    }

    /// Dry-run a lowered cart against the canonical db — non-mutating (see
    /// [`EditSession::preview_import`]).
    pub fn preview_import(
        &self,
        ops: &[tdb_edit::txn::ImportOp],
        skipped: Vec<crate::import::SkippedField>,
    ) -> Result<crate::import::ImportPreview> {
        self.nhlng.preview_import(ops, skipped)
    }

    /// Project the roster tables' fill after `creates` against the canonical db + this workspace's
    /// title (the S4 ceiling gate the preview shows). See [`crate::capacity::project_capacity`].
    pub fn roster_capacity(
        &self,
        creates: crate::capacity::PendingCreates,
    ) -> Result<Vec<crate::capacity::TableCapacity>> {
        let db = TdbFile::parse(self.nhlng.bytes())?;
        Ok(crate::capacity::project_capacity(&db, self.game_id.as_deref(), creates))
    }

    /// Commit a lowered cart for real — one transaction, one undo entry. With a roster attached, the
    /// identical ops apply to the mirror too and **full record alignment is re-verified**; any
    /// divergence rolls both files back (the [`Workspace::create_player`] guard, for a whole cart).
    pub fn commit_import(&mut self, ops: &[tdb_edit::txn::ImportOp]) -> Result<OpOutcome> {
        if self.roster_only() {
            let outcome = self.roster_mut().commit_import(ops)?;
            self.record(OpTarget::Roster);
            return Ok(outcome);
        }
        let outcome = self.nhlng.commit_import(ops)?;
        let roster_result = self.roster.as_mut().map(|m| m.session.commit_import(ops));
        match roster_result {
            None => {
                self.record_mirrored(false);
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
                        "import cart desynced the two files; both rolled back".into(),
                    ));
                }
                self.record_mirrored(true);
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
    /// When the roster save lives inside a recomp/Xenia content tree that indexes saves through
    /// `Headers/*.header` sidecars, the matching sidecar is created if missing — a save without one
    /// is invisible to the game's load list (`crate::content_header`).
    pub fn save_in_place(&self) -> Result<u64> {
        self.nhlng.save_to(&self.nhlng_path)?;
        // A roster save opened *alone* occupies the canonical slot (`Workspace::single`), so the
        // sidecar check runs on both paths; it is a no-op outside header-indexed content trees.
        crate::content_header::ensure_content_header(std::path::Path::new(&self.nhlng_path))?;
        let mut files = 1u64;
        if let Some(m) = &self.roster {
            m.session.save_to(&m.path)?;
            crate::content_header::ensure_content_header(std::path::Path::new(&m.path))?;
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
