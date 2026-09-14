//! Selection, the four axes, qualification and roll-ups —
//! [contract §4](../../../../docs/plans/qa-evidence-contract.md) as revised by R2 §12.
//!
//! Everything in this module is a pure function of an [`EvidenceSet`]. It reads no clock, no
//! filesystem and no timestamp field (**R-NO-TIME**), and it resolves no disagreement by recency,
//! file order or path (**R-NOWIN**).

use std::collections::{BTreeMap, BTreeSet};

use super::key::CoverageKey;
use super::model::{
    EvidenceSet, ManualRecord, ManualResult, Observation, Outcome, ProcedureKind, Run, RunState,
};
use super::vocab::{self, Applicability};

/// Contract §4.3 `automated` axis, with the R2 value `ExecutedUnselected`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomatedAxis {
    Pass,
    FailCurrent,
    FailHistorical,
    Mixed,
    SkippedMissingFixture,
    DeclaredNotExecuted,
    /// R2 §12.3: the key **was executed**, but only in runs that are not selectable.
    ExecutedUnselected,
    NotAttempted,
}

impl AutomatedAxis {
    pub fn as_str(self) -> &'static str {
        match self {
            AutomatedAxis::Pass => "pass",
            AutomatedAxis::FailCurrent => "fail-current",
            AutomatedAxis::FailHistorical => "fail-historical",
            AutomatedAxis::Mixed => "mixed",
            AutomatedAxis::SkippedMissingFixture => "skipped-missing-fixture",
            AutomatedAxis::DeclaredNotExecuted => "declared-not-executed",
            AutomatedAxis::ExecutedUnselected => "executed-unselected",
            AutomatedAxis::NotAttempted => "not-attempted",
        }
    }
}

/// Contract §4.3 `manual` axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualAxis {
    Pass,
    Fail,
    Partial,
    Inconclusive,
    None,
}

impl ManualAxis {
    pub fn as_str(self) -> &'static str {
        match self {
            ManualAxis::Pass => "pass",
            ManualAxis::Fail => "fail",
            ManualAxis::Partial => "partial",
            ManualAxis::Inconclusive => "inconclusive",
            ManualAxis::None => "none",
        }
    }
}

/// Freshness is content-identity based: never dates, never commits, never mtimes (R-CONTENT).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale,
    /// A missing or unreadable freshness input. Treated as not-fresh everywhere
    /// (R-FRESH-UNKNOWN) — it never produces `fresh`.
    Unknown,
}

impl Freshness {
    pub fn as_str(self) -> &'static str {
        match self {
            Freshness::Fresh => "fresh",
            Freshness::Stale => "stale",
            Freshness::Unknown => "unknown",
        }
    }
}

/// The derived value of contract §4.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qualification {
    NotApplicable,
    Withheld,
    Blocked,
    GameVerified,
    AutomatedQualified,
    Unqualified,
}

impl Qualification {
    pub fn as_str(self) -> &'static str {
        match self {
            Qualification::NotApplicable => "not-applicable",
            Qualification::Withheld => "withheld",
            Qualification::Blocked => "blocked",
            Qualification::GameVerified => "game-verified",
            Qualification::AutomatedQualified => "automated-qualified",
            Qualification::Unqualified => "unqualified",
        }
    }
}

/// Contract §4.4 roll-up values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollUp {
    UndefinedScope,
    Unknown,
    Blocked,
    Withheld,
    Unqualified,
    GameVerified,
    AutomatedQualified,
    NotApplicable,
}

impl RollUp {
    pub fn as_str(self) -> &'static str {
        match self {
            RollUp::UndefinedScope => "undefined-scope",
            RollUp::Unknown => "unknown",
            RollUp::Blocked => "blocked",
            RollUp::Withheld => "withheld",
            RollUp::Unqualified => "unqualified",
            RollUp::GameVerified => "game-verified",
            RollUp::AutomatedQualified => "automated-qualified",
            RollUp::NotApplicable => "not-applicable",
        }
    }
}

