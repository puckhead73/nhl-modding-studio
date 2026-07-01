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
