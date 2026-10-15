//! Phase 2.4 (C9, overnight 2026-09-20) — team logo families per title: the team's art id →
//! every logo package / registry member the title keys by it. The family table is checked (a) on
//! the anchors for its shape and paths, and (b) `--ignored` against the developer census of the real
//! archives / trees (`teamlogos/manifest.json` under `NHL_STUDIO_FIXTURES`): every offered family
//! exists on the title with the team-sized count, and every team-sized `t#` family the title ships
//! is offered — nothing invented, nothing silently dropped.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use studio_domain::{team_logos, EditSession, Platform, Project, ProjectRole};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn session(game: &str, platform: Platform, db: &str, meta: &str) -> EditSession {
    let project = Project::load(
        repo(db),
        platform,
        ProjectRole::Nhlng,
        repo(meta),
        repo("defs"),
    )
    .unwrap();
    let mut s = EditSession::begin(project);
    s.set_game_id(Some(game.to_string()));
    s
}

#[test]
fn legacy_and_nhl21_team_logo_sets_follow_the_measured_grammar() {
    let s = session(
        "nhllegacy",
        Platform::Xbox,
        "anchors/xbox360/nhllegacy/databases/nhlng.db",
        "anchors/xbox360/nhllegacy/databases/nhlng-meta.xml",
    );
    let team = s.team_summaries().unwrap()[0].team_id;
    let set = s
        .team_logo_set(team, None)
        .unwrap()
        .expect("Legacy team has an art id");
    assert_eq!(set.evidence, "measured");
    let fams: Vec<&str> = set.variants.iter().map(|v| v.family.as_str()).collect();
    assert_eq!(
        fams,
        [
            "teamlogos",
            "teamlogoscalendar",
            "teamlogosdynasty",
            "teamlogossmall",
            "teamlogoswide",
            "teamlogosreflection"
        ]
    );
    let a = set.artid;
    assert_eq!(
        set.variants[0].rel_path,
        format!("fe/ion/artassets/teamlogos/t{a}.big")
    );
    assert_eq!(
        set.variants[3].rel_path,
        format!("fe/ion/artassets/teamlogossmall/s{a}.big")
    );
    assert_eq!(
        set.variants[5].rel_path,
        format!("fe/ion/artassets/teamlogosreflection/r{a}.big")
    );
    assert!(
        set.variants.iter().all(|v| v.exists.is_none()),
        "no root given"
    );
    // Without an install root the archive is still named (the console reads cache.big).
    assert_eq!(set.variants[0].archive.as_deref(), Some("cache.big"));

    // NHL 12: the pre-13 `t` prefix everywhere and no reflection family.
    let s12 = session(
        "nhl12",
        Platform::Xbox,
        "anchors/xbox360/nhl12/databases/nhlng.db",
        "anchors/xbox360/nhl12/databases/nhlng-meta.xml",
    );
    let t = s12.team_summaries().unwrap()[0].team_id;
    let set12 = s12.team_logo_set(t, None).unwrap().unwrap();
    assert_eq!(set12.variants.len(), 5);
    assert!(set12
        .variants
        .iter()
        .all(|v| v.rel_path.contains(&format!("/t{}.big", set12.artid))));

    // PS4 NHL 21: kettle families, the HUT rare trio as three variants of one folder.
    let p = session(
        "nhl21",
        Platform::Ps4,
        "anchors/ps4/nhl21/databases/nhl_game.db",
        "anchors/ps4/nhl21/databases/nhl_game-meta.xml",
    );
    let t = p.team_summaries().unwrap()[0].team_id;
    let set21 = p.team_logo_set(t, None).unwrap().unwrap();
    assert_eq!(set21.variants.len(), 15);
    let a = set21.artid;
    assert!(set21
        .variants
        .iter()
        .any(|v| v.rel_path == format!("nhl/ui/kettle_assets/teamlogos/large/t{a}.gnf")));
    assert!(set21
        .variants
        .iter()
        .any(|v| v.family == "hutcardrare/t_gold"
            && v.rel_path == format!("nhl/ui/kettle_assets/teamlogos/hutcardrare/t_gold_{a}.gnf")));
    assert!(set21
        .variants
        .iter()
        .all(|v| v.archive.as_deref() == Some("databigs/inxp_10.big")));

    // PS4 NHL 15: PNGs under ui/assets.
    let p15 = session(
        "nhl15",
        Platform::Ps4,
        "anchors/ps4/nhl15/databases/nhl_game.db",
        "anchors/ps4/nhl15/databases/nhl_game-meta.xml",
    );
    let t = p15.team_summaries().unwrap()[0].team_id;
    let set15 = p15.team_logo_set(t, None).unwrap().unwrap();
    assert_eq!(set15.variants.len(), 5);
    assert!(set15
        .variants
        .iter()
        .any(|v| v.rel_path.starts_with("nhl/ui/assets/playnow/teamlogos/t")));
    assert!(set15.variants[0]
        .rel_path
        .starts_with("nhl/ui/assets/teamlogos/fullsizeflat/t"));
    assert!(set15.variants[0].rel_path.ends_with(".png"));
}

