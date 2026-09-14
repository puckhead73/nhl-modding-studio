//! Layer-C rendering and journalled publication —
//! [contract §8](../../../../docs/plans/qa-evidence-contract.md).
//!
//! `build` renders; `publish` installs; `recover` finishes or undoes an interrupted publication.
//! **`build` and `check` never publish**, and no operation here touches layer B.
//!
//! **R-NO-GIT.** `git checkout` is not a recovery mechanism and is never suggested: this repository
//! routinely carries dozens of unrelated working-tree changes. Recovery uses the journal and the
//! retained prior generation only.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::csv;
use super::merge::{Merged, SummaryRow};
use super::paths::{self, Layout};
use super::store::{sha256_prefixed, StoreError};

type Result<T> = std::result::Result<T, StoreError>;

fn io(path: &Path) -> impl Fn(std::io::Error) -> StoreError + '_ {
    move |source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn rule(rule: &'static str, path: &Path, detail: impl Into<String>) -> StoreError {
    StoreError::Rule {
        rule,
        path: path.to_path_buf(),
        detail: detail.into(),
    }
}

/// The four published files, in the contract's order.
pub const MATRIX: &str = "qa-evidence-matrix.csv";
pub const CONFLICTS: &str = "qa-evidence-conflicts.csv";
pub const UNRESOLVED: &str = "qa-evidence-unresolved.csv";
pub const DIVERGENCE: &str = "asset-matrix-divergence.csv";
/// **C0-S2-INT1-R1 extension.** Executed cases whose emitter operation has no operation in the
/// closed §4.1 vocabulary. They carry no coverage key, so they appear in no row of the matrix and
/// qualify nothing — this file exists so the execution is not lost, and so the vocabulary gap is
/// countable rather than a comment. Contract §8.1 names four files; this is a fifth, additive one.
pub const UNMAPPED: &str = "qa-evidence-unmapped.csv";
/// The journal, which lives beside the published set.
pub const JOURNAL: &str = ".qa-publish-journal.json";
/// Recorded in the staged set so `publish` can prove the inputs have not moved since `build`.
pub const BUILD_INPUTS: &str = "build-inputs.json";

pub const OUTPUT_FILES: &[&str] = &[MATRIX, CONFLICTS, UNRESOLVED, DIVERGENCE, UNMAPPED];

const MATRIX_HEADER: &[&str] = &[
    "schema",
    "key",
    "target",
    "family",
    "operation",
    "variant",
    "conversion",
    "applicability",
    "applicability_reason",
    "implementation",
    "automated",
    "manual",
    "qualification",
    "fresh",
    "reason",
    "auto_runs",
    "manual_records",
    "legacy_claims",
    "conflict",
    "unresolved",
];

const CONFLICTS_HEADER: &[&str] = &[
    "schema",
    "key",
    "conflict",
    "citation_kind",
    "citation_id",
    "citation_state",
    "selected",
    "detail",
];

const UNRESOLVED_HEADER: &[&str] = &["schema", "key", "unresolved"];

const UNMAPPED_HEADER_C: &[&str] = &[
    "schema",
    "source_kind",
    "source_id",
    "source_state",
    "target",
    "emitter_operation",
    "case_id",
    "outcome",
    "fixture_sha256",
    "unresolved",
    "detail",
];

const DIVERGENCE_HEADER: &[&str] = &[
    "schema",
    "subject",
    "left_source",
    "left_value",
    "right_source",
    "right_value",
    "unresolved",
];

/// One published unmapped execution. It is deliberately not a [`SummaryRow`]: it has no key, no
/// axes and no qualification, and nothing reads it to decide anything.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnmappedRow {
    /// `auto-run` or `auto-journal`.
    pub source_kind: String,
    pub source_id: String,
    /// The source's state — `completed`, `failed`, `interrupted`, `incomplete`.
    pub source_state: String,
    pub target: String,
    pub emitter_operation: String,
    pub case_id: String,
    pub outcome: String,
    pub fixture_sha256: String,
    pub unresolved: String,
    pub detail: String,
}

/// One row of the duplicate-asset-matrix divergence report (contract R-DUP). Neither side wins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivergenceRow {
    pub subject: String,
    pub left_source: String,
    pub left_value: String,
    pub right_source: String,
    pub right_value: String,
    pub unresolved: String,
}

/// What a sidecar records about one build.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Provenance {
    pub schema: String,
    pub tool_version: String,
    /// Run manifests, manual record digests, the legacy freeze digest.
    pub merge_input_digests: BTreeMap<String, String>,
    pub scope_definitions_digest: String,
    pub output_digest: String,
}

/// Everything `build` needs beyond the merged rows. Supplied by the caller so `build` stays a pure
/// function of its inputs.
#[derive(Debug, Clone, Default)]
pub struct BuildInputs {
    pub tool_version: String,
    pub merge_input_digests: BTreeMap<String, String>,
    pub scope_definitions_digest: String,
    pub divergence: Vec<DivergenceRow>,
    /// Executed cases with no coverage key, from every run and journal (see [`UNMAPPED`]).
    pub unmapped: Vec<UnmappedRow>,
}

/// The rendered output set: file name → contents. Nothing is written by `build` itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSet {
    pub files: BTreeMap<String, String>,
}

