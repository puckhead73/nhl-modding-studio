//! ONB-002 / ONB-003 — the project-folder surface of the [`Registry`]: creation from the bundled
//! baseline inventory or from imported files (main database, dictionary and extracted assets), opening and resuming an existing project folder, the
//! project save (a copied-target transaction on the project's working root), the write guards
//! every other route consults, relinking of machine-local destinations, and the explicit, reviewed
//! deployment of project outputs to the game installation.
//!
//! Design notes: `docs/plans/onb-002-persistence-design.md`,
//! `docs/plans/onboarding-20261001-resume-projects.md`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use studio_domain::baselines::{
    BaselineStore, ROLE_DICTIONARY, ROLE_DICTIONARY_META, ROLE_MAIN_DB, ROLE_META, ROLE_ROSTER_SAVE,
    ROLE_ROSTER_SAVE_AUX,
};
use studio_domain::in_place_save::{self as ips, Expectation, PlannedWrite, Prior};
use studio_domain::project_store::{
    canonical_or_planned, check_project_folder, is_within, shown, CopyPlan, CopySource, Destinations,
    InputRecord, MemberEditRecord, NewProject, ProjectLock, ProjectStore, WorkingSet, SOURCE_FRESH,
    SOURCE_IMPORT,
};
use studio_domain::tdb_txn::FileIdentity;
use studio_domain::{Platform, Project, ProjectProfile, ProjectRole, Workspace};

use super::{build_session, ps4_chosen_roster, Registry};
use crate::convert;
use crate::dto::{
    BaselineComponentDto, BaselineGapDto, BaselineInventoryDto, BaselineTargetDto, DeployApplyDto,
    DeployItemDto, DeployOutcomeDto, DeployPlanDto, DeployRestoreDto, DeploymentRecordDto,
    FileIdentityDto, ImportItemReviewDto, ImportReviewDto, ImportSourcesDto, ProjectComponentDto,
    ProjectDestinationsDto, ProjectInfoDto, ProjectInputDto, ProjectOpenDto, ProjectRelinkDto,
    ProjectSaveDto, SaveCodecDto, StagedFileDto,
};
use crate::error::ApiError;

/// The input roles a project records.
pub const INPUT_MAIN_DB: &str = "main_db";
pub const INPUT_ROSTER: &str = "roster_save";
pub const INPUT_DICTIONARY: &str = "dictionary";
pub const INPUT_ASSETS: &str = "assets";

/// The open project's folder binding.
pub(crate) struct ProjectBinding {
    pub store: ProjectStore,
    /// Held for the life of the binding; dropping the binding releases the project.
    _lock: Option<ProjectLock>,
    /// ONB-003: the project source captured for each pending archive-member preview, keyed by
    /// `(archive_relative, member_path)`, joined to the member edit when the save commits.
    pub pending_member_sources: HashMap<(String, String), String>,
}

/// Deployment plans held server-side, keyed by handle.
pub(crate) struct DeployPlan {
    project_id: u32,
    workspace_revision: String,
    seq: u64,
    items: Vec<PlannedItem>,
    #[allow(dead_code)]
    allow_changed: bool,
}

#[derive(Clone)]
struct PlannedItem {
    key: String,
    role: &'static str,
    label: String,
    source_path: PathBuf,
    source: FileIdentity,
    destination: Option<PathBuf>,
    destination_identity: Option<FileIdentity>,
    state: &'static str,
    blocked: Option<String>,
    /// Planned bytes that have no project file (a save-index header the destination layout needs).
    inline: Option<Vec<u8>>,
    /// A source the planned bytes were derived from (the sibling header cloned); it must be
    /// unchanged when the plan is applied.
    template: Option<(PathBuf, FileIdentity)>,
}

#[derive(Default)]
pub(crate) struct ProjectState {
    pub bindings: HashMap<u32, ProjectBinding>,
    plans: HashMap<u32, DeployPlan>,
    next_plan: u32,
}

fn identity_dto(i: &FileIdentity) -> FileIdentityDto {
    FileIdentityDto { len: i.len, sha256: i.sha256.clone() }
}

fn rel_file(dir: &str, name: &str) -> String {
    format!("{dir}/{name}")
}

fn s(p: &Path) -> String {
    shown(p).to_string_lossy().into_owned()
}

fn nonempty(v: Option<&str>) -> Option<&str> {
    v.map(str::trim).filter(|v| !v.is_empty())
}

/// Xenia's default content directory is only a discovery suggestion. A blank
/// save location must not silently turn that unrelated folder into a target.
fn automatic_roster_destination(platform: Platform, save_path: Option<&str>, saves: &[studio_domain::discovery::SaveSlot]) -> Option<String> {
    if matches!(platform, Platform::Xbox | Platform::Recomp) && nonempty(save_path).is_none() {
        return None;
    }
    saves.iter().find(|slot| slot.kind == "roster" && slot.writable).map(|slot| slot.path.clone())
}

/// The project's mode from its working set: `database` (a main database opens), `roster` (only a
/// roster save opens) or `assets` (neither; asset routes only).
pub(crate) fn project_mode(w: &WorkingSet) -> &'static str {
    if w.main_db.is_some() {
        "database"
    } else if w.roster_save.is_some() {
        "roster"
    } else {
        "assets"
    }
}

impl Registry {
    // ------------------------------------------------------------------ inventory

    /// The bundled baseline inventory as the UI sees it.
    pub fn baseline_inventory(&self, store: &BaselineStore) -> BaselineInventoryDto {
        BaselineInventoryDto {
            schema: store.inventory.schema.clone(),
            head: store.inventory.generated_from.head.clone(),
            inventory_path: s(&store.inventory_path),
            files_root: s(&store.files_root),
            targets: store
                .inventory
                .targets
                .iter()
                .map(|t| BaselineTargetDto {
                    game_id: t.game_id.clone(),
                    platform: t.platform.clone(),
                    title: t.title.clone(),
                    status: t.status.clone(),
                    components: t
                        .components
                        .iter()
                        .map(|c| BaselineComponentDto {
                            role: c.role.clone(),
                            file_name: c.file_name.clone(),
                            container_dir: c.container_dir.clone(),
                            len: c.len,
                            sha256: c.sha256.clone(),
                            provenance: c.provenance.clone(),
                            witness: c.witness.clone(),
                            codec: c.codec.as_ref().map(|k| SaveCodecDto {
                                container_version: k.container_version.clone(),
                                kind: k.kind.clone(),
                                readable: k.readable,
                                writable: k.writable,
                                experimental: k.experimental,
                            }),
                        })
                        .collect(),
                    gaps: t.gaps.iter().map(|g| BaselineGapDto { component: g.component.clone(), reason: g.reason.clone() }).collect(),
                })
                .collect(),
        }
    }

    // ------------------------------------------------------------------ creation

    /// Create a default-database mod project from bundled database/asset references.
    /// The optional game installation is a deployment destination. Legacy save arguments
    /// are retained for API compatibility but ignored; save editing is a separate workflow.
    #[allow(clippy::too_many_arguments)]
    pub fn project_create_fresh(
        &self,
        project_dir: &str,
        game_id: &str,
        platform: Platform,
        picked_root: &str,
        _save_path: Option<&str>,
        _roster_save: Option<&str>,
        store: &BaselineStore,
        defs_root: &str,
    ) -> Result<ProjectOpenDto, ApiError> {
        let target = studio_domain::shipping_target(game_id, platform)
            .ok_or_else(|| ApiError::Workspace(format!("{game_id} on {} is not a shipping target", platform.as_str())))?;
        let tb = store.target(game_id, platform).ok_or_else(|| {
            ApiError::Workspace(format!("the bundled baseline inventory has no entry for {}", target.title))
        })?;
        if !tb.is_available() {
            return Err(ApiError::Workspace(format!(
                "Start fresh is not available for {}: {}",
                tb.title,
                tb.gaps.iter().map(|g| format!("{} — {}", g.component, g.reason)).collect::<Vec<_>>().join("; ")
            )));
        }
        if tb.component(ROLE_MAIN_DB).is_none() || tb.component(ROLE_META).is_none() {
            return Err(ApiError::Workspace("A new Mod Studio project requires the game's default database and metadata.".into()));
        }
        // Mod projects own the default database and assets. Legacy clients may still
        // send save arguments; never discover/select a save destination for fresh projects.
        let (destinations, legacy_sidecar) = self.resolve_project_destinations(game_id, platform, picked_root, None, None, None, false)?;
        let dir = Path::new(project_dir);
        let mut others: Vec<(&str, PathBuf)> = vec![("the bundled baselines", store.files_root.clone())];
        if let Some(root) = nonempty(Some(picked_root)) {
            others.push(("the game installation", PathBuf::from(root)));
        }
        push_destination_roots(&destinations, &mut others);
        let others_ref: Vec<(&str, &Path)> = others.iter().map(|(l, p)| (*l, p.as_path())).collect();
        check_project_folder(dir, &others_ref).map_err(ApiError::Domain)?;

        let mut copies = Vec::new();
        let mut working = WorkingSet::default();
        let mut inputs: Vec<InputRecord> = Vec::new();
        for c in &tb.components {
            if matches!(c.role.as_str(), ROLE_ROSTER_SAVE | ROLE_ROSTER_SAVE_AUX) {
                continue;
            }
            let relative = match c.role.as_str() {
                ROLE_MAIN_DB | ROLE_META => rel_file("db", &c.file_name),
                ROLE_DICTIONARY | ROLE_DICTIONARY_META => rel_file("loc", &c.file_name),
                ROLE_ROSTER_SAVE | ROLE_ROSTER_SAVE_AUX => format!(
                    "saves/{}/{}",
                    c.container_dir.clone().unwrap_or_default(),
                    c.container_relative.clone().unwrap_or_else(|| c.file_name.clone())
                ),
                other => return Err(ApiError::Workspace(format!("unknown baseline component role {other}"))),
            };
            match c.role.as_str() {
                ROLE_MAIN_DB => {
                    working.main_db = Some(format!("working/{relative}"));
                    inputs.push(InputRecord { role: INPUT_MAIN_DB.into(), origin: "bundled".into(), note: Some(c.provenance.clone()) });
                }
                ROLE_META => working.meta = Some(format!("working/{relative}")),
                ROLE_DICTIONARY => {
                    working.dictionary = Some(format!("working/{relative}"));
                    inputs.push(InputRecord { role: INPUT_DICTIONARY.into(), origin: "bundled".into(), note: Some(c.provenance.clone()) });
                }
                ROLE_ROSTER_SAVE => {
                    working.roster_save = Some(format!("working/{relative}"));
                    working.roster_container_dir = c.container_dir.clone();
                    inputs.push(InputRecord { role: INPUT_ROSTER.into(), origin: "bundled".into(), note: Some(c.provenance.clone()) });
                }
                _ => {}
            }
            copies.push(CopyPlan {
                role: c.role.clone(),
                source: CopySource::Bundled(c.clone()),
                relative,
                container_relative: c.container_relative.clone(),
                provenance: c.provenance.clone(),
                witness: c.witness.clone(),
            });
        }
        for g in &tb.gaps {
            let role = match g.component.as_str() {
                "dictionary" => INPUT_DICTIONARY,
                _ => continue,
            };
            inputs.push(InputRecord { role: role.into(), origin: "absent".into(), note: Some(g.reason.clone()) });
        }
        inputs.push(InputRecord {
            role: INPUT_ASSETS.into(),
            origin: if destinations.install_dir.is_some() || destinations.game_dir.is_some() { "install" } else { "absent" }.into(),
            note: Some("asset routes read the linked game installation (or an extracted-assets folder)".into()),
        });
        let spec = NewProject {
            game_id: game_id.into(),
            platform: platform.as_str().into(),
            title: tb.title.clone(),
            source_kind: SOURCE_FRESH.into(),
            inventory: Some((store.inventory.schema.clone(), store.inventory.generated_from.head.clone())),
            destinations,
            legacy_sidecar,
            working,
            copies,
            inputs,
        };
        let mut project = ProjectStore::create(dir, spec, Some(store)).map_err(ApiError::Domain)?;
        project.manifest.source.gaps = tb.gaps.iter().filter(|g| !matches!(g.component.as_str(), ROLE_ROSTER_SAVE | ROLE_ROSTER_SAVE_AUX)).map(|g| studio_domain::project_store::GapRecord { component: g.component.clone(), reason: g.reason.clone() }).collect();
        project.save_manifest().map_err(ApiError::Domain)?;
        let lock = ProjectLock::acquire(&project.dir).map_err(ApiError::Domain)?;
        self.open_store(project, Some(lock), defs_root, Vec::new())
    }

