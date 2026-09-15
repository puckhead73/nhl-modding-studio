//! C0-EMIT1-R2 — fixture identity verified during discovery, end to end.
//!
//! Every case builds a synthetic repository from the repository's **real** `defs/qa/scopes.toml`
//! (stub content under every declared glob), captures runs through the real [`Emitter`] that
//! attribute the fixtures they consumed, and then discovers the current fixture content with the
//! real [`store::verify_fixture_inputs`] and merges with the real merge. Only the synthetic
//! repository is ever mutated; nothing here reads `anchors/`, `docs/audits/` or a game tree, and
//! the only repository content read is `defs/qa/scopes.toml`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tdb_qa::evidence::emit::{file_digest, scope_digests, Emitter, ROOT_ENV};
use tdb_qa::evidence::merge::{self, AutomatedAxis, Freshness, Qualification, SummaryRow};
use tdb_qa::evidence::model::{EvidenceSet, InputState, Outcome};
use tdb_qa::evidence::scopes::Scopes;
use tdb_qa::evidence::{store, vocab};

fn env_lock() -> MutexGuard<'static, ()> {
    static ENV: OnceLock<Mutex<()>> = OnceLock::new();
    ENV.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn sandbox(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir()
        .join("tdb-qa-evidence-fixtures")
        .join(format!("{name}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("sandbox");
    dir
}

fn committed_scopes_text() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("defs/qa/scopes.toml");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

const META: &str = "anchors/ps3/nhl15/databases/nhlng-meta.xml";
const DB: &str = "anchors/ps3/nhl15/databases/nhlng.db";
const SAVE: &str = "anchors/ps3/nhl15/savefiles/BLUS314360200 - ROSTER/SYS-DATA";
const READ: &str = "nhl15-ps3/db-presentation/read";
const EDIT: &str = "nhl15-ps3/db-operations/scalar-edit";
const REPACK: &str = "nhl15-ps3/save-inner-container/container-repack";
const CASE: &str = "tdb_qa::evidence_fixtures::synthetic";

fn synthetic_repo(dir: &Path) {
    let text = committed_scopes_text();
    let scopes = Scopes::parse(&text).expect("the committed scopes.toml parses");
    write(dir, "defs/qa/scopes.toml", &text);
    for scope in &scopes.scopes {
        for glob in scope.globs() {
            let stub = glob
                .strip_suffix("/**")
                .or_else(|| glob.strip_suffix("/*"))
                .map(|prefix| format!("{prefix}/stub.txt"))
                .unwrap_or_else(|| glob.to_string());
            write(dir, &stub, "stub\n");
        }
    }
    write(dir, META, "<meta version=\"1\"/>\n");
    write(dir, DB, "TDB-synthetic-main-database\n");
    write(dir, SAVE, "PS3RosterFile-synthetic\n");
}

fn write(dir: &Path, rel: &str, content: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, content).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

fn append(dir: &Path, rel: &str, extra: &str) {
    let path = dir.join(rel);
    let mut bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    bytes.extend_from_slice(extra.as_bytes());
    std::fs::write(&path, bytes).expect("write");
}

/// The four emitters' attribution, in miniature: the read and edit runs attribute the database,
/// the save run attributes the save fixture, and every observation carries the fixture's hash.
fn capture_all(repo: &Path, evroot: &Path) {
    std::env::set_var(ROOT_ENV, evroot);
    for (emitter, scope, key, fixture) in [
        ("surface_matrix", "db-read", READ, DB),
        ("mutation_matrix", "db-edit", EDIT, DB),
        ("save_matrix", "save-containers", REPACK, SAVE),
    ] {
        let mut ev = Emitter::from_env(emitter, scope, repo);
        let digest = file_digest(&repo.join(fixture));
        ev.input_digest(fixture, digest.clone());
        let k = ev.declare(key);
        ev.observe_hashed(
            &k,
            CASE,
            Outcome::Pass,
            &digest.unwrap_or_default(),
            "",
            "ok",
        );
        ev.finish().expect("sealed");
    }
    std::env::remove_var(ROOT_ENV);
}

/// Discovery + merge, exactly as the CLI does it: scope digests and fixture states are both
/// resolved against `repo` **now**; the runs are what is sealed under `evroot`.
fn discover(repo: &Path, evroot: &Path) -> (merge::Merged, store::FixtureVerification) {
    let runs = store::load_runs(evroot).expect("runs");
    let fixtures = store::verify_fixture_inputs(repo, &runs);
    let set = EvidenceSet {
        journals: Vec::new(),
        runs,
        manual: Vec::new(),
        legacy: Vec::new(),
        implementation: BTreeMap::new(),
        current_scope_digests: scope_digests(repo),
        current_input_digests: fixtures.states.clone(),
        declared_universe: vocab::catalog_universe(),
    };
    (merge::merge(&set), fixtures)
}

fn row<'a>(m: &'a merge::Merged, key: &str) -> &'a SummaryRow {
    m.row(key).unwrap_or_else(|| panic!("no row for {key}"))
}