/// One input that mentions a key. **R-CITE-ALL**: every mention is cited, selected or not, with the
/// state that made it selectable or not.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Citation {
    /// `auto-run` · `manual` · `legacy`.
    pub kind: &'static str,
    pub id: String,
    /// Why it is or is not selected: `completed`, `failed`, `interrupted`, `superseded-at-key`,
    /// `active`, `example`, `claim`.
    pub state: String,
    pub selected: bool,
    pub detail: String,
}

/// One published layer-C row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryRow {
    pub key: CoverageKey,
    pub applicability: Applicability,
    pub applicability_reason: String,
    /// Declared, never inferred. Absent from the register means `unknown`.
    pub implementation: String,
    pub automated: AutomatedAxis,
    pub manual: ManualAxis,
    pub qualification: Qualification,
    pub reason: String,
    pub fresh: Freshness,
    pub citations: Vec<Citation>,
    pub conflict: Vec<String>,
    pub unresolved: Vec<String>,
}

impl SummaryRow {
    /// Citations of one kind, in stable order.
    pub fn citations_of(&self, kind: &str) -> Vec<&Citation> {
        self.citations.iter().filter(|c| c.kind == kind).collect()
    }
}

/// The full merge result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merged {
    /// Key-sorted (R-ORDER).
    pub rows: Vec<SummaryRow>,
}

impl Merged {
    pub fn row(&self, key: &str) -> Option<&SummaryRow> {
        self.rows.iter().find(|r| r.key.to_string() == key)
    }
}

/// A run's freshness against the current scope digests.
fn run_freshness(run: &Run, current: &BTreeMap<(String, u32), String>) -> Freshness {
    digest_freshness(
        &run.manifest.implementation_digest,
        &run.manifest.scope_id,
        run.manifest.scope_version,
        current,
    )
}

fn record_freshness(record: &ManualRecord, current: &BTreeMap<(String, u32), String>) -> Freshness {
    digest_freshness(
        &record.implementation_digest,
        &record.scope_id,
        record.scope_version,
        current,
    )
}

fn digest_freshness(
    declared: &str,
    scope_id: &str,
    scope_version: u32,
    current: &BTreeMap<(String, u32), String>,
) -> Freshness {
    if declared == "unknown" || declared.is_empty() {
        return Freshness::Unknown;
    }
    match current.get(&(scope_id.to_string(), scope_version)) {
        None => Freshness::Unknown,
        Some(now) if now == declared => Freshness::Fresh,
        Some(_) => Freshness::Stale,
    }
}

/// Combine the freshness of every piece of selected evidence: `Unknown` dominates `Stale`, which
/// dominates `Fresh`. Nothing selected at all is `Unknown` — absence never produces `fresh`.
fn combine(values: &[Freshness]) -> Freshness {
    if values.is_empty() {
        return Freshness::Unknown;
    }
    if values.contains(&Freshness::Unknown) {
        Freshness::Unknown
    } else if values.contains(&Freshness::Stale) {
        Freshness::Stale
    } else {
        Freshness::Fresh
    }
}

/// Every key the merge must emit a row for: the declared universe plus every key any input mentions.
/// The universe is included unconditionally so that an absent emitter can never make a required cell
/// disappear (R-ABSENT, R-CITE-ALL).
fn universe(set: &EvidenceSet) -> BTreeSet<CoverageKey> {
    let mut keys = set.declared_universe.clone();
    for run in &set.runs {
        keys.extend(run.keys_declared.iter().cloned());
        keys.extend(run.observations.iter().map(|o| o.key.clone()));
    }
    for journal in &set.journals {
        keys.extend(journal.keys_declared.iter().cloned());
        keys.extend(journal.observations.iter().map(|o| o.key.clone()));
    }
    keys.extend(set.manual.iter().map(|r| r.key.clone()));
    keys
}

/// Run the deterministic merge. Rows come back key-sorted; input order is irrelevant (A3).
pub fn merge(set: &EvidenceSet) -> Merged {
    let rows = universe(set)
        .into_iter()
        .map(|key| merge_key(set, &key))
        .collect();
    Merged { rows }
}

