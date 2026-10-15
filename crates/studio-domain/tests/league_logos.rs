//! Phase 1.4 — league logo resolution per title (research handoff P1.4-X): traced index on the
//! Xbox executables (by reference on PS3), native `artid` on PS4, shared indices disclosed, and
//! the package paths the texture surface addresses.

use std::path::{Path, PathBuf};

use studio_domain::{EditSession, Platform, Project, ProjectRole};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn session(game: &str, platform: Platform, db: &str, meta: &str) -> EditSession {
    let project = Project::load(repo(db), platform, ProjectRole::Nhlng, repo(meta), repo("defs")).unwrap();
    let mut s = EditSession::begin(project);
    s.set_game_id(Some(game.to_string()));
    s
}

#[test]
fn legacy_logo_indices_are_traced_and_shared_indices_disclosed() {
    let s = session("nhllegacy", Platform::Xbox, "anchors/xbox360/nhllegacy/databases/nhlng.db", "anchors/xbox360/nhllegacy/databases/nhlng-meta.xml");
    let norway = s.league_logo(7, None).unwrap().expect("Norway");
    assert_eq!((norway.art_index, norway.evidence.as_str()), (15, "traced"));
    assert!(norway.shared_with.is_empty());
    assert_eq!(norway.variants.len(), 4);
    assert_eq!(norway.variants[1].rel_path, "fe/ion/artassets/leaguelogosbig/l15.big");
    assert!(norway.variants.iter().all(|v| v.exists.is_none()), "no root given");
    // The NHL shield is shared by the Winter Classic copies, national squads, ROW and HUT codes.
    let nhl = s.league_logo(0, None).unwrap().unwrap();
    assert_eq!(nhl.art_index, 0);
    let codes: Vec<u64> = nhl.shared_with.iter().map(|(c, _)| *c).collect();
    assert!(codes.contains(&14), "{codes:?}");
    assert!(nhl.notes.iter().any(|n| n.contains("shared")));
    // Prospects and CHL share the CHL crest.
    assert_eq!(s.league_logo(12, None).unwrap().unwrap().art_index, 14);
    assert_eq!(s.league_logo(16, None).unwrap().unwrap().art_index, 14);

    // PS3 Legacy: same table by reference.
    let p = session("nhllegacy", Platform::Ps3, "anchors/ps3/nhllegacy/databases/nhlng.db", "anchors/ps3/nhllegacy/databases/nhlng-meta.xml");
    assert_eq!(p.league_logo(7, None).unwrap().unwrap().evidence, "by-reference");

    // NHL 09: Superliga (code 6) still has its own logo 8; Historic teams (8) → 9.
    let s09 = session("nhl09", Platform::Xbox, "anchors/xbox360/nhl09/databases/nhlng.db", "anchors/xbox360/nhl09/databases/nhlng-meta.xml");
    assert_eq!(s09.league_logo(6, None).unwrap().unwrap().art_index, 8);
    assert_eq!(s09.league_logo(8, None).unwrap().unwrap().art_index, 9);
    // NHL 08: inferred (index = code).
    let s08 = session("nhl08", Platform::Xbox, "anchors/xbox360/nhl08/databases/nhlng.db", "anchors/xbox360/nhl08/databases/nhlng-meta.xml");
    let l = s08.league_logo(3, None).unwrap().unwrap();
    assert_eq!((l.art_index, l.evidence.as_str()), (3, "inferred"));
}

#[test]
fn ps4_uses_the_native_artid_and_kettle_paths() {
    let s = session("nhl21", Platform::Ps4, "anchors/ps4/nhl21/databases/nhl_game.db", "anchors/ps4/nhl21/databases/nhl_game-meta.xml");
    let ahl = s.league_logo(1, None).unwrap().unwrap();
    assert_eq!((ahl.art_index, ahl.evidence.as_str()), (1, "native"));
    assert!(ahl.variants.iter().any(|v| v.rel_path == "nhl/ui/kettle_assets/leaguelogos/size256/l1.gnf"));
    assert!(ahl.variants.iter().any(|v| v.family == "size64_fr"));
    assert_eq!(ahl.variants.len(), 6);
    // NHL 15: two PNG sizes under ui/assets; NHL 16: two GNF sizes, no French twin.
    let s15 = session("nhl15", Platform::Ps4, "anchors/ps4/nhl15/databases/nhl_game.db", "anchors/ps4/nhl15/databases/nhl_game-meta.xml");
    let l15 = s15.league_logo(1, None).unwrap().unwrap();
    assert_eq!(l15.variants.iter().map(|v| v.rel_path.as_str()).collect::<Vec<_>>(), ["nhl/ui/assets/leaguelogos/size64/l1.png", "nhl/ui/assets/leaguelogos/size256/l1.png"]);
    let s16 = session("nhl16", Platform::Ps4, "anchors/ps4/nhl16/databases/nhl_game.db", "anchors/ps4/nhl16/databases/nhl_game-meta.xml");
    let l16 = s16.league_logo(1, None).unwrap().unwrap();
    assert_eq!(l16.variants.iter().map(|v| v.family.as_str()).collect::<Vec<_>>(), ["size64", "size256"]);
    assert!(l16.variants[0].rel_path.ends_with(".gnf"));
}

