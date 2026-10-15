//! C0-S2-INT1-R5 — file-read semantics for optional evidence files.
//!
//! Every case runs over a **synthetic** evidence root under the system temp tree. Nothing reads or
//! writes `docs/audits/`, `anchors/`, a game tree or a private fixture.
//!
//! The subject is the one remaining shape of silent evidence loss: a *file* read whose failure was
//! converted into empty data. `unmapped.csv` and `facets.csv` are genuinely optional — a run
//! written before they existed simply lacks them — and that backward compatibility was implemented
//! as "any read failure means no records". This suite fixes the boundary in both directions: the
//! legitimate absence must keep working, and every other outcome must reach `check`/`build` as a
//! non-success without publishing anything.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

use tdb_qa::evidence::emit::{Emitter, ROOT_ENV};
use tdb_qa::evidence::model::{Outcome, RunState};
use tdb_qa::evidence::{paths, store};

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
        .join("tdb-qa-evidence-file-reads")
        .join(format!("{name}-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("sandbox");
    dir
}

const READ: &str = "nhl15-ps3/db-presentation/read";
const CASE: &str = "tdb_qa::evidence_file_reads::synthetic";

/// A run carrying an observation, one faceted observation and one unmapped record — so both
/// optional files exist and have real content.
fn full_run(root: &Path) -> PathBuf {
    std::env::set_var(ROOT_ENV, root);
    let mut ev = Emitter::from_env("synthetic", "db-read", &repo_root());
    let fixture = attribute_scopes_file(&mut ev);
    let key = ev.declare(READ);
    ev.observe_faceted(
        &key,
        &format!("{CASE}#faceted"),
        Outcome::Pass,
        &fixture,
        "",
        "a faceted observation",
        &[("codec", "raw"), ("container-version", "v4")],
    );
    ev.observe_unmapped(
        "nhl15-ps3",
        "an-emitter-operation",
        &format!("{CASE}#unmapped"),
        Outcome::Pass,
        "",
        "operation-not-in-closed-db-operations-vocabulary",
        "an unmapped execution",
    );
    let dir = ev.finish().expect("sealed");
    std::env::remove_var(ROOT_ENV);
    dir
}

/// SHA-256 of every file in a run directory, for proving a failed read changed nothing.
fn run_bytes(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).expect("read_dir") {
        let entry = entry.expect("entry");
        if entry.path().is_file() {
            out.push((
                entry.file_name().to_string_lossy().into_owned(),
                store::sha256_prefixed(&fs::read(entry.path()).expect("read")),
            ));
        }
    }
    out.sort();
    out
}