fn assert_qualified(m: &merge::Merged, key: &str, why: &str) {
    let r = row(m, key);
    assert_eq!(r.automated, AutomatedAxis::Pass, "{key}: {why}");
    assert_eq!(r.fresh, Freshness::Fresh, "{key}: {why}");
    assert_eq!(
        r.qualification,
        Qualification::AutomatedQualified,
        "{key}: {why}"
    );
    assert!(
        !r.unresolved.iter().any(|u| u.starts_with("fixture-")),
        "{key}: a verified run carries no fixture finding: {:?}",
        r.unresolved
    );
}

fn assert_withheld(m: &merge::Merged, key: &str, fresh: Freshness, token: &str, why: &str) {
    let r = row(m, key);
    assert_eq!(
        r.automated,
        AutomatedAxis::Pass,
        "{key}: the pass is retained ({why})"
    );
    assert_eq!(r.fresh, fresh, "{key}: {why}");
    assert_eq!(r.qualification, Qualification::Withheld, "{key}: {why}");
    assert_eq!(r.reason, "stale-evidence", "{key}: {why}");
    assert!(
        r.unresolved.iter().any(|u| u.starts_with(token)),
        "{key}: expected an `{token}…` finding, got {:?}",
        r.unresolved
    );
}

// ------------------------------------------------------------------ content changes ------

#[test]
fn a_db_only_change_with_metadata_and_code_unchanged_stales_exactly_the_runs_that_read_it() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("db-change"), sandbox("db-change-ev"));
    synthetic_repo(&repo);
    capture_all(&repo, &evroot);
    let (before, _) = discover(&repo, &evroot);
    for key in [READ, EDIT, REPACK] {
        assert_qualified(&before, key, "captured against the current tree");
    }
    let scopes_before = scope_digests(&repo);

    append(&repo, DB, "\n-- one changed record --\n");
    assert_eq!(
        scope_digests(&repo),
        scopes_before,
        "the database is a fixture, not a scope input: metadata and code digests are unchanged"
    );
    let (after, _) = discover(&repo, &evroot);
    assert_withheld(
        &after,
        READ,
        Freshness::Stale,
        "fixture-changed:",
        "the read run attributed the database",
    );
    assert_withheld(
        &after,
        EDIT,
        Freshness::Stale,
        "fixture-changed:",
        "the edit run attributed the database",
    );
    assert_qualified(&after, REPACK, "the save run never read the database");
}

#[test]
fn a_save_only_change_stales_the_save_run_and_nothing_else() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("save-change"), sandbox("save-change-ev"));
    synthetic_repo(&repo);
    capture_all(&repo, &evroot);
    append(&repo, SAVE, "\n-- repacked --\n");
    let (after, _) = discover(&repo, &evroot);
    assert_qualified(&after, READ, "no dependency on the save fixture");
    assert_qualified(&after, EDIT, "no dependency on the save fixture");
    assert_withheld(
        &after,
        REPACK,
        Freshness::Stale,
        "fixture-changed:",
        "the save run attributed this fixture",
    );
}

// ------------------------------------------------------------------ absence and unreadability ------