impl OutputSet {
    /// A digest over the whole set, used as the publication id and in each sidecar.
    pub fn digest(&self) -> String {
        let mut joined = String::new();
        for (name, contents) in &self.files {
            joined.push_str(name);
            joined.push('\0');
            joined.push_str(contents);
            joined.push('\0');
        }
        sha256_prefixed(joined.as_bytes())
    }

    /// Write the set plus `build-inputs.json` into an output directory.
    ///
    /// **R2:** there is no unguarded variant. A validated [`Layout`] is required, so a library
    /// caller gets the same boundary checks the CLI does, and **every individual file** — each
    /// output and `build-inputs.json` — is validated as its own mutation target before it is
    /// written. Resolving only the directory, as R1 did, still let `fs::write` follow a child link.
    pub fn write_to(&self, dir: &Path, inputs: &BuildInputs, layout: &Layout) -> Result<()> {
        // Checked BEFORE anything is created: `resolve_for_write` walks up to the deepest existing
        // ancestor, so a destination that does not exist yet is still resolved and still proven
        // disjoint from every protected root before a single directory is made.
        let resolved = layout.assert_output_is_safe("the build output directory", dir)?;
        fs::create_dir_all(&resolved).map_err(io(&resolved))?;
        for (name, contents) in &self.files {
            paths::write_file(&resolved.join(name), contents.as_bytes())?;
        }
        let path = resolved.join(BUILD_INPUTS);
        let json = serde_json::to_string_pretty(&inputs.merge_input_digests)
            .map_err(|e| rule("build-inputs", &path, e.to_string()))?;
        paths::write_file(&path, json.as_bytes())?;
        Ok(())
    }
}

fn provenance_name(file: &str) -> String {
    format!("{file}.provenance.json")
}

/// Render the layer-C set. Deterministic: rows are key-sorted by the merge, and every list column is
/// sorted before joining (A1, A3).
pub fn build(merged: &Merged, inputs: &BuildInputs) -> OutputSet {
    let mut files = BTreeMap::new();

    let mut matrix = csv::row(MATRIX_HEADER);
    let mut conflicts = csv::row(CONFLICTS_HEADER);
    let mut unresolved = csv::row(UNRESOLVED_HEADER);

    for row in &merged.rows {
        matrix.push_str(&matrix_row(row));
        // R-CITE-ALL: a conflicted row publishes every citation that mentions the key, selected or
        // not, so the disagreement can be read without opening the run directories.
        for conflict in &row.conflict {
            for citation in &row.citations {
                conflicts.push_str(&csv::row([
                    "qa-evidence-conflict/1",
                    &row.key.to_string(),
                    conflict,
                    citation.kind,
                    &citation.id,
                    &citation.state,
                    if citation.selected { "true" } else { "false" },
                    &citation.detail,
                ]));
            }
        }
        for token in &row.unresolved {
            unresolved.push_str(&csv::row([
                "qa-evidence-unresolved/1",
                &row.key.to_string(),
                token,
            ]));
        }
    }

    let mut divergence = csv::row(DIVERGENCE_HEADER);
    let mut rows = inputs.divergence.clone();
    rows.sort_by(|a, b| (&a.subject, &a.left_source).cmp(&(&b.subject, &b.left_source)));
    for d in rows {
        divergence.push_str(&csv::row([
            "qa-asset-matrix-divergence/1",
            &d.subject,
            &d.left_source,
            &d.left_value,
            &d.right_source,
            &d.right_value,
            &d.unresolved,
        ]));
    }

    let mut unmapped_rows = inputs.unmapped.clone();
    unmapped_rows.sort();
    unmapped_rows.dedup();
    let mut unmapped_out = csv::row(UNMAPPED_HEADER_C);
    for u in &unmapped_rows {
        unmapped_out.push_str(&csv::row([
            "qa-evidence-unmapped/1",
            &u.source_kind,
            &u.source_id,
            &u.source_state,
            &u.target,
            &u.emitter_operation,
            &u.case_id,
            &u.outcome,
            &u.fixture_sha256,
            &u.unresolved,
            &u.detail,
        ]));
    }

    files.insert(MATRIX.to_string(), matrix);
    files.insert(UNMAPPED.to_string(), unmapped_out);
    files.insert(CONFLICTS.to_string(), conflicts);
    files.insert(UNRESOLVED.to_string(), unresolved);
    files.insert(DIVERGENCE.to_string(), divergence);

    // Sidecars last, so each can carry the digest of its own file.
    let sidecars: Vec<(String, String)> = files
        .iter()
        .map(|(name, contents)| {
            let provenance = Provenance {
                schema: "qa-evidence-provenance/1".into(),
                tool_version: inputs.tool_version.clone(),
                merge_input_digests: inputs.merge_input_digests.clone(),
                scope_definitions_digest: inputs.scope_definitions_digest.clone(),
                output_digest: sha256_prefixed(contents.as_bytes()),
            };
            (
                provenance_name(name),
                serde_json::to_string_pretty(&provenance).unwrap_or_default() + "\n",
            )
        })
        .collect();
    files.extend(sidecars);

    OutputSet { files }
}

