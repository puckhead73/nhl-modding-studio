//! R1 F2 — undo of a texture-member apply restores the replacement it superseded. The pending
//! model keeps an operation log beside the coalesced (one-per-member) effective list: A→B→undo on
//! one member leaves A pending (same position, same revision as after A), interleaved members
//! unwind in apply order, and every undo discards the held reviews. Synthetic NHL 15-shaped
//! archive with raw PNG members; nothing here is evidence a game accepts the result.

use std::path::{Path, PathBuf};

use studio_domain::tdb_archive_big::eb::FILE_TABLE_OFFSET;
use studio_domain::tdb_archive_big::stage::synth::build_archive;
use studio_domain::{EditSession, Platform, Project, ProjectRole, Workspace};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-r1f2-{tag}-{}", std::process::id()));
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

const ARCHIVE: &str = "databigs/inxp10.big";
const L0: &str = "nhl/ui/assets/leaguelogos/size64/l0.png";
const L5: &str = "nhl/ui/assets/leaguelogos/size64/l5.png";
const L100: &str = "nhl/ui/assets/leaguelogos/size64/l100.png";

fn rgba(seed: u8) -> Vec<u8> {
    (0..64u32 * 64).flat_map(|i| [(i % 64) as u8 ^ seed, (i / 64) as u8, seed, 255]).collect()
}

fn png(seed: u8) -> Vec<u8> {
    studio_domain::DecodedImage { width: 64, height: 64, rgba: rgba(seed) }.to_png().unwrap()
}

/// A copied-target root with the NHL 15 anchor database and an archive of three PNG logos,
/// `l100` sharing `l5`'s span (as shipped).
fn build_root(tmp: &Tmp) -> (PathBuf, PathBuf) {
    let root = tmp.0.join("copy");
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let db_dir = root.join("nhl/data/common/gamemodes/database/t3db");
    std::fs::create_dir_all(&db_dir).unwrap();
    let db = db_dir.join("nhl_game.db");
    std::fs::copy(repo("anchors/ps4/nhl15/databases/nhl_game.db"), &db).unwrap();
    let files: Vec<(u16, &str, Vec<u8>)> = vec![
        (1, "first.bin", b"FIRST".to_vec()),
        (0, "l0.png", png(1)),
        (0, "l5.png", png(5)),
        (0, "l100.png", png(100)),
        (1, "last.bin", vec![0xAB; 64]),
    ];
    let mut bytes = build_archive(&["nhl/ui/assets/leaguelogos/size64", "nhl/data/other"], &files);
    let r2 = FILE_TABLE_OFFSET + 2 * 16;
    let r3 = FILE_TABLE_OFFSET + 3 * 16;
    let row2 = bytes[r2..r2 + 12].to_vec();
    bytes[r3..r3 + 12].copy_from_slice(&row2);
    std::fs::create_dir_all(root.join("databigs")).unwrap();
    std::fs::write(root.join(ARCHIVE), &bytes).unwrap();
    (root, db)
}

fn open(db: &Path) -> Workspace {
    let project = Project::load(db, Platform::Ps4, ProjectRole::Nhlng, repo("anchors/ps4/nhl15/databases/nhl_game-meta.xml"), repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(project), db.to_string_lossy().into_owned());
    ws.set_game_id("nhl15");
    ws
}

/// Preview + apply `seed` onto `member`; returns the new stored identity of the plan.
fn apply(ws: &mut Workspace, root: &Path, member: &str, seed: u8) -> String {
    let (ticket, plan) = ws.asset_member_preview(root, ARCHIVE, member, &rgba(seed), 64, 64, false).unwrap_or_else(|r| panic!("{r:?}"));
    let review = ws.asset_member_apply_review(&ticket).unwrap();
    assert_eq!(review.plan.member.new_stored_sha256, plan.member.new_stored_sha256);
    plan.member.new_stored_sha256
}

fn pending(ws: &Workspace) -> Vec<(String, String)> {
    ws.asset_member_edits().iter().map(|p| (p.member_path.clone(), p.member.new_stored_sha256.clone())).collect()
}

fn row(member: &str, sha: &str) -> (String, String) {
    (member.to_string(), sha.to_string())
}

