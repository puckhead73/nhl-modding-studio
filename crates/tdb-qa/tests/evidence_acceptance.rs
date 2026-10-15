//! Acceptance cases for the QA evidence core (S1) —
//! [contract §10 and §10.1](../../../docs/plans/qa-evidence-contract.md).
//!
//! Every case here runs over synthetic fixtures built by `tdb_qa::evidence::testkit`. Nothing reads
//! a fixture, an anchor, a game tree or the clock, and **no case here is evidence about any real
//! target**: they assert the behaviour of the merge, not the status of anything.
//!
//! Case ids match the contract. S2 cases (A2, A29, A30) and S3 cases (A11, A12, A25–A28, A31, A32)
//! live with their stages; the cases below are the ones §11 assigns to S1, plus the R2 regression
//! cases A33–A45.

use std::collections::{BTreeMap, BTreeSet};

use tdb_qa::evidence::merge::{self, AutomatedAxis, Freshness, ManualAxis, Qualification, RollUp};
use tdb_qa::evidence::model::{
    EvidenceSet, LegacyClaim, ManualResult, Outcome, ProcedureKind, RunState, SupersedeMode,
};
use tdb_qa::evidence::testkit::*;
use tdb_qa::evidence::validate::validate;
use tdb_qa::evidence::vocab::{self, Applicability};

const SAVE: &str = "nhl15-ps3/save-raw-db/save";
const REOPEN: &str = "nhl15-ps3/save-raw-db/reopen";
const HEAD_A: &str =
    "nhllegacy-xbox360/assets-heads/install?from=nhl24;class=head-authored;path=authored-weights;recipe=patha@1";
const HEAD_B: &str =
    "nhllegacy-xbox360/assets-heads/install?from=nhl21;class=head-authored;path=authored-weights;recipe=patha@1";

fn merged_row(set: &EvidenceSet, key: &str) -> merge::SummaryRow {
    merge::merge(set)
        .row(key)
        .cloned()
        .unwrap_or_else(|| panic!("row {key} must exist"))
}

// ---------------------------------------------------------------- A1, A3 -----

#[test]
fn a1_merging_twice_from_identical_inputs_is_identical() {
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run_pass("r1", &[SAVE, REOPEN])],
        ..default_set()
    };
    assert_eq!(merge::merge(&set), merge::merge(&set));
}

#[test]
fn a3_input_order_does_not_change_the_output() {
    let a = run_pass("r1", &[SAVE]);
    let b = run_pass("r2", &[REOPEN]);
    let one = EvidenceSet {
        journals: Vec::new(),
        runs: vec![a.clone(), b.clone()],
        ..default_set()
    };
    let other = EvidenceSet {
        journals: Vec::new(),
        runs: vec![b, a],
        ..default_set()
    };
    assert_eq!(merge::merge(&one), merge::merge(&other));
}

// ------------------------------------------------------------ A5, A8, A15 -----

#[test]
fn a5_an_empty_merge_qualifies_nothing_and_a_present_run_still_selects() {
    let empty = default_set();
    assert!(merge::merge(&empty).rows.is_empty());

    // "No new run for the emitter" is the same input set: the prior completed run stays selected.
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run_pass("r1", &[SAVE])],
        ..default_set()
    };
    let row = merged_row(&set, SAVE);
    assert_eq!(row.automated, AutomatedAxis::Pass);
    assert_eq!(row.qualification, Qualification::AutomatedQualified);
    assert!(row.citations_of("auto-run").iter().any(|c| c.selected));
}

#[test]
fn a8_a_skipped_missing_fixture_never_qualifies() {
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run_skipped("r1", &[SAVE])],
        ..default_set()
    };
    let row = merged_row(&set, SAVE);
    assert_eq!(row.automated, AutomatedAxis::SkippedMissingFixture);
    assert_ne!(row.qualification, Qualification::AutomatedQualified);
    assert_eq!(row.qualification, Qualification::Unqualified);
}

#[test]
fn a15_an_operation_pass_never_raises_the_family_roll_up() {
    // One required operation passes; the rest of the family has no evidence at all.
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run_pass("r1", &[SAVE])],
        declared_universe: family_universe("nhl15-ps3", "save-raw-db"),
        ..default_set()
    };
    let merged = merge::merge(&set);
    assert_eq!(
        merged.row(SAVE).expect("row").qualification,
        Qualification::AutomatedQualified
    );
    let scope = merge::rollup_by_scope(&merged);
    let family = scope
        .iter()
        .find(|s| s.scope == "nhl15-ps3/save-raw-db")
        .expect("scope");
    assert_eq!(
        family.value,
        RollUp::Unqualified,
        "a child pass is not a family qualification"
    );
}

