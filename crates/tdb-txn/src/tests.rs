//! Engine cases. Every case builds its own copied-target root under the OS
//! temp directory and removes it at the end.

use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;

use crate::changeset::{
    Artifact, ChangeSet, ChangeSetError, Expected, Mode, Qualification, QualificationItem,
    Validation,
};
use crate::engine::{
    apply, create_copied_target_root, prepare, recover, restore, status, Event, Hooks, NoHooks,
    TxnError, COPIED_TARGET_MARKER,
};
use crate::identity::FileIdentity;
use crate::journal::{ArtifactPhase, Journal, State, ACTIVE_MARKER, JOURNAL_FILE, TXN_DIR};

// ------------------------------------------------------------------ fixtures

pub(crate) struct Fixture {
    pub base: PathBuf,
    pub root: PathBuf,
    pub staging: PathBuf,
}

impl Fixture {
    pub fn new(tag: &str) -> Self {
        let base = std::env::temp_dir().join(format!(
            "tdb-txn-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let root = base.join("copy");
        create_copied_target_root(&root).unwrap();
        std::fs::create_dir_all(root.join("data")).unwrap();
        let staging = base.join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        Self {
            base,
            root,
            staging,
        }
    }
    pub fn write_dest(&self, rel: &str, bytes: &[u8]) -> FileIdentity {
        let p = self.root.join(rel);
        std::fs::write(&p, bytes).unwrap();
        FileIdentity::of_bytes(bytes)
    }
    pub fn write_replacement(&self, name: &str, bytes: &[u8]) -> (PathBuf, FileIdentity) {
        let p = self.staging.join(name);
        std::fs::write(&p, bytes).unwrap();
        (p, FileIdentity::of_bytes(bytes))
    }
    pub fn dest_bytes(&self, rel: &str) -> Option<Vec<u8>> {
        std::fs::read(self.root.join(rel)).ok()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

pub(crate) fn validation(qualified: bool) -> Validation {
    Validation {
        producer: "test".into(),
        structurally_verified: true,
        deploy_blockers: vec![],
        qualification: Qualification {
            state: if qualified {
                "qualified"
            } else {
                "unqualified"
            }
            .into(),
            items: vec![QualificationItem {
                subject: "native loader acceptance".into(),
                status: "unresolved".into(),
                note: "carried".into(),
            }],
        },
        detail: BTreeMap::new(),
    }
}

pub(crate) fn artifact(
    key: &str,
    dest: &str,
    prior: Expected,
    replacement: (PathBuf, FileIdentity),
) -> Artifact {
    Artifact {
        key: key.into(),
        destination: dest.into(),
        expected_prior: prior,
        replacement: replacement.0,
        replacement_identity: replacement.1,
        validation: validation(false),
        dependencies: vec![],
    }
}

/// Two present destinations and one absent companion.
pub(crate) fn standard_set(fx: &Fixture) -> (ChangeSet, Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let old_a = b"archive: original bytes\n".repeat(200);
    let old_b = b"db: original\n".repeat(50);
    let new_a = b"archive: replaced bytes with a longer body\n".repeat(300);
    let new_b = b"db: new\n".repeat(20);
    let new_c = b"companion: new file\n".to_vec();
    let ia = fx.write_dest("data/a.big", &old_a);
    let ib = fx.write_dest("data/b.db", &old_b);
    let ra = fx.write_replacement("a.new", &new_a);
    let rb = fx.write_replacement("b.new", &new_b);
    let rc = fx.write_replacement("c.new", &new_c);
    let mut b = artifact("b", "data/b.db", Expected::Present(ib), rb);
    b.dependencies = vec!["a".into()];
    let mut c = artifact("c", "data/c.txt", Expected::Absent, rc);
    c.dependencies = vec!["b".into()];
    let cs = ChangeSet::new(
        "standard",
        Mode::CopiedTarget,
        vec![c, artifact("a", "data/a.big", Expected::Present(ia), ra), b],
    );
    (cs, vec![old_a, old_b], vec![new_a, new_b, new_c])
}

fn assert_prior(fx: &Fixture, olds: &[Vec<u8>]) {
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&olds[0][..]));
    assert_eq!(fx.dest_bytes("data/b.db").as_deref(), Some(&olds[1][..]));
    assert!(fx.dest_bytes("data/c.txt").is_none());
}

fn assert_replaced(fx: &Fixture, news: &[Vec<u8>]) {
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&news[0][..]));
    assert_eq!(fx.dest_bytes("data/b.db").as_deref(), Some(&news[1][..]));
    assert_eq!(fx.dest_bytes("data/c.txt").as_deref(), Some(&news[2][..]));
}

