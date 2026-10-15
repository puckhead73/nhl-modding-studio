//! Studio workflow over an Xbox 360 disc image (`p1.1-xbox360-iso-member-route`):
//!
//! 1. **Stage** ([`stage`]): read the database and dictionary out of the image into a
//!    Studio-managed folder laid out like a loose install (`db/nhlng.db` + `db/nhlng-meta.xml`,
//!    `fe/loc/nhl_eng_us.db`) and write `ISO-SOURCE.json` — the image path, where each file was
//!    located inside it and the identity it had there. The ordinary workspace then opens the
//!    staged files; every edit and in-place save lands in the staging folder, never on the image.
//! 2. **Plan / apply** ([`plan_apply`], [`apply_planned`]): compare each staged file with its
//!    recorded prior identity; the files that changed become same-length range patches on the
//!    image ([`crate::iso_patch`]), applied in order with records under `<staging>/records/<n>/`.
//!    The plan the caller reviewed is the plan applied — [`apply_planned`] takes the planned
//!    patches (replacement bytes included) rather than re-reading the staged files. If any patch
//!    fails at or after its range write, every attempted range — the failing one included — is
//!    put back from its verified backup and classified; the outcome ([`ApplyOutcome`]) says
//!    whether the image is as it was (`RolledBack`) or which ranges hold what (`Split`). A
//!    failure before the first byte of a range changes is a plain refusal.
//! 3. **Restore** ([`restore`]): put every record of an apply back, newest first, verified;
//!    an interrupted range write is recovered from the record (bounded, see `iso_patch`).
//!
//! Nothing here launches anything or decides which source Xenia reads; that stays a runtime
//! question. The tests run on synthetic images only.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tdb_txn::FileIdentity;

use crate::error::DomainError;
use crate::iso_patch::{self, ApplyResult, PatchRecord, RangePatch, RangeState, RestoreOutcome};
use crate::xiso_source::{self, Located};
use crate::Result;

pub const STAGING_SCHEMA: &str = "studio-domain/iso-staging/1";
pub const MANIFEST: &str = "ISO-SOURCE.json";
pub const RECORDS_DIR: &str = "records";

/// One file staged from the image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagedFile {
    /// `nhlng` or `dictionary`.
    pub role: String,
    /// Path relative to the staging root.
    pub staged: String,
    pub located: LocatedRecord,
    /// Identity of the bytes as read from the image at staging time (the range's prior).
    pub prior: FileIdentity,
}

/// A serializable copy of [`Located`] (the disc member and, when packed, the entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocatedRecord {
    pub member_path: String,
    pub member_sector: u32,
    pub member_size: u32,
    pub member_attributes: u8,
    /// `None` for a loose member.
    pub entry: Option<EntryRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryRecord {
    pub kind: String,
    pub index: usize,
    pub path: String,
    pub offset: u64,
    pub stored_size: u32,
}

impl LocatedRecord {
    fn from_located(l: &Located) -> Self {
        match l {
            Located::Loose { member } => LocatedRecord {
                member_path: member.path.clone(),
                member_sector: member.sector,
                member_size: member.size,
                member_attributes: member.attributes,
                entry: None,
            },
            Located::InArchive {
                archive,
                kind,
                entry_index,
                entry_path,
                entry_offset,
                stored_size,
            } => LocatedRecord {
                member_path: archive.path.clone(),
                member_sector: archive.sector,
                member_size: archive.size,
                member_attributes: archive.attributes,
                entry: Some(EntryRecord {
                    kind: format!("{kind:?}").to_ascii_lowercase(),
                    index: *entry_index,
                    path: entry_path.clone(),
                    offset: *entry_offset,
                    stored_size: *stored_size,
                }),
            },
        }
    }

    fn to_located(&self) -> Located {
        let member = tdb_xiso::Member {
            path: self.member_path.clone(),
            sector: self.member_sector,
            size: self.member_size,
            attributes: self.member_attributes,
        };
        match &self.entry {
            None => Located::Loose { member },
            Some(e) => Located::InArchive {
                archive: member,
                kind: if e.kind == "eb" {
                    xiso_source::ArchiveKind::Eb
                } else {
                    xiso_source::ArchiveKind::Bigf
                },
                entry_index: e.index,
                entry_path: e.path.clone(),
                entry_offset: e.offset,
                stored_size: e.stored_size,
            },
        }
    }

    pub fn describe(&self) -> String {
        self.to_located().describe()
    }
}

/// `ISO-SOURCE.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagingManifest {
    pub schema: String,
    pub image: PathBuf,
    /// Identity of the whole image is not recorded (multi-GB); the members' ranges are.
    pub image_len: u64,
    pub layout: String,
    pub game_id: String,
    pub platform: String,
    pub files: Vec<StagedFile>,
    pub staged_at: String,
}