#[test]
fn a_deleted_or_unreadable_fixture_is_unknown_and_never_fresh() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("db-missing"), sandbox("db-missing-ev"));
    synthetic_repo(&repo);
    capture_all(&repo, &evroot);

    std::fs::remove_file(repo.join(DB)).expect("remove");
    let (gone, states) = discover(&repo, &evroot);
    assert_eq!(states.states.get(DB), Some(&InputState::Missing));
    assert_withheld(
        &gone,
        READ,
        Freshness::Unknown,
        "fixture-missing:",
        "deleted",
    );
    assert_withheld(
        &gone,
        EDIT,
        Freshness::Unknown,
        "fixture-missing:",
        "deleted",
    );
    assert_qualified(&gone, REPACK, "the save fixture is still there");

    // A directory where the file was: something is there, but it is not the fixture.
    std::fs::create_dir_all(repo.join(DB)).expect("dir at the fixture path");
    let (dir, states) = discover(&repo, &evroot);
    assert!(
        matches!(states.states.get(DB), Some(InputState::Unreadable(_))),
        "{:?}",
        states.states.get(DB)
    );
    assert_withheld(
        &dir,
        READ,
        Freshness::Unknown,
        "fixture-unreadable:",
        "a directory",
    );
    std::fs::remove_dir(repo.join(DB)).expect("remove dir");

    // Restored byte-equal: fresh again, and nothing about the run had to change.
    write(&repo, DB, "TDB-synthetic-main-database\n");
    let (back, _) = discover(&repo, &evroot);
    for key in [READ, EDIT, REPACK] {
        assert_qualified(&back, key, "restored");
    }
}

#[test]
fn a_path_that_is_not_repository_relative_is_unreadable_not_fresh() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("escape"), sandbox("escape-ev"));
    synthetic_repo(&repo);
    // A sibling of the repository that exists and is readable — attributing it must still fail.
    let outside = sandbox("escape-outside");
    std::fs::write(outside.join("stray.db"), b"outside").expect("stray");
    std::env::set_var(ROOT_ENV, &evroot);
    let mut ev = Emitter::from_env("synthetic", "db-read", &repo);
    ev.input_digest(
        "../escape-outside/stray.db",
        file_digest(&outside.join("stray.db")),
    );
    ev.input_digest(
        &outside
            .join("stray.db")
            .to_string_lossy()
            .replace('\\', "/"),
        file_digest(&outside.join("stray.db")),
    );
    let k = ev.declare(READ);
    ev.observe(&k, CASE, Outcome::Pass, "ok");
    ev.finish().expect("sealed");
    std::env::remove_var(ROOT_ENV);

    let (m, states) = discover(&repo, &evroot);
    assert!(states
        .states
        .values()
        .all(|s| matches!(s, InputState::Unreadable(_))));
    assert_eq!(
        states.files_hashed, 0,
        "nothing outside the repository is ever read"
    );
    assert_withheld(
        &m,
        READ,
        Freshness::Unknown,
        "fixture-unreadable:",
        "an escaping or absolute attribution",
    );
}

// ------------------------------------------------------------------ legacy attribution ------

#[test]
fn an_older_completed_pass_without_attribution_is_cited_but_cannot_qualify() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("legacy"), sandbox("legacy-ev"));
    synthetic_repo(&repo);
    std::env::set_var(ROOT_ENV, &evroot);
    // What the emitters wrote before C0-EMIT1-R2: a completed pass, real scope digest, fixture
    // hash on the observation — and no `input_digests` at all.
    let mut ev = Emitter::from_env("surface_matrix", "db-read", &repo);
    let k = ev.declare(READ);
    ev.observe_hashed(
        &k,
        CASE,
        Outcome::Pass,
        &file_digest(&repo.join(DB)).unwrap_or_default(),
        "",
        "an older run",
    );
    let dir = ev.finish().expect("sealed");
    std::env::remove_var(ROOT_ENV);

    let (m, states) = discover(&repo, &evroot);
    assert!(
        states.states.is_empty(),
        "nothing to verify, nothing hashed"
    );
    let r = row(&m, READ);
    assert!(
        r.citations
            .iter()
            .any(|c| c.kind == "auto-run" && c.selected),
        "the run is still discovered, cited and selected: {:?}",
        r.citations
    );
    assert_eq!(r.automated, AutomatedAxis::Pass);
    assert_eq!(r.fresh, Freshness::Unknown);
    assert_eq!(r.qualification, Qualification::Withheld);
    assert_eq!(r.reason, "stale-evidence");
    assert!(r
        .unresolved
        .iter()
        .any(|u| u.starts_with("fixture-attribution-missing:")));
    assert!(
        dir.join("manifest.json").is_file(),
        "the run itself is untouched"
    );
}

