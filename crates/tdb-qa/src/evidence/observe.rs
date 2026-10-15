//! The emitter-facing API for writing one immutable layer-A run —
//! [contract §3.1 and §7.1](../../../../docs/plans/qa-evidence-contract.md).
//!
//! An emitter *declares* every key it intends to cover, then records what it actually did. The two
//! sets are written as separate files and **are** the membership evidence; the manifest counts are a
//! derived check over them (V4). A key the emitter does not cover is expressed by **absence**, never
//! by an observation row (§3.1).
//!
//! Two properties are structural rather than conventional:
//!
//! * [`RunBuilder::observe`] refuses an undeclared key, so V6 cannot be violated by a caller;
//! * [`RunBuilder::write`] takes the terminal state as an argument and `state.json` is written last,
//!   so a crashed emitter leaves a directory that classifies `interrupted` (**R-STATE**) rather than
//!   a run that looks completed.
//!
//! Timestamps passed in here are recorded metadata only. **R-NO-TIME** forbids any decision reading
//! them, which is also why this module never calls a clock: the caller supplies the value.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::key::CoverageKey;
use super::model::{
    Facet, Observation, Outcome, Run, RunManifest, RunState, SupersedeMode, UnmappedObservation,
};
use super::store::{self, StoreError};

/// Accumulates one run. Nothing is written until [`RunBuilder::write`].
#[derive(Debug, Clone)]
pub struct RunBuilder {
    emitter: String,
    started_utc: String,
    host_os: String,
    toolchain: String,
    scope_id: String,
    scope_version: u32,
    implementation_digest: String,
    input_digests: BTreeMap<String, String>,
    supersedes: Vec<String>,
    supersede_mode: SupersedeMode,
    supersede_keys: Vec<CoverageKey>,
    declared: BTreeSet<CoverageKey>,
    executed: BTreeSet<CoverageKey>,
    observations: Vec<Observation>,
    unmapped: Vec<UnmappedObservation>,
    facets: Vec<Facet>,
}

/// A caller error the builder refuses rather than writing an invalid run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObserveError {
    /// An observation on a key the run never declared (V6).
    Undeclared(String),
}

impl std::fmt::Display for ObserveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObserveError::Undeclared(k) => {
                write!(
                    f,
                    "observation on undeclared key {k}; declare it or leave it absent"
                )
            }
        }
    }
}

impl std::error::Error for ObserveError {}

impl RunBuilder {
    /// Start a run. `implementation_digest` is the freshness digest of `scope_id`/`scope_version`;
    /// pass the literal `"unknown"` when it cannot be computed — never a placeholder value.
    pub fn new(
        emitter: &str,
        started_utc: &str,
        scope_id: &str,
        scope_version: u32,
        implementation_digest: &str,
    ) -> Self {
        RunBuilder {
            emitter: emitter.to_string(),
            started_utc: started_utc.to_string(),
            host_os: std::env::consts::OS.to_string(),
            toolchain: "unknown".to_string(),
            scope_id: scope_id.to_string(),
            scope_version,
            implementation_digest: implementation_digest.to_string(),
            input_digests: BTreeMap::new(),
            supersedes: Vec::new(),
            supersede_mode: SupersedeMode::None,
            supersede_keys: Vec::new(),
            declared: BTreeSet::new(),
            executed: BTreeSet::new(),
            observations: Vec::new(),
            unmapped: Vec::new(),
            facets: Vec::new(),
        }
    }

    pub fn toolchain(mut self, toolchain: &str) -> Self {
        self.toolchain = toolchain.to_string();
        self
    }

    pub fn input_digest(mut self, name: &str, digest: &str) -> Self {
        self.input_digests
            .insert(name.to_string(), digest.to_string());
        self
    }

    /// Supersede an earlier run in full. The superset rule (V8) is checked at load time over the
    /// whole tree, which is the only place both key sets are available.
    pub fn supersedes_fully(mut self, run_id: &str) -> Self {
        self.supersedes.push(run_id.to_string());
        self.supersede_mode = SupersedeMode::Full;
        self
    }

    /// Supersede an earlier run **only at the listed keys** (R-PARTIAL-KEYS).
    pub fn supersedes_partially(mut self, run_id: &str, keys: &[CoverageKey]) -> Self {
        self.supersedes.push(run_id.to_string());
        self.supersede_mode = SupersedeMode::Partial;
        self.supersede_keys.extend(keys.iter().cloned());
        self
    }

    /// Declare a key this run intends to cover.
    pub fn declare(&mut self, key: CoverageKey) -> &mut Self {
        self.declared.insert(key);
        self
    }

    /// Record what a case did. Refuses an undeclared key (V6).
    pub fn observe(
        &mut self,
        key: &CoverageKey,
        case_id: &str,
        outcome: Outcome,
        detail: &str,
    ) -> Result<&mut Self, ObserveError> {
        if !self.declared.contains(key) {
            return Err(ObserveError::Undeclared(key.to_string()));
        }
        if outcome.is_executed() {
            self.executed.insert(key.clone());
        }
        self.observations.push(Observation {
            key: key.clone(),
            case_id: case_id.to_string(),
            outcome,
            fixture_sha256: String::new(),
            output_sha256: String::new(),
            detail: detail.to_string(),
        });
        Ok(self)
    }

    /// As [`RunBuilder::observe`], with the fixture and output hashes an operation produced.
    pub fn observe_hashed(
        &mut self,
        key: &CoverageKey,
        case_id: &str,
        outcome: Outcome,
        fixture_sha256: &str,
        output_sha256: &str,
        detail: &str,
    ) -> Result<&mut Self, ObserveError> {
        self.observe(key, case_id, outcome, detail)?;
        if let Some(last) = self.observations.last_mut() {
            last.fixture_sha256 = fixture_sha256.to_string();
            last.output_sha256 = output_sha256.to_string();
        }
        Ok(self)
    }