    /// Validate import sources without writing anything: each named file is read, identified and
    /// opened with the title's codecs. A main database or an
    /// extracted-assets folder is enough; schema metadata comes from
    /// the bundled baseline when the sibling `-meta.xml` is absent.
    pub fn project_import_review(&self, game_id: &str, platform: Platform, sources: &ImportSourcesDto, store: Option<&BaselineStore>, defs_root: &str) -> Result<ImportReviewDto, ApiError> {
        let target = studio_domain::shipping_target(game_id, platform)
            .ok_or_else(|| ApiError::Workspace(format!("{game_id} on {} is not a shipping target", platform.as_str())))?;
        let mut items = Vec::new();
        let main_db = nonempty(sources.main_db.as_deref()).map(PathBuf::from);
        if nonempty(sources.roster_save.as_deref()).is_some() {
            return Ok(ImportReviewDto { ok: false, items: vec![ImportItemReviewDto { role: ROLE_ROSTER_SAVE.into(), path: sources.roster_save.clone().unwrap(), len: 0, sha256: String::new(), ok: false, detail: "Save files belong in Save Editor. Import a game database or assets into Mod Studio.".into() }], meta_source: None });
        }
        let dictionary = nonempty(sources.dictionary.as_deref()).map(PathBuf::from);
        let assets = nonempty(sources.assets_dir.as_deref()).map(PathBuf::from);
        if main_db.is_none() && assets.is_none() {
            return Ok(ImportReviewDto {
                ok: false,
                items: vec![ImportItemReviewDto { role: "inputs".into(), path: String::new(), len: 0, sha256: String::new(), ok: false, detail: "choose at least one input: the main database or a folder of extracted game assets".into() }],
                meta_source: None,
            });
        }
        // The schema metadata: the sibling of the main database, an explicit file, or the bundled
        // baseline's metadata for this target.
        let (meta, meta_source): (Option<PathBuf>, Option<String>) = match (nonempty(sources.meta.as_deref()), &main_db) {
            (Some(m), _) => (Some(PathBuf::from(m)), Some("explicit".into())),
            (None, Some(db)) if db.with_file_name(target.meta).is_file() => (Some(db.with_file_name(target.meta)), Some("sibling".into())),
            (None, _) => match store.and_then(|st| st.target(game_id, platform)).and_then(|tb| tb.component(ROLE_META).map(|c| (st_path(store.unwrap(), c), c.provenance.clone()))) {
                Some((p, prov)) => (Some(p), Some(format!("bundled ({prov})"))),
                None => (None, None),
            },
        };
        let needs_meta = main_db.is_some();
        let mut meta_ok = false;
        if needs_meta {
            match &meta {
                Some(m) => {
                    items.push(review_file(ROLE_META, m, |_| Ok(format!("schema metadata ({})", meta_source.clone().unwrap_or_default()))));
                    meta_ok = items.last().map(|i| i.ok).unwrap_or(false);
                }
                None => items.push(ImportItemReviewDto { role: ROLE_META.into(), path: String::new(), len: 0, sha256: String::new(), ok: false, detail: format!("no schema metadata: {} has no bundled {} and no main database was given to find it beside", target.title, target.meta) }),
            }
        }
        if let Some(db) = &main_db {
            let meta_p = meta.clone().unwrap_or_default();
            items.push(review_file(ROLE_MAIN_DB, db, |p| {
                if !meta_ok {
                    return Err("its schema metadata could not be read".into());
                }
                Project::load(p, platform, ProjectRole::Nhlng, &meta_p, defs_root)
                    .and_then(|pr| {
                        if game_id == "nhl07" { pr.validate_metadata_schema()?; }
                        if !matches!(pr.source(), studio_domain::Source::RawDb) {
                            return Err(studio_domain::DomainError::Validation("Save containers belong in Save Editor, not the game database input.".into()));
                        }
                        studio_domain::EditSession::begin(pr).overview()
                    })
                    .map(|o| format!("opens as {} ({} tables)", target.main_db, o.table_count))
                    .map_err(|e| format!("does not open as the main database of {}: {e}", target.title))
            }));
        }
        if let Some(d) = &dictionary {
            items.push(review_file(ROLE_DICTIONARY, d, |p| {
                let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
                studio_domain::tdb_loc::LocDictionary::parse(&bytes).map(|l| format!("dictionary with {} strings", l.len())).map_err(|e| format!("not a localization dictionary: {e}"))
            }));
        }
        if let Some(a) = &assets {
            items.push(review_dir(INPUT_ASSETS, a, platform));
        }
        let ok = items.iter().all(|i| i.ok);
        Ok(ImportReviewDto { ok, items, meta_source })
    }

