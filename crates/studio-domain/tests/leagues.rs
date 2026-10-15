//! League identity resolution across the committed anchors (O1 / L1.1-B).
//!
//! Every anchor is exercised: PS4 titles must resolve their `stockleagues` rows natively, gen6/gen7
//! titles must select exactly the vocabulary measured for their game id, ambiguous layouts must
//! stay raw without a game id, and unknown codes must remain explicit. The PS3 and Xbox 360
//! anchors are byte-identical per title, so they exercise lookup selection, not separate runtime
//! qualification.

use std::path::{Path, PathBuf};

use studio_domain::{
    EditSession, FieldValue, LabelConfidence, LeagueCatalog, LeagueSource, LeagueVocabularies,
    Platform, Project, ProjectRole,
};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn vocabs() -> LeagueVocabularies {
    LeagueVocabularies::from_dir(repo("defs/leagues")).unwrap()
}

struct Anchor {
    label: String,
    game: String,
    db: PathBuf,
    meta: PathBuf,
}

fn anchors() -> Vec<Anchor> {
    let mut out = Vec::new();
    for platform in ["ps3", "xbox360", "ps4"] {
        let root = repo("anchors").join(platform);
        let mut games: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        games.sort_by_key(|e| e.file_name());
        for game in games {
            let dbs = game.path().join("databases");
            for (db, meta) in [
                ("nhl_game.db", "nhl_game-meta.xml"),
                ("nhlng.db", "nhlng-meta.xml"),
                ("NHL07NG.db", "NHL07NG-meta.xml"),
            ] {
                if dbs.join(db).is_file() && dbs.join(meta).is_file() {
                    out.push(Anchor {
                        label: format!("{platform}/{}", game.file_name().to_string_lossy()),
                        game: game.file_name().to_string_lossy().into_owned(),
                        db: dbs.join(db),
                        meta: dbs.join(meta),
                    });
                    break;
                }
            }
        }
    }
    assert_eq!(out.len(), 27, "27 main-database anchors");
    out
}

fn catalog(a: &Anchor, game_id: Option<&str>) -> LeagueCatalog {
    let bytes = std::fs::read(&a.db).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&a.meta).unwrap()).unwrap();
    LeagueCatalog::build(&db, &meta, &vocabs(), game_id)
}

fn label(c: &LeagueCatalog, code: u64) -> Option<String> {
    c.league(code).and_then(|l| l.label.clone())
}

/// Every anchor resolves through the right source; the labels the matrix records are produced
/// for the titles they were measured on; unknown codes stay raw.
#[test]
fn every_anchor_selects_the_measured_layout() {
    let expected_vocab = |game: &str| -> Option<&'static str> {
        Some(match game {
            "nhl07" => "gen6-nhl07",
            "nhl08" => "gen7-nhl08",
            "nhl09" => "gen7-nhl09",
            "nhl10" => "gen7-nhl10",
            "nhl11" => "gen7-nhl11",
            "nhl12" | "nhl13" | "nhl14" | "nhl15" => "gen7-nhl12-15",
            "nhllegacy" => "gen7-nhllegacy",
            _ => return None,
        })
    };
    for a in anchors() {
        let c = catalog(&a, Some(&a.game));
        let is_ps4 = a.label.starts_with("ps4/");
        if is_ps4 {
            assert_eq!(c.source, LeagueSource::StockLeagues, "{}", a.label);
            assert!(
                c.vocabulary.is_none() && c.unresolved_reason.is_none(),
                "{}",
                a.label
            );
            // Native rows: leagueid is the code, the name is a key, nothing is labelled.
            assert!(!c.leagues.is_empty(), "{}", a.label);
            for l in &c.leagues {
                if l.record.is_some() {
                    assert_eq!(l.confidence, Some(LabelConfidence::Native), "{}", a.label);
                    assert!(
                        l.label.is_none(),
                        "{}: PS4 names are keys, not labels",
                        a.label
                    );
                }
            }
            assert_eq!(
                c.league(0).unwrap().name_key.as_deref(),
                Some("NHL"),
                "{}",
                a.label
            );
            assert_eq!(c.display(0), "NHL", "{}: key shown as key", a.label);
        } else {
            assert_eq!(c.source, LeagueSource::DerivedCodes, "{}", a.label);
            assert_eq!(
                c.vocabulary.as_deref(),
                expected_vocab(&a.game),
                "{}: {:?}",
                a.label,
                c.unresolved_reason
            );
            assert_eq!(label(&c, 0).as_deref(), Some("NHL"), "{}", a.label);
        }
        // Team codes always count members (every anchor has NHL teams).
        assert!(c.league(0).unwrap().member_teams >= 30, "{}", a.label);
        // Selection keys are native codes: no two identities share a code.
        let mut codes: Vec<u64> = c.leagues.iter().map(|l| l.code).collect();
        codes.dedup();
        assert_eq!(codes.len(), c.leagues.len(), "{}", a.label);
    }
}