#[test]
fn undoing_the_second_replacement_of_a_member_restores_the_first() {
    let tmp = Tmp::new("ab");
    let (root, db) = build_root(&tmp);
    let mut ws = open(&db);
    let clean = ws.revision();
    assert!(!ws.is_dirty());

    let a = apply(&mut ws, &root, L0, 42);
    let after_a = ws.revision();
    assert_ne!(after_a, clean);
    assert_eq!(pending(&ws), vec![row(L0, &a)]);
    assert_eq!(ws.asset_member_undo_depth(), 1);

    let b = apply(&mut ws, &root, L0, 77);
    assert_ne!(a, b);
    let after_b = ws.revision();
    assert_ne!(after_b, after_a, "a different image is a different revision");
    // Coalesced: one effective replacement, two undoable operations.
    assert_eq!(pending(&ws), vec![row(L0, &b)]);
    assert_eq!(ws.asset_member_undo_depth(), 2);

    // Undo B: A is pending again at the same revision; the project stays dirty.
    assert!(ws.undo_asset_member_edit());
    assert_eq!(pending(&ws), vec![row(L0, &a)]);
    assert_eq!(ws.revision(), after_a);
    assert!(ws.is_dirty());
    assert_eq!(ws.asset_member_undo_depth(), 1);

    // Undo A: clean.
    assert!(ws.undo_asset_member_edit());
    assert!(pending(&ws).is_empty());
    assert_eq!(ws.revision(), clean);
    assert!(!ws.is_dirty());
    assert_eq!(ws.asset_member_undo_depth(), 0);
    assert!(!ws.undo_asset_member_edit(), "nothing left to undo");
}

#[test]
fn interleaved_members_unwind_in_apply_order_and_keep_their_positions() {
    let tmp = Tmp::new("interleaved");
    let (root, db) = build_root(&tmp);
    let mut ws = open(&db);

    let l0_a = apply(&mut ws, &root, L0, 1);
    let rev_1 = ws.revision();
    let l100_x = apply(&mut ws, &root, L100, 2);
    let rev_2 = ws.revision();
    let l0_b = apply(&mut ws, &root, L0, 3);
    let rev_3 = ws.revision();
    let l5_y = apply(&mut ws, &root, L5, 4);
    let rev_4 = ws.revision();
    let l100_z = apply(&mut ws, &root, L100, 5);
    // Effective list: one entry per member, first-seen order, latest bytes.
    assert_eq!(pending(&ws), vec![row(L0, &l0_b), row(L100, &l100_z), row(L5, &l5_y)]);
    assert_eq!(ws.asset_member_undo_depth(), 5);

    assert!(ws.undo_asset_member_edit()); // l100: Z → X (position kept)
    assert_eq!(pending(&ws), vec![row(L0, &l0_b), row(L100, &l100_x), row(L5, &l5_y)]);
    assert_eq!(ws.revision(), rev_4);
    assert!(ws.undo_asset_member_edit()); // l5: Y removed
    assert_eq!(pending(&ws), vec![row(L0, &l0_b), row(L100, &l100_x)]);
    assert_eq!(ws.revision(), rev_3);
    assert!(ws.undo_asset_member_edit()); // l0: B → A
    assert_eq!(pending(&ws), vec![row(L0, &l0_a), row(L100, &l100_x)]);
    assert_eq!(ws.revision(), rev_2);
    assert!(ws.undo_asset_member_edit()); // l100: X removed
    assert_eq!(pending(&ws), vec![row(L0, &l0_a)]);
    assert_eq!(ws.revision(), rev_1);
    assert!(ws.undo_asset_member_edit()); // l0: A removed
    assert!(pending(&ws).is_empty());
    assert!(!ws.is_dirty());
    assert!(!ws.undo_asset_member_edit());
}

#[test]
fn every_undo_discards_held_reviews_and_a_reload_clears_the_log() {
    let tmp = Tmp::new("reviews");
    let (root, db) = build_root(&tmp);
    let mut ws = open(&db);

    apply(&mut ws, &root, L0, 10);
    apply(&mut ws, &root, L0, 11);
    // A review held now is bound to the revision with B pending; undo changes the revision and
    // discards the review, so applying it afterwards refuses without mutating.
    let (held, _) = ws.asset_member_preview(&root, ARCHIVE, L100, &rgba(12), 64, 64, false).unwrap();
    assert!(ws.asset_member_review(&held).is_some());
    assert!(ws.undo_asset_member_edit());
    assert!(ws.asset_member_review(&held).is_none());
    let before = pending(&ws);
    let err = ws.asset_member_apply_review(&held).err().expect("stale review refused").to_string();
    assert!(err.contains("no longer held"), "{err}");
    assert_eq!(pending(&ws), before);
    assert_eq!(ws.asset_member_undo_depth(), 1);

    // A ticket from another workspace instance is foreign here (unchanged protection).
    let mut other = open(&db);
    let (foreign, _) = other.asset_member_preview(&root, ARCHIVE, L0, &rgba(13), 64, 64, false).unwrap();
    let err = ws.asset_member_apply_review(&foreign).err().expect("foreign ticket refused").to_string();
    assert!(err.contains("another project"), "{err}");
    assert_eq!(pending(&ws), before);

    // A reload drops the effective list and the operation log together.
    apply(&mut ws, &root, L100, 14);
    assert_eq!(ws.asset_member_undo_depth(), 2);
    ws.reload_from_disk().unwrap();
    assert!(pending(&ws).is_empty());
    assert_eq!(ws.asset_member_undo_depth(), 0);
    assert!(!ws.undo_asset_member_edit());
    assert!(!ws.is_dirty());
}