    /// Create a project from imported files and open it. The sources are copied, never written.
    /// Any admitted combination of inputs is accepted (ONB-003 §1); the installation and save
    /// destinations may be empty.
    #[allow(clippy::too_many_arguments)]
    pub fn project_create_import(
        &self,
        project_dir: &str,
        game_id: &str,
        platform: Platform,
        sources: &ImportSourcesDto,
        picked_root: &str,
        _save_path: Option<&str>,
        _roster_save: Option<&str>,
        store: Option<&BaselineStore>,
        defs_root: &str,
    ) -> Result<ProjectOpenDto, ApiError> {
        let review = self.project_import_review(game_id, platform, sources, store, defs_root)?;
        if !review.ok {
            let why: Vec<String> = review.items.iter().filter(|i| !i.ok).map(|i| format!("{}: {}", i.role, i.detail)).collect();
            return Err(ApiError::Workspace(format!("the import sources were not accepted: {}", why.join("; "))));
        }
        let target = studio_domain::shipping_target(game_id, platform).unwrap();
        let assets_dir = nonempty(sources.assets_dir.as_deref());
        let (destinations, legacy_sidecar) = self.resolve_project_destinations(game_id, platform, picked_root, None, None, assets_dir, false)?;
        let dir = Path::new(project_dir);
        let main_db = nonempty(sources.main_db.as_deref()).map(PathBuf::from);

        let folder_of = |p: &Path| p.parent().map(Path::to_path_buf).unwrap_or_else(|| p.to_path_buf());
        let mut others: Vec<(&str, PathBuf)> = Vec::new();
        if let Some(root) = nonempty(Some(picked_root)) {
            others.push(("the game installation", PathBuf::from(root)));
        }
        if let Some(a) = assets_dir {
            others.push(("the extracted-assets folder", PathBuf::from(a)));
        }
        push_destination_roots(&destinations, &mut others);
        let mut copies = Vec::new();
        let mut working = WorkingSet::default();
        let mut inputs: Vec<InputRecord> = Vec::new();
        // Metadata: beside the main database, explicit, or the bundled baseline's.
        let (meta_path, meta_source): (Option<PathBuf>, &str) = match (nonempty(sources.meta.as_deref()), &main_db) {
            (Some(m), _) => (Some(PathBuf::from(m)), "import"),
            (None, Some(db)) if db.with_file_name(target.meta).is_file() => (Some(db.with_file_name(target.meta)), "import"),
            (None, _) => (store.and_then(|st| st.target(game_id, platform)).and_then(|tb| tb.component(ROLE_META).map(|c| st_path(store.unwrap(), c))), "bundled"),
        };
        if let Some(db) = &main_db {
            others.push(("the imported main database's folder", folder_of(db)));
        }
        if main_db.is_some() {
            let meta = meta_path.clone().ok_or_else(|| ApiError::Workspace(format!("no schema metadata for {}", target.title)))?;
            others.push(("the metadata's folder", folder_of(&meta)));
            let source = if meta_source == "bundled" {
                let st = store.unwrap();
                let c = st.target(game_id, platform).unwrap().component(ROLE_META).unwrap().clone();
                CopyPlan { role: ROLE_META.into(), source: CopySource::Bundled(c.clone()), relative: rel_file("db", target.meta), container_relative: None, provenance: c.provenance.clone(), witness: c.witness.clone() }
            } else {
                CopyPlan { role: ROLE_META.into(), source: CopySource::File(meta.clone()), relative: rel_file("db", target.meta), container_relative: None, provenance: "import".into(), witness: s(&meta) }
            };
            copies.push(source);
            working.meta = Some(format!("working/db/{}", target.meta));
        }
        if let Some(db) = &main_db {
            copies.push(CopyPlan { role: ROLE_MAIN_DB.into(), source: CopySource::File(db.clone()), relative: rel_file("db", target.main_db), container_relative: None, provenance: "import".into(), witness: s(db) });
            working.main_db = Some(format!("working/db/{}", target.main_db));
            inputs.push(InputRecord { role: INPUT_MAIN_DB.into(), origin: "import".into(), note: Some(s(db)) });
        } else {
            inputs.push(InputRecord { role: INPUT_MAIN_DB.into(), origin: "absent".into(), note: Some("no main database was given; default-game-database edits are unavailable".into()) });
        }
        if let Some(d) = nonempty(sources.dictionary.as_deref()) {
            let p = PathBuf::from(d);
            others.push(("the imported dictionary's folder", folder_of(&p)));
            copies.push(CopyPlan { role: ROLE_DICTIONARY.into(), source: CopySource::File(p.clone()), relative: rel_file("loc", target.dictionary), container_relative: None, provenance: "import".into(), witness: s(&p) });
            working.dictionary = Some(format!("working/loc/{}", target.dictionary));
            inputs.push(InputRecord { role: INPUT_DICTIONARY.into(), origin: "import".into(), note: Some(s(&p)) });
        } else {
            inputs.push(InputRecord { role: INPUT_DICTIONARY.into(), origin: "absent".into(), note: Some("no dictionary was given; names show as keys where the database holds keys".into()) });
        }
        inputs.push(match assets_dir {
            Some(a) => InputRecord { role: INPUT_ASSETS.into(), origin: "import".into(), note: Some(a.to_string()) },
            None if destinations.install_dir.is_some() || destinations.game_dir.is_some() => InputRecord { role: INPUT_ASSETS.into(), origin: "install".into(), note: Some("asset routes read the linked game installation".into()) },
            None => InputRecord { role: INPUT_ASSETS.into(), origin: "absent".into(), note: Some("no game installation or extracted-assets folder is linked; asset features are unavailable until one is".into()) },
        });
        let others_ref: Vec<(&str, &Path)> = others.iter().map(|(l, p)| (*l, p.as_path())).collect();
        check_project_folder(dir, &others_ref).map_err(ApiError::Domain)?;
        let spec = NewProject {
            game_id: game_id.into(),
            platform: platform.as_str().into(),
            title: format!("{} ({})", target.title, platform_label(platform)),
            source_kind: SOURCE_IMPORT.into(),
            inventory: store.map(|st| (st.inventory.schema.clone(), st.inventory.generated_from.head.clone())).filter(|_| meta_source == "bundled"),
            destinations,
            legacy_sidecar,
            working,
            copies,
            inputs,
        };
        let project = ProjectStore::create(dir, spec, store).map_err(ApiError::Domain)?;
        let lock = ProjectLock::acquire(&project.dir).map_err(ApiError::Domain)?;
        self.open_store(project, Some(lock), defs_root, Vec::new())
    }

    /// Resolve the destinations from the chosen folders with the existing discovery rules
    /// (read-only). The install may be absent, empty or incomplete: the project is still created,
    /// and the deployment stays blocked until the destination resolves.
    #[allow(clippy::too_many_arguments)]
    fn resolve_project_destinations(
        &self, game_id: &str, platform: Platform, picked_root: &str,
        save_path: Option<&str>, roster_save: Option<&str>, assets_dir: Option<&str>,
        include_saves: bool,
    ) -> Result<(Destinations, Option<String>), ApiError> {
        let Some(root) = nonempty(Some(picked_root)) else {
            // ONB-003: no installation yet. An extracted-assets folder stands in as the asset
            // read root (and the loose-asset deployment destination) until an installation is
            // linked.
            let save_root = nonempty(save_path).map(str::to_string);
            return Ok((
                Destinations {
                    picked_root: String::new(),
                    install_dir: assets_dir.map(str::to_string),
                    game_dir: None,
                    save_root,
                    roster_save: None,
                    main_db_path: None,
                    meta_xml_path: None,
                    localization_path: None,
                    assets_dir: assets_dir.map(str::to_string),
                },
                None,
            ));
        };
        let report = studio_domain::detect_install(game_id, platform, Path::new(root), nonempty(save_path).map(Path::new));
        let roster = if !include_saves { None } else { match platform {
            Platform::Ps4 => ps4_chosen_roster(&report, roster_save)?.map(|s| s.path.clone()),
            _ => match nonempty(roster_save) {
                Some(r) => {
                    let known = report.saves.iter().find(|s| s.kind == "roster" && Path::new(&s.path) == Path::new(r));
                    match known {
                        Some(k) => Some(k.path.clone()),
                        None => return Err(ApiError::Workspace(format!("the chosen roster save {r} is not one of the roster saves found in the save location; detect again and choose a save"))),
                    }
                }
                None => automatic_roster_destination(platform, save_path, &report.saves),
            },
        }};
        let legacy = report.install_dir.as_ref().map(|d| ProjectProfile::sidecar_path(Path::new(d))).filter(|p| p.is_file()).map(|p| s(&p));
        Ok((
            Destinations {
                picked_root: root.into(),
                install_dir: report.install_dir.clone().or_else(|| assets_dir.map(str::to_string)),
                game_dir: report.game_dir.clone(),
                save_root: if include_saves { report.save_root.clone().or_else(|| nonempty(save_path).map(str::to_string)) } else { None },
                roster_save: roster,
                main_db_path: report.db_paths.nhlng.clone(),
                meta_xml_path: report.db_paths.meta_xml.clone(),
                localization_path: report.db_paths.localization.clone(),
                assets_dir: assets_dir.map(str::to_string),
            },
            legacy,
        ))
    }

    // ------------------------------------------------------------------ open / info

    /// Open an existing project folder and resume it (ONB-003 §3): the manifest is validated (and
    /// a `nhlms-project/1` manifest migrated losslessly), the project lock is taken, the working
    /// copies are reopened, and the stored destinations are re-discovered read-only. Missing or
    /// changed external resources become warnings — nothing is substituted, installed or reset.
    pub fn project_open(&self, project_dir: &str, defs_root: &str) -> Result<ProjectOpenDto, ApiError> {
        let mut store = ProjectStore::open(Path::new(project_dir)).map_err(ApiError::Domain)?;
        let dir = canonical_or_planned(&store.dir).unwrap_or_else(|| store.dir.clone());
        let existing = {
            let projects = self.projects.lock().unwrap();
            projects.bindings.iter().find(|(_, b)| canonical_or_planned(&b.store.dir).map(|c| is_within(&c, &dir) && is_within(&dir, &c)).unwrap_or(false))
                .map(|(id, binding)| (*id, binding.store.clone()))
        };
        // A frontend reload can lose its context while the native session remains alive.
        // Reattach to that workspace; never discard its unsaved edits or replace its lock.
        let (existing_id, lock) = if let Some((id, live_store)) = existing {
            store = live_store;
            (Some(id), None)
        } else {
            (None, Some(ProjectLock::acquire(&store.dir).map_err(ApiError::Domain)?))
        };
        let mut warnings = Vec::new();
        let store = store;
        if let Some(from) = store.manifest.migrated_from.clone() {
            match store.save_manifest() {
                Ok(()) => warnings.push(format!("the project manifest was written by schema {from} and has been migrated to {} (all earlier content kept)", studio_domain::project_store::MANIFEST_SCHEMA)),
                Err(e) => warnings.push(format!("the project manifest was read as schema {from} but could not be rewritten as the current schema ({e}); it stays as it was")),
            }
        }
        // An interrupted project save: the working root's journal says so; the copied-target
        // status / recovery route handles it explicitly. Opening never resumes it.
        match self.copied_target_status(&store.working_dir().to_string_lossy()) {
            Ok(status) if !status.apply_allowed => warnings.push(format!("the project's last save did not complete: {}. The saved files are as the journal left them; use Project Overview → Recover interrupted save to recover before saving again", status.guidance)),
            Ok(_) => {}
            Err(e) => warnings.push(format!("the project's last save did not complete cleanly: its journal cannot be read ({e}). The saved files are as the journal left them; use Project Overview → Recover interrupted save to recover before saving again")),
        }
        // The engine's ACTIVE marker names a transaction that was in flight when the process
        // stopped; the engine refuses a new save until it is recovered explicitly.
        let active = store.working_dir().join(studio_domain::tdb_txn::journal::TXN_DIR).join(studio_domain::tdb_txn::journal::ACTIVE_MARKER);
        if let Ok(id) = std::fs::read_to_string(&active) {
            let id = id.trim();
            if !id.is_empty() {
                warnings.push(format!("the project's last save did not complete: transaction {id} is still marked active in the working folder's journal. Saving is refused until it is recovered (Project Overview → Recover interrupted save); the saved files were not changed by opening"));
            }
        }
        // Destinations as they are now (nothing is chosen in place of a missing one).
        let d = &store.manifest.destinations;
        if !d.picked_root.is_empty() && !Path::new(&d.picked_root).is_dir() {
            warnings.push(format!("the game source folder {} is not reachable; project edits are preserved. Update the source under Project Overview → Game source folders", d.picked_root));
        }
        if let Some(a) = &d.assets_dir {
            if !Path::new(a).is_dir() {
                warnings.push(format!("the extracted-assets folder {a} is not reachable; asset features wait until it is relinked"));
            }
        }
        if let Some(id) = existing_id {
            let summary = self.summary(id)?;
            return Ok(ProjectOpenDto {
                summary,
                project: project_info_dto(&store),
                discovery: self.revalidate_destinations(&store),
                warnings,
                last_view: store.manifest.last_view.clone(),
            });
        }
        self.open_store(store, lock, defs_root, warnings)
    }

