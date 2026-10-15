//! Bundled baseline inventory (ONB-002 §2, §4): the versioned, hashed list of the approved anchor
//! files a fresh project may start from, and the store that copies them out with verification.
//!
//! The inventory (`INVENTORY.json`, schema [`INVENTORY_SCHEMA`]) is generated from `anchors/` by
//! `cargo run -p studio-api --bin baseline-inventory`; the approval table (which anchor is admitted
//! for which shipping target, and with what provenance) lives in that generator's source so it is
//! reviewed as code. At runtime the inventory and its files are read beside the executable
//! (`baselines/INVENTORY.json`, `baselines/files/<platform>/<game>/…`) or, in a development run,
//! from the repository's `baselines/INVENTORY.json` with the files under `anchors/`. Both layouts
//! use the same relative component paths.
//!
//! **File presence, identity, vanilla provenance, readable format and usable save support are
//! separate claims** — each component carries its hash, its provenance status and (for saves) the
//! codec probe, and each target lists its explicit gaps. A target whose main database or metadata
//! is not admitted is `unavailable`; one that is missing an optional component (dictionary, roster
//! save) is `partial` and says what is missing. Nothing is substituted from a neighbouring title or
//! platform.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tdb_txn::FileIdentity;

use crate::{DomainError, Platform, Result};

/// The inventory schema this build reads and writes.
pub const INVENTORY_SCHEMA: &str = "nhlms-baselines/1";
/// The inventory's file name, under the baselines directory.
pub const INVENTORY_FILE: &str = "INVENTORY.json";
/// The bundled files directory, beside the inventory, in the packaged layout.
pub const BUNDLED_FILES_DIR: &str = "files";

/// Component roles.
pub const ROLE_MAIN_DB: &str = "main_db";
pub const ROLE_META: &str = "meta";
pub const ROLE_DICTIONARY: &str = "dictionary";
pub const ROLE_DICTIONARY_META: &str = "dictionary_meta";
/// The roster save's data file (`SYS-DATA`, the Xbox container file, PS4 `DATA`).
pub const ROLE_ROSTER_SAVE: &str = "roster_save";
/// The other files of the roster save's container directory (`PARAM.SFO`, icons, `sce_sys/…`).
pub const ROLE_ROSTER_SAVE_AUX: &str = "roster_save_aux";

