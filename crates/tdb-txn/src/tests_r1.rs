//! S2-B-R1 regressions: containment (R1), transaction history (R2) and
//! authoritative state under the lock (R3).

use std::path::Path;

use crate::changeset::{ChangeSet, ChangeSetError, Expected, Mode};
use crate::engine::{apply, prepare, recover, restore, status, Event, Hooks, NoHooks, TxnError};
use crate::identity::FileIdentity;
use crate::journal::{Journal, State, ACTIVE_MARKER, JOURNAL_FILE, TXN_DIR};
use crate::tests::{artifact, standard_set, Fixture};

fn active(fx: &Fixture) -> Option<String> {
    std::fs::read_to_string(fx.root.join(TXN_DIR).join(ACTIVE_MARKER)).ok()
}

fn journal_of(fx: &Fixture, txn_id: &str) -> Journal {
    Journal::read(&fx.root.join(TXN_DIR).join(txn_id)).unwrap()
}

fn single(fx: &Fixture, key: &str, dest: &str, old: Option<&[u8]>, new: &[u8]) -> ChangeSet {
    let prior = match old {
        Some(b) => Expected::Present(FileIdentity::of_bytes(b)),
        None => Expected::Absent,
    };
    let rep = fx.write_replacement(&format!("{key}-{}.new", new.len()), new);
    ChangeSet::new(
        format!("{key} -> {dest}"),
        Mode::CopiedTarget,
        vec![artifact(key, dest, prior, rep)],
    )
}

fn commit(fx: &Fixture, cs: &ChangeSet) -> String {
    let mut t = prepare(&fx.root, cs, &mut NoHooks).unwrap();
    apply(&mut t, cs, &mut NoHooks).unwrap();
    t.txn_id
}

#[cfg(windows)]
fn junction(link: &Path, target: &Path) {
    let out = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(std::fs::symlink_metadata(link)
        .unwrap()
        .file_type()
        .is_symlink());
}

// ------------------------------------------------------------------ R1 containment

#[cfg(windows)]
#[test]
fn a_junction_under_the_destination_parent_is_refused_at_prepare_and_at_apply() {
    let fx = Fixture::new("r1-junction");
    let outside = fx.base.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("a"), b"old").unwrap();
    junction(&fx.root.join("escape"), &outside);

    // Prepare refuses the junctioned parent.
    let cs = single(&fx, "a", "escape/a", Some(b"old"), b"new");
    let e = prepare(&fx.root, &cs, &mut NoHooks).unwrap_err();
    assert!(matches!(e, TxnError::Escaping { .. }), "{e}");
    assert_eq!(std::fs::read(outside.join("a")).unwrap(), b"old");
    assert!(status(&fx.root).unwrap().is_empty());

    // A junction inserted after prepare is caught at the mutation boundary.
    std::fs::create_dir(fx.root.join("sub")).unwrap();
    std::fs::write(fx.root.join("sub/a"), b"old").unwrap();
    let cs = single(&fx, "a", "sub/a", Some(b"old"), b"new");
    let mut t = prepare(&fx.root, &cs, &mut NoHooks).unwrap();
    std::fs::remove_dir_all(fx.root.join("sub")).unwrap();
    junction(&fx.root.join("sub"), &outside);
    let e = apply(&mut t, &cs, &mut NoHooks).unwrap_err();
    assert!(e.to_string().contains("not contained"), "{e}");
    assert_eq!(
        std::fs::read(outside.join("a")).unwrap(),
        b"old",
        "outside bytes untouched"
    );
    assert_eq!(journal_of(&fx, &t.txn_id).state, State::Aborted);
    assert_eq!(active(&fx), None);

    // A junction inserted between staging and the rename is caught too.
    std::fs::remove_dir(fx.root.join("sub")).unwrap(); // removes the junction only
    std::fs::create_dir(fx.root.join("sub")).unwrap();
    std::fs::write(fx.root.join("sub/a"), b"old").unwrap();
    struct SwapAfterStaging(std::path::PathBuf, std::path::PathBuf);
    impl Hooks for SwapAfterStaging {
        fn event(&mut self, e: &Event) -> std::io::Result<()> {
            if *e == Event::AfterJournal(State::Applying) {
                std::fs::remove_dir_all(&self.0).unwrap();
                junction(&self.0, &self.1);
            }
            Ok(())
        }
    }
    let cs = single(&fx, "a", "sub/a", Some(b"old"), b"newer");
    let mut hooks = SwapAfterStaging(fx.root.join("sub"), outside.clone());
    let mut t = prepare(&fx.root, &cs, &mut hooks).unwrap();
    let e = apply(&mut t, &cs, &mut hooks).unwrap_err();
    assert!(e.to_string().contains("not contained"), "{e}");
    assert_eq!(
        std::fs::read(outside.join("a")).unwrap(),
        b"old",
        "outside bytes untouched"
    );
    // Rollback also cannot resolve the destination; the set is left for recovery,
    // and the root stays marked. Recovery with the junction still in place
    // reports the problem without following it; once the junction is gone the
    // set rolls back normally. Outside bytes are never touched.
    let j = journal_of(&fx, &t.txn_id);
    assert_eq!(j.state, State::Applying, "{}", j.log.join(" | "));
    assert_eq!(active(&fx).as_deref(), Some(t.txn_id.as_str()));
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    let mine = s.iter().find(|s| s.txn_id == t.txn_id).unwrap();
    assert!(
        mine.problem.as_deref().unwrap().contains("not contained"),
        "{:?}",
        mine
    );
    assert_eq!(
        active(&fx).as_deref(),
        Some(t.txn_id.as_str()),
        "root stays blocked"
    );
    assert_eq!(std::fs::read(outside.join("a")).unwrap(), b"old");
    let (cs_probe, _, _) = standard_set(&fx);
    assert!(matches!(
        prepare(&fx.root, &cs_probe, &mut NoHooks).unwrap_err(),
        TxnError::RootBusy { .. }
    ));
    std::fs::remove_dir(fx.root.join("sub")).unwrap();
    std::fs::create_dir(fx.root.join("sub")).unwrap();
    std::fs::write(fx.root.join("sub/a"), b"old").unwrap();
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    let mine = s.iter().find(|s| s.txn_id == t.txn_id).unwrap();
    assert_eq!(mine.state, Some(State::RolledBack), "{:?}", mine);
    assert_eq!(active(&fx), None);
    assert_eq!(std::fs::read(outside.join("a")).unwrap(), b"old");
}