/// The codes that the old global enum mislabelled now resolve per layout (matrix §3.3).
#[test]
fn layout_specific_codes_resolve_per_title() {
    let by_game = |g: &str| {
        anchors()
            .into_iter()
            .find(|a| a.game == g && a.label.starts_with("ps3/"))
            .unwrap()
    };
    let c08 = catalog(&by_game("nhl08"), Some("nhl08"));
    assert_eq!(label(&c08, 4).as_deref(), Some("National Teams"));
    assert_eq!(label(&c08, 5).as_deref(), Some("Custom Teams"));

    let c09 = catalog(&by_game("nhl09"), Some("nhl09"));
    assert_eq!(label(&c09, 6).as_deref(), Some("Russian League"));
    assert_eq!(label(&c09, 7).as_deref(), Some("National Teams"));
    // P1.1-X (2026-09-18): the NHL 09 executable binds code 8 to HISTORIC_TEAMS; the label is the
    // traced key's dictionary text class ("enum"), no longer a raw code.
    assert_eq!(label(&c09, 8).as_deref(), Some("Historic Teams"));
    assert_eq!(c09.league(8).unwrap().confidence, Some(LabelConfidence::Enum));
    assert_eq!(c09.league(8).unwrap().member_teams, 1);
    assert_eq!(c09.league(4).unwrap().dictionary_key.as_deref(), Some("DEL"), "NHL 09 binds code 4 to `DEL`");

    let c10 = catalog(&by_game("nhl10"), Some("nhl10"));
    assert_eq!(label(&c10, 6).as_deref(), Some("NLA (Switzerland)"));
    assert_eq!(label(&c10, 9).as_deref(), Some("Prospect Teams"), "P1.1-X: NHL 10 code 9 = PROSPECT_TEAMS");
    assert_eq!(label(&c10, 8).as_deref(), Some("Historic Teams"));

    let c11 = catalog(&by_game("nhl11"), Some("nhl11"));
    assert_eq!(label(&c11, 8).as_deref(), Some("OHL"));
    assert_eq!(label(&c11, 11).as_deref(), Some("Top Prospects"));

    let c15 = catalog(&by_game("nhl15"), Some("nhl15"));
    assert_eq!(label(&c15, 7).as_deref(), Some("National Teams"));
    assert_eq!(label(&c15, 8).as_deref(), Some("OHL"));
    assert_eq!(label(&c15, 12).as_deref(), Some("Legend Teams"));
    assert_eq!(
        c15.league(8).unwrap().confidence,
        Some(LabelConfidence::Corroborated)
    );

    let cl = catalog(&by_game("nhllegacy"), Some("nhllegacy"));
    assert_eq!(label(&cl, 7).as_deref(), Some("GET-ligaen (Norway)"));
    assert_eq!(label(&cl, 8).as_deref(), Some("National Teams"));
    assert_eq!(label(&cl, 15).as_deref(), Some("EASHL"));
    // Group labels come from the title's competition group table (P1.2-X) — the observed English
    // text without a dictionary attached.
    assert_eq!(cl.group_label("OwTu", 0, 1).as_deref(), Some("Western Conference"));
    assert_eq!(
        cl.group_label("CGMK", 0, 6).as_deref(),
        Some("Metropolitan Division")
    );
    // Group ids are global on this family: id 1 is the NHL's Western Conference whichever league
    // code asks (an AHL team would carry 8/9); id 8 is the AHL's Western Conference.
    assert_eq!(cl.group_label("OwTu", 1, 8).as_deref(), Some("Western Conference"));
    // NHL 15 has its own table (P1.2-X): id 1 is its Western Conference too, but its divisions are
    // the A–D block (id 3 = "Pacific Division" in its dictionary text).
    assert_eq!(c15.group_label("OwTu", 0, 1).as_deref(), Some("Western Conference"));
    assert_eq!(c15.group_label("CGMK", 0, 3).as_deref(), Some("Pacific Division"));

    let c07 = anchors()
        .into_iter()
        .find(|a| a.label == "xbox360/nhl07")
        .unwrap();
    let c07 = catalog(&c07, Some("nhl07"));
    assert_eq!(c07.team_league_table, "HqzU");
    assert_eq!(label(&c07, 1).as_deref(), Some("National Teams"));
    assert!(label(&c07, 5).is_none(), "DAV stays raw");
}

