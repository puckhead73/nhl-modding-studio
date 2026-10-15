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
    /// Set when a restore/recovery changed a file underneath a **dirty** session (S2-C): the
    /// on-disk baseline no longer matches what the session was loaded from, and the edits were kept
    /// rather than discarded. Copied-target saves refuse until the user reloads explicitly
    /// ([`Workspace::reload_from_disk`]); the reason is what they are told.
    stale_baseline: Option<StaleBaseline>,
    /// The target's own localization dictionary as attached (the baseline the LOC-W1 rename is
    /// planned against and the identity a copied-target save expects at its destination).
    dictionary: Option<crate::leagues::AttachedDictionary>,
    /// Receipt 67 (research 2026-09-28 handoff 12 §2): why the install's own dictionary could not be attached at open, so the
    /// summary says so instead of showing keys silently.
    dictionary_problem: Option<String>,
    /// The one pending dictionary rename (LOC-W1). Held beside the database edits; packaged into
    /// the same change set; adopted as the new baseline only when that save commits.
    dictionary_edit: Option<crate::dictionary_edit::DictionaryEdit>,
    /// P1-A: schedule previews held server-side, keyed by review id, each bound to the workspace
    /// revision it was computed against (see [`ScheduleReview`]). Apply consumes a review id and
    /// never recomputes a plan.
    schedule_reviews: Vec<ScheduleReview>,
    /// D3: stored-default edit previews held server-side (same ownership and revision rules as the
    /// schedule reviews; one held plan per ticket, Apply never recomputes).
    settings_reviews: Vec<SettingsReview>,
    /// D4: the NHL 16–21 `setting_csv` working set — the members as read from the install's
    /// `inxp_00.big` plus the pending stored-default edits applied on top (a stack; each entry is
    /// one review's plan, the last one holds the current text of every member it rewrote). Held
    /// beside the database edits like the dictionary rename: packaged into the same copied-target
    /// change set, adopted as the new baseline only when that save commits.
    settings_csv: Option<CsvWorkingSet>,
    /// D4: CSV stored-default previews held server-side.
    settings_csv_reviews: Vec<SettingsCsvReview>,
    /// D8 (P1-F PS4): the **effective** pending texture-member replacements inside packed
    /// archives, in first-seen order — exactly one per archive member (a later apply on the same
    /// member replaces it in place). This is what the copied-target save installs.
    asset_members: Vec<crate::asset_member_edit::AssetMemberPlan>,
    /// R1 F2: the operation log behind `asset_members`, one entry per accepted apply, newest
    /// last. Coalescing lives only in `asset_members`; undo pops the newest operation and puts
    /// back whatever it superseded, so an earlier replacement of the same member survives the
    /// undo of the later one instead of being silently discarded.
    asset_member_ops: Vec<AssetMemberOp>,
    /// D8: asset-member previews held server-side.
    asset_member_reviews: Vec<AssetMemberReview>,
    /// This workspace instance's ticket namespace: unique per constructed `Workspace` (a
    /// process-wide sequence plus the construction time), so a ticket minted by another
    /// workspace — including a replacement instance opened on the same file — can never
    /// alias one of ours (R2).
    review_namespace: String,
    next_schedule_review: u64,
}

/// A fresh, process-unique ticket namespace for one workspace instance.
fn new_review_namespace() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(1);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    format!("{nanos:x}-{seq:x}")
}

/// What a schedule preview was computed from, so Apply can refuse anything that changed since.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleOperation {
    Edit(crate::schedule_edit::GameEdit),
    Import {
        format: crate::schedule_edit::ImportFormat,
        /// The exact bytes previewed (snapshot); Apply writes from these, never from a re-read.
        text: String,
        /// SHA-256 of `text`, for a caller that re-reads the source file on Apply.
        text_sha256: String,
        /// Where the text came from, when it was a file (opaque to the domain; the API re-checks it).
        source_path: Option<String>,
        mapping: Vec<crate::schedule_edit::TeamMapping>,
    },
    Generate(crate::schedule_edit::GenerationConfig),
}

/// A server-held schedule review: the plan exactly as previewed, the table and workspace revision
/// (primary + roster + dictionary identities) it was derived from, and the operation inputs.
#[derive(Debug, Clone)]
pub struct ScheduleReview {
    /// Opaque ticket: `<workspace namespace>/<sequence>`; carried losslessly as a string.
    pub ticket: String,
    pub studio_table: String,
    pub revision: WorkspaceRevision,
    pub operation: ScheduleOperation,
    pub plan: crate::schedule_edit::ChangePlan,
    pub import: Option<crate::schedule_edit::ImportPlan>,
    pub generation: Option<crate::schedule_edit::GenerationPlan>,
}

/// A server-held stored-default edit review (D3): the plan exactly as previewed and the workspace
/// revision it was derived from. Ticket namespace as [`ScheduleReview`].
#[derive(Debug, Clone)]
pub struct SettingsReview {
    pub ticket: String,
    pub revision: WorkspaceRevision,
    pub plan: crate::settings_edit::DefaultEditPlan,
}

/// A server-held NHL 16–21 CSV stored-default review (D4): the plan (member rewrites included)
/// and the workspace revision it was derived from.
#[derive(Debug, Clone)]
pub struct SettingsCsvReview {
    pub ticket: String,
    pub revision: WorkspaceRevision,
    pub plan: crate::settings_csv_edit::CsvDefaultPlan,
}

/// A server-held asset-member replacement review (D8): the plan and the revision it was derived from.
#[derive(Debug, Clone)]
pub struct AssetMemberReview {
    pub ticket: String,
    pub revision: WorkspaceRevision,
    pub plan: crate::asset_member_edit::AssetMemberPlan,
}

/// One accepted asset-member apply (R1 F2): where its plan sits in the effective list and the
/// plan on the same member it superseded there, if any (`None` when the member was not pending).
#[derive(Debug, Clone)]
struct AssetMemberOp {
    position: usize,
    superseded: Option<crate::asset_member_edit::AssetMemberPlan>,
}

/// The NHL 16–21 catalogue members of one archive with the pending edits on top.
#[derive(Debug, Clone)]
pub struct CsvWorkingSet {
    /// The archive the members were read from (`<install root>/databigs/inxp_00.big`).
    pub archive_path: std::path::PathBuf,
    pub slots: crate::settings_csv_edit::ArchiveSlots,
    /// `(member path, stored SHA-256, stored length)` of every catalogue entry as read.
    pub stored_identities: Vec<(String, String, usize)>,
    pub baseline: crate::settings_catalogue::SettingCsvMembers,
    /// Pending edits, oldest first; the working text is the baseline with every plan applied.
    pub edits: Vec<crate::settings_csv_edit::CsvDefaultPlan>,
}

impl CsvWorkingSet {
    /// The members as they read now (baseline + every pending edit).
    pub fn working(&self) -> crate::settings_catalogue::SettingCsvMembers {
        let mut m = self.baseline.clone();
        for e in &self.edits {
            m = crate::settings_csv_edit::apply_to_members(&m, e);
        }
        m
    }

    /// A stable identity of the working text (member name + plain SHA-256 per member).
    pub fn working_identity(&self) -> String {
        use sha2::Digest;
        let w = self.working();
        let mut h = sha2::Sha256::new();
        for (name, (bytes, _)) in &w.members {
            h.update(name.as_bytes());
            h.update(sha2::Sha256::digest(bytes));
        }
        format!("{:x}", h.finalize())
    }

    /// The member replacements a copied-target save must install: for every member any pending
    /// edit rewrote, the latest new stored bytes with the archive entry's expected identity.
    pub fn replacements(&self) -> Vec<crate::settings_csv_edit::CsvMemberEdit> {
        let mut out: Vec<crate::settings_csv_edit::CsvMemberEdit> = Vec::new();
        for e in &self.edits {
            for m in &e.members {
                if let Some(existing) = out.iter_mut().find(|x| x.member_path == m.member_path) {
                    let mut merged = m.clone();
                    merged.source_plain_sha256 = existing.source_plain_sha256.clone();
                    merged.source_plain_len = existing.source_plain_len;
                    merged.source_stored_sha256 = existing.source_stored_sha256.clone();
                    merged.source_stored_len = existing.source_stored_len;
                    let mut cells = existing.cells.clone();
                    cells.extend(m.cells.iter().cloned());
                    merged.cells = cells;
                    *existing = merged;
                } else {
                    out.push(m.clone());
                }
            }
        }
        out
    }
}

/// What invalidated the workspace baseline (which deliberate action clears it again).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaleOrigin {
    /// The main database and/or the roster save: only an explicit reload clears it.
    Databases,
    /// The attached dictionary only: a reload, a re-attach or a detach clears it.
    Dictionary,
    /// A split in-place publication (R1 F1): only an explicit reload clears it.
    Split,
}

