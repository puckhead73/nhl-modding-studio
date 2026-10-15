//! ONB-003 §4–§5 — the mod package builder (from an open, saved project) and the built-in
//! installer (from a package file onto an owned destination), both on the [`Registry`].
//!
//! **Builder.** The package carries *semantic* edits measured between the project's immutable
//! baseline copies and its saved working copies: database cells (table, row identity, field,
//! before, after, with the entity and field names), dictionary texts by key, and loose texture
//! replacements with their authored source images. Structural table changes (rows added or
//! removed), archive-member replacements and native-only outputs are inventoried and **block** the
//! build while selected — nothing is omitted silently; the author narrows the selection explicitly.
//!
//! **Row identity (R71-R1).** A cell edit names its row by the table's metadata **key fields** and
//! the values the author's baseline row held, plus a fingerprint of that row; the installer resolves
//! the recipient's row by those keys (exactly one match) — never by the row number the author saw.
//! A keyless table falls back to the position only when the recipient's row there is byte-identical
//! to the author's baseline row. A row that cannot be found, is ambiguous or differs is refused.
//!
//! **File roles (R71-R2).** `main_db` edits touch the main database only, `roster` edits the roster
//! save only, `both` requires both files and is replayed in each independently (its own row
//! resolution and before-value check per file). Nothing is mirrored by the installer; a role whose
//! file is missing blocks the whole plan; a package that needs no roster never asks for one.
//!
//! **Installer.** Reads and validates the package (container, hashes, paths, limits), checks the
//! chosen destination (same game, declared platform), requires only what the package needs, stages
//! every conversion in an installer-owned folder (the recipient's current files are copied there and
//! the edits replayed through the same edit session the Studio uses), reports every planned
//! destination change — including a save-index sidecar the destination layout requires — and every
//! conflict, and writes only on an explicit Apply through the two-phase in-place publisher with
//! backups under an installation record. Restore and recovery after an interrupted or split
//! operation go through the shared checked route in [`super::recovery`].
//!
//! **Platforms.** Cells on another platform of the same title only when its bundled baseline
//! database is byte-identical to the author's (measured); dictionary edits are key-based and
//! verified at install; loose textures on another platform only where the title's loose-asset
//! layout is registered as shared (NHL Legacy Xbox 360 ↔ Recompiled), and every placement is then
//! verified against the recipient's own file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use studio_domain::baselines::{sha256_hex, BaselineStore, ROLE_DICTIONARY, ROLE_MAIN_DB, ROLE_META};
use studio_domain::in_place_save::{self as ips, Expectation, Hooks, PlannedWrite, Prior};
use studio_domain::mod_package::{
    self as pkg, BaseIdentity, CategoryCount, CellOp, CellValue, ComponentSupport, DictionaryOp, InspectedPackage, KeyValue,
    PackageEdits, PackageManifest, ProjectProvenance, StudioProvenance, TargetSupport, TextureOp,
};
use studio_domain::project_store::{shown, ProjectStore};
use studio_domain::tdb_txn::FileIdentity;
use studio_domain::{EditSession, FieldRaw, FieldValue, FieldView, InstallReport, Platform, Project, ProjectRole, Workspace};

use super::recovery;
use super::{build_session, ps4_chosen_roster, Registry};
use crate::dto::{
    DeployOutcomeDto, FileIdentityDto, InstallApplyDto, InstallItemDto, InstallPlanDto, InstallRecordDto,
    InstallRequirementDto, PackageBuildDto, PackageBuildRequestDto, PackageCategoryDto, PackageCellDto,
    PackageComponentDto, PackageDictionaryDto, PackageEntryDto, PackageInspectDto, PackageInventoryDto,
    PackageTargetDto, PackageTextureDto,
};
use crate::error::ApiError;

pub const CAT_CELLS: &str = "cells";
pub const CAT_DICTIONARY: &str = "dictionary";
pub const CAT_TEXTURES: &str = "textures";
pub const CAT_MEMBERS: &str = "members";
pub const CAT_NATIVE: &str = "native";
pub const CAT_STRUCTURAL: &str = "structural";
pub const RECORD_SCHEMA: &str = "nhlms-install/1";

/// Installer plans held server-side, keyed by handle.
#[derive(Default)]
pub(crate) struct PackageState {
    plans: HashMap<u32, InstallPlan>,
    next: u32,
}

struct InstallPlan {
    records_dir: PathBuf,
    package: PackageIdentity,
    game_id: String,
    platform: Platform,
    picked_root: String,
    save_root: Option<String>,
    items: Vec<PlannedInstall>,
    blocked: Vec<String>,
}

#[derive(Clone)]
struct PackageIdentity {
    package_id: String,
    name: String,
    version: String,
    file_sha256: String,
}

#[derive(Clone)]
struct PlannedInstall {
    key: String,
    role: &'static str,
    label: String,
    destination: PathBuf,
    destination_identity: Option<FileIdentity>,
    bytes: Vec<u8>,
    replacement: FileIdentity,
    /// `replace` | `create` | `unchanged`.
    state: &'static str,
    notes: Vec<String>,
    /// A source the planned bytes were derived from (a sibling header the sidecar was cloned
    /// from): it must be unchanged when the plan is applied.
    template: Option<(PathBuf, FileIdentity)>,
}

fn s(p: &Path) -> String {
    shown(p).to_string_lossy().into_owned()
}

fn identity_dto(i: &FileIdentity) -> FileIdentityDto {
    FileIdentityDto { len: i.len, sha256: i.sha256.clone() }
}

fn cell_value(v: &FieldView) -> CellValue {
    match &v.raw {
        FieldRaw::Int(raw) => CellValue::Int { raw: *raw, decoded: v.decoded.unwrap_or(*raw as i64), display: v.display.clone() },
        FieldRaw::Text(t) => CellValue::Text { text: t.clone() },
    }
}

fn value_display(v: &CellValue) -> String {
    match v {
        CellValue::Int { display, .. } => display.clone(),
        CellValue::Text { text } => text.clone(),
    }
}

fn cell_dto(c: &CellOp) -> PackageCellDto {
    PackageCellDto {
        file: c.file.clone(),
        table_name: c.table_name.clone(),
        record: c.record,
        entity: c.entity.clone(),
        field_name: c.field_name.clone(),
        before: value_display(&c.before),
        after: value_display(&c.after),
    }
}

/// What a saved project's baseline ↔ working comparison yields, per category.
struct ProjectEdits {
    schedules: Vec<pkg::ScheduleOp>,
    career_removals: Vec<pkg::CareerRemovalOp>,
    free_agents: Option<pkg::FreeAgentPoolOp>,
    cells: Vec<CellOp>,
    dictionary: Vec<DictionaryOp>,
    textures: Vec<(TextureOp, Vec<u8>, String)>,
    /// Per blocked category: the reasons.
    structural: Vec<String>,
    members: Vec<String>,
    native: Vec<String>,
    base: Vec<BaseIdentity>,
}

/// The key fields of a table as the title's metadata declares them.
fn key_fields<'a>(db: &tdb_core::TdbFile, meta: &tdb_defs::FieldMetaIndex, table: &'a tdb_core::Table) -> Vec<&'a tdb_core::FieldDescriptor> {
    let _ = db;
    table.fields.iter().filter(|f| meta.field(&table.id.raw(), &f.id.raw()).is_some_and(|m| m.key)).collect()
}

/// SHA-256 of one fixed-length record's bytes.
fn row_fingerprint(bytes: &[u8], table: &tdb_core::Table, record: u32) -> Option<String> {
    let len = table.record_len_bytes as usize;
    let start = table.record_block.checked_add((record as usize).checked_mul(len)?)?;
    bytes.get(start..start.checked_add(len)?).map(sha256_hex)
}

/// Compare two files of one role through the title's codecs and name every changed cell with its
/// row identity (key values and fingerprint from the baseline row).
fn cells_between(baseline: &Path, working: &Path, meta: &Path, platform: Platform, role: ProjectRole, defs: &str, file: &str, schedule_ops:bool) -> Result<(Vec<CellOp>, Vec<String>, Vec<pkg::CareerRemovalOp>), ApiError> {
    let base = Project::load(baseline, platform, role, meta, defs).map_err(|e| ApiError::Workspace(format!("the project's baseline {} does not open: {e}", s(baseline))))?;
    let work = Project::load(working, platform, role, meta, defs).map_err(|e| ApiError::Workspace(format!("the project's saved {} does not open: {e}", s(working))))?;
    let pending = studio_domain::pending::pending_changes(base.db_bytes(), work.db_bytes(), work.meta(), work.enums(), None, usize::MAX);
    if let Some(e) = pending.error {
        return Err(ApiError::Workspace(format!("the saved {} could not be compared with the baseline: {e}", s(working))));
    }
    let bdb = tdb_core::TdbFile::parse(base.db_bytes()).map_err(|e| ApiError::Workspace(format!("the project's baseline {} does not parse: {e}", s(baseline))))?;
    let is_schedule = |table: &str| schedule_ops && bdb.table_canonical(table).and_then(|t| base.meta().table_name(&t.id.raw())).is_some_and(|name| matches!((table,name),
        ("vufv","intldelschedule")|("fBWa","intlelitserienschedule")|("ADlP","intlsmliigaschedule")|("uRbu","nhlschedulelongleap")|("ksGb","nhlschedulelongnoleap")|("qyyN","nhlschedulemedium")|("zcVp","nhlschedulesmall")));
    let mut cells = Vec::new();
    for c in &pending.cells {
        if c.studio_table == "VoEQ" || is_schedule(&c.studio_table) { continue; }
        let table = bdb.table_canonical(&c.studio_table).ok_or_else(|| ApiError::Workspace(format!("{}: table {} is not in the baseline", c.entity, c.table_name)))?;
        let keys: Vec<KeyValue> = key_fields(&bdb, base.meta(), table)
            .into_iter()
            .map(|f| {
                let view = studio_domain::read_field(&bdb, base.meta(), base.enums(), table, c.record, f);
                let name = base.meta().field(&table.id.raw(), &f.id.raw()).map(|m| m.name.clone()).unwrap_or_else(|| f.id.canonical(bdb.endian));
                KeyValue { studio_field: f.id.canonical(bdb.endian), field_name: name, value: cell_value(&view) }
            })
            .collect();
        let row_sha256 = row_fingerprint(base.db_bytes(), table, c.record).ok_or_else(|| ApiError::Workspace(format!("{}: row {} of {} is outside the baseline file", c.entity, c.record, c.table_name)))?;
        cells.push(CellOp {
            file: file.into(),
            studio_table: c.studio_table.clone(),
            table_name: c.table_name.clone(),
            record: c.record,
            entity: c.entity.clone(),
            studio_field: c.studio_field.clone(),
            field_name: c.field_name.clone(),
            before: cell_value(&c.before),
            after: cell_value(&c.after),
            keys,
            row_sha256: Some(row_sha256),
        });
    }
    let mut structural = Vec::new();
    let mut career_removals = Vec::new();
    for t in &pending.tables {
        if t.studio_table == "VoEQ" || is_schedule(&t.studio_table) { continue; }
        if schedule_ops && ["tsSy", "OAqO", "USfF", "nCDp"].contains(&t.studio_table.as_str()) && t.rows_only_in_working.is_empty() && !t.rows_only_in_baseline.is_empty() &&
            t.before_records.zip(t.after_records).is_some_and(|(before, after)| before.checked_sub(after) == Some(t.rows_only_in_baseline.len() as u32)) {
            let table = bdb.table_canonical(&t.studio_table).ok_or_else(|| ApiError::Workspace("Missing career table".into()))?;
            let before_rows = (0..t.before_records.unwrap()).map(|r| row_fingerprint(base.db_bytes(), table, r).ok_or_else(|| ApiError::Workspace("Invalid career row".into()))).collect::<Result<Vec<_>, _>>()?;
            // pending_changes reports row-count tails, not semantic deletions. Match
            // the surviving ordered records to avoid interpreting shifts as cell edits.
            let wdb = tdb_core::TdbFile::parse(work.db_bytes()).map_err(|e| ApiError::Workspace(e.to_string()))?;
            let wt = wdb.table_canonical(&t.studio_table).ok_or_else(|| ApiError::Workspace("Missing saved career table".into()))?;
            let record = |bytes: &[u8], table: &tdb_core::Table, r: u32| -> Option<Vec<u8>> {
                let len = table.record_len_bytes as usize;
                bytes.get(table.record_block + r as usize * len..table.record_block + (r as usize + 1) * len).map(<[u8]>::to_vec)
            };
            let base_rows = (0..t.before_records.unwrap()).map(|r| record(base.db_bytes(), table, r)).collect::<Option<Vec<_>>>().ok_or_else(|| ApiError::Workspace("Invalid career row".into()))?;
            let saved_rows = (0..u32::from(wt.current_records)).map(|r| record(work.db_bytes(), wt, r)).collect::<Option<Vec<_>>>().ok_or_else(|| ApiError::Workspace("Invalid saved career row".into()))?;
            // Align the saved rows onto the original ones: a matching row survives unchanged, an
            // original row is removed when a later one (within the remaining removals) matches,
            // and otherwise the original row survives with the author's edits. Any alignment
            // reproduces the saved table exactly; this one keeps rewrites to the rows edited.
            if wt.record_len_bytes == table.record_len_bytes {
                // Fields that differ between two rows (wide text fields compared as bytes).
                let differing = |a: &[u8], b: &[u8]| -> Vec<&tdb_core::FieldDescriptor> {
                    table.fields.iter().filter(|f| if f.bit_width > 64 { row_bits(a, f) != row_bits(b, f) } else {
                        tdb_core::read_bits(a, f.bit_offset, f.bit_width, bdb.endian) != tdb_core::read_bits(b, f.bit_offset, f.bit_width, bdb.endian) }).collect()
                };
                let (mut removed, mut rewritten) = (Vec::new(), Vec::new());
                let (mut i, mut j) = (0usize, 0usize);
                let mut keyed = false;
                while j < saved_rows.len() {
                    if base_rows[i] == saved_rows[j] { i += 1; j += 1; continue; }
                    // Pair the saved row with the closest original row still available (an exact
                    // match first, else the fewest differing fields); the rows skipped are removed.
                    let spare = (base_rows.len() - i) - (saved_rows.len() - j);
                    let skip = (0..=spare).find(|&d| base_rows[i + d] == saved_rows[j]).unwrap_or_else(|| {
                        (0..=spare).min_by_key(|&d| (differing(&base_rows[i + d], &saved_rows[j]).len(), d)).unwrap_or(0)
                    });
                    removed.extend((i..i + skip).map(|r| r as u32));
                    i += skip;
                    if base_rows[i] != saved_rows[j] {
                        keyed |= differing(&base_rows[i], &saved_rows[j]).iter().any(|f| base.meta().field(&table.id.raw(), &f.id.raw()).is_some_and(|m| m.key));
                        rewritten.push(pkg::CareerRowRewrite { row: i as u32, bytes: pkg::to_hex(&saved_rows[j]) });
                    }
                    i += 1; j += 1;
                }
                removed.extend((i..base_rows.len()).map(|r| r as u32));
                // A survivor whose player changed is not an edit the installer can replay.
                if !keyed {
                    career_removals.push(pkg::CareerRemovalOp { table: t.studio_table.clone(), before_rows, removed, rewritten });
                    continue;
                }
            }
        }
        structural.push(format!(
            "{} ({}): {} — rows only in the saved file: {}, only in the baseline: {}. Row insertions and removals cannot be expressed as portable edits yet",
            t.table_name,
            file.replace('_', " "),
            t.note,
            t.rows_only_in_working.len(),
            t.rows_only_in_baseline.len()
        ));
    }
    cells.retain(|c| !career_removals.iter().any(|op| op.table == c.studio_table));
    Ok((cells, structural, career_removals))
}