/// NHL 21's stockleagues rows are read natively, joined on leagueid, with member counts from the
/// team table (matrix §1.3).
#[test]
fn nhl21_stockleagues_resolve_natively() {
    let a = anchors()
        .into_iter()
        .find(|a| a.label == "ps4/nhl21")
        .unwrap();
    let c = catalog(&a, Some("nhl21"));
    assert_eq!(c.leagues.iter().filter(|l| l.record.is_some()).count(), 31);
    let alls = c.league(8).unwrap();
    assert_eq!(alls.name_key.as_deref(), Some("Allsvenskan"));
    assert_eq!(alls.record, Some(8));
    assert_eq!(alls.member_teams, 14);
    assert_eq!(alls.art_id, Some(19));
    assert_eq!(alls.render_id, Some(22));
    assert_eq!(alls.order_id, Some(11));
    assert_eq!(
        c.league(7).unwrap().name_key.as_deref(),
        Some("TXT_LEAGUE_EBEL")
    );
    assert_eq!(c.league(7).unwrap().member_teams, 11);
    assert_eq!(c.league(23).unwrap().member_teams, 97);
    // Three CUSTOM_TEAMS rows share a key: identities stay distinct by code.
    let custom: Vec<u64> = c
        .leagues
        .iter()
        .filter(|l| l.name_key.as_deref() == Some("CUSTOM_TEAMS"))
        .map(|l| l.code)
        .collect();
    assert_eq!(custom, vec![16, 24, 25]);
    // P1-C: the PS4 group namespace is labelled through the title's own executable table
    // (research 17), selected by the stockleagues source even without a platform.
    assert_eq!(c.group_label("OwTu", 0, 6).as_deref(), Some("Western Conference"));
    assert_eq!(c.competition.as_ref().unwrap().evidence, "by-reference", "no platform given");
}

/// Without a game id, the NHL 12–15 / Legacy fingerprint is ambiguous and codes stay raw with a
/// reason; a unique fingerprint (NHL 11) still selects.
#[test]
fn ambiguous_layout_without_game_id_stays_raw() {
    let legacy = anchors()
        .into_iter()
        .find(|a| a.label == "ps3/nhllegacy")
        .unwrap();
    let c = catalog(&legacy, None);
    assert!(c.vocabulary.is_none());
    let reason = c.unresolved_reason.clone().unwrap();
    assert!(
        reason.contains("gen7-nhl12-15") && reason.contains("gen7-nhllegacy"),
        "{reason}"
    );
    assert!(label(&c, 7).is_none());
    assert_eq!(c.display(7), "League code 7");

    let nhl11 = anchors()
        .into_iter()
        .find(|a| a.label == "ps3/nhl11")
        .unwrap();
    let c = catalog(&nhl11, None);
    assert_eq!(c.vocabulary.as_deref(), Some("gen7-nhl11"));

    // A game id no vocabulary was measured for leaves codes raw and says so.
    let c = catalog(&legacy, Some("nhl99"));
    assert!(c.vocabulary.is_none());
    assert!(c.unresolved_reason.unwrap().contains("nhl99"));
}