/// A staging folder as returned to the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staged {
    pub root: PathBuf,
    pub manifest: StagingManifest,
    pub db_path: PathBuf,
    pub meta_xml: PathBuf,
    pub dictionary_path: Option<PathBuf>,
    /// `true` when an existing staging for the same image was reused (its files may carry edits).
    pub reused: bool,
}

fn io_err(path: &Path, source: std::io::Error) -> DomainError {
    DomainError::Io {
        path: path.display().to_string(),
        source,
    }
}

fn manifest_path(root: &Path) -> PathBuf {
    root.join(MANIFEST)
}

/// Read a staging folder's manifest.
pub fn read_manifest(root: &Path) -> Result<StagingManifest> {
    let p = manifest_path(root);
    let text = fs::read_to_string(&p).map_err(|e| io_err(&p, e))?;
    let m: StagingManifest = serde_json::from_str(&text)
        .map_err(|e| DomainError::Validation(format!("{}: {e}", p.display())))?;
    if m.schema != STAGING_SCHEMA {
        return Err(DomainError::Validation(format!(
            "{} is a {:?} manifest, not {STAGING_SCHEMA}",
            p.display(),
            m.schema
        )));
    }
    Ok(m)
}

fn write_manifest(root: &Path, m: &StagingManifest) -> Result<()> {
    let p = manifest_path(root);
    let text = serde_json::to_string_pretty(m)
        .map_err(|e| DomainError::Validation(format!("manifest serialization: {e}")))?;
    fs::write(&p, text).map_err(|e| io_err(&p, e))
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// Stage the image's database and dictionary under `root` (created; an existing staging for the
/// **same image** is reused as-is, an existing folder for anything else is refused). `meta_xml`
/// is copied beside the database as `nhlng-meta.xml` so the ordinary open path finds it.
pub fn stage(
    image: &Path,
    game_id: &str,
    platform: &str,
    meta_xml: &Path,
    root: &Path,
) -> Result<Staged> {
    let db_path = root.join("db").join("nhlng.db");
    let meta_dst = root.join("db").join("nhlng-meta.xml");
    let dict_path = root.join("fe").join("loc").join("nhl_eng_us.db");
    if manifest_path(root).exists() {
        let m = read_manifest(root)?;
        if !same_file(&m.image, image) {
            return Err(DomainError::Validation(format!(
                "{} already stages {}; choose another staging folder for {}",
                root.display(),
                m.image.display(),
                image.display()
            )));
        }
        // A reused staging is validated as a whole, not by the image's pathname alone (R2 F2):
        // the title/platform it was staged for, its staged files, its metadata copy and the
        // image's length must all still agree with what is being opened.
        if m.game_id != game_id || m.platform != platform {
            return Err(DomainError::Validation(format!(
                "{} was staged for {} on {}, not {game_id} on {platform}; choose another staging folder",
                root.display(),
                m.game_id,
                m.platform
            )));
        }
        for f in &m.files {
            let p = root.join(&f.staged);
            if !p.is_file() {
                return Err(DomainError::Validation(format!(
                    "{} is missing its staged {} ({}); the staging folder is incomplete — stage into a new folder",
                    root.display(),
                    f.role,
                    p.display()
                )));
            }
        }
        if !meta_dst.is_file() {
            return Err(DomainError::Validation(format!(
                "{} is missing its metadata copy {}; stage into a new folder",
                root.display(),
                meta_dst.display()
            )));
        }
        if meta_xml.is_file() {
            let given = FileIdentity::of_file(meta_xml).map_err(|e| io_err(meta_xml, e))?;
            let held = FileIdentity::of_file(&meta_dst).map_err(|e| io_err(&meta_dst, e))?;
            if given != held {
                return Err(DomainError::Validation(format!(
                    "{} holds metadata {held} but {} is {given}; a staging folder is bound to the metadata it was opened with",
                    root.display(),
                    meta_xml.display()
                )));
            }
        }
        let image_len = fs::metadata(image).map_err(|e| io_err(image, e))?.len();
        if image_len != m.image_len {
            return Err(DomainError::Validation(format!(
                "{} is {image_len} bytes now but was {} bytes when staged; the image changed — stage it again into a new folder",
                image.display(),
                m.image_len
            )));
        }
        let has_dict = m.files.iter().any(|f| f.role == "dictionary");
        return Ok(Staged {
            root: root.to_path_buf(),
            manifest: m,
            db_path,
            meta_xml: meta_dst,
            dictionary_path: has_dict.then_some(dict_path),
            reused: true,
        });
    }
    if root.exists() && fs::read_dir(root).map(|mut d| d.next().is_some()).unwrap_or(true) {
        return Err(DomainError::Validation(format!(
            "{} exists and is not a staging folder for this image; choose an empty or new folder",
            root.display()
        )));
    }
    if !meta_xml.is_file() {
        return Err(DomainError::Validation(format!(
            "metadata {} is not a file",
            meta_xml.display()
        )));
    }
    let found = xiso_source::locate_databases(image)?;
    let Some(nhlng) = found.nhlng.as_ref() else {
        return Err(DomainError::Validation(format!(
            "{} carries no {} (loose or in a root archive); probed: {}",
            image.display(),
            xiso_source::DB_MEMBER,
            found
                .probed
                .iter()
                .map(|p| format!("{} = {}", p.path, p.verdict))
                .collect::<Vec<_>>()
                .join("; ")
        )));
    };
    let db_bytes = xiso_source::read_located(image, nhlng)?;
    // The bytes must be a parseable database before anything is staged.
    tdb_core::TdbFile::parse(&db_bytes)?;
    let dict = match found.dictionary.as_ref() {
        Some(l) => {
            let bytes = xiso_source::read_located(image, l)?;
            tdb_loc::verify(&bytes)
                .map_err(|e| DomainError::Validation(format!("{}: {e}", xiso_source::DICT_MEMBER)))?;
            Some((l.clone(), bytes))
        }
        None => None,
    };
    fs::create_dir_all(root.join("db")).map_err(|e| io_err(root, e))?;
    fs::write(&db_path, &db_bytes).map_err(|e| io_err(&db_path, e))?;
    fs::copy(meta_xml, &meta_dst).map_err(|e| io_err(&meta_dst, e))?;
    let mut files = vec![StagedFile {
        role: "nhlng".into(),
        staged: "db/nhlng.db".into(),
        located: LocatedRecord::from_located(nhlng),
        prior: FileIdentity::of_bytes(&db_bytes),
    }];
    if let Some((l, bytes)) = &dict {
        fs::create_dir_all(root.join("fe").join("loc")).map_err(|e| io_err(root, e))?;
        fs::write(&dict_path, bytes).map_err(|e| io_err(&dict_path, e))?;
        files.push(StagedFile {
            role: "dictionary".into(),
            staged: "fe/loc/nhl_eng_us.db".into(),
            located: LocatedRecord::from_located(l),
            prior: FileIdentity::of_bytes(bytes),
        });
    }
    let image_len = fs::metadata(image).map_err(|e| io_err(image, e))?.len();
    let manifest = StagingManifest {
        schema: STAGING_SCHEMA.into(),
        image: image.to_path_buf(),
        image_len,
        layout: found.layout.to_string(),
        game_id: game_id.to_string(),
        platform: platform.to_string(),
        files,
        staged_at: now_iso8601(),
    };
    write_manifest(root, &manifest)?;
    Ok(Staged {
        root: root.to_path_buf(),
        manifest,
        db_path,
        meta_xml: meta_dst,
        dictionary_path: dict.is_some().then_some(dict_path),
        reused: false,
    })
}

fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Civil date from days since epoch (Howard Hinnant's algorithm); UTC.
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
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
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// One planned file of an apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    pub role: String,
    pub staged: String,
    /// `None` when the staged file still carries its prior identity (nothing to write).
    pub patch: Option<RangePatch>,
    pub staged_identity: FileIdentity,
}

/// Plan the apply: which staged files differ from their prior and the exact range each would
/// write. Every planned patch has been verified against the image as it is now; nothing written.
pub fn plan_apply(root: &Path) -> Result<(StagingManifest, Vec<PlannedFile>)> {
    let m = read_manifest(root)?;
    let mut out = Vec::new();
    for f in &m.files {
        let p = root.join(&f.staged);
        let bytes = fs::read(&p).map_err(|e| io_err(&p, e))?;
        let staged_identity = FileIdentity::of_bytes(&bytes);
        let patch = if staged_identity == f.prior {
            None
        } else {
            Some(iso_patch::plan(&m.image, &f.located.to_located(), &f.prior, bytes)?)
        };
        out.push(PlannedFile {
            role: f.role.clone(),
            staged: f.staged.clone(),
            patch,
            staged_identity,
        });
    }
    Ok((m, out))
}

/// The result of one committed apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// `<staging>/records/<n>`.
    pub records_dir: PathBuf,
    pub records: Vec<PatchRecord>,
    pub unchanged: Vec<String>,
}

