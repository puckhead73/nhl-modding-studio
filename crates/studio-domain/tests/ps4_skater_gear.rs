//! Receipt 52 — PS4 skater gear against each title's own Creation Zone lists and vault (research 2026-09-28 handoff 02b /
//! R136). Real installs under `H:\Emulators\ShadPS4\Games\<CUSA>` (only `databigs/*.big` read), anchors and the genuine
//! NHL 20 private fixture (read only).
use std::path::{Path, PathBuf};

use studio_domain::ps4_skater_gear::{thumbnail_png, SkaterGearCatalogue};
use studio_domain::{EditSession, Platform, Project, ProjectRole};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn install(cusa: &str) -> PathBuf {
    PathBuf::from(format!(r"H:\Emulators\ShadPS4\Games\{cusa}"))
}
fn session(db: &Path, meta: &Path) -> EditSession {
    EditSession::begin(Project::load(db, Platform::Ps4, ProjectRole::Nhlng, meta, repo("defs")).unwrap())
}

#[test]
#[ignore = "real PS4 installs under H:/Emulators/ShadPS4/Games and the genuine NHL 20 fixture (NHL_STUDIO_FIXTURES): select with --ignored"]
fn lists_thumbnails_and_census_match_r136() {
    let defs = repo("defs");
    // (title, CUSA, rows helmet / gloves / skates / stick / visor) — R136 §1.
    let cases: [(&str, &str, Option<[usize; 5]>); 7] = [
        ("nhl15", "CUSA00561", None),
        ("nhl16", "CUSA01909", None),
        ("nhl17", "CUSA04326", Some([12, 23, 10, 30, 6])),
        ("nhl18", "CUSA07569", Some([16, 17, 9, 33, 7])),
        ("nhl19", "CUSA11126", Some([16, 24, 41, 86, 7])),
        ("nhl20", "CUSA15439", Some([15, 26, 101, 161, 7])),
        ("nhl21", "CUSA18182", Some([18, 44, 123, 192, 7])),
    ];
    for (title, cusa, rows) in cases {
        let t0 = std::time::Instant::now();
        let cat = SkaterGearCatalogue::load(&install(cusa), &defs, title).unwrap_or_else(|e| panic!("{title}: {e}"));
        let got: Option<[usize; 5]> = rows.map(|_| std::array::from_fn(|i| cat.families[i].rows.as_ref().unwrap().len()));
        assert_eq!(got, rows, "{title}: Creation Zone rows");
        for f in &cat.families {
            assert!(!f.enum_values.is_empty(), "{title} {}: the named enum has values", f.family);
            if let Some(r) = &f.rows {
                assert!(r.iter().all(|x| f.enum_values.contains(&x.id)), "{title} {}: every row id is in the enum", f.family);
                assert!(r.iter().all(|x| f.thumbnails.contains_key(&x.id)), "{title} {}: every row has its thumbnail", f.family);
            }
        }
        eprintln!("{title}: rows {:?}, thumbnails {:?} ({:.1} s)", got, cat.families.iter().map(|f| f.thumbnails.len()).collect::<Vec<_>>(), t0.elapsed().as_secs_f32());
    }
    // Genuine NHL 20: 6 803 skater rows; helmet: 6 734 rows are Creation Zone items, 69 store 99 (outside); others all rows.
    let fx = PathBuf::from(std::env::var("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES (fails, never skips)")).join("databases/nhl20-ps4-cusa15439/inxp_00.big");
    let s20 = session(&fx.join("nhl_game.db"), &fx.join("nhl_game-meta.xml"));
    let cat20 = SkaterGearCatalogue::load(&install("CUSA15439"), &defs, "nhl20").unwrap();
    let c = s20.ps4_skater_gear_census(&cat20).unwrap();
    let n = |f: &str, k: &str| c.get(&(f.to_string(), k.to_string())).copied().unwrap_or(0);
    assert_eq!((n("helmet", "row"), n("helmet", "outside")), (6_734, 69), "{c:?}");
    for f in ["gloves", "skates", "stick", "visor"] {
        assert_eq!(n(f, "row"), 6_803, "{f}: {c:?}");
    }
    let h99 = cat20.check("helmet", 99).unwrap();
    assert_eq!((h99.class.as_str(), h99.text.as_str()), ("outside", "stored 99; not in the catalogue"));
    let v3 = cat20.check("visor", 3).unwrap();
    assert_eq!((v3.class.as_str(), v3.key.as_deref()), ("row", Some("$TXT_VISOR_AVIATOR")));
    // A thumbnail decodes (the first helmet row).
    let first = cat20.families[0].rows.as_ref().unwrap()[0].id;
    let th = cat20.check("helmet", first).unwrap().thumbnail.expect("thumbnail");
    let (w, h, png) = thumbnail_png(&install("CUSA15439"), &th, 128).unwrap();
    assert!(w > 0 && h > 0 && png.starts_with(b"\x89PNG"), "{th:?}");
    eprintln!("helmet {first}: {} → {w} × {h}", th.member);
    // NHL 15: stick 71 (one row) is not in the catalogue enum; no names, no thumbnails.
    let d15 = repo("anchors/ps4/nhl15/databases");
    let cat15 = SkaterGearCatalogue::load(&install("CUSA00561"), &defs, "nhl15").unwrap();
    let s71 = cat15.check("stick", 71).unwrap();
    assert_eq!(s71.class, "not_in_catalogue");
    let c15 = session(&d15.join("nhl_game.db"), &d15.join("nhl_game-meta.xml")).ps4_skater_gear_census(&cat15).unwrap();
    assert_eq!(c15.get(&("stick".into(), "not_in_catalogue".into())), Some(&1), "{c15:?}");
    // Identity: an NHL 17 project against the NHL 18 install is refused.
    assert!(SkaterGearCatalogue::load(&install("CUSA07569"), &defs, "nhl17").unwrap_err().to_string().contains("source mismatch"));
}

