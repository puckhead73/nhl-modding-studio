//! C0-EMIT1-R1 — the committed scope definitions, exercised end to end.
//!
//! Every case here copies the repository's **real** `defs/qa/scopes.toml` into a synthetic
//! repository, satisfies every declared glob with stub content, captures a run through the real
//! [`Emitter`] (so the run carries the digest the real definitions produce), and then feeds the
//! sealed run and the *current* digests through the real merge. What is asserted is therefore the
//! behaviour of the definitions as committed — not that a glob string appears in the file.
//!
//! Nothing here writes to the repository or reads a game tree or private fixture. The only
//! repository content read is `defs/qa/scopes.toml`; the guard case additionally *lists* the
//! `*-meta.xml` names under `anchors/` (names only, no content) to hold the definitions to them. `QA_EVIDENCE_ROOT` is process-global, so
//! every case holds [`env_lock`] for its duration.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tdb_qa::evidence::emit::{file_digest, scope_digests, Emitter, ROOT_ENV};
use tdb_qa::evidence::merge::{self, AutomatedAxis, Freshness, Qualification, SummaryRow};
use tdb_qa::evidence::model::{EvidenceSet, Outcome, RunState};
use tdb_qa::evidence::scopes::Scopes;
use tdb_qa::evidence::{store, vocab};

fn env_lock() -> MutexGuard<'static, ()> {
    static ENV: OnceLock<Mutex<()>> = OnceLock::new();
    ENV.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// A synthetic directory, unique per process and per call. Never a repository or game path.
