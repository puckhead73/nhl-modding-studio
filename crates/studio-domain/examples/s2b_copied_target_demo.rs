//! `s2b_copied_target_demo` — package S2-B: one coordinated change set on a
//! **new** copied target: an EB archive replaced through the S2-A stager
//! (one fit-in-slot and one append-and-retarget edit), a database re-saved
//! through `EditSession`, and an originally absent companion file — applied,
//! reopened/verified, and restored to the exact original hashes and lengths
//! through the shared `tdb-txn` journal.
//!
//! ```sh
//! cargo run -p studio-domain --example s2b_copied_target_demo -- \
//!     --copy-root <new dir> --archive <patch.big> --db <nhlng.db> \
//!     --meta <nhlng-meta.xml> --defs <defs dir> [--keep]
//! ```
//!
//! The copy root must not exist; the source files are only read. Every
//! artifact is **unqualified** (S2-A's items are carried, the session and
//! companion carry their own); copied-target mode records that. Nothing here
//! is evidence that a game would accept the result.

use std::path::PathBuf;

use studio_domain::txn::{archive_artifact, companion_artifact, session_artifact, stage_archive};
use studio_domain::{EditSession, FieldValue, Platform, Project, ProjectRole};
use tdb_archive_big::eb::{decode_payload, Payload};
use tdb_archive_big::stage::{plan, BoundSource, ExpectedDecoded, Replacement};
use tdb_archive_codec::compress_chunkzip;
use tdb_txn::engine::{
    apply, create_copied_target_root, prepare, recover, restore, status, Event, Hooks,
};
use tdb_txn::{ChangeSet, FileIdentity, Mode};

struct Print;
impl Hooks for Print {
    fn event(&mut self, e: &Event) -> std::io::Result<()> {
        match e {
            Event::BeforeJournal(_) | Event::BeforeReplace(_) | Event::BeforeRevert(_) => {}
            other => println!("    [txn] {other:?}"),
        }
        Ok(())
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut root, mut archive, mut db, mut meta, mut defs, mut keep) =
        (None, None, None, None, None, false);
    while let Some(a) = args.next() {
        let mut v = || PathBuf::from(args.next().expect("value"));
        match a.as_str() {
            "--copy-root" => root = Some(v()),
            "--archive" => archive = Some(v()),
            "--db" => db = Some(v()),
            "--meta" => meta = Some(v()),
            "--defs" => defs = Some(v()),
            "--keep" => keep = true,
            other => panic!("unknown argument {other}"),
        }
    }
    let root = root.expect("--copy-root");
    let (archive, db, meta, defs) = (
        archive.expect("--archive"),
        db.expect("--db"),
        meta.expect("--meta"),
        defs.expect("--defs"),
    );

    // 1. A new copied target: the sources are copied in, never touched again.
    create_copied_target_root(&root).expect("create copied-target root (must not exist)");
    std::fs::create_dir_all(root.join("USRDIR/databases")).unwrap();
    let archive_dest = root.join("USRDIR/patch.big");
    let db_dest = root.join("USRDIR/databases/nhlng.db");
    std::fs::copy(&archive, &archive_dest).unwrap();
    std::fs::copy(&db, &db_dest).unwrap();
    let staging = root.parent().unwrap().join(format!(
        "{}-staging",
        root.file_name().unwrap().to_string_lossy()
    ));
    std::fs::create_dir_all(&staging).unwrap();
    let orig_archive = FileIdentity::of_file(&archive_dest).unwrap();
    let orig_db = FileIdentity::of_file(&db_dest).unwrap();
    println!("copied target: {}", root.display());
    println!("  USRDIR/patch.big            {orig_archive}");
    println!("  USRDIR/databases/nhlng.db   {orig_db}");
    println!(
        "  source archive sha256 (read-only): {}",
        FileIdentity::of_file(&archive).unwrap().sha256
    );

    // 2. S2-A: fit (entry 139 recompressed, smaller) + append (entry 4 grown raw).
    let bound = BoundSource::bind(&mut std::fs::File::open(&archive_dest).unwrap()).unwrap();
    let pick = |i: usize| -> (usize, Vec<u8>) {
        let e = &bound.entries[i];
        let mut f = std::fs::File::open(&archive_dest).unwrap();
        use std::io::{Read, Seek, SeekFrom};
        f.seek(SeekFrom::Start(e.offset)).unwrap();
        let mut v = vec![0u8; e.size as usize];
        f.read_exact(&mut v).unwrap();
        match decode_payload(v).unwrap() {
            Payload::Data(d) => (i, d),
            Payload::UnsupportedLzx => panic!("entry {i} is chunklzx"),
        }
    };
    let (i_fit, plain_fit) = pick(139);
    let (i_grow, plain_grow) = pick(4);
    let mut grown = plain_grow.clone();
    grown.extend(std::iter::repeat_n(b' ', 4096));
    let reqs = vec![
        Replacement {
            entry_index: i_fit,
            expected_path: bound.entries[i_fit].path.clone(),
            expected_stored_sha256: bound.entries[i_fit].stored_sha256,
            new_stored: compress_chunkzip(&plain_fit),
            expected_decoded: ExpectedDecoded::of(&plain_fit),
        },
        Replacement {
            entry_index: i_grow,
            expected_path: bound.entries[i_grow].path.clone(),
            expected_stored_sha256: bound.entries[i_grow].stored_sha256,
            new_stored: grown.clone(),
            expected_decoded: ExpectedDecoded::of(&grown),
        },
    ];
    let p = plan(&bound, reqs).unwrap();
    for e in &p.edits {
        println!(
            "  S2-A edit {} `{}`: {:?}",
            e.entry_index, e.path, e.placement
        );
    }
    let staged_archive = staging.join("patch.big.staged");
    let staged = stage_archive(bound, p, &archive_dest, &staged_archive).unwrap();
    let report = staged.report().clone();
    println!(
        "  S2-A staged {} -> {} bytes, {:?}, deployable {}",
        report.original_eof,
        report.new_eof,
        report.qualification.state,
        report.deployable()
    );