struct AutoView {
    axis: AutomatedAxis,
    freshness: Vec<Freshness>,
    citations: Vec<Citation>,
    conflict: Vec<String>,
    unresolved: Vec<String>,
    /// An uncleared executed failure whose evidence is not stale (R2 §12.3, qualification row 4a).
    uncleared_failure: bool,
    /// **C0-S2-INT1-R2 row 4b.** Some evidence covering this key could not be read back with
    /// certainty — a corrupt or truncated journal record, or a journal whose claimed sealed run
    /// does not match it. The damaged material's own key cannot be trusted, so the conservatively
    /// affected scope is every key its journal *declared*, and qualification is withheld across it.
    damaged_evidence: Vec<String>,
}

/// Is `run` suppressed at `key` by some **completed** run (R-SUPERSEDE-STATE, R-PARTIAL-KEYS)?
fn suppressed_at(set: &EvidenceSet, run: &Run, key: &CoverageKey) -> Option<String> {
    set.runs
        .iter()
        .find(|other| other.run_id() != run.run_id() && other.suppressed_keys_of(run).contains(key))
        .map(|other| other.run_id().to_string())
}

/// Does a completed run that supersedes `observer` at `key` itself execute `key` with a pass?
/// That, and only that, clears an executed failure (R2 §12.3 clause 2).
fn failure_is_cleared(set: &EvidenceSet, observer: &Run, key: &CoverageKey) -> bool {
    set.runs.iter().any(|other| {
        other.run_id() != observer.run_id()
            && other.state == RunState::Completed
            && other.suppressed_keys_of(observer).contains(key)
            && other.keys_executed.contains(key)
            && other
                .observations_for(key)
                .any(|o| o.outcome == Outcome::Pass)
    })
}

