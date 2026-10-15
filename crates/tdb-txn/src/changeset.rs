//! The change set: what will be replaced, with what, under which expectations.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::identity::{hex, FileIdentity};

/// Schema tag of the serialized change set.
pub const CHANGESET_SCHEMA: &str = "tdb-txn/changeset/1";

/// The expected identity of a destination before the transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Expected {
    /// The destination must not exist.
    Absent,
    /// The destination must be exactly this.
    Present(FileIdentity),
}

/// One qualification item carried from the producer (S2-A's items, verbatim).
/// The engine never resolves or reinterprets these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualificationItem {
    pub subject: String,
    /// `resolved` / `unresolved` / `blocked` as the producer states it.
    pub status: String,
    pub note: String,
}

/// The producer's deployment qualification, carried verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Qualification {
    /// `qualified` or `unqualified` as the producer states it. Anything else
    /// is refused at prepare.
    pub state: String,
    pub items: Vec<QualificationItem>,
}

impl Qualification {
    pub fn is_qualified(&self) -> bool {
        self.state == "qualified"
    }
}

/// What the producer established about the replacement file. The engine
/// treats this as **input to validate**, not as proof: it rechecks the
/// replacement's identity on disk and refuses structural blockers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Validation {
    /// Producer name, e.g. `tdb-archive-big::stage`, `studio-domain::session`.
    pub producer: String,
    /// The producer's own statement that the replacement re-parsed and
    /// matched its plan. `false` is refused at prepare.
    pub structurally_verified: bool,
    /// Structural contradictions reported by the producer; any entry is
    /// refused at prepare.
    pub deploy_blockers: Vec<String>,
    pub qualification: Qualification,
    /// Free-form producer detail (counts, plan summary) for the record.
    pub detail: BTreeMap<String, String>,
}

/// One destination and its replacement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    /// Unique key within the change set (used for dependencies and reports).
    pub key: String,
    /// Destination path **relative to the target root**: forward slashes,
    /// no `..`, no root, no prefix.
    pub destination: String,
    pub expected_prior: Expected,
    /// The replacement file as produced (absolute or relative to the process
    /// cwd); copied into the recovery directory at stage time.
    pub replacement: PathBuf,
    pub replacement_identity: FileIdentity,
    pub validation: Validation,
    /// Keys of artifacts this one depends on (ordering only: dependencies
    /// are applied first).
    pub dependencies: Vec<String>,
}

/// Where the change set may be applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Mode {
    /// A newly created, explicitly selected copy of a game tree, used to
    /// exercise transactions. Structurally valid but **unqualified**
    /// artifacts are accepted here and the fact is recorded; nothing
    /// applied in this mode is evidence of live-game acceptance.
    CopiedTarget,
}

/// A versioned set of coordinated replacements inside one target root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSet {
    pub schema: String,
    /// Content-derived identity: SHA-256 of the canonical serialization of
    /// everything below. Two change sets with the same id will do the same
    /// thing to the same files.
    pub id: String,
    pub title: String,
    pub mode: Mode,
    pub artifacts: Vec<Artifact>,
}

impl ChangeSet {
    /// Build a change set and compute its identity. Artifacts are kept in
    /// the given order; `prepare` re-orders by dependency.
    pub fn new(title: impl Into<String>, mode: Mode, artifacts: Vec<Artifact>) -> Self {
        let mut cs = Self {
            schema: CHANGESET_SCHEMA.to_owned(),
            id: String::new(),
            title: title.into(),
            mode,
            artifacts,
        };
        cs.id = cs.compute_id();
        cs
    }

    /// The identity the artifacts imply (recomputed on load to detect a
    /// tampered record).
    pub fn compute_id(&self) -> String {
        let mut probe = self.clone();
        probe.id = String::new();
        let canon = serde_json::to_vec(&probe).expect("serializable");
        hex(&Sha256::digest(canon))
    }
}

/// Reasons a change set is not valid before anything is looked at on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeSetError {
    Empty,
    DuplicateKey(String),
    /// Two artifacts name the same destination (after normalization).
    DuplicateDestination {
        key_a: String,
        key_b: String,
        destination: String,
    },
    /// A destination is not a plain relative path under the root.
    BadDestination {
        key: String,
        destination: String,
        why: &'static str,
    },
    UnknownDependency {
        key: String,
        dependency: String,
    },
    DependencyCycle(String),
    NotStructurallyVerified(String),
    StructuralBlockers {
        key: String,
        blockers: Vec<String>,
    },
    UnknownQualificationState {
        key: String,
        state: String,
    },
    /// The mode does not accept an unqualified artifact (no such mode exists
    /// yet; `CopiedTarget` accepts and records it).
    UnqualifiedNotAllowed(String),
    BadId {
        declared: String,
        computed: String,
    },
}