#[test]
fn an_observation_whose_fixture_is_not_among_the_attributed_inputs_cannot_be_fresh() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("partial"), sandbox("partial-ev"));
    synthetic_repo(&repo);
    std::env::set_var(ROOT_ENV, &evroot);
    let mut ev = Emitter::from_env("save_matrix", "save-containers", &repo);
    ev.input_digest(DB, file_digest(&repo.join(DB)));
    let k = ev.declare(REPACK);
    // The observation says it ran over the save fixture, but the run never attributed it.
    ev.observe_hashed(
        &k,
        CASE,
        Outcome::Pass,
        &file_digest(&repo.join(SAVE)).unwrap_or_default(),
        "",
        "partially attributed",
    );
    ev.finish().expect("sealed");
    std::env::remove_var(ROOT_ENV);

    let (m, _) = discover(&repo, &evroot);
    assert_withheld(
        &m,
        REPACK,
        Freshness::Unknown,
        "fixture-unattributed-observations:",
        "the observation's fixture is not verifiable through the run's inputs",
    );
}

// ------------------------------------------------------------------ failures survive ------

#[test]
fn unknown_fixture_freshness_does_not_bypass_a_recorded_failure() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("failure"), sandbox("failure-ev"));
    synthetic_repo(&repo);
    std::env::set_var(ROOT_ENV, &evroot);
    let mut ev = Emitter::from_env("mutation_matrix", "db-edit", &repo);
    ev.input_digest(DB, file_digest(&repo.join(DB)));
    let k = ev.declare(EDIT);
    ev.observe(
        &k,
        CASE,
        Outcome::Fail,
        "assertion failed: the deliberate failure",
    );
    ev.finish().expect("sealed");
    std::env::remove_var(ROOT_ENV);
    std::fs::remove_file(repo.join(DB)).expect("remove");

    let (m, _) = discover(&repo, &evroot);
    let r = row(&m, EDIT);
    assert_eq!(r.fresh, Freshness::Unknown, "the fixture is gone");
    assert_eq!(
        r.reason, "unreviewed-failure",
        "§4.3 row 4a stays above the freshness row: {:?}",
        r
    );
    assert_eq!(r.qualification, Qualification::Withheld);
    assert!(r
        .unresolved
        .iter()
        .any(|u| u.starts_with("uncleared-executed-failure:")));
}

// ------------------------------------------------------------------ determinism and cost ------

#[test]
fn unchanged_inputs_and_restored_bytes_reproduce_identical_results() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("restore"), sandbox("restore-ev"));
    synthetic_repo(&repo);
    capture_all(&repo, &evroot);
    let (first, states_first) = discover(&repo, &evroot);
    let (again, states_again) = discover(&repo, &evroot);
    assert_eq!(
        states_first, states_again,
        "same bytes, same states, same counts"
    );
    assert_eq!(first.rows, again.rows, "same inputs, same rows");

    let db = std::fs::read(repo.join(DB)).expect("db");
    append(&repo, DB, "x");
    let (changed, _) = discover(&repo, &evroot);
    assert_ne!(changed.rows, first.rows);
    std::fs::write(repo.join(DB), &db).expect("restore");
    let (restored, states_restored) = discover(&repo, &evroot);
    assert_eq!(states_restored.states, states_first.states);
    assert_eq!(
        restored.rows, first.rows,
        "restored bytes reproduce the original rows"
    );
}

