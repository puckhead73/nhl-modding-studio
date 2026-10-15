//! S2-B: one change set coordinates an S2-A staged archive (fit + append), an
//! `EditSession` save re-parsed through the loader, and an originally absent
//! companion file, through the shared `tdb-txn` engine on a copied target.

use std::path::{Path, PathBuf};

use studio_domain::txn::{archive_artifact, companion_artifact, session_artifact, stage_archive};
use studio_domain::{EditSession, FieldValue, Platform, Project, ProjectRole};
use tdb_archive_big::stage::{
    plan, synth::build_archive, BoundSource, ExpectedDecoded, Placement, Replacement,
};
use tdb_archive_codec::compress_chunkzip;
use tdb_txn::engine::{apply, create_copied_target_root, prepare, recover, restore, NoHooks};
use tdb_txn::{ChangeSet, FileIdentity, Mode, State};

const DB: &str = "anchors/ps3/nhllegacy/databases/nhlng.db";
const META: &str = "anchors/ps3/nhllegacy/databases/nhlng-meta.xml";
const DEFS: &str = "defs";

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn noise(len: usize, seed: u64) -> Vec<u8> {
    let mut state = seed | 1;
    (0..len)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as u8
        })
        .collect()
}

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn archive_session_and_companion_apply_recover_and_restore_through_one_journal() {
    let base = std::env::temp_dir().join(format!("studio-txn-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let _tmp = Tmp(base.clone());
    let root = base.join("copy");
    create_copied_target_root(&root).unwrap();
    std::fs::create_dir_all(root.join("USRDIR/databases")).unwrap();
    let staging = base.join("staging");
    std::fs::create_dir_all(&staging).unwrap();

    // --- the copied target: a synthetic EB archive and the committed Legacy anchor DB.
    let plain0 = noise(3000, 1);
    let archive = build_archive(
        &["rendering/blocker", "fe/loc"],
        &[
            (0, "blocker_0_82_0_dm.rpsgl", compress_chunkzip(&plain0)),
            (1, "nhl_eng_us-meta.xml", b"<meta/>original".to_vec()),
        ],
    );
    let archive_dest = root.join("USRDIR/patch.big");
    std::fs::write(&archive_dest, &archive).unwrap();
    let db_dest = root.join("USRDIR/databases/nhlng.db");
    std::fs::copy(repo(DB), &db_dest).unwrap();
    let original_archive = FileIdentity::of_file(&archive_dest).unwrap();
    let original_db = FileIdentity::of_file(&db_dest).unwrap();

    // --- S2-A: fit (raw member shrinks) + append (chunkzip member grows).
    let bound = BoundSource::bind(&mut std::fs::File::open(&archive_dest).unwrap()).unwrap();
    let small = b"<meta/>".to_vec();
    let big = noise(20_000, 7);
    let p = plan(
        &bound,
        vec![
            Replacement {
                entry_index: 1,
                expected_path: bound.entries[1].path.clone(),
                expected_stored_sha256: bound.entries[1].stored_sha256,
                new_stored: small.clone(),
                expected_decoded: ExpectedDecoded::of(&small),
            },
            Replacement {
                entry_index: 0,
                expected_path: bound.entries[0].path.clone(),
                expected_stored_sha256: bound.entries[0].stored_sha256,
                new_stored: compress_chunkzip(&big),
                expected_decoded: ExpectedDecoded::of(&big),
            },
        ],
    )
    .unwrap();
    assert!(matches!(
        p.edits[0].placement,
        Placement::AppendAndRetarget { .. }
    ));
    assert!(matches!(p.edits[1].placement, Placement::FitInSlot { .. }));
    let staged_archive = staging.join("patch.big.staged");
    let staged = stage_archive(bound, p, &archive_dest, &staged_archive).unwrap();
    assert!(!staged.report().deployable(), "S2-A output is unqualified");
    let report = staged.report().clone();

    // --- the session: load the copied DB, make a real edit, package it.
    let project = Project::load(
        &db_dest,
        Platform::Ps3,
        ProjectRole::Nhlng,
        repo(META),
        repo(DEFS),
    )
    .unwrap();
    let mut session = EditSession::begin(project);
    session
        .set_field("ubPc", 0, "vjla", FieldValue::Int(2))
        .unwrap();
    if !session.is_dirty() {
        session
            .set_field("ubPc", 0, "vjla", FieldValue::Int(3))
            .unwrap();
    }
    assert!(session.is_dirty());

    // --- the change set.
    let a = archive_artifact("patch", "USRDIR/patch.big", &staged).unwrap();
    let d = session_artifact(
        "nhlng",
        "USRDIR/databases/nhlng.db",
        &db_dest,
        &session,
        &staging,
    )
    .unwrap();
    assert!(d.validation.detail["table_count"].parse::<usize>().unwrap() > 50);
    let c = companion_artifact(
        "sidecar",
        "USRDIR/databases/nhlng.db.studio.json",
        b"{\"schema\":\"studio-sidecar/1\",\"written_by\":\"S2-B test\"}\n",
        &staging,
        vec!["nhlng".into()],
    )
    .unwrap();
    assert!(a
        .validation
        .qualification
        .items
        .iter()
        .any(|i| i.subject == "native loader acceptance" && i.status == "unresolved"));
    let cs = ChangeSet::new("S2-B coordination", Mode::CopiedTarget, vec![c, a, d]);

    // --- prepare / apply.
    let mut txn = prepare(&root, &cs, &mut NoHooks).unwrap();
    assert!(txn.journal.applied_unqualified);
    let keys: Vec<_> = txn
        .journal
        .artifacts
        .iter()
        .map(|a| a.key.clone())
        .collect();
    let pos = |k: &str| keys.iter().position(|x| x == k).unwrap();
    assert!(
        pos("nhlng") < pos("sidecar"),
        "the companion is applied after the DB it depends on: {keys:?}"
    );
    assert_eq!(keys.len(), 3);
    let out = apply(&mut txn, &cs, &mut NoHooks).unwrap();
    assert_eq!(out.state, State::Committed);

    // --- reopen through the existing readers.
    let reopened = BoundSource::bind(&mut std::fs::File::open(&archive_dest).unwrap()).unwrap();
    assert_eq!(reopened.identity.len, report.new_eof);
    assert_eq!(reopened.entries[1].size as usize, small.len());
    assert!(
        reopened.entries[0].offset >= original_archive.len,
        "grown entry was appended"
    );
    let reloaded = Project::load(
        &db_dest,
        Platform::Ps3,
        ProjectRole::Nhlng,
        repo(META),
        repo(DEFS),
    )
    .unwrap();
    assert_eq!(
        reloaded.db_bytes(),
        session.bytes(),
        "the saved edit is what reloads"
    );
    assert_ne!(FileIdentity::of_file(&db_dest).unwrap(), original_db);
    assert!(root.join("USRDIR/databases/nhlng.db.studio.json").is_file());
    // The staged archive is byte-identical to the stager's output.
    assert_eq!(
        FileIdentity::of_file(&archive_dest).unwrap().sha256,
        tdb_archive_big::stage::hex(&report.output.sha256)
    );

    // --- recovery is a no-op on the committed set; restore puts the originals back.
    let s = recover(&root, &mut NoHooks).unwrap();
    assert_eq!(s[0].state, Some(State::Committed));
    let out = restore(&root, &txn.txn_id, &mut NoHooks).unwrap();
    assert_eq!(out.state, State::Restored);
    assert_eq!(
        FileIdentity::of_file(&archive_dest).unwrap(),
        original_archive
    );
    assert_eq!(FileIdentity::of_file(&db_dest).unwrap(), original_db);
    assert!(!root.join("USRDIR/databases/nhlng.db.studio.json").exists());
    assert_eq!(
        std::fs::read(&archive_dest).unwrap(),
        archive,
        "exact original bytes and EOF"
    );
}

#[test]
fn archive_artifact_reverifies_the_staged_output_against_its_plan() {
    let base = std::env::temp_dir().join(format!("studio-txn-recheck-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let _tmp = Tmp(base.clone());
    let archive = build_archive(&["d"], &[(0, "x.bin", b"stored".to_vec())]);
    let src = base.join("src.big");
    std::fs::write(&src, &archive).unwrap();
    let mk_plan = |bound: &BoundSource| {
        plan(
            bound,
            vec![Replacement {
                entry_index: 0,
                expected_path: "d/x.bin".into(),
                expected_stored_sha256: bound.entries[0].stored_sha256,
                new_stored: b"new".to_vec(),
                expected_decoded: ExpectedDecoded::of(b"new"),
            }],
        )
        .unwrap()
    };
    let bound = BoundSource::bind(&mut std::fs::File::open(&src).unwrap()).unwrap();
    let out = base.join("out.big");
    let staged = stage_archive(bound.clone(), mk_plan(&bound), &src, &out).unwrap();
    assert!(archive_artifact("k", "src.big", &staged).is_ok());

    // Staged output changed after staging: re-verification refuses.
    let mut t = std::fs::read(&out).unwrap();
    let last = t.len() - 1;
    t[last] ^= 1;
    std::fs::write(&out, &t).unwrap();
    let e = archive_artifact("k", "src.big", &staged).unwrap_err();
    assert!(e.to_string().contains("re-verification"), "{e}");
    t[last] ^= 1;
    std::fs::write(&out, &t).unwrap();
    assert!(
        archive_artifact("k", "src.big", &staged).is_ok(),
        "restored bytes verify again"
    );

    // Source changed after staging: the untouched-byte comparison refuses.
    let mut s2 = archive.clone();
    let last2 = s2.len() - 1;
    s2[last2] ^= 1;
    std::fs::write(&src, &s2).unwrap();
    let e = archive_artifact("k", "src.big", &staged).unwrap_err();
    assert!(e.to_string().contains("re-verification"), "{e}");
    std::fs::write(&src, &archive).unwrap();

    // An existing output path is refused by the stager; keys are validated.
    assert!(stage_archive(bound.clone(), mk_plan(&bound), &src, &out).is_err());
    let e = archive_artifact("../k", "src.big", &staged).unwrap_err();
    assert!(e.to_string().contains("plain name"), "{e}");
}

#[test]
fn session_artifact_binds_the_loaded_identity_not_the_current_disk_bytes() {
    let base = std::env::temp_dir().join(format!("studio-txn-session-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let _tmp = Tmp(base.clone());
    let db = base.join("nhlng.db");
    std::fs::copy(repo(DB), &db).unwrap();
    let staging = base.join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    let project = Project::load(
        &db,
        Platform::Ps3,
        ProjectRole::Nhlng,
        repo(META),
        repo(DEFS),
    )
    .unwrap();
    let loaded = project.loaded_identity().cloned().unwrap();
    assert_eq!(loaded, FileIdentity::of_file(&db).unwrap());
    let mut session = EditSession::begin(project);
    session
        .set_field("ubPc", 0, "vjla", FieldValue::Int(2))
        .unwrap();

    // The destination is edited after the session opened: refused, and the
    // expected prior would have been the loaded identity, not the new bytes.
    let mut edited = std::fs::read(&db).unwrap();
    edited[100] ^= 1;
    std::fs::write(&db, &edited).unwrap();
    let e = session_artifact("nhlng", "nhlng.db", &db, &session, &staging).unwrap_err();
    assert!(e.to_string().contains("changed since"), "{e}");
    assert!(
        !staging.join("nhlng.packaged").exists(),
        "nothing staged on a conflict"
    );

    // Put the loaded bytes back: the artifact binds to the loaded identity.
    std::fs::copy(repo(DB), &db).unwrap();
    let a = session_artifact("nhlng", "nhlng.db", &db, &session, &staging).unwrap();
    assert_eq!(a.expected_prior, tdb_txn::Expected::Present(loaded));
    assert!(a.validation.qualification.items[0].note.contains("ps3"));
    // A second staging with the same key never overwrites the first file.
    let e = session_artifact("nhlng", "nhlng.db", &db, &session, &staging).unwrap_err();
    assert!(
        e.to_string().contains("exists") || e.to_string().contains("io error"),
        "{e}"
    );
    // A project built from bytes has no baseline and is refused.
    for bad in ["../x", "a/b", "a\\b", ".hidden", ""] {
        let e = companion_artifact(bad, "c.txt", b"x", &staging, vec![]).unwrap_err();
        assert!(e.to_string().contains("plain name"), "{bad:?}: {e}");
    }
    assert!(companion_artifact("c", "c.txt", b"x", &staging, vec![]).is_ok());
    assert!(
        companion_artifact("c", "c.txt", b"y", &staging, vec![]).is_err(),
        "no overwrite"
    );
}
