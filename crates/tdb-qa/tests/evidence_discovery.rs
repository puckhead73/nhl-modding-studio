//! C0-S2-INT1-R4 — consistent, fail-closed evidence discovery.
//!
//! Every case runs over a **synthetic** evidence root under the system temp tree. Nothing reads or
//! writes `docs/audits/`, `anchors/`, a game tree or a private fixture, and no value produced here
//! is evidence about any real target.
//!
//! The cases are in two groups, and the distinction matters when reading a failure:
//!
//! * **End-to-end** cases plant real on-disk state and go through the production traversal —
//!   `store::load_runs`, `store::load_manual`, `store::manual_digest`, `journal::load_journals` —
//!   exactly as `qa-evidence check` and `build` do. They assert what happens to **qualification**,
//!   not merely that a helper returned an error.
//! * **Helper** cases exercise `paths::` primitives directly, for the branches no test can provoke
//!   reliably on every platform (a metadata read that fails for a reason other than absence).
//!
//! Links are real in both groups: a junction on Windows (`mklink /J`, which needs no elevation) and
//! a symlink elsewhere. Neither is simulated.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tdb_qa::evidence::emit::{Emitter, ROOT_ENV};
use tdb_qa::evidence::merge::{self, Qualification};
use tdb_qa::evidence::model::{EvidenceSet, Outcome};
use tdb_qa::evidence::{journal, paths, store, vocab};