/// R1 F1 (20 September 2026): contradictory scopes on one deduplicated block never join the pending
/// list. With a whole-group replacement of `l100` pending (it owns `l5`'s row too), a private copy of
/// `l5` and a whole-group replacement of `l5` are both refused at preview (nothing is held) with the
/// pending list, the revision and the undo depth exactly as they were; the reverse order (private
/// `l5` pending, then whole-group `l100`) refuses the same way. Two private copies of the block, a
/// re-plan of the *same* member with either scope (it supersedes), and an unrelated member remain
/// accepted, and undo clears the way again. The apply-side guard is defensive: every apply changes
/// the revision and discards held reviews, so a review that conflicts with a *later* apply is
/// refused as stale before the scope check — shown here too.
#[test]
fn contradictory_scopes_on_one_block_are_refused_and_leave_pending_and_history_unchanged() {
    let tmp = Tmp::new("scope-conflict");
    let (root, db) = build_root(&tmp);
    let mut ws = open(&db);

    // A private `l5` review held before any group plan exists.
    let (early_l5, _) = ws.asset_member_preview(&root, ARCHIVE, L5, &rgba(1), 64, 64, false).unwrap();

    // Whole-group `l100` pending.
    let (t, plan) = ws.asset_member_preview(&root, ARCHIVE, L100, &rgba(2), 64, 64, true).unwrap();
    assert!(plan.whole_group && plan.shared_with == vec![L5.to_string()]);
    ws.asset_member_apply_review(&t).unwrap();
    let pending_before = pending(&ws);
    let rev_before = ws.revision();
    assert_eq!(ws.asset_member_undo_depth(), 1);

    // Private `l5` and whole-group `l5` are both refused at preview; nothing moves.
    for whole in [false, true] {
        let r = ws.asset_member_preview(&root, ARCHIVE, L5, &rgba(3), 64, 64, whole).err().expect("conflict refused at preview");
        assert_eq!(r.kind, "scope-conflict");
        assert!(r.reason.contains(L5) && r.reason.contains(L100) && r.reason.contains("whole group"), "{}", r.reason);
        assert_eq!(pending(&ws), pending_before);
        assert_eq!(ws.revision(), rev_before);
        assert_eq!(ws.asset_member_undo_depth(), 1);
    }
    // The early private-`l5` review was discarded by the apply: refused as stale, pending unchanged.
    let err = ws.asset_member_apply_review(&early_l5).err().expect("stale review refused").to_string();
    assert!(err.contains("no longer held"), "{err}");
    assert_eq!(pending(&ws), pending_before);
    assert_eq!(ws.asset_member_undo_depth(), 1);

    // An unrelated member is unaffected by the pending group plan.
    let (t0, _) = ws.asset_member_preview(&root, ARCHIVE, L0, &rgba(7), 64, 64, false).unwrap();
    ws.asset_member_apply_review(&t0).unwrap();
    assert_eq!(ws.asset_member_undo_depth(), 2);
    assert_eq!(pending(&ws).len(), 2);

    // The same member with the other scope supersedes (never a conflict): `l100` becomes a
    // private copy, so a private `l5` is now accepted (two private copies of one block).
    let (same, _) = ws.asset_member_preview(&root, ARCHIVE, L100, &rgba(4), 64, 64, false).unwrap();
    ws.asset_member_apply_review(&same).unwrap();
    let l100 = |ws: &Workspace| ws.asset_member_edits().iter().find(|p| p.member_path == L100).unwrap().whole_group;
    assert!(!l100(&ws), "the private re-plan superseded the group plan in place");
    assert_eq!(pending(&ws)[0].0, L100, "position kept");
    assert_eq!(ws.asset_member_undo_depth(), 3);
    let (t5, _) = ws.asset_member_preview(&root, ARCHIVE, L5, &rgba(5), 64, 64, false).unwrap();
    ws.asset_member_apply_review(&t5).unwrap();
    assert_eq!(pending(&ws).len(), 3);
    assert_eq!(ws.asset_member_undo_depth(), 4);

    // Reverse order: a whole-group `l100` now contradicts the pending private `l5`.
    let snapshot = (pending(&ws), ws.revision());
    let r = ws.asset_member_preview(&root, ARCHIVE, L100, &rgba(6), 64, 64, true).err().expect("reverse-order conflict refused");
    assert_eq!(r.kind, "scope-conflict");
    assert!(r.reason.contains("this member only"), "{}", r.reason);
    assert_eq!((pending(&ws), ws.revision()), snapshot);
    assert_eq!(ws.asset_member_undo_depth(), 4);

    // Undo the private `l5`: the group scope on `l100` is accepted again and applies.
    assert!(ws.undo_asset_member_edit());
    assert_eq!(ws.asset_member_undo_depth(), 3);
    let (tg, _) = ws.asset_member_preview(&root, ARCHIVE, L100, &rgba(6), 64, 64, true).unwrap();
    ws.asset_member_apply_review(&tg).unwrap();
    assert!(l100(&ws));
    assert_eq!(ws.asset_member_undo_depth(), 4);
    assert_eq!(pending(&ws).len(), 2);
}

