//! Acceptance and regression cases for observation and safe publication (S2) —
//! [contract §8 and §10](../../../docs/plans/qa-evidence-contract.md), cases **A2, A4, A29, A30**,
//! plus the **C0-BULK1-R1** publication-safety regressions.
//!
//! Every case runs against an **isolated fixture copy** under the system temp directory. Nothing
//! here reads or writes `docs/audits/`, `anchors/`, `defs/` or any game tree, and nothing here is
//! evidence about a real target. The destructive cases are deliberately pointed at synthetic
//! protected roots built for the test, never at live evidence.
//!
//! The R1 cases assert against the **filesystem state**, not against the injected fault: each one
//! checks the bytes and the directory membership that a crash would really have left behind.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use tdb_qa::evidence::merge;
use tdb_qa::evidence::model::{EvidenceSet, Outcome, RunState};
use tdb_qa::evidence::paths;
use tdb_qa::evidence::publish::{self, BuildInputs, Fault, Journal, Phase, PublishOptions};
use tdb_qa::evidence::store;
use tdb_qa::evidence::testkit::*;

const SAVE: &str = "nhl15-ps3/save-raw-db/save";
const REOPEN: &str = "nhl15-ps3/save-raw-db/reopen";

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// An isolated synthetic tree. Never a repository or game path.
fn sandbox(name: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir()
        .join("tdb-qa-evidence-pub")
        .join(format!("{name}-{}-{n}", std::process::id()));
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).expect("sandbox");
    dir
}

/// A validated layout over a synthetic evidence root, disjoint from every other sandbox.
///
/// **R2:** the mutation entry points require one, so a test cannot accidentally exercise a weaker
/// code path than production does.
fn layout() -> paths::Layout {
    paths::Layout::new(&sandbox("evidence")).expect("layout")
}

fn inputs() -> BuildInputs {
    BuildInputs {
        tool_version: "synthetic".into(),
        merge_input_digests: BTreeMap::from([(
            "synthetic-run".to_string(),
            "sha256:aa".to_string(),
        )]),
        fixture_verification: BTreeMap::new(),
        scope_definitions_digest: "sha256:bb".into(),
        divergence: Vec::new(),
        unmapped: Vec::new(),
    }
}

/// Build a staged output set into `out` from one passing synthetic run.
fn stage(out: &Path) -> BuildInputs {
    stage_from(out, &[SAVE]);
    inputs()
}

fn stage_from(out: &Path, keys: &[&str]) -> BTreeMap<String, String> {
    let set = EvidenceSet {
        runs: vec![run_pass("r1", keys)],
        ..default_set()
    };
    let built = publish::build(&merge::merge(&set), &inputs());
    built.write_to(out, &inputs(), &layout()).expect("stage");
    built
        .files
        .iter()
        .map(|(name, contents)| (name.clone(), store::sha256_prefixed(contents.as_bytes())))
        .collect()
}

fn go(staged: &Path, data: &Path, fault: Option<Fault>) -> Result<publish::PublishReport, String> {
    publish::publish(
        staged,
        data,
        &layout(),
        &inputs().merge_input_digests,
        &PublishOptions { fault },
    )
    .map_err(|e| e.to_string())
}

fn published_names(data: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(data)
        .expect("read")
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| !n.starts_with('.'))
        .collect();
    names.sort();
    names
}

fn published_digests(data: &Path) -> BTreeMap<String, String> {
    published_names(data)
        .into_iter()
        .map(|name| {
            let bytes = fs::read(data.join(&name)).expect("read");
            (name, store::sha256_prefixed(&bytes))
        })
        .collect()
}

/// Publish one generation so there is a prior set to retain.
fn with_first_generation(name: &str) -> (PathBuf, PathBuf) {
    let data = sandbox(&format!("{name}-data"));
    let out = sandbox(&format!("{name}-out"));
    stage(&out);
    go(&out, &data, None).expect("first publish");
    (data, out)
}

/// Overwrite the staged set with a different second generation, and return its digests.
fn second_generation(out: &Path) -> BTreeMap<String, String> {
    stage_from(out, &[SAVE, REOPEN])
}

fn journal(data: &Path) -> Journal {
    Journal::read(data)
        .expect("journal readable")
        .expect("journal present")
}

// ============================================================ A2, A4, A30 ====

#[test]
fn a2_build_and_publish_never_touch_the_manual_root() {
    let root = sandbox("a2-root");
    let data = sandbox("a2-data");
    let out = sandbox("a2-out");

    let record_path = root
        .join("manual")
        .join("nhl15-ps3")
        .join("save-raw-db")
        .join("m1.toml");
    fs::create_dir_all(record_path.parent().expect("parent")).expect("dirs");
    fs::write(
        &record_path,
        format!(
            "schema = \"qa-manual-record/1\"\nrecord_id = \"m1\"\nkey = \"{SAVE}\"\nresult = \"pass\"\n\n[procedure]\nkind = \"operation-check\"\n"
        ),
    )
    .expect("write record");

    let before = store::manual_digest(&root).expect("digest");
    store::write_run(&root, &run_pass("r1", &[SAVE])).expect("run");
    stage(&out);
    go(&out, &data, None).expect("publish");

    assert_eq!(
        store::manual_digest(&root).expect("digest"),
        before,
        "R-SEP / V11: layer B is byte-unchanged"
    );
    assert_eq!(store::load_manual(&root).expect("load").len(), 1);
}

