//! Sync-engine gate: a [`Workspace`] mirrors edits from `nhlng.db` into the roster save and saves
//! both in place. Verified against the nhl14 PS3 anchors (record-index-aligned pair — memory
//! `roster-save-mirrors-nhlng`). All edits are made on **temp copies**; the anchors are never written.

use std::path::{Path, PathBuf};

use studio_domain::{EditSession, FieldValue, Platform, Project, ProjectRole, Workspace};
use tdb_core::TdbFile;
use tdb_savedata::SaveContainer;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
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
        Project::load(&nhlng_path, Platform::Ps3, ProjectRole::Nhlng, repo(META), repo(DEFS)).unwrap(),
    );
    let roster = EditSession::begin(
        Project::load(&roster_path, Platform::Ps3, ProjectRole::Roster, repo(META), repo(DEFS)).unwrap(),
    );
    Workspace::with_roster(nhlng, nhlng_path, roster, roster_path).unwrap()
}

/// Read the raw bits of `ubPc.vjla` (player position) at a record from a raw nhlng db.
fn read_vjla(db_bytes: &[u8], record: u32) -> u64 {
    let db = TdbFile::parse(db_bytes).unwrap();
    let t = db.table_canonical("ubPc").unwrap();
    let f = t.fields.iter().find(|f| f.id.canonical(db.endian) == "vjla").unwrap();
    db.read_u64(t, record, f).unwrap()
}

/// Same read, but for a roster save container (extract the inner db first).
fn read_vjla_roster(file_bytes: &[u8], record: u32) -> u64 {
    let inner = SaveContainer::parse(file_bytes).unwrap().extract_db().unwrap();
    read_vjla(&inner, record)
}

#[test]
fn nhl14_pair_is_attached_and_aligned() {
    let tmp = Tmp::new("status");
    let ws = open_ws(&tmp);
    assert!(ws.roster_attached());
    assert!(ws.roster_aligned(), "nhl14 PS3 nhlng/roster are record-index aligned");
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

    ws.set_field("ubPc", 0, "vjla", FieldValue::Int(target as i64)).unwrap();
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

    ws.set_field("ubPc", 0, "vjla", FieldValue::Int(if baseline == 2 { 3 } else { 2 })).unwrap();
    assert!(ws.undo(), "undo returns true");
    assert!(!ws.is_dirty(), "undo restores the loaded baseline on both files");

    ws.save_in_place().unwrap();
    assert_eq!(std::fs::read(&nhlng_path).unwrap(), nhlng_orig, "nhlng.db back to baseline");
    // The roster repacks; its inner db must match the original inner db after undo.
    let inner_now = SaveContainer::parse(&std::fs::read(&roster_path).unwrap()).unwrap().extract_db().unwrap();
    let inner_orig = SaveContainer::parse(&roster_orig).unwrap().extract_db().unwrap();
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
    let f = t.fields.iter().find(|f| f.id.canonical(db.endian) == "vjla").unwrap();
    assert_eq!(db.read_u64(t, 0, f).unwrap(), baseline);

    // An edit made while displaying the roster still lands in BOTH files.
    let target = if baseline == 2 { 3 } else { 2 };
    ws.set_field("ubPc", 0, "vjla", FieldValue::Int(target as i64)).unwrap();
    ws.save_in_place().unwrap();
    assert_eq!(read_vjla(&std::fs::read(&nhlng_path).unwrap(), 0), target);
    assert_eq!(read_vjla_roster(&std::fs::read(&roster_path).unwrap(), 0), target);

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
        Project::load(&nhlng_path, Platform::Ps3, ProjectRole::Nhlng, repo(META), repo(DEFS)).unwrap(),
    );
    let team = grow.model().unwrap().slots.iter().find(|s| s.player_id.is_some()).unwrap().team_id;
    grow.create_player(team, 0).unwrap();
    grow.save_to(&nhlng_path).unwrap();

    let nhlng = EditSession::begin(
        Project::load(&nhlng_path, Platform::Ps3, ProjectRole::Nhlng, repo(META), repo(DEFS)).unwrap(),
    );
    let roster = EditSession::begin(
        Project::load(&roster_path, Platform::Ps3, ProjectRole::Roster, repo(META), repo(DEFS)).unwrap(),
    );
    Workspace::with_roster(nhlng, nhlng_path, roster, roster_path).unwrap()
}

