//! Reading and writing the evidence tree —
//! [contract §2.4/§2.5 and §3](../../../../docs/plans/qa-evidence-contract.md).
//!
//! ```text
//! <root>/auto/runs/<run_id>/{manifest.json,state.json,keys-declared.txt,keys-executed.txt,observations.csv}
//! <root>/manual/<target>/<family>/<record_id>.toml
//! <root>/legacy/claims/<claim_id>.toml
//! ```
//!
//! **R-SEP.** Nothing in this module writes under `manual/`. [`write_manual_record`] exists, is
//! `pub(crate)`, and is reachable only from
//! [`super::migrate::install`](super::migrate::install), which requires a matching approval digest.
//!
//! The three structural validation rules live here because they are statements about files:
//! **V3** (`run_id` equals its directory name), **V4** (manifest digests and counts match the key
//! and observation files) and **V11** ([`manual_digest`], the fingerprint a caller compares across a
//! generator invocation).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::csv;
use super::key::CoverageKey;
use super::model::{
    Facet, LegacyClaim, ManualRecord, ManualResult, Observation, Outcome, ProcedureKind, Run,
    RunManifest, RunState, SupersedeMode, UnmappedObservation,
};

/// Everything that can go wrong reading or writing the tree. Each variant names the file, because a
/// diagnostic that does not name the file is useless during recovery.
#[derive(Debug)]
pub enum StoreError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Malformed {
        path: PathBuf,
        detail: String,
    },
    /// A structural rule the bytes violate.
    Rule {
        rule: &'static str,
        path: PathBuf,
        detail: String,
    },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Io { path, source } => write!(f, "{}: {source}", path.display()),
            StoreError::Malformed { path, detail } => write!(f, "{}: {detail}", path.display()),
            StoreError::Rule { rule, path, detail } => {
                write!(f, "{rule} [{}]: {detail}", path.display())
            }
        }
    }
}

impl std::error::Error for StoreError {}

pub type Result<T> = std::result::Result<T, StoreError>;

fn io(path: &Path) -> impl Fn(std::io::Error) -> StoreError + '_ {
    move |source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn malformed(path: &Path, detail: impl Into<String>) -> StoreError {
    StoreError::Malformed {
        path: path.to_path_buf(),
        detail: detail.into(),
    }
}

/// `sha256:<hex>` over bytes. The `sha256:` prefix is part of the recorded value everywhere so a
/// digest can never be confused with a bare hex id.
pub fn sha256_prefixed(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Bare hex, for the manifest's `*_sha256` fields, which the contract shows unprefixed.
pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(io(path))
}

/// **R2:** every write goes through [`super::paths::write_file`], which validates the final
/// component as its own mutation target. A bare `fs::write` follows an existing child link.
fn write(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io(parent))?;
    }
    super::paths::write_file(path, contents.as_bytes())?;
    Ok(())
}

// ------------------------------------------------------------- manifest ------

/// The on-disk shape of `manifest.json` (`qa-run-manifest/1`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFile {
    pub schema: String,
    pub run_id: String,
    pub emitter: String,
    pub started_utc: String,
    pub host_os: String,
    pub toolchain: String,
    pub keys_declared_sha256: String,
    pub keys_executed_sha256: String,
    pub keys_declared_count: usize,
    pub keys_executed_count: usize,
    pub observations_sha256: String,
    pub scope_id: String,
    pub scope_version: u32,
    pub implementation_digest: String,
    #[serde(default)]
    pub input_digests: BTreeMap<String, String>,
    #[serde(default)]
    pub supersedes: Vec<String>,
    pub supersede_mode: String,
    #[serde(default)]
    pub supersede_keys: Vec<String>,
}

/// The on-disk shape of `state.json` (`qa-run-state/1`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateFile {
    pub schema: String,
    pub run_id: String,
    pub state: String,
    pub ended_utc: String,
    pub failure: Option<String>,
}

pub(crate) const OBSERVATION_HEADER: &[&str] = &[
    "schema",
    "key",
    "case_id",
    "outcome",
    "fixture_sha256",
    "output_sha256",
    "detail",
];

/// `unmapped.csv` — executed cases with no coverage key. Never qualifying, never merged into a key.
pub(crate) const UNMAPPED_HEADER: &[&str] = &[
    "schema",
    "target",
    "emitter_operation",
    "case_id",
    "outcome",
    "fixture_sha256",
    "unresolved",
    "detail",
];

/// `facets.csv` — structured descriptors of an observation (codec, container version, save mode).
pub(crate) const FACET_HEADER: &[&str] = &["schema", "case_id", "key", "facet", "value"];

// ------------------------------------------------------------- layer A -------

