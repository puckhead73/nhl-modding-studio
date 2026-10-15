//! The durable journal: one JSON document per transaction, rewritten
//! atomically (temp → fsync → rename) at every transition.

use std::fs::File;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::changeset::Expected;
use crate::identity::FileIdentity;

/// Schema tag of the serialized journal.
pub const JOURNAL_SCHEMA: &str = "tdb-txn/journal/1";
/// File names inside the recovery directory.
pub const JOURNAL_FILE: &str = "journal.json";
pub const CHANGESET_FILE: &str = "changeset.json";
pub const BACKUPS_DIR: &str = "backups";
pub const STAGED_DIR: &str = "staged";
pub const RESTORE_DIR: &str = "restore";
/// Under the target root.
pub const TXN_DIR: &str = ".tdb-txn";
pub const ACTIVE_MARKER: &str = "ACTIVE";
pub const LOCK_FILE: &str = "lock";
/// Per-root monotonically increasing transaction sequence.
pub const SEQUENCE_FILE: &str = "SEQUENCE";

/// Transaction state. Terminal states: `Aborted`, `RolledBack`, `Committed`
/// (until restored), `Restored`, `Blocked`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// Change set recorded; nothing copied yet.
    Prepared,
    /// Every backup and staged replacement is on disk and fsynced.
    Staged,
    /// Mutation of destinations has begun (the durable "in progress" record).
    Applying,
    /// Every destination replaced and verified (the durable commit record).
    Committed,
    /// Restore of the originals has begun.
    Restoring,
    /// Every destination verified back to its prior identity.
    Restored,
    /// Cancelled or failed before any destination changed; nothing to undo.
    Aborted,
    /// Failed or cancelled after mutation began; every destination verified
    /// back to its prior identity.
    RolledBack,
    /// Recovery found bytes that are neither the prior nor the replacement,
    /// or material it needs is missing. Nothing was overwritten; the
    /// journal and backups are retained; the root refuses new transactions
    /// until this is resolved by hand.
    Blocked,
}

impl State {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Aborted | Self::RolledBack | Self::Restored | Self::Blocked
        )
    }
    /// Committed sets are complete but still hold backups for restore; they
    /// do not block new transactions on the root.
    pub fn blocks_root(self) -> bool {
        !matches!(
            self,
            Self::Aborted | Self::RolledBack | Self::Restored | Self::Committed
        )
    }
}

/// Per-artifact progress inside an apply or restore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactPhase {
    Pending,
    /// Backup (if any) and staged replacement are on disk and verified.
    Staged,
    /// The rename over the destination is about to happen or has happened;
    /// the destination is either the prior or the replacement.
    Replacing,
    /// Destination verified as the replacement.
    Replaced,
    /// The restore rename is about to happen or has happened.
    Reverting,
    /// Destination verified as the prior (or absent).
    Reverted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalArtifact {
    pub key: String,
    /// Relative to the target root.
    pub destination: String,
    pub prior: Expected,
    /// Lineage: the committed transaction whose replacement this artifact's
    /// prior was (the previous owner of the destination), if any.
    #[serde(default)]
    pub lineage_prior: Option<String>,
    /// Relative to the recovery directory; `None` for an absent prior.
    pub backup: Option<String>,
    /// Relative to the recovery directory.
    pub staged: String,
    pub replacement: FileIdentity,
    pub phase: ArtifactPhase,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    pub schema: String,
    pub txn_id: String,
    /// Position in the root's transaction history (strictly increasing per
    /// root; assigned at prepare). Ownership of a destination belongs to the
    /// highest-sequence transaction that took it and did not give it back.
    #[serde(default)]
    pub sequence: u64,
    pub changeset_id: String,
    /// Recorded for the operator; recovery uses the directory it was found
    /// in, never this string.
    pub target_root: String,
    pub state: State,
    /// True when the change set carried an unqualified artifact and the
    /// copied-target mode accepted it.
    pub applied_unqualified: bool,
    /// Artifacts in apply order.
    pub artifacts: Vec<JournalArtifact>,
    /// Human-readable transition log (state, artifact key, note).
    pub log: Vec<String>,
}

impl Journal {
    /// The artifact recorded for a (normalized) destination, if any.
    pub fn artifact_for_destination(&self, normalized: &str) -> Option<&JournalArtifact> {
        self.artifacts.iter().find(|a| {
            crate::changeset::normalized_destination(&a.destination)
                .ok()
                .as_deref()
                == Some(normalized)
        })
    }

    pub fn artifact(&self, key: &str) -> Option<&JournalArtifact> {
        self.artifacts.iter().find(|a| a.key == key)
    }
    pub fn artifact_mut(&mut self, key: &str) -> Option<&mut JournalArtifact> {
        self.artifacts.iter_mut().find(|a| a.key == key)
    }

    /// Serialize to `<dir>/journal.json` durably: write `journal.json.tmp`,
    /// fsync it, rename it over the journal.
    pub fn write(&self, dir: &Path) -> io::Result<()> {
        let tmp = dir.join(format!("{JOURNAL_FILE}.tmp"));
        let final_path = dir.join(JOURNAL_FILE);
        let bytes = serde_json::to_vec_pretty(self).expect("serializable");
        {
            let mut f = File::create(&tmp)?;
            f.write_all(&bytes)?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, &final_path)?;
        crate::identity::fsync_path(&final_path)
    }