/// Real extraction tree, opt-in and read-only: `NHL_STUDIO_XBOX_TREE=<X:\…\NHL Legacy\NHL Legacy>`
/// — the four Legacy Norway packages exist loose and the small/big ones decode through the
/// portrait chain with the dimensions the research decoded (64×64 / 256×256).
#[test]
#[ignore]
fn real_tree_packages_exist_and_decode() {
    let root = std::env::var("NHL_STUDIO_XBOX_TREE").expect("NHL_STUDIO_XBOX_TREE");
    let s = session("nhllegacy", Platform::Xbox, "anchors/xbox360/nhllegacy/databases/nhlng.db", "anchors/xbox360/nhllegacy/databases/nhlng-meta.xml");
    let l = s.league_logo(7, Some(Path::new(&root))).unwrap().unwrap();
    for v in &l.variants {
        eprintln!("{} {} exists={:?}", v.family, v.rel_path, v.exists);
        assert_eq!(v.exists, Some(true), "{}", v.rel_path);
    }
    let small = studio_domain::decode_portrait(&Path::new(&root).join(&l.variants[0].rel_path)).unwrap();
    assert_eq!((small.width, small.height), (64, 64));
    let big = studio_domain::decode_portrait(&Path::new(&root).join(&l.variants[1].rel_path)).unwrap();
    assert_eq!((big.width, big.height), (256, 256));
    eprintln!("decoded leaguelogos {}x{} and leaguelogosbig {}x{}", small.width, small.height, big.width, big.height);
    // Replace round-trip on a temp copy of the small package (never the tree): a solid colour
    // 64x64 goes in through the same chain the UI uses and decodes back.
    let tmp = std::env::temp_dir().join(format!("nhlms-leaguelogo-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let copy = tmp.join("l15.big");
    std::fs::copy(Path::new(&root).join(&l.variants[0].rel_path), &copy).unwrap();
    let rgba: Vec<u8> = (0..64 * 64).flat_map(|_| [200u8, 30, 30, 255]).collect();
    let write = studio_domain::replace_texture_in_file(std::fs::read(&copy).unwrap(), &rgba, 64, 64, None).unwrap();
    std::fs::write(&copy, &write.bytes).unwrap();
    let back = studio_domain::decode_portrait(&copy).unwrap();
    assert_eq!((back.width, back.height), (64, 64));
    assert_eq!(&back.rgba[..4], &[200, 30, 30, 255]);
    eprintln!("replace round-trip: {} bytes written as {:?}", write.bytes.len(), write.container);
    let _ = std::fs::remove_dir_all(&tmp);
}

/// D7 (P1 accounting): NHL 07 Xbox 360 — the index is inferred (= league code, no table in the
/// executable), the four package paths follow the family convention, and loose-file existence is
/// reported per family under an install root.
#[test]
fn nhl07_xbox_infers_the_index_and_lists_packages_with_existence() {
    let s = session("nhl07", Platform::Xbox, "anchors/xbox360/nhl07/databases/NHL07NG.db", "anchors/xbox360/nhl07/databases/NHL07NG-meta.xml");
    let l = s.league_logo(1, None).unwrap().expect("National Teams (code 1)");
    assert_eq!((l.art_index, l.evidence.as_str()), (1, "inferred"));
    assert!(l.notes.iter().any(|n| n.contains("inferred")));
    let paths: Vec<&str> = l.variants.iter().map(|v| v.rel_path.as_str()).collect();
    assert_eq!(paths, ["fe/ion/artassets/leaguelogos/l1.big", "fe/ion/artassets/leaguelogosbig/l1.big", "fe/ion/artassets/leaguelogossmall/l1.big", "fe/ion/artassets/leaguelogoswide/l1.big"]);
    assert!(l.variants.iter().all(|v| v.exists.is_none()));
    // Under a root holding only the small package (NHL 07 ships no `leaguelogosbig`): one present, three absent.
    let dir = std::env::temp_dir().join(format!("nhlms-logo07-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("fe/ion/artassets/leaguelogossmall")).unwrap();
    std::fs::write(dir.join("fe/ion/artassets/leaguelogossmall/l1.big"), b"BIGF").unwrap();
    let l = s.league_logo(1, Some(&dir)).unwrap().unwrap();
    assert_eq!(l.variants.iter().map(|v| v.exists).collect::<Vec<_>>(), [Some(false), Some(false), Some(true), Some(false)]);
    std::fs::remove_dir_all(&dir).ok();
}