/// Target availability.
pub const STATUS_AVAILABLE: &str = "available";
pub const STATUS_PARTIAL: &str = "partial";
pub const STATUS_UNAVAILABLE: &str = "unavailable";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inventory {
    pub schema: String,
    pub generated_from: GeneratedFrom,
    pub targets: Vec<TargetBaseline>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneratedFrom {
    /// The repository commit the anchors were hashed at (working tree may carry more).
    pub head: String,
    pub generator: String,
    /// The audited anchors manifest the hashes were cross-checked against.
    pub anchors_manifest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TargetBaseline {
    pub game_id: String,
    pub platform: String,
    pub title: String,
    /// `available` | `partial` | `unavailable`.
    pub status: String,
    pub components: Vec<Component>,
    pub gaps: Vec<Gap>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Component {
    pub role: String,
    /// Relative to the files root, forward slashes (`ps3/nhl14/databases/nhlng.db`).
    pub path: String,
    pub file_name: String,
    /// For save components: the container directory's name (`BLUS311790200 - ROSTER`).
    #[serde(default)]
    pub container_dir: Option<String>,
    /// For save components: the file's path inside the container directory (`sce_sys/param.sfo`).
    #[serde(default)]
    pub container_relative: Option<String>,
    pub len: u64,
    pub sha256: String,
    /// `verified-vanilla` | `cross-platform-schema` | `derived` (see the module docs).
    pub provenance: String,
    pub witness: String,
    #[serde(default)]
    pub codec: Option<SaveCodec>,
}

/// What `tdb-savedata` says about a save file's bytes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaveCodec {
    /// `v1` | `v2` | `v3` | `v4` | `ps4` | `unrecognised`.
    pub container_version: String,
    /// `roster` | `settings` | … (from the magic), `unknown` when unreadable.
    pub kind: String,
    pub readable: bool,
    pub writable: bool,
    pub experimental: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gap {
    pub component: String,
    pub reason: String,
}

impl TargetBaseline {
    pub fn component(&self, role: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.role == role)
    }
    /// The save components (data file first, then the aux files), if a roster save is admitted.
    pub fn roster_save_components(&self) -> Vec<&Component> {
        let mut out: Vec<&Component> = self
            .components
            .iter()
            .filter(|c| c.role == ROLE_ROSTER_SAVE || c.role == ROLE_ROSTER_SAVE_AUX)
            .collect();
        out.sort_by_key(|c| (c.role != ROLE_ROSTER_SAVE, c.path.clone()));
        out
    }
    pub fn is_available(&self) -> bool {
        self.status != STATUS_UNAVAILABLE
    }
}

/// Probe a save file's bytes with the same codecs the Studio edits through.
pub fn probe_save_codec(bytes: &[u8]) -> SaveCodec {
    if tdb_savedata::is_ps4_save(bytes) {
        let readable = tdb_savedata::ps4_extract_roster(bytes).is_ok();
        return SaveCodec {
            container_version: "ps4".into(),
            kind: if readable { "roster".into() } else { "unknown".into() },
            readable,
            // The PS4 repack is structurally checked only; on-console load is unverified.
            writable: readable,
            experimental: true,
        };
    }
    match tdb_savedata::SaveContainer::parse(bytes) {
        Ok(c) => SaveCodec {
            container_version: match c.header.version {
                tdb_savedata::ContainerVersion::RawV3 => "v3",
                tdb_savedata::ContainerVersion::RawV1 => "v1",
                tdb_savedata::ContainerVersion::RawV2 => "v2",
                tdb_savedata::ContainerVersion::ZlibV4 => "v4",
            }
            .into(),
            kind: match c.header.save_type {
                tdb_savedata::SaveType::Roster => "roster",
                tdb_savedata::SaveType::Settings => "settings",
                tdb_savedata::SaveType::BeAGm => "beagm",
                tdb_savedata::SaveType::BeAPro => "beapro",
                tdb_savedata::SaveType::HockeyCard => "hockeycard",
                tdb_savedata::SaveType::Season => "season",
                tdb_savedata::SaveType::Unknown => "unknown",
            }
            .into(),
            readable: c.extract_db().is_ok(),
            writable: c.writeable(),
            experimental: c.header.version.write_is_experimental(),
        },
        Err(_) => SaveCodec {
            container_version: "unrecognised".into(),
            kind: "unknown".into(),
            readable: false,
            writable: false,
            experimental: false,
        },
    }
}

/// SHA-256 of a byte slice, lower-case hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/// The baseline store: an inventory plus the directory its component paths are relative to.
#[derive(Clone, Debug)]
pub struct BaselineStore {
    pub inventory: Inventory,
    pub inventory_path: PathBuf,
    pub files_root: PathBuf,
}

impl BaselineStore {
    /// Open an inventory and bind it to a files root. The inventory's schema must be the one this
    /// build reads.
    pub fn open(inventory_path: &Path, files_root: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(inventory_path).map_err(|source| DomainError::Io {
            path: inventory_path.display().to_string(),
            source,
        })?;
        let inventory: Inventory = serde_json::from_str(&text).map_err(|e| {
            DomainError::Validation(format!(
                "the baseline inventory {} is not readable: {e}",
                inventory_path.display()
            ))
        })?;
        if inventory.schema != INVENTORY_SCHEMA {
            return Err(DomainError::Validation(format!(
                "the baseline inventory {} has schema {:?}; this build reads {INVENTORY_SCHEMA:?}",
                inventory_path.display(),
                inventory.schema
            )));
        }
        Ok(BaselineStore {
            inventory,
            inventory_path: inventory_path.to_path_buf(),
            files_root: files_root.to_path_buf(),
        })
    }

    /// The packaged layout: `<dir>/INVENTORY.json` with files under `<dir>/files/`. `None` when the
    /// directory does not hold an inventory.
    pub fn open_bundled(dir: &Path) -> Option<Result<Self>> {
        let inv = dir.join(INVENTORY_FILE);
        inv.is_file().then(|| Self::open(&inv, &dir.join(BUNDLED_FILES_DIR)))
    }

    pub fn target(&self, game_id: &str, platform: Platform) -> Option<&TargetBaseline> {
        self.inventory
            .targets
            .iter()
            .find(|t| t.game_id == game_id && t.platform == platform.as_str())
    }

    /// The absolute path of a component's file in this store.
    pub fn component_path(&self, c: &Component) -> PathBuf {
        let mut p = self.files_root.clone();
        for part in c.path.split('/') {
            p.push(part);
        }
        p
    }

    /// Read a component and verify it is exactly the inventoried file (length and SHA-256).
    pub fn read_verified(&self, c: &Component) -> Result<Vec<u8>> {
        let path = self.component_path(c);
        let bytes = std::fs::read(&path).map_err(|source| DomainError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let identity = FileIdentity::of_bytes(&bytes);
        if identity.len != c.len || identity.sha256 != c.sha256 {
            return Err(DomainError::Validation(format!(
                "the bundled baseline file {} is {identity}, but the inventory lists {} bytes / {}; the bundle is damaged or substituted and is not used",
                path.display(),
                c.len,
                c.sha256
            )));
        }
        Ok(bytes)
    }

    /// Copy a component to `dest` (parent directories created; written through a sibling temporary
    /// file and renamed), verifying the source against the inventory and the written file against
    /// the source. Returns the written file's identity.
    pub fn copy_verified(&self, c: &Component, dest: &Path) -> Result<FileIdentity> {
        let bytes = self.read_verified(c)?;
        write_atomic(dest, &bytes)?;
        let back = std::fs::read(dest).map_err(|source| DomainError::Io {
            path: dest.display().to_string(),
            source,
        })?;
        let identity = FileIdentity::of_bytes(&back);
        if identity.sha256 != c.sha256 {
            return Err(DomainError::Validation(format!(
                "{} did not read back as the baseline it was copied from",
                dest.display()
            )));
        }
        Ok(identity)
    }

    /// Verify every component of every target (used by the layout test and the generator's
    /// self-check). Returns the component paths that failed.
    pub fn verify_all(&self) -> Vec<String> {
        let mut bad = Vec::new();
        for t in &self.inventory.targets {
            for c in &t.components {
                if self.read_verified(c).is_err() {
                    bad.push(c.path.clone());
                }
            }
        }
        bad
    }
}

/// Write `bytes` to `path` through a sibling temporary file, then rename over the destination.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let io = |p: &Path| {
        let p = p.display().to_string();
        move |source| DomainError::Io { path: p.clone(), source }
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io(parent))?;
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    let tmp = path.with_file_name(format!("{name}.nhlms-tmp-{}", std::process::id()));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp).map_err(io(&tmp))?;
        f.write_all(bytes).map_err(io(&tmp))?;
        f.sync_all().map_err(io(&tmp))?;
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(DomainError::Io {
            path: path.display().to_string(),
            source: e,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_damaged_bundle_file_is_refused_not_copied() {
        let dir = std::env::temp_dir().join(format!("nhlms-baselines-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("files/x")).unwrap();
        std::fs::write(dir.join("files/x/a.db"), b"hello").unwrap();
        let inv = Inventory {
            schema: INVENTORY_SCHEMA.into(),
            generated_from: GeneratedFrom { head: "test".into(), generator: "test".into(), anchors_manifest: "".into() },
            targets: vec![TargetBaseline {
                game_id: "nhl14".into(),
                platform: "ps3".into(),
                title: "NHL 14".into(),
                status: STATUS_AVAILABLE.into(),
                components: vec![Component {
                    role: ROLE_MAIN_DB.into(),
                    path: "x/a.db".into(),
                    file_name: "a.db".into(),
                    container_dir: None,
                    container_relative: None,
                    len: 5,
                    sha256: sha256_hex(b"hello"),
                    provenance: "verified-vanilla".into(),
                    witness: "".into(),
                    codec: None,
                }],
                gaps: vec![],
            }],
        };
        std::fs::write(dir.join(INVENTORY_FILE), serde_json::to_string(&inv).unwrap()).unwrap();
        let store = BaselineStore::open_bundled(&dir).unwrap().unwrap();
        let c = store.target("nhl14", Platform::Ps3).unwrap().component(ROLE_MAIN_DB).unwrap().clone();
        let dest = dir.join("out/a.db");
        let id = store.copy_verified(&c, &dest).unwrap();
        assert_eq!(id.len, 5);
        std::fs::write(dir.join("files/x/a.db"), b"hellp").unwrap();
        let err = store.copy_verified(&c, &dir.join("out/b.db")).unwrap_err().to_string();
        assert!(err.contains("damaged or substituted"), "{err}");
        assert!(!dir.join("out/b.db").exists());
        assert_eq!(store.verify_all(), vec!["x/a.db".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn probe_recognises_anchor_saves() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ps3 = std::fs::read(repo.join("anchors/ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA")).unwrap();
        let c = probe_save_codec(&ps3);
        assert_eq!((c.container_version.as_str(), c.kind.as_str(), c.readable, c.writable), ("v4", "roster", true, true));
        let ps4 = std::fs::read(repo.join("anchors/ps4/nhl17/savefiles/RosterCustom0/DATA")).unwrap();
        let c = probe_save_codec(&ps4);
        assert_eq!((c.container_version.as_str(), c.readable), ("ps4", true));
        let c = probe_save_codec(b"not a save at all, just bytes that are long enough to look at");
        assert!(!c.readable && c.container_version == "unrecognised");
    }
}