/// Every declared key of one `(target, family)`, so a roll-up sees its whole required set.
fn family_universe(target: &str, family: &str) -> BTreeSet<tdb_qa::evidence::CoverageKey> {
    vocab::operation_defs(family)
        .expect("known family")
        .iter()
        .map(|d| key(&format!("{target}/{family}/{}", d.id)))
        .collect()
}

// ---------------------------------------------------------------- A6, A17 -----

#[test]
fn a6_a_manual_pass_cannot_mask_a_current_automated_failure() {
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run_fail("r1", &[SAVE])],
        manual: vec![manual(
            "m1",
            SAVE,
            ProcedureKind::OperationCheck,
            ManualResult::Pass,
        )],
        ..default_set()
    };
    let row = merged_row(&set, SAVE);
    assert_eq!(row.automated, AutomatedAxis::FailCurrent);
    assert_eq!(row.manual, ManualAxis::Pass);
    assert_eq!(row.qualification, Qualification::Blocked);
    assert!(row
        .conflict
        .contains(&"manual-pass-automated-fail".to_string()));
    // Both sides cited.
    assert!(row.citations_of("auto-run").iter().any(|c| c.id == "r1"));
    assert!(row.citations_of("manual").iter().any(|c| c.id == "m1"));
}

#[test]
fn a17_two_completed_runs_that_disagree_are_both_retained_and_withheld() {
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run_pass("r1", &[SAVE]), run_fail("r2", &[SAVE])],
        ..default_set()
    };
    let row = merged_row(&set, SAVE);
    assert_eq!(row.automated, AutomatedAxis::Mixed);
    assert_eq!(row.qualification, Qualification::Withheld);
    assert_eq!(row.reason, "conflict");
    assert!(row.conflict.contains(&"automated-disagreement".to_string()));
    assert_eq!(row.citations_of("auto-run").len(), 2);
    assert!(row.citations_of("auto-run").iter().all(|c| c.selected));
}

// ------------------------------------------------------------- A7 freshness ---

#[test]
fn a7_stale_evidence_is_withheld_rather_than_qualified() {
    let mut run = run_pass("r1", &[SAVE]);
    run.manifest.implementation_digest = STALE_DIGEST.to_string();
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run],
        ..default_set()
    };
    let row = merged_row(&set, SAVE);
    assert_eq!(row.fresh, Freshness::Stale);
    assert_eq!(row.qualification, Qualification::Withheld);
    assert_eq!(row.reason, "stale-evidence");
}

#[test]
fn an_unknown_freshness_input_never_produces_fresh() {
    let mut run = run_pass("r1", &[SAVE]);
    run.manifest.implementation_digest = "unknown".to_string();
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run],
        ..default_set()
    };
    let row = merged_row(&set, SAVE);
    assert_eq!(row.fresh, Freshness::Unknown);
    assert_ne!(row.qualification, Qualification::AutomatedQualified);
}

// ---------------------------------------------------------------- A9, A10 -----

#[test]
fn a9_the_ps3_nhl07_save_claim_is_refuted_and_the_cell_is_not_applicable() {
    let claim = legacy_claim(
        "gameqa-nhl07-ps3-row03",
        "nhl07-ps3",
        "save-raw-db",
        "edit-roundtrip-pass",
    );
    let set = EvidenceSet {
        journals: Vec::new(),
        legacy: vec![claim],
        declared_universe: BTreeSet::from([key("nhl07-ps3/save-raw-db/save")]),
        ..default_set()
    };
    let row = merged_row(&set, "nhl07-ps3/save-raw-db/save");
    // No observation is reproduced from the claim.
    assert_eq!(row.automated, AutomatedAxis::NotAttempted);
    assert_eq!(row.manual, ManualAxis::None);
    // R2: the unit is research-only, so the write cell is not applicable — with a cited reason.
    assert_eq!(row.applicability, Applicability::NotApplicable);
    assert!(row.applicability_reason.contains("research-only-unit"));
    assert_eq!(row.qualification, Qualification::NotApplicable);
    // The claim is cited, and never selected.
    let cited = row.citations_of("legacy");
    assert_eq!(cited.len(), 1);
    assert!(!cited[0].selected);
    assert!(cited[0].detail.contains("edit-roundtrip-pass"));
}