/// Load one run directory, enforcing V3 and V4.
///
/// **R-STATE**: a directory whose `state.json` is absent or unreadable is `interrupted`. The state
/// is never inferred from file presence, timestamps or counts, and an unreadable state file is never
/// treated as a completed run.
pub fn load_run(dir: &Path) -> Result<Run> {
    let manifest_path = dir.join("manifest.json");
    let manifest_text = read(&manifest_path)?;
    let manifest: ManifestFile = serde_json::from_str(&manifest_text)
        .map_err(|e| malformed(&manifest_path, format!("manifest.json did not parse: {e}")))?;
    if manifest.schema != "qa-run-manifest/1" {
        return Err(malformed(
            &manifest_path,
            format!("unsupported schema {:?}", manifest.schema),
        ));
    }

    // V3 — the directory name is the run id.
    let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    if dir_name != manifest.run_id {
        return Err(StoreError::Rule {
            rule: "V3",
            path: manifest_path.clone(),
            detail: format!(
                "run_id {:?} does not match directory {dir_name:?}",
                manifest.run_id
            ),
        });
    }

    let declared_path = dir.join("keys-declared.txt");
    let executed_path = dir.join("keys-executed.txt");
    let observations_path = dir.join("observations.csv");
    let declared_text = read(&declared_path)?;
    let executed_text = read(&executed_path)?;
    let observations_text = read(&observations_path)?;

    // V4 — digests and counts. The key files are the membership evidence; the counts are a derived
    // check over them, never a substitute.
    for (label, path, text, expected_digest, expected_count) in [
        (
            "keys_declared",
            &declared_path,
            &declared_text,
            &manifest.keys_declared_sha256,
            Some(manifest.keys_declared_count),
        ),
        (
            "keys_executed",
            &executed_path,
            &executed_text,
            &manifest.keys_executed_sha256,
            Some(manifest.keys_executed_count),
        ),
        (
            "observations",
            &observations_path,
            &observations_text,
            &manifest.observations_sha256,
            None,
        ),
    ] {
        let actual = sha256_hex(text.as_bytes());
        if &actual != expected_digest {
            return Err(StoreError::Rule {
                rule: "V4",
                path: path.to_path_buf(),
                detail: format!("{label}_sha256 is {expected_digest}, file hashes to {actual}"),
            });
        }
        if let Some(expected) = expected_count {
            let lines = text.lines().filter(|l| !l.trim().is_empty()).count();
            if lines != expected {
                return Err(StoreError::Rule {
                    rule: "V4",
                    path: path.to_path_buf(),
                    detail: format!("{label}_count is {expected}, file has {lines} line(s)"),
                });
            }
        }
    }

    let keys_declared = parse_key_file(&declared_path, &declared_text)?;
    let keys_executed = parse_key_file(&executed_path, &executed_text)?;

    let mut observations = Vec::new();
    for (i, record) in csv::parse_with_header(&observations_text, OBSERVATION_HEADER)
        .map_err(|e| malformed(&observations_path, e.to_string()))?
        .into_iter()
        .enumerate()
    {
        if record[0] != "qa-observation/1" {
            return Err(malformed(
                &observations_path,
                format!("row {} has unsupported schema {:?}", i + 2, record[0]),
            ));
        }
        let key = CoverageKey::parse(&record[1])
            .map_err(|e| malformed(&observations_path, format!("row {}: {e}", i + 2)))?;
        let outcome = Outcome::parse(&record[3]).ok_or_else(|| {
            malformed(
                &observations_path,
                format!("row {}: unknown outcome {:?}", i + 2, record[3]),
            )
        })?;
        observations.push(Observation {
            key,
            case_id: record[2].clone(),
            outcome,
            fixture_sha256: record[4].clone(),
            output_sha256: record[5].clone(),
            detail: record[6].clone(),
        });
    }

    // **R-STATE**, and R5's refinement of *how it is reported*. The contract says a run directory
    // with no **readable** `state.json` is `interrupted`, and that classification stands: it is the
    // one degradation in this module that can only ever *remove* qualification. An interrupted run
    // is never selectable, its observations are all retained and citable, and an executed failure in
    // it still triggers row 4a — so downgrading here cannot erase a failure or grant anything.
    //
    // What R5 fixes is the *reason*. `Err(_)` reported every cause as "no state.json", so a
    // directory, a link, a permission failure and invalid UTF-8 at that name were all indexed as an
    // absent terminal marker. Absence and unreadability have the same conservative consequence but
    // are not the same finding, and only one of them means "the emitter died before sealing".
    let state_path = dir.join("state.json");
    let (state, ended_utc, failure) =
        match super::paths::read_optional_file(&state_path, RUN_WRONG_TYPE, RUN_UNREADABLE) {
            Ok(Some(text)) => match serde_json::from_str::<StateFile>(&text) {
                Ok(s) => match RunState::parse(&s.state) {
                    // A state.json naming an unknown state is not a completed run.
                    Some(parsed) => (parsed, s.ended_utc, s.failure),
                    None => (
                        RunState::Interrupted,
                        String::new(),
                        Some(format!("unknown terminal state {:?}", s.state)),
                    ),
                },
                Err(e) => (
                    RunState::Interrupted,
                    String::new(),
                    Some(format!("state.json did not parse: {e}")),
                ),
            },
            Ok(None) => (
                RunState::Interrupted,
                String::new(),
                Some("no state.json".to_string()),
            ),
            Err(e) => (
                RunState::Interrupted,
                String::new(),
                Some(format!("state.json is present but unreadable: {e}")),
            ),
        };

    let supersede_mode = SupersedeMode::parse(&manifest.supersede_mode).ok_or_else(|| {
        malformed(
            &manifest_path,
            format!("unknown supersede_mode {:?}", manifest.supersede_mode),
        )
    })?;
    let mut supersede_keys = Vec::new();
    for raw in &manifest.supersede_keys {
        supersede_keys.push(
            CoverageKey::parse(raw)
                .map_err(|e| malformed(&manifest_path, format!("supersede_keys: {e}")))?,
        );
    }

    Ok(Run {
        manifest: RunManifest {
            run_id: manifest.run_id,
            emitter: manifest.emitter,
            started_utc: manifest.started_utc,
            host_os: manifest.host_os,
            toolchain: manifest.toolchain,
            scope_id: manifest.scope_id,
            scope_version: manifest.scope_version,
            implementation_digest: manifest.implementation_digest,
            input_digests: manifest.input_digests,
            supersedes: manifest.supersedes,
            supersede_mode,
            supersede_keys,
        },
        state,
        ended_utc,
        failure,
        keys_declared,
        keys_executed,
        observations,
        unmapped: read_unmapped(&dir.join("unmapped.csv"))?,
        facets: read_facets(&dir.join("facets.csv"))?,
    })
}

