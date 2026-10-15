//! C0-S2-INT1-R1 / -R2 — incomplete execution evidence, integrity gates, record framing,
//! journal-to-run equivalence, unmapped executions, structural variants and freshness dependencies.
//!
//! Every case runs over a **synthetic** evidence root under the system temp tree, or over a
//! synthetic repository built by the test. Nothing reads or writes `docs/audits/`, `anchors/`, a
//! game tree or a private fixture, and no value produced here is evidence about any real target.
//!
//! The R2 damage cases all share one shape: **start from an older, fresh, completed pass that is
//! genuinely `automated-qualified`**, then introduce one kind of damage and assert that the
//! qualification does not survive it silently.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tdb_qa::evidence::emit::{Emitter, ROOT_ENV};
use tdb_qa::evidence::merge::{self, AutomatedAxis, Freshness, Qualification};
use tdb_qa::evidence::model::{EvidenceSet, Outcome, Run, RunState};
use tdb_qa::evidence::scopes::Scopes;
use tdb_qa::evidence::{journal, publish, store, vocab, CoverageKey};

fn env_lock() -> MutexGuard<'static, ()> {
    static ENV: OnceLock<Mutex<()>> = OnceLock::new();
    ENV.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The one fixture every synthetic run attributes (C0-EMIT1-R2/R3): a real, read-only repository
/// file. A run that attributes nothing — or whose executed observations carry no identity among
/// its inputs — can never be fresh, so the emitters here attribute this file and hash every
/// executed observation with it. What the file is does not matter to these cases.
fn attribute_scopes_file(ev: &mut Emitter) -> String {
    let digest = tdb_qa::evidence::emit::file_digest(&repo_root().join("defs/qa/scopes.toml"));
    ev.input_digest("defs/qa/scopes.toml", digest.clone());
    digest.unwrap_or_default()
}