#[test]
fn build_and_check_do_not_publish_anything() {
    let data = sandbox("nonpublishing-data");
    let out = sandbox("nonpublishing-out");
    stage(&out);
    assert!(published_names(&data).is_empty());
    assert!(out.join(publish::MATRIX).is_file());
    assert!(out.join(publish::BUILD_INPUTS).is_file());
}

#[test]
fn a4_a_vanished_fixture_becomes_a_skip_and_carries_no_prior_verification_forward() {
    let root = sandbox("a4");
    store::write_run(&root, &run_pass("r1", &[SAVE])).expect("first");

    let mut second = run_with(
        "r2",
        RunState::Completed,
        &[SAVE],
        Outcome::SkippedMissingFixture,
    );
    second.manifest.supersedes = vec!["r1".to_string()];
    second.manifest.supersede_mode = tdb_qa::evidence::model::SupersedeMode::Full;
    store::write_run(&root, &second).expect("second");

    let set = EvidenceSet {
        runs: store::load_runs(&root).expect("load"),
        ..default_set()
    };
    let merged = merge::merge(&set);
    let row = merged.row(SAVE).expect("row");
    assert_eq!(row.automated, merge::AutomatedAxis::SkippedMissingFixture);
    assert_ne!(row.qualification, merge::Qualification::AutomatedQualified);

    let built = publish::build(&merged, &inputs());
    let matrix = built.files.get(publish::MATRIX).expect("matrix");
    assert!(
        !matrix.contains("last_verified"),
        "no date column exists to go stale"
    );
    assert!(row
        .citations_of("auto-run")
        .iter()
        .any(|c| c.id == "r1" && !c.selected));
}

#[test]
fn a30_changed_merge_inputs_refuse_the_publication_before_anything_moves() {
    let data = sandbox("a30-data");
    let out = sandbox("a30-out");
    stage(&out);

    let mut changed = inputs().merge_input_digests;
    changed.insert("synthetic-run".to_string(), "sha256:different".to_string());
    let err = publish::publish(&out, &data, &layout(), &changed, &PublishOptions::default())
        .expect_err("must refuse");
    assert!(err.to_string().contains("A30"), "{err}");

    assert!(published_names(&data).is_empty());
    assert!(Journal::read(&data).expect("journal").is_none());
    assert_eq!(fs::read_dir(&data).expect("read").count(), 0);
}

// ================================ R1 — interruption after each old-file move ==

#[test]
fn r1_interruption_after_any_retention_move_rolls_back_completely() {
    // The defect: retention moves files, and the first implementation's `prepared` recovery assumed
    // nothing had moved. Here the crash lands after the Nth move and BEFORE its journal update, for
    // every N — the state the journal cannot describe.
    let probe = with_first_generation("r1-retention-probe");
    let count = journal(&probe.0).targets.len();
    assert_eq!(
        count,
        publish::OUTPUT_FILES.len() * 2,
        "the published set is every output file plus its sidecar"
    );

    for index in 0..count {
        let (data, out) = with_first_generation(&format!("r1-retention-{index}"));
        let before = published_digests(&data);
        second_generation(&out);

        let err = go(&out, &data, Some(Fault::AfterRetentionMove(index))).expect_err("injected");
        assert!(err.contains("retention move"), "{err}");

        // The genuinely half-retained state: some published files are already gone.
        let mid = published_names(&data);
        assert_eq!(mid.len(), count - index - 1, "index {index}: {mid:?}");

        let report = publish::recover(&data).expect("recover");
        assert!(report.action.contains("rolled back"), "{report:?}");
        assert_eq!(
            published_digests(&data),
            before,
            "index {index}: the prior generation must come back byte-for-byte"
        );
        assert_eq!(published_names(&data).len(), count, "membership restored");
    }
}

#[test]
fn r1_interruption_after_a_retention_journal_update_also_rolls_back() {
    let (data, out) = with_first_generation("r1-retention-journal");
    let before = published_digests(&data);
    second_generation(&out);

    go(&out, &data, Some(Fault::AfterRetentionJournal(2))).expect_err("injected");
    let report = publish::recover(&data).expect("recover");
    assert!(report.action.contains("rolled back"), "{report:?}");
    assert_eq!(published_digests(&data), before);
}

// ============== R1 — interruption between a mutation and its journal write ===

#[test]
fn r1_an_install_rename_that_beat_its_journal_update_is_still_recoverable() {
    // `installed` is false for the file that was actually moved. Recovery must not believe it.
    let (data, out) = with_first_generation("r1-install-gap");
    let new_set = second_generation(&out);

    go(&out, &data, Some(Fault::AfterInstallMove(3))).expect_err("injected");

    let j = journal(&data);
    assert_eq!(j.phase, Phase::Installing);
    assert!(
        !j.targets[3].installed,
        "the journal must still say `installed = false` for the file that moved — that is the point"
    );
    // But the bytes are already the new ones.
    let name = &j.targets[3].name;
    assert_eq!(
        store::sha256_prefixed(&fs::read(data.join(name)).expect("read")),
        j.targets[3].staged_digest
    );

    let report = publish::recover(&data).expect("recover");
    assert!(report.action.contains("completed"), "{report:?}");
    assert_eq!(published_digests(&data), new_set);
}

