//! Fresh-process interruption tests. A child process (this same test binary,
//! re-invoked with `TDB_TXN_CHILD` set) runs `apply` or `restore` with a hook
//! that calls `std::process::exit` at one protocol point — no cleanup, no
//! destructors, no in-memory state survives. The parent then inspects only
//! what is on disk, runs `recover` from a process that never held the
//! transaction, and checks the deterministic policy outcome.
//!
//! This is process-interruption simulation. It says nothing about power loss
//! or write-cache durability.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use tdb_txn::engine::{
    apply, create_copied_target_root, prepare, recover, restore, Event, Hooks, NoHooks,
};
use tdb_txn::{
    Artifact, ChangeSet, Expected, FileIdentity, Journal, Mode, Qualification, QualificationItem,
    State, Validation,
};

const TXN_DIR: &str = ".tdb-txn";

fn validation() -> Validation {
    Validation {
        producer: "crash-test".into(),
        structurally_verified: true,
        deploy_blockers: vec![],
        qualification: Qualification {
            state: "unqualified".into(),
            items: vec![QualificationItem {
                subject: "native loader acceptance".into(),
                status: "unresolved".into(),
                note: "carried".into(),
            }],
        },
        detail: BTreeMap::new(),
    }
}

struct Files {
    olds: Vec<Vec<u8>>,
    news: Vec<Vec<u8>>,
}

fn files() -> Files {
    Files {
        olds: vec![b"A original\n".repeat(400), b"B original\n".repeat(90)],
        news: vec![
            b"A replaced, longer\n".repeat(600),
            b"B replaced\n".repeat(30),
            b"C companion\n".to_vec(),
        ],
    }
}

/// Build the copied root, the staging files and the change set; save the
/// change set as JSON for the child.
fn build(base: &Path) -> (PathBuf, ChangeSet, Files) {
    let f = files();
    let root = base.join("copy");
    create_copied_target_root(&root).unwrap();
    std::fs::create_dir_all(root.join("d")).unwrap();
    std::fs::write(root.join("d/a.big"), &f.olds[0]).unwrap();
    std::fs::write(root.join("d/b.db"), &f.olds[1]).unwrap();
    let staging = base.join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    let mut arts = Vec::new();
    for (i, (key, dest, prior)) in [
        ("a", "d/a.big", Some(&f.olds[0])),
        ("b", "d/b.db", Some(&f.olds[1])),
        ("c", "d/c.txt", None),
    ]
    .into_iter()
    .enumerate()
    {
        let rep = staging.join(format!("{key}.new"));
        std::fs::write(&rep, &f.news[i]).unwrap();
        arts.push(Artifact {
            key: key.into(),
            destination: dest.into(),
            expected_prior: match prior {
                Some(b) => Expected::Present(FileIdentity::of_bytes(b)),
                None => Expected::Absent,
            },
            replacement: rep,
            replacement_identity: FileIdentity::of_bytes(&f.news[i]),
            validation: validation(),
            dependencies: if i == 0 {
                vec![]
            } else {
                vec![["a", "b"][i - 1].into()]
            },
        });
    }
    let cs = ChangeSet::new("crash", Mode::CopiedTarget, arts);
    std::fs::write(
        base.join("changeset.json"),
        serde_json::to_vec(&cs).unwrap(),
    )
    .unwrap();
    (root, cs, f)
}

struct ExitAt(String);
impl Hooks for ExitAt {
    fn event(&mut self, e: &Event) -> std::io::Result<()> {
        if format!("{e:?}") == self.0 {
            // No unwinding, no Drop: the process is gone.
            std::process::exit(9);
        }
        Ok(())
    }
}