fn sandbox(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir()
        .join("tdb-qa-evidence-journal")
        .join(format!("{name}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("sandbox");
    dir
}

const READ: &str = "nhl15-ps3/db-presentation/read";
const REPACK: &str = "nhl15-ps3/save-inner-container/container-repack";
const REPACK_V4: &str = "nhl15-ps3/save-inner-container/container-repack/v4";
const CASE: &str = "tdb_qa::evidence_journal::synthetic";

fn key(k: &str) -> CoverageKey {
    CoverageKey::parse(k).expect("key")
}

fn set_of(root: &Path) -> EvidenceSet {
    try_set_of(root).expect("the evidence tree loads")
}

fn try_set_of(root: &Path) -> Result<EvidenceSet, String> {
    let runs = store::load_runs(root).map_err(|e| e.to_string())?;
    let journals = journal::load_journals(root, &runs).map_err(|e| e.to_string())?;
    let fixtures = tdb_qa::evidence::store::verify_fixture_inputs(&repo_root(), &runs).states;
    Ok(EvidenceSet {
        runs,
        journals,
        manual: store::load_manual(root).map_err(|e| e.to_string())?,
        legacy: store::load_legacy(root).map_err(|e| e.to_string())?,
        implementation: BTreeMap::new(),
        current_scope_digests: tdb_qa::evidence::emit::scope_digests(&repo_root()),
        current_input_digests: fixtures,
        declared_universe: vocab::catalog_universe(),
    })
}

fn row_of(set: &EvidenceSet, k: &str) -> merge::SummaryRow {
    merge::merge(set)
        .rows
        .into_iter()
        .find(|r| r.key.to_string() == k)
        .unwrap_or_else(|| panic!("no row for {k}"))
}

/// Run one emitter to completion, recording `outcome` on every listed key. Requires `ROOT_ENV` set.
fn sealed_run(keys: &[&str], outcome: Outcome, detail: &str) -> PathBuf {
    let mut ev = Emitter::from_env("synthetic", "db-read", &repo_root());
    let fixture = attribute_scopes_file(&mut ev);
    for k in keys {
        let parsed = ev.declare(k);
        ev.observe_hashed(
            &parsed,
            &format!("{CASE}#{detail}"),
            outcome,
            &fixture,
            "",
            detail,
        );
    }
    ev.finish().expect("sealed")
}

/// The shared R2 starting point: an older completed run that passes [`READ`] and genuinely
/// qualifies. Returns the evidence root and the sealed run directory.
fn older_qualified_pass(name: &str) -> (PathBuf, PathBuf) {
    let root = sandbox(name);
    std::env::set_var(ROOT_ENV, &root);
    let run = sealed_run(&[READ], Outcome::Pass, "the older completed pass");
    std::env::remove_var(ROOT_ENV);
    let before = row_of(&set_of(&root), READ);
    assert_eq!(
        before.qualification,
        Qualification::AutomatedQualified,
        "the baseline must actually qualify, or the case proves nothing"
    );
    (root, run)
}

/// A journal written by hand, so a test can plant exactly the on-disk state it means to.
struct JournalWriter {
    dir: PathBuf,
}

impl JournalWriter {
    fn new(root: &Path, journal_id: &str, sealed_run_id: Option<&str>) -> Self {
        Self::with_identity(
            root,
            journal::IdentityFile {
                schema: journal::IDENTITY_SCHEMA.to_string(),
                journal_id: journal_id.to_string(),
                emitter: "synthetic".to_string(),
                started_utc: "2026-09-08T00:00:00Z".to_string(),
                host_os: "synthetic".to_string(),
                toolchain: "unknown".to_string(),
                scope_id: "db-read".to_string(),
                scope_version: 2,
                implementation_digest: "unknown".to_string(),
                sealed_run_id: sealed_run_id.map(str::to_string),
            },
        )
    }

    fn with_identity(root: &Path, identity: journal::IdentityFile) -> Self {
        let dir = journal::journals_dir(root).join(&identity.journal_id);
        std::fs::create_dir_all(&dir).expect("journal dir");
        std::fs::write(
            dir.join("journal.json"),
            serde_json::to_string_pretty(&identity).expect("json"),
        )
        .expect("identity");
        JournalWriter { dir }
    }

    /// A journal that genuinely *is* the evidence of `run` — identity, declarations and
    /// observations all copied from it. This is what a seal whose cleanup failed leaves behind.
    fn mirroring(root: &Path, journal_id: &str, run: &Run) -> Self {
        let m = &run.manifest;
        let w = Self::with_identity(
            root,
            journal::IdentityFile {
                schema: journal::IDENTITY_SCHEMA.to_string(),
                journal_id: journal_id.to_string(),
                emitter: m.emitter.clone(),
                started_utc: m.started_utc.clone(),
                host_os: m.host_os.clone(),
                toolchain: m.toolchain.clone(),
                scope_id: m.scope_id.clone(),
                scope_version: m.scope_version,
                implementation_digest: m.implementation_digest.clone(),
                sealed_run_id: Some(run.run_id().to_string()),
            },
        );
        for k in &run.keys_declared {
            w.declare(&k.to_string());
        }
        for o in &run.observations {
            w.observe_full(
                &o.key.to_string(),
                &o.case_id,
                o.outcome,
                &o.fixture_sha256,
                &o.output_sha256,
                &o.detail,
            );
        }
        w
    }

    fn declare(&self, key: &str) -> &Self {
        let mut text =
            std::fs::read_to_string(self.dir.join("keys-declared.txt")).unwrap_or_default();
        text.push_str(key);
        text.push('\n');
        std::fs::write(self.dir.join("keys-declared.txt"), text).expect("declare");
        self
    }

    fn observe(&self, key: &str, case_id: &str, outcome: Outcome, detail: &str) -> &Self {
        self.observe_full(key, case_id, outcome, "", "", detail)
    }

    fn observe_full(
        &self,
        key: &str,
        case_id: &str,
        outcome: Outcome,
        fixture: &str,
        output: &str,
        detail: &str,
    ) -> &Self {
        let path = self.dir.join("observations.csv");
        let mut text = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| tdb_qa::evidence::csv::row(journal::OBSERVATION_HEADER));
        text.push_str(&journal::record_line(&[
            journal::OBSERVATION_ROW,
            key,
            case_id,
            outcome.as_str(),
            fixture,
            output,
            detail,
        ]));
        std::fs::write(&path, text).expect("observe");
        self
    }

    /// Chop `n` bytes off the end of a record file — what an interrupted append leaves behind.
    fn truncate(&self, name: &str, n: usize) {
        let path = self.dir.join(name);
        let bytes = std::fs::read(&path).expect("read");
        std::fs::write(&path, &bytes[..bytes.len() - n]).expect("truncate");
    }
}

// ========================================================== R2 — integrity gates ==============

#[test]
fn r2_a_corrupt_failure_record_withholds_the_older_completed_pass() {
    let _guard = env_lock();
    let (root, _run) = older_qualified_pass("r2-corrupt-failure");

    // A later, unsealed run declared the same key and persisted a record that is now damaged. Its
    // outcome could have been a failure on exactly this key — which is what cannot be known.
    let w = JournalWriter::new(&root, "synthetic-corrupt-0001", None);
    w.declare(READ);
    w.observe(READ, "later#1", Outcome::Fail, "about to be tampered with");
    let path = w.dir.join("observations.csv");
    let text = std::fs::read_to_string(&path).expect("read");
    std::fs::write(
        &path,
        text.replace("about to be tampered with", "silently rewritten!!!!!"),
    )
    .expect("tamper");

    let set = set_of(&root);
    let row = row_of(&set, READ);
    assert_ne!(
        row.qualification,
        Qualification::AutomatedQualified,
        "damaged evidence must not leave an older pass qualified"
    );
    assert_eq!(row.qualification, Qualification::Withheld);
    assert_eq!(row.reason, "damaged-evidence");
    assert!(
        row.unresolved
            .iter()
            .any(|u| u.starts_with("damaged-evidence:synthetic-corrupt-0001")),
        "{:?}",
        row.unresolved
    );
    // The damaged outcome and key are never guessed at: no observation is invented from it.
    assert!(set.journals[0].observations.is_empty());
    assert_eq!(set.journals[0].damaging_findings().len(), 1);
}

#[test]
fn r2_a_truncated_record_of_unknown_outcome_withholds_the_older_completed_pass() {
    let _guard = env_lock();
    let (root, _run) = older_qualified_pass("r2-truncated-unknown");

    let w = JournalWriter::new(&root, "synthetic-trunc-0001", None);
    w.declare(READ).declare(REPACK);
    w.observe(REPACK, "later#1", Outcome::Pass, "verified neighbour");
    w.observe(READ, "later#2", Outcome::Pass, "the interrupted record");
    w.truncate("observations.csv", 40);

    let set = set_of(&root);
    let j = &set.journals[0];
    assert_eq!(
        j.observations.len(),
        1,
        "the verified neighbour is preserved for diagnostics"
    );
    assert_eq!(j.observations[0].case_id, "later#1");
    assert!(j
        .integrity
        .iter()
        .any(|i| i == "truncated-trailing-record:observations.csv"));

    // Damage bounds to the journal's declared scope, so *both* declared keys are withheld — the
    // lost record could have been about either one.
    for k in [READ, REPACK] {
        let row = row_of(&set, k);
        assert_eq!(row.qualification, Qualification::Withheld, "{k}");
        assert_eq!(row.reason, "damaged-evidence", "{k}");
    }
}

#[test]
fn r2_an_unreadable_or_wrong_type_record_file_is_never_read_as_empty() {
    let _guard = env_lock();
    let (root, _run) = older_qualified_pass("r2-wrong-type");
    let w = JournalWriter::new(&root, "synthetic-type-0001", None);
    w.declare(READ);

    // A directory where a record file belongs. The R1 loader turned every read error into an empty
    // list, which reported this as "the emitter recorded nothing".
    std::fs::create_dir(w.dir.join("observations.csv")).expect("plant");
    let err = try_set_of(&root).expect_err("must fail closed");
    assert!(err.contains("observations.csv"), "{err}");
    assert!(
        err.contains("journal-unexpected-entry-type"),
        "the diagnostic must say what was found: {err}"
    );
}

#[test]
fn r2_invalid_utf8_in_a_record_file_is_an_error_not_an_absence() {
    let _guard = env_lock();
    let (root, _run) = older_qualified_pass("r2-invalid-utf8");
    let w = JournalWriter::new(&root, "synthetic-utf8-0001", None);
    w.declare(READ);
    w.observe(READ, "later#1", Outcome::Pass, "intact");
    let path = w.dir.join("observations.csv");
    let mut bytes = std::fs::read(&path).expect("read");
    bytes.extend_from_slice(&[0xff, 0xfe, b'\n']);
    std::fs::write(&path, bytes).expect("plant");

    let err = try_set_of(&root).expect_err("must fail closed");
    assert!(err.contains("journal-unreadable-file"), "{err}");
    assert!(err.contains("not valid UTF-8"), "{err}");
}

#[test]
fn r2_damaged_declarations_are_fatal_because_nothing_could_bound_the_damage() {
    let _guard = env_lock();

    // (a) a declaration list cut mid-line
    let (root, _run) = older_qualified_pass("r2-declarations-truncated");
    let w = JournalWriter::new(&root, "synthetic-decl-0001", None);
    w.declare(READ).declare(REPACK);
    w.truncate("keys-declared.txt", 12);
    let err = try_set_of(&root).expect_err("must fail closed");
    assert!(err.contains("journal-declarations"), "{err}");
    assert!(err.contains("declared scope is unknown"), "{err}");

    // (b) a declaration that is not a coverage key
    let (root, _run) = older_qualified_pass("r2-declarations-unparseable");
    JournalWriter::new(&root, "synthetic-decl-0002", None).declare("not a coverage key at all");
    let err = try_set_of(&root).expect_err("must fail closed");
    assert!(err.contains("journal-declarations"), "{err}");

    // (c) damage with nothing declared at all — unbounded, so also fatal
    let (root, _run) = older_qualified_pass("r2-damage-unbounded");
    let w = JournalWriter::new(&root, "synthetic-decl-0003", None);
    w.observe(READ, "later#1", Outcome::Pass, "only record");
    w.truncate("observations.csv", 30);
    let err = try_set_of(&root).expect_err("must fail closed");
    assert!(err.contains("journal-unbounded-damage"), "{err}");
}

#[test]
fn r2_a_false_sealed_run_id_naming_an_existing_unrelated_run_is_not_deduplicated() {
    let _guard = env_lock();
    let (root, run) = older_qualified_pass("r2-false-sealed-id");
    let run_id = run.file_name().unwrap().to_string_lossy().into_owned();

    // A journal that has nothing to do with that run, but names it. Under R1 the matching string
    // alone deleted this journal's evidence from the merge.
    let w = JournalWriter::new(&root, "synthetic-false-0001", Some(&run_id));
    w.declare(READ);
    w.observe(READ, "unrelated#1", Outcome::Fail, "an unrelated failure");

    let set = set_of(&root);
    assert_eq!(
        set.journals.len(),
        1,
        "an unverified link must not delete the journal"
    );
    let finding = set.journals[0]
        .integrity
        .iter()
        .find(|i| i.starts_with("sealed-run-mismatch:"))
        .unwrap_or_else(|| panic!("{:?}", set.journals[0].integrity));
    assert!(finding.contains(&run_id), "{finding}");

    let row = row_of(&set, READ);
    assert_ne!(row.qualification, Qualification::AutomatedQualified);
    assert_eq!(row.qualification, Qualification::Withheld);
    // The mismatch is visible in the published citations, not only in a log line.
    assert!(row
        .citations
        .iter()
        .any(|c| c.kind == "auto-journal" && c.detail.contains("sealed-run-mismatch")));
}

#[test]
fn r2_a_legitimate_sealing_overlap_is_still_deduplicated_and_still_qualifies() {
    let _guard = env_lock();
    let (root, run_dir) = older_qualified_pass("r2-legitimate-overlap");
    let run = store::load_run(&run_dir).expect("load");

    // What a seal whose cleanup failed really leaves: a journal that *is* the run's evidence.
    JournalWriter::mirroring(&root, "synthetic-mirror-0001", &run);

    let set = set_of(&root);
    assert!(
        set.journals.is_empty(),
        "a verified link is the same evidence, not a second source"
    );
    let row = row_of(&set, READ);
    assert_eq!(row.citations.len(), 1, "cited once: {:?}", row.citations);
    assert_eq!(row.qualification, Qualification::AutomatedQualified);
}

#[test]
fn r2_a_sealed_run_that_lost_a_record_the_journal_proves_was_made_is_a_mismatch() {
    let _guard = env_lock();
    let (root, run_dir) = older_qualified_pass("r2-incomplete-seal");
    let run = store::load_run(&run_dir).expect("load");

    // Identity matches, but the journal holds a verified observation the run does not.
    let w = JournalWriter::mirroring(&root, "synthetic-mirror-0002", &run);
    w.declare(REPACK);
    w.observe(
        REPACK,
        "extra#1",
        Outcome::Fail,
        "never made it into the run",
    );

    let set = set_of(&root);
    assert_eq!(set.journals.len(), 1, "the extra evidence is not discarded");
    assert!(set.journals[0]
        .integrity
        .iter()
        .any(|i| i.contains("absent from the run")));
    assert_eq!(row_of(&set, READ).qualification, Qualification::Withheld);
}

#[test]
fn r2_a_clean_active_journal_does_not_invalidate_an_older_completed_pass() {
    let _guard = env_lock();
    let (root, _run) = older_qualified_pass("r2-clean-active");

    // An undamaged run in flight, mid-execution, with a clean pass of its own.
    JournalWriter::new(&root, "synthetic-clean-0001", None)
        .declare(READ)
        .observe(READ, "live#1", Outcome::Pass, "clean and incomplete");

    let set = set_of(&root);
    assert!(set.journals[0].damaging_findings().is_empty());
    let row = row_of(&set, READ);
    assert_eq!(
        row.qualification,
        Qualification::AutomatedQualified,
        "an active run is not by itself a reason to distrust older evidence"
    );
    assert!(
        row.unresolved
            .iter()
            .any(|u| u.starts_with("executed-only-in-unsealed-journal:")),
        "but the incomplete work is still reported"
    );
}

#[test]
fn r2_an_ordinary_completed_run_is_unaffected_by_any_of_this() {
    let _guard = env_lock();
    let (root, _run) = older_qualified_pass("r2-ordinary");
    let set = set_of(&root);
    assert!(set.journals.is_empty(), "a sealed run leaves no journal");
    let row = row_of(&set, READ);
    assert_eq!(row.automated, AutomatedAxis::Pass);
    assert_eq!(row.fresh, Freshness::Fresh);
    assert_eq!(row.qualification, Qualification::AutomatedQualified);
    assert!(row.unresolved.is_empty(), "{:?}", row.unresolved);
}

// ========================================================== R2 — record framing ===============

#[test]
fn r2_an_interruption_inside_a_quoted_multiline_field_costs_only_that_record() {
    let _guard = env_lock();
    let root = sandbox("r2-multiline");
    let multiline = "first line\nsecond line, with \"quotes\"\r\nand a \u{1f} unit separator";

    let w = JournalWriter::new(&root, "synthetic-multiline-0001", None);
    w.declare(READ).declare(REPACK);
    w.observe(READ, "kept#1", Outcome::Pass, multiline);
    w.observe(REPACK, "lost#1", Outcome::Fail, multiline);

    // Escaping makes one record one line, so the file is header + exactly 2 records.
    let text = std::fs::read_to_string(w.dir.join("observations.csv")).expect("read");
    assert_eq!(text.lines().count(), 3, "{text}");

    // Interrupt inside what would otherwise have been a quoted multi-line field.
    w.truncate("observations.csv", 30);
    let set = set_of(&root);
    let j = &set.journals[0];
    assert_eq!(j.observations.len(), 1, "the earlier record survives");
    assert_eq!(j.observations[0].case_id, "kept#1");
    assert_eq!(
        j.observations[0].detail, multiline,
        "and its multi-line value round-trips exactly"
    );
    assert!(j
        .integrity
        .iter()
        .any(|i| i == "truncated-trailing-record:observations.csv"));
}

#[test]
fn r2_a_journal_written_under_the_old_framing_is_refused_not_reinterpreted() {
    let _guard = env_lock();
    let root = sandbox("r2-old-schema");
    let dir = journal::journals_dir(&root).join("synthetic-old-0001");
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(
        dir.join("journal.json"),
        r#"{"schema":"qa-run-journal/2","journal_id":"synthetic-old-0001","emitter":"s","started_utc":"","host_os":"","toolchain":"","scope_id":"db-read","scope_version":2,"implementation_digest":"unknown"}"#,
    )
    .expect("write");
    let err = try_set_of(&root).expect_err("must be refused");
    assert!(err.contains("unsupported schema"), "{err}");
    assert!(err.contains("qa-run-journal/3"), "{err}");
}

// ========================================================== R1 — carried forward ==============

#[test]
fn an_incomplete_failure_withholds_a_key_an_older_completed_run_had_passed() {
    let _guard = env_lock();
    let (root, run) = older_qualified_pass("older-pass-plus-incomplete-failure");

    JournalWriter::new(&root, "synthetic-later-0001", None)
        .declare(READ)
        .observe(READ, "later#1", Outcome::Fail, "the persisted failure");

    let set = set_of(&root);
    assert_eq!(set.journals.len(), 1);
    let after = row_of(&set, READ);
    assert_eq!(
        after.automated,
        AutomatedAxis::Pass,
        "the completed run is still selected and still cited"
    );
    assert_eq!(after.qualification, Qualification::Withheld);
    assert_eq!(after.reason, "unreviewed-failure");
    assert!(
        after
            .unresolved
            .iter()
            .any(|u| u.starts_with("uncleared-executed-failure:synthetic-later-0001")),
        "{:?}",
        after.unresolved
    );
    let ids: Vec<(&str, &str, bool)> = after
        .citations
        .iter()
        .map(|c| (c.kind, c.state.as_str(), c.selected))
        .collect();
    assert!(ids.contains(&("auto-run", "completed", true)), "{ids:?}");
    assert!(
        ids.contains(&("auto-journal", "incomplete", false)),
        "{ids:?}"
    );
    assert!(run.is_dir());
}

#[test]
fn an_incomplete_pass_confers_no_qualification_on_its_own() {
    let _guard = env_lock();
    let root = sandbox("incomplete-pass");
    JournalWriter::new(&root, "synthetic-live-0001", None)
        .declare(READ)
        .observe(READ, "live#1", Outcome::Pass, "persisted, not sealed");

    let row = row_of(&set_of(&root), READ);
    assert_eq!(
        row.automated,
        AutomatedAxis::ExecutedUnselected,
        "a journal is never selected"
    );
    assert_ne!(row.qualification, Qualification::AutomatedQualified);
    assert_eq!(row.fresh, Freshness::Unknown);
    assert_eq!(row.qualification, Qualification::Withheld);
    assert_eq!(row.reason, "stale-evidence");
    assert!(row
        .unresolved
        .iter()
        .any(|u| u == "executed-only-in-unsealed-journal:synthetic-live-0001"));
}

#[test]
fn an_unexecuted_declaration_stays_distinct_from_an_executed_observation() {
    let _guard = env_lock();
    let root = sandbox("declared-not-executed");
    JournalWriter::new(&root, "synthetic-live-0002", None)
        .declare(READ)
        .declare(REPACK)
        .observe(REPACK, "live#1", Outcome::Pass, "executed");

    let set = set_of(&root);
    let declared_only = row_of(&set, READ);
    let executed = row_of(&set, REPACK);
    assert_eq!(declared_only.automated, AutomatedAxis::DeclaredNotExecuted);
    assert!(declared_only
        .unresolved
        .iter()
        .any(|u| u == "declared-in-unsealed-journal-not-executed:synthetic-live-0002"));
    assert_eq!(executed.automated, AutomatedAxis::ExecutedUnselected);
    assert_ne!(declared_only.automated, executed.automated);
}

#[test]
fn an_empty_journal_contributes_nothing_and_is_not_an_error() {
    let _guard = env_lock();
    let root = sandbox("empty-journal");
    JournalWriter::new(&root, "synthetic-empty-0001", None);

    let set = set_of(&root);
    assert_eq!(set.journals.len(), 1, "identity alone is still a journal");
    assert!(set.journals[0].keys_declared.is_empty());
    assert!(set.journals[0].observations.is_empty());
    assert!(
        set.journals[0].integrity.is_empty(),
        "an empty journal is not corrupt"
    );
    let row = row_of(&set, READ);
    assert_eq!(row.automated, AutomatedAxis::NotAttempted);
    assert!(row.citations.is_empty(), "an empty journal cites nothing");
}

#[test]
fn a_journal_naming_an_absent_run_is_kept_flagged_and_still_trusted() {
    let _guard = env_lock();
    let (root, run_dir) = older_qualified_pass("sealed-run-missing");
    let run = store::load_run(&run_dir).expect("load");
    let run_id = run.run_id().to_string();
    JournalWriter::mirroring(&root, "synthetic-mirror-0003", &run);
    std::fs::remove_dir_all(&run_dir).expect("remove run");

    let set = set_of(&root);
    assert_eq!(set.journals.len(), 1, "the only surviving copy is kept");
    assert!(set.journals[0]
        .integrity
        .iter()
        .any(|i| i == &format!("sealed-run-missing:{run_id}")));
    assert!(
        set.journals[0].damaging_findings().is_empty(),
        "a missing link is not damaged evidence: every record is still digest-verified"
    );
    assert_ne!(
        row_of(&set, READ).reason,
        "damaged-evidence",
        "and it must not be treated as corruption"
    );
}

#[test]
fn check_and_build_leave_the_journal_bytes_untouched() {
    let _guard = env_lock();
    let root = sandbox("read-only");
    JournalWriter::new(&root, "synthetic-live-0003", None)
        .declare(READ)
        .observe(READ, "live#1", Outcome::Pass, "persisted");
    let dir = journal::journals_dir(&root).join("synthetic-live-0003");
    let before = journal::journal_bytes(&dir).expect("hash");

    let set = set_of(&root);
    let merged = merge::merge(&set);
    let inputs = publish::BuildInputs {
        tool_version: "synthetic".into(),
        merge_input_digests: BTreeMap::new(),
        fixture_verification: BTreeMap::new(),
        scope_definitions_digest: "unknown".into(),
        divergence: Vec::new(),
        unmapped: Vec::new(),
    };
    let out = publish::build(&merged, &inputs);
    let layout = tdb_qa::evidence::paths::Layout::new(&root).expect("layout");
    let out_dir = sandbox("read-only-out");
    out.write_to(&out_dir, &inputs, &layout).expect("write");

    assert_eq!(
        before,
        journal::journal_bytes(&dir).expect("hash"),
        "loading and building must not touch the source bytes"
    );
}

#[test]
fn a_corrupt_record_is_dropped_and_reported_while_its_neighbours_survive() {
    let _guard = env_lock();
    let root = sandbox("corrupt-record");
    let w = JournalWriter::new(&root, "synthetic-corrupt-0002", None);
    w.declare(READ).declare(REPACK);
    w.observe(READ, "good#1", Outcome::Pass, "intact");
    w.observe(REPACK, "bad#1", Outcome::Pass, "about to be tampered with");
    w.observe(READ, "good#2", Outcome::RefusedClean, "also intact");

    let path = w.dir.join("observations.csv");
    let text = std::fs::read_to_string(&path).expect("read");
    let tampered = text.replace("about to be tampered with", "silently rewritten!!!!!");
    assert_ne!(text, tampered);
    std::fs::write(&path, tampered).expect("write");

    let set = set_of(&root);
    let j = &set.journals[0];
    assert_eq!(
        j.observations.len(),
        2,
        "only the tampered record is dropped"
    );
    assert!(j.observations.iter().all(|o| o.case_id != "bad#1"));
    assert!(
        j.integrity
            .iter()
            .any(|i| i.starts_with("corrupt-record:observations.csv:")),
        "{:?}",
        j.integrity
    );
    assert!(
        j.observations.iter().any(|o| o.case_id == "good#1")
            && j.observations.iter().any(|o| o.case_id == "good#2"),
        "records either side of the damage are unaffected"
    );
}

#[test]
fn a_journal_whose_identity_cannot_be_established_fails_closed() {
    let _guard = env_lock();
    for (name, plant) in [
        ("absent", 0usize),
        ("unparseable", 1),
        ("wrong-schema", 2),
        ("id-mismatch", 3),
    ] {
        let root = sandbox(&format!("identity-{name}"));
        let dir = journal::journals_dir(&root).join("synthetic-identity-0001");
        std::fs::create_dir_all(&dir).expect("dir");
        match plant {
            0 => {}
            1 => std::fs::write(dir.join("journal.json"), "{not json").expect("write"),
            2 => std::fs::write(
                dir.join("journal.json"),
                r#"{"schema":"qa-run-journal/1","journal_id":"synthetic-identity-0001","emitter":"s","started_utc":"","host_os":"","toolchain":"","scope_id":"db-read","scope_version":2,"implementation_digest":"unknown"}"#,
            )
            .expect("write"),
            _ => std::fs::write(
                dir.join("journal.json"),
                r#"{"schema":"qa-run-journal/3","journal_id":"a-different-id","emitter":"s","started_utc":"","host_os":"","toolchain":"","scope_id":"db-read","scope_version":2,"implementation_digest":"unknown"}"#,
            )
            .expect("write"),
        }
        std::fs::write(
            dir.join("observations.csv"),
            tdb_qa::evidence::csv::row(journal::OBSERVATION_HEADER),
        )
        .expect("write");

        let err = journal::load_journals(&root, &[])
            .expect_err(&format!("{name} must fail closed"))
            .to_string();
        assert!(
            err.contains("journal.json"),
            "the diagnostic must name the file: {err}"
        );
    }
}

#[test]
fn a_persistence_failure_makes_the_workflow_non_success_and_retains_what_was_written() {
    let _guard = env_lock();
    let root = sandbox("persist-failure");
    std::env::set_var(ROOT_ENV, &root);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = std::panic::catch_unwind(|| {
        let mut ev = Emitter::from_env("synthetic", "db-read", &repo_root());
        let fixture = attribute_scopes_file(&mut ev);
        let read = ev.declare(READ);
        ev.observe_hashed(
            &read,
            CASE,
            Outcome::Pass,
            &fixture,
            "",
            "persisted before the fault",
        );

        let dir = ev.journal_dir().expect("enabled").to_path_buf();
        std::fs::remove_file(dir.join("observations.csv")).expect("remove");
        std::fs::create_dir(dir.join("observations.csv")).expect("plant");

        let repack = ev.declare(REPACK);
        ev.observe_hashed(
            &repack,
            CASE,
            Outcome::Pass,
            &fixture,
            "",
            "this one cannot be persisted",
        );
        unreachable!("a persistence failure must not be swallowed");
    });
    std::panic::set_hook(previous);
    std::env::remove_var(ROOT_ENV);

    let message = outcome
        .err()
        .map(|p| {
            p.downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_string()))
                .unwrap_or_default()
        })
        .expect("the workflow must not report success");
    assert!(message.contains("could not persist"), "{message}");
    assert!(message.contains("retained at"), "{message}");

    let runs = store::load_runs(&root).expect("runs");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].state, RunState::Failed);
    assert!(runs[0]
        .observations
        .iter()
        .any(|o| o.detail == "persisted before the fault"));
}

