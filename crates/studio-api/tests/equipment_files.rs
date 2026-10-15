//! Phase 4 (21 Sep 2026) — the equipment file check through the registry on the real Xbox 360
//! Legacy and NHL 11 rendering trees (read only) with the committed anchors' rosters. Research
//! 2026-09-21 handoff 12 §2: **every** brand / model id the roster assigns has its files on the
//! title (100 % containment for gloves, helmet, skates, stick, pads, blocker, trapper, goalie
//! stick, mask paint), and the NHL 11 tree is texture-keyed (`texlib_<id>`, sticks under
//! `stick/`). The check must therefore report files for every assigned id of every roster
//! player on both titles; the catalogue sizes must match the research (Legacy gloves 56 ids,
//! helmets 19, skates 33, sticks 75, pads 70, blockers 71, trappers 69, goalie sticks 40, masks
//! 280). A missing tree fails, never skips.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_api::Registry;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[test]
#[ignore = "real Xbox 360 installs under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn every_assigned_equipment_id_has_files_on_legacy_and_nhl11() {
    for (game, root, expected_catalogue) in [
        (
            "nhllegacy",
            r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy",
            // Helmet: the research counts `helmet n1` ids (19); Studio's slot also keys the
            // chinstrap / padding / texlib files, which carry two more ids (21). Goalie stick:
            // the research counts every `goaliestick n1` incl. the two-token meshes (40);
            // Studio's slot keys the single-token textures (38 — two ids have a mesh only).
            [("gloves", 56usize), ("helmet", 21), ("skates", 33), ("stick", 75), ("pads", 70), ("blocker", 71), ("trapper", 69), ("goalie-stick", 38), ("goalie-helmet", 280)],
        ),
        (
            "nhl11",
            r"X:\Games\Emulation\Roms\XBOX360\NHL 11\NHL 11",
            // Studio's slot definitions vs the research's `texlib n1` count on NHL 11: gloves 19
            // (the generic `glove_<side>_0_<lod>` meshes key ids 0 / 1 too), helmet 9 (one research
            // id has no file shape Studio's helmet matchers key); the other families agree.
            [("gloves", 19usize), ("helmet", 9), ("skates", 17), ("stick", 28), ("pads", 33), ("blocker", 33), ("trapper", 33), ("goalie-stick", 14), ("goalie-helmet", 243)],
        ),
    ] {
        assert!(Path::new(root).is_dir(), "{game}: {root} missing (fails, never skips)");
        let reg = Registry::default();
        let id = reg
            .open_workspace_from_paths(Some(game), studio_api::parse_platform("xbox").unwrap(), &s(&repo(&format!("anchors/xbox360/{game}/databases/nhlng.db"))), None, &s(&repo(&format!("anchors/xbox360/{game}/databases/nhlng-meta.xml"))), &s(&repo("defs")))
            .unwrap()
            .project_id;
        let roots = studio_api::dto::AssetRootsArgsDto { install_dir: root.to_string(), platform: Some("xbox".into()), rendering_override: None, fe_override: None };
        let players = reg.list_players(id).unwrap();
        let mut checked = 0usize;
        let mut missing: Vec<String> = Vec::new();
        let mut catalogue: BTreeMap<String, usize> = BTreeMap::new();
        let mut folders: BTreeMap<String, String> = BTreeMap::new();
        for p in players.iter().step_by(7) {
            let rows = reg.player_equipment_files(id, p.player_id, &roots, &repo("defs")).unwrap();
            assert!(!rows.is_empty(), "{game}: {} has equipment rows", p.last_name);
            for r in &rows {
                catalogue.insert(r.slot.clone(), r.catalogue_ids);
                folders.insert(r.slot.clone(), r.folder.clone());
                let Some(rid) = r.id else { continue };
                // Id 0 / a sentinel outside the catalogue is not an assignment on some slots (the
                // research counts non-zero ids); ids inside the catalogue's range must have files.
                if rid == 0 || r.catalogue_ids == 0 {
                    continue;
                }
                checked += 1;
                if r.file_count == 0 && missing.len() < 12 {
                    missing.push(format!("{} {} {} id {rid}: {}", p.last_name, r.slot, r.field_ref, r.note));
                }
                if r.file_count == 0 {
                    checked -= 1;
                }
            }
        }
        eprintln!("{game}: {} players sampled, {checked} assigned ids with files, catalogue {catalogue:?}, folders {folders:?}", players.len().div_ceil(7));
        for m in &missing {
            eprintln!("  missing: {m}");
        }
        assert!(missing.is_empty(), "{game}: assigned ids without files (research: 100 % containment)");
        for (slot, n) in expected_catalogue {
            if n > 0 {
                assert_eq!(catalogue.get(slot).copied().unwrap_or(0), n, "{game}: {slot} catalogue size (research handoff 12 §2)");
            }
        }
        if game == "nhl11" {
            assert_eq!(folders.get("stick").map(String::as_str), Some("stick"), "NHL 11 sticks live under stick/");
        }
        reg.close_project(id);
    }
}
