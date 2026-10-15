//! Validation V1–V15 — [contract §5.1](../../../../docs/plans/qa-evidence-contract.md) plus the R2
//! additions V13–V15 (§12).
//!
//! **R-ACTIVE-STRICT.** Every rule here is a hard error for active inputs (`auto/runs/`, `manual/`,
//! published layer C). There are no warnings. Layer L and migration candidates are *not* active
//! inputs: they carry `unresolved` tokens instead (R-IMPORT-SOFT), which is why no rule below reads
//! [`LegacyClaim`](super::model::LegacyClaim).
//!
//! Three rules are structural and are enforced where the bytes are read rather than here, because
//! they are statements about files rather than about the model:
//!
//! | Rule | Enforced in |
//! |---|---|
//! | V3 — `run_id` equals its directory name | [`super::store`] |
//! | V4 — manifest digests and counts match the key files | [`super::store`] |
//! | V11 — every file under the manual root is byte-unchanged across a generator invocation | [`super::store`] |
//!
//! V10 (closed vocabularies) is enforced by construction: the model has no variant for an unknown
//! token, so an unknown token cannot be represented and fails at parse time.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use super::key::CoverageKey;
use super::model::{EvidenceSet, Run, SupersedeMode};
use super::vocab;

/// One hard validation failure, naming the rule and the input it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// `"V1"` … `"V15"`.
    pub rule: &'static str,
    /// The run id, record id or claim id the failure belongs to.
    pub subject: String,
    pub detail: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]: {}", self.rule, self.subject, self.detail)
    }
}

fn v(rule: &'static str, subject: impl Into<String>, detail: impl Into<String>) -> Violation {
    Violation {
        rule,
        subject: subject.into(),
        detail: detail.into(),
    }
}

/// Run every model-level rule over an evidence set. Returns every violation found, sorted, rather
/// than stopping at the first: a `check` run must be able to name everything that is wrong.
pub fn validate(set: &EvidenceSet) -> Vec<Violation> {
    let mut out = Vec::new();
    validate_keys(set, &mut out);
    validate_uniqueness(set, &mut out);
    validate_membership(set, &mut out);
    validate_supersession(set, &mut out);
    validate_applicability(set, &mut out);
    out.sort_by(|a, b| (a.rule, &a.subject, &a.detail).cmp(&(b.rule, &b.subject, &b.detail)));
    out
}

/// V1 (vocabulary half) and V12 (conversion component required).
fn validate_keys(set: &EvidenceSet, out: &mut Vec<Violation>) {
    let mut check = |key: &CoverageKey, subject: &str| {
        if let Err(e) = key.validate_vocabulary() {
            out.push(v("V1", subject, e.to_string()));
        }
        if vocab::requires_conversion(&key.family, &key.operation) && key.conversion.is_none() {
            out.push(v(
                "V12",
                subject,
                format!("{key} is conversion-dependent but carries no conversion component"),
            ));
        }
    };
    for run in &set.runs {
        for key in run.keys_declared.iter().chain(run.keys_executed.iter()) {
            check(key, run.run_id());
        }
        for obs in &run.observations {
            check(&obs.key, run.run_id());
        }
    }
    for record in &set.manual {
        check(&record.key, &record.record_id);
    }
}

/// V2 — `(run_id, case_id, key)` unique within a run; `record_id` globally unique.
fn validate_uniqueness(set: &EvidenceSet, out: &mut Vec<Violation>) {
    let mut run_ids = HashSet::new();
    for run in &set.runs {
        if !run_ids.insert(run.run_id().to_string()) {
            out.push(v("V2", run.run_id(), "duplicate run_id"));
        }
        let mut seen = HashSet::new();
        for obs in &run.observations {
            let triple = (obs.case_id.clone(), obs.key.to_string());
            if !seen.insert(triple) {
                out.push(v(
                    "V2",
                    run.run_id(),
                    format!(
                        "duplicate (run_id, case_id, key): {} / {}",
                        obs.case_id, obs.key
                    ),
                ));
            }
        }
    }
    let mut record_ids = HashSet::new();
    for record in &set.manual {
        if !record_ids.insert(record.record_id.clone()) {
            out.push(v("V2", &record.record_id, "duplicate record_id"));
        }
    }
    let mut claim_ids = HashSet::new();
    for claim in &set.legacy {
        if !claim_ids.insert(claim.claim_id.clone()) {
            out.push(v("V2", &claim.claim_id, "duplicate claim_id"));
        }
    }
}