#[test]
fn misaligned_display_enters_roster_only_mode() {
    let tmp = Tmp::new("roster-only");
    let mut ws = open_misaligned_ws(&tmp);
    assert!(ws.roster_attached());
    assert!(!ws.roster_aligned(), "the grown nhlng must break record alignment");

    // The toggle is allowed despite the misalignment, and reads switch to the roster session.
    ws.set_display_roster(true).unwrap();
    assert!(ws.display_roster());
    assert!(ws.roster_only());
    assert!(!std::ptr::eq(ws.display(), ws.primary()));

    // A field edit applies strictly to the roster file; nhlng.db stays clean.
    let roster_baseline = {
        let db = TdbFile::parse(ws.display().bytes()).unwrap();
        let t = db.table_canonical("ubPc").unwrap();
        let f = t.fields.iter().find(|f| f.id.canonical(db.endian) == "vjla").unwrap();
        db.read_u64(t, 0, f).unwrap()
    };
    let target = if roster_baseline == 2 { 3 } else { 2 };
    ws.set_field("ubPc", 0, "vjla", FieldValue::Int(target as i64)).unwrap();
    assert!(!ws.primary().is_dirty(), "nhlng.db untouched in roster-only mode");
    assert!(ws.display().is_dirty(), "the roster session carries the edit");

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
        Project::load(&nhlng_path, Platform::Ps3, ProjectRole::Nhlng, repo(META), repo(DEFS)).unwrap(),
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
    assert!(container.checksums_valid(), "repacked roster container checksums verify");
    TdbFile::parse(&container.extract_db().unwrap()).unwrap();
}

// --- Roster-block auto-repair (both files checked independently) ---

/// Retag one strictly-interior `eGlu` slot to a foreign team — the block-coherence corruption the
/// old `dXSB`-retag `move_player` produced — and fix the db CRCs.
fn corrupt_eglu(db_bytes: &mut Vec<u8>) {
    let view = db_bytes.clone();
    let db = TdbFile::parse(&view).unwrap();
    let t = db.table_canonical("eGlu").unwrap();
    let f = t.fields.iter().find(|f| f.id.canonical(db.endian) == "dXSB").unwrap();
    let teams: Vec<u64> =
        (0..t.current_records as u32).map(|i| db.read_u64(t, i, f).unwrap()).collect();
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
    let mut inner = SaveContainer::parse(&original).unwrap().extract_db().unwrap();
    corrupt_eglu(&mut inner);
    let corrupted = tdb_savedata::repack_db(&original, &inner).unwrap();
    let roster_path = tmp.0.join("ROSTER-SYS-DATA");
    std::fs::write(&roster_path, &corrupted).unwrap();

    let nhlng_path = tmp.copy_in(&repo(NHLNG), "nhlng.db");
    let nhlng = EditSession::begin(
        Project::load(&nhlng_path, Platform::Ps3, ProjectRole::Nhlng, repo(META), repo(DEFS)).unwrap(),
    );
    let roster = EditSession::begin(
        Project::load(roster_path.to_str().unwrap(), Platform::Ps3, ProjectRole::Roster, repo(META), repo(DEFS))
            .unwrap(),
    );
    let mut ws =
        Workspace::with_roster(nhlng, nhlng_path.clone(), roster, roster_path.to_string_lossy().into_owned())
            .unwrap();

    let (_, summary) = ws.repair_roster_blocks().unwrap();
    assert!(summary.moved_slots > 0, "the corrupt roster save must be repaired");

    ws.save_in_place().unwrap();
    let healed = SaveContainer::parse(&std::fs::read(&roster_path).unwrap()).unwrap().extract_db().unwrap();
    assert!(towx_clean(&healed), "roster save block-coherent after repair");
    assert!(towx_clean(&std::fs::read(&nhlng_path).unwrap()), "main db untouched and clean");
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
    let mut inner = SaveContainer::parse(&original).unwrap().extract_db().unwrap();
    corrupt_eglu(&mut inner);
    let corrupted = tdb_savedata::repack_db(&original, &inner).unwrap();
    let roster_path = tmp.0.join("ROSTER-SYS-DATA");
    std::fs::write(&roster_path, &corrupted).unwrap();

    let nhlng = EditSession::begin(
        Project::load(&nhlng_path, Platform::Ps3, ProjectRole::Nhlng, repo(META), repo(DEFS)).unwrap(),
    );
    let roster = EditSession::begin(
        Project::load(roster_path.to_str().unwrap(), Platform::Ps3, ProjectRole::Roster, repo(META), repo(DEFS))
            .unwrap(),
    );
    let mut ws =
        Workspace::with_roster(nhlng, nhlng_path.clone(), roster, roster_path.to_string_lossy().into_owned())
            .unwrap();

    let (_, summary) = ws.repair_roster_blocks().unwrap();
    assert!(summary.moved_slots > 0);

    // Idempotent: a second pass finds both files coherent.
    let (_, again) = ws.repair_roster_blocks().unwrap();
    assert_eq!(again.moved_slots, 0);

    ws.save_in_place().unwrap();
    assert!(towx_clean(&std::fs::read(&nhlng_path).unwrap()), "main db healed");
    let healed = SaveContainer::parse(&std::fs::read(&roster_path).unwrap()).unwrap().extract_db().unwrap();
    assert!(towx_clean(&healed), "roster save healed");
}
