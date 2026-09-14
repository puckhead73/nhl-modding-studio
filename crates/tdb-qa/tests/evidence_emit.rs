//! C0-S2-INT1 — the emitter observation lifecycle
//! ([`tdb_qa::evidence::emit`](../src/evidence/emit.rs)).
//!
//! Every case here runs over a **synthetic** evidence root under the system temp tree. Nothing
//! reads or writes `docs/audits/`, `anchors/`, a game tree or a private fixture, and no value
//! produced here is evidence about any real target.
//!
//! `QA_EVIDENCE_ROOT` is process-global, so the cases that set it hold [`ENV`] for their duration.
//! The default-mode case holds the same lock while asserting the variable is *unset*.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tdb_qa::evidence::emit::{Emitter, ROOT_ENV};
use tdb_qa::evidence::merge::{self, AutomatedAxis, Freshness, Qualification};
use tdb_qa::evidence::model::{EvidenceSet, Outcome, RunState};
use tdb_qa::evidence::validate::validate;
use tdb_qa::evidence::{store, vocab};

fn env_lock() -> MutexGuard<'static, ()> {
    static ENV: OnceLock<Mutex<()>> = OnceLock::new();
    ENV.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A synthetic root, unique per process and per call. Never a repository or game path.
fn sandbox(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir()
        .join("tdb-qa-evidence-emit")
        .join(format!("{name}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("sandbox");
    dir
}

const READ: &str = "nhl15-ps3/db-presentation/read";
const REPACK: &str = "nhl15-ps3/save-inner-container/container-repack";
const CASE: &str = "tdb_qa::evidence_emit::synthetic";

fn count_files(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| {
            if e.path().is_dir() {
                count_files(&e.path())
            } else {
                1
            }
        })
        .sum()
}

// ------------------------------------------------------------------ default mode ------

#[test]
fn without_the_opt_in_variable_an_emitter_creates_no_file_at_all() {
    let _guard = env_lock();
    std::env::remove_var(ROOT_ENV);
    let root = sandbox("default-mode");

    let mut ev = Emitter::from_env("synthetic", "db-read", &repo_root());
    assert!(!ev.is_enabled(), "no opt-in variable, no observation");
    assert_eq!(ev.journal_dir(), None);
    let key = ev.declare(READ);
    ev.observe(&key, CASE, Outcome::Pass, "would have been recorded");
    assert_eq!(ev.finish(), None);

    assert_eq!(count_files(&root), 0, "the sandbox must be untouched");
    assert!(
        !root.join("auto").exists(),
        "no evidence tree may be created in default mode"
    );
    assert_eq!(
        store::load_runs(&root)
            .expect("absent runs are not an error")
            .len(),
        0
    );
}

#[test]
fn an_evidence_root_inside_the_repository_is_refused() {
    let _guard = env_lock();
    let inside = repo_root().join("target/tmp-qa-evidence-refusal");
    std::fs::create_dir_all(&inside).expect("create");
    std::env::set_var(ROOT_ENV, &inside);
    let refused = std::panic::catch_unwind(|| {
        Emitter::from_env("synthetic", "db-read", &repo_root());
    });
    std::env::remove_var(ROOT_ENV);
    let _ = std::fs::remove_dir_all(&inside);
    assert!(
        refused.is_err(),
        "layer-A storage in git is undecided (U3); a root inside the repo must be refused"
    );
}

// ------------------------------------------------------------------ lifecycle ------

#[test]
fn a_completed_run_separates_a_pass_from_an_absent_fixture() {
    let _guard = env_lock();
    let root = sandbox("completed");
    std::env::set_var(ROOT_ENV, &root);
    let dir = {
        let mut ev = Emitter::from_env("synthetic", "db-read", &repo_root());
        assert!(ev.is_enabled());
        let read = ev.declare(READ);
        let repack = ev.declare(REPACK);
        ev.input_digest("synthetic/fixture", Some("a".repeat(64)));
        let value = ev.case(&read, CASE, || 41 + 1);
        assert_eq!(value, 42, "case returns the closure's value unchanged");
        ev.observe_hashed(&read, CASE, Outcome::Pass, &"b".repeat(64), "", "read ok");
        ev.observe(
            &repack,
            CASE,
            Outcome::SkippedMissingFixture,
            "no fixture present",
        );
        ev.finish().expect("sealed")
    };
    std::env::remove_var(ROOT_ENV);

    let run = store::load_run(&dir).expect("the sealed run loads");
    assert_eq!(run.state, RunState::Completed);
    assert_eq!(run.keys_declared.len(), 2, "both keys declared");
    assert_eq!(
        run.keys_executed.len(),
        1,
        "a skip is declared but never executed"
    );
    assert!(run.keys_executed.iter().all(|k| k.to_string() == READ));
    assert_eq!(run.observations.len(), 2);
    assert_eq!(
        run.manifest
            .input_digests
            .get("synthetic/fixture")
            .map(String::as_str),
        Some("a".repeat(64).as_str())
    );
    // Real toolchain identity, or the literal unknown — never a fabricated value.
    assert!(
        run.manifest.toolchain == "unknown" || run.manifest.toolchain.starts_with("rustc "),
        "{:?}",
        run.manifest.toolchain
    );
    // The journal is gone once the run is sealed: there is exactly one record, not two.
    assert!(
        !root.join("auto/in-progress").exists() || count_files(&root.join("auto/in-progress")) == 0
    );
    assert!(validate(&set_of(&root)).is_empty(), "the run validates");
}

#[test]
fn a_panicking_case_records_an_executed_failure_keeps_the_earlier_work_and_resumes_the_panic() {
    let _guard = env_lock();
    let root = sandbox("failure");
    std::env::set_var(ROOT_ENV, &root);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {})); // the panic below is expected; keep the log readable
    let escaped = std::panic::catch_unwind(|| {
        let mut ev = Emitter::from_env("synthetic", "db-read", &repo_root());
        let read = ev.declare(READ);
        let repack = ev.declare(REPACK);
        ev.observe(&read, CASE, Outcome::Pass, "recorded before the failure");
        ev.case(&repack, CASE, || {
            assert_eq!(1, 2, "the deliberate failure");
        });
        unreachable!("the panic must not be swallowed");
    });
    std::panic::set_hook(previous);
    std::env::remove_var(ROOT_ENV);

    assert!(
        escaped.is_err(),
        "the original failure must still propagate"
    );

    let runs = store::load_runs(&root).expect("load");
    assert_eq!(runs.len(), 1, "the drop guard sealed exactly one run");
    let run = &runs[0];
    assert_eq!(
        run.state,
        RunState::Failed,
        "a run whose case panicked is failed, never completed"
    );
    let outcomes: BTreeMap<String, Outcome> = run
        .observations
        .iter()
        .map(|o| (o.key.to_string(), o.outcome))
        .collect();
    assert_eq!(
        outcomes.get(READ),
        Some(&Outcome::Pass),
        "work completed before the panic is not discarded"
    );
    assert_eq!(
        outcomes.get(REPACK),
        Some(&Outcome::Fail),
        "the panicking case is an executed failure, not an absence"
    );
    assert!(
        run.observations
            .iter()
            .any(|o| o.outcome == Outcome::Fail && o.detail.contains("deliberate failure")),
        "the failure carries the assertion message"
    );

    // §4.2 / §12.3: a failed run is retained and cited, selects nothing, and its executed failure
    // withholds qualification rather than reading as "never attempted".
    let merged = merge::merge(&set_of(&root));
    let row = merged
        .rows
        .iter()
        .find(|r| r.key.to_string() == REPACK)
        .expect("row");
    assert_eq!(row.automated, AutomatedAxis::ExecutedUnselected);
    assert_eq!(row.qualification, Qualification::Withheld);
    let read_row = merged
        .rows
        .iter()
        .find(|r| r.key.to_string() == READ)
        .expect("row");
    assert_eq!(
        read_row.automated,
        AutomatedAxis::ExecutedUnselected,
        "a pass inside a failed run qualifies nothing either"
    );
}