/// V5 — executed ⊆ declared. V6 — every observation's key is declared, and every executed outcome's
/// key is in `keys-executed.txt`.
fn validate_membership(set: &EvidenceSet, out: &mut Vec<Violation>) {
    for run in &set.runs {
        for key in run.keys_executed.difference(&run.keys_declared) {
            out.push(v(
                "V5",
                run.run_id(),
                format!("{key} is executed but not declared"),
            ));
        }
        for obs in &run.observations {
            if !run.keys_declared.contains(&obs.key) {
                out.push(v(
                    "V6",
                    run.run_id(),
                    format!("observation on undeclared key {}", obs.key),
                ));
            }
            if obs.outcome.is_executed() && !run.keys_executed.contains(&obs.key) {
                out.push(v(
                    "V6",
                    run.run_id(),
                    format!(
                        "{} observation on {} is not in keys-executed.txt",
                        obs.outcome.as_str(),
                        obs.key
                    ),
                ));
            }
        }
    }
}

/// V7 (acyclic), V8 (resolvable + superset rule), V13 (`supersede_keys` declared by both runs),
/// V14 (`supersede_mode` well-formed) — for runs and, for V7/V8, for manual records.
fn validate_supersession(set: &EvidenceSet, out: &mut Vec<Violation>) {
    let by_id: BTreeMap<&str, &Run> = set.runs.iter().map(|r| (r.run_id(), r)).collect();

    for run in &set.runs {
        let m = &run.manifest;
        // V14 — the mode and the fields must agree.
        match m.supersede_mode {
            SupersedeMode::None if !m.supersedes.is_empty() => {
                out.push(v(
                    "V14",
                    run.run_id(),
                    "supersede_mode=none with a non-empty supersedes list",
                ));
            }
            SupersedeMode::Partial if m.supersede_keys.is_empty() => {
                out.push(v(
                    "V14",
                    run.run_id(),
                    "supersede_mode=partial with no supersede_keys",
                ));
            }
            SupersedeMode::Full if !m.supersede_keys.is_empty() => {
                out.push(v(
                    "V14",
                    run.run_id(),
                    "supersede_mode=full must not list supersede_keys",
                ));
            }
            _ => {}
        }
        for target_id in &m.supersedes {
            let Some(target) = by_id.get(target_id.as_str()) else {
                out.push(v(
                    "V8",
                    run.run_id(),
                    format!("supersedes names an absent run {target_id}"),
                ));
                continue;
            };
            match m.supersede_mode {
                SupersedeMode::Full => {
                    let missing: Vec<String> = target
                        .keys_declared
                        .difference(&run.keys_declared)
                        .map(ToString::to_string)
                        .collect();
                    if !missing.is_empty() {
                        out.push(v(
                            "V8",
                            run.run_id(),
                            format!(
                                "full supersession of {target_id} is not a superset; {} key(s) uncovered, first {}",
                                missing.len(),
                                missing[0]
                            ),
                        ));
                    }
                }
                SupersedeMode::Partial => {
                    // V13 — both runs must declare every listed key.
                    for key in &m.supersede_keys {
                        if !run.keys_declared.contains(key) {
                            out.push(v(
                                "V13",
                                run.run_id(),
                                format!("supersede_keys entry {key} is not declared by the superseding run"),
                            ));
                        }
                        if !target.keys_declared.contains(key) {
                            out.push(v(
                                "V13",
                                run.run_id(),
                                format!("supersede_keys entry {key} is not declared by superseded run {target_id}"),
                            ));
                        }
                    }
                }
                SupersedeMode::None => {}
            }
        }
    }

    // V7 — acyclic over runs.
    let edges: BTreeMap<String, Vec<String>> = set
        .runs
        .iter()
        .map(|r| (r.run_id().to_string(), r.manifest.supersedes.clone()))
        .collect();
    for id in edges.keys() {
        if has_cycle(id, &edges, &mut BTreeSet::new()) {
            out.push(v("V7", id, "supersession cycle over runs"));
        }
    }

    // V7/V8 — the same, over manual records.
    let record_ids: HashSet<&str> = set.manual.iter().map(|r| r.record_id.as_str()).collect();
    let record_edges: BTreeMap<String, Vec<String>> = set
        .manual
        .iter()
        .map(|r| (r.record_id.clone(), r.supersedes.clone()))
        .collect();
    for record in &set.manual {
        for target in &record.supersedes {
            if !record_ids.contains(target.as_str()) {
                out.push(v(
                    "V8",
                    &record.record_id,
                    format!("supersedes names an absent record {target}"),
                ));
            }
        }
    }
    for id in record_edges.keys() {
        if has_cycle(id, &record_edges, &mut BTreeSet::new()) {
            out.push(v("V7", id, "supersession cycle over manual records"));
        }
    }
}