/// Team summaries and the team detail share the catalog: the header text, the list label and the
/// decorated `league` field agree, and `leaguegroup` is never presented as the league.
#[test]
fn team_surfaces_share_the_resolver() {
    let project = Project::load(
        repo("anchors/ps3/nhllegacy/databases/nhlng.db"),
        Platform::Ps3,
        ProjectRole::Nhlng,
        repo("anchors/ps3/nhllegacy/databases/nhlng-meta.xml"),
        repo("defs"),
    )
    .unwrap();
    let mut session = EditSession::begin(project);
    session.set_game_id(Some("nhllegacy".to_string()));
    let teams = session.team_summaries().unwrap();
    let norway = teams
        .iter()
        .find(|t| t.abbrev == "STAV")
        .expect("Stavanger is in the Legacy anchor");
    assert_eq!(norway.league, Some(7));
    assert_eq!(norway.league_label.as_deref(), Some("GET-ligaen (Norway)"));
    let view = session.team_view(norway.team_id).unwrap().unwrap();
    assert_eq!(view.league_label.as_deref(), Some("GET-ligaen (Norway)"));
    // Norway is a single-group league on Legacy: its conference field carries the league's own
    // group (id 19), which the competition table resolves to the league's text.
    assert_eq!(view.conference_label.as_deref(), Some("Norway"));
    let league_cell = view
        .profile
        .sections
        .iter()
        .flat_map(|s| s.cells.iter())
        .find(|c| c.label == "League")
        .expect("League cell");
    let f = league_cell.field.as_ref().unwrap();
    assert_eq!(f.studio_field, "xMjj");
    assert_eq!(f.display, "GET-ligaen (Norway)");
    assert!(f
        .enum_options
        .iter()
        .any(|(code, text)| *code == 8 && text == "National Teams"));
    assert!(
        view.profile
            .sections
            .iter()
            .flat_map(|s| s.cells.iter())
            .any(|c| c.label == "League group" && c.field.as_ref().unwrap().studio_field == "iIBl"),
        "leaguegroup is exposed under its own name"
    );
    let nhl = teams.iter().find(|t| t.abbrev == "BOS").unwrap();
    let view = session.team_view(nhl.team_id).unwrap().unwrap();
    assert_eq!(view.conference_label.as_deref(), Some("Eastern Conference"));
    assert_eq!(view.division_label.as_deref(), Some("Atlantic Division"));

    // Without the game id the same session shows raw codes rather than a neighbour's labels.
    session.set_game_id(None);
    let teams = session.team_summaries().unwrap();
    let norway = teams.iter().find(|t| t.abbrev == "STAV").unwrap();
    assert_eq!(norway.league_label.as_deref(), Some("League code 7"));
}

/// The PS4 league row's key field is protected; the name key is editable within its byte width.
#[test]
fn nhl21_league_key_protected_and_name_bounded() {
    let project = Project::load(
        repo("anchors/ps4/nhl21/databases/nhl_game.db"),
        Platform::Ps4,
        ProjectRole::Nhlng,
        repo("anchors/ps4/nhl21/databases/nhl_game-meta.xml"),
        repo("defs"),
    )
    .unwrap();
    let mut session = EditSession::begin(project);
    session.set_game_id(Some("nhl21".to_string()));
    let err = session
        .set_field("fsLK", 8, "aQrQ", FieldValue::Int(9))
        .expect_err("leagueid is a key");
    assert!(err.to_string().contains("not updatable"), "{err}");
    let err = session
        .set_field("fsLK", 8, "vIys", FieldValue::Text("X".repeat(26)))
        .expect_err("26 bytes exceed the 25-byte field");
    // LEAGUE-SMOKE-1: the refusal is the sentence the row shows the user — the field's metadata
    // name, the byte count and the limit — not the internal "invalid string length: 26".
    assert_eq!(
        err.to_string(),
        "name: the text is 26 bytes but this field holds at most 25 bytes; the edit was not applied",
    );
    session
        .set_field("fsLK", 8, "vIys", FieldValue::Text("TXT_ROW_LEAGUE".into()))
        .unwrap();
    let c = session.league_catalog().unwrap();
    assert_eq!(
        c.league(8).unwrap().name_key.as_deref(),
        Some("TXT_ROW_LEAGUE")
    );
    assert_eq!(
        c.league(8).unwrap().member_teams,
        14,
        "membership untouched"
    );
    assert!(session.undo());
    let c = session.league_catalog().unwrap();
    assert_eq!(
        c.league(8).unwrap().name_key.as_deref(),
        Some("Allsvenskan")
    );
}

