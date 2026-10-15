//! O4 / L1.1-E checkpoint: a league identity edit runs the full direct-editing loop on a copied
//! NHL 21 fixture — edit → pending review → copied-target review → apply → reopen → restore —
//! plus the changed-after-review refusal and the discard path. Everything stays in a temp copied
//! root; the anchor is read only.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};
use studio_domain::FieldValue;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn s(p: &Path) -> String {
    p.to_str().unwrap().to_string()
}

fn sha(path: &Path) -> String {
    studio_domain::tdb_txn::FileIdentity::of_file(path)
        .unwrap()
        .sha256
}

const NHL21: &str = "anchors/ps4/nhl21/databases/nhl_game.db";
const META: &str = "anchors/ps4/nhl21/databases/nhl_game-meta.xml";
const ANCHOR_SHA: &str = "90ee1ec49d36dbb0f6d9cefe75e259d9661e3aa05338d03347b4842e33c8e9f9";

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("nhlms-league-ct-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A marked copied root holding the NHL 21 main DB at its PS4 archive-relative path.
fn copied_tree(base: &Path) -> (PathBuf, PathBuf) {
    let root = base.join("copy");
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let dir = root.join("nhl/data/common/gamemodes");
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("nhl_game.db");
    std::fs::copy(repo(NHL21), &db).unwrap();
    (root, db)
}

fn open(reg: &Registry, db: &Path) -> u32 {
    reg.open_workspace_from_paths(
        Some("nhl21"),
        parse_platform("ps4").unwrap(),
        &s(db),
        None,
        &s(&repo(META)),
        &s(&repo("defs")),
    )
    .unwrap()
    .project_id
}

#[test]
fn league_edit_review_apply_reopen_restore() {
    let tmp = Tmp::new("loop");
    let (root, db) = copied_tree(&tmp.0);
    assert_eq!(sha(&db), ANCHOR_SHA);
    let reg = Registry::default();
    let id = open(&reg, &db);

    // The project is recognised as living inside the marked root, reported as a plain path (the
    // engine's canonical `\\?\` spelling is a display concern the API strips — LEAGUE-SMOKE-1).
    let found = reg.copied_target_root_for(id).unwrap();
    let canonical = studio_domain::tdb_txn::engine::copied_target_root(&root)
        .unwrap()
        .display()
        .to_string();
    let expected = canonical
        .strip_prefix(r"\\?\")
        .map(str::to_owned)
        .unwrap_or(canonical);
    assert_eq!(
        found.root.as_deref().map(|r| r.to_lowercase()),
        Some(expected.to_lowercase())
    );

    // Edit: re-point Allsvenskan (league 8) at the existing key TXT_ROW_LEAGUE.
    reg.set_field(
        id,
        "fsLK",
        8,
        "vIys",
        FieldValue::Text("TXT_ROW_LEAGUE".into()),
    )
    .unwrap();
    let pending = reg.pending_changes(id, 100).unwrap();
    assert!(pending.dirty && pending.roster.is_none());
    assert_eq!(pending.primary.total_cells, 1);
    assert_eq!(pending.primary.cells[0].entity, "TXT_ROW_LEAGUE");
    assert_eq!(pending.primary.cells[0].before, "Allsvenskan");

    // Review → apply.
    let review = reg.copied_target_review(id, &s(&root)).unwrap();
    assert_eq!(review.mode, "copied-target");
    assert!(!review.live_install_allowed);
    assert_eq!(review.artifacts.len(), 1);
    assert_eq!(
        review.artifacts[0].prior.as_ref().unwrap().sha256,
        ANCHOR_SHA
    );
    let applied = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert!(applied.applied_unqualified);
    assert!(
        !applied.project.dirty,
        "the session adopted the installed file"
    );
    assert!(!reg.summary(id).unwrap().dirty);
    assert_eq!(reg.pending_changes(id, 100).unwrap().primary.total_cells, 0);
    let after_sha = sha(&db);
    assert_ne!(after_sha, ANCHOR_SHA);
    assert_eq!(
        applied.destinations[0].identity.as_ref().unwrap().sha256,
        after_sha
    );

    // Allowed-difference audit on the installed file: the edited field, and nothing else in any
    // record; derived integrity bytes may differ.
    let before = std::fs::read(repo(NHL21)).unwrap();
    let after = std::fs::read(&db).unwrap();
    assert_eq!(before.len(), after.len());
    let bf = tdb_core::TdbFile::parse(&before).unwrap();
    let af = tdb_core::TdbFile::parse(&after).unwrap();
    let mut changed_records = Vec::new();
    for t in &af.tables {
        let bt = bf.table(&t.id.raw()).unwrap();
        let rl = t.record_len_bytes as usize;
        for rec in 0..t.current_records as usize {
            let a = &after[t.record_block + rec * rl..t.record_block + (rec + 1) * rl];
            let b = &before[bt.record_block + rec * rl..bt.record_block + (rec + 1) * rl];
            if a != b {
                changed_records.push((t.id.canonical(af.endian), rec));
            }
        }
    }
    assert_eq!(changed_records, vec![("fsLK".to_string(), 8usize)]);
    assert!(af.header_crc_ok() && af.size_ok());

    // Reopen the installed file in a fresh registry: the edit persisted.
    let reg2 = Registry::default();
    let id2 = open(&reg2, &db);
    assert_eq!(
        reg2.league_view(id2, 8)
            .unwrap()
            .unwrap()
            .summary
            .name_key
            .as_deref(),
        Some("TXT_ROW_LEAGUE")
    );
    drop(reg2);

    // Restore the exact original through the transaction; the open session is refreshed.
    let status = reg.copied_target_status(&s(&root)).unwrap();
    let txn = status.transactions.iter().find(|t| t.restorable).unwrap();
    let restored = reg.copied_target_restore(&s(&root), &txn.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(sha(&db), ANCHOR_SHA, "byte-exact restore");
    assert_eq!(
        reg.league_view(id, 8)
            .unwrap()
            .unwrap()
            .summary
            .name_key
            .as_deref(),
        Some("Allsvenskan"),
        "the clean session reloaded the restored file"
    );
    assert!(!reg.summary(id).unwrap().dirty);
}

#[test]
fn edit_after_review_is_refused_and_discard_frees_the_root() {
    let tmp = Tmp::new("refuse");
    let (root, db) = copied_tree(&tmp.0);
    let reg = Registry::default();
    let id = open(&reg, &db);
    reg.set_field(
        id,
        "fsLK",
        8,
        "vIys",
        FieldValue::Text("TXT_ROW_LEAGUE".into()),
    )
    .unwrap();
    let review = reg.copied_target_review(id, &s(&root)).unwrap();
    // Edit after review: the content revision moved, so apply must refuse without touching disk.
    reg.set_field(id, "fsLK", 8, "XMjH", FieldValue::Int(3))
        .unwrap();
    let err = match reg.copied_target_apply(id, review.handle) {
        Err(e) => e.to_string(),
        Ok(_) => panic!("apply must refuse an edited workspace"),
    };
    assert!(err.contains("revision") || err.contains("edited"), "{err}");
    assert_eq!(sha(&db), ANCHOR_SHA, "destination untouched");
    assert!(reg.summary(id).unwrap().dirty, "edits kept");
    let status = reg.copied_target_status(&s(&root)).unwrap();
    assert!(
        status.apply_allowed,
        "the refused set was aborted and the root is free: {}",
        status.guidance
    );

    // A fresh review can be discarded explicitly.
    let review = reg.copied_target_review(id, &s(&root)).unwrap();
    let status = reg.copied_target_status(&s(&root)).unwrap();
    assert_eq!(status.pending_handles.len(), 1);
    let status = reg.copied_target_discard(review.handle).unwrap();
    assert!(status.pending_handles.is_empty() && status.apply_allowed);
    // A stale handle is refused.
    assert!(reg.copied_target_apply(id, review.handle).is_err());

    // Creating a root refuses an existing path and never marks it.
    let err = reg
        .copied_target_create_root(&s(&tmp.0))
        .err()
        .map(|e| e.to_string())
        .unwrap();
    assert!(err.contains("already exists"), "{err}");
    let fresh = tmp.0.join("fresh-root");
    let created = reg.copied_target_create_root(&s(&fresh)).unwrap();
    assert!(Path::new(&created)
        .join(studio_domain::tdb_txn::engine::COPIED_TARGET_MARKER)
        .is_file());

    // LEAGUE-SMOKE-1: the root strings the panel shows and echoes back are plain paths — never
    // the canonicalised `\\?\` verbatim spelling — and they round-trip through the status query.
    let shown = reg.copied_target_root_for(id).unwrap().root.unwrap();
    for root in [&shown, &created] {
        assert!(!root.starts_with(r"\\?\"), "verbatim prefix shown: {root}");
        let status = reg.copied_target_status(root).unwrap();
        assert_eq!(&status.root, root);
    }
}

/// Build a minimal PS4-shape (20-byte row) `EB\0\3` archive holding `files` as raw members, with
/// the measured row facts: `+4` zero, `+12` an opaque ascending key, `+16` the djb2 path hash.
fn build_ps4_archive(dirs: &[&str], files: &[(u16, &str, Vec<u8>)]) -> Vec<u8> {
    use studio_domain::tdb_archive_big::stage::path_hash;
    let row_len = 20usize;
    let n = files.len();
    let name_slot = files.iter().map(|(_, f, _)| f.len() + 3).max().unwrap();
    let dir_slot = dirs.iter().map(|d| d.len() + 1).max().unwrap();
    let name_off = 0x30 + n * row_len + 16;
    let name_end = name_off + n * name_slot;
    let dir_start = (name_end + 15) & !15;
    let table_end = dir_start + dirs.len() * dir_slot;
    let data_start = (table_end + 15) & !15;
    let mut offsets = Vec::new();
    let mut cursor = data_start as u64;
    for (_, _, body) in files {
        offsets.push(cursor);
        cursor = (cursor + body.len() as u64 + 15) & !15;
    }
    let mut out = vec![0u8; cursor as usize];
    out[0..4].copy_from_slice(b"EB\0\x03");
    out[4..8].copy_from_slice(&(n as u32).to_be_bytes());
    out[8..12].copy_from_slice(&0x0011_0400u32.to_be_bytes());
    out[12..16].copy_from_slice(&(name_off as u32).to_be_bytes());
    out[0x14] = name_slot as u8;
    out[0x15] = dir_slot as u8;
    out[0x16..0x18].copy_from_slice(&(dirs.len() as u16).to_be_bytes());
    out[0x18..0x20].copy_from_slice(&cursor.to_be_bytes());
    for (i, (di, name, body)) in files.iter().enumerate() {
        let e = 0x30 + i * row_len;
        out[e..e + 4].copy_from_slice(&((offsets[i] >> 4) as u32).to_be_bytes());
        out[e + 8..e + 12].copy_from_slice(&(body.len() as u32).to_be_bytes());
        out[e + 12..e + 16].copy_from_slice(&(0x0100_0000u32 + i as u32).to_be_bytes());
        let path = format!("{}/{}", dirs[*di as usize], name);
        out[e + 16..e + 20].copy_from_slice(&path_hash(&path).to_be_bytes());
        let o = name_off + i * name_slot;
        out[o..o + 2].copy_from_slice(&di.to_be_bytes());
        out[o + 2..o + 2 + name.len()].copy_from_slice(name.as_bytes());
        let p = offsets[i] as usize;
        out[p..p + body.len()].copy_from_slice(body);
    }
    for (i, d) in dirs.iter().enumerate() {
        let o = dir_start + i * dir_slot;
        out[o..o + d.len()].copy_from_slice(d.as_bytes());
    }
    out
}

/// O7: the edited NHL 21 database also replaces the raw `nhl_game.db` member of a copied
/// PS4-shape archive in the same transaction; the other member is untouched; restore is exact.
#[test]
fn league_edit_also_replaces_the_archive_member() {
    let tmp = Tmp::new("archive");
    let (root, db) = copied_tree(&tmp.0);
    let anchor = std::fs::read(repo(NHL21)).unwrap();
    let archive = build_ps4_archive(
        &[
            "nhl/data/common/gamemodes/database/t3db",
            "nhl/data/kettle/loc",
        ],
        &[
            (1, "eng_us-meta.xml", b"<meta/>".to_vec()),
            (0, "nhl_game.db", anchor.clone()),
        ],
    );
    let archive_rel = "databigs/inxp_00.big";
    let archive_path = root.join(archive_rel);
    std::fs::create_dir_all(archive_path.parent().unwrap()).unwrap();
    std::fs::write(&archive_path, &archive).unwrap();
    let archive_sha = sha(&archive_path);

    let reg = Registry::default();
    let id = open(&reg, &db);
    reg.set_field(
        id,
        "fsLK",
        8,
        "vIys",
        FieldValue::Text("TXT_ROW_LEAGUE".into()),
    )
    .unwrap();
    let member = "nhl/data/common/gamemodes/database/t3db/nhl_game.db";
    let review = reg
        .copied_target_review_archive_member(id, &s(&root), archive_rel, member)
        .unwrap();
    let kinds: Vec<(&str, &str)> = review
        .artifacts
        .iter()
        .map(|a| (a.key.as_str(), a.kind.as_str()))
        .collect();
    assert_eq!(kinds, vec![("primary", "database"), ("archive", "archive")]);
    assert_eq!(review.qualification_state, "unqualified");
    let arch = &review.artifacts[1];
    assert_eq!(arch.prior.as_ref().unwrap().sha256, archive_sha);
    assert!(arch
        .qualification_items
        .iter()
        .any(|i| i.subject == "row +12 word (PS4 sort key)" && i.status == "unresolved"));

    let applied = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert_eq!(applied.destinations.len(), 2);
    // The archive member now holds exactly the loose file's bytes; the other member is unchanged.
    let packaged = std::fs::read(&db).unwrap();
    let out = std::fs::read(&archive_path).unwrap();
    assert_eq!(
        out.len(),
        archive.len(),
        "same-length replacement fits in place"
    );
    let bound = studio_domain::tdb_archive_big::stage::BoundSource::bind(
        &mut std::io::Cursor::new(out.clone()),
    )
    .unwrap();
    let e = bound.entries.iter().find(|e| e.path == member).unwrap();
    assert_eq!(
        &out[e.offset as usize..e.offset as usize + e.size as usize],
        &packaged[..]
    );
    let m = bound
        .entries
        .iter()
        .find(|e| e.path.ends_with("eng_us-meta.xml"))
        .unwrap();
    assert_eq!(
        &out[m.offset as usize..m.offset as usize + m.size as usize],
        b"<meta/>"
    );
    assert!(bound.layout.path_hash_verified);

    // Refusals: a missing member, an archive outside the root.
    reg.set_field(id, "fsLK", 8, "XMjH", FieldValue::Int(3))
        .unwrap();
    assert!(reg
        .copied_target_review_archive_member(id, &s(&root), archive_rel, "nhl/nope.db")
        .is_err());
    assert!(reg
        .copied_target_review_archive_member(id, &s(&root), "../outside.big", member)
        .is_err());
    assert!(reg.undo(id).unwrap());

    // Exact restore of both destinations.
    let txn = applied.txn_id.clone();
    let restored = reg.copied_target_restore(&s(&root), &txn).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(sha(&archive_path), archive_sha);
    assert_eq!(sha(&db), ANCHOR_SHA);
}

/// LEAGUE-R1 F2: a newly dangling league reference is refused at the generic edit seam without
/// touching working bytes, pending state or the destination; valid references and pre-existing
/// violations are not over-blocked.
#[test]
fn dangling_league_reference_is_refused_before_it_can_be_saved() {
    let tmp = Tmp::new("dangling");
    let (_root, db) = copied_tree(&tmp.0);
    let reg = Registry::default();
    let id = open(&reg, &db);
    let before = reg.summary(id).unwrap();
    assert!(!reg
        .list_leagues(id)
        .unwrap()
        .leagues
        .iter()
        .any(|l| l.code == 31));

    // Team 0 (Anaheim) -> league 31: the 5-bit field allows it, no stockleagues row backs it.
    let err = match reg.set_field(id, "kOtt", 0, "xMjj", FieldValue::Int(31)) {
        Err(e) => e.to_string(),
        Ok(_) => panic!("a dangling league reference must be refused"),
    };
    assert!(err.contains("refers to no stockleagues row"), "{err}");
    assert!(
        err.contains("Team Anaheim") || err.contains("Team ANA"),
        "{err}"
    );
    let after = reg.summary(id).unwrap();
    assert!(
        !after.dirty && after.undo_depth == before.undo_depth,
        "nothing staged"
    );
    assert_eq!(after.workspace_revision, before.workspace_revision);
    assert_eq!(reg.pending_changes(id, 10).unwrap().primary.total_cells, 0);
    assert_eq!(sha(&db), ANCHOR_SHA);

    // Valid references are fine: an existing row without teams (30 = HUT) and back again.
    reg.set_field(id, "kOtt", 0, "xMjj", FieldValue::Int(30))
        .unwrap();
    assert_eq!(reg.list_teams(id).unwrap()[0].league, Some(30));
    reg.set_field(id, "kOtt", 0, "xMjj", FieldValue::Int(0))
        .unwrap();

    // Pre-existing violation: a database that already carries a dangling code (written directly
    // into the copied file) still opens, browses the raw code, and accepts unrelated edits and an
    // edit that does not add a *new* dangling reference.
    let original = std::fs::read(&db).unwrap();
    let mut bytes = original.clone();
    {
        let parsed = tdb_core::TdbFile::parse(&original).unwrap();
        let kott = parsed.table_canonical("kOtt").unwrap().clone();
        let f = kott
            .fields
            .iter()
            .find(|f| f.id.canonical(parsed.endian) == "xMjj")
            .unwrap()
            .clone();
        let rec = 5u32;
        let start = kott.record_block + rec as usize * kott.record_len_bytes as usize;
        let end = start + kott.record_len_bytes as usize;
        tdb_core::write_bits(
            &mut bytes[start..end],
            f.bit_offset,
            f.bit_width,
            31,
            parsed.endian,
        );
        parsed.recompute_crcs(&mut bytes);
    }
    let corrupt = tmp
        .0
        .join("copy/nhl/data/common/gamemodes/nhl_game.corrupt.db");
    std::fs::write(&corrupt, &bytes).unwrap();
    let id2 = open(&reg, &corrupt);
    let catalog = reg.list_leagues(id2).unwrap();
    let raw = catalog
        .leagues
        .iter()
        .find(|l| l.code == 31)
        .expect("the raw code is browsable");
    assert!(raw.record.is_none() && raw.label.is_none() && raw.member_teams == 1);
    assert_eq!(raw.display, "League code 31");
    // Unrelated edit on another team is accepted; the pre-existing violation is not repaired.
    reg.set_field(id2, "kOtt", 1, "xsnn", FieldValue::Text("XYZ".into()))
        .unwrap();
    assert_eq!(reg.list_teams(id2).unwrap()[5].league, Some(31));
    // Editing another field of the already-dangling team leaves its violation in place (not
    // repaired, not blocked); moving it to a valid code is accepted; re-introducing the dangling
    // code afterwards is a *new* violation and is refused like any other.
    reg.set_field(id2, "kOtt", 5, "xsnn", FieldValue::Text("QQQ".into()))
        .unwrap();
    assert_eq!(reg.list_teams(id2).unwrap()[5].league, Some(31));
    reg.set_field(id2, "kOtt", 5, "xMjj", FieldValue::Int(29))
        .unwrap();
    assert!(reg
        .set_field(id2, "kOtt", 5, "xMjj", FieldValue::Int(31))
        .is_err());
    assert!(reg.undo(id2).unwrap(), "back to the pre-existing state");
    // A second team dangling is one more: refused.
    assert!(reg
        .set_field(id2, "kOtt", 6, "xMjj", FieldValue::Int(31))
        .is_err());
}

/// LEAGUE-R1 F2 (gen7): the rule is table-gated - a Legacy team's league code is a vocabulary
/// code, and any in-range code is still accepted (no PS4 row set is imposed).
#[test]
fn gen7_league_codes_are_not_subject_to_the_ps4_rule() {
    let reg = Registry::default();
    let tmp = Tmp::new("gen7");
    let src = repo("anchors/ps3/nhllegacy/databases/nhlng.db");
    let db = tmp.0.join("nhlng.db");
    std::fs::copy(&src, &db).unwrap();
    let id = reg
        .open_workspace_from_paths(
            Some("nhllegacy"),
            parse_platform("ps3").unwrap(),
            &s(&db),
            None,
            &s(&repo("anchors/ps3/nhllegacy/databases/nhlng-meta.xml")),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    reg.set_field(id, "kOtt", 0, "xMjj", FieldValue::Int(15))
        .unwrap();
    reg.set_field(id, "kOtt", 0, "xMjj", FieldValue::Int(0))
        .unwrap();
    assert!(!reg.summary(id).unwrap().dirty);
}

/// LEAGUE-R1 F1: the archive member is derived from the plan's own staged primary artifact, so
/// the loose destination and the decoded member are byte-identical for the accepted revision,
/// and an edit after review is refused before any write.
#[test]
fn archive_member_bytes_come_from_the_plan_snapshot() {
    let tmp = Tmp::new("snapshot");
    let (root, db) = copied_tree(&tmp.0);
    let anchor = std::fs::read(repo(NHL21)).unwrap();
    let archive = build_ps4_archive(
        &["nhl/data/common/gamemodes/database/t3db"],
        &[(0, "nhl_game.db", anchor.clone())],
    );
    let archive_rel = "databigs/inxp_00.big";
    let archive_path = root.join(archive_rel);
    std::fs::create_dir_all(archive_path.parent().unwrap()).unwrap();
    std::fs::write(&archive_path, &archive).unwrap();
    let member = "nhl/data/common/gamemodes/database/t3db/nhl_game.db";
    let reg = Registry::default();
    let id = open(&reg, &db);

    // Revision A: one edit, reviewed with the archive. Then revision B: an edit after review;
    // the apply refuses on the revision check and neither destination is written.
    reg.set_field(
        id,
        "fsLK",
        8,
        "vIys",
        FieldValue::Text("TXT_ROW_LEAGUE".into()),
    )
    .unwrap();
    let rev_a = reg.summary(id).unwrap().workspace_revision;
    let review = reg
        .copied_target_review_archive_member(id, &s(&root), archive_rel, member)
        .unwrap();
    assert_eq!(review.workspace_revision, rev_a);
    let primary_a = review
        .artifacts
        .iter()
        .find(|a| a.key == "primary")
        .unwrap()
        .output
        .sha256
        .clone();
    reg.set_field(id, "fsLK", 8, "XMjH", FieldValue::Int(3))
        .unwrap();
    assert_ne!(reg.summary(id).unwrap().workspace_revision, rev_a);
    assert!(reg.copied_target_apply(id, review.handle).is_err());
    assert_eq!(sha(&db), ANCHOR_SHA);
    assert_eq!(std::fs::read(&archive_path).unwrap(), archive);
    assert!(reg.copied_target_status(&s(&root)).unwrap().apply_allowed);

    // Revision B reviewed and applied: the decoded member equals the loose destination equals
    // the plan's primary output identity - one snapshot, two destinations.
    let review = reg
        .copied_target_review_archive_member(id, &s(&root), archive_rel, member)
        .unwrap();
    let primary_b = review
        .artifacts
        .iter()
        .find(|a| a.key == "primary")
        .unwrap()
        .output
        .sha256
        .clone();
    assert_ne!(primary_b, primary_a, "revision B packages differently");
    let applied = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let loose = std::fs::read(&db).unwrap();
    assert_eq!(sha(&db), primary_b);
    let out = std::fs::read(&archive_path).unwrap();
    let bound = studio_domain::tdb_archive_big::stage::BoundSource::bind(
        &mut std::io::Cursor::new(out.clone()),
    )
    .unwrap();
    let e = bound.entries.iter().find(|e| e.path == member).unwrap();
    assert_eq!(
        &out[e.offset as usize..e.offset as usize + e.size as usize],
        &loose[..]
    );
    assert!(!reg.summary(id).unwrap().dirty);
}

/// LEAGUE-R2: the reviewer's NHL 18 reproduction — a pre-existing invalid league code (31,
/// seeded straight into the copied file) replaced by a *different* invalid code (30) keeps the
/// violation count at one and must still be refused at the API seam, leaving the working bytes,
/// undo/redo, pending state and the destination file untouched. Unrelated edits on that row and
/// a valid repair stay accepted.
#[test]
fn nhl18_equal_count_invalid_substitution_is_refused() {
    let tmp = Tmp::new("r2-nhl18");
    let root = tmp.0.join("copy");
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let dir = root.join("nhl/data/common/gamemodes");
    std::fs::create_dir_all(&dir).unwrap();
    let original = std::fs::read(repo("anchors/ps4/nhl18/databases/nhl_game.db")).unwrap();
    let mut bytes = original.clone();
    {
        let parsed = tdb_core::TdbFile::parse(&original).unwrap();
        let kott = parsed.table_canonical("kOtt").unwrap();
        let f = kott
            .fields
            .iter()
            .find(|f| f.id.canonical(parsed.endian) == "xMjj")
            .unwrap();
        let start = kott.record_block;
        let end = start + kott.record_len_bytes as usize;
        tdb_core::write_bits(
            &mut bytes[start..end],
            f.bit_offset,
            f.bit_width,
            31,
            parsed.endian,
        );
        parsed.recompute_crcs(&mut bytes);
    }
    let db = dir.join("nhl_game.db");
    std::fs::write(&db, &bytes).unwrap();
    let seeded_sha = sha(&db);

    let reg = Registry::default();
    let id = reg
        .open_workspace_from_paths(
            Some("nhl18"),
            parse_platform("ps4").unwrap(),
            &s(&db),
            None,
            &s(&repo("anchors/ps4/nhl18/databases/nhl_game-meta.xml")),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    let codes: Vec<i64> = reg
        .list_leagues(id)
        .unwrap()
        .leagues
        .iter()
        .filter(|l| l.record.is_some())
        .map(|l| l.code)
        .collect();
    assert!(!codes.contains(&30) && !codes.contains(&31), "{codes:?}");
    assert_eq!(reg.list_teams(id).unwrap()[0].league, Some(31));
    let before = reg.summary(id).unwrap();

    let err = match reg.set_field(id, "kOtt", 0, "xMjj", FieldValue::Int(30)) {
        Err(e) => e.to_string(),
        Ok(_) => panic!("invalid 31 -> invalid 30 must be refused"),
    };
    assert!(
        err.contains("league = 30 refers to no stockleagues row"),
        "{err}"
    );
    let after = reg.summary(id).unwrap();
    assert!(!after.dirty && after.undo_depth == 0 && after.redo_depth == 0);
    assert_eq!(after.workspace_revision, before.workspace_revision);
    assert_eq!(reg.pending_changes(id, 10).unwrap().primary.total_cells, 0);
    assert_eq!(reg.list_teams(id).unwrap()[0].league, Some(31), "still 31");
    assert_eq!(sha(&db), seeded_sha, "destination untouched");

    // Unrelated edit on the still-invalid row is accepted; the value is preserved, not repaired.
    reg.set_field(id, "kOtt", 0, "xsnn", FieldValue::Text("ZZZ".into()))
        .unwrap();
    assert_eq!(reg.list_teams(id).unwrap()[0].league, Some(31));
    // Repair to a real row is accepted; re-introducing either absent code afterwards is refused.
    reg.set_field(id, "kOtt", 0, "xMjj", FieldValue::Int(0))
        .unwrap();
    assert!(reg
        .set_field(id, "kOtt", 0, "xMjj", FieldValue::Int(30))
        .is_err());
    assert!(reg
        .set_field(id, "kOtt", 0, "xMjj", FieldValue::Int(31))
        .is_err());
    assert_eq!(reg.summary(id).unwrap().undo_depth, 2);
    assert_eq!(sha(&db), seeded_sha, "nothing saved");
}