#[cfg(windows)]
#[test]
fn a_junctioned_recovery_directory_or_txn_dir_is_refused() {
    let fx = Fixture::new("r1-recovery-junction");
    let (cs, olds, _) = standard_set(&fx);
    let outside = fx.base.join("outside-recovery");
    std::fs::create_dir_all(&outside).unwrap();

    // The staged/ subdirectory replaced by a junction after prepare.
    let mut t = prepare(&fx.root, &cs, &mut NoHooks).unwrap();
    std::fs::remove_dir(t.recovery_dir.join("staged")).unwrap();
    junction(&t.recovery_dir.join("staged"), &outside);
    let e = apply(&mut t, &cs, &mut NoHooks).unwrap_err();
    assert!(e.to_string().contains("not contained"), "{e}");
    assert!(
        std::fs::read_dir(&outside).unwrap().next().is_none(),
        "nothing written outside"
    );
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&olds[0][..]));
    let j = journal_of(&fx, &t.txn_id);
    assert_eq!(j.state, State::Aborted);

    // The whole .tdb-txn directory as a junction: refused before anything.
    let fx2 = Fixture::new("r1-txndir-junction");
    let outside2 = fx2.base.join("outside2");
    std::fs::create_dir_all(&outside2).unwrap();
    junction(&fx2.root.join(TXN_DIR), &outside2);
    let (cs2, _, _) = standard_set(&fx2);
    let e = prepare(&fx2.root, &cs2, &mut NoHooks).unwrap_err();
    assert!(e.to_string().contains("not contained"), "{e}");
    assert!(std::fs::read_dir(&outside2).unwrap().next().is_none());
    assert!(matches!(
        recover(&fx2.root, &mut NoHooks).unwrap_err(),
        TxnError::Escaping { .. }
    ));
}

#[test]
fn reserved_device_stream_and_engine_namespace_destinations_are_refused() {
    let fx = Fixture::new("r1-reserved");
    for bad in [
        ".tdb-txn/x",
        ".TDB-TXN/journal.json",
        "COPIED-TARGET.tdb-txn",
        "data/a.big:stream",
        "data/CON",
        "data/nul.txt",
        "data/com1.big",
        "data/a?b",
        "data/a*",
        "data/a|b",
        "data/x\u{1}y",
        "//server/share/a",
    ] {
        let cs = single(&fx, "k", bad, None, b"x");
        let e = prepare(&fx.root, &cs, &mut NoHooks).unwrap_err();
        assert!(
            matches!(
                e,
                TxnError::ChangeSet(ChangeSetError::BadDestination { .. })
            ),
            "{bad:?}: {e}"
        );
    }
    assert!(status(&fx.root).unwrap().is_empty());
}