fn matrix_row(row: &SummaryRow) -> String {
    let cite = |kind: &str| {
        let mut ids: Vec<String> = row
            .citations_of(kind)
            .iter()
            .map(|c| {
                format!(
                    "{}[{}{}]",
                    c.id,
                    c.state,
                    if c.selected { "+selected" } else { "" }
                )
            })
            .collect();
        ids.sort();
        ids.join(" ")
    };
    csv::row([
        "qa-summary/2",
        &row.key.to_string(),
        &row.key.target,
        &row.key.family,
        &row.key.operation,
        row.key.variant.as_deref().unwrap_or(""),
        &row.key
            .conversion
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        row.applicability.as_str(),
        &row.applicability_reason,
        &row.implementation,
        row.automated.as_str(),
        row.manual.as_str(),
        row.qualification.as_str(),
        row.fresh.as_str(),
        &row.reason,
        &cite("auto-run"),
        &cite("manual"),
        &cite("legacy"),
        &row.conflict.join(" "),
        &row.unresolved.join(" "),
    ])
}

// -------------------------------------------------------------- journal ------
//
// **C0-BULK1-R1.** The first journal was written by truncating the single journal file in place and
// its recovery trusted the per-target `installed` boolean. Both were wrong: a truncating write can
// destroy the only record of an in-flight transaction, and a boolean cannot describe a rename that
// succeeded before its journal update was persisted. The journal below is replaced atomically
// through a temporary file, and **recovery derives state from digests of the bytes actually on
// disk**; the booleans are advisory breadcrumbs that no decision reads.

/// The journal schema this build writes and accepts. An unknown schema fails closed.
pub const JOURNAL_SCHEMA: &str = "qa-publish-journal/2";

/// Length of a publication attempt id, in hex characters.
const ATTEMPT_ID_LEN: usize = 16;

/// The exact set of names a journal may name as targets: the four outputs and their sidecars.
fn expected_target_names() -> BTreeSet<String> {
    let mut names: BTreeSet<String> = OUTPUT_FILES.iter().map(|f| (*f).to_string()).collect();
    // The sidecar names are derived from the same constants, so this set cannot drift from `build`.
    for file in OUTPUT_FILES {
        names.insert(provenance_name(file));
    }
    names
}

/// `sha256:` followed by exactly 64 lowercase hex characters.
fn is_digest(value: &str) -> bool {
    match value.strip_prefix("sha256:") {
        Some(hex) => {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }
        None => false,
    }
}

/// Exactly [`ATTEMPT_ID_LEN`] lowercase hex characters, and nothing else — no separator, no dot, no
/// `..`. Both working-directory names are built from it, so its syntax bounds those paths.
fn is_attempt_id(value: &str) -> bool {
    value.len() == ATTEMPT_ID_LEN
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn staging_name(id: &str) -> String {
    format!(".qa-publish-{id}")
}

fn prev_name(id: &str) -> String {
    format!(".qa-prev-{id}")
}

/// Remove a publication working directory **only** after proving we own it.
///
/// **R2:** the R1 code called `fs::remove_dir_all` on the journal's `staging`/`prev` strings. Between
/// the journal's authority limits and this check there are now two independent barriers: the journal
/// can only name the derived child of the published directory, and even that directory is deleted
/// only if every entry in it is an expected target file. Anything unexpected — a subdirectory, a
/// stray file, a link — refuses and **keeps the material**, because an operator's file inside a
/// recovery directory is far more likely to be evidence than garbage.
fn cleanup_owned_dir(dir: &Path, context: &str) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(StoreError::Io {
                path: dir.to_path_buf(),
                source,
            })
        }
    };
    let expected = expected_target_names();
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(io(dir))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let metadata = fs::symlink_metadata(&path).map_err(io(&path))?;
        if paths::is_reparse_point(&metadata) || !metadata.file_type().is_file() {
            return Err(rule(
                "cleanup-unexpected-entry",
                &path,
                format!(
                    "{context} contains {name:?}, which is not a regular file. Nothing was deleted \
                     and the whole directory is kept for inspection."
                ),
            ));
        }
        if !expected.contains(name.as_str()) {
            return Err(rule(
                "cleanup-unexpected-entry",
                &path,
                format!(
                    "{context} contains {name:?}, which is not part of the published output set. \
                     Nothing was deleted and the whole directory is kept: an unexplained file here \
                     is evidence, not garbage. Inspect it, then remove the directory by hand."
                ),
            ));
        }
        files.push(path);
    }
    for file in files {
        fs::remove_file(&file).map_err(io(&file))?;
    }
    fs::remove_dir(dir).map_err(io(dir))?;
    Ok(())
}

/// Where a publication had got to. Recorded for diagnostics and to choose a recovery *strategy*;
/// the per-file work is always decided by hashing, never by the phase alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    /// Staged copies written; nothing in the published set has moved.
    Prepared,
    /// Retention moves are in progress. A file may already have moved without its journal update.
    Retaining,
    /// Every retention move is known to have completed.
    Retained,
    /// Install moves are in progress.
    Installing,
    /// A rollback is in progress.
    RollingBack,
    /// The publication is finished and the transaction is resolved.
    Completed,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Prepared => "prepared",
            Phase::Retaining => "retaining",
            Phase::Retained => "retained",
            Phase::Installing => "installing",
            Phase::RollingBack => "rolling-back",
            Phase::Completed => "completed",
        }
    }

    /// A transaction is unresolved until it reaches `Completed`.
    pub fn is_unresolved(self) -> bool {
        self != Phase::Completed
    }
}