/// The rule token for a run file that exists but is not a regular file.
pub const RUN_WRONG_TYPE: &str = "run-unexpected-entry-type";
/// The rule token for a run file that exists and cannot be decoded.
pub const RUN_UNREADABLE: &str = "run-unreadable-file";

/// `unmapped.csv`, or an empty list when the file is **absent** — a run written before this file
/// existed is not malformed, it simply carries no unmapped records. That backward compatibility is
/// the whole justification for the fallback, and it applies to absence alone.
///
/// **R5:** the previous `let Ok(text) = … else { return Ok(Vec::new()) }` extended it to every
/// failure. An unreadable file, invalid UTF-8, a directory or a link at the name all reported "this
/// run made no unmapped observations" — a positive claim about the run, manufactured from an error.
pub(crate) fn read_unmapped(path: &Path) -> Result<Vec<UnmappedObservation>> {
    let Some(text) = super::paths::read_optional_file(path, RUN_WRONG_TYPE, RUN_UNREADABLE)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (i, record) in csv::parse_with_header(&text, UNMAPPED_HEADER)
        .map_err(|e| malformed(path, e.to_string()))?
        .into_iter()
        .enumerate()
    {
        let outcome = Outcome::parse(&record[4]).ok_or_else(|| {
            malformed(
                path,
                format!("row {}: unknown outcome {:?}", i + 1, record[4]),
            )
        })?;
        out.push(UnmappedObservation {
            target: record[1].clone(),
            emitter_operation: record[2].clone(),
            case_id: record[3].clone(),
            outcome,
            fixture_sha256: record[5].clone(),
            unresolved: record[6].clone(),
            detail: record[7].clone(),
        });
    }
    Ok(out)
}

/// `facets.csv`, or an empty list when the file is absent.
pub(crate) fn read_facets(path: &Path) -> Result<Vec<Facet>> {
    let Some(text) = super::paths::read_optional_file(path, RUN_WRONG_TYPE, RUN_UNREADABLE)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for record in
        csv::parse_with_header(&text, FACET_HEADER).map_err(|e| malformed(path, e.to_string()))?
    {
        out.push(Facet {
            case_id: record[1].clone(),
            key: record[2].clone(),
            name: record[3].clone(),
            value: record[4].clone(),
        });
    }
    Ok(out)
}

fn parse_key_file(path: &Path, text: &str) -> Result<BTreeSet<CoverageKey>> {
    let mut out = BTreeSet::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let key = CoverageKey::parse(line)
            .map_err(|e| malformed(path, format!("line {}: {e}", i + 1)))?;
        out.insert(key);
    }
    Ok(out)
}

/// Entry names permitted alongside run directories in `<root>/auto/runs`.
///
/// Deliberately empty, and asserted so by test: nothing writes a file at that level. The list exists
/// so that permitting one later is a recorded decision rather than a widened filter predicate.
pub const PERMITTED_RUNS_ENTRIES: &[&str] = &[];

/// Load every run under `<root>/auto/runs`, sorted by run id. A root with no `auto/runs` directory
/// yields no runs — which is **R-ABSENT**: absence asserts nothing and erases nothing.
pub fn load_runs(root: &Path) -> Result<Vec<Run>> {
    let runs_dir = root.join("auto").join("runs");
    // R4: `child_dirs` distinguishes "no runs directory" from an unreadable, wrong-type or
    // link-aliased one, propagates enumeration and metadata failures, and refuses a stray entry
    // rather than filtering it away. `!is_dir()` answered "absent" to all of those.
    let dirs = super::paths::child_dirs(&runs_dir, PERMITTED_RUNS_ENTRIES)?;
    dirs.iter().map(|d| load_run(d)).collect()
}

