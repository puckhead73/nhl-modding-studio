//! Sync-engine gate: a [`Workspace`] mirrors edits from `nhlng.db` into the roster save and saves
//! both in place. Verified against the nhl14 PS3 anchors (record-index-aligned pair — memory
//! `roster-save-mirrors-nhlng`). All edits are made on **temp copies**; the anchors are never written.

use std::path::{Path, PathBuf};

use studio_domain::{EditSession, FieldValue, Platform, Project, ProjectRole, Workspace};
use tdb_core::TdbFile;
use tdb_savedata::SaveContainer;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

const NHLNG: &str = "anchors/ps3/nhl14/databases/nhlng.db";
const META: &str = "anchors/ps3/nhl14/databases/nhlng-meta.xml";
const DEFS: &str = "defs";
const ROSTER: &str = "anchors/ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA";

/// A temp dir cleaned up on drop.
struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-ws-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
    fn copy_in(&self, src: &Path, name: &str) -> String {
        let dst = self.0.join(name);
        std::fs::copy(src, &dst).unwrap();
        dst.to_string_lossy().into_owned()
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Open a workspace over temp copies of the nhl14 nhlng.db + roster save.
fn open_ws(tmp: &Tmp) -> Workspace {
    let nhlng_path = tmp.copy_in(&repo(NHLNG), "nhlng.db");
    let roster_path = tmp.copy_in(&repo(ROSTER), "ROSTER-SYS-DATA");

    let nhlng = EditSession::begin(
        Project::load(
            &nhlng_path,
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    let roster = EditSession::begin(
        Project::load(
            &roster_path,
            Platform::Ps3,
            ProjectRole::Roster,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    Workspace::with_roster(nhlng, nhlng_path, roster, roster_path).unwrap()
}

/// Read the raw bits of `ubPc.vjla` (player position) at a record from a raw nhlng db.
fn read_vjla(db_bytes: &[u8], record: u32) -> u64 {
    let db = TdbFile::parse(db_bytes).unwrap();
    let t = db.table_canonical("ubPc").unwrap();
    let f = t
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == "vjla")
        .unwrap();
    db.read_u64(t, record, f).unwrap()
}

/// Same read, but for a roster save container (extract the inner db first).
fn read_vjla_roster(file_bytes: &[u8], record: u32) -> u64 {
    let inner = SaveContainer::parse(file_bytes)
        .unwrap()
        .extract_db()
        .unwrap();
    read_vjla(&inner, record)
}

#[test]
fn nhl14_pair_is_attached_and_aligned() {
    let tmp = Tmp::new("status");
    let ws = open_ws(&tmp);
    assert!(ws.roster_attached());
    assert!(
        ws.roster_aligned(),
        "nhl14 PS3 nhlng/roster are record-index aligned"
    );
    assert!(!ws.is_dirty());
}

#[test]
fn shared_field_edit_mirrors_into_both_files() {
    let tmp = Tmp::new("mirror");
    let mut ws = open_ws(&tmp);
    let nhlng_path = tmp.0.join("nhlng.db");
    let roster_path = tmp.0.join("ROSTER-SYS-DATA");

    // Pick a position value different from the current one (vjla is 0..=4; raw == value there).
    let baseline = read_vjla(&std::fs::read(&nhlng_path).unwrap(), 0);
    let target = if baseline == 2 { 3 } else { 2 };

    ws.set_field("ubPc", 0, "vjla", FieldValue::Int(target as i64))
        .unwrap();
    assert!(ws.is_dirty());
    assert_eq!(ws.save_in_place().unwrap(), 2, "both files written");

    let nhlng_after = read_vjla(&std::fs::read(&nhlng_path).unwrap(), 0);
    let roster_after = read_vjla_roster(&std::fs::read(&roster_path).unwrap(), 0);
    assert_eq!(nhlng_after, target, "edit landed in nhlng.db");
    assert_eq!(roster_after, target, "edit mirrored into the roster save");
    assert_ne!(roster_after, baseline, "roster value actually changed");
}

#[test]
fn undo_reverts_both_files_to_baseline() {
    let tmp = Tmp::new("undo");
    let mut ws = open_ws(&tmp);
    let nhlng_path = tmp.0.join("nhlng.db");
    let roster_path = tmp.0.join("ROSTER-SYS-DATA");

    let nhlng_orig = std::fs::read(&nhlng_path).unwrap();
    let roster_orig = std::fs::read(&roster_path).unwrap();
    let baseline = read_vjla(&nhlng_orig, 0);

    ws.set_field(
        "ubPc",
        0,
        "vjla",
        FieldValue::Int(if baseline == 2 { 3 } else { 2 }),
    )
    .unwrap();
    assert!(ws.undo(), "undo returns true");
    assert!(
        !ws.is_dirty(),
        "undo restores the loaded baseline on both files"
    );

    ws.save_in_place().unwrap();
    assert_eq!(
        std::fs::read(&nhlng_path).unwrap(),
        nhlng_orig,
        "nhlng.db back to baseline"
    );
    // The roster repacks; its inner db must match the original inner db after undo.
    let inner_now = SaveContainer::parse(&std::fs::read(&roster_path).unwrap())
        .unwrap()
        .extract_db()
        .unwrap();
    let inner_orig = SaveContainer::parse(&roster_orig)
        .unwrap()
        .extract_db()
        .unwrap();
    assert_eq!(inner_now, inner_orig, "roster inner db back to baseline");
}

#[test]
fn display_toggle_serves_roster_and_edits_still_fan_out() {
    let tmp = Tmp::new("display");
    let mut ws = open_ws(&tmp);
    let nhlng_path = tmp.0.join("nhlng.db");
    let roster_path = tmp.0.join("ROSTER-SYS-DATA");

    // Default: display == canonical session.
    assert!(!ws.display_roster());
    assert!(std::ptr::eq(ws.display(), ws.primary()));

    // Toggle on: reads now come from the roster session (distinct from canonical).
    ws.set_display_roster(true).unwrap();
    assert!(ws.display_roster());
    assert!(!std::ptr::eq(ws.display(), ws.primary()));
    // Aligned pair → the displayed roster shows the same cell values as the canonical db.
    let baseline = read_vjla(&std::fs::read(&nhlng_path).unwrap(), 0);
    let db = TdbFile::parse(ws.display().bytes()).unwrap();
    let t = db.table_canonical("ubPc").unwrap();
    let f = t
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == "vjla")
        .unwrap();
    assert_eq!(db.read_u64(t, 0, f).unwrap(), baseline);

    // An edit made while displaying the roster still lands in BOTH files.
    let target = if baseline == 2 { 3 } else { 2 };
    ws.set_field("ubPc", 0, "vjla", FieldValue::Int(target as i64))
        .unwrap();
    ws.save_in_place().unwrap();
    assert_eq!(read_vjla(&std::fs::read(&nhlng_path).unwrap(), 0), target);
    assert_eq!(
        read_vjla_roster(&std::fs::read(&roster_path).unwrap(), 0),
        target
    );

    // Toggle back off restores the canonical session as the read source.
    ws.set_display_roster(false).unwrap();
    assert!(std::ptr::eq(ws.display(), ws.primary()));
}

/// Open a workspace whose pair is deliberately misaligned: grow `nhlng.db` by one player before
/// pairing it with the untouched roster save (shared-table record counts then differ).
fn open_misaligned_ws(tmp: &Tmp) -> Workspace {
    let nhlng_path = tmp.copy_in(&repo(NHLNG), "nhlng.db");
    let roster_path = tmp.copy_in(&repo(ROSTER), "ROSTER-SYS-DATA");
    let mut grow = EditSession::begin(
        Project::load(
            &nhlng_path,
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    let team = grow
        .model()
        .unwrap()
        .slots
        .iter()
        .find(|s| s.player_id.is_some())
        .unwrap()
        .team_id;
    grow.create_player(team, 0).unwrap();
    grow.save_to(&nhlng_path).unwrap();

    let nhlng = EditSession::begin(
        Project::load(
            &nhlng_path,
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    let roster = EditSession::begin(
        Project::load(
            &roster_path,
            Platform::Ps3,
            ProjectRole::Roster,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    Workspace::with_roster(nhlng, nhlng_path, roster, roster_path).unwrap()
}

#[test]
fn misaligned_display_enters_roster_only_mode() {
    let tmp = Tmp::new("roster-only");
    let mut ws = open_misaligned_ws(&tmp);
    assert!(ws.roster_attached());
    assert!(
        !ws.roster_aligned(),
        "the grown nhlng must break record alignment"
    );

    // The toggle is allowed despite the misalignment, and reads switch to the roster session.
    ws.set_display_roster(true).unwrap();
    assert!(ws.display_roster());
    assert!(ws.roster_only());
    assert!(!std::ptr::eq(ws.display(), ws.primary()));

    // A field edit applies strictly to the roster file; nhlng.db stays clean.
    let roster_baseline = {
        let db = TdbFile::parse(ws.display().bytes()).unwrap();
        let t = db.table_canonical("ubPc").unwrap();
        let f = t
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == "vjla")
            .unwrap();
        db.read_u64(t, 0, f).unwrap()
    };
    let target = if roster_baseline == 2 { 3 } else { 2 };
    ws.set_field("ubPc", 0, "vjla", FieldValue::Int(target as i64))
        .unwrap();
    assert!(
        !ws.primary().is_dirty(),
        "nhlng.db untouched in roster-only mode"
    );
    assert!(
        ws.display().is_dirty(),
        "the roster session carries the edit"
    );

    // Coordinated history: undo/redo pop the roster edit without touching nhlng state.
    assert!(ws.undo());
    assert!(!ws.is_dirty());
    assert!(ws.redo());

    // Save: only the roster file's content changes on disk.
    let nhlng_before = std::fs::read(tmp.0.join("nhlng.db")).unwrap();
    ws.save_in_place().unwrap();
    assert_eq!(
        std::fs::read(tmp.0.join("nhlng.db")).unwrap(),
        nhlng_before,
        "nhlng.db bytes unchanged"
    );
    assert_eq!(
        read_vjla_roster(&std::fs::read(tmp.0.join("ROSTER-SYS-DATA")).unwrap(), 0),
        target,
        "edit landed in the roster save"
    );

    // Toggling back re-displays the canonical db and leaves roster-only mode.
    ws.set_display_roster(false).unwrap();
    assert!(std::ptr::eq(ws.display(), ws.primary()));
    assert!(!ws.roster_only());
}

#[test]
fn display_toggle_refused_without_roster() {
    // A single-file workspace has nothing to display but the canonical db.
    let tmp = Tmp::new("display-single");
    let nhlng_path = tmp.copy_in(&repo(NHLNG), "nhlng.db");
    let nhlng = EditSession::begin(
        Project::load(
            &nhlng_path,
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    let mut ws = Workspace::single(nhlng, nhlng_path);
    assert!(ws.set_display_roster(true).is_err());
    assert!(!ws.display_roster());
    // Turning it off is always fine.
    ws.set_display_roster(false).unwrap();
}

#[test]
fn entity_op_mirrors_and_create_guard_passes() {
    let tmp = Tmp::new("entity");
    let mut ws = open_ws(&tmp);

    // A captaincy edit (entity-id op) fans out without error and dirties the workspace.
    let model = ws.primary().model().unwrap();
    let slot = model.slots.iter().find(|s| s.player_id.is_some()).unwrap();
    let (team, player) = (slot.team_id, slot.player_id.unwrap());
    ws.set_captaincy(team, player, 1).unwrap();
    assert!(ws.is_dirty());

    // create_player on both files: the strict guard must pass (aligned anchors → matching wBIz/index).
    let (new_id, outcome) = ws.create_player(team, 0).unwrap();
    assert!(new_id > 0);
    assert!(outcome.edits >= 1);
}

#[test]
fn save_in_place_round_trips_valid_files() {
    let tmp = Tmp::new("roundtrip");
    let mut ws = open_ws(&tmp);
    let nhlng_path = tmp.0.join("nhlng.db");
    let roster_path = tmp.0.join("ROSTER-SYS-DATA");

    ws.set_field("ubPc", 0, "vjla", FieldValue::Int(1)).unwrap();
    ws.save_in_place().unwrap();

    // nhlng.db re-parses and is CRC-valid.
    let nhlng_bytes = std::fs::read(&nhlng_path).unwrap();
    let db = TdbFile::parse(&nhlng_bytes).unwrap();
    assert!(db.header_crc_ok() && db.size_ok());

    // The roster repacks into a valid container.
    let container_bytes = std::fs::read(&roster_path).unwrap();
    let container = SaveContainer::parse(&container_bytes).unwrap();
    assert!(
        container.checksums_valid(),
        "repacked roster container checksums verify"
    );
    TdbFile::parse(&container.extract_db().unwrap()).unwrap();
}

// --- Roster-block auto-repair (both files checked independently) ---

/// Retag one strictly-interior `eGlu` slot to a foreign team — the block-coherence corruption the
/// old `dXSB`-retag `move_player` produced — and fix the db CRCs.
fn corrupt_eglu(db_bytes: &mut [u8]) {
    let view = db_bytes.to_vec();
    let db = TdbFile::parse(&view).unwrap();
    let t = db.table_canonical("eGlu").unwrap();
    let f = t
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == "dXSB")
        .unwrap();
    let teams: Vec<u64> = (0..t.current_records as u32)
        .map(|i| db.read_u64(t, i, f).unwrap())
        .collect();
    // First record whose neighbours share its team (interior of a block), retagged to the last
    // record's team (a different block by construction on the vanilla anchor).
    let victim = (1..teams.len() - 1)
        .find(|&i| teams[i - 1] == teams[i] && teams[i] == teams[i + 1])
        .expect("an interior slot");
    let foreign = *teams.last().unwrap();
    assert_ne!(teams[victim], foreign);
    assert!(db.set_u64(db_bytes, t, victim as u32, f, foreign));
    db.recompute_crcs(db_bytes);
}

fn towx_clean(db_bytes: &[u8]) -> bool {
    tdb_domain_validation::towx_coherence(&TdbFile::parse(db_bytes).unwrap()).is_empty()
}

/// Corruption in the ROSTER SAVE only: the workspace repair must find and heal it even though the
/// main db is clean (each file is checked from its own bytes).
#[test]
fn block_repair_heals_a_corrupt_roster_save_under_a_clean_main_db() {
    let tmp = Tmp::new("blockfix-roster");
    // Corrupt the roster save's inner db on disk before the workspace opens it.
    let roster_file = repo(ROSTER);
    let original = std::fs::read(&roster_file).unwrap();
    let mut inner = SaveContainer::parse(&original)
        .unwrap()
        .extract_db()
        .unwrap();
    corrupt_eglu(&mut inner);
    let corrupted = tdb_savedata::repack_db(&original, &inner).unwrap();
    let roster_path = tmp.0.join("ROSTER-SYS-DATA");
    std::fs::write(&roster_path, &corrupted).unwrap();

    let nhlng_path = tmp.copy_in(&repo(NHLNG), "nhlng.db");
    let nhlng = EditSession::begin(
        Project::load(
            &nhlng_path,
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    let roster = EditSession::begin(
        Project::load(
            roster_path.to_str().unwrap(),
            Platform::Ps3,
            ProjectRole::Roster,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    let mut ws = Workspace::with_roster(
        nhlng,
        nhlng_path.clone(),
        roster,
        roster_path.to_string_lossy().into_owned(),
    )
    .unwrap();

    let (_, summary) = ws.repair_roster_blocks().unwrap();
    assert!(
        summary.moved_slots > 0,
        "the corrupt roster save must be repaired"
    );

    ws.save_in_place().unwrap();
    let healed = SaveContainer::parse(&std::fs::read(&roster_path).unwrap())
        .unwrap()
        .extract_db()
        .unwrap();
    assert!(
        towx_clean(&healed),
        "roster save block-coherent after repair"
    );
    assert!(
        towx_clean(&std::fs::read(&nhlng_path).unwrap()),
        "main db untouched and clean"
    );
}

/// Corruption in BOTH files: one call heals both, and a second call is a no-op.
#[test]
fn block_repair_heals_both_files_and_is_idempotent() {
    let tmp = Tmp::new("blockfix-both");
    // Corrupt the main db copy.
    let nhlng_path = tmp.copy_in(&repo(NHLNG), "nhlng.db");
    let mut main_bytes = std::fs::read(&nhlng_path).unwrap();
    corrupt_eglu(&mut main_bytes);
    std::fs::write(&nhlng_path, &main_bytes).unwrap();
    // Corrupt the roster save copy the same way.
    let original = std::fs::read(repo(ROSTER)).unwrap();
    let mut inner = SaveContainer::parse(&original)
        .unwrap()
        .extract_db()
        .unwrap();
    corrupt_eglu(&mut inner);
    let corrupted = tdb_savedata::repack_db(&original, &inner).unwrap();
    let roster_path = tmp.0.join("ROSTER-SYS-DATA");
    std::fs::write(&roster_path, &corrupted).unwrap();

    let nhlng = EditSession::begin(
        Project::load(
            &nhlng_path,
            Platform::Ps3,
            ProjectRole::Nhlng,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    let roster = EditSession::begin(
        Project::load(
            roster_path.to_str().unwrap(),
            Platform::Ps3,
            ProjectRole::Roster,
            repo(META),
            repo(DEFS),
        )
        .unwrap(),
    );
    let mut ws = Workspace::with_roster(
        nhlng,
        nhlng_path.clone(),
        roster,
        roster_path.to_string_lossy().into_owned(),
    )
    .unwrap();

    let (_, summary) = ws.repair_roster_blocks().unwrap();
    assert!(summary.moved_slots > 0);

    // Idempotent: a second pass finds both files coherent.
    let (_, again) = ws.repair_roster_blocks().unwrap();
    assert_eq!(again.moved_slots, 0);

    ws.save_in_place().unwrap();
    assert!(
        towx_clean(&std::fs::read(&nhlng_path).unwrap()),
        "main db healed"
    );
    let healed = SaveContainer::parse(&std::fs::read(&roster_path).unwrap())
        .unwrap()
        .extract_db()
        .unwrap();
    assert!(towx_clean(&healed), "roster save healed");
}

// ------------------------------------------------------------------ P1 audit (21 Sep 2026)

/// A coordinated op whose roster half is refused must be rolled back as if it never ran: nhlng
/// bytes back, history untouched, and **nothing redoable** — before this audit the canonical
/// half was `undo()`ne, which parked it on the session's redo stack; with a redo entry already
/// on the workspace stack from an earlier real undo, a Redo then re-applied the half-written op
/// as if it were the earlier one. The roster refusal is produced deterministically through the
/// session's fault-injection seam (the coherence and referential gates are delta gates, so no
/// stock pair refuses an unrelated mirrored edit).
#[test]
fn a_refused_roster_mirror_leaves_nothing_redoable_and_history_untouched() {
    let tmp = Tmp::new("mirror-refused");
    let mut ws = open_ws(&tmp);
    assert!(ws.roster_aligned());
    let baseline = read_vjla(ws.primary().bytes(), 0) as i64;
    let flipped = if baseline == 2 { 3 } else { 2 };
    // A real op, undone: a redo entry exists on the workspace and on both sessions.
    ws.set_field("ubPc", 0, "vjla", FieldValue::Int(flipped)).unwrap();
    assert!(ws.undo());
    assert_eq!((ws.undo_depth(), ws.redo_depth(), ws.primary().redo_depth()), (0, 1, 1));
    let before = ws.primary().bytes().to_vec();
    let roster_before = ws.roster_session().unwrap().bytes().to_vec();
    assert!(ws.inject_roster_fault(Some("roster refuses".into())));
    for (label, r) in [
        ("set_cells_int", ws.set_cells_int("ubPc", 0, &[("vjla".into(), flipped)]).map(|_| ())),
    ] {
        let e = r.unwrap_err();
        assert!(e.to_string().contains("roster refuses"), "{label}: {e}");
    }
    assert_eq!(ws.primary().bytes(), &before[..], "nhlng bytes rolled back");
    assert_eq!(ws.roster_session().unwrap().bytes(), &roster_before[..], "roster untouched");
    assert_eq!(ws.undo_depth(), 0, "history untouched");
    assert!(!ws.is_dirty());
    // Nothing redoable: the earlier undone op is gone (the refused op's transaction discarded
    // it on the session, and the workspace agrees), and the refused op's canonical half is not
    // a redo entry either.
    assert_eq!((ws.redo_depth(), ws.primary().redo_depth()), (0, 0), "no phantom redo on either level");
    assert!(!ws.redo(), "redo has nothing to re-apply");
    assert_eq!(ws.primary().bytes(), &before[..], "redo changed nothing");
    assert_eq!(ws.undo_depth(), 0, "a no-op redo adds no history");
    // The same contract through set_field, set_cells_int_rows and an entity op.
    assert!(ws.set_field("ubPc", 0, "vjla", FieldValue::Int(flipped)).unwrap_err().to_string().contains("roster refuses"));
    assert!(ws.set_cells_int_rows("ubPc", &[(0, vec![("vjla".into(), flipped)])]).unwrap_err().to_string().contains("roster refuses"));
    let team = ws.primary().model().unwrap().slots.iter().find(|s| s.player_id.is_some()).unwrap().team_id;
    let player = ws.primary().model().unwrap().slots.iter().find(|s| s.player_id.is_some()).unwrap().player_id.unwrap();
    assert!(ws.set_captaincy(team, player, 1).is_err());
    assert_eq!((ws.undo_depth(), ws.redo_depth(), ws.primary().redo_depth()), (0, 0, 0));
    assert_eq!(ws.primary().bytes(), &before[..]);
    assert_eq!(ws.roster_session().unwrap().bytes(), &roster_before[..]);
    // The fault cleared: a mirrored op works again and undoes as one step on both files.
    ws.inject_roster_fault(None);
    ws.set_field("ubPc", 0, "vjla", FieldValue::Int(flipped)).unwrap();
    assert_eq!(ws.undo_depth(), 1);
    assert_eq!(read_vjla(ws.roster_session().unwrap().bytes(), 0) as i64, flipped, "mirrored");
    assert!(ws.undo());
    assert!(!ws.is_dirty());
    assert_eq!(ws.redo_depth(), 1);
    assert!(ws.redo());
    assert_eq!(read_vjla(ws.primary().bytes(), 0) as i64, flipped);
    assert_eq!(read_vjla(ws.roster_session().unwrap().bytes(), 0) as i64, flipped);
}

/// `set_cells_int` refuses an empty write set and a field named twice in one op before touching
/// anything; a valid multi-cell write is one undo step that restores every cell.
#[test]
fn set_cells_int_refuses_empty_and_duplicate_fields_and_undoes_as_one_step() {
    let tmp = Tmp::new("cells");
    let mut ws = open_ws(&tmp);
    let before = ws.primary().bytes().to_vec();
    assert!(ws.set_cells_int("ubPc", 0, &[]).is_err());
    let baseline = read_vjla(&before, 0) as i64;
    let flipped = if baseline == 2 { 3 } else { 2 };
    let e = ws.set_cells_int("ubPc", 0, &[("vjla".into(), flipped), ("vjla".into(), baseline)]).unwrap_err();
    assert!(e.to_string().contains("twice"), "{e}");
    assert_eq!(ws.primary().bytes(), &before[..]);
    assert_eq!(ws.undo_depth(), 0);
    ws.set_cells_int("ubPc", 0, &[("vjla".into(), flipped)]).unwrap();
    assert_eq!(read_vjla(ws.primary().bytes(), 0) as i64, flipped);
    assert_eq!(ws.undo_depth(), 1);
    assert!(ws.undo());
    assert_eq!(ws.primary().bytes(), &before[..]);
}

// ------------------------------------------------------------------ Phase 2.1 art-id swap (21 Sep 2026)

/// The art-id swap under the re-identification contract on the NHL 14 PS3 pair: the dependency
/// listing counts the index / art-id consumers; the swap writes exactly the two team rows and the
/// jersey rows of both teams (each row's `teamartid` follows its team), as ONE undo step, mirrored
/// into the roster save; every other byte of both files is untouched; undo restores both
/// byte-exact; the refusals (same team, roster-only) leave nothing changed; the profile shows the
/// art id read-only.
#[test]
fn artid_swap_cascades_the_jersey_rows_of_both_teams_as_one_mirrored_step() {
    use studio_domain::team_reid::{plan_artid_swap, team_identity_dependencies};
    let tmp = Tmp::new("artid-swap");
    let mut ws = open_ws(&tmp);
    let teams = ws.primary().team_summaries().unwrap();
    let (a, b) = (teams[0].team_id, teams[1].team_id);
    let deps = team_identity_dependencies(ws.primary(), a).unwrap();
    assert!(deps.artid.is_some());
    assert!(deps.index_dependencies.iter().any(|d| d.table == "exhibitionrostertable" && d.rows > 0), "{:?}", deps.index_dependencies);
    assert!(deps.artid_dependencies.iter().any(|d| d.table == "stockteamjerseys" && d.rows > 0), "{:?}", deps.artid_dependencies);
    assert!(deps.index_change.starts_with("refused"));
    assert!(deps.teams_sharing_artid.is_empty());
    let plan = plan_artid_swap(ws.primary(), a, b).unwrap();
    assert_ne!(plan.artid_a, plan.artid_b);
    assert!(plan.tables.iter().any(|(t, n)| t == "stockteamjerseys" && *n > 0), "{:?}", plan.tables);
    assert!(plan.total_cells >= 2 + 2);
    let jersey_rows: usize = plan.tables.iter().filter(|(t, _)| t != "exhibitionteams").map(|(_, n)| *n).sum();
    let nhlng_before = ws.primary().bytes().to_vec();
    let roster_before = ws.roster_session().unwrap().bytes().to_vec();
    let (p2, outcome) = ws.apply_artid_swap(a, b).unwrap();
    assert_eq!(p2.summary, plan.summary);
    assert_eq!(outcome.edits as usize, plan.total_cells);
    assert_eq!(ws.undo_depth(), 1, "one undo step");
    // Read back: the two team rows swapped, the jersey rows follow, everything else verbatim.
    let after = ws.primary().bytes().to_vec();
    let db0 = TdbFile::parse(&nhlng_before).unwrap();
    let db1 = TdbFile::parse(&after).unwrap();
    let meta = ws.primary().project().meta();
    let kott0 = db0.table_canonical("kOtt").unwrap();
    let kott1 = db1.table_canonical("kOtt").unwrap();
    let art = kott0.fields.iter().find(|f| meta.field(&kott0.id.raw(), &f.id.raw()).map(|m| m.name == "artid").unwrap_or(false)).unwrap();
    assert_eq!(db1.read_u64(kott1, teams[0].record, art), Some(plan.artid_b));
    assert_eq!(db1.read_u64(kott1, teams[1].record, art), Some(plan.artid_a));
    let deps_after = team_identity_dependencies(ws.primary(), a).unwrap();
    assert_eq!(deps_after.artid, Some(plan.artid_b));
    assert!(deps_after.teams_sharing_artid.is_empty(), "no shared art id after the swap");
    // Jersey rows: the changed cells are exactly the planned count; count them from the bytes.
    let jt0 = db0.tables.iter().find(|t| meta.table_name(&t.id.raw()) == Some("stockteamjerseys")).unwrap();
    let jt1 = db1.table_canonical(&jt0.id.canonical(db0.endian)).unwrap();
    let tf = jt0.fields.iter().find(|f| meta.field(&jt0.id.raw(), &f.id.raw()).map(|m| m.name == "teamid").unwrap_or(false)).unwrap();
    let af = jt0.fields.iter().find(|f| meta.field(&jt0.id.raw(), &f.id.raw()).map(|m| m.name == "teamartid").unwrap_or(false)).unwrap();
    let mut changed = 0;
    for r in 0..jt0.current_records as u32 {
        let (v0, v1) = (db0.read_u64(jt0, r, af), db1.read_u64(jt1, r, af));
        let owner = db0.read_u64(jt0, r, tf);
        if v0 != v1 {
            changed += 1;
            assert!(owner == Some(a) || owner == Some(b), "row {r}: only the two teams' rows change");
            assert_eq!(v1, Some(if owner == Some(a) { plan.artid_b } else { plan.artid_a }));
        } else {
            assert!(owner != Some(a) && owner != Some(b) || v0 != Some(plan.artid_a) && v0 != Some(plan.artid_b), "row {r}: a row of the swapped teams carrying their art id must follow");
        }
    }
    assert!(changed > 0 && changed <= jersey_rows);
    // Mirrored into the roster (the team table is shared).
    let rdb = TdbFile::parse(ws.roster_session().unwrap().bytes()).unwrap();
    let rk = rdb.table_canonical("kOtt").unwrap();
    assert_eq!(rdb.read_u64(rk, teams[0].record, art), Some(plan.artid_b), "roster mirror");
    assert!(ws.roster_session().unwrap().bytes() != &roster_before[..]);
    // Undo: both files byte-exact.
    assert!(ws.undo());
    assert_eq!(ws.primary().bytes(), &nhlng_before[..]);
    assert_eq!(ws.roster_session().unwrap().bytes(), &roster_before[..]);
    assert!(!ws.is_dirty());
    // Refusals leave nothing changed.
    assert!(ws.apply_artid_swap(a, a).unwrap_err().to_string().contains("two different teams"));
    assert!(ws.apply_artid_swap(a, 999_999).unwrap_err().to_string().contains("no team"));
    assert_eq!(ws.primary().bytes(), &nhlng_before[..]);
    assert_eq!(ws.undo_depth(), 0);
    // A bare cell write of the art id is refused with the contract reason (set_field and the
    // bulk path), nothing changes.
    let art_studio = art.id.canonical(db0.endian);
    let e = ws.set_field("kOtt", teams[0].record, &art_studio, FieldValue::Int(plan.artid_b as i64)).unwrap_err().to_string();
    assert!(e.contains("swap art ids"), "{e}");
    let e = ws.set_cells_int_rows("kOtt", &[(teams[0].record, vec![(art_studio.clone(), plan.artid_b as i64)])]).unwrap_err().to_string();
    assert!(e.contains("swap art ids"), "{e}");
    assert_eq!(ws.primary().bytes(), &nhlng_before[..]);
    // The profile shows the art id read-only.
    let view = ws.primary().team_view(a).unwrap().expect("team view");
    let art_cell = view.profile.sections.iter().flat_map(|s| s.cells.iter()).find(|c| c.label == "Art ID").expect("Art ID cell");
    assert!(!art_cell.field.as_ref().unwrap().editable, "art id is not a bare editable cell");
}