fn project_edits(store: &ProjectStore, defs: &str) -> Result<ProjectEdits, ApiError> {
    let m = &store.manifest;
    let platform = super::parse_platform(&m.platform)?;
    let baseline_of = |role: &str| m.source.components.iter().find(|c| c.role == role);
    let mut free_agents = None;
    let mut schedules = Vec::new();
    let mut career_removals = Vec::new();
    let mut cells = Vec::new();
    let mut structural = Vec::new();
    let mut base = Vec::new();
    let meta = m.working.meta.as_ref().map(|r| store.path(r));
    if let (Some(main), Some(meta)) = (&m.working.main_db, &meta) {
        if let Some(b) = baseline_of(ROLE_MAIN_DB) {
            let (c, st, cr) = cells_between(&store.path(&b.baseline), &store.path(main), meta, platform, ProjectRole::Nhlng, defs, "main_db", m.game_id == "nhl07" && m.platform == "xbox")?;
            let before_bytes = std::fs::read(store.path(&b.baseline)).map_err(|e| ApiError::Workspace(e.to_string()))?;
            let after_bytes = std::fs::read(store.path(main)).map_err(|e| ApiError::Workspace(e.to_string()))?;
            if m.game_id == "nhl07" && m.platform == "xbox" {
                let load = |path: &Path| -> Result<studio_domain::EditSession,ApiError> {
                    let mut p=Project::load(path,platform,ProjectRole::Nhlng,meta,defs)?;
                    p.set_game_id(Some("nhl07".into())); Ok(studio_domain::EditSession::begin(p))
                };
                let original=load(&store.path(&b.baseline))?; let current=load(&store.path(main))?;
                for t in original.schedule_tables()? {
                    if t.league_code.is_none() { continue; }
                    let before=original.schedule_model(&t.studio_table)?;
                    if !studio_domain::schedule_edit::supports_resize(&before) {continue;}
                    let after=current.schedule_model(&t.studio_table)?;
                    if before.rows != after.rows {schedules.push(pkg::ScheduleOp{table:t.studio_table,before:before.rows,after:after.rows});}
                }
            }
            let before = studio_domain::explicit_free_agents(&before_bytes).map_err(|e| ApiError::Workspace(e.to_string()))?;
            let after = studio_domain::explicit_free_agents(&after_bytes).map_err(|e| ApiError::Workspace(e.to_string()))?;
            if before != after {
                if m.game_id != "nhl07" || m.platform != "xbox" {
                    return Err(ApiError::Workspace("Free-agent packages are currently qualified only for NHL 07 Xbox 360".into()));
                }
                free_agents = Some(pkg::FreeAgentPoolOp { before: before.ok_or_else(|| ApiError::Workspace("Missing baseline free-agent list".into()))?, after: after.ok_or_else(|| ApiError::Workspace("Missing saved free-agent list".into()))? });
            }
            career_removals.extend(cr);
            cells.extend(c);
            structural.extend(st);
            base.push(BaseIdentity { role: ROLE_MAIN_DB.into(), file_name: b.file_name.clone(), len: b.len, sha256: b.sha256.clone(), provenance: b.provenance.clone() });
        }
    }
    let mut dictionary = Vec::new();
    if let Some(d) = &m.working.dictionary {
        if let Some(b) = baseline_of(ROLE_DICTIONARY) {
            base.push(BaseIdentity { role: ROLE_DICTIONARY.into(), file_name: b.file_name.clone(), len: b.len, sha256: b.sha256.clone(), provenance: b.provenance.clone() });
            let bb = std::fs::read(store.path(&b.baseline)).map_err(|e| ApiError::Workspace(format!("the baseline dictionary cannot be read: {e}")))?;
            let wb = std::fs::read(store.path(d)).map_err(|e| ApiError::Workspace(format!("the saved dictionary cannot be read: {e}")))?;
            if bb != wb {
                let bd = studio_domain::tdb_loc::LocDictionary::parse(&bb).map_err(|e| ApiError::Workspace(format!("the baseline dictionary does not parse: {e}")))?;
                let wd = studio_domain::tdb_loc::LocDictionary::parse(&wb).map_err(|e| ApiError::Workspace(format!("the saved dictionary does not parse: {e}")))?;
                // Rows are compared in position: an in-place re-point / re-encode keeps the row
                // set, a growth (added keys) changes it and is a structural change this version
                // cannot carry. Keys are compared as stored, never resolved through the hash.
                if bd.rows.len() != wd.rows.len() {
                    structural.push(format!("the saved dictionary has {} rows and the baseline {}; added or removed dictionary keys cannot be expressed as portable edits yet", wd.rows.len(), bd.rows.len()));
                } else {
                    for (b, w) in bd.rows.iter().zip(wd.rows.iter()) {
                        if b.key != w.key || b.hash != w.hash {
                            structural.push(format!("dictionary row {:?} became {:?}; re-keyed rows cannot be expressed as portable edits yet", b.key, w.key));
                            continue;
                        }
                        if b.text != w.text {
                            let Some(key) = &w.key else {
                                structural.push("a dictionary row without a key changed; it cannot be addressed portably".into());
                                continue;
                            };
                            dictionary.push(DictionaryOp { key: key.clone(), before: b.text.clone().unwrap_or_default(), after: w.text.clone().unwrap_or_default() });
                        }
                    }
                }
            }
        }
    }
    let mut textures = Vec::new();
    let mut native = Vec::new();
    for sf in m.staged_files.iter().flat_map(|file| {
        if file.texture_layers.is_empty() || file.origin != "texture-layers" { return vec![file.clone()]; }
        file.texture_layers.iter().map(|layer| {
            let mut entry = file.clone();
            entry.origin = "texture-replace".into();
            entry.source_sha256 = Some(layer.source_sha256.clone());
            entry.texture_index = layer.texture_index;
            entry
        }).collect()
    }) {
        match &sf.source_sha256 {
            Some(sha) if matches!(sf.origin.as_str(), "texture-replace" | "nameplate") => {
                let Some(src) = m.sources.iter().find(|x| &x.sha256 == sha) else {
                    native.push(format!("{}: its source record {sha} is missing from the project", sf.relative));
                    continue;
                };
                let bytes = std::fs::read(store.path(&src.relative)).map_err(|e| ApiError::Workspace(format!("the project source {} cannot be read: {e}", src.relative)))?;
                if sha256_hex(&bytes) != src.sha256 {
                    return Err(ApiError::Workspace(format!("the project source {} does not match its recorded hash; the project is damaged", src.relative)));
                }
                let img = studio_domain::decode_source_image_bytes(&bytes).map_err(|e| ApiError::Workspace(format!("the project source {} does not decode: {e}", src.relative)))?;
                let ext = src.relative.rsplit('.').next().unwrap_or("bin").to_string();
                let payload = format!("payloads/{}.{ext}", src.sha256);
                textures.push((
                    TextureOp {
                        rel_path: sf.relative.clone(),
                        texture_index: sf.texture_index,
                        payload: payload.clone(),
                        width: img.width,
                        height: img.height,
                        prior_len: sf.prior.as_ref().map(|p| p.len),
                        prior_sha256: sf.prior.as_ref().map(|p| p.sha256.clone()),
                        platform: m.platform.clone(),
                    },
                    bytes,
                    payload,
                ));
            }
            _ => native.push(format!("{} ({}): a native output with no retained source; it can only be deployed to this exact installation, not packaged", sf.relative, sf.origin)),
        }
    }
    let members: Vec<String> = m
        .member_edits
        .iter()
        .map(|e| format!("{} › {} ({}×{} {}{}): archive-member replacements are recorded with their source but this build has no installer adapter for them yet", e.archive_relative, e.member_path, e.width, e.height, e.container, if e.whole_group { ", whole group" } else { "" }))
        .collect();
    Ok(ProjectEdits { career_removals, schedules, free_agents, cells, dictionary, textures, structural, members, native, base })
}

/// Loose-asset layouts registered as shared between two platforms of one title: NHL Legacy
/// Recompiled inherits the Xbox 360 Legacy asset profiles by reference (Q47, package C0-AS5;
/// `defs/adapters/asset-profiles`). No other pair is registered as shared: NHL Legacy on PS3 has no
/// registered loose-asset profile, and the Xbox 360 / PS3 families of the other titles are
/// registered per platform and not verified identical.
pub(crate) fn shared_loose_layout(a: &str, b: &str) -> bool {
    matches!((a, b), ("xbox", "recomp") | ("recomp", "xbox"))
}