/// The child entry point: only acts when `TDB_TXN_CHILD` is set.
#[test]
fn child() {
    let Ok(op) = std::env::var("TDB_TXN_CHILD") else {
        return;
    };
    let base = PathBuf::from(std::env::var("TDB_TXN_BASE").unwrap());
    let at = std::env::var("TDB_TXN_AT").unwrap();
    let root = base.join("copy");
    let mut hooks = ExitAt(at);
    match op.as_str() {
        "apply" => {
            let cs: ChangeSet =
                serde_json::from_slice(&std::fs::read(base.join("changeset.json")).unwrap())
                    .unwrap();
            let mut txn = prepare(&root, &cs, &mut hooks).unwrap();
            std::fs::write(base.join("txn_id"), &txn.txn_id).unwrap();
            let _ = apply(&mut txn, &cs, &mut hooks);
        }
        "restore" => {
            let id = std::fs::read_to_string(base.join("txn_id")).unwrap();
            let _ = restore(&root, &id, &mut hooks);
        }
        other => panic!("unknown op {other}"),
    }
    std::process::exit(0);
}

fn spawn_child(base: &Path, op: &str, at: &str) -> i32 {
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child", "-q", "--test-threads=1"])
        .env("TDB_TXN_CHILD", op)
        .env("TDB_TXN_BASE", base)
        .env("TDB_TXN_AT", at)
        .status()
        .unwrap();
    status.code().unwrap_or(-1)
}

fn fresh_base(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "tdb-txn-crash-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    base
}

fn read(root: &Path, rel: &str) -> Option<Vec<u8>> {
    std::fs::read(root.join(rel)).ok()
}

fn journal(root: &Path, txn_id: &str) -> Journal {
    Journal::read(&root.join(TXN_DIR).join(txn_id)).unwrap()
}

/// What the parent expects to find on disk right after the child died at a
/// point, before any recovery.
enum OnDisk {
    /// Nothing replaced yet.
    AllPrior,
    /// `a` replaced, `b` and `c` prior.
    AOnly,
    /// `a` and `b` replaced, `c` absent.
    AB,
    /// Everything replaced.
    All,
}

fn check_on_disk(root: &Path, f: &Files, what: &OnDisk) {
    let (a, b, c) = (
        read(root, "d/a.big"),
        read(root, "d/b.db"),
        read(root, "d/c.txt"),
    );
    match what {
        OnDisk::AllPrior => {
            assert_eq!(a.as_deref(), Some(&f.olds[0][..]));
            assert_eq!(b.as_deref(), Some(&f.olds[1][..]));
            assert!(c.is_none());
        }
        OnDisk::AOnly => {
            assert_eq!(a.as_deref(), Some(&f.news[0][..]));
            assert_eq!(b.as_deref(), Some(&f.olds[1][..]));
            assert!(c.is_none());
        }
        OnDisk::AB => {
            assert_eq!(a.as_deref(), Some(&f.news[0][..]));
            assert_eq!(b.as_deref(), Some(&f.news[1][..]));
            assert!(c.is_none());
        }
        OnDisk::All => {
            assert_eq!(a.as_deref(), Some(&f.news[0][..]));
            assert_eq!(b.as_deref(), Some(&f.news[1][..]));
            assert_eq!(c.as_deref(), Some(&f.news[2][..]));
        }
    }
}

