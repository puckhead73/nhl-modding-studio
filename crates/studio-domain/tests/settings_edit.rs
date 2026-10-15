//! D3 / P1-BS-3 — editing one stored catalogue default on NHL 15 PS4 (handoff 22 rev. 3 §2 with
//! the P1-BS-R1 acceptance note): eligibility on the opened file, the held review, the write's
//! preservation accounting (only the selected field's bits and the covering body CRC may change;
//! header CRCs and the file-header hash stay byte-identical), undo/redo, pending, a second edit on
//! a file already carrying one, save/reopen, the copied-target transaction with restore, and the
//! refusals. Everything runs on temp copies of the committed anchor (and, `--ignored`, of the
//! private patch 01.06 database). Nothing here says what the game does with a stored default.

use std::path::{Path, PathBuf};

use studio_domain::copied_target::stage_workspace;
use studio_domain::settings_catalogue::CatalogueSource;
use studio_domain::settings_edit::{accounted_diff, checksum_offsets, verify_single_cell_diff, DefaultTarget};
use studio_domain::tdb_txn::{self, FileIdentity, NoHooks, State};
use studio_domain::{EditSession, FieldValue, Platform, Project, ProjectRole, Workspace};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

const META: &str = "anchors/ps4/nhl15/databases/nhl_game-meta.xml";
const DB: &str = "anchors/ps4/nhl15/databases/nhl_game.db";

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-d3-{tag}-{}", std::process::id()));
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

fn open(db: &Path) -> Workspace {
    let project = Project::load(db, Platform::Ps4, ProjectRole::Nhlng, repo(META), repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(project), db.to_string_lossy().into_owned());
    ws.set_game_id("nhl15");
    ws
}

fn anchor_copy(tmp: &Tmp) -> PathBuf {
    let p = tmp.0.join("nhl_game.db");
    std::fs::copy(repo(DB), &p).unwrap();
    p
}

fn ctx(legacy_id: i64, context: i64) -> DefaultTarget {
    DefaultTarget { legacy_id, context: Some(context) }
}
fn global(legacy_id: i64) -> DefaultTarget {
    DefaultTarget { legacy_id, context: None }
}

fn stored(ws: &Workspace, target: DefaultTarget) -> Option<i64> {
    let cat = ws.display().settings_catalogue(None).unwrap().unwrap();
    let s = cat.setting(target.legacy_id)?;
    match target.context {
        None => s.global.as_ref()?.code,
        Some(c) => s.context_defaults.iter().find(|d| d.context == c)?.stored.code,
    }
}

/// Whole-file accounting of one edit: every changed byte lies in the selected field's bits or
/// the covering CRC word; every other checksum word (both header CRCs of every table, the file
/// header hash, the other prior/EOF CRCs) is byte-identical.
fn assert_single_cell_only(before: &[u8], after: &[u8], table: &str, record: u32) {
    let db = TdbFile::parse(before).unwrap();
    let acc = accounted_diff(&db, table, record, "npHF").unwrap();
    verify_single_cell_diff(before, after, &acc, db.endian).unwrap_or_else(|e| panic!("{e}"));
    let changed: Vec<usize> = (0..before.len()).filter(|&i| before[i] != after[i]).collect();
    assert!(!changed.is_empty(), "nothing changed");
    for (off, what) in checksum_offsets(&db) {
        let moved = (off..off + 4).any(|i| before[i] != after[i]);
        if off == acc.covering_crc_offset {
            assert!(moved, "the covering {what} at {off:#x} must move");
        } else {
            assert!(!moved, "{what} at {off:#x} must not move");
        }
    }
    // The record's other fields: masked compare.
    let rec = &before[acc.record_start..acc.record_start + acc.record_len];
    let rec2 = &after[acc.record_start..acc.record_start + acc.record_len];
    for (i, (b, a)) in rec.iter().zip(rec2).enumerate() {
        assert_eq!(b & !acc.field_mask[i], a & !acc.field_mask[i], "byte {i} of the record outside the field");
    }
    // Exactly: changed bytes ⊆ record bytes ∪ crc word.
    for i in changed {
        assert!((acc.record_start..acc.record_start + acc.record_len).contains(&i) || (acc.covering_crc_offset..acc.covering_crc_offset + 4).contains(&i), "byte {i:#x}");
    }
}

