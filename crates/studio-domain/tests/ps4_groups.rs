//! P1-C / P1-C-R1 — PS4 competition groups (research handoff P1.2-PS4, 2026-09-18): the NHL
//! 15 → 21 executables' group tables baked in `defs/leagues/groups/ps4-<game>.toml` by the
//! repository tool `competition-groups`, bound to the title by recorded identity or structural
//! signature, carrying the witnessed (conference, division) arrangements, and consumed by the
//! same league/membership machinery as NHL 12 → Legacy.
//!
//! Offline implementation checks only (committed anchors, private fixtures where `--ignored`,
//! temp copies for saves). Nothing here says what the game does with a moved team.

use std::path::{Path, PathBuf};

use studio_domain::competition_groups::{LeagueShape, Provenance, EVIDENCE_INFERRED, EVIDENCE_TRACED_STATIC, EVIDENCE_UNKNOWN};
use studio_domain::leagues::AttachedDictionary;
use studio_domain::membership::GroupModel;
use studio_domain::{EditSession, LeagueCatalog, LeagueVocabularies, Platform, Project, ProjectRole, Workspace};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn vocabs() -> LeagueVocabularies {
    LeagueVocabularies::from_dir(repo("defs/leagues")).unwrap()
}

/// The PS4 anchors that are what they claim; `anchors/ps4/nhl20` is an NHL 19 base copy and is
/// exercised separately.
const G77: &str = "TXT_LEAGUE_NHL_ALUMNI / TXT_LEAGUE_NHL_ALL_TIME (single group)";
const PS4_ANCHORS: [&str; 6] = ["nhl15", "nhl16", "nhl17", "nhl18", "nhl19", "nhl21"];

fn catalog_of(bytes: &[u8], meta: &FieldMetaIndex, claimed: Option<&str>, platform: Option<Platform>, dict: Option<&AttachedDictionary>) -> LeagueCatalog {
    let db = TdbFile::parse(bytes).unwrap();
    LeagueCatalog::build_for_target(&db, meta, &vocabs(), claimed, platform, dict)
}

/// Build the catalog over a committed PS4 anchor, with its dictionary when the anchor ships one
/// (NHL 15/16/17/18; the nhl20 anchor's `eng_us.db` is the NHL 19 dictionary — a base copy).
fn ps4_catalog(game: &str, claimed: &str, platform: Option<Platform>) -> LeagueCatalog {
    let dir = repo(&format!("anchors/ps4/{game}/databases"));
    let bytes = std::fs::read(dir.join("nhl_game.db")).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(dir.join("nhl_game-meta.xml")).unwrap()).unwrap();
    let dict_path = dir.join("eng_us.db");
    let dict = dict_path
        .is_file()
        .then(|| AttachedDictionary::load(dict_path.to_str().unwrap()).unwrap());
    catalog_of(&bytes, &meta, Some(claimed), platform, dict.as_ref())
}

fn open_ps4(game: &str, db: &Path, meta: &Path) -> Workspace {
    let project = Project::load(db, Platform::Ps4, ProjectRole::Nhlng, meta, repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(project), db.to_string_lossy().into_owned());
    ws.set_game_id(game);
    ws
}

fn anchor_ws(game: &str) -> Workspace {
    let dir = repo(&format!("anchors/ps4/{game}/databases"));
    open_ps4(game, &dir.join("nhl_game.db"), &dir.join("nhl_game-meta.xml"))
}

fn team_by_abbrev(ws: &Workspace, abbrev: &str) -> u64 {
    ws.display()
        .team_summaries()
        .unwrap()
        .iter()
        .find(|t| t.abbrev == abbrev)
        .unwrap_or_else(|| panic!("team {abbrev}"))
        .team_id
}

fn league_by_key(cat: &LeagueCatalog, key: &str) -> u64 {
    cat.leagues
        .iter()
        .find(|l| l.name_key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("league {key}"))
        .code
}

fn unchanged(ws: &Workspace, bytes: &[u8], depth: usize, rev: &studio_domain::WorkspaceRevision, what: &str) {
    assert_eq!(ws.primary().bytes(), bytes, "{what}: bytes unchanged");
    assert_eq!(ws.undo_depth(), depth, "{what}: history unchanged");
    assert_eq!(&ws.revision(), rev, "{what}: revision unchanged");
}

// ---------------------------------------------------------------------------------------------
// 1. Every PS4 anchor selects its own table by recorded identity; ids are title-specific;
//    evidence is carried per row; the witnessed structure is present.
// ---------------------------------------------------------------------------------------------

#[test]
fn every_ps4_anchor_selects_its_own_table_by_recorded_identity() {
    for game in PS4_ANCHORS {
        let cat = ps4_catalog(game, game, Some(Platform::Ps4));
        let c = cat.competition.as_ref().unwrap_or_else(|| panic!("{game}: {:?}", cat.competition_blocker));
        assert_eq!(c.evidence, "traced", "{game}: witnessed on this (game, platform), title verified");
        assert!(c.witness.contains("CUSA"), "{game}: PS4 witness, not the Xbox xex: {}", c.witness);
        assert!(matches!(c.provenance, Some(Provenance::Recorded { ref role, .. }) if role == "base"), "{game}: {:?}", c.provenance);
        let fit = c.fit.as_ref().unwrap();
        assert_eq!((fit.conflict, fit.unknown, fit.misplaced), (0, 0, 0), "{game}: {}", fit.summary());
        assert!(fit.codes_without_group.is_empty(), "{game}: {:?}", fit.codes_without_group);
        assert!(c.has_pairs(), "{game}: witnessed structure baked");
        // The NHL block is ids 1 / 6–11 on every PS4 title; West pairs with Pacific/Central,
        // East with Atlantic/Metropolitan — and nothing else.
        assert_eq!(c.league_group(0).unwrap().id, 1, "{game}");
        assert_eq!(c.regular_pairs(0), [(6, 8), (6, 9), (7, 10), (7, 11)], "{game}");
        assert_eq!(c.group(6).unwrap().label, "Western Conference", "{game}");
        assert_eq!(c.group(2).unwrap().kind, "unused", "{game}: duplicate NHL pointer");
        let src = c.group(8).unwrap().label_source;
        match game {
            "nhl15" | "nhl16" | "nhl17" | "nhl18" => assert_eq!(src, "dictionary", "{game}"),
            _ => assert_eq!(src, "observed", "{game}: no dictionary committed for this anchor"),
        }
        assert_eq!(c.group(8).unwrap().label, "Pacific Division", "{game}");
        for g in c.all() {
            assert!(
                [EVIDENCE_TRACED_STATIC, EVIDENCE_INFERRED, EVIDENCE_UNKNOWN].contains(&g.evidence.as_str()),
                "{game}: group {} evidence {:?}",
                g.id,
                g.evidence
            );
            if g.evidence == EVIDENCE_UNKNOWN {
                // Research 2026-09-21 handoff 08 §3: the keyless league-kind groups (77 / 78) are
                // labelled by rule from their leagues; any other keyless group stays unnamed.
                if g.kind == "league" {
                    assert!(g.key.is_empty() && g.label_source == "derived-from-leagues" && g.label.ends_with(" (single group)"), "{game}: {g:?}");
                } else {
                    assert!(g.key.is_empty() && g.label_source == "unknown" && g.label == format!("Unnamed group {}", g.id), "{game}: {g:?}");
                }
            } else if g.kind != "unused" {
                assert!(!g.key.is_empty(), "{game}: group {} has a key", g.id);
            }
        }
    }
}