    fn open_store(&self, store: ProjectStore, lock: Option<ProjectLock>, defs_root: &str, mut warnings: Vec<String>) -> Result<ProjectOpenDto, ApiError> {
        let platform = super::parse_platform(&store.manifest.platform)?;
        let game_id = store.manifest.game_id.clone();
        let main = store.manifest.working.main_db.as_ref().map(|m| store.path(m));
        let meta = store.manifest.working.meta.as_ref().map(|m| store.path(m));
        if store.manifest.working.roster_save.is_some() {
            if main.is_none() {
                return Err(ApiError::Workspace("This project contains only a save database. It belongs in Save Editor; all project files have been preserved.".into()));
            }
            warnings.push("The project's existing save files are preserved, but Mod Studio edits and builds only the game database and assets.".into());
        }
        let dictionary = store.manifest.working.dictionary.as_ref().map(|d| store.path(d));
        let meta_s = meta.as_ref().map(|m| m.to_string_lossy().into_owned()).unwrap_or_default();
        let last_view = store.manifest.last_view.clone();
        let summary = if let Some(m) = &main {
            let session = build_session(&m.to_string_lossy(), platform, ProjectRole::Nhlng, &meta_s, defs_root)?;
            if game_id == "nhl07" { session.project().validate_metadata_schema()?; }
            let mut ws = Workspace::single(session, m.to_string_lossy().into_owned());
            ws.set_game_id(game_id.clone());
            if let Some(d) = &dictionary { ws.attach_dictionary_at_open(&d.to_string_lossy()); }
            self.register(ws)?
        } else {
            convert::asset_only_summary_dto(self.register_asset_only(), &game_id, platform)
        };
        let (info, discovery) = {
            let discovery = self.revalidate_destinations(&store);
            let info = project_info_dto(&store);
            (info, discovery)
        };
        self.projects.lock().unwrap().bindings.insert(summary.project_id, ProjectBinding { store, _lock: lock, pending_member_sources: HashMap::new() });
        Ok(ProjectOpenDto { summary, project: info, discovery, warnings, last_view })
    }

    /// Re-run discovery on the stored destinations (read-only) so the UI and the deployment see
    /// the install as it is now, not as it was at creation. Without an installation the report
    /// names the extracted-assets folder as the asset root and nothing else.
    fn revalidate_destinations(&self, store: &ProjectStore) -> crate::dto::InstallReportDto {
        let d = &store.manifest.destinations;
        let platform = super::parse_platform(&store.manifest.platform).unwrap_or(Platform::Xbox);
        if d.picked_root.is_empty() {
            let report = studio_domain::InstallReport {
                game_id: store.manifest.game_id.clone(),
                platform: platform.as_str().to_string(),
                picked_root: String::new(),
                install_dir: d.assets_dir.clone().filter(|a| Path::new(a).is_dir()),
                db_paths: studio_domain::DbPaths::default(),
                saves: Vec::new(),
                minimum_met: false,
                extract_needed: false,
                game_dir: None,
                save_root: None,
                game_candidates: Vec::new(),
                problems: Vec::new(),
            };
            return convert::install_report_dto(&report, false);
        }
        let mut report = studio_domain::detect_install(&store.manifest.game_id, platform, Path::new(&d.picked_root), d.save_root.as_deref().map(Path::new));
        if let Some(assets) = d.assets_dir.clone().filter(|a| Path::new(a).is_dir()) {
            report.install_dir = Some(assets.clone());
            if platform == Platform::Ps4 { report.game_dir = Some(assets); }
        }
        let has_sidecar = report.install_dir.as_ref().map(|d| ProjectProfile::sidecar_path(Path::new(d)).exists()).unwrap_or(false);
        convert::install_report_dto(&report, has_sidecar)
    }

    pub fn project_info(&self, id: u32) -> Result<ProjectInfoDto, ApiError> {
        let projects = self.projects.lock().unwrap();
        let b = projects.bindings.get(&id).ok_or_else(|| ApiError::Workspace(format!("project {id} is not a project-folder session")))?;
        Ok(project_info_dto(&b.store))
    }

    /// ONB-003 §3: remember the last meaningful Studio route for the reopen.
    pub fn project_set_view(&self, id: u32, route: &str) -> Result<(), ApiError> {
        let route = route.trim();
        if route.is_empty() || route.len() > 512 || !route.starts_with("/app/") {
            return Err(ApiError::Workspace(format!("{route:?} is not a Studio view route")));
        }
        let mut projects = self.projects.lock().unwrap();
        let b = projects.bindings.get_mut(&id).ok_or_else(|| ApiError::Workspace(format!("project {id} is not a project-folder session")))?;
        b.store.record_view(route).map_err(ApiError::Domain)
    }

    /// ONB-003 §1/§3: link or relink the machine-local destinations (game installation, save
    /// location, extracted-assets folder) of an open project. Re-discovers the
    /// installation read-only; the project folder must still not overlap any destination. Nothing
    /// in the destinations is written.
    pub fn project_relink(&self, id: u32, root: Option<&str>, _save_path: Option<&str>, _roster_save: Option<&str>, assets_dir: Option<&str>) -> Result<ProjectRelinkDto, ApiError> {
        let (game_id, platform, dir, current) = {
            let projects = self.projects.lock().unwrap();
            let b = projects.bindings.get(&id).ok_or_else(|| ApiError::Workspace(format!("project {id} is not a project-folder session")))?;
            (b.store.manifest.game_id.clone(), super::parse_platform(&b.store.manifest.platform)?, b.store.dir.clone(), b.store.manifest.destinations.clone())
        };
        let root = root.map(str::trim).unwrap_or(&current.picked_root);
        let assets = match assets_dir {
            Some(a) => nonempty(Some(a)).map(str::to_string),
            None => current.assets_dir.clone(),
        };
        if let Some(a) = &assets {
            if !Path::new(a).is_dir() {
                return Err(ApiError::Workspace(format!("the extracted-assets folder {a} is not a folder")));
            }
        }
        let (destinations, legacy) = self.resolve_project_destinations(&game_id, platform, root, None, None, assets.as_deref(), false)?;
        let mut others: Vec<(&str, PathBuf)> = Vec::new();
        if let Some(r) = nonempty(Some(root)) {
            others.push(("the game installation", PathBuf::from(r)));
        }
        if let Some(a) = &assets {
            others.push(("the extracted-assets folder", PathBuf::from(a)));
        }
        push_destination_roots(&destinations, &mut others);
        let others_ref: Vec<(&str, &Path)> = others.iter().map(|(l, p)| (*l, p.as_path())).collect();
        check_project_folder(&dir, &others_ref).map_err(ApiError::Domain)?;
        let mut warnings = Vec::new();
        if nonempty(Some(root)).is_some() && destinations.install_dir.is_none() && destinations.game_dir.is_none() {
            warnings.push(format!("no {game_id} game files were recognised under {root}; the source path is saved, but some source assets may be unavailable"));
        }
        let mut projects = self.projects.lock().unwrap();
        let b = projects.bindings.get_mut(&id).ok_or(ApiError::UnknownProject(id))?;
        b.store.manifest.destinations = destinations;
        if legacy.is_some() {
            b.store.manifest.legacy_sidecar = legacy;
        }
        let assets_origin = if b.store.manifest.destinations.assets_dir.is_some() { "import" } else if b.store.manifest.destinations.install_dir.is_some() || b.store.manifest.destinations.game_dir.is_some() { "install" } else { "absent" };
        match b.store.manifest.inputs.iter_mut().find(|i| i.role == INPUT_ASSETS) {
            Some(i) => {
                i.origin = assets_origin.into();
                i.note = b.store.manifest.destinations.assets_dir.clone().or_else(|| Some("asset routes read the linked game installation".into()));
            }
            None => b.store.manifest.inputs.push(InputRecord { role: INPUT_ASSETS.into(), origin: assets_origin.into(), note: b.store.manifest.destinations.assets_dir.clone() }),
        }
        b.store.save_manifest().map_err(ApiError::Domain)?;
        let discovery = self.revalidate_destinations(&b.store);
        Ok(ProjectRelinkDto { project: project_info_dto(&b.store), discovery, warnings })
    }

    /// The project folder bound to an open session, if any.
    pub(crate) fn project_dir_of(&self, id: u32) -> Option<PathBuf> {
        self.projects.lock().unwrap().bindings.get(&id).map(|b| b.store.dir.clone())
    }

    /// `true` when `id` is a project-folder binding (with or without an edit session).
    pub(crate) fn is_project_binding(&self, id: u32) -> bool {
        self.projects.lock().unwrap().bindings.contains_key(&id)
    }

    pub(crate) fn forget_project_binding(&self, id: u32) {
        let mut projects = self.projects.lock().unwrap();
        projects.bindings.remove(&id);
        projects.plans.retain(|_, p| p.project_id != id);
    }

    pub fn project_set_extraction(&self, id: u32, choice: &str) -> Result<ProjectInfoDto, ApiError> {
        if !matches!(choice, "skipped" | "extracted" | "not_needed" | "pending") {
            return Err(ApiError::Workspace(format!("unknown extraction choice {choice:?}")));
        }
        let mut projects = self.projects.lock().unwrap();
        let b = projects.bindings.get_mut(&id).ok_or_else(|| ApiError::Workspace(format!("project {id} is not a project-folder session")))?;
        b.store.set_extraction(choice).map_err(ApiError::Domain)?;
        Ok(project_info_dto(&b.store))
    }

    // ------------------------------------------------------------------ write guards

    /// Refuse an ordinary write route whose target lies inside any of the open projects' guarded
    /// roots (destinations and import sources). `what` names the route for the message.
    pub(crate) fn refuse_guarded_write(&self, project_id: Option<u32>, what: &str, path: &Path) -> Result<(), ApiError> {
        let projects = self.projects.lock().unwrap();
        let bindings: Vec<&ProjectBinding> = match project_id {
            Some(id) => projects.bindings.get(&id).into_iter().collect(),
            None => projects.bindings.values().collect(),
        };
        for b in bindings {
            b.store.refuse_guarded_write(what, path).map_err(ApiError::Domain)?;
        }
        Ok(())
    }