/// One attempted range after a failed apply: what the image holds there now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptedRange {
    pub role: String,
    pub entry_path: String,
    pub record: PatchRecord,
    /// `prior` / `replacement` / `partial` / `unknown` — after the rollback.
    pub state: RangeState,
    /// Why this range could not be put back, when it could not.
    pub restore_error: Option<String>,
}

/// What an apply left behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// Every planned range written and verified.
    Committed(Applied),
    /// A range failed; every attempted range (the failing one included) verified back to its
    /// prior bytes. The image is as it was; the records are kept for inspection.
    RolledBack {
        records_dir: PathBuf,
        failed: String,
        cause: String,
        attempted: Vec<AttemptedRange>,
    },
    /// A range failed and at least one attempted range could not be verified back. Nothing is
    /// hidden: `attempted` names the state of every range and the record that holds its backup.
    Split {
        records_dir: PathBuf,
        failed: String,
        cause: String,
        attempted: Vec<AttemptedRange>,
    },
}

impl ApplyOutcome {
    /// `true` only when every attempted range verified back to its prior bytes.
    pub fn nothing_changed(&self) -> bool {
        matches!(self, ApplyOutcome::RolledBack { .. })
    }

    pub fn records_dir(&self) -> &Path {
        match self {
            ApplyOutcome::Committed(a) => &a.records_dir,
            ApplyOutcome::RolledBack { records_dir, .. } | ApplyOutcome::Split { records_dir, .. } => records_dir,
        }
    }