/// P1 audit (21 Sep 2026) — LIFO undo re-establishes a group's ownership exactly: a whole-group
/// `l100` (owning `l5`) superseded in place by a private `l100`, then a private `l5` accepted on
/// top; undoing in order removes `l5`, then restores the whole-group `l100` — at which point a
/// private `l5` is refused again — and undoing the group frees `l5`. Every step keeps one plan per
/// member, positions stable, and the log depth exact; a stale ticket held across an undo is
/// refused with the pending list unchanged.
#[test]
fn lifo_undo_restores_whole_group_ownership_and_its_refusals() {
    let tmp = Tmp::new("scope-lifo");
    let (root, db) = build_root(&tmp);
    let mut ws = open(&db);
    let whole = |ws: &Workspace, m: &str| ws.asset_member_edits().iter().find(|p| p.member_path == m).map(|p| p.whole_group);

    let (t1, _) = ws.asset_member_preview(&root, ARCHIVE, L100, &rgba(2), 64, 64, true).unwrap();
    ws.asset_member_apply_review(&t1).unwrap();
    let (t2, _) = ws.asset_member_preview(&root, ARCHIVE, L100, &rgba(3), 64, 64, false).unwrap();
    ws.asset_member_apply_review(&t2).unwrap();
    assert_eq!(whole(&ws, L100), Some(false));
    let (t3, _) = ws.asset_member_preview(&root, ARCHIVE, L5, &rgba(4), 64, 64, false).unwrap();
    ws.asset_member_apply_review(&t3).unwrap();
    assert_eq!(pending(&ws).len(), 2);
    assert_eq!(ws.asset_member_undo_depth(), 3);
    // A review held now, then invalidated by the undo below.
    let (held, _) = ws.asset_member_preview(&root, ARCHIVE, L0, &rgba(9), 64, 64, false).unwrap();

    assert!(ws.undo_asset_member_edit(), "undo the private l5");
    assert_eq!(pending(&ws).len(), 1);
    assert_eq!(whole(&ws, L100), Some(false));
    assert_eq!(whole(&ws, L5), None);
    let err = ws.asset_member_apply_review(&held).unwrap_err().to_string();
    assert!(err.contains("no longer held"), "{err}");
    assert_eq!(pending(&ws).len(), 1);

    assert!(ws.undo_asset_member_edit(), "undo the private l100: the whole-group plan returns");
    assert_eq!(whole(&ws, L100), Some(true));
    assert_eq!(pending(&ws).len(), 1);
    assert_eq!(ws.asset_member_undo_depth(), 1);
    let r = ws.asset_member_preview(&root, ARCHIVE, L5, &rgba(5), 64, 64, false).err().expect("l5 owned by the group again");
    assert_eq!(r.kind, "scope-conflict");
    assert_eq!(pending(&ws).len(), 1);

    assert!(ws.undo_asset_member_edit(), "undo the group");
    assert!(pending(&ws).is_empty());
    assert_eq!(ws.asset_member_undo_depth(), 0);
    assert!(!ws.undo_asset_member_edit());
    let (t5, _) = ws.asset_member_preview(&root, ARCHIVE, L5, &rgba(5), 64, 64, false).unwrap();
    ws.asset_member_apply_review(&t5).unwrap();
    assert_eq!(pending(&ws), vec![row(L5, &pending(&ws)[0].1)]);
    assert_eq!(ws.asset_member_undo_depth(), 1);
}