/// LEAGUE-R1 F2 at the domain seams: every committed operation (single cell, multi-cell) runs
/// the delta referential gate; a refused op leaves the working bytes, undo depth and pending
/// review untouched.
#[test]
fn nhl21_dangling_league_reference_refused_on_every_edit_seam() {
    let project = Project::load(
        repo("anchors/ps4/nhl21/databases/nhl_game.db"),
        Platform::Ps4,
        ProjectRole::Nhlng,
        repo("anchors/ps4/nhl21/databases/nhl_game-meta.xml"),
        repo("defs"),
    )
    .unwrap();
    let mut session = EditSession::begin(project);
    session.set_game_id(Some("nhl21".to_string()));
    let baseline = session.bytes().to_vec();
    let err = session
        .set_field("kOtt", 0, "xMjj", FieldValue::Int(31))
        .expect_err("dangling league reference");
    assert!(
        err.to_string().contains("refers to no stockleagues row"),
        "{err}"
    );
    assert_eq!(session.bytes(), &baseline[..]);
    assert_eq!(session.undo_depth(), 0);
    assert!(!session.pending_changes(10).dirty);
    // The multi-cell seam is gated identically.
    let err = session
        .set_cells_int("kOtt", 0, &[("xMjj".to_string(), 31)])
        .expect_err("bulk dangling league reference");
    assert!(
        err.to_string().contains("refers to no stockleagues row"),
        "{err}"
    );
    assert_eq!(session.bytes(), &baseline[..]);
    // Valid: code 30 exists (HUT) even though no team uses it.
    session
        .set_cells_int("kOtt", 0, &[("xMjj".to_string(), 30)])
        .unwrap();
    assert_eq!(session.undo_depth(), 1);
    assert!(session.undo());
    assert_eq!(session.bytes(), &baseline[..]);
}