/// Run the real `qa-evidence` binary. Returns (exit code, stderr).
fn qa_evidence(args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_qa-evidence"))
        .args(args)
        .arg("--repo")
        .arg(repo_root())
        .output()
        .expect("run qa-evidence");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Both commands must refuse, and `build` must leave its output directory empty: a damaged input
/// never becomes a published matrix.
fn assert_check_and_build_refuse(root: &Path, expect: &str) {
    let out = sandbox("refused-out");
    let data = sandbox("refused-data");

    let (code, stderr) = qa_evidence(&[
        "check",
        "--root",
        &root.to_string_lossy(),
        "--data",
        &data.to_string_lossy(),
    ]);
    assert_ne!(code, 0, "check must not succeed: {stderr}");
    assert_eq!(code, 3, "a malformed active input is exit 3: {stderr}");
    assert!(stderr.contains(expect), "{stderr}");

    let (code, stderr) = qa_evidence(&[
        "build",
        "--root",
        &root.to_string_lossy(),
        "--out",
        &out.to_string_lossy(),
    ]);
    assert_ne!(code, 0, "build must not succeed: {stderr}");
    assert!(stderr.contains(expect), "{stderr}");
    assert_eq!(
        fs::read_dir(&out).expect("out").count(),
        0,
        "a refused build publishes nothing"
    );
}

/// The whole point of the fallback: a run written before these files existed still loads.
fn strip_optional_files(dir: &Path) {
    for name in ["unmapped.csv", "facets.csv"] {
        let path = dir.join(name);
        if path.exists() {
            fs::remove_file(&path).expect("remove");
        }
    }
}

// ============================================ the optional files ==============================

#[test]
fn r5_an_older_run_without_the_optional_files_still_loads() {
    let _guard = env_lock();
    let root = sandbox("r5-absent");
    let dir = full_run(&root);
    strip_optional_files(&dir);

    let run = store::load_run(&dir).expect("an older run is not malformed");
    assert!(run.unmapped.is_empty() && run.facets.is_empty());
    assert_eq!(run.state, RunState::Completed);
    assert_eq!(run.observations.len(), 1, "the run itself is intact");

    // And the whole tree loads through the production path.
    assert_eq!(store::load_runs(&root).expect("load").len(), 1);
}

#[test]
fn r5_valid_optional_contents_are_read_back() {
    let _guard = env_lock();
    let root = sandbox("r5-valid");
    let dir = full_run(&root);

    let run = store::load_run(&dir).expect("load");
    assert_eq!(run.unmapped.len(), 1);
    assert_eq!(run.unmapped[0].emitter_operation, "an-emitter-operation");
    assert_eq!(run.facets.len(), 2, "both facets of the observation");
    assert!(run.facets.iter().any(|f| f.name == "container-version"));
}

#[test]
fn r5_invalid_utf8_in_an_optional_file_is_an_error_not_an_empty_list() {
    let _guard = env_lock();
    let root = sandbox("r5-utf8");
    let dir = full_run(&root);
    let path = dir.join("unmapped.csv");

    // A valid header, then bytes that are not UTF-8 — the shape a truncated or mangled write
    // leaves. The old code reported "this run made no unmapped observations".
    let mut bytes = fs::read(&path).expect("read");
    bytes.extend_from_slice(&[0xff, 0xfe, 0x9c, b'\n']);
    fs::write(&path, &bytes).expect("write");
    let before = run_bytes(&dir);

    let e = store::load_run(&dir).expect_err("an unreadable file is not an empty one");
    assert!(e.to_string().contains(store::RUN_UNREADABLE), "{e}");
    assert_check_and_build_refuse(&root, store::RUN_UNREADABLE);
    assert_eq!(run_bytes(&dir), before, "the original bytes are retained");
}

#[test]
fn r5_a_directory_at_an_optional_file_path_is_an_error() {
    let _guard = env_lock();
    let root = sandbox("r5-wrong-type");
    let dir = full_run(&root);
    let path = dir.join("facets.csv");
    fs::remove_file(&path).expect("remove");
    fs::create_dir(&path).expect("mkdir");

    let e = store::load_run(&dir).expect_err("a directory is not an absent file");
    assert!(e.to_string().contains(store::RUN_WRONG_TYPE), "{e}");
    assert_check_and_build_refuse(&root, store::RUN_WRONG_TYPE);
}

#[test]
fn r5_a_link_at_an_optional_file_path_is_refused() {
    let _guard = env_lock();
    let root = sandbox("r5-link");
    let elsewhere = sandbox("r5-link-target");
    let dir = full_run(&root);
    let path = dir.join("unmapped.csv");
    fs::remove_file(&path).expect("remove");
    make_dir_link(&path, &elsewhere);

    // No link is followed to decide what stands at an evidence file's name: it could point outside
    // the run, and it could be re-pointed between the check and the read.
    let e = store::load_run(&dir).expect_err("a link is refused");
    assert!(e.to_string().contains("discovery-reparse-point"), "{e}");
    assert_check_and_build_refuse(&root, "discovery-reparse-point");
}

#[test]
fn r5_an_injected_read_failure_propagates_with_its_path() {
    let _guard = env_lock();
    let root = sandbox("r5-injected");
    let dir = full_run(&root);
    let path = dir.join("unmapped.csv");

    // Deterministic injection for the branch a permission bit cannot reach reliably: the file is
    // held open with **no sharing** on Windows, and made unreadable on Unix. Either way the entry
    // exists, classifies as a regular file, and then fails to read — the exact sequence the old
    // `let Ok(text) = … else` swallowed.
    let before = run_bytes(&dir);
    let _lock = deny_reads(&path);

    let e = store::load_run(&dir).expect_err("a read failure is not an empty file");
    let text = e.to_string();
    assert!(
        text.contains("unmapped.csv"),
        "the error names the path: {text}"
    );
    assert_check_and_build_refuse(&root, "unmapped.csv");
    drop(_lock);
    assert_eq!(run_bytes(&dir), before, "nothing was rewritten");

    // With the injection released the same run loads normally again — the failure was the read,
    // not the file.
    assert_eq!(store::load_run(&dir).expect("load").unmapped.len(), 1);
}

#[test]
fn r5_only_a_genuine_absence_yields_empty_data() {
    let dir = sandbox("r5-absence-boundary");

    // A directory at the name is not an absence...
    let e = paths::read_optional_file(&dir, store::RUN_WRONG_TYPE, store::RUN_UNREADABLE)
        .expect_err("a directory at the name");
    assert!(e.to_string().contains(store::RUN_WRONG_TYPE), "{e}");

    // ...and a name that truly does not exist is the only case allowed to yield nothing.
    assert!(paths::read_optional_file(
        &dir.join("missing.csv"),
        store::RUN_WRONG_TYPE,
        store::RUN_UNREADABLE
    )
    .expect("absent is not an error")
    .is_none());
}

// ============================================ state.json ======================================

#[test]
fn r5_an_absent_state_marker_and_an_unreadable_one_are_both_interrupted_but_distinguishable() {
    let _guard = env_lock();

    // Absent — the documented R-STATE case, and the only one that means "the emitter died before
    // sealing". Unchanged by R5.
    let root = sandbox("r5-state-absent");
    let dir = full_run(&root);
    fs::remove_file(dir.join("state.json")).expect("remove");
    let run = store::load_run(&dir).expect("an interrupted run still loads");
    assert_eq!(run.state, RunState::Interrupted);
    assert_eq!(run.failure.as_deref(), Some("no state.json"));

    // Present but unreadable — the same conservative classification, because it can only ever
    // *remove* qualification, but no longer reported as an absent marker.
    let root = sandbox("r5-state-unreadable");
    let dir = full_run(&root);
    let path = dir.join("state.json");
    fs::remove_file(&path).expect("remove");
    fs::create_dir(&path).expect("mkdir");
    let run = store::load_run(&dir).expect("still loads, still interrupted");
    assert_eq!(run.state, RunState::Interrupted);
    let failure = run.failure.clone().unwrap_or_default();
    assert!(
        failure.contains("present but unreadable"),
        "absence and unreadability are different findings: {failure}"
    );
    assert!(failure.contains(store::RUN_WRONG_TYPE), "{failure}");
}

#[test]
fn r5_an_unreadable_state_marker_cannot_erase_a_failure_or_qualify_anything() {
    let _guard = env_lock();
    let root = sandbox("r5-state-no-erasure");
    std::env::set_var(ROOT_ENV, &root);
    let mut ev = Emitter::from_env("synthetic", "db-read", &repo_root());
    let fixture = attribute_scopes_file(&mut ev);
    let key = ev.declare(READ);
    ev.observe_hashed(
        &key,
        &format!("{CASE}#fail"),
        Outcome::Fail,
        &fixture,
        "",
        "an executed failure",
    );
    let dir = ev.finish().expect("sealed");
    std::env::remove_var(ROOT_ENV);

    // Damage the terminal marker so the run is downgraded to `interrupted`.
    let path = dir.join("state.json");
    fs::remove_file(&path).expect("remove");
    fs::create_dir(&path).expect("mkdir");

    let run = store::load_run(&dir).expect("load");
    assert_eq!(run.state, RunState::Interrupted);
    // The observation — the failure itself — survives the downgrade intact.
    assert_eq!(run.observations.len(), 1);
    assert_eq!(run.observations[0].outcome, Outcome::Fail);

    // And the downgrade cannot grant anything: an interrupted run is never selectable, so the key
    // is not qualified, and the executed failure still shows up as unreviewed (row 4a).
    let runs = store::load_runs(&root).expect("runs");
    let fixtures = tdb_qa::evidence::store::verify_fixture_inputs(&repo_root(), &runs).states;
    let set = tdb_qa::evidence::model::EvidenceSet {
        runs,
        journals: Vec::new(),
        manual: Vec::new(),
        legacy: Vec::new(),
        implementation: std::collections::BTreeMap::new(),
        current_scope_digests: tdb_qa::evidence::emit::scope_digests(&repo_root()),
        current_input_digests: fixtures,
        declared_universe: tdb_qa::evidence::vocab::catalog_universe(),
    };
    let row = tdb_qa::evidence::merge::merge(&set)
        .rows
        .into_iter()
        .find(|r| r.key.to_string() == READ)
        .expect("row");
    assert_ne!(
        row.qualification,
        tdb_qa::evidence::merge::Qualification::AutomatedQualified
    );
    assert!(
        row.unresolved
            .iter()
            .any(|u| u.starts_with("uncleared-executed-failure:")),
        "{:?}",
        row.unresolved
    );
}

/// A **real** link — a junction on Windows (`mklink /J` needs no elevation), a symlink elsewhere.
fn make_dir_link(link: &Path, target: &Path) {
    #[cfg(windows)]
    {
        let out = Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(link)
            .arg(target)
            .output()
            .expect("run mklink");
        assert!(
            out.status.success(),
            "mklink /J failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).expect("symlink");
}

/// Make `path` unreadable for the lifetime of the returned guard, without touching permissions on
/// Windows: an exclusive open with `share_mode(0)` makes every other read fail with a sharing
/// violation. On Unix the mode bits are cleared and restored.
struct DenyReads {
    #[cfg(windows)]
    _handle: fs::File,
    #[cfg(unix)]
    path: PathBuf,
    #[cfg(unix)]
    original: fs::Permissions,
}

#[cfg(unix)]
impl Drop for DenyReads {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.path, self.original.clone());
    }
}

fn deny_reads(path: &Path) -> DenyReads {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let handle = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(path)
            .expect("open exclusively");
        DenyReads { _handle: handle }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let original = fs::metadata(path).expect("metadata").permissions();
        fs::set_permissions(path, fs::Permissions::from_mode(0o000)).expect("chmod");
        DenyReads {
            path: path.to_path_buf(),
            original,
        }
    }
}