fn sandbox(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir()
        .join("tdb-qa-evidence-freshness")
        .join(format!("{name}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("sandbox");
    dir
}

/// The committed scope definitions, read from the real repository.
fn committed_scopes_text() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("defs/qa/scopes.toml");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

// The three files the emitters consume for `nhl15-ps3`, and where they live in the real tree.
const META: &str = "anchors/ps3/nhl15/databases/nhlng-meta.xml";
const DB: &str = "anchors/ps3/nhl15/databases/nhlng.db";
const SCHEMA_SNAPSHOT: &str = "docs/audits/data/schema/ps3-nhl15.json";

// One representative implementation file per newly covered dependency (C0-EMIT1 / R1).
const ENUMS_CODE: &str = "crates/tdb-enums/src/lib.rs";
const ATTRIBUTES: &str = "crates/studio-domain/src/attributes.rs";
const SURFACES: &str = "crates/studio-domain/src/surfaces.rs";

/// Build a synthetic repository in which every glob of the committed definitions matches at
/// least one file, so every scope resolves to a real digest. Returns the parsed definitions.
fn synthetic_repo(dir: &Path) -> Scopes {
    let text = committed_scopes_text();
    let scopes = Scopes::parse(&text).expect("the committed scopes.toml parses");
    write(dir, "defs/qa/scopes.toml", &text);
    for scope in &scopes.scopes {
        for glob in scope.globs() {
            if let Some(prefix) = glob.strip_suffix("/**") {
                write(dir, &format!("{prefix}/stub.txt"), "stub\n");
            } else if let Some(prefix) = glob.strip_suffix("/*") {
                write(dir, &format!("{prefix}/stub.txt"), "stub\n");
            } else {
                write(dir, glob, "stub\n");
            }
        }
    }
    // The concrete inputs the cases below mutate. They sit under globs the definitions declare;
    // the stubs above keep every other glob satisfied.
    write(dir, META, "<meta version=\"1\"/>\n");
    write(dir, DB, "TDB-synthetic-bytes\n");
    write(dir, SCHEMA_SNAPSHOT, "{}\n");
    write(dir, ENUMS_CODE, "pub struct EnumRegistry;\n");
    write(dir, ATTRIBUTES, "pub fn rating_label() {}\n");
    write(dir, SURFACES, "pub fn probe() {}\n");
    scopes
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

/// The keys each emitter's scope is asserted on — all `required` in the catalog, so a fresh pass
/// reaches `automated-qualified` and nothing but freshness stands between the two outcomes.
const CASES: &[(&str, &str, &str)] = &[
    (
        "surface_matrix",
        "db-read",
        "nhl15-ps3/db-presentation/read",
    ),
    (
        "mutation_matrix",
        "db-edit",
        "nhl15-ps3/db-operations/scalar-edit",
    ),
    (
        "save_matrix",
        "save-containers",
        "nhl15-ps3/save-inner-container/container-repack",
    ),
];

/// Capture one completed passing run per emitter through the real emitter, against `repo`.
fn capture_all(repo: &Path, evroot: &Path) {
    std::env::set_var(ROOT_ENV, evroot);
    for (emitter, scope, key) in CASES {
        let mut ev = Emitter::from_env(emitter, scope, repo);
        // The fixture this synthetic run consumed, attributed by repository-relative path with its
        // captured content hash (C0-EMIT1-R2).
        let digest = file_digest(&repo.join(DB));
        ev.input_digest(DB, digest.clone());
        let k = ev.declare(key);
        ev.observe_hashed(
            &k,
            "tdb_qa::evidence_freshness::synthetic",
            Outcome::Pass,
            &digest.unwrap_or_default(),
            "",
            "ok",
        );
        ev.finish().expect("sealed");
    }
    std::env::remove_var(ROOT_ENV);
}

/// The merge over the sealed runs in `evroot` against the digests `repo` produces **now**.
fn merged(repo: &Path, evroot: &Path) -> merge::Merged {
    let runs = store::load_runs(evroot).expect("runs");
    let fixtures = store::verify_fixture_inputs(repo, &runs).states;
    let set = EvidenceSet {
        journals: Vec::new(),
        runs,
        manual: Vec::new(),
        legacy: Vec::new(),
        implementation: BTreeMap::new(),
        current_scope_digests: scope_digests(repo),
        current_input_digests: fixtures,
        declared_universe: vocab::catalog_universe(),
    };
    merge::merge(&set)
}

fn row<'a>(m: &'a merge::Merged, key: &str) -> &'a SummaryRow {
    m.row(key).unwrap_or_else(|| panic!("no row for {key}"))
}

fn assert_fresh_and_qualified(m: &merge::Merged, key: &str, why: &str) {
    let r = row(m, key);
    assert_eq!(r.automated, AutomatedAxis::Pass, "{key}: {why}");
    assert_eq!(r.fresh, Freshness::Fresh, "{key}: {why}");
    assert_eq!(
        r.qualification,
        Qualification::AutomatedQualified,
        "{key}: {why}"
    );
}

fn assert_not_fresh_and_withheld(m: &merge::Merged, key: &str, why: &str) {
    let r = row(m, key);
    assert_eq!(
        r.automated,
        AutomatedAxis::Pass,
        "{key}: the pass is retained, it just no longer qualifies ({why})"
    );
    assert_ne!(r.fresh, Freshness::Fresh, "{key}: {why}");
    assert_eq!(r.qualification, Qualification::Withheld, "{key}: {why}");
    assert_eq!(r.reason, "stale-evidence", "{key}: {why}");
}

// ------------------------------------------------------------------ the definitions ------

#[test]
fn every_scope_the_emitters_declare_resolves_to_a_real_digest_over_the_synthetic_repo() {
    let repo = sandbox("resolve");
    let scopes = synthetic_repo(&repo);
    let digests = scope_digests(&repo);
    for scope in &scopes.scopes {
        let d = digests
            .get(&(scope.id.clone(), scope.version))
            .unwrap_or_else(|| panic!("{} v{} has no digest", scope.id, scope.version));
        assert!(
            d.starts_with("sha256:"),
            "{} v{}: every declared glob must match something in the synthetic repo, got {d}",
            scope.id,
            scope.version
        );
    }
    // The versions this package fixed, stated so a silent rollback of scopes.toml is a failure.
    for (id, version) in [
        ("shared-core", 3),
        ("db-read", 3),
        ("db-edit", 4),
        ("save-containers", 4),
    ] {
        assert!(
            digests.contains_key(&(id.to_string(), version)),
            "{id} is expected at v{version} (C0-EMIT1-R1)"
        );
    }
}

/// Every `*-meta.xml` that exists under the real `anchors/` tree, repo-relative with `/`.
fn metadata_files_present(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with("-meta.xml"))
            {
                let rel = path
                    .strip_prefix(root)
                    .expect("under root")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(&root.join("anchors"), root, &mut out);
    out.sort();
    out
}

#[test]
fn every_metadata_file_under_anchors_is_declared_by_shared_core() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let present = metadata_files_present(&root);
    assert!(
        !present.is_empty(),
        "the committed anchor tree carries per-title metadata; none found under {}",
        root.join("anchors").display()
    );
    let scopes = Scopes::parse(&committed_scopes_text()).expect("parse");
    let core = scopes.get("shared-core").expect("shared-core");
    let declared: std::collections::BTreeSet<&str> = core.globs().into_iter().collect();
    let missing: Vec<&String> = present
        .iter()
        .filter(|p| !declared.contains(p.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "metadata present under anchors/ but not declared by shared-core — a change to it would leave dependent evidence fresh; add it to defs/qa/scopes.toml and bump the version: {missing:?}"
    );
    let stale_entries: Vec<&str> = declared
        .iter()
        .copied()
        .filter(|g| g.starts_with("anchors/") && !present.iter().any(|p| p == g))
        .collect();
    assert!(
        stale_entries.is_empty(),
        "shared-core declares metadata that no longer exists (every scope is `unknown` until the list is corrected): {stale_entries:?}"
    );
}

// ------------------------------------------------------------------ metadata ------

#[test]
fn a_metadata_edit_with_unchanged_db_bytes_makes_every_dependent_pass_stale_and_withheld() {
    let _guard = env_lock();
    let repo = sandbox("meta-edit");
    let evroot = sandbox("meta-edit-evidence");
    synthetic_repo(&repo);
    capture_all(&repo, &evroot);

    let before = merged(&repo, &evroot);
    for (_, _, key) in CASES {
        assert_fresh_and_qualified(&before, key, "captured against the current tree");
    }

    // One byte of the per-title metadata; the database beside it is untouched.
    let db_before = std::fs::read(repo.join(DB)).expect("db");
    append(&repo, META, "<!-- offset changed -->\n");
    assert_eq!(std::fs::read(repo.join(DB)).expect("db"), db_before);

    let after = merged(&repo, &evroot);
    for (_, _, key) in CASES {
        assert_not_fresh_and_withheld(
            &after,
            key,
            "the metadata every codec decision flows from changed",
        );
        assert_eq!(row(&after, key).fresh, Freshness::Stale);
    }
}

#[test]
fn a_removed_metadata_file_cannot_leave_dependent_evidence_fresh() {
    let _guard = env_lock();
    let repo = sandbox("meta-removed");
    let evroot = sandbox("meta-removed-evidence");
    synthetic_repo(&repo);
    capture_all(&repo, &evroot);
    assert_fresh_and_qualified(&merged(&repo, &evroot), CASES[0].2, "baseline");

    // The single consumed file goes away: its literal path matches nothing, so under
    // R-FRESH-UNKNOWN the shared scope — and every scope sharing it — is `unknown`.
    std::fs::remove_file(repo.join(META)).expect("remove");
    let after = merged(&repo, &evroot);
    for (_, _, key) in CASES {
        assert_not_fresh_and_withheld(&after, key, "a required metadata input is missing");
    }
    write(
        &repo,
        META,
        "<meta version=\"1\"/>
",
    );
    assert_fresh_and_qualified(
        &merged(&repo, &evroot),
        CASES[0].2,
        "the file is back, byte-equal",
    );

    // The whole anchor tree goes away: no declared metadata path matches, the shared scope is
    // `unknown`, and — because every emitter scope shares it — so is every digest.
    std::fs::remove_dir_all(repo.join("anchors")).expect("remove anchors");
    let current = scope_digests(&repo);
    for (_, scope, _) in CASES {
        let d = current
            .iter()
            .find(|((id, _), _)| id == scope)
            .map(|(_, d)| d.as_str())
            .unwrap_or("<absent>");
        assert_eq!(
            d, "unknown",
            "{scope}: R-FRESH-UNKNOWN over the anchor tree"
        );
    }
    let gone = merged(&repo, &evroot);
    for (_, _, key) in CASES {
        assert_not_fresh_and_withheld(&gone, key, "no anchor tree at all");
    }
}

#[test]
fn a_run_captured_while_metadata_is_missing_records_unknown_and_never_qualifies() {
    let _guard = env_lock();
    let repo = sandbox("meta-missing-at-capture");
    let evroot = sandbox("meta-missing-at-capture-evidence");
    synthetic_repo(&repo);
    std::fs::remove_dir_all(repo.join("anchors")).expect("remove anchors");
    capture_all(&repo, &evroot);

    let runs = store::load_runs(&evroot).expect("runs");
    assert_eq!(runs.len(), CASES.len());
    for run in &runs {
        assert_eq!(run.state, RunState::Completed);
        assert_eq!(
            run.manifest.implementation_digest, "unknown",
            "{}: a missing input is recorded as the literal unknown, never a substitute",
            run.manifest.emitter
        );
    }
    let m = merged(&repo, &evroot);
    for (_, _, key) in CASES {
        let r = row(&m, key);
        assert_eq!(r.fresh, Freshness::Unknown, "{key}");
        assert_eq!(r.qualification, Qualification::Withheld, "{key}");
        assert_eq!(r.reason, "stale-evidence", "{key}");
    }
    // Restoring the tree later does not retroactively qualify a run that never knew its inputs.
    write(&repo, META, "<meta version=\"1\"/>\n");
    write(&repo, DB, "TDB-synthetic-bytes\n");
    let restored = merged(&repo, &evroot);
    for (_, _, key) in CASES {
        assert_ne!(row(&restored, key).fresh, Freshness::Fresh, "{key}");
    }
}

// ------------------------------------------------------------------ implementation ------

#[test]
fn a_change_to_a_newly_covered_dependency_invalidates_exactly_the_scopes_that_declare_it() {
    let _guard = env_lock();
    let repo = sandbox("impl-edit");
    let evroot = sandbox("impl-edit-evidence");
    synthetic_repo(&repo);
    capture_all(&repo, &evroot);
    let (read_key, edit_key, save_key) = (CASES[0].2, CASES[1].2, CASES[2].2);

    // `tdb-enums` code: declared by `db-edit` v4 and `save-containers` v4, not by `db-read`.
    append(&repo, ENUMS_CODE, "// registry changed\n");
    let m = merged(&repo, &evroot);
    assert_fresh_and_qualified(&m, read_key, "db-read does not execute the enum registry");
    assert_not_fresh_and_withheld(&m, edit_key, "db-edit v4 declares crates/tdb-enums/src/**");
    assert_not_fresh_and_withheld(
        &m,
        save_key,
        "save-containers v4 declares crates/tdb-enums/src/**",
    );

    // `attributes.rs`: declared by all three (db-read v3, db-edit v4, save-containers v4).
    let repo2 = sandbox("impl-edit-2");
    let evroot2 = sandbox("impl-edit-2-evidence");
    synthetic_repo(&repo2);
    capture_all(&repo2, &evroot2);
    append(&repo2, ATTRIBUTES, "// label changed\n");
    let m2 = merged(&repo2, &evroot2);
    for (_, _, key) in CASES {
        assert_not_fresh_and_withheld(&m2, key, "attributes.rs is on every exercised path");
    }

    // `surfaces.rs`: the read probe only — db-edit and save-containers stay fresh.
    let repo3 = sandbox("impl-edit-3");
    let evroot3 = sandbox("impl-edit-3-evidence");
    synthetic_repo(&repo3);
    capture_all(&repo3, &evroot3);
    append(&repo3, SURFACES, "// probe changed\n");
    let m3 = merged(&repo3, &evroot3);
    assert_not_fresh_and_withheld(&m3, read_key, "db-read declares surfaces.rs");
    assert_fresh_and_qualified(&m3, edit_key, "the edit path never probes surfaces");
    assert_fresh_and_qualified(&m3, save_key, "the save path never probes surfaces");
}

// ------------------------------------------------------------------ determinism ------

#[test]
fn restoring_the_original_bytes_restores_the_identical_digest_and_freshness() {
    let _guard = env_lock();
    let repo = sandbox("restore");
    let evroot = sandbox("restore-evidence");
    synthetic_repo(&repo);
    capture_all(&repo, &evroot);

    let original = scope_digests(&repo);
    let meta_bytes = std::fs::read(repo.join(META)).expect("meta");
    let enums_bytes = std::fs::read(repo.join(ENUMS_CODE)).expect("enums");

    append(&repo, META, "x");
    append(&repo, ENUMS_CODE, "x");
    assert_ne!(scope_digests(&repo), original);
    let while_changed = merged(&repo, &evroot);
    for (_, _, key) in CASES {
        assert_not_fresh_and_withheld(&while_changed, key, "changed");
    }

    std::fs::write(repo.join(META), &meta_bytes).expect("restore meta");
    std::fs::write(repo.join(ENUMS_CODE), &enums_bytes).expect("restore enums");
    assert_eq!(
        scope_digests(&repo),
        original,
        "R-CONTENT: identical bytes, identical digests — no mtime or ordering participates"
    );
    let restored = merged(&repo, &evroot);
    for (_, _, key) in CASES {
        assert_fresh_and_qualified(&restored, key, "the inputs are what the run saw again");
    }
    // Same inputs twice → same rows, byte for byte.
    let again = merged(&repo, &evroot);
    for (a, b) in restored.rows.iter().zip(again.rows.iter()) {
        assert_eq!(a.key, b.key);
        assert_eq!(a.qualification, b.qualification);
        assert_eq!(a.fresh, b.fresh);
        assert_eq!(a.reason, b.reason);
    }
}