    // 3. The session: a real edit, packaged through the existing save path.
    let project = Project::load(&db_dest, Platform::Ps3, ProjectRole::Nhlng, &meta, &defs).unwrap();
    let mut session = EditSession::begin(project);
    session
        .set_field("ubPc", 0, "vjla", FieldValue::Int(2))
        .unwrap();
    if !session.is_dirty() {
        session
            .set_field("ubPc", 0, "vjla", FieldValue::Int(3))
            .unwrap();
    }
    println!(
        "  session: edited ubPc[0].vjla, dirty = {}",
        session.is_dirty()
    );

    // 4. The change set.
    let a = archive_artifact("patch", "USRDIR/patch.big", &staged).unwrap();
    let d = session_artifact(
        "nhlng",
        "USRDIR/databases/nhlng.db",
        &db_dest,
        &session,
        &staging,
    )
    .unwrap();
    let c = companion_artifact(
        "sidecar",
        "USRDIR/databases/nhlng.db.studio.json",
        b"{\"schema\":\"studio-sidecar/1\",\"note\":\"S2-B demo companion\"}\n",
        &staging,
        vec!["nhlng".into()],
    )
    .unwrap();
    let cs = ChangeSet::new("S2-B copied-target demo", Mode::CopiedTarget, vec![a, d, c]);
    println!("change set {} ({} artifacts)", cs.id, cs.artifacts.len());
    for art in &cs.artifacts {
        println!(
            "  `{}` -> {} [{}; qualification {}; {} unresolved item(s)]",
            art.key,
            art.destination,
            art.validation.producer,
            art.validation.qualification.state,
            art.validation
                .qualification
                .items
                .iter()
                .filter(|i| i.status != "resolved")
                .count()
        );
    }

    // 5. Prepare / apply.
    let mut txn = prepare(&root, &cs, &mut Print).unwrap();
    println!(
        "prepared {} (applied_unqualified = {})",
        txn.txn_id, txn.journal.applied_unqualified
    );
    let out = apply(&mut txn, &cs, &mut Print).unwrap();
    println!("apply -> {:?}", out.state);
    for (k, id) in &out.destinations {
        println!(
            "  {k}: {}",
            id.as_ref()
                .map(|i| i.to_string())
                .unwrap_or("absent".into())
        );
    }
    println!("journal log:");
    for l in &txn.journal.log {
        println!("  {l}");
    }

    // 6. Reopen through the existing readers.
    let re = BoundSource::bind(&mut std::fs::File::open(&archive_dest).unwrap()).unwrap();
    println!(
        "reopened archive: {} bytes, entry {} now at {} ({} bytes), entry {} at {} ({} bytes)",
        re.identity.len,
        i_fit,
        re.entries[i_fit].offset,
        re.entries[i_fit].size,
        i_grow,
        re.entries[i_grow].offset,
        re.entries[i_grow].size
    );
    let reloaded =
        Project::load(&db_dest, Platform::Ps3, ProjectRole::Nhlng, &meta, &defs).unwrap();
    println!(
        "reloaded db: {} inner bytes, equals the session's working bytes: {}",
        reloaded.db_bytes().len(),
        reloaded.db_bytes() == session.bytes()
    );
    println!(
        "companion present: {}",
        root.join("USRDIR/databases/nhlng.db.studio.json").is_file()
    );
    let s = recover(&root, &mut Print).unwrap();
    println!(
        "startup recovery on the committed set: {:?}",
        s.iter()
            .map(|s| (s.txn_id.clone(), s.state))
            .collect::<Vec<_>>()
    );

    // 7. Restore and verify the exact originals.
    let out = restore(&root, &txn.txn_id, &mut Print).unwrap();
    println!("restore -> {:?}", out.state);
    let now_archive = FileIdentity::of_file(&archive_dest).unwrap();
    let now_db = FileIdentity::of_file(&db_dest).unwrap();
    println!(
        "  patch.big  {now_archive}  original: {}",
        now_archive == orig_archive
    );
    println!("  nhlng.db   {now_db}  original: {}", now_db == orig_db);
    println!(
        "  companion removed: {}",
        !root.join("USRDIR/databases/nhlng.db.studio.json").exists()
    );
    println!(
        "transactions under the root: {:?}",
        status(&root)
            .unwrap()
            .iter()
            .map(|s| (s.txn_id.clone(), s.state))
            .collect::<Vec<_>>()
    );
    if !keep {
        let _ = std::fs::remove_dir_all(&staging);
        println!(
            "(copy root retained at {}; staging removed)",
            root.display()
        );
    }
}