#[test]
fn a10_outer_packaging_is_never_not_applicable_on_ps3_or_ps4() {
    for target in ["nhl15-ps3", "nhl21-ps4", "nhl15-xbox360"] {
        let k = format!("{target}/save-outer-package/outer-package");
        let set = EvidenceSet {
            journals: Vec::new(),
            declared_universe: BTreeSet::from([key(&k)]),
            ..default_set()
        };
        let row = merged_row(&set, &k);
        assert_eq!(row.applicability, Applicability::Required, "{target}");
        assert_ne!(row.qualification, Qualification::NotApplicable, "{target}");
    }
}

fn legacy_claim(id: &str, target: &str, family: &str, status: &str) -> LegacyClaim {
    LegacyClaim {
        claim_id: id.to_string(),
        source_file: "legacy/frozen/game-qa-matrix.frozen.csv".to_string(),
        source_sha256: "0".repeat(64),
        source_row: 3,
        source_row_raw: String::new(),
        claim_status_raw: status.to_string(),
        claim_last_verified_raw: "2026-08-02".to_string(),
        claim_evidence_raw: String::new(),
        maintenance: "generated".to_string(),
        target: Some(target.to_string()),
        family: Some(family.to_string()),
        family_source: "positional-assumption".to_string(),
        confidence: "positional".to_string(),
        operation: "unknown".to_string(),
        unresolved: vec!["cited-gate-has-no-rows-for-this-unit".to_string()],
    }
}

// -------------------------------------------------------------- A13, A14 -----

#[test]
fn a13_a_malformed_active_record_is_a_hard_validation_error() {
    let mut record = manual(
        "m1",
        SAVE,
        ProcedureKind::OperationCheck,
        ManualResult::Pass,
    );
    record.key = key("nhl15-ps3/not-a-family/save");
    let set = EvidenceSet {
        journals: Vec::new(),
        manual: vec![record],
        ..default_set()
    };
    let violations = validate(&set);
    assert!(
        violations
            .iter()
            .any(|v| v.rule == "V1" && v.subject == "m1"),
        "{violations:?}"
    );
}

#[test]
fn a14_a_legacy_claim_with_unresolved_tokens_is_not_a_hard_error() {
    let mut claim = legacy_claim("c1", "nhl15-ps3", "save-raw-db", "not_run");
    claim.family = None; // unmappable — recorded, not thrown
    claim.unresolved.push("positional-conflict".to_string());
    let set = EvidenceSet {
        journals: Vec::new(),
        legacy: vec![claim],
        ..default_set()
    };
    assert_eq!(
        validate(&set),
        vec![],
        "R-IMPORT-SOFT: layer L is not an active input"
    );
}

// -------------------------------------------------------------- A16, A20 -----

#[test]
fn a16_an_interrupted_run_is_never_selected_but_stays_citable() {
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run_with(
            "r1",
            RunState::Interrupted,
            &[SAVE],
            Outcome::Pass,
        )],
        ..default_set()
    };
    let row = merged_row(&set, SAVE);
    let cited = row.citations_of("auto-run");
    assert_eq!(cited.len(), 1);
    assert!(!cited[0].selected);
    assert_eq!(cited[0].state, "interrupted");
    assert_ne!(row.qualification, Qualification::AutomatedQualified);
    // It executed, so it is not "not attempted" (R2 §12.3).
    assert_eq!(row.automated, AutomatedAxis::ExecutedUnselected);
}

#[test]
fn a20_two_independent_cases_on_one_key_are_both_retained() {
    let mut run = run_pass("r1", &[SAVE]);
    run.observations
        .push(observation(SAVE, "synthetic::second-case", Outcome::Pass));
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run],
        ..default_set()
    };
    assert_eq!(
        validate(&set),
        vec![],
        "uniqueness is (run_id, case_id, key), not key alone"
    );
    let row = merged_row(&set, SAVE);
    let detail = &row.citations_of("auto-run")[0].detail;
    assert!(detail.contains("second-case"), "{detail}");
}

// -------------------------------------------------------------- A21, A22 -----