#[test]
fn a_file_attributed_by_several_runs_is_hashed_once_per_invocation() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("dedup"), sandbox("dedup-ev"));
    synthetic_repo(&repo);
    capture_all(&repo, &evroot); // two runs attribute DB, one attributes SAVE
    let runs = store::load_runs(&evroot).expect("runs");
    let attributed: usize = runs.iter().map(|r| r.manifest.input_digests.len()).sum();
    assert_eq!(attributed, 3, "three attributions across the three runs");
    let v = store::verify_fixture_inputs(&repo, &runs);
    assert_eq!(v.states.len(), 2, "two distinct paths");
    assert_eq!(
        v.files_hashed, 2,
        "each distinct file is hashed exactly once"
    );
    let expected: u64 = [DB, SAVE]
        .iter()
        .map(|p| std::fs::metadata(repo.join(p)).expect("len").len())
        .sum();
    assert_eq!(
        v.bytes_hashed, expected,
        "only the referenced bytes were read"
    );
    assert!(matches!(v.states.get(DB), Some(InputState::Present(_))));
    assert!(matches!(v.states.get(SAVE), Some(InputState::Present(_))));

    // Runs with no attribution at all contribute nothing to the pass — nothing is walked.
    let none = store::verify_fixture_inputs(&repo, &[]);
    assert_eq!(none.files_hashed, 0);
    assert!(none.states.is_empty());
}

#[test]
fn the_provenance_sidecar_records_what_was_compared() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("provenance"), sandbox("provenance-ev"));
    synthetic_repo(&repo);
    capture_all(&repo, &evroot);
    let runs = store::load_runs(&evroot).expect("runs");
    let fixtures = store::verify_fixture_inputs(&repo, &runs);
    let mut merge_input_digests = BTreeMap::new();
    for (path, state) in &fixtures.states {
        merge_input_digests.insert(format!("fixture:{path}"), state.render());
    }
    let mut fixture_verification = BTreeMap::new();
    for run in &runs {
        fixture_verification.insert(
            run.run_id().to_string(),
            merge::run_fixture_verdict(run, &fixtures.states).render(),
        );
    }
    let (merged, _) = discover(&repo, &evroot);
    let out = tdb_qa::evidence::publish::build(
        &merged,
        &tdb_qa::evidence::publish::BuildInputs {
            tool_version: "synthetic".into(),
            merge_input_digests,
            fixture_verification,
            scope_definitions_digest: "sha256:synthetic".into(),
            divergence: Vec::new(),
            unmapped: Vec::new(),
        },
    );
    let sidecar = out
        .files
        .get("qa-evidence-matrix.csv.provenance.json")
        .expect("sidecar");
    let parsed: tdb_qa::evidence::publish::Provenance =
        serde_json::from_str(sidecar).expect("schema 2 parses");
    assert_eq!(parsed.schema, "qa-evidence-provenance/2");
    assert!(parsed
        .merge_input_digests
        .get(&format!("fixture:{DB}"))
        .is_some_and(|v| v.starts_with("sha256:")));
    assert!(parsed
        .merge_input_digests
        .get(&format!("fixture:{SAVE}"))
        .is_some_and(|v| v.starts_with("sha256:")));
    assert_eq!(parsed.fixture_verification.len(), 3);
    assert!(parsed
        .fixture_verification
        .values()
        .all(|v| v == "verified"));

    // A schema-1 sidecar — no `fixture_verification` — still parses.
    let legacy = r#"{"schema":"qa-evidence-provenance/1","tool_version":"x","merge_input_digests":{},"scope_definitions_digest":"unknown","output_digest":"sha256:00"}"#;
    let parsed: tdb_qa::evidence::publish::Provenance =
        serde_json::from_str(legacy).expect("schema 1 parses");
    assert!(parsed.fixture_verification.is_empty());
}

// ------------------------------------------------------------------ R3: identity on executed observations ------

/// Seal one run attributing `DB` and recording `observations` as `(key, outcome, hashed)`.
fn seal_run(
    repo: &Path,
    evroot: &Path,
    emitter: &str,
    scope: &str,
    observations: &[(&str, Outcome, bool)],
) {
    std::env::set_var(ROOT_ENV, evroot);
    let mut ev = Emitter::from_env(emitter, scope, repo);
    let digest = file_digest(&repo.join(DB)).unwrap_or_default();
    ev.input_digest(DB, Some(digest.clone()));
    let keys: Vec<_> = observations.iter().map(|(k, _, _)| ev.declare(k)).collect();
    for ((_, outcome, hashed), key) in observations.iter().zip(&keys) {
        if *hashed {
            ev.observe_hashed(key, CASE, *outcome, &digest, "", "hashed");
        } else {
            ev.observe(key, CASE, *outcome, "no identity recorded");
        }
    }
    ev.finish().expect("sealed");
    std::env::remove_var(ROOT_ENV);
}

