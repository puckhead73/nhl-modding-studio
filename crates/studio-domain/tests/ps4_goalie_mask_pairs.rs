//! Receipt 51 — G3: PS4 goalie (maskid, masktype) pairs against each title's own vault (research 2026-09-28 handoff 02 /
//! R135 / R135a). Real PS4 installs under `H:\Emulators\ShadPS4\Games\<CUSA>` (only `databigs/boot.big` is read) and
//! the committed anchors / the genuine NHL 20 private fixture (read only).
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_domain::ps4_goalie_masks::{MaskCatalogue, PairClass};
use studio_domain::{EditSession, Platform, Project, ProjectRole};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn install(cusa: &str) -> PathBuf {
    PathBuf::from(format!(r"H:\Emulators\ShadPS4\Games\{cusa}"))
}

fn anchor(title: &str) -> EditSession {
    let d = repo(&format!("anchors/ps4/{title}/databases"));
    EditSession::begin(Project::load(d.join("nhl_game.db"), Platform::Ps4, ProjectRole::Nhlng, d.join("nhl_game-meta.xml"), repo("defs")).unwrap())
}

fn census(s: &EditSession, cat: &MaskCatalogue) -> BTreeMap<&'static str, usize> {
    let mut m = BTreeMap::new();
    for r in s.ps4_goalie_rows(cat).unwrap() {
        *m.entry(r.class.code()).or_insert(0) += 1;
    }
    m
}

#[test]
#[ignore = "real PS4 installs under H:/Emulators/ShadPS4/Games (boot.big read only): select with --ignored"]
fn every_title_catalogue_and_goalie_census_match_r135() {
    let defs = repo("defs");
    // (title, CUSA, packs, anchor census: unique / other types / absent / not a helmet type) — R135 §2 / §3.
    let cases: [(&str, &str, usize, Option<[usize; 4]>); 7] = [
        ("nhl15", "CUSA00561", 506, Some([559, 0, 0, 0])),
        ("nhl16", "CUSA01909", 492, Some([556, 3, 7, 0])),
        ("nhl17", "CUSA04326", 526, Some([596, 0, 5, 0])),
        ("nhl18", "CUSA07569", 578, Some([714, 0, 0, 0])),
        ("nhl19", "CUSA11126", 940, Some([743, 0, 0, 4])),
        ("nhl20", "CUSA15439", 974, None),
        ("nhl21", "CUSA18182", 991, Some([744, 0, 0, 5])),
    ];
    for (title, cusa, packs, want) in cases {
        let root = install(cusa);
        assert!(root.join("databigs/boot.big").is_file(), "{}: missing (fails, never skips)", root.display());
        let t0 = std::time::Instant::now();
        let cat = MaskCatalogue::load(&root, &defs, title).unwrap_or_else(|e| panic!("{title}: {e}"));
        assert_eq!(cat.packs.len(), packs, "{title}: packs (R135 §2)");
        assert_eq!(cat.source.title.as_deref(), Some(title));
        assert!(cat.packs.iter().all(|p| p.colour_map.as_deref().is_some_and(|c| c.contains("goaliehelmet_"))), "{title}: every pack resolves its colour map");
        let got = want.map(|_| {
            let c = census(&anchor(title), &cat);
            [c.get("unique").copied().unwrap_or(0), c.get("only_other_helmet_types").copied().unwrap_or(0), c.get("absent").copied().unwrap_or(0), c.get("not_a_catalogue_helmet_type").copied().unwrap_or(0)]
        });
        assert_eq!(got, want, "{title}: goalie census (R135 §3)");
        // Receipt 54: thumbnails — NHL 17 / 18 by the list's 28 rows; NHL 19–21 every pair whose `mask<brand × 1000 + maskid>`
        // exists (R135 §5: 426 / 443 / 453); NHL 15 / 16 none.
        let thumbs = cat.thumbnails.len();
        let want_thumbs = match title { "nhl15" | "nhl16" => 0, "nhl17" | "nhl18" => 28, "nhl19" => 426, "nhl20" => 443, _ => 453 };
        assert_eq!(thumbs, want_thumbs, "{title}: mask thumbnails (R135 §5)");
        eprintln!("{title}: {} packs, helmet types {:?}, census {:?}, thumbnails {thumbs} ({:.1} s)", cat.packs.len(), cat.helmet_types, got, t0.elapsed().as_secs_f32());
    }
}