/// One target file's role in a publication.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JournalTarget {
    pub name: String,
    /// The digest the published file had before this publication. `None` means **the file did not
    /// exist**, which is what tells a rollback to *remove* it rather than restore it.
    pub previous_digest: Option<String>,
    pub staged_digest: String,
    /// Advisory only. Recovery never reads it (R1).
    pub retained: bool,
    /// Advisory only. Recovery never reads it (R1).
    pub installed: bool,
}

/// `.qa-publish-journal.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Journal {
    pub schema: String,
    pub id: String,
    pub phase: Phase,
    pub staging: String,
    pub prev: String,
    /// Recorded so recovery can prove the journal belongs to the directory it was found in.
    pub data_dir: String,
    pub targets: Vec<JournalTarget>,
}

/// Why a journal could not be used. Every variant is fail-closed: **only `NotFound` means "no
/// journal"**, and everything else stops the operation with a diagnostic naming the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalFault {
    Unreadable(String),
    Corrupt(String),
    UnsupportedSchema(String),
    Inconsistent(String),
}

impl Journal {
    fn temp_path(data_dir: &Path) -> PathBuf {
        data_dir.join(format!("{JOURNAL}.new"))
    }

    fn path(data_dir: &Path) -> PathBuf {
        data_dir.join(JOURNAL)
    }

    /// Replace the journal atomically: serialise, write a sibling temporary, then rename it over the
    /// existing file. A failure before the rename leaves the previous journal **intact**; the rename
    /// itself is the single commit point.
    ///
    /// `fault` lets a test fail *between* the temporary write and the rename, which is the partial
    /// replacement this design exists to survive.
    /// **R2:** both the temporary and the journal itself are validated as mutation targets. A link
    /// at either name would otherwise redirect the write — the `.new` path in particular, because
    /// it is created fresh on every journal update and so is the easiest name to pre-plant.
    fn write(&self, data_dir: &Path, fault: Option<&Fault>) -> Result<()> {
        let temp = Self::temp_path(data_dir);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| rule("journal", &temp, format!("could not serialise: {e}")))?;
        let temp = paths::write_file(&temp, json.as_bytes())?;
        if matches!(fault, Some(Fault::JournalReplaceAfterTemp)) {
            return Err(rule(
                "injected-failure",
                &temp,
                "journal replacement interrupted after the temporary was written",
            ));
        }
        paths::rename_file(&temp, &Self::path(data_dir))?;
        Ok(())
    }

    /// Read the journal.
    ///
    /// `Ok(None)` is returned for **`NotFound` only**. An unreadable, corrupt, wrongly-schema'd or
    /// self-inconsistent journal is `Err` — never silently treated as absent, because "no journal"
    /// and "I cannot read the journal" have opposite consequences.
    pub fn read(data_dir: &Path) -> Result<Option<Journal>> {
        let path = Self::path(data_dir);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(StoreError::Io {
                    path: path.clone(),
                    source,
                });
            }
        };
        let journal: Journal = serde_json::from_str(&text).map_err(|e| {
            rule(
                "journal-corrupt",
                &path,
                format!(
                    "the publication journal did not parse ({e}). It is NOT treated as absent. \
                     Inspect it by hand; the staged and retained material it names is still on disk."
                ),
            )
        })?;
        journal.validate(data_dir, &path)?;
        Ok(Some(journal))
    }

    /// Structural checks that must pass **before recovery writes or deletes anything**.
    fn validate(&self, data_dir: &Path, path: &Path) -> Result<()> {
        if self.schema != JOURNAL_SCHEMA {
            return Err(rule(
                "journal-schema",
                path,
                format!(
                    "unsupported journal schema {:?} (this build writes {JOURNAL_SCHEMA}); \
                     refusing to act on a journal whose meaning is not known",
                    self.schema
                ),
            ));
        }
        if !is_attempt_id(&self.id) {
            return Err(rule(
                "journal-id",
                path,
                format!(
                    "publication id {:?} is not the expected {ATTEMPT_ID_LEN} lowercase hex \
                     characters; the staging and retained directory names are derived from it, so \
                     an unconstrained id is an unconstrained path",
                    self.id
                ),
            ));
        }

        // ---- the exact expected target set: no duplicates, no omissions, nothing extra ----
        //
        // R1 checked only that each name was *a member* of the output set, which let a modified
        // journal omit targets (so a rollback would silently skip restoring them) or repeat one.
        let expected = expected_target_names();
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for target in &self.targets {
            let name = Path::new(&target.name);
            let plain = name.components().count() == 1
                && matches!(name.components().next(), Some(Component::Normal(_)));
            if !plain {
                return Err(rule(
                    "journal-target-name",
                    path,
                    format!("target name {:?} is not a plain file name", target.name),
                ));
            }
            if !expected.contains(target.name.as_str()) {
                return Err(rule(
                    "journal-target-name",
                    path,
                    format!(
                        "target name {:?} is not part of the published output set",
                        target.name
                    ),
                ));
            }
            if !seen.insert(target.name.as_str()) {
                return Err(rule(
                    "journal-target-set",
                    path,
                    format!(
                        "target {:?} appears more than once; a duplicate makes the recorded \
                         previous/staged digests for that name ambiguous",
                        target.name
                    ),
                ));
            }
            if !is_digest(&target.staged_digest) {
                return Err(rule(
                    "journal-digest",
                    path,
                    format!(
                        "staged digest for {:?} is not a `sha256:<64 hex>` value",
                        target.name
                    ),
                ));
            }
            if let Some(previous) = &target.previous_digest {
                if !is_digest(previous) {
                    return Err(rule(
                        "journal-digest",
                        path,
                        format!(
                            "previous digest for {:?} is not a `sha256:<64 hex>` value",
                            target.name
                        ),
                    ));
                }
            }
        }
        let missing: Vec<&str> = expected
            .iter()
            .map(String::as_str)
            .filter(|name| !seen.contains(name))
            .collect();
        if !missing.is_empty() {
            return Err(rule(
                "journal-target-set",
                path,
                format!(
                    "the journal omits {missing:?} from the published output set; recovery acts \
                     only on recorded targets, so an omitted one would be left half-published"
                ),
            ));
        }

        // ---- containment, and the exact derived directory names ----
        let resolved_data = paths::resolve_for_write(data_dir)?;
        let recorded_data = paths::resolve_for_write(Path::new(&self.data_dir))?;
        if resolved_data != recorded_data {
            return Err(rule(
                "journal-foreign",
                path,
                format!(
                    "this journal records data_dir {:?}, but it was found in {:?}; refusing to \
                     recover a transaction that belongs to another directory",
                    self.data_dir,
                    data_dir.display()
                ),
            ));
        }
        // R2: "somewhere under data_dir" is not enough — recovery removes these directories
        // recursively, so a modified journal could otherwise point `prev` at any existing child and
        // have recovery delete it. Both names are derived from the validated id and compared
        // exactly, and they must differ from each other.
        for (label, raw, expected_name) in [
            ("staging", &self.staging, staging_name(&self.id)),
            ("retained generation", &self.prev, prev_name(&self.id)),
        ] {
            let resolved = paths::resolve_for_write(Path::new(raw))?;
            if resolved != resolved_data.join(&expected_name) {
                return Err(rule(
                    "journal-containment",
                    path,
                    format!(
                        "the {label} directory {raw:?} is not the expected direct child \
                         {expected_name:?} of the published directory"
                    ),
                ));
            }
        }
        if self.staging == self.prev || staging_name(&self.id) == prev_name(&self.id) {
            return Err(rule(
                "journal-containment",
                path,
                "the staging and retained directories are the same location; the staged and \
                 previous generations would overwrite each other",
            ));
        }
        Ok(())
    }
}