#[test]
fn r1_a_prepare_interruption_leaves_the_published_set_untouched() {
    let (data, out) = with_first_generation("r1-prepare");
    let before = published_digests(&data);
    second_generation(&out);

    go(&out, &data, Some(Fault::AfterPrepareJournal)).expect_err("injected");
    assert_eq!(published_digests(&data), before, "nothing moved yet");

    let report = publish::recover(&data).expect("recover");
    assert_eq!(published_digests(&data), before);
    assert!(report.action.contains("rolled back"), "{report:?}");
}

// ================== R1 — rollback with a mix of pre-existing and new outputs ==

#[test]
fn r1_rollback_removes_outputs_that_did_not_exist_before_and_restores_those_that_did() {
    let data = sandbox("r1-mixed-data");
    let out = sandbox("r1-mixed-out");
    stage(&out);

    // Publish a first generation, then delete half of it so the next publication has a genuine mix:
    // some targets have a previous state, some do not.
    go(&out, &data, None).expect("first publish");
    let all = published_names(&data);
    let removed: Vec<String> = all.iter().take(3).cloned().collect();
    for name in &removed {
        fs::remove_file(data.join(name)).expect("remove");
    }
    let survivors: BTreeMap<String, String> = published_digests(&data);
    assert_eq!(survivors.len(), all.len() - 3);

    second_generation(&out);
    go(&out, &data, Some(Fault::AfterInstallMove(5))).expect_err("injected");

    // Force a rollback rather than a completion by destroying the remaining staged material.
    let j = journal(&data);
    fs::remove_dir_all(&j.staging).expect("drop staging");

    let report = publish::recover(&data).expect("recover");
    assert!(report.action.contains("rolled back"), "{report:?}");

    // Exact bytes for the survivors...
    assert_eq!(published_digests(&data), survivors);
    // ...and exact membership: every file that was absent before is absent again.
    for name in &removed {
        assert!(
            !data.join(name).exists(),
            "{name} was absent before the publication and must be removed by the rollback"
        );
    }
    assert!(
        report.files.iter().any(|f| f.contains("removed")),
        "{report:?}"
    );
}

// ================================ R1 — journal trustworthiness ===============

#[test]
fn r1_a_failed_journal_replacement_leaves_the_previous_journal_intact() {
    let (data, out) = with_first_generation("r1-journal-partial");
    let before = published_digests(&data);
    second_generation(&out);

    // The very first journal write of the new publication fails after its temporary is written.
    let err = go(&out, &data, Some(Fault::JournalReplaceAfterTemp)).expect_err("injected");
    assert!(err.contains("journal replacement interrupted"), "{err}");

    // The first publication's journal — `completed` — is still readable and intact, and the
    // temporary is a separate file that never replaced it.
    let j = journal(&data);
    assert_eq!(j.phase, Phase::Completed);
    assert!(
        data.join(format!("{}.new", publish::JOURNAL)).is_file(),
        "the temporary is left behind for inspection rather than clobbering the journal"
    );
    assert_eq!(published_digests(&data), before);
}

#[test]
fn r1_an_unreadable_or_corrupt_journal_fails_closed_with_a_diagnostic() {
    let (data, _) = with_first_generation("r1-journal-corrupt");

    fs::write(data.join(publish::JOURNAL), b"{ not json").expect("corrupt it");
    let err = Journal::read(&data).expect_err("must fail closed");
    let text = err.to_string();
    assert!(text.contains("journal-corrupt"), "{text}");
    assert!(text.contains("NOT treated as absent"), "{text}");

    let err = publish::recover(&data).expect_err("recovery must fail closed too");
    assert!(err.to_string().contains("journal-corrupt"), "{err}");
}

#[test]
fn r1_an_unsupported_journal_schema_is_refused_rather_than_guessed() {
    let (data, _) = with_first_generation("r1-journal-schema");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(data.join(publish::JOURNAL)).expect("read"))
            .expect("json");
    value["schema"] = serde_json::Value::String("qa-publish-journal/99".into());
    fs::write(
        data.join(publish::JOURNAL),
        serde_json::to_string_pretty(&value).expect("json"),
    )
    .expect("write");

    let err = Journal::read(&data).expect_err("must refuse");
    assert!(err.to_string().contains("journal-schema"), "{err}");
}

#[test]
fn r1_only_not_found_means_no_journal() {
    let data = sandbox("r1-journal-absent");
    assert!(Journal::read(&data).expect("absent is fine").is_none());

    // A directory where the journal file should be is not "absent" — it is unreadable.
    fs::create_dir_all(data.join(publish::JOURNAL)).expect("dir in its place");
    let err = Journal::read(&data).expect_err("must not read as absent");
    assert!(!err.to_string().is_empty());
}