#[test]
fn a_process_killed_around_every_durable_transition_recovers_deterministically() {
    // (point, journal state right after the crash, bytes on disk, state after recover)
    let cases: Vec<(&str, State, OnDisk, State)> = vec![
        // Staging: nothing outside the recovery dir; recover aborts.
        (
            "AfterStageReplacement(\"b\")",
            State::Prepared,
            OnDisk::AllPrior,
            State::Aborted,
        ),
        (
            "AfterBackup(\"b\")",
            State::Prepared,
            OnDisk::AllPrior,
            State::Aborted,
        ),
        (
            "AfterJournal(Staged)",
            State::Staged,
            OnDisk::AllPrior,
            State::Aborted,
        ),
        // The "mutation begins" record is on disk but nothing was replaced.
        (
            "AfterJournal(Applying)",
            State::Applying,
            OnDisk::AllPrior,
            State::RolledBack,
        ),
        // Immediately after the first rename, before it was journaled Replaced.
        (
            "AfterReplace(\"a\")",
            State::Applying,
            OnDisk::AOnly,
            State::RolledBack,
        ),
        // First target changed and journaled, second not yet.
        (
            "AfterReplaced(\"a\")",
            State::Applying,
            OnDisk::AOnly,
            State::RolledBack,
        ),
        (
            "BeforeReplace(\"b\")",
            State::Applying,
            OnDisk::AOnly,
            State::RolledBack,
        ),
        (
            "AfterReplace(\"b\")",
            State::Applying,
            OnDisk::AB,
            State::RolledBack,
        ),
        (
            "AfterReplace(\"c\")",
            State::Applying,
            OnDisk::All,
            State::RolledBack,
        ),
        // Everything replaced and verified, commit record not yet written.
        (
            "BeforeCommitRecord",
            State::Applying,
            OnDisk::All,
            State::RolledBack,
        ),
        (
            "BeforeJournal(Committed)",
            State::Applying,
            OnDisk::All,
            State::RolledBack,
        ),
        // Commit record on disk: the set stands.
        (
            "AfterJournal(Committed)",
            State::Committed,
            OnDisk::All,
            State::Committed,
        ),
        (
            "AfterCommitRecord",
            State::Committed,
            OnDisk::All,
            State::Committed,
        ),
    ];
    for (i, (at, expect_state, on_disk, after)) in cases.iter().enumerate() {
        let base = fresh_base(&format!("apply{i}"));
        let (root, _cs, f) = build(&base);
        let code = spawn_child(&base, "apply", at);
        assert_eq!(code, 9, "{at}: child must die at the point");
        let txn_id = std::fs::read_to_string(base.join("txn_id")).unwrap();

        // On-disk state right after the crash, read by this process only.
        let j = journal(&root, &txn_id);
        assert_eq!(j.state, *expect_state, "{at}: journal state after crash");
        check_on_disk(&root, &f, on_disk);
        let active = std::fs::read_to_string(root.join(TXN_DIR).join("ACTIVE")).ok();
        if *expect_state != State::Committed {
            assert_eq!(
                active.as_deref(),
                Some(txn_id.as_str()),
                "{at}: root marked active"
            );
        }

        // Fresh-process recovery from the journal and the files alone.
        let s = recover(&root, &mut NoHooks).unwrap();
        let mine = s.iter().find(|s| s.txn_id == txn_id).unwrap();
        assert_eq!(mine.state, Some(*after), "{at}: recovery outcome");
        let j = journal(&root, &txn_id);
        assert_eq!(j.state, *after);
        match after {
            State::Committed => {
                check_on_disk(&root, &f, &OnDisk::All);
                // Exact hashes of every new target.
                for (rel, bytes) in [
                    ("d/a.big", &f.news[0]),
                    ("d/b.db", &f.news[1]),
                    ("d/c.txt", &f.news[2]),
                ] {
                    assert_eq!(
                        FileIdentity::of_file(&root.join(rel)).unwrap(),
                        FileIdentity::of_bytes(bytes)
                    );
                }
            }
            _ => {
                check_on_disk(&root, &f, &OnDisk::AllPrior);
                for (rel, bytes) in [("d/a.big", &f.olds[0]), ("d/b.db", &f.olds[1])] {
                    assert_eq!(
                        FileIdentity::of_file(&root.join(rel)).unwrap(),
                        FileIdentity::of_bytes(bytes)
                    );
                }
            }
        }
        assert!(
            !root.join(TXN_DIR).join("ACTIVE").exists(),
            "{at}: resolved sets free the root"
        );
        // Backups are still there whatever happened.
        assert!(root
            .join(TXN_DIR)
            .join(&txn_id)
            .join("backups/000.bak")
            .is_file());

        // Repeated recovery changes nothing.
        let s2 = recover(&root, &mut NoHooks).unwrap();
        assert_eq!(s2, s, "{at}: recovery is idempotent");
        let j2 = journal(&root, &txn_id);
        assert_eq!(j2.state, j.state);

        // A committed set can then be restored to the exact originals.
        if *after == State::Committed {
            let out = restore(&root, &txn_id, &mut NoHooks).unwrap();
            assert_eq!(out.state, State::Restored);
            check_on_disk(&root, &f, &OnDisk::AllPrior);
        }
        let _ = std::fs::remove_dir_all(&base);
    }
}

