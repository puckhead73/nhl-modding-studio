//! Phase 2.5 (22 Sep 2026) — the gen7 arena-asset join of research 2026-09-21 pm handoff R62 on the
//! real Xbox 360 / PS3 rendering trees, through Studio's own resolver: every arena row's
//! `arenaartid` names its venue model (09 → Legacy) and jumbotron (12 → Legacy), the spare-venue
//! counts equal the handoff's residuals, the centre-ice logo / banners resolve through the home
//! team's art id for every tenanted arena, and nothing in the candidate / unbound families is
//! asserted. Read-only over `X:`; a missing tree fails, never skips.

use std::path::{Path, PathBuf};

use studio_domain::arena_assets::{arena_asset_contract, arena_asset_inputs, arena_assets, Binding};
use studio_domain::kit_files::ComponentStatus;
use studio_domain::Platform;
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

struct Title {
    game: &'static str,
    platform: Platform,
    anchor_dir: &'static str,
    rendering: &'static str,
    /// R62 (`data/r62-gen7-arena-asset-join.json` residuals): arena rows, spare stadium ids, spare jumbotron ids (`None` = no jumbotron dir ships).
    rows: usize,
    spare_stadiums: usize,
    spare_jumbotrons: Option<usize>,
}

const X360: &str = r"X:\Games\Emulation\Roms\XBOX360";
const PS3: &str = r"X:\Games\Emulation\Roms\Sony Playstation 3";

const TITLES: &[Title] = &[
    Title { game: "nhl09", platform: Platform::Xbox, anchor_dir: "xbox360", rendering: r"NHL 09\NHL 09 (USA)\rendering", rows: 188, spare_stadiums: 4, spare_jumbotrons: None },
    Title { game: "nhl11", platform: Platform::Xbox, anchor_dir: "xbox360", rendering: r"NHL 11\NHL 11\rendering", rows: 242, spare_stadiums: 10, spare_jumbotrons: None },
    Title { game: "nhl13", platform: Platform::Xbox, anchor_dir: "xbox360", rendering: r"NHL 13\NHL 13\rendering", rows: 174, spare_stadiums: 10, spare_jumbotrons: Some(12) },
    Title { game: "nhl15", platform: Platform::Xbox, anchor_dir: "xbox360", rendering: r"NHL 15\NHL 15\rendering", rows: 178, spare_stadiums: 12, spare_jumbotrons: Some(14) },
    Title { game: "nhllegacy", platform: Platform::Xbox, anchor_dir: "xbox360", rendering: r"NHL Legacy\NHL Legacy\rendering", rows: 187, spare_stadiums: 12, spare_jumbotrons: Some(14) },
    Title { game: "nhllegacy", platform: Platform::Ps3, anchor_dir: "ps3", rendering: r"NHL Legacy - Vanilla\PS3_GAME\USRDIR\rendering", rows: 187, spare_stadiums: 12, spare_jumbotrons: Some(14) },
];

