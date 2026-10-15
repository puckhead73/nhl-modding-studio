//! Pending-change review (O3 / L1.1-D): baseline ↔ working, against the acknowledged baseline.

use std::path::{Path, PathBuf};

use studio_domain::{EditSession, FieldValue, Platform, Project, ProjectRole, Workspace};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

const NHLNG: &str = "anchors/ps3/nhl14/databases/nhlng.db";
const META: &str = "anchors/ps3/nhl14/databases/nhlng-meta.xml";
const ROSTER: &str = "anchors/ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA";
const NHL21: &str = "anchors/ps4/nhl21/databases/nhl_game.db";
const NHL21_META: &str = "anchors/ps4/nhl21/databases/nhl_game-meta.xml";

#[test]
fn nhl07_free_agent_capacity_growth_does_not_report_relocated_checksum() {
    let path = repo("anchors/xbox360/nhl07/databases/NHL07NG.db");
    let project = Project::load(path.to_str().unwrap(), Platform::Xbox, ProjectRole::Nhlng,
        repo("anchors/xbox360/nhl07/databases/NHL07NG-meta.xml"), repo("defs")).unwrap();
    let base = project.db_bytes();
    let db = tdb_core::TdbFile::parse(base).unwrap();
    let pool = db.table_canonical("VoEQ").unwrap();
    let grown = db.grow_table_max(&pool.id.raw(), pool.max_records + 1).unwrap();
    let review = |a: &[u8], b: &[u8]| studio_domain::pending::pending_changes(a, b, project.meta(), project.enums(), None, 1000);
    assert_eq!(grown.len(), base.len() + 4);
    assert_eq!(review(base, &grown).integrity.other_bytes, 0);
    assert_eq!(review(&grown, base).integrity.other_bytes, 0, "undoing growth must also recognize the footer");
    assert!(review(base, &grown).tables.iter().any(|t| t.studio_table == "VoEQ"));
    let mut damaged = grown.clone();
    *damaged.last_mut().unwrap() ^= 1;
    assert!(review(base, &damaged).integrity.other_bytes > 0, "invalid checksum must not be hidden");
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-pending-{tag}-{}", std::process::id()));
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

fn session(
    path: &str,
    platform: Platform,
    role: ProjectRole,
    meta: &str,
    game: &str,
) -> EditSession {
    let mut s =
        EditSession::begin(Project::load(path, platform, role, repo(meta), repo("defs")).unwrap());
    s.set_game_id(Some(game.to_string()));
    s
}

#[test]
fn nhl21_league_edit_reviews_as_one_cell_with_entity_names() {
    let tmp = Tmp::new("nhl21");
    let path = tmp.copy_in(&repo(NHL21), "nhl_game.db");
    let mut s = session(
        &path,
        Platform::Ps4,
        ProjectRole::Nhlng,
        NHL21_META,
        "nhl21",
    );
    let clean = s.pending_changes(1000);
    assert!(
        !clean.dirty && clean.cells.is_empty() && clean.tables.is_empty() && clean.error.is_none()
    );

    s.set_field("fsLK", 8, "vIys", FieldValue::Text("TXT_ROW_LEAGUE".into()))
        .unwrap();
    let p = s.pending_changes(1000);
    assert!(p.dirty && p.error.is_none());
    assert_eq!(
        p.total_cells,
        1,
        "{:?}",
        p.cells
            .iter()
            .map(|c| (&c.entity, &c.field_name))
            .collect::<Vec<_>>()
    );
    let c = &p.cells[0];
    assert_eq!(c.studio_table, "fsLK");
    assert_eq!(c.table_name, "stockleagues");
    assert_eq!(c.record, 8);
    assert_eq!(
        c.entity, "TXT_ROW_LEAGUE",
        "labelled with the league's current display text"
    );
    assert_eq!(c.field_name, "name");
    assert_eq!(c.before.display, "Allsvenskan");
    assert_eq!(c.after.display, "TXT_ROW_LEAGUE");
    assert!(p.tables.is_empty(), "no structural change");
    // Derived integrity bytes are reported apart from the edit, never as a cell.
    assert!(
        p.integrity.file_header || !p.integrity.table_headers.is_empty(),
        "a CRC or header must have moved with the edit"
    );
    assert_eq!(p.integrity.other_bytes, 0);

    // Repeated edits to one field collapse to the net difference; a net-zero edit disappears.
    s.set_field("fsLK", 8, "vIys", FieldValue::Text("TXT_ECHL".into()))
        .unwrap();
    let p = s.pending_changes(1000);
    assert_eq!(p.total_cells, 1);
    assert_eq!(p.cells[0].after.display, "TXT_ECHL");
    s.set_field("fsLK", 8, "vIys", FieldValue::Text("Allsvenskan".into()))
        .unwrap();
    let p = s.pending_changes(1000);
    assert!(
        !p.dirty && p.cells.is_empty(),
        "net-zero edit sequence shows nothing"
    );

    // Rejected edits leave the review untouched.
    assert!(s
        .set_field("fsLK", 8, "vIys", FieldValue::Text("X".repeat(26)))
        .is_err());
    assert!(!s.pending_changes(1000).dirty);

    // Team-entity labelling and undo/redo.
    let teams = s.team_summaries().unwrap();
    let aik = teams.iter().find(|t| t.abbrev == "AIK").unwrap();
    s.set_field("kOtt", aik.record, "xsnn", FieldValue::Text("AIKX".into()))
        .unwrap();
    let p = s.pending_changes(1000);
    assert_eq!(p.total_cells, 1);
    assert_eq!(p.cells[0].entity, "AIK");
    assert_eq!(p.cells[0].field_name, "abbrname");
    assert!(s.undo());
    assert!(!s.pending_changes(1000).dirty, "edit → undo is clean");
    assert!(s.redo());
    assert_eq!(
        s.pending_changes(1000).total_cells,
        1,
        "edit → redo is pending again"
    );
    assert!(s.undo());

    // A truncated list keeps the exact count.
    for rec in 0..5u32 {
        s.set_field("fsLK", rec, "XMjH", FieldValue::Int(30 - rec as i64))
            .unwrap();
    }
    let p = s.pending_changes(2);
    assert_eq!(p.total_cells, 5);
    assert_eq!(p.cells.len(), 2);
    assert!(p.truncated);
}

#[test]
fn save_rebinds_the_baseline_and_undo_past_it_is_pending() {
    let tmp = Tmp::new("save");
    let nhlng_path = tmp.copy_in(&repo(NHLNG), "nhlng.db");
    let roster_path = tmp.copy_in(&repo(ROSTER), "ROSTER-SYS-DATA");
    let nhlng = session(
        &nhlng_path,
        Platform::Ps3,
        ProjectRole::Nhlng,
        META,
        "nhl14",
    );
    let roster = session(
        &roster_path,
        Platform::Ps3,
        ProjectRole::Roster,
        META,
        "nhl14",
    );
    let mut ws = Workspace::with_roster(nhlng, nhlng_path.clone(), roster, roster_path).unwrap();

    ws.set_field("kOtt", 0, "xsnn", FieldValue::Text("ZZZ".into()))
        .unwrap();
    let (primary, roster) = ws.pending_changes(1000);
    assert_eq!(primary.total_cells, 1);
    let roster = roster.expect("roster attached");
    assert_eq!(
        roster.total_cells, 1,
        "the mirrored edit is reported once per file role"
    );
    assert_eq!(roster.cells[0].field_name, "abbrname");

    assert_eq!(ws.save_in_place().unwrap(), 2);
    assert!(
        !ws.is_dirty(),
        "save adopts the written files as the baseline"
    );
    let (primary, roster) = ws.pending_changes(1000);
    assert!(!primary.dirty && primary.cells.is_empty());
    assert!(!roster.unwrap().dirty);

    // Undo past the save: pending again, as the reverse change against the saved file.
    assert!(ws.undo());
    assert!(ws.is_dirty());
    let (primary, _) = ws.pending_changes(1000);
    assert_eq!(primary.total_cells, 1);
    assert_eq!(primary.cells[0].before.display, "ZZZ");
    assert_ne!(primary.cells[0].after.display, "ZZZ");

    // Reload from disk adopts the saved file and clears the review.
    ws.reload_from_disk().unwrap();
    assert!(!ws.is_dirty());
    let (primary, _) = ws.pending_changes(1000);
    assert!(!primary.dirty && primary.cells.is_empty());
    // The saved file carries the edit.
    let s2 = session(
        &nhlng_path,
        Platform::Ps3,
        ProjectRole::Nhlng,
        META,
        "nhl14",
    );
    assert_eq!(s2.team_summaries().unwrap()[0].abbrev, "ZZZ");
}

#[test]
fn a_structural_change_is_reported_as_a_table_change() {
    let tmp = Tmp::new("rows");
    let path = tmp.copy_in(&repo(NHL21), "nhl_game.db");
    let mut s = session(
        &path,
        Platform::Ps4,
        ProjectRole::Nhlng,
        NHL21_META,
        "nhl21",
    );
    let teams = s.team_summaries().unwrap();
    let (team, player) = {
        let m = s.model().unwrap();
        let slot = m.slots.iter().find(|x| x.player_id.is_some()).unwrap();
        (slot.team_id, slot.player_id.unwrap())
    };
    let _ = teams;
    let (_new_id, _) = s.clone_player(player, team).unwrap();
    let p = s.pending_changes(50);
    assert!(p.dirty && p.error.is_none());
    let bio = p
        .tables
        .iter()
        .find(|t| t.studio_table == "ubPc")
        .expect("bio table grew");
    assert_eq!(bio.after_records.unwrap(), bio.before_records.unwrap() + 1);
    assert_eq!(bio.rows_only_in_working.len(), 1);
    assert!(bio.note.contains("1 row(s) added"));
}