// ---------------------------------------------------------------------------------------------
// 1. The accepted edit: (3, 72) 3 → 2, preview → apply → accounting → undo/redo → pending
// ---------------------------------------------------------------------------------------------

#[test]
fn context_default_edit_changes_only_the_cell_and_its_covering_crc_with_undo_redo_and_pending() {
    let tmp = Tmp::new("ctx");
    let path = anchor_copy(&tmp);
    let mut ws = open(&path);
    let before = ws.primary().bytes().to_vec();
    let (ticket, plan) = ws.settings_default_preview(ctx(72, 3), 2).unwrap().unwrap();
    assert_eq!((plan.setting_name.as_str(), plan.role.as_str(), plan.record, plan.studio_table.as_str(), plan.table_name.as_str()), ("PlayoffSeriesLength", "DYNASTY_PLAYOFFS_LENGTH", 152, "gZtX", "stocksettingdefaults"));
    assert_eq!((plan.before, plan.after, plan.min, plan.max, plan.size, plan.no_op), (3, 2, 0, 4, 5, false));
    assert_eq!((plan.before_label.as_deref(), plan.after_label.as_deref(), plan.label_source.as_str()), (Some("BestOf7"), Some("BestOf5"), "inferred"));
    assert_eq!(plan.context_name.as_deref(), Some("FROM_DYNASTY_FLOW"));
    assert_eq!(plan.siblings, [86, 116]);
    assert_eq!(plan.unchanged, [("global".to_string(), 3), ("context 5 (FROM_MUD_CREATION_FLOW)".to_string(), 3), ("context 30 (FROM_BE_A_PRO_FLOW)".to_string(), 3)]);
    assert_eq!(plan.copy, "base 01.00");
    assert!(plan.description.contains("changes from code 3 (*BestOf7*, label inferred) to code 2 (*BestOf5*, label inferred)"), "{}", plan.description);
    assert!(plan.description.contains("Unchanged: global = 3, context 5") && plan.description.contains("has not been established"), "{}", plan.description);
    assert!(plan.stock_anomalies.iter().any(|a| a.contains("legacyid 236")));
    assert!(plan.notes.iter().any(|n| n.contains("patch_inxp00.big")));
    assert_eq!(ws.primary().bytes(), &before[..], "preview writes nothing");
    assert!(ws.settings_review(&ticket).is_some());

    let (review, outcome) = ws.settings_apply_review(&ticket).unwrap();
    assert_eq!(review.plan.after, 2);
    assert!(outcome.edits >= 1, "{outcome:?}");
    let after = ws.primary().bytes().to_vec();
    assert_single_cell_only(&before, &after, "gZtX", 152);
    // The 4-byte record: legacyid and context bits unchanged, only `default` (bits 14..18).
    let db = TdbFile::parse(&after).unwrap();
    let acc = accounted_diff(&db, "gZtX", 152, "npHF").unwrap();
    assert_eq!(acc.record_len, 4);
    assert_eq!(acc.field_mask, [0x00, 0xC0, 0x03, 0x00], "LSB-first bits 14..18");
    assert!(!acc.covering_crc_is_eof, "stocksettingdefaults is not the last body");
    // Re-decode: equals the source except that one cell; the catalogue's stock anomalies unchanged.
    assert_eq!(stored(&ws, ctx(72, 3)), Some(2));
    assert_eq!(stored(&ws, global(72)), Some(3));
    assert_eq!(stored(&ws, ctx(72, 5)), Some(3));
    assert_eq!(stored(&ws, ctx(86, 9)), None, "no context-9 rows on the base copy");
    let cat = ws.display().settings_catalogue(None).unwrap().unwrap();
    assert_eq!(cat.anomalies, plan.stock_anomalies);
    let CatalogueSource::Database { copy, .. } = &cat.source else { panic!() };
    assert!(copy.starts_with("unrecorded"));
    // Pending: exactly one cell, named by the setting and the screen.
    let (pending, _) = ws.pending_changes(20);
    assert_eq!(pending.total_cells, 1);
    let c = &pending.cells[0];
    assert_eq!((c.table_name.as_str(), c.field_name.as_str(), c.before.decoded, c.after.decoded), ("stocksettingdefaults", "default", Some(3), Some(2)));
    assert_eq!(c.entity, "Setting PlayoffSeriesLength (id 72) — stored default for screen context 3");
    // The ticket is consumed.
    let e = ws.settings_apply_review(&ticket).unwrap_err().to_string();
    assert!(e.contains("no longer held"), "{e}");
    assert_eq!(ws.primary().bytes(), &after[..]);
    // Undo restores byte identity; redo re-applies the same bytes.
    assert!(ws.undo());
    assert_eq!(ws.primary().bytes(), &before[..]);
    assert!(!ws.is_dirty());
    assert!(ws.redo());
    assert_eq!(ws.primary().bytes(), &after[..]);
    assert_eq!(stored(&ws, ctx(72, 3)), Some(2));
}