#[test]
fn group_ids_are_title_specific_on_ps4_and_the_gen7_tables_are_untouched() {
    let c15 = ps4_catalog("nhl15", "nhl15", Some(Platform::Ps4));
    let c16 = ps4_catalog("nhl16", "nhl16", Some(Platform::Ps4));
    assert_eq!(c15.competition.as_ref().unwrap().league_group(league_by_key(&c15, "OHL")).unwrap().id, 26);
    assert_eq!(c16.competition.as_ref().unwrap().league_group(league_by_key(&c16, "OHL")).unwrap().id, 25);
    assert_eq!(c15.competition.as_ref().unwrap().for_league(1, "division").len(), 6, "NHL 15 AHL: six divisions");
    assert_eq!(c16.competition.as_ref().unwrap().for_league(1, "division").len(), 4);
    assert_eq!(c15.competition.as_ref().unwrap().group(54).unwrap().key, "CUSTOM_TEAMS");
    assert_eq!(c16.competition.as_ref().unwrap().group(53).unwrap().key, "CUSTOM_TEAMS");
    // The Xbox 360 / PS3 NHL 15 table (NHL = id 0): same game id, other source; the Xbox anchor
    // is its recorded identity, the byte-identical PS3 anchor therefore too.
    let a = repo("anchors/xbox360/nhl15/databases");
    let bytes = std::fs::read(a.join("nhlng.db")).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(a.join("nhlng-meta.xml")).unwrap()).unwrap();
    for p in [Platform::Xbox, Platform::Ps3] {
        let cx = catalog_of(&bytes, &meta, Some("nhl15"), Some(p), None);
        let c = cx.competition.as_ref().unwrap();
        assert!(c.witness.contains("default.xex"), "{p:?}: {}", c.witness);
        assert_eq!(c.league_group(0).unwrap().id, 0);
        assert_eq!(c.evidence, if p == Platform::Xbox { "traced" } else { "by-reference" });
        assert!(matches!(c.provenance, Some(Provenance::Recorded { .. })));
    }
    // The PS4 table is refused for an Xbox platform claim on a PS4 database (platform bound).
    let cat = ps4_catalog("nhl15", "nhl15", Some(Platform::Xbox));
    assert!(cat.competition.is_none());
    assert!(cat.competition_blocker.as_deref().unwrap().contains("no competition group table is baked for nhl15 on xbox"), "{:?}", cat.competition_blocker);
}

// ---------------------------------------------------------------------------------------------
// 2. Unknown and inferred mappings stay honest.
// ---------------------------------------------------------------------------------------------

#[test]
fn nhl17_block_order_is_inferred_and_nhl18_spengler_is_inferred() {
    let c17 = ps4_catalog("nhl17", "nhl17", Some(Platform::Ps4));
    let c = c17.competition.as_ref().unwrap();
    for id in 1..=42 {
        assert_eq!(c.group(id).unwrap().evidence, EVIDENCE_TRACED_STATIC, "nhl17 id {id}");
    }
    for id in 43..=60 {
        let g = c.group(id).unwrap();
        assert_eq!(g.evidence, EVIDENCE_INFERRED, "nhl17 id {id}");
        assert!(g.note.as_deref().unwrap().contains("executable array stores the ECHL block earlier"), "{g:?}");
    }
    let echl = league_by_key(&c17, "TXT_ECHL");
    assert_eq!(c.league_group(echl).unwrap().id, 60);
    assert_eq!(c.regular_pairs(echl), [(54, 56), (54, 57), (55, 58), (55, 59)], "ECHL West: Central/Mountain, East: North/South");
    // The custom league whose three rows carry the ECHL Western Conference in both slots: an
    // exceptional stock arrangement of block 53, recorded with its rows, never offered.
    assert_eq!(c.group(54).unwrap().league_codes, vec![echl]);
    assert!(matches!(c.shape(17), Some(LeagueShape::Single { league_group: 53 })));
    let exc = c.exceptional_pairs(17);
    assert_eq!(exc.len(), 1);
    assert_eq!((exc[0].conference, exc[0].division, exc[0].rows, exc[0].teams.len()), (54, 54, 3, 3));
    assert!(c.validate_membership(17, 54, 54).unwrap_err().contains("exceptional stock arrangement"));
    // World Cup (league 14, eight teams): the tournament head 43 with pools 44/44 and 45/45; the
    // block's Conference A/B and Division A–D exist in the executable but no row carries them,
    // so they are not offered; the 21-team International league's own-id arrangement is.
    assert_eq!(c.regular_pairs(14), [(43, 43), (44, 44), (45, 45)]);
    assert!(c.validate_membership(14, 44, 44).is_ok());
    assert!(c.validate_membership(14, 44, 45).unwrap_err().contains("not paired"), "cross-pool triple refused");
    assert!(c.validate_membership(14, 47, 49).unwrap_err().contains("not a conference"), "unused Conference A / Division A never offered");
    assert!(c.validate_membership(8, 43, 43).is_ok());

    let c18 = ps4_catalog("nhl18", "nhl18", Some(Platform::Ps4));
    let c = c18.competition.as_ref().unwrap();
    for id in [74, 75, 76] {
        let g = c.group(id).unwrap();
        assert_eq!(g.evidence, EVIDENCE_INFERRED, "nhl18 id {id}");
        assert!(g.note.as_deref().unwrap().contains("not in the NHL 18 array"), "{g:?}");
    }
    assert_eq!(c.group(74).unwrap().label, "Spengler Cup");
    assert_eq!(c.group(75).unwrap().label_source, "key", "NHL 18's dictionary has no text for the key");
    // NHL 18's six Spengler rows all carry Cattini (75/75); Torriani is listed in the executable
    // but no row carries it — not offered, disclosed as unresolved rather than invented.
    assert_eq!(c.regular_pairs(20), [(75, 75)]);
    assert!(c.validate_membership(20, 76, 76).unwrap_err().contains("not a conference"));
    assert_eq!(c.conference_choices(20).iter().map(|g| g.id).collect::<Vec<_>>(), [75]);
    // Champions Hockey League: head 61, Group A in both slots; Group B–H unused.
    assert_eq!(c.regular_pairs(9), [(64, 64)]);
    assert!(c.group(72).unwrap().league_codes.is_empty() && c.group(73).unwrap().league_codes.is_empty());
    assert_eq!(c.group(24).unwrap().key, "TXT_LEAGUE_NORWAY");
    assert!(c.group(24).unwrap().league_codes.is_empty() && !c.group(24).unwrap().used);
}