#[test]
fn a_completed_pass_with_an_empty_hash_is_withheld_even_though_the_run_attributes_a_valid_input() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("empty-hash"), sandbox("empty-hash-ev"));
    synthetic_repo(&repo);
    seal_run(
        &repo,
        &evroot,
        "surface_matrix",
        "db-read",
        &[(READ, Outcome::Pass, false)],
    );
    let (m, states) = discover(&repo, &evroot);
    assert!(
        matches!(states.states.get(DB), Some(InputState::Present(_))),
        "the attributed input itself is valid and present"
    );
    assert_withheld(
        &m,
        READ,
        Freshness::Unknown,
        "fixture-identity-missing:",
        "an executed pass with no fixture identity cannot ride on another valid input",
    );
}

#[test]
fn mixed_hashed_and_unhashed_executed_observations_make_the_whole_run_unknown() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("mixed"), sandbox("mixed-ev"));
    synthetic_repo(&repo);
    seal_run(
        &repo,
        &evroot,
        "mutation_matrix",
        "db-edit",
        &[
            (EDIT, Outcome::Pass, true),
            (
                "nhl15-ps3/db-operations/roster-create",
                Outcome::RefusedClean,
                false,
            ),
        ],
    );
    let (m, _) = discover(&repo, &evroot);
    // Run-level: the hashed pass is withheld too, because the run's attribution is incomplete.
    assert_withheld(
        &m,
        EDIT,
        Freshness::Unknown,
        "fixture-identity-missing:",
        "one unhashed executed observation makes the run's fixture identity unknown",
    );
    let refused = row(&m, "nhl15-ps3/db-operations/roster-create");
    assert!(
        matches!(
            refused.automated,
            AutomatedAxis::Pass | AutomatedAxis::Mixed
        ),
        "refused-clean is an executed outcome, not an absence: {:?}",
        refused.automated
    );
    assert_eq!(refused.fresh, Freshness::Unknown);
}

#[test]
fn an_unhashed_failure_still_blocks_qualification() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("unhashed-fail"), sandbox("unhashed-fail-ev"));
    synthetic_repo(&repo);
    seal_run(
        &repo,
        &evroot,
        "mutation_matrix",
        "db-edit",
        &[(EDIT, Outcome::Fail, false)],
    );
    let (m, _) = discover(&repo, &evroot);
    let r = row(&m, EDIT);
    assert_eq!(r.fresh, Freshness::Unknown);
    assert_eq!(r.qualification, Qualification::Withheld);
    assert_eq!(
        r.reason, "unreviewed-failure",
        "the missing identity does not erase or outrank the recorded failure"
    );
    assert!(r
        .unresolved
        .iter()
        .any(|u| u.starts_with("uncleared-executed-failure:")));
    assert!(r
        .unresolved
        .iter()
        .any(|u| u.starts_with("fixture-identity-missing:")));
}

#[test]
fn skipped_and_not_attempted_rows_stay_valid_without_an_invented_hash() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("skips"), sandbox("skips-ev"));
    synthetic_repo(&repo);
    // The save emitter's shape: a hashed pass on a variant key, a deliberate `not-attempted` on
    // the variant-less parent, and a `skipped-missing-fixture` on a unit with no fixture — the
    // last two with no fixture identity, exactly as the emitters record them.
    seal_run(
        &repo,
        &evroot,
        "save_matrix",
        "save-containers",
        &[
            (
                "nhl15-ps3/save-inner-container/container-repack/v4",
                Outcome::Pass,
                true,
            ),
            (REPACK, Outcome::NotAttempted, false),
            (
                "legacyrecomp-recomp/save-inner-container/container-repack",
                Outcome::SkippedMissingFixture,
                false,
            ),
        ],
    );
    let (m, states) = discover(&repo, &evroot);
    let runs = store::load_runs(&evroot).expect("runs");
    let verdict = merge::run_fixture_verdict(&runs[0], &states.states);
    assert_eq!(
        verdict.freshness,
        Freshness::Fresh,
        "{:?}",
        verdict.findings
    );
    assert!(verdict.findings.is_empty());

    let variant = row(&m, "nhl15-ps3/save-inner-container/container-repack/v4");
    assert_eq!(variant.automated, AutomatedAxis::Pass);
    assert_eq!(variant.fresh, Freshness::Fresh);
    assert_eq!(
        variant.reason, "unknown-applicability",
        "the variant rule, unchanged"
    );
    let parent = row(&m, REPACK);
    assert_eq!(parent.automated, AutomatedAxis::DeclaredNotExecuted);
    assert_ne!(parent.qualification, Qualification::AutomatedQualified);
    let skipped = row(
        &m,
        "legacyrecomp-recomp/save-inner-container/container-repack",
    );
    assert_eq!(skipped.automated, AutomatedAxis::SkippedMissingFixture);
    assert!(
        !skipped.unresolved.iter().any(|u| u.starts_with("fixture-")),
        "a skip needs no identity: {:?}",
        skipped.unresolved
    );
}

