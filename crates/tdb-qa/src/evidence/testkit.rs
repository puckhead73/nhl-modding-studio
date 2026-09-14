//! Synthetic fixture builders for the acceptance cases.
//!
//! **Everything here is synthetic.** No builder reads a fixture, a game tree, an anchor or the
//! clock, and no value produced here is evidence about any real target. The module is public so the
//! integration-level acceptance cases in `tests/` can use the same builders as the unit tests, which
//! is what keeps A1–A45 expressed against one model rather than two.

use std::collections::{BTreeMap, BTreeSet};

use super::key::CoverageKey;
use super::model::{
    EvidenceSet, ManualRecord, ManualResult, Observation, Outcome, ProcedureKind, Run, RunManifest,
    RunState, SupersedeMode,
};

/// The synthetic "current" implementation digest. A run or record carrying it is fresh; carrying
/// [`STALE_DIGEST`] it is stale; carrying `"unknown"` it is unknown.
pub const FRESH_DIGEST: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";
/// A synthetic digest that is deliberately not the current one.
pub const STALE_DIGEST: &str =
    "sha256:2222222222222222222222222222222222222222222222222222222222222222";
/// The synthetic freshness scope every builder declares.
pub const SCOPE: &str = "synthetic-scope";

/// Parse a key, panicking on a malformed literal — test code only.
pub fn key(raw: &str) -> CoverageKey {
    CoverageKey::parse(raw).unwrap_or_else(|e| panic!("test key {raw:?} must parse: {e}"))
}

/// An empty set whose current scope digest is [`FRESH_DIGEST`].
pub fn default_set() -> EvidenceSet {
    EvidenceSet {
        journals: Vec::new(),
        runs: Vec::new(),
        manual: Vec::new(),
        legacy: Vec::new(),
        implementation: BTreeMap::new(),
        current_scope_digests: BTreeMap::from([((SCOPE.to_string(), 1), FRESH_DIGEST.to_string())]),
        declared_universe: BTreeSet::new(),
    }
}

/// One observation with no fixture or output hash.
pub fn observation(key_raw: &str, case_id: &str, outcome: Outcome) -> Observation {
    Observation {
        key: key(key_raw),
        case_id: case_id.to_string(),
        outcome,
        fixture_sha256: String::new(),
        output_sha256: String::new(),
        detail: String::new(),
    }
}

fn manifest(run_id: &str) -> RunManifest {
    RunManifest {
        run_id: run_id.to_string(),
        emitter: "synthetic".to_string(),
        // Recorded metadata only — R-NO-TIME forbids any decision reading it.
        started_utc: "2026-09-07T00:00:00Z".to_string(),
        host_os: "synthetic".to_string(),
        toolchain: "synthetic".to_string(),
        scope_id: SCOPE.to_string(),
        scope_version: 1,
        implementation_digest: FRESH_DIGEST.to_string(),
        input_digests: BTreeMap::new(),
        supersedes: Vec::new(),
        supersede_mode: SupersedeMode::None,
        supersede_keys: Vec::new(),
    }
}

/// A run declaring and executing `keys`, with one observation per key carrying `outcome`.
pub fn run_with(run_id: &str, state: RunState, keys: &[&str], outcome: Outcome) -> Run {
    let declared: BTreeSet<CoverageKey> = keys.iter().map(|k| key(k)).collect();
    let executed = if outcome.is_executed() {
        declared.clone()
    } else {
        BTreeSet::new()
    };
    Run {
        manifest: manifest(run_id),
        state,
        ended_utc: "2026-09-07T00:10:00Z".to_string(),
        failure: None,
        keys_declared: declared,
        keys_executed: executed,
        observations: keys
            .iter()
            .map(|k| observation(k, &format!("synthetic::{run_id}#{k}"), outcome))
            .collect(),
        unmapped: Vec::new(),
        facets: Vec::new(),
    }
}

/// A completed run that passed every listed key.
pub fn run_pass(run_id: &str, keys: &[&str]) -> Run {
    run_with(run_id, RunState::Completed, keys, Outcome::Pass)
}

/// A completed run that failed every listed key.
pub fn run_fail(run_id: &str, keys: &[&str]) -> Run {
    run_with(run_id, RunState::Completed, keys, Outcome::Fail)
}

/// A run that declares every listed key and skips it for want of a fixture.
pub fn run_skipped(run_id: &str, keys: &[&str]) -> Run {
    run_with(
        run_id,
        RunState::Completed,
        keys,
        Outcome::SkippedMissingFixture,
    )
}

/// A manual record. `example = false`, so it counts as evidence in the merge.
pub fn manual(
    record_id: &str,
    key_raw: &str,
    kind: ProcedureKind,
    result: ManualResult,
) -> ManualRecord {
    let k = key(key_raw);
    ManualRecord {
        record_id: record_id.to_string(),
        key: k.clone(),
        result,
        example: false,
        procedure_kind: kind,
        procedure_reference: "synthetic".to_string(),
        procedure_evidence: Vec::new(),
        performed_by: "synthetic".to_string(),
        date_raw: "2026-09-07".to_string(),
        date_precision: "day".to_string(),
        runtime: "synthetic-runtime".to_string(),
        runtime_build: "synthetic-build".to_string(),
        game_build: "synthetic-game-build".to_string(),
        studio_commit: "0".repeat(40),
        implementation_digest: FRESH_DIGEST.to_string(),
        scope_id: SCOPE.to_string(),
        scope_version: 1,
        artifacts: vec![(
            "output".to_string(),
            "(synthetic)".to_string(),
            "0".repeat(64),
        )],
        origin: "authored".to_string(),
        supersedes: Vec::new(),
        supersede_reason: String::new(),
    }
}