#[test]
fn escaping_transaction_ids_never_reach_the_filesystem() {
    let fx = Fixture::new("r1-txnid");
    for bad in ["../x", "..", "a/b", "a\\b", ".hidden", "", "-x", "x:y"] {
        let e = restore(&fx.root, bad, &mut NoHooks).unwrap_err();
        assert!(matches!(e, TxnError::Escaping { .. }), "{bad:?}: {e}");
    }
    let e = restore(&fx.root, "000001-1-abc", &mut NoHooks).unwrap_err();
    assert!(matches!(e, TxnError::NoSuchTransaction(_)), "{e}");
    // A journal whose txn_id is not a valid id is corrupt.
    let (cs, _, _) = standard_set(&fx);
    let t = prepare(&fx.root, &cs, &mut NoHooks).unwrap();
    let jp = t.recovery_dir.join(JOURNAL_FILE);
    let text = std::fs::read_to_string(&jp).unwrap();
    std::fs::write(&jp, text.replace(&t.txn_id, "../escape")).unwrap();
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert!(s[0].problem.is_some(), "{:?}", s[0]);
}

// ------------------------------------------------------------------ R2 history

#[test]
fn successive_commits_on_one_destination_are_legitimate_history() {
    let fx = Fixture::new("r2-succession");
    fx.write_dest("data/a.big", b"old");
    let t1 = commit(&fx, &single(&fx, "a", "data/a.big", Some(b"old"), b"one"));
    let t2 = commit(&fx, &single(&fx, "a", "data/a.big", Some(b"one"), b"two"));
    assert!(t1 < t2, "ids sort in sequence order");
    assert_eq!(
        journal_of(&fx, &t2).artifacts[0].lineage_prior.as_deref(),
        Some(t1.as_str())
    );

    // Recovery: both committed, T1 superseded on `a`, root free.
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s[0].state, Some(State::Committed));
    assert_eq!(s[1].state, Some(State::Committed));
    assert_eq!(
        s[0].superseded,
        vec![("data/a.big".to_string(), t2.clone())]
    );
    assert!(s[1].superseded.is_empty());
    assert_eq!(active(&fx), None);
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&b"two"[..]));
    let s2 = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s2, s, "repeated recovery is stable");

    // Reverse-order restore is refused without changing anything.
    let e = restore(&fx.root, &t1, &mut NoHooks).unwrap_err();
    assert!(
        matches!(e, TxnError::RestoreOutOfOrder { ref later, .. } if *later == t2),
        "{e}"
    );
    assert_eq!(journal_of(&fx, &t1).state, State::Committed);
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&b"two"[..]));
    // In order: T2 then T1.
    restore(&fx.root, &t2, &mut NoHooks).unwrap();
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&b"one"[..]));
    restore(&fx.root, &t1, &mut NoHooks).unwrap();
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&b"old"[..]));
    assert!(fx
        .root
        .join(TXN_DIR)
        .join(&t1)
        .join("backups/000.bak")
        .is_file());
    assert!(fx
        .root
        .join(TXN_DIR)
        .join(&t2)
        .join("backups/000.bak")
        .is_file());
}