#[test]
fn nhl19_and_21_ids_77_and_78_are_labelled_from_their_leagues_with_the_hypothesis_disclosed() {
    for game in ["nhl19", "nhl21"] {
        let cat = ps4_catalog(game, game, Some(Platform::Ps4));
        let c = cat.competition.as_ref().unwrap();
        assert_eq!(c.unknown().iter().map(|g| g.id).collect::<Vec<_>>(), [77, 78], "{game}");
        let g77 = c.group(77).unwrap();
        // Research 2026-09-21 handoff 08 §3: one flat pseudo-group shared by the Alumni and
        // All-Time leagues — labelled by rule from the leagues that carry it (their key when no
        // dictionary is attached; NHL 19 keys both rows ALUMNI, so one name), never "unknown".
        let expect77 = if game == "nhl21" { "TXT_LEAGUE_NHL_ALUMNI / TXT_LEAGUE_NHL_ALL_TIME (single group)" } else { "TXT_LEAGUE_NHL_ALUMNI (single group)" };
        assert_eq!(g77.label, expect77, "{game}");
        assert_eq!(g77.label_source, "derived-from-leagues");
        assert_eq!(c.group(78).unwrap().label, "TXT_LEAGUE_PROAM (single group)");
        assert_eq!(c.group(78).unwrap().evidence, EVIDENCE_UNKNOWN, "the evidence class is unchanged by the rule");
        assert_eq!(g77.hypothesis_key.as_deref(), Some("TXT_LEAGUE_NHL_ALUMNI"));
        let note = g77.note.as_deref().unwrap();
        assert!(note.contains("hypothesis only"), "{game}");
        // The note carries the research wording AND the keys this witness database actually
        // uses for the sharing rows (NHL 19: 22 is keyed ALUMNI too; NHL 21: ALL_TIME).
        let expect22 = if game == "nhl21" { "22 = TXT_LEAGUE_NHL_ALL_TIME" } else { "22 = TXT_LEAGUE_NHL_ALUMNI" };
        assert!(note.contains(expect22), "{game}: {note}");
        assert_eq!(g77.league_codes, vec![21, 22], "{game}: one block shared by two stockleagues rows");
        assert_eq!(c.group(78).unwrap().league_codes, vec![23]);
        assert_eq!(cat.group_label("iIBl", 21, 77).as_deref(), Some(expect77));
        // Spengler Cup rows carry both pools on 19–21 (traced), one pool per row.
        let s = cat.structure(20).unwrap();
        assert_eq!(s.league_group.label, "Spengler Cup");
        assert_eq!(s.pools.iter().map(|g| g.label.as_str()).collect::<Vec<_>>(), ["Group Cattini", "Group Torriani"]);
        assert_eq!(s.pairs, [(75, 75), (76, 76)]);
        assert!(c.validate_membership(20, 75, 76).is_err(), "{game}: cross-pool triple refused");
        // The Alumni structure discloses its block sharer and the unknown label.
        let s = cat.structure(21).unwrap();
        assert_eq!(s.league_group.evidence, EVIDENCE_UNKNOWN);
        assert_eq!(s.shared_with.len(), 1);
        assert_eq!(s.shared_with[0].0, 22);
        assert!(s.provenance.as_deref().unwrap().contains("recorded identity"), "{game}: {:?}", s.provenance);
        // Three custom rows plus the prospect league share block 53.
        let s = cat.structure(14).unwrap();
        assert_eq!(s.league_group.id, 53);
        assert_eq!(s.shared_with.len(), 3, "{game}: {:?}", s.shared_with);
    }
}

// ---------------------------------------------------------------------------------------------
// 3. Provenance (R1 F2): recorded identity, structural identification, cross-title claims.
// ---------------------------------------------------------------------------------------------