impl std::fmt::Display for ChangeSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "change set has no artifacts"),
            Self::DuplicateKey(k) => write!(f, "artifact key `{k}` appears twice"),
            Self::DuplicateDestination {
                key_a,
                key_b,
                destination,
            } => write!(
                f,
                "artifacts `{key_a}` and `{key_b}` both name destination `{destination}`"
            ),
            Self::BadDestination {
                key,
                destination,
                why,
            } => write!(f, "artifact `{key}` destination `{destination}`: {why}"),
            Self::UnknownDependency { key, dependency } => {
                write!(f, "artifact `{key}` depends on unknown `{dependency}`")
            }
            Self::DependencyCycle(k) => write!(f, "dependency cycle through `{k}`"),
            Self::NotStructurallyVerified(k) => {
                write!(
                    f,
                    "artifact `{k}` is not structurally verified by its producer"
                )
            }
            Self::StructuralBlockers { key, blockers } => {
                write!(
                    f,
                    "artifact `{key}` carries structural blockers: {blockers:?}"
                )
            }
            Self::UnknownQualificationState { key, state } => {
                write!(
                    f,
                    "artifact `{key}` has unknown qualification state `{state}`"
                )
            }
            Self::UnqualifiedNotAllowed(k) => {
                write!(
                    f,
                    "artifact `{k}` is unqualified and the mode does not accept that"
                )
            }
            Self::BadId { declared, computed } => {
                write!(
                    f,
                    "change set id {declared} does not match its content ({computed})"
                )
            }
        }
    }
}

impl std::error::Error for ChangeSetError {}

/// Names the engine reserves under a target root; no artifact may name
/// them or anything below them.
pub const RESERVED_TOP_LEVEL: &[&str] = &[".tdb-txn", "copied-target.tdb-txn"];

/// Windows reserved device names; a component equal to one of these (with
/// any extension) names a device, not a file.
const DEVICE_NAMES: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Normalize a destination for duplicate detection: forward slashes, and
/// case-folded (Windows filesystems are case-insensitive, so `A.big` and
/// `a.big` alias the same file). Refuses anything that is not a plain
/// relative path of ordinary file-name components: `..`, roots, prefixes,
/// backslashes, alternate data streams (`name:stream`), Windows device
/// names, control characters, trailing dots/spaces, and the engine's own
/// reserved names.
pub fn normalized_destination(dest: &str) -> Result<String, &'static str> {
    if dest.is_empty() {
        return Err("empty");
    }
    if dest.contains('\\') {
        return Err("must use forward slashes");
    }
    if dest.contains(':') {
        return Err("contains `:` (drive, device or alternate data stream)");
    }
    if dest.chars().any(|c| c.is_control() || c == '\0') {
        return Err("contains a control character");
    }
    if dest.starts_with("//") || dest.starts_with("\\\\") {
        return Err("UNC or device spelling");
    }
    let p = Path::new(dest);
    let mut parts = Vec::new();
    for c in p.components() {
        match c {
            Component::Normal(n) => parts.push(n.to_string_lossy().to_lowercase()),
            Component::CurDir => {}
            Component::ParentDir => return Err("contains `..`"),
            Component::RootDir | Component::Prefix(_) => return Err("must be relative"),
        }
    }
    if parts.is_empty() {
        return Err("names no file");
    }
    if parts.iter().any(|p| p.ends_with('.') || p.ends_with(' ')) {
        return Err("component ends with a dot or space (aliases on Windows)");
    }
    if parts.iter().any(|p| {
        let stem = p.split('.').next().unwrap_or("");
        DEVICE_NAMES.contains(&stem)
    }) {
        return Err("component is a Windows device name");
    }
    if parts.iter().any(|p| {
        p.contains('<')
            || p.contains('>')
            || p.contains('"')
            || p.contains('|')
            || p.contains('?')
            || p.contains('*')
    }) {
        return Err("component contains a character Windows does not allow in names");
    }
    if RESERVED_TOP_LEVEL.contains(&parts[0].as_str()) {
        return Err("names the engine's reserved namespace");
    }
    Ok(parts.join("/"))
}