// ------------------------------------------------------------------ R3: path guarantees ------

/// A real directory link — a junction on Windows (no elevation needed), a symlink elsewhere.
fn make_dir_link(link: &Path, target: &Path) {
    #[cfg(windows)]
    {
        // `mklink` reads a `/` inside a path as a switch, so spell both with backslashes.
        let spell = |p: &Path| p.to_string_lossy().replace('/', "\\");
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(spell(link))
            .arg(spell(target))
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

/// A file symlink. On Windows this needs Developer Mode or elevation, which is why the case
/// using it is `#[ignore]`d by default; once selected, failure to create the link is a failure.
fn file_link(link: &Path, target: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link)
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
}

#[test]
fn a_junction_ancestor_redirecting_inside_the_repository_is_refused() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("junction"), sandbox("junction-ev"));
    synthetic_repo(&repo);
    capture_all(&repo, &evroot);
    let (before, _) = discover(&repo, &evroot);
    assert_qualified(&before, READ, "plain directories all the way down");

    // Replace the `databases` directory with a junction to another directory *inside* the
    // repository that holds byte-identical content. Resolution lands under the repository and
    // the bytes match, so a final-entry-only check would have verified it.
    let databases = repo.join("anchors/ps3/nhl15/databases");
    let elsewhere = repo.join("anchors/ps3/nhl15/elsewhere");
    std::fs::rename(&databases, &elsewhere).expect("move aside");
    make_dir_link(&databases, &elsewhere);
    assert_eq!(
        std::fs::read(repo.join(DB)).expect("readable through the link"),
        std::fs::read(elsewhere.join("nhlng.db")).expect("target"),
        "the content is identical through the link — which is exactly why it must not count"
    );

    let (after, states) = discover(&repo, &evroot);
    match states.states.get(DB) {
        Some(InputState::Unreadable(reason)) => assert!(
            reason.contains("reparse"),
            "the refusal names the reparse point: {reason}"
        ),
        other => panic!("a junction ancestor must be refused, got {other:?}"),
    }
    assert_eq!(
        states.files_hashed, 1,
        "only the save fixture was read; nothing through the junction"
    );
    assert_withheld(
        &after,
        READ,
        Freshness::Unknown,
        "fixture-unreadable:",
        "an ancestor that is a junction",
    );
    assert_withheld(
        &after,
        EDIT,
        Freshness::Unknown,
        "fixture-unreadable:",
        "same",
    );
    assert_qualified(&after, REPACK, "the save fixture's path has no link on it");

    // Cleanup: remove the junction itself, never its target.
    #[cfg(windows)]
    std::fs::remove_dir(&databases).expect("remove junction");
    #[cfg(unix)]
    std::fs::remove_file(&databases).expect("remove symlink");
    assert!(elsewhere.join("nhlng.db").is_file(), "the target survived");

    // The same refusal at the **final** entry: a directory link standing where the fixture file
    // is expected is a reparse point first and a wrong type second — refused before either the
    // type or the bytes are consulted. (A *file* symlink at the final entry is the ignored-by-default
    // case `a_file_link_at_the_fixture_itself_is_refused`, which needs symlink privilege.)
    std::fs::rename(&elsewhere, &databases).expect("restore the directory");
    let real = repo.join("anchors/ps3/nhl15/databases/nhlng-real");
    std::fs::create_dir_all(&real).expect("target dir");
    std::fs::rename(repo.join(DB), real.join("nhlng.db")).expect("move the file");
    make_dir_link(&repo.join(DB), &real);
    let (final_entry, states) = discover(&repo, &evroot);
    match states.states.get(DB) {
        Some(InputState::Unreadable(reason)) => assert!(reason.contains("reparse"), "{reason}"),
        other => panic!("a link at the fixture itself must be refused, got {other:?}"),
    }
    assert_withheld(
        &final_entry,
        READ,
        Freshness::Unknown,
        "fixture-unreadable:",
        "a junction at the fixture's own path",
    );
    #[cfg(windows)]
    std::fs::remove_dir(repo.join(DB)).expect("remove junction");
    #[cfg(unix)]
    std::fs::remove_file(repo.join(DB)).expect("remove symlink");
}