#[test]
#[ignore = "real PS4 installs (boot.big) and the genuine NHL 20 private fixture (NHL_STUDIO_FIXTURES): select with --ignored"]
fn contract_fixture_cases_and_source_identity() {
    let defs = repo("defs");
    let fx = PathBuf::from(std::env::var("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES (fails, never skips)")).join("databases/nhl20-ps4-cusa15439/inxp_00.big");
    let s20 = EditSession::begin(Project::load(fx.join("nhl_game.db"), Platform::Ps4, ProjectRole::Nhlng, fx.join("nhl_game-meta.xml"), defs.clone()).unwrap());
    let cat20 = MaskCatalogue::load(&install("CUSA15439"), &defs, "nhl20").unwrap();
    let rows = s20.ps4_goalie_rows(&cat20).unwrap();
    // unique: row 1 maskid 125 type 0 → goaliehelmet_itech_generic_8_texpack, colour map …_generic_8_cm.
    let r1 = &rows[1];
    assert_eq!((r1.maskid, r1.helmet_type), (125, 0));
    let PairClass::Unique(p) = &r1.class else { panic!("{:?}", r1.class) };
    assert_eq!(p.debug_name.as_deref(), Some("goaliehelmet_itech_generic_8_texpack"));
    assert!(p.colour_map.as_deref().unwrap().ends_with("generic_8_cm"), "{:?}", p.colour_map);
    // Thumbnail: the list's row 0 (assetid 119 = 0 × 1000 + 119) names mask119 for (119, 0).
    let (t, basis) = cat20.thumbnail(119, 0).expect("mask 119 thumbnail");
    assert!(t.member.to_ascii_lowercase().ends_with("/creationzone/equipment/mask/mask119.gnf"), "{t:?}");
    assert_eq!(basis.as_str(), "creation_zone_row");
    let (w, h, _) = studio_domain::ps4_skater_gear::thumbnail_png(&install("CUSA15439"), t, 128).unwrap();
    assert!(w > 0 && h > 0);
    // A pack outside the list rows: a name-convention candidate.
    assert!(cat20.thumbnails.values().any(|(_, b)| b.as_str() == "name_convention_candidate"));
    // type 99: row 88.
    assert_eq!((rows[88].maskid, rows[88].helmet_type, rows[88].class.clone()), (0, 99, PairClass::NotCatalogueHelmetType));
    let c = rows.iter().fold(BTreeMap::new(), |mut m, r| {
        *m.entry(r.class.code()).or_insert(0) += 1;
        m
    });
    assert_eq!((rows.len(), c.get("unique"), c.get("not_a_catalogue_helmet_type")), (773, Some(&768), Some(&5)), "genuine NHL 20 (R135 §3)");

    // NHL 16: row 0 maskid 198 type 0 → absent; row 78 maskid 77 type 0 → only other helmet types.
    let cat16 = MaskCatalogue::load(&install("CUSA01909"), &defs, "nhl16").unwrap();
    let r16 = anchor("nhl16").ps4_goalie_rows(&cat16).unwrap();
    assert_eq!((r16[0].maskid, r16[0].class.clone()), (198, PairClass::Absent));
    assert!(matches!(&r16[78].class, PairClass::OnlyOtherHelmetTypes(t) if !t.contains(&0)), "{:?}", r16[78]);
    eprintln!("NHL 16 row 78: {}", r16[78].class.describe(r16[78].maskid, r16[78].helmet_type));

    // Team slot paint 248 (PRSA): no record on NHL 21; a shipped paint lists its helmet types.
    let cat21 = MaskCatalogue::load(&install("CUSA18182"), &defs, "nhl21").unwrap();
    assert!(cat21.paint_helmet_types(248).is_empty() && cat21.paint_helmet_types(249).is_empty());
    assert!(!cat21.paint_helmet_types(125).is_empty());

    // Source identity: an NHL 19 project with the NHL 20 install attached is refused, never borrowed.
    let e = MaskCatalogue::load(&install("CUSA15439"), &defs, "nhl19").unwrap_err().to_string();
    assert!(e.contains("source mismatch") && e.contains("nhl20"), "{e}");
    // `anchors/ps4/nhl20` is the NHL 19 base copy: classified against NHL 19's catalogue it matches NHL 19's census.
    let cat19 = MaskCatalogue::load(&install("CUSA11126"), &defs, "nhl19").unwrap();
    assert_eq!(census(&anchor("nhl20"), &cat19), census(&anchor("nhl19"), &cat19));
}