/// Measured compatibility of each category on each platform of the game.
fn target_support(store: &ProjectStore, edits: &ProjectEdits, inventory: Option<&BaselineStore>, roster_only: bool) -> Vec<TargetSupport> {
    let m = &store.manifest;
    let author = m.platform.as_str();
    let author_main = edits.base.iter().find(|b| b.role == ROLE_MAIN_DB).map(|b| b.sha256.clone());
    let mut out = Vec::new();
    for t in studio_domain::SHIPPING_TARGETS.iter().filter(|t| same_title(t.game_id, &m.game_id)) {
        let p = t.platform.as_str();
        let same = p == author;
        let inv = inventory.and_then(|st| st.target(t.game_id, t.platform));
        let mut components = Vec::new();
        // Cells.
        let cells = if edits.cells.is_empty() && edits.free_agents.is_none() && edits.schedules.is_empty() && edits.career_removals.is_empty() {
            ComponentSupport { category: CAT_CELLS.into(), status: "empty".into(), reason: "no database cell edits".into() }
        } else if same {
            ComponentSupport { category: CAT_CELLS.into(), status: "supported".into(), reason: "the author's platform; each row is found by its key values in the recipient's own database and checked for its before value".into() }
        } else if roster_only {
            ComponentSupport { category: CAT_CELLS.into(), status: "unsupported".into(), reason: "roster-save-only edits are not mapped across save containers".into() }
        } else {
            match (inv.and_then(|tb| tb.component(ROLE_MAIN_DB)), &author_main) {
                (Some(c), Some(a)) if &c.sha256 == a => ComponentSupport { category: CAT_CELLS.into(), status: "supported".into(), reason: format!("{}'s bundled baseline database is byte-identical to the author's baseline ({}…), so the tables, key fields and codecs are the same; each row is found by its key values in the recipient's own files and checked for its before value", t.title, &a[..12]) },
                (Some(c), Some(a)) => ComponentSupport { category: CAT_CELLS.into(), status: "unsupported".into(), reason: format!("{}'s baseline database ({}…) differs from the author's ({}…); no verified cross-platform field mapping exists", t.title, &c.sha256[..12], &a[..12]) },
                (None, _) => ComponentSupport { category: CAT_CELLS.into(), status: "unsupported".into(), reason: format!("no bundled baseline database is available for {} to measure against", t.title) },
                (_, None) => ComponentSupport { category: CAT_CELLS.into(), status: "unsupported".into(), reason: "the project has no baseline database identity".into() },
            }
        };
        components.push(cells);
        // Dictionary.
        let dictionary = if edits.dictionary.is_empty() {
            ComponentSupport { category: CAT_DICTIONARY.into(), status: "empty".into(), reason: "no dictionary edits".into() }
        } else if same || inv.is_some_and(|tb| tb.component(ROLE_DICTIONARY).is_some()) {
            ComponentSupport { category: CAT_DICTIONARY.into(), status: "supported".into(), reason: "key-based: each key's current text is checked on the recipient's dictionary before it is re-encoded".into() }
        } else {
            ComponentSupport { category: CAT_DICTIONARY.into(), status: "unsupported".into(), reason: format!("{} has no bundled dictionary to measure against", t.title) }
        };
        components.push(dictionary);
        // Textures.
        let textures = if edits.textures.is_empty() {
            ComponentSupport { category: CAT_TEXTURES.into(), status: "empty".into(), reason: "no loose texture edits".into() }
        } else if same {
            ComponentSupport { category: CAT_TEXTURES.into(), status: "supported".into(), reason: "re-encoded from the authored source into the recipient's own installed file".into() }
        } else if shared_loose_layout(author, p) {
            ComponentSupport { category: CAT_TEXTURES.into(), status: "supported".into(), reason: format!("{} shares the Legacy Xbox 360 loose-asset layout by registered inheritance (Q47); the recipient's own file at the same path is re-encoded from the authored source, and each placement is verified against that file at install", t.title) }
        } else if p == "ps3" && same_title(&m.game_id, "nhllegacy") {
            ComponentSupport { category: CAT_TEXTURES.into(), status: "unsupported".into(), reason: "NHL Legacy on PS3 has no registered loose-asset profile (defs/adapters/asset-profiles): the path families are unverified, so no placement can be checked".into() }
        } else {
            ComponentSupport { category: CAT_TEXTURES.into(), status: "unsupported".into(), reason: format!("loose-asset path families are registered per platform and not verified identical between {} and {}", author, p) }
        };
        components.push(textures);
        let status = if components.iter().any(|c| c.status == "unsupported") { "unsupported" } else { "supported" };
        out.push(TargetSupport { platform: p.into(), status: status.into(), components });
    }
    out
}

fn category_dto(category: &str, count: usize, items: Vec<String>, blockers: Vec<String>) -> PackageCategoryDto {
    let status = if !blockers.is_empty() { "blocked" } else if count == 0 { "empty" } else { "ready" };
    PackageCategoryDto { category: category.into(), count: count as u64, status: status.into(), items, blockers }
}

fn target_dto(t: &TargetSupport) -> PackageTargetDto {
    PackageTargetDto {
        platform: t.platform.clone(),
        status: t.status.clone(),
        components: t.components.iter().map(|c| PackageComponentDto { category: c.category.clone(), status: c.status.clone(), reason: c.reason.clone() }).collect(),
    }
}

/// NHL Legacy and NHL Legacy Recompiled are one title on two runtimes (the Recompiled baseline is
/// derived from the Xbox 360 Legacy files, handoff R128): a package of either names the other as a
/// platform of the same game. Every other pair is the same title only when the ids are equal.
pub(crate) fn same_title(a: &str, b: &str) -> bool {
    a == b || matches!((a, b), ("nhllegacy", "legacyrecomp") | ("legacyrecomp", "nhllegacy"))
}

fn safe_file_stem(name: &str) -> String {
    let mut out: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ' ') { c } else { '_' }).collect();
    out = out.trim().trim_end_matches('.').to_string();
    if out.is_empty() {
        "mod".into()
    } else {
        out
    }
}

/// The folders a destination's loose assets are addressed from, platform-correct and independent
/// of where its database sits (R71-R5): PS4 → the matched game folder (the `nhl/…` tree hangs off
/// it); Xbox 360 / PS3 → the resolved install folder; Recompiled → `_compiled/` beside the
/// executable when it exists, else the install folder. Empty when no installation was recognised.
pub(crate) fn asset_roots(report: &InstallReport, platform: Platform) -> Vec<PathBuf> {
    match platform {
        Platform::Ps4 => report.game_dir.iter().map(PathBuf::from).collect(),
        Platform::Recomp => {
            let Some(inst) = report.install_dir.as_deref().map(PathBuf::from) else { return Vec::new() };
            let compiled = inst.join("_compiled");
            let mut v = Vec::new();
            if compiled.is_dir() {
                v.push(compiled);
            }
            v.push(inst);
            v
        }
        _ => report.install_dir.iter().map(PathBuf::from).collect(),
    }
}

/// Which roster save a package that needs one is installed into: the explicit choice when it is
/// one of the saves found now (a stale choice refuses), the only save found otherwise, and a
/// choice is required when several were found — on every platform.
fn chosen_roster(report: &InstallReport, roster_save: Option<&str>) -> Result<Option<String>, String> {
    ps4_chosen_roster(report, roster_save).map(|s| s.map(|s| s.path.clone())).map_err(|e| e.to_string())
}

/// Where a cell edit lands at the recipient (`main_db` | `roster`).
fn cell_targets(file: &str) -> &'static [&'static str] {
    match file {
        "main_db" => &["main_db"],
        "roster" => &["roster"],
        _ => &["main_db", "roster"],
    }
}

/// Resolve the recipient's row for a cell edit: by key values (exactly one match) or, for a
/// keyless table, by position with the author's row fingerprint. `Err` is the plain-language
/// refusal.
fn locate_row(session: &EditSession, db: &tdb_core::TdbFile<'_>, c: &CellOp, where_: &str) -> Result<Result<u32, String>, ApiError> {
    let Some(table) = db.table_canonical(&c.studio_table) else {
        return Ok(Err(format!("table {} does not exist in the destination's {where_}", c.table_name)));
    };
    if c.keys.is_empty() {
        let Some(expected) = &c.row_sha256 else {
            return Ok(Err(format!("{} has no key field and the package carries no row fingerprint; the row cannot be identified safely", c.table_name)));
        };
        if c.record >= table.current_records as u32 {
            return Ok(Err(format!("row {} of {} does not exist in the destination's {where_} ({} rows)", c.record, c.table_name, table.current_records)));
        }
        return Ok(match row_fingerprint(session.bytes(), table, c.record) {
            Some(h) if &h == expected => Ok(c.record),
            _ => Err(format!("{} has no key field and row {} of the destination's {where_} is not the row the author edited; the row cannot be identified safely", c.table_name, c.record)),
        });
    }
    let mut fields = Vec::with_capacity(c.keys.len());
    for k in &c.keys {
        let Some(f) = table.fields.iter().find(|f| f.id.canonical(db.endian) == k.studio_field) else {
            return Ok(Err(format!("key field {} of {} does not exist in the destination's {where_}", k.field_name, c.table_name)));
        };
        fields.push(f);
    }
    let mut matches = Vec::new();
    for rec in 0..table.current_records as u32 {
        let all = c.keys.iter().zip(&fields).all(|(k, f)| {
            // Identity compares stored values, not labels or decoded rating/appearance values.
            // Avoid constructing editor views (and their vocabularies) for every scanned row.
            match &k.value {
                CellValue::Int { raw, .. } => db.read_u64(table, rec, f) == Some(*raw),
                CellValue::Text { text } => db.read_inline_str(table, rec, f)
                    .is_some_and(|bytes| String::from_utf8_lossy(bytes) == text.as_str()),
            }
        });
        if all {
            matches.push(rec);
        }
    }
    // Some databases carry the same key on more than one row (imported or third-party-edited
    // rosters). Then the row is the one whose bytes are exactly the author's original row.
    if matches.len() > 1 {
        if let Some(expected) = &c.row_sha256 {
            let same: Vec<u32> = matches.iter().copied().filter(|&r| row_fingerprint(db.bytes(), table, r).as_ref() == Some(expected)).collect();
            if same.len() == 1 || (same.len() > 1 && same.contains(&c.record)) {
                return Ok(Ok(if same.len() == 1 { same[0] } else { c.record }));
            }
        }
    }
    let keys_text = c.keys.iter().map(|k| format!("{} = {}", k.field_name, value_display(&k.value))).collect::<Vec<_>>().join(", ");
    Ok(match matches.as_slice() {
        [one] => Ok(*one),
        [] => Err(format!("no row of {} in the destination's {where_} has {keys_text}; the {} the author edited is not there", c.table_name, c.entity)),
        _ => Err(format!("more than one row of {} in the destination's {where_} has {keys_text}; the row is ambiguous and is not edited", c.table_name)),
    })
}

/// Write surviving career rows as one validated batch (the recipient's table was
/// checked to hold exactly the author's original rows, so the row index is the author's), then
/// require the row to be exactly the author's bytes.
fn rewrite_career_rows(
    ws: &mut Workspace,
    studio_table: &str,
    rewrites: &[pkg::CareerRowRewrite],
) -> Result<(), ApiError> {
    let mut cells = Vec::new();
    let mut expected = Vec::new();
    let bytes = ws.primary().bytes().to_vec();
    let db = tdb_core::TdbFile::parse(&bytes).map_err(|e| ApiError::Workspace(e.to_string()))?;
    for rewrite in rewrites {
        let bad = || {
            ApiError::Workspace(format!(
                "the career edit for record {} of {studio_table} does not fit this database",
                rewrite.row + 1
            ))
        };
        let row = pkg::from_hex(&rewrite.bytes).ok_or_else(bad)?;
        let current_row = |ws: &Workspace| -> Result<
            (Vec<u8>, Vec<(String, Option<i64>, Option<String>)>),
            ApiError,
        > {
            let table = db.table_canonical(studio_table).ok_or_else(bad)?;
            let len = table.record_len_bytes as usize;
            let start = table.record_block + rewrite.row as usize * len;
            if row.len() != len || rewrite.row >= u32::from(table.current_records) {
                return Err(bad());
            }
            let current = bytes.get(start..start + len).ok_or_else(bad)?.to_vec();
            let mut writes = Vec::new();
            for f in &table.fields {
                let field = f.id.canonical(db.endian);
                if f.bit_width > 64 {
                    let (new, old) = (
                        row_bits(&row, f).ok_or_else(bad)?,
                        row_bits(&current, f).ok_or_else(bad)?,
                    );
                    if new != old {
                        let text = new.split(|b| *b == 0).next().unwrap_or_default();
                        writes.push((
                            field,
                            None,
                            Some(String::from_utf8_lossy(text).into_owned()),
                        ));
                    }
                    continue;
                }
                let new = tdb_core::read_bits(&row, f.bit_offset, f.bit_width, db.endian);
                if tdb_core::read_bits(&current, f.bit_offset, f.bit_width, db.endian) != new {
                    // Row payloads contain stored bits; the editor accepts decoded
                    // values (NHL07 career plus/minus is a signed, biased field).
                    let decoded = ws
                        .primary()
                        .project()
                        .meta()
                        .field(&table.id.raw(), &f.id.raw())
                        .map(|m| m.decode_raw(new))
                        .unwrap_or(new as i64);
                    writes.push((field, Some(decoded), None));
                }
            }
            Ok((current, writes))
        };
        let (_, writes) = current_row(ws)?;
        for (field, int, text) in writes {
            let value = match (int, text) {
                (Some(v), _) => FieldValue::Int(v),
                (None, Some(t)) => FieldValue::Text(t),
                _ => continue,
            };
            cells.push((studio_table.to_string(), rewrite.row, field, value));
        }
        let table = db.table_canonical(studio_table).ok_or_else(bad)?;
        expected.push((
            table.record_block + rewrite.row as usize * table.record_len_bytes as usize,
            row,
        ));
    }
    // One audit for the complete career replay, rather than one full-database
    // transaction per statistic across thousands of surviving rows.
    if !cells.is_empty() {
        ws.set_package_fields(&cells)?;
    }
    for (start, row) in expected {
        if ws.primary().bytes().get(start..start + row.len()) != Some(row.as_slice()) {
            return Err(ApiError::Workspace(format!(
                "Career replay of {studio_table} differs from the package"
            )));
        }
    }
    Ok(())
}