#[test]
fn a_process_killed_during_restore_is_completed_by_recovery_and_repeated_recovery_is_a_noop() {
    let cases: Vec<(&str, OnDisk)> = vec![
        // Restore reverts in reverse order: c (removed), then b, then a.
        ("AfterJournal(Restoring)", OnDisk::All),
        ("AfterRevert(\"c\")", OnDisk::AB),
        ("AfterReverted(\"c\")", OnDisk::AB),
        ("AfterRevert(\"b\")", OnDisk::AOnly),
        ("BeforeRevert(\"a\")", OnDisk::AOnly),
        ("AfterRevert(\"a\")", OnDisk::AllPrior),
    ];
    for (i, (at, on_disk)) in cases.iter().enumerate() {
        let base = fresh_base(&format!("restore{i}"));
        let (root, cs, f) = build(&base);
        // Apply fully in this process, then let a child crash mid-restore.
        let mut txn = prepare(&root, &cs, &mut NoHooks).unwrap();
        apply(&mut txn, &cs, &mut NoHooks).unwrap();
        std::fs::write(base.join("txn_id"), &txn.txn_id).unwrap();
        check_on_disk(&root, &f, &OnDisk::All);

        let code = spawn_child(&base, "restore", at);
        assert_eq!(code, 9, "{at}");
        let j = journal(&root, &txn.txn_id);
        assert_eq!(j.state, State::Restoring, "{at}");
        check_on_disk(&root, &f, on_disk);

        let s = recover(&root, &mut NoHooks).unwrap();
        assert_eq!(s[0].state, Some(State::Restored), "{at}");
        check_on_disk(&root, &f, &OnDisk::AllPrior);
        for (rel, bytes) in [("d/a.big", &f.olds[0]), ("d/b.db", &f.olds[1])] {
            assert_eq!(
                FileIdentity::of_file(&root.join(rel)).unwrap(),
                FileIdentity::of_bytes(bytes)
            );
        }
        assert!(!root.join(TXN_DIR).join("ACTIVE").exists());
        let s2 = recover(&root, &mut NoHooks).unwrap();
        assert_eq!(s2, s);
        // Calling restore again on the restored set is a no-op.
        let out = restore(&root, &txn.txn_id, &mut NoHooks).unwrap();
        assert_eq!(out.state, State::Restored);
        check_on_disk(&root, &f, &OnDisk::AllPrior);
        let _ = std::fs::remove_dir_all(&base);
    }
}

#[test]
fn a_crash_mid_rollback_is_finished_by_recovery() {
    // Kill the child while it is rolling back after a failure: the child
    // fails at BeforeReplace("c") through... we cannot inject two faults
    // from one hook string, so use the restore path's reverse-order revert
    // via an Applying journal: crash right after `b` was replaced, recover
    // (rolls back), and crash *that* recovery is not spawnable — instead
    // exercise "repeated recovery after an interrupted revert" by killing
    // a restore (covered above) and by re-running recover on an Applying
    // set whose first artifact is already back to its prior.
    let base = fresh_base("midrollback");
    let (root, _cs, f) = build(&base);
    let code = spawn_child(&base, "apply", "AfterReplace(\"b\")");
    assert_eq!(code, 9);
    let txn_id = std::fs::read_to_string(base.join("txn_id")).unwrap();
    // Simulate a rollback that got as far as reverting `b` (last first) by
    // hand-restoring `b` from the backup, exactly as the engine would.
    let dir = root.join(TXN_DIR).join(&txn_id);
    std::fs::copy(dir.join("backups/001.bak"), root.join("d/b.db")).unwrap();
    check_on_disk(&root, &f, &OnDisk::AOnly);
    let s = recover(&root, &mut NoHooks).unwrap();
    assert_eq!(s[0].state, Some(State::RolledBack));
    check_on_disk(&root, &f, &OnDisk::AllPrior);
    let _ = std::fs::remove_dir_all(&base);
}