/// Receipt 57 — research 2026-09-28 handoff 08 (R141): every Creation Zone name cell resolves through the title's own
/// English dictionary (exact key, else the stored `loc_hash`), with R141's exact / hash / unresolved totals per title and only
/// the two placeholder words unresolved; the gear check and the mask catalogue carry the names; another title's dictionary
/// is never read.
#[test]
#[ignore = "real PS4 installs under H:/Emulators/ShadPS4/Games (read only): select with --ignored"]
fn creation_zone_names_resolve_as_r141() {
    use studio_domain::ps4_cz_names::{name_census, CzNames, Resolution};
    let defs = repo("defs");
    // (title, CUSA, exact, hash, unresolved) — R141 §1.
    let cases = [
        ("nhl17", "CUSA04326", 154, 19, 0),
        ("nhl18", "CUSA07569", 140, 34, 0),
        ("nhl19", "CUSA11126", 229, 49, 20),
        ("nhl20", "CUSA15439", 363, 61, 44),
        ("nhl21", "CUSA18182", 437, 64, 50),
    ];
    for (title, cusa, exact, hash, unres) in cases {
        let names = CzNames::load(&install(cusa), &defs, title).unwrap().expect("a recorded dictionary");
        let (counts, unresolved) = name_census(&install(cusa), &names).unwrap();
        let total = |c: &str| counts.iter().filter(|((_, _, k), _)| k == c).map(|(_, n)| n).sum::<usize>();
        assert_eq!((total("exact"), total("hash"), total("unresolved")), (exact, hash, unres), "{title}: {counts:?}");
        assert!(unresolved.keys().all(|k| k == "GoalieMaskPattern" || k == "GoalieMaskBrandAbbr"), "{title}: {unresolved:?}");
        eprintln!("{title}: {} in {} — exact {exact}, hash {hash}, unresolved {unres}", &names.sha256[..12], names.archive);
    }
    assert!(CzNames::load(&install("CUSA00561"), &defs, "nhl15").unwrap().is_none(), "NHL 15 has no lists and no dictionary record");
    // Another title's dictionary is never read: NHL 19's record names a patch archive the NHL 20 install does not have.
    assert!(CzNames::load(&install("CUSA15439"), &defs, "nhl19").is_err());

    // Genuine NHL 20 examples (handoff 08 §1).
    let n = CzNames::load(&install("CUSA15439"), &defs, "nhl20").unwrap().unwrap();
    assert_eq!(n.resolve("$bauer4500"), Some(("BAUER 4500".to_string(), Resolution::Hash)));
    assert_eq!(n.resolve("$TXT_VISOR_AVIATOR"), Some(("Aviator".to_string(), Resolution::Exact)));
    assert_eq!(n.resolve("$GoalieMaskPattern"), None);
    assert_eq!(n.mask_name("$TXT_STANDARD_MASK_DESC", "$bauer", "$yellow").as_deref(), Some("BAUER Yellow"));
    assert_eq!(n.mask_name("$TXT_STANDARD_MASK_DESC", "$bauer", "$GoalieMaskPattern"), None, "a template fills only when both parts resolve");

    let cat = SkaterGearCatalogue::load(&install("CUSA15439"), &defs, "nhl20").unwrap();
    assert!(cat.names_problem.is_none());
    let f = &cat.families[0];
    let row = &f.rows.as_ref().unwrap()[0];
    let c = cat.check(&f.family, row.id).unwrap();
    assert_eq!(c.name, n.resolve(&row.key).map(|(t, _)| t.trim().to_string()), "the check carries the row's name");
    assert!(c.name.is_some(), "{:?}", c);
    let masks = studio_domain::ps4_goalie_masks::MaskCatalogue::load(&install("CUSA15439"), &defs, "nhl20").unwrap();
    assert!(masks.names_problem.is_none());
    assert!(!masks.names.is_empty() && masks.names.values().all(|t| !t.contains('%') && !t.contains("GoalieMask")), "{:?}", masks.names);
    eprintln!("nhl20: {} named mask pairs, e.g. {:?}", masks.names.len(), masks.names.iter().next());
}
