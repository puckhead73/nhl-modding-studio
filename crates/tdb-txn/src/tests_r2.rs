//! S2-B-R2 regressions: the mutation boundary. A destination that becomes
//! foreign after inspection but before the rename/remove must be left in
//! place and the set blocked — for restore, rollback and recovery, and for
//! both backup-replacement and absent-target deletion.

use std::io;

use crate::changeset::{ChangeSet, Expected, Mode};
use crate::engine::{apply, prepare, recover, restore, Event, Hooks, NoHooks, TxnError};
use crate::identity::FileIdentity;
use crate::journal::{Journal, State, ACTIVE_MARKER, TXN_DIR};
use crate::tests::{artifact, Fixture};

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
        format!("{key}->{dest}"),
        Mode::CopiedTarget,
        vec![artifact(key, dest, prior, rep)],
    )
}
fn commit(fx: &Fixture, cs: &ChangeSet) -> String {
    let mut t = prepare(&fx.root, cs, &mut NoHooks).unwrap();
    apply(&mut t, cs, &mut NoHooks).unwrap();
    t.txn_id
}

/// Writes `bytes` to `path` (absolute) the first time `at` fires.
struct EditAt {
    at: Event,
    path: std::path::PathBuf,
    bytes: Vec<u8>,
    done: bool,
}
impl EditAt {
    fn new(at: Event, path: std::path::PathBuf, bytes: &[u8]) -> Self {
        Self {
            at,
            path,
            bytes: bytes.to_vec(),
            done: false,
        }
    }
}
impl Hooks for EditAt {
    fn event(&mut self, e: &Event) -> io::Result<()> {
        if !self.done && *e == self.at {
            self.done = true;
            std::fs::write(&self.path, &self.bytes).unwrap();
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ restore boundary

#[test]
fn restore_leaves_a_foreign_edit_made_at_before_revert_and_blocks() {
    // The exact review reproducer.
    let fx = Fixture::new("r2-restore-boundary");
    fx.write_dest("data/a.big", b"old");
    let cs = single(&fx, "a", "data/a.big", Some(b"old"), b"new");
    let t = commit(&fx, &cs);
    assert_eq!(fx.dest_bytes("data/a.big").as_deref(), Some(&b"new"[..]));

    let mut hooks = EditAt::new(
        Event::BeforeRevert("a".into()),
        fx.root.join("data/a.big"),
        b"external edit",
    );
    let e = restore(&fx.root, &t, &mut hooks).unwrap_err();
    assert!(matches!(e, TxnError::Blocked { .. }), "{e}");
    // The foreign bytes are preserved, not overwritten with `old`.
    assert_eq!(
        fx.dest_bytes("data/a.big").as_deref(),
        Some(&b"external edit"[..])
    );
    let j = journal_of(&fx, &t);
    assert_eq!(j.state, State::Blocked, "{}", j.log.join(" | "));
    assert_eq!(
        active(&fx).as_deref(),
        Some(t.as_str()),
        "root stays blocked"
    );
    // Recovery material is retained.
    assert!(fx
        .root
        .join(TXN_DIR)
        .join(&t)
        .join("backups/000.bak")
        .is_file());
    // Recovery does not resolve the conflict on its own.
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s[0].state, Some(State::Blocked));
    assert_eq!(
        fx.dest_bytes("data/a.big").as_deref(),
        Some(&b"external edit"[..])
    );
    // Explicit resolution: put the replacement back, then restore succeeds.
    fx.write_dest("data/a.big", b"new");
    let out = restore(&fx.root, &t, &mut NoHooks);
    // Still Blocked in the journal; restore refuses a blocked set.
    assert!(out.is_err(), "a blocked set is not silently restorable");
}

#[test]
fn restore_of_an_absent_companion_leaves_a_foreign_file_and_blocks() {
    let fx = Fixture::new("r2-absent-boundary");
    let cs = single(&fx, "c", "data/c.txt", None, b"installed");
    let t = commit(&fx, &cs);
    assert_eq!(
        fx.dest_bytes("data/c.txt").as_deref(),
        Some(&b"installed"[..])
    );

    // At BeforeRevert an external writer replaces the companion with its own bytes.
    let mut hooks = EditAt::new(
        Event::BeforeRevert("c".into()),
        fx.root.join("data/c.txt"),
        b"someone else's file",
    );
    let e = restore(&fx.root, &t, &mut hooks).unwrap_err();
    assert!(matches!(e, TxnError::Blocked { .. }), "{e}");
    // The foreign file is NOT deleted.
    assert_eq!(
        fx.dest_bytes("data/c.txt").as_deref(),
        Some(&b"someone else's file"[..])
    );
    assert_eq!(journal_of(&fx, &t).state, State::Blocked);
    assert_eq!(active(&fx).as_deref(), Some(t.as_str()));
}

#[test]
fn an_edit_after_backup_preparation_is_still_caught_at_the_boundary() {
    // The edit fires at AfterRevert of a *different* artifact so the backup
    // copy of `a` has already run; the boundary check for `a` still catches it.
    // Two artifacts: b reverts first (reverse order), its AfterRevert edits a.
    let fx = Fixture::new("r2-after-backup");
    fx.write_dest("data/a.big", b"a-old");
    fx.write_dest("data/b.db", b"b-old");
    let arts = vec![
        artifact(
            "a",
            "data/a.big",
            Expected::Present(FileIdentity::of_bytes(b"a-old")),
            fx.write_replacement("a.new", b"a-new"),
        ),
        artifact(
            "b",
            "data/b.db",
            Expected::Present(FileIdentity::of_bytes(b"b-old")),
            fx.write_replacement("b.new", b"b-new"),
        ),
    ];
    let cs = ChangeSet::new("two", Mode::CopiedTarget, arts);
    let t = commit(&fx, &cs);
    // Restore reverts b (last) first; its BeforeRevert edits a, after a's
    // backup was already staged at commit time.
    let mut hooks = EditAt::new(
        Event::BeforeRevert("b".into()),
        fx.root.join("data/a.big"),
        b"a-foreign",
    );
    let e = restore(&fx.root, &t, &mut hooks).unwrap_err();
    assert!(matches!(e, TxnError::Blocked { .. }), "{e}");
    // b was reverted (its revert ran before the conflict on a was reached);
    // a keeps the foreign bytes, never clobbered by its backup.
    assert_eq!(
        fx.dest_bytes("data/a.big").as_deref(),
        Some(&b"a-foreign"[..])
    );
    assert_eq!(journal_of(&fx, &t).state, State::Blocked);
}

// ------------------------------------------------------------------ rollback boundary

#[test]
fn rollback_during_apply_leaves_a_foreign_edit_at_the_boundary() {
    // Force a rollback by failing the last replace, and edit an already-replaced
    // destination at that artifact's BeforeRevert.
    let fx = Fixture::new("r2-rollback-boundary");
    fx.write_dest("data/a.big", b"a-old");
    fx.write_dest("data/b.db", b"b-old");
    let arts = vec![
        artifact(
            "a",
            "data/a.big",
            Expected::Present(FileIdentity::of_bytes(b"a-old")),
            fx.write_replacement("a.new", b"a-new"),
        ),
        artifact(
            "b",
            "data/b.db",
            Expected::Present(FileIdentity::of_bytes(b"b-old")),
            fx.write_replacement("b.new", b"b-new"),
        ),
    ];
    let cs = ChangeSet::new("two", Mode::CopiedTarget, arts);
    // Fail at b's AfterReplace so apply rolls back; at a's BeforeRevert, edit a.
    struct FailBAndEditA {
        a_path: std::path::PathBuf,
        edited: bool,
    }
    impl Hooks for FailBAndEditA {
        fn event(&mut self, e: &Event) -> io::Result<()> {
            if *e == Event::AfterReplace("b".into()) {
                return Err(io::Error::other("injected"));
            }
            if *e == Event::BeforeRevert("a".into()) && !self.edited {
                self.edited = true;
                std::fs::write(&self.a_path, b"a-foreign").unwrap();
            }
            Ok(())
        }
    }
    let mut hooks = FailBAndEditA {
        a_path: fx.root.join("data/a.big"),
        edited: false,
    };
    let mut t = prepare(&fx.root, &cs, &mut hooks).unwrap();
    let e = apply(&mut t, &cs, &mut hooks).unwrap_err();
    assert!(e.to_string().contains("injected"), "{e}");
    // b rolled back to b-old (its revert ran first); a keeps the foreign bytes.
    assert_eq!(fx.dest_bytes("data/b.db").as_deref(), Some(&b"b-old"[..]));
    assert_eq!(
        fx.dest_bytes("data/a.big").as_deref(),
        Some(&b"a-foreign"[..])
    );
    assert_eq!(
        journal_of(&fx, &t.txn_id).state,
        State::Blocked,
        "{}",
        journal_of(&fx, &t.txn_id).log.join(" | ")
    );
    assert_eq!(active(&fx).as_deref(), Some(t.txn_id.as_str()));
}

// ------------------------------------------------------------------ fresh-process recovery boundary

#[test]
fn recovery_applies_the_boundary_rule_to_a_committed_set_edited_during_its_own_revert() {
    // A committed set is recovered (no-op) but then a hook edits the destination
    // during the restore that recovery does *not* run — here we drive it via a
    // crashed-restore journal state and let recover finish, with an edit at the
    // revert boundary supplied through the recover hooks.
    let fx = Fixture::new("r2-recover-boundary");
    fx.write_dest("data/a.big", b"old");
    let cs = single(&fx, "a", "data/a.big", Some(b"old"), b"new");
    let t = commit(&fx, &cs);
    // Simulate an interrupted restore: journal Restoring, dest still the replacement.
    let dir = fx.root.join(TXN_DIR).join(&t);
    let mut j = journal_of(&fx, &t);
    j.state = State::Restoring;
    j.write(&dir).unwrap();
    std::fs::write(fx.root.join(TXN_DIR).join(ACTIVE_MARKER), &t).unwrap();
    // recover continues the restore; a hook edits `a` at BeforeRevert.
    let mut hooks = EditAt::new(
        Event::BeforeRevert("a".into()),
        fx.root.join("data/a.big"),
        b"foreign during recovery",
    );
    let s = recover(&fx.root, &mut hooks).unwrap();
    assert_eq!(s[0].state, Some(State::Blocked), "{:?}", s[0]);
    assert_eq!(
        fx.dest_bytes("data/a.big").as_deref(),
        Some(&b"foreign during recovery"[..])
    );
    assert_eq!(active(&fx).as_deref(), Some(t.as_str()));
    // Idempotent: recover again, still blocked, bytes intact.
    let s2 = recover(&fx.root, &mut NoHooks).unwrap();
    assert_eq!(s2[0].state, Some(State::Blocked));
    assert_eq!(
        fx.dest_bytes("data/a.big").as_deref(),
        Some(&b"foreign during recovery"[..])
    );
}

// ------------------------------------------------------------------ apply parent-junction boundary

#[cfg(windows)]
#[test]
fn a_parent_junction_swapped_between_the_replacing_write_and_the_rename_is_refused() {
    let fx = Fixture::new("r2-apply-parent");
    let outside = fx.base.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("a"), b"outside-old").unwrap();
    std::fs::create_dir(fx.root.join("sub")).unwrap();
    std::fs::write(fx.root.join("sub/a"), b"old").unwrap();
    let cs = single(&fx, "a", "sub/a", Some(b"old"), b"new");

    // Swap `sub` for a junction to `outside` at the Replacing journal write —
    // after the earlier containment + identity checks, before the rename.
    struct SwapAtReplacing {
        sub: std::path::PathBuf,
        target: std::path::PathBuf,
        done: bool,
    }
    impl Hooks for SwapAtReplacing {
        fn event(&mut self, e: &Event) -> io::Result<()> {
            if !self.done && *e == Event::BeforeRename("a".into()) {
                self.done = true;
                std::fs::remove_dir_all(&self.sub).unwrap();
                let out = std::process::Command::new("cmd")
                    .args(["/C", "mklink", "/J"])
                    .arg(&self.sub)
                    .arg(&self.target)
                    .output()
                    .unwrap();
                assert!(out.status.success());
            }
            Ok(())
        }
    }
    let mut hooks = SwapAtReplacing {
        sub: fx.root.join("sub"),
        target: outside.clone(),
        done: false,
    };
    let mut t = prepare(&fx.root, &cs, &mut hooks).unwrap();
    let e = apply(&mut t, &cs, &mut hooks).unwrap_err();
    assert!(e.to_string().contains("not contained"), "{e}");
    // The mutation did not follow the junction into `outside`; the boundary
    // re-resolution refused it before the rename.
    assert_eq!(std::fs::read(outside.join("a")).unwrap(), b"outside-old");
    // `sub/a` itself was never replaced. The set is left for recovery (the
    // junction blocks resolution); removing it lets recovery finish cleanly.
    assert_eq!(active(&fx).as_deref(), Some(t.txn_id.as_str()));
    std::fs::remove_dir(fx.root.join("sub")).unwrap();
    std::fs::create_dir(fx.root.join("sub")).unwrap();
    std::fs::write(fx.root.join("sub/a"), b"old").unwrap();
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    let mine = s.iter().find(|x| x.txn_id == t.txn_id).unwrap();
    assert_eq!(mine.state, Some(State::RolledBack), "{mine:?}");
    assert_eq!(std::fs::read(fx.root.join("sub/a")).unwrap(), b"old");
    assert_eq!(std::fs::read(outside.join("a")).unwrap(), b"outside-old");
    assert_eq!(active(&fx), None);
}

#[test]
fn a_parent_swapped_to_a_sibling_directory_at_the_boundary_does_not_redirect_the_write() {
    // A non-junction redirect: `sub` is replaced by a different real directory
    // whose `a` holds foreign bytes, at the rename boundary. Re-resolution
    // succeeds (it is contained) but the identity re-check refuses it, so the
    // staged bytes never land on the wrong file.
    let fx = Fixture::new("r2-apply-sibling");
    std::fs::create_dir(fx.root.join("sub")).unwrap();
    std::fs::write(fx.root.join("sub/a"), b"old").unwrap();
    let cs = single(&fx, "a", "sub/a", Some(b"old"), b"new");
    struct SwapDir {
        sub: std::path::PathBuf,
        done: bool,
    }
    impl Hooks for SwapDir {
        fn event(&mut self, e: &Event) -> io::Result<()> {
            if !self.done && *e == Event::BeforeRename("a".into()) {
                self.done = true;
                std::fs::remove_dir_all(&self.sub).unwrap();
                std::fs::create_dir(&self.sub).unwrap();
                std::fs::write(self.sub.join("a"), b"a different file").unwrap();
            }
            Ok(())
        }
    }
    let mut hooks = SwapDir {
        sub: fx.root.join("sub"),
        done: false,
    };
    let mut t = prepare(&fx.root, &cs, &mut hooks).unwrap();
    let e = apply(&mut t, &cs, &mut hooks).unwrap_err();
    assert!(e.to_string().contains("expected"), "{e}");
    // The foreign file was neither overwritten with the staged bytes nor lost.
    assert_eq!(
        std::fs::read(fx.root.join("sub/a")).unwrap(),
        b"a different file"
    );
    // The rollback sees foreign bytes at the destination and blocks, preserving
    // them; the root stays marked until the conflict is resolved by hand.
    assert_eq!(journal_of(&fx, &t.txn_id).state, State::Blocked);
    assert_eq!(active(&fx).as_deref(), Some(t.txn_id.as_str()));
    assert_eq!(
        std::fs::read(fx.root.join("sub/a")).unwrap(),
        b"a different file"
    );
}

// ---------------------------------------------------------------- S2-C helper: abort

/// `abort` frees a root from its own prepared-but-never-applied set without
/// root-wide recovery: only from `Prepared`, only with a handle that matches
/// the persisted journal, never after mutation began.
#[test]
fn abort_discards_a_prepared_set_and_refuses_everything_else() {
    let fx = Fixture::new("abort");
    let old = b"old".to_vec();
    fx.write_dest("a", &old);
    let cs = single(&fx, "a", "a", Some(&old), b"new");
    let t = prepare(&fx.root, &cs, &mut NoHooks).unwrap();
    assert_eq!(active(&fx).as_deref(), Some(t.txn_id.as_str()));

    // A stale handle (state differs from disk) is refused.
    let mut stale = t.clone();
    stale.journal.state = State::Staged;
    assert!(matches!(
        crate::engine::abort(&stale, &mut NoHooks),
        Err(TxnError::StaleHandle { .. })
    ));

    let out = crate::engine::abort(&t, &mut NoHooks).unwrap();
    assert_eq!(out.state, State::Aborted);
    assert_eq!(journal_of(&fx, &t.txn_id).state, State::Aborted);
    assert_eq!(active(&fx), None, "the root is free again");
    assert_eq!(fx.dest_bytes("a").as_deref(), Some(old.as_slice()));
    // Idempotence is not offered: the handle is now stale.
    assert!(matches!(
        crate::engine::abort(&t, &mut NoHooks),
        Err(TxnError::StaleHandle { .. })
    ));
    // Neither can the aborted set be applied.
    let mut t2 = t.clone();
    assert!(apply(&mut t2, &cs, &mut NoHooks).is_err());

    // A committed set cannot be aborted (restore is the way back).
    let cs2 = single(&fx, "a", "a", Some(&old), b"newer");
    let mut c = prepare(&fx.root, &cs2, &mut NoHooks).unwrap();
    apply(&mut c, &cs2, &mut NoHooks).unwrap();
    assert!(matches!(
        crate::engine::abort(&c, &mut NoHooks),
        Err(TxnError::WrongState { .. })
    ));
    assert_eq!(fx.dest_bytes("a").as_deref(), Some(b"newer".as_slice()));
    let _ = restore(&fx.root, &c.txn_id, &mut NoHooks).unwrap();
    assert_eq!(fx.dest_bytes("a").as_deref(), Some(old.as_slice()));
    // The next prepare on the root succeeds: recovery sees only terminal sets.
    let s = recover(&fx.root, &mut NoHooks).unwrap();
    assert!(s.iter().all(|x| x.state.unwrap().is_terminal()));
    assert!(crate::engine::copied_target_root(&fx.root).is_ok());
    assert!(matches!(
        crate::engine::copied_target_root(&fx.root.join("nope")),
        Err(TxnError::Io { .. }) | Err(TxnError::NotCopiedTarget(_))
    ));
}