#[test]
fn a_run_that_is_never_sealed_leaves_an_observable_journal_and_no_run() {
    let _guard = env_lock();
    let root = sandbox("interrupted");
    std::env::set_var(ROOT_ENV, &root);
    let journal = {
        let mut ev = Emitter::from_env("synthetic", "db-read", &repo_root());
        let read = ev.declare(READ);
        ev.observe(&read, CASE, Outcome::Pass, "recorded before the kill");
        let dir = ev.journal_dir().expect("enabled").to_path_buf();
        // A hard kill runs no destructor. `forget` reproduces exactly that: the sink is dropped
        // from scope without `Drop` ever running.
        std::mem::forget(ev);
        dir
    };
    std::env::remove_var(ROOT_ENV);

    assert!(journal.is_dir(), "the journal survives an unsealed run");
    assert!(
        journal.join("journal.json").is_file(),
        "run identity persisted before any case ran"
    );
    let declared = std::fs::read_to_string(journal.join("keys-declared.txt")).expect("declared");
    assert!(declared.contains(READ), "the declaration persisted");
    let observations =
        std::fs::read_to_string(journal.join("observations.csv")).expect("observations");
    assert!(
        observations.contains("recorded before the kill"),
        "observations persist as they happen, not at the end"
    );

    assert_eq!(
        store::load_runs(&root).expect("load").len(),
        0,
        "an unsealed journal is not a run and qualifies nothing"
    );
    assert!(
        !root.join("auto/runs").exists() || count_files(&root.join("auto/runs")) == 0,
        "nothing was sealed"
    );
}