    /// Plain-language account of a failed apply.
    pub fn describe(&self) -> String {
        let list = |v: &[AttemptedRange], f: &dyn Fn(&AttemptedRange) -> bool| {
            let names: Vec<String> = v.iter().filter(|a| f(a)).map(|a| a.entry_path.clone()).collect();
            if names.is_empty() {
                "none".to_string()
            } else {
                names.join(", ")
            }
        };
        match self {
            ApplyOutcome::Committed(a) => format!("{} range(s) written and verified", a.records.len()),
            ApplyOutcome::RolledBack {
                failed, cause, attempted, ..
            } => format!(
                "writing {failed} to the image failed: {cause}. Every attempted range was put back from its backup and verified ({}), so the image is as it was",
                list(attempted, &|_| true)
            ),
            ApplyOutcome::Split {
                failed,
                cause,
                attempted,
                records_dir,
            } => format!(
                "writing {failed} to the image failed: {cause}, and putting the attempted ranges back did not complete. State on the image now — holding the NEW bytes: {}; put back to the ORIGINAL bytes: {}; interrupted (recoverable from the record): {}; unknown bytes (not overwritten): {}. The records under {} hold the prior bytes; use Restore once the image is reachable again",
                list(attempted, &|a| a.state == RangeState::Replacement),
                list(attempted, &|a| a.state == RangeState::Prior),
                list(attempted, &|a| a.state == RangeState::Partial),
                list(attempted, &|a| matches!(a.state, RangeState::Unknown | RangeState::Mixed)),
                records_dir.display()
            ),
        }
    }
}

fn next_records_dir(root: &Path) -> Result<PathBuf> {
    let base = root.join(RECORDS_DIR);
    fs::create_dir_all(&base).map_err(|e| io_err(&base, e))?;
    let mut n = 1u32;
    loop {
        let d = base.join(format!("{n:04}"));
        if !d.exists() {
            fs::create_dir(&d).map_err(|e| io_err(&d, e))?;
            return Ok(d);
        }
        n += 1;
    }
}

/// Plan now and apply that plan, with a fresh records directory (the convenience used by the
/// domain tests; the Studio route applies the plan it reviewed through [`apply_planned`]).
/// **Writes to the image**: a live image is a user-gated action.
pub fn apply(root: &Path) -> Result<ApplyOutcome> {
    let (_, planned) = plan_apply(root)?;
    let records_dir = next_records_dir(root)?;
    let out = apply_planned(&planned, &records_dir, &mut iso_patch::NoHooks);
    if out.is_err() && fs::read_dir(&records_dir).map(|mut d| d.next().is_none()).unwrap_or(false) {
        let _ = fs::remove_dir(&records_dir);
    }
    out
}

/// A fresh `<root>/records/<n>` directory for one apply.
pub fn new_records_dir(root: &Path) -> Result<PathBuf> {
    next_records_dir(root)
}