#[test]
fn an_unmapped_execution_is_published_verbatim_and_qualifies_nothing() {
    let _guard = env_lock();
    let root = sandbox("unmapped");
    std::env::set_var(ROOT_ENV, &root);
    let run = {
        let mut ev = Emitter::from_env("synthetic", "db-read", &repo_root());
        let fixture = attribute_scopes_file(&mut ev);
        let read = ev.declare(READ);
        ev.observe_hashed(&read, CASE, Outcome::Pass, &fixture, "", "mapped");
        ev.observe_unmapped(
            "nhl15-ps3",
            "retire-unretire-jersey",
            "synthetic#retire",
            Outcome::Pass,
            "ab".repeat(32).as_str(),
            "operation-not-in-closed-db-operations-vocabulary",
            "applied and fully reversible",
        );
        ev.finish().expect("sealed")
    };
    std::env::remove_var(ROOT_ENV);

    let reloaded = store::load_run(&run).expect("load");
    assert_eq!(reloaded.unmapped.len(), 1);
    let u = &reloaded.unmapped[0];
    assert_eq!(u.emitter_operation, "retire-unretire-jersey");
    assert_eq!(u.outcome, Outcome::Pass);
    assert_eq!(u.fixture_sha256, "ab".repeat(32));

    let inputs = publish::BuildInputs {
        tool_version: "synthetic".into(),
        merge_input_digests: BTreeMap::new(),
        fixture_verification: BTreeMap::new(),
        scope_definitions_digest: "unknown".into(),
        divergence: Vec::new(),
        unmapped: vec![publish::UnmappedRow {
            source_kind: "auto-run".into(),
            source_id: reloaded.run_id().to_string(),
            source_state: reloaded.state.as_str().to_string(),
            target: u.target.clone(),
            emitter_operation: u.emitter_operation.clone(),
            case_id: u.case_id.clone(),
            outcome: u.outcome.as_str().to_string(),
            fixture_sha256: u.fixture_sha256.clone(),
            unresolved: u.unresolved.clone(),
            detail: u.detail.clone(),
        }],
    };
    let out = publish::build(&merge::merge(&set_of(&root)), &inputs);
    let published = out.files.get(publish::UNMAPPED).expect("the unmapped file");
    assert!(published.contains("retire-unretire-jersey"), "{published}");
    assert!(published.contains("operation-not-in-closed-db-operations-vocabulary"));
    let matrix = out.files.get(publish::MATRIX).expect("matrix");
    assert!(
        !matrix.contains("retire-unretire-jersey"),
        "an unmapped execution must never appear as a coverage cell"
    );
}