    /// The project (if any) whose game destination (or extracted-assets folder) contains
    /// `install_dir`: loose-asset routes redirect their outputs into that project's overlay.
    pub(crate) fn project_for_install_dir(&self, install_dir: &str) -> Option<(u32, PathBuf)> {
        let target = canonical_or_planned(Path::new(install_dir))?;
        let projects = self.projects.lock().unwrap();
        for (id, b) in &projects.bindings {
            let d = &b.store.manifest.destinations;
            for root in [Some(&d.picked_root).filter(|r| !r.is_empty()), d.install_dir.as_ref(), d.game_dir.as_ref(), d.assets_dir.as_ref()].into_iter().flatten() {
                if let Some(r) = canonical_or_planned(Path::new(root)) {
                    if is_within(&target, &r) || is_within(&r, &target) {
                        return Some((*id, b.store.dir.clone()));
                    }
                }
            }
        }
        None
    }

    /// Resolve a loose-asset target for a read-modify-write route: inside an open project it is
    /// the overlay copy (materialised from the install on first use); otherwise the installed file.
    pub(crate) fn project_asset_target(&self, install_dir: &str, rel_path: &str) -> Result<(PathBuf, Option<u32>), ApiError> {
        let installed = studio_domain::resolve_install_path(Path::new(install_dir), rel_path)
            .ok_or_else(|| ApiError::Workspace(format!("{rel_path} escapes the install root")))?;
        let Some((id, _)) = self.project_for_install_dir(install_dir) else {
            return Ok((installed, None));
        };
        let projects = self.projects.lock().unwrap();
        let b = projects.bindings.get(&id).ok_or(ApiError::UnknownProject(id))?;
        let rel = installed
            .strip_prefix(Path::new(install_dir))
            .map(|r| r.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| rel_path.replace('\\', "/"));
        let (overlay, _) = b.store.materialize_overlay(Path::new(install_dir), &rel).map_err(ApiError::Domain)?;
        if !overlay.exists() {
            return Ok((installed, Some(id)));
        }
        Ok((overlay, Some(id)))
    }

    /// Snapshot the overlay root before taking a workspace lock. Preview reads must not
    /// materialize copies or acquire the project lock from inside a workspace operation.
    pub(crate) fn preview_overlay_root(&self, install_dir: &str) -> Option<PathBuf> {
        self.project_for_install_dir(install_dir).map(|(_, dir)|
            dir.join(studio_domain::project_store::WORKING_DIR)
                .join(studio_domain::project_store::OVERLAY_DIR))
    }

    /// Absolute-path preview routes also need to show the recorded project output.
    pub(crate) fn preview_source_path(&self, source: &Path) -> PathBuf {
        let source_key = source.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
        let projects = self.projects.lock().unwrap();
        for binding in projects.bindings.values() {
            let d = &binding.store.manifest.destinations;
            for file in &binding.store.manifest.staged_files {
                for root in [Some(&d.picked_root), d.install_dir.as_ref(), d.game_dir.as_ref(), d.assets_dir.as_ref()].into_iter().flatten() {
                    if root.is_empty() { continue; }
                    let candidate = Path::new(root).join(&file.relative);
                    if candidate.to_string_lossy().replace('\\', "/").to_ascii_lowercase() == source_key {
                        let staged = binding.store.overlay_path(&file.relative);
                        if staged.is_file() { return staged; }
                    }
                }
            }
        }
        source.to_path_buf()
    }

    /// Record an overlay output after a route wrote it (no retained source: kit clones).
    pub(crate) fn record_staged_file(&self, id: u32, install_dir: &str, rel_path: &str, origin: &str) -> Result<(), ApiError> {
        let mut projects = self.projects.lock().unwrap();
        let b = projects.bindings.get_mut(&id).ok_or(ApiError::UnknownProject(id))?;
        let rel = rel_path.replace('\\', "/");
        let prior = std::fs::read(Path::new(install_dir).join(&rel)).ok().map(|bytes| FileIdentity::of_bytes(&bytes));
        b.store.record_staged_file(&rel, origin, prior.as_ref()).map_err(ApiError::Domain)
    }

    /// ONB-003: record an overlay output together with the editable source it was encoded from
    /// (kept inside the project under `sources/`), so the edit stays portable intent and a mod
    /// package can carry the source rather than only the native output.
    pub(crate) fn record_staged_output(&self, id: u32, install_dir: &str, rel_path: &str, origin: &str, source: Option<(&[u8], &str, Option<&str>)>, texture_index: Option<u32>) -> Result<(), ApiError> {
        let mut projects = self.projects.lock().unwrap();
        let b = projects.bindings.get_mut(&id).ok_or(ApiError::UnknownProject(id))?;
        let rel = rel_path.replace('\\', "/");
        let prior = std::fs::read(Path::new(install_dir).join(&rel)).ok().map(|bytes| FileIdentity::of_bytes(&bytes));
        let sha = match source {
            Some((bytes, ext, name)) => Some(b.store.store_source(bytes, ext, name).map_err(ApiError::Domain)?.sha256),
            None => None,
        };
        b.store.record_staged_output(&rel, origin, prior.as_ref(), sha.as_deref(), texture_index).map_err(ApiError::Domain)
    }

    /// ONB-003: keep the source image of an archive-member preview inside the project, so the
    /// member edit the next save commits carries its editable source.
    pub(crate) fn capture_member_source(&self, id: u32, archive_relative: &str, member_path: &str, source_image_path: &str) -> Result<(), ApiError> {
        let mut projects = self.projects.lock().unwrap();
        let Some(b) = projects.bindings.get_mut(&id) else { return Ok(()) };
        let p = Path::new(source_image_path);
        let bytes = std::fs::read(p).map_err(|e| ApiError::Workspace(format!("the source image {} could not be read: {e}", p.display())))?;
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("bin");
        let name = p.file_name().and_then(|n| n.to_str());
        let rec = b.store.store_source(&bytes, ext, name).map_err(ApiError::Domain)?;
        b.pending_member_sources.insert((archive_relative.replace('\\', "/"), member_path.to_string()), rec.sha256);
        Ok(())
    }

    /// The synthetic summary of a bound assets-only project (no edit session), else `None`.
    pub(crate) fn asset_only_summary(&self, id: u32) -> Option<crate::dto::ProjectSummaryDto> {
        let projects = self.projects.lock().unwrap();
        let b = projects.bindings.get(&id)?;
        let platform = super::parse_platform(&b.store.manifest.platform).ok()?;
        Some(convert::asset_only_summary_dto(id, &b.store.manifest.game_id, platform))
    }

    // ------------------------------------------------------------------ save

