//! Layer A / B / L / C data model — [contract §3](../../../../docs/plans/qa-evidence-contract.md).
//!
//! These are plain in-memory types with no I/O. Loading them from disk is
//! [`super::store`](../store/index.html); the merge over them is [`super::merge`](../merge/index.html).
//! Keeping the model pure is what lets every acceptance case run over synthetic fixtures.

use std::collections::{BTreeMap, BTreeSet};

use super::key::CoverageKey;

/// Contract §3.1 `R-STATE`. A run directory with no readable `state.json` is `Interrupted` — the
/// state is never inferred from file presence, timestamps or counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Completed,
    Failed,
    Interrupted,
}

impl RunState {
    pub fn as_str(self) -> &'static str {
        match self {
            RunState::Completed => "completed",
            RunState::Failed => "failed",
            RunState::Interrupted => "interrupted",
        }
    }

    /// V10: the vocabulary is closed.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "completed" => Some(RunState::Completed),
            "failed" => Some(RunState::Failed),
            "interrupted" => Some(RunState::Interrupted),
            _ => None,
        }
    }
}

/// Contract §3.1 `outcome`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    Pass,
    Fail,
    RefusedClean,
    /// A case that returned before its assertions because its fixture was absent. Mandatory where
    /// that happens — it must never read as a pass (C0.2 §4.4).
    SkippedMissingFixture,
    /// A declared key the run reached and deliberately did not run.
    NotAttempted,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Pass => "pass",
            Outcome::Fail => "fail",
            Outcome::RefusedClean => "refused-clean",
            Outcome::SkippedMissingFixture => "skipped-missing-fixture",
            Outcome::NotAttempted => "not-attempted",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pass" => Some(Outcome::Pass),
            "fail" => Some(Outcome::Fail),
            "refused-clean" => Some(Outcome::RefusedClean),
            "skipped-missing-fixture" => Some(Outcome::SkippedMissingFixture),
            "not-attempted" => Some(Outcome::NotAttempted),
            _ => None,
        }
    }

    /// **R-EXECUTED-FAIL** (§12.3): the case actually ran. A skip and a deliberate non-attempt did
    /// not.
    pub fn is_executed(self) -> bool {
        matches!(self, Outcome::Pass | Outcome::Fail | Outcome::RefusedClean)
    }
}

/// One row of `observations.csv`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub key: CoverageKey,
    pub case_id: String,
    pub outcome: Outcome,
    pub fixture_sha256: String,
    pub output_sha256: String,
    pub detail: String,
}

/// One record of `unmapped.csv` — a case that **executed and asserted** but whose emitter operation
/// has no honest operation in the closed §4.1 vocabulary.
///
/// It is deliberately *not* an [`Observation`]: it carries no [`CoverageKey`], so it can never enter
/// the merge's per-key axes, contribute to a roll-up, or qualify anything. Its purpose is that the
/// execution is not lost — inventing an adjacent key for it is exactly the E1/E2 positional error
/// the contract exists to remove.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnmappedObservation {
    /// `<title>-<platform>`, so the record is still attributable to a unit.
    pub target: String,
    /// The emitter's own operation identifier, verbatim.
    pub emitter_operation: String,
    pub case_id: String,
    pub outcome: Outcome,
    pub fixture_sha256: String,
    /// Why no key was assigned. Never empty.
    pub unresolved: String,
    pub detail: String,
}

/// One structured facet of an observation — `codec=v4`, `save-mode=roster`, … .
///
/// Facets keep format and mode identity **structured and machine-readable** instead of buried in an
/// observation's free-text `detail`. They are descriptive only: nothing in the merge reads a facet
/// to decide an axis, a conflict or a qualification.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Facet {
    pub case_id: String,
    /// The observation's key, serialized. A facet on an unmapped record carries an empty key.
    pub key: String,
    pub name: String,
    pub value: String,
}

/// Contract §4.2 supersession mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupersedeMode {
    None,
    Full,
    Partial,
}

impl SupersedeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            SupersedeMode::None => "none",
            SupersedeMode::Full => "full",
            SupersedeMode::Partial => "partial",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "none" => Some(SupersedeMode::None),
            "full" => Some(SupersedeMode::Full),
            "partial" => Some(SupersedeMode::Partial),
            _ => None,
        }
    }
}

