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
use tdb_domain_validation::{towx_coherence, validate_invariants, IntegrityReport};
use tdb_edit::txn::{BlockRepairSummary, EditTransaction, JerseyReassignment, OpOutcome};
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

/// A file read, detected and parsed for [`EditSession::adopt_reload`]; nothing about the session
/// changed yet.
pub struct PreparedReload {
    file_bytes: Vec<u8>,
    inner: Vec<u8>,
    identity: tdb_txn::FileIdentity,
}

impl PreparedReload {
    /// The inner TDB bytes the reload would adopt.
    pub fn inner(&self) -> &[u8] {
        &self.inner
    }
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
    /// The target's own localization dictionary, when attached (league label resolution).
    dictionary: Option<crate::leagues::AttachedDictionary>,
    /// Test seam (P1 audit, 21 Sep 2026): when set, every transaction of this session is refused
    /// with this reason before it commits — the deterministic way to exercise the coordinated
    /// rollback of a workspace whose roster mirror fails. Never set in production paths.
    injected_fault: Option<String>,
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
            dictionary: None,
            injected_fault: None,
        }
    }

    /// Fault injection for tests: `Some(reason)` makes every subsequent transaction of this
    /// session fail with `reason` before committing (working bytes, history and redo untouched);
    /// `None` clears it.
    #[doc(hidden)]
    pub fn inject_fault(&mut self, reason: Option<String>) {
        self.injected_fault = reason;
    }

    /// Attach (or detach) the target's localization dictionary for label resolution.
    pub fn set_dictionary(&mut self, dictionary: Option<crate::leagues::AttachedDictionary>) {
        self.dictionary = dictionary;
    }

    pub fn dictionary(&self) -> Option<&crate::leagues::AttachedDictionary> {
        self.dictionary.as_ref()
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
    pub fn grow_table_max(
        &mut self,
        canonical_table: &str,
        new_max: u16,
    ) -> Result<Option<(u16, u16)>> {
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

    /// Probe the per-surface availability contract the UI gates tabs/queries on (Phase 1). Like
    /// [`Self::capabilities`] it reads the working file's own schema (so it reflects the current
    /// display file, roster mirror included), and reuses the field-level capabilities for the rating
    /// tallies.
    pub fn surface_capabilities(&self) -> Result<crate::SurfaceCapabilities> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        let caps = crate::Capabilities::probe(&db, self.project.meta());
        Ok(crate::SurfaceCapabilities::probe(
            &db,
            self.project.meta(),
            &model,
            &caps,
        ))
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
        let mut players = crate::view::player_summaries(
            &db,
            self.project.meta(),
            self.project.enums(),
            &model,
        );
        if let Some(pool) = crate::explicit_free_agents(&self.working)? {
            for p in &mut players { if pool.contains(&p.player_id) { p.team_id = None; } }
        }
        Ok(players)
    }

    /// Build the EA name+DOB match index over the open project (Phase 5.1 — Roster Builder import).
    /// Reads the *working* state. Caching this across cart items is the service layer's job (5.4);
    /// this builds it fresh, so callers should not invoke it per lookup.
    pub fn ea_slug_index(&self) -> Result<crate::import::EaSlugIndex> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        Ok(crate::import::EaSlugIndex::build(
            &db,
            self.project.meta(),
            self.project.enums(),
            &model,
        ))
    }

    /// Every player's art (portrait / head mesh / head textures), resolved against `roots`.
    ///
    /// Reads the *working* state, so a staged edit to a player's name or head id is reflected in
    /// the export names before it's ever saved.
    pub fn player_assets(&self, roots: &crate::AssetRoots) -> Result<Vec<crate::PlayerAssets>> {
        let db = TdbFile::parse(&self.working)?;
        crate::assets::scan_player_assets(&db, self.project.meta(), roots)
    }

    /// The full Player detail view for one `wBIz`, or `None` if no such player.
    pub fn player_view(&self, player_id: u64) -> Result<Option<crate::PlayerView>> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        let mut player = crate::view::player_view(
            &db,
            self.project.meta(),
            self.project.enums(),
            &model,
            player_id,
        );
        if crate::explicit_free_agents(&self.working)?.is_some_and(|pool| pool.contains(&player_id)) {
            if let Some(p) = &mut player {
                p.header.primary_team_id = None;
                p.header.primary_team_name = None;
                for cell in p.profile.sections.iter_mut().flat_map(|s| &mut s.cells) {
                    if cell.label == "Main Team" {
                        cell.control = crate::field_profile::CellControl::Derived;
                        cell.field = None;
                        cell.derived_display = Some("Free agent".into());
                    }
                }
            }
        }
        Ok(player)
    }

    /// The pending review: every difference between the acknowledged baseline
    /// (`Project::db_bytes`, i.e. the loaded or last verified-saved file) and the working bytes.
    /// `limit` caps the decoded cell list; counts stay exact.
    pub fn pending_changes(&self, limit: usize) -> crate::pending::PendingChanges {
        let leagues = TdbFile::parse(&self.working)
            .ok()
            .map(|db| self.leagues_for(&db));
        crate::pending::pending_changes(
            self.project.db_bytes(),
            &self.working,
            self.project.meta(),
            self.project.enums(),
            leagues.as_ref(),
            limit,
        )
    }

    /// Tag the session with the studio game id it was opened as (league-vocabulary evidence).
    pub fn set_game_id(&mut self, game_id: Option<String>) {
        self.project.set_game_id(game_id);
    }

    /// The league catalog over an already-parsed working database.
    fn leagues_for(&self, db: &TdbFile) -> crate::leagues::LeagueCatalog {
        crate::leagues::LeagueCatalog::build_for_target(
            db,
            self.project.meta(),
            self.project.leagues(),
            self.project.game_id(),
            Some(self.project.platform),
            self.dictionary.as_ref(),
        )
    }

    /// The league catalog over the current working state: every league identity the target
    /// stores, resolved per target (see [`crate::leagues`]).
    pub fn league_catalog(&self) -> Result<crate::leagues::LeagueCatalog> {
        let db = TdbFile::parse(&self.working)?;
        Ok(self.leagues_for(&db))
    }

    /// One league's detail (identity, editable row on PS4, member teams), or `None` when no
    /// league with that native code exists.
    pub fn league_view(&self, code: u64) -> Result<Option<crate::leagues::LeagueView>> {
        let db = TdbFile::parse(&self.working)?;
        let leagues = self.leagues_for(&db);
        let Some(identity) = leagues.league(code).cloned() else {
            return Ok(None);
        };
        let model = self.build_model(&db)?;
        let members: Vec<crate::TeamSummary> = crate::view::team_summaries(
            &db,
            self.project.meta(),
            self.project.enums(),
            &model,
            &leagues,
        )
        .into_iter()
        .filter(|t| t.league == Some(code as i64))
        .collect();
        let (fields, row_table, name_max_bytes) = match identity.record {
            Some(rec) => {
                let fields = crate::view::entity_fields(
                    &db,
                    self.project.meta(),
                    self.project.enums(),
                    crate::leagues::STOCK_LEAGUES,
                    rec,
                );
                let width = db
                    .table_canonical(crate::leagues::STOCK_LEAGUES)
                    .and_then(|t| {
                        crate::leagues::field_by_studio(&db, t, crate::leagues::STOCK_LEAGUE_NAME)
                    })
                    .map(|f| f.bit_width / 8);
                (
                    fields,
                    Some(crate::leagues::STOCK_LEAGUES.to_string()),
                    width,
                )
            }
            None => (Vec::new(), None, None),
        };
        Ok(Some(crate::leagues::LeagueView {
            identity,
            fields,
            row_table,
            name_max_bytes,
            members,
            name_edit_blocker: leagues.name_edit_blocker(),
            structure: leagues.structure(code),
            structure_blocker: if leagues.competition.is_none() { leagues.competition_blocker.clone() } else { None },
        }))
    }

    /// The team master list over the current working state.
    pub fn team_summaries(&self) -> Result<Vec<crate::TeamSummary>> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        let leagues = self.leagues_for(&db);
        Ok(crate::view::team_summaries(
            &db,
            self.project.meta(),
            self.project.enums(),
            &model,
            &leagues,
        ))
    }

    /// Phase 2.3: the dictionary text a team owns (per-team key families keyed by `artabbr`).
    pub fn team_text(&self, team_id: u64) -> Result<Option<crate::team_text::TeamText>> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        let Some(team) = model.team(team_id) else {
            return Ok(None);
        };
        let Some(kott) = db.table_canonical(tdb_relations::entity::ids::TEAM) else {
            return Ok(None);
        };
        Ok(Some(crate::team_text::team_text(
            &db,
            self.project.meta(),
            kott,
            team.record,
            team_id,
            self.dictionary.as_ref(),
        )))
    }

    /// Phase 1.4: one league's logo binding and package paths on this title.
    pub fn league_logo(&self, code: u64, install_root: Option<&Path>) -> Result<Option<crate::league_logos::LeagueLogo>> {
        let db = TdbFile::parse(&self.working)?;
        let leagues = self.leagues_for(&db);
        Ok(crate::league_logos::league_logo(
            &leagues,
            self.project.leagues().logo_tables(),
            self.project.game_id(),
            Some(self.project.platform),
            code,
            install_root,
        ))
    }

    /// Phase 2.4 (C9): a team's logo packages / registry members on this title, keyed by its art
    /// id (`None` when the team has no art id — the 255 sentinel — or the title is not measured).
    pub fn team_logo_set(&self, team_id: u64, install_root: Option<&Path>) -> Result<Option<crate::team_logos::TeamLogoSet>> {
        let Some(view) = self.team_view(team_id)? else { return Ok(None) };
        let Some(artid) = view.logo_artid else { return Ok(None) };
        Ok(crate::team_logos::team_logo_set(self.project.game_id(), Some(self.project.platform), artid, install_root))
    }

    /// Phase 1.4 / P1-G: the league-keyed presentation imagery of this title (playoff-tree
    /// logos, trophies, PS4 registry types) with loose-file existence under `install_root`.
    pub fn league_imagery(&self, install_root: Option<&Path>) -> Option<crate::league_imagery::LeagueImagery> {
        crate::league_imagery::league_imagery(
            self.project.league_imagery_defs(),
            self.project.game_id()?,
            self.project.platform,
            install_root,
        )
    }

    /// Phase 1.2 (D2): the mode-setup settings catalogue of this target, read-only — NHL 15 PS4
    /// from the open database's five tables, NHL 16–21 PS4 from the install's
    /// `databigs/inxp_00.big` (`install_root` required there), every other target a blocker with
    /// the reason. Labels resolve through the attached dictionary when one is present.
    pub fn settings_catalogue(
        &self,
        install_root: Option<&Path>,
    ) -> Result<std::result::Result<crate::settings_catalogue::SettingsCatalogue, crate::settings_catalogue::SettingsBlocker>> {
        use crate::settings_catalogue as sc;
        let db = TdbFile::parse(&self.working)?;
        let defs = self.project.settings_defs();
        let dict = self.dictionary.as_ref();
        let game = self.project.game_id();
        if self.project.platform == crate::Platform::Ps4 && sc::database_has_catalogue(&db) {
            return Ok(sc::read_database(&db, self.project.meta(), defs.nhl15_ps4(), dict));
        }
        if self.project.platform == crate::Platform::Ps4 {
            if let Some(g) = game.filter(|g| matches!(*g, "nhl16" | "nhl17" | "nhl18" | "nhl19" | "nhl20" | "nhl21")) {
                let Some(root) = install_root else {
                    return Ok(Err(sc::SettingsBlocker {
                        kind: "no-install-root",
                        reason: format!("{g}: the settings catalogue is read from the install's databigs/inxp_00.big; open the project from its install folder"),
                    }));
                };
                let members = match sc::SettingCsvMembers::from_eb_archive(&sc::csv_archive_path(root)) {
                    Ok(m) => m,
                    Err(b) => return Ok(Err(b)),
                };
                return Ok(sc::read_setting_csv(g, &members, defs.nhl15_ps4(), dict));
            }
        }
        Ok(Err(sc::SettingsBlocker { kind: "unsupported-title", reason: sc::unsupported_reason(self.project.platform, game) }))
    }

    /// D3: plan the edit of one stored catalogue default on the working state (NHL 15 PS4 only;
    /// refused with the reason elsewhere). Writes nothing.
    pub fn settings_default_plan(
        &self,
        target: crate::settings_edit::DefaultTarget,
        code: i64,
    ) -> Result<std::result::Result<crate::settings_edit::DefaultEditPlan, crate::settings_edit::DefaultEditRefusal>> {
        let db = TdbFile::parse(&self.working)?;
        if self.project.platform != crate::Platform::Ps4 || !crate::settings_catalogue::database_has_catalogue(&db) {
            return Ok(Err(crate::settings_edit::DefaultEditRefusal {
                kind: "not-offered",
                reason: "stored-default editing is offered on NHL 15 PS4 databases only (the catalogue tables); NHL 16–21 wait for the CSV member packaging route, other targets have no data-side catalogue".to_string(),
            }));
        }
        Ok(crate::settings_edit::plan_default_edit(
            &db,
            self.project.meta(),
            self.project.settings_defs().nhl15_ps4(),
            self.dictionary.as_ref(),
            target,
            code,
        ))
    }

    /// Phase 1.2: every schedule table of the database with its league match.
    pub fn schedule_tables(&self) -> Result<Vec<crate::schedules::ScheduleTableSummary>> {
        let db = TdbFile::parse(&self.working)?;
        let leagues = self.leagues_for(&db);
        Ok(crate::schedules::schedule_tables(&db, self.project.meta(), &leagues))
    }

    /// Phase 1.2: one schedule table's games with slots resolved to teams, plus validation.
    pub fn schedule_view(&self, studio_table: &str) -> Result<Option<crate::schedules::ScheduleView>> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        let leagues = self.leagues_for(&db);
        let teams = crate::view::team_summaries(&db, self.project.meta(), self.project.enums(), &model, &leagues);
        crate::schedules::schedule_view(&db, self.project.meta(), &leagues, &teams, studio_table)
    }

    /// P1-A: one schedule table as rows + slot map + layout — the input of edit/import/generation
    /// plans (see [`crate::schedule_edit`]).
    pub fn schedule_model(&self, studio_table: &str) -> Result<crate::schedule_edit::ScheduleModel> {
        self.schedule_model_from_bytes(&self.working, studio_table)
    }

    /// Resolve a historical schedule against its own membership, not today's slots.
    pub fn schedule_model_from_bytes(&self, bytes: &[u8], studio_table: &str) -> Result<crate::schedule_edit::ScheduleModel> {
        let db = TdbFile::parse(bytes)?;
        let model = self.build_model(&db)?;
        let leagues = self.leagues_for(&db);
        let teams = crate::view::team_summaries(&db, self.project.meta(), self.project.enums(), &model, &leagues);
        crate::schedule_edit::read_model(&db, self.project.meta(), &leagues, &teams, studio_table)
    }

    /// P1-A: the export header identity (game id + platform) for schedule files.
    pub fn schedule_export_context(&self) -> crate::schedule_edit::ExportContext {
        crate::schedule_edit::ExportContext {
            game_id: self.project.game_id().map(str::to_string),
            platform: format!("{:?}", self.project.platform),
        }
    }

    /// The full Team detail for one `kOtt.vfEq`, or `None` if absent.
    pub fn team_view(&self, team_id: u64) -> Result<Option<crate::TeamView>> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        let leagues = self.leagues_for(&db);
        Ok(crate::view::team_view(
            &db,
            self.project.meta(),
            self.project.enums(),
            &model,
            &leagues,
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
        let leagues = self.leagues_for(&db);
        Ok(crate::view::arena_view(
            &db,
            self.project.meta(),
            self.project.enums(),
            &leagues,
            arena_id,
        ))
    }

    /// Phase 3 (22 Sep 2026) — every bio row's `(artid, headid, hasportrait)` for the
    /// portrait / head containment card, plus whether the flag column exists.
    pub fn player_bio_rows(&self) -> Result<Option<(Vec<crate::player_containment::BioRow>, bool)>> {
        let db = TdbFile::parse(&self.working)?;
        Ok(crate::player_containment::bio_rows(&db, self.project.meta()))
    }

    /// Phase 2.5 (22 Sep 2026) — the database side of the arena-asset join: the row's
    /// `arenaartid` / `bouncemap`, its tenants with their art ids, the residual id sets.
    pub fn arena_asset_inputs(&self, arena_id: u64) -> Result<Option<crate::arena_assets::ArenaAssetInputs>> {
        let db = TdbFile::parse(&self.working)?;
        Ok(crate::arena_assets::arena_asset_inputs(&db, self.project.meta(), arena_id))
    }

    /// Phase 2.5 — the arena's venue-parameter inventory (classes, constants, colours) over the
    /// current working state. `defs_root` supplies the developer-owned class map.
    pub fn arena_venue_view(
        &self,
        arena_id: u64,
        defs_root: &std::path::Path,
        install_root: Option<&std::path::Path>,
    ) -> Result<Option<crate::arena_venue::ArenaVenueView>> {
        let db = TdbFile::parse(&self.working)?;
        let map = crate::arena_venue::VenueFieldMap::load(defs_root)?;
        // The goal-horn catalogue is a PS4 install file. Its absence is not an error here — the
        // title may not ship one, or no install may be attached — so the horn simply stays a
        // number and the card says the catalogue was not found.
        let horns = install_root.and_then(|r| crate::goalhorn_catalogue::GoalhornCatalogue::load(r).ok());
        // Gen7 (P4.4): the numbered `audio/sfx` horn / chant banks of the attached install, probed
        // read-only under the R78 / R79 / R81 rules; PS4 keeps its catalogue join above.
        let horn_sources = match (install_root, self.project.platform) {
            (Some(r), p) if p != crate::Platform::Ps4 => Some(crate::gen7_horn_sources::Gen7HornSources::probe(self.project.game_id().unwrap_or(""), p, r)),
            _ => None,
        };
        // The horn row's name key resolves through this session's own attached dictionary.
        Ok(crate::view::arena_venue_view(&db, self.project.meta(), self.project.enums(), &map, horns.as_ref(), self.dictionary.as_ref(), horn_sources.as_ref(), arena_id))
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

    /// Phase 2.6 (22 Sep 2026): a team's kit rows together with the team's own `artid` from the
    /// team table — the pair `crate::kit_files::team_kits` joins. The art id is `None` when the
    /// title's team table carries no `artid` column (NHL 07) or the team is not in it.
    pub fn team_kit_rows(&self, team_id: u64) -> Result<(Option<u64>, Vec<crate::TeamJersey>)> {
        let db = TdbFile::parse(&self.working)?;
        let art = crate::kit_files::team_art_ids(&db, self.project.meta()).get(&team_id).copied();
        Ok((art, crate::team_tabs::team_jerseys(&db, team_id)))
    }

    /// Outfit filename tokens: registered jersey art ids, or the resolved stock/custom
    /// team art ids on early titles without a jersey registry.
    pub fn jersey_art_ids(&self) -> Result<std::collections::HashMap<u64, Vec<u64>>> {
        let db = TdbFile::parse(&self.working)?;
        let mut ids = crate::team_tabs::jersey_art_ids(&db);
        // Early games have no jersey registry. Their stock/custom team art id
        // differs from the exhibition index (notably after NHL07's All-Stars).
        if ids.is_empty() {
            let Some(table) = db.table_canonical("kOtt") else { return Ok(ids) };
            for team in self.build_model(&db)?.teams {
                let source = crate::view::team_name_source(&db, self.project.meta(), table, team.record);
                if let Some(art) = crate::view::read_u64_by_studio(&db, source.table, source.record, "lOnr") {
                    ids.insert(team.team_id, vec![art]);
                }
            }
        }
        Ok(ids)
    }

    /// Player ids occupying `team_id`'s roster block (`eGlu` slots), stored order, deduped. The
    /// roster source for membership-only teams (Winter Classic / all-star / national teams),
    /// whose players keep another main team — verified on the Legacy anchor: team 216 (NY Rangers
    /// WC) carries 21 roster slots but zero main-team players.
    pub fn team_roster_ids(&self, team_id: u64) -> Result<Vec<u64>> {
        let db = TdbFile::parse(&self.working)?;
        let model = self.build_model(&db)?;
        let mut seen = std::collections::HashSet::new();
        Ok(model
            .slots
            .iter()
            .filter(|s| s.team_id == team_id)
            .filter_map(|s| s.player_id)
            .filter(|p| seen.insert(*p))
            .collect())
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
        let mut raw = validate_invariants(&db, self.project.relations(), self.project.enums());
        // EA's NHL09 Centennial Canadiens (team 157) intentionally retain historical
        // captains. Exempt this team's captain-count audit only, not other findings.
        if self.project.game_id() == Some("nhl09") {
            raw.findings.retain(|f| !(f.predicate == "single_captain" && f.team_id == Some(157)));
        }
        let baseline = TdbFile::parse(self.project.db_bytes())?;
        let existing = validate_invariants(&baseline, self.project.relations(), self.project.enums());
        // Preserve severity and write guards. Explain findings that already existed
        // in the loaded/saved baseline without treating them as newly introduced edits.
        let mut report = crate::audit::humanize(
            &db,
            self.project.meta(),
            self.project.enums(),
            raw.findings,
        );
        for finding in &mut report.findings {
            if existing.findings.iter().any(|old| old.predicate == finding.predicate
                && old.table == finding.studio_table && old.field == finding.studio_field
                && old.record == finding.record && old.message == finding.technical) {
                finding.message.push_str(" This finding is already present in the loaded or last-saved file.");
            }
        }
        Ok(report)
    }

    /// Run one Layer 3 operation through the repairing-transaction harness, recording undo history.
    /// On any error the working bytes are left exactly as they were.
    fn run<R>(
        &mut self,
        op: impl FnOnce(&mut EditTransaction) -> std::result::Result<R, EditError>,
    ) -> Result<R> {
        if let Some(reason) = &self.injected_fault {
            return Err(DomainError::Validation(format!("injected fault: {reason}")));
        }
        let mut tx = EditTransaction::begin(&self.working)?;
        tx.set_allow_pre_existing_violations(self.allow_pre_existing_violations);
        let result = op(&mut tx)?;
        let new_bytes = tx.commit()?;
        // Save Editor: tables supplied from the game's database are read-only reference data.
        if let Some(frozen) = self.project.frozen_tables() {
            if let Some(id) = crate::table_overlay::frozen_changed(&self.working, &new_bytes, frozen)? {
                let name = self.project.meta().table_name(&id).unwrap_or(&id).to_string();
                return Err(DomainError::Validation(format!(
                    "This change would modify {name}, which this save does not contain. That data is shown from the game's default database and is read-only here."
                )));
            }
        }
        // A roster change list must be able to record the result (field, capacity).
        self.project.check_recordable(&new_bytes)?;
        // LEAGUE-R1 F2: the confirmed inter-table relationships are enforced at operation time
        // as a **delta** gate — an op that introduces a new dangling reference is refused and
        // the working bytes, undo history and pending state stay exactly as they were; a
        // pre-existing dangling value elsewhere never blocks an unrelated edit. Edges whose
        // tables are absent on this title are skipped by the audit itself (no PS4 rule reaches
        // gen7).
        self.refuse_new_dangling_references(&new_bytes)?;
        self.undo
            .push(std::mem::replace(&mut self.working, new_bytes));
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

    /// Discard the last committed operation as if it had never run: the working bytes go back
    /// to their previous state and **nothing is pushed on the redo stack**. Used by the
    /// workspace when the mirrored half of a coordinated op is refused — an `undo()` there would
    /// leave the unmirrored half as a redoable entry (P1 audit, 21 Sep 2026). Returns `false`
    /// if there is nothing to discard.
    pub fn abort_last(&mut self) -> bool {
        match self.undo.pop() {
            Some(prev) => {
                self.working = prev;
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

    /// Compare every confirmed relationship edge before and after an operation and refuse the
    /// operation if any edge gained a dangling reference. The comparison is a multiset of
    /// `(row key, value)` identities (see [`crate::referential`]) — never a count — so a
    /// substitution of one invalid value for another, or a violation moved to a different row
    /// while an old one is repaired, is refused even when the total stays equal or drops. The
    /// message names the entity, the field and the target table in human terms.
    fn refuse_new_dangling_references(&self, new_bytes: &[u8]) -> Result<()> {
        let before = TdbFile::parse(&self.working)?;
        let after = TdbFile::parse(new_bytes)?;
        let meta = self.project.meta();
        // NHL07 has a polymorphic arena selector rather than kOtt's direct link.
        // Validate it after every transaction, including bulk/package replay.
        let invalid_arenas = |db: &TdbFile<'_>| {
            let mut invalid = std::collections::HashSet::new();
            if db.table_canonical("cubz").is_none() { return invalid; }
            if let Some(selector) = db.table_canonical("StEO") {
                let read = |r, id| crate::view::read_u64_by_studio(db, selector, r, id);
                for row in 0..selector.current_records as u32 {
                    let (Some(index), Some(kind), Some(arena)) = (read(row, "vfEq"), read(row, "RREB"), read(row, "buDa")) else { continue; };
                    let table = match kind { 0 => db.table_canonical("cubz"), 1 => db.table_canonical("SulW"), _ => None };
                    let exists = table.is_some_and(|t| (0..t.current_records as u32).any(|r| crate::view::read_u64_by_studio(db, t, r, "vfEq") == Some(arena)));
                    if !exists { invalid.insert((index, kind, arena)); }
                }
            }
            invalid
        };
        if invalid_arenas(&after).difference(&invalid_arenas(&before)).next().is_some() {
            return Err(DomainError::Validation("Choose an arena that exists in this game's venue list. The edit was not applied.".into()));
        }
        let Some(v) = crate::referential::new_dangling_references(
            &before,
            &after,
            self.project.relations(),
            meta,
        )
        .into_iter()
        .next() else {
            return Ok(());
        };
        let field_name = after
            .table_canonical(&v.source_table)
            .and_then(|t| {
                t.fields
                    .iter()
                    .find(|f| f.id.canonical(after.endian) == v.source_field)
                    .and_then(|f| meta.field(&t.id.raw(), &f.id.raw()))
                    .map(|m| m.name.clone())
            })
            .unwrap_or_else(|| v.source_field.clone());
        let target_name = after
            .table_canonical(&v.target_table)
            .and_then(|t| meta.table_name(&t.id.raw()).map(str::to_string))
            .unwrap_or_else(|| v.target_table.clone());
        let entity = self.entity_label(&after, &v.source_table, v.record);
        Err(DomainError::Validation(format!(
            "{entity}: {field_name} = {} refers to no {target_name} row on this title ({} relationship); the edit was not applied",
            v.value, v.relationship
        )))
    }

    /// A human label for a record of a table (team name, player name, league display, or
    /// `<table> #<record>`), for refusal messages.
    fn entity_label(&self, db: &TdbFile, studio_table: &str, record: u32) -> String {
        let meta = self.project.meta();
        let fallback = || {
            let tn = db
                .table_canonical(studio_table)
                .and_then(|t| meta.table_name(&t.id.raw()).map(str::to_string))
                .unwrap_or_else(|| studio_table.to_string());
            format!("{tn} #{record}")
        };
        match studio_table {
            tdb_relations::entity::ids::TEAM => db
                .table_canonical(studio_table)
                .map(|kott| {
                    let src = crate::view::team_name_source(db, meta, kott, record);
                    let pick = ["fullname", "teamname", "abbrname"]
                        .iter()
                        .map(|n| crate::view::read_str_by_name(db, meta, src.table, src.record, n))
                        .find(|v| !v.trim().is_empty());
                    match pick {
                        Some(v) => format!("Team {}", v.trim()),
                        None => fallback(),
                    }
                })
                .unwrap_or_else(fallback),
            tdb_relations::entity::ids::BIO => db
                .table_canonical(studio_table)
                .map(|bio| {
                    let first = crate::view::read_str_by_name(db, meta, bio, record, "firstname");
                    let last = crate::view::read_str_by_name(db, meta, bio, record, "lastname");
                    let name = format!("{} {}", first.trim(), last.trim())
                        .trim()
                        .to_string();
                    if name.is_empty() {
                        fallback()
                    } else {
                        format!("Player {name}")
                    }
                })
                .unwrap_or_else(fallback),
            _ => fallback(),
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
        if self.project.game_id() == Some("nhl07") && studio_table == "HqzU" && matches!(studio_field, "xMjj" | "rasz" | "xgeX") {
            return Err(DomainError::Validation("Use League and division on the team's Team Data page to keep organization and schedules together.".into()));
        }
        let prepared =
            self.prepare_field_edit(studio_table, record, studio_field, value.clone())?;
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

    /// Replay an explicit package change set. Apply the normal scalar admission checks to every
    /// cell first, then audit the final state once. Implicit mirrors are already explicit cells
    /// in the package and must not introduce additional writes during replay.
    /// Change membership and translate league-ranked fixtures in one undoable transaction.
    pub(crate) fn apply_memberships(&mut self, plans: &[crate::membership::MembershipPlan], swaps: &[(u64,u64)]) -> Result<OpOutcome> {
        let db=TdbFile::parse(&self.working)?;
        let mut proposed=self.working.clone(); let mut cells=Vec::new();
        for plan in plans {
            let table=db.table_canonical(&plan.table).ok_or_else(|| DomainError::Validation("Membership table unavailable.".into()))?;
            for (id,value) in &plan.writes {
                let f=table.fields.iter().find(|f|f.id.canonical(db.endian)==*id).ok_or_else(||DomainError::Validation("Membership field unavailable.".into()))?;
                let meta=self.project.meta().field(&table.id.raw(),&f.id.raw()).ok_or_else(||DomainError::Validation("Membership metadata unavailable.".into()))?;
                meta.validate(*value).map_err(EditError::Invalid)?;
                let encoded=meta.encode(*value);
                if !db.set_u64(&mut proposed,table,plan.record,f,encoded) {return Err(DomainError::Validation("Membership record unavailable.".into()));}
                cells.push((plan.table.clone(),plan.record,id.clone(),encoded));
            }
        }
        let mut schedules=Vec::new();
        for table in self.schedule_tables()? {
            if table.league_code.is_none(){continue;}
            let old=self.schedule_model(&table.studio_table)?;
            if !crate::schedule_edit::supports_resize(&old){continue;}
            let new=self.schedule_model_from_bytes(&proposed,&table.studio_table)?;
            if old.slots.fingerprint==new.slots.fingerprint {continue;}
            let translate=|slot:u64|->Result<u64>{
                let team=old.slots.by_slot(slot).ok_or_else(||DomainError::Validation("An existing fixture has an unknown team.".into()))?.team_id;
                let replacement=swaps.iter().find_map(|(a,b)|if *a==team{Some(*b)}else if *b==team{Some(*a)}else{None}).unwrap_or(team);
                new.slots.by_team(replacement).map(|t|t.slot).ok_or_else(||DomainError::Validation("This team has scheduled games. Swap it with a team in the destination league to preserve the schedule.".into()))
            };
            let mut rows=old.rows.clone();for r in &mut rows{r.home_slot=translate(r.home_slot)?;r.away_slot=translate(r.away_slot)?;}
            schedules.push((table.studio_table,crate::schedule_edit::replacement_cells(&new,&rows)?));
        }
        self.run(|tx|{
            let mut out=OpOutcome::default();
            for (table,record,field,value) in cells {out.edits+=tx.set_cells_int_many(&table,&[(record,field,value)])?.edits;}
            for (table,rows) in schedules {out.edits+=tx.replace_integer_rows(&table,&rows)?.edits;}
            Ok(out)
        })
    }

    /// Organization and schedule slots are one linked overview change.
    pub fn restore_competition(&mut self, baseline:&[u8]) -> Result<OpOutcome> {
        if self.project.game_id()!=Some("nhl07"){return Err(DomainError::Validation("Unsupported competition layout.".into()));}
        let old=TdbFile::parse(baseline)?;let now=TdbFile::parse(&self.working)?;
        let before=old.table_canonical("HqzU").ok_or_else(||DomainError::Validation("Original teams unavailable.".into()))?;
        let current=now.table_canonical("HqzU").ok_or_else(||DomainError::Validation("Current teams unavailable.".into()))?;
        if before.current_records!=current.current_records{return Err(DomainError::Validation("Restore team record changes before restoring organization.".into()));}
        let mut cells=Vec::new();
        for id in ["xMjj","rasz","xgeX"]{
            let f=before.fields.iter().find(|f|f.id.canonical(old.endian)==id).ok_or_else(||DomainError::Validation("Original organization field unavailable.".into()))?;
            for row in 0..before.current_records as u32 {cells.push((row,id.to_owned(),old.read_u64(before,row,f).ok_or_else(||DomainError::Validation("Original organization record unavailable.".into()))?));}
        }
        let mut schedules=Vec::new();for t in self.schedule_tables()?{
            if t.league_code.is_none(){continue;}
            let model=self.schedule_model_from_bytes(baseline,&t.studio_table)?;
            if crate::schedule_edit::supports_resize(&model){schedules.push((t.studio_table,crate::schedule_edit::replacement_cells(&model,&model.rows)?));}
        }
        self.run(|tx|{let mut out=tx.set_cells_int_many("HqzU",&cells)?;for (table,rows) in schedules{out.edits+=tx.replace_integer_rows(&table,&rows)?.edits;}Ok(out)})
    }

    pub fn replace_schedules(&mut self, schedules: &[(String, Vec<crate::schedule_edit::GameRow>)]) -> Result<OpOutcome> {
        if self.project.game_id() != Some("nhl07") { return Err(DomainError::Validation("Schedule resizing is qualified for NHL07 only.".into())); }
        let mut prepared = Vec::new();
        for (table,rows) in schedules {
            let model=self.schedule_model(table)?;
            let validation=crate::schedule_edit::validate(&model,rows,None);
            if !validation.ok() { return Err(DomainError::Validation(validation.errors.join("; "))); }
            prepared.push((table.clone(),crate::schedule_edit::replacement_cells(&model,rows)?));
        }
        self.run(|tx| { let mut out=OpOutcome::default(); for (table,rows) in prepared { out.edits += tx.replace_integer_rows(&table,&rows)?.edits; } Ok(out) })
    }

    pub fn set_package_fields(&mut self, cells: &[(String, u32, String, FieldValue)]) -> Result<OpOutcome> {
        self.set_project_revert(cells, None)
    }

    /// Restore selected baseline cells and the associated free-agent membership atomically.
    pub fn set_project_revert(&mut self, cells: &[(String, u32, String, FieldValue)], pool: Option<&[u64]>) -> Result<OpOutcome> {
        // All admission checks inspect the same unchanged database; parse it once per batch.
        let db = TdbFile::parse(&self.working)?;
        // Calendar validation needs the complete proposed birthday, not an intermediate
        // date such as February 31 while replaying month and day in field order.
        let mut calendar_bytes = self.working.clone();
        for (table, record, field, value) in cells {
            if table != "ubPc" { continue; }
            let Some(t) = db.table_canonical(table) else { continue };
            let Some(f) = t.fields.iter().find(|f| f.id.canonical(db.endian) == *field) else { continue };
            let Some(m) = self.project.meta().field(&t.id.raw(), &f.id.raw()) else { continue };
            if matches!(m.name.as_str(), "year" | "month" | "day") {
                if let FieldValue::Int(v) = value {
                    m.validate(*v).map_err(EditError::Invalid)?;
                    if !db.set_u64(&mut calendar_bytes, t, *record, f, m.encode(*v)) {
                        return Err(DomainError::Validation("Invalid birth-date record".into()));
                    }
                }
            }
        }
        let art_cells = crate::team_reid::package_art_cells(self, cells)?;
        // Roster block repair reorders whole rows, including their immutable slot
        // keys. Admit only a permutation of the existing keys in batch replay;
        // ordinary scalar edits still cannot rewrite identities.
        let slot_keys = cells.iter().any(|(t, _, f, _)| t == "eGlu" && f == "toWX");
        if slot_keys {
            let t = db.table_canonical("eGlu").ok_or_else(|| DomainError::Validation("Missing roster slots".into()))?;
            let f = t.fields.iter().find(|f| f.id.canonical(db.endian) == "toWX")
                .ok_or_else(|| DomainError::Validation("Missing roster slot key".into()))?;
            let mut before: Vec<u64> = (0..u32::from(t.current_records)).map(|r| db.read_u64(t, r, f).unwrap()).collect();
            let mut after = before.clone();
            let mut seen = std::collections::HashSet::new();
            for (table, record, field, value) in cells {
                if table != "eGlu" || field != "toWX" { continue; }
                let FieldValue::Int(v) = value else { return Err(DomainError::Validation("Roster slot key must be an integer".into())); };
                if *v < 0 || *record as usize >= after.len() || !seen.insert(*record) {
                    return Err(DomainError::Validation("Invalid roster slot key permutation".into()));
                }
                after[*record as usize] = *v as u64;
            }
            before.sort_unstable(); after.sort_unstable();
            if before != after { return Err(DomainError::Validation("Package roster slot keys must preserve existing identities".into())); }
        }
        let proposed_calendar = TdbFile::parse(&calendar_bytes)?;
        let mut prepared = Vec::with_capacity(cells.len());
        for (table, record, field, value) in cells {
            if slot_keys && table == "eGlu" && field == "toWX" {
                let FieldValue::Int(v) = value else { unreachable!() };
                prepared.push((table.clone(), *record, field.clone(), tdb_edit::txn::ImportCell::Int(*v as u64)));
                continue;
            }
            if art_cells.contains(&(table.clone(), *record, field.clone())) {
                let FieldValue::Int(v) = value else { unreachable!() };
                prepared.push((table.clone(), *record, field.clone(), tdb_edit::txn::ImportCell::Int(*v as u64)));
                continue;
            }
            // NHL07's roster occupant is part of a composite key. Release/sign may
            // change it, but ordinary scalar editing must still refuse key writes.
            if self.project.game_id() == Some("nhl07") && table == "eGlu" && field == "XSWT" {
                let FieldValue::Int(v) = value else { return Err(DomainError::Validation("Roster occupant must be an integer".into())); };
                let slots = db.table_canonical("eGlu").ok_or_else(|| DomainError::Validation("Missing roster slots".into()))?;
                let index = db.table_canonical("ZBac").ok_or_else(|| DomainError::Validation("Missing player index".into()))?;
                let key = index.fields.iter().find(|f| f.id.canonical(db.endian) == "vfEq").ok_or_else(|| DomainError::Validation("Missing player index key".into()))?;
                if *record >= slots.current_records as u32 || *v < 0 || (*v != 0 && !(0..index.current_records as u32).any(|r| db.read_u64(index, r, key) == Some(*v as u64))) {
                    return Err(DomainError::Validation("Invalid roster occupant reference".into()));
                }
                prepared.push((table.clone(), *record, field.clone(), tdb_edit::txn::ImportCell::Int(*v as u64)));
                continue;
            }
            let p = self.prepare_field_edit_in(&proposed_calendar, table, *record, field, value.clone())?;
            let value = match p.payload {
                PreparedPayload::Int(v) => tdb_edit::txn::ImportCell::Int(v),
                PreparedPayload::Text(v) => tdb_edit::txn::ImportCell::Str(v),
            };
            prepared.push((p.table, p.record, p.field, value));
        }
        self.run(|tx| {
            let mut result = tx.set_cells_atomic(&prepared)?;
            if let Some(pool) = pool {
                let other = tx.set_free_agent_pool(pool)?;
                result.edits += other.edits;
                result.repairs.extend(other.repairs);
            }
            Ok(result)
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
        // One op is one set of distinct cells: an empty set or a field named twice is a caller
        // error, refused before anything is prepared (P1 audit, 21 Sep 2026).
        if writes.is_empty() {
            return Err(DomainError::Validation("a multi-cell write needs at least one cell".into()));
        }
        for (i, (field, _)) in writes.iter().enumerate() {
            if writes[..i].iter().any(|(f, _)| f == field) {
                return Err(DomainError::Validation(format!(
                    "field {field} is written twice in one multi-cell write"
                )));
            }
        }
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
                    PreparedPayload::Int(v) => tx.set_cell_int(&p.table, p.record, &p.field, *v)?,
                    PreparedPayload::Text(s) => tx.set_cell_str(&p.table, p.record, &p.field, s)?,
                };
                total.edits += outcome.edits;
                total.repairs.extend(outcome.repairs);
            }
            Ok(total)
        })
    }

    /// Stage integer cell edits across **many records** of one table as a single transaction
    /// (one undo entry). Every `(record, field, value)` is range/updatable-checked and encoded
    /// against the metadata exactly like [`Self::set_field`], with the database parsed once for
    /// the whole batch; the write itself goes through the repairing transaction's bulk path, so
    /// a 1 231-row schedule import is one audit, one undo step and one pending-review delta. Any
    /// rejected cell leaves the working bytes, history and pending state untouched.
    pub fn set_cells_int_rows(
        &mut self,
        studio_table: &str,
        rows: &[(u32, Vec<(String, i64)>)],
    ) -> Result<OpOutcome> {
        let view = self.working.clone();
        let db = TdbFile::parse(&view)?;
        let table = db
            .table_canonical(studio_table)
            .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?;
        let mut cells: Vec<(u32, String, u64)> = Vec::new();
        for (record, writes) in rows {
            if *record >= table.current_records as u32 {
                return Err(EditError::OutOfRange {
                    table: studio_table.to_string(),
                    record: *record,
                }
                .into());
            }
            for (studio_field, value) in writes {
                let field = table
                    .fields
                    .iter()
                    .find(|f| f.id.canonical(db.endian) == *studio_field)
                    .ok_or_else(|| EditError::UnknownField {
                        table: studio_table.to_string(),
                        field: studio_field.to_string(),
                    })?;
                let meta = self
                    .project
                    .meta()
                    .field(&table.id.raw(), &field.id.raw())
                    .ok_or_else(|| EditError::NoMeta {
                        table: studio_table.to_string(),
                        field: studio_field.to_string(),
                    })?;
                if meta.key {
                    return Err(EditError::Invalid(ValidationError::NotUpdatable {
                        field: studio_field.to_string(),
                    })
                    .into());
                }
                if let Some(tn) = self.project.meta().table_name(&table.id.raw()) {
                    if crate::team_reid::is_guarded_identity(tn, &meta.name) {
                        return Err(DomainError::Validation(crate::team_reid::guarded_identity_reason(tn, &meta.name)));
                    }
                }
                meta.validate(*value).map_err(EditError::Invalid)?;
                cells.push((*record, studio_field.clone(), meta.encode(*value)));
            }
        }
        if cells.is_empty() {
            return Ok(OpOutcome::default());
        }
        let table_id = studio_table.to_string();
        self.run(move |tx| tx.set_cells_int_many(&table_id, &cells))
    }

    /// Stage integer cell edits across **several tables** as a single transaction (one undo
    /// entry): every cell of every table is range / updatable-checked and encoded against the
    /// metadata first; the write runs through the repairing transaction's bulk path table by
    /// table inside one commit, so a refusal anywhere leaves the working bytes, history and
    /// pending state untouched. Phase 2.1 (21 Sep 2026): the art-id swap cascade.
    pub fn set_cells_int_tables(&mut self, tables: &[(String, Vec<(u32, String, i64)>)]) -> Result<OpOutcome> {
        if tables.is_empty() || tables.iter().all(|(_, rows)| rows.is_empty()) {
            return Err(DomainError::Validation("a multi-table write needs at least one cell".into()));
        }
        let view = self.working.clone();
        let db = TdbFile::parse(&view)?;
        let mut prepared: Vec<(String, Vec<(u32, String, u64)>)> = Vec::new();
        for (studio_table, rows) in tables {
            let table = db
                .table_canonical(studio_table)
                .ok_or_else(|| EditError::UnknownTable(studio_table.to_string()))?;
            let mut cells: Vec<(u32, String, u64)> = Vec::new();
            for (record, studio_field, value) in rows {
                if *record >= table.current_records as u32 {
                    return Err(EditError::OutOfRange { table: studio_table.to_string(), record: *record }.into());
                }
                let field = table
                    .fields
                    .iter()
                    .find(|f| f.id.canonical(db.endian) == *studio_field)
                    .ok_or_else(|| EditError::UnknownField { table: studio_table.to_string(), field: studio_field.to_string() })?;
                let meta = self
                    .project
                    .meta()
                    .field(&table.id.raw(), &field.id.raw())
                    .ok_or_else(|| EditError::NoMeta { table: studio_table.to_string(), field: studio_field.to_string() })?;
                if meta.key {
                    return Err(EditError::Invalid(ValidationError::NotUpdatable { field: studio_field.to_string() }).into());
                }
                meta.validate(*value).map_err(EditError::Invalid)?;
                cells.push((*record, studio_field.clone(), meta.encode(*value)));
            }
            if !cells.is_empty() {
                prepared.push((studio_table.clone(), cells));
            }
        }
        self.run(move |tx| {
            let mut total = OpOutcome::default();
            for (table, cells) in &prepared {
                let o = tx.set_cells_int_many(table, cells)?;
                total.edits += o.edits;
                total.repairs.extend(o.repairs);
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
        let db = TdbFile::parse(&self.working)?;
        self.prepare_field_edit_in(&db, studio_table, record, studio_field, value)
    }

    fn prepare_field_edit_in(
        &self,
        db: &TdbFile<'_>,
        studio_table: &str,
        record: u32,
        studio_field: &str,
        value: FieldValue,
    ) -> Result<PreparedEdit> {
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

        // NHL07 attendance bounds and unqualified settings apply to packages as well as UI edits.
        if self.project.game_id() == Some("nhl07") && studio_table == "cubz" {
            if matches!(meta.name.as_str(), "icequality" | "crowdtype" | "crease") {
                return Err(DomainError::Validation("This arena setting's NHL 07 behavior has not been verified.".into()));
            }
            if let FieldValue::Int(v) = &value {
                if meta.name == "maxattendance" && *v < 1 {
                    return Err(DomainError::Validation("Capacity must be at least one seat.".into()));
                }
                if meta.name == "avgattendance" && !(0..=100).contains(v) {
                    return Err(DomainError::Validation("Average attendance must be between 0 and 100 percent.".into()));
                }
            }
        }

        // Encoding contract (25 Sep 2026): a cell whose meaning is title-encoded (weight, six-bit
        // ratings, birth month / day, contract dollars) refuses to write while the declared title and
        // the schema disagree, or while no family claims the schema — the offset it would be
        // written with is not established.
        if crate::encoding::is_contract_encoded(&db, self.project.meta(), studio_table, studio_field) {
            if let Some(reason) = crate::encoding::encoding_conflict(
                &db,
                self.project.meta(),
                self.project.game_id(),
                self.project.platform,
            ) {
                return Err(EditError::Refused(format!("{} was not written: {reason}", meta.name)).into());
            }
            if crate::encoding::player_encoding(&db, self.project.meta()).is_none() {
                return Err(EditError::Refused(format!(
                    "{} was not written: {}",
                    meta.name,
                    crate::encoding::UNRESOLVED_NOTE
                ))
                .into());
            }
        }
        // Phase 2.1 (21 Sep 2026): a guarded identity (the team art id) is never a bare cell edit
        // either — its dependents would dangle; the swap operation is the supported path.
        if let Some(tn) = self.project.meta().table_name(&table.id.raw()) {
            if crate::team_reid::is_guarded_identity(tn, &meta.name) {
                return Err(DomainError::Validation(crate::team_reid::guarded_identity_reason(tn, &meta.name)));
            }
        }
        // Keys/identity fields are never editable as scalars — even when the schema marks them
        // `update=True` (e.g. NHL07 `vfEq`). Editing a primary key in place would orphan its foreign
        // keys; the structural operations that legitimately assign ids mint them through the Layer-1
        // path, never this validated scalar seam. (Presentation mirrors this: `FieldView.editable`
        // is `updatable && !key`.)
        if meta.key {
            return Err(EditError::Invalid(ValidationError::NotUpdatable {
                field: studio_field.to_string(),
            })
            .into());
        }

        let payload = match value {
            FieldValue::Int(i) => {
                meta.validate(i).map_err(EditError::Invalid)?;
                if studio_table == "ubPc" && matches!(meta.name.as_str(), "year" | "month" | "day") {
                    if let Some(enc) = crate::encoding::player_encoding(db, self.project.meta()) {
                        let read_part = |name: &str| -> Option<(i64, u32)> {
                            let f = table.fields.iter().find(|f| self.project.meta().field(&table.id.raw(), &f.id.raw()).is_some_and(|m| m.name == name))?;
                            let m = self.project.meta().field(&table.id.raw(), &f.id.raw())?;
                            Some((if name == meta.name { i } else { m.decode_raw(db.read_u64(table, record, f)?) }, f.bit_width))
                        };
                        if let (Some((year, width)), Some((month, _)), Some((day, _))) = (read_part("year"), read_part("month"), read_part("day")) {
                            let year = year + if width <= 6 { 1959 } else { 1900 };
                            let month = month + enc.birthdate.offset();
                            let day = day + enc.birthdate.offset();
                            let days = match month {
                                4 | 6 | 9 | 11 => 30,
                                2 => if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 29 } else { 28 },
                                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                                _ => 0,
                            };
                            if day < 1 || day > days {
                                return Err(EditError::Refused("Choose a valid birth date. For a shorter month, change the day first.".into()).into());
                            }
                        }
                    }
                }
                if crate::attributes::rating_offset(studio_table, &meta.name, meta.range_low, meta.range_high).is_some() {
                    if let Some(enc) = crate::encoding::player_encoding(db, self.project.meta()) {
                        let offset = if field.bit_width == 6 { enc.sixbit_rating_offset } else { 0 };
                        if i + offset > 99 {
                            return Err(EditError::Refused("Attribute ratings cannot exceed 99.".into()).into());
                        }
                    }
                }
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
                    return Err(EditError::TextTooLong {
                        field: meta.name.clone(),
                        bytes: s.len(),
                        max: width_bytes,
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
        self.require_slot_key("create a player")?;
        self.run(|tx| tx.create_player(team, position))
    }

    /// Adding a player to a roster mints a slot key in the roster-lines table. NHL 07's roster
    /// table has no such column, so every operation that adds a roster slot (create, clone, the
    /// historical import) is refused **before** the transaction, with the reason in words —
    /// previously the engine surfaced the raw `unknown field "toWX" in table "eGlu"` (Players 3.2,
    /// 25 Sep 2026). Titles with the column are unaffected.
    fn require_slot_key(&self, what: &str) -> Result<()> {
        let db = TdbFile::parse(&self.working)?;
        let lines = tdb_relations::entity::ids::ROSTER_LINES;
        let has_key = db.table_canonical(lines).is_some_and(|t| {
            t.fields
                .iter()
                .any(|f| f.id.canonical(db.endian) == tdb_relations::entity::ids::SLOT_KEY)
        });
        if has_key {
            Ok(())
        } else {
            Err(EditError::Refused(format!(
                "cannot {what} on this title: its roster table carries no slot-key column. Creating, cloning, deleting and importing new player records are not supported here; existing players stay editable"
            ))
            .into())
        }
    }

    /// The semantic import seams (lowering human values, applying lowered ops) refuse while the
    /// declared title disagrees with the schema (review 2026-09-26 F2). The raw bulk routes
    /// (`set_cells_int*`) write stored values as given and are deliberately not gated; the
    /// per-field "unresolved schema -> skip" rule lives in the lowering itself.
    fn require_encoding_agreement(&self, what: &str) -> Result<()> {
        let db = TdbFile::parse(&self.working)?;
        if let Some(reason) = crate::encoding::encoding_conflict(
            &db,
            self.project.meta(),
            self.project.game_id(),
            self.project.platform,
        ) {
            return Err(EditError::Refused(format!("cannot {what}: {reason}")).into());
        }
        Ok(())
    }

    /// Apply a whole Roster Builder cart — many creates/updates — as **one** transaction and **one**
    /// undo entry (Spike S3). The `ops` carry already-encoded cell values; `lower()` (Phase 5.2)
    /// builds them from the corpus. Preview and commit run the identical `ops`, so they lower to
    /// byte-identical edits.
    pub fn apply_import(
        &mut self,
        ops: &[tdb_edit::txn::ImportOp],
    ) -> Result<tdb_edit::txn::ImportApplied> {
        if ops.iter().any(|op| matches!(op, tdb_edit::txn::ImportOp::Create { .. })) {
            self.require_slot_key("import a new player")?;
        }
        self.require_encoding_agreement("apply the import")?;
        self.run(|tx| tx.apply_import(ops))
    }

    /// **Dry-run** a cart: lower + apply the ops against a throwaway transaction and report what commit
    /// *would* do — player-count delta, the audit over the resulting state, the skip report, and the
    /// exact bytes commit would write — **without** mutating this session (Phase 5.2 preview). Because
    /// [`commit_import`](Self::commit_import) re-runs the same ops from the same state, `previewed` is
    /// byte-identical to the committed db.
    pub fn preview_import(
        &self,
        ops: &[tdb_edit::txn::ImportOp],
        skipped: Vec<crate::import::SkippedField>,
    ) -> Result<crate::import::ImportPreview> {
        let players_before = self
            .build_model(&TdbFile::parse(&self.working)?)?
            .players
            .len();

        let mut tx = EditTransaction::begin(&self.working)?;
        tx.set_allow_pre_existing_violations(self.allow_pre_existing_violations);
        tx.apply_import(ops)?;
        let previewed = tx.commit()?; // finalize CRCs → exactly what commit writes

        let db = TdbFile::parse(&previewed)?;
        let players_after = self.build_model(&db)?.players.len();
        let raw = validate_invariants(&db, self.project.relations(), self.project.enums());
        let audit =
            crate::audit::humanize(&db, self.project.meta(), self.project.enums(), raw.findings);
        Ok(crate::import::ImportPreview {
            players_before,
            players_after,
            audit,
            skipped,
            previewed,
        })
    }

    /// Commit a previewed cart for real — the same ops, one transaction, one undo entry. Alias of
    /// [`apply_import`](Self::apply_import), named for the two-phase pipeline. Returns the op outcome
    /// plus every newly-created `wBIz` (for the ledger + the UI result).
    pub fn commit_import(
        &mut self,
        ops: &[tdb_edit::txn::ImportOp],
    ) -> Result<tdb_edit::txn::ImportApplied> {
        self.apply_import(ops)
    }

    /// Auto-detect EA identity matches for a corpus person — the Roster Builder's Match/Create toggle,
    /// and the first read that bridges the corpus and the open project. Reads the person's bio from the
    /// corpus, folds it with the **same** slug normalization the [`EaSlugIndex`](crate::import::EaSlugIndex)
    /// uses, and returns ranked candidates (name+DOB high-confidence first, then same-name review
    /// candidates). Empty ⇒ no match ⇒ the cart should Create. Identity is season-independent (name +
    /// DOB), so no season year is needed. Builds the index fresh; caching it across a cart is 5.4's job.
    pub fn match_ea_player(
        &self,
        corpus: &crate::historical::FhmCorpus,
        person_id: i64,
    ) -> Result<Vec<crate::import::MatchCandidate>> {
        let Some(detail) = corpus.person_detail(person_id)? else {
            return Ok(Vec::new());
        };
        let index = self.ea_slug_index()?;
        Ok(index.match_player(&detail.last_name, &detail.first_name, Some(&detail.dob)))
    }

    /// Lower a whole Roster Builder **cart** against the working state — the corpus→EA orchestration
    /// (Phase 5.2b). Resolves each [`ImportItem`](crate::import::ImportItem) through the corpus adapter
    /// and lowers it to an encoded op; returns the [`LoweredChangeSet`](crate::import::LoweredChangeSet)
    /// whose `ops` feed [`preview_import`](Self::preview_import) then [`commit_import`](Self::commit_import).
    /// Lowering **once** and passing the same `ops` to both keeps preview and commit byte-identical.
    /// This is the first read that bridges the EA project and the FHM corpus.
    pub fn lower_change_set(
        &self,
        corpus: &crate::historical::FhmCorpus,
        items: &[crate::import::ImportItem],
        profile: &tdb_formula::FormulaProfile,
        maxima: &tdb_fhm_store::AttributeMaxima,
    ) -> Result<crate::import::LoweredChangeSet> {
        self.require_encoding_agreement("lower the import")?;
        let db = TdbFile::parse(&self.working)?;
        Ok(crate::import::lower_change_set(
            corpus,
            items,
            profile,
            maxima,
            &db,
            self.project.meta(),
            self.project.enums(),
        )?)
    }

    /// Clone an existing player onto a team (deep-copy of bio + companions); returns the new `wBIz`.
    pub fn clone_player(&mut self, source_player: u64, to_team: u64) -> Result<(u64, OpOutcome)> {
        self.require_slot_key("clone a player")?;
        self.run(|tx| tx.clone_player(source_player, to_team))
    }

    /// Delete a player and its whole cluster.
    pub fn delete_player(&mut self, player: u64) -> Result<OpOutcome> {
        self.require_slot_key("delete a player")?;
        self.run(|tx| tx.delete_player(player))
    }

    /// Move a player's roster membership from `from_team` to another team.
    pub fn move_player(&mut self, player: u64, from_team: u64, to_team: u64) -> Result<OpOutcome> {
        let r = self.run(|tx| tx.move_player(player, from_team, to_team));
        self.name_full_roster(r, "move")
    }

    /// A destination roster block with no free slot refuses as [`EditError::TeamRosterFull`], which
    /// carries only the team id. Resolve it to the team's name here (the layer that has the name
    /// data), so the surface reads "Anaheim Ducks has no free roster slot …" rather than
    /// "team 12 roster block is full" (Players 3.2, 25 Sep 2026). Other errors pass through.
    fn name_full_roster<R>(&self, r: Result<R>, what: &str) -> Result<R> {
        match r {
            Err(DomainError::Edit(EditError::TeamRosterFull { team })) => {
                // The team's resolved name; its abbreviation when the name is a dictionary key this
                // project cannot resolve (PS4 16–21 without an attached dictionary); the id last.
                let name = self
                    .team_summaries()
                    .ok()
                    .and_then(|ts| ts.into_iter().find(|t| t.team_id == team))
                    .map(|t| {
                        if !t.name.trim().is_empty() {
                            t.name
                        } else if !t.abbrev.trim().is_empty() {
                            format!("{} (team #{team}; name not in this project's dictionary)", t.abbrev.trim())
                        } else {
                            format!("team #{team}")
                        }
                    })
                    .unwrap_or_else(|| format!("team #{team}"));
                Err(EditError::Refused(format!(
                    "{name} has no free roster slot, so the {what} was not applied: on this title a team's roster block holds a fixed number of slots and every one is occupied. Release or move a player off {name} first"
                ))
                .into())
            }
            other => other,
        }
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

        let r = self.run(|tx| tx.sign_from_fa(player, to_team));
        self.name_full_roster(r, "signing")
    }

    /// Add an extra roster membership (national / all-star / alternate roster) on `to_team`,
    /// leaving the player's primary team untouched.
    pub fn add_membership(&mut self, player: u64, to_team: u64) -> Result<OpOutcome> {
        let r = self.run(|tx| tx.add_membership(player, to_team));
        self.name_full_roster(r, "added membership")
    }

    pub fn set_free_agent_pool(&mut self, players: &[u64]) -> Result<OpOutcome> {
        self.run(|tx| tx.set_free_agent_pool(players))
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

    /// Clone an `xjgw` jersey row into a new kit on `team`; returns the new record index (see
    /// [`EditTransaction::clone_jersey`]).
    pub fn clone_jersey(
        &mut self,
        source_record: u32,
        team: u64,
        style: u64,
        art_id: u64,
        variant: u64,
        jersey_index: Option<u64>,
    ) -> Result<(u32, OpOutcome)> {
        self.run(|tx| tx.clone_jersey(source_record, team, style, art_id, variant, jersey_index))
    }

    /// Trade a future draft pick (`qHav` row) to another team.
    pub fn trade_draft_pick(&mut self, record: u32, to_team: u64) -> Result<OpOutcome> {
        self.run(|tx| tx.trade_draft_pick(record, to_team))
    }

    /// Auto-correct every duplicate jersey number in the DB (the default reassignment logic), returning
    /// the corrections applied. A no-op when there are none — and, in that case, it deliberately does
    /// **not** run a transaction, so it leaves no undo entry and doesn't mark the project dirty.
    pub fn autocorrect_duplicate_jerseys(
        &mut self,
    ) -> Result<(OpOutcome, Vec<JerseyReassignment>)> {
        // Peek read-only: nothing to fix → return without touching the working bytes or undo history.
        let peek = EditTransaction::begin(&self.working)?;
        if peek.plan_jersey_corrections()?.is_empty() {
            return Ok((OpOutcome::default(), Vec::new()));
        }
        self.run(|tx| tx.autocorrect_duplicate_jerseys())
    }

    /// Repair a corrupted roster-block layout (`towx_coherence` errors): permute the `eGlu` records
    /// back into contiguous per-team blocks keyed by the team label each row already carries. An
    /// order-only structural repair — no field value changes, so no membership can change (see
    /// [`EditTransaction::repair_roster_blocks`]). A no-op on a coherent layout — and, like the
    /// jersey autocorrect, the no-op deliberately skips the transaction so it leaves no undo entry
    /// and doesn't mark the project dirty.
    pub fn repair_roster_blocks(&mut self) -> Result<(OpOutcome, BlockRepairSummary)> {
        // Peek read-only: already coherent → return without touching working bytes or history.
        let db = TdbFile::parse(&self.working)?;
        if towx_coherence(&db).is_empty() {
            return Ok((OpOutcome::default(), BlockRepairSummary::default()));
        }
        drop(db);
        self.run(|tx| tx.repair_roster_blocks())
    }

    /// Set a player's lineup status (dressed vs. scratched) on their slot in `team`'s block — the
    /// per-slot `eGlu.hSZj` boolean (Spike S4). Not `ZBac.RREB` (that is player type).
    pub fn set_dressed(&mut self, team: u64, player: u64, dressed: bool) -> Result<OpOutcome> {
        if !dressed {
            let lines = self.lines_view(team)?.ok_or(EditError::NoSuchSlot { team_id: team, player_id: player })?;
            let entry = lines.entries.iter().find(|e| e.player_id == Some(player))
                .ok_or(EditError::NoSuchSlot { team_id: team, player_id: player })?;
            let mut writes: Vec<_> = entry.slots.iter().map(|field| (field.clone(), 0)).collect();
            writes.push(("hSZj".into(), 0));
            if entry.captaincy.is_some_and(|role| role != 0) { writes.push(("mCcl".into(), 0)); }
            return self.set_cells_int_rows("eGlu", &[(entry.slot_record, writes)]);
        }
        self.run(|tx| tx.set_dressed(team, player, dressed))
    }

    // --- Save spine ---

    /// Package the current working db into the project's on-disk form (raw db, or repacked
    /// container) — the bytes to write back.
    pub fn save(&self) -> Result<Vec<u8>> {
        self.project.package(&self.working)
    }

    /// Post-commit rebind (S2-C): the file at `path` now holds this session's packaged working
    /// bytes (a copied-target transaction installed them and verified `identity`). Re-read the
    /// file, re-detect it exactly as a load would, and require its inner TDB to be **byte-identical
    /// to the working copy** — then it becomes the loaded baseline: `is_dirty()` turns false, the
    /// next save binds to `identity` as its expected prior, and a container's repack template is
    /// the container that is now on disk. The undo stack is kept: undoing past the save makes the
    /// session dirty against the new baseline, which is the truth. If the file does not verify
    /// (someone replaced it in the window), nothing changes and the edits stay in the session.
    pub fn rebind_saved_baseline(
        &mut self,
        path: &Path,
        identity: &tdb_txn::FileIdentity,
    ) -> Result<()> {
        let file_bytes = std::fs::read(path).map_err(|source| DomainError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let actual = tdb_txn::FileIdentity::of_bytes(&file_bytes);
        if actual != *identity {
            return Err(DomainError::Sync(format!(
                "{} is {actual} after the save, not the installed {identity}; the session keeps its edits and its previous baseline",
                path.display()
            )));
        }
        // Detect on a scratch copy first: the project only changes once the inner bytes verify.
        let (_, inner) = crate::project::detect_source_for_check(file_bytes.clone())?;
        let inner = self.project.compose_inner(inner)?;
        if inner != self.working {
            return Err(DomainError::Sync(format!(
                "{} re-parses to a different inner database than the session's working bytes; the session keeps its edits and its previous baseline",
                path.display()
            )));
        }
        self.project.rebind_to_file(file_bytes, identity.clone())?;
        Ok(())
    }

    /// Reload the session's baseline **and working copy** from the file at `path` (after a restore
    /// or recovery changed it underneath a clean session). Discards the undo/redo history, which
    /// was recorded against a baseline that no longer exists. The caller decides whether a dirty
    /// session may be reloaded; this method does not check.
    pub fn reload_from_disk(&mut self, path: &Path) -> Result<()> {
        let prepared = self.prepare_reload(path)?;
        self.adopt_reload(prepared);
        Ok(())
    }

    /// Phase one of a reload: read, detect and parse the file at `path` exactly as a load would,
    /// **without changing the session**. Every failure (missing, unreadable, not a database, wrong
    /// byte order) is returned here, so a workspace can prepare every session's reload before
    /// adopting any of them.
    pub fn prepare_reload(&self, path: &Path) -> Result<PreparedReload> {
        let file_bytes = std::fs::read(path).map_err(|source| DomainError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let identity = tdb_txn::FileIdentity::of_bytes(&file_bytes);
        let (_, inner) = crate::project::detect_source_for_check(file_bytes.clone())?;
        let inner = self.project.compose_inner(inner)?;
        let db = tdb_core::TdbFile::parse(&inner)?;
        if db.endian != self.project.platform.endian() {
            return Err(DomainError::EndianMismatch {
                platform: self.project.platform.as_str(),
                endian: db.endian,
            });
        }
        Ok(PreparedReload {
            file_bytes,
            inner,
            identity,
        })
    }

    /// Phase two of a reload: adopt a prepared file as the baseline **and** the working copy, and
    /// discard the undo/redo history (recorded against a baseline that no longer exists). Cannot
    /// fail: everything fallible happened in [`EditSession::prepare_reload`].
    pub fn adopt_reload(&mut self, prepared: PreparedReload) {
        let PreparedReload {
            file_bytes,
            inner,
            identity,
        } = prepared;
        self.project
            .rebind_to_file(file_bytes, identity)
            .expect("prepare_reload already detected and parsed these bytes");
        self.working = inner;
        self.undo.clear();
        self.redo.clear();
    }

    /// Does the file this session was loaded from still carry the loaded identity? `None` when the
    /// project has no baseline identity (built from bytes).
    pub fn baseline_matches_disk(&self, path: &Path) -> Result<Option<bool>> {
        let Some(loaded) = self.project.loaded_identity() else {
            return Ok(None);
        };
        let actual = tdb_txn::FileIdentity::of_file(path).map_err(|source| DomainError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Ok(Some(actual == *loaded))
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