fn journal_of(fx: &Fixture, txn_id: &str) -> Journal {
    Journal::read(&fx.root.join(TXN_DIR).join(txn_id)).unwrap()
}

fn active(fx: &Fixture) -> Option<String> {
    std::fs::read_to_string(fx.root.join(TXN_DIR).join(ACTIVE_MARKER)).ok()
}

/// Fails (as an I/O error) at one point, optionally also flips the cancel flag.
struct FaultAt {
    at: Event,
    kind: io::ErrorKind,
    seen: Vec<Event>,
    cancel_at: Option<Event>,
    cancelled: bool,
}

impl FaultAt {
    fn new(at: Event) -> Self {
        Self {
            at,
            kind: io::ErrorKind::Other,
            seen: Vec::new(),
            cancel_at: None,
            cancelled: false,
        }
    }
}

impl Hooks for FaultAt {
    fn event(&mut self, e: &Event) -> io::Result<()> {
        self.seen.push(e.clone());
        if Some(e) == self.cancel_at.as_ref() {
            self.cancelled = true;
        }
        if *e == self.at {
            return Err(io::Error::new(self.kind, format!("injected at {e:?}")));
        }
        Ok(())
    }
    fn cancelled(&self) -> bool {
        self.cancelled
    }
}

struct Recording(Vec<Event>);
impl Hooks for Recording {
    fn event(&mut self, e: &Event) -> io::Result<()> {
        self.0.push(e.clone());
        Ok(())
    }
}

// ------------------------------------------------------------------ happy path

#[test]
fn prepare_apply_commit_restore_round_trip() {
    let fx = Fixture::new("roundtrip");
    let (cs, olds, news) = standard_set(&fx);
    let mut rec = Recording(Vec::new());
    let mut txn = prepare(&fx.root, &cs, &mut rec).unwrap();
    assert_eq!(txn.journal.state, State::Prepared);
    assert!(txn.journal.applied_unqualified);
    // Dependency order: a, b, c.
    let keys: Vec<_> = txn
        .journal
        .artifacts
        .iter()
        .map(|a| a.key.as_str())
        .collect();
    assert_eq!(keys, ["a", "b", "c"]);
    assert_eq!(active(&fx).as_deref(), Some(txn.txn_id.as_str()));
    assert_prior(&fx, &olds);
    assert!(txn.recovery_dir.join("changeset.json").is_file());

    let out = apply(&mut txn, &cs, &mut rec).unwrap();
    assert_eq!(out.state, State::Committed);
    assert_replaced(&fx, &news);
    assert_eq!(active(&fx), None, "a committed set does not block the root");
    let j = journal_of(&fx, &txn.txn_id);
    assert_eq!(j.state, State::Committed);
    assert!(j
        .artifacts
        .iter()
        .all(|a| a.phase == ArtifactPhase::Replaced));
    // Backups are full pre-images.
    assert_eq!(
        std::fs::read(
            txn.recovery_dir
                .join(j.artifacts[0].backup.as_ref().unwrap())
        )
        .unwrap(),
        olds[0]
    );
    assert!(j.artifacts[2].backup.is_none());
    // Staged copies were consumed by the renames.
    assert!(!txn.recovery_dir.join(&j.artifacts[0].staged).exists());
    // Event order: stage everything, then Applying, then each replace, then commit.
    let names: Vec<String> = rec.0.iter().map(|e| format!("{e:?}")).collect();
    let pos = |s: &str| names.iter().position(|n| n.starts_with(s)).unwrap();
    assert!(pos("AfterJournal(Staged)") < pos("BeforeJournal(Applying)"));
    assert!(pos("AfterJournal(Applying)") < pos("BeforeReplace(\"a\")"));
    assert!(pos("AfterReplaced(\"c\")") < pos("BeforeCommitRecord"));

    // Recover is a no-op on a committed, intact set.
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s[0].state, Some(State::Committed));
    assert_replaced(&fx, &news);

    // Restore puts the exact originals back and removes the companion.
    let out = restore(&fx.root, &txn.txn_id, &mut NoHooks).unwrap();
    assert_eq!(out.state, State::Restored);
    assert_prior(&fx, &olds);
    assert_eq!(active(&fx), None);
    // Idempotent.
    let again = restore(&fx.root, &txn.txn_id, &mut NoHooks).unwrap();
    assert_eq!(again.state, State::Restored);
    assert_prior(&fx, &olds);
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s[0].state, Some(State::Restored));

    // A second transaction gets its own directory; the first's backups stay.
    let (cs2, _, _) = standard_set(&fx);
    let mut t2 = prepare(&fx.root, &cs2, &mut NoHooks).unwrap();
    assert_ne!(t2.recovery_dir, txn.recovery_dir);
    apply(&mut t2, &cs2, &mut NoHooks).unwrap();
    assert!(txn.recovery_dir.join("backups/000.bak").is_file());
    assert_eq!(status(&fx.root).unwrap().len(), 2);
}