/// Write one immutable run directory. Refuses if the directory already exists: a run is created,
/// populated and sealed, never edited (contract §3.1).
///
/// `state.json` is written **last**, so an interrupted write leaves a directory that classifies as
/// `interrupted` rather than as a completed run.
pub fn write_run(root: &Path, run: &Run) -> Result<PathBuf> {
    let dir = root.join("auto").join("runs").join(run.run_id());
    if dir.exists() {
        return Err(StoreError::Rule {
            rule: "immutable-run",
            path: dir,
            detail: "run directory already exists; runs are write-once".into(),
        });
    }
    fs::create_dir_all(&dir).map_err(io(&dir))?;

    let declared = key_file_text(&run.keys_declared);
    let executed = key_file_text(&run.keys_executed);
    let observations = observations_text(run);
    write(&dir.join("keys-declared.txt"), &declared)?;
    write(&dir.join("keys-executed.txt"), &executed)?;
    write(&dir.join("observations.csv"), &observations)?;
    write(&dir.join("unmapped.csv"), &unmapped_text(&run.unmapped))?;
    write(&dir.join("facets.csv"), &facets_text(&run.facets))?;

    let manifest = ManifestFile {
        schema: "qa-run-manifest/1".into(),
        run_id: run.manifest.run_id.clone(),
        emitter: run.manifest.emitter.clone(),
        started_utc: run.manifest.started_utc.clone(),
        host_os: run.manifest.host_os.clone(),
        toolchain: run.manifest.toolchain.clone(),
        keys_declared_sha256: sha256_hex(declared.as_bytes()),
        keys_executed_sha256: sha256_hex(executed.as_bytes()),
        keys_declared_count: run.keys_declared.len(),
        keys_executed_count: run.keys_executed.len(),
        observations_sha256: sha256_hex(observations.as_bytes()),
        scope_id: run.manifest.scope_id.clone(),
        scope_version: run.manifest.scope_version,
        implementation_digest: run.manifest.implementation_digest.clone(),
        input_digests: run.manifest.input_digests.clone(),
        supersedes: run.manifest.supersedes.clone(),
        supersede_mode: run.manifest.supersede_mode.as_str().into(),
        supersede_keys: run
            .manifest
            .supersede_keys
            .iter()
            .map(ToString::to_string)
            .collect(),
    };
    let manifest_path = dir.join("manifest.json");
    write(
        &manifest_path,
        &serde_json::to_string_pretty(&manifest)
            .map_err(|e| malformed(&manifest_path, e.to_string()))?,
    )?;

    // Written last — R-STATE.
    let state = StateFile {
        schema: "qa-run-state/1".into(),
        run_id: run.manifest.run_id.clone(),
        state: run.state.as_str().into(),
        ended_utc: run.ended_utc.clone(),
        failure: run.failure.clone(),
    };
    let state_path = dir.join("state.json");
    write(
        &state_path,
        &serde_json::to_string(&state).map_err(|e| malformed(&state_path, e.to_string()))?,
    )?;
    Ok(dir)
}

/// Key-sorted for determinism (R-ORDER); row order carries no meaning.
pub(crate) fn unmapped_text(records: &[UnmappedObservation]) -> String {
    let mut rows: Vec<&UnmappedObservation> = records.iter().collect();
    rows.sort();
    let mut out = csv::row(UNMAPPED_HEADER);
    for r in rows {
        out.push_str(&csv::row([
            "qa-unmapped-observation/1",
            &r.target,
            &r.emitter_operation,
            &r.case_id,
            r.outcome.as_str(),
            &r.fixture_sha256,
            &r.unresolved,
            &r.detail,
        ]));
    }
    out
}

pub(crate) fn facets_text(facets: &[Facet]) -> String {
    let mut rows: Vec<&Facet> = facets.iter().collect();
    rows.sort();
    let mut out = csv::row(FACET_HEADER);
    for f in rows {
        out.push_str(&csv::row([
            "qa-observation-facet/1",
            &f.case_id,
            &f.key,
            &f.name,
            &f.value,
        ]));
    }
    out
}

fn key_file_text(keys: &BTreeSet<CoverageKey>) -> String {
    let mut out = String::new();
    for key in keys {
        out.push_str(&key.to_string());
        out.push('\n');
    }
    out
}

fn observations_text(run: &Run) -> String {
    let mut out = csv::row(OBSERVATION_HEADER);
    // Key-sorted, then case id: row order carries no meaning, so it is fixed for determinism.
    let mut rows: Vec<&Observation> = run.observations.iter().collect();
    rows.sort_by(|a, b| (&a.key, &a.case_id).cmp(&(&b.key, &b.case_id)));
    for o in rows {
        out.push_str(&csv::row([
            "qa-observation/1",
            &o.key.to_string(),
            &o.case_id,
            o.outcome.as_str(),
            &o.fixture_sha256,
            &o.output_sha256,
            &o.detail,
        ]));
    }
    out
}

