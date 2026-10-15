//! Phase 4 (22 Sep 2026) — research 2026-09-21 pm handoff R52 §1 reproduced by Studio's component
//! sets on the real trees: per family, the ids in the folder and how many carry the majority set,
//! with the research's incomplete-id lists for Legacy. Read-only; a missing tree fails, never skips.

use std::path::Path;

use studio_domain::equipment_components::{family_census, set_contract};
use studio_domain::Platform;

/// R52 §1 (X360 Legacy; NHL 13 / 15 "identical in shape") and its JSON (`ids`, `ids_missing_count`):
/// `(slot, ids, ids with every majority component, the research's incomplete ids on Legacy)`.
const LEGACY: &[(&str, usize, usize, &[u32])] = &[
    ("gloves", 53, 53, &[]),
    ("pads", 67, 57, &[12, 14, 15, 17, 18, 22, 23, 26, 27, 35]),
    // R52 counts `blocker_0_am` / `trapper_0_am` as "id 0, `am` only"; Studio's blocker / trapper
    // matchers key the id at the second numeric position, so a single-number stem is a style-level
    // file, not an equipment id — one id fewer than the handoff on each (observation O-9).
    ("blocker", 68, 50, &[11, 12, 14, 15, 17, 18, 21, 22, 23, 25, 26, 27, 34, 35, 81, 82, 83, 84]),
    ("trapper", 66, 58, &[12, 14, 15, 17, 22, 23, 26, 27]),
    ("goalie-stick", 38, 38, &[]),
    ("goalie-helmet", 280, 280, &[]),
];

#[test]
#[ignore = "real Xbox 360 / PS3 installs under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn every_measured_family_reproduces_the_research_component_census() {
    for (game, platform, root) in [
        ("nhllegacy", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\rendering"),
        ("nhllegacy", Platform::Ps3, r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL Legacy - Vanilla\PS3_GAME\USRDIR\rendering"),
        ("nhl13", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL 13\NHL 13\rendering"),
        ("nhl15", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15\rendering"),
    ] {
        let tag = format!("{game} {}", platform.as_str());
        let root = Path::new(root);
        assert!(root.is_dir(), "{tag}: {} missing (fails, never skips)", root.display());
        for (slot, ids, complete, incomplete_ids) in LEGACY {
            let c = set_contract(slot, Some(game), platform);
            let census = family_census(root, slot, &c).unwrap_or_else(|| panic!("{tag}: {slot} folder or contract missing"));
            let inc: Vec<u32> = census.incomplete.iter().map(|(id, _)| *id).collect();
            eprintln!("{tag} {slot}: {} ids, {} complete, incomplete {:?}, front-end incomplete {}", census.ids, census.complete, census.incomplete.iter().map(|(id, m)| format!("{id}:{}", m.join("+"))).collect::<Vec<_>>(), census.front_end_incomplete.len());
            if game == "nhllegacy" {
                assert_eq!(census.ids, *ids, "{tag} {slot}: ids in the folder (R52 §1)");
                assert_eq!(census.complete, *complete, "{tag} {slot}: ids with every majority component (R52: ids − ids_missing_count)");
                assert_eq!(inc, incomplete_ids.to_vec(), "{tag} {slot}: the research's incomplete ids");
            } else {
                // NHL 13 / 15: the same shape — every family present, most ids complete, and every
                // incomplete id lacks only components the research names as optional (a tint map,
                // a normal map, or a front-end diffuse) — or, on the mask paints, its shell-0 file
                // (NHL 13 ships paint 260 for other shells only; recorded, observation O-9).
                assert!(census.ids > 0 && census.complete * 10 >= census.ids * 6, "{tag} {slot}: {} of {} complete", census.complete, census.ids);
                for (id, missing) in &census.incomplete {
                    assert!(
                        missing.iter().all(|m| m.ends_with("_2_tm") || m.ends_with("_nm") || m.ends_with("_1_tm") || m.ends_with("_0_dm") || m.ends_with("_0_tm") || m.ends_with("_sm") || (*slot == "goalie-helmet" && m.ends_with("_cm"))),
                        "{tag} {slot} id {id}: unexpected missing {missing:?}"
                    );
                }
            }
        }
        // The single-container families have no set census, by contract.
        assert!(family_census(root, "stick", &set_contract("stick", Some(game), platform)).is_none(), "{tag}");
    }
}