// -------------------------------------------------------- fault injection ----

/// Test-only interruption points. Each one models a **real** filesystem state a crash can leave, not
/// merely a clean phase boundary: a rename that completed before its journal update, a journal
/// replacement that failed after its temporary was written, and so on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// Fail immediately after the prepare journal is persisted.
    AfterPrepareJournal,
    /// Fail after the Nth old file has been **moved aside**, *before* its journal update. This is
    /// the state the first implementation could not recover from.
    AfterRetentionMove(usize),
    /// Fail after the Nth retention move's journal update.
    AfterRetentionJournal(usize),
    /// Fail after the Nth staged file has been **moved into place**, *before* its journal update.
    AfterInstallMove(usize),
    /// Fail after the Nth install move's journal update.
    AfterInstallJournal(usize),
    /// Make the next journal replacement fail after its temporary is written but before the rename.
    JournalReplaceAfterTemp,
}

/// Options for one publication.
#[derive(Debug, Clone, Default)]
pub struct PublishOptions {
    /// Test-only. Production callers leave this `None`.
    pub fault: Option<Fault>,
}

impl PublishOptions {
    fn journal_fault(&self) -> Option<&Fault> {
        match &self.fault {
            Some(Fault::JournalReplaceAfterTemp) => self.fault.as_ref(),
            _ => None,
        }
    }
}

// ------------------------------------------------------------- publishing ----

/// What a completed publication did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishReport {
    pub id: String,
    pub installed: Vec<String>,
    pub retained_generation: PathBuf,
}

fn digest_file(path: &Path) -> Result<String> {
    Ok(sha256_prefixed(&fs::read(path).map_err(io(path))?))
}

/// The digest of a file, or `None` when it is absent. Any other error propagates: an unreadable file
/// is never reported as absent.
fn digest_if_present(path: &Path) -> Result<Option<String>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(sha256_prefixed(&bytes))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(StoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// A publication attempt's identity.
///
/// **R1:** this is deliberately *not* a digest of the output alone. Publishing identical content
/// twice used to produce the same id, so the second attempt's `.qa-prev-<id>` collided with the
/// first's retained generation and overwrote history. The nonce below makes every attempt distinct.
///
/// The process id, counter and clock reading are **identity material only**. Nothing compares them,
/// orders by them or infers chronology from them — R-NO-TIME is about decisions, and no decision
/// reads this value.
fn attempt_id(staged: &BTreeMap<String, String>) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut seed: String = staged.iter().map(|(k, v)| format!("{k}={v};")).collect();
    seed.push_str(&format!(
        "pid={};n={};t={:?}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos()),
    ));
    sha256_prefixed(seed.as_bytes())[7..23].to_string()
}