#[test]
fn a21_two_head_pairs_into_one_destination_never_share_evidence() {
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run_pass("r1", &[HEAD_A])],
        declared_universe: BTreeSet::from([key(HEAD_A), key(HEAD_B)]),
        ..default_set()
    };
    let merged = merge::merge(&set);
    assert_eq!(
        merged.row(HEAD_A).expect("row").qualification,
        Qualification::AutomatedQualified
    );
    let other = merged.row(HEAD_B).expect("row");
    assert_eq!(other.automated, AutomatedAxis::NotAttempted);
    assert_eq!(other.qualification, Qualification::Unqualified);
}

#[test]
fn a22_nhl24_is_a_conversion_source_and_never_a_coverage_target() {
    assert!(vocab::unit_kind("nhl24-ps4").is_none());
    assert!(!tdb_qa::catalog::CATALOG.iter().any(|u| u.game == "nhl24"));
    // It does appear as a source, in a key whose target is a real catalog unit.
    let k = key(HEAD_A);
    assert_eq!(k.conversion.expect("conversion").from, "nhl24");
    assert_eq!(k.target, "nhllegacy-xbox360");
}

// -------------------------------------------------------------- A23, A24 -----

#[test]
fn a23_a_manual_install_pass_is_not_game_verification() {
    let install = "nhl15-xbox360/assets-portraits/install";
    let set = EvidenceSet {
        journals: Vec::new(),
        manual: vec![manual(
            "m1",
            install,
            ProcedureKind::OperationCheck,
            ManualResult::Pass,
        )],
        ..default_set()
    };
    let row = merged_row(&set, install);
    assert_eq!(row.manual, ManualAxis::Pass);
    assert_ne!(row.qualification, Qualification::GameVerified);
}

#[test]
fn a24_an_incomplete_identity_withholds_game_verification() {
    let acceptance = "nhl15-xbox360/assets-portraits/game-acceptance";
    let mut record = manual(
        "m1",
        acceptance,
        ProcedureKind::GameAcceptance,
        ManualResult::Pass,
    );
    record.runtime_build = "unknown".to_string();
    let set = EvidenceSet {
        journals: Vec::new(),
        manual: vec![record],
        ..default_set()
    };
    let row = merged_row(&set, acceptance);
    assert_eq!(row.qualification, Qualification::Withheld);
    assert_eq!(row.reason, "unknown-identity");
    assert!(row
        .unresolved
        .iter()
        .any(|u| u.contains("target.runtime_build")));

    // With every identity known it does qualify — the rule is about identity, not about doubt.
    let complete = manual(
        "m2",
        acceptance,
        ProcedureKind::GameAcceptance,
        ManualResult::Pass,
    );
    let set = EvidenceSet {
        journals: Vec::new(),
        manual: vec![complete],
        ..default_set()
    };
    assert_eq!(
        merged_row(&set, acceptance).qualification,
        Qualification::GameVerified
    );
}

// ============================ R2 regression cases (A33–A45) ==================

#[test]
fn a33_a_failed_replacement_run_does_not_erase_the_run_it_supersedes() {
    let good = run_pass("good", &[SAVE]);
    let mut bad = run_with("bad", RunState::Failed, &[SAVE], Outcome::NotAttempted);
    bad.manifest.supersedes = vec!["good".to_string()];
    bad.manifest.supersede_mode = SupersedeMode::Full;
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![good, bad],
        ..default_set()
    };

    let row = merged_row(&set, SAVE);
    assert_eq!(
        row.automated,
        AutomatedAxis::Pass,
        "the completed run must still be selected"
    );
    assert_eq!(row.qualification, Qualification::AutomatedQualified);
    let good_citation = row
        .citations_of("auto-run")
        .into_iter()
        .find(|c| c.id == "good")
        .expect("cited")
        .clone();
    assert!(good_citation.selected);
    // The failed run is reported, not applied.
    assert!(row
        .citations_of("auto-run")
        .iter()
        .any(|c| c.id == "bad" && !c.selected));
}

#[test]
fn a34_an_interrupted_replacement_run_does_not_erase_either() {
    let good = run_pass("good", &[SAVE]);
    let mut bad = run_with("bad", RunState::Interrupted, &[SAVE], Outcome::NotAttempted);
    bad.manifest.supersedes = vec!["good".to_string()];
    bad.manifest.supersede_mode = SupersedeMode::Full;
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![good, bad],
        ..default_set()
    };
    assert_eq!(merged_row(&set, SAVE).automated, AutomatedAxis::Pass);
}