fn automated_view(set: &EvidenceSet, key: &CoverageKey) -> AutoView {
    let mut citations = Vec::new();
    let mut conflict = Vec::new();
    let mut unresolved = Vec::new();
    let mut selected_outcomes: Vec<Outcome> = Vec::new();
    let mut freshness = Vec::new();
    let mut any_active_declaring = false;
    let mut executed_outside_selection = false;
    let mut uncleared_failure = false;
    let mut damaged_evidence: Vec<String> = Vec::new();

    for run in &set.runs {
        let declares = run.keys_declared.contains(key);
        let observations: Vec<&Observation> = run.observations_for(key).collect();
        if !declares && observations.is_empty() {
            continue;
        }
        let suppression = suppressed_at(set, run, key);
        let active = run.state == RunState::Completed && suppression.is_none();
        let state = match (&suppression, run.state) {
            (Some(by), _) => format!("superseded-at-key-by:{by}"),
            (None, s) => s.as_str().to_string(),
        };
        let detail = if observations.is_empty() {
            "declared, no observation".to_string()
        } else {
            observations
                .iter()
                .map(|o| format!("{}={}", o.case_id, o.outcome.as_str()))
                .collect::<Vec<_>>()
                .join(" ")
        };
        citations.push(Citation {
            kind: "auto-run",
            id: run.run_id().to_string(),
            state,
            selected: active,
            detail,
        });

        let run_fresh = run_freshness(run, &set.current_scope_digests);
        if active {
            any_active_declaring = true;
            freshness.push(run_fresh);
            selected_outcomes.extend(observations.iter().map(|o| o.outcome));
        } else if observations.iter().any(|o| o.outcome.is_executed()) {
            executed_outside_selection = true;
        }

        // R2 §12.3 — an executed failure is visible regardless of its run's terminal state.
        for obs in observations.iter().filter(|o| o.outcome == Outcome::Fail) {
            if failure_is_cleared(set, run, key) {
                unresolved.push(format!("cleared-failure:{}:{}", run.run_id(), obs.case_id));
            } else {
                unresolved.push(format!(
                    "uncleared-executed-failure:{}:{}",
                    run.run_id(),
                    obs.case_id
                ));
                if run_fresh != Freshness::Stale {
                    uncleared_failure = true;
                } else {
                    unresolved.push("historical-failure-not-rerun".to_string());
                }
            }
        }
    }

    // ---- incomplete execution evidence: unsealed journals ----
    //
    // A journal is never selected, so an incomplete `pass` confers nothing. A persisted `fail` is
    // an executed failure like any other, and it can never be cleared: clearing requires a
    // *completed run that supersedes the observer* (§12.3 clause 2), and no run supersedes a
    // journal. That is what stops an older completed pass from burying it.
    let mut journal_declaration = false;
    for journal in &set.journals {
        let declares = journal.keys_declared.contains(key);
        let observations: Vec<&Observation> = journal.observations_for(key).collect();
        if !declares && observations.is_empty() {
            continue;
        }
        let mut detail = if observations.is_empty() {
            "declared, no observation persisted".to_string()
        } else {
            observations
                .iter()
                .map(|o| format!("{}={}", o.case_id, o.outcome.as_str()))
                .collect::<Vec<_>>()
                .join(" ")
        };
        // **Row 4b.** Damage is never attributed to an exact key — a damaged record's key is
        // precisely what cannot be trusted — so it withholds across the journal's whole declared
        // scope. A journal that declares nothing and is damaged never reaches here: the loader
        // refuses it, because there would be nothing to bound.
        if !journal.integrity.is_empty() {
            detail.push_str(&format!(" [integrity: {}]", journal.integrity.join(",")));
            unresolved.push(format!("journal-integrity-findings:{}", journal.journal_id));
        }
        let damaging = journal.damaging_findings();
        if !damaging.is_empty() {
            damaged_evidence.push(format!(
                "damaged-evidence:{}:{}",
                journal.journal_id,
                damaging.join("; ")
            ));
        }
        citations.push(Citation {
            kind: "auto-journal",
            // Not sealed and not asserted to be dead: only "this is what is persisted so far".
            id: journal.journal_id.clone(),
            state: "incomplete".to_string(),
            selected: false,
            detail,
        });

        let journal_fresh = digest_freshness(
            &journal.implementation_digest,
            &journal.scope_id,
            journal.scope_version,
            &set.current_scope_digests,
        );
        if observations.iter().any(|o| o.outcome.is_executed()) {
            executed_outside_selection = true;
            unresolved.push(format!(
                "executed-only-in-unsealed-journal:{}",
                journal.journal_id
            ));
        } else if declares {
            journal_declaration = true;
            unresolved.push(format!(
                "declared-in-unsealed-journal-not-executed:{}",
                journal.journal_id
            ));
        }
        for obs in observations.iter().filter(|o| o.outcome == Outcome::Fail) {
            unresolved.push(format!(
                "uncleared-executed-failure:{}:{}",
                journal.journal_id, obs.case_id
            ));
            if journal_fresh != Freshness::Stale {
                uncleared_failure = true;
            } else {
                unresolved.push("historical-failure-not-rerun".to_string());
            }
        }
    }

    let has_pass = selected_outcomes.contains(&Outcome::Pass);
    let has_fail = selected_outcomes.contains(&Outcome::Fail);
    let combined = combine(&freshness);
    let axis = if selected_outcomes.is_empty() {
        if executed_outside_selection {
            AutomatedAxis::ExecutedUnselected
        } else if any_active_declaring || journal_declaration {
            AutomatedAxis::DeclaredNotExecuted
        } else {
            AutomatedAxis::NotAttempted
        }
    } else if has_pass && has_fail {
        conflict.push("automated-disagreement".to_string());
        AutomatedAxis::Mixed
    } else if has_fail {
        if combined == Freshness::Fresh {
            AutomatedAxis::FailCurrent
        } else {
            AutomatedAxis::FailHistorical
        }
    } else if has_pass {
        AutomatedAxis::Pass
    } else if selected_outcomes.contains(&Outcome::SkippedMissingFixture) {
        AutomatedAxis::SkippedMissingFixture
    } else if selected_outcomes.contains(&Outcome::RefusedClean) {
        AutomatedAxis::Mixed
    } else {
        AutomatedAxis::DeclaredNotExecuted
    };

    if axis == AutomatedAxis::FailHistorical {
        unresolved.push("historical-failure-not-rerun".to_string());
    }
    if axis == AutomatedAxis::ExecutedUnselected {
        unresolved.push("executed-only-in-non-selectable-run".to_string());
    }

    unresolved.extend(damaged_evidence.iter().cloned());

    AutoView {
        axis,
        freshness,
        citations,
        conflict,
        unresolved,
        uncleared_failure,
        damaged_evidence,
    }
}