#[test]
fn overlapping_and_disjoint_sets_order_restore_by_ownership() {
    let fx = Fixture::new("r2-overlap");
    fx.write_dest("data/a.big", b"a0");
    fx.write_dest("data/b.db", b"b0");
    fx.write_dest("data/c.txt", b"c0");
    let mk = |pairs: &[(&str, &str, &[u8], &[u8])], title: &str| {
        let arts = pairs
            .iter()
            .map(|(k, d, old, new)| {
                let rep = fx.write_replacement(&format!("{title}-{k}.new"), new);
                artifact(k, d, Expected::Present(FileIdentity::of_bytes(old)), rep)
            })
            .collect();
        ChangeSet::new(title, Mode::CopiedTarget, arts)
    };
    let t1 = commit(
        &fx,
        &mk(
            &[
                ("a", "data/a.big", b"a0", b"a1"),
                ("b", "data/b.db", b"b0", b"b1"),
            ],
            "t1",
        ),
    );
    let t2 = commit(
        &fx,
        &mk(
            &[
                ("b", "data/b.db", b"b1", b"b2"),
                ("c", "data/c.txt", b"c0", b"c2"),
            ],
            "t2",
        ),
    );
    let t3 = commit(&fx, &mk(&[("a", "data/a.big", b"a1", b"a3")], "t3"));
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert!(s.iter().all(|s| s.state == Some(State::Committed)));
    assert_eq!(s[0].superseded.len(), 2, "{:?}", s[0].superseded);
    // T1 owns nothing now; T2 owns b and c; T3 owns a.
    assert!(matches!(
        restore(&fx.root, &t1, &mut NoHooks).unwrap_err(),
        TxnError::RestoreOutOfOrder { .. }
    ));
    // T3 (disjoint from T2) can be restored before T2.
    restore(&fx.root, &t3, &mut NoHooks).unwrap();
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&b"a1"[..]));
    // T1 still blocked by T2 on `b`.
    assert!(
        matches!(restore(&fx.root, &t1, &mut NoHooks).unwrap_err(), TxnError::RestoreOutOfOrder { ref destination, .. } if destination == "data/b.db")
    );
    restore(&fx.root, &t2, &mut NoHooks).unwrap();
    restore(&fx.root, &t1, &mut NoHooks).unwrap();
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&b"a0"[..]));
    assert_eq!(fx.dest_bytes("data/b.db").as_deref(), Some(&b"b0"[..]));
    assert_eq!(fx.dest_bytes("data/c.txt").as_deref(), Some(&b"c0"[..]));
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert!(s.iter().all(|s| s.state == Some(State::Restored)));
}

#[test]
fn a_crash_or_rollback_of_the_later_transaction_leaves_the_earlier_one_intact() {
    let fx = Fixture::new("r2-later-rollback");
    fx.write_dest("data/a.big", b"old");
    let t1 = commit(&fx, &single(&fx, "a", "data/a.big", Some(b"old"), b"one"));
    // T2 fails after replacing `a` and rolls back.
    struct FailAfter(Event);
    impl Hooks for FailAfter {
        fn event(&mut self, e: &Event) -> std::io::Result<()> {
            if *e == self.0 {
                return Err(std::io::Error::other("injected"));
            }
            Ok(())
        }
    }
    let cs2 = single(&fx, "a", "data/a.big", Some(b"one"), b"two");
    let mut hooks = FailAfter(Event::AfterReplaced("a".into()));
    let mut t2 = prepare(&fx.root, &cs2, &mut hooks).unwrap();
    assert!(apply(&mut t2, &cs2, &mut hooks).is_err());
    assert_eq!(journal_of(&fx, &t2.txn_id).state, State::RolledBack);
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&b"one"[..]));
    // T1 owns `a` again and verifies intact; nothing is superseded.
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s[0].state, Some(State::Committed));
    assert!(s[0].superseded.is_empty());
    assert_eq!(s[1].state, Some(State::RolledBack));
    assert_eq!(active(&fx), None);
    restore(&fx.root, &t1, &mut NoHooks).unwrap();
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&b"old"[..]));

    // A crash-shaped interruption: T2' left Applying with `a` replaced; recovery
    // (newest first) rolls it back, then judges T1'' on the restored bytes.
    let fx = Fixture::new("r2-later-crash");
    fx.write_dest("data/a.big", b"old");
    let _t1 = commit(&fx, &single(&fx, "a", "data/a.big", Some(b"old"), b"one"));
    let cs2 = single(&fx, "a", "data/a.big", Some(b"one"), b"two");
    let mut t2 = prepare(&fx.root, &cs2, &mut NoHooks).unwrap();
    // Drive to Applying with `a` replaced by hand-simulating the crash: fail
    // at AfterReplace so no rollback verification happens... rollback would
    // run; instead stop the process-equivalent by making the hook panic-free
    // "exit": use the journal directly.
    let mut hooks = FailAfter(Event::AfterReplace("a".into()));
    let _ = apply(&mut t2, &cs2, &mut hooks); // rolls back in-process
                                              // Recreate the crash state: put `two` in place and journal Applying/Replacing.
    let mut j = journal_of(&fx, &t2.txn_id);
    std::fs::write(fx.root.join("data/a.big"), b"two").unwrap();
    j.state = State::Applying;
    j.artifacts[0].phase = crate::journal::ArtifactPhase::Replacing;
    j.write(&fx.root.join(TXN_DIR).join(&t2.txn_id)).unwrap();
    std::fs::write(fx.root.join(TXN_DIR).join(ACTIVE_MARKER), &t2.txn_id).unwrap();
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s[1].state, Some(State::RolledBack), "{:?}", s);
    assert_eq!(s[0].state, Some(State::Committed));
    assert!(s[0].superseded.is_empty());
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&b"one"[..]));
    assert_eq!(active(&fx), None);
}