// ------------------------------------------------------------------ change-set refusals

#[test]
fn change_set_refusals_before_any_disk_state() {
    let fx = Fixture::new("refusals");
    let (cs, _, _) = standard_set(&fx);
    let rep = || fx.write_replacement("x.new", b"x");
    let mk = |arts: Vec<Artifact>| ChangeSet::new("t", Mode::CopiedTarget, arts);
    let base = || cs.artifacts[1].clone();

    let e = prepare(&fx.root, &mk(vec![]), &mut NoHooks).unwrap_err();
    assert!(
        matches!(e, TxnError::ChangeSet(ChangeSetError::Empty)),
        "{e}"
    );

    // Duplicate and aliased destinations.
    for alias in ["data/a.big", "data/A.BIG", "./data/a.big", "data//a.big"] {
        let mut b = base();
        b.key = "dup".into();
        b.destination = alias.into();
        let e = prepare(&fx.root, &mk(vec![base(), b]), &mut NoHooks).unwrap_err();
        assert!(
            matches!(
                e,
                TxnError::ChangeSet(ChangeSetError::DuplicateDestination { .. })
            ),
            "{alias}: {e}"
        );
    }
    for bad in [
        "../x",
        "/abs",
        "C:/x",
        "data\\a.big",
        "",
        "data/a.big.",
        "data/a.big ",
    ] {
        let mut b = base();
        b.destination = bad.into();
        let e = prepare(&fx.root, &mk(vec![b]), &mut NoHooks).unwrap_err();
        assert!(
            matches!(
                e,
                TxnError::ChangeSet(ChangeSetError::BadDestination { .. })
            ),
            "{bad:?}: {e}"
        );
    }
    let mut b = base();
    b.dependencies = vec!["nope".into()];
    assert!(matches!(
        prepare(&fx.root, &mk(vec![b]), &mut NoHooks).unwrap_err(),
        TxnError::ChangeSet(ChangeSetError::UnknownDependency { .. })
    ));
    let mut b1 = base();
    b1.dependencies = vec!["two".into()];
    let mut b2 = artifact("two", "data/two", Expected::Absent, rep());
    b2.dependencies = vec!["a".into()];
    assert!(matches!(
        prepare(&fx.root, &mk(vec![b1, b2]), &mut NoHooks).unwrap_err(),
        TxnError::ChangeSet(ChangeSetError::DependencyCycle(_))
    ));
    let mut b = base();
    b.validation.structurally_verified = false;
    assert!(matches!(
        prepare(&fx.root, &mk(vec![b]), &mut NoHooks).unwrap_err(),
        TxnError::ChangeSet(ChangeSetError::NotStructurallyVerified(_))
    ));
    let mut b = base();
    b.validation.deploy_blockers = vec!["row +4 nonzero".into()];
    assert!(matches!(
        prepare(&fx.root, &mk(vec![b]), &mut NoHooks).unwrap_err(),
        TxnError::ChangeSet(ChangeSetError::StructuralBlockers { .. })
    ));
    let mut b = base();
    b.validation.qualification.state = "probably".into();
    assert!(matches!(
        prepare(&fx.root, &mk(vec![b]), &mut NoHooks).unwrap_err(),
        TxnError::ChangeSet(ChangeSetError::UnknownQualificationState { .. })
    ));
    // A tampered id.
    let mut t = cs.clone();
    t.id = "0".repeat(64);
    assert!(matches!(
        prepare(&fx.root, &t, &mut NoHooks).unwrap_err(),
        TxnError::ChangeSet(ChangeSetError::BadId { .. })
    ));
    // Nothing was created under the root by any refusal.
    assert!(
        !fx.root.join(TXN_DIR).exists()
            || std::fs::read_dir(fx.root.join(TXN_DIR))
                .unwrap()
                .all(|e| !e.unwrap().path().is_dir())
    );
}