#[test]
fn a_variant_cell_is_never_required_and_never_satisfies_its_parent() {
    let _guard = env_lock();
    let root = sandbox("variants");
    std::env::set_var(ROOT_ENV, &root);
    {
        let mut ev = Emitter::from_env("synthetic", "save-containers", &repo_root());
        let fixture = attribute_scopes_file(&mut ev);
        let v4 = ev.declare(REPACK_V4);
        ev.observe_faceted(
            &v4,
            CASE,
            Outcome::Pass,
            &fixture,
            "",
            "repacked",
            &[("codec", "v4"), ("save-mode", "roster")],
        );
        ev.finish().expect("sealed");
    }
    std::env::remove_var(ROOT_ENV);

    let set = set_of(&root);
    let variant = row_of(&set, REPACK_V4);
    let parent = row_of(&set, REPACK);

    assert_eq!(
        variant.applicability,
        tdb_qa::evidence::Applicability::Unknown
    );
    assert_eq!(variant.automated, AutomatedAxis::Pass);
    assert_eq!(variant.qualification, Qualification::Withheld);
    assert_eq!(variant.reason, "unknown-applicability");

    assert_eq!(
        parent.applicability,
        tdb_qa::evidence::Applicability::Required
    );
    assert_eq!(parent.automated, AutomatedAxis::NotAttempted);
    assert_ne!(parent.qualification, Qualification::AutomatedQualified);

    let run = &set.runs[0];
    let names: BTreeSet<(&str, &str)> = run
        .facets
        .iter()
        .map(|f| (f.name.as_str(), f.value.as_str()))
        .collect();
    assert!(names.contains(&("codec", "v4")), "{names:?}");
    assert!(names.contains(&("save-mode", "roster")), "{names:?}");
    assert!(run.facets.iter().all(|f| f.key == REPACK_V4));
}