#[test]
fn a_foreign_edit_blames_the_owner_not_the_superseded_history() {
    let fx = Fixture::new("r2-foreign");
    fx.write_dest("data/a.big", b"old");
    let t1 = commit(&fx, &single(&fx, "a", "data/a.big", Some(b"old"), b"one"));
    let t2 = commit(&fx, &single(&fx, "a", "data/a.big", Some(b"one"), b"two"));
    fx.write_dest("data/a.big", b"someone else");
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s[1].state, Some(State::Blocked), "the owner is blocked");
    assert_eq!(s[0].state, Some(State::Committed), "history is not blamed");
    assert_eq!(
        s[0].superseded,
        vec![("data/a.big".to_string(), t2.clone())]
    );
    assert_eq!(active(&fx).as_deref(), Some(t2.as_str()));
    assert_eq!(
        fx.dest_bytes("data/a.big").as_deref(),
        Some(&b"someone else"[..])
    );
    assert!(matches!(
        restore(&fx.root, &t1, &mut NoHooks).unwrap_err(),
        TxnError::RootBusy { .. }
    ));
    let s2 = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s2, s);
}

// ------------------------------------------------------------------ R3 authoritative state

#[test]
fn a_stale_handle_cannot_resurrect_an_aborted_or_superseded_transaction() {
    let fx = Fixture::new("r3-stale");
    let (cs, olds, _) = standard_set(&fx);
    let mut t1 = prepare(&fx.root, &cs, &mut NoHooks).unwrap();
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s[0].state, Some(State::Aborted));
    let e = apply(&mut t1, &cs, &mut NoHooks).unwrap_err();
    assert!(matches!(e, TxnError::StaleHandle { .. }), "{e}");
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&olds[0][..]));
    assert_eq!(journal_of(&fx, &t1.txn_id).state, State::Aborted);
    assert_eq!(active(&fx), None);

    // A second transaction prepared, then the stale first handle applied.
    let (cs2, _, _) = standard_set(&fx);
    let t2 = prepare(&fx.root, &cs2, &mut NoHooks).unwrap();
    let e = apply(&mut t1, &cs, &mut NoHooks).unwrap_err();
    assert!(matches!(e, TxnError::StaleHandle { .. }), "{e}");
    assert_eq!(
        active(&fx).as_deref(),
        Some(t2.txn_id.as_str()),
        "T2 keeps the marker"
    );
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&olds[0][..]));
    // A mutated handle (state edited in memory) is refused too.
    let mut t2m = t2.clone();
    t2m.journal.state = State::Prepared;
    t2m.journal.log.push("tampered".into());
    let e = apply(&mut t2m, &cs2, &mut NoHooks).unwrap_err();
    assert!(matches!(e, TxnError::StaleHandle { .. }), "{e}");
}

#[test]
fn a_different_change_set_cannot_be_applied_to_a_prepared_transaction() {
    let fx = Fixture::new("r3-changeset");
    let (cs, olds, _) = standard_set(&fx);
    let mut t = prepare(&fx.root, &cs, &mut NoHooks).unwrap();
    // Same artifacts, different title → different id.
    let other = ChangeSet::new("other", Mode::CopiedTarget, cs.artifacts.clone());
    let e = apply(&mut t, &other, &mut NoHooks).unwrap_err();
    assert!(matches!(e, TxnError::ChangeSetMismatch { .. }), "{e}");
    // Same id claimed, content differs.
    let mut forged = cs.clone();
    forged.artifacts[1].replacement_identity = FileIdentity::of_bytes(b"forged");
    let e = apply(&mut t, &forged, &mut NoHooks).unwrap_err();
    assert!(matches!(e, TxnError::ChangeSetMismatch { .. }), "{e}");
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&olds[0][..]));
    assert_eq!(journal_of(&fx, &t.txn_id).state, State::Prepared);
    // The genuine change set still applies.
    apply(&mut t, &cs, &mut NoHooks).unwrap();
}