/// `manifest.json`. Timestamps are recorded metadata only — **R-NO-TIME** forbids reading them in
/// any decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunManifest {
    pub run_id: String,
    pub emitter: String,
    pub started_utc: String,
    pub host_os: String,
    pub toolchain: String,
    pub scope_id: String,
    pub scope_version: u32,
    /// `"unknown"` is legal and makes freshness `unknown` (R-FRESH-UNKNOWN).
    pub implementation_digest: String,
    pub input_digests: BTreeMap<String, String>,
    pub supersedes: Vec<String>,
    pub supersede_mode: SupersedeMode,
    pub supersede_keys: Vec<CoverageKey>,
}

/// An immutable layer-A run: manifest + terminal state + membership + observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub manifest: RunManifest,
    pub state: RunState,
    pub ended_utc: String,
    pub failure: Option<String>,
    /// `keys-declared.txt`. **This** is the membership evidence; counts are derived checks (V4).
    pub keys_declared: BTreeSet<CoverageKey>,
    /// `keys-executed.txt`.
    pub keys_executed: BTreeSet<CoverageKey>,
    pub observations: Vec<Observation>,
    /// `unmapped.csv` — executed cases with no key (see [`UnmappedObservation`]). Never qualifying.
    pub unmapped: Vec<UnmappedObservation>,
    /// `facets.csv` — structured descriptors of the observations above.
    pub facets: Vec<Facet>,
}

impl Run {
    pub fn run_id(&self) -> &str {
        &self.manifest.run_id
    }

    /// Every observation this run holds for `key`, in file order (order carries no meaning; it is
    /// preserved only so citations are reproducible).
    pub fn observations_for<'a>(
        &'a self,
        key: &'a CoverageKey,
    ) -> impl Iterator<Item = &'a Observation> {
        self.observations.iter().filter(move |o| &o.key == key)
    }

    /// **R-SUPERSEDE-STATE / R-PARTIAL-KEYS**: the keys at which this run suppresses `other`.
    /// A run that is not `completed` suppresses nothing at all.
    pub fn suppressed_keys_of(&self, other: &Run) -> BTreeSet<CoverageKey> {
        if self.state != RunState::Completed
            || !self.manifest.supersedes.contains(&other.manifest.run_id)
        {
            return BTreeSet::new();
        }
        match self.manifest.supersede_mode {
            SupersedeMode::None => BTreeSet::new(),
            SupersedeMode::Full => other.keys_declared.clone(),
            SupersedeMode::Partial => self
                .manifest
                .supersede_keys
                .iter()
                .filter(|k| other.keys_declared.contains(*k))
                .cloned()
                .collect(),
        }
    }
}

/// **Incomplete execution evidence** — a persisted emitter journal that has not been sealed into a
/// run.
///
/// A journal is *not* a [`Run`] and must never be treated as one. Loading it asserts only what is
/// on disk: this emitter declared these keys and persisted these observations. It asserts **nothing**
/// about whether the emitter is still running, finished, crashed or was killed, and nothing here
/// seals it — calling a possibly-live process dead would be a fabricated terminal state.
///
/// Consequences in the merge:
///
/// * a journal is **never selected**, so an incomplete `pass` confers no qualification;
/// * a persisted `fail` is an executed failure like any other (R2 §12.3): it is uncleared — no run
///   can supersede a journal — so it withholds qualification even when an older completed run
///   passed the same key;
/// * a declaration with no persisted observation stays distinct from an executed one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Journal {
    pub journal_id: String,
    pub emitter: String,
    pub started_utc: String,
    pub host_os: String,
    pub toolchain: String,
    pub scope_id: String,
    pub scope_version: u32,
    pub implementation_digest: String,
    /// Written by a successful seal so the sealed run and its journal are never counted twice. When
    /// the named run is present the journal is not loaded at all; when it is absent the journal is
    /// loaded and flagged.
    pub sealed_run_id: Option<String>,
    pub keys_declared: BTreeSet<CoverageKey>,
    pub observations: Vec<Observation>,
    pub unmapped: Vec<UnmappedObservation>,
    pub facets: Vec<Facet>,
    /// Integrity findings over the persisted material — a truncated trailing record, a record whose
    /// digest does not match, an unparseable key. Never empty-and-silent: every one of these is
    /// reported, and the evidence it describes is dropped rather than guessed at.
    pub integrity: Vec<String>,
}