#[test]
fn prepare_refuses_non_copied_roots_mismatched_disk_state_and_busy_roots() {
    let fx = Fixture::new("prep");
    let (cs, olds, _) = standard_set(&fx);

    // A directory without the marker, even an empty new one.
    let live = fx.base.join("live");
    std::fs::create_dir_all(live.join("data")).unwrap();
    assert!(matches!(
        prepare(&live, &cs, &mut NoHooks).unwrap_err(),
        TxnError::NotCopiedTarget(_)
    ));
    assert!(!live.join(TXN_DIR).exists());
    // A root whose marker was copied by hand is still refused if it is not a
    // regular file... (the marker must be a file). And a missing root.
    assert!(prepare(&fx.base.join("absent"), &cs, &mut NoHooks).is_err());

    // Destination changed since the change set was built.
    fx.write_dest("data/a.big", b"edited by someone else");
    let e = prepare(&fx.root, &cs, &mut NoHooks).unwrap_err();
    assert!(
        matches!(e, TxnError::PriorMismatch { ref key, .. } if key == "a"),
        "{e}"
    );
    fx.write_dest("data/a.big", &olds[0]);
    // Expected-absent destination present.
    fx.write_dest("data/c.txt", b"surprise");
    let e = prepare(&fx.root, &cs, &mut NoHooks).unwrap_err();
    assert!(
        matches!(e, TxnError::PriorMismatch { ref key, .. } if key == "c"),
        "{e}"
    );
    std::fs::remove_file(fx.root.join("data/c.txt")).unwrap();
    // Replacement file changed after its identity was computed.
    std::fs::write(fx.staging.join("b.new"), b"not what was validated").unwrap();
    let e = prepare(&fx.root, &cs, &mut NoHooks).unwrap_err();
    assert!(
        matches!(e, TxnError::ReplacementMismatch { ref key, .. } if key == "b"),
        "{e}"
    );
    std::fs::write(fx.staging.join("b.new"), b"db: new\n".repeat(20)).unwrap();
    // Missing parent directory.
    let mut cs2 = cs.clone();
    cs2.artifacts[0].destination = "nowhere/c.txt".into();
    let cs2 = ChangeSet::new("t", Mode::CopiedTarget, cs2.artifacts);
    assert!(matches!(
        prepare(&fx.root, &cs2, &mut NoHooks).unwrap_err(),
        TxnError::MissingParent { .. }
    ));
    // None of the refusals created a transaction.
    assert!(status(&fx.root).unwrap().is_empty());

    // A prepared-but-not-applied transaction makes the root busy.
    let t = prepare(&fx.root, &cs, &mut NoHooks).unwrap();
    let (cs_b, _, _) = standard_set(&fx);
    let e = prepare(&fx.root, &cs_b, &mut NoHooks).unwrap_err();
    assert!(
        matches!(e, TxnError::RootBusy { ref txn_id, .. } if *txn_id == t.txn_id),
        "{e}"
    );
    // Recovery aborts it (nothing was mutated) and frees the root.
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s[0].state, Some(State::Aborted));
    assert_prior(&fx, &olds);
    assert_eq!(active(&fx), None);
    assert!(prepare(&fx.root, &cs_b, &mut NoHooks).is_ok());
}