#[test]
fn global_default_edit_changes_the_stocksettings_body_only() {
    let tmp = Tmp::new("global");
    let path = anchor_copy(&tmp);
    let mut ws = open(&path);
    let before = ws.primary().bytes().to_vec();
    let (ticket, plan) = ws.settings_default_preview(global(71), 0).unwrap().unwrap();
    assert_eq!((plan.studio_table.as_str(), plan.record, plan.before, plan.after), ("fRvu", 71, 1, 0));
    assert_eq!((plan.before_label.as_deref(), plan.after_label.as_deref()), (Some("1Point"), Some("0Points")));
    assert_eq!(plan.unchanged, [("context 3 (FROM_DYNASTY_FLOW)".to_string(), 1), ("context 5 (FROM_MUD_CREATION_FLOW)".to_string(), 1), ("context 30 (FROM_BE_A_PRO_FLOW)".to_string(), 1)]);
    ws.settings_apply_review(&ticket).unwrap();
    let after = ws.primary().bytes().to_vec();
    assert_single_cell_only(&before, &after, "fRvu", 71);
    let db = TdbFile::parse(&after).unwrap();
    let acc = accounted_diff(&db, "fRvu", 71, "npHF").unwrap();
    assert_eq!(acc.record_len, 56);
    // `default` @408:3 → byte 51, bits 0..3.
    assert_eq!(acc.field_mask.iter().enumerate().filter(|(_, m)| **m != 0).collect::<Vec<_>>(), [(51, &0x07u8)]);
    assert_eq!(stored(&ws, global(71)), Some(0));
    assert_eq!(stored(&ws, ctx(71, 3)), Some(1));
    // The 56-byte record's stringid / id / min / max / size are untouched.
    let cat = ws.display().settings_catalogue(None).unwrap().unwrap();
    let s = cat.setting(71).unwrap();
    assert_eq!((s.name.as_str(), s.setting_id, s.min, s.max, s.size), ("SeasonOvertimeLoss", Some(155), Some(0), Some(1), Some(2)));
    assert!(ws.undo());
    assert_eq!(ws.primary().bytes(), &before[..]);
}

// ---------------------------------------------------------------------------------------------
// 2. Two successive edits, save/reopen, restore
// ---------------------------------------------------------------------------------------------