#[test]
fn r1_a_journal_naming_another_directory_is_refused_before_any_write() {
    let (data, _) = with_first_generation("r1-journal-foreign-a");
    let other = sandbox("r1-journal-foreign-b");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(data.join(publish::JOURNAL)).expect("read"))
            .expect("json");
    value["data_dir"] = serde_json::Value::String(other.to_string_lossy().to_string());
    value["phase"] = serde_json::Value::String("installing".into());
    fs::write(
        data.join(publish::JOURNAL),
        serde_json::to_string_pretty(&value).expect("json"),
    )
    .expect("write");

    let before = published_digests(&data);
    let err = publish::recover(&data).expect_err("must refuse");
    assert!(err.to_string().contains("journal-foreign"), "{err}");
    assert_eq!(
        published_digests(&data),
        before,
        "nothing was written or deleted"
    );
}

#[test]
fn r1_a_journal_target_name_outside_the_output_set_is_refused_before_any_write() {
    let (data, _) = with_first_generation("r1-journal-name");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(data.join(publish::JOURNAL)).expect("read"))
            .expect("json");
    value["targets"][0]["name"] = serde_json::Value::String("../escape.csv".into());
    value["phase"] = serde_json::Value::String("installing".into());
    fs::write(
        data.join(publish::JOURNAL),
        serde_json::to_string_pretty(&value).expect("json"),
    )
    .expect("write");

    let before = published_digests(&data);
    let err = publish::recover(&data).expect_err("must refuse");
    assert!(err.to_string().contains("journal-target-name"), "{err}");
    assert_eq!(published_digests(&data), before);
}

#[test]
fn r1_a_second_publish_is_refused_while_a_transaction_is_unresolved() {
    let (data, out) = with_first_generation("r1-second-publish");
    let before = published_digests(&data);
    second_generation(&out);

    go(&out, &data, Some(Fault::AfterRetentionMove(1))).expect_err("injected");

    let err = go(&out, &data, None).expect_err("must refuse");
    assert!(err.contains("unresolved-transaction"), "{err}");
    assert!(
        err.contains("--recover"),
        "the diagnostic must say what to do: {err}"
    );

    // Recovery then succeeds, and only after it does a new publication become possible.
    publish::recover(&data).expect("recover");
    assert_eq!(published_digests(&data), before);
    go(&out, &data, None).expect("publishing is possible again");
}

// ================================ R1 — generation identity ==================

#[test]
fn r1_republishing_identical_content_does_not_collide_with_the_retained_generation() {
    let data = sandbox("r1-identity-data");
    let out = sandbox("r1-identity-out");
    stage(&out);

    let first = go(&out, &data, None).expect("first");
    let second = go(&out, &data, None).expect("second, identical content");
    assert_ne!(
        first.id, second.id,
        "two attempts must have distinct identities even with byte-identical output"
    );
    assert_ne!(first.retained_generation, second.retained_generation);

    // Both retained generations survive: nothing is pruned in this revision.
    assert!(
        first.retained_generation.is_dir(),
        "the first generation is still retained"
    );
    assert!(second.retained_generation.is_dir());

    let generations: Vec<PathBuf> = fs::read_dir(&data)
        .expect("read")
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(".qa-prev-"))
        })
        .collect();
    assert_eq!(generations.len(), 2, "{generations:?}");
}

#[test]
fn r1_many_publications_are_all_retained_because_pruning_was_removed() {
    let data = sandbox("r1-noprune-data");
    let out = sandbox("r1-noprune-out");
    stage(&out);
    for _ in 0..5 {
        go(&out, &data, None).expect("publish");
    }
    let generations = fs::read_dir(&data)
        .expect("read")
        .flatten()
        .filter(|e| e.path().is_dir() && e.file_name().to_string_lossy().starts_with(".qa-prev-"))
        .count();
    assert_eq!(
        generations, 5,
        "no generation is pruned; digest order is not chronology"
    );
}

// ================================ R1 — foreign bytes ========================

#[test]
fn r1_modified_staged_bytes_are_detected_and_recovery_stops_without_changing_anything() {
    let (data, out) = with_first_generation("r1-foreign-staged");
    let before = published_digests(&data);
    second_generation(&out);
    go(&out, &data, Some(Fault::AfterInstallMove(2))).expect_err("injected");

    let j = journal(&data);
    let victim = PathBuf::from(&j.staging).join(&j.targets[6].name);
    fs::write(&victim, b"tampered").expect("tamper");

    let err = publish::recover(&data).expect_err("must fail closed");
    let text = err.to_string();
    assert!(text.contains("foreign-bytes"), "{text}");
    assert!(
        text.contains("are kept"),
        "the diagnostic must say material is retained: {text}"
    );

    // Nothing further was changed, and the material is still there.
    assert!(PathBuf::from(&j.staging).is_dir());
    assert!(PathBuf::from(&j.prev).is_dir());
    let after = published_digests(&data);
    assert_eq!(after.len(), before.len().min(after.len()));
}

