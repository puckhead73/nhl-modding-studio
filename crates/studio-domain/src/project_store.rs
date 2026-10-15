//! The project folder (ONB-002 §2): functional isolation of a Studio project from the game
//! installation, the save location and its own sources.
//!
//! ```text
//! <project>/
//!   nhl-studio-project.json   the manifest (schema `nhlms-project/2`; `/1` is migrated), written temp → rename
//!   sources/                  project-owned editable source assets (content-addressed; ONB-003)
//!   baseline/                 immutable copies of the sources + BASELINE.json (identities)
//!   working/                  a tdb-txn copied-target root: db/, loc/, saves/, overlay/, .tdb-txn/
//!   deployments/<seq>/        one record + backups per explicit deployment
//!   exports/                  the default Save As location
//! ```
//!
//! The edit session opens the **working** copies; a project save is a copied-target transaction on
//! `working/`; a deployment copies working outputs to the destinations recorded here after an
//! explicit review. Nothing in this module writes outside the project folder.
//!
//! **Folder rules.** The project folder must not be inside, equal to, or contain the game root, the
//! resolved install directory, the PS4 game folder, the save location or any import source. Paths
//! are compared after canonicalisation (junctions and symlinks resolved, on-disk spelling), so a
//! different spelling, a case variant or a junction into a destination is still a conflict. A
//! project folder that is itself a reparse point, or lies under one that resolves elsewhere, is
//! refused. The conflict is reported; no other folder is chosen.

use std::path::{Component as PathComponent, Path, PathBuf};

use serde::{Deserialize, Serialize};
use tdb_txn::FileIdentity;

use crate::baselines::{write_atomic, BaselineStore, Component};
use crate::{DomainError, Result};

pub const MANIFEST_FILE: &str = "nhl-studio-project.json";
/// The manifest schema this build writes (ONB-003: optional inputs, semantic records, last view).
pub const MANIFEST_SCHEMA: &str = "nhlms-project/2";
/// The ONB-002 schema; read and migrated in place (every added field has a default).
pub const MANIFEST_SCHEMA_V1: &str = "nhlms-project/1";
/// The advisory lock held while a Studio process has the project open.
pub const LOCK_FILE: &str = "nhl-studio-project.lock";
/// Project-owned editable source assets (the PNG / DDS / … a texture edit was made from).
pub const SOURCES_DIR: &str = "sources";
pub const BASELINE_DIR: &str = "baseline";
pub const BASELINE_RECORD: &str = "BASELINE.json";
pub const WORKING_DIR: &str = "working";
pub const OVERLAY_DIR: &str = "overlay";
pub const DEPLOYMENTS_DIR: &str = "deployments";
pub const EXPORTS_DIR: &str = "exports";

pub const SOURCE_FRESH: &str = "fresh";
pub const SOURCE_IMPORT: &str = "import";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectManifest {
    /// Display metadata travels with the project; changing its name never moves its folder.
    #[serde(default)]
    pub details: ProjectDetails,
    pub schema: String,
    pub project_id: String,
    pub created: String,
    pub game_id: String,
    pub platform: String,
    pub title: String,
    pub source: SourceRecord,
    pub working: WorkingSet,
    pub destinations: Destinations,
    /// A `.nhlmodproj` seen in the install's database folder when the project was created. Read as
    /// legacy configuration only; never migrated, rewritten or deleted.
    #[serde(default)]
    pub legacy_sidecar: Option<String>,
    #[serde(default)]
    pub extraction: ExtractionChoice,
    #[serde(default)]
    pub saves: Vec<SaveRecord>,
    /// Most recent successful save action, including a clean-session save.
    #[serde(default)]
    pub last_save_at: Option<String>,
    #[serde(default)]
    pub deployments: Vec<DeploymentRecord>,
    /// Project-owned loose-asset outputs under `working/overlay/`, install-relative.
    #[serde(default)]
    pub staged_files: Vec<StagedFile>,
    /// Installation archives copied into `working/` for pending member edits.
    #[serde(default)]
    pub materialized_archives: Vec<MaterializedArchive>,
    /// ONB-003: the schema the manifest was read as before this build migrated it (`None` when it
    /// was written by this schema).
    #[serde(default)]
    pub migrated_from: Option<String>,
    /// ONB-003: the last meaningful Studio route (`/app/database/players/12`), restored on reopen
    /// when it is still valid.
    #[serde(default)]
    pub last_view: Option<String>,
    /// ONB-003: which inputs this project was created with (main database, roster save,
    /// dictionary, extracted assets) and where each came from. A missing input gates only the
    /// features that need it.
    #[serde(default)]
    pub inputs: Vec<InputRecord>,
    /// ONB-003: project-owned editable sources (the image a texture or archive-member edit was
    /// made from), content-addressed under `sources/`.
    #[serde(default)]
    pub sources: Vec<SourceAsset>,
    /// ONB-003: the archive-member replacements committed by project saves, as semantic intent
    /// (archive, member, scope, source) rather than only the rewritten archive.
    #[serde(default)]
    pub member_edits: Vec<MemberEditRecord>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProjectDetails {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

/// One input the project was created with.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputRecord {
    /// `main_db` | `roster_save` | `dictionary` | `assets`.
    pub role: String,
    /// `bundled` | `import` | `absent`.
    pub origin: String,
    /// Why the input is absent (gaps), or where it came from.
    #[serde(default)]
    pub note: Option<String>,
}

/// A project-owned editable source file (content-addressed).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceAsset {
    /// Project-relative path (`sources/<sha256>.<ext>`).
    pub relative: String,
    pub len: u64,
    pub sha256: String,
    /// `image` | `other`.
    pub kind: String,
    /// The original file name the source was taken from (no directory), for display only.
    #[serde(default)]
    pub original_name: Option<String>,
    pub at: String,
}