    /// Save the project: materialise the archives the pending edits need, then run the
    /// copied-target review and apply on the project's working root. The project is marked saved
    /// only when the engine committed; the archive-member edits it committed are recorded as
    /// semantic intent with their captured sources.
    pub fn project_save(&self, id: u32) -> Result<ProjectSaveDto, ApiError> {
        let dir = self.project_dir_of(id).ok_or_else(|| ApiError::Workspace(format!("project {id} is not a project-folder session; use the copied-target or disc-image routes for it")))?;
        if !self.has_session(id) {
            // R71-R4: an assets-only project has no database workspace. Its outputs (staged
            // overlay files, kept sources, edit intent) are written into the project folder the
            // moment each route runs, so Save is valid and accurately has nothing left to save.
            self.projects.lock().unwrap().bindings.get_mut(&id).ok_or(ApiError::UnknownProject(id))?.store.record_save_action().map_err(ApiError::Domain)?;
            let summary = self.summary(id)?;
            return Ok(ProjectSaveDto { saved: false, nothing_to_save: true, txn_id: None, seq: None, materialized: Vec::new(), summary });
        }
        let working = dir.join(studio_domain::project_store::WORKING_DIR);
        let (dirty, needed, members) = self.with_workspace(id, |ws| {
            let mut needed: Vec<(String, PathBuf)> = Vec::new();
            let mut members = Vec::new();
            for m in ws.asset_member_edits() {
                needed.push((m.archive_relative.clone(), m.archive_path.clone()));
                if let Some(b) = &m.big4 {
                    needed.push((b.sidecar_relative.clone(), m.archive_path.with_file_name(Path::new(&b.sidecar_relative).file_name().unwrap_or_default())));
                }
                members.push((m.archive_relative.clone(), m.member_path.clone(), m.whole_group, m.container.clone(), m.width, m.height, m.description.clone()));
            }
            if let Some(w) = ws.settings_csv_working_set() {
                if !w.edits.is_empty() {
                    needed.push(("databigs/inxp_00.big".into(), w.archive_path.clone()));
                }
            }
            Ok((ws.is_dirty(), needed, members))
        })?;
        if !dirty {
            self.projects.lock().unwrap().bindings.get_mut(&id).ok_or(ApiError::UnknownProject(id))?.store.record_save_action().map_err(ApiError::Domain)?;
            let summary = self.summary(id)?;
            return Ok(ProjectSaveDto { saved: false, nothing_to_save: true, txn_id: None, seq: None, materialized: Vec::new(), summary });
        }
        let mut materialized = Vec::new();
        {
            let mut projects = self.projects.lock().unwrap();
            let b = projects.bindings.get_mut(&id).ok_or(ApiError::UnknownProject(id))?;
            let mut seen = std::collections::BTreeSet::new();
            for (rel, source) in needed {
                if !seen.insert(rel.clone()) {
                    continue;
                }
                let dest = b.store.working_dir().join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
                if dest.is_file() {
                    continue;
                }
                let len = std::fs::metadata(&source).map(|m| m.len()).map_err(|e| ApiError::Workspace(format!("the archive {} the pending edit was planned on cannot be read: {e}", source.display())))?;
                check_free_space(&b.store.dir, len, "materialising the archive into the project")?;
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(|source| ApiError::Io { path: parent.display().to_string(), source })?;
                }
                let tmp = dest.with_extension("nhlms-copy-tmp");
                std::fs::copy(&source, &tmp).map_err(|e| {
                    let _ = std::fs::remove_file(&tmp);
                    ApiError::Workspace(format!("could not copy {} into the project: {e}", source.display()))
                })?;
                std::fs::rename(&tmp, &dest).map_err(|source| ApiError::Io { path: dest.display().to_string(), source })?;
                let identity = FileIdentity::of_file(&dest).map_err(|source| ApiError::Io { path: dest.display().to_string(), source })?;
                b.store.record_materialized(&rel, &source, &identity).map_err(ApiError::Domain)?;
                materialized.push(rel);
            }
        }
        let review = self.copied_target_review(id, &working.to_string_lossy())?;
        let applied = self.copied_target_apply(id, review.handle)?;
        if applied.state != "committed" {
            return Err(ApiError::Workspace(format!("the project save did not commit (state {}); the previous saved state is intact", applied.state)));
        }
        let seq = {
            let mut projects = self.projects.lock().unwrap();
            let b = projects.bindings.get_mut(&id).ok_or(ApiError::UnknownProject(id))?;
            b.store.record_save(&applied.txn_id).map_err(ApiError::Domain)?;
            let seq = b.store.manifest.saves.last().map(|s| s.seq).unwrap_or(0);
            let at = studio_domain::project_store::now_iso();
            let records: Vec<MemberEditRecord> = members
                .into_iter()
                .map(|(archive_relative, member_path, whole_group, container, width, height, description)| {
                    let source_sha256 = b.pending_member_sources.remove(&(archive_relative.replace('\\', "/"), member_path.clone()));
                    MemberEditRecord { archive_relative, member_path, whole_group, container, width, height, source_sha256, description, save_seq: seq, at: at.clone() }
                })
                .collect();
            b.store.record_member_edits(records).map_err(ApiError::Domain)?;
            Some(seq)
        };
        let summary = self.summary(id)?;
        Ok(ProjectSaveDto { saved: true, nothing_to_save: false, txn_id: Some(applied.txn_id), seq, materialized, summary })
    }

    // ------------------------------------------------------------------ deployment

    /// Plan a deployment: every project output mapped to its destination with the destination's
    /// current state. Read-only. The plan is held server-side and bound to the workspace revision.
    pub fn project_deploy_plan(&self, id: u32, allow_changed: bool) -> Result<DeployPlanDto, ApiError> {
        let (revision, dirty) = if self.has_session(id) {
            self.with_workspace(id, |ws| Ok((format!("{:?}", ws.revision()), ws.is_dirty())))?
        } else {
            ("assets-only".to_string(), false)
        };
        let mut projects = self.projects.lock().unwrap();
        let b = projects.bindings.get(&id).ok_or_else(|| ApiError::Workspace(format!("project {id} is not a project-folder session")))?;
        let store = &b.store;
        let m = &store.manifest;
        let platform = super::parse_platform(&m.platform)?;
        // Destinations as they are now.
        let report = if m.destinations.picked_root.is_empty() {
            None
        } else {
            Some(studio_domain::detect_install(&m.game_id, platform, Path::new(&m.destinations.picked_root), m.destinations.save_root.as_deref().map(Path::new)))
        };
        let last_deployed: HashMap<String, FileIdentity> = read_last_deployed(store);
        let mut items: Vec<PlannedItem> = Vec::new();
        let baseline_of = |role: &str| m.source.components.iter().find(|c| c.role == role).map(|c| FileIdentity { len: c.len, sha256: c.sha256.clone() });
        // A linked installation that is not reachable now (moved, unmounted, deleted): nothing is
        // created in its place; every destination under it is unresolved until it is relinked.
        let unreachable_root = (!m.destinations.picked_root.is_empty() && !Path::new(&m.destinations.picked_root).is_dir()).then(|| m.destinations.picked_root.clone());
        let unreachable_assets = m.destinations.assets_dir.clone().filter(|a| !Path::new(a).is_dir());
        let mut push = |key: &str, role: &'static str, label: String, source_path: PathBuf, destination: Option<PathBuf>, prior: Option<FileIdentity>| -> Result<(), ApiError> {
            let source = FileIdentity::of_file(&source_path).map_err(|e| ApiError::Workspace(format!("{label}: the project's file {} cannot be read: {e}", source_path.display())))?;
            let destination = destination.filter(|d| {
                let under_root = unreachable_root.as_ref().is_some_and(|r| canonical_or_planned(d).zip(canonical_or_planned(Path::new(r))).is_some_and(|(d, r)| is_within(&d, &r)));
                let under_assets = unreachable_assets.as_ref().is_some_and(|a| canonical_or_planned(d).zip(canonical_or_planned(Path::new(a))).is_some_and(|(d, a)| is_within(&d, &a)));
                !(under_root || under_assets) && d.parent().is_some_and(|p| p.is_dir())
            });
            let (destination_identity, state, blocked) = match &destination {
                None => (None, "unresolved", Some(format!("{label}: no reachable destination is known for this file in the game installation or save location; link (or relink) the destination first (Settings → Project)"))),
                Some(d) => match std::fs::read(d) {
                    Ok(bytes) => {
                        let cur = FileIdentity::of_bytes(&bytes);
                        if cur == source {
                            (Some(cur), "already-current", None)
                        } else if prior.as_ref() == Some(&cur) || last_deployed.get(key) == Some(&cur) {
                            (Some(cur), "unchanged", None)
                        } else {
                            let blocked = (!allow_changed).then(|| format!("{label}: {} changed outside Studio since the project was created or last deployed ({cur}); review it and allow replacing changed files to deploy over it", s(d)));
                            (Some(cur), "changed-externally", blocked)
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (None, "absent", None),
                    Err(e) => (None, "unreadable", Some(format!("{label}: {} cannot be read: {e}", s(d)))),
                },
            };
            items.push(PlannedItem { key: key.into(), role, label, source_path, source, destination, destination_identity, state, blocked, inline: None, template: None });
            Ok(())
        };
        // Main database.
        if let Some(main) = &m.working.main_db {
            let main_dest = report.as_ref().and_then(|r| r.db_paths.nhlng.clone()).or(m.destinations.main_db_path.clone()).map(PathBuf::from);
            push("main_db", "database", format!("Main database ({})", Path::new(main).file_name().unwrap_or_default().to_string_lossy()), store.path(main), main_dest, baseline_of(ROLE_MAIN_DB))?;
        }
        // Roster save.
        if let Some(d) = &m.working.dictionary {
            let dest = report.as_ref().and_then(|r| r.db_paths.localization.clone()).or(m.destinations.localization_path.clone()).map(PathBuf::from);
            push("dictionary", "dictionary", "Localization dictionary".into(), store.path(d), dest, baseline_of(ROLE_DICTIONARY))?;
        }
        // Overlay outputs: relative to the asset read root the routes authored against — the
        // platform's asset root (PS4: the game folder, never the database folder; R71-R5), else
        // the project's own assets folder.
        let install_dir = report
            .as_ref()
            .and_then(|r| super::package::asset_roots(r, platform).into_iter().last().map(|p| s(&p)))
            .or(if platform == Platform::Ps4 { m.destinations.game_dir.clone() } else { m.destinations.install_dir.clone() })
            .or(m.destinations.assets_dir.clone())
            .map(PathBuf::from);
        for sf in &m.staged_files {
            let dest = install_dir.as_ref().map(|d| d.join(sf.relative.replace('/', std::path::MAIN_SEPARATOR_STR)));
            push(&format!("overlay:{}", sf.relative), "loose-asset", format!("{} ({})", sf.relative, sf.origin), store.overlay_path(&sf.relative), dest, sf.prior.as_ref().map(|p| p.to_identity()))?;
        }
        // Materialised archives.
        for a in &m.materialized_archives {
            push(&format!("archive:{}", a.relative), "archive", format!("Archive {}", a.relative), store.working_dir().join(a.relative.replace('/', std::path::MAIN_SEPARATOR_STR)), Some(PathBuf::from(&a.destination)), Some(a.prior.to_identity()))?;
        }
        let changing: Vec<&PlannedItem> = items.iter().filter(|i| i.state != "already-current").collect();
        let mut reasons: Vec<String> = changing.iter().filter_map(|i| i.blocked.clone()).collect();
        if dirty {
            reasons.insert(0, "the project has unsaved edits; save the project first (deployment ships saved files only)".into());
        }
        if changing.is_empty() {
            reasons.push(if items.is_empty() { "the project has no saved outputs to deploy".into() } else { "every destination already holds the project's files; nothing to deploy".into() });
        }
        let backup_bytes: u64 = changing.iter().filter_map(|i| i.destination_identity.as_ref().map(|d| d.len)).sum();
        let replacement_bytes: u64 = changing.iter().map(|i| i.source.len).sum();
        let deployable = reasons.is_empty();
        let seq = store.next_deployment_seq();
        let handle = {
            let st = &mut *projects;
            let h = st.next_plan;
            st.next_plan += 1;
            st.plans.retain(|_, p| p.project_id != id);
            st.plans.insert(h, DeployPlan { project_id: id, workspace_revision: revision, seq, items: items.clone(), allow_changed });
            h
        };
        Ok(DeployPlanDto {
            handle,
            seq,
            deployable,
            dirty,
            reasons,
            backup_bytes,
            replacement_bytes,
            items: items.iter().map(item_dto).collect(),
        })
    }

    /// Apply a reviewed deployment plan. Backups are written under the project first; every
    /// destination is then published through the two-phase in-place publisher (temp → rename →
    /// verify → reverse rollback). A refusal leaves every destination and the project intact.
    pub fn project_deploy_apply(&self, id: u32, handle: u32) -> Result<DeployApplyDto, ApiError> {
        let plan = {
            let mut projects = self.projects.lock().unwrap();
            let p = projects.plans.remove(&handle).ok_or_else(|| ApiError::Workspace(format!("deployment plan {handle} is not current (already applied, superseded or never existed); review again")))?;
            if p.project_id != id {
                return Err(ApiError::Workspace(format!("deployment plan {handle} belongs to another project")));
            }
            p
        };
        let (revision, dirty) = if self.has_session(id) {
            self.with_workspace(id, |ws| Ok((format!("{:?}", ws.revision()), ws.is_dirty())))?
        } else {
            ("assets-only".to_string(), false)
        };
        if dirty {
            return Err(ApiError::Workspace("the project has unsaved edits; save the project, then review the deployment again".into()));
        }
        if revision != plan.workspace_revision {
            return Err(ApiError::Workspace("the project changed since the deployment was reviewed; review it again".into()));
        }
        let changing: Vec<&PlannedItem> = plan.items.iter().filter(|i| i.state != "already-current").collect();
        if let Some(b) = changing.iter().find_map(|i| i.blocked.clone()) {
            return Err(ApiError::Workspace(format!("the deployment is blocked: {b}")));
        }
        if changing.is_empty() {
            return Err(ApiError::Workspace("nothing to deploy".into()));
        }
        let project_dir = self.project_dir_of(id).ok_or(ApiError::UnknownProject(id))?;
        let record_dir = project_dir.join(studio_domain::project_store::DEPLOYMENTS_DIR).join(plan.seq.to_string());
        if record_dir.exists() {
            return Err(ApiError::Workspace(format!("deployment record {} already exists; review again", record_dir.display())));
        }
        let backup_dir = record_dir.join("backups");
        std::fs::create_dir_all(&backup_dir).map_err(|source| ApiError::Io { path: backup_dir.display().to_string(), source })?;
        let backup_bytes: u64 = changing.iter().filter_map(|i| i.destination_identity.as_ref().map(|d| d.len)).sum();
        check_free_space(&project_dir, backup_bytes, "writing the deployment backups").map_err(|e| { let _ = std::fs::remove_dir_all(&record_dir); e })?;
        let replacement_bytes: u64 = changing.iter().map(|i| i.source.len).sum();
        if let Some(first) = changing.first().and_then(|i| i.destination.clone()) {
            check_free_space(first.parent().unwrap_or(&first), replacement_bytes, "writing the deployed files").map_err(|e| { let _ = std::fs::remove_dir_all(&record_dir); e })?;
        }
        let mut writes: Vec<PlannedWrite> = Vec::new();
        let mut record_items: Vec<DeployRecordItem> = Vec::new();
        for (n, item) in changing.iter().enumerate() {
            let dest = item.destination.clone().expect("checked: unresolved items are blocked");
            let expectation = match &item.destination_identity {
                Some(i) => Expectation::Identity(i.clone()),
                None => Expectation::Absent,
            };
            let prior = ips::read_prior(item.role, &dest, &expectation).map_err(|e| { let _ = std::fs::remove_dir_all(&record_dir); ApiError::Domain(e) })?;
            let backup = match &prior {
                Prior::Bytes(bytes) => {
                    let name = format!("{n:03}-{}", dest.file_name().unwrap_or_default().to_string_lossy());
                    let p = backup_dir.join(&name);
                    studio_domain::baselines::write_atomic(&p, bytes).map_err(|e| { let _ = std::fs::remove_dir_all(&record_dir); ApiError::Domain(e) })?;
                    Some(format!("backups/{name}"))
                }
                Prior::Absent => None,
            };
            if let Some((tp, id)) = &item.template {
                let now = FileIdentity::of_file(tp).map_err(|e| { let _ = std::fs::remove_dir_all(&record_dir); ApiError::Workspace(format!("{}: the sibling {} the plan cloned cannot be read now: {e}; review again", item.label, s(tp))) })?;
                if &now != id {
                    let _ = std::fs::remove_dir_all(&record_dir);
                    return Err(ApiError::Workspace(format!("{}: the sibling {} the plan cloned changed since the review; review again", item.label, s(tp))));
                }
            }
            let bytes = match &item.inline {
                Some(b) => b.clone(),
                None => std::fs::read(&item.source_path).map_err(|e| { let _ = std::fs::remove_dir_all(&record_dir); ApiError::Workspace(format!("{}: cannot read the project's file: {e}", item.label)) })?,
            };
            let identity = FileIdentity::of_bytes(&bytes);
            if identity != item.source {
                let _ = std::fs::remove_dir_all(&record_dir);
                return Err(ApiError::Workspace(format!("{}: the project's file changed since the deployment was reviewed; review again", item.label)));
            }
            record_items.push(DeployRecordItem { key: item.key.clone(), label: item.label.clone(), destination: s(&dest), prior: item.destination_identity.clone().map(|i| studio_domain::project_store::IdentityRecord::from(&i)), replacement: studio_domain::project_store::IdentityRecord::from(&identity), backup, state: "planned".into() });
            writes.push(PlannedWrite { role: item.role, path: dest, prior, bytes, identity });
        }
        let mut record = DeployRecord { schema: "nhlms-deployment/1".into(), seq: plan.seq, at: studio_domain::project_store::now_iso(), state: "publishing".into(), items: record_items };
        write_record(&record_dir, &record)?;
        {
            let mut projects = self.projects.lock().unwrap();
            if let Some(b) = projects.bindings.get_mut(&id) {
                b.store.record_deployment(plan.seq, "publishing").map_err(ApiError::Domain)?;
            }
        }
        let outcome = ips::publish(&writes, &mut ips::NoHooks);
        let (state, detail) = match &outcome {
            ips::Outcome::Committed => ("committed", None),
            ips::Outcome::RolledBack { .. } => ("rolled-back", Some(outcome.describe(&writes))),
            ips::Outcome::Split { .. } => ("split", Some(outcome.describe(&writes))),
        };
        for it in record.items.iter_mut() {
            it.state = super::recovery::state_on_disk(Path::new(&it.destination), it.prior.as_ref().map(|p| p.to_identity()).as_ref(), &it.replacement.to_identity(), "deployed");
        }
        record.state = state.into();
        write_record(&record_dir, &record)?;
        {
            let mut projects = self.projects.lock().unwrap();
            if let Some(b) = projects.bindings.get_mut(&id) {
                b.store.record_deployment(plan.seq, state).map_err(ApiError::Domain)?;
            }
        }
        let dto = DeployApplyDto {
            seq: plan.seq,
            state: state.into(),
            detail,
            record_dir: s(&record_dir),
            destinations: record.items.iter().map(|i| DeployOutcomeDto { key: i.key.clone(), label: i.label.clone(), path: i.destination.clone(), state: i.state.clone(), backup: i.backup.clone() }).collect(),
        };
        if state == "committed" {
            Ok(dto)
        } else {
            Err(ApiError::Workspace(format!("the deployment did not commit ({state}): {}", dto.detail.clone().unwrap_or_default())))
        }
    }

    /// Restore a deployment (or recover an interrupted one): every destination is classified by
    /// what it holds now (see [`super::recovery`]) — the deployed bytes are put back from the
    /// backups and a created sidecar is removed, as one coordinated step set; a later foreign
    /// change refuses the whole restore and nothing is touched.
    pub fn project_deploy_restore(&self, id: u32, seq: u64) -> Result<DeployRestoreDto, ApiError> {
        self.project_deploy_restore_with(id, seq, &mut ips::NoHooks)
    }

    /// [`Registry::project_deploy_restore`] with publication hooks (fault injection in tests).
    pub fn project_deploy_restore_with(&self, id: u32, seq: u64, hooks: &mut dyn studio_domain::in_place_save::Hooks) -> Result<DeployRestoreDto, ApiError> {
        let project_dir = self.project_dir_of(id).ok_or(ApiError::UnknownProject(id))?;
        let record_dir = project_dir.join(studio_domain::project_store::DEPLOYMENTS_DIR).join(seq.to_string());
        let mut record = read_record(&record_dir)?;
        if !super::recovery::recoverable(&record.state) {
            return Err(ApiError::Workspace(format!("deployment {seq} is {}; there is nothing to restore or recover", record.state)));
        }
        let items: Vec<super::recovery::RecordedItem> = record
            .items
            .iter()
            .map(|it| super::recovery::RecordedItem {
                label: it.label.clone(),
                destination: PathBuf::from(&it.destination),
                prior: it.prior.as_ref().map(|p| p.to_identity()),
                replacement: it.replacement.to_identity(),
                backup: it.backup.as_ref().map(|b| record_dir.join(b.replace('/', std::path::MAIN_SEPARATOR_STR))),
            })
            .collect();
        let plan = super::recovery::plan_restore(&items, "deployed")?;
        let outcome = if super::recovery::nothing_to_do(&plan) { ips::Outcome::Committed } else { ips::publish_all(&plan.writes, &plan.removals, hooks) };
        let (state, item_states) = super::recovery::classify(&outcome, &items, &plan, "deployed", &record.state);
        for (it, st) in record.items.iter_mut().zip(item_states) {
            it.state = st;
        }
        record.state = state.clone();
        write_record(&record_dir, &record)?;
        {
            let mut projects = self.projects.lock().unwrap();
            if let Some(b) = projects.bindings.get_mut(&id) {
                b.store.record_deployment(seq, &state).map_err(ApiError::Domain)?;
            }
        }
        if state != "restored" {
            return Err(ApiError::Workspace(format!(
                "the restore did not complete (record {}): {}; retry once the destination is reachable again — every destination still holding the deployed bytes is put back then",
                state,
                outcome.describe_all(&plan.writes, &plan.removals)
            )));
        }
        Ok(DeployRestoreDto { seq, state, items: record.items.iter().map(|i| DeployOutcomeDto { key: i.key.clone(), label: i.label.clone(), path: i.destination.clone(), state: i.state.clone(), backup: i.backup.clone() }).collect() })
    }

    pub fn project_deployments(&self, id: u32) -> Result<Vec<DeploymentRecordDto>, ApiError> {
        let projects = self.projects.lock().unwrap();
        let b = projects.bindings.get(&id).ok_or_else(|| ApiError::Workspace(format!("project {id} is not a project-folder session")))?;
        Ok(b.store.manifest.deployments.iter().map(|d| DeploymentRecordDto { seq: d.seq, at: d.at.clone(), state: d.state.clone(), restorable: super::recovery::recoverable(&d.state), recovery: d.state != "committed" && super::recovery::recoverable(&d.state) }).collect())
    }
}