/// Platform prerequisite: creating a *file* symlink on Windows needs Developer Mode or elevation,
/// so this case is ignored by default and never reports a pass it did not execute. Select it with
/// `--ignored` where the privilege exists; there, an uncreatable link fails the case outright.
/// The junction cases above cover the reparse-point refusal on an unprivileged Windows account.
#[test]
#[ignore = "needs file-symlink privilege (Windows: Developer Mode or elevation); run with --ignored"]
fn a_file_link_at_the_fixture_itself_is_refused() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("file-link"), sandbox("file-link-ev"));
    synthetic_repo(&repo);
    capture_all(&repo, &evroot);
    let real = repo.join("anchors/ps3/nhl15/databases/nhlng-real.db");
    std::fs::rename(repo.join(DB), &real).expect("move aside");
    if let Err(err) = file_link(&repo.join(DB), &real) {
        std::fs::rename(&real, repo.join(DB)).expect("restore");
        panic!(
            "this case was selected but the file symlink could not be created \
             (missing platform privilege?): {err}"
        );
    }
    let (m, states) = discover(&repo, &evroot);
    match states.states.get(DB) {
        Some(InputState::Unreadable(reason)) => assert!(reason.contains("reparse"), "{reason}"),
        other => panic!("a file link at the fixture must be refused, got {other:?}"),
    }
    assert_withheld(
        &m,
        READ,
        Freshness::Unknown,
        "fixture-unreadable:",
        "a file symlink",
    );
}

#[test]
fn an_ancestor_that_is_a_file_and_an_unresolvable_repository_root_are_refused() {
    let _guard = env_lock();
    let (repo, evroot) = (sandbox("file-ancestor"), sandbox("file-ancestor-ev"));
    synthetic_repo(&repo);
    capture_all(&repo, &evroot);

    // A regular file where the `databases` directory should be.
    let databases = repo.join("anchors/ps3/nhl15/databases");
    let aside = repo.join("anchors/ps3/nhl15/databases-aside");
    std::fs::rename(&databases, &aside).expect("move aside");
    std::fs::write(&databases, b"not a directory").expect("file at the ancestor");
    let (m, states) = discover(&repo, &evroot);
    match states.states.get(DB) {
        Some(InputState::Unreadable(reason)) => {
            assert!(reason.contains("not a directory"), "{reason}")
        }
        other => panic!("got {other:?}"),
    }
    assert_withheld(
        &m,
        READ,
        Freshness::Unknown,
        "fixture-unreadable:",
        "file ancestor",
    );
    std::fs::remove_file(&databases).expect("remove");
    std::fs::rename(&aside, &databases).expect("restore");

    // A repository root that does not exist cannot be canonicalized: nothing is verified, and
    // nothing is read — fail closed rather than approximate.
    let runs = store::load_runs(&evroot).expect("runs");
    let missing_root = sandbox("never-a-repo").join("gone");
    let v = store::verify_fixture_inputs(&missing_root, &runs);
    assert_eq!(v.files_hashed, 0);
    assert!(!v.states.is_empty());
    assert!(v.states.values().all(|s| matches!(
        s,
        InputState::Unreadable(r) if r.contains("repository root cannot be resolved")
    )));
}