#[test]
fn a_research_only_unit_still_reports_not_applicable_on_a_variant_key() {
    let resolved =
        vocab::applicability_for_key(&key("nhl07-ps3/save-inner-container/container-repack/v3"))
            .expect("resolved");
    assert_eq!(
        resolved.value,
        tdb_qa::evidence::Applicability::NotApplicable
    );
    assert!(resolved.reason.contains("research-only-unit"));
}

/// A synthetic repository: a scope definition plus the files it declares.
fn synthetic_repo(dir: &Path, assertion_body: &str) -> String {
    std::fs::create_dir_all(dir.join("defs/qa")).expect("defs");
    std::fs::create_dir_all(dir.join("crates/impl/src")).expect("impl");
    std::fs::create_dir_all(dir.join("crates/emitter/tests")).expect("tests");
    std::fs::write(dir.join("crates/impl/src/lib.rs"), "fn under_test() {}\n").expect("impl");
    std::fs::write(
        dir.join("crates/emitter/tests/matrix.rs"),
        format!("fn case() {{ {assertion_body} }}\n"),
    )
    .expect("test");
    let scopes = r#"
schema = "qa-scopes/1"

[[scope]]
id = "synthetic"
version = 1
paths = ["crates/impl/src/**"]
mapping = ["crates/emitter/tests/matrix.rs"]
"#;
    std::fs::write(dir.join("defs/qa/scopes.toml"), scopes).expect("scopes");
    let parsed = Scopes::parse(scopes).expect("parse");
    let contents = store::collect_scope_contents(dir, &parsed).expect("contents");
    parsed.digest("synthetic", &contents, &|_| None)
}