fn merge_key(set: &EvidenceSet, key: &CoverageKey) -> SummaryRow {
    let resolved = vocab::applicability_for_key(key).unwrap_or(vocab::ResolvedApplicability {
        value: Applicability::Unknown,
        reason: String::new(),
    });

    let auto = automated_view(set, key);
    let mut citations = auto.citations;
    let mut conflict = auto.conflict;
    let mut unresolved = auto.unresolved;
    let mut freshness_inputs = auto.freshness;

    // ---- layer B ----
    let for_key: Vec<&ManualRecord> = set.manual.iter().filter(|r| &r.key == key).collect();
    let superseded: BTreeSet<&str> = for_key
        .iter()
        .flat_map(|r| r.supersedes.iter().map(String::as_str))
        .filter(|id| for_key.iter().any(|r| r.record_id == *id))
        .collect();
    let mut manual_results = Vec::new();
    let mut missing_identity = false;
    for record in &for_key {
        let active = !superseded.contains(record.record_id.as_str()) && !record.example;
        let state = if record.example {
            "example".to_string()
        } else if superseded.contains(record.record_id.as_str()) {
            "superseded".to_string()
        } else {
            "active".to_string()
        };
        citations.push(Citation {
            kind: "manual",
            id: record.record_id.clone(),
            state,
            selected: active,
            detail: format!(
                "{} ({})",
                record.result.as_str(),
                record.procedure_kind.as_str()
            ),
        });
        if active {
            manual_results.push(record.result);
            freshness_inputs.push(record_freshness(record, &set.current_scope_digests));
            if record.procedure_kind == ProcedureKind::GameAcceptance
                && record.result == ManualResult::Pass
            {
                let missing = record.missing_identities();
                if !missing.is_empty() {
                    missing_identity = true;
                    unresolved.push(format!("unknown-identity:{}", missing.join("+")));
                }
            }
        }
    }
    let manual_axis = if manual_results.is_empty() {
        ManualAxis::None
    } else {
        let distinct: BTreeSet<&str> = manual_results.iter().map(|r| r.as_str()).collect();
        if distinct.len() > 1 {
            conflict.push("manual-disagreement".to_string());
            ManualAxis::Partial
        } else {
            match manual_results[0] {
                ManualResult::Pass => ManualAxis::Pass,
                ManualResult::Fail => ManualAxis::Fail,
                ManualResult::Partial => ManualAxis::Partial,
                ManualResult::Inconclusive => ManualAxis::Inconclusive,
            }
        }
    };

    // ---- layer L: cited, never qualifying ----
    let prefix = format!("{}/{}", key.target, key.family);
    for claim in set
        .legacy
        .iter()
        .filter(|c| c.key_prefix().as_deref() == Some(prefix.as_str()))
    {
        citations.push(Citation {
            kind: "legacy",
            id: claim.claim_id.clone(),
            state: "claim".to_string(),
            selected: false,
            detail: format!("{} ({})", claim.claim_status_raw, claim.confidence),
        });
        unresolved.extend(claim.unresolved.iter().cloned());
    }

    if manual_axis == ManualAxis::Pass && auto.axis == AutomatedAxis::FailCurrent {
        conflict.push("manual-pass-automated-fail".to_string());
    }

    let fresh = combine(&freshness_inputs);
    let implementation = set
        .implementation
        .get(key)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    // ---- qualification (§4.3, with the R2 row 4a) ----
    let (qualification, reason) = qualify(
        resolved.value,
        &conflict,
        auto.axis,
        auto.uncleared_failure,
        !auto.damaged_evidence.is_empty(),
        manual_axis,
        missing_identity,
        fresh,
        key,
    );

    if resolved.value == Applicability::NotApplicable {
        unresolved.retain(|u| !u.starts_with("executed-only-in"));
    }
    citations.sort();
    citations.dedup();
    conflict.sort();
    conflict.dedup();
    unresolved.sort();
    unresolved.dedup();

    SummaryRow {
        key: key.clone(),
        applicability: resolved.value,
        applicability_reason: resolved.reason,
        implementation,
        automated: auto.axis,
        manual: manual_axis,
        qualification,
        reason,
        fresh,
        citations,
        conflict,
        unresolved,
    }
}