/// The raw bytes of a byte-aligned wide (text) field of one record.
fn row_bits(row: &[u8], f: &tdb_core::FieldDescriptor) -> Option<Vec<u8>> {
    let (start, end) = ((f.bit_offset / 8) as usize, ((f.bit_offset + f.bit_width) / 8) as usize);
    (f.bit_offset % 8 == 0 && f.bit_width % 8 == 0).then(|| row.get(start..end).map(<[u8]>::to_vec)).flatten()
}

/// One staged recipient file the cells replay into.
struct StagedTarget {
    ws: Workspace,
    destination: PathBuf,
    where_: &'static str,
}

impl Registry {
    // ------------------------------------------------------------------ builder

    /// What the open project could package right now: per category the items and blockers, per
    /// platform of the game the measured support, and the author's base identities. Read-only.
    pub fn package_inventory(&self, id: u32, inventory: Option<&BaselineStore>, defs: &str) -> Result<PackageInventoryDto, ApiError> {
        let dirty = if self.has_session(id) { self.with_workspace(id, |ws| Ok(ws.is_dirty()))? } else { false };
        let projects = self.projects.lock().unwrap();
        let b = projects.bindings.get(&id).ok_or_else(|| ApiError::Workspace(format!("project {id} is not a project-folder session")))?;
        let store = &b.store;
        let edits = project_edits(store, defs)?;
        let roster_only = store.manifest.working.main_db.is_none();
        let targets = target_support(store, &edits, inventory, roster_only);
        let mut blockers = Vec::new();
        if dirty {
            blockers.push("the project has unsaved edits; save the project first (a package carries saved edits only)".into());
        }
        let categories = vec![
            category_dto(CAT_CELLS, edits.cells.len() + usize::from(edits.free_agents.is_some()) + edits.schedules.len() + edits.career_removals.len(), edits.cells.iter().take(200).map(|c| format!("{} › {} › {}: {} → {}{}", c.table_name, c.entity, c.field_name, value_display(&c.before), value_display(&c.after), if c.file == "both" { " (database + roster)" } else if c.file == "roster" { " (roster save)" } else { "" })).chain(edits.schedules.iter().map(|s|format!("Schedule {}: {} → {} games",s.table,s.before.len(),s.after.len()))).collect(), Vec::new()),
            category_dto(CAT_DICTIONARY, edits.dictionary.len(), edits.dictionary.iter().take(200).map(|d| format!("{}: {:?} → {:?}", d.key, d.before, d.after)).collect(), Vec::new()),
            category_dto(CAT_TEXTURES, edits.textures.len(), edits.textures.iter().map(|(t, _, _)| format!("{} ({}×{}{})", t.rel_path, t.width, t.height, t.texture_index.map(|i| format!(", texture {i}")).unwrap_or_default())).collect(), Vec::new()),
            category_dto(CAT_MEMBERS, edits.members.len(), Vec::new(), edits.members.clone()),
            category_dto(CAT_NATIVE, edits.native.len(), Vec::new(), edits.native.clone()),
            category_dto(CAT_STRUCTURAL, edits.structural.len(), Vec::new(), edits.structural.clone()),
        ];
        let title = store.manifest.title.clone();
        let default_name = safe_file_stem(&format!("{} mod", title.split(" (").next().unwrap_or(&title)));
        Ok(PackageInventoryDto {
            dirty,
            saves: store.manifest.saves.len() as u64,
            categories,
            targets: targets.iter().map(target_dto).collect(),
            blockers,
            default_name,
            default_version: format!("1.0.{}", store.manifest.saves.len()),
            exports_dir: s(&store.exports_dir()),
            base: edits.base.iter().map(|b| crate::dto::PackageBaseDto { role: b.role.clone(), file_name: b.file_name.clone(), len: b.len, sha256: b.sha256.clone(), provenance: b.provenance.clone() }).collect(),
        })
    }

    /// Build one `.nhlmod` file from the open, saved project. Every selected category must be
    /// packageable and every selected platform supported; otherwise the build is refused with the
    /// items named. The file is written beside a temporary name and renamed into place; an existing
    /// file is never overwritten.
    pub fn package_build(&self, id: u32, req: &PackageBuildRequestDto, inventory: Option<&BaselineStore>, defs: &str) -> Result<PackageBuildDto, ApiError> {
        let dirty = if self.has_session(id) { self.with_workspace(id, |ws| Ok(ws.is_dirty()))? } else { false };
        if dirty {
            return Err(ApiError::Workspace("the project has unsaved edits; save the project before building a package".into()));
        }
        let projects = self.projects.lock().unwrap();
        let b = projects.bindings.get(&id).ok_or(ApiError::UnknownProject(id))?;
        Self::build_saved_package(&b.store, req, inventory, defs)
    }