#[test]
fn repeated_runs_are_all_retained() {
    let _guard = env_lock();
    let root = sandbox("repeated");
    std::env::set_var(ROOT_ENV, &root);
    let mut dirs = Vec::new();
    for detail in ["first", "second"] {
        let mut ev = Emitter::from_env("synthetic", "db-read", &repo_root());
        let read = ev.declare(READ);
        ev.observe(&read, &format!("{CASE}#{detail}"), Outcome::Pass, detail);
        dirs.push(ev.finish().expect("sealed"));
    }
    std::env::remove_var(ROOT_ENV);

    assert_ne!(dirs[0], dirs[1], "two runs never share a directory");
    let runs = store::load_runs(&root).expect("load");
    assert_eq!(runs.len(), 2, "nothing overwrites an earlier run (E10)");
    assert!(runs.iter().all(|r| r.state == RunState::Completed));
    assert!(validate(&set_of(&root)).is_empty());
}

// ------------------------------------------------------------------ qualification ------

#[test]
fn a_completed_pass_qualifies_only_while_its_implementation_digest_is_current() {
    let _guard = env_lock();
    let root = sandbox("qualification");
    std::env::set_var(ROOT_ENV, &root);
    {
        let mut ev = Emitter::from_env("synthetic", "db-read", &repo_root());
        let read = ev.declare(READ);
        ev.observe(&read, CASE, Outcome::Pass, "read ok");
        ev.finish().expect("sealed");
    }
    std::env::remove_var(ROOT_ENV);

    let set = set_of(&root);
    let declared = set.runs[0].manifest.implementation_digest.clone();
    assert!(
        declared.starts_with("sha256:"),
        "the emitter must record a real digest, not a placeholder: {declared}"
    );

    let row = |set: &EvidenceSet| {
        merge::merge(set)
            .rows
            .into_iter()
            .find(|r| r.key.to_string() == READ)
            .expect("row")
    };

    let fresh = row(&set);
    assert_eq!(fresh.automated, AutomatedAxis::Pass);
    assert_eq!(fresh.fresh, Freshness::Fresh);
    assert_eq!(fresh.qualification, Qualification::AutomatedQualified);
    assert!(
        fresh
            .citations_of("auto-run")
            .iter()
            .any(|c| c.id == set.runs[0].manifest.run_id && c.selected),
        "the run is cited, and selected, on the row it qualified"
    );

    // One byte of the declared scope changes → the same observation is retained, cited and stale,
    // and it qualifies nothing (R-CONTENT, §4.3 row 6).
    let mut stale_set = set_of(&root);
    for value in stale_set.current_scope_digests.values_mut() {
        *value = format!("sha256:{}", "0".repeat(64));
    }
    let stale = row(&stale_set);
    assert_eq!(stale.automated, AutomatedAxis::Pass, "history is retained");
    assert_eq!(stale.fresh, Freshness::Stale);
    assert_eq!(stale.qualification, Qualification::Withheld);
}

/// The evidence set a merge runs over, assembled from a sandbox root and the real repository's
/// current scope digests — the same two inputs `qa-evidence` uses.
fn set_of(root: &Path) -> EvidenceSet {
    EvidenceSet {
        journals: Vec::new(),
        runs: store::load_runs(root).expect("runs"),
        manual: store::load_manual(root).expect("manual"),
        legacy: store::load_legacy(root).expect("legacy"),
        implementation: BTreeMap::new(),
        current_scope_digests: tdb_qa::evidence::emit::scope_digests(&repo_root()),
        declared_universe: vocab::catalog_universe(),
    }
}