/// A conflict *within* one layer has no defined resolution and must withhold. A conflict *between*
/// layers whose resolution the contract states is recorded but resolved by its own rule
/// (R-CONFLICT-CLASS, §12.9).
fn is_unresolved_conflict(token: &str) -> bool {
    matches!(token, "automated-disagreement" | "manual-disagreement")
}

/// Contract §4.3's decision table, first match wins, with R2 row 4a.
#[allow(clippy::too_many_arguments)]
fn qualify(
    applicability: Applicability,
    conflict: &[String],
    automated: AutomatedAxis,
    uncleared_failure: bool,
    damaged_evidence: bool,
    manual: ManualAxis,
    missing_identity: bool,
    fresh: Freshness,
    key: &CoverageKey,
) -> (Qualification, String) {
    // 1
    if applicability == Applicability::NotApplicable {
        return (Qualification::NotApplicable, String::new());
    }
    // 2
    if applicability == Applicability::Unknown {
        return (Qualification::Withheld, "unknown-applicability".into());
    }
    // 3 — only *unresolved* conflicts withhold. A cross-layer conflict whose resolution the
    // contract states (a manual pass against a current automated failure ⇒ `blocked`, row 4) is
    // recorded as a conflict token and resolved by the row that defines it (R-CONFLICT-CLASS,
    // §12.9).
    if conflict.iter().any(|c| is_unresolved_conflict(c)) {
        return (Qualification::Withheld, "conflict".into());
    }
    // 4
    if automated == AutomatedAxis::FailCurrent {
        return (Qualification::Blocked, "current-automated-failure".into());
    }
    // 4a (R2)
    if uncleared_failure {
        return (Qualification::Withheld, "unreviewed-failure".into());
    }
    // 4b (C0-S2-INT1-R2) — evidence covering this key could not be read back with certainty.
    // Placed above the freshness and pass rows deliberately: an older completed pass must not go on
    // qualifying a key whose newer evidence is damaged, because the damage may *be* a failure on it.
    if damaged_evidence {
        return (Qualification::Withheld, "damaged-evidence".into());
    }
    // 5
    if manual == ManualAxis::Pass && missing_identity {
        return (Qualification::Withheld, "unknown-identity".into());
    }
    // 6
    if fresh != Freshness::Fresh {
        // A row with no evidence at all is `unqualified`, not `withheld` for staleness: there is
        // nothing whose freshness could be in question.
        let has_evidence = automated != AutomatedAxis::NotAttempted || manual != ManualAxis::None;
        if has_evidence {
            return (Qualification::Withheld, "stale-evidence".into());
        }
        return (Qualification::Unqualified, "no-observation".into());
    }
    // 7
    if key.operation == "game-acceptance" && manual == ManualAxis::Pass {
        // R-CONV: an `unknown` in the conversion component blocks qualification.
        if key.conversion.as_ref().is_some_and(|c| !c.is_fully_known()) {
            return (
                Qualification::Withheld,
                "unknown-conversion-identity".into(),
            );
        }
        return (Qualification::GameVerified, String::new());
    }
    // 8
    if automated == AutomatedAxis::Pass {
        if key.conversion.as_ref().is_some_and(|c| !c.is_fully_known()) {
            return (
                Qualification::Withheld,
                "unknown-conversion-identity".into(),
            );
        }
        return (Qualification::AutomatedQualified, String::new());
    }
    // 9
    (Qualification::Unqualified, automated.as_str().to_string())
}