/// Apply exactly `planned` (the reviewed plan, replacement bytes included) into `records_dir`
/// (existing, empty). `Err` only before any image byte changed (nothing to write); every later
/// failure is an [`ApplyOutcome`] with the records of every attempted range.
pub fn apply_planned(
    planned: &[PlannedFile],
    records_dir: &Path,
    hooks: &mut dyn iso_patch::Hooks,
) -> Result<ApplyOutcome> {
    let unchanged: Vec<String> = planned
        .iter()
        .filter(|p| p.patch.is_none())
        .map(|p| p.role.clone())
        .collect();
    let todo: Vec<&PlannedFile> = planned.iter().filter(|p| p.patch.is_some()).collect();
    if todo.is_empty() {
        return Err(DomainError::Validation(
            "nothing to write: every staged file still carries the identity it had on the image"
                .into(),
        ));
    }
    let mut done: Vec<(String, PatchRecord)> = Vec::new();
    for p in todo {
        let patch = p.patch.as_ref().expect("filtered");
        let dir = records_dir.join(&p.role);
        // The failing range, when it got as far as a record, is the first member of the
        // recovery set; the earlier ranges follow, newest first.
        let (cause, mut attempted) = match iso_patch::apply_with(patch, &dir, hooks) {
            ApplyResult::Applied(r) => {
                done.push((p.role.clone(), r));
                continue;
            }
            ApplyResult::Refused(e) => (e.to_string(), Vec::new()),
            ApplyResult::Failed {
                record,
                cause,
                state,
                restore_error,
            } => (
                cause,
                vec![AttemptedRange {
                    role: p.role.clone(),
                    entry_path: record.entry_path.clone(),
                    record,
                    state,
                    restore_error,
                }],
            ),
        };
        for (role, r) in done.iter().rev() {
            let (state, restore_error) = match iso_patch::restore(r) {
                Ok(_) => (RangeState::Prior, None),
                Err(re) => (
                    iso_patch::state(r).unwrap_or(RangeState::Unknown),
                    Some(re.to_string()),
                ),
            };
            attempted.push(AttemptedRange {
                role: role.clone(),
                entry_path: r.entry_path.clone(),
                record: iso_patch::read_record(&r.dir).unwrap_or_else(|_| r.clone()),
                state,
                restore_error,
            });
        }
        let all_prior = attempted.iter().all(|a| a.state == RangeState::Prior);
        return Ok(if all_prior {
            ApplyOutcome::RolledBack {
                records_dir: records_dir.to_path_buf(),
                failed: p.staged.clone(),
                cause,
                attempted,
            }
        } else {
            ApplyOutcome::Split {
                records_dir: records_dir.to_path_buf(),
                failed: p.staged.clone(),
                cause,
                attempted,
            }
        });
    }
    Ok(ApplyOutcome::Committed(Applied {
        records_dir: records_dir.to_path_buf(),
        records: done.into_iter().map(|(_, r)| r).collect(),
        unchanged,
    }))
}

/// Check that `records_dir` is one apply's directory under `<root>/records/` and that every
/// record in it names this staging's image and one of its staged ranges — a restore only ever
/// writes the image the manifest names, at a range the manifest recorded.
pub fn validate_records_dir(root: &Path, records_dir: &Path) -> Result<StagingManifest> {
    let m = read_manifest(root)?;
    let base = root.join(RECORDS_DIR);
    let canon_base = fs::canonicalize(&base).map_err(|e| io_err(&base, e))?;
    let canon_dir = fs::canonicalize(records_dir).map_err(|e| io_err(records_dir, e))?;
    if canon_dir.parent() != Some(canon_base.as_path()) {
        return Err(DomainError::Validation(format!(
            "{} is not an apply directory under {}; a restore only takes records this staging wrote",
            records_dir.display(),
            base.display()
        )));
    }
    for d in record_dirs(records_dir)? {
        let r = iso_patch::read_record(&d)?;
        if !same_file(&r.image, &m.image) {
            return Err(DomainError::Validation(format!(
                "record {} names image {}, not this staging's {}; refusing",
                d.display(),
                r.image.display(),
                m.image.display()
            )));
        }
        let known = m.files.iter().any(|f| {
            let l = f.located.to_located();
            let (member, entry, len) = match &l {
                Located::Loose { member } => (member.path.as_str(), member.path.as_str(), member.size as u64),
                Located::InArchive {
                    archive,
                    entry_path,
                    stored_size,
                    ..
                } => (archive.path.as_str(), entry_path.as_str(), *stored_size as u64),
            };
            member == r.member_path && entry == r.entry_path && r.end - r.start == len
        });
        if !known {
            return Err(DomainError::Validation(format!(
                "record {} names {} › {} [{}, {}), which this staging never located; refusing",
                d.display(),
                r.member_path,
                r.entry_path,
                r.start,
                r.end
            )));
        }
    }
    Ok(m)
}

fn record_dirs(records_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(records_dir)
        .map_err(|e| io_err(records_dir, e))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("record.json").is_file())
        .collect();
    dirs.sort();
    Ok(dirs)
}

/// Restore one apply's records, newest first. Each is idempotent; an unknown range refuses and
/// the others are still attempted, with every outcome reported.
pub fn restore(records_dir: &Path) -> Result<Vec<(String, std::result::Result<RestoreOutcome, String>)>> {
    let mut dirs = record_dirs(records_dir)?;
    dirs.reverse();
    if dirs.is_empty() {
        return Err(DomainError::Validation(format!(
            "{} holds no patch records",
            records_dir.display()
        )));
    }
    let mut out = Vec::new();
    for d in dirs {
        let r = iso_patch::read_record(&d)?;
        let name = r.entry_path.clone();
        out.push((name, iso_patch::restore(&r).map_err(|e| e.to_string())));
    }
    Ok(out)
}