// ---------------------------------------------------------------------- records

#[derive(serde::Serialize, serde::Deserialize)]
struct DeployRecord {
    schema: String,
    seq: u64,
    at: String,
    state: String,
    items: Vec<DeployRecordItem>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct DeployRecordItem {
    key: String,
    label: String,
    destination: String,
    prior: Option<studio_domain::project_store::IdentityRecord>,
    replacement: studio_domain::project_store::IdentityRecord,
    backup: Option<String>,
    state: String,
}

fn write_record(dir: &Path, r: &DeployRecord) -> Result<(), ApiError> {
    let text = serde_json::to_string_pretty(r).map_err(|e| ApiError::Workspace(e.to_string()))?;
    studio_domain::baselines::write_atomic(&dir.join("record.json"), text.as_bytes()).map_err(ApiError::Domain)
}

fn read_record(dir: &Path) -> Result<DeployRecord, ApiError> {
    let text = std::fs::read_to_string(dir.join("record.json")).map_err(|e| ApiError::Workspace(format!("no deployment record in {}: {e}", dir.display())))?;
    serde_json::from_str(&text).map_err(|e| ApiError::Workspace(format!("the deployment record in {} is not readable: {e}", dir.display())))
}

/// `key → identity` of the files the most recent committed deployment placed.
fn read_last_deployed(store: &ProjectStore) -> HashMap<String, FileIdentity> {
    let mut out = HashMap::new();
    for d in store.manifest.deployments.iter().filter(|d| d.state == "committed") {
        if let Ok(r) = read_record(&store.deployments_dir().join(d.seq.to_string())) {
            for it in r.items {
                out.insert(it.key, it.replacement.to_identity());
            }
        }
    }
    out
}

fn item_dto(i: &PlannedItem) -> DeployItemDto {
    DeployItemDto {
        key: i.key.clone(),
        role: i.role.into(),
        label: i.label.clone(),
        source_path: s(&i.source_path),
        source: identity_dto(&i.source),
        destination_path: i.destination.as_ref().map(|p| s(p)),
        destination: i.destination_identity.as_ref().map(identity_dto),
        state: i.state.into(),
        blocked: i.blocked.clone(),
    }
}

fn push_destination_roots<'a>(d: &Destinations, others: &mut Vec<(&'a str, PathBuf)>) {
    if let Some(p) = &d.install_dir {
        others.push(("the game's database folder", PathBuf::from(p)));
    }
    if let Some(p) = &d.game_dir {
        others.push(("the game folder", PathBuf::from(p)));
    }
    if let Some(p) = &d.save_root {
        others.push(("the save location", PathBuf::from(p)));
    }
    if let Some(p) = &d.assets_dir {
        others.push(("the extracted-assets folder", PathBuf::from(p)));
    }
}