#[test]
fn cross_title_claims_are_refused_by_structure_not_by_row_agreement() {
    // The reproduced cases: the NHL 21 anchor opened under every other PS4 title's game id. Row
    // agreement was high for 16/17/18 and total for 19 (shared structure); the structural
    // signature (table set, capacities) identifies it as NHL 21 alone.
    for claimed in ["nhl15", "nhl16", "nhl17", "nhl18", "nhl19", "nhl20"] {
        let cat = ps4_catalog("nhl21", claimed, Some(Platform::Ps4));
        assert!(cat.competition.is_none(), "{claimed}: no table");
        let b = cat.competition_blocker.as_deref().unwrap();
        assert!(b.contains(&format!("opened as {claimed}")) && b.contains("it identifies as nhl21"), "{claimed}: {b}");
        assert!(cat.group_label("OwTu", 0, 6).is_none(), "{claimed}: no labels from a refused table");
    }
    // The other way round: every anchor opened as NHL 21 is refused and names itself.
    for game in ["nhl15", "nhl16", "nhl17", "nhl18", "nhl19"] {
        let cat = ps4_catalog(game, "nhl21", Some(Platform::Ps4));
        let b = cat.competition_blocker.as_deref().unwrap_or_else(|| panic!("{game}: accepted as nhl21"));
        assert!(b.contains(&format!("it identifies as {game}")), "{game}: {b}");
    }
    // anchors/ps4/nhl20 is an NHL 19 base copy: opened as nhl20 it is refused and identified as
    // NHL 19; opened as nhl19 it is NHL 19's recorded base identity.
    let cat = ps4_catalog("nhl20", "nhl20", Some(Platform::Ps4));
    let b = cat.competition_blocker.as_deref().unwrap();
    assert!(b.contains("not nhl20") && b.contains("NHL 19 base copy"), "{b}");
    let cat = ps4_catalog("nhl20", "nhl19", Some(Platform::Ps4));
    assert!(matches!(cat.competition.as_ref().unwrap_or_else(|| panic!("{:?}", cat.competition_blocker)).provenance, Some(Provenance::Recorded { .. })));
    // No game id: nothing can be selected, and the reason says so.
    let dir = repo("anchors/ps4/nhl21/databases");
    let bytes = std::fs::read(dir.join("nhl_game.db")).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(dir.join("nhl_game-meta.xml")).unwrap()).unwrap();
    let cat = catalog_of(&bytes, &meta, None, Some(Platform::Ps4), None);
    assert!(cat.competition.is_none());
    assert!(cat.competition_blocker.as_deref().unwrap().contains("no game id"), "{:?}", cat.competition_blocker);
    // A gen7 title without group fields (NHL 11) records no blocker: nothing to label.
    let a = repo("anchors/xbox360/nhl11/databases");
    let bytes11 = std::fs::read(a.join("nhlng.db")).unwrap();
    let meta11 = FieldMetaIndex::from_xml(&std::fs::read_to_string(a.join("nhlng-meta.xml")).unwrap()).unwrap();
    let c11 = catalog_of(&bytes11, &meta11, Some("nhl11"), Some(Platform::Xbox), None);
    assert!(c11.competition.is_none() && c11.competition_blocker.is_none());
    // A gen7 wrong-title claim is refused by structure too (NHL 12 opened as NHL 15).
    let a = repo("anchors/xbox360/nhl12/databases");
    let bytes12 = std::fs::read(a.join("nhlng.db")).unwrap();
    let meta12 = FieldMetaIndex::from_xml(&std::fs::read_to_string(a.join("nhlng-meta.xml")).unwrap()).unwrap();
    let c = catalog_of(&bytes12, &meta12, Some("nhl15"), Some(Platform::Xbox), None);
    assert!(c.competition_blocker.as_deref().unwrap().contains("it identifies as nhl12"), "{:?}", c.competition_blocker);
}

#[test]
fn an_edited_database_keeps_its_title_by_structure_and_says_the_identity_is_not_recorded() {
    // A legitimate user edit changes the bytes: no recorded identity any more, but the structure
    // still identifies NHL 21 uniquely, so the table applies with that provenance.
    let mut ws = anchor_ws("nhl21");
    let bos = team_by_abbrev(&ws, "BOS");
    ws.set_team_membership(bos, 1, Some(13), Some(17)).unwrap();
    let cat = ws.display().league_catalog().unwrap();
    let c = cat.competition.as_ref().unwrap();
    assert_eq!(c.provenance, Some(Provenance::Structural));
    assert_eq!(c.evidence, "traced", "the executable table binding is unchanged by an edit");
    let opts = ws.team_membership_options(bos).unwrap();
    assert!(opts.notes.iter().any(|n| n.contains("file identity not recorded")), "{:?}", opts.notes);
    // The mis-claimed database keeps the raw model, disclosed on every surface.
    let mut ws = anchor_ws("nhl21");
    ws.set_game_id("nhl15");
    let opts = ws.team_membership_options(bos).unwrap();
    assert_eq!(opts.model, GroupModel::Ps4);
    assert!(opts.current.conference.as_ref().unwrap().label.contains("no group table applies"));
    assert!(opts.notes.iter().any(|n| n.contains("it identifies as nhl21")), "{:?}", opts.notes);
    let view = ws.display().league_view(0).unwrap().unwrap();
    assert!(view.structure.is_none());
    assert!(view.structure_blocker.as_deref().unwrap().contains("it identifies as nhl21"));
    let ahl = league_by_key(&ws.display().league_catalog().unwrap(), "AHL");
    let (plan, outcome) = ws.set_team_membership(bos, ahl, Some(13), Some(15)).unwrap();
    assert_eq!(outcome.edits, 3);
    assert!(plan.notes.iter().any(|n| n.contains("written raw")), "{:?}", plan.notes);
    assert!(ws.undo());
}

// ---------------------------------------------------------------------------------------------
// 4. Parent relationships (R1 F1) and membership through the shared global-groups path.
// ---------------------------------------------------------------------------------------------