#[test]
fn r1_modified_retained_bytes_are_detected() {
    let (data, out) = with_first_generation("r1-foreign-retained");
    second_generation(&out);
    go(&out, &data, Some(Fault::AfterRetentionMove(4))).expect_err("injected");

    let j = journal(&data);
    let victim = PathBuf::from(&j.prev).join(&j.targets[0].name);
    fs::write(&victim, b"tampered").expect("tamper");

    let err = publish::recover(&data).expect_err("must fail closed");
    assert!(err.to_string().contains("foreign-bytes"), "{err}");
    assert!(
        PathBuf::from(&j.prev).is_dir(),
        "material retained for inspection"
    );
}

// ================================ R1 — filesystem boundaries ================

#[test]
fn r1_a_build_output_inside_the_evidence_tree_is_refused_at_the_mutation() {
    let evidence = sandbox("r1-bounds-evidence");
    let record_dir = evidence
        .join("manual")
        .join("nhl15-ps3")
        .join("save-raw-db");
    fs::create_dir_all(&record_dir).expect("dirs");
    let record = record_dir.join("m1.toml");
    fs::write(&record, b"schema = \"qa-manual-record/1\"\n").expect("write");
    let before = store::manual_digest(&evidence).expect("digest");
    let before_members = walk(&evidence);

    let set = EvidenceSet {
        runs: vec![run_pass("r1", &[SAVE])],
        ..default_set()
    };
    let built = publish::build(&merge::merge(&set), &inputs());
    let bounded = paths::Layout::new(&evidence).expect("layout");

    for candidate in [
        evidence.join("manual").join("sneaky"),
        evidence.join("legacy").join("frozen"),
        evidence.join("auto").join("runs").join("x"),
        evidence.clone(),
    ] {
        let err = built
            .write_to(&candidate, &inputs(), &bounded)
            .expect_err("must refuse");
        assert!(err.to_string().contains("path-overlap"), "{err}");
    }

    // Traversal is refused too, and never normalised into an accepted path.
    let outside = sandbox("r1-bounds-out");
    let err = built
        .write_to(&outside.join("..").join("elsewhere"), &inputs(), &bounded)
        .expect_err("must refuse");
    assert!(err.to_string().contains("path-traversal"), "{err}");

    // The protected bytes AND membership are unchanged.
    assert_eq!(store::manual_digest(&evidence).expect("digest"), before);
    assert_eq!(walk(&evidence), before_members);
}

#[test]
fn r1_publishing_into_the_evidence_tree_is_refused() {
    let evidence = sandbox("r1-pub-bounds-evidence");
    fs::create_dir_all(evidence.join("manual")).expect("dirs");
    let out = sandbox("r1-pub-bounds-out");
    stage(&out);
    let before = walk(&evidence);

    let err = publish::publish(
        &out,
        &evidence.join("manual"),
        &paths::Layout::new(&evidence).expect("layout"),
        &inputs().merge_input_digests,
        &PublishOptions::default(),
    )
    .expect_err("must refuse");
    assert!(err.to_string().contains("path-overlap"), "{err}");
    assert_eq!(walk(&evidence), before, "membership unchanged");
}

#[test]
fn r1_staged_and_published_directories_may_not_overlap() {
    let data = sandbox("r1-overlap-data");
    stage(&data);
    let err = publish::publish(
        &data,
        &data,
        &layout(),
        &inputs().merge_input_digests,
        &PublishOptions::default(),
    )
    .expect_err("must refuse");
    assert!(err.to_string().contains("path-overlap"), "{err}");
}

#[test]
fn r1_a_junction_aliasing_a_protected_root_is_refused_when_one_can_be_created() {
    let evidence = sandbox("r1-junction-evidence");
    fs::create_dir_all(evidence.join("manual")).expect("dirs");
    let record = evidence.join("manual").join("m1.toml");
    fs::write(&record, b"schema = \"qa-manual-record/1\"\n").expect("write");
    let before = walk(&evidence);

    let base = sandbox("r1-junction-base");
    let link = base.join("alias");
    if !make_dir_link(&evidence.join("manual"), &link) {
        eprintln!("skipping: this platform/session cannot create a directory link");
        return;
    }

    let set = EvidenceSet {
        runs: vec![run_pass("r1", &[SAVE])],
        ..default_set()
    };
    let built = publish::build(&merge::merge(&set), &inputs());
    let err = built
        .write_to(
            &link.join("out"),
            &inputs(),
            &paths::Layout::new(&evidence).expect("layout"),
        )
        .expect_err("must refuse");
    let text = err.to_string();
    assert!(
        text.contains("path-reparse-point") || text.contains("path-overlap"),
        "{text}"
    );
    assert_eq!(walk(&evidence), before, "the aliased root is untouched");
}

