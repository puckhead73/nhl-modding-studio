//! Phase 2.1 (21 Sep 2026) — the team re-identification contract through the registry seam on
//! committed anchors (temp copies): the dependency listing, the art-id swap as one undoable
//! step that the pending review shows, the refusals (same team, unknown team, bare cell edit of
//! the art id through `set_field`), undo restoring the file byte-exact, and the swap on a PS4
//! title whose `stockawardstable` also carries `teamartid`.

use std::path::{Path, PathBuf};

use studio_api::Registry;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-reid-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Tmp(d)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn open(reg: &Registry, tmp: &Tmp, platform: &str, dir: &str, game: &str, stem: &str) -> (u32, PathBuf) {
    let db = tmp.0.join(format!("{stem}.db"));
    std::fs::copy(repo(&format!("anchors/{dir}/{game}/databases/{stem}.db")), &db).unwrap();
    let id = reg
        .open_workspace_from_paths(Some(game), studio_api::parse_platform(platform).unwrap(), &s(&db), None, &s(&repo(&format!("anchors/{dir}/{game}/databases/{stem}-meta.xml"))), &s(&repo("defs")))
        .unwrap()
        .project_id;
    (id, db)
}

#[test]
fn artid_swap_through_the_registry_is_one_undoable_step_with_refusals() {
    for (platform, dir, game, stem) in [("xbox", "xbox360", "nhllegacy", "nhlng"), ("ps4", "ps4", "nhl21", "nhl_game")] {
        let tmp = Tmp::new(game);
        let reg = Registry::default();
        let (id, db) = open(&reg, &tmp, platform, dir, game, stem);
        let teams = reg.list_teams(id).unwrap();
        // Two clubs whose art ids are unique and non-zero (a placeholder row sharing art id 0 with
        // another is exactly what the shared-art refusal is for — exercised below).
        let mut pick = Vec::new();
        for t in &teams {
            let d = reg.team_identity_dependencies(id, t.team_id).unwrap();
            if d.artid.is_some_and(|a| a != 0) && d.teams_sharing_artid.is_empty() {
                pick.push(t.team_id);
            }
            if pick.len() == 2 {
                break;
            }
        }
        let (a, b) = (pick[0], pick[1]);
        if let Some(shared) = teams.iter().find(|t| reg.team_identity_dependencies(id, t.team_id).map(|d| !d.teams_sharing_artid.is_empty()).unwrap_or(false)) {
            let e = match reg.team_artid_swap(id, a, shared.team_id) {
                Ok(_) => panic!("{game}: a swap with a shared art id must be refused"),
                Err(e) => e.to_string(),
            };
            assert!(e.contains("shared art id"), "{game}: {e}");
        }
        let deps = reg.team_identity_dependencies(id, a).unwrap();
        assert!(deps.artid.is_some(), "{game}");
        assert!(deps.index_dependencies.iter().any(|d| d.rows > 0), "{game}: {:?}", deps.index_dependencies);
        assert!(deps.artid_dependencies.iter().any(|d| d.table == "stockteamjerseys" && d.rows > 0), "{game}: {:?}", deps.artid_dependencies);
        if platform == "ps4" {
            assert!(deps.artid_dependencies.iter().any(|d| d.table == "stockawardstable"), "{game}: awards carry teamartid on PS4: {:?}", deps.artid_dependencies);
        }
        let before = std::fs::read(&db).unwrap();
        assert!(!reg.summary(id).unwrap().dirty);
        // Refusals first: nothing changes.
        assert!(reg.team_artid_swap(id, a, a).is_err(), "{game}: same team");
        assert!(reg.team_artid_swap(id, a, 987_654).is_err(), "{game}: unknown team");
        assert!(!reg.summary(id).unwrap().dirty, "{game}: refusals leave the project clean");
        // The swap: one undo step, the pending review reflects it, the art ids are exchanged.
        let r = reg.team_artid_swap(id, a, b).unwrap();
        assert_eq!((r.team_a, r.team_b), (a, b));
        assert_ne!(r.artid_a, r.artid_b);
        assert!(r.tables.iter().any(|(t, n)| t == "exhibitionteams" && *n == 2), "{game}: {:?}", r.tables);
        assert!(r.outcome.edits as usize == r.total_cells && r.total_cells >= 4, "{game}: {}", r.summary);
        let sum = reg.summary(id).unwrap();
        assert!(sum.dirty && sum.undo_depth == 1, "{game}: one step");
        let da = reg.team_identity_dependencies(id, a).unwrap();
        let dbb = reg.team_identity_dependencies(id, b).unwrap();
        assert_eq!((da.artid, dbb.artid), (Some(r.artid_b), Some(r.artid_a)), "{game}: exchanged");
        assert!(da.teams_sharing_artid.is_empty() && dbb.teams_sharing_artid.is_empty());
        // A bare cell edit of the art id is refused with the contract reason.
        let view = reg.team_view(id, a).unwrap().unwrap();
        let art = view.profile.sections.iter().flat_map(|s| s.cells.iter()).find(|c| c.label == "Art ID").expect("Art ID cell");
        let f = art.field.as_ref().unwrap();
        assert!(!f.editable, "{game}: art id read-only in the profile");
        let e = match reg.set_field(id, &f.studio_table, art.record, &f.studio_field, studio_domain::FieldValue::Int(r.artid_a as i64)) {
            Ok(_) => panic!("{game}: a bare art-id edit must be refused"),
            Err(e) => e.to_string(),
        };
        assert!(e.contains("swap art ids"), "{game}: {e}");
        // Undo restores the working bytes; save-to-copy would write them — compare the session.
        assert!(reg.undo(id).unwrap());
        assert!(!reg.summary(id).unwrap().dirty, "{game}: undo → clean");
        let out = tmp.0.join("saved.db");
        reg.save(id, &s(&out)).unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), before, "{game}: byte-exact after undo");
        reg.close_project(id);
    }
}