// ------------------------------------------------------------- layer B -------

/// Load every manual record under `<root>/manual`, sorted by record id.
pub fn load_manual(root: &Path) -> Result<Vec<ManualRecord>> {
    let dir = root.join("manual");
    let mut out = Vec::new();
    for path in walk_toml(&dir)? {
        out.push(parse_manual(&path, &read(&path)?)?);
    }
    out.sort_by(|a, b| a.record_id.cmp(&b.record_id));
    Ok(out)
}

/// Every `.toml` record under `dir`, depth-first in sorted order.
///
/// **The file-selection rule is unchanged** (R4): a record is a file whose extension is `toml`, and
/// nothing else is parsed as one. What changed is what happens to the rest of the tree — the walk
/// itself is [`super::paths::walk_files`], so a link, an unreadable entry, a failed enumeration or
/// an entry that is neither file nor directory is an error instead of a silent omission.
///
/// A non-`.toml` file is *excluded from parsing, not from sight*: `walk_all` includes it in the V11
/// [`manual_digest`], so it cannot be added, edited or removed without the digest changing. That is
/// what keeps discovery and digest agreeing about the tree while still refusing to treat a README
/// as active evidence.
fn walk_toml(dir: &Path) -> Result<Vec<PathBuf>> {
    Ok(super::paths::walk_files(dir)?
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .collect())
}

fn str_at(value: &toml::Value, path: &[&str]) -> String {
    let mut current = value;
    for part in path {
        match current.get(part) {
            Some(next) => current = next,
            None => return "unknown".to_string(),
        }
    }
    current.as_str().unwrap_or("unknown").to_string()
}

/// Parse one layer-B TOML record. **R-UNKNOWN**: an absent field becomes the literal `"unknown"`,
/// never a default, a back-date or an inference.
pub fn parse_manual(path: &Path, text: &str) -> Result<ManualRecord> {
    let value: toml::Value = text
        .parse()
        .map_err(|e| malformed(path, format!("TOML: {e}")))?;
    let schema = str_at(&value, &["schema"]);
    if schema != "qa-manual-record/1" {
        return Err(malformed(path, format!("unsupported schema {schema:?}")));
    }
    let key_raw = str_at(&value, &["key"]);
    let key = CoverageKey::parse(&key_raw).map_err(|e| malformed(path, e.to_string()))?;
    let result_raw = str_at(&value, &["result"]);
    let result = ManualResult::parse(&result_raw)
        .ok_or_else(|| malformed(path, format!("unknown result {result_raw:?}")))?;
    let kind_raw = str_at(&value, &["procedure", "kind"]);
    let procedure_kind = ProcedureKind::parse(&kind_raw)
        .ok_or_else(|| malformed(path, format!("unknown procedure.kind {kind_raw:?}")))?;

    let artifacts = value
        .get("artifact")
        .and_then(|a| a.as_array())
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    (
                        str_at(item, &["role"]),
                        str_at(item, &["path"]),
                        str_at(item, &["sha256"]),
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(ManualRecord {
        record_id: str_at(&value, &["record_id"]),
        key,
        result,
        example: value
            .get("example")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false),
        procedure_kind,
        procedure_reference: str_at(&value, &["procedure", "reference"]),
        procedure_evidence: string_list(&value, &["procedure", "evidence"]),
        performed_by: str_at(&value, &["performed", "by"]),
        date_raw: str_at(&value, &["performed", "date_raw"]),
        date_precision: str_at(&value, &["performed", "date_precision"]),
        runtime: str_at(&value, &["target", "runtime"]),
        runtime_build: str_at(&value, &["target", "runtime_build"]),
        game_build: str_at(&value, &["target", "game_build"]),
        studio_commit: str_at(&value, &["source", "studio_commit"]),
        implementation_digest: str_at(&value, &["source", "implementation_digest"]),
        scope_id: str_at(&value, &["source", "scope_id"]),
        scope_version: value
            .get("source")
            .and_then(|s| s.get("scope_version"))
            .and_then(toml::Value::as_integer)
            .unwrap_or(0) as u32,
        artifacts,
        origin: str_at(&value, &["provenance", "origin"]),
        supersedes: string_list(&value, &["supersedes"]),
        supersede_reason: str_at(&value, &["supersede_reason"]),
    })
}