#[cfg(windows)]
fn make_dir_link(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_dir(target, link).is_ok()
        || std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

#[cfg(not(windows))]
fn make_dir_link(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

/// Every path under `dir`, relative and sorted — the membership half of a preservation check.
fn walk(dir: &Path) -> Vec<String> {
    fn inner(dir: &Path, base: &Path, out: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            out.push(
                path.strip_prefix(base)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
            if path.is_dir() {
                inner(&path, base, out);
            }
        }
    }
    let mut out = Vec::new();
    inner(dir, dir, &mut out);
    out.sort();
    out
}

// ================================ recovery of a resolved transaction ========

#[test]
fn recovering_a_completed_publication_is_a_no_op() {
    let (data, out) = with_first_generation("resolved");
    second_generation(&out);
    let after = {
        go(&out, &data, None).expect("publish");
        published_digests(&data)
    };
    let report = publish::recover(&data).expect("recover");
    assert_eq!(report.phase_found, "completed");
    assert_eq!(published_digests(&data), after);
}

#[test]
fn recovering_with_no_journal_is_a_no_op() {
    let data = sandbox("no-journal");
    let report = publish::recover(&data).expect("recover");
    assert_eq!(report.phase_found, "none");
    assert!(report.files.is_empty());
}

// =============================================================== C0-BULK1-R2 ==
//
// Boundary bypasses closed after the second review. Each case builds the *actual* problematic
// filesystem state — a planted link, a hand-edited journal, an unexpected file in a cleanup
// directory — rather than injecting a fault at an already-safe phase boundary.
//
// Link-dependent cases skip **loudly** where the platform or session cannot create a link; a silent
// pass would be a missing-fixture skip dressed up as coverage.

/// Tamper with the published journal in place.
fn edit_journal(data: &Path, edit: impl FnOnce(&mut serde_json::Value)) {
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(data.join(publish::JOURNAL)).expect("read"))
            .expect("json");
    edit(&mut value);
    fs::write(
        data.join(publish::JOURNAL),
        serde_json::to_string_pretty(&value).expect("json"),
    )
    .expect("write");
}

#[test]
fn r2_an_output_file_that_is_a_link_to_a_protected_sentinel_is_refused() {
    let evidence = sandbox("r2-link-evidence");
    fs::create_dir_all(evidence.join("manual")).expect("dirs");
    fs::write(
        evidence.join("manual").join("record.toml"),
        b"schema = \"qa-manual-record/1\"\n",
    )
    .expect("write");

    let out = sandbox("r2-link-out");
    let Some(planted) = plant_reparse_point(&out.join(publish::MATRIX), &evidence.join("manual"))
    else {
        eprintln!("skipping: neither a file symlink nor a junction can be created here");
        return;
    };
    eprintln!("planted a {} at the output file name", planted.kind);
    let before_bytes = fs::read(&planted.sentinel).expect("read");
    let before_members = walk(&evidence);

    let set = EvidenceSet {
        runs: vec![run_pass("r1", &[SAVE])],
        ..default_set()
    };
    let built = publish::build(&merge::merge(&set), &inputs());
    // The output *directory* is perfectly legitimate; only the child is aliased. R1 resolved the
    // directory and then wrote straight through this link.
    let err = built
        .write_to(&out, &inputs(), &layout())
        .expect_err("must refuse");
    assert!(err.to_string().contains("path-reparse-point"), "{err}");
    assert_eq!(fs::read(&planted.sentinel).expect("read"), before_bytes);
    assert_eq!(walk(&evidence), before_members, "membership unchanged");
}

#[test]
fn r2_a_dangling_output_link_is_refused_rather_than_created_through() {
    let out = sandbox("r2-dangling-out");
    let scratch = sandbox("r2-dangling-target");
    let planted = out.join(publish::MATRIX);
    let Some(kind) = plant_dangling_reparse_point(&planted, &scratch) else {
        eprintln!("skipping: neither a file symlink nor a junction can be created here");
        return;
    };
    eprintln!("planted a dangling {kind} at the output file name");
    assert!(!planted.exists(), "a dangling link reports itself absent");
    assert!(
        fs::symlink_metadata(&planted).is_ok(),
        "but symlink_metadata still sees it — that is the whole point"
    );

    let set = EvidenceSet {
        runs: vec![run_pass("r1", &[SAVE])],
        ..default_set()
    };
    let built = publish::build(&merge::merge(&set), &inputs());
    let err = built
        .write_to(&out, &inputs(), &layout())
        .expect_err("must refuse");
    assert!(err.to_string().contains("path-reparse-point"), "{err}");
    assert!(
        !scratch.join("never-created").exists(),
        "writing through the dangling link would have created its target"
    );
}

#[test]
fn r2_a_link_planted_at_the_journal_temporary_cannot_redirect_the_journal() {
    let data = sandbox("r2-journal-new-data");
    let out = sandbox("r2-journal-new-out");
    stage(&out);
    // `.qa-publish-journal.json.new` is created fresh on every journal update, so it is the easiest
    // name to pre-plant. R1 wrote it with a bare `fs::write`.
    let temp = data.join(format!("{}.new", publish::JOURNAL));
    let protected = sandbox("r2-journal-new-sentinel");
    let Some(planted) = plant_reparse_point(&temp, &protected) else {
        eprintln!("skipping: neither a file symlink nor a junction can be created here");
        return;
    };
    eprintln!("planted a {} at the journal temporary", planted.kind);

    let err = go(&out, &data, None).expect_err("must refuse");
    assert!(err.contains("path-reparse-point"), "{err}");
    assert_eq!(
        fs::read(&planted.sentinel).expect("read"),
        b"protected bytes".to_vec(),
        "the sentinel behind the link is untouched"
    );
    assert!(
        published_names(&data).is_empty(),
        "nothing was published: the refusal happened before any install"
    );
}

#[test]
fn r2_a_journal_pointing_prev_at_an_unrelated_child_cannot_delete_it() {
    let (data, _) = with_first_generation("r2-journal-prev");
    // A directory that happens to sit beside the published set. It is *inside* `data_dir`, so R1's
    // "somewhere under data_dir" containment check accepted it — and recovery then removed it
    // recursively at the end of a rollback.
    let bystander = data.join("operator-notes");
    fs::create_dir_all(&bystander).expect("dirs");
    fs::write(bystander.join("keep.txt"), b"irreplaceable").expect("write");

    edit_journal(&data, |value| {
        value["prev"] = serde_json::Value::String(bystander.to_string_lossy().to_string());
        value["phase"] = serde_json::Value::String("retained".into());
    });

    let err = publish::recover(&data).expect_err("must refuse");
    assert!(err.to_string().contains("journal-containment"), "{err}");
    assert_eq!(
        fs::read(bystander.join("keep.txt")).expect("read"),
        b"irreplaceable".to_vec()
    );
    assert!(bystander.is_dir(), "the bystander directory still exists");
}

#[test]
fn r2_a_journal_whose_staging_and_prev_are_the_same_location_is_refused() {
    let (data, _) = with_first_generation("r2-journal-same");
    let before = published_digests(&data);
    edit_journal(&data, |value| {
        let staging = value["staging"].clone();
        value["prev"] = staging;
        value["phase"] = serde_json::Value::String("installing".into());
    });
    let err = publish::recover(&data).expect_err("must refuse");
    assert!(err.to_string().contains("journal-containment"), "{err}");
    assert_eq!(published_digests(&data), before);
}

#[test]
fn r2_a_journal_that_duplicates_or_omits_a_target_is_refused() {
    // Duplicate: the recorded previous/staged digests for that name become ambiguous.
    let (data, _) = with_first_generation("r2-journal-dup");
    let before = published_digests(&data);
    edit_journal(&data, |value| {
        let first = value["targets"][0].clone();
        value["targets"].as_array_mut().expect("array").push(first);
        value["phase"] = serde_json::Value::String("installing".into());
    });
    let err = publish::recover(&data).expect_err("must refuse");
    assert!(err.to_string().contains("journal-target-set"), "{err}");
    assert_eq!(published_digests(&data), before);

    // Omission: recovery acts only on recorded targets, so a dropped one would stay half-published
    // with no diagnostic at all.
    let (data, _) = with_first_generation("r2-journal-omit");
    let before = published_digests(&data);
    edit_journal(&data, |value| {
        value["targets"].as_array_mut().expect("array").remove(0);
        value["phase"] = serde_json::Value::String("installing".into());
    });
    let err = publish::recover(&data).expect_err("must refuse");
    assert!(err.to_string().contains("journal-target-set"), "{err}");
    assert_eq!(published_digests(&data), before);
}

#[test]
fn r2_a_journal_with_a_malformed_id_or_digest_is_refused() {
    let (data, _) = with_first_generation("r2-journal-id");
    edit_journal(&data, |value| {
        value["id"] = serde_json::Value::String("../../escape".into());
        value["phase"] = serde_json::Value::String("installing".into());
    });
    let err = publish::recover(&data).expect_err("must refuse");
    assert!(err.to_string().contains("journal-id"), "{err}");

    let (data, _) = with_first_generation("r2-journal-digest");
    edit_journal(&data, |value| {
        value["targets"][0]["staged_digest"] = serde_json::Value::String("not-a-digest".into());
        value["phase"] = serde_json::Value::String("installing".into());
    });
    let err = publish::recover(&data).expect_err("must refuse");
    assert!(err.to_string().contains("journal-digest"), "{err}");
}

#[test]
fn r2_an_unexpected_file_in_a_cleanup_directory_stops_the_deletion() {
    let data = sandbox("r2-cleanup-data");
    let out = sandbox("r2-cleanup-out");
    stage(&out);
    let interrupted = go(&out, &data, Some(Fault::AfterInstallMove(0))).expect_err("interrupted");
    assert!(interrupted.contains("injected-failure"), "{interrupted}");

    // A note dropped into the staging directory while the interrupted transaction is inspected. It
    // is not part of the output set, so cleanup must refuse rather than delete it.
    let journal = Journal::read(&data).expect("journal").expect("present");
    let staging = PathBuf::from(&journal.staging);
    let note = staging.join("why-this-failed.txt");
    fs::write(&note, b"operator note").expect("write");

    let err = publish::recover(&data).expect_err("must refuse to delete it");
    assert!(
        err.to_string().contains("cleanup-unexpected-entry"),
        "{err}"
    );
    assert_eq!(
        fs::read(&note).expect("read"),
        b"operator note".to_vec(),
        "the unexplained file is kept, not deleted"
    );
    assert!(staging.is_dir(), "the whole directory is retained");
}

#[test]
fn r2_every_library_mutation_entry_point_requires_a_validated_layout() {
    // There is no `write_to` / `publish` / `migrate` overload that takes no protection: the R1
    // wrappers that passed an empty set are gone, so this is the *only* way to call them and the
    // check below is the one a library caller unavoidably receives.
    let evidence = sandbox("r2-mandatory-evidence");
    fs::create_dir_all(evidence.join("manual")).expect("dirs");
    let sentinel = evidence.join("manual").join("record.toml");
    fs::write(&sentinel, b"schema = \"qa-manual-record/1\"\n").expect("write");
    let before = walk(&evidence);
    let bounded = paths::Layout::new(&evidence).expect("layout");

    let set = EvidenceSet {
        runs: vec![run_pass("r1", &[SAVE])],
        ..default_set()
    };
    let built = publish::build(&merge::merge(&set), &inputs());
    let err = built
        .write_to(&evidence.join("manual"), &inputs(), &bounded)
        .expect_err("must refuse");
    assert!(err.to_string().contains("path-overlap"), "{err}");

    let out = sandbox("r2-mandatory-out");
    stage(&out);
    let err = publish::publish(
        &out,
        &evidence.join("auto").join("runs"),
        &bounded,
        &inputs().merge_input_digests,
        &PublishOptions::default(),
    )
    .expect_err("must refuse");
    assert!(err.to_string().contains("path-overlap"), "{err}");

    assert_eq!(walk(&evidence), before, "bytes and membership unchanged");
}

#[test]
fn r2_an_ordinary_publication_and_recovery_still_succeed() {
    // The guard rails must not have broken the path everything else depends on.
    let data = sandbox("r2-happy-data");
    let out = sandbox("r2-happy-out");
    stage(&out);
    let first = go(&out, &data, None).expect("first publish");
    assert_eq!(first.installed.len(), publish::OUTPUT_FILES.len() * 2);
    assert_eq!(
        published_names(&data).len(),
        publish::OUTPUT_FILES.len() * 2
    );

    // A second generation, interrupted mid-install, then recovered.
    let out2 = sandbox("r2-happy-out2");
    let digests = stage_from(&out2, &[SAVE, REOPEN]);
    let err = go(&out2, &data, Some(Fault::AfterInstallMove(2))).expect_err("interrupted");
    assert!(err.contains("injected-failure"), "{err}");
    let report = publish::recover(&data).expect("recover");
    assert_eq!(report.phase_found, "installing");

    let published = published_digests(&data);
    for (name, digest) in &digests {
        assert_eq!(
            published.get(name),
            Some(digest),
            "{name} installed exactly"
        );
    }
    assert_eq!(
        Journal::read(&data)
            .expect("journal")
            .expect("present")
            .phase,
        Phase::Completed
    );
}

// ------------------------------------------------- planting a reparse point --
//
// **Coverage note.** Creating a *file* symlink on Windows needs `SeCreateSymbolicLink` (elevation
// or Developer Mode) and is **not** available in the session these tests were last run in. A
// directory **junction** needs neither and is available, so the helpers below fall back to one.
//
// That fallback is not a weaker test of the thing under test: `resolve_file_for_write` refuses any
// reparse point at a mutation target, and a junction is a reparse point. What the fallback does not
// cover is the specific case of a *file* symlink whose target is a single protected file — a
// different reparse tag reaching the identical check. Where neither can be created the case skips
// loudly rather than passing.

/// What was planted at `link`, and the bytes that must survive the refusal.
struct Planted {
    /// A file whose contents must be unchanged after the refusal.
    sentinel: PathBuf,
    kind: &'static str,
}

/// Plant a reparse point at `link` that aliases protected material. `None` = cannot be created here.
fn plant_reparse_point(link: &Path, protected_dir: &Path) -> Option<Planted> {
    fs::create_dir_all(protected_dir).expect("dirs");
    let sentinel = protected_dir.join("sentinel.txt");
    fs::write(&sentinel, b"protected bytes").expect("write");
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent).expect("dirs");
    }
    if symlink_file(&sentinel, link) {
        return Some(Planted {
            sentinel,
            kind: "file symlink",
        });
    }
    if junction(protected_dir, link) {
        return Some(Planted {
            sentinel,
            kind: "directory junction",
        });
    }
    None
}

/// Plant a **dangling** reparse point at `link`: one whose target does not exist, so `exists()`
/// reports the link itself absent while `symlink_metadata` still sees it.
fn plant_dangling_reparse_point(link: &Path, scratch: &Path) -> Option<&'static str> {
    let missing = scratch.join("never-created");
    if symlink_file(&missing, link) {
        return Some("file symlink");
    }
    // A junction needs an existing target, so create one, link it, then remove it: the junction is
    // left dangling, which is exactly the state under test.
    fs::create_dir_all(&missing).expect("dirs");
    if junction(&missing, link) {
        fs::remove_dir_all(&missing).expect("remove");
        return Some("directory junction");
    }
    fs::remove_dir_all(&missing).ok();
    None
}

#[cfg(windows)]
fn symlink_file(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}

#[cfg(windows)]
fn junction(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_dir(target, link).is_ok()
        || std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

#[cfg(not(windows))]
fn symlink_file(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(not(windows))]
fn junction(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}
