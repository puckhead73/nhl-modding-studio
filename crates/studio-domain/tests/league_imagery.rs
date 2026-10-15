//! P1-G — league-keyed presentation imagery per title from `defs/league_imagery.toml` (research
//! handoff 03): every gen7 source and PS4 title resolves, paths follow the package / registry
//! conventions, French twins expand, and the Recompiled target reads the Xbox Legacy families.

use std::path::{Path, PathBuf};

use studio_domain::league_imagery::{league_imagery, LeagueImageryDefs};
use studio_domain::Platform;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn defs() -> LeagueImageryDefs {
    LeagueImageryDefs::load(repo("defs/league_imagery.toml")).unwrap()
}

#[test]
fn every_gen7_source_and_ps4_title_resolves() {
    let d = defs();
    assert_eq!(d.gen7.len(), 19, "10 Xbox 360 + 9 PS3 sources");
    assert_eq!(d.ps4.len(), 7);
    for s in &d.gen7 {
        let platform = if s.platform == "ps3" { Platform::Ps3 } else { Platform::Xbox };
        let li = league_imagery(&d, &s.title, platform, None).unwrap_or_else(|| panic!("{} {}", s.platform, s.title));
        assert!(!li.families.is_empty(), "{} {}", s.platform, s.title);
        assert!(li.families.iter().any(|f| f.family == "divisionlogos"), "{} {}", s.platform, s.title);
        for f in &li.families {
            for m in &f.members {
                assert!(m.rel_path.starts_with(&format!("fe/ion/artassets/{}/", f.family)), "{}", m.rel_path);
                assert!(m.exists.is_none() && m.fr_rel_path.is_none());
            }
        }
    }
    for t in &d.ps4 {
        let li = league_imagery(&d, &t.title, Platform::Ps4, None).unwrap();
        assert!(li.registry.contains("assetpaths"), "{}", t.title);
        assert!(li.families.iter().any(|f| f.family == "region_logo_big"), "{}", t.title);
    }
}

#[test]
fn legacy_xbox_families_and_recompiled_by_identity() {
    let d = defs();
    let x = league_imagery(&d, "nhllegacy", Platform::Xbox, None).unwrap();
    let names: Vec<&str> = x.families.iter().map(|f| f.family.as_str()).collect();
    assert_eq!(names, ["divisionlogos", "trophies", "trophiesbroadcast", "mudleaguelogos"]);
    let div = &x.families[0];
    assert_eq!(div.members.iter().map(|m| m.id).collect::<Vec<_>>(), [0, 1, 2, 3, 4, 5]);
    assert_eq!(div.members[3].rel_path, "fe/ion/artassets/divisionlogos/d3.big");
    assert!(div.consumers.iter().any(|c| c.contains("dm36_playofftree")));
    assert!(div.reading.contains("Western Conference"));
    assert_eq!(x.families[3].members.iter().map(|m| m.id).collect::<Vec<_>>(), [0, 5], "MUD logos: ids 0 and 5");
    assert!(x.registry.contains("generalhelpers"));
    let r = league_imagery(&d, "nhllegacy", Platform::Recomp, None).unwrap();
    assert_eq!(r.families.len(), 4);
    // NHL 10 Xbox: d6, decoded texture facts, screens not traced.
    let n10 = league_imagery(&d, "nhl10", Platform::Xbox, None).unwrap();
    assert_eq!(n10.families[0].members.len(), 7);
    assert!(n10.families[0].texture_facts.iter().any(|f| f == "0:256x256:0x86"));
    assert!(n10.notes.iter().any(|n| n.contains("not traced")));
    // PS3 NHL 08: four division logos, five trophies, no broadcast set.
    let p08 = league_imagery(&d, "nhl08", Platform::Ps3, None).unwrap();
    assert_eq!(p08.families.iter().map(|f| f.members.len()).collect::<Vec<_>>(), [4, 5]);
    assert!(league_imagery(&d, "nhl07", Platform::Ps3, None).is_none(), "no PS3 NHL 07");
}

#[test]
fn ps4_registry_types_expand_with_french_twins_and_existence() {
    let d = defs();
    let n21 = league_imagery(&d, "nhl21", Platform::Ps4, None).unwrap();
    let large = n21.families.iter().find(|f| f.family == "large_league_logo").unwrap();
    let l7 = large.members.iter().find(|m| m.id == 7).unwrap();
    assert_eq!(l7.rel_path, "nhl/ui/kettle_assets/leaguelogos/size512/l7.gnf");
    assert_eq!(l7.fr_rel_path.as_deref(), Some("nhl/ui/kettle_assets/leaguelogos/size512/l7_fr.gnf"));
    let mem = n21.families.iter().find(|f| f.family == "memorial_cup_logo").unwrap();
    assert_eq!(mem.members[0].rel_path, "nhl/ui/kettle_assets/leaguelogos/size256/l99.gnf");
    let n15 = league_imagery(&d, "nhl15", Platform::Ps4, None).unwrap();
    let region = n15.families.iter().find(|f| f.family == "region_logo").unwrap();
    assert_eq!(region.members[0].rel_path, "nhl/ui/assets/gmconnected/scouting/regionsLogos/region-0.png");
    assert!(region.members.iter().all(|m| m.fr_rel_path.is_none()));
    assert!(n15.families.iter().all(|f| f.family != "large_league_logo"), "NHL 15 has no size512 type");
    // Existence under a root: one member present, the rest absent.
    let dir = std::env::temp_dir().join(format!("nhlms-imagery-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("nhl/ui/kettle_assets/leaguelogos/size512")).unwrap();
    std::fs::write(dir.join("nhl/ui/kettle_assets/leaguelogos/size512/l7.gnf"), b"GNF ").unwrap();
    let with_root = league_imagery(&d, "nhl21", Platform::Ps4, Some(&dir)).unwrap();
    let large = with_root.families.iter().find(|f| f.family == "large_league_logo").unwrap();
    assert_eq!(large.members.iter().find(|m| m.id == 7).unwrap().exists, Some(true));
    assert_eq!(large.members.iter().find(|m| m.id == 7).unwrap().fr_exists, Some(false));
    assert_eq!(large.members.iter().find(|m| m.id == 0).unwrap().exists, Some(false));
    std::fs::remove_dir_all(&dir).ok();
}