fn string_list(value: &toml::Value, path: &[&str]) -> Vec<String> {
    let mut current = value;
    for part in path {
        match current.get(part) {
            Some(next) => current = next,
            None => return Vec::new(),
        }
    }
    current
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// **V11**: a fingerprint over every file under `<root>/manual`, path and content. A caller takes it
/// before and after a generator invocation; any difference is a violation of R-SEP.
pub fn manual_digest(root: &Path) -> Result<String> {
    let dir = root.join("manual");
    let mut hasher = Sha256::new();
    for path in walk_all(&dir)? {
        let rel = path
            .strip_prefix(&dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        hasher.update(rel.as_bytes());
        hasher.update([0]);
        hasher.update(fs::read(&path).map_err(io(&path))?);
        hasher.update([0]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

/// Every file under `dir`, whatever its name. The V11 digest's input, and the superset of
/// [`walk_toml`]'s — the same traversal, so the two can never disagree about what the tree holds.
fn walk_all(dir: &Path) -> Result<Vec<PathBuf>> {
    super::paths::walk_files(dir)
}

/// The only writer into layer B. `pub(crate)` and reachable only through an approved migration
/// install (R-MIG-SEP); nothing else in the crate calls it.
pub(crate) fn write_manual_record(
    root: &Path,
    record: &ManualRecord,
    toml_text: &str,
) -> Result<PathBuf> {
    let path = root
        .join("manual")
        .join(&record.key.target)
        .join(&record.key.family)
        .join(format!("{}.toml", record.record_id));
    // R2: `symlink_metadata`, not `exists` — a dangling link at this name reports itself absent,
    // and writing "through" it would create the file the link points at, outside `manual/`.
    if fs::symlink_metadata(&path).is_ok() {
        return Err(StoreError::Rule {
            rule: "R-SEP",
            path,
            detail: "a manual record file is never overwritten".into(),
        });
    }
    write(&path, toml_text)?;
    Ok(path)
}

// ------------------------------------------------------------- layer L -------

/// Load every legacy claim under `<root>/legacy/claims`, sorted by claim id.
///
/// **R-IMPORT-SOFT**: layer L is not an active input. A field this parser cannot map becomes an
/// `unresolved` token on the claim rather than a hard error — except a claim that is not TOML at
/// all, which cannot be represented and is reported.
pub fn load_legacy(root: &Path) -> Result<Vec<LegacyClaim>> {
    let dir = root.join("legacy").join("claims");
    let mut out = Vec::new();
    for path in walk_toml(&dir)? {
        out.push(parse_legacy(&path, &read(&path)?)?);
    }
    out.sort_by(|a, b| a.claim_id.cmp(&b.claim_id));
    Ok(out)
}

/// Parse one layer-L claim.
pub fn parse_legacy(path: &Path, text: &str) -> Result<LegacyClaim> {
    let value: toml::Value = text
        .parse()
        .map_err(|e| malformed(path, format!("TOML: {e}")))?;
    let schema = str_at(&value, &["schema"]);
    if schema != "qa-legacy-claim/1" {
        return Err(malformed(path, format!("unsupported schema {schema:?}")));
    }
    let opt = |v: &toml::Value, keys: &[&str]| -> Option<String> {
        let mut current = v;
        for k in keys {
            current = current.get(k)?;
        }
        current.as_str().map(str::to_string)
    };
    Ok(LegacyClaim {
        claim_id: str_at(&value, &["claim_id"]),
        source_file: str_at(&value, &["source_file"]),
        source_sha256: str_at(&value, &["source_sha256"]),
        source_row: value
            .get("source_row")
            .and_then(toml::Value::as_integer)
            .unwrap_or(0) as usize,
        source_row_raw: str_at(&value, &["source_row_raw"]),
        claim_status_raw: str_at(&value, &["claim_status_raw"]),
        claim_last_verified_raw: str_at(&value, &["claim_last_verified_raw"]),
        claim_evidence_raw: str_at(&value, &["claim_evidence_raw"]),
        maintenance: str_at(&value, &["maintenance"]),
        target: opt(&value, &["mapping", "target"]),
        family: opt(&value, &["mapping", "family"]),
        family_source: str_at(&value, &["mapping", "family_source"]),
        confidence: str_at(&value, &["mapping", "confidence"]),
        operation: str_at(&value, &["mapping", "operation"]),
        unresolved: string_list(&value, &["mapping", "unresolved"]),
    })
}

// ------------------------------------------------------- freshness inputs ----

/// Collect `(repo-relative slash path, sha256)` for every file a scope definition could match.
///
/// Only the directories the definitions actually name are walked, so this never scans the whole
/// repository and never leaves it. A glob that matches nothing simply contributes nothing here, and
/// [`super::scopes::Scopes::digest`] then reports `unknown` for that scope — a missing input is
/// never silently dropped.
pub fn collect_scope_contents(
    repo_root: &Path,
    scopes: &super::scopes::Scopes,
) -> Result<BTreeMap<String, String>> {
    let mut roots: BTreeSet<String> = BTreeSet::new();
    for scope in &scopes.scopes {
        for glob in scope.globs() {
            let base = glob.trim_end_matches("/**").trim_end_matches("/*");
            roots.insert(base.to_string());
        }
    }
    let mut out = BTreeMap::new();
    for rel in roots {
        let path = repo_root.join(&rel);
        // R4: a scope root that does not exist contributes nothing — that is the documented rule,
        // and `Scopes::digest` then reports the scope `unknown`. But a root that *does* exist and
        // cannot be classified is a different thing entirely: the old `is_file()`/`is_dir()` pair
        // answered "neither" for an unreadable or link-aliased root and dropped its contents from
        // the digest **without changing it**, which would have made a stale scope look fresh.
        let kind = match fs::symlink_metadata(&path) {
            Ok(metadata) => Some(super::paths::entry_kind_of(&path, &metadata)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(source) => return Err(StoreError::Io { path, source }),
        };
        if kind == Some(super::paths::EntryKind::File) {
            out.insert(
                rel.clone(),
                sha256_hex(&fs::read(&path).map_err(io(&path))?),
            );
        } else if kind == Some(super::paths::EntryKind::Dir) {
            for file in walk_all(&path)? {
                let suffix = file
                    .strip_prefix(&path)
                    .unwrap_or(&file)
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(
                    format!("{rel}/{suffix}"),
                    sha256_hex(&fs::read(&file).map_err(io(&file))?),
                );
            }
        }
    }
    Ok(out)
}

/// What one fixture-verification pass did, for reporting (C0-EMIT1-R2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FixtureVerification {
    /// Current state per attributed repository-relative path, exactly as the runs name them.
    pub states: BTreeMap<String, super::model::InputState>,
    /// Distinct files whose bytes were actually hashed in this pass.
    pub files_hashed: usize,
    pub bytes_hashed: u64,
}

/// Hash every fixture the given runs attribute in their `input_digests`, **once per resolved
/// file per invocation**, and classify what stands at each path now (C0-EMIT1-R2, R3).
///
/// Only explicitly referenced inputs are read — never a directory walk — and nothing but the
/// bytes is trusted: no manifest, timestamp or size stands in for content. Only a readable
/// regular file reached through plain directories under the repository is `Present`:
///
/// * the repository root itself must canonicalize, or nothing is verified (fail closed);
/// * an attributed path must be repository-relative — no absolute path, prefix, root or `..`;
/// * **every** component below the repository root is classified without following links
///   (`symlink_metadata` + [`super::paths::entry_kind_of`]): a symlink, junction or other
///   reparse point at any ancestor or at the file itself is refused, even one that redirects to
///   another location inside the repository; an ancestor that is not a directory is refused;
/// * the final entry must be a regular file whose canonical path lies under the repository
///   root, and a canonicalization failure is refused rather than approximated.
///
/// Every refusal is `Unreadable` with its reason; `Missing` is the one reading of absence. None
/// of these is an error for the caller: a missing or unreadable anchor is a legitimate state the
/// merge reports as not-fresh, not a reason to refuse the build. The classification and the read
/// are separate operations, so this is discovery-time verification, not a race-proof guarantee
/// against a writer that changes the file between the two.
pub fn verify_fixture_inputs(repo_root: &Path, runs: &[Run]) -> FixtureVerification {
    use super::model::InputState;
    use super::paths::{entry_kind_of, EntryKind};
    use std::path::Component;

    let mut states: BTreeMap<String, InputState> = BTreeMap::new();
    let mut by_resolved: BTreeMap<PathBuf, InputState> = BTreeMap::new();
    let mut files_hashed = 0usize;
    let mut bytes_hashed = 0u64;

    let repo = match repo_root.canonicalize() {
        Ok(repo) => repo,
        Err(e) => {
            // Fail closed: with no trustworthy root nothing can be attributed to it.
            for run in runs {
                for rel in run.manifest.input_digests.keys() {
                    states.entry(rel.clone()).or_insert_with(|| {
                        InputState::Unreadable(format!("repository root cannot be resolved: {e}"))
                    });
                }
            }
            return FixtureVerification {
                states,
                files_hashed,
                bytes_hashed,
            };
        }
    };

    for run in runs {
        for rel in run.manifest.input_digests.keys() {
            if states.contains_key(rel) {
                continue;
            }
            let state = classify_fixture(&repo, rel, &mut by_resolved, |bytes| {
                files_hashed += 1;
                bytes_hashed += bytes as u64;
            });
            states.insert(rel.clone(), state);
        }
    }

    /// One attributed path, walked component by component below the canonical repository root.
    fn classify_fixture(
        repo: &Path,
        rel: &str,
        by_resolved: &mut BTreeMap<PathBuf, InputState>,
        mut on_hashed: impl FnMut(usize),
    ) -> InputState {
        let relative = Path::new(rel);
        let components: Vec<Component> = relative.components().collect();
        if components.is_empty()
            || relative.is_absolute()
            || components
                .iter()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return InputState::Unreadable("not a repository-relative path".into());
        }
        let mut current = repo.to_path_buf();
        let last = components.len() - 1;
        for (i, component) in components.iter().enumerate() {
            current.push(component);
            let kind = match fs::symlink_metadata(&current) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return InputState::Missing;
                }
                Err(e) => return InputState::Unreadable(format!("metadata: {e}")),
                // `entry_kind_of` refuses every reparse point (symlink, junction, mount point)
                // before classifying, so an aliased ancestor never gets a chance to redirect.
                Ok(metadata) => match entry_kind_of(&current, &metadata) {
                    Ok(kind) => kind,
                    Err(e) => return InputState::Unreadable(e.to_string()),
                },
            };
            match (i == last, kind) {
                (false, EntryKind::Dir) => {}
                (false, _) => {
                    return InputState::Unreadable(format!(
                        "ancestor {} is not a directory",
                        current.display()
                    ))
                }
                (true, EntryKind::File) => {}
                (true, EntryKind::Dir) => {
                    return InputState::Unreadable("a directory, not a file".into())
                }
                (true, EntryKind::Other) => {
                    return InputState::Unreadable("not a regular file".into())
                }
            }
        }
        let resolved = match current.canonicalize() {
            Ok(resolved) => resolved,
            Err(e) => return InputState::Unreadable(format!("cannot be resolved: {e}")),
        };
        if !resolved.starts_with(repo) {
            return InputState::Unreadable("resolves outside the repository".into());
        }
        if let Some(seen) = by_resolved.get(&resolved) {
            return seen.clone();
        }
        let state = match fs::read(&current) {
            Ok(bytes) => {
                on_hashed(bytes.len());
                InputState::Present(sha256_hex(&bytes))
            }
            Err(e) => InputState::Unreadable(format!("read: {e}")),
        };
        by_resolved.insert(resolved, state.clone());
        state
    }

    FixtureVerification {
        states,
        files_hashed,
        bytes_hashed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::testkit::*;

    /// A synthetic directory unique per process and per call. Never a repository or game path.
    fn tmp(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir()
            .join("tdb-qa-evidence")
            .join(format!("{name}-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn a_run_round_trips_through_the_filesystem() {
        let root = tmp("roundtrip");
        let run = run_pass(
            "emitter-20260907t000000z-aaaaaaaa",
            &["nhl15-ps3/save-raw-db/save"],
        );
        write_run(&root, &run).expect("write");
        let loaded = load_runs(&root).expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].keys_declared, run.keys_declared);
        assert_eq!(loaded[0].observations, run.observations);
        assert_eq!(loaded[0].state, RunState::Completed);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_run_directory_is_write_once() {
        let root = tmp("writeonce");
        let run = run_pass("e-1", &["nhl15-ps3/save-raw-db/save"]);
        write_run(&root, &run).expect("first write");
        let err = write_run(&root, &run).expect_err("second write must refuse");
        assert!(matches!(
            err,
            StoreError::Rule {
                rule: "immutable-run",
                ..
            }
        ));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a16_a_directory_without_state_json_loads_as_interrupted() {
        let root = tmp("nostate");
        let run = run_pass("e-2", &["nhl15-ps3/save-raw-db/save"]);
        let dir = write_run(&root, &run).expect("write");
        fs::remove_file(dir.join("state.json")).expect("remove");
        let loaded = load_run(&dir).expect("load");
        assert_eq!(loaded.state, RunState::Interrupted);
        // The observations survive and stay citable.
        assert_eq!(loaded.observations.len(), 1);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn v3_and_v4_are_enforced_on_load() {
        let root = tmp("v3v4");
        let run = run_pass("e-3", &["nhl15-ps3/save-raw-db/save"]);
        let dir = write_run(&root, &run).expect("write");

        // V4 — edit a key file without updating the manifest.
        fs::write(
            dir.join("keys-declared.txt"),
            "nhl15-ps3/save-raw-db/reopen\n",
        )
        .expect("write");
        let err = load_run(&dir).expect_err("V4");
        assert!(matches!(err, StoreError::Rule { rule: "V4", .. }), "{err}");

        // V3 — rename the directory.
        let renamed = root.join("auto").join("runs").join("different-name");
        fs::rename(&dir, &renamed).expect("rename");
        let err = load_run(&renamed).expect_err("V3");
        assert!(matches!(err, StoreError::Rule { rule: "V3", .. }), "{err}");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_absent_runs_directory_yields_no_runs_rather_than_an_error() {
        let root = tmp("absent");
        assert_eq!(load_runs(&root).expect("load").len(), 0);
        assert_eq!(load_manual(&root).expect("load").len(), 0);
        assert_eq!(load_legacy(&root).expect("load").len(), 0);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_absent_manual_field_becomes_the_literal_unknown() {
        let text = r#"
schema = "qa-manual-record/1"
record_id = "r1"
key = "nhl15-xbox360/assets-portraits/install"
result = "pass"

[procedure]
kind = "operation-check"
"#;
        let record = parse_manual(Path::new("synthetic.toml"), text).expect("parses");
        assert_eq!(record.runtime, "unknown");
        assert_eq!(record.date_raw, "unknown");
        assert_eq!(record.studio_commit, "unknown");
        assert!(!record.example);
    }
}