impl Journal {
    /// The findings that make this journal's evidence **uncertain**, and therefore withhold
    /// qualification across its declared scope.
    ///
    /// `sealed-run-missing` is deliberately excluded: it says the run this journal claims to have
    /// been sealed into is absent, so the journal could not be deduplicated — but every record it
    /// holds is still digest-verified. Nothing about the *evidence* is in doubt, only its linkage,
    /// so it is reported without withholding. Every other finding — a corrupt or truncated record,
    /// an unknown schema, an unparseable key or outcome, a sealed run that does not match — means
    /// something covering this journal's declared keys could not be read back with certainty.
    pub fn damaging_findings(&self) -> Vec<&str> {
        self.integrity
            .iter()
            .map(String::as_str)
            .filter(|f| !f.starts_with("sealed-run-missing:"))
            .collect()
    }

    /// Every persisted observation for `key`.
    pub fn observations_for<'a>(
        &'a self,
        key: &'a CoverageKey,
    ) -> impl Iterator<Item = &'a Observation> {
        self.observations.iter().filter(move |o| &o.key == key)
    }
}

/// Contract §3.2 `result`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualResult {
    Pass,
    Fail,
    Partial,
    Inconclusive,
}

impl ManualResult {
    pub fn as_str(self) -> &'static str {
        match self {
            ManualResult::Pass => "pass",
            ManualResult::Fail => "fail",
            ManualResult::Partial => "partial",
            ManualResult::Inconclusive => "inconclusive",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pass" => Some(ManualResult::Pass),
            "fail" => Some(ManualResult::Fail),
            "partial" => Some(ManualResult::Partial),
            "inconclusive" => Some(ManualResult::Inconclusive),
            _ => None,
        }
    }
}

/// Contract §4.1 `procedure.kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcedureKind {
    OperationCheck,
    GameAcceptance,
}

impl ProcedureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProcedureKind::OperationCheck => "operation-check",
            ProcedureKind::GameAcceptance => "game-acceptance",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "operation-check" => Some(ProcedureKind::OperationCheck),
            "game-acceptance" => Some(ProcedureKind::GameAcceptance),
            _ => None,
        }
    }
}

/// One durable human record. Written only by a human or by an approved `migrate-install` (R-SEP).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualRecord {
    pub record_id: String,
    pub key: CoverageKey,
    pub result: ManualResult,
    /// A synthetic illustration is never evidence.
    pub example: bool,
    pub procedure_kind: ProcedureKind,
    pub procedure_reference: String,
    pub procedure_evidence: Vec<String>,
    pub performed_by: String,
    /// Preserved exactly as the source had it (R-UNKNOWN); never converted to a UTC instant.
    pub date_raw: String,
    pub date_precision: String,
    pub runtime: String,
    pub runtime_build: String,
    pub game_build: String,
    pub studio_commit: String,
    pub implementation_digest: String,
    pub scope_id: String,
    pub scope_version: u32,
    /// `(role, path, sha256)`.
    pub artifacts: Vec<(String, String, String)>,
    pub origin: String,
    pub supersedes: Vec<String>,
    pub supersede_reason: String,
}

impl ManualRecord {
    /// **R-GAME / R-IDENTITY**: the six identities game acceptance requires, all known.
    pub fn missing_identities(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        let output_sha = self
            .artifacts
            .iter()
            .find(|(role, _, _)| role == "output")
            .map(|(_, _, sha)| sha.as_str())
            .unwrap_or("unknown");
        for (name, value) in [
            ("artifact[output].sha256", output_sha),
            ("target.runtime", self.runtime.as_str()),
            ("target.runtime_build", self.runtime_build.as_str()),
            ("target.game_build", self.game_build.as_str()),
            ("source.studio_commit", self.studio_commit.as_str()),
            (
                "source.implementation_digest",
                self.implementation_digest.as_str(),
            ),
        ] {
            if value == "unknown" || value.is_empty() {
                missing.push(name);
            }
        }
        missing
    }
}

/// A pre-contract status label, migrated verbatim. **Never** an observation and never a manual
/// record (R-LEGACY); `qualifies` is structurally always false.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyClaim {
    pub claim_id: String,
    pub source_file: String,
    pub source_sha256: String,
    pub source_row: usize,
    pub source_row_raw: String,
    pub claim_status_raw: String,
    pub claim_last_verified_raw: String,
    pub claim_evidence_raw: String,
    pub maintenance: String,
    /// The mapped target, when one could be resolved without guessing.
    pub target: Option<String>,
    pub family: Option<String>,
    pub family_source: String,
    pub confidence: String,
    pub operation: String,
    pub unresolved: Vec<String>,
}

