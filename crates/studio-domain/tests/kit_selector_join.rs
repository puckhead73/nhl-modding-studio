//! Phase 2.6 (22 Sep 2026) — the gen7 kit-selector join against research 2026-09-21 pm handoff R56
//! on the real Xbox 360 and PS3 rendering trees.
//!
//! R56 measured `(style, teamartid, variant)` → jersey / pant / sock at **100 % of the
//! `stockteamjerseys` rows** on every title from NHL 11 to Legacy (627 / 676 / 684 / 704 / 659 /
//! 703). This reproduces that with Studio's own resolver — under the title's *measured contract*,
//! never by relaxing the rule to what the tree happens to hold (coordinator F1, 22 Sep 2026) — over
//! every team of the anchor, and also checks the residuals the handoff records: the shipped spare
//! pool (kits with files but no row) is non-empty and every one of its complete candidates really
//! has every required file, and the teams that carry no row at all are the league-13 custom clubs.
//!
//! Read-only over `X:`. Nothing here claims what the game dresses a team in.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_domain::kit_files::{completeness, kit_contract, nameplate_measured, team_kits, ComponentStatus, Completeness, KitContract, KitPart, SockGrammar, CUSTOM_TEAM_ART_ID};
use studio_domain::team_tabs::team_jerseys;
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
    /// Rows the research counted (R56 §1).
    rows: usize,
    sock: SockGrammar,
}

const X360: &str = r"X:\Games\Emulation\Roms\XBOX360";
const PS3: &str = r"X:\Games\Emulation\Roms\Sony Playstation 3";

/// The six Xbox 360 titles R56 measured, the four PS3 titles whose loose `USRDIR/rendering` tree
/// is on disk, and — since the coordinator review of receipts 10–17 (item 3) — PS3 NHL 12 / 13,
/// whose kits live inside `cacherender.big` / `nocacherender.big`: the resolver lists those from
/// the archives' file tables (the same way R56 measured them) and a present component names its
/// archive member instead of a loose path.
const TITLES: &[Title] = &[
    Title { game: "nhl11", platform: Platform::Xbox, anchor_dir: "xbox360", rendering: r"NHL 11\NHL 11\rendering", rows: 627, sock: SockGrammar::TwoFile },
    Title { game: "nhl12", platform: Platform::Xbox, anchor_dir: "xbox360", rendering: r"NHL 12\NHL 12\rendering", rows: 676, sock: SockGrammar::TwoFile },
    Title { game: "nhl13", platform: Platform::Xbox, anchor_dir: "xbox360", rendering: r"NHL 13\NHL 13\rendering", rows: 684, sock: SockGrammar::SingleFile },
    Title { game: "nhl14", platform: Platform::Xbox, anchor_dir: "xbox360", rendering: r"NHL 14\NHL 14\rendering", rows: 704, sock: SockGrammar::SingleFile },
    Title { game: "nhl15", platform: Platform::Xbox, anchor_dir: "xbox360", rendering: r"NHL 15\NHL 15\rendering", rows: 659, sock: SockGrammar::SingleFile },
    Title { game: "nhllegacy", platform: Platform::Xbox, anchor_dir: "xbox360", rendering: r"NHL Legacy\NHL Legacy\rendering", rows: 703, sock: SockGrammar::SingleFile },
    Title { game: "nhl11", platform: Platform::Ps3, anchor_dir: "ps3", rendering: r"NHL 11\PS3_GAME\USRDIR\rendering", rows: 627, sock: SockGrammar::TwoFile },
    Title { game: "nhl14", platform: Platform::Ps3, anchor_dir: "ps3", rendering: r"NHL 14\PS3_GAME\USRDIR\rendering", rows: 704, sock: SockGrammar::SingleFile },
    Title { game: "nhl15", platform: Platform::Ps3, anchor_dir: "ps3", rendering: r"NHL 15\PS3_GAME\USRDIR\rendering", rows: 659, sock: SockGrammar::SingleFile },
    Title { game: "nhllegacy", platform: Platform::Ps3, anchor_dir: "ps3", rendering: r"NHL Legacy - Vanilla\PS3_GAME\USRDIR\rendering", rows: 703, sock: SockGrammar::SingleFile },
    Title { game: "nhl12", platform: Platform::Ps3, anchor_dir: "ps3", rendering: "NHL 12\\[BLUS 30791] NHL\u{ae} 12\\PS3_GAME\\USRDIR\\rendering", rows: 676, sock: SockGrammar::TwoFile },
    Title { game: "nhl13", platform: Platform::Ps3, anchor_dir: "ps3", rendering: r"NHL 13\PS3_GAME\USRDIR\rendering", rows: 684, sock: SockGrammar::SingleFile },
];