// ------------------------------------------------------------------ faults at every point

fn points_for(cs: &ChangeSet) -> Vec<Event> {
    let keys: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
    let mut v = vec![
        Event::BeforeStageReplacement(keys[0].clone()),
        Event::AfterStageReplacement(keys[1].clone()),
        Event::BeforeBackup(keys[0].clone()),
        Event::AfterBackup(keys[1].clone()),
        Event::BeforeJournal(State::Staged),
        Event::AfterJournal(State::Staged),
        Event::BeforeJournal(State::Applying),
        Event::AfterJournal(State::Applying),
    ];
    for k in &keys {
        v.push(Event::BeforeReplace(k.clone()));
        v.push(Event::AfterReplace(k.clone()));
        v.push(Event::AfterReplaced(k.clone()));
    }
    v.push(Event::BeforeCommitRecord);
    v.push(Event::BeforeJournal(State::Committed));
    let _ = cs;
    v
}

/// An injected I/O failure at every protocol point before the commit record
/// leaves every destination at its prior, with a terminal journal, a
/// retained recovery directory, and a free root; recovery afterwards changes
/// nothing. A failure after the commit record leaves a committed set.
#[test]
fn a_failure_at_any_point_before_commit_rolls_back_to_the_pre_images() {
    let fx = Fixture::new("faults");
    let (cs, olds, news) = standard_set(&fx);
    for point in points_for(&cs) {
        let mut hooks = FaultAt::new(point.clone());
        hooks.kind = io::ErrorKind::StorageFull; // "disk full" / flush failure shape
        let mut txn = prepare(&fx.root, &cs, &mut hooks).unwrap();
        let err = apply(&mut txn, &cs, &mut hooks).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("injected at"), "{point:?}: {msg}");
        assert!(
            msg.contains(&txn.recovery_dir.display().to_string()) || msg.contains("retained"),
            "{point:?}: the failure names the retained recovery dir: {msg}"
        );
        let j = journal_of(&fx, &txn.txn_id);
        let mutation_began = matches!(point, Event::AfterJournal(State::Applying))
            || matches!(
                point,
                Event::BeforeReplace(_) | Event::AfterReplace(_) | Event::AfterReplaced(_)
            )
            || matches!(
                point,
                Event::BeforeCommitRecord | Event::BeforeJournal(State::Committed)
            );
        if mutation_began {
            assert_eq!(j.state, State::RolledBack, "{point:?}");
        } else {
            assert_eq!(j.state, State::Aborted, "{point:?}");
        }
        assert_prior(&fx, &olds);
        assert_eq!(active(&fx), None, "{point:?}");
        assert!(txn.recovery_dir.is_dir());
        // Recovery is a no-op on the terminal record.
        let s = recover(&fx.root, &mut NoHooks).unwrap();
        assert!(s.iter().all(|s| s
            .state
            .is_some_and(|st| st.is_terminal() || st == State::Committed)));
        assert_prior(&fx, &olds);
    }
    // After the commit record the set is committed and stays so.
    let mut hooks = FaultAt::new(Event::AfterCommitRecord);
    let mut txn = prepare(&fx.root, &cs, &mut hooks).unwrap();
    let err = apply(&mut txn, &cs, &mut hooks).unwrap_err();
    assert!(err.to_string().contains("injected"));
    assert_eq!(journal_of(&fx, &txn.txn_id).state, State::Committed);
    assert_replaced(&fx, &news);
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert!(s
        .iter()
        .any(|s| s.txn_id == txn.txn_id && s.state == Some(State::Committed)));
    assert_replaced(&fx, &news);
    restore(&fx.root, &txn.txn_id, &mut NoHooks).unwrap();
    assert_prior(&fx, &olds);
}