/// Install a staged set into `data_dir`, journalled.
///
/// Order of operations, and what each guarantees:
///
/// 1. **Boundary checks** — the staged directory and the published directory are resolved and proven
///    disjoint from each other and from every protected evidence root, before anything is read.
/// 2. **Unresolved-transaction check** — a journal in any phase but `completed` refuses the
///    publication outright. Recover first.
/// 3. **Input re-verification** — a merge input digest that changed since `build` refuses before any
///    move (A30).
/// 4. **Prepare** — staged copies and the journal.
/// 5. **Retain** — each existing published file moves aside, with the journal updated after each.
/// 6. **Install** — each staged file moves into place, with the journal updated after each.
/// 7. **Seal** — the journal reaches `completed`; the staging directory is removed. **The retained
///    generation is kept**; nothing is pruned (R1).
///
/// **Durability, stated precisely.** These guarantees hold against *process* interruption — a crash,
/// a kill, or an error at any point above — on the assumption that the operating system has already
/// made completed `write` and `rename` calls visible. **No `fsync`/`FlushFileBuffers` is issued
/// anywhere**, so this is *not* power-loss or OS-crash safe: after such an event the journal and the
/// data files may disagree in ways recovery reports as foreign bytes rather than repairs. Making that
/// claim would require flushing every file and its parent directory and testing it, which this
/// revision does not do.
pub fn publish(
    staged_dir: &Path,
    data_dir: &Path,
    layout: &Layout,
    current_inputs: &BTreeMap<String, String>,
    options: &PublishOptions,
) -> Result<PublishReport> {
    // ---- 1. boundaries, before anything is read or written ----
    //
    // **R2:** the layout is mandatory. There is no `publish` that skips this, so a library caller
    // cannot obtain weaker checks than the CLI by choosing a different entry point.
    let resolved_data = layout.assert_output_is_safe("the published set", data_dir)?;
    let resolved_staged = layout.assert_output_is_safe("the staged set", staged_dir)?;
    paths::assert_disjoint(
        "the staged set",
        &resolved_staged,
        "the published set",
        &resolved_data,
    )?;

    // ---- 2. an unresolved transaction blocks a new one ----
    if let Some(existing) = Journal::read(data_dir)? {
        if existing.phase.is_unresolved() {
            return Err(rule(
                "unresolved-transaction",
                &Journal::path(data_dir),
                format!(
                    "publication {} is unresolved at phase `{}`; run `qa-evidence publish --recover` \
                     before publishing again. Its staged material is at {} and its retained \
                     generation at {}.",
                    existing.id,
                    existing.phase.as_str(),
                    existing.staging,
                    existing.prev
                ),
            ));
        }
    }

    // ---- 3. re-verify the merge inputs ----
    let inputs_path = staged_dir.join(BUILD_INPUTS);
    let recorded: BTreeMap<String, String> =
        serde_json::from_str(&fs::read_to_string(&inputs_path).map_err(io(&inputs_path))?)
            .map_err(|e| rule("build-inputs", &inputs_path, format!("did not parse: {e}")))?;
    if &recorded != current_inputs {
        let changed: Vec<String> = recorded
            .keys()
            .chain(current_inputs.keys())
            .filter(|k| recorded.get(*k) != current_inputs.get(*k))
            .cloned()
            .collect();
        return Err(rule(
            "A30",
            staged_dir,
            format!("merge inputs changed since build; refusing before any move: {changed:?}"),
        ));
    }

    let mut staged: BTreeMap<String, String> = BTreeMap::new();
    for name in OUTPUT_FILES {
        for file in [name.to_string(), provenance_name(name)] {
            let path = staged_dir.join(&file);
            staged.insert(file, digest_file(&path)?);
        }
    }

    let id = attempt_id(&staged);
    debug_assert!(
        is_attempt_id(&id),
        "attempt ids must satisfy their own syntax"
    );
    let staging = data_dir.join(staging_name(&id));
    let prev = data_dir.join(prev_name(&id));
    if staging.exists() || prev.exists() {
        return Err(rule(
            "generation-collision",
            &prev,
            "a publication working directory with this id already exists; refusing to reuse it",
        ));
    }

    // ---- 4. prepare ----
    fs::create_dir_all(&staging).map_err(io(&staging))?;
    let mut targets = Vec::new();
    for (name, staged_digest) in &staged {
        let from = staged_dir.join(name);
        paths::copy_file(&from, &staging.join(name))?;
        targets.push(JournalTarget {
            name: name.clone(),
            previous_digest: digest_if_present(&data_dir.join(name))?,
            staged_digest: staged_digest.clone(),
            retained: false,
            installed: false,
        });
    }
    let mut journal = Journal {
        schema: JOURNAL_SCHEMA.to_string(),
        id: id.clone(),
        phase: Phase::Prepared,
        staging: staging.to_string_lossy().to_string(),
        prev: prev.to_string_lossy().to_string(),
        data_dir: data_dir.to_string_lossy().to_string(),
        targets,
    };
    journal.write(data_dir, options.journal_fault())?;
    if options.fault == Some(Fault::AfterPrepareJournal) {
        return Err(rule(
            "injected-failure",
            data_dir,
            "interrupted after the prepare phase",
        ));
    }

    // ---- 5. retain ----
    // The phase is recorded as `Retaining` BEFORE the first move, so an interruption between a move
    // and its journal update is still inside a phase that recovery treats as "files may have moved".
    journal.phase = Phase::Retaining;
    journal.write(data_dir, options.journal_fault())?;
    fs::create_dir_all(&prev).map_err(io(&prev))?;
    for index in 0..journal.targets.len() {
        let name = journal.targets[index].name.clone();
        let published = data_dir.join(&name);
        if published.is_file() {
            paths::rename_file(&published, &prev.join(&name))?;
        }
        if options.fault == Some(Fault::AfterRetentionMove(index)) {
            return Err(rule(
                "injected-failure",
                data_dir,
                format!("interrupted after retention move {index}, before its journal update"),
            ));
        }
        journal.targets[index].retained = true;
        journal.write(data_dir, options.journal_fault())?;
        if options.fault == Some(Fault::AfterRetentionJournal(index)) {
            return Err(rule(
                "injected-failure",
                data_dir,
                format!("interrupted after retention journal update {index}"),
            ));
        }
    }
    journal.phase = Phase::Retained;
    journal.write(data_dir, options.journal_fault())?;

    // ---- 6. install ----
    journal.phase = Phase::Installing;
    journal.write(data_dir, options.journal_fault())?;
    let mut installed = Vec::new();
    for index in 0..journal.targets.len() {
        let name = journal.targets[index].name.clone();
        let from = staging.join(&name);
        paths::rename_file(&from, &data_dir.join(&name))?;
        if options.fault == Some(Fault::AfterInstallMove(index)) {
            return Err(rule(
                "injected-failure",
                data_dir,
                format!("interrupted after install move {index}, before its journal update"),
            ));
        }
        journal.targets[index].installed = true;
        journal.write(data_dir, options.journal_fault())?;
        if options.fault == Some(Fault::AfterInstallJournal(index)) {
            return Err(rule(
                "injected-failure",
                data_dir,
                format!("interrupted after install journal update {index}"),
            ));
        }
        installed.push(name);
    }

    // ---- 7. seal ----
    journal.phase = Phase::Completed;
    journal.write(data_dir, options.journal_fault())?;
    // R2: verified ownership, never a blind recursive delete.
    cleanup_owned_dir(&staging, "the staging directory")?;
    Ok(PublishReport {
        id,
        installed,
        retained_generation: prev,
    })
}

