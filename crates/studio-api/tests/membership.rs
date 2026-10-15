//! Phase 1.2 — league membership through the Registry seam: options resolve per title, a move
//! is one transaction reflected in the team view, the league view carries the structure, and the
//! pending review shows the moved cells with names.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn s(p: &Path) -> String {
    p.to_str().unwrap().to_string()
}

#[test]
fn legacy_membership_move_through_the_registry() {
    let reg = Registry::default();
    let id = reg
        .open_workspace_from_paths(
            Some("nhllegacy"),
            parse_platform("xbox360").unwrap(),
            &s(&repo("anchors/xbox360/nhllegacy/databases/nhlng.db")),
            None,
            &s(&repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml")),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    reg.attach_dictionary(id, &s(&repo("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db"))).unwrap();
    let teams = reg.list_teams(id).unwrap();
    let bos = teams.iter().find(|t| t.abbrev == "BOS").unwrap().team_id;

    let opts = reg.team_membership_options(id, bos).unwrap();
    assert_eq!(opts.model, "global-groups");
    assert_eq!(opts.group_evidence.as_deref(), Some("traced"));
    assert_eq!(opts.current.division.as_ref().unwrap().label, "Atlantic Division");
    let nhl = opts.groups_by_league.iter().find(|g| g.league_code == 0).unwrap();
    let west = nhl.conferences.iter().find(|c| c.label == "Western Conference").unwrap().value;
    let pacific = nhl.divisions.iter().find(|d| d.label == "Pacific Division").unwrap().value;

    let league = reg.league_view(id, 0).unwrap().unwrap();
    let st = league.structure.as_ref().expect("structure on Legacy");
    assert_eq!(st.evidence, "traced");
    assert_eq!(st.conferences.len(), 2);
    assert_eq!(st.divisions.iter().map(|d| d.label.as_str()).collect::<Vec<_>>(), ["Pacific Division", "Central Division", "Atlantic Division", "Metropolitan Division"]);
    assert!(reg.league_view(id, 7).unwrap().unwrap().structure.as_ref().unwrap().conferences.is_empty());

    let err = match reg.set_team_membership(id, bos, 0, Some(west), Some(10)) {
        Ok(_) => panic!("AHL division under the NHL must be refused"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("not paired with Western Conference"), "{err}");
    // R1 F1 through the seam: Western + Atlantic (both NHL ids) refused; the DTO carries the
    // witnessed pairs the card filters by and the two All-Star exceptions.
    let atlantic = nhl.divisions.iter().find(|d| d.label == "Atlantic Division").unwrap().value;
    assert!(reg.set_team_membership(id, bos, 0, Some(west), Some(atlantic)).is_err());
    assert_eq!(nhl.pairs, [(1, 3), (1, 4), (2, 5), (2, 6)]);
    assert_eq!(nhl.exceptional.len(), 2, "{:?}", nhl.exceptional);
    assert!(st.exceptional[0].contains("team id 31"), "{:?}", st.exceptional);
    assert_eq!(st.pairs, nhl.pairs);
    assert!(st.provenance.as_deref().unwrap().contains("recorded identity"), "{:?}", st.provenance);
    let r = reg.set_team_membership(id, bos, 0, Some(west), Some(pacific)).unwrap();
    assert_eq!(r.writes.len(), 4);
    assert_eq!(r.after.conference.as_ref().unwrap().label, "Western Conference");
    let view = reg.team_view(id, bos).unwrap().unwrap();
    assert_eq!(view.division_label.as_deref(), Some("Pacific Division"));
    let summary = reg.summary(id).unwrap();
    assert!(summary.dirty);
    assert_eq!(summary.undo_depth, 1);
    let pending = reg.pending_changes(id, 20).unwrap();
    assert_eq!(pending.primary.total_cells, 2, "conference + division changed; league and league group unchanged");
    assert!(pending.primary.cells.iter().all(|c| c.field_name == "conferencegroup" || c.field_name == "divisiongroup"), "{:?}", pending.primary.cells.iter().map(|c| c.field_name.clone()).collect::<Vec<_>>());
    assert!(reg.undo(id).unwrap());
    assert!(!reg.summary(id).unwrap().dirty);
}

/// Phase 1.3 tie-in: a rename preview names the Studio-mapped consumers of the key — the NHL key
/// labels two league codes (0 and the Winter Classic copies, 14) and one league group; a shared
/// conference key names four groups across leagues.
#[test]
fn rename_preview_discloses_key_consumers() {
    let reg = Registry::default();
    let id = reg
        .open_workspace_from_paths(
            Some("nhllegacy"),
            parse_platform("xbox360").unwrap(),
            &s(&repo("anchors/xbox360/nhllegacy/databases/nhlng.db")),
            None,
            &s(&repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml")),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    reg.attach_dictionary(id, &s(&repo("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db"))).unwrap();
    let p = reg.dictionary_rename_preview(id, "WesternConference", "EasternConference").unwrap();
    let w = p.warnings.iter().find(|w| w.contains("competition group")).expect("group consumers named");
    assert!(w.contains("4 competition groups"), "{w}");
    assert!(w.contains("AHL") && w.contains("WHL"), "{w}");
    let p = reg.dictionary_rename_preview(id, "NHL", "AHL").unwrap();
    assert!(p.warnings.iter().any(|w| w.contains("labels 2 leagues")), "{:?}", p.warnings);
    assert!(p.warnings.iter().any(|w| w.contains("1 competition group ")), "{:?}", p.warnings);
    let p = reg.dictionary_rename_preview(id, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE").unwrap();
    assert!(p.warnings.iter().any(|w| w.contains("league Norway of Norway")), "{:?}", p.warnings);
}

/// Phase 2.3: a team's dictionary key families resolve on the Legacy anchor by `artabbr`
/// (P2.3-X): Toronto's name/city/abbreviation/two-line/nickname/ticker keys, then an in-place
/// free-text edit of the nickname key holds as the pending dictionary change.
#[test]
fn team_text_resolves_by_artabbr_and_edits_through_the_dictionary_route() {
    let reg = Registry::default();
    let id = reg
        .open_workspace_from_paths(
            Some("nhllegacy"),
            parse_platform("xbox360").unwrap(),
            &s(&repo("anchors/xbox360/nhllegacy/databases/nhlng.db")),
            None,
            &s(&repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml")),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    let teams = reg.list_teams(id).unwrap();
    let tor = teams.iter().find(|t| t.abbrev == "TOR").unwrap().team_id;
    // No dictionary yet: keys listed by template, all absent.
    let t = reg.team_text(id, tor).unwrap().unwrap();
    assert_eq!(t.artabbr.as_deref(), Some("TOR"));
    assert!(t.keys.iter().all(|k| k.lookup == "absent"));
    assert!(t.db_fullname.as_deref().unwrap_or("").contains("Toronto"));

    reg.attach_dictionary(id, &s(&repo("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db"))).unwrap();
    let t = reg.team_text(id, tor).unwrap().unwrap();
    let by = |f: &str| t.keys.iter().find(|k| k.family == f).unwrap();
    assert_eq!(by("full name").text.as_deref(), Some("Toronto Maple Leafs\u{ae}"));
    assert_eq!(by("city").text.as_deref(), Some("Toronto"));
    assert_eq!(by("abbreviation").text.as_deref(), Some("TOR"));
    assert_eq!(by("line 2").text.as_deref(), Some("Maple Leafs\u{ae}"));
    assert_eq!(by("nickname").text.as_deref(), Some("Maple Leafs"));
    assert_eq!(by("nickname (short)").text.as_deref(), Some("Leafs"));
    assert_eq!(by("ticker").text.as_deref(), Some("TOR"));
    assert!(t.keys.iter().filter(|k| k.lookup != "absent").count() >= 8, "{:?}", t.keys.iter().map(|k| (&k.family, &k.lookup)).collect::<Vec<_>>());

    // Free text in place on the short nickname (fits: "Leafs" -> "Buds").
    let p = reg.dictionary_retext_set(id, "TXT_NICKNAME_ALT_TOR", "Buds").unwrap();
    assert_eq!(p.text_after, "Buds");
    let t = reg.team_text(id, tor).unwrap().unwrap();
    assert_eq!(by("nickname (short)").text.as_deref(), Some("Leafs"), "the query above was taken before the edit");
    assert_eq!(t.keys.iter().find(|k| k.family == "nickname (short)").unwrap().text.as_deref(), Some("Buds"));
    let pending = reg.pending_changes(id, 10).unwrap();
    assert_eq!(pending.dictionary.as_ref().unwrap().kind, "retext");
    reg.dictionary_rename_clear(id).unwrap();

    // An EASHL placeholder has an artabbr with no keys: listed as absent, not an error.
    if let Some(e) = teams.iter().find(|t| t.abbrev.starts_with("EASHL")) {
        let te = reg.team_text(id, e.team_id).unwrap().unwrap();
        assert!(te.keys.iter().all(|k| k.lookup == "absent"));
    }
}

/// P1-C: the PS4 NHL 18 title through the Registry seam — the DTOs carry per-row evidence,
/// the league view lists pools and block sharers, a rename preview names the PS4 group consumers
/// of a key, a validated move shows in the pending review, and a save to a copy reopens with the
/// same labels. A wrong-title claim on the same database is refused by the rows and the league
/// view says why.
#[test]
fn ps4_membership_and_structure_through_the_registry() {
    let reg = Registry::default();
    let dir = std::env::temp_dir().join(format!("nhl-studio-p1c-api-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("nhl_game.db");
    let id = reg
        .open_workspace_from_paths(
            Some("nhl18"),
            parse_platform("ps4").unwrap(),
            &s(&repo("anchors/ps4/nhl18/databases/nhl_game.db")),
            None,
            &s(&repo("anchors/ps4/nhl18/databases/nhl_game-meta.xml")),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    reg.attach_dictionary(id, &s(&repo("anchors/ps4/nhl18/databases/eng_us.db"))).unwrap();
    let teams = reg.list_teams(id).unwrap();
    let bos = teams.iter().find(|t| t.abbrev == "BOS").unwrap().team_id;

    let opts = reg.team_membership_options(id, bos).unwrap();
    assert_eq!(opts.model, "global-groups");
    assert_eq!(opts.group_evidence.as_deref(), Some("traced"));
    assert_eq!(opts.current.conference.as_ref().unwrap().evidence.as_deref(), Some("traced-static"));
    assert_eq!(opts.current.division.as_ref().unwrap().label, "Atlantic Division");
    let spengler = opts.groups_by_league.iter().find(|g| g.league_code == 20).expect("Spengler Cup");
    assert_eq!((spengler.league_group, spengler.league_group_evidence.as_str()), (74, "inferred"));
    // NHL 18's Spengler rows all carry Cattini in both slots: the one witnessed pair (inferred);
    // Torriani is in the executable but no row carries it, so it is not offered.
    assert_eq!(spengler.conferences.iter().map(|c| c.evidence.as_deref().unwrap()).collect::<Vec<_>>(), ["inferred"]);
    assert_eq!(spengler.pairs, [(75, 75)]);
    assert!(opts.notes.iter().any(|n| n.contains("CUSA07569")), "{:?}", opts.notes);
    assert!(opts.notes.iter().any(|n| n.contains("inferred from database usage")), "{:?}", opts.notes);

    let league = reg.league_view(id, 20).unwrap().unwrap();
    let st = league.structure.as_ref().expect("structure on PS4");
    assert_eq!(st.evidence, "traced");
    assert_eq!(st.league_group.label, "Spengler Cup");
    assert_eq!(st.league_group.evidence, "inferred");
    assert!(st.league_group.note.as_deref().unwrap().contains("not in the NHL 18 array"));
    assert_eq!(st.pools.iter().map(|p| p.label.as_str()).collect::<Vec<_>>(), ["TXT_GROUP_CATTINI", "TXT_GROUP_TORRIANI"]);
    assert_eq!(st.pools[0].label_source, "key", "NHL 18's dictionary has no text for the key");
    assert!(st.conferences.is_empty() && st.divisions.is_empty());
    assert_eq!(st.pairs, [(75, 75)]);
    assert!(league.structure_blocker.is_none());
    // Prospect Teams (14) shares block 53 with the EASHL / Mascot / 3-on-3 rows; the Custom Teams
    // row (16) has no team on this anchor, so no block is established for it.
    let league = reg.league_view(id, 14).unwrap().unwrap();
    let st = league.structure.as_ref().unwrap();
    assert_eq!(st.league_group.id, 53);
    assert_eq!(st.shared_with.len(), 3, "{:?}", st.shared_with);
    assert_eq!(st.league_group.league_codes, vec![14, 17, 18, 19]);
    let league = reg.league_view(id, 16).unwrap().unwrap();
    assert!(league.structure.is_none() && league.structure_blocker.is_none(), "no rows: nothing to show, no blocker");
    // The NHL structure (dictionary labels) and the rename disclosure through the PS4 table.
    let league = reg.league_view(id, 0).unwrap().unwrap();
    let st = league.structure.as_ref().unwrap();
    assert_eq!(st.divisions.iter().map(|d| d.label.as_str()).collect::<Vec<_>>(), ["Pacific Division", "Central Division", "Atlantic Division", "Metropolitan Division"]);
    assert_eq!(st.divisions[0].label_source, "dictionary");
    let p = reg.dictionary_rename_preview(id, "WesternConference", "EasternConference").unwrap();
    let w = p.warnings.iter().find(|w| w.contains("competition group")).expect("group consumers named");
    assert!(w.contains("5 competition groups"), "{w}");
    assert!(w.contains("traced"), "{w}");

    // A move into the AHL (East / North — the pair NHL 18's rows carry); refusals first: an NHL
    // division under the AHL, and West / North (both AHL ids, not a witnessed pair).
    let ahl = opts.groups_by_league.iter().find(|g| g.league_code == 1).unwrap();
    let west = ahl.conferences.iter().find(|c| c.label == "Western Conference").unwrap().value;
    let east = ahl.conferences.iter().find(|c| c.label == "Eastern Conference").unwrap().value;
    let north = ahl.divisions.iter().find(|d| d.label == "North Division").unwrap().value;
    assert!(ahl.pairs.contains(&(east, north)) && !ahl.pairs.contains(&(west, north)), "{:?}", ahl.pairs);
    let err = match reg.set_team_membership(id, bos, 1, Some(west), Some(8)) {
        Ok(_) => panic!("NHL division under the AHL must be refused"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("not paired"), "{err}");
    let err = match reg.set_team_membership(id, bos, 1, Some(west), Some(north)) {
        Ok(_) => panic!("West / North is not a witnessed AHL pair on NHL 18"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("North Division") && err.contains("not paired with Western Conference"), "{err}");
    assert!(!reg.summary(id).unwrap().dirty);
    let r = reg.set_team_membership(id, bos, 1, Some(east), Some(north)).unwrap();
    assert_eq!(r.writes.len(), 4);
    assert_eq!(r.after.division.as_ref().unwrap().label, "North Division");
    assert_eq!(r.after.division.as_ref().unwrap().evidence.as_deref(), Some("traced-static"));
    let view = reg.team_view(id, bos).unwrap().unwrap();
    assert_eq!(view.league_label.as_deref(), Some("AHL"));
    assert_eq!(view.division_label.as_deref(), Some("North Division"));
    let pending = reg.pending_changes(id, 20).unwrap();
    assert_eq!(pending.primary.total_cells, 4);
    // Save to a copy, reopen: the moved team reads back through the same table; the original
    // anchor is untouched.
    reg.save(id, out.to_str().unwrap()).unwrap();
    let id2 = reg
        .open_workspace_from_paths(
            Some("nhl18"),
            parse_platform("ps4").unwrap(),
            out.to_str().unwrap(),
            None,
            &s(&repo("anchors/ps4/nhl18/databases/nhl_game-meta.xml")),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    let view = reg.team_view(id2, bos).unwrap().unwrap();
    assert_eq!(view.division_label.as_deref(), Some("North Division"));
    assert!(reg.undo(id).unwrap());
    assert!(!reg.summary(id).unwrap().dirty);
    reg.save(id, out.to_str().unwrap()).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), std::fs::read(repo("anchors/ps4/nhl18/databases/nhl_game.db")).unwrap());
    reg.close_project(id2);
    std::fs::remove_dir_all(&dir).unwrap();

    // Wrong-title claim: the same NHL 18 database opened as NHL 15 is refused by its rows.
    let id3 = reg
        .open_workspace_from_paths(
            Some("nhl15"),
            parse_platform("ps4").unwrap(),
            &s(&repo("anchors/ps4/nhl18/databases/nhl_game.db")),
            None,
            &s(&repo("anchors/ps4/nhl18/databases/nhl_game-meta.xml")),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    let opts = reg.team_membership_options(id3, bos).unwrap();
    assert_eq!(opts.model, "ps4-foreign-key");
    let league = reg.league_view(id3, 0).unwrap().unwrap();
    assert!(league.structure.is_none());
    assert!(league.structure_blocker.as_deref().unwrap().contains("it identifies as nhl18"), "{:?}", league.structure_blocker);
}

/// R2 F1 through the seam: a single-group league validates every supplied slot on its own. The
/// current card omits both values here, but an explicit conflicting value from any client is
/// refused rather than normalised into the own-id write — on a PS4 title (NHL 18, Liiga) and on
/// the shared gen7 path (Legacy, Norway).
#[test]
fn single_group_slots_are_validated_independently_through_the_registry() {
    let reg = Registry::default();
    for (game, platform, db, meta, key) in [
        ("nhl18", "ps4", "anchors/ps4/nhl18/databases/nhl_game.db", "anchors/ps4/nhl18/databases/nhl_game-meta.xml", "SMLIIGA"),
        ("nhllegacy", "xbox360", "anchors/xbox360/nhllegacy/databases/nhlng.db", "anchors/xbox360/nhllegacy/databases/nhlng-meta.xml", "GET-ligaen (Norway)"),
    ] {
        let id = reg
            .open_workspace_from_paths(Some(game), parse_platform(platform).unwrap(), &s(&repo(db)), None, &s(&repo(meta)), &s(&repo("defs")))
            .unwrap()
            .project_id;
        let teams = reg.list_teams(id).unwrap();
        let bos = teams.iter().find(|t| t.abbrev == "BOS").unwrap().team_id;
        let opts = reg.team_membership_options(id, bos).unwrap();
        let league = reg
            .list_leagues(id)
            .unwrap()
            .leagues
            .into_iter()
            .find(|l| l.name_key.as_deref() == Some(key) || l.label.as_deref() == Some(key))
            .unwrap_or_else(|| panic!("{game}: league {key}"))
            .code as u64;
        let g = opts.groups_by_league.iter().find(|g| g.league_code == league).unwrap();
        assert!(g.conferences.is_empty() && g.divisions.is_empty() && g.pairs.is_empty(), "{game}: single group");
        let own = g.league_group;
        let nhl = opts.groups_by_league.iter().find(|g| g.league_code == 0).unwrap();
        let west = nhl.conferences.iter().find(|c| c.label == "Western Conference").unwrap().value;
        let pacific = nhl.divisions.iter().find(|d| d.label == "Pacific Division").unwrap().value;

        let before = reg.summary(id).unwrap();
        let pending_before = reg.pending_changes(id, 20).unwrap().primary.total_cells;
        for (conf, div, slot, what) in [
            (Some(own), Some(pacific), "division", "correct conference + wrong division"),
            (Some(west), Some(own), "conference", "wrong conference + correct division"),
            (None, Some(pacific), "division", "omitted conference + wrong division"),
        ] {
            let err = match reg.set_team_membership(id, bos, league, conf, div) {
                Ok(r) => panic!("{game}: {what} must be refused, got writes {:?}", r.writes),
                Err(e) => e.to_string(),
            };
            assert!(err.contains("single-group") && err.contains(&format!("the {slot} given (")), "{game}: {what}: {err}");
            let after = reg.summary(id).unwrap();
            assert_eq!(after.workspace_revision, before.workspace_revision, "{game}: {what}: revision unchanged");
            assert_eq!((after.dirty, after.undo_depth, after.redo_depth), (false, 0, 0), "{game}: {what}: history unchanged");
            assert_eq!(reg.pending_changes(id, 20).unwrap().primary.total_cells, pending_before, "{game}: {what}: pending unchanged");
        }
        for (conf, div) in [(None, None), (Some(own), Some(own)), (Some(own), None), (None, Some(own))] {
            let r = reg.set_team_membership(id, bos, league, conf, div).unwrap();
            assert_eq!(r.writes.iter().filter(|(_, v)| *v == own as i64).count(), 3, "{game}: {conf:?}/{div:?}: {:?}", r.writes);
            assert!(reg.undo(id).unwrap());
            assert_eq!(reg.summary(id).unwrap().workspace_revision, before.workspace_revision, "{game}");
        }
        reg.close_project(id);
    }
}

/// C10 (Phase 2.3 parity, overnight 2026-09-20): the same `artabbr`-keyed families resolve on the
/// **PS4** titles — the committed NHL 15 and NHL 21 `eng_us.db` anchors by default, and, `--ignored`,
/// the extracted `inxp_00.big` English dictionaries of all seven installs (`loc-ps4/` fixtures).
/// Measured 20 Sep: every PS4 English dictionary carries `NHLTeamName_`, `NHLCityName_`,
/// `NHLTeamName_Abbr3_`, `TEAMLINE1/2_`, `TXT_NICKNAME[_ALT]_`, `X_XLA_TEAM_`, `NICKLINE2_` keys
/// (e.g. 1 116 / 551 / 405 / 917 / 295 / 133 on NHL 21).
fn ps4_team_text_case(game: &str, db_rel: &str, dictionary: &std::path::Path) {
    let reg = Registry::default();
    let anchor = if game == "nhl20" { "nhl19" } else { game };
    let id = reg
        .open_workspace_from_paths(
            Some(game),
            parse_platform("ps4").unwrap(),
            &s(&repo(db_rel)),
            None,
            &s(&repo(&format!("anchors/ps4/{anchor}/databases/nhl_game-meta.xml"))),
            &s(&repo("defs")),
        )
        .unwrap()
        .project_id;
    let teams = reg.list_teams(id).unwrap();
    let tor = teams.iter().find(|t| t.abbrev == "TOR").unwrap_or_else(|| panic!("{game}: TOR")).team_id;
    reg.attach_dictionary(id, &s(dictionary)).unwrap_or_else(|e| panic!("{game}: attach {}: {e}", dictionary.display()));
    let t = reg.team_text(id, tor).unwrap().unwrap();
    assert_eq!(t.artabbr.as_deref(), Some("TOR"), "{game}");
    let by = |f: &str| t.keys.iter().find(|k| k.family == f).unwrap();
    assert!(by("full name").text.as_deref().unwrap_or("").starts_with("Toronto Maple Leafs"), "{game}: {:?}", by("full name").text);
    assert_eq!(by("city").text.as_deref(), Some("Toronto"), "{game}");
    assert_eq!(by("abbreviation").text.as_deref(), Some("TOR"), "{game}");
    assert_eq!(by("nickname (short)").text.as_deref(), Some("Leafs"), "{game}");
    assert_eq!(by("ticker").text.as_deref(), Some("TOR"), "{game}");
    let resolved = t.keys.iter().filter(|k| k.lookup != "absent").count();
    assert!(resolved >= 8, "{game}: {:?}", t.keys.iter().map(|k| (&k.family, &k.lookup)).collect::<Vec<_>>());
    // The free-text edit route holds on the PS4 dictionary too.
    let p = reg.dictionary_retext_preview(id, &by("nickname (short)").key, "Buds").unwrap_or_else(|e| panic!("{game}: retext preview: {e}"));
    assert_eq!(p.text_after, "Buds", "{game}");
}

/// NHL 15 is the one PS4 anchor that commits its `eng_us.db` (NHL 21 commits only the meta).
#[test]
fn ps4_team_text_resolves_by_artabbr_on_the_committed_anchor() {
    ps4_team_text_case("nhl15", "anchors/ps4/nhl15/databases/nhl_game.db", &repo("anchors/ps4/nhl15/databases/eng_us.db"));
}

#[test]
#[ignore = "private fixtures loc-ps4/<title>/<archive>/eng_us.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn ps4_team_text_resolves_on_every_install_dictionary() {
    let root = PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES"));
    for game in ["nhl15", "nhl16", "nhl17", "nhl18", "nhl19", "nhl20", "nhl21"] {
        let anchor = if game == "nhl20" { "nhl19" } else { game };
        // NHL 15's base archive is `inxp00.big`; every later title `inxp_00.big`. The patch-archive
        // copies (NHL 15 `patch_inxp00.big`, NHL 19 `patch_inxp_51…56.big`) are swept too.
        let title_dir = root.join("loc-ps4").join(game);
        let mut dicts: Vec<PathBuf> = std::fs::read_dir(&title_dir)
            .unwrap_or_else(|e| panic!("{}: {e}", title_dir.display()))
            .flatten()
            .map(|e| e.path().join("eng_us.db"))
            .filter(|p| p.is_file())
            .collect();
        dicts.sort();
        assert!(!dicts.is_empty(), "{}: no eng_us.db", title_dir.display());
        for dict in dicts {
            ps4_team_text_case(game, &format!("anchors/ps4/{anchor}/databases/nhl_game.db"), &dict);
        }
    }
}