/// R1 F5: the code -> key evidence is resolved per target against the vocabulary's witness. The
/// Legacy mapping was traced on NHL Legacy Xbox 360's executable only: that target shows
/// `traced`; NHL Legacy PS3 and Legacy Recompiled show `by-reference` for the same key; a
/// catalog built without a platform never shows `traced`; NHL 12-15 stay `hypothesis`.
#[test]
fn key_evidence_is_scoped_to_the_witnessed_target() {
    let all = anchors();
    let a = all.iter().find(|a| a.label == "xbox360/nhllegacy").unwrap();
    let bytes = std::fs::read(&a.db).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&a.meta).unwrap()).unwrap();
    let v = vocabs();
    let ev = |game: &str, platform: Option<Platform>| {
        let c = LeagueCatalog::build_for_target(&db, &meta, &v, Some(game), platform, None);
        let l = c.league(7).expect("code 7");
        (
            l.dictionary_key.clone(),
            l.dictionary_key_evidence.clone(),
            l.evidence.clone(),
        )
    };
    let (k, e, note) = ev("nhllegacy", Some(Platform::Xbox));
    assert_eq!(k.as_deref(), Some("TXT_LEAGUE_NORWAY"));
    assert_eq!(e.as_deref(), Some("traced"));
    assert!(note.contains("default.xex"), "{note}");

    let (k, e, note) = ev("nhllegacy", Some(Platform::Ps3));
    assert_eq!(k.as_deref(), Some("TXT_LEAGUE_NORWAY"), "the mapping is shared");
    assert_eq!(e.as_deref(), Some("by-reference"));
    assert!(note.contains("traced on nhllegacy xbox"), "{note}");
    assert!(note.contains("not inspected"), "{note}");

    let (_, e, _) = ev("legacyrecomp", Some(Platform::Recomp));
    assert_eq!(e.as_deref(), Some("by-reference"));

    let (_, e, _) = ev("nhllegacy", None);
    assert_eq!(e.as_deref(), Some("by-reference"), "no platform: never traced");

    // NHL 14 (same fingerprint, its own vocabulary): traced on its own Xbox 360 executable
    // (P1.1-X), by-reference on PS3.
    let b = all.iter().find(|a| a.label == "xbox360/nhl14").unwrap();
    let bytes = std::fs::read(&b.db).unwrap();
    let db14 = TdbFile::parse(&bytes).unwrap();
    let meta14 = FieldMetaIndex::from_xml(&std::fs::read_to_string(&b.meta).unwrap()).unwrap();
    for (p, want) in [(Platform::Xbox, "traced"), (Platform::Ps3, "by-reference")] {
        let c = LeagueCatalog::build_for_target(&db14, &meta14, &v, Some("nhl14"), Some(p), None);
        let l = c.league(12).expect("code 12");
        assert_eq!(l.dictionary_key_evidence.as_deref(), Some(want), "{p:?}");
    }
    // NHL 08: literal block only — never traced.
    let b = all.iter().find(|a| a.label == "xbox360/nhl08").unwrap();
    let bytes = std::fs::read(&b.db).unwrap();
    let db08 = TdbFile::parse(&bytes).unwrap();
    let meta08 = FieldMetaIndex::from_xml(&std::fs::read_to_string(&b.meta).unwrap()).unwrap();
    let c = LeagueCatalog::build_for_target(&db08, &meta08, &v, Some("nhl08"), Some(Platform::Xbox), None);
    assert_eq!(c.league(3).unwrap().dictionary_key_evidence.as_deref(), Some("hypothesis"));
}

/// Research handoff P1.1-X (2026-09-18): every gen7 vocabulary's `dictionary_key` must equal (case-
/// insensitively — the game's lookup is by case-folded hash) the key the title's Xbox 360 executable
/// binds the code to, for every title with a traced pointer table; a `traced` claim must be witnessed
/// on exactly those Xbox targets; and every traced code the executable knows is catalogued. NHL 07/08
/// (literal blocks, consuming path untraced) must not claim `traced`.
#[test]
fn vocabulary_keys_match_the_traced_xbox360_tables() {
    let text = std::fs::read_to_string(repo(
        "docs/research/overnight-2026-09-18/data/xbox360-league-code-key-tables.json",
    ))
    .expect("the research key-table handoff is in the repo");
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let titles = json["titles"].as_object().unwrap();
    let v = vocabs();
    let mut checked = 0;
    for (game, t) in titles {
        let pointer = t["table_kind"] == "pointer";
        let vocab = v
            .vocabularies()
            .iter()
            .find(|x| x.applies_to.iter().any(|g| g == game))
            .unwrap_or_else(|| panic!("no vocabulary applies to {game}"));
        let witnessed = vocab
            .key_witness
            .as_ref()
            .map(|w| w.traced_on.iter().any(|x| x.game == *game && x.platform == "xbox"))
            .unwrap_or(false);
        assert_eq!(witnessed, pointer, "{game}: witness iff a pointer table was traced");
        for e in t["entries"].as_array().unwrap() {
            let code = e["code"].as_u64().unwrap();
            let key = e["key"].as_str().unwrap();
            let entry = vocab.entries.iter().find(|x| x.raw == code);
            if !pointer {
                if let Some(x) = entry {
                    assert_ne!(x.key_evidence.as_deref(), Some("traced"), "{game} code {code}");
                }
                continue;
            }
            let x = entry.unwrap_or_else(|| panic!("{game}: traced code {code} ({key}) is not catalogued"));
            let k = x
                .dictionary_key
                .as_deref()
                .unwrap_or_else(|| panic!("{game} code {code}: no dictionary_key (traced {key})"));
            assert!(k.eq_ignore_ascii_case(key), "{game} code {code}: vocabulary {k} vs traced {key}");
            assert_eq!(x.key_evidence.as_deref(), Some("traced"), "{game} code {code}");
            checked += 1;
        }
    }
    assert!(checked >= 100, "{checked} traced bindings checked");
}