#[test]
fn nhl21_membership_offers_only_witnessed_pairs_and_survives_save_reopen_restore() {
    let dir = std::env::temp_dir().join(format!("nhl-studio-p1c-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = repo("anchors/ps4/nhl21/databases");
    let db_path = dir.join("nhl_game.db");
    std::fs::copy(src.join("nhl_game.db"), &db_path).unwrap();
    let mut ws = open_ps4("nhl21", &db_path, &src.join("nhl_game-meta.xml"));
    let cat = ws.display().league_catalog().unwrap();
    let bos = team_by_abbrev(&ws, "BOS");
    let opts = ws.team_membership_options(bos).unwrap();
    assert_eq!(opts.model, GroupModel::GlobalGroups, "PS4 with a fitting table takes the shared path");
    assert_eq!(opts.group_evidence.as_deref(), Some("traced"));
    assert_eq!(opts.current.conference.as_ref().unwrap().label, "Eastern Conference");
    assert_eq!(opts.current.division.as_ref().unwrap().label, "Atlantic Division");
    assert!(opts.notes.iter().any(|n| n.contains("CUSA18182")), "{:?}", opts.notes);
    assert!(opts.notes.iter().any(|n| n.contains("title provenance: recorded identity")), "{:?}", opts.notes);
    assert!(opts.notes.iter().any(|n| n.contains("no known name") && n.contains("77")), "{:?}", opts.notes);
    let ahl = league_by_key(&cat, "AHL");
    let spengler = league_by_key(&cat, "TXT_SPENGLER_CUP");
    let champions = league_by_key(&cat, "TXT_CHAMPIONS_LEAGUE");
    let qmjhl = league_by_key(&cat, "QMJHL");
    let alumni = 21;
    let chl = league_by_key(&cat, "CHL");
    let g = |code: u64| opts.groups_by_league.iter().find(|g| g.league_code == code);
    // NHL: the pairs the card filters by; AHL likewise.
    let nhl = g(0).unwrap();
    assert_eq!(nhl.pairs, [(6, 8), (6, 9), (7, 10), (7, 11)]);
    assert_eq!(nhl.conferences.iter().map(|c| c.value).collect::<Vec<_>>(), [6, 7]);
    assert_eq!(nhl.divisions.iter().map(|c| c.value).collect::<Vec<_>>(), [8, 9, 10, 11]);
    assert!(nhl.exceptional.is_empty(), "NHL 21's All-Star rows carry regular divisions");
    // AHL on NHL 21: West with Central/Pacific, East with Atlantic/North (the geography the rows
    // carry — not the id order).
    assert_eq!(g(ahl).unwrap().pairs, [(13, 17), (13, 18), (14, 15), (14, 16)]);
    // QMJHL: no conferences — the own id in the conference slot with one of three divisions.
    let q = g(qmjhl).unwrap();
    assert_eq!(q.pairs, [(32, 33), (32, 34), (32, 35)]);
    assert!(q.conferences[0].label.contains("no conference: the league's own id"));
    // Spengler: one pool in both slots; Champions: Group A in both; Alumni: single, unknown id.
    assert_eq!(g(spengler).unwrap().pairs, [(75, 75), (76, 76)]);
    assert_eq!(g(champions).unwrap().pairs, [(64, 64)]);
    let al = g(alumni).unwrap();
    assert_eq!((al.league_group, al.league_group_evidence.as_str()), (77, EVIDENCE_UNKNOWN));
    assert!(al.conferences.is_empty() && al.pairs.is_empty(), "single group");
    assert!(g(chl).is_none(), "CHL has no teams and no established block");

    // Refusals leave bytes, history and revision untouched — including the reproduced case:
    // West + Atlantic are both NHL ids but not a witnessed pair.
    let bytes = ws.primary().bytes().to_vec();
    let rev = ws.revision();
    let err = ws.set_team_membership(bos, 0, Some(6), Some(10)).unwrap_err().to_string();
    assert!(err.contains("not paired with Western Conference") && err.contains("Pacific Division") && err.contains("Central Division"), "{err}");
    let err = ws.set_team_membership(bos, ahl, Some(13), Some(8)).unwrap_err().to_string();
    assert!(err.contains("not paired"), "{err}");
    let err = ws.set_team_membership(bos, ahl, Some(13), Some(15)).unwrap_err().to_string();
    assert!(err.contains("Atlantic Division (15) is not paired with Western Conference under AHL"), "{err}: the P1-C receipt's own example was an invalid pair");
    let err = ws.set_team_membership(bos, ahl, Some(6), Some(15)).unwrap_err().to_string();
    assert!(err.contains("not a conference AHL's teams carry"), "{err}");
    let err = ws.set_team_membership(bos, ahl, None, None).unwrap_err().to_string();
    assert!(err.contains("choose one"), "{err}");
    let err = ws.set_team_membership(bos, spengler, Some(75), Some(76)).unwrap_err().to_string();
    assert!(err.contains("not paired"), "{err}: cross-pool");
    let err = ws.set_team_membership(bos, spengler, Some(75), Some(8)).unwrap_err().to_string();
    assert!(err.contains("not paired"), "{err}");
    let err = ws.set_team_membership(bos, qmjhl, Some(32), Some(32)).unwrap_err().to_string();
    assert!(err.contains("not paired"), "{err}: QMJHL never carries its own id in the division slot");
    let err = ws.set_team_membership(bos, alumni, Some(6), None).unwrap_err().to_string();
    assert!(err.contains("single-group"), "{err}");
    let err = ws.set_team_membership(bos, chl, None, None).unwrap_err().to_string();
    assert!(err.contains("no group in this title's competition table"), "{err}");
    let err = ws.set_team_membership(bos, 99, None, None).unwrap_err().to_string();
    assert!(err.contains("not a league on this title"), "{err}");
    unchanged(&ws, &bytes, 0, &rev, "refusals");

    // Accepted: AHL East / Atlantic (four cells, one undo step, labels resolved).
    let (plan, outcome) = ws.set_team_membership(bos, ahl, Some(14), Some(15)).unwrap();
    assert_eq!(outcome.edits, 4);
    assert_eq!(plan.after.conference.as_ref().unwrap().label, "Eastern Conference");
    assert_eq!(plan.after.division.as_ref().unwrap().label, "Atlantic Division");
    assert_eq!(plan.after.league_group.as_ref().unwrap().value, 12);
    let view = ws.display().team_view(bos).unwrap().unwrap();
    assert_eq!(view.league_label.as_deref(), Some("AHL"));
    assert_eq!(view.conference_label.as_deref(), Some("Eastern Conference"));
    assert_eq!(view.division_label.as_deref(), Some("Atlantic Division"));
    // Every witnessed NHL pair is accepted; QMJHL division-only; Spengler pool; Champions;
    // Alumni's unknown 77 in all three; Custom Teams 53.
    for (c, d) in [(6, 8), (6, 9), (7, 10), (7, 11)] {
        ws.set_team_membership(bos, 0, Some(c), Some(d)).unwrap();
    }
    let (plan, _) = ws.set_team_membership(bos, qmjhl, Some(32), Some(34)).unwrap();
    assert_eq!(plan.after.division.as_ref().unwrap().label, "Maritimes Division");
    let (plan, _) = ws.set_team_membership(bos, spengler, Some(76), Some(76)).unwrap();
    assert_eq!(plan.after.conference.as_ref().unwrap().label, "Group Torriani");
    let (plan, _) = ws.set_team_membership(bos, champions, Some(64), Some(64)).unwrap();
    assert_eq!(plan.after.division.as_ref().unwrap().label, "Group A");
    let (plan, _) = ws.set_team_membership(bos, alumni, None, None).unwrap();
    assert_eq!(plan.writes.iter().filter(|(_, v)| *v == 77).count(), 3);
    assert!(plan.notes.iter().any(|n| n.contains("group 77") && n.contains("unknown")), "{:?}", plan.notes);
    let (plan, _) = ws.set_team_membership(bos, 14, None, None).unwrap();
    assert_eq!(plan.writes.iter().filter(|(_, v)| *v == 53).count(), 3);
    assert_eq!(ws.undo_depth(), 10);
    let (pending, _) = ws.pending_changes(100);
    assert_eq!(pending.total_cells, 4, "league + three group cells differ from the file");
    // Undo / redo walk the history; the labels follow.
    assert!(ws.undo() && ws.undo());
    assert_eq!(ws.display().team_view(bos).unwrap().unwrap().division_label.as_deref(), Some("Group A"));
    assert!(ws.redo());
    assert_eq!(ws.display().team_view(bos).unwrap().unwrap().conference_label.as_deref(), Some(G77));
    // Save to the copy, reopen: the moved team reads back through the same table (structural
    // provenance now: the bytes are no recorded identity).
    ws.save_primary_to(db_path.to_str().unwrap()).unwrap();
    let reopened = open_ps4("nhl21", &db_path, &src.join("nhl_game-meta.xml"));
    let view = reopened.display().team_view(bos).unwrap().unwrap();
    assert_eq!(view.league_label.as_deref(), Some("TXT_LEAGUE_NHL_ALUMNI"));
    assert_eq!(view.conference_label.as_deref(), Some(G77));
    assert_eq!(reopened.display().league_catalog().unwrap().competition.unwrap().provenance, Some(Provenance::Structural));
    assert_eq!(std::fs::read(&db_path).unwrap().len(), std::fs::read(src.join("nhl_game.db")).unwrap().len());
    // Restore: undo everything and save equals the anchor byte-for-byte.
    while ws.undo() {}
    assert!(!ws.is_dirty());
    ws.save_primary_to(db_path.to_str().unwrap()).unwrap();
    assert_eq!(std::fs::read(&db_path).unwrap(), std::fs::read(src.join("nhl_game.db")).unwrap());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn all_star_arrangements_are_recorded_as_exceptional_and_never_offered() {
    // NHL 15/16/17 PS4 (and every gen7 table): the two All-Star squads carry a conference id in
    // the division slot. They browse as they are; the pair is refused for a club with the row ids
    // named; the squads' own rows are untouched by that refusal.
    for game in ["nhl15", "nhl16", "nhl17"] {
        let mut ws = anchor_ws(game);
        let cat = ws.display().league_catalog().unwrap();
        let c = cat.competition.as_ref().unwrap();
        let exc = c.exceptional_pairs(0);
        assert_eq!(exc.iter().map(|p| (p.conference, p.division, p.rows)).collect::<Vec<_>>(), [(6, 6, 1), (7, 7, 1)], "{game}");
        assert_eq!(exc[0].teams, [31], "{game}: the Western All-Star squad");
        assert_eq!(exc[1].teams, [30], "{game}");
        let squad = ws.display().team_view(31).unwrap().unwrap();
        assert_eq!(squad.conference_label.as_deref(), Some("Western Conference"), "{game}: browses as it is");
        assert_eq!(squad.division_label.as_deref(), Some("Western Conference"), "{game}");
        let bos = team_by_abbrev(&ws, "BOS");
        let bytes = ws.primary().bytes().to_vec();
        let rev = ws.revision();
        let err = ws.set_team_membership(bos, 0, Some(6), Some(6)).unwrap_err().to_string();
        assert!(err.contains("exceptional stock arrangement") && err.contains("team id 31"), "{game}: {err}");
        unchanged(&ws, &bytes, 0, &rev, game);
        let opts = ws.team_membership_options(bos).unwrap();
        let nhl = opts.groups_by_league.iter().find(|g| g.league_code == 0).unwrap();
        assert_eq!(nhl.exceptional.len(), 2, "{game}: disclosed to the card");
        assert!(!nhl.pairs.contains(&(6, 6)), "{game}");
    }
}

#[test]
fn every_ps4_anchor_moves_a_team_within_the_nhl_and_into_a_single_group_league() {
    for game in PS4_ANCHORS {
        let mut ws = anchor_ws(game);
        let cat = ws.display().league_catalog().unwrap();
        let bos = team_by_abbrev(&ws, "BOS");
        let opts = ws.team_membership_options(bos).unwrap();
        assert_eq!(opts.model, GroupModel::GlobalGroups, "{game}");
        let bytes = ws.primary().bytes().to_vec();
        let rev = ws.revision();
        // An AHL division under the NHL, and the unpaired West/Atlantic, are refused on every title.
        let err = ws.set_team_membership(bos, 0, Some(6), Some(15)).unwrap_err().to_string();
        assert!(err.contains("not paired"), "{game}: {err}");
        let err = ws.set_team_membership(bos, 0, Some(6), Some(10)).unwrap_err().to_string();
        assert!(err.contains("not paired"), "{game}: {err}");
        unchanged(&ws, &bytes, 0, &rev, game);
        let (plan, outcome) = ws.set_team_membership(bos, 0, Some(6), Some(8)).unwrap();
        assert_eq!(outcome.edits, 4, "{game}: league code (unchanged value still written) + three groups");
        assert_eq!(plan.after.division.as_ref().unwrap().label, "Pacific Division", "{game}");
        let liiga = league_by_key(&cat, "SMLIIGA");
        let (plan, _) = ws.set_team_membership(bos, liiga, None, None).unwrap();
        let lg = cat.competition.as_ref().unwrap().league_group(liiga).unwrap().id;
        assert_eq!(lg, if game == "nhl15" { 23 } else { 21 }, "{game}: Liiga's own id is title-specific");
        assert!(plan.writes.iter().filter(|(_, v)| *v == lg as i64).count() == 3, "{game}");
        assert_eq!(ws.display().team_view(bos).unwrap().unwrap().conference_label.as_deref(), Some("Liiga"), "{game}");
        assert!(ws.undo() && ws.undo() && !ws.is_dirty(), "{game}");
    }
}

// ---------------------------------------------------------------------------------------------
// 4b. R2 F1: on a single-group league each supplied slot is checked on its own. A valid
//     conference never excuses a conflicting division (the reproduced case: Liiga with
//     conference 21 and division 6 was accepted and silently written as 21/21).
// ---------------------------------------------------------------------------------------------

#[test]
fn single_group_league_validates_conference_and_division_independently() {
    let mut ws = anchor_ws("nhl21");
    let cat = ws.display().league_catalog().unwrap();
    let bos = team_by_abbrev(&ws, "BOS");
    let liiga = league_by_key(&cat, "SMLIIGA");
    let own = cat.competition.as_ref().unwrap().league_group(liiga).unwrap().id;
    assert_eq!(own, 21);
    assert!(matches!(cat.competition.as_ref().unwrap().shape(liiga), Some(LeagueShape::Single { league_group: 21 })));
    let bytes = ws.primary().bytes().to_vec();
    let rev = ws.revision();
    let pending_before = ws.pending_changes(100).0.total_cells;
    let refused = [
        (Some(own), Some(6), "division", "correct conference + wrong division (the reproduced case)"),
        (Some(6), Some(own), "conference", "wrong conference + correct division"),
        (None, Some(6), "division", "omitted conference + wrong division"),
        (Some(6), None, "conference", "wrong conference + omitted division"),
        (Some(6), Some(8), "conference", "both wrong: the conference is named first"),
    ];
    for (conf, div, slot, what) in refused {
        let err = ws.set_team_membership(bos, liiga, conf, div).unwrap_err().to_string();
        assert!(err.contains("single-group") && err.contains(&format!("the {slot} given (6)")), "{what}: {err}");
        unchanged(&ws, &bytes, 0, &rev, what);
        assert_eq!(ws.redo_depth(), 0, "{what}: redo unchanged");
        assert!(!ws.is_dirty(), "{what}");
        assert_eq!(ws.pending_changes(100).0.total_cells, pending_before, "{what}: no pending edits");
    }
    // Accepted: omitted, matching, and each slot alone — always the own-id arrangement.
    for (conf, div) in [(None, None), (Some(own), Some(own)), (Some(own), None), (None, Some(own))] {
        let (plan, outcome) = ws.set_team_membership(bos, liiga, conf, div).unwrap();
        assert_eq!(outcome.edits, 4, "{conf:?}/{div:?}");
        assert_eq!(plan.writes.iter().filter(|(_, v)| *v == own as i64).count(), 3, "{conf:?}/{div:?}: {:?}", plan.writes);
        assert_eq!(ws.display().team_view(bos).unwrap().unwrap().conference_label.as_deref(), Some("Liiga"));
        assert!(ws.undo() && !ws.is_dirty());
        assert_eq!(ws.redo_depth(), 1);
        ws.redo();
        ws.undo();
    }
    unchanged(&ws, &bytes, 0, &rev, "after undo of every accepted form");
    // R1 behaviour preserved alongside: witnessed pairs and the exceptional All-Star row.
    assert!(ws.set_team_membership(bos, 0, Some(6), Some(10)).is_err());
    let (plan, _) = ws.set_team_membership(bos, 0, Some(6), Some(8)).unwrap();
    assert_eq!(plan.after.division.as_ref().unwrap().label, "Pacific Division");
    assert!(ws.undo() && !ws.is_dirty());
}

// ---------------------------------------------------------------------------------------------
// 5. Private fixtures (`--ignored`): the real NHL 20 install database (the nhl20 table's
//    witness) and the NHL 19 patch DB (a recorded patch identity).
// ---------------------------------------------------------------------------------------------

fn fixture(key: &str, sha: &str) -> PathBuf {
    let root = std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the private PS4 fixtures cannot be resolved");
    let path = PathBuf::from(root).join(key);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    assert!(!bytes.is_empty(), "{key} is empty");
    assert_eq!(tdb_txn::FileIdentity::of_bytes(&bytes).sha256, sha, "{key} is not the recorded source");
    path
}

#[test]
#[ignore = "private fixtures databases/nhl20-ps4-cusa15439 and databases/nhl19-ps4-cusa11126: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn real_nhl20_and_nhl19_patch_databases_are_recorded_identities() {
    // Real NHL 20: the witnessed database and dictionary of the nhl20 table.
    let db = fixture("databases/nhl20-ps4-cusa15439/inxp_00.big/nhl_game.db", "1e2004784bccfdd6bed6e82533a67595798379f2c2d763c0636d9d370977a57b");
    let meta = fixture("databases/nhl20-ps4-cusa15439/inxp_00.big/nhl_game-meta.xml", "2fac0e733c9792038a64f966f277b163136517438509283e7f646d2920ad1f93");
    let dict = fixture("databases/nhl20-ps4-cusa15439/inxp_00.big/eng_us.db", "0fc95ec84b71aaf23c2447783b277ba2df22b4ed762f901acf220af83f7739ad");
    let mut ws = open_ps4("nhl20", &db, &meta);
    ws.attach_dictionary(dict.to_str().unwrap()).unwrap();
    let cat = ws.display().league_catalog().unwrap();
    let c = cat.competition.as_ref().unwrap_or_else(|| panic!("{:?}", cat.competition_blocker));
    assert_eq!(c.evidence, "traced");
    assert!(matches!(c.provenance, Some(Provenance::Recorded { ref role, .. }) if role == "base"), "{:?}", c.provenance);
    let fit = c.fit.as_ref().unwrap();
    assert_eq!((fit.conflict, fit.unknown), (0, 0), "{}", fit.summary());
    assert_eq!(c.group(75).unwrap().label, "Group Cattini");
    assert_eq!(c.group(75).unwrap().label_source, "dictionary");
    assert_eq!(c.regular_pairs(0), [(6, 8), (6, 9), (7, 10), (7, 11)]);
    let proam = league_by_key(&cat, "TXT_LEAGUE_PROAM");
    assert_eq!(cat.league(proam).unwrap().member_teams, 80, "NHL 20: 80 Pro Am teams (97 on NHL 21)");
    assert_eq!(c.league_group(proam).unwrap().id, 78);
    assert!(c.group(77).unwrap().note.as_deref().unwrap().contains("22 = TXT_LEAGUE_NHL_ALUMNI"), "NHL 20 keys 22 as ALUMNI");
    let bos = team_by_abbrev(&ws, "BOS");
    assert!(ws.set_team_membership(bos, 1, Some(13), Some(16)).is_err(), "West + North is not an AHL pair on NHL 20");
    let (plan, _) = ws.set_team_membership(bos, 1, Some(14), Some(16)).unwrap();
    assert_eq!(plan.after.division.as_ref().unwrap().label, "North Division");
    assert!(ws.undo() && !ws.is_dirty());
    // Opened as NHL 19 the real NHL 20 database is refused by structure (31 league rows, 114 tables).
    let ws = open_ps4("nhl19", &db, &meta);
    let cat = ws.display().league_catalog().unwrap();
    assert!(cat.competition_blocker.as_deref().unwrap().contains("it identifies as nhl20"), "{:?}", cat.competition_blocker);

    // NHL 19 patch_inxp_56 database: a recorded patch identity; same ids, Pro Am grown to 84.
    let db = fixture("databases/nhl19-ps4-cusa11126/patch_inxp_56.big/nhl_game.db", "61332e666a6b4afd4032212e9b8710658a9eddeea2637d3a4d288b1f30cab58d");
    let meta = fixture("databases/nhl19-ps4-cusa11126/inxp_00.big/nhl_game-meta.xml", "c67f4bfd3d152bfb68dd3d3c16823ef4f5b7c833cb46751f0834cb14a0cc90b3");
    let ws = open_ps4("nhl19", &db, &meta);
    let cat = ws.display().league_catalog().unwrap();
    let c = cat.competition.as_ref().unwrap_or_else(|| panic!("{:?}", cat.competition_blocker));
    assert!(matches!(c.provenance, Some(Provenance::Recorded { ref role, .. }) if role == "patch"), "{:?}", c.provenance);
    assert_eq!(c.evidence, "traced");
    let fit = c.fit.as_ref().unwrap();
    assert_eq!((fit.conflict, fit.unknown), (0, 0), "{}", fit.summary());
    let proam = league_by_key(&cat, "TXT_LEAGUE_PROAM");
    assert_eq!(cat.league(proam).unwrap().member_teams, 84);
    assert!(c.witness_notes.iter().any(|n| n.contains("patch_inxp_56")), "{:?}", c.witness_notes);
    // Base NHL 19 for comparison: 80, the base identity.
    let db = fixture("databases/nhl19-ps4-cusa11126/inxp_00.big/nhl_game.db", "265e4a68d23ce44cef3240017beda0583c2cb6d246ea6672ca5612472429f408");
    let ws = open_ps4("nhl19", &db, &meta);
    let cat = ws.display().league_catalog().unwrap();
    assert_eq!(cat.league(proam).unwrap().member_teams, 80);
    assert!(matches!(cat.competition.as_ref().unwrap().provenance, Some(Provenance::Recorded { ref role, .. }) if role == "base"));
}

/// C7 (overnight 2026-09-20): the NHL 15 **patch** database (`patch_inxp00.big`, the copy the
/// console is believed to read first; research 22 / receipt 02 held the fixture, no test ran it):
/// its identity is not recorded on the `nhl15` table, so the table binds by **structural
/// signature** (with the ambiguity check against every other baked title), the witnessed groups
/// fit with no conflict, and the same membership moves as on the base anchor work and undo.
#[test]
#[ignore = "private fixture databases/nhl15-ps4-cusa00561/patch_inxp00.big/nhl_game.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn real_nhl15_patch_database_binds_by_structure_and_takes_membership_moves() {
    let db = fixture("databases/nhl15-ps4-cusa00561/patch_inxp00.big/nhl_game.db", "abe36f58fd0a65ab074aff13f83f2e671b707afc539ee57c01df128200c320ff");
    let meta = repo("anchors/ps4/nhl15/databases/nhl_game-meta.xml");
    let mut ws = open_ps4("nhl15", &db, &meta);
    let cat = ws.display().league_catalog().unwrap();
    let c = cat.competition.as_ref().unwrap_or_else(|| panic!("{:?}", cat.competition_blocker));
    assert!(matches!(c.provenance, Some(Provenance::Structural)), "{:?}", c.provenance);
    let fit = c.fit.as_ref().unwrap();
    assert_eq!((fit.conflict, fit.unknown), (0, 0), "{}", fit.summary());
    assert!(c.witness_notes.iter().any(|n| n.contains("patch_inxp00")), "{:?}", c.witness_notes);
    let bos = team_by_abbrev(&ws, "BOS");
    let bytes = ws.primary().bytes().to_vec();
    let rev = ws.revision();
    let err = ws.set_team_membership(bos, 0, Some(6), Some(10)).unwrap_err().to_string();
    assert!(err.contains("not paired"), "{err}");
    unchanged(&ws, &bytes, 0, &rev, "nhl15 patch");
    let (plan, outcome) = ws.set_team_membership(bos, 0, Some(6), Some(8)).unwrap();
    assert_eq!(outcome.edits, 4);
    assert_eq!(plan.after.division.as_ref().unwrap().label, "Pacific Division");
    let liiga = league_by_key(&cat, "SMLIIGA");
    let (_, _) = ws.set_team_membership(bos, liiga, None, None).unwrap();
    assert_eq!(c.league_group(liiga).unwrap().id, 23, "NHL 15: Liiga's own id");
    assert!(ws.undo() && ws.undo() && !ws.is_dirty());
    // The base anchor keeps its recorded identity for comparison.
    let base = anchor_ws("nhl15");
    let bc = base.display().league_catalog().unwrap();
    assert!(matches!(bc.competition.as_ref().unwrap().provenance, Some(Provenance::Recorded { ref role, .. }) if role == "base"));
}
