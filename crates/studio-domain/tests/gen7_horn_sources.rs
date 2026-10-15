//! P4.4 (overnight 2026-09-24) — the gen7 horn-source probe on real trees (read only), checked
//! against research R81's per-title residual sets: X360 NHL 15 (loose numbered banks: ids 0–29, 100,
//! 194 goal / period; residual `goalhorn` ids 30–36, 38, 41, 45, 61, 136) and X360 NHL 09 (no loose
//! numbered banks; the r81 data records them as `data0.big` members — the handoff text says
//! `nocache.big`; the data is followed and the discrepancy recorded). Nothing is written.
use std::path::Path;

use studio_domain::gen7_horn_sources::Gen7HornSources;
use studio_domain::Platform;

#[test]
#[ignore = "real gen7 trees under X:/Games/Emulation/Roms/XBOX360 (read only): select with --ignored"]
fn real_x360_nhl15_and_nhl09_horn_sources_match_the_r81_residual_classes() {
    let nhl15 = Path::new(r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15");
    assert!(nhl15.is_dir(), "{} missing (fails, never skips)", nhl15.display());
    let p = Gen7HornSources::probe("nhl15", Platform::Xbox, nhl15);
    assert!(p.refusal.is_none(), "{:?}", p.refusal);
    eprintln!("NHL 15 X360: loose goal {:?}", p.loose_goal);
    eprintln!("NHL 15 X360: loose period {:?}; chant {} banks; horns.abk {:?}; aemshorn {}; {:?}", p.loose_period, p.loose_chant.len(), p.horns_abk_len, p.aemshorn_csi_present, p.archive_status);
    let expect: std::collections::BTreeSet<u32> = (0..=29).chain([100, 194]).collect();
    assert_eq!(p.loose_goal, expect, "R78 / R81: NHL 14–Legacy ship goal banks 0–29, 100, 194");
    assert_eq!(p.loose_period, expect);
    assert_eq!(p.horns_abk_len, Some(3268), "R81: horns.abk is 3 268 bytes with an empty S10A table");
    assert!(p.aemshorn_csi_present, "R81: aemshorn.csi is loose on 10–Legacy");
    // R85 §1: the loader table lists exactly the shipped banks (goal / period 0–29, 100, 194; 73 team chants; 45 goalie chants).
    let l = p.loader.as_ref().expect("na_en_loadfile.xml read");
    eprintln!("NHL 15 X360 loader: {} — goal {} / period {} / team chant {} / goalie chant {}", l.source, l.goal.len(), l.period.len(), l.team_chant.len(), l.goalie_chant.len());
    assert_eq!(l.goal, expect);
    assert_eq!(l.period, expect);
    assert_eq!((l.team_chant.len(), l.goalie_chant.len(), l.horns_bank_listed, l.horns_event_definition_listed), (73, 45, true, true));
    // R85 §1 states the chant tables match the folder "100 %"; the probe measures one exception on
    // this tree: the loader lists goalie chant `0587_chant.abk`, which is not shipped, while the
    // shipped `0584_chant.abk` is unlisted (the r85 data compared only the goal family to members).
    let listed = &l.team_chant | &l.goalie_chant;
    assert_eq!((&listed - &p.loose_chant).into_iter().collect::<Vec<_>>(), vec![587], "listed but not shipped");
    assert_eq!((&p.loose_chant - &listed).into_iter().collect::<Vec<_>>(), vec![584], "shipped but not listed");
    // R97 §1.4 (26 Sep): chant values are never resolved or linked; the note names the same-numbered bank.
    let (r, n) = p.resolve("chantaudioid", 584).unwrap();
    assert!(r.is_none() && n.contains("`584_chant.abk`") && n.contains("shipped but NOT listed"), "{n}");
    let (r, n) = p.resolve("chantaudioid", 587).unwrap();
    assert!(r.is_none() && n.contains("lists it as a **goalie** chant bank") && n.contains("no such file is shipped"), "{n}");
    let (_, n) = p.resolve("chantaudioid", 1011).unwrap();
    assert!(n.contains("goalie") && n.contains("announcer audio id"), "{n}");
    for id in 1..=29u32 {
        let (r, n) = p.resolve("goalhorn", id as i64).unwrap();
        assert_eq!(r.as_deref(), Some(format!("audio/sfx/{id:03}_goal.abk").as_str()));
        assert!(n.contains("listed in the loader asset table"), "{n}");
        assert_eq!(p.bank_file("goalhorn", id as i64), Some(format!("{id:03}_goal.abk")), "X360 NHL 15 is in the bank listing: horn links");
    }
    assert_eq!(p.bank_file("periodhorn", 1).as_deref(), Some("001_period.abk"));
    for id in [30, 31, 32, 33, 34, 35, 36, 38, 41, 45, 61, 136] {
        let (r, n) = p.resolve("goalhorn", id).unwrap();
        assert!(r.is_none(), "{id}: {r:?}");
        assert!(n.starts_with("no goal horn bank found in the loose folder or the audio archives") && n.contains("no loader asset for this id") && n.contains("aemshorn") && n.contains("bank 0"), "{id}: {n}");
    }
    let (r, n) = p.resolve("goalhorn", 0).unwrap();
    assert!(r.is_none() && n.contains("unreferenced"));
    let (r, n) = p.resolve("chantaudioid", 61).unwrap();
    assert!(r.is_none() && n.contains("`061_chant.abk`") && n.contains("one of the twelve club-chant banks") && n.contains("No consumer that selects a chant bank has been traced"), "{n}");
    assert_eq!(p.bank_file("chantaudioid", 61), None, "chants never link");
    assert_eq!(p.bank_file("goalhorn", 41), None, "no bank found, no link");
    let (r, n) = p.resolve("cityaudioid", 61).unwrap();
    assert!(r.is_none() && n.contains("no established binding"));

    // NHL 09 X360: archive-only numbered banks (R81 §1).
    let nhl09 = Path::new(r"X:\Games\Emulation\Roms\XBOX360\NHL 09\NHL 09 (USA)");
    assert!(nhl09.is_dir(), "{} missing (fails, never skips)", nhl09.display());
    let p9 = Gen7HornSources::probe("nhl09", Platform::Xbox, nhl09);
    eprintln!("NHL 09 X360: loose goal {} banks; {:?}; archive goal {:?}", p9.loose_goal.len(), p9.archive_status, p9.archive_goal.keys().collect::<Vec<_>>());
    assert!(p9.loose_goal.is_empty(), "R81: NHL 09 X360 ships no loose numbered banks");
    let expect09: std::collections::BTreeSet<u32> = (0..=29).chain([100]).collect();
    assert_eq!(p9.archive_goal.keys().copied().collect::<std::collections::BTreeSet<u32>>(), expect09, "r81 data: NHL 09 X360 goal banks 0–29 + 100 as data0.big members");
    assert!(p9.archive_goal.values().all(|a| a.starts_with("data0.big:audio/sfx/ram/")), "{:?}", p9.archive_goal);
    let (r, n) = p9.resolve("goalhorn", 5).unwrap();
    // The member's real path (26 Sep; previously printed as `audio/sfx/005_goal.abk`, R98 §1 `audio/sfx/ram`).
    assert_eq!(r.as_deref(), Some("data0.big:audio/sfx/ram/005_goal.abk"));
    assert!(n.contains("by name") && n.contains("R98") && n.contains("listed in the loader asset table"), "{n}");
    assert_eq!(p9.bank_file("goalhorn", 5), None, "X360 NHL 09 is refused by the bank listing: no link");
    // The NHL 09 X360 loader table ships loose (27 393 B) and as a RefPack `audioboot.big` member (R85 copies).
    let l9 = p9.loader.as_ref().expect("na_en_loadfile.xml read");
    eprintln!("NHL 09 X360 loader: {} — goal {} / period {}", l9.source, l9.goal.len(), l9.period.len());
    assert!(l9.source.ends_with("na_en_loadfile.xml"), "{}", l9.source);
    assert_eq!(l9.goal, expect09);
    assert_eq!(l9.period, expect09);

    // PS3 09 / 10 / 12 / 13 (26 Sep): the probe reads the R98 archive members; every club goal horn
    // 1–29 resolves to its member and links to the sound-bank browser (the listing covers them).
    for (game, root, archive) in [
        ("nhl09", r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 09\PS3_GAME\USRDIR", "data0.big"),
        ("nhl10", r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 10\PS3_GAME\USRDIR", "audio.big"),
        ("nhl12", r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 12\[BLUS 30791] NHL® 12\PS3_GAME\USRDIR", "nocache.big"),
        ("nhl13", r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 13\PS3_GAME\USRDIR", "nocache.big"),
    ] {
        let p = Gen7HornSources::probe(game, Platform::Ps3, Path::new(root));
        assert!(p.refusal.is_none(), "{game}: {:?}", p.refusal);
        eprintln!("PS3 {game}: {:?}; goal ids {:?}", p.archive_status, p.archive_goal.keys().collect::<Vec<_>>());
        for id in 1..=29u32 {
            let (r, n) = p.resolve("goalhorn", id as i64).unwrap();
            assert!(r.as_deref().is_some_and(|r| r.starts_with(&format!("{archive}:")) && r.ends_with(&format!("/{id:03}_goal.abk"))), "{game} {id}: {r:?} {n}");
            assert_eq!(p.bank_file("goalhorn", id as i64), Some(format!("{id:03}_goal.abk")), "{game} {id}");
        }
        assert_eq!(p.bank_file("chantaudioid", 61), None);
    }

    // Archived-only copies (R85: no loose load file on PS3 09 / 10 / 12 / 13): PS3 NHL 13 is a
    // chunkzip `audioboot.big` member (2 950 → 31 715 B), PS3 NHL 09 a RefPack one (3 318 → 27 394 B).
    // The table read is exercised directly as well.
    for (root, wrapper, expect) in [
        (r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 13\PS3_GAME\USRDIR", "chunkzip", (0..=29).chain([100]).collect::<std::collections::BTreeSet<u32>>()),
        (r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 09\PS3_GAME\USRDIR", "refpack", expect09.clone()),
    ] {
        let root = Path::new(root);
        assert!(!root.join("audio/na_en_loadfile.xml").is_file(), "R85: no loose load file on {}", root.display());
        let l = studio_domain::gen7_horn_sources::LoaderTable::read(root).unwrap_or_else(|| panic!("archived load file on {}", root.display()));
        eprintln!("{} loader: {} — goal {} / period {} / team chant {} / goalie chant {}", root.display(), l.source, l.goal.len(), l.period.len(), l.team_chant.len(), l.goalie_chant.len());
        assert!(l.source.starts_with("audioboot.big:") && l.source.contains(wrapper), "{}", l.source);
        assert_eq!(l.goal, expect, "{}", root.display());
    }
}