    /// Record an executed case whose emitter operation has **no** honest key in the closed §4.1
    /// vocabulary. It never becomes an [`Observation`], so it can never qualify anything; the point
    /// is that the execution survives instead of being lost or forced onto an adjacent key.
    pub fn observe_unmapped(&mut self, record: UnmappedObservation) -> &mut Self {
        self.unmapped.push(record);
        self
    }

    /// Attach a structured descriptor to an observation or unmapped record — `codec=v4`,
    /// `save-mode=roster`. Descriptive only: no facet is read by any merge decision.
    pub fn facet(
        &mut self,
        case_id: &str,
        key: Option<&CoverageKey>,
        name: &str,
        value: &str,
    ) -> &mut Self {
        self.facets.push(Facet {
            case_id: case_id.to_string(),
            key: key.map(ToString::to_string).unwrap_or_default(),
            name: name.to_string(),
            value: value.to_string(),
        });
        self
    }

    /// The in-memory run, without writing it. `state` is supplied by the caller: an emitter that
    /// aborts records `Failed`, and one that never reaches this call leaves no `state.json` at all,
    /// which loads as `Interrupted`.
    pub fn finish(&self, state: RunState, ended_utc: &str, failure: Option<String>) -> Run {
        Run {
            manifest: RunManifest {
                run_id: self.run_id(),
                emitter: self.emitter.clone(),
                started_utc: self.started_utc.clone(),
                host_os: self.host_os.clone(),
                toolchain: self.toolchain.clone(),
                scope_id: self.scope_id.clone(),
                scope_version: self.scope_version,
                implementation_digest: self.implementation_digest.clone(),
                input_digests: self.input_digests.clone(),
                supersedes: self.supersedes.clone(),
                supersede_mode: self.supersede_mode,
                supersede_keys: self.supersede_keys.clone(),
            },
            state,
            ended_utc: ended_utc.to_string(),
            failure,
            keys_declared: self.declared.clone(),
            keys_executed: self.executed.clone(),
            observations: self.observations.clone(),
            unmapped: self.unmapped.clone(),
            facets: self.facets.clone(),
        }
    }

    /// Seal the run into a new immutable directory under `<root>/auto/runs/`.
    pub fn write(
        &self,
        root: &Path,
        state: RunState,
        ended_utc: &str,
        failure: Option<String>,
    ) -> Result<PathBuf, StoreError> {
        store::write_run(root, &self.finish(state, ended_utc, failure))
    }

    /// `run_id = <emitter>-<utc-basic>-<8 hex of a seed over the declared content>`.
    ///
    /// The timestamp makes the directory name legible; the digest makes two runs of one emitter at
    /// the same second distinct. Neither is ever read by a decision.
    fn run_id(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.emitter.as_bytes());
        hasher.update(self.started_utc.as_bytes());
        for key in &self.declared {
            hasher.update(key.to_string().as_bytes());
            hasher.update([0]);
        }
        for observation in &self.observations {
            hasher.update(observation.case_id.as_bytes());
            hasher.update(observation.outcome.as_str().as_bytes());
            hasher.update([0]);
        }
        for record in &self.unmapped {
            hasher.update(record.target.as_bytes());
            hasher.update(record.emitter_operation.as_bytes());
            hasher.update(record.case_id.as_bytes());
            hasher.update(record.outcome.as_str().as_bytes());
            hasher.update([0]);
        }
        let seed = format!("{:x}", hasher.finalize());
        let basic: String = self
            .started_utc
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        format!("{}-{basic}-{}", self.emitter, &seed[..8])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::testkit::key;

    #[test]
    fn an_undeclared_key_cannot_be_observed() {
        let mut builder = RunBuilder::new("synthetic", "2026-09-07T00:00:00Z", "s", 1, "unknown");
        let k = key("nhl15-ps3/save-raw-db/save");
        let err = builder
            .observe(&k, "c#1", Outcome::Pass, "")
            .expect_err("must refuse");
        assert_eq!(err, ObserveError::Undeclared(k.to_string()));
        builder.declare(k.clone());
        builder
            .observe(&k, "c#1", Outcome::Pass, "")
            .expect("now declared");
    }

    #[test]
    fn a_skipped_case_is_declared_but_not_executed() {
        let mut builder = RunBuilder::new("synthetic", "2026-09-07T00:00:00Z", "s", 1, "unknown");
        let k = key("nhl15-ps3/save-raw-db/save");
        builder.declare(k.clone());
        builder
            .observe(&k, "c#1", Outcome::SkippedMissingFixture, "no fixture")
            .expect("declared");
        let run = builder.finish(RunState::Completed, "2026-09-07T00:01:00Z", None);
        assert!(run.keys_declared.contains(&k));
        assert!(!run.keys_executed.contains(&k), "a skip is not execution");
    }

    #[test]
    fn the_run_id_is_stable_for_identical_content_and_differs_otherwise() {
        let build = |outcome: Outcome| {
            let mut b = RunBuilder::new("save_matrix", "2026-09-07T12:04:11Z", "s", 1, "unknown");
            let k = key("nhl15-ps3/save-raw-db/save");
            b.declare(k.clone());
            b.observe(&k, "c#1", outcome, "").expect("declared");
            b.finish(RunState::Completed, "", None).manifest.run_id
        };
        let a = build(Outcome::Pass);
        assert_eq!(a, build(Outcome::Pass));
        assert_ne!(a, build(Outcome::Fail));
        assert!(a.starts_with("save_matrix-20260907t120411z-"), "{a}");
    }
}