#[test]
fn a35_partial_supersession_touches_only_the_listed_keys() {
    let old = run_pass("old", &[SAVE, REOPEN]);
    let mut new = run_fail("new", &[SAVE, REOPEN]);
    new.manifest.supersedes = vec!["old".to_string()];
    new.manifest.supersede_mode = SupersedeMode::Partial;
    new.manifest.supersede_keys = vec![key(SAVE)];
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![old, new],
        ..default_set()
    };
    assert_eq!(validate(&set), vec![]);

    // At SAVE the old run is suppressed, so only the new (failing) run is selected.
    let save = merged_row(&set, SAVE);
    assert_eq!(save.automated, AutomatedAxis::FailCurrent);
    assert!(save
        .citations_of("auto-run")
        .iter()
        .any(|c| c.id == "old" && !c.selected));

    // At REOPEN nothing was superseded: both runs are active and they disagree.
    let reopen = merged_row(&set, REOPEN);
    assert_eq!(reopen.automated, AutomatedAxis::Mixed);
    assert!(reopen.citations_of("auto-run").iter().all(|c| c.selected));
}

#[test]
fn a37_an_executed_failure_in_an_interrupted_run_withholds_rather_than_vanishing() {
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run_with(
            "r1",
            RunState::Interrupted,
            &[SAVE],
            Outcome::Fail,
        )],
        ..default_set()
    };
    let row = merged_row(&set, SAVE);
    assert_eq!(row.automated, AutomatedAxis::ExecutedUnselected);
    assert_ne!(row.automated, AutomatedAxis::NotAttempted);
    assert_eq!(row.qualification, Qualification::Withheld);
    assert_eq!(row.reason, "unreviewed-failure");
    assert!(row
        .unresolved
        .iter()
        .any(|u| u.starts_with("uncleared-executed-failure:r1")));
}

#[test]
fn a38_a_passing_run_cannot_bury_an_executed_failure_it_did_not_supersede() {
    let failed = run_with("crashed", RunState::Failed, &[SAVE], Outcome::Fail);
    let passing = run_pass("later", &[SAVE]);
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![failed, passing],
        ..default_set()
    };
    let row = merged_row(&set, SAVE);
    assert_eq!(
        row.automated,
        AutomatedAxis::Pass,
        "the completed run is still selected"
    );
    assert_eq!(row.qualification, Qualification::Withheld);
    assert_eq!(row.reason, "unreviewed-failure");
    assert_eq!(row.citations_of("auto-run").len(), 2);
}

#[test]
fn a39_an_explicit_supersession_clears_the_failure_without_deleting_it() {
    let failed = run_with("crashed", RunState::Failed, &[SAVE], Outcome::Fail);
    let mut passing = run_pass("later", &[SAVE]);
    passing.manifest.supersedes = vec!["crashed".to_string()];
    passing.manifest.supersede_mode = SupersedeMode::Full;
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![failed, passing],
        ..default_set()
    };
    assert_eq!(validate(&set), vec![]);
    let row = merged_row(&set, SAVE);
    assert_eq!(row.qualification, Qualification::AutomatedQualified);
    // Retained as history and still cited.
    assert!(row
        .unresolved
        .iter()
        .any(|u| u.starts_with("cleared-failure:crashed")));
    assert!(row
        .citations_of("auto-run")
        .iter()
        .any(|c| c.id == "crashed"));
}

#[test]
fn a40_read_only_families_qualify_without_a_manual_record() {
    for (family, operation) in [
        ("db-presentation", "read"),
        ("onboarding-install", "discover"),
    ] {
        let k = format!("nhl15-xbox360/{family}/{operation}");
        let set = EvidenceSet {
            journals: Vec::new(),
            runs: vec![run_pass("r1", &[&k])],
            declared_universe: family_universe("nhl15-xbox360", family),
            ..default_set()
        };
        let merged = merge::merge(&set);
        assert_eq!(
            merged.row(&k).expect("row").qualification,
            Qualification::AutomatedQualified,
            "{k}"
        );
        let scope = merge::rollup_by_scope(&merged);
        let rolled = scope
            .iter()
            .find(|s| s.scope == format!("nhl15-xbox360/{family}"))
            .expect("scope");
        assert_eq!(
            rolled.value,
            RollUp::AutomatedQualified,
            "{family} must not need game acceptance"
        );
    }
}