/// Existence under a root is reported per variant; a title outside the measured table yields `None`.
#[test]
fn existence_is_checked_under_the_root_and_unmeasured_titles_yield_none() {
    let dir = std::env::temp_dir().join(format!("nhlms-c9-teamlogos-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("fe/ion/artassets/teamlogossmall")).unwrap();
    std::fs::write(dir.join("fe/ion/artassets/teamlogossmall/s7.big"), b"x").unwrap();
    let set =
        team_logos::team_logo_set(Some("nhl14"), Some(Platform::Xbox), 7, Some(&dir)).unwrap();
    let small = set
        .variants
        .iter()
        .find(|v| v.family == "teamlogossmall")
        .unwrap();
    assert_eq!(small.exists, Some(true));
    assert_eq!(
        set.variants
            .iter()
            .find(|v| v.family == "teamlogos")
            .unwrap()
            .exists,
        Some(false)
    );
    assert!(team_logos::team_logo_set(Some("nhl99"), Some(Platform::Xbox), 7, None).is_none());
    assert!(team_logos::team_logo_set(None, Some(Platform::Xbox), 7, None).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The table against the census: every offered `(folder, prefix#.ext)` is a census row of that
/// platform/title whose count is team-sized (≥ 100; NHL 07's 56-team database ≥ 50; the 30-member
/// reflection set), and every team-sized `t#`-keyed census row is offered.
#[test]
#[ignore = "private fixtures teamlogos/manifest.json (developer census of the real archives): set NHL_STUDIO_FIXTURES and select with --ignored"]
fn family_table_matches_the_census_of_the_real_archives() {
    let root = PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES"));
    let m: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("teamlogos/manifest.json")).unwrap())
            .unwrap();
    let mut census: std::collections::BTreeMap<(String, String), BTreeSet<(String, String, u64)>> =
        Default::default();
    for r in m["census"].as_array().unwrap() {
        let plat = r["platform"].as_str().unwrap().to_string();
        let title = r["title"].as_str().unwrap().to_string();
        let dir = r["family_dir"].as_str().unwrap();
        // gen7: the artassets folder; PS4: the path under `nhl/ui/<tree>/`, with the plain
        // `teamlogos/` prefix stripped (`playnow/teamlogos` keeps its own name). NHL 15 ships
        // three trees for `overlaylogo` — key on the tree too.
        let (tree, folder) = if plat == "ps4" {
            let mut it = dir.splitn(4, '/');
            let (_, _, tree, rest) = (
                it.next(),
                it.next(),
                it.next().unwrap_or(""),
                it.next().unwrap_or(""),
            );
            (
                tree.to_string(),
                rest.strip_prefix("teamlogos/").unwrap_or(rest).to_string(),
            )
        } else {
            (String::new(), dir.rsplit('/').next().unwrap().to_string())
        };
        let pattern = r["pattern"].as_str().unwrap().to_string();
        census.entry((plat, title)).or_default().insert((
            format!("{tree}:{folder}"),
            pattern,
            r["count"].as_u64().unwrap(),
        ));
    }
    let mut checked = 0;
    for ((plat, title), rows) in &census {
        let platform = match plat.as_str() {
            "x360" => Platform::Xbox,
            "ps3" | "ps3-cache" => Platform::Ps3,
            "ps4" => Platform::Ps4,
            _ => unreachable!(),
        };
        let offered = team_logos::families_for(title, platform);
        assert!(!offered.is_empty(), "{plat} {title}: no families offered");
        eprintln!("{plat} {title}: {} families offered", offered.len());
        let tree = match (plat.as_str(), title.as_str()) {
            ("ps4", "nhl15") => "assets",
            ("ps4", _) => "kettle_assets",
            _ => "",
        };
        let team_sized = if title == "nhl07" { 50 } else { 100 };
        // (a) every offered family is a team-sized census row
        for (folder, prefix, ext) in &offered {
            let pattern = format!("{prefix}#.{ext}");
            let row = rows
                .iter()
                .find(|(f, p, _)| f == &format!("{tree}:{folder}") && p == &pattern);
            let (_, _, count) = row.unwrap_or_else(|| {
                panic!("{plat} {title}: offered {folder}/{pattern} is not in the census")
            });
            assert!(
                *count >= team_sized || (folder == "teamlogosreflection" && *count == 30),
                "{plat} {title}: {folder}/{pattern} has only {count} members"
            );
        }
        // (b) every team-sized `t#` / letter-prefixed census row is offered
        for (f, p, count) in rows {
            let (t, folder) = f.split_once(':').unwrap();
            if t != tree || *count < team_sized {
                continue;
            }
            let ok = offered
                .iter()
                .any(|(of, pre, ext)| of == folder && format!("{pre}#.{ext}") == *p);
            assert!(
                ok,
                "{plat} {title}: census family {folder}/{p} ({count} members) is not offered"
            );
        }
        checked += 1;
    }
    assert_eq!(
        checked, 27,
        "27 platform/title sources in the census (10 X360, 9 PS3 + PS3 cache, 7 PS4)"
    );
}

/// P1 audit (21 Sep 2026): the closed `teamlogosreflection` family (30 members, art ids 0–29 on
/// every 13–Legacy source of the census) is reported absent — no loose path claimed, no archive
/// route offered, a note saying why — for a club whose art id lies beyond it, while the
/// team-sized families keep their per-file existence; an NHL club inside the range keeps the
/// archive route.
#[test]
fn the_closed_reflection_family_is_absent_beyond_its_measured_ids() {
    let s = session(
        "nhllegacy",
        Platform::Xbox,
        "anchors/xbox360/nhllegacy/databases/nhlng.db",
        "anchors/xbox360/nhllegacy/databases/nhlng-meta.xml",
    );
    let teams = s.team_summaries().unwrap();
    let sets: Vec<_> = teams.iter().filter_map(|t| s.team_logo_set(t.team_id, None).unwrap()).collect();
    let inside = sets.iter().find(|x| x.artid <= 29).expect("an NHL club with art id 0–29");
    let beyond = sets.iter().find(|x| x.artid > 29).expect("a club with an art id beyond 29");
    let refl = |x: &studio_domain::team_logos::TeamLogoSet| x.variants.iter().find(|v| v.family == "teamlogosreflection").cloned().unwrap();
    let r_in = refl(inside);
    assert_eq!(r_in.exists, None, "no root: unknown, as for every family");
    assert!(r_in.archive.is_some(), "inside the measured ids the archive route stays");
    let r_out = refl(beyond);
    assert_eq!(r_out.exists, Some(false), "beyond the measured ids: absent even without a root");
    assert_eq!(r_out.archive, None, "no archive route against a member that never ships");
    assert!(beyond.notes.iter().any(|n| n.contains("teamlogosreflection") && n.contains("0–29")), "{:?}", beyond.notes);
    assert!(!inside.notes.iter().any(|n| n.contains("has none")));
    // The team-sized families of the same club are untouched by the ceiling.
    for v in beyond.variants.iter().filter(|v| v.family != "teamlogosreflection") {
        assert_eq!(v.exists, None, "{}", v.family);
        assert!(v.archive.is_some(), "{}", v.family);
    }
}