#[test]
fn cancellation_before_mutation_aborts_and_after_first_replacement_rolls_back() {
    let fx = Fixture::new("cancel");
    let (cs, olds, _) = standard_set(&fx);

    let mut hooks = FaultAt::new(Event::RecoveryDecision("never".into()));
    hooks.cancel_at = Some(Event::AfterBackup("b".into()));
    let mut txn = prepare(&fx.root, &cs, &mut hooks).unwrap();
    let e = apply(&mut txn, &cs, &mut hooks).unwrap_err();
    assert!(
        matches!(e, TxnError::Cancelled(_) | TxnError::Failed { .. }),
        "{e}"
    );
    let j = journal_of(&fx, &txn.txn_id);
    assert_eq!(j.state, State::Aborted, "{}", j.log.join(" | "));
    assert_prior(&fx, &olds);
    assert_eq!(active(&fx), None);

    let mut hooks = FaultAt::new(Event::RecoveryDecision("never".into()));
    hooks.cancel_at = Some(Event::AfterReplaced("a".into()));
    let mut txn = prepare(&fx.root, &cs, &mut hooks).unwrap();
    let e = apply(&mut txn, &cs, &mut hooks).unwrap_err();
    assert!(e.to_string().contains("cancelled"), "{e}");
    let j = journal_of(&fx, &txn.txn_id);
    assert_eq!(j.state, State::RolledBack, "{}", j.log.join(" | "));
    assert_prior(&fx, &olds);
    assert_eq!(active(&fx), None);
}

#[test]
fn a_destination_changed_between_prepare_and_apply_is_a_conflict_and_rolls_back() {
    let fx = Fixture::new("tamper");
    let (cs, olds, _) = standard_set(&fx);
    // Tamper with `b` after `a` was replaced (via a hook that edits the file).
    struct TamperB(PathBuf, bool);
    impl Hooks for TamperB {
        fn event(&mut self, e: &Event) -> io::Result<()> {
            if *e == Event::AfterReplaced("a".into()) && !self.1 {
                self.1 = true;
                std::fs::write(&self.0, b"third-party edit of b").unwrap();
            }
            Ok(())
        }
    }
    let mut hooks = TamperB(fx.root.join("data/b.db"), false);
    let mut txn = prepare(&fx.root, &cs, &mut hooks).unwrap();
    let e = apply(&mut txn, &cs, &mut hooks).unwrap_err();
    assert!(e.to_string().contains("expected"), "{e}");
    // `a` was rolled back; `b` holds foreign bytes → Blocked, nothing overwritten.
    let j = journal_of(&fx, &txn.txn_id);
    assert_eq!(j.state, State::Blocked, "{}", j.log.join(" | "));
    assert_eq!(
        fx.dest_bytes("data/b.db").as_deref(),
        Some(&b"third-party edit of b"[..]),
        "{e}"
    );
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&olds[0][..]));
    assert_eq!(
        active(&fx).as_deref(),
        Some(txn.txn_id.as_str()),
        "root stays blocked"
    );
    assert!(matches!(
        prepare(&fx.root, &cs, &mut NoHooks).unwrap_err(),
        TxnError::RootBusy { .. }
    ));
    // Recovery does not change a blocked set.
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s[0].state, Some(State::Blocked));
    assert_eq!(
        fx.dest_bytes("data/b.db").as_deref(),
        Some(&b"third-party edit of b"[..])
    );
}