#[test]
fn a41_the_research_unit_has_no_applicable_write_cell_and_keeps_its_read_cells() {
    let set = EvidenceSet {
        journals: Vec::new(),
        declared_universe: family_universe("nhl07-ps3", "db-operations")
            .into_iter()
            .chain(family_universe("nhl07-ps3", "db-presentation"))
            .collect(),
        ..default_set()
    };
    let merged = merge::merge(&set);
    for row in merged
        .rows
        .iter()
        .filter(|r| r.key.family == "db-operations")
    {
        assert_eq!(
            row.applicability,
            Applicability::NotApplicable,
            "{}",
            row.key
        );
        assert!(
            !row.applicability_reason.is_empty(),
            "V15 needs a cited reason"
        );
    }
    let read = merged.row("nhl07-ps3/db-presentation/read").expect("row");
    assert_eq!(read.applicability, Applicability::Required);
}

#[test]
fn a42_a_later_timestamp_wins_nothing() {
    let mut early = run_pass("r1", &[SAVE]);
    let mut late = run_fail("r2", &[SAVE]);
    early.manifest.started_utc = "2020-01-01T00:00:00Z".to_string();
    late.manifest.started_utc = "2099-01-01T00:00:00Z".to_string();
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![early, late],
        ..default_set()
    };
    let row = merged_row(&set, SAVE);
    assert_eq!(row.automated, AutomatedAxis::Mixed);
    assert_eq!(row.qualification, Qualification::Withheld);
    assert_eq!(row.reason, "conflict");
}

#[test]
fn a43_an_absent_emitter_erases_no_citation_or_conflict() {
    let with_both = EvidenceSet {
        journals: Vec::new(),
        runs: vec![
            run_pass("emitter-a", &[SAVE]),
            run_fail("emitter-b", &[SAVE]),
        ],
        ..default_set()
    };
    let before = merged_row(&with_both, SAVE);
    assert!(!before.conflict.is_empty());

    // Emitter B contributes nothing new this time; its earlier run is still present.
    let unchanged = merged_row(&with_both, SAVE);
    assert_eq!(before.citations, unchanged.citations);
    assert_eq!(before.conflict, unchanged.conflict);

    // Removing a *run* is a different thing from an emitter being silent, and is the only way a
    // citation disappears.
    let only_a = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run_pass("emitter-a", &[SAVE])],
        ..default_set()
    };
    assert_eq!(merged_row(&only_a, SAVE).citations_of("auto-run").len(), 1);
}

#[test]
fn a44_one_passing_pair_does_not_raise_the_destination_family() {
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run_pass("r1", &[HEAD_A])],
        declared_universe: BTreeSet::from([key(HEAD_A), key(HEAD_B)]),
        ..default_set()
    };
    let merged = merge::merge(&set);
    let scopes = merge::rollup_by_scope(&merged);
    // Two distinct pair scopes, not one family scope.
    let pair_scopes: Vec<&str> = scopes
        .iter()
        .filter(|s| s.scope.starts_with("nhllegacy-xbox360/assets-heads"))
        .map(|s| s.scope.as_str())
        .collect();
    assert_eq!(pair_scopes.len(), 2, "{pair_scopes:?}");
    let a = scopes
        .iter()
        .find(|s| s.scope.contains("from=nhl24"))
        .expect("pair A");
    let b = scopes
        .iter()
        .find(|s| s.scope.contains("from=nhl21"))
        .expect("pair B");
    assert_eq!(a.value, RollUp::AutomatedQualified);
    assert_eq!(b.value, RollUp::Unqualified);
}

#[test]
fn a45_a_not_applicable_cell_always_carries_a_reason() {
    let set = EvidenceSet {
        journals: Vec::new(),
        declared_universe: family_universe("nhl07-ps3", "save-raw-db"),
        ..default_set()
    };
    assert_eq!(
        validate(&set),
        vec![],
        "V15 must find nothing: every not-applicable is cited"
    );
    let row = merged_row(&set, "nhl07-ps3/save-raw-db/save");
    assert!(row.applicability_reason.contains("roadmap"));
}

// --------------------------------------------------- implementation axis ------

#[test]
fn the_implementation_axis_is_declared_and_never_inferred_from_evidence() {
    let set = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run_pass("r1", &[SAVE])],
        ..default_set()
    };
    assert_eq!(merged_row(&set, SAVE).implementation, "unknown");

    let declared = EvidenceSet {
        journals: Vec::new(),
        runs: vec![run_pass("r1", &[SAVE])],
        implementation: BTreeMap::from([(key(SAVE), "implemented".to_string())]),
        ..default_set()
    };
    assert_eq!(merged_row(&declared, SAVE).implementation, "implemented");
}