fn st_path(store: &BaselineStore, c: &studio_domain::baselines::Component) -> PathBuf {
    store.component_path(c)
}

fn review_file(role: &str, path: &Path, check: impl FnOnce(&Path) -> Result<String, String>) -> ImportItemReviewDto {
    let identity = FileIdentity::of_file(path);
    match identity {
        Err(e) => ImportItemReviewDto { role: role.into(), path: s(path), len: 0, sha256: String::new(), ok: false, detail: format!("cannot be read: {e}") },
        Ok(id) => match check(path) {
            Ok(detail) => ImportItemReviewDto { role: role.into(), path: s(path), len: id.len, sha256: id.sha256, ok: true, detail },
            Err(detail) => ImportItemReviewDto { role: role.into(), path: s(path), len: id.len, sha256: id.sha256, ok: false, detail },
        },
    }
}

/// ONB-003: an extracted-assets folder is accepted when it is a readable, non-empty directory;
/// the detail names what the platform's asset layout found there (informational, not a gate).
fn review_dir(role: &str, path: &Path, platform: Platform) -> ImportItemReviewDto {
    let mut item = ImportItemReviewDto { role: role.into(), path: s(path), len: 0, sha256: String::new(), ok: false, detail: String::new() };
    match std::fs::read_dir(path) {
        Err(e) => item.detail = format!("cannot be listed: {e}"),
        Ok(mut rd) => {
            if rd.next().is_none() {
                item.detail = "the folder is empty".into();
                return item;
            }
            let roots = studio_domain::AssetRoots::resolve(path, platform);
            let mut found = Vec::new();
            if Path::new(&roots.rendering).is_dir() {
                found.push(format!("rendering ({})", roots.rendering));
            }
            if Path::new(&roots.fe).is_dir() {
                found.push(format!("fe ({})", roots.fe));
            }
            let bigs = std::fs::read_dir(path).map(|rd| rd.flatten().filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("big"))).count()).unwrap_or(0);
            if bigs > 0 {
                found.push(format!("{bigs} .big archive(s) at the top level"));
            }
            item.ok = true;
            item.detail = if found.is_empty() {
                "extracted-assets folder (no rendering/ or fe/ tree recognised at its top level; asset routes resolve paths under it as given)".into()
            } else {
                format!("extracted-assets folder: {}", found.join(", "))
            };
        }
    }
    item
}

fn platform_label(p: Platform) -> &'static str {
    match p {
        Platform::Ps3 => "PS3",
        Platform::Xbox => "Xbox 360",
        Platform::Ps4 => "PS4",
        Platform::Recomp => "Recompiled",
    }
}

pub(crate) fn project_info_dto(store: &ProjectStore) -> ProjectInfoDto {
    let m = &store.manifest;
    let d = &m.destinations;
    ProjectInfoDto {
        dir: s(&store.dir),
        project_id: m.project_id.clone(),
        created: m.created.clone(),
        game_id: m.game_id.clone(),
        platform: m.platform.clone(),
        title: m.title.clone(),
        source_kind: m.source.kind.clone(),
        inventory_head: m.source.inventory_head.clone(),
        components: m.source.components.iter().filter(|c| !c.role.starts_with("roster_save")).map(|c| ProjectComponentDto { role: c.role.clone(), file_name: c.file_name.clone(), origin: c.origin.clone(), len: c.len, sha256: c.sha256.clone(), provenance: c.provenance.clone(), witness: c.witness.clone() }).collect(),
        gaps: m.source.gaps.iter().filter(|g| !g.component.starts_with("roster_save")).map(|g| BaselineGapDto { component: g.component.clone(), reason: g.reason.clone() }).collect(),
        destinations: ProjectDestinationsDto {
            picked_root: d.picked_root.clone(),
            install_dir: d.install_dir.clone(),
            game_dir: d.game_dir.clone(),
            save_root: None,
            roster_save: None,
            main_db_path: d.main_db_path.clone(),
            meta_xml_path: d.meta_xml_path.clone(),
            localization_path: d.localization_path.clone(),
            assets_dir: d.assets_dir.clone(),
        },
        legacy_sidecar: m.legacy_sidecar.clone(),
        extraction_choice: m.extraction.choice.clone(),
        working_dir: s(&store.working_dir()),
        exports_dir: s(&store.exports_dir()),
        working_main_db: m.working.main_db.as_ref().map(|r| s(&store.path(r))),
        working_roster_save: None,
        working_dictionary: m.working.dictionary.as_ref().map(|r| s(&store.path(r))),
        saves: m.saves.len() as u64,
        last_saved: m.last_save_at.clone().or_else(|| m.saves.last().map(|x| x.at.clone())),
        deployments: m.deployments.iter().map(|x| DeploymentRecordDto { seq: x.seq, at: x.at.clone(), state: x.state.clone(), restorable: super::recovery::recoverable(&x.state), recovery: x.state != "committed" && super::recovery::recoverable(&x.state) }).collect(),
        staged_files: m.staged_files.iter().map(|f| StagedFileDto { relative: f.relative.clone(), origin: f.origin.clone(), at: f.at.clone(), source_sha256: f.source_sha256.clone() }).collect(),
        mode: project_mode(&m.working).into(),
        inputs: m.inputs.iter().filter(|i| i.role != INPUT_ROSTER).map(|i| ProjectInputDto { role: i.role.clone(), origin: i.origin.clone(), note: i.note.clone() }).collect(),
        last_view: m.last_view.clone(),
        migrated_from: m.migrated_from.clone(),
        schema: m.schema.clone(),
        sources: m.sources.len() as u64,
        member_edits: m.member_edits.len() as u64,
    }
}

/// Refuse when the volume holding `path` has less than `needed` bytes free (plus a 64 MiB margin).
pub(crate) fn check_free_space(path: &Path, needed: u64, what: &str) -> Result<(), ApiError> {
    const MARGIN: u64 = 64 * 1024 * 1024;
    let Some(free) = studio_domain::free_space(path) else { return Ok(()) };
    if free < needed.saturating_add(MARGIN) {
        return Err(ApiError::Workspace(format!(
            "{what} needs {} MiB but the volume holding {} has only {} MiB free; nothing was written",
            needed / (1024 * 1024),
            shown(path).display(),
            free / (1024 * 1024)
        )));
    }
    Ok(())
}

/// Used by the write guards: whether `path` lies inside the open project's own folder (writes
/// there are the project's own business).
#[allow(dead_code)]
pub(crate) fn inside_project(project_dir: &Path, path: &Path) -> bool {
    match (canonical_or_planned(project_dir), canonical_or_planned(path)) {
        (Some(p), Some(t)) => is_within(&t, &p),
        _ => false,
    }
}

#[cfg(test)]
mod save_destination_tests {
    use super::*;
    #[test]
    fn default_xenia_discovery_needs_an_explicit_save_location() {
        let mut slot = studio_domain::discovery::SaveSlot {
            path: "discovered/default/content/roster".into(), kind: "roster".into(),
            container_version: "v2".into(), writable: true,
            profile: None, title_id: None, problem: None,
        };
        for platform in [Platform::Xbox, Platform::Recomp] {
            for blank in [None, Some(""), Some("  ")] {
                assert_eq!(automatic_roster_destination(platform, blank, &[slot.clone()]), None);
            }
            assert_eq!(automatic_roster_destination(platform, Some("selected/save/location"), &[slot.clone()]), Some(slot.path.clone()));
        }
        // PS3's saves remain discoverable inside the explicitly selected HDD.
        assert_eq!(automatic_roster_destination(Platform::Ps3, None, &[slot.clone()]), Some(slot.path.clone()));
        slot.writable = false;
        assert_eq!(automatic_roster_destination(Platform::Xbox, Some("selected/save/location"), &[slot]), None);
    }
}