/// One roll-up scope and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeRollUp {
    /// `<target>/<family>` or `<target>/<family>?<conversion>` (R-CONV-PAIR), or a bare `<target>`.
    pub scope: String,
    pub value: RollUp,
    /// The required operations that decided it.
    pub required: Vec<String>,
}

/// Contract §4.4, applied per **conversion-preserving** scope (R-CONV-PAIR §12.6).
pub fn rollup_by_scope(merged: &Merged) -> Vec<ScopeRollUp> {
    let mut groups: BTreeMap<String, Vec<&SummaryRow>> = BTreeMap::new();
    for row in &merged.rows {
        groups.entry(row.key.rollup_scope()).or_default().push(row);
    }
    groups
        .into_iter()
        .map(|(scope, rows)| ScopeRollUp {
            value: rollup(&rows),
            required: required_ids(&rows),
            scope,
        })
        .collect()
}

/// A target-level roll-up over every family. `R-CHILD`: still a presentation value only.
pub fn rollup_by_target(merged: &Merged) -> Vec<ScopeRollUp> {
    let mut groups: BTreeMap<String, Vec<&SummaryRow>> = BTreeMap::new();
    for row in &merged.rows {
        groups.entry(row.key.target.clone()).or_default().push(row);
    }
    groups
        .into_iter()
        .map(|(scope, rows)| ScopeRollUp {
            value: rollup(&rows),
            required: required_ids(&rows),
            scope,
        })
        .collect()
}

fn required_ids(rows: &[&SummaryRow]) -> Vec<String> {
    let mut ids: Vec<String> = rows
        .iter()
        .filter(|r| r.applicability == Applicability::Required)
        .map(|r| r.key.to_string())
        .collect();
    ids.sort();
    ids
}

/// Contract §4.4's table, first match wins. `min` is never used — the axis values are not ordered.
pub fn rollup(rows: &[&SummaryRow]) -> RollUp {
    let required: Vec<&&SummaryRow> = rows
        .iter()
        .filter(|r| r.applicability == Applicability::Required)
        .collect();
    // 1
    if required.is_empty() {
        // Every applicable cell being `not-applicable` is a distinct, meaningful answer (row 8);
        // a scope with no operations defined at all is `undefined-scope`.
        if !rows.is_empty()
            && rows
                .iter()
                .all(|r| r.qualification == Qualification::NotApplicable)
        {
            return RollUp::NotApplicable;
        }
        return RollUp::UndefinedScope;
    }
    // 2
    if required
        .iter()
        .any(|r| r.qualification == Qualification::Withheld && r.reason == "unknown-applicability")
    {
        return RollUp::Unknown;
    }
    // 3
    if required
        .iter()
        .any(|r| r.qualification == Qualification::Blocked)
    {
        return RollUp::Blocked;
    }
    // 4
    if required
        .iter()
        .any(|r| r.qualification == Qualification::Withheld)
    {
        return RollUp::Withheld;
    }
    // 5
    if required
        .iter()
        .any(|r| r.qualification == Qualification::Unqualified)
    {
        return RollUp::Unqualified;
    }
    // 6
    if required
        .iter()
        .all(|r| r.qualification == Qualification::GameVerified)
    {
        return RollUp::GameVerified;
    }
    // 7
    if required.iter().all(|r| {
        matches!(
            r.qualification,
            Qualification::GameVerified | Qualification::AutomatedQualified
        )
    }) {
        return RollUp::AutomatedQualified;
    }
    // 8
    if required
        .iter()
        .all(|r| r.qualification == Qualification::NotApplicable)
    {
        return RollUp::NotApplicable;
    }
    // 9
    RollUp::Unqualified
}