#[test]
fn changing_the_assertion_or_mapping_logic_makes_prior_evidence_stale() {
    let a = sandbox("fresh-a");
    let b = sandbox("fresh-b");
    let c = sandbox("fresh-c");

    let first = synthetic_repo(&a, "assert!(x == 1);");
    let same = synthetic_repo(&b, "assert!(x == 1);");
    let changed = synthetic_repo(&c, "assert!(x == 2);");

    assert!(first.starts_with("sha256:"), "a real digest: {first}");
    assert_eq!(first, same, "identical content is identical evidence");
    assert_ne!(
        first, changed,
        "an observation is a claim about what a case asserted; changing the assertion must \
         invalidate it exactly as changing the implementation does"
    );

    // The run's one attributed fixture is unchanged, so the only thing that moved is the
    // implementation digest — and that alone must read as `stale`, not `unknown`.
    let set = EvidenceSet {
        current_scope_digests: BTreeMap::from([(("db-read".to_string(), 2), changed)]),
        current_input_digests: BTreeMap::from([(
            SYNTHETIC_FIXTURE.to_string(),
            tdb_qa::evidence::model::InputState::Present(SYNTHETIC_FIXTURE_SHA.to_string()),
        )]),
        ..declared_pass(&first)
    };
    let row = row_of(&set, READ);
    assert_eq!(row.fresh, Freshness::Stale);
    assert_eq!(row.qualification, Qualification::Withheld);
    assert_eq!(row.reason, "stale-evidence");
}