#[test]
fn restore_respects_another_active_transaction_and_its_marker() {
    let fx = Fixture::new("r3-restore-active");
    fx.write_dest("data/a.big", b"old");
    fx.write_dest("data/z.bin", b"z0");
    let t1 = commit(&fx, &single(&fx, "a", "data/a.big", Some(b"old"), b"one"));
    let cs2 = single(&fx, "z", "data/z.bin", Some(b"z0"), b"z1");
    let t2 = prepare(&fx.root, &cs2, &mut NoHooks).unwrap();
    let e = restore(&fx.root, &t1, &mut NoHooks).unwrap_err();
    assert!(
        matches!(e, TxnError::RootBusy { ref txn_id, .. } if *txn_id == t2.txn_id),
        "{e}"
    );
    assert_eq!(
        active(&fx).as_deref(),
        Some(t2.txn_id.as_str()),
        "marker untouched"
    );
    assert_eq!(journal_of(&fx, &t1).state, State::Committed);
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&b"one"[..]));
}

#[test]
fn semantically_invalid_journals_are_retained_errors_never_panics() {
    let fx = Fixture::new("r3-journal-semantics");
    let (cs, _, news) = standard_set(&fx);
    let mut t = prepare(&fx.root, &cs, &mut NoHooks).unwrap();
    apply(&mut t, &cs, &mut NoHooks).unwrap();
    let jp = t.recovery_dir.join(JOURNAL_FILE);
    let good = std::fs::read_to_string(&jp).unwrap();
    let cases: Vec<(&str, String)> = vec![
        (
            "present prior without backup",
            good.replacen("\"backup\": \"backups/000.bak\"", "\"backup\": null", 1),
        ),
        (
            "duplicate key",
            good.replace("\"key\": \"b\"", "\"key\": \"a\""),
        ),
        (
            "duplicate destination",
            good.replace("data/b.db", "data/a.big"),
        ),
        (
            "sequence 0",
            good.replacen("\"sequence\": 1", "\"sequence\": 0", 1),
        ),
        (
            "illegal phase for state",
            good.replace("\"state\": \"committed\"", "\"state\": \"prepared\""),
        ),
        (
            "backup outside backups/",
            good.replacen("backups/000.bak", "staged/000.bak", 1),
        ),
    ];
    for (label, text) in cases {
        assert_ne!(text, good, "{label}: case must change the journal");
        std::fs::write(&jp, &text).unwrap();
        let s = recover(&fx.root, &mut NoHooks).unwrap();
        assert!(s[0].problem.is_some(), "{label}: {:?}", s[0]);
        assert_eq!(
            active(&fx).as_deref(),
            Some(t.txn_id.as_str()),
            "{label}: root blocked"
        );
        assert_eq!(
            fx.dest_bytes("data/a.big").as_deref(),
            Some(&news[0][..]),
            "{label}: untouched"
        );
        let e = restore(&fx.root, &t.txn_id, &mut NoHooks).unwrap_err();
        assert!(
            matches!(e, TxnError::Journal(_) | TxnError::RootBusy { .. }),
            "{label}: {e}"
        );
    }
    // A change set that no longer matches its journal.
    std::fs::write(&jp, &good).unwrap();
    let cp = t.recovery_dir.join("changeset.json");
    let cs_text = std::fs::read_to_string(&cp).unwrap();
    std::fs::write(
        &cp,
        cs_text.replace("\"title\": \"standard\"", "\"title\": \"edited\""),
    )
    .unwrap();
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert!(
        s[0].problem.as_deref().unwrap().contains("change set"),
        "{:?}",
        s[0]
    );
    std::fs::write(&cp, &cs_text).unwrap();
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s[0].state, Some(State::Committed));
    assert_eq!(active(&fx), None);
}

#[test]
fn a_committed_set_of_identical_bytes_and_the_engine_root_marker_are_left_alone() {
    let fx = Fixture::new("r3-misc");
    // Nothing in this test writes the marker directly; the engine owns it.
    assert!(fx.root.join(crate::engine::COPIED_TARGET_MARKER).is_file());
    let cs = single(&fx, "k", "data/same", None, b"same");
    let _ = commit(&fx, &cs);
    assert!(fx.root.join(crate::engine::COPIED_TARGET_MARKER).is_file());
}