#[test]
fn a_second_edit_stays_eligible_on_the_edited_file_and_save_reopen_restore_round_trip() {
    let tmp = Tmp::new("second");
    let path = anchor_copy(&tmp);
    let original = std::fs::read(&path).unwrap();
    let mut ws = open(&path);
    let (t1, _) = ws.settings_default_preview(ctx(72, 3), 2).unwrap().unwrap();
    ws.settings_apply_review(&t1).unwrap();
    let after_first = ws.primary().bytes().to_vec();
    // Second target on the file that already carries the first edit: eligible, re-validated on
    // the current bytes; the first cell is preserved.
    let (t2, plan2) = ws.settings_default_preview(ctx(71, 3), 0).unwrap().unwrap();
    assert_eq!((plan2.before, plan2.after, plan2.record), (1, 0, 151));
    ws.settings_apply_review(&t2).unwrap();
    let after_second = ws.primary().bytes().to_vec();
    assert_single_cell_only(&after_first, &after_second, "gZtX", 151);
    assert_eq!(stored(&ws, ctx(72, 3)), Some(2));
    assert_eq!(stored(&ws, ctx(71, 3)), Some(0));
    assert_eq!(ws.undo_depth(), 2);
    let (pending, _) = ws.pending_changes(20);
    assert_eq!(pending.total_cells, 2);
    // Two edits to the same cell: the last value wins, each its own undo step.
    let (t3, p3) = ws.settings_default_preview(ctx(72, 3), 4).unwrap().unwrap();
    assert_eq!((p3.before, p3.after), (2, 4));
    ws.settings_apply_review(&t3).unwrap();
    assert_eq!(stored(&ws, ctx(72, 3)), Some(4));
    assert!(ws.undo());
    assert_eq!(stored(&ws, ctx(72, 3)), Some(2));
    // Save to the copy and reopen: the edits read back, the copy is "unrecorded", and the file
    // is eligible for a further edit without any ticket carried over.
    ws.save_primary_to(path.to_str().unwrap()).unwrap();
    let saved = std::fs::read(&path).unwrap();
    assert_eq!(saved.len(), original.len());
    let mut reopened = open(&path);
    assert_eq!(stored(&reopened, ctx(72, 3)), Some(2));
    assert_eq!(stored(&reopened, ctx(71, 3)), Some(0));
    assert!(reopened.settings_review(&t1).is_none() && reopened.settings_review(&t2).is_none());
    let (t4, p4) = reopened.settings_default_preview(ctx(72, 30), 1).unwrap().unwrap();
    assert_eq!((p4.before, p4.after), (3, 1));
    assert!(p4.copy.starts_with("unrecorded"), "{}", p4.copy);
    reopened.settings_apply_review(&t4).unwrap();
    assert_single_cell_only(&saved, reopened.primary().bytes(), "gZtX", p4.record);
    // Restore: undo everything on the original workspace and save equals the anchor byte-for-byte.
    while ws.undo() {}
    assert!(!ws.is_dirty());
    ws.save_primary_to(path.to_str().unwrap()).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn copied_target_transaction_installs_the_one_cell_and_restore_returns_the_exact_bytes() {
    let tmp = Tmp::new("copied");
    let root = tmp.0.join("copy");
    tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let dest = root.join("nhl/data/common/gamemodes/database/t3db");
    std::fs::create_dir_all(&dest).unwrap();
    let db_path = dest.join("nhl_game.db");
    std::fs::copy(repo(DB), &db_path).unwrap();
    let original = FileIdentity::of_file(&db_path).unwrap();
    let staging = tmp.0.join("staging");
    std::fs::create_dir_all(&staging).unwrap();

    let mut ws = open(&db_path);
    let before = ws.primary().bytes().to_vec();
    let (t, previewed) = ws.settings_default_preview(ctx(72, 3), 2).unwrap().unwrap();
    ws.settings_apply_review(&t).unwrap();
    let plan = stage_workspace(&ws, &root, &staging, "D3 (3,72) 3→2").unwrap();
    let primary = plan.artifact(studio_domain::copied_target::KEY_PRIMARY).unwrap();
    assert_eq!(primary.artifact.expected_prior, studio_domain::tdb_txn::Expected::Present(original.clone()));
    let mut txn = tdb_txn::prepare(&plan.root, &plan.change_set, &mut NoHooks).unwrap();
    let out = tdb_txn::apply(&mut txn, &plan.change_set, &mut NoHooks).unwrap();
    assert_eq!(out.state, State::Committed);
    let installed = std::fs::read(&db_path).unwrap();
    assert_single_cell_only(&before, &installed, "gZtX", previewed.record);
    assert_eq!(installed, ws.primary().bytes());
    // Reopen the installed copy: the cell reads back, the copy is unrecorded.
    let reopened = open(&db_path);
    assert_eq!(stored(&reopened, ctx(72, 3)), Some(2));
    // Restore the transaction: the exact original bytes return.
    tdb_txn::restore(&root, &txn.txn_id, &mut NoHooks).unwrap();
    assert_eq!(FileIdentity::of_file(&db_path).unwrap(), original);
    assert_eq!(std::fs::read(&db_path).unwrap(), before);
}

// ---------------------------------------------------------------------------------------------
// 3. Tickets: stale, foreign, consumed, no-op
// ---------------------------------------------------------------------------------------------

#[test]
fn stale_foreign_and_consumed_tickets_refuse_without_mutation_and_a_no_op_writes_nothing() {
    let tmp = Tmp::new("tickets");
    let path = anchor_copy(&tmp);
    let mut ws = open(&path);
    let before = ws.primary().bytes().to_vec();
    // Stale: any change after the preview (here an unrelated cell edit) moves the revision.
    let (t, _) = ws.settings_default_preview(ctx(72, 3), 2).unwrap().unwrap();
    ws.set_field("kOtt", 0, "xsnn", FieldValue::Text("ZZZ".into())).unwrap();
    let e = ws.settings_apply_review(&t).unwrap_err().to_string();
    assert!(e.contains("changed since this preview"), "{e}");
    assert_eq!(stored(&ws, ctx(72, 3)), Some(3));
    assert!(ws.undo());
    assert_eq!(ws.primary().bytes(), &before[..]);
    // Undo/redo after a preview also moves the revision back and forth: refused after undo.
    let (t, _) = ws.settings_default_preview(ctx(72, 3), 2).unwrap().unwrap();
    ws.set_field("kOtt", 0, "xsnn", FieldValue::Text("ZZZ".into())).unwrap();
    assert!(ws.undo());
    // Same bytes as at preview time → the revision matches again → apply is allowed.
    ws.settings_apply_review(&t).unwrap();
    assert_eq!(stored(&ws, ctx(72, 3)), Some(2));
    assert!(ws.undo());
    // Foreign: a ticket minted by another workspace instance on the same file.
    let mut other = open(&path);
    let (foreign, _) = other.settings_default_preview(ctx(72, 3), 2).unwrap().unwrap();
    let e = ws.settings_apply_review(&foreign).unwrap_err().to_string();
    assert!(e.contains("another project"), "{e}");
    assert!(ws.settings_review(&foreign).is_none());
    assert_eq!(ws.primary().bytes(), &before[..]);
    // Unknown / garbage.
    assert!(ws.settings_apply_review("nope/1").is_err());
    assert!(ws.settings_apply_review("").is_err());
    // A schedule-style ticket string from this namespace that was never minted for settings.
    let (t, _) = ws.settings_default_preview(ctx(72, 3), 2).unwrap().unwrap();
    let never = format!("{}9999", &t[..t.rfind('/').unwrap() + 1]);
    assert!(ws.settings_apply_review(&never).unwrap_err().to_string().contains("no longer held"));
    // No-op: the current value; apply writes nothing and keeps the file clean.
    let (t, plan) = ws.settings_default_preview(ctx(72, 3), 3).unwrap().unwrap();
    assert!(plan.no_op && plan.description.contains("already code 3"), "{}", plan.description);
    let (_, outcome) = ws.settings_apply_review(&t).unwrap();
    assert_eq!(outcome.edits, 0);
    assert!(!ws.is_dirty());
    assert_eq!(ws.primary().bytes(), &before[..]);
}

// ---------------------------------------------------------------------------------------------
// 4. Refusals (handoff 22 §7 T5)
// ---------------------------------------------------------------------------------------------

#[test]
fn refusals_name_the_reason_and_write_nothing() {
    let tmp = Tmp::new("refuse");
    let path = anchor_copy(&tmp);
    let mut ws = open(&path);
    let before = ws.primary().bytes().to_vec();
    let refused = |ws: &mut Workspace, t: DefaultTarget, code: i64| ws.settings_default_preview(t, code).unwrap().unwrap_err();
    // (37, 72): the Season creation context has no defaults row — growth, no precedence claim.
    let r = refused(&mut ws, ctx(72, 37), 2);
    assert_eq!(r.kind, "no-row");
    assert!(r.reason.contains("screen 37 (FROM_SEASON_MODE_FLOW)") && r.reason.contains("table-growth") && r.reason.contains("says nothing about which default the game would use"), "{}", r.reason);
    // Out of range.
    let r = refused(&mut ws, ctx(72, 3), 5);
    assert_eq!(r.kind, "out-of-bounds");
    assert!(r.reason.contains("0..4"), "{}", r.reason);
    assert_eq!(refused(&mut ws, global(72), 8).kind, "out-of-bounds");
    assert_eq!(refused(&mut ws, global(72), -1).kind, "out-of-bounds");
    // Excluded rows by name with their reason.
    for (id, needle) in [(74, "free-text"), (82, "patch"), (122, "label array"), (125, "BaseLeague"), (115, "no label array")] {
        let r = refused(&mut ws, global(id), 0);
        assert_eq!(r.kind, "excluded", "{id}");
        assert!(r.reason.contains(needle), "{id}: {}", r.reason);
    }
    // Out of scope (a game rule) and a nonexistent id.
    assert_eq!(refused(&mut ws, global(0), 1).kind, "not-offered");
    assert_eq!(refused(&mut ws, global(230), 1).kind, "absent");
    // A sibling of an eligible row is not a target through the sibling list: 116 is itself
    // eligible, 121 (TXT_LEAGUETYPE) is excluded even though it shares id 209 with 115.
    assert_eq!(refused(&mut ws, global(121), 0).kind, "excluded");
    assert_eq!(ws.primary().bytes(), &before[..], "refusals write nothing");

    // Synthetic duplicate key: give the (3, 71) defaults row (record 151) legacyid 72 → (3, 72) stored twice.
    ws.set_field("gZtX", 151, "jjHq", FieldValue::Int(72)).unwrap();
    let r = refused(&mut ws, ctx(72, 3), 2);
    assert_eq!(r.kind, "ambiguous");
    assert!(r.reason.contains("stored 2 times"), "{}", r.reason);
    assert!(ws.undo());
    // Synthetic inconsistent declaration: size 4 on row 72 (max 4 → size must be 5).
    ws.set_field("fRvu", 72, "qrwM", FieldValue::Int(4)).unwrap();
    let r = refused(&mut ws, ctx(72, 3), 2);
    assert_eq!(r.kind, "shape");
    assert!(r.reason.contains("size 4 ≠ max − min + 1"), "{}", r.reason);
    assert!(ws.undo());
    // A row whose min is not 0 is not offered even if it were listed (PeriodLength is not; the
    // rule is checked on the opened bytes): set 72's min raw to 1 (= -7) → refused as shape.
    ws.set_field("fRvu", 72, "SbEu", FieldValue::Int(1)).unwrap();
    let r = refused(&mut ws, ctx(72, 3), 2);
    assert_eq!(r.kind, "shape");
    assert!(r.reason.contains("starts at 1"), "{}", r.reason);
    assert!(ws.undo());
    assert_eq!(ws.primary().bytes(), &before[..]);
}

#[test]
fn other_titles_are_refused_with_the_packaging_or_representation_reason() {
    let dir = repo("anchors/ps4/nhl16/databases");
    let project = Project::load(dir.join("nhl_game.db"), Platform::Ps4, ProjectRole::Nhlng, dir.join("nhl_game-meta.xml"), repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(project), String::new());
    ws.set_game_id("nhl16");
    let r = ws.settings_default_preview(ctx(72, 3), 2).unwrap().unwrap_err();
    assert_eq!(r.kind, "not-offered");
    assert!(r.reason.contains("NHL 16–21 wait for the CSV member packaging route"), "{}", r.reason);
    let dir = repo("anchors/xbox360/nhllegacy/databases");
    let project = Project::load(dir.join("nhlng.db"), Platform::Xbox, ProjectRole::Nhlng, dir.join("nhlng-meta.xml"), repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(project), String::new());
    let r = ws.settings_default_preview(global(72), 2).unwrap().unwrap_err();
    assert_eq!(r.kind, "not-offered");
}

/// The accounting itself is checked against a deliberately wrong write: a second byte changed
/// outside the cell, a header CRC touched, a wrong covering CRC.
#[test]
fn the_preservation_accounting_rejects_writes_that_touch_anything_else() {
    let bytes = std::fs::read(repo(DB)).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let acc = accounted_diff(&db, "gZtX", 152, "npHF").unwrap();
    // A correct write, produced independently of the workspace: patch the field, recompute.
    let t = db.table("gZtX").unwrap();
    let f = t.field("npHF").unwrap();
    let mut good = bytes.clone();
    assert!(db.set_u64(&mut good, t, 152, f, 2));
    db.recompute_crcs(&mut good);
    verify_single_cell_diff(&bytes, &good, &acc, db.endian).unwrap();
    // (a) an extra byte in another record.
    let mut bad = good.clone();
    bad[acc.record_start + 8] ^= 0x01;
    assert!(verify_single_cell_diff(&bytes, &bad, &acc, db.endian).unwrap_err().contains("outside the selected cell"));
    // (b) a bit of the same record outside the field (the legacyid byte).
    let mut bad = good.clone();
    bad[acc.record_start] ^= 0x01;
    assert!(verify_single_cell_diff(&bytes, &bad, &acc, db.endian).unwrap_err().contains("outside the selected field's bits"));
    // (c) the table's own header CRC "repaired" (touched).
    let mut bad = good.clone();
    let hdr = t.block_offset + 0x24;
    bad[hdr] ^= 0x01;
    assert!(verify_single_cell_diff(&bytes, &bad, &acc, db.endian).unwrap_err().contains("outside the selected cell"));
    // (d) the covering CRC left stale.
    let mut bad = good.clone();
    bad[acc.covering_crc_offset..acc.covering_crc_offset + 4].copy_from_slice(&bytes[acc.covering_crc_offset..acc.covering_crc_offset + 4]);
    assert!(verify_single_cell_diff(&bytes, &bad, &acc, db.endian).unwrap_err().contains("covering checksum"));
    // (e) a length change.
    let mut bad = good.clone();
    bad.push(0);
    assert!(verify_single_cell_diff(&bytes, &bad, &acc, db.endian).unwrap_err().contains("length"));
    // The accounted offsets are what the layout says: the next physical table after
    // stocksettingdefaults (by block offset) owns the covering prior_crc.
    let mut order: Vec<&tdb_core::Table> = db.tables.iter().collect();
    order.sort_by_key(|t| t.block_offset);
    let pos = order.iter().position(|t| t.id.raw() == "gZtX").unwrap();
    assert_eq!(acc.covering_crc_offset, order[pos + 1].block_offset);
    assert_eq!(acc.body_start, t.block_offset + 0x28);
    assert_eq!(acc.body_end, order[pos + 1].block_offset);
}

// ---------------------------------------------------------------------------------------------
// 5. Private fixture: the patch 01.06 copy (--ignored)
// ---------------------------------------------------------------------------------------------

#[test]
#[ignore = "private fixture databases/nhl15-ps4-cusa00561/patch_inxp00.big/nhl_game.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn the_patch_copy_takes_the_same_edit_and_its_context_9_rows_are_targets() {
    let root = std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the private NHL 15 PS4 patch fixture cannot be resolved");
    let src = PathBuf::from(root).join("databases/nhl15-ps4-cusa00561/patch_inxp00.big/nhl_game.db");
    let tmp = Tmp::new("patch");
    let path = tmp.0.join("nhl_game.db");
    std::fs::copy(&src, &path).unwrap_or_else(|e| panic!("cannot copy {}: {e}", src.display()));
    let original = std::fs::read(&path).unwrap();
    assert_eq!(original.len(), 16_981_204);
    let mut ws = open(&path);
    let (t, plan) = ws.settings_default_preview(ctx(72, 3), 2).unwrap().unwrap();
    assert_eq!(plan.copy, "patch 01.06");
    assert!(plan.notes.iter().any(|n| n.contains("inxp00.big, base 01.00")), "{:?}", plan.notes);
    assert_eq!((plan.before, plan.after), (3, 2));
    ws.settings_apply_review(&t).unwrap();
    assert_single_cell_only(&original, ws.primary().bytes(), "gZtX", plan.record);
    // The patch-only context-9 tournament rows are targets (86 = PlayoffSeriesLength, 3 → 1).
    let (t, p9) = ws.settings_default_preview(ctx(86, 9), 1).unwrap().unwrap();
    assert_eq!((p9.before, p9.after), (3, 1));
    assert!(p9.context_name.as_deref().is_some_and(|n| n.contains("TOURNAMENT")), "{:?}", p9.context_name);
    let mid = ws.primary().bytes().to_vec();
    ws.settings_apply_review(&t).unwrap();
    assert_single_cell_only(&mid, ws.primary().bytes(), "gZtX", p9.record);
    assert_eq!(stored(&ws, ctx(86, 9)), Some(1));
    // 82 NumberOfTeams stays excluded on the patch copy too (0..3 there).
    let r = ws.settings_default_preview(global(82), 1).unwrap().unwrap_err();
    assert_eq!(r.kind, "excluded");
    while ws.undo() {}
    ws.save_primary_to(path.to_str().unwrap()).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
}