const SYNTHETIC_FIXTURE: &str = "anchors/synthetic/databases/synthetic.db";
const SYNTHETIC_FIXTURE_SHA: &str =
    "4444444444444444444444444444444444444444444444444444444444444444";

/// An in-memory completed run that passed [`READ`] with `digest` as its implementation digest,
/// attributing one synthetic fixture (C0-EMIT1-R2: an unattributed run is never fresh).
fn declared_pass(digest: &str) -> EvidenceSet {
    use tdb_qa::evidence::model::{Observation, RunManifest, SupersedeMode};
    let k = key(READ);
    let run = Run {
        manifest: RunManifest {
            run_id: "synthetic-0001".into(),
            emitter: "synthetic".into(),
            started_utc: String::new(),
            host_os: String::new(),
            toolchain: String::new(),
            scope_id: "db-read".into(),
            scope_version: 2,
            implementation_digest: digest.to_string(),
            input_digests: BTreeMap::from([(
                SYNTHETIC_FIXTURE.to_string(),
                SYNTHETIC_FIXTURE_SHA.to_string(),
            )]),
            supersedes: Vec::new(),
            supersede_mode: SupersedeMode::None,
            supersede_keys: Vec::new(),
        },
        state: RunState::Completed,
        ended_utc: String::new(),
        failure: None,
        keys_declared: BTreeSet::from([k.clone()]),
        keys_executed: BTreeSet::from([k.clone()]),
        observations: vec![Observation {
            key: k,
            case_id: CASE.into(),
            outcome: Outcome::Pass,
            fixture_sha256: SYNTHETIC_FIXTURE_SHA.to_string(),
            output_sha256: String::new(),
            detail: String::new(),
        }],
        unmapped: Vec::new(),
        facets: Vec::new(),
    };
    EvidenceSet {
        runs: vec![run],
        journals: Vec::new(),
        manual: Vec::new(),
        legacy: Vec::new(),
        implementation: BTreeMap::new(),
        current_scope_digests: BTreeMap::new(),
        current_input_digests: BTreeMap::new(),
        declared_universe: vocab::catalog_universe(),
    }
}

// ============================= R3 — dedup integrity and fail-closed discovery ==============