#[test]
#[ignore = "real Xbox 360 / PS3 installs under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn every_arena_row_names_its_venue_files_and_the_spare_venues_match_the_research() {
    for t in TITLES {
        let tag = format!("{} {}", t.game, t.platform.as_str());
        let rendering = Path::new(if t.platform == Platform::Ps3 { PS3 } else { X360 }).join(t.rendering);
        assert!(rendering.is_dir(), "{tag}: {} missing (fails, never skips)", rendering.display());
        let base = repo(&format!("anchors/{}/{}/databases", t.anchor_dir, t.game));
        let bytes = std::fs::read(base.join("nhlng.db")).unwrap_or_else(|e| panic!("{tag}: {e}"));
        let db = TdbFile::parse(&bytes).expect("anchor parses");
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(base.join("nhlng-meta.xml")).unwrap()).expect("meta");
        let contract = arena_asset_contract(Some(t.game), t.platform);

        let arena = db.table_canonical("StEO").expect("arena table");
        let key = arena.fields.iter().find(|f| f.id.canonical(db.endian) == "vfEq").expect("arena key");
        let arena_ids: Vec<u64> = (0..arena.current_records as u32).filter_map(|i| db.read_u64(arena, i, key)).collect();
        assert_eq!(arena_ids.len(), t.rows, "{tag}: arena rows differ from the research census");

        let (mut venue_ok, mut jumbo_ok, mut tenanted, mut logo_ok, mut banner_ok, mut bm_present) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
        let mut home_arena_link = false;
        let mut spare_stadiums: Option<Vec<u64>> = None;
        let mut spare_jumbotrons: Option<Vec<u64>> = None;
        let mut failures: Vec<String> = Vec::new();
        for id in &arena_ids {
            let inputs = arena_asset_inputs(&db, &meta, *id).unwrap_or_else(|| panic!("{tag}: arena {id} has no inputs"));
            assert!(inputs.arena_art_id.is_some(), "{tag}: arena {id} carries no arenaartid");
            home_arena_link = inputs.home_arena_link;
            let v = arena_assets(&inputs, contract.clone(), Some(&rendering));
            let fam = |name: &str| v.families.iter().find(|f| f.family == name).unwrap();
            let venue = fam("venue_model");
            assert_eq!(venue.binding, Binding::Established);
            if !venue.files.is_empty() && venue.files.iter().all(|f| f.status == ComponentStatus::Present) {
                venue_ok += 1;
            } else if failures.len() < 8 {
                failures.push(format!("arena {id} art {:?}: venue {:?}", inputs.arena_art_id, venue.files.iter().map(|f| (&f.file_stem, f.status)).collect::<Vec<_>>()));
            }
            spare_stadiums.get_or_insert_with(|| venue.spare_keys.clone());
            let jt = fam("jumbotron");
            if t.spare_jumbotrons.is_some() {
                assert_eq!(jt.binding, Binding::Established, "{tag}");
                if jt.files.iter().all(|f| f.status == ComponentStatus::Present) && !jt.files.is_empty() {
                    jumbo_ok += 1;
                }
                spare_jumbotrons.get_or_insert_with(|| jt.spare_keys.clone());
            } else {
                assert_eq!(jt.binding, Binding::NoEstablishedBinding, "{tag}: no jumbotron directory ships");
                assert!(jt.files.is_empty());
            }
            if !inputs.home_teams.is_empty() {
                tenanted += 1;
                let logo = fam("centre_ice_logo");
                assert_eq!(logo.binding, Binding::ViaHomeTeam);
                assert_eq!(logo.files.len(), inputs.home_teams.len());
                if logo.files.iter().all(|f| f.status == ComponentStatus::Present) {
                    logo_ok += 1;
                }
                if fam("banners").files.iter().all(|f| f.status == ComponentStatus::Present) {
                    banner_ok += 1;
                }
            }
            let bm = fam("bounce_map");
            assert_eq!(bm.binding, Binding::Candidate);
            if bm.files.iter().any(|f| f.status == ComponentStatus::Present) {
                bm_present += 1;
            }
            assert!(fam("ice_layers").files.is_empty(), "{tag}: an unbound family must resolve nothing");
        }
        let spare_stadiums = spare_stadiums.unwrap();
        eprintln!(
            "{tag}: {} arenas — venue {venue_ok}, jumbotron {jumbo_ok}, bounce map present {bm_present}; {tenanted} tenanted: logo {logo_ok}, banners {banner_ok}; spare stadiums {} {:?}, spare jumbotrons {:?}",
            arena_ids.len(),
            spare_stadiums.len(),
            spare_stadiums,
            spare_jumbotrons.as_ref().map(Vec::len)
        );
        for f in &failures {
            eprintln!("  failed: {f}");
        }
        assert_eq!(venue_ok, t.rows, "{tag}: every arena row names an existing venue model (R62: 100 %)");
        assert_eq!(spare_stadiums.len(), t.spare_stadiums, "{tag}: spare stadium ids differ from R62 §1");
        if let Some(n) = t.spare_jumbotrons {
            assert_eq!(jumbo_ok, t.rows, "{tag}: every arena row names an existing jumbotron (R62: 100 %)");
            assert_eq!(spare_jumbotrons.unwrap().len(), n, "{tag}: spare jumbotron ids differ from R62 §1");
        }
        if home_arena_link {
            assert!(tenanted > 0, "{tag}: the team table links home arenas but no arena has a tenant?");
            // The team-keyed families resolve for the tenanted arenas at the rate R62 §2 measured
            // for team art ids (191 / 218 logos, 210 / 218 banners → the large majority, never all).
            assert!(logo_ok * 10 >= tenanted * 7, "{tag}: centre logos resolved for only {logo_ok} of {tenanted} tenanted arenas");
            assert!(banner_ok * 10 >= tenanted * 7, "{tag}: banners resolved for only {banner_ok} of {tenanted} tenanted arenas");
        } else {
            // The early gen7 titles carry no `kOtt.buDa`: the team-keyed families say so.
            assert_eq!(tenanted, 0, "{tag}");
            eprintln!("  (no home-arena link on this title: the team-keyed families are not resolved through a tenant)");
        }
        if t.game != "nhl09" && t.game != "nhl11" {
            assert_eq!(bm_present, t.rows, "{tag}: R62 §3 measured every row's bouncemap value naming a file on 13 / Legacy");
        }
    }
}