#[test]
#[ignore = "real Xbox 360 / PS3 installs under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn every_stock_kit_row_resolves_its_required_components_on_the_real_trees() {
    for t in TITLES {
        let tag = format!("{} {}", t.game, t.platform.as_str());
        let rendering = Path::new(if t.platform == Platform::Ps3 { PS3 } else { X360 }).join(t.rendering);
        assert!(rendering.is_dir(), "{tag}: {} missing (fails, never skips)", rendering.display());
        let base = repo(&format!("anchors/{}/{}/databases", t.anchor_dir, t.game));
        let bytes = std::fs::read(base.join("nhlng.db")).unwrap_or_else(|e| panic!("{tag}: {e}"));
        let db = TdbFile::parse(&bytes).expect("anchor parses");
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(base.join("nhlng-meta.xml")).unwrap()).expect("meta");

        let contract = kit_contract(Some(t.game), t.platform);
        let KitContract::Measured { sock, extension, .. } = &contract else { panic!("{tag}: the contract must be measured") };
        assert_eq!(*sock, t.sock, "{tag}: the contract's sock grammar is not the measured one");
        assert_eq!(extension.as_str(), if t.platform == Platform::Ps3 { "rpsgl" } else { "rx2" }, "{tag}");

        // Every team and its own art id, from the team table.
        let teams = studio_domain::kit_files::team_art_ids(&db, &meta);
        assert!(!teams.is_empty(), "{tag}: no teams read");

        // R101 / R103 (26 Sep): the nameplate association, on the six measured trees only.
        let nameplates = nameplate_measured(Some(t.game), t.platform);
        let (mut nameplate_present, mut nameplate_looked) = (0usize, 0usize);
        let mut rows = 0usize;
        let mut verdicts: BTreeMap<Completeness, usize> = BTreeMap::new();
        let mut refused = 0usize;
        let (mut teams_with_rows, mut sock_components_present, mut archive_members) = (0usize, 0usize, 0usize);
        let mut spare_verdicts: BTreeMap<Completeness, usize> = BTreeMap::new();
        let mut not_complete_examples: Vec<String> = Vec::new();
        let components_per_kit = 2 + match sock {
            SockGrammar::SingleFile => 1,
            SockGrammar::TwoFile => 2,
        };
        for (team_id, art_id) in &teams {
            let jerseys = team_jerseys(&db, *team_id);
            if jerseys.is_empty() {
                continue;
            }
            teams_with_rows += 1;
            let v = team_kits(*team_id, Some(*art_id), &jerseys, contract.clone(), Some(&rendering), nameplates);
            rows += v.kits.len();
            for k in &v.kits {
                if k.refusal.is_some() {
                    refused += 1;
                    continue;
                }
                assert_eq!(k.components.len(), components_per_kit, "{tag}: every row lists every required component");
                if let Some(np) = &k.nameplate {
                    nameplate_looked += 1;
                    assert_eq!(np.file_stem, format!("name_{}_{}_{}_cm", k.style, k.team_art_id, k.variant), "{tag}");
                    if np.status == ComponentStatus::Present {
                        nameplate_present += 1;
                    }
                }
                // The verdict on the view is the shared rule's verdict, recomputed here.
                assert_eq!(k.completeness, completeness(&k.components), "{tag}");
                *verdicts.entry(k.completeness).or_default() += 1;
                sock_components_present += k.components.iter().filter(|c| c.part == KitPart::Sock && c.status == ComponentStatus::Present).count();
                if k.completeness != Completeness::Complete && not_complete_examples.len() < 8 {
                    let missing: Vec<&str> = k.components.iter().filter(|c| c.status != ComponentStatus::Present).map(|c| c.label.as_str()).collect();
                    not_complete_examples.push(format!("team {team_id} art {} ({}, {}, {}) {:?}: {missing:?}", k.team_art_id, k.style, k.team_art_id, k.variant, k.completeness));
                }
                for c in &k.components {
                    if c.status == ComponentStatus::Present {
                        match (&c.path, &c.archive_member) {
                            (Some(path), None) => {
                                let p = Path::new(path);
                                assert!(p.is_file() && p.extension().is_some_and(|e| e == extension.as_str()), "{tag}: {} must be a real file with the platform extension", p.display());
                            }
                            (None, Some(member)) => {
                                archive_members += 1;
                                assert!((member.starts_with("cacherender.big:") || member.starts_with("nocacherender.big:")) && member.ends_with(&format!(".{extension}")), "{tag}: {member} must name a render archive member with the platform extension");
                            }
                            other => panic!("{tag}: a present component has neither a loose path nor an archive member: {other:?}"),
                        }
                    }
                }
            }
            for s in &v.spares {
                assert_eq!(s.components.len(), components_per_kit, "{tag}");
                assert_eq!(s.completeness, completeness(&s.components), "{tag}");
                // A complete spare candidate has every required file on disk — no complete spare
                // without every component (the F1 rule, checked on the real pool).
                if s.completeness == Completeness::Complete {
                    assert!(s.components.iter().all(|c| c.path.as_deref().is_some_and(|p| Path::new(p).is_file()) || c.archive_member.is_some()), "{tag}: complete spare ({}, {}) has a component with no file", s.style, s.variant);
                }
                *spare_verdicts.entry(s.completeness).or_default() += 1;
            }
            assert!(!v.reassignment_supported, "{tag}: no write contract exists");
        }
        let complete = verdicts.get(&Completeness::Complete).copied().unwrap_or(0);
        let spare_complete = spare_verdicts.get(&Completeness::Complete).copied().unwrap_or(0);
        eprintln!(
            "{tag}: {teams_with_rows} teams with rows, {rows} rows — {complete} complete, {} incomplete, {} unknown, {refused} refused; {sock_components_present} sock components present; {archive_members} components listed from render archives; spare candidates {} complete / {} incomplete / {} unknown",
            verdicts.get(&Completeness::Incomplete).copied().unwrap_or(0),
            verdicts.get(&Completeness::Unknown).copied().unwrap_or(0),
            spare_complete,
            spare_verdicts.get(&Completeness::Incomplete).copied().unwrap_or(0),
            spare_verdicts.get(&Completeness::Unknown).copied().unwrap_or(0),
        );
        for e in &not_complete_examples {
            eprintln!("  not complete: {e}");
        }
        eprintln!("{tag}: nameplate measured {nameplates}; looked up on {nameplate_looked} rows, present on {nameplate_present}");
        assert_eq!(rows, t.rows, "{tag}: row count differs from the research census");
        if nameplates {
            assert_eq!((nameplate_looked, nameplate_present), (t.rows, t.rows), "{tag}: R103 resolves every row's nameplate");
        } else {
            assert_eq!(nameplate_looked, 0, "{tag}: not measured, not looked up");
        }
        assert_eq!(refused, 0, "{tag}: a stock row was refused — every shipped row's art id matches its team's");
        assert_eq!(complete, t.rows, "{tag}: {} rows did not resolve every required component (examples above); the research measured 100 %", t.rows - complete);
        assert_eq!(sock_components_present, t.rows * (components_per_kit - 2), "{tag}: every row names every sock file its grammar requires");
        assert!(spare_complete > 0, "{tag}: no complete shipped spare candidate, but the research counted 26–322 unused kits");
        // The custom art id never appears among the stock rows.
        assert!(!teams.values().any(|a| *a == CUSTOM_TEAM_ART_ID) || teams_with_rows < teams.len(), "{tag}");
    }
}