/// Status of every record under a staging folder (all applies), newest apply first.
pub fn records(root: &Path) -> Result<Vec<(PathBuf, PatchRecord, &'static str)>> {
    let base = root.join(RECORDS_DIR);
    let mut applies: Vec<PathBuf> = match fs::read_dir(&base) {
        Ok(d) => d.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect(),
        Err(_) => return Ok(Vec::new()),
    };
    applies.sort();
    applies.reverse();
    let mut out = Vec::new();
    for a in applies {
        let mut recs: Vec<PathBuf> = fs::read_dir(&a)
            .map_err(|e| io_err(&a, e))?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.join("record.json").is_file())
            .collect();
        recs.sort();
        for d in recs {
            let r = iso_patch::read_record(&d)?;
            let s = iso_patch::status(&r)?;
            out.push((a.clone(), r, s));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tdb_xiso::synth::{image, SynthFile};
    use tdb_xiso::Layout;

    fn repo(rel: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
    }

    fn tmp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nhlms-isostage-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A synthetic image carrying the real NHL Legacy anchor database and dictionary inside a
    /// BIGF root archive (packed, like the real images) plus a loose default.xex.
    fn legacy_image(dir: &Path) -> (PathBuf, Vec<u8>, Vec<u8>) {
        let db = fs::read(repo("anchors/xbox360/nhllegacy/databases/nhlng.db")).unwrap();
        let dict = fs::read(repo("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db")).unwrap();
        let mut b = tdb_archive_big::ArchiveBuilder::new();
        b.add_entry("fe/loc/nhl_eng_us.db", dict.clone());
        b.add_entry("db/nhlng.db", db.clone());
        let big = b.build().unwrap();
        let files = [
            SynthFile {
                path: "cacheboot.big",
                bytes: &big,
            },
            SynthFile {
                path: "default.xex",
                bytes: b"XEX2",
            },
        ];
        let p = dir.join("legacy.iso");
        fs::write(&p, image(Layout::Xgd3, &files, None)).unwrap();
        (p, db, dict)
    }

    #[test]
    fn stage_open_edit_save_apply_restore_on_a_packed_synthetic_image() {
        let dir = tmp("flow");
        let (iso, db, dict) = legacy_image(&dir);
        let before = fs::read(&iso).unwrap();
        let meta = repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml");
        let root = dir.join("staging");
        let staged = stage(&iso, "nhllegacy", "xbox", &meta, &root).unwrap();
        assert!(!staged.reused);
        assert_eq!(fs::read(&staged.db_path).unwrap(), db);
        assert_eq!(fs::read(staged.dictionary_path.as_ref().unwrap()).unwrap(), dict);
        assert!(staged.meta_xml.is_file());
        assert_eq!(staged.manifest.files.len(), 2);
        assert_eq!(staged.manifest.files[0].prior, FileIdentity::of_bytes(&db));
        assert!(staged.manifest.files[1].located.entry.is_some(), "packed entry recorded");

        // Re-staging the same image reuses the folder; another image is refused.
        assert!(stage(&iso, "nhllegacy", "xbox", &meta, &root).unwrap().reused);
        let other = dir.join("other.iso");
        fs::copy(&iso, &other).unwrap();
        let err = stage(&other, "nhllegacy", "xbox", &meta, &root).unwrap_err().to_string();
        assert!(err.contains("already stages"), "{err}");

        // Nothing changed: apply refuses plainly.
        let (_, planned) = plan_apply(&root).unwrap();
        assert!(planned.iter().all(|p| p.patch.is_none()));
        let err = apply(&root).unwrap_err().to_string();
        assert!(err.contains("nothing to write"), "{err}");

        // Edit through the real workspace: a DB field + the dictionary re-point, saved in place
        // to the staging folder (never the image).
        let session = crate::EditSession::begin(
            crate::Project::load(
                &staged.db_path,
                crate::Platform::Xbox,
                crate::ProjectRole::Nhlng,
                &staged.meta_xml,
                repo("defs"),
            )
            .unwrap(),
        );
        let mut ws = crate::Workspace::single(session, staged.db_path.to_string_lossy().into_owned());
        ws.attach_dictionary(staged.dictionary_path.as_ref().unwrap().to_str().unwrap())
            .unwrap();
        ws.set_field("kOtt", 0, "xsnn", crate::FieldValue::Text("ZZZ".into()))
            .unwrap();
        ws.set_dictionary_repoint("TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE")
            .unwrap();
        assert_eq!(ws.save_in_place().unwrap(), 2);
        assert_eq!(fs::read(&iso).unwrap(), before, "a Studio save never touches the image");

        let (_, planned) = plan_apply(&root).unwrap();
        assert!(planned.iter().all(|p| p.patch.is_some()), "both files changed");
        let applied = match apply(&root).unwrap() {
            ApplyOutcome::Committed(a) => a,
            other => panic!("{other:?}"),
        };
        assert_eq!(applied.records.len(), 2);
        assert!(applied.unchanged.is_empty());
        let after = fs::read(&iso).unwrap();
        assert_eq!(after.len(), before.len());
        // The image now carries exactly the staged bytes at the two ranges and nothing else moved.
        let found = xiso_source::locate_databases(&iso).unwrap();
        assert_eq!(
            xiso_source::read_located(&iso, found.nhlng.as_ref().unwrap()).unwrap(),
            fs::read(&staged.db_path).unwrap()
        );
        let new_dict = xiso_source::read_located(&iso, found.dictionary.as_ref().unwrap()).unwrap();
        assert_eq!(
            tdb_loc::LocDictionary::parse(&new_dict).unwrap().text("TXT_LEAGUE_NORWAY"),
            Some("Rest Of World")
        );
        let differing = after.iter().zip(&before).filter(|(a, b)| a != b).count();
        assert!(differing > 0 && differing < 64, "a field edit + a 4-byte re-point + CRCs: {differing} bytes differ");
        let st = records(&root).unwrap();
        assert_eq!(st.len(), 2);
        assert!(st.iter().all(|(_, _, s)| *s == "replacement"));

        // Restore: byte-identical image; idempotent.
        let outcomes = restore(&applied.records_dir).unwrap();
        assert!(outcomes.iter().all(|(_, o)| matches!(o, Ok(RestoreOutcome::Restored))));
        assert_eq!(fs::read(&iso).unwrap(), before);
        let outcomes = restore(&applied.records_dir).unwrap();
        assert!(outcomes.iter().all(|(_, o)| matches!(o, Ok(RestoreOutcome::AlreadyPrior))));
        // The staged edits are still there: a second apply mints a new records directory.
        let applied2 = apply(&root).unwrap();
        assert_ne!(applied2.records_dir(), applied.records_dir);
        validate_records_dir(&root, applied2.records_dir()).unwrap();
        restore(applied2.records_dir()).unwrap();
        assert_eq!(fs::read(&iso).unwrap(), before);
        // A records directory from elsewhere is refused before anything is read back.
        let foreign = dir.join("elsewhere");
        fs::create_dir_all(&foreign).unwrap();
        let err = validate_records_dir(&root, &foreign).unwrap_err().to_string();
        assert!(err.contains("not an apply directory"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    /// R2 F2: a reused staging is validated as a whole, not by the image path alone.
    #[test]
    fn r2_f2_reusing_a_staging_validates_title_platform_metadata_files_and_image_length() {
        let dir = tmp("reuse");
        let (iso, _, _) = legacy_image(&dir);
        let meta = repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml");
        let root = dir.join("staging");
        stage(&iso, "nhllegacy", "xbox", &meta, &root).unwrap();
        let err = stage(&iso, "nhl15", "xbox", &meta, &root).unwrap_err().to_string();
        assert!(err.contains("was staged for nhllegacy on xbox"), "{err}");
        let err = stage(&iso, "nhllegacy", "ps3", &meta, &root).unwrap_err().to_string();
        assert!(err.contains("was staged for nhllegacy on xbox"), "{err}");
        let other_meta = repo("anchors/xbox360/nhl15/databases/nhlng-meta.xml");
        let err = stage(&iso, "nhllegacy", "xbox", &other_meta, &root).unwrap_err().to_string();
        assert!(err.contains("bound to the metadata"), "{err}");
        assert!(stage(&iso, "nhllegacy", "xbox", &meta, &root).unwrap().reused);
        let staged_dict = root.join("fe").join("loc").join("nhl_eng_us.db");
        let keep = fs::read(&staged_dict).unwrap();
        fs::remove_file(&staged_dict).unwrap();
        let err = stage(&iso, "nhllegacy", "xbox", &meta, &root).unwrap_err().to_string();
        assert!(err.contains("missing its staged dictionary"), "{err}");
        fs::write(&staged_dict, keep).unwrap();
        let mut grown = fs::read(&iso).unwrap();
        grown.push(0);
        fs::write(&iso, grown).unwrap();
        let err = stage(&iso, "nhllegacy", "xbox", &meta, &root).unwrap_err().to_string();
        assert!(err.contains("the image changed"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    /// R2 F1: a fault on the **second** range (after the first was written and verified) —
    /// at its range write, at its flush, and an interrupted (partial) write — puts both ranges
    /// back, classifies both, and the image is byte-identical; the records survive.
    #[test]
    fn r2_f1_a_second_range_failure_rolls_back_both_ranges_and_keeps_both_records() {
        let dir = tmp("second");
        let (iso, db, dict) = legacy_image(&dir);
        let before = fs::read(&iso).unwrap();
        let meta = repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml");
        let root = dir.join("staging");
        let staged = stage(&iso, "nhllegacy", "xbox", &meta, &root).unwrap();
        let mut edited = db.clone();
        edited[4096] ^= 1;
        fs::write(&staged.db_path, &edited).unwrap();
        let dict_path = staged.dictionary_path.clone().unwrap();
        let mut d2 = dict.clone();
        d2[dict.len() - 1] ^= 1;
        fs::write(&dict_path, &d2).unwrap();

        struct FailAt(iso_patch::Event);
        impl iso_patch::Hooks for FailAt {
            fn event(&mut self, e: &iso_patch::Event) -> std::io::Result<()> {
                if *e == self.0 {
                    Err(std::io::Error::other("injected"))
                } else {
                    Ok(())
                }
            }
        }
        struct TruncateDict;
        impl iso_patch::Hooks for TruncateDict {
            fn truncate_range_write(&mut self, entry: &str) -> Option<usize> {
                (entry == "fe/loc/nhl_eng_us.db").then_some(100_000)
            }
        }
        let entry = "fe/loc/nhl_eng_us.db".to_string();
        let mut hooks: Vec<(&str, Box<dyn iso_patch::Hooks>)> = vec![
            ("range write", Box::new(FailAt(iso_patch::Event::BeforeRangeWrite(entry.clone())))),
            ("flush", Box::new(FailAt(iso_patch::Event::BeforeFlush(entry.clone())))),
            ("final record", Box::new(FailAt(iso_patch::Event::BeforeFinalRecord(entry.clone())))),
            ("partial", Box::new(TruncateDict)),
        ];
        for (name, h) in hooks.iter_mut() {
            let (_, planned) = plan_apply(&root).unwrap();
            let records_dir = new_records_dir(&root).unwrap();
            let out = apply_planned(&planned, &records_dir, h.as_mut()).unwrap();
            match &out {
                ApplyOutcome::RolledBack {
                    failed, attempted, ..
                } => {
                    assert_eq!(failed, "fe/loc/nhl_eng_us.db", "{name}");
                    assert_eq!(attempted.len(), 2, "{name}: the failing range and the earlier one");
                    assert_eq!(attempted[0].entry_path, "fe/loc/nhl_eng_us.db");
                    assert_eq!(attempted[1].entry_path, "db/nhlng.db");
                    assert!(attempted.iter().all(|a| a.state == RangeState::Prior), "{name}");
                    assert!(attempted.iter().all(|a| a.record.status == iso_patch::RecordStatus::Restored), "{name}");
                }
                other => panic!("{name}: {other:?}"),
            }
            assert!(out.nothing_changed());
            assert_eq!(fs::read(&iso).unwrap(), before, "{name}: byte-identical");
            let text = out.describe();
            assert!(text.contains("the image is as it was"), "{name}: {text}");
            // Both records are on disk, listed with the range state, for a failed apply too.
            let st = records(&root).unwrap();
            assert!(st.iter().filter(|(a, _, _)| *a == records_dir).count() == 2, "{name}");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_second_patch_failure_restores_the_first() {
        let dir = tmp("coord");
        let (iso, db, _) = legacy_image(&dir);
        let before = fs::read(&iso).unwrap();
        let meta = repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml");
        let root = dir.join("staging");
        let staged = stage(&iso, "nhllegacy", "xbox", &meta, &root).unwrap();
        // Change both staged files; make the dictionary's staged copy the wrong length so its
        // patch cannot be planned — the apply must then leave the image untouched.
        let mut edited = db.clone();
        edited[4096] ^= 1;
        fs::write(&staged.db_path, &edited).unwrap();
        let dict_path = staged.dictionary_path.clone().unwrap();
        let mut d = fs::read(&dict_path).unwrap();
        d.push(0);
        fs::write(&dict_path, &d).unwrap();
        let err = apply(&root).unwrap_err().to_string();
        assert!(err.contains("cannot change length"), "{err}");
        assert_eq!(fs::read(&iso).unwrap(), before, "planning failed before any write");

        // Now a plan that passes but whose second write is obstructed: the DB patch applies, the
        // dictionary's record directory is pre-created so its apply refuses, and the DB is put back.
        let good = fs::read(repo("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db")).unwrap();
        let mut d2 = good.clone();
        d2[good.len() - 1] ^= 1;
        fs::write(&dict_path, &d2).unwrap();
        let records = root.join(RECORDS_DIR).join("obstructed");
        fs::create_dir_all(records.join("dictionary")).unwrap();
        let (_, planned) = plan_apply(&root).unwrap();
        let out = apply_planned(&planned, &records, &mut iso_patch::NoHooks).unwrap();
        match &out {
            ApplyOutcome::RolledBack { cause, attempted, .. } => {
                assert!(cause.contains("must not exist"), "{cause}");
                assert_eq!(attempted.len(), 1, "the refused range never got a record; the earlier one is put back");
                assert_eq!(attempted[0].entry_path, "db/nhlng.db");
            }
            other => panic!("{other:?}"),
        }
        assert!(out.describe().contains("the image is as it was"));
        assert_eq!(fs::read(&iso).unwrap(), before, "the first patch was rolled back");
        let _ = fs::remove_dir_all(&dir);
    }
}