#[test]
fn missing_or_corrupt_backups_block_rollback_and_restore_without_overwriting() {
    let fx = Fixture::new("backups");
    let (cs, _, news) = standard_set(&fx);
    // Corrupt the backup of `a` once staging is done, then fail at `b`.
    struct CorruptBackup {
        dir: Option<PathBuf>,
        fail: Event,
    }
    impl Hooks for CorruptBackup {
        fn event(&mut self, e: &Event) -> io::Result<()> {
            if *e == Event::AfterJournal(State::Applying) {
                let p = self.dir.as_ref().unwrap().join("backups/000.bak");
                std::fs::write(&p, b"corrupt").unwrap();
            }
            if *e == self.fail {
                return Err(io::Error::other("injected"));
            }
            Ok(())
        }
    }
    let mut hooks = CorruptBackup {
        dir: None,
        fail: Event::AfterReplace("b".into()),
    };
    let mut txn = prepare(&fx.root, &cs, &mut hooks).unwrap();
    hooks.dir = Some(txn.recovery_dir.clone());
    let e = apply(&mut txn, &cs, &mut hooks).unwrap_err();
    assert!(
        e.to_string().contains("corrupt") || e.to_string().contains("blocked"),
        "{e}"
    );
    let j = journal_of(&fx, &txn.txn_id);
    assert_eq!(j.state, State::Blocked, "{}", j.log.join(" | "));
    // `b` was reverted (its backup is fine); `a` still holds the replacement.
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&news[0][..]));
    assert_eq!(active(&fx).as_deref(), Some(txn.txn_id.as_str()));

    // Restore on a committed set with a missing backup: blocked, nothing overwritten.
    let fx2 = Fixture::new("backups2");
    let (cs, _, news) = standard_set(&fx2);
    let mut txn = prepare(&fx2.root, &cs, &mut NoHooks).unwrap();
    apply(&mut txn, &cs, &mut NoHooks).unwrap();
    std::fs::remove_file(txn.recovery_dir.join("backups/001.bak")).unwrap();
    let e = restore(&fx2.root, &txn.txn_id, &mut NoHooks).unwrap_err();
    assert!(matches!(e, TxnError::Blocked { .. }), "{e}");
    assert_eq!(fx2.dest_bytes("data/b.db").as_deref(), Some(&news[1][..]));
    assert_eq!(fx2.dest_bytes("data/a.big").as_deref(), Some(&news[0][..]));
}

#[test]
fn restore_refuses_a_destination_edited_after_commit() {
    let fx = Fixture::new("external");
    let (cs, _, news) = standard_set(&fx);
    let mut txn = prepare(&fx.root, &cs, &mut NoHooks).unwrap();
    apply(&mut txn, &cs, &mut NoHooks).unwrap();
    fx.write_dest("data/a.big", b"a later external edit");
    let e = restore(&fx.root, &txn.txn_id, &mut NoHooks).unwrap_err();
    assert!(matches!(e, TxnError::Blocked { .. }), "{e}");
    assert_eq!(
        fx.dest_bytes("data/a.big").as_deref(),
        Some(&b"a later external edit"[..])
    );
    assert_eq!(
        fx.dest_bytes("data/b.db").as_deref(),
        Some(&news[1][..]),
        "untouched"
    );
    // Recovery of a committed set with foreign bytes: blocked, unchanged.
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s[0].state, Some(State::Blocked));
    assert_eq!(
        fx.dest_bytes("data/a.big").as_deref(),
        Some(&b"a later external edit"[..])
    );
}