fn has_cycle(
    start: &str,
    edges: &BTreeMap<String, Vec<String>>,
    seen: &mut BTreeSet<String>,
) -> bool {
    if !seen.insert(start.to_string()) {
        return true;
    }
    let found = edges
        .get(start)
        .into_iter()
        .flatten()
        .any(|next| has_cycle(next, edges, seen));
    seen.remove(start);
    found
}

/// V15 (R2) — a `not-applicable` cell must carry a cited reason. The resolution itself lives in
/// [`vocab::applicability`]; this rule guards against a resolver that ever returned an uncited one.
fn validate_applicability(set: &EvidenceSet, out: &mut Vec<Violation>) {
    let mut keys: BTreeSet<&CoverageKey> = set.declared_universe.iter().collect();
    for run in &set.runs {
        keys.extend(run.keys_declared.iter());
    }
    keys.extend(set.manual.iter().map(|r| &r.key));
    for key in keys {
        if let Some(a) = vocab::applicability(&key.target, &key.family, &key.operation) {
            if a.value == vocab::Applicability::NotApplicable && a.reason.trim().is_empty() {
                out.push(v(
                    "V15",
                    key.to_string(),
                    "not-applicable with no cited reason",
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::RunState;
    use super::super::testkit::*;
    use super::*;

    #[test]
    fn a_clean_set_produces_no_violation() {
        let set = EvidenceSet {
            journals: Vec::new(),
            runs: vec![run_pass("r1", &["nhl15-ps3/save-raw-db/save"])],
            ..default_set()
        };
        assert_eq!(validate(&set), vec![]);
    }

    #[test]
    fn a18_duplicate_ids_supersession_cycles_and_missing_targets_are_distinct_errors() {
        // Duplicate run id.
        let set = EvidenceSet {
            journals: Vec::new(),
            runs: vec![
                run_pass("r1", &["nhl15-ps3/save-raw-db/save"]),
                run_pass("r1", &["nhl15-ps3/save-raw-db/save"]),
            ],
            ..default_set()
        };
        assert!(validate(&set).iter().any(|x| x.rule == "V2"));

        // supersedes -> absent run.
        let mut r2 = run_pass("r2", &["nhl15-ps3/save-raw-db/save"]);
        r2.manifest.supersedes = vec!["nowhere".into()];
        r2.manifest.supersede_mode = SupersedeMode::Full;
        let set = EvidenceSet {
            journals: Vec::new(),
            runs: vec![r2],
            ..default_set()
        };
        assert!(validate(&set).iter().any(|x| x.rule == "V8"));

        // Cycle.
        let mut a = run_pass("ra", &["nhl15-ps3/save-raw-db/save"]);
        let mut b = run_pass("rb", &["nhl15-ps3/save-raw-db/save"]);
        a.manifest.supersedes = vec!["rb".into()];
        a.manifest.supersede_mode = SupersedeMode::Full;
        b.manifest.supersedes = vec!["ra".into()];
        b.manifest.supersede_mode = SupersedeMode::Full;
        let set = EvidenceSet {
            journals: Vec::new(),
            runs: vec![a, b],
            ..default_set()
        };
        assert!(validate(&set).iter().any(|x| x.rule == "V7"));
    }

    #[test]
    fn a19_partial_supersession_without_the_mode_is_v8() {
        let old = run_pass(
            "old",
            &["nhl15-ps3/save-raw-db/save", "nhl15-ps3/save-raw-db/reopen"],
        );
        let mut new = run_pass("new", &["nhl15-ps3/save-raw-db/save"]);
        new.manifest.supersedes = vec!["old".into()];
        new.manifest.supersede_mode = SupersedeMode::Full;
        let set = EvidenceSet {
            journals: Vec::new(),
            runs: vec![old, new],
            ..default_set()
        };
        let violations = validate(&set);
        assert!(violations
            .iter()
            .any(|x| x.rule == "V8" && x.detail.contains("not a superset")));
    }

    #[test]
    fn a36_supersede_keys_must_be_declared_by_both_runs() {
        let old = run_pass("old", &["nhl15-ps3/save-raw-db/save"]);
        let mut new = run_pass("new", &["nhl15-ps3/save-raw-db/reopen"]);
        new.manifest.supersedes = vec!["old".into()];
        new.manifest.supersede_mode = SupersedeMode::Partial;
        new.manifest.supersede_keys = vec![key("nhl15-ps3/save-raw-db/reopen")];
        let set = EvidenceSet {
            journals: Vec::new(),
            runs: vec![old, new],
            ..default_set()
        };
        let violations = validate(&set);
        assert!(
            violations
                .iter()
                .any(|x| x.rule == "V13" && x.detail.contains("superseded run")),
            "{violations:?}"
        );
    }

    #[test]
    fn v5_and_v6_catch_membership_that_the_counts_alone_would_hide() {
        let mut run = run_pass("r", &["nhl15-ps3/save-raw-db/save"]);
        run.keys_executed
            .insert(key("nhl15-ps3/save-raw-db/reopen"));
        run.observations.push(observation(
            "nhl15-ps3/save-raw-db/reopen",
            "c#2",
            super::super::model::Outcome::Pass,
        ));
        let set = EvidenceSet {
            journals: Vec::new(),
            runs: vec![run],
            ..default_set()
        };
        let violations = validate(&set);
        assert!(violations.iter().any(|x| x.rule == "V5"));
        assert!(violations.iter().any(|x| x.rule == "V6"));
    }

    #[test]
    fn v12_requires_a_conversion_component_on_head_install() {
        let set = EvidenceSet {
            journals: Vec::new(),
            runs: vec![run_pass("r", &["nhllegacy-xbox360/assets-heads/install"])],
            ..default_set()
        };
        assert!(validate(&set).iter().any(|x| x.rule == "V12"));
    }

    #[test]
    fn v14_rejects_an_ill_formed_supersede_mode() {
        let mut run = run_pass("r", &["nhl15-ps3/save-raw-db/save"]);
        run.manifest.supersede_mode = SupersedeMode::Partial;
        let set = EvidenceSet {
            journals: Vec::new(),
            runs: vec![run],
            ..default_set()
        };
        assert!(validate(&set).iter().any(|x| x.rule == "V14"));
    }

    #[test]
    fn a_failed_run_is_still_validated_rather_than_ignored() {
        let mut run = run_pass("r", &["nhl15-ps3/save-raw-db/save"]);
        run.state = RunState::Failed;
        run.observations.push(observation(
            "nhl15-ps3/not-a-family/save",
            "c#3",
            super::super::model::Outcome::Fail,
        ));
        let set = EvidenceSet {
            journals: Vec::new(),
            runs: vec![run],
            ..default_set()
        };
        let violations = validate(&set);
        assert!(violations.iter().any(|x| x.rule == "V1"));
    }
}