// -------------------------------------------------------------- recovery -----

/// The bytes actually at one location, established by hashing rather than by trusting a recorded
/// boolean.
///
/// **R1 correction.** An earlier ordered enum classified a file as *previous* or *staged*, first
/// match winning — which is wrong whenever the two generations share a byte-identical file (a
/// provenance sidecar that did not change, say). Such a file is legitimately **both**, and treating
/// it as only the first turned a completable install into a rollback. The predicates below are
/// therefore independent, and `foreign` means "matches neither".
#[derive(Debug, Clone, PartialEq, Eq)]
struct Observed {
    digest: Option<String>,
}

impl Observed {
    fn read(path: &Path) -> Result<Self> {
        Ok(Observed {
            digest: digest_if_present(path)?,
        })
    }

    fn is_absent(&self) -> bool {
        self.digest.is_none()
    }

    fn is_previous(&self, target: &JournalTarget) -> bool {
        self.digest == target.previous_digest
    }

    fn is_staged(&self, target: &JournalTarget) -> bool {
        self.digest.as_deref() == Some(target.staged_digest.as_str())
    }

    /// Present, but neither the recorded previous nor the staged content.
    fn is_foreign(&self, target: &JournalTarget) -> bool {
        !self.is_absent() && !self.is_previous(target) && !self.is_staged(target)
    }
}

/// What a recovery did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryReport {
    pub phase_found: String,
    pub action: String,
    pub files: Vec<String>,
    /// Material deliberately kept because it may still be needed.
    pub retained: Vec<PathBuf>,
}