    pub(super) fn build_saved_package(store: &ProjectStore, req: &PackageBuildRequestDto, inventory: Option<&BaselineStore>, defs: &str) -> Result<PackageBuildDto, ApiError> {
        let name = req.name.trim();
        let version = req.version.trim();
        if name.is_empty() || version.is_empty() {
            return Err(ApiError::Workspace("the package needs a name and a version".into()));
        }
        if name.len() > 120 || version.len() > 40 || req.description.len() > 4000 {
            return Err(ApiError::Workspace("the package name, version or description is too long".into()));
        }
        let m = &store.manifest;
        let edits = project_edits(store, defs)?;
        let roster_only = m.working.main_db.is_none();
        let support = target_support(store, &edits, inventory, roster_only);
        let include: Vec<&str> = req.include.iter().map(String::as_str).collect();
        let mut refusals = Vec::new();
        for cat in &include {
            match *cat {
                CAT_CELLS | CAT_DICTIONARY | CAT_TEXTURES => {}
                CAT_MEMBERS => refusals.extend(edits.members.iter().cloned()),
                CAT_NATIVE => refusals.extend(edits.native.iter().cloned()),
                CAT_STRUCTURAL => refusals.extend(edits.structural.iter().cloned()),
                other => refusals.push(format!("unknown category {other:?}")),
            }
        }
        // Unselected but present categories are a disclosure, not a silent omission: the build
        // refuses unless the author acknowledged them by leaving them out explicitly (`exclude`).
        for (cat, items) in [(CAT_MEMBERS, &edits.members), (CAT_NATIVE, &edits.native), (CAT_STRUCTURAL, &edits.structural)] {
            if !items.is_empty() && !include.contains(&cat) && !req.exclude.iter().any(|e| e == cat) {
                refusals.push(format!("the project has {} {} item(s) that cannot be packaged; exclude the category explicitly to build without them: {}", items.len(), cat, items.join(" | ")));
            }
        }
        for (cat, count) in [(CAT_CELLS, (edits.cells.len() + usize::from(edits.free_agents.is_some()) + edits.schedules.len() + edits.career_removals.len())), (CAT_DICTIONARY, edits.dictionary.len()), (CAT_TEXTURES, edits.textures.len())] {
            if count > 0 && !include.contains(&cat) && !req.exclude.iter().any(|e| e == cat) {
                refusals.push(format!("{count} {cat} edit(s) are neither included nor explicitly excluded"));
            }
        }
        let cells: Vec<CellOp> = if include.contains(&CAT_CELLS) { edits.cells.clone() } else { Vec::new() };
        let career_removals = if include.contains(&CAT_CELLS) { edits.career_removals.clone() } else { Vec::new() };
        let schedules = if include.contains(&CAT_CELLS) { edits.schedules.clone() } else { Vec::new() };
        let free_agents = if include.contains(&CAT_CELLS) { edits.free_agents.clone() } else { None };
        let dictionary: Vec<DictionaryOp> = if include.contains(&CAT_DICTIONARY) { edits.dictionary.clone() } else { Vec::new() };
        let textures: Vec<&(TextureOp, Vec<u8>, String)> = if include.contains(&CAT_TEXTURES) { edits.textures.iter().collect() } else { Vec::new() };
        if cells.is_empty() && free_agents.is_none() && schedules.is_empty() && career_removals.is_empty() && dictionary.is_empty() && textures.is_empty() {
            refusals.push("nothing selected: the package would carry no edits".into());
        }
        // Platforms: every requested platform must be supported for every included non-empty category.
        let mut platforms: Vec<String> = req.platforms.iter().map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
        if platforms.is_empty() {
            platforms.push(m.platform.clone());
        }
        let mut targets = Vec::new();
        for p in &platforms {
            let Some(t) = support.iter().find(|t| &t.platform == p) else {
                refusals.push(format!("{p} is not a platform of {}", m.title));
                continue;
            };
            let mut comps = Vec::new();
            for c in &t.components {
                let included = match c.category.as_str() {
                    CAT_CELLS => !cells.is_empty() || free_agents.is_some() || !schedules.is_empty() || !career_removals.is_empty(),
                    CAT_DICTIONARY => !dictionary.is_empty(),
                    CAT_TEXTURES => !textures.is_empty(),
                    _ => false,
                };
                if included && c.status == "unsupported" {
                    refusals.push(format!("{p}: {} — {}", c.category, c.reason));
                }
                comps.push(ComponentSupport { category: c.category.clone(), status: if included { c.status.clone() } else { "empty".into() }, reason: c.reason.clone() });
            }
            let status = if comps.iter().any(|c| c.status == "unsupported") { "unsupported" } else { "supported" };
            targets.push(TargetSupport { platform: p.clone(), status: status.into(), components: comps });
        }
        if !refusals.is_empty() {
            return Err(ApiError::Workspace(format!("the package was not built: {}", refusals.join("; "))));
        }
        let out_path = match req.out_path.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
            Some(p) => PathBuf::from(p),
            None => store.exports_dir().join(format!("{}-{}.{}", safe_file_stem(name), safe_file_stem(version), pkg::PACKAGE_EXTENSION)),
        };
        if out_path.exists() {
            return Err(ApiError::Workspace(format!("{} already exists; the builder never overwrites a package — choose another build name or output path", s(&out_path))));
        }
        store.refuse_guarded_write("the package build", &out_path).map_err(ApiError::Domain)?;
        let manifest = PackageManifest {
            schema: pkg::PACKAGE_SCHEMA.into(),
            package_id: String::new(),
            name: name.into(),
            author: m.details.author.clone(),
            version: version.into(),
            description: req.description.trim().into(),
            created: studio_domain::project_store::now_iso(),
            studio: StudioProvenance { version: env!("CARGO_PKG_VERSION").into(), inventory_head: m.source.inventory_head.clone() },
            game_id: m.game_id.clone(),
            title: m.title.split(" (").next().unwrap_or(&m.title).to_string(),
            author_platform: m.platform.clone(),
            targets: targets.clone(),
            base: edits.base.clone(),
            categories: vec![
                CategoryCount { category: CAT_CELLS.into(), count: (cells.len() + usize::from(free_agents.is_some()) + schedules.len() + career_removals.len()) as u64 },
                CategoryCount { category: CAT_DICTIONARY.into(), count: dictionary.len() as u64 },
                CategoryCount { category: CAT_TEXTURES.into(), count: textures.len() as u64 },
            ],
            entries: Vec::new(),
            project: ProjectProvenance { project_id: m.project_id.clone(), source_kind: m.source.kind.clone(), save_seq: m.saves.last().map(|x| x.seq).unwrap_or(0) },
        };
        let edits_doc = PackageEdits { schema: pkg::EDITS_SCHEMA.into(), career_removals, schedules, free_agents, cells, dictionary, textures: textures.iter().map(|(t, _, _)| t.clone()).collect() };
        let mut payloads: Vec<(String, Vec<u8>)> = Vec::new();
        for (_, bytes, payload) in &textures {
            if !payloads.iter().any(|(n, _)| n == payload) {
                payloads.push((payload.clone(), bytes.clone()));
            }
        }
        let bytes = pkg::build_package(manifest, &edits_doc, &payloads).map_err(ApiError::Domain)?;
        // Read back through the validator before anything is placed.
        let inspected = pkg::inspect_package_bytes(bytes.clone(), &pkg::Limits::default()).map_err(|e| ApiError::Workspace(format!("the built package did not validate: {e}")))?;
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| ApiError::Io { path: parent.display().to_string(), source })?;
        }
        super::project::check_free_space(out_path.parent().unwrap_or(&out_path), bytes.len() as u64, "writing the package")?;
        let tmp = out_path.with_extension(format!("{}.nhlms-tmp-{}", pkg::PACKAGE_EXTENSION, std::process::id()));
        std::fs::write(&tmp, &bytes).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            ApiError::Workspace(format!("could not write {}: {e}", s(&tmp)))
        })?;
        if out_path.exists() {
            let _ = std::fs::remove_file(&tmp);
            return Err(ApiError::Workspace(format!("{} appeared while the package was being written; nothing replaced", s(&out_path))));
        }
        std::fs::rename(&tmp, &out_path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            ApiError::Workspace(format!("could not place {}: {e}", s(&out_path)))
        })?;
        Ok(PackageBuildDto {
            path: s(&out_path),
            package_id: inspected.manifest.package_id.clone(),
            len: bytes.len() as u64,
            sha256: inspected.file_sha256.clone(),
            entries: inspected.manifest.entries.len() as u64,
            categories: inspected.manifest.categories.iter().map(|c| crate::dto::PackageCategoryCountDto { category: c.category.clone(), count: c.count }).collect(),
            targets: targets.iter().map(target_dto).collect(),
        })
    }

    // ------------------------------------------------------------------ installer

    /// Read and validate a package file; nothing is written. The summary names the mod, its
    /// declared targets and support, every category and every entry.
    pub fn package_inspect(&self, path: &str) -> Result<PackageInspectDto, ApiError> {
        let p = Path::new(path);
        let ins = pkg::inspect_package(p, &pkg::Limits::default()).map_err(ApiError::Domain)?;
        Ok(inspect_dto(path, &ins))
    }

    /// Plan an installation (read-only): validate the package, check the chosen destination and
    /// stage every conversion. Conflicts block unless `allow_conflicts`; an unsupported category on
    /// the chosen platform blocks the whole plan. The plan is held server-side under a handle and
    /// bound to the destinations' current identities. Schema metadata for a roster-only package
    /// comes from the registry's baseline store (the shipped one in the app).
    #[allow(clippy::too_many_arguments)]
    pub fn package_preflight(
        &self,
        package_path: &str,
        game_id: &str,
        platform: Platform,
        picked_root: &str,
        save_path: Option<&str>,
        roster_save: Option<&str>,
        allow_conflicts: bool,
        records_dir: &str,
        defs: &str,
    ) -> Result<InstallPlanDto, ApiError> {
        let store = self.baselines();
        self.package_preflight_with(package_path, game_id, platform, picked_root, save_path, roster_save, allow_conflicts, records_dir, defs, store.as_deref())
    }

    /// [`Registry::package_preflight`] with an explicit baseline store (`None`: no bundled schema is
    /// available, so a roster-only package without an installation refuses with that reason).
    #[allow(clippy::too_many_arguments)]
    pub fn package_preflight_with(
        &self,
        package_path: &str,
        game_id: &str,
        platform: Platform,
        picked_root: &str,
        save_path: Option<&str>,
        roster_save: Option<&str>,
        allow_conflicts: bool,
        records_dir: &str,
        defs: &str,
        store: Option<&BaselineStore>,
    ) -> Result<InstallPlanDto, ApiError> {
        let ins = pkg::inspect_package(Path::new(package_path), &pkg::Limits::default()).map_err(ApiError::Domain)?;
        let m = &ins.manifest;
        let edits = &ins.edits;
        if edits.schema == pkg::EDITS_SCHEMA_V1 && !(edits.cells.is_empty() && edits.free_agents.is_none() && edits.schedules.is_empty() && edits.career_removals.is_empty()) {
            return Err(ApiError::Workspace(format!(
                "this package ({} {}) was built by an earlier Studio that bound its {} database edit(s) to row positions instead of row identities; it cannot be installed safely. Rebuild it from its project with this Studio",
                m.name,
                m.version,
                (edits.cells.len() + usize::from(edits.free_agents.is_some()) + edits.schedules.len() + edits.career_removals.len())
            )));
        }
        let mut blocked: Vec<String> = Vec::new();
        let mut conflicts: Vec<String> = Vec::new();
        let mut requirements: Vec<InstallRequirementDto> = Vec::new();
        let target = studio_domain::shipping_target(game_id, platform).ok_or_else(|| ApiError::Workspace(format!("{game_id} on {} is not a shipping target", platform.as_str())))?;
        if !same_title(&m.game_id, game_id) {
            return Err(ApiError::Workspace(format!("this package is for {} ({}), not {}; cross-game installation is not offered", m.title, m.game_id, target.title)));
        }
        let declared = m.targets.iter().find(|t| t.platform == platform.as_str());
        match declared {
            None => return Err(ApiError::Workspace(format!("this package declares no {} target ({}); it was authored on {} and declares: {}", platform.as_str(), target.title, m.author_platform, m.targets.iter().map(|t| format!("{} ({})", t.platform, t.status)).collect::<Vec<_>>().join(", ")))),
            Some(t) if t.status != "supported" => {
                let why: Vec<String> = t.components.iter().filter(|c| c.status == "unsupported").map(|c| format!("{}: {}", c.category, c.reason)).collect();
                return Err(ApiError::Workspace(format!("this package declares {} as unsupported: {}", platform.as_str(), why.join("; "))));
            }
            Some(_) => {}
        }
        let records_dir = PathBuf::from(records_dir);
        std::fs::create_dir_all(&records_dir).map_err(|source| ApiError::Io { path: records_dir.display().to_string(), source })?;
        // Dependencies come from every selected operation's declared file role — nothing else —
        // and they are known before any path is validated (R72-R1).
        let needs_main = !edits.career_removals.is_empty() || !edits.schedules.is_empty() || edits.free_agents.is_some() || edits.cells.iter().any(|c| c.file != "roster");
        let needs_roster = edits.cells.iter().any(|c| c.file != "main_db");
        let needs_installation = needs_main || !edits.dictionary.is_empty() || !edits.textures.is_empty();
        let root_given = !picked_root.trim().is_empty();
        if !root_given {
            if needs_installation {
                return Err(ApiError::Workspace("choose the destination game installation first".into()));
            }
            // A roster-only package: the save location stands alone. PS3 keeps its saves inside the
            // RPCS3 folder, so that folder is the location to choose there.
            match platform {
                Platform::Ps3 => return Err(ApiError::Workspace("this package edits the roster save only; choose the RPCS3 folder that holds the save (its saves live inside it) — no game installation is needed".into())),
                _ => {
                    if save_path.map(str::trim).filter(|s| !s.is_empty()).is_none() && roster_save.map(str::trim).filter(|s| !s.is_empty()).is_none() {
                        return Err(ApiError::Workspace("this package edits the roster save only; choose the save-data location (or the roster save itself) — no game installation is needed".into()));
                    }
                }
            }
        }
        // A roster save named without a save location: its folder is the location.
        let save_path_owned: Option<String> = match (save_path.map(str::trim).filter(|s| !s.is_empty()), roster_save.map(str::trim).filter(|s| !s.is_empty())) {
            (Some(sp), _) => Some(sp.to_string()),
            (None, Some(rs)) if !root_given => Path::new(rs).parent().and_then(|p| if platform == Platform::Ps4 { p.parent().and_then(|t| t.parent()).and_then(|pr| pr.parent()) } else { p.parent() }).map(|p| p.to_string_lossy().into_owned()),
            _ => None,
        };
        let report = studio_domain::detect_install(game_id, platform, Path::new(picked_root), save_path_owned.as_deref().map(Path::new));
        let main_db = if root_given { report.db_paths.nhlng.clone() } else { None };
        // Schema metadata: the installation's own when the package edits the database; a roster-only
        // package takes the title's verified bundled metadata (never a file the recipient must copy).
        let mut schema: Option<(Vec<u8>, String)> = None;
        if needs_main {
            if let Some(m) = report.db_paths.meta_xml.clone().filter(|_| root_given) {
                match std::fs::read(&m) {
                    Ok(bytes) => schema = Some((bytes, m)),
                    Err(e) => blocked.push(format!("the schema metadata {m} cannot be read: {e}")),
                }
            }
        } else if needs_roster {
            match store {
                None => blocked.push(format!("no bundled schema metadata is available in this installation of Studio (the baseline inventory is missing), so the roster save of {} cannot be opened; reinstall Studio or choose a game installation that carries {}", target.title, target.meta)),
                Some(st) => match st.target(game_id, platform).and_then(|tb| tb.component(ROLE_META)) {
                    None => blocked.push(format!("{} has no bundled schema metadata for {}; the roster save cannot be opened without a game installation that carries {}", target.title, platform.as_str(), target.meta)),
                    Some(c) => match st.read_verified(c) {
                        Ok(bytes) => schema = Some((bytes, format!("bundled {} ({}, {}…)", c.file_name, c.provenance, &c.sha256[..12]))),
                        Err(e) => blocked.push(format!("the bundled schema metadata {} does not verify: {e}; nothing is applied", st.component_path(c).display())),
                    },
                },
            }
        }
        let meta: Option<String> = schema.as_ref().map(|(_, src)| src.clone());
        let roster: Option<String> = if needs_roster {
            match chosen_roster(&report, roster_save) {
                Ok(r) => r,
                Err(why) => {
                    blocked.push(format!("roster save: {why}"));
                    None
                }
            }
        } else {
            None
        };
        if !(edits.cells.is_empty() && edits.free_agents.is_none() && edits.schedules.is_empty() && edits.career_removals.is_empty()) {
            requirements.push(InstallRequirementDto {
                role: "main_db".into(),
                status: if !needs_main { "not-needed" } else if main_db.is_some() && meta.is_some() { "found" } else { "missing" }.into(),
                detail: if !needs_main { "the package has no main-database edit; the database is not touched".into() } else { main_db.clone().unwrap_or_else(|| format!("no {} (with its {}) found under {picked_root}", target.main_db, target.meta)) },
            });
            requirements.push(InstallRequirementDto {
                role: "schema".into(),
                status: if meta.is_some() { "found" } else { "missing" }.into(),
                detail: meta.clone().unwrap_or_else(|| if needs_main { format!("no {} beside the database", target.meta) } else { format!("no bundled {} for {}", target.meta, target.title) }),
            });
            requirements.push(InstallRequirementDto {
                role: "roster_save".into(),
                status: if !needs_roster { "not-needed" } else if roster.is_some() { "found" } else { "missing" }.into(),
                detail: if !needs_roster { "the package has no roster-save edit; no save is touched".into() } else { roster.clone().unwrap_or_else(|| "no writable roster save was chosen or found in the save location".into()) },
            });
            if needs_main && (main_db.is_none() || meta.is_none()) {
                blocked.push(format!("the package edits the main database but no {} (with its {}) was found under {picked_root}", target.main_db, target.meta));
            }
            if needs_roster && !needs_main && meta.is_none() && !blocked.iter().any(|b| b.contains("schema metadata")) {
                blocked.push(format!("the package edits the roster save but no schema metadata ({}) is available to open it", target.meta));
            }
            if needs_roster && roster.is_none() && !blocked.iter().any(|b| b.starts_with("roster save:")) {
                blocked.push("the package edits the roster save, and no writable roster save was found in the save location; every roster edit (including each edit that targets both files) needs it".into());
            }
        }
        if !edits.dictionary.is_empty() {
            requirements.push(InstallRequirementDto { role: "dictionary".into(), status: if report.db_paths.localization.is_some() { "found" } else { "missing" }.into(), detail: report.db_paths.localization.clone().unwrap_or_else(|| format!("no {} found", target.dictionary)) });
            if report.db_paths.localization.is_none() {
                blocked.push(format!("the package edits the dictionary but no {} was found under {picked_root}", target.dictionary));
            }
        }
        let roots = asset_roots(&report, platform);
        if !edits.textures.is_empty() {
            requirements.push(InstallRequirementDto {
                role: "assets".into(),
                status: if roots.is_empty() { "missing" } else { "found" }.into(),
                detail: if roots.is_empty() { format!("no {} installation was recognised under {picked_root}; choose the game folder itself", target.title) } else { roots.iter().map(|r| s(r)).collect::<Vec<_>>().join(" | ") },
            });
            if roots.is_empty() {
                blocked.push(format!("the package replaces loose textures but no {} installation was recognised under {picked_root} (wrong folder?); its asset tree cannot be located", target.title));
            }
        }
        let identity = PackageIdentity { package_id: m.package_id.clone(), name: m.name.clone(), version: m.version.clone(), file_sha256: ins.file_sha256.clone() };
        let mut items: Vec<PlannedInstall> = Vec::new();
        let mut applied = 0usize;
        let mut already = 0usize;
        // Staging: the recipient's current files, copied into an installer-owned folder and edited
        // there through the same session the Studio uses; nothing live is touched.
        let staging = records_dir.join(format!("staging-{}-{}", std::process::id(), {
            let mut st = self.packages.lock().unwrap();
            st.next += 1;
            st.next
        }));
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging).map_err(|source| ApiError::Io { path: staging.display().to_string(), source })?;
        let result = (|| -> Result<(), ApiError> {
            if !(edits.cells.is_empty() && edits.free_agents.is_none() && edits.schedules.is_empty() && edits.career_removals.is_empty()) && blocked.is_empty() {
                let (meta_bytes, meta_src) = schema.clone().ok_or_else(|| ApiError::Workspace(format!("no {} found", target.meta)))?;
                let st_meta = staging.join(target.meta);
                std::fs::write(&st_meta, &meta_bytes).map_err(|e| ApiError::Workspace(format!("could not stage the schema metadata ({meta_src}): {e}")))?;
                let meta_s = st_meta.to_string_lossy().into_owned();
                // One independent session per file role; the installer mirrors nothing.
                let mut targets: HashMap<&'static str, StagedTarget> = HashMap::new();
                if needs_main {
                    let db = main_db.clone().expect("checked: needs_main blocked when absent");
                    let st = staging.join(target.main_db);
                    std::fs::copy(&db, &st).map_err(|e| ApiError::Workspace(format!("could not stage {db}: {e}")))?;
                    let mut ws = Workspace::single(build_session(&st.to_string_lossy(), platform, ProjectRole::Nhlng, &meta_s, defs)?, st.to_string_lossy().into_owned());
                    ws.set_game_id(game_id.to_string());
                    targets.insert("main_db", StagedTarget { ws, destination: PathBuf::from(&db), where_: "database" });
                }
                if needs_roster {
                    let r = roster.clone().expect("checked: needs_roster blocked when absent");
                    let rp = Path::new(&r);
                    let st = staging.join("roster").join(rp.file_name().unwrap_or_default());
                    std::fs::create_dir_all(st.parent().unwrap()).map_err(|source| ApiError::Io { path: st.display().to_string(), source })?;
                    std::fs::copy(rp, &st).map_err(|e| ApiError::Workspace(format!("could not stage {r}: {e}")))?;
                    let session = build_session(&st.to_string_lossy(), platform, ProjectRole::Roster, &meta_s, defs).map_err(|e| ApiError::Workspace(format!("the chosen roster save {r} does not open as a {} roster save with the {} schema ({}): {e}; nothing is applied", target.title, platform.as_str(), meta_src)))?;
                    let mut ws = Workspace::single(session, st.to_string_lossy().into_owned());
                    ws.set_game_id(game_id.to_string());
                    targets.insert("roster", StagedTarget { ws, destination: rp.to_path_buf(), where_: "roster save" });
                }
                for op in &edits.career_removals {
                    let target = targets.get("main_db").ok_or_else(|| ApiError::Workspace("Career removal requires a game database".into()))?;
                    let db = tdb_core::TdbFile::parse(target.ws.primary().bytes()).map_err(|e| ApiError::Workspace(e.to_string()))?;
                    let table = db.table_canonical(&op.table).ok_or_else(|| ApiError::Workspace("Career table is missing".into()))?;
                    let hashes = (0..u32::from(table.current_records)).map(|r| row_fingerprint(target.ws.primary().bytes(), table, r)).collect::<Option<Vec<_>>>();
                    if hashes.as_ref() != Some(&op.before_rows) { blocked.push(format!("Career history {} differs from this mod's original database. Restore the original installation first.", op.table)); }
                }
                let mut replay: HashMap<&str, Vec<(String, u32, String, FieldValue)>> = HashMap::new();
                // Resolve against immutable recipient snapshots. Reuse parsing and row searches
                // across a player's many edited fields, but never across files or identities.
                let databases = targets.iter().map(|(&file, t)| {
                    tdb_core::TdbFile::parse(t.ws.primary().bytes())
                        .map(|db| (file, db)).map_err(|e| ApiError::Workspace(e.to_string()))
                }).collect::<Result<HashMap<_, _>, _>>()?;
                let mut rows = HashMap::new();
                for c in &edits.cells {
                    let label = format!("{} › {} › {}", c.table_name, c.entity, c.field_name);
                    // Resolve and check every file the edit names before touching any of them.
                    let mut resolved: Vec<(&'static str, u32)> = Vec::new();
                    let mut op_blocked = false;
                    let mut op_conflict = false;
                    let mut op_already = 0usize;
                    for &file in cell_targets(&c.file) {
                        let Some(t) = targets.get(file) else {
                            blocked.push(format!("{label}: edits the {} and this plan has no {} to apply it to", if file == "roster" { "roster save" } else { "main database" }, if file == "roster" { "roster save" } else { "database" }));
                            op_blocked = true;
                            continue;
                        };
                        let db = &databases[file];
                        let identity = serde_json::to_string(&(&c.studio_table, &c.keys, c.record, &c.row_sha256))
                            .map_err(|e| ApiError::Workspace(e.to_string()))?;
                        let key = (file, identity);
                        if !rows.contains_key(&key) {
                            rows.insert(key.clone(), locate_row(t.ws.primary(), db, c, t.where_)?);
                        }
                        let rec = match rows[&key].clone() {
                            Ok(rec) => rec,
                            Err(why) => {
                                blocked.push(format!("{label}: {why}"));
                                op_blocked = true;
                                continue;
                            }
                        };
                        let Some(cur) = read_cell(t.ws.primary(), db, &c.studio_table, rec, &c.studio_field)? else {
                            blocked.push(format!("{label}: field {} is not in the destination's {}", c.field_name, t.where_));
                            op_blocked = true;
                            continue;
                        };
                        if values_equal(&cur, &c.after) {
                            op_already += 1;
                            continue;
                        }
                        if !values_equal(&cur, &c.before) {
                            let msg = format!("{label} ({}): the destination holds {} where the package expected {} (the recipient's own change); the package would set {}", t.where_, value_display(&cur), value_display(&c.before), value_display(&c.after));
                            if allow_conflicts {
                                conflicts.push(format!("{msg} — replacing it as allowed"));
                            } else {
                                conflicts.push(msg);
                                op_conflict = true;
                                continue;
                            }
                        }
                        resolved.push((file, rec));
                    }
                    if op_blocked || op_conflict {
                        continue;
                    }
                    if resolved.is_empty() {
                        if op_already > 0 {
                            already += 1;
                        }
                        continue;
                    }
                    let value = match &c.after {
                        CellValue::Int { decoded, .. } => FieldValue::Int(*decoded),
                        CellValue::Text { text } => FieldValue::Text(text.clone()),
                    };
                    for (file, rec) in resolved {
                        replay.entry(file).or_default().push((c.studio_table.clone(), rec, c.studio_field.clone(), value.clone()));
                    }
                    applied += 1;
                }
                drop(databases);
                if blocked.is_empty() && (allow_conflicts || conflicts.is_empty()) {
                    for (file, cells) in replay {
                        let t = targets.get_mut(file).expect("resolved against a staged target");
                        t.ws.set_package_fields(&cells).map_err(|e| ApiError::Workspace(format!("the package change set is not valid on the destination's {}: {e}", t.where_)))?;
                    }
                }
                if let Some(pool) = &edits.free_agents {
                    let t = targets.get_mut("main_db").ok_or_else(|| ApiError::Workspace("Free-agent package requires a main database".into()))?;
                    let current = studio_domain::explicit_free_agents(t.ws.primary().bytes()).map_err(|e| ApiError::Workspace(e.to_string()))?;
                    if current.as_ref() == Some(&pool.after) {
                        // Already applied; do not reorder or duplicate the list.
                    } else if current.as_ref() != Some(&pool.before) {
                        blocked.push("The game's free-agent list differs from this mod's baseline; restore the original installation first".into());
                    } else if blocked.is_empty() && (allow_conflicts || conflicts.is_empty()) {
                        t.ws.set_package_free_agents(&pool.after).map_err(|e| ApiError::Workspace(format!("Free-agent list: {e}")))?;
                    }
                }
                if !edits.schedules.is_empty() {
                    let target=targets.get_mut("main_db").ok_or_else(||ApiError::Workspace("Schedule replacement requires the game database".into()))?;
                    for schedule in &edits.schedules {
                        let current=target.ws.primary().schedule_model(&schedule.table)?;
                        if current.rows==schedule.after {continue;}
                        if current.rows!=schedule.before {blocked.push(format!("Schedule {} differs from this mod's baseline. Restore the original installation first.",schedule.table));}
                        else if blocked.is_empty() && (allow_conflicts || conflicts.is_empty()) {
                            target.ws.replace_schedules(&[(schedule.table.clone(),schedule.after.clone())])?;
                        }
                    }
                }
                if blocked.is_empty() && (allow_conflicts || conflicts.is_empty()) {
                    for op in &edits.career_removals {
                        let target = targets.get_mut("main_db").expect("career target checked");
                        rewrite_career_rows(&mut target.ws, &op.table, &op.rewritten)?;
                        for &record in op.removed.iter().rev() { target.ws.delete_career_row(&op.table, record)?; }
                    }
                }
                // The staged outputs, in role order.
                for file in ["main_db", "roster"] {
                    let Some(t) = targets.get(file) else { continue };
                    let bytes = t.ws.primary().save().map_err(ApiError::Domain)?;
                    let (role, label): (&'static str, String) = if file == "roster" {
                        ("roster-save", format!("Roster save ({})", t.destination.file_name().unwrap_or_default().to_string_lossy()))
                    } else {
                        ("database", format!("Main database ({})", target.main_db))
                    };
                    push_item(&mut items, file, role, label, t.destination.clone(), bytes)?;
                    if file == "roster" && items.last().is_some_and(|i| i.state != "unchanged") {
                        // A content-indexed save layout needs its sidecar: it is part of the
                        // reviewed plan, with the sibling it is cloned from bound by identity.
                        match studio_domain::content_header::plan_content_header(&t.destination).map_err(ApiError::Domain)? {
                            studio_domain::content_header::HeaderPlan::Required { header_path, bytes, template } => {
                                let n = items.len();
                                push_item(&mut items, "roster.header", "content-header", "Save index header".into(), header_path, bytes)?;
                                if let Some(it) = items.get_mut(n) {
                                    it.template = template.as_ref().map(|tp| (tp.path.clone(), tp.identity.clone()));
                                    it.notes.push(match &template {
                                        Some(tp) => format!("the save-data index needs a header for this roster; cloned from {}", s(&tp.path)),
                                        None => "the save-data index needs a header for this roster; synthesized from the verified layout".into(),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            if !edits.dictionary.is_empty() && blocked.is_empty() {
                let loc = report.db_paths.localization.clone().unwrap();
                let mut d = studio_domain::leagues::AttachedDictionary::load(&loc).map_err(|e| ApiError::Workspace(format!("the destination dictionary {loc} does not open: {e}")))?;
                let mut changed = false;
                for op in &edits.dictionary {
                    match d.dictionary.text(&op.key) {
                        None => {
                            blocked.push(format!("dictionary key {:?} does not exist in the destination dictionary; not applied", op.key));
                            continue;
                        }
                        Some(cur) if cur == op.after => {
                            already += 1;
                            continue;
                        }
                        Some(cur) if cur != op.before => {
                            let msg = format!("dictionary {:?}: the destination reads {cur:?} where the package expected {:?}", op.key, op.before);
                            if allow_conflicts {
                                conflicts.push(format!("{msg} — replacing it as allowed"));
                            } else {
                                conflicts.push(msg);
                                continue;
                            }
                        }
                        Some(_) => {}
                    }
                    let edit = studio_domain::dictionary_edit::apply_retext(&d, &op.key, &op.after).map_err(|e| ApiError::Workspace(format!("dictionary {:?}: the text cannot be encoded on this destination: {e}", op.key)))?;
                    let bytes = edit.output.as_ref().clone();
                    d = studio_domain::leagues::AttachedDictionary::from_bytes(&loc, bytes).map_err(|e| ApiError::Workspace(format!("dictionary {:?}: the re-encoded dictionary does not parse: {e}", op.key)))?;
                    changed = true;
                    applied += 1;
                }
                if changed {
                    push_item(&mut items, "dictionary", "dictionary", "Localization dictionary".into(), PathBuf::from(&loc), d.bytes.as_ref().clone())?;
                }
            }
            if !edits.textures.is_empty() && blocked.is_empty() {
                for t in &edits.textures {
                    // The recipient's own file at the same relative path under its asset root(s).
                    let mut candidates = Vec::new();
                    for root in &roots {
                        match studio_domain::resolve_install_path(root, &t.rel_path) {
                            Some(p) => candidates.push(p),
                            None => {
                                blocked.push(format!("{}: escapes the destination's asset tree; refused", t.rel_path));
                                continue;
                            }
                        }
                    }
                    let Some(dest) = candidates.iter().find(|p| p.is_file()).cloned() else {
                        blocked.push(format!(
                            "{}: the destination has no such file ({}); a loose texture replacement needs the installed, extracted file — extract the game's archives first or choose the right game folder",
                            t.rel_path,
                            candidates.iter().map(|p| s(p)).collect::<Vec<_>>().join(" | ")
                        ));
                        continue;
                    };
                    let current = std::fs::read(&dest).map_err(|e| ApiError::Workspace(format!("{}: cannot be read: {e}", s(&dest))))?;
                    let cur_id = FileIdentity::of_bytes(&current);
                    if let Some(prior) = &t.prior_sha256 {
                        if &cur_id.sha256 != prior {
                            let msg = format!("{}: the destination file ({}) is not the file the author replaced ({}…); it may hold the recipient's own edit", t.rel_path, cur_id, &prior[..12]);
                            if allow_conflicts {
                                conflicts.push(format!("{msg} — replacing it as allowed"));
                            } else {
                                conflicts.push(msg);
                                continue;
                            }
                        }
                    }
                    // Several raster edits can target the same container. Verify each
                    // against the installed baseline, but encode onto the preceding edit.
                    let current = items.iter().find(|item| item.destination == dest && item.role == "loose-asset")
                        .map(|item| item.bytes.clone()).unwrap_or(current);
                    let payload = ins.payload(&t.payload).map_err(ApiError::Domain)?;
                    let img = studio_domain::decode_source_image_bytes(&payload).map_err(|e| ApiError::Workspace(format!("{}: the package's source image does not decode: {e}", t.rel_path)))?;
                    if (img.width, img.height) != (t.width, t.height) {
                        blocked.push(format!("{}: the package's source image is {}×{} but the edit records {}×{}", t.rel_path, img.width, img.height, t.width, t.height));
                        continue;
                    }
                    let plan = match studio_domain::plan_texture_replace(&current, img.width, img.height, t.texture_index.map(|i| i as usize)) {
                        Ok(p) => p,
                        Err(e) => {
                            blocked.push(format!("{}: the destination file is not a texture container this Studio can write ({e}); the replacement is not applied", t.rel_path));
                            continue;
                        }
                    };
                    if !plan.allowed {
                        blocked.push(format!("{}: {}", t.rel_path, plan.refusal.unwrap_or_else(|| "the texture writer refuses this replacement on the destination file".into())));
                        continue;
                    }
                    let write = match studio_domain::replace_texture_in_file(current, &img.rgba, img.width, img.height, t.texture_index.map(|i| i as usize)) {
                        Ok(w) => w,
                        Err(e) => {
                            blocked.push(format!("{}: {e}", t.rel_path));
                            continue;
                        }
                    };
                    if let Some(it) = items.iter_mut().find(|item| item.destination == dest && item.role == "loose-asset") {
                        it.replacement = FileIdentity::of_bytes(&write.bytes);
                        it.bytes = write.bytes;
                        it.state = if it.destination_identity.as_ref() == Some(&it.replacement) { "unchanged" } else { "replace" };
                        it.notes.extend(write.notes);
                    } else {
                        let n = items.len();
                        push_item(&mut items, &format!("texture:{}", t.rel_path), "loose-asset", format!("{} ({}×{} {})", t.rel_path, write.width, write.height, write.container.label()), dest, write.bytes)?;
                        if let Some(it) = items.get_mut(n) { it.notes.extend(write.notes); }
                    }
                    applied += 1;
                }
            }
            Ok(())
        })();
        let _ = std::fs::remove_dir_all(&staging);
        result?;
        let changing: Vec<&PlannedInstall> = items.iter().filter(|i| i.state != "unchanged").collect();
        let mut reasons: Vec<String> = blocked.clone();
        if !allow_conflicts && !conflicts.is_empty() {
            reasons.push(format!("{} conflict(s) with the recipient's own changes; review them and allow replacing conflicting values to continue", conflicts.len()));
        }
        if changing.is_empty() && reasons.is_empty() {
            reasons.push(if already > 0 { "every edit in the package is already present at the destination; nothing to install".into() } else { "nothing would change at the destination".into() });
        }
        let installable = reasons.is_empty();
        let handle = {
            let mut st = self.packages.lock().unwrap();
            let h = st.next;
            st.next += 1;
            st.plans.insert(h, InstallPlan { records_dir: records_dir.clone(), package: identity.clone(), game_id: game_id.into(), platform, picked_root: picked_root.into(), save_root: report.save_root.clone().or(save_path_owned.clone()), items: items.clone(), blocked: reasons.clone() });
            h
        };
        Ok(InstallPlanDto {
            handle,
            installable,
            reasons,
            conflicts,
            requirements,
            items: items.iter().map(install_item_dto).collect(),
            applied: applied as u64,
            already_present: already as u64,
            backup_bytes: changing.iter().filter_map(|i| i.destination_identity.as_ref().map(|d| d.len)).sum(),
            replacement_bytes: changing.iter().map(|i| i.replacement.len).sum(),
            package_name: m.name.clone(),
            package_version: m.version.clone(),
            package_id: m.package_id.clone(),
            game_id: game_id.into(),
            platform: platform.as_str().into(),
            records_dir: s(&records_dir),
        })
    }

    /// Apply a reviewed installation plan: every destination must still be what the plan saw;
    /// backups go under the installation record first; the writes publish as one two-phase set.
    pub fn package_apply(&self, handle: u32) -> Result<InstallApplyDto, ApiError> {
        self.package_apply_with(handle, &mut ips::NoHooks)
    }

    /// [`Registry::package_apply`] with publication hooks (fault injection in tests).
    pub fn package_apply_with(&self, handle: u32, hooks: &mut dyn Hooks) -> Result<InstallApplyDto, ApiError> {
        let plan = {
            let mut st = self.packages.lock().unwrap();
            st.plans.remove(&handle).ok_or_else(|| ApiError::Workspace(format!("installation plan {handle} is not current (already applied, superseded or never existed); review again")))?
        };
        if !plan.blocked.is_empty() {
            return Err(ApiError::Workspace(format!("the installation is blocked: {}", plan.blocked.join("; "))));
        }
        let changing: Vec<&PlannedInstall> = plan.items.iter().filter(|i| i.state != "unchanged").collect();
        if changing.is_empty() {
            return Err(ApiError::Workspace("nothing to install".into()));
        }
        // Bound inputs the planned bytes were derived from must be as the plan saw them.
        for item in &changing {
            if let Some((path, identity)) = &item.template {
                let now = FileIdentity::of_file(path).map_err(|e| ApiError::Workspace(format!("{}: the sibling {} the plan cloned cannot be read now: {e}; review again", item.label, s(path))))?;
                if &now != identity {
                    return Err(ApiError::Workspace(format!("{}: the sibling {} the plan cloned changed since the review; review again", item.label, s(path))));
                }
            }
        }
        let seq = next_install_seq(&plan.records_dir)?;
        let record_dir = plan.records_dir.join(seq.to_string());
        if record_dir.exists() {
            return Err(ApiError::Workspace(format!("installation record {} already exists; review again", record_dir.display())));
        }
        let backup_dir = record_dir.join("backups");
        std::fs::create_dir_all(&backup_dir).map_err(|source| ApiError::Io { path: backup_dir.display().to_string(), source })?;
        let fail = |e: ApiError| {
            let _ = std::fs::remove_dir_all(&record_dir);
            e
        };
        let backup_bytes: u64 = changing.iter().filter_map(|i| i.destination_identity.as_ref().map(|d| d.len)).sum();
        super::project::check_free_space(&plan.records_dir, backup_bytes, "writing the installation backups").map_err(fail)?;
        let replacement_bytes: u64 = changing.iter().map(|i| i.replacement.len).sum();
        if let Some(first) = changing.first().map(|i| i.destination.clone()) {
            super::project::check_free_space(first.parent().unwrap_or(&first), replacement_bytes, "writing the installed files").map_err(fail)?;
        }
        let mut writes: Vec<PlannedWrite> = Vec::new();
        let mut record_items: Vec<InstallRecordItem> = Vec::new();
        for (n, item) in changing.iter().enumerate() {
            let expectation = match &item.destination_identity {
                Some(i) => Expectation::Identity(i.clone()),
                None => Expectation::Absent,
            };
            let prior = ips::read_prior(item.role, &item.destination, &expectation).map_err(|e| fail(ApiError::Domain(e)))?;
            let backup = match &prior {
                Prior::Bytes(bytes) => {
                    let name = format!("{n:03}-{}", item.destination.file_name().unwrap_or_default().to_string_lossy());
                    studio_domain::baselines::write_atomic(&backup_dir.join(&name), bytes).map_err(|e| fail(ApiError::Domain(e)))?;
                    Some(format!("backups/{name}"))
                }
                Prior::Absent => None,
            };
            record_items.push(InstallRecordItem {
                key: item.key.clone(),
                label: item.label.clone(),
                destination: s(&item.destination),
                prior: item.destination_identity.clone().map(|i| studio_domain::project_store::IdentityRecord::from(&i)),
                replacement: studio_domain::project_store::IdentityRecord::from(&item.replacement),
                backup,
                state: "planned".into(),
            });
            writes.push(PlannedWrite { role: item.role, path: item.destination.clone(), prior, bytes: item.bytes.clone(), identity: item.replacement.clone() });
        }
        let mut record = InstallRecord {
            schema: RECORD_SCHEMA.into(),
            seq,
            at: studio_domain::project_store::now_iso(),
            state: "publishing".into(),
            package_id: plan.package.package_id.clone(),
            package_name: plan.package.name.clone(),
            package_version: plan.package.version.clone(),
            package_sha256: plan.package.file_sha256.clone(),
            game_id: plan.game_id.clone(),
            platform: plan.platform.as_str().into(),
            picked_root: plan.picked_root.clone(),
            save_root: plan.save_root.clone(),
            items: record_items,
        };
        write_install_record(&record_dir, &record)?;
        let outcome = ips::publish(&writes, hooks);
        let (state, detail) = match &outcome {
            ips::Outcome::Committed => ("committed", None),
            ips::Outcome::RolledBack { .. } => ("rolled-back", Some(outcome.describe(&writes))),
            ips::Outcome::Split { .. } => ("split", Some(outcome.describe(&writes))),
        };
        for it in record.items.iter_mut() {
            it.state = recovery::state_on_disk(Path::new(&it.destination), it.prior.as_ref().map(|p| p.to_identity()).as_ref(), &it.replacement.to_identity(), "installed");
        }
        record.state = state.into();
        write_install_record(&record_dir, &record)?;
        let dto = InstallApplyDto {
            seq,
            state: state.into(),
            detail,
            record_dir: s(&record_dir),
            destinations: record.items.iter().map(|i| DeployOutcomeDto { key: i.key.clone(), label: i.label.clone(), path: i.destination.clone(), state: i.state.clone(), backup: i.backup.clone() }).collect(),
        };
        if state == "committed" {
            Ok(dto)
        } else {
            Err(ApiError::Workspace(format!("the installation did not commit ({state}): {}; the record and its backups are kept in {}{}", dto.detail.clone().unwrap_or_default(), dto.record_dir, if state == "split" { " — use Recover on that record once the destination is reachable again" } else { "" })))
        }
    }

    /// Every installation record under `records_dir`, newest first.
    pub fn package_installs(&self, records_dir: &str) -> Result<Vec<InstallRecordDto>, ApiError> {
        let dir = Path::new(records_dir);
        let mut out = Vec::new();
        let Ok(rd) = std::fs::read_dir(dir) else { return Ok(out) };
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_dir() || !p.join("record.json").is_file() {
                continue;
            }
            if let Ok(r) = read_install_record(&p) {
                out.push(InstallRecordDto {
                    record_dir: s(&p),
                    seq: r.seq,
                    at: r.at.clone(),
                    state: r.state.clone(),
                    package_name: r.package_name.clone(),
                    package_version: r.package_version.clone(),
                    package_id: r.package_id.clone(),
                    game_id: r.game_id.clone(),
                    platform: r.platform.clone(),
                    picked_root: r.picked_root.clone(),
                    restorable: recovery::recoverable(&r.state),
                    recovery: r.state != "committed" && recovery::recoverable(&r.state),
                    items: r.items.iter().map(|i| DeployOutcomeDto { key: i.key.clone(), label: i.label.clone(), path: i.destination.clone(), state: i.state.clone(), backup: i.backup.clone() }).collect(),
                });
            }
        }
        out.sort_by(|a, b| b.seq.cmp(&a.seq));
        Ok(out)
    }

    /// Restore an installation (uninstall) or recover an interrupted one: every destination is
    /// classified by what it holds now (see [`super::recovery`]); a later foreign change refuses
    /// the whole restore and nothing is touched.
    pub fn package_restore(&self, record_dir: &str) -> Result<InstallApplyDto, ApiError> {
        self.package_restore_with(record_dir, &mut ips::NoHooks)
    }

    /// [`Registry::package_restore`] with publication hooks (fault injection in tests).
    pub fn package_restore_with(&self, record_dir: &str, hooks: &mut dyn Hooks) -> Result<InstallApplyDto, ApiError> {
        let record_dir = PathBuf::from(record_dir);
        let mut record = read_install_record(&record_dir)?;
        if !recovery::recoverable(&record.state) {
            return Err(ApiError::Workspace(format!("installation {} is {}; there is nothing to restore or recover", record.seq, record.state)));
        }
        let items: Vec<recovery::RecordedItem> = record
            .items
            .iter()
            .map(|it| recovery::RecordedItem {
                label: it.label.clone(),
                destination: PathBuf::from(&it.destination),
                prior: it.prior.as_ref().map(|p| p.to_identity()),
                replacement: it.replacement.to_identity(),
                backup: it.backup.as_ref().map(|b| record_dir.join(b.replace('/', std::path::MAIN_SEPARATOR_STR))),
            })
            .collect();
        let plan = recovery::plan_restore(&items, "installed")?;
        let outcome = if recovery::nothing_to_do(&plan) { ips::Outcome::Committed } else { ips::publish_all(&plan.writes, &plan.removals, hooks) };
        let (state, item_states) = recovery::classify(&outcome, &items, &plan, "installed", &record.state);
        for (it, st) in record.items.iter_mut().zip(item_states) {
            it.state = st;
        }
        record.state = state.clone();
        write_install_record(&record_dir, &record)?;
        let dto = InstallApplyDto { seq: record.seq, state: state.clone(), detail: None, record_dir: s(&record_dir), destinations: record.items.iter().map(|i| DeployOutcomeDto { key: i.key.clone(), label: i.label.clone(), path: i.destination.clone(), state: i.state.clone(), backup: i.backup.clone() }).collect() };
        if state != "restored" {
            return Err(ApiError::Workspace(format!(
                "the restore did not complete (record {}): {}; retry once the destination is reachable again — every destination still holding the installed bytes is put back then",
                state,
                outcome.describe_all(&plan.writes, &plan.removals)
            )));
        }
        Ok(dto)
    }
}

// ---------------------------------------------------------------------- helpers

/// Read one cell of a session's current working bytes as the package's value shape.
fn read_cell(session: &EditSession, db: &tdb_core::TdbFile<'_>, studio_table: &str, record: u32, studio_field: &str) -> Result<Option<CellValue>, ApiError> {
    let Some(table) = db.table_canonical(studio_table) else { return Ok(None) };
    if record >= table.current_records as u32 {
        return Ok(None);
    }
    let endian = db.endian;
    let Some(field) = table.fields.iter().find(|f| f.id.canonical(endian) == studio_field) else { return Ok(None) };
    let project = session.project();
    let view = studio_domain::read_field(&db, project.meta(), project.enums(), table, record, field);
    Ok(Some(cell_value(&view)))
}

fn values_equal(a: &CellValue, b: &CellValue) -> bool {
    match (a, b) {
        (CellValue::Int { raw: x, .. }, CellValue::Int { raw: y, .. }) => x == y,
        (CellValue::Text { text: x }, CellValue::Text { text: y }) => x == y,
        _ => false,
    }
}

fn push_item(items: &mut Vec<PlannedInstall>, key: &str, role: &'static str, label: String, destination: PathBuf, bytes: Vec<u8>) -> Result<(), ApiError> {
    let replacement = FileIdentity::of_bytes(&bytes);
    let (destination_identity, state) = match std::fs::read(&destination) {
        Ok(cur) => {
            let id = FileIdentity::of_bytes(&cur);
            if id == replacement { (Some(id), "unchanged") } else { (Some(id), "replace") }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (None, "create"),
        Err(e) => return Err(ApiError::Workspace(format!("{label}: {} cannot be read: {e}", s(&destination)))),
    };
    items.push(PlannedInstall { key: key.into(), role, label, destination, destination_identity, bytes, replacement, state, notes: Vec::new(), template: None });
    Ok(())
}

fn install_item_dto(i: &PlannedInstall) -> InstallItemDto {
    InstallItemDto {
        key: i.key.clone(),
        role: i.role.into(),
        label: i.label.clone(),
        destination_path: s(&i.destination),
        destination: i.destination_identity.as_ref().map(identity_dto),
        replacement: identity_dto(&i.replacement),
        state: i.state.into(),
        notes: i.notes.clone(),
    }
}

/// Build the same read-only expandable rows used by Project Overview. This reads
/// only the validated package, so it works without an open authoring project or installation.
fn package_changes(edits: &PackageEdits) -> Vec<crate::dto::ProjectOverviewChangeDto> {
    let mut changes = Vec::new();
    let mut add = |groups: Vec<String>, label: String, before: String, after: String| {
        changes.push(crate::dto::ProjectOverviewChangeDto {
            id: format!("package:{}", changes.len()), groups, label, before, after, route: None,
        });
    };
    for c in &edits.cells {
        let category = match c.studio_table.as_str() {
            "ubPc" => "Player data", "dSvy" | "mHuy" => "Attributes",
            "xmja" | "fMVl" => "Equipment", "eGlu" => "Rosters",
            "kOtt" | "HqzU" | "fLEY" => "Teams", _ => &c.table_name,
        };
        add(vec!["Database".into(), category.into(), c.entity.clone()],
            c.field_name.clone(), value_display(&c.before), value_display(&c.after));
    }
    if let Some(pool) = &edits.free_agents {
        for id in pool.before.iter().filter(|id| !pool.after.contains(id)) {
            add(vec!["Free agency".into()], format!("Player {id}"), "Free agent".into(), "Removed from free agents".into());
        }
        for id in pool.after.iter().filter(|id| !pool.before.contains(id)) {
            add(vec!["Free agency".into()], format!("Player {id}"), "Not a free agent".into(), "Free agent".into());
        }
    }
    for schedule in &edits.schedules {
        let describe = |row: &studio_domain::schedule_edit::GameRow| format!("{:02}/{:02} · home slot {} vs away slot {}", row.month + 1, row.day + 1, row.home_slot, row.away_slot);
        for index in 0..schedule.before.len().max(schedule.after.len()) {
            let before = schedule.before.get(index); let after = schedule.after.get(index);
            if before == after { continue; }
            add(vec!["Schedules".into(), schedule.table.clone()], format!("Game {}", index + 1),
                before.map(&describe).unwrap_or_else(|| "Not present".into()), after.map(&describe).unwrap_or_else(|| "Removed".into()));
        }
    }
    for removal in &edits.career_removals {
        let table = match removal.table.as_str() { "tsSy" => "Skater regular season", "OAqO" => "Skater playoffs", "USfF" => "Goalie regular season", "nCDp" => "Goalie playoffs", _ => &removal.table };
        for row in &removal.removed {
            add(vec!["Career statistics".into(), table.into()], format!("Record {}", row + 1), "Existing record".into(), "Removed".into());
        }
        for rewrite in &removal.rewritten {
            add(vec!["Career statistics".into(), table.into()], format!("Record {}", rewrite.row + 1), "Existing record".into(), "Edited".into());
        }
    }
    for d in &edits.dictionary { add(vec!["Localization".into()], d.key.clone(), d.before.clone(), d.after.clone()); }
    for t in &edits.textures {
        let mut groups = vec!["Art assets".into()];
        let normalized = t.rel_path.replace('\\', "/");
        let mut parts: Vec<_> = normalized.split('/').collect();
        let label = parts.pop().unwrap_or(&t.rel_path).to_string();
        groups.extend(parts.into_iter().map(str::to_owned));
        add(groups, label, "Original texture".into(), format!("Replacement · {} × {}{}", t.width, t.height, t.texture_index.map(|i| format!(" · texture {}", i + 1)).unwrap_or_default()));
    }
    changes
}

fn inspect_dto(path: &str, ins: &InspectedPackage) -> PackageInspectDto {
    let m = &ins.manifest;
    PackageInspectDto {
        author: ins.manifest.author.clone(),
        path: path.into(),
        len: ins.file_len,
        sha256: ins.file_sha256.clone(),
        package_id: m.package_id.clone(),
        name: m.name.clone(),
        version: m.version.clone(),
        description: m.description.clone(),
        created: m.created.clone(),
        studio_version: m.studio.version.clone(),
        game_id: m.game_id.clone(),
        title: m.title.clone(),
        author_platform: m.author_platform.clone(),
        targets: m.targets.iter().map(target_dto).collect(),
        base: m.base.iter().map(|b| crate::dto::PackageBaseDto { role: b.role.clone(), file_name: b.file_name.clone(), len: b.len, sha256: b.sha256.clone(), provenance: b.provenance.clone() }).collect(),
        categories: m.categories.iter().map(|c| crate::dto::PackageCategoryCountDto { category: c.category.clone(), count: c.count }).collect(),
        entries: m.entries.iter().map(|e| PackageEntryDto { path: e.path.clone(), len: e.len, sha256: e.sha256.clone(), kind: e.kind.clone() }).collect(),
        cells: ins.edits.cells.iter().map(cell_dto).collect(),
        dictionary: ins.edits.dictionary.iter().map(|d| PackageDictionaryDto { key: d.key.clone(), before: d.before.clone(), after: d.after.clone() }).collect(),
        textures: ins.edits.textures.iter().map(|t| PackageTextureDto { rel_path: t.rel_path.clone(), texture_index: t.texture_index, width: t.width, height: t.height, platform: t.platform.clone(), payload: t.payload.clone() }).collect(),
        changes: package_changes(&ins.edits),
        project_source_kind: m.project.source_kind.clone(),
        edits_schema: ins.edits.schema.clone(),
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct InstallRecord {
    schema: String,
    seq: u64,
    at: String,
    state: String,
    package_id: String,
    package_name: String,
    package_version: String,
    package_sha256: String,
    game_id: String,
    platform: String,
    picked_root: String,
    save_root: Option<String>,
    items: Vec<InstallRecordItem>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct InstallRecordItem {
    key: String,
    label: String,
    destination: String,
    prior: Option<studio_domain::project_store::IdentityRecord>,
    replacement: studio_domain::project_store::IdentityRecord,
    backup: Option<String>,
    state: String,
}

fn write_install_record(dir: &Path, r: &InstallRecord) -> Result<(), ApiError> {
    let text = serde_json::to_string_pretty(r).map_err(|e| ApiError::Workspace(e.to_string()))?;
    studio_domain::baselines::write_atomic(&dir.join("record.json"), text.as_bytes()).map_err(ApiError::Domain)
}

fn read_install_record(dir: &Path) -> Result<InstallRecord, ApiError> {
    let text = std::fs::read_to_string(dir.join("record.json")).map_err(|e| ApiError::Workspace(format!("no installation record in {}: {e}", dir.display())))?;
    let r: InstallRecord = serde_json::from_str(&text).map_err(|e| ApiError::Workspace(format!("the installation record in {} is not readable: {e}", dir.display())))?;
    if r.schema != RECORD_SCHEMA {
        return Err(ApiError::Workspace(format!("the installation record in {} has schema {:?}, not {RECORD_SCHEMA:?}", dir.display(), r.schema)));
    }
    Ok(r)
}

fn next_install_seq(records_dir: &Path) -> Result<u64, ApiError> {
    let mut max = 0u64;
    if let Ok(rd) = std::fs::read_dir(records_dir) {
        for e in rd.flatten() {
            if let Some(n) = e.file_name().to_str().and_then(|n| n.parse::<u64>().ok()) {
                max = max.max(n);
            }
        }
    }
    Ok(max + 1)
}