    /// Read and validate `<dir>/journal.json`: schema, and every recorded
    /// path must be a plain relative path (recovery never follows a journal
    /// outside its own roots).
    pub fn read(dir: &Path) -> Result<Self, JournalError> {
        let path = dir.join(JOURNAL_FILE);
        let bytes = std::fs::read(&path).map_err(|source| JournalError::Unreadable {
            path: path.clone(),
            source,
        })?;
        let j: Journal = serde_json::from_slice(&bytes).map_err(|e| JournalError::Corrupt {
            path: path.clone(),
            why: e.to_string(),
        })?;
        j.validate()
            .map_err(|why| JournalError::Corrupt { path, why })?;
        Ok(j)
    }

    /// Semantic invariants beyond JSON shape: schema, a well-formed
    /// transaction id and sequence, unique keys and destinations, plain
    /// recovery paths, a backup for every present prior and none for an
    /// absent one, and phases that are legal in the recorded state.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != JOURNAL_SCHEMA {
            return Err(format!("schema `{}`", self.schema));
        }
        if !crate::changeset::valid_txn_id(&self.txn_id) {
            return Err(format!(
                "txn_id `{}` is not a valid transaction id",
                self.txn_id
            ));
        }
        if self.sequence == 0 {
            return Err("sequence 0 (unassigned)".into());
        }
        if self.changeset_id.len() != 64
            || !self.changeset_id.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("changeset_id is not a SHA-256".into());
        }
        if self.artifacts.is_empty() {
            return Err("no artifacts".into());
        }
        let mut keys = std::collections::BTreeSet::new();
        let mut dests = std::collections::BTreeSet::new();
        let mut recovery_paths = std::collections::BTreeSet::new();
        for a in &self.artifacts {
            if !crate::changeset::valid_key(&a.key) || !keys.insert(a.key.clone()) {
                return Err(format!("artifact key `{}` invalid or duplicated", a.key));
            }
            let norm = crate::changeset::normalized_destination(&a.destination)
                .map_err(|why| format!("destination `{}` escapes: {why}", a.destination))?;
            if !dests.insert(norm) {
                return Err(format!("destination `{}` recorded twice", a.destination));
            }
            match (&a.prior, &a.backup) {
                (Expected::Present(_), Some(b)) => {
                    check_plain_relative(b).map_err(|e| e.to_string())?;
                    if !b.starts_with(&format!("{BACKUPS_DIR}/")) {
                        return Err(format!("backup `{b}` is not under {BACKUPS_DIR}/"));
                    }
                    if !recovery_paths.insert(b.clone()) {
                        return Err(format!("backup `{b}` recorded twice"));
                    }
                }
                (Expected::Present(_), None) => {
                    return Err(format!(
                        "artifact `{}` has a present prior but no backup",
                        a.key
                    ))
                }
                (Expected::Absent, Some(_)) => {
                    return Err(format!(
                        "artifact `{}` has an absent prior but a backup",
                        a.key
                    ))
                }
                (Expected::Absent, None) => {}
            }
            check_plain_relative(&a.staged).map_err(|e| e.to_string())?;
            if !a.staged.starts_with(&format!("{STAGED_DIR}/")) {
                return Err(format!("staged `{}` is not under {STAGED_DIR}/", a.staged));
            }
            if !recovery_paths.insert(a.staged.clone()) {
                return Err(format!("staged `{}` recorded twice", a.staged));
            }
            if a.replacement.sha256.len() != 64 {
                return Err(format!(
                    "artifact `{}` replacement identity malformed",
                    a.key
                ));
            }
            let legal = match self.state {
                State::Prepared => matches!(a.phase, ArtifactPhase::Pending),
                State::Staged => matches!(a.phase, ArtifactPhase::Pending | ArtifactPhase::Staged),
                State::Aborted => matches!(a.phase, ArtifactPhase::Pending | ArtifactPhase::Staged),
                _ => true,
            };
            if !legal {
                return Err(format!(
                    "artifact `{}` phase {:?} is not legal in state {:?}",
                    a.key, a.phase, self.state
                ));
            }
        }
        Ok(())
    }
}

/// A recovery-directory-relative path must be exactly `<subdir>/<name>`.
fn check_plain_relative(p: &str) -> Result<(), JournalError> {
    let path = Path::new(p);
    let comps: Vec<_> = path.components().collect();
    let ok = comps.len() == 2
        && comps
            .iter()
            .all(|c| matches!(c, Component::Normal(n) if !n.is_empty()))
        && !p.contains('\\');
    if !ok {
        return Err(JournalError::EscapingPath {
            path: p.to_owned(),
            why: "recovery paths must be `<dir>/<file>`",
        });
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("journal {path} cannot be read: {source}")]
    Unreadable { path: PathBuf, source: io::Error },
    #[error("journal {path} is corrupt: {why}")]
    Corrupt { path: PathBuf, why: String },
    #[error("journal names a path that escapes its root: `{path}` ({why})")]
    EscapingPath { path: String, why: &'static str },
}