impl LegacyClaim {
    /// R-LEGACY: a claim never qualifies anything, under any circumstance.
    pub fn qualifies(&self) -> bool {
        false
    }

    /// The coverage key this claim is *about*, when both parts resolved. A claim with no resolvable
    /// family has no key and is reported under `_unmapped` (R-POSITIONAL).
    pub fn key_prefix(&self) -> Option<String> {
        Some(format!(
            "{}/{}",
            self.target.as_ref()?,
            self.family.as_ref()?
        ))
    }
}

/// Everything the merge reads. Assembling it is the caller's job, which is what keeps the merge
/// pure and testable.
/// What discovery found at a fixture path a run attributes (C0-EMIT1-R2).
///
/// Only `Present` can agree with a captured digest. `Missing` is the one reading of absence
/// (R-ABSENT); everything else that is not a readable regular file under the repository —
/// a directory, a link or reparse point, an unreadable file, an absolute or escaping path — is
/// `Unreadable` with the reason, and is never fresh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputState {
    /// The file's current SHA-256 (bare hex, the form the emitters record).
    Present(String),
    Missing,
    Unreadable(String),
}

impl InputState {
    /// The provenance rendering: `sha256:<hex>`, `missing`, or `unreadable:<reason>`.
    pub fn render(&self) -> String {
        match self {
            InputState::Present(hex) => format!("sha256:{hex}"),
            InputState::Missing => "missing".to_string(),
            InputState::Unreadable(reason) => format!("unreadable:{reason}"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvidenceSet {
    pub runs: Vec<Run>,
    /// Persisted-but-unsealed emitter journals — incomplete execution evidence (see [`Journal`]).
    /// Never selected, always cited.
    pub journals: Vec<Journal>,
    pub manual: Vec<ManualRecord>,
    pub legacy: Vec<LegacyClaim>,
    /// Declared implementation state per key. Absent means `unknown` — never `implemented`.
    pub implementation: BTreeMap<CoverageKey, String>,
    /// Current freshness digest per `(scope_id, scope_version)`. Absent means `unknown`
    /// (R-FRESH-UNKNOWN).
    pub current_scope_digests: BTreeMap<(String, u32), String>,
    /// The current content of every fixture a run attributes in its `input_digests`, keyed by the
    /// run's repository-relative path. Computed by discovery (`store::verify_fixture_inputs`),
    /// never by the merge, so the merge stays a pure function. A path a run names that is absent
    /// here was not verified, which is `unknown` — never fresh (C0-EMIT1-R2).
    pub current_input_digests: BTreeMap<String, InputState>,
    /// Keys that must appear in the output even with no evidence at all, so a required cell can
    /// never be hidden by the absence of an emitter (R-CITE-ALL).
    pub declared_universe: BTreeSet<CoverageKey>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_execution_is_the_r2_distinction() {
        assert!(Outcome::Pass.is_executed());
        assert!(Outcome::Fail.is_executed());
        assert!(Outcome::RefusedClean.is_executed());
        assert!(!Outcome::SkippedMissingFixture.is_executed());
        assert!(!Outcome::NotAttempted.is_executed());
    }

    #[test]
    fn every_closed_vocabulary_rejects_an_unknown_token() {
        assert!(RunState::parse("done").is_none());
        assert!(Outcome::parse("ok").is_none());
        assert!(SupersedeMode::parse("all").is_none());
        assert!(ManualResult::parse("passed").is_none());
        assert!(ProcedureKind::parse("manual").is_none());
    }

    #[test]
    fn a_legacy_claim_can_never_qualify() {
        let claim = LegacyClaim {
            claim_id: "c".into(),
            source_file: "f".into(),
            source_sha256: "0".repeat(64),
            source_row: 1,
            source_row_raw: String::new(),
            claim_status_raw: "edit-roundtrip-pass".into(),
            claim_last_verified_raw: "2026-08-02".into(),
            claim_evidence_raw: String::new(),
            maintenance: "generated".into(),
            target: Some("nhl07-ps3".into()),
            family: Some("save-raw-db".into()),
            family_source: "positional-assumption".into(),
            confidence: "positional".into(),
            operation: "unknown".into(),
            unresolved: vec![],
        };
        assert!(!claim.qualifies());
        assert_eq!(claim.key_prefix().as_deref(), Some("nhl07-ps3/save-raw-db"));
    }
}