fn env_lock() -> MutexGuard<'static, ()> {
    static ENV: OnceLock<Mutex<()>> = OnceLock::new();
    ENV.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sandbox(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir()
        .join("tdb-qa-evidence-discovery")
        .join(format!("{name}-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("sandbox");
    dir
}

const READ: &str = "nhl15-ps3/db-presentation/read";
const CASE: &str = "tdb_qa::evidence_discovery::synthetic";

/// The production load path — the same four loaders `check` and `build` call, in the same order.
fn try_set_of(root: &Path) -> Result<EvidenceSet, String> {
    let runs = store::load_runs(root).map_err(|e| e.to_string())?;
    let journals = journal::load_journals(root, &runs).map_err(|e| e.to_string())?;
    Ok(EvidenceSet {
        runs,
        journals,
        manual: store::load_manual(root).map_err(|e| e.to_string())?,
        legacy: store::load_legacy(root).map_err(|e| e.to_string())?,
        implementation: std::collections::BTreeMap::new(),
        current_scope_digests: tdb_qa::evidence::emit::scope_digests(&repo_root()),
        declared_universe: vocab::catalog_universe(),
    })
}

fn set_of(root: &Path) -> EvidenceSet {
    try_set_of(root).expect("the evidence tree loads")
}

fn row_of(root: &Path, k: &str) -> merge::SummaryRow {
    merge::merge(&set_of(root))
        .rows
        .into_iter()
        .find(|r| r.key.to_string() == k)
        .unwrap_or_else(|| panic!("no row for {k}"))
}

/// Seal one run recording `outcome` on `READ`. Requires `ROOT_ENV` to be set.
fn sealed_run(outcome: Outcome, detail: &str) -> PathBuf {
    let mut ev = Emitter::from_env("synthetic", "db-read", &repo_root());
    let key = ev.declare(READ);
    ev.observe(&key, &format!("{CASE}#{detail}"), outcome, detail);
    ev.finish().expect("sealed")
}

fn write_manual(root: &Path, rel: &[&str], record_id: &str, result: &str) -> PathBuf {
    let mut path = root.join("manual");
    for part in rel {
        path = path.join(part);
    }
    path = path.join(format!("{record_id}.toml"));
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(
        &path,
        format!(
            "schema = \"qa-manual-record/1\"\nrecord_id = \"{record_id}\"\nkey = \"{READ}\"\n\
             result = \"{result}\"\n\n[procedure]\nkind = \"operation-check\"\n"
        ),
    )
    .expect("write record");
    path
}

/// A **real** link — a junction on Windows, a symlink elsewhere. Not simulated, and not skipped:
/// `mklink /J` needs no elevation, so a failure here is a genuine platform finding.
fn make_dir_link(link: &Path, target: &Path) {
    #[cfg(windows)]
    {
        let status = std::process::Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(link)
            .arg(target)
            .output()
            .expect("run mklink");
        assert!(
            status.status.success(),
            "mklink /J failed: {}",
            String::from_utf8_lossy(&status.stderr)
        );
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).expect("symlink");
}

// ===================================================== end-to-end: layer A ====================

#[test]
fn r4_a_run_that_cannot_be_discovered_fails_instead_of_qualifying() {
    let _guard = env_lock();
    let root = sandbox("r4-run-lost");
    let aside = sandbox("r4-run-lost-aside");
    std::env::set_var(ROOT_ENV, &root);
    let _pass = sealed_run(Outcome::Pass, "the older passing run");
    let fail = sealed_run(Outcome::Fail, "the failing run");
    std::env::remove_var(ROOT_ENV);

    // Baseline: with both runs present the failure is unreconciled against the pass, so the row
    // is withheld and names the failure that has not been reviewed.
    let before = row_of(&root, READ);
    assert_eq!(
        before.qualification,
        Qualification::Withheld,
        "the failing run must actually change the verdict, or the case proves nothing"
    );
    assert_eq!(before.reason, "conflict");
    assert!(before
        .unresolved
        .iter()
        .any(|u| u.starts_with("uncleared-executed-failure:")));

    // What silent loss produced: the failure-bearing input simply gone. This is the outcome the
    // old `.filter(|p| p.is_dir())` reached without saying a word.
    let moved = aside.join("moved-run");
    fs::rename(&fail, &moved).expect("move aside");
    assert_eq!(
        row_of(&root, READ).qualification,
        Qualification::AutomatedQualified,
        "losing the failure is exactly what turns this row green"
    );

    // R4: an unexplained entry standing where that run was is an error, not a filtered-out
    // non-directory. `check`/`build` refuse instead of publishing the green row above.
    fs::write(&fail, b"a run directory replaced by a file\n").expect("inject");
    let e = try_set_of(&root).expect_err("discovery must fail closed");
    assert!(e.contains("discovery-unexpected-entry"), "{e}");
    assert!(
        store::PERMITTED_RUNS_ENTRIES.is_empty(),
        "nothing writes a file beside the run directories; permitting one is a decision"
    );
}

#[test]
fn r4_a_wrong_type_runs_root_is_an_error_not_an_empty_layer() {
    let _guard = env_lock();
    let root = sandbox("r4-runs-root-type");
    std::env::set_var(ROOT_ENV, &root);
    sealed_run(Outcome::Fail, "the failing run");
    std::env::remove_var(ROOT_ENV);
    assert_eq!(row_of(&root, READ).qualification, Qualification::Blocked);

    let runs = root.join("auto").join("runs");
    fs::remove_dir_all(&runs).expect("clear");
    fs::write(&runs, b"not a directory\n").expect("inject");

    let e = try_set_of(&root).expect_err("a wrong-type root is not an absent root");
    assert!(e.contains("discovery-unexpected-root-type"), "{e}");
}

#[test]
fn r4_a_link_inside_the_runs_root_is_refused() {
    let _guard = env_lock();
    let root = sandbox("r4-runs-link");
    let elsewhere = sandbox("r4-runs-link-target");
    std::env::set_var(ROOT_ENV, &root);
    sealed_run(Outcome::Pass, "the only run");
    std::env::remove_var(ROOT_ENV);
    assert_eq!(
        row_of(&root, READ).qualification,
        Qualification::AutomatedQualified
    );

    // A junction beside the runs would let material from outside the evidence root be read as a
    // run — and could be re-pointed between discovery and reading.
    make_dir_link(&root.join("auto").join("runs").join("aliased"), &elsewhere);
    let e = try_set_of(&root).expect_err("a link inside an evidence root is refused");
    assert!(e.contains("discovery-reparse-point"), "{e}");
}

// ===================================================== end-to-end: layer B ====================

#[test]
fn r4_a_lost_manual_record_must_not_resolve_a_disagreement() {
    let _guard = env_lock();
    let root = sandbox("r4-manual-lost");
    std::env::set_var(ROOT_ENV, &root);
    sealed_run(Outcome::Pass, "the automated pass");
    std::env::remove_var(ROOT_ENV);

    // Two manual records that disagree, one of them nested deeper than the other. The
    // disagreement is unresolved, so the row is withheld (§4.3 row 3).
    write_manual(&root, &["nhl15-ps3", "db-presentation"], "m1", "fail");
    let deeper = write_manual(
        &root,
        &["nhl15-ps3", "db-presentation", "reviewed", "2026-09"],
        "m2",
        "pass",
    );
    let before = row_of(&root, READ);
    assert_eq!(before.qualification, Qualification::Withheld);
    assert_eq!(before.reason, "conflict");
    assert_eq!(
        store::load_manual(&root).expect("load").len(),
        2,
        "nested records are discovered at every depth"
    );

    // Losing the deeper record silently resolves the disagreement in favour of the pass: the
    // conflict simply is not there any more, and nothing in the output says a record went missing.
    fs::remove_file(&deeper).expect("remove");
    let after = row_of(&root, READ);
    assert!(
        !after.conflict.iter().any(|c| c == "manual-disagreement"),
        "losing a nested manual record is what dissolves the conflict: {:?}",
        after.conflict
    );
    assert_ne!(
        (after.qualification, after.reason.clone()),
        (before.qualification, before.reason.clone()),
        "and the verdict changes with it"
    );
    assert_eq!(
        after.reason, "stale-evidence",
        "the conflict is simply gone; what remains is the ordinary freshness verdict"
    );
}

#[test]
fn r4_a_wrong_type_manual_root_is_an_error_not_an_empty_layer() {
    let _guard = env_lock();
    let root = sandbox("r4-manual-root-type");
    std::env::set_var(ROOT_ENV, &root);
    sealed_run(Outcome::Pass, "the automated pass");
    std::env::remove_var(ROOT_ENV);
    write_manual(&root, &["nhl15-ps3", "db-presentation"], "m1", "pass");
    write_manual(&root, &["nhl15-ps3", "db-presentation"], "m2", "fail");
    assert_eq!(row_of(&root, READ).reason, "conflict");

    fs::remove_dir_all(root.join("manual")).expect("clear");
    fs::write(root.join("manual"), b"not a directory\n").expect("inject");

    let e = try_set_of(&root).expect_err("the whole of layer B must not read as absent");
    assert!(e.contains("discovery-unexpected-root-type"), "{e}");
    // And the V11 digest over the same tree refuses too, rather than fingerprinting nothing.
    let e = store::manual_digest(&root).expect_err("digest traversal must fail closed");
    assert!(
        e.to_string().contains("discovery-unexpected-root-type"),
        "{e}"
    );
}

#[test]
fn r4_a_link_inside_the_manual_tree_fails_both_discovery_and_the_digest() {
    let _guard = env_lock();
    let root = sandbox("r4-manual-link");
    let elsewhere = sandbox("r4-manual-link-target");
    write_manual(&root, &["nhl15-ps3", "db-presentation"], "m1", "pass");
    make_dir_link(&root.join("manual").join("aliased"), &elsewhere);

    let e = store::load_manual(&root).expect_err("record discovery refuses the link");
    assert!(e.to_string().contains("discovery-reparse-point"), "{e}");
    let e = store::manual_digest(&root).expect_err("the digest refuses the same link");
    assert!(e.to_string().contains("discovery-reparse-point"), "{e}");
}

#[test]
fn r4_the_digest_and_record_discovery_walk_the_same_tree() {
    let _guard = env_lock();
    let root = sandbox("r4-digest-agreement");
    write_manual(&root, &["nhl15-ps3", "db-presentation"], "m1", "pass");
    let before = store::manual_digest(&root).expect("digest");

    // The documented file-selection rule is preserved: only `.toml` is parsed as a record, so a
    // note left beside one does not become active evidence...
    let note = root
        .join("manual")
        .join("nhl15-ps3")
        .join("db-presentation")
        .join("README.md");
    fs::write(&note, b"why m1 was recorded\n").expect("note");
    assert_eq!(
        store::load_manual(&root).expect("load").len(),
        1,
        "a non-record file is excluded from parsing"
    );

    // ...but it is *not* excluded from sight: the V11 digest covers every file under `manual/`, so
    // nothing can be added, edited or removed there without the fingerprint changing.
    assert_ne!(
        store::manual_digest(&root).expect("digest"),
        before,
        "the digest walks the whole tree, not just the records"
    );
}

#[test]
fn r4_a_valid_empty_root_and_a_normal_tree_are_unaffected() {
    let _guard = env_lock();

    // Absent roots: absence still asserts nothing (R-ABSENT) and is still not an error.
    let empty = sandbox("r4-empty");
    let set = set_of(&empty);
    assert!(set.runs.is_empty() && set.manual.is_empty() && set.journals.is_empty());
    assert_eq!(
        store::manual_digest(&empty).expect("digest"),
        store::manual_digest(&sandbox("r4-empty-2")).expect("digest"),
        "two empty trees fingerprint identically"
    );

    // Present-but-empty roots, the other legitimate shape.
    fs::create_dir_all(empty.join("auto").join("runs")).expect("dirs");
    fs::create_dir_all(empty.join("manual")).expect("dirs");
    fs::create_dir_all(journal::journals_dir(&empty)).expect("dirs");
    assert!(set_of(&empty).runs.is_empty());

    // And an ordinary tree still loads and still qualifies.
    let root = sandbox("r4-normal");
    std::env::set_var(ROOT_ENV, &root);
    sealed_run(Outcome::Pass, "an ordinary run");
    std::env::remove_var(ROOT_ENV);
    assert_eq!(
        row_of(&root, READ).qualification,
        Qualification::AutomatedQualified
    );
    write_manual(&root, &["nhl15-ps3", "db-presentation"], "m1", "pass");
    let set = set_of(&root);
    assert_eq!(set.runs.len(), 1);
    assert_eq!(set.manual.len(), 1, "both layers load from a normal tree");
}

// ===================================================== end-to-end: journals ===================

#[test]
fn r4_the_journal_root_and_its_entries_share_the_platform_policy() {
    let _guard = env_lock();
    let root = sandbox("r4-journal-policy");
    let elsewhere = sandbox("r4-journal-policy-target");
    fs::create_dir_all(journal::journals_dir(&root)).expect("dirs");

    // A link *among* the journals — refused by `classify_entry` as `journal-unsupported-link`,
    // because R4 makes `read_entry_kind` classify a reparse point as `Other`.
    make_dir_link(&journal::journals_dir(&root).join("aliased"), &elsewhere);
    let e = try_set_of(&root).expect_err("a link among the journals is refused");
    assert!(e.contains("journal-unsupported-link"), "{e}");

    // A wrong-type journal root — now the same `discovery-unexpected-root-type` every other layer
    // reports, rather than a message unique to this loader.
    let other = sandbox("r4-journal-root-type");
    fs::create_dir_all(other.join("auto")).expect("dirs");
    fs::write(journal::journals_dir(&other), b"not a directory\n").expect("inject");
    let e = try_set_of(&other).expect_err("a wrong-type journal root is not an absent one");
    assert!(e.contains("discovery-unexpected-root-type"), "{e}");
}

// ===================================================== helper-level coverage ==================
//
// These exercise the `paths::` primitives directly. They are *not* end-to-end filesystem coverage:
// they exist for the branches that cannot be produced reliably on every platform.

#[test]
fn r4_helper_optional_dir_exists_separates_the_four_conditions() {
    let dir = sandbox("r4-helper-optional");

    assert!(
        !paths::optional_dir_exists(&dir.join("missing")).expect("absent is not an error"),
        "absent"
    );
    assert!(
        paths::optional_dir_exists(&dir).expect("present"),
        "present"
    );

    let file = dir.join("a-file");
    fs::write(&file, b"x").expect("write");
    let e = paths::optional_dir_exists(&file).expect_err("wrong type");
    assert!(
        e.to_string().contains("discovery-unexpected-root-type"),
        "{e}"
    );

    let link = dir.join("a-link");
    make_dir_link(&link, &dir);
    let e = paths::optional_dir_exists(&link).expect_err("a link is not a directory to walk");
    assert!(e.to_string().contains("discovery-reparse-point"), "{e}");
}

#[test]
fn r4_helper_a_failed_child_metadata_read_is_not_an_absence() {
    let dir = sandbox("r4-helper-metadata");
    // Deterministic injection: a path that cannot be stat'ed. `entry_kind` is only ever called on
    // an entry the directory listing just produced, so any failure here means the entry changed
    // under the walk — never that it is "not a directory".
    let e = paths::entry_kind(&dir.join("vanished")).expect_err("a stat failure must surface");
    assert!(e.to_string().contains("vanished"), "the path is named: {e}");
    assert_eq!(
        paths::entry_kind(&dir).expect("kind"),
        paths::EntryKind::Dir
    );
}

#[test]
fn r4_helper_walk_files_is_sorted_deep_and_refuses_links() {
    let dir = sandbox("r4-helper-walk");
    fs::create_dir_all(dir.join("b").join("deep")).expect("dirs");
    fs::write(dir.join("a.txt"), b"a").expect("w");
    fs::write(dir.join("b").join("deep").join("c.txt"), b"c").expect("w");

    let files = paths::walk_files(&dir).expect("walk");
    assert_eq!(files.len(), 2);
    assert!(
        files[0].ends_with("a.txt") && files[1].ends_with("c.txt"),
        "sorted depth-first"
    );

    // An absent directory yields nothing; a link inside one is refused.
    assert!(paths::walk_files(&dir.join("missing"))
        .expect("absent")
        .is_empty());
    make_dir_link(&dir.join("b").join("aliased"), &dir.join("b").join("deep"));
    let e = paths::walk_files(&dir).expect_err("no link is followed");
    assert!(e.to_string().contains("discovery-reparse-point"), "{e}");
}

#[test]
fn r4_helper_child_dirs_permits_only_what_is_named() {
    let dir = sandbox("r4-helper-children");
    fs::create_dir_all(dir.join("run-0001")).expect("dirs");
    fs::write(dir.join("index.json"), b"{}").expect("w");

    let e = paths::child_dirs(&dir, &[]).expect_err("an unnamed file is not filtered away");
    assert!(e.to_string().contains("discovery-unexpected-entry"), "{e}");

    let kept = paths::child_dirs(&dir, &["index.json"]).expect("permitted");
    assert_eq!(kept.len(), 1);
    assert!(kept[0].ends_with("run-0001"));
}