/// The stale-baseline state: why copied-target and in-place saves are refused right now.
#[derive(Debug, Clone)]
struct StaleBaseline {
    origin: StaleOrigin,
    reason: String,
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
            stale_baseline: None,
            dictionary: None,
            dictionary_problem: None,
            dictionary_edit: None,
            schedule_reviews: Vec::new(),
            settings_reviews: Vec::new(),
            settings_csv: None,
            settings_csv_reviews: Vec::new(),
            asset_members: Vec::new(),
            asset_member_ops: Vec::new(),
            asset_member_reviews: Vec::new(),
            review_namespace: new_review_namespace(),
            next_schedule_review: 1,
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
            stale_baseline: None,
            dictionary: None,
            dictionary_problem: None,
            dictionary_edit: None,
            schedule_reviews: Vec::new(),
            settings_reviews: Vec::new(),
            settings_csv: None,
            settings_csv_reviews: Vec::new(),
            asset_members: Vec::new(),
            asset_member_ops: Vec::new(),
            asset_member_reviews: Vec::new(),
            review_namespace: new_review_namespace(),
            next_schedule_review: 1,
        })
    }

    /// Attach the target's localization dictionary (parsed and CRC-verified) to every session so
    /// league labels resolve through it. The caller supplies the file discovery found in the same
    /// install, never a neighbouring title's.
    pub fn attach_dictionary(&mut self, path: &str) -> Result<crate::leagues::DictionaryRef> {
        let d = crate::leagues::AttachedDictionary::load(path)?;
        let r = crate::leagues::DictionaryRef {
            path: d.path.clone(),
            locale: d.locale.clone(),
            sha256: d.sha256.clone(),
            rows: d.dictionary.len(),
        };
        // A (re)attach is a new baseline: any pending rename planned on other bytes is dropped,
        // and a stale reason that was about the previous dictionary bytes no longer applies.
        self.dictionary_edit = None;
        self.set_session_dictionary(Some(d.clone()));
        self.dictionary = Some(d);
        self.clear_dictionary_staleness();
        Ok(r)
    }

    fn set_session_dictionary(&mut self, d: Option<crate::leagues::AttachedDictionary>) {
        if let Some(m) = self.roster.as_mut() {
            m.session.set_dictionary(d.clone());
        }
        self.nhlng.set_dictionary(d);
    }

    /// Attach the install's dictionary at open, recording (rather than dropping) a failure (receipt 67).
    pub fn attach_dictionary_at_open(&mut self, path: &str) {
        self.dictionary_problem = match self.attach_dictionary(path) {
            Ok(_) => None,
            Err(e) => Some(format!("the install's localization dictionary {path} could not be attached ({e}); names that come from it show as keys")),
        };
    }

    /// Why the install's dictionary could not be attached at open, if it could not.
    pub fn dictionary_problem(&self) -> Option<&str> {
        self.dictionary_problem.as_deref()
    }

    /// The attached dictionary's baseline (as loaded from disk), if any.
    pub fn dictionary(&self) -> Option<&crate::leagues::AttachedDictionary> {
        self.dictionary.as_ref()
    }

    /// The pending LOC-W1 rename, if any.
    pub fn dictionary_edit(&self) -> Option<&crate::dictionary_edit::DictionaryEdit> {
        self.dictionary_edit.as_ref()
    }

    /// The dictionary bytes a save would write: the pending rename's output, else the baseline.
    pub fn dictionary_working_identity(&self) -> Option<tdb_txn::FileIdentity> {
        match (&self.dictionary_edit, &self.dictionary) {
            (Some(e), _) => Some(e.output_identity.clone()),
            (None, Some(d)) => Some(tdb_txn::FileIdentity {
                len: d.bytes.len() as u64,
                sha256: d.sha256.clone(),
            }),
            (None, None) => None,
        }
    }

    /// Handoff 09 §4: C1 controls are refused on input (every new / grown text of the ops).
    fn refuse_growth_controls(&self, ops: &[tdb_loc::GrowthOp]) -> Result<()> {
        crate::font_glyphs::refuse_input_controls(&growth_texts(ops))
    }

    fn require_dictionary(&self) -> Result<&crate::leagues::AttachedDictionary> {
        self.dictionary.as_ref().ok_or_else(|| {
            DomainError::Validation(
                "no dictionary is attached to this project; attach the title's own localization file first".into(),
            )
        })
    }

    /// Preview (read-only) the rename of `key`'s text to `donor`'s text on the attached
    /// dictionary's **baseline** bytes. One rename is held at a time: the preview is always
    /// against the baseline, never chained on a pending edit.
    pub fn dictionary_repoint_preview(
        &self,
        key: &str,
        donor: &str,
    ) -> Result<crate::dictionary_edit::DictionaryRepointPreview> {
        let mut p = crate::dictionary_edit::preview(self.require_dictionary()?, key, donor)?;
        p.warnings.extend(self.dictionary_key_consumers(key));
        Ok(p)
    }

    /// Phase 1.3: the known Studio-side consumers of a dictionary key — leagues whose code binds
    /// to it and competition groups (conferences/divisions) that name it — so a rename preview
    /// says what else changes. Only the consumers Studio has mapped; screens are runtime evidence.
    fn dictionary_key_consumers(&self, key: &str) -> Vec<String> {
        let mut out = Vec::new();
        let Ok(cat) = self.display().league_catalog() else {
            return out;
        };
        let leagues: Vec<String> = cat
            .leagues
            .iter()
            .filter(|l| {
                l.dictionary_key.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(key))
                    || l.name_key.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(key))
            })
            .map(|l| format!("league code {} ({})", l.code, l.display()))
            .collect();
        if leagues.len() > 1 {
            out.push(format!(
                "this key labels {} leagues on this title — all of them change: {}",
                leagues.len(),
                leagues.join(", ")
            ));
        }
        if let Some(c) = &cat.competition {
            let groups: Vec<String> = c
                .all()
                .filter(|g| g.key.eq_ignore_ascii_case(key))
                .map(|g| {
                    let league = g
                        .league_code
                        .and_then(|lc| c.league_group(lc))
                        .map(|l| l.label.clone())
                        .unwrap_or_else(|| "?".into());
                    format!("{} {} of {league}", g.kind, g.label)
                })
                .collect();
            if !groups.is_empty() {
                out.push(format!(
                    "this key also names {} competition group{} in this title's table ({}): {}",
                    groups.len(),
                    if groups.len() == 1 { "" } else { "s" },
                    c.evidence,
                    groups.join("; ")
                ));
            }
        }
        out
    }

    /// Plan, apply in memory and hold the rename as the workspace's pending dictionary change,
    /// replacing any previous one. Labels resolve through the edited dictionary from now on; the
    /// file on disk is untouched until a copied-target save commits or an in-place save writes it.
    pub fn set_dictionary_repoint(
        &mut self,
        key: &str,
        donor: &str,
    ) -> Result<crate::dictionary_edit::DictionaryRepointPreview> {
        let baseline = self.require_dictionary()?.clone();
        let edit = crate::dictionary_edit::apply(&baseline, key, donor)?;
        let working = crate::leagues::AttachedDictionary::from_bytes(
            &baseline.path,
            edit.output.as_ref().clone(),
        )?;
        let mut preview = edit.preview.clone();
        preview.warnings.extend(self.dictionary_key_consumers(key));
        self.set_session_dictionary(Some(working));
        self.dictionary_edit = Some(edit);
        Ok(preview)
    }

    /// C5 glyph gate, static half (research 2026-09-21 handoff 09): the verdict line for one new
    /// text on this title — every character in the shipped fonts, the missing ones named, or
    /// "unmeasured" / "no tree". `None` when the workspace has no game id (nothing to check
    /// against; the runtime gate line stays).
    fn glyph_gate_warning(&self, text: &str) -> Option<String> {
        let game = self.game_id()?;
        let platform = self.nhlng.project().platform;
        Some(self.nhlng.project().glyph_coverage().check(game, platform, text).warning())
    }

    /// LOC-W2: preview (read-only) the in-place re-encode of `key`'s text to `text` on the
    /// attached dictionary's baseline bytes.
    pub fn dictionary_retext_preview(
        &self,
        key: &str,
        text: &str,
    ) -> Result<crate::dictionary_edit::DictionaryRetextPreview> {
        crate::font_glyphs::refuse_input_controls(text)?;
        let mut p = crate::dictionary_edit::preview_retext(self.require_dictionary()?, key, text)?;
        p.warnings.extend(self.dictionary_key_consumers(key));
        p.warnings.extend(self.glyph_gate_warning(text));
        Ok(p)
    }

    /// LOC-W2: plan, apply in memory and hold the in-place re-encode as the workspace's pending
    /// dictionary change (replacing any previous one).
    pub fn set_dictionary_retext(
        &mut self,
        key: &str,
        text: &str,
    ) -> Result<crate::dictionary_edit::DictionaryRetextPreview> {
        crate::font_glyphs::refuse_input_controls(text)?;
        let baseline = self.require_dictionary()?.clone();
        let edit = crate::dictionary_edit::apply_retext(&baseline, key, text)?;
        let working = crate::leagues::AttachedDictionary::from_bytes(
            &baseline.path,
            edit.output.as_ref().clone(),
        )?;
        let mut preview = edit.retext_preview.clone().expect("retext edit carries its preview");
        preview.warnings.extend(self.dictionary_key_consumers(key));
        preview.warnings.extend(self.glyph_gate_warning(text));
        self.set_session_dictionary(Some(working));
        self.dictionary_edit = Some(edit);
        Ok(preview)
    }

    /// LOC-W3: preview (read-only) a growth of the attached dictionary's **baseline** bytes.
    pub fn dictionary_growth_preview(&self, ops: &[tdb_loc::GrowthOp]) -> Result<crate::dictionary_edit::DictionaryGrowthPreview> {
        self.dictionary_growth_preview_with(ops, crate::dictionary_edit::GrowthSerializer::Preserving)
    }

    /// [`Workspace::dictionary_growth_preview`] with an explicit serializer (the canonical rebuild
    /// is never chosen silently).
    pub fn dictionary_growth_preview_with(&self, ops: &[tdb_loc::GrowthOp], how: crate::dictionary_edit::GrowthSerializer) -> Result<crate::dictionary_edit::DictionaryGrowthPreview> {
        self.refuse_growth_controls(ops)?;
        let mut p = crate::dictionary_edit::preview_growth_with(self.require_dictionary()?, ops, how)?;
        for op in ops {
            if let tdb_loc::GrowthOp::TextGrow { key, .. } | tdb_loc::GrowthOp::SetNull { key } = op {
                p.warnings.extend(self.dictionary_key_consumers(key));
            }
        }
        p.warnings.extend(self.glyph_gate_warning(&growth_texts(ops)));
        Ok(p)
    }

    /// LOC-W3: hold a growth as the pending dictionary change (one pending operation at a time,
    /// planned against the baseline).
    pub fn set_dictionary_growth(&mut self, ops: &[tdb_loc::GrowthOp]) -> Result<crate::dictionary_edit::DictionaryGrowthPreview> {
        self.set_dictionary_growth_with(ops, crate::dictionary_edit::GrowthSerializer::Preserving)
    }

    /// [`Workspace::set_dictionary_growth`] with an explicit serializer.
    pub fn set_dictionary_growth_with(&mut self, ops: &[tdb_loc::GrowthOp], how: crate::dictionary_edit::GrowthSerializer) -> Result<crate::dictionary_edit::DictionaryGrowthPreview> {
        self.refuse_growth_controls(ops)?;
        let baseline = self.require_dictionary()?.clone();
        let (edit, mut preview) = crate::dictionary_edit::apply_growth_with(&baseline, ops, how)?;
        let working = crate::leagues::AttachedDictionary::from_bytes(&baseline.path, edit.output.as_ref().clone())?;
        for op in ops {
            if let tdb_loc::GrowthOp::TextGrow { key, .. } | tdb_loc::GrowthOp::SetNull { key } = op {
                preview.warnings.extend(self.dictionary_key_consumers(key));
            }
        }
        preview.warnings.extend(self.glyph_gate_warning(&growth_texts(ops)));
        self.set_session_dictionary(Some(working));
        self.dictionary_edit = Some(edit);
        Ok(preview)
    }

    /// Drop the pending rename (the undo of [`Workspace::set_dictionary_repoint`]); labels resolve
    /// through the baseline again. `false` when there was nothing pending.
    pub fn clear_dictionary_repoint(&mut self) -> bool {
        if self.dictionary_edit.take().is_none() {
            return false;
        }
        let d = self.dictionary.clone();
        self.set_session_dictionary(d);
        true
    }

    /// The pending-review view of the dictionary rename, when one is held.
    pub fn pending_dictionary(&self) -> Option<crate::dictionary_edit::PendingDictionary> {
        match (&self.dictionary_edit, &self.dictionary) {
            (Some(e), Some(d)) => Some(e.pending(d)),
            _ => None,
        }
    }

    /// Tag the workspace with the studio game id it was opened as (onboarding path).
    pub fn set_game_id(&mut self, game_id: impl Into<String>) {
        let g = game_id.into();
        self.nhlng.set_game_id(Some(g.clone()));
        if let Some(m) = self.roster.as_mut() {
            m.session.set_game_id(Some(g.clone()));
        }
        self.game_id = Some(g);
    }

    // --- Status (for the project summary DTO) ---

    /// The studio game id (`nhl10`…`nhllegacy`), when known.
    pub fn game_id(&self) -> Option<&str> {
        self.game_id.as_deref()
    }

    /// Save Editor: the read-only tables supplied from the game's database, when supplemented.
    pub fn frozen_tables(&self) -> Option<&std::collections::BTreeSet<String>> {
        self.nhlng.project().frozen_tables()
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
        &mut self
            .roster
            .as_mut()
            .expect("roster_only implies an attached mirror")
            .session
    }

    pub fn roster_attached(&self) -> bool {
        self.roster.is_some()
    }

    /// The attached roster mirror's session, if any (read-only; writes fan out through the
    /// workspace's own operations).
    pub fn roster_session(&self) -> Option<&EditSession> {
        self.roster.as_ref().map(|m| &m.session)
    }

    /// Test seam: make the attached roster session refuse every transaction (see
    /// [`EditSession::inject_fault`]). `false` when no roster is attached.
    #[doc(hidden)]
    pub fn inject_roster_fault(&mut self, reason: Option<String>) -> bool {
        match self.roster.as_mut() {
            Some(m) => {
                m.session.inject_fault(reason);
                true
            }
            None => false,
        }
    }

    /// The attached roster save's on-disk path, if any.
    pub fn roster_path(&self) -> Option<&str> {
        self.roster.as_ref().map(|m| m.path.as_str())
    }

    // --- Baseline lifecycle (S2-C copied-target saves) ---

    /// The workspace's content revision: the identity of every session's current working bytes.
    /// A prepared copied-target change set is bound to this; any edit, undo or redo that changes
    /// the bytes (and nothing that leaves them identical) changes it, so an apply against a
    /// different revision is refused and the user re-prepares.
    pub fn revision(&self) -> WorkspaceRevision {
        WorkspaceRevision {
            primary: tdb_txn::FileIdentity::of_bytes(self.nhlng.bytes()),
            roster: self
                .roster
                .as_ref()
                .map(|m| tdb_txn::FileIdentity::of_bytes(m.session.bytes())),
            dictionary: self.dictionary_working_identity(),
            text_members: self.settings_csv.as_ref().filter(|w| !w.edits.is_empty()).map(|w| w.working_identity()),
            asset_members: self.asset_members_identity(),
        }
    }

    /// A stable identity of the pending asset-member replacements (`None` when none).
    fn asset_members_identity(&self) -> Option<String> {
        use sha2::Digest;
        if self.asset_members.is_empty() {
            return None;
        }
        let mut h = sha2::Sha256::new();
        for m in &self.asset_members {
            h.update(m.archive_relative.as_bytes());
            h.update(m.member_path.as_bytes());
            h.update(m.member.new_stored_sha256.as_bytes());
        }
        Some(format!("{:x}", h.finalize()))
    }

    /// Why copied-target saves are refused right now, if a restore/recovery invalidated the
    /// baseline under unsaved edits (see the field).
    pub fn stale_baseline(&self) -> Option<&str> {
        self.stale_baseline.as_ref().map(|s| s.reason.as_str())
    }

    fn mark_stale(&mut self, origin: StaleOrigin, reason: String) {
        self.stale_baseline = Some(StaleBaseline { origin, reason });
    }

    /// Clear a stale-baseline reason that was about the dictionary only (an attach or detach
    /// replaces that baseline deliberately); a reason about the databases stays.
    fn clear_dictionary_staleness(&mut self) {
        if self
            .stale_baseline
            .as_ref()
            .is_some_and(|s| s.origin == StaleOrigin::Dictionary)
        {
            self.stale_baseline = None;
        }
    }

    /// After a copied-target transaction **committed** this workspace's packaged sessions: adopt
    /// the installed files as the new baselines (`is_dirty()` becomes false; the next save's
    /// expected prior is the installed identity). Each session verifies the file it is told it was
    /// saved to ([`EditSession::rebind_saved_baseline`]); a session that fails to verify keeps its
    /// edits and its old baseline and the error is returned — the transaction itself is already
    /// committed and is reported by the caller as such.
    pub fn rebind_after_copied_save(
        &mut self,
        primary: Option<&tdb_txn::FileIdentity>,
        roster: Option<&tdb_txn::FileIdentity>,
        dictionary: Option<&tdb_txn::FileIdentity>,
    ) -> Result<()> {
        if let Some(id) = primary {
            self.nhlng
                .rebind_saved_baseline(std::path::Path::new(&self.nhlng_path), id)?;
        }
        if let (Some(id), Some(m)) = (roster, self.roster.as_mut()) {
            m.session
                .rebind_saved_baseline(std::path::Path::new(&m.path), id)?;
        }
        if let (Some(id), Some(edit), Some(base)) =
            (dictionary, self.dictionary_edit.as_ref(), self.dictionary.as_ref())
        {
            // The installed file must be exactly the rename's output and must be on disk as such;
            // only then does the output become the baseline and the pending rename close.
            if *id != edit.output_identity {
                return Err(DomainError::Validation(format!(
                    "the installed dictionary {} is {id}, not the pending rename's output {}",
                    base.path, edit.output_identity
                )));
            }
            let on_disk = tdb_txn::FileIdentity::of_file(std::path::Path::new(&base.path))
                .map_err(|e| DomainError::Sync(format!("{}: {e}", base.path)))?;
            if on_disk != *id {
                return Err(DomainError::Validation(format!(
                    "the dictionary at {} is {on_disk} on disk, not the installed {id}",
                    base.path
                )));
            }
            let adopted = crate::leagues::AttachedDictionary::from_bytes(
                &base.path,
                edit.output.as_ref().clone(),
            )?;
            self.set_session_dictionary(Some(adopted.clone()));
            self.dictionary = Some(adopted);
            self.dictionary_edit = None;
        }
        self.stale_baseline = None;
        Ok(())
    }

    /// After a restore or recovery may have changed this workspace's files on disk: for every
    /// session whose file no longer carries the loaded identity, **reload** it when the session is
    /// clean, or **keep the edits and mark the baseline stale** when it is dirty. Never discards an
    /// edit.
    ///
    /// Two-phase: every decision and every reload is **prepared** first (files read and parsed,
    /// the mirror alignment re-derived on the prepared bytes) and only then adopted, so an error
    /// leaves the workspace exactly as it was — no session half-refreshed. On error the workspace
    /// is marked unusable for copied-target saves ([`Workspace::stale_baseline`]) with the reason,
    /// until an explicit reload succeeds; the edits are kept.
    pub fn refresh_baselines(&mut self) -> Result<BaselineRefresh> {
        match self.prepare_refresh() {
            Ok(prepared) => Ok(self.adopt_refresh(prepared)),
            Err(e) => {
                self.mark_stale(StaleOrigin::Databases, format!(
                    "the workspace's files could not be re-read after a restore or recovery ({e}); the unsaved edits are kept. Put the file back and reload the workspace before saving again"
                ));
                Err(e)
            }
        }
    }

    fn prepare_refresh(&self) -> Result<PreparedRefresh> {
        type Decision = (BaselineAction, Option<crate::session::PreparedReload>);
        let decide = |session: &EditSession, path: &str| -> Result<Decision> {
            let p = std::path::Path::new(path);
            match session.baseline_matches_disk(p)? {
                None => Ok((BaselineAction::NoBaseline, None)),
                Some(true) => Ok((BaselineAction::Unchanged, None)),
                Some(false) if session.is_dirty() => Ok((BaselineAction::StaleKeptEdits, None)),
                Some(false) => Ok((BaselineAction::Reloaded, Some(session.prepare_reload(p)?))),
            }
        };
        let (primary, primary_reload) = decide(&self.nhlng, &self.nhlng_path)?;
        let (roster, roster_reload, mirror) = match &self.roster {
            None => (None, None, None),
            Some(m) => {
                let (action, reload) = decide(&m.session, &m.path)?;
                // Alignment is re-derived on the bytes the workspace will hold afterwards.
                let mirror = if primary_reload.is_some() || reload.is_some() {
                    let nb = primary_reload
                        .as_ref()
                        .map(|r| r.inner())
                        .unwrap_or(self.nhlng.bytes());
                    let rb = reload
                        .as_ref()
                        .map(|r| r.inner())
                        .unwrap_or(m.session.bytes());
                    Some(analyze_bytes(nb, rb)?)
                } else {
                    None
                };
                (Some(action), reload, mirror)
            }
        };
        Ok(PreparedRefresh {
            primary,
            primary_reload,
            roster,
            roster_reload,
            mirror,
        })
    }

    fn adopt_refresh(&mut self, prepared: PreparedRefresh) -> BaselineRefresh {
        let PreparedRefresh {
            primary,
            primary_reload,
            roster,
            roster_reload,
            mirror,
        } = prepared;
        let mut reloaded = false;
        if let Some(r) = primary_reload {
            self.nhlng.adopt_reload(r);
            reloaded = true;
        }
        if let Some(m) = self.roster.as_mut() {
            if let Some(r) = roster_reload {
                m.session.adopt_reload(r);
                reloaded = true;
            }
            if let Some((shared_tables, aligned)) = mirror {
                m.shared_tables = shared_tables;
                m.aligned = aligned;
            }
        }
        let stale: Vec<&str> = [
            (primary == BaselineAction::StaleKeptEdits).then_some("the main database"),
            (roster == Some(BaselineAction::StaleKeptEdits)).then_some("the roster save"),
        ]
        .into_iter()
        .flatten()
        .collect();
        if stale.is_empty() {
            if reloaded {
                // A reload replaced the working copies; the coordinated history no longer applies.
                self.history.clear();
                self.redo.clear();
            }
            self.stale_baseline = None;
        } else {
            self.mark_stale(StaleOrigin::Databases, format!(
                "{} changed on disk (a restore or recovery) while this workspace had unsaved edits; the edits were kept. Reload the workspace to continue saving, or export the edits first",
                stale.join(" and ")
            ));
        }
        self.refresh_dictionary_baseline();
        BaselineRefresh { primary, roster }
    }

    /// The dictionary's share of a baseline refresh: when the attached file no longer carries the
    /// attached identity (a restore or recovery put other bytes there), reload it from disk if no
    /// rename is pending, or keep the pending rename and mark the baseline stale. An unreadable
    /// file also marks the baseline stale; nothing is discarded.
    fn refresh_dictionary_baseline(&mut self) {
        let Some(base) = self.dictionary.as_ref() else {
            return;
        };
        let path = base.path.clone();
        let expected = tdb_txn::FileIdentity {
            len: base.bytes.len() as u64,
            sha256: base.sha256.clone(),
        };
        match tdb_txn::FileIdentity::of_file(std::path::Path::new(&path)) {
            Ok(current) if current == expected => {}
            Ok(_) if self.dictionary_edit.is_none() => {
                match crate::leagues::AttachedDictionary::load(&path) {
                    Ok(d) => {
                        self.set_session_dictionary(Some(d.clone()));
                        self.dictionary = Some(d);
                    }
                    Err(e) => {
                        self.mark_stale(StaleOrigin::Dictionary, format!(
                            "the dictionary {path} changed on disk and could not be re-read ({e}); re-attach it before saving"
                        ));
                    }
                }
            }
            Ok(_) => {
                self.mark_stale(StaleOrigin::Dictionary, format!(
                    "the dictionary {path} changed on disk (a restore or recovery) while a rename was pending; the rename was kept. Re-attach the dictionary (which drops the rename) before saving again"
                ));
            }
            Err(e) => {
                self.mark_stale(StaleOrigin::Dictionary, format!(
                    "the dictionary {path} could not be re-read after a restore or recovery ({e}); re-attach it before saving"
                ));
            }
        }
    }

    /// Explicitly reload every session from disk, discarding the working copies and the history.
    /// The caller gates this on the user's decision when the workspace is dirty; here it is
    /// unconditional. Two-phase like [`Workspace::refresh_baselines`]: an unreadable file leaves
    /// the workspace (and its edits) exactly as they were, with the stale-baseline reason updated.
    pub fn reload_from_disk(&mut self) -> Result<()> {
        // Phase one: prepare every file — database, roster save **and** the attached dictionary
        // (R1 F2) — without changing the workspace. A missing, unreadable or CRC-invalid dictionary
        // fails the reload here, before anything is adopted, exactly like an unreadable database.
        let prepared = (|| -> std::result::Result<_, (StaleOrigin, DomainError)> {
            let db = |e| (StaleOrigin::Databases, e);
            let primary = self
                .nhlng
                .prepare_reload(std::path::Path::new(&self.nhlng_path))
                .map_err(db)?;
            let roster = match &self.roster {
                None => None,
                Some(m) => {
                    let r = m
                        .session
                        .prepare_reload(std::path::Path::new(&m.path))
                        .map_err(db)?;
                    let mirror = analyze_bytes(primary.inner(), r.inner()).map_err(db)?;
                    Some((r, mirror))
                }
            };
            let dictionary = match &self.dictionary {
                None => None,
                Some(d) => Some(crate::leagues::AttachedDictionary::load(&d.path).map_err(
                    |e| {
                        (StaleOrigin::Dictionary, DomainError::Sync(format!(
                            "the attached dictionary {} could not be reloaded: {e}. The workspace keeps its edits, the attached dictionary and the pending rename; put the file back and reload again, or detach the dictionary deliberately",
                            d.path
                        )))
                    },
                )?),
            };
            Ok((primary, roster, dictionary))
        })();
        let (primary, roster, dictionary) = match prepared {
            Ok(p) => p,
            Err((origin, e)) => {
                self.mark_stale(origin, format!(
                    "the workspace could not be reloaded from disk ({e}); the unsaved edits are kept. Put the file back and reload again before saving"
                ));
                return Err(e);
            }
        };
        // Phase two: adopt everything. The dictionary reloads from its path too, dropping any
        // pending rename with the rest of the working state (a reload is the user's explicit
        // choice to take what is on disk).
        self.nhlng.adopt_reload(primary);
        if let (Some(m), Some((r, (shared_tables, aligned)))) = (self.roster.as_mut(), roster) {
            m.session.adopt_reload(r);
            m.shared_tables = shared_tables;
            m.aligned = aligned;
        }
        self.history.clear();
        self.redo.clear();
        self.stale_baseline = None;
        // The CSV working set is re-read from the archive on the next preview/catalogue read.
        self.settings_csv = None;
        self.settings_csv_reviews.clear();
        self.asset_members.clear();
        self.asset_member_ops.clear();
        self.asset_member_reviews.clear();
        if let Some(d) = dictionary {
            self.dictionary_edit = None;
            self.set_session_dictionary(Some(d.clone()));
            self.dictionary = Some(d);
        }
        Ok(())
    }

    /// Deliberately drop the attached dictionary and any pending rename planned on it (R1 F2:
    /// detaching is an intentional action, never a side effect of a failed reload). Labels resolve
    /// through the vocabularies alone again. `false` when nothing was attached.
    pub fn detach_dictionary(&mut self) -> bool {
        if self.dictionary.is_none() {
            return false;
        }
        self.dictionary_edit = None;
        self.set_session_dictionary(None);
        self.dictionary = None;
        // A stale-baseline reason that was about the dictionary no longer applies; one about the
        // databases still does.
        self.clear_dictionary_staleness();
        true
    }

    /// `true` when a roster is attached **and** the two files are record-index-aligned (so generic
    /// field edits mirror). A `false` here with `roster_attached() == true` means degraded
    /// (`nhlng`-only for field edits).
    pub fn roster_aligned(&self) -> bool {
        self.roster.as_ref().map(|m| m.aligned).unwrap_or(false)
    }

    pub fn is_dirty(&self) -> bool {
        self.nhlng.is_dirty()
            || self
                .roster
                .as_ref()
                .map(|m| m.session.is_dirty())
                .unwrap_or(false)
            || self.dictionary_edit.is_some()
            || self.settings_csv.as_ref().is_some_and(|w| !w.edits.is_empty())
            || !self.asset_members.is_empty()
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
        self.record(if mirrored {
            OpTarget::Both
        } else {
            OpTarget::Canonical
        });
    }

    /// Roll back the canonical half of a coordinated op whose roster half was refused (or
    /// diverged): the session entry is discarded (not undone — an undone entry would be redoable
    /// without its mirror), and because the session's own `run` already cleared its redo stack,
    /// the workspace redo stack is cleared with it so the two never disagree about what a Redo
    /// would re-apply. History is untouched: the op never happened. P1 audit, 21 Sep 2026.
    fn abort_canonical(&mut self) {
        self.nhlng.abort_last();
        self.redo.clear();
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
                    OpTarget::Roster => self
                        .roster
                        .as_mut()
                        .map(|m| m.session.undo())
                        .unwrap_or(false),
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
                    OpTarget::Roster => self
                        .roster
                        .as_mut()
                        .map(|m| m.session.redo())
                        .unwrap_or(false),
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
        let consts = self
            .game_id
            .as_deref()
            .and_then(crate::capacity::gen7_ceilings);
        let mut grown = Vec::new();
        for (i, cid) in crate::capacity::ROSTER_TABLES.iter().enumerate() {
            let main_max = main
                .table_canonical(cid)
                .map(|t| t.max_records)
                .unwrap_or(0);
            let ceiling = consts.map(|c| c[i]).unwrap_or(0).max(main_max);
            if ceiling == 0 {
                continue; // no ceiling source for this table — leave it alone
            }
            if let Some((from, to)) = m.session.grow_table_max(cid, ceiling)? {
                grown.push(crate::capacity::CapacityGrowth {
                    table: cid.to_string(),
                    from,
                    to,
                });
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

    /// Package installation opens each destination independently. Never fan these explicit
    /// per-file edits into another save or database.
    pub fn set_package_free_agents(&mut self, players: &[u64]) -> Result<OpOutcome> {
        if self.roster.is_some() {
            return Err(DomainError::Validation("package replay requires a single-file workspace".into()));
        }
        let outcome = self.nhlng.set_free_agent_pool(players)?;
        self.record_mirrored(false);
        Ok(outcome)
    }

    /// A project-overview revert is one undoable operation, including dependent roster cells.
    pub fn set_project_revert(&mut self, cells: &[(String, u32, String, FieldValue)], pool: Option<&[u64]>) -> Result<OpOutcome> {
        if self.roster.is_some() { return Err(DomainError::Validation("Overview reverts require a default-database project.".into())); }
        let outcome = self.nhlng.set_project_revert(cells, pool)?;
        self.record_mirrored(false);
        Ok(outcome)
    }

    pub fn replace_schedules(&mut self, schedules: &[(String, Vec<crate::schedule_edit::GameRow>)]) -> Result<OpOutcome> {
        if self.roster.is_some() { return Err(DomainError::Validation("Edit source schedules in a default-database project.".into())); }
        let outcome = self.nhlng.replace_schedules(schedules)?;
        self.record_mirrored(false);
        Ok(outcome)
    }

    pub fn set_package_fields(&mut self, cells: &[(String, u32, String, FieldValue)]) -> Result<OpOutcome> {
        if self.roster.is_some() {
            return Err(DomainError::Validation("package replay requires a single-file workspace".into()));
        }
        let outcome = self.nhlng.set_package_fields(cells)?;
        self.record_mirrored(false);
        Ok(outcome)
    }

    pub fn set_field(
        &mut self,
        studio_table: &str,
        record: u32,
        studio_field: &str,
        value: FieldValue,
    ) -> Result<OpOutcome> {
        // The player page's team and number controls are relational edits on NHL07.
        // Changing only the biography would leave the actual roster slot unchanged.
        if self.game_id.as_deref() == Some("nhl07") && self.roster.is_none()
            && studio_table == "ubPc" && matches!(studio_field, "dXSB" | "sVRt") {
            let FieldValue::Int(v) = &value else { return Err(DomainError::Validation("Choose a valid team or jersey number".into())); };
            if *v < 0 { return Err(DomainError::Validation("Choose a valid team or jersey number".into())); }
            let model = self.nhlng.model()?;
            let player = model.players.iter().find(|p| p.bio_record == record)
                .ok_or_else(|| DomainError::Validation("Player record was not found".into()))?.player_id;
            let view = self.nhlng.player_view(player)?.ok_or_else(|| DomainError::Validation("Player was not found".into()))?;
            if studio_field == "dXSB" {
                return match view.header.primary_team_id {
                    Some(from) if from == *v as u64 => Ok(OpOutcome::default()),
                    Some(from) => self.move_player(player, from, *v as u64),
                    None => self.sign_from_fa(player, *v as u64),
                };
            }
            if let Some(team) = view.header.primary_team_id { return self.set_jersey_number(team, player, *v as u64); }
        }
        if self.roster_only() {
            // Misaligned display mode: the record index came from a roster view and only maps
            // there — apply strictly to the roster file.
            let outcome = self
                .roster_mut()
                .set_field(studio_table, record, studio_field, value)?;
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
            self.roster.as_mut().map(|m| {
                m.session
                    .set_field(studio_table, record, studio_field, value)
            })
        } else {
            None
        };
        let mirror = match roster_result {
            None => false,
            Some(Ok(_)) => true,
            Some(Err(e)) => {
                self.abort_canonical();
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
            let writes =
                crate::attributes::plan_trait_toggle(&slots, value, active).map_err(|()| {
                    DomainError::Edit(tdb_edit::EditError::InvalidValue {
                        what: "no free trait slot (all 32 occupied)",
                        value,
                    })
                })?;
            if writes.is_empty() {
                return Ok(OpOutcome::default());
            }
            let outcome = self
                .roster_mut()
                .set_cells_int(studio_table, record, &writes)?;
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
                self.abort_canonical();
                return Err(e);
            }
        };
        self.record_mirrored(mirror);
        Ok(outcome)
    }

    /// Stage several integer cell edits on one record as one transaction (one undo entry),
    /// mirrored into an aligned roster save that shares the table; roster-only mode edits the
    /// roster alone.
    pub fn set_cells_int(
        &mut self,
        studio_table: &str,
        record: u32,
        writes: &[(String, i64)],
    ) -> Result<OpOutcome> {
        if self.roster_only() {
            let outcome = self
                .roster_mut()
                .set_cells_int(studio_table, record, writes)?;
            self.record(OpTarget::Roster);
            return Ok(outcome);
        }
        let outcome = self.nhlng.set_cells_int(studio_table, record, writes)?;
        let do_mirror = matches!(
            self.roster.as_ref(),
            Some(m) if m.aligned && m.shared_tables.contains(studio_table)
        );
        let roster_result = if do_mirror {
            self.roster
                .as_mut()
                .map(|m| m.session.set_cells_int(studio_table, record, writes))
        } else {
            None
        };
        let mirror = match roster_result {
            None => false,
            Some(Ok(_)) => true,
            Some(Err(e)) => {
                self.abort_canonical();
                return Err(e);
            }
        };
        self.record_mirrored(mirror);
        Ok(outcome)
    }

    /// Phase 2.1 (21 Sep 2026): swap the art ids of two teams under the re-identification
    /// contract (`team_reid`): planned read-only, then written as **one** multi-table transaction
    /// on the canonical file, mirrored into an aligned roster save for the shared tables, one
    /// undo step. Refusals leave everything untouched.
    pub fn apply_artid_swap(&mut self, a: u64, b: u64) -> Result<(crate::team_reid::ArtIdSwapPlan, OpOutcome)> {
        if self.roster_only() {
            return Err(DomainError::Validation("re-identification inside a roster-only session is refused: the mode tables mirror the stock ids and no cascade path is established".into()));
        }
        let plan = crate::team_reid::plan_artid_swap(&self.nhlng, a, b)?;
        let outcome = self.nhlng.set_cells_int_tables(&plan.cells)?;
        let do_mirror = matches!(self.roster.as_ref(), Some(m) if m.aligned);
        let roster_result = if do_mirror {
            self.roster.as_mut().map(|m| {
                let shared: Vec<(String, Vec<(u32, String, i64)>)> = plan.cells.iter().filter(|(t, _)| m.shared_tables.contains(t)).cloned().collect();
                if shared.is_empty() { Ok(OpOutcome::default()) } else { m.session.set_cells_int_tables(&shared) }
            })
        } else {
            None
        };
        let mirror = match roster_result {
            None => false,
            Some(Ok(_)) => true,
            Some(Err(e)) => {
                self.abort_canonical();
                return Err(e);
            }
        };
        self.record_mirrored(mirror);
        Ok((plan, outcome))
    }

    /// Stage integer cell edits across many records of one table as one transaction (one undo
    /// entry), mirrored into an aligned roster save that shares the table (schedule tables are
    /// not roster members, so the mirror is a no-op there); roster-only mode edits the roster
    /// alone. See [`EditSession::set_cells_int_rows`].
    pub fn set_cells_int_rows(
        &mut self,
        studio_table: &str,
        rows: &[(u32, Vec<(String, i64)>)],
    ) -> Result<OpOutcome> {
        if self.roster_only() {
            let outcome = self
                .roster_mut()
                .set_cells_int_rows(studio_table, rows)?;
            self.record(OpTarget::Roster);
            return Ok(outcome);
        }
        let outcome = self.nhlng.set_cells_int_rows(studio_table, rows)?;
        let do_mirror = matches!(
            self.roster.as_ref(),
            Some(m) if m.aligned && m.shared_tables.contains(studio_table)
        );
        let roster_result = if do_mirror {
            self.roster
                .as_mut()
                .map(|m| m.session.set_cells_int_rows(studio_table, rows))
        } else {
            None
        };
        let mirror = match roster_result {
            None => false,
            Some(Ok(_)) => true,
            Some(Err(e)) => {
                self.abort_canonical();
                return Err(e);
            }
        };
        self.record_mirrored(mirror);
        Ok(outcome)
    }

    /// Phase 1.2: the membership choices for a team on this title (see [`crate::membership`]).
    pub fn team_membership_options(
        &self,
        team_id: u64,
    ) -> Result<crate::membership::MembershipOptions> {
        crate::membership::options(self.display(), team_id)
    }

    /// Phase 1.2: move a team to a league with validated group ids, as one transaction. The plan
    /// is computed on the session that receives the edit.
    pub fn set_team_membership(
        &mut self,
        team_id: u64,
        league_code: u64,
        conference: Option<u64>,
        division: Option<u64>,
    ) -> Result<(crate::membership::MembershipPlan, OpOutcome)> {
        let plan = crate::membership::plan(
            if self.roster_only() { self.roster_session().expect("roster_only implies a mirror") } else { &self.nhlng },
            team_id,
            league_code,
            conference,
            division,
        )?;
        let outcome = if self.nhlng.project().game_id() == Some("nhl07") && !self.roster_only() {
            let out = self.nhlng.apply_memberships(std::slice::from_ref(&plan), &[])?;
            self.record_mirrored(false);
            out
        } else { self.set_cells_int(&plan.table, plan.record, &plan.writes)? };
        Ok((plan, outcome))
    }

    pub fn swap_team_memberships(&mut self, first: u64, second: u64) -> Result<OpOutcome> {
        if self.nhlng.project().game_id() != Some("nhl07") || self.roster.is_some() || first == second {
            return Err(DomainError::Validation("Choose two different teams in an NHL 07 database project.".into()));
        }
        let a = crate::membership::options(&self.nhlng, first)?.current;
        let b = crate::membership::options(&self.nhlng, second)?.current;
        if a.league_code == b.league_code { return Err(DomainError::Validation("Choose a team from another league.".into())); }
        let plans = [crate::membership::plan(&self.nhlng, first, b.league_code, b.conference.map(|c|c.value), b.division.map(|c|c.value))?,
            crate::membership::plan(&self.nhlng, second, a.league_code, a.conference.map(|c|c.value), a.division.map(|c|c.value))?];
        let out = self.nhlng.apply_memberships(&plans, &[(first, second)])?;
        self.record_mirrored(false);
        Ok(out)
    }

    pub fn restore_competition(&mut self, baseline: &[u8]) -> Result<OpOutcome> {
        if self.roster.is_some() { return Err(DomainError::Validation("Use a database project to restore organization.".into())); }
        let out=self.nhlng.restore_competition(baseline)?;
        self.record_mirrored(false);
        Ok(out)
    }

    // --- P1-A: source schedule editing / import / generation ---
    //
    // Preview → ticket → Apply. A preview computes the plan against the current model and
    // stores it with the workspace revision; Apply looks the review up (a ticket minted by another
    // workspace instance carries a different namespace and is refused as such), refuses if the revision moved (any edit, undo/redo, reload, membership
    // change, attach…), refuses an import whose source file no longer hashes to the previewed
    // bytes, and writes the **stored** plan — never a recomputation. Any successful mutation
    // drops every held review, so a stale one cannot survive an intervening change.

    const MAX_SCHEDULE_REVIEWS: usize = 16;

    /// The session a schedule plan is computed against (the one that receives the edit).
    pub fn schedule_session(&self) -> &EditSession {
        if self.roster_only() {
            self.roster_session().expect("roster_only implies a mirror")
        } else {
            &self.nhlng
        }
    }

    fn hold_schedule_review(
        &mut self,
        studio_table: &str,
        operation: ScheduleOperation,
        plan: crate::schedule_edit::ChangePlan,
        import: Option<crate::schedule_edit::ImportPlan>,
        generation: Option<crate::schedule_edit::GenerationPlan>,
    ) -> String {
        let ticket = format!("{}/{}", self.review_namespace, self.next_schedule_review);
        self.next_schedule_review += 1;
        if self.schedule_reviews.len() >= Self::MAX_SCHEDULE_REVIEWS {
            self.schedule_reviews.remove(0);
        }
        self.schedule_reviews.push(ScheduleReview {
            ticket: ticket.clone(),
            studio_table: studio_table.to_string(),
            revision: self.revision(),
            operation,
            plan,
            import,
            generation,
        });
        ticket
    }

    /// Whether a ticket was minted by **this** workspace instance (its namespace prefix).
    pub fn owns_schedule_ticket(&self, ticket: &str) -> bool {
        ticket
            .strip_prefix(&self.review_namespace)
            .is_some_and(|rest| rest.starts_with('/'))
    }

    /// A held review, by ticket (`None` when unknown; a ticket from another workspace instance
    /// never matches because its namespace differs).
    pub fn schedule_review(&self, ticket: &str) -> Option<&ScheduleReview> {
        if !self.owns_schedule_ticket(ticket) {
            return None;
        }
        self.schedule_reviews.iter().find(|r| r.ticket == ticket)
    }

    /// Apply a held review as one transaction. Refuses — before any mutation — an unknown id, a
    /// plan with errors, a workspace whose revision moved since the preview, and an import whose
    /// source bytes (`source_text_now`, re-read by the caller) differ from the previewed snapshot.
    /// Returns the review as previewed, the outcome and the table's fresh browse view.
    pub fn schedule_apply_review(
        &mut self,
        ticket: &str,
        source_text_now: Option<&str>,
    ) -> Result<(ScheduleReview, OpOutcome, crate::schedules::ScheduleView)> {
        if !self.owns_schedule_ticket(ticket) {
            return Err(DomainError::Validation(
                "this schedule review was issued by another project (or by an earlier instance of this one); preview again here".into(),
            ));
        }
        let review = self
            .schedule_review(ticket)
            .cloned()
            .ok_or_else(|| DomainError::Validation(
                "this schedule review is no longer held by the project (it was applied, or an earlier change discarded it); preview again".into(),
            ))?;
        if !review.plan.validation.ok() {
            return Err(DomainError::Validation(format!(
                "{}: the change was not applied — {}",
                review.plan.table_name,
                review.plan.validation.errors.join("; ")
            )));
        }
        if self.revision() != review.revision {
            return Err(DomainError::Validation(format!(
                "{}: the project changed since this preview (an edit, undo/redo, reload or membership change); preview again",
                review.plan.table_name
            )));
        }
        if let ScheduleOperation::Import { text_sha256, .. } = &review.operation {
            if let Some(now) = source_text_now {
                if tdb_txn::FileIdentity::of_bytes(now.as_bytes()).sha256 != *text_sha256 {
                    return Err(DomainError::Validation(format!(
                        "{}: the schedule file changed since it was previewed; preview it again",
                        review.plan.table_name
                    )));
                }
            }
        }
        let outcome = if let Some(rows) = &review.plan.replacement {
            self.replace_schedules(&[(review.plan.studio_table.clone(),rows.clone())])?
        } else if review.plan.rows.is_empty() {
            OpOutcome::default()
        } else {
            self.set_cells_int_rows(&review.plan.studio_table, &review.plan.rows)?
        };
        // The revision moved (or nothing changed and the review served its purpose): drop them all.
        self.schedule_reviews.clear();
        let view = self
            .display()
            .schedule_view(&review.plan.studio_table)?
            .ok_or_else(|| DomainError::Validation(format!("{}: table vanished after the edit", review.plan.table_name)))?;
        Ok((review, outcome, view))
    }

    // --- D3: stored-default edits (NHL 15 PS4) -------------------------------------------------

    /// Preview the edit of one stored catalogue default; the plan is held under a ticket bound to
    /// this workspace instance and its current revision. Writes nothing. A refusal is returned as
    /// `Ok(Err(..))` so the caller can show the reason beside the catalogue.
    pub fn settings_default_preview(
        &mut self,
        target: crate::settings_edit::DefaultTarget,
        code: i64,
    ) -> Result<std::result::Result<(String, crate::settings_edit::DefaultEditPlan), crate::settings_edit::DefaultEditRefusal>> {
        if self.roster_only() {
            return Ok(Err(crate::settings_edit::DefaultEditRefusal {
                kind: "not-offered",
                reason: "the roster view has no settings catalogue; switch to the main database".to_string(),
            }));
        }
        let plan = match self.nhlng.settings_default_plan(target, code)? {
            Ok(p) => p,
            Err(r) => return Ok(Err(r)),
        };
        let ticket = format!("{}/{}", self.review_namespace, self.next_schedule_review);
        self.next_schedule_review += 1;
        if self.settings_reviews.len() >= Self::MAX_SCHEDULE_REVIEWS {
            self.settings_reviews.remove(0);
        }
        self.settings_reviews.push(SettingsReview { ticket: ticket.clone(), revision: self.revision(), plan: plan.clone() });
        Ok(Ok((ticket, plan)))
    }

    /// A held stored-default review, by ticket (`None` when unknown or foreign).
    pub fn settings_review(&self, ticket: &str) -> Option<&SettingsReview> {
        if !self.owns_schedule_ticket(ticket) {
            return None;
        }
        self.settings_reviews.iter().find(|r| r.ticket == ticket)
    }

    /// Apply a held stored-default review: one cell, one undo entry. Refuses — before any
    /// mutation — a foreign, consumed or unknown ticket and a workspace whose revision moved since
    /// the preview. After the write the working bytes are compared with the bytes before it under
    /// the single-cell accounting ([`crate::settings_edit::verify_single_cell_diff`]); any other
    /// difference rolls the edit back and is reported. A no-op review writes nothing.
    pub fn settings_apply_review(&mut self, ticket: &str) -> Result<(SettingsReview, OpOutcome)> {
        if !self.owns_schedule_ticket(ticket) {
            return Err(DomainError::Validation(
                "this settings review was issued by another project (or by an earlier instance of this one); preview again here".into(),
            ));
        }
        let review = self
            .settings_review(ticket)
            .cloned()
            .ok_or_else(|| DomainError::Validation("this settings review is no longer held by the project (it was applied, or an earlier change discarded it); preview again".into()))?;
        if self.revision() != review.revision {
            return Err(DomainError::Validation(format!(
                "{}: the project changed since this preview (an edit, undo/redo, reload or another apply); preview again",
                review.plan.setting_name
            )));
        }
        if review.plan.no_op {
            self.settings_reviews.clear();
            return Ok((review, OpOutcome::default()));
        }
        // Re-validate the target on the bytes about to be written (identity + bounds), exactly as
        // the preview did; the revision equality makes this the same file, but the accounting is
        // derived here, independently of the plan.
        let before = self.nhlng.bytes().to_vec();
        let acc = {
            let db = TdbFile::parse(&before)?;
            crate::settings_edit::accounted_diff(&db, &review.plan.studio_table, review.plan.record, &review.plan.studio_field).map_err(DomainError::Validation)?
        };
        let outcome = self.set_cells_int_rows(
            &review.plan.studio_table,
            &[(review.plan.record, vec![(review.plan.studio_field.clone(), review.plan.after)])],
        )?;
        let after = self.nhlng.bytes().to_vec();
        if let Err(e) = crate::settings_edit::verify_single_cell_diff(&before, &after, &acc, self.nhlng.project().platform.endian()) {
            let rolled_back = self.undo();
            return Err(DomainError::Validation(format!(
                "{}: the write changed more than the selected cell and its covering checksum ({e}); the edit was {} and nothing is kept",
                review.plan.setting_name,
                if rolled_back { "rolled back" } else { "NOT rolled back (undo unavailable)" }
            )));
        }
        self.settings_reviews.clear();
        Ok((review, outcome))
    }

    // --- D4: NHL 16–21 CSV stored-default edits -----------------------------------------------

    fn csv_title(&self) -> Option<&str> {
        self.game_id.as_deref().filter(|g| matches!(*g, "nhl16" | "nhl17" | "nhl18" | "nhl19" | "nhl20" | "nhl21"))
    }

    /// Load (or reuse) the `setting_csv` working set for the install root's `inxp_00.big`. A
    /// different archive path than the loaded one is refused while edits are pending.
    fn csv_working_set(&mut self, install_root: &std::path::Path) -> std::result::Result<&mut CsvWorkingSet, crate::settings_edit::DefaultEditRefusal> {
        use crate::settings_edit::DefaultEditRefusal;
        let archive_path = crate::settings_catalogue::csv_archive_path(install_root);
        if let Some(w) = &self.settings_csv {
            if w.archive_path != archive_path {
                if !w.edits.is_empty() {
                    return Err(DefaultEditRefusal { kind: "ambiguous", reason: format!("pending catalogue edits were planned on {}; they must be saved or undone before another archive ({}) is opened", w.archive_path.display(), archive_path.display()) });
                }
                self.settings_csv = None;
            }
        }
        if self.settings_csv.is_none() {
            let slots = crate::settings_csv_edit::ArchiveSlots::read(&archive_path)?;
            let baseline = crate::settings_catalogue::SettingCsvMembers::from_eb_archive(&archive_path).map_err(|b| DefaultEditRefusal { kind: b.kind, reason: b.reason })?;
            let stored_identities = crate::settings_catalogue::csv_stored_identities(&archive_path).map_err(|b| DefaultEditRefusal { kind: b.kind, reason: b.reason })?;
            self.settings_csv = Some(CsvWorkingSet { archive_path, slots, stored_identities, baseline, edits: Vec::new() });
        }
        Ok(self.settings_csv.as_mut().expect("just set"))
    }

    /// The read-only catalogue with the pending CSV edits applied (NHL 16–21), else the session's
    /// own reading (NHL 15 tables, or the blocker).
    pub fn settings_catalogue(&mut self, install_root: Option<&std::path::Path>) -> Result<std::result::Result<crate::settings_catalogue::SettingsCatalogue, crate::settings_catalogue::SettingsBlocker>> {
        use crate::settings_catalogue as sc;
        if let (Some(game), Some(root)) = (self.csv_title().map(str::to_string), install_root) {
            if self.roster_only() {
                return self.display().settings_catalogue(install_root);
            }
            let ws = match self.csv_working_set(root) {
                Ok(w) => w,
                Err(r) => return Ok(Err(sc::SettingsBlocker { kind: r.kind, reason: r.reason })),
            };
            let members = ws.working();
            let defs = self.nhlng.project().settings_defs().nhl15_ps4();
            return Ok(sc::read_setting_csv(&game, &members, defs, self.dictionary.as_ref()));
        }
        self.display().settings_catalogue(install_root)
    }

    /// Preview the edit of one NHL 16–21 stored default; the plan (with the member rewrites and
    /// their re-encoded bytes) is held under a ticket bound to this workspace and its revision.
    pub fn settings_csv_default_preview(
        &mut self,
        install_root: &std::path::Path,
        target: crate::settings_edit::DefaultTarget,
        code: i64,
    ) -> Result<std::result::Result<(String, crate::settings_csv_edit::CsvDefaultPlan), crate::settings_edit::DefaultEditRefusal>> {
        use crate::settings_edit::DefaultEditRefusal;
        let Some(game) = self.csv_title().map(str::to_string) else {
            return Ok(Err(DefaultEditRefusal { kind: "not-offered", reason: "CSV stored-default editing is offered on NHL 16–21 PS4 only".into() }));
        };
        if self.roster_only() {
            return Ok(Err(DefaultEditRefusal { kind: "not-offered", reason: "the roster view has no settings catalogue; switch to the main database".into() }));
        }
        let defs = self.nhlng.project().settings_defs().nhl15_ps4().cloned();
        let dict = self.dictionary.clone();
        let ws = match self.csv_working_set(install_root) {
            Ok(w) => w,
            Err(r) => return Ok(Err(r)),
        };
        let members = ws.working();
        let plan = match crate::settings_csv_edit::plan_csv_default_edit(&game, &members, defs.as_ref(), dict.as_ref(), Some(&ws.slots), &ws.stored_identities, target, code) {
            Ok(p) => p,
            Err(r) => return Ok(Err(r)),
        };
        let ticket = format!("{}/{}", self.review_namespace, self.next_schedule_review);
        self.next_schedule_review += 1;
        if self.settings_csv_reviews.len() >= Self::MAX_SCHEDULE_REVIEWS {
            self.settings_csv_reviews.remove(0);
        }
        self.settings_csv_reviews.push(SettingsCsvReview { ticket: ticket.clone(), revision: self.revision(), plan: plan.clone() });
        Ok(Ok((ticket, plan)))
    }

    pub fn settings_csv_review(&self, ticket: &str) -> Option<&SettingsCsvReview> {
        if !self.owns_schedule_ticket(ticket) {
            return None;
        }
        self.settings_csv_reviews.iter().find(|r| r.ticket == ticket)
    }

    /// Apply a held CSV stored-default review: the plan joins the pending edits (one operation,
    /// one undo step, both members of a pair together). Refuses foreign / consumed / stale tickets
    /// before any change. A no-op review changes nothing.
    pub fn settings_csv_apply_review(&mut self, ticket: &str) -> Result<SettingsCsvReview> {
        if !self.owns_schedule_ticket(ticket) {
            return Err(DomainError::Validation("this settings review was issued by another project (or by an earlier instance of this one); preview again here".into()));
        }
        let review = self
            .settings_csv_review(ticket)
            .cloned()
            .ok_or_else(|| DomainError::Validation("this settings review is no longer held by the project (it was applied, or an earlier change discarded it); preview again".into()))?;
        if self.revision() != review.revision {
            return Err(DomainError::Validation(format!("{}: the project changed since this preview (an edit, undo/redo, reload or another apply); preview again", review.plan.setting_name)));
        }
        if !review.plan.no_op {
            let ws = self.settings_csv.as_mut().ok_or_else(|| DomainError::Validation("the catalogue working set is gone (reloaded); preview again".into()))?;
            // The plan's source texts must be the working texts now (revision equality implies it;
            // re-checked on the bytes).
            let working = ws.working();
            for m in &review.plan.members {
                let cur = working.members.get(&m.member).map(|(b, _)| format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(b)));
                if cur.as_deref() != Some(m.source_plain_sha256.as_str()) {
                    return Err(DomainError::Validation(format!("{}: the member text changed since this preview; preview again", m.member)));
                }
            }
            ws.edits.push(review.plan.clone());
        }
        self.settings_csv_reviews.clear();
        self.schedule_reviews.clear();
        self.settings_reviews.clear();
        Ok(review)
    }

    /// The pending CSV stored-default edits (oldest first).
    pub fn settings_csv_edits(&self) -> &[crate::settings_csv_edit::CsvDefaultPlan] {
        self.settings_csv.as_ref().map(|w| w.edits.as_slice()).unwrap_or(&[])
    }

    /// The working set (archive, identities, pending replacements) for the copied-target save.
    pub fn settings_csv_working_set(&self) -> Option<&CsvWorkingSet> {
        self.settings_csv.as_ref()
    }

    /// Drop the most recent pending CSV stored-default edit (the undo of its apply). `false`
    /// when none is pending. A paired edit is one entry: both cells go together.
    pub fn undo_settings_csv_edit(&mut self) -> bool {
        let Some(w) = self.settings_csv.as_mut() else { return false };
        let popped = w.edits.pop().is_some();
        if popped {
            self.settings_csv_reviews.clear();
            self.schedule_reviews.clear();
            self.settings_reviews.clear();
        }
        popped
    }

    /// After a copied-target save installed the archive with the pending member replacements:
    /// adopt the rewritten texts as the baseline and close the pending edits. `installed` is the
    /// archive's identity after apply (recorded for the caller's bookkeeping; the archive is
    /// re-read on the next preview).
    pub fn rebind_csv_after_copied_save(&mut self) {
        if let Some(w) = self.settings_csv.as_mut() {
            if !w.edits.is_empty() {
                w.baseline = w.working();
                let ids: Vec<(String, String, usize)> = w.replacements().into_iter().map(|m| (m.member_path, m.new_stored_sha256, m.new_stored.len())).collect();
                for (p, s, l) in ids {
                    if let Some(e) = w.stored_identities.iter_mut().find(|(x, ..)| *x == p) {
                        e.1 = s;
                        e.2 = l;
                    }
                }
                w.edits.clear();
            }
            // Slots moved on an append: re-read lazily.
            self.settings_csv = None;
        }
        self.settings_csv_reviews.clear();
    }

    // --- D8: texture members inside packed archives (PS4 logos / imagery) ---

    /// Preview the replacement of a texture member inside `<install_root>/<archive_relative>`:
    /// the plan is held under a ticket bound to the current revision. A refusal is returned as
    /// the `Err` (the member is missing, packaged, undecodable, or the image does not fit).
    pub fn asset_member_preview(
        &mut self,
        install_root: &std::path::Path,
        archive_relative: &str,
        member_path: &str,
        rgba: &[u8],
        w: u32,
        h: u32,
        whole_group: bool,
    ) -> std::result::Result<(String, crate::asset_member_edit::AssetMemberPlan), crate::asset_member_edit::AssetMemberRefusal> {
        if self.roster_only() {
            return Err(crate::asset_member_edit::AssetMemberRefusal { kind: "unsupported", reason: "a roster-only project has no install to place archive members into".into() });
        }
        let game_id = self.game_id().map(str::to_owned);
        let plan = crate::asset_member_edit::plan_asset_member_edit(game_id.as_deref(), install_root, archive_relative, member_path, rgba, w, h, whole_group)?;
        // R1 F1 (20 Sep): a plan whose scope contradicts a pending replacement of the same block is
        // refused here, before it is ever held — the pending list and its history stay as they are.
        if let Some(reason) = self.asset_member_scope_conflict(&plan) {
            return Err(crate::asset_member_edit::AssetMemberRefusal { kind: "scope-conflict", reason });
        }
        // A pending replacement of the same member is superseded on apply; the plan is always
        // made on the archive as it is on disk (the pending ones are not installed yet).
        let ticket = format!("{}/{}", self.review_namespace, self.next_schedule_review);
        self.next_schedule_review += 1;
        if self.asset_member_reviews.len() >= Self::MAX_SCHEDULE_REVIEWS {
            self.asset_member_reviews.remove(0);
        }
        self.asset_member_reviews.push(AssetMemberReview { ticket: ticket.clone(), revision: self.revision(), plan: plan.clone() });
        Ok((ticket, plan))
    }

    /// Receipt 67 (C37): preview a PS3 board-ad raster replacement of `package` (raster `descriptor`) in the install at
    /// `install_root` (its `USRDIR`). The plan — the archive member plus, on a pair title, its loose copy — is held under a
    /// ticket like any asset-member review and joins the pending replacements through [`Self::asset_member_apply_review`].
    pub fn board_replacement_preview(
        &mut self,
        install_root: &std::path::Path,
        package: &str,
        descriptor: usize,
        rgba: &[u8],
        w: u32,
        h: u32,
    ) -> std::result::Result<(String, crate::board_replace::BoardPlan), crate::asset_member_edit::AssetMemberRefusal> {
        if self.roster_only() {
            return Err(crate::asset_member_edit::AssetMemberRefusal { kind: "unsupported", reason: "a roster-only project has no install to place archive members into".into() });
        }
        let game_id = self.game_id().map(str::to_owned);
        let platform = self.primary().project().platform;
        let board = crate::board_replace::plan_ps3_board_replacement(game_id.as_deref(), platform, install_root, package, descriptor, rgba, w, h)?;
        if let Some(reason) = self.asset_member_scope_conflict(&board.plan) {
            return Err(crate::asset_member_edit::AssetMemberRefusal { kind: "scope-conflict", reason });
        }
        let ticket = format!("{}/{}", self.review_namespace, self.next_schedule_review);
        self.next_schedule_review += 1;
        if self.asset_member_reviews.len() >= Self::MAX_SCHEDULE_REVIEWS {
            self.asset_member_reviews.remove(0);
        }
        self.asset_member_reviews.push(AssetMemberReview { ticket: ticket.clone(), revision: self.revision(), plan: board.plan.clone() });
        Ok((ticket, board))
    }

    pub fn asset_member_review(&self, ticket: &str) -> Option<&AssetMemberReview> {
        if !self.owns_schedule_ticket(ticket) {
            return None;
        }
        self.asset_member_reviews.iter().find(|r| r.ticket == ticket)
    }

    /// Apply a held asset-member review: the plan joins the pending replacements (a plan on a
    /// member already pending supersedes it). Refuses foreign / consumed / stale tickets.
    pub fn asset_member_apply_review(&mut self, ticket: &str) -> Result<AssetMemberReview> {
        if !self.owns_schedule_ticket(ticket) {
            return Err(DomainError::Validation("this texture review was issued by another project (or by an earlier instance of this one); preview again here".into()));
        }
        let review = self
            .asset_member_review(ticket)
            .cloned()
            .ok_or_else(|| DomainError::Validation("this texture review is no longer held by the project (it was applied, or an earlier change discarded it); preview again".into()))?;
        if self.revision() != review.revision {
            return Err(DomainError::Validation(format!("{}: the project changed since this preview (an edit, undo/redo, reload or another apply); preview again", review.plan.member_path)));
        }
        // R1 F1 (20 Sep): the engine guard (`stage::plan_scoped`) is the last line; this is the
        // first — a contradictory scope never joins the pending list, which stays exactly as it
        // was (the held review is kept too: nothing changed).
        if let Some(reason) = self.asset_member_scope_conflict(&review.plan) {
            return Err(DomainError::Validation(reason));
        }
        // Coalesce in place: the member keeps its position in the effective list and the plan it
        // had there is remembered by the operation so undo can put it back (R1 F2).
        let key = (review.plan.archive_relative.as_str(), review.plan.member_path.as_str());
        let op = match self.asset_members.iter().position(|m| (m.archive_relative.as_str(), m.member_path.as_str()) == key) {
            Some(i) => {
                let superseded = std::mem::replace(&mut self.asset_members[i], review.plan.clone());
                AssetMemberOp { position: i, superseded: Some(superseded) }
            }
            None => {
                self.asset_members.push(review.plan.clone());
                AssetMemberOp { position: self.asset_members.len() - 1, superseded: None }
            }
        };
        self.asset_member_ops.push(op);
        self.asset_member_reviews.clear();
        self.settings_csv_reviews.clear();
        self.schedule_reviews.clear();
        self.settings_reviews.clear();
        Ok(review)
    }

    /// R1 F1 (20 Sep): the rows a member plan owns in its archive — its own member, plus every
    /// sibling of the block under the whole-group scope. Two pending plans of one archive that own
    /// the same member contradict each other (whole-group vs whole-group on one block, or a
    /// whole-group replacement vs a private copy of one of its siblings); a plan on the *same*
    /// member supersedes the pending one and is never a conflict. Returns the refusal text naming
    /// both members and the contested row, else `None`.
    fn asset_member_scope_conflict(&self, plan: &crate::asset_member_edit::AssetMemberPlan) -> Option<String> {
        fn owned(p: &crate::asset_member_edit::AssetMemberPlan) -> Vec<&str> {
            let mut v = vec![p.member_path.as_str()];
            if p.whole_group {
                v.extend(p.shared_with.iter().map(String::as_str));
            }
            v
        }
        let mine = owned(plan);
        for other in &self.asset_members {
            if other.archive_relative != plan.archive_relative || other.member_path == plan.member_path {
                continue;
            }
            let theirs = owned(other);
            if let Some(contested) = mine.iter().find(|m| theirs.contains(m)) {
                let scope = |p: &crate::asset_member_edit::AssetMemberPlan| if p.whole_group { "whole group" } else { "this member only" };
                return Some(format!(
                    "{}: replacing {} ({}) contradicts the pending replacement of {} ({}) — both would rewrite the row of {contested}, which shares one stored block; undo or discard one of them, or choose one scope for the block",
                    plan.archive_relative,
                    plan.member_path,
                    scope(plan),
                    other.member_path,
                    scope(other),
                ));
            }
        }
        None
    }

    /// The effective pending asset-member replacements (one per member, first-seen order).
    pub fn asset_member_edits(&self) -> &[crate::asset_member_edit::AssetMemberPlan] {
        &self.asset_members
    }

    /// How many asset-member applies are undoable (the operation log's depth, which can exceed
    /// the number of effective replacements when a member was replaced more than once).
    pub fn asset_member_undo_depth(&self) -> usize {
        self.asset_member_ops.len()
    }

    /// Undo the most recent asset-member apply: the plan it installed leaves the effective list
    /// and the plan it superseded on that member (if any) returns to its position. `false` when
    /// nothing is pending.
    pub fn undo_asset_member_edit(&mut self) -> bool {
        let Some(op) = self.asset_member_ops.pop() else { return false };
        match op.superseded {
            Some(prior) => {
                debug_assert!(op.position < self.asset_members.len());
                self.asset_members[op.position] = prior;
            }
            None => {
                // A fresh insert is always the newest entry of the effective list at the time it
                // is undone (every later op has been undone first), so it is the last element.
                debug_assert_eq!(op.position + 1, self.asset_members.len());
                self.asset_members.truncate(op.position);
            }
        }
        self.asset_member_reviews.clear();
        self.settings_csv_reviews.clear();
        self.schedule_reviews.clear();
        self.settings_reviews.clear();
        true
    }

    /// After a copied-target save installed the archives with the pending member replacements:
    /// the replacements are done (the archive is re-read on the next preview).
    pub fn rebind_assets_after_copied_save(&mut self) {
        self.asset_members.clear();
        self.asset_member_ops.clear();
        self.asset_member_reviews.clear();
    }

    /// Preview a single game edit (home/away/date) with whole-table validation; returns the held
    /// review ticket and the plan.
    pub fn schedule_edit_preview(
        &mut self,
        studio_table: &str,
        edit: &crate::schedule_edit::GameEdit,
    ) -> Result<(String, crate::schedule_edit::ChangePlan)> {
        let model = self.schedule_session().schedule_model(studio_table)?;
        let plan = crate::schedule_edit::plan_game_edit(&model, edit)?;
        let id = self.hold_schedule_review(studio_table, ScheduleOperation::Edit(edit.clone()), plan.clone(), None, None);
        Ok((id, plan))
    }

    /// Export a schedule table as `csv` or `json` text.
    pub fn schedule_export(&self, studio_table: &str, format: crate::schedule_edit::ImportFormat) -> Result<String> {
        let s = self.schedule_session();
        let model = s.schedule_model(studio_table)?;
        let ctx = s.schedule_export_context();
        Ok(match format {
            crate::schedule_edit::ImportFormat::Csv => crate::schedule_edit::export_csv(&model, &ctx),
            crate::schedule_edit::ImportFormat::Json => crate::schedule_edit::export_json(&model, &ctx),
        })
    }

    /// Preview an import: parse, resolve keys through the explicit mapping, validate the whole
    /// proposal; the text is snapshotted into the held review. Malformed input is an error;
    /// unresolved keys are refusals in the plan.
    pub fn schedule_import_preview(
        &mut self,
        studio_table: &str,
        format: crate::schedule_edit::ImportFormat,
        text: &str,
        source_path: Option<&str>,
        mapping: &[crate::schedule_edit::TeamMapping],
    ) -> Result<(String, crate::schedule_edit::ImportPlan)> {
        let model = self.schedule_session().schedule_model(studio_table)?;
        let doc = crate::schedule_edit::parse_import(text, format)?;
        let plan = crate::schedule_edit::plan_import(&model, &doc, mapping, None)?;
        let op = ScheduleOperation::Import {
            format,
            text: text.to_string(),
            text_sha256: tdb_txn::FileIdentity::of_bytes(text.as_bytes()).sha256,
            source_path: source_path.map(str::to_string),
            mapping: mapping.to_vec(),
        };
        let id = self.hold_schedule_review(studio_table, op, plan.change.clone(), Some(plan.clone()), None);
        Ok((id, plan))
    }

    /// The topology the table supports for generation (or the precise blocker).
    pub fn schedule_generation_options(&self, studio_table: &str) -> Result<crate::schedule_edit::GenerationOptions> {
        let model = self.schedule_session().schedule_model(studio_table)?;
        Ok(crate::schedule_edit::generation_options(&model))
    }

    /// Receipt 48: this table's shipped single-phase groups when the opened file, the table and the membership
    /// equal a recorded R113 source exactly; otherwise the plain-language reason (never a guess).
    pub fn schedule_stock_groups(&self, studio_table: &str) -> Result<std::result::Result<crate::schedule_stock::StockGroups, String>> {
        let s = self.schedule_session();
        let model = s.schedule_model(studio_table)?;
        let sha = s.project().loaded_identity().map(|i| i.sha256.clone());
        Ok(crate::schedule_stock::stock_groups(s.project().schedule_stock(), &model.summary.name, sha.as_deref(), &model.slots))
    }

    /// Preview a deterministic generation for an explicit configuration; returns the held review.
    pub fn schedule_generate_preview(
        &mut self,
        studio_table: &str,
        cfg: &crate::schedule_edit::GenerationConfig,
    ) -> Result<(String, crate::schedule_edit::GenerationPlan)> {
        let model = self.schedule_session().schedule_model(studio_table)?;
        let plan = crate::schedule_edit::plan_generation(&model, cfg)?;
        let id = self.hold_schedule_review(studio_table, ScheduleOperation::Generate(cfg.clone()), plan.change.clone(), None, Some(plan.clone()));
        Ok((id, plan))
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
    pub fn autocorrect_duplicate_jerseys(
        &mut self,
    ) -> Result<(OpOutcome, Vec<JerseyReassignment>)> {
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
            self.roster
                .as_mut()
                .map(|m| m.session.autocorrect_duplicate_jerseys())
        } else {
            None
        };
        let mirror = match roster_result {
            None => false,
            Some(Ok(_)) => true,
            Some(Err(e)) => {
                self.abort_canonical();
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
        let roster_repair = match self
            .roster
            .as_mut()
            .map(|m| m.session.repair_roster_blocks())
        {
            None => None,
            Some(Ok(r)) => Some(r),
            Some(Err(e)) => {
                if summary.moved_slots > 0 {
                    self.abort_canonical(); // keep the pair consistent
                }
                return Err(e);
            }
        };
        let roster_moved = roster_repair
            .as_ref()
            .map(|(_, rs)| rs.moved_slots)
            .unwrap_or(0);
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
        self.fan_out_shared(tdb_edit::txn::retired::TABLE, move |s| {
            s.retire_jersey(team, number)
        })
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
            let (record, outcome) = self.roster_mut().add_career_season(
                studio_table,
                player,
                year_end,
                league,
                team,
            )?;
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
            self.roster.as_mut().map(|m| {
                m.session
                    .add_career_season(studio_table, player, year_end, league, team)
            })
        } else {
            None
        };
        let mirror = match roster_result {
            None => false,
            Some(Ok((r, _))) if r == record => true,
            Some(Ok((r, _))) => {
                // Divergent append index — the files were not as aligned as advertised; undo both.
                self.abort_canonical();
                if let Some(m) = self.roster.as_mut() {
                    m.session.abort_last();
                }
                return Err(DomainError::Sync(format!(
                    "career append diverged: nhlng record {record} vs roster record {r}; both rolled back"
                )));
            }
            Some(Err(e)) => {
                self.abort_canonical();
                return Err(e);
            }
        };
        self.record_mirrored(mirror);
        Ok((record, outcome))
    }

    /// Delete one stock career row (ordered shift), mirrored when the table is shared and aligned.
    pub fn delete_career_row(&mut self, studio_table: &str, record: u32) -> Result<OpOutcome> {
        self.fan_out_shared(studio_table, move |s| {
            s.delete_career_row(studio_table, record)
        })
    }

    /// Clone an `xjgw` jersey row into a new kit; returns the new record index. Mirrored into the
    /// roster save when `xjgw` is shared and aligned (both files append at the same index by
    /// construction); a diverging mirror index rolls both back — the [`Workspace::add_career_season`]
    /// guard for the jersey registry.
    pub fn clone_jersey(
        &mut self,
        source_record: u32,
        team: u64,
        style: u64,
        art_id: u64,
        variant: u64,
        jersey_index: Option<u64>,
    ) -> Result<(u32, OpOutcome)> {
        if self.roster_only() {
            let (record, outcome) = self.roster_mut().clone_jersey(
                source_record,
                team,
                style,
                art_id,
                variant,
                jersey_index,
            )?;
            self.record(OpTarget::Roster);
            return Ok((record, outcome));
        }
        let (record, outcome) =
            self.nhlng
                .clone_jersey(source_record, team, style, art_id, variant, jersey_index)?;
        let do_mirror = matches!(
            self.roster.as_ref(),
            Some(m) if m.aligned && m.shared_tables.contains(tdb_edit::txn::jerseys::TABLE)
        );
        let roster_result = if do_mirror {
            self.roster.as_mut().map(|m| {
                m.session
                    .clone_jersey(source_record, team, style, art_id, variant, jersey_index)
            })
        } else {
            None
        };
        let mirror = match roster_result {
            None => false,
            Some(Ok((r, _))) if r == record => true,
            Some(Ok((r, _))) => {
                self.abort_canonical();
                if let Some(m) = self.roster.as_mut() {
                    m.session.abort_last();
                }
                return Err(DomainError::Sync(format!(
                    "jersey clone diverged: nhlng record {record} vs roster record {r}; both rolled back"
                )));
            }
            Some(Err(e)) => {
                self.abort_canonical();
                return Err(e);
            }
        };
        self.record_mirrored(mirror);
        Ok((record, outcome))
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
                self.abort_canonical();
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
                self.abort_canonical();
                return Err(DomainError::Sync(format!(
                    "The attached roster refused this player transaction: {e}. \
                     The database change was rolled back. Switch to Roster save to edit \
                     that file independently, or use a roster matching this database."
                )));
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
        let roster_result = self
            .roster
            .as_mut()
            .map(|m| m.session.create_player(team, position));
        match roster_result {
            None => {
                self.record_mirrored(false);
                Ok((nid, outcome))
            }
            Some(Err(e)) => {
                self.abort_canonical();
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
                self.abort_canonical();
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
        let roster_result = self
            .roster
            .as_mut()
            .map(|m| m.session.delete_player(player));
        match roster_result {
            None => {
                self.record_mirrored(false);
                Ok(outcome)
            }
            Some(Err(e)) => {
                self.abort_canonical();
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
        Ok(crate::capacity::project_capacity(
            &db,
            self.game_id.as_deref(),
            creates,
        ))
    }

    /// Commit a lowered cart for real — one transaction, one undo entry. With a roster attached, the
    /// identical ops apply to the mirror too and **full record alignment is re-verified**; any
    /// divergence rolls both files back (the [`Workspace::create_player`] guard, for a whole cart).
    pub fn commit_import(
        &mut self,
        ops: &[tdb_edit::txn::ImportOp],
    ) -> Result<tdb_edit::txn::ImportApplied> {
        if self.roster_only() {
            let applied = self.roster_mut().commit_import(ops)?;
            self.record(OpTarget::Roster);
            return Ok(applied);
        }
        // The canonical db is the authority for the created ids (an aligned mirror mints the same).
        let applied = self.nhlng.commit_import(ops)?;
        let roster_result = self.roster.as_mut().map(|m| m.session.commit_import(ops));
        match roster_result {
            None => {
                self.record_mirrored(false);
                Ok(applied)
            }
            Some(Err(e)) => {
                self.abort_canonical();
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
                Ok(applied)
            }
        }
    }

    /// Revert the most recent op on both sessions (used by the structural-op guard on divergence).
    fn roll_back_both(&mut self) {
        if let Some(m) = self.roster.as_mut() {
            m.session.abort_last();
        }
        self.abort_canonical();
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
    pub fn save_in_place(&mut self) -> Result<u64> {
        self.save_in_place_with(&mut crate::in_place_save::NoHooks)
    }

    /// [`Workspace::save_in_place`] with publication hooks (progress and test fault injection).
    ///
    /// Two-phase (R1 F1): the complete save set — database, roster save, content-header sidecars,
    /// dictionary — is packaged and every destination is checked against the identity this
    /// workspace holds for it **before** any file is touched; then the files are published
    /// through [`crate::in_place_save::publish`], which rolls back every already-written file if
    /// a later one fails. A refusal or a rolled-back failure leaves every destination, the
    /// sessions' baselines, the pending edits, the attached dictionary and the pending rename
    /// exactly as they were. A rollback that itself fails is reported as the explicit split
    /// state it is (which file holds which bytes) and marks the baseline stale; nothing is
    /// adopted from a split publication.
    pub fn save_in_place_with(
        &mut self,
        hooks: &mut dyn crate::in_place_save::Hooks,
    ) -> Result<u64> {
        use crate::in_place_save::{self as ips, Expectation, PlannedWrite, Prior};
        if let Some(reason) = self.stale_baseline() {
            return Err(DomainError::Sync(format!(
                "the save is refused because the workspace baseline is stale: {reason}"
            )));
        }
        // --- Preflight: package everything, read and verify every destination, write nothing ---
        let mut plan: Vec<PlannedWrite> = Vec::new();
        let mut plan_db = |session: &EditSession,
                           path: &str,
                           role: &'static str,
                           header_role: &'static str|
         -> Result<()> {
            let bytes = session.save()?;
            let p = std::path::Path::new(path);
            let expectation = match session.project().loaded_identity() {
                Some(id) => Expectation::Identity(id.clone()),
                None => Expectation::AnyPresent,
            };
            let prior = ips::read_prior(role, p, &expectation)?;
            let identity = tdb_txn::FileIdentity::of_bytes(&bytes);
            plan.push(PlannedWrite {
                role,
                path: p.to_path_buf(),
                prior,
                bytes,
                identity,
            });
            // A roster save opened *alone* occupies the canonical slot (`Workspace::single`), so
            // the sidecar check runs on both paths; it plans nothing outside header-indexed
            // content trees, and never rewrites a sidecar that exists.
            if let Some((header_path, header_bytes)) =
                crate::content_header::plan_content_header(p)?.required()
            {
                let prior = ips::read_prior(header_role, header_path, &Expectation::Absent)?;
                plan.push(PlannedWrite {
                    role: header_role,
                    path: header_path.to_path_buf(),
                    prior,
                    bytes: header_bytes.to_vec(),
                    identity: tdb_txn::FileIdentity::of_bytes(header_bytes),
                });
            }
            Ok(())
        };
        plan_db(
            &self.nhlng,
            &self.nhlng_path,
            "the main database",
            "the main database's content header",
        )?;
        if let Some(m) = &self.roster {
            plan_db(
                &m.session,
                &m.path,
                "the roster save",
                "the roster save's content header",
            )?;
        }
        // The pending dictionary rename, if any: the file must still be the attached baseline
        // (never overwrite bytes the rename was not planned on).
        if let (Some(edit), Some(base)) = (&self.dictionary_edit, &self.dictionary) {
            let p = std::path::Path::new(&base.path);
            let expected = tdb_txn::FileIdentity {
                len: base.bytes.len() as u64,
                sha256: base.sha256.clone(),
            };
            let prior = match ips::read_prior("the dictionary", p, &Expectation::Identity(expected.clone())) {
                Ok(prior) => prior,
                Err(DomainError::Sync(_)) => {
                    let current = tdb_txn::FileIdentity::of_file(p)
                        .map_err(|e| DomainError::Sync(format!("{}: {e}", base.path)))?;
                    return Err(DomainError::Validation(format!(
                        "the dictionary {} is {current} on disk but the rename was planned on {expected}; it changed since it was attached — nothing was written; re-attach it and plan the rename again",
                        base.path
                    )));
                }
                Err(e) => return Err(e),
            };
            debug_assert!(matches!(prior, Prior::Bytes(_)));
            plan.push(PlannedWrite {
                role: "the dictionary",
                path: p.to_path_buf(),
                prior,
                bytes: edit.output.as_ref().clone(),
                identity: edit.output_identity.clone(),
            });
        }
        let files = plan.len() as u64;
        // --- Publish: all or nothing, with the split case named ---
        let outcome = ips::publish(&plan, hooks);
        match &outcome {
            ips::Outcome::Committed => {}
            ips::Outcome::RolledBack { .. } => {
                return Err(DomainError::Sync(outcome.describe(&plan)));
            }
            ips::Outcome::Split { .. } => {
                let text = outcome.describe(&plan);
                self.mark_stale(StaleOrigin::Split, text.clone());
                return Err(DomainError::Sync(text));
            }
        }
        // The files now hold the packaged working bytes: adopt them as the baseline (the same
        // verified rebind a copied-target commit performs), so the pending review and the dirty
        // flag measure against what is on disk. A rebind that fails to verify keeps the edits and
        // the old baseline and reports why; the files were still written and verified.
        let identity_of = |role: &str| plan.iter().find(|w| w.role == role).map(|w| w.identity.clone());
        let primary = identity_of("the main database");
        let roster = identity_of("the roster save");
        let dictionary = identity_of("the dictionary");
        self.rebind_after_copied_save(primary.as_ref(), roster.as_ref(), dictionary.as_ref())?;
        Ok(files)
    }

    /// The pending review for the primary database and, when attached, the roster save — each
    /// against its own acknowledged baseline, so a mirrored edit is reported once per file role
    /// rather than conflated.
    pub fn pending_changes(
        &self,
        limit: usize,
    ) -> (crate::pending::PendingChanges, Option<crate::pending::PendingChanges>) {
        (
            self.nhlng.pending_changes(limit),
            self.roster.as_ref().map(|m| m.session.pending_changes(limit)),
        )
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

/// The content revision a copied-target change set is bound to (see [`Workspace::revision`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRevision {
    pub primary: tdb_txn::FileIdentity,
    pub roster: Option<tdb_txn::FileIdentity>,
    /// The attached dictionary's working identity (pending rename output, else baseline).
    pub dictionary: Option<tdb_txn::FileIdentity>,
    /// D4: the identity of the pending NHL 16–21 catalogue member texts (`None` when no CSV
    /// stored-default edit is pending).
    pub text_members: Option<String>,
    /// D8: the identity of the pending asset-member replacements (`None` when none).
    pub asset_members: Option<String>,
}

impl std::fmt::Display for WorkspaceRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.primary.sha256[..16])?;
        if let Some(r) = &self.roster {
            write!(f, "+{}", &r.sha256[..16])?;
        }
        if let Some(d) = &self.dictionary {
            write!(f, "+loc:{}", &d.sha256[..16])?;
        }
        if let Some(t) = &self.text_members {
            write!(f, "+csv:{}", &t[..16])?;
        }
        if let Some(a) = &self.asset_members {
            write!(f, "+asset:{}", &a[..16])?;
        }
        Ok(())
    }
}

/// What [`Workspace::refresh_baselines`] did to one session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BaselineAction {
    /// The file still carries the loaded identity; nothing changed.
    #[default]
    Unchanged,
    /// The file changed and the session was clean: reloaded from disk.
    Reloaded,
    /// The file changed under unsaved edits: edits kept, baseline marked stale.
    StaleKeptEdits,
    /// The session has no baseline identity (built from bytes); left alone.
    NoBaseline,
}

/// Per-session outcome of [`Workspace::refresh_baselines`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaselineRefresh {
    pub primary: BaselineAction,
    pub roster: Option<BaselineAction>,
}

/// Everything a [`Workspace::refresh_baselines`] will do, computed without touching the workspace.
struct PreparedRefresh {
    primary: BaselineAction,
    primary_reload: Option<crate::session::PreparedReload>,
    roster: Option<BaselineAction>,
    roster_reload: Option<crate::session::PreparedReload>,
    /// Re-derived mirror relationship when either file is reloaded.
    mirror: Option<(HashSet<String>, bool)>,
}

/// Compute the shared canonical table set and whether the two files are record-index-aligned.
fn analyze(nhlng: &EditSession, roster: &EditSession) -> Result<(HashSet<String>, bool)> {
    if crate::roster_difflist::recognize(nhlng.bytes(), roster.bytes(), nhlng.project().meta()).is_ok()
    {
        return Err(crate::DomainError::Validation(
            "This roster stores changes to the game database. Roster-save editing for this save model is not yet enabled; open the game database without attaching the save.".into(),
        ));
    }
    analyze_bytes(nhlng.bytes(), roster.bytes())
}

/// [`analyze`] on raw inner-db bytes (the prepared bytes of a reload that is not adopted yet).
fn analyze_bytes(nhlng: &[u8], roster: &[u8]) -> Result<(HashSet<String>, bool)> {
    let ndb = TdbFile::parse(nhlng)?;
    let rdb = TdbFile::parse(roster)?;
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
    // An empty intersection used to count as aligned: every (nonexistent)
    // shared table had matching counts. NHL07's legacy-format save fixture
    // exposed this by making all Xbox360 entity edits fail in the mirror.
    if shared.is_empty() {
        return Err(crate::DomainError::Validation(
            "This roster save shares no database tables with the selected game database. Its save model cannot use record-index mirroring; open the database without this save.".into(),
        ));
    }
    Ok((shared, aligned))
}

/// Every new text a growth op carries, concatenated for the glyph gate / C1 refusal.
fn growth_texts(ops: &[tdb_loc::GrowthOp]) -> String {
    let mut out = String::new();
    for op in ops {
        match op {
            tdb_loc::GrowthOp::TextGrow { new_text, .. } => out.push_str(new_text),
            tdb_loc::GrowthOp::InsertKey { text: Some(t), .. } => out.push_str(t),
            _ => {}
        }
        out.push('\n');
    }
    out
}