/// Assert the combined-state defect is fixed: a journal that names a genuinely matching sealed run
/// but carries damage is retained, cited and keeps withholding its declared scope.
fn assert_damaged_leftover_is_retained(set: &EvidenceSet, journal_id: &str) {
    assert_eq!(
        set.journals.len(),
        1,
        "a damaged journal is never deduplicated: its dropped records are exactly the ones \
         equivalence could not examine"
    );
    let j = &set.journals[0];
    assert_eq!(j.journal_id, journal_id);
    assert!(
        j.integrity
            .iter()
            .any(|i| i.starts_with("sealed-run-unverifiable:")),
        "the reason it survived dedup must be recorded: {:?}",
        j.integrity
    );

    let row = row_of(set, READ);
    assert!(
        row.citations
            .iter()
            .any(|c| c.kind == "auto-journal" && c.id == journal_id),
        "the retained journal must still be cited: {:?}",
        row.citations
    );
    assert_ne!(
        row.qualification,
        Qualification::AutomatedQualified,
        "the defect: dedup discarded the journal and with it its withholding effect"
    );
    assert_eq!(row.qualification, Qualification::Withheld);
    assert_eq!(row.reason, "damaged-evidence");
    assert!(
        row.unresolved
            .iter()
            .any(|u| u.starts_with(&format!("damaged-evidence:{journal_id}"))),
        "{:?}",
        row.unresolved
    );
}

#[test]
fn r3_a_corrupt_record_in_a_matching_leftover_is_not_deduplicated_away() {
    let _guard = env_lock();
    let (root, run_dir) = older_qualified_pass("r3-dedup-corrupt");
    let run = store::load_run(&run_dir).expect("load");

    // A real sealing leftover — identity and every record copied from the run it names — plus one
    // extra record whose digest is then broken. Every *surviving* record still matches the run.
    let w = JournalWriter::mirroring(&root, "synthetic-dedup-0001", &run);
    w.observe(READ, "later#corrupt", Outcome::Fail, "tamper target");
    let path = w.dir.join("observations.csv");
    let text = std::fs::read_to_string(&path).expect("read");
    std::fs::write(&path, text.replace("tamper target", "rewritten!!!")).expect("tamper");

    let set = set_of(&root);
    assert!(
        set.journals[0]
            .integrity
            .iter()
            .any(|i| i.starts_with("corrupt-record:observations.csv")),
        "{:?}",
        set.journals[0].integrity
    );
    assert_damaged_leftover_is_retained(&set, "synthetic-dedup-0001");
}

#[test]
fn r3_a_truncated_tail_in_a_matching_leftover_is_not_deduplicated_away() {
    let _guard = env_lock();
    let (root, run_dir) = older_qualified_pass("r3-dedup-truncated");
    let run = store::load_run(&run_dir).expect("load");

    // The other interruption shape: the append that was in flight when the process died. The
    // partial record is dropped, so what remains is once again a perfect subset of the run.
    let w = JournalWriter::mirroring(&root, "synthetic-dedup-0002", &run);
    w.observe(READ, "later#truncated", Outcome::Fail, "interrupted append");
    w.truncate("observations.csv", 45);

    let set = set_of(&root);
    assert!(
        set.journals[0]
            .integrity
            .iter()
            .any(|i| i == "truncated-trailing-record:observations.csv"),
        "{:?}",
        set.journals[0].integrity
    );
    assert!(
        journal::equivalence_findings(&set.journals[0], &run).is_empty(),
        "the case is only meaningful while the surviving records do match the run"
    );
    assert_damaged_leftover_is_retained(&set, "synthetic-dedup-0002");
}

#[test]
fn r3_a_clean_matching_leftover_is_still_deduplicated() {
    let _guard = env_lock();
    let (root, run_dir) = older_qualified_pass("r3-dedup-clean");
    let run = store::load_run(&run_dir).expect("load");
    JournalWriter::mirroring(&root, "synthetic-dedup-0003", &run);

    let set = set_of(&root);
    assert!(
        set.journals.is_empty(),
        "the new guard must not disturb genuine deduplication"
    );
    assert_eq!(
        row_of(&set, READ).qualification,
        Qualification::AutomatedQualified
    );
}

#[test]
fn r3_a_stray_file_in_the_journal_directory_fails_discovery_closed() {
    let _guard = env_lock();
    let (root, _run) = older_qualified_pass("r3-discovery-file");
    std::fs::write(
        journal::journals_dir(&root).join("observations.csv.orig"),
        b"records rescued by hand\n",
    )
    .expect("stray");

    let e = try_set_of(&root).expect_err("an unexplained entry must not be filtered away silently");
    assert!(e.contains("journal-unexpected-entry"), "{e}");
    assert!(e.contains("observations.csv.orig"), "{e}");
    assert!(
        journal::PERMITTED_AUXILIARY_ENTRIES.is_empty(),
        "nothing in this crate writes an auxiliary entry there; adding one is a decision, not a \
         filter"
    );
}

#[test]
fn r3_an_unsupported_link_is_refused_rather_than_skipped() {
    // Fault injection: a symlink cannot be created reliably on every platform (Windows needs
    // developer mode or elevation), so the classification is exercised directly. `read_entry_kind`
    // does not follow links, so a symlink reaches it as exactly this kind.
    let path = journal::journals_dir(Path::new("/synthetic")).join("linked-journal");
    let e = journal::classify_entry(&path, journal::EntryKind::Other)
        .expect_err("a link must not be resolved through, nor silently dropped");
    assert!(e.to_string().contains("journal-unsupported-link"), "{e}");

    // The other two dispositions, for contrast.
    assert_eq!(
        journal::classify_entry(&path, journal::EntryKind::Dir).expect("a journal directory"),
        Some(path.clone())
    );
    assert!(journal::classify_entry(&path, journal::EntryKind::File).is_err());
}

#[test]
fn r3_a_failed_metadata_read_is_an_error_not_an_absence() {
    let dir = sandbox("r3-discovery-metadata");
    // An entry that is listed but cannot be stat'ed — the shape a vanished or inaccessible entry
    // takes. Silently skipping it would drop a journal, and with it any withholding it carried.
    let e = journal::read_entry_kind(&dir.join("never-created"))
        .expect_err("a metadata failure must surface");
    assert!(
        e.to_string().contains("never-created"),
        "the error must name the entry: {e}"
    );

    // And a real directory still classifies normally.
    assert_eq!(
        journal::read_entry_kind(&dir).expect("kind"),
        journal::EntryKind::Dir
    );
}