/// One archive-member replacement a project save committed (semantic intent).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemberEditRecord {
    /// Install-relative archive path (`databigs/inxp_10.big`).
    pub archive_relative: String,
    pub member_path: String,
    pub whole_group: bool,
    pub container: String,
    pub width: u32,
    pub height: u32,
    /// The project source the new member bytes were encoded from (`sources` sha256), when the
    /// edit was made inside this project.
    #[serde(default)]
    pub source_sha256: Option<String>,
    pub description: String,
    /// The save that committed it.
    pub save_seq: u64,
    pub at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceRecord {
    /// `fresh` (bundled baseline) or `import` (user files).
    pub kind: String,
    #[serde(default)]
    pub inventory_schema: Option<String>,
    #[serde(default)]
    pub inventory_head: Option<String>,
    pub components: Vec<SourceComponent>,
    /// The baseline's explicit gaps at creation (fresh projects): what is not bundled and why.
    #[serde(default)]
    pub gaps: Vec<GapRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GapRecord {
    pub component: String,
    pub reason: String,
}

/// An installation archive copied into `working/` because a pending member edit needed it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterializedArchive {
    /// Working-root-relative path (forward slashes), equal to the install-relative archive path.
    pub relative: String,
    /// The installed archive it was copied from (the deployment destination).
    pub destination: String,
    /// The installed archive's identity when it was copied.
    pub prior: IdentityRecord,
    pub at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceComponent {
    pub role: String,
    /// Where the bytes came from: the inventory path (fresh) or the absolute source path (import).
    pub origin: String,
    pub file_name: String,
    #[serde(default)]
    pub container_relative: Option<String>,
    pub len: u64,
    pub sha256: String,
    pub provenance: String,
    pub witness: String,
    /// Project-relative path of the immutable baseline copy.
    pub baseline: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct WorkingSet {
    /// Project-relative paths of the working copies the session opens. ONB-003: a project may have
    /// no main database (roster-only, assets-only); the metadata is then the schema the roster save
    /// opens with, or absent for an assets-only project.
    #[serde(default, deserialize_with = "opt_string")]
    pub main_db: Option<String>,
    #[serde(default, deserialize_with = "opt_string")]
    pub meta: Option<String>,
    #[serde(default)]
    pub dictionary: Option<String>,
    #[serde(default)]
    pub roster_save: Option<String>,
    /// The roster save's container directory name (`BLUS311790200 - ROSTER`, `RosterCustom0`).
    #[serde(default)]
    pub roster_container_dir: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Destinations {
    /// The folder picked as the game installation (RPCS3 root, Xbox install dir, PS4 game folder
    /// or library, Recompiled folder).
    pub picked_root: String,
    #[serde(default)]
    pub install_dir: Option<String>,
    #[serde(default)]
    pub game_dir: Option<String>,
    #[serde(default)]
    pub save_root: Option<String>,
    /// The roster save path the deployment writes (chosen slot). `None` = to be created or absent.
    #[serde(default)]
    pub roster_save: Option<String>,
    #[serde(default)]
    pub main_db_path: Option<String>,
    #[serde(default)]
    pub meta_xml_path: Option<String>,
    #[serde(default)]
    pub localization_path: Option<String>,
    /// ONB-003: an explicit folder of extracted game assets (asset routes read it and stage their
    /// outputs into the overlay). Used as the install directory when no game installation is set.
    #[serde(default)]
    pub assets_dir: Option<String>,
}

/// ONB-002 wrote the working paths as plain strings; an empty string reads as absent.
fn opt_string<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Option<String>, D::Error> {
    let v: Option<String> = Option::deserialize(d)?;
    Ok(v.filter(|s| !s.is_empty()))
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ExtractionChoice {
    /// `pending` | `skipped` | `extracted` | `not_needed`.
    #[serde(default = "pending")]
    pub choice: String,
    #[serde(default)]
    pub at: Option<String>,
}

fn pending() -> String {
    "pending".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveRecord {
    pub seq: u64,
    pub at: String,
    pub txn_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeploymentRecord {
    pub seq: u64,
    pub at: String,
    /// `committed` | `rolled-back` | `split` | `restored`.
    pub state: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StagedFile {
    /// Install-relative path (forward slashes).
    pub relative: String,
    /// Which route produced it.
    pub origin: String,
    /// Identity of the installed file the output replaces, when one existed when it was staged.
    #[serde(default)]
    pub prior: Option<IdentityRecord>,
    pub at: String,
    /// ONB-003: the project source (`sources` sha256) the output was encoded from, when the route
    /// kept one (texture replace, nameplate). `None` for native-only outputs (kit clones).
    #[serde(default)]
    pub source_sha256: Option<String>,
    /// ONB-003: the texture index the replace targeted inside a multi-texture package.
    #[serde(default)]
    pub texture_index: Option<u32>,
    /// Ordered image operations for a container edited more than once. Keeping only
    /// the last source loses edits to other rasters when the build is replayed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub texture_layers: Vec<TextureLayer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextureLayer {
    pub source_sha256: String,
    pub texture_index: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityRecord {
    pub len: u64,
    pub sha256: String,
}

impl From<&FileIdentity> for IdentityRecord {
    fn from(i: &FileIdentity) -> Self {
        IdentityRecord { len: i.len, sha256: i.sha256.clone() }
    }
}

impl IdentityRecord {
    pub fn to_identity(&self) -> FileIdentity {
        FileIdentity { len: self.len, sha256: self.sha256.clone() }
    }
}

/// `BASELINE.json`: the immutable copies and their identities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaselineRecord {
    pub schema: String,
    pub components: Vec<SourceComponent>,
}

/// One file to place in the project at creation.
pub struct CopyPlan {
    pub role: String,
    pub source: CopySource,
    /// Project-relative path under `baseline/` and under `working/` (same relative path).
    pub relative: String,
    pub container_relative: Option<String>,
    pub provenance: String,
    pub witness: String,
}

pub enum CopySource {
    Bundled(Component),
    File(PathBuf),
}

/// What a new project is created from.
pub struct NewProject {
    pub game_id: String,
    pub platform: String,
    pub title: String,
    pub source_kind: String,
    pub inventory: Option<(String, String)>,
    pub destinations: Destinations,
    pub legacy_sidecar: Option<String>,
    pub working: WorkingSet,
    pub copies: Vec<CopyPlan>,
    pub inputs: Vec<InputRecord>,
}

/// An opened project folder.
#[derive(Clone, Debug)]
pub struct ProjectStore {
    pub dir: PathBuf,
    pub manifest: ProjectManifest,
}

fn io(path: &Path) -> impl Fn(std::io::Error) -> DomainError + '_ {
    move |source| DomainError::Io { path: path.display().to_string(), source }
}

/// Strip Windows' verbatim prefix from a canonical path, for display and comparison.
pub fn shown(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    PathBuf::from(s.strip_prefix(r"\\?\UNC\").map(|r| format!(r"\\{r}")).unwrap_or_else(|| s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()))
}

fn fold(p: &Path) -> String {
    let s = shown(p).to_string_lossy().replace('/', "\\");
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s
    }
}

/// Canonicalise `p`, or — when it does not exist yet — its nearest existing ancestor joined with
/// the missing tail. Returns `None` when no ancestor exists.
pub fn canonical_or_planned(p: &Path) -> Option<PathBuf> {
    let abs = if p.is_absolute() { p.to_path_buf() } else { std::env::current_dir().ok()?.join(p) };
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut cur = abs.clone();
    loop {
        if let Ok(c) = std::fs::canonicalize(&cur) {
            let mut out = c;
            for t in tail.iter().rev() {
                out.push(t);
            }
            return Some(out);
        }
        let name = cur.file_name()?.to_os_string();
        tail.push(name);
        cur = cur.parent()?.to_path_buf();
    }
}

/// `true` when `inner` equals or lies under `outer` (both canonical).
pub fn is_within(inner: &Path, outer: &Path) -> bool {
    let a = fold(inner);
    let b = fold(outer);
    a == b || a.starts_with(&format!("{b}\\"))
}

/// Refuse a project folder that aliases or overlaps any of `others` (label, path), or that lies
/// through a reparse point. `others` entries that do not exist are compared by their planned path.
pub fn check_project_folder(project: &Path, others: &[(&str, &Path)]) -> Result<PathBuf> {
    let planned = canonical_or_planned(project).ok_or_else(|| {
        DomainError::Validation(format!(
            "the project folder {} cannot be placed: none of its parent folders exists",
            project.display()
        ))
    })?;
    // A reparse point anywhere on the project path: the canonical path no longer spells the path
    // the user typed (after the drive letter's own spelling is folded).
    let typed = if project.is_absolute() {
        project.to_path_buf()
    } else {
        std::env::current_dir().map_err(io(project))?.join(project)
    };
    let typed_norm: PathBuf = typed
        .components()
        .filter(|c| !matches!(c, PathComponent::CurDir))
        .collect();
    if fold(&planned) != fold(&typed_norm) && !same_modulo_dotdot(&planned, &typed_norm) {
        return Err(DomainError::Validation(format!(
            "the project folder {} resolves to {}: it goes through a junction or symbolic link. Pick the real folder instead",
            project.display(),
            shown(&planned).display()
        )));
    }
    for (label, other) in others {
        let Some(o) = canonical_or_planned(other) else { continue };
        if is_within(&planned, &o) {
            return Err(DomainError::Validation(format!(
                "the project folder {} is inside {label} ({}). Staged work would overwrite the files it is meant to protect; choose a folder outside it",
                shown(&planned).display(),
                shown(&o).display()
            )));
        }
        if is_within(&o, &planned) {
            return Err(DomainError::Validation(format!(
                "the project folder {} contains {label} ({}). Choose a folder that does not hold the game, its saves or your source files",
                shown(&planned).display(),
                shown(&o).display()
            )));
        }
    }
    Ok(planned)
}

/// A typed path with `..` components still compares equal to its resolved form.
fn same_modulo_dotdot(canonical: &Path, typed: &Path) -> bool {
    let mut out = PathBuf::new();
    for c in typed.components() {
        match c {
            PathComponent::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    fold(canonical) == fold(&out)
}

/// A readable timestamp without a date crate (UTC, seconds).
pub fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Howard Hinnant's civil-from-days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60, rem % 60)
}

fn new_project_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:016x}{:08x}", nanos as u64, std::process::id())
}

fn rel_to_path(dir: &Path, rel: &str) -> PathBuf {
    let mut p = dir.to_path_buf();
    for part in rel.split('/') {
        if !part.is_empty() {
            p.push(part);
        }
    }
    p
}

impl ProjectStore {
    pub fn manifest_path(dir: &Path) -> PathBuf {
        dir.join(MANIFEST_FILE)
    }

    /// `true` when `dir` holds a Studio project manifest.
    pub fn is_project(dir: &Path) -> bool {
        Self::manifest_path(dir).is_file()
    }

    pub fn working_dir(&self) -> PathBuf {
        self.dir.join(WORKING_DIR)
    }
    pub fn baseline_dir(&self) -> PathBuf {
        self.dir.join(BASELINE_DIR)
    }
    pub fn overlay_dir(&self) -> PathBuf {
        self.working_dir().join(OVERLAY_DIR)
    }
    pub fn exports_dir(&self) -> PathBuf {
        self.dir.join(EXPORTS_DIR)
    }
    pub fn deployments_dir(&self) -> PathBuf {
        self.dir.join(DEPLOYMENTS_DIR)
    }
    /// A project-relative path (forward slashes) as an absolute path.
    pub fn path(&self, rel: &str) -> PathBuf {
        rel_to_path(&self.dir, rel)
    }
    /// The absolute path of an install-relative overlay output.
    pub fn overlay_path(&self, install_relative: &str) -> PathBuf {
        rel_to_path(&self.overlay_dir(), install_relative)
    }

    /// Create a new project folder. `dir` must be absent or an empty directory; every copy is
    /// verified; on any failure the folder this call created is removed again and the error says
    /// what failed. Missing parent directories are created and retained; existing parent
    /// contents are untouched, and all project files are written inside `dir`.
    pub fn create(dir: &Path, spec: NewProject, store: Option<&BaselineStore>) -> Result<ProjectStore> {
        let created_dir = match std::fs::read_dir(dir) {
            Ok(mut rd) => {
                if rd.next().is_some() {
                    return Err(DomainError::Validation(format!(
                        "{} is not empty. A new project needs an empty folder (or a new one); pick another folder",
                        dir.display()
                    )));
                }
                false
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Organized storage may not yet have its platform/game folders. Create
                // those first, then claim the project leaf exclusively to prevent reuse.
                if let Some(parent) = dir.parent().filter(|p| !p.as_os_str().is_empty()) {
                    std::fs::create_dir_all(parent).map_err(io(parent))?;
                }
                std::fs::create_dir(dir).map_err(io(dir))?;
                true
            }
            Err(e) => {
                return Err(DomainError::Io { path: dir.display().to_string(), source: e });
            }
        };
        let result = Self::populate(dir, spec, store);
        if result.is_err() {
            // Only what this call made: the folder was empty or absent on entry.
            if created_dir {
                let _ = std::fs::remove_dir_all(dir);
            } else {
                for sub in [BASELINE_DIR, WORKING_DIR, DEPLOYMENTS_DIR, EXPORTS_DIR] {
                    let _ = std::fs::remove_dir_all(dir.join(sub));
                }
                let _ = std::fs::remove_file(Self::manifest_path(dir));
            }
        }
        result
    }

    fn populate(dir: &Path, spec: NewProject, store: Option<&BaselineStore>) -> Result<ProjectStore> {
        let baseline = dir.join(BASELINE_DIR);
        let working = dir.join(WORKING_DIR);
        std::fs::create_dir_all(&baseline).map_err(io(&baseline))?;
        tdb_txn::engine::create_copied_target_root(&working).map_err(|e| {
            DomainError::Validation(format!("could not mark {} as the project's working root: {e}", working.display()))
        })?;
        std::fs::create_dir_all(dir.join(DEPLOYMENTS_DIR)).map_err(io(dir))?;
        std::fs::create_dir_all(dir.join(EXPORTS_DIR)).map_err(io(dir))?;
        let mut components = Vec::new();
        for plan in &spec.copies {
            let dest_baseline = rel_to_path(&baseline, &plan.relative);
            let dest_working = rel_to_path(&working, &plan.relative);
            let (identity, origin) = match &plan.source {
                CopySource::Bundled(c) => {
                    let store = store.ok_or_else(|| DomainError::Validation("a bundled component needs the baseline store".into()))?;
                    let id = store.copy_verified(c, &dest_baseline)?;
                    (id, c.path.clone())
                }
                CopySource::File(src) => {
                    let bytes = std::fs::read(src).map_err(io(src))?;
                    write_atomic(&dest_baseline, &bytes)?;
                    let back = std::fs::read(&dest_baseline).map_err(io(&dest_baseline))?;
                    if back != bytes {
                        return Err(DomainError::Validation(format!("{} did not read back as its source", dest_baseline.display())));
                    }
                    (FileIdentity::of_bytes(&bytes), shown(&canonical_or_planned(src).unwrap_or_else(|| src.clone())).display().to_string())
                }
            };
            let bytes = std::fs::read(&dest_baseline).map_err(io(&dest_baseline))?;
            write_atomic(&dest_working, &bytes)?;
            let back = FileIdentity::of_file(&dest_working).map_err(io(&dest_working))?;
            if back != identity {
                return Err(DomainError::Validation(format!("{} did not read back as the baseline", dest_working.display())));
            }
            components.push(SourceComponent {
                role: plan.role.clone(),
                origin,
                file_name: Path::new(&plan.relative).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                container_relative: plan.container_relative.clone(),
                len: identity.len,
                sha256: identity.sha256.clone(),
                provenance: plan.provenance.clone(),
                witness: plan.witness.clone(),
                baseline: format!("{BASELINE_DIR}/{}", plan.relative),
            });
        }
        let record = BaselineRecord { schema: MANIFEST_SCHEMA.into(), components: components.clone() };
        write_atomic(&baseline.join(BASELINE_RECORD), serde_json::to_string_pretty(&record).unwrap().as_bytes())?;
        let manifest = ProjectManifest {
            details: ProjectDetails::default(),
            schema: MANIFEST_SCHEMA.into(),
            project_id: new_project_id(),
            created: now_iso(),
            game_id: spec.game_id,
            platform: spec.platform,
            title: spec.title,
            source: SourceRecord {
                kind: spec.source_kind,
                inventory_schema: spec.inventory.as_ref().map(|(s, _)| s.clone()),
                inventory_head: spec.inventory.as_ref().map(|(_, h)| h.clone()),
                components,
                gaps: Vec::new(),
            },
            working: spec.working,
            destinations: spec.destinations,
            legacy_sidecar: spec.legacy_sidecar,
            extraction: ExtractionChoice::default(),
            saves: Vec::new(),
            last_save_at: None,
            deployments: Vec::new(),
            staged_files: Vec::new(),
            materialized_archives: Vec::new(),
            migrated_from: None,
            last_view: None,
            inputs: spec.inputs,
            sources: Vec::new(),
            member_edits: Vec::new(),
        };
        let store = ProjectStore { dir: dir.to_path_buf(), manifest };
        store.save_manifest()?;
        Ok(store)
    }

    /// Open an existing project folder. Refuses a missing or unreadable manifest, another schema,
    /// a missing working marker and missing working files — each named.
    pub fn open(dir: &Path) -> Result<ProjectStore> {
        let path = Self::manifest_path(dir);
        let text = std::fs::read_to_string(&path).map_err(|e| {
            DomainError::Validation(format!("{} is not a Studio project: {} ({e})", dir.display(), MANIFEST_FILE))
        })?;
        // The schema is checked before the full parse so an unknown (newer) schema is named as
        // such rather than as a field error.
        let head: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
            DomainError::Validation(format!("the project manifest {} is not readable: {e}. Nothing in the folder was changed", path.display()))
        })?;
        let schema = head.get("schema").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if schema != MANIFEST_SCHEMA && schema != MANIFEST_SCHEMA_V1 {
            return Err(DomainError::Validation(format!(
                "the project {} was written with schema {:?}; this build reads {MANIFEST_SCHEMA:?} (and migrates {MANIFEST_SCHEMA_V1:?}). Open it with the Studio version that wrote it. Nothing in the folder was changed",
                dir.display(),
                if schema.is_empty() { "<missing>" } else { schema.as_str() }
            )));
        }
        let mut manifest: ProjectManifest = serde_json::from_str(&text).map_err(|e| {
            DomainError::Validation(format!("the project manifest {} is not readable: {e}. Nothing in the folder was changed", path.display()))
        })?;
        if manifest.schema == MANIFEST_SCHEMA_V1 {
            // Lossless: every ONB-003 field defaults. The inputs are derived from the working set
            // ONB-002 wrote (a main database + metadata always; dictionary and roster when present).
            manifest.migrated_from = Some(MANIFEST_SCHEMA_V1.into());
            manifest.schema = MANIFEST_SCHEMA.into();
            if manifest.inputs.is_empty() {
                let origin = if manifest.source.kind == SOURCE_FRESH { "bundled" } else { "import" };
                let mut inputs = vec![InputRecord { role: "main_db".into(), origin: origin.into(), note: None }];
                if manifest.working.dictionary.is_some() {
                    inputs.push(InputRecord { role: "dictionary".into(), origin: origin.into(), note: None });
                }
                if manifest.working.roster_save.is_some() {
                    inputs.push(InputRecord { role: "roster_save".into(), origin: origin.into(), note: None });
                }
                manifest.inputs = inputs;
            }
        }
        let store = ProjectStore { dir: dir.to_path_buf(), manifest };
        tdb_txn::copied_target_root(&store.working_dir()).map_err(|e| {
            DomainError::Validation(format!("the project's working folder {} is not usable: {e}", store.working_dir().display()))
        })?;
        if store.manifest.working.main_db.is_none() && store.manifest.working.roster_save.is_none() && store.manifest.destinations.assets_dir.is_none() && store.manifest.destinations.install_dir.is_none() {
            return Err(DomainError::Validation(format!(
                "the project {} names no main database, roster save or extracted-assets folder; it cannot be opened. Nothing in the folder was changed",
                dir.display()
            )));
        }
        for (what, rel) in [
            ("main database", store.manifest.working.main_db.as_ref()),
            ("metadata", store.manifest.working.meta.as_ref()),
            ("dictionary", store.manifest.working.dictionary.as_ref()),
            ("roster save", store.manifest.working.roster_save.as_ref()),
        ] {
            // A paired project's retained save is archival, not an input to Mod Studio.
            if what == "roster save" && store.manifest.working.main_db.is_some() {
                continue;
            }
            if let Some(rel) = rel {
                let p = store.path(rel);
                if !p.is_file() {
                    return Err(DomainError::Validation(format!(
                        "the project's working {what} {} is missing; the project cannot be opened until it is restored from {}/{}",
                        p.display(),
                        BASELINE_DIR,
                        BASELINE_RECORD
                    )));
                }
            }
        }
        Ok(store)
    }

    /// Persist the manifest (temp → rename).
    pub fn save_manifest(&self) -> Result<()> {
        let text = serde_json::to_string_pretty(&self.manifest).map_err(|e| DomainError::Validation(e.to_string()))?;
        write_atomic(&Self::manifest_path(&self.dir), format!("{text}\n").as_bytes())
    }

    pub fn record_save(&mut self, txn_id: &str) -> Result<()> {
        let seq = self.manifest.saves.last().map(|s| s.seq + 1).unwrap_or(1);
        self.manifest.saves.push(SaveRecord { seq, at: now_iso(), txn_id: txn_id.into() });
        self.record_save_action()
    }

    pub fn record_save_action(&mut self) -> Result<()> {
        let previous = self.manifest.last_save_at.replace(now_iso());
        if let Err(error) = self.save_manifest() {
            self.manifest.last_save_at = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn set_extraction(&mut self, choice: &str) -> Result<()> {
        self.manifest.extraction = ExtractionChoice { choice: choice.into(), at: Some(now_iso()) };
        self.save_manifest()
    }

    pub fn record_deployment(&mut self, seq: u64, state: &str) -> Result<()> {
        match self.manifest.deployments.iter_mut().find(|d| d.seq == seq) {
            Some(d) => d.state = state.into(),
            None => self.manifest.deployments.push(DeploymentRecord { seq, at: now_iso(), state: state.into() }),
        }
        self.save_manifest()
    }

    pub fn next_deployment_seq(&self) -> u64 {
        self.manifest.deployments.iter().map(|d| d.seq + 1).max().unwrap_or(1)
    }

    /// Record (or re-record) an overlay output.
    pub fn record_staged_file(&mut self, relative: &str, origin: &str, prior: Option<&FileIdentity>) -> Result<()> {
        let rel = relative.replace('\\', "/");
        let entry = StagedFile { relative: rel.clone(), origin: origin.into(), prior: prior.map(IdentityRecord::from), at: now_iso(), source_sha256: None, texture_index: None, texture_layers: Vec::new() };
        match self.manifest.staged_files.iter_mut().find(|s| s.relative.eq_ignore_ascii_case(&rel)) {
            Some(s) => {
                // Keep the first prior (the installed bytes the first staging replaced).
                entry_keep_prior(s, entry);
            }
            None => self.manifest.staged_files.push(entry),
        }
        self.save_manifest()
    }

    /// Record an installation archive copied into `working/`.
    pub fn record_materialized(&mut self, relative: &str, source: &Path, identity: &FileIdentity) -> Result<()> {
        let rel = relative.replace('\\', "/");
        let entry = MaterializedArchive { relative: rel.clone(), destination: shown(&canonical_or_planned(source).unwrap_or_else(|| source.to_path_buf())).display().to_string(), prior: IdentityRecord::from(identity), at: now_iso() };
        match self.manifest.materialized_archives.iter_mut().find(|m| m.relative.eq_ignore_ascii_case(&rel)) {
            Some(m) => *m = entry,
            None => self.manifest.materialized_archives.push(entry),
        }
        self.save_manifest()
    }

    /// ONB-003: remember the last meaningful Studio route.
    pub fn record_view(&mut self, route: &str) -> Result<()> {
        if self.manifest.last_view.as_deref() == Some(route) {
            return Ok(());
        }
        self.manifest.last_view = Some(route.to_string());
        self.save_manifest()
    }

    pub fn sources_dir(&self) -> PathBuf {
        self.dir.join(SOURCES_DIR)
    }

    /// ONB-003: keep an editable source file inside the project (content-addressed; a second copy
    /// of the same bytes is the same record). Returns the record.
    pub fn store_source(&mut self, bytes: &[u8], ext: &str, original_name: Option<&str>) -> Result<SourceAsset> {
        let sha = crate::baselines::sha256_hex(bytes);
        if let Some(existing) = self.manifest.sources.iter().find(|s| s.sha256 == sha) {
            if self.path(&existing.relative).is_file() {
                return Ok(existing.clone());
            }
        }
        let ext = ext.trim_start_matches('.').to_ascii_lowercase();
        let ext = if ext.is_empty() || ext.len() > 8 || !ext.chars().all(|c| c.is_ascii_alphanumeric()) { "bin".to_string() } else { ext };
        let relative = format!("{SOURCES_DIR}/{sha}.{ext}");
        let dest = self.path(&relative);
        if !dest.is_file() {
            write_atomic(&dest, bytes)?;
        }
        let kind = if matches!(ext.as_str(), "png" | "dds" | "jpg" | "jpeg" | "bmp" | "tga" | "gif" | "webp") { "image" } else { "other" };
        let record = SourceAsset { relative, len: bytes.len() as u64, sha256: sha, kind: kind.into(), original_name: original_name.map(str::to_string), at: now_iso() };
        self.manifest.sources.retain(|s| s.sha256 != record.sha256);
        self.manifest.sources.push(record.clone());
        self.save_manifest()?;
        Ok(record)
    }

    /// ONB-003: record the archive-member replacements a save committed.
    pub fn record_member_edits(&mut self, edits: Vec<MemberEditRecord>) -> Result<()> {
        if edits.is_empty() {
            return Ok(());
        }
        self.manifest.member_edits.extend(edits);
        self.save_manifest()
    }

    /// ONB-003: record (or re-record) an overlay output together with the source it was encoded
    /// from. The first prior is kept like [`Self::record_staged_file`].
    pub fn record_staged_output(&mut self, relative: &str, origin: &str, prior: Option<&FileIdentity>, source_sha256: Option<&str>, texture_index: Option<u32>) -> Result<()> {
        let rel = relative.replace('\\', "/");
        let entry = StagedFile {
            relative: rel.clone(),
            origin: origin.into(),
            prior: prior.map(IdentityRecord::from),
            at: now_iso(),
            source_sha256: source_sha256.map(str::to_string),
            texture_index,
            texture_layers: Vec::new(),
        };
        match self.manifest.staged_files.iter_mut().find(|s| s.relative.eq_ignore_ascii_case(&rel)) {
            Some(s) => entry_keep_prior(s, entry),
            None => self.manifest.staged_files.push(entry),
        }
        self.save_manifest()
    }

    /// The project-owned output path for an installed file, materialising the installed bytes
    /// into the overlay the first time (so routes that read-modify-write see the install's file).
    /// Returns the overlay path and the installed file's identity (`None` when the install has no
    /// such file).
    pub fn materialize_overlay(&self, install_dir: &Path, install_relative: &str) -> Result<(PathBuf, Option<FileIdentity>)> {
        let rel = install_relative.replace('\\', "/");
        let overlay = self.overlay_path(&rel);
        let installed = rel_to_path(install_dir, &rel);
        let prior = match std::fs::read(&installed) {
            Ok(bytes) => Some(FileIdentity::of_bytes(&bytes)).map(|id| (id, bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(DomainError::Io { path: installed.display().to_string(), source: e }),
        };
        if !overlay.exists() {
            if let Some((_, bytes)) = &prior {
                write_atomic(&overlay, bytes)?;
            }
        }
        Ok((overlay, prior.map(|(id, _)| id)))
    }

    /// Every path that may never be written by an ordinary edit route: the destinations and the
    /// import sources. Used by the write guards.
    pub fn guarded_roots(&self) -> Vec<(String, PathBuf)> {
        let d = &self.manifest.destinations;
        let mut out = vec![("the game installation".to_string(), PathBuf::from(&d.picked_root))];
        for (label, p) in [
            ("the game's database folder", &d.install_dir),
            ("the game folder", &d.game_dir),
            ("the save location", &d.save_root),
        ] {
            if let Some(p) = p {
                out.push((label.to_string(), PathBuf::from(p)));
            }
        }
        if let Some(p) = &d.roster_save {
            out.push(("the roster save".to_string(), PathBuf::from(p)));
        }
        if self.manifest.source.kind == SOURCE_IMPORT {
            for c in &self.manifest.source.components {
                out.push((format!("the imported {}", c.role.replace('_', " ")), PathBuf::from(&c.origin)));
            }
        }
        out
    }

    /// Refuse a write to `path` when it lies inside any guarded root. The error names the root.
    pub fn refuse_guarded_write(&self, what: &str, path: &Path) -> Result<()> {
        let Some(target) = canonical_or_planned(path) else { return Ok(()) };
        for (label, root) in self.guarded_roots() {
            let Some(r) = canonical_or_planned(&root) else { continue };
            let inside = if r.is_file() || root.is_file() { fold(&target) == fold(&r) } else { is_within(&target, &r) };
            if inside {
                return Err(DomainError::Validation(format!(
                    "{what} would write into {label} ({}). Edit outputs stay in the project ({}); the game and its saves are only written by an explicit deployment",
                    shown(&r).display(),
                    shown(&self.dir).display()
                )));
            }
        }
        Ok(())
    }
}

fn entry_keep_prior(existing: &mut StagedFile, fresh: StagedFile) {
    if !matches!(fresh.origin.as_str(), "texture-replace" | "nameplate") {
        existing.texture_layers.clear();
    }
    if matches!(existing.origin.as_str(), "texture-replace" | "nameplate" | "texture-layers")
        && matches!(fresh.origin.as_str(), "texture-replace" | "nameplate") {
        if let (Some(previous), Some(next)) = (&existing.source_sha256, &fresh.source_sha256) {
            if existing.texture_layers.is_empty() {
                existing.texture_layers.push(TextureLayer { source_sha256: previous.clone(), texture_index: existing.texture_index });
            }
            existing.texture_layers.push(TextureLayer { source_sha256: next.clone(), texture_index: fresh.texture_index });
        }
    }
    existing.origin = fresh.origin;
    if !existing.texture_layers.is_empty() {
        // Older builds refuse this origin instead of silently packaging only its last image.
        existing.origin = "texture-layers".into();
    }
    existing.at = fresh.at;
    if existing.prior.is_none() {
        existing.prior = fresh.prior;
    }
    if fresh.source_sha256.is_some() {
        existing.source_sha256 = fresh.source_sha256;
        existing.texture_index = fresh.texture_index;
    }
}

/// ONB-003: the advisory lock a Studio process holds while a project is open. On Windows the lock
/// file is opened with no sharing, so a second process (or a second open in this one) cannot
/// open it until the first handle is dropped — a crashed process releases it automatically. On
/// other systems the file is removed when the lock drops.
pub struct ProjectLock {
    path: PathBuf,
    _file: std::fs::File,
}

impl ProjectLock {
    pub fn acquire(dir: &Path) -> Result<ProjectLock> {
        let path = dir.join(LOCK_FILE);
        let mut opts = std::fs::OpenOptions::new();
        opts.read(true).write(true).create(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            opts.share_mode(0);
        }
        let file = match opts.open(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(32) => {
                return Err(DomainError::Validation(format!(
                    "the project {} is open in another Studio window or process (its lock {} is held). Close it there first; nothing in the folder was changed",
                    dir.display(),
                    LOCK_FILE
                )));
            }
            Err(e) => return Err(DomainError::Io { path: path.display().to_string(), source: e }),
        };
        {
            use std::io::Write;
            let mut f = &file;
            let _ = f.write_all(format!("pid {} at {}\n", std::process::id(), now_iso()).as_bytes());
        }
        Ok(ProjectLock { path, _file: file })
    }
}

impl Drop for ProjectLock {
    fn drop(&mut self) {
        #[cfg(not(windows))]
        {
            let _ = std::fs::remove_file(&self.path);
        }
        #[cfg(windows)]
        {
            // The handle closes after this body; the stale lock file is re-opened exclusively by
            // the next acquire, so it is left in place.
            let _ = &self.path;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Self {
            let d = std::env::temp_dir().join(format!("nhlms-pstore-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            Tmp(d)
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn spec(src: &Path) -> NewProject {
        NewProject {
            game_id: "nhl14".into(),
            platform: "ps3".into(),
            title: "NHL 14 (PS3)".into(),
            source_kind: SOURCE_IMPORT.into(),
            inventory: None,
            destinations: Destinations { picked_root: "C:\\does-not-exist\\rpcs3".into(), ..Default::default() },
            legacy_sidecar: None,
            working: WorkingSet { main_db: Some("working/db/nhlng.db".into()), meta: Some("working/db/nhlng-meta.xml".into()), ..Default::default() },
            copies: vec![
                CopyPlan { role: "main_db".into(), source: CopySource::File(src.join("nhlng.db")), relative: "db/nhlng.db".into(), container_relative: None, provenance: "import".into(), witness: "".into() },
                CopyPlan { role: "meta".into(), source: CopySource::File(src.join("nhlng-meta.xml")), relative: "db/nhlng-meta.xml".into(), container_relative: None, provenance: "import".into(), witness: "".into() },
            ],
            inputs: vec![InputRecord { role: "main_db".into(), origin: "import".into(), note: None }],
        }
    }

    #[test]
    fn create_open_and_guard() {
        let t = Tmp::new("create");
        let src = t.0.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("nhlng.db"), b"DB-BYTES").unwrap();
        std::fs::write(src.join("nhlng-meta.xml"), b"<meta/>").unwrap();
        let dir = t.0.join("xbox360/nhl07/QA Test - NHL07");
        assert!(!dir.parent().unwrap().exists(), "first project has no platform/game folders yet");
        let store = ProjectStore::create(&dir, spec(&src), None).unwrap();
        assert!(ProjectStore::is_project(&dir));
        assert_eq!(std::fs::read(store.path("working/db/nhlng.db")).unwrap(), b"DB-BYTES");
        assert_eq!(std::fs::read(store.path("baseline/db/nhlng.db")).unwrap(), b"DB-BYTES");
        assert!(tdb_txn::copied_target_root(&store.working_dir()).is_ok());
        assert_eq!(std::fs::read(src.join("nhlng.db")).unwrap(), b"DB-BYTES", "the source is untouched");
        let reopened = ProjectStore::open(&dir).unwrap();
        assert_eq!(reopened.manifest.source.components.len(), 2);
        assert_eq!(reopened.manifest.source.components[0].sha256, crate::baselines::sha256_hex(b"DB-BYTES"));
        // A second create into the same (now non-empty) folder is refused and changes nothing.
        let err = ProjectStore::create(&dir, spec(&src), None).unwrap_err().to_string();
        assert!(err.contains("not empty"), "{err}");
        assert!(ProjectStore::open(&dir).is_ok());
        // The guard refuses writes into the import source and the picked root.
        let err = reopened.refuse_guarded_write("Save As", &src.join("nhlng.db")).unwrap_err().to_string();
        assert!(err.contains("imported main db"), "{err}");
        reopened.refuse_guarded_write("Save As", &t.0.join("elsewhere.db")).unwrap();
    }

    #[test]
    fn a_failed_copy_removes_the_created_folder() {
        let t = Tmp::new("fail");
        let src = t.0.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("nhlng.db"), b"DB").unwrap();
        // the meta file is missing → the copy fails after the db was placed
        let dir = t.0.join("xbox360/nhl07/QA Test - NHL07");
        let err = ProjectStore::create(&dir, spec(&src), None).unwrap_err().to_string();
        assert!(err.contains("nhlng-meta.xml"), "{err}");
        assert!(!dir.exists(), "the half-made folder is removed");
        assert!(dir.parent().unwrap().is_dir(), "shared platform/game folders are retained");
    }

    #[test]
    fn folder_rules_refuse_overlap_both_ways_and_accept_siblings() {
        let t = Tmp::new("rules");
        let game = t.0.join("game");
        std::fs::create_dir_all(game.join("USRDIR")).unwrap();
        let inside = game.join("project");
        let err = check_project_folder(&inside, &[("the game installation", &game)]).unwrap_err().to_string();
        assert!(err.contains("inside the game installation"), "{err}");
        let err = check_project_folder(&t.0, &[("the game installation", &game)]).unwrap_err().to_string();
        assert!(err.contains("contains the game installation"), "{err}");
        // Case variant of the game folder is still the game folder on a case-folding file system.
        if cfg!(windows) {
            let upper = PathBuf::from(game.to_string_lossy().to_uppercase()).join("x");
            let err = check_project_folder(&upper, &[("the game installation", &game)]).unwrap_err().to_string();
            assert!(err.contains("inside"), "{err}");
        }
        let ok = check_project_folder(&t.0.join("sibling"), &[("the game installation", &game)]).unwrap();
        assert!(ok.ends_with("sibling"));
        // A planned (not yet existing) destination still counts.
        let planned_game = t.0.join("future-game");
        let err = check_project_folder(&planned_game.join("p"), &[("the game installation", &planned_game)]).unwrap_err().to_string();
        assert!(err.contains("inside"), "{err}");
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_into_the_destination_is_refused() {
        let t = Tmp::new("junction");
        let game = t.0.join("game");
        std::fs::create_dir_all(&game).unwrap();
        let link = t.0.join("link");
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J", &link.to_string_lossy(), &game.to_string_lossy()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !made {
            eprintln!("skipping: mklink /J unavailable");
            return;
        }
        let err = check_project_folder(&link.join("p"), &[("the game installation", &game)]).unwrap_err().to_string();
        assert!(err.contains("junction") || err.contains("inside"), "{err}");
        let _ = std::fs::remove_dir(&link);
    }

    #[test]
    fn overlay_materialises_the_installed_file_once_and_records_it() {
        let t = Tmp::new("overlay");
        let src = t.0.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("nhlng.db"), b"DB").unwrap();
        std::fs::write(src.join("nhlng-meta.xml"), b"<m/>").unwrap();
        let install = t.0.join("install");
        std::fs::create_dir_all(install.join("fe/x")).unwrap();
        std::fs::write(install.join("fe/x/logo.big"), b"INSTALLED").unwrap();
        let dir = t.0.join("project");
        let mut store = ProjectStore::create(&dir, spec(&src), None).unwrap();
        let (p, prior) = store.materialize_overlay(&install, "fe/x/logo.big").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"INSTALLED");
        assert_eq!(prior.unwrap().len, 9);
        std::fs::write(&p, b"EDITED").unwrap();
        let (p2, _) = store.materialize_overlay(&install, "fe\\x\\logo.big").unwrap();
        assert_eq!(p2, p);
        assert_eq!(std::fs::read(&p).unwrap(), b"EDITED", "the overlay is not re-copied over an edit");
        assert_eq!(std::fs::read(install.join("fe/x/logo.big")).unwrap(), b"INSTALLED");
        store.record_staged_file("fe/x/logo.big", "texture-replace", prior_of(b"INSTALLED").as_ref()).unwrap();
        let again = ProjectStore::open(&dir).unwrap();
        assert_eq!(again.manifest.staged_files.len(), 1);
        assert_eq!(again.manifest.staged_files[0].prior.as_ref().unwrap().len, 9);
        let (_, none) = store.materialize_overlay(&install, "fe/x/absent.big").unwrap();
        assert!(none.is_none());
        store.record_staged_output("fe/x/logo.big", "texture-replace", prior_of(b"INSTALLED").as_ref(), Some("first-image"), Some(0)).unwrap();
        store.record_staged_output("fe/x/logo.big", "texture-replace", prior_of(b"INSTALLED").as_ref(), Some("second-image"), Some(1)).unwrap();
        let again = ProjectStore::open(&dir).unwrap();
        let staged = &again.manifest.staged_files[0];
        assert_eq!(staged.origin, "texture-layers");
        assert_eq!(staged.texture_layers.iter().map(|l| (l.source_sha256.as_str(), l.texture_index)).collect::<Vec<_>>(), vec![("first-image", Some(0)), ("second-image", Some(1))]);
        assert_eq!(staged.prior.as_ref().unwrap().len, 9);
        // An unrelated native overwrite must not keep replaying stale image operations.
        store.record_staged_output("fe/x/logo.big", "kit-clone", None, None, None).unwrap();
        assert!(store.manifest.staged_files[0].texture_layers.is_empty());
        assert_eq!(store.manifest.staged_files[0].origin, "kit-clone");
    }

    fn prior_of(b: &[u8]) -> Option<FileIdentity> {
        Some(FileIdentity::of_bytes(b))
    }

    #[test]
    fn now_iso_is_a_calendar_timestamp() {
        let s = now_iso();
        assert_eq!(s.len(), 20, "{s}");
        assert!(s.starts_with("20"), "{s}");
    }
}