/// `qa-evidence publish --recover`: complete or undo one interrupted publication.
///
/// The strategy comes from the phase; **every per-file action comes from hashing the bytes on disk**
/// against the journal's recorded digests. That is what makes a rename that completed before its
/// journal update recoverable, in either direction.
///
/// Fails closed, retaining all material, when it finds bytes it cannot account for — a modified
/// staged file, a modified retained file, or a published file that is neither the previous nor the
/// staged content.
pub fn recover(data_dir: &Path) -> Result<RecoveryReport> {
    let Some(mut journal) = Journal::read(data_dir)? else {
        return Ok(RecoveryReport {
            phase_found: "none".into(),
            action: "no journal; nothing to recover".into(),
            files: Vec::new(),
            retained: Vec::new(),
        });
    };
    if journal.phase == Phase::Completed {
        return Ok(RecoveryReport {
            phase_found: journal.phase.as_str().into(),
            action: "the transaction is already resolved; nothing to do".into(),
            files: Vec::new(),
            retained: vec![PathBuf::from(&journal.prev)],
        });
    }

    let staging = PathBuf::from(&journal.staging);
    let prev = PathBuf::from(&journal.prev);
    let phase_found = journal.phase.as_str().to_string();

    // Establish the actual state of every location before deciding anything, and refuse outright on
    // bytes that match neither the recorded previous nor the staged content.
    let mut state = Vec::new();
    for target in &journal.targets {
        let published = Observed::read(&data_dir.join(&target.name))?;
        let retained = Observed::read(&prev.join(&target.name))?;
        let staged = Observed::read(&staging.join(&target.name))?;
        for (observed, role, path) in [
            (&published, "published", data_dir.join(&target.name)),
            (&retained, "retained", prev.join(&target.name)),
            (&staged, "staged", staging.join(&target.name)),
        ] {
            if observed.is_foreign(target) {
                return Err(foreign(
                    &path,
                    role,
                    observed.digest.as_deref().unwrap_or("<absent>"),
                    target,
                    &staging,
                    &prev,
                ));
            }
        }
        state.push((published, retained, staged));
    }

    // Finishing is only possible from `Installing`, and only when every target's new bytes are
    // either already in place or still available in staging.
    let can_finish = journal.phase == Phase::Installing
        && journal
            .targets
            .iter()
            .enumerate()
            .all(|(i, target)| state[i].0.is_staged(target) || state[i].2.is_staged(target));

    journal.phase = if can_finish {
        Phase::Installing
    } else {
        Phase::RollingBack
    };
    journal.write(data_dir, None)?;

    let mut files = Vec::new();
    if can_finish {
        for (index, target) in journal.targets.clone().iter().enumerate() {
            if state[index].0.is_staged(target) {
                continue;
            }
            let from = staging.join(&target.name);
            paths::rename_file(&from, &data_dir.join(&target.name))?;
            journal.targets[index].installed = true;
            journal.write(data_dir, None)?;
            files.push(target.name.clone());
        }
        verify(data_dir, &journal, true)?;
        journal.phase = Phase::Completed;
        journal.write(data_dir, None)?;
        cleanup_owned_dir(&staging, "the staging directory")?;
        return Ok(RecoveryReport {
            phase_found,
            action: "completed the remaining installs".into(),
            files,
            retained: vec![prev],
        });
    }

    // Roll back: restore what existed, and REMOVE what did not.
    for (index, target) in journal.targets.clone().iter().enumerate() {
        let published_path = data_dir.join(&target.name);
        match &target.previous_digest {
            Some(_) => {
                if !state[index].0.is_previous(target) {
                    let from = prev.join(&target.name);
                    if !state[index].1.is_previous(target) {
                        return Err(rule(
                            "recovery-incomplete",
                            &from,
                            format!(
                                "{} must be restored, but its previous bytes are in neither the \
                                 published set nor the retained generation. Nothing further was \
                                 changed; the staged material at {} and the retained generation at \
                                 {} are kept for inspection.",
                                target.name,
                                staging.display(),
                                prev.display()
                            ),
                        ));
                    }
                    if published_path.exists() {
                        fs::remove_file(&published_path).map_err(io(&published_path))?;
                    }
                    paths::rename_file(&from, &published_path)?;
                    files.push(target.name.clone());
                }
            }
            None => {
                // The file did not exist before this publication. A rollback must delete it, not
                // leave a half-published new output behind.
                if published_path.exists() {
                    fs::remove_file(&published_path).map_err(io(&published_path))?;
                    files.push(format!("{} (removed — absent before)", target.name));
                }
            }
        }
    }

    verify(data_dir, &journal, false)?;
    journal.phase = Phase::Completed;
    journal.write(data_dir, None)?;
    // Only now, with the restored state verified, is the recovery material disposable — and even
    // then only after proving both directories hold nothing but expected output files (R2).
    cleanup_owned_dir(&staging, "the staging directory")?;
    cleanup_owned_dir(&prev, "the retained generation")?;
    fs::remove_file(Journal::path(data_dir)).ok();
    Ok(RecoveryReport {
        phase_found,
        action: "rolled back to the state before the publication".into(),
        files,
        retained: Vec::new(),
    })
}

fn foreign(
    path: &Path,
    role: &str,
    digest: &str,
    target: &JournalTarget,
    staging: &Path,
    prev: &Path,
) -> StoreError {
    rule(
        "foreign-bytes",
        path,
        format!(
            "the {role} copy of {} hashes to {digest}, which is neither its recorded previous \
             ({:?}) nor its staged ({}) content. Recovery stopped without changing anything; the \
             staged material at {} and the retained generation at {} are kept.",
            target.name,
            target.previous_digest,
            target.staged_digest,
            staging.display(),
            prev.display()
        ),
    )
}

/// Post-condition check: exact bytes **and** membership.
fn verify(data_dir: &Path, journal: &Journal, installed: bool) -> Result<()> {
    for target in &journal.targets {
        let path = data_dir.join(&target.name);
        let expected = if installed {
            Some(target.staged_digest.clone())
        } else {
            target.previous_digest.clone()
        };
        let actual = digest_if_present(&path)?;
        if actual != expected {
            return Err(rule(
                "recovery-verification",
                &path,
                format!(
                    "after recovery {} is {:?}, expected {:?}. All recovery material is retained.",
                    target.name, actual, expected
                ),
            ));
        }
    }
    // Membership: a rollback must leave no published output the journal says was absent.
    if !installed {
        for target in &journal.targets {
            if target.previous_digest.is_none() && data_dir.join(&target.name).exists() {
                return Err(rule(
                    "recovery-verification",
                    &data_dir.join(&target.name),
                    format!("{} should have been removed by the rollback", target.name),
                ));
            }
        }
    }
    Ok(())
}