/// A transaction id as the engine mints it: `NNNNNN-<millis>-<hex12>[-n]`.
/// Anything else is refused before it is joined to a path.
pub fn valid_txn_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !id.starts_with('-')
        && !id.starts_with('.')
}

/// An artifact key usable as a file-name fragment by adapters: ASCII
/// letters, digits, `_`, `-`, `.`; not empty, not starting with a dot, no
/// separators.
pub fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 120
        && !key.starts_with('.')
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
}

/// Validate the change set on its own terms and return the artifacts in
/// dependency order (stable for equal rank).
pub fn validate(cs: &ChangeSet) -> Result<Vec<usize>, ChangeSetError> {
    if cs.schema != CHANGESET_SCHEMA {
        return Err(ChangeSetError::BadId {
            declared: cs.schema.clone(),
            computed: CHANGESET_SCHEMA.to_owned(),
        });
    }
    let computed = cs.compute_id();
    if cs.id != computed {
        return Err(ChangeSetError::BadId {
            declared: cs.id.clone(),
            computed,
        });
    }
    if cs.artifacts.is_empty() {
        return Err(ChangeSetError::Empty);
    }
    let mut keys = BTreeMap::new();
    let mut dests: BTreeMap<String, String> = BTreeMap::new();
    for (i, a) in cs.artifacts.iter().enumerate() {
        if !valid_key(&a.key) {
            return Err(ChangeSetError::DuplicateKey(format!(
                "{} (invalid key)",
                a.key
            )));
        }
        if keys.insert(a.key.clone(), i).is_some() {
            return Err(ChangeSetError::DuplicateKey(a.key.clone()));
        }
        let norm = normalized_destination(&a.destination).map_err(|why| {
            ChangeSetError::BadDestination {
                key: a.key.clone(),
                destination: a.destination.clone(),
                why,
            }
        })?;
        if let Some(other) = dests.insert(norm, a.key.clone()) {
            return Err(ChangeSetError::DuplicateDestination {
                key_a: other,
                key_b: a.key.clone(),
                destination: a.destination.clone(),
            });
        }
        if !a.validation.structurally_verified {
            return Err(ChangeSetError::NotStructurallyVerified(a.key.clone()));
        }
        if !a.validation.deploy_blockers.is_empty() {
            return Err(ChangeSetError::StructuralBlockers {
                key: a.key.clone(),
                blockers: a.validation.deploy_blockers.clone(),
            });
        }
        match a.validation.qualification.state.as_str() {
            "qualified" => {}
            "unqualified" => match cs.mode {
                Mode::CopiedTarget => {}
            },
            other => {
                return Err(ChangeSetError::UnknownQualificationState {
                    key: a.key.clone(),
                    state: other.to_owned(),
                })
            }
        }
    }
    for a in &cs.artifacts {
        for d in &a.dependencies {
            if !keys.contains_key(d) {
                return Err(ChangeSetError::UnknownDependency {
                    key: a.key.clone(),
                    dependency: d.clone(),
                });
            }
        }
    }
    // Topological order, stable.
    let mut order = Vec::with_capacity(cs.artifacts.len());
    let mut done: BTreeSet<usize> = BTreeSet::new();
    let mut visiting: BTreeSet<usize> = BTreeSet::new();
    fn visit(
        i: usize,
        cs: &ChangeSet,
        keys: &BTreeMap<String, usize>,
        done: &mut BTreeSet<usize>,
        visiting: &mut BTreeSet<usize>,
        order: &mut Vec<usize>,
    ) -> Result<(), ChangeSetError> {
        if done.contains(&i) {
            return Ok(());
        }
        if !visiting.insert(i) {
            return Err(ChangeSetError::DependencyCycle(cs.artifacts[i].key.clone()));
        }
        for d in &cs.artifacts[i].dependencies {
            visit(keys[d], cs, keys, done, visiting, order)?;
        }
        visiting.remove(&i);
        done.insert(i);
        order.push(i);
        Ok(())
    }
    for i in 0..cs.artifacts.len() {
        visit(i, cs, &keys, &mut done, &mut visiting, &mut order)?;
    }
    Ok(order)
}