#[test]
fn corrupt_truncated_and_escaping_journals_block_the_root_and_are_retained() {
    let fx = Fixture::new("journal");
    let (cs, olds, _) = standard_set(&fx);
    let mut txn = prepare(&fx.root, &cs, &mut NoHooks).unwrap();
    apply(&mut txn, &cs, &mut NoHooks).unwrap();
    let jpath = txn.recovery_dir.join(JOURNAL_FILE);
    let good = std::fs::read(&jpath).unwrap();

    // Truncated.
    std::fs::write(&jpath, &good[..good.len() / 2]).unwrap();
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert!(
        s[0].problem.as_deref().unwrap().contains("corrupt"),
        "{:?}",
        s[0]
    );
    assert_eq!(active(&fx).as_deref(), Some(txn.txn_id.as_str()));
    assert!(matches!(
        prepare(&fx.root, &cs, &mut NoHooks).unwrap_err(),
        TxnError::RootBusy { .. }
    ));
    assert!(matches!(
        restore(&fx.root, &txn.txn_id, &mut NoHooks).unwrap_err(),
        TxnError::Journal(_)
    ));
    assert!(jpath.is_file() && txn.recovery_dir.join("backups/000.bak").is_file());

    // Escaping destination.
    let text = String::from_utf8(good.clone()).unwrap();
    std::fs::write(&jpath, text.replace("data/a.big", "../../a.big")).unwrap();
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert!(
        s[0].problem.as_deref().unwrap().contains("escapes"),
        "{:?}",
        s[0]
    );
    // Escaping backup path.
    std::fs::write(&jpath, text.replace("backups/000.bak", "../../../etc")).unwrap();
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert!(
        s[0].problem.as_deref().unwrap().contains("escapes"),
        "{:?}",
        s[0]
    );
    // A journal whose txn_id names another directory.
    std::fs::write(&jpath, text.replace(&txn.txn_id, "some-other-txn")).unwrap();
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert!(s[0].problem.is_some());

    // Restored good journal: recovery verifies and the root is free again.
    std::fs::write(&jpath, &good).unwrap();
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s[0].state, Some(State::Committed));
    assert_eq!(active(&fx), None);
    restore(&fx.root, &txn.txn_id, &mut NoHooks).unwrap();
    assert_prior(&fx, &olds);
}

#[cfg(windows)]
#[test]
fn a_destination_held_open_without_sharing_fails_the_rename_and_rolls_back() {
    use std::os::windows::fs::OpenOptionsExt;
    let fx = Fixture::new("locked");
    let (cs, olds, _) = standard_set(&fx);
    let mut txn = prepare(&fx.root, &cs, &mut NoHooks).unwrap();
    // FILE_SHARE_READ only: the backup copy can read it, the rename cannot
    // replace it (no delete sharing).
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(fx.root.join("data/b.db"))
        .unwrap();
    let e = apply(&mut txn, &cs, &mut NoHooks).unwrap_err();
    let msg = e.to_string();
    assert!(
        matches!(e, TxnError::Failed { ref key, .. } if key == "b"),
        "{msg}"
    );
    drop(held);
    let j = journal_of(&fx, &txn.txn_id);
    assert_eq!(j.state, State::RolledBack, "{}", j.log.join(" | "));
    assert_prior(&fx, &olds);
}

#[test]
fn the_root_lock_excludes_a_concurrent_operation() {
    let fx = Fixture::new("lock");
    let (cs, _, _) = standard_set(&fx);
    let _txn = prepare(&fx.root, &cs, &mut NoHooks).unwrap();
    // Hold the lock as another operation would.
    let lock_path = fx.root.join(TXN_DIR).join(crate::journal::LOCK_FILE);
    let mut opts = std::fs::OpenOptions::new();
    opts.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        opts.share_mode(0);
    }
    let _held = opts.open(&lock_path).unwrap();
    #[cfg(windows)]
    {
        let e = recover(&fx.root, &mut NoHooks).unwrap_err();
        assert!(matches!(e, TxnError::Locked(_)), "{e}");
    }
}

#[test]
fn marker_and_identity_helpers() {
    let fx = Fixture::new("helpers");
    assert!(fx.root.join(COPIED_TARGET_MARKER).is_file());
    assert!(
        create_copied_target_root(&fx.root).is_err(),
        "existing dir refused"
    );
    let p = fx.root.join("data/x");
    assert_eq!(FileIdentity::of_path_or_absent(&p).unwrap(), None);
    std::fs::write(&p, b"abc").unwrap();
    let id = FileIdentity::of_path_or_absent(&p).unwrap().unwrap();
    assert_eq!(id, FileIdentity::of_bytes(b"abc"));
    assert_eq!(
        id.sha256,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert!(
        FileIdentity::of_path_or_absent(&fx.root.join("data")).is_err(),
        "a directory is neither"
    );
}