/// Phase 1.2 (research handoff P1.2-X): the NHL 12 → Legacy titles' team rows carry global group
/// ids indexing a title-specific executable table. The baked tables resolve conference / division
/// labels for every team (through the attached dictionary when present), are scoped to their own
/// title, and validate a (league, conference, division) triple.
#[test]
fn competition_groups_resolve_per_title_and_validate_membership() {
    use studio_domain::leagues::AttachedDictionary;
    let all = anchors();
    let v = vocabs();
    let a = all.iter().find(|a| a.label == "xbox360/nhllegacy").unwrap();
    let bytes = std::fs::read(&a.db).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&a.meta).unwrap()).unwrap();
    let dict = AttachedDictionary::load(
        repo("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db").to_str().unwrap(),
    )
    .unwrap();
    let cat = LeagueCatalog::build_for_target(&db, &meta, &v, Some("nhllegacy"), Some(Platform::Xbox), Some(&dict));
    let c = cat.competition.as_ref().expect("Legacy has a group table");
    assert_eq!(c.evidence, "traced");
    assert_eq!(c.all().count(), 49);
    // League 0 = NHL: two conferences, four divisions, labels from the dictionary.
    let confs: Vec<&str> = c.for_league(0, "conference").iter().map(|g| g.label.as_str()).collect();
    assert_eq!(confs, ["Western Conference", "Eastern Conference"]);
    let divs: Vec<&str> = c.for_league(0, "division").iter().map(|g| g.label.as_str()).collect();
    assert_eq!(divs, ["Pacific Division", "Central Division", "Atlantic Division", "Metropolitan Division"]);
    assert_eq!(c.group(3).unwrap().label_source, "dictionary");
    assert_eq!(c.group(3).unwrap().key, "NHLDivisionA");
    // The catalog's group labels go through the table (field ids are the team-table group fields).
    assert_eq!(cat.group_label("CGMK", 0, 6).as_deref(), Some("Metropolitan Division"));
    assert_eq!(cat.group_label("OwTu", 0, 1).as_deref(), Some("Western Conference"));
    // Norway (code 7) is a single group: its own id 19 in all three fields.
    assert_eq!(c.league_group(7).unwrap().id, 19);
    assert!(c.for_league(7, "conference").is_empty());
    assert!(c.validate_membership(7, 19, 19).is_ok());
    assert!(c.validate_membership(7, 1, 3).unwrap_err().contains("single-group"));
    // NHL membership (P1-C-R1: witnessed parent pairs): Western pairs with Pacific/Central only;
    // an AHL division, an Eastern division or a conference id under Western is refused.
    assert!(c.validate_membership(0, 1, 3).is_ok());
    assert!(c.validate_membership(0, 1, 4).is_ok());
    assert!(c.validate_membership(0, 2, 5).is_ok());
    assert!(c.validate_membership(0, 2, 6).is_ok());
    for div in [10, 5, 2] {
        let err = c.validate_membership(0, 1, div).unwrap_err();
        assert!(err.contains("is not paired with Western Conference"), "{div}: {err}");
    }
    assert!(c.validate_membership(0, 99, 3).unwrap_err().contains("not a conference"));
    // The All-Star rows carry their own conference id as their division: recorded as the two
    // squads' exceptional arrangement, refused for a club, browsed as it is.
    let err = c.validate_membership(0, 1, 1).unwrap_err();
    assert!(err.contains("exceptional stock arrangement") && err.contains("team id 31"), "{err}");
    assert_eq!(c.exceptional_pairs(0).len(), 2);
    assert_eq!(c.regular_pairs(0), [(1, 3), (1, 4), (2, 5), (2, 6)]);
    // QMJHL (code 10 on Legacy) has no conferences: its own id in the conference slot with one
    // of its three divisions; its own id in both slots is not witnessed.
    assert_eq!(c.regular_pairs(10), [(27, 28), (27, 29), (27, 30)]);
    assert!(c.validate_membership(10, 27, 27).unwrap_err().contains("not paired"));

    // PS3 Legacy: same table by reference; Recompiled likewise.
    let cat_ps3 = LeagueCatalog::build_for_target(&db, &meta, &v, Some("nhllegacy"), Some(Platform::Ps3), None);
    assert_eq!(cat_ps3.competition.as_ref().unwrap().evidence, "by-reference");
    assert_eq!(cat_ps3.competition.as_ref().unwrap().group(3).unwrap().label_source, "observed");
    let cat_rc = LeagueCatalog::build_for_target(&db, &meta, &v, Some("legacyrecomp"), Some(Platform::Recomp), None);
    assert_eq!(cat_rc.competition.as_ref().unwrap().game, "nhllegacy");

    // NHL 12: a different numbering (six named NHL divisions at ids 3..8); NHL 14: A–D at 3..6.
    for (label, game, want) in [
        ("xbox360/nhl12", "nhl12", vec!["NORTHWEST DIVISION", "PACIFIC DIVISION", "CENTRAL DIVISION", "SOUTHEAST DIVISION", "ATLANTIC DIVISION", "NORTHEAST DIVISION"]),
        ("xbox360/nhl14", "nhl14", vec!["Division A", "Division B", "Division C", "Division D"]),
    ] {
        let b = all.iter().find(|a| a.label == label).unwrap();
        let bytes = std::fs::read(&b.db).unwrap();
        let dbx = TdbFile::parse(&bytes).unwrap();
        let metax = FieldMetaIndex::from_xml(&std::fs::read_to_string(&b.meta).unwrap()).unwrap();
        let cx = LeagueCatalog::build_for_target(&dbx, &metax, &v, Some(game), Some(Platform::Xbox), None);
        let c = cx.competition.as_ref().unwrap();
        let divs: Vec<String> = c.for_league(0, "division").iter().map(|g| g.label.clone()).collect();
        assert_eq!(divs, want, "{game}");
    }
    // NHL 11 has no group table (local conference/division bits).
    let b = all.iter().find(|a| a.label == "xbox360/nhl11").unwrap();
    let bytes = std::fs::read(&b.db).unwrap();
    let db11 = TdbFile::parse(&bytes).unwrap();
    let meta11 = FieldMetaIndex::from_xml(&std::fs::read_to_string(&b.meta).unwrap()).unwrap();
    let c11 = LeagueCatalog::build_for_target(&db11, &meta11, &v, Some("nhl11"), Some(Platform::Xbox), None);
    assert!(c11.competition.is_none());
}

/// Phase 1.3 tie-in: conference keys are shared across leagues on the Legacy table (renaming
/// `WesternConference` renames the NHL, AHL, OHL and WHL conferences at once); division keys of
/// the NHL are its own.
#[test]
fn legacy_structure_discloses_shared_group_keys() {
    let all = anchors();
    let a = all.iter().find(|a| a.label == "xbox360/nhllegacy").unwrap();
    let bytes = std::fs::read(&a.db).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&a.meta).unwrap()).unwrap();
    let cat = LeagueCatalog::build_for_target(&db, &meta, &vocabs(), Some("nhllegacy"), Some(Platform::Xbox), None);
    let st = cat.structure(0).unwrap();
    let west = st.shared_keys.iter().find(|(k, _)| k == "WesternConference").expect("shared");
    assert_eq!(west.1.len(), 3, "{:?}", west.1);
    assert!(west.1.iter().any(|l| l.contains("AHL")) && west.1.iter().any(|l| l.contains("WHL")));
    assert!(!st.shared_keys.iter().any(|(k, _)| k.starts_with("NHLDivision")));
    // Norway: a single group; its key is its own.
    assert!(cat.structure(7).unwrap().shared_keys.is_empty());
}
