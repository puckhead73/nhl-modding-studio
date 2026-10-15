//! Phase 1.5 (21 Sep 2026) — the read-only commentary catalogue through the registry seam on the
//! real Xbox 360 NHL 15 and Legacy trees (read only): the totals equal the research trace
//! (2026-09-20 handoff 13 rev. 4 §"Plain-language summary": NHL 15 / Legacy only 1 489 / 1 718 of
//! 3 714 / 3 982 events have a sentence carrying their name; every AHL / CHL / OHL / QMJHL / WHL /
//! tournament cup event is among those without), the query / league filter and the cap work, and
//! an out-of-coverage title refuses with its reason. A missing install fails, never skips.

use std::path::{Path, PathBuf};

use studio_api::Registry;
use studio_domain::speech_catalogue::is_league_name;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn open(reg: &Registry, platform: &str, game: &str) -> u32 {
    let dir = if platform == "xbox" { "xbox360" } else { "ps3" };
    let db = repo(&format!("anchors/{dir}/{game}/databases/nhlng.db"));
    reg.open_workspace_from_paths(Some(game), studio_api::parse_platform(platform).unwrap(), &s(&db), None, &s(&repo(&format!("anchors/{dir}/{game}/databases/nhlng-meta.xml"))), &s(&repo("defs")))
        .unwrap()
        .project_id
}

#[test]
#[ignore = "real Xbox 360 installs under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn nhl15_and_legacy_catalogues_match_the_research_trace_and_filter() {
    for (game, root, events, sentences, with_sentence) in [
        ("nhl15", r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15", 3714usize, 1489usize, 1489usize),
        ("nhllegacy", r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy", 3982, 1718, 1718),
    ] {
        assert!(Path::new(root).is_dir(), "{root} missing (fails, never skips)");
        let reg = Registry::default();
        let id = open(&reg, "xbox", game);
        let all = reg.speech_catalogue(id, root, "", false, 5000).unwrap();
        assert_eq!(all.stats.events, events, "{game}: events");
        assert_eq!(all.stats.sentences, sentences, "{game}: sentences");
        assert_eq!(all.stats.sentences_slot_ok, sentences, "{game}: every sentence on its FNV slot");
        assert_eq!(all.stats.events_with_a_sentence, with_sentence, "{game}: events with a sentence sharing their name");
        assert_eq!(all.stats.events_without_a_sentence, events - with_sentence);
        assert_eq!(all.stats.phrases, all.stats.phrases_with_table, "{game}: every phrase has its sample-group table");
        assert!(all.stats.keywords > 2000, "{game}: the loose keyword index");
        assert_eq!(all.events.len(), events, "{game}: cap 5000 holds everything");
        // Every phrase's candidates are listed; unresolved ids are reported, never dropped.
        let mut cand = 0usize;
        let mut unresolved = 0usize;
        for st in &all.sentences {
            for b in &st.bodies {
                for p in &b.phrases {
                    assert!(p.table_found, "{game}: {}: table", st.name);
                    assert_eq!(p.candidate_count, p.bank_resolved + p.unresolved_sample_ids.len());
                    assert!(p.candidate_sample_ids.len() <= 64 && p.candidate_sample_ids.len() == p.candidate_count.min(64));
                    cand += p.candidate_count;
                    unresolved += p.unresolved_sample_ids.len();
                }
            }
        }
        assert!(cand > 1_000_000, "{game}: candidates listed ({cand})");
        eprintln!("{game}: {} events / {} sentences / {} phrases; {cand} candidates, {unresolved} unresolved in the bank; {} ambiguous ids", all.stats.events, all.stats.sentences, all.stats.phrases, all.stats.ambiguous_event_ids);
        // League / cup filter: every cup event is among those without a sentence (the trace).
        let league = reg.speech_catalogue(id, root, "", true, 5000).unwrap();
        assert!(league.events.iter().all(|e| is_league_name(&e.name)));
        // The research trace (r20 rev. 4 `trace.league_without_sentence` / `league_with_sentence`):
        // 83 / 27 on NHL 15, 88 / 28 on Legacy; the cup events with a sentence are exactly
        // CUP_END_CUP_NHL and CUP_END_CUP_PRESENTATION_NHL_CONFERENCE.
        let (lw, lwith) = if game == "nhl15" { (83usize, 27usize) } else { (88, 28) };
        assert_eq!(league.events.iter().filter(|e| e.sentences == 0).count(), lw, "{game}: league events without a sentence");
        assert_eq!(league.events.iter().filter(|e| e.sentences > 0).count(), lwith, "{game}: league events with a sentence");
        let cups_with: Vec<&str> = league.events.iter().filter(|e| e.name.starts_with("CUP_END_CUP_") && e.sentences > 0).map(|e| e.name.as_str()).collect();
        assert_eq!(cups_with, vec!["CUP_END_CUP_NHL", "CUP_END_CUP_PRESENTATION_NHL_CONFERENCE"], "{game}");
        for n in ["CUP_END_CUP_AHL", "CUP_END_CUP_CHL", "CUP_END_CUP_OHL", "CUP_END_CUP_QMJHL", "CUP_END_CUP_WHL", "CUP_END_CUP_TOURNAMENT"] {
            let e = league.events.iter().find(|e| e.name == n).unwrap_or_else(|| panic!("{game}: {n}"));
            assert_eq!(e.sentences, 0, "{game}: {n} — no sentence carries this event name");
        }
        let nhl_cup = league.events.iter().find(|e| e.name == "CUP_END_CUP_NHL").expect("CUP_END_CUP_NHL");
        assert_eq!(nhl_cup.id, 29243, "{game}: the stable interfaceCrc (handoff 04 §3.5)");
        assert!(nhl_cup.parameters.iter().any(|p| p == "team_ID") || !nhl_cup.parameters.is_empty(), "{game}: parameters named");
        // Query + cap.
        let q = reg.speech_catalogue(id, root, "cup_end", false, 3).unwrap();
        assert!(q.events_matching >= 5 && q.events.len() == 3 && q.truncated, "{game}: cap");
        assert!(q.events.iter().all(|e| e.name.to_ascii_lowercase().contains("cup_end")));
        assert!(q.sources.iter().any(|f| f.role == "sentences" && f.sha256.len() == 64));
    }
    // Out of coverage: the reason, not an empty listing.
    let reg = Registry::default();
    let id = open(&reg, "xbox", "nhl10");
    let e = match reg.speech_catalogue(id, r"X:\Games\Emulation\Roms\XBOX360\NHL 10\NHL 10", "", false, 10) {
        Ok(_) => panic!("NHL 10 is outside the binary coverage and must refuse"),
        Err(e) => e.to_string(),
    };
    assert!(e.contains("XML"), "{e}");
}

/// Phase 2.7 (21 Sep 2026): the team name-call listing on the real NHL 15 / Legacy speech trees.
/// The research (2026-09-21 handoff 07 §2 row `exhibitionteams.audioid`, measured on the **PS3**
/// anchors' team tables against the loose speech files) counts **207 / 211** team audio ids inside
/// the `team_ID` key set on NHL 15 and 206 / 212 on Legacy. The listing must reproduce those on
/// the PS3 anchors; on the Xbox 360 anchors the same four / six ids are without a call and every
/// other id has one. The research counts **non-zero** ids: one NHL 15 club carries audio id 0 (a
/// `(team_ID, 0)` table exists but is not a club's name — the listing treats 0 as "no recorded
/// name id"). Every listed sample must resolve in the bank, and a club without a call says so
/// instead of erroring.
#[test]
#[ignore = "real Xbox 360 installs under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn team_name_calls_resolve_the_research_counts() {
    for (platform, game, root, distinct, with_calls) in [
        ("ps3", "nhl15", r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15", 211usize, 207usize),
        ("xbox", "nhl15", r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15", 211, 207),
        ("ps3", "nhllegacy", r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy", 212, 206),
        ("xbox", "nhllegacy", r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy", 212, 206),
    ] {
        assert!(Path::new(root).is_dir(), "{root} missing (fails, never skips)");
        let reg = Registry::default();
        let id = open(&reg, platform, game);
        let teams = reg.list_teams(id).unwrap();
        let mut ids = std::collections::BTreeSet::new();
        let mut called = std::collections::BTreeSet::new();
        let mut uncalled = std::collections::BTreeSet::new();
        for t in &teams {
            let c = reg.team_name_calls(id, t.team_id, root).unwrap();
            let a = c.team_audio_id.expect("gen7 team table carries audioid");
            if a != 0 {
                ids.insert(a);
            }
            assert!(c.team_tag.is_some(), "{game}: team_ID parameter");
            assert_eq!(c.team_name, t.name);
            if a == 0 {
                assert!(c.tables.is_empty() && c.notes.iter().any(|n| n.contains("audio id 0")), "{game}: {}: {:?}", t.name, c.notes);
                continue;
            }
            if c.tables.is_empty() {
                uncalled.insert(a);
                assert!(c.notes.iter().any(|n| n.contains("no play-by-play table")), "{game}: {}: {:?}", t.name, c.notes);
                assert_eq!(c.sample_ids_distinct, 0);
            } else {
                called.insert(a);
                for tb in &c.tables {
                    assert!(tb.num_tags >= 1 && tb.sample_count >= 1, "{game}: {}", t.name);
                    assert_eq!(tb.bank_resolved, tb.sample_count, "{game}: {}: every listed sample resolves in the bank", t.name);
                    assert!(tb.sample_ids.len() <= 64 && tb.sample_ids.len() == tb.sample_count.min(64));
                }
                assert!(c.sample_ids_distinct >= 1 && c.bank_resolved_distinct == c.sample_ids_distinct, "{game}: {}", t.name);
            }
            assert!(c.keyword_mentions.is_some(), "{game}: the keyword index is loose on 15 / Legacy");
        }
        eprintln!("{platform} {game}: teams {} distinct ids {} called {} uncalled {:?}", teams.len(), ids.len(), called.len(), uncalled);
        assert_eq!(ids.len(), distinct, "{platform} {game}: distinct team audio ids");
        assert_eq!(called.len(), with_calls, "{platform} {game}: audio ids with a team_ID table");
        assert_eq!(uncalled.len(), distinct - with_calls, "{platform} {game}: audio ids without a call: {uncalled:?}");
    }
}

/// Phase 3.5 (21 Sep 2026): the player name-call listing on the real NHL 15 tree with the PS3
/// anchor's roster. Research 2026-09-21 handoff 07 §2: `exhibitionplayerbiotable.audioid` has
/// 5 311 distinct non-zero ids of which **4 464** are announcer `playerid` keys (84 %);
/// `playernames.audioid` has 1 172 distinct ids of which 1 169 are pbp `player_ID` keys. The
/// listing must reproduce the announcer figure over the roster's distinct ids and, for the
/// play-by-play, every matched name row must resolve unless it is one of the three keyless ids
/// (or carries audio id 0). One run ≈ 8 min (the speech files are parsed per player).
#[test]
#[ignore = "real Xbox 360 install under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn player_name_calls_resolve_the_research_counts_on_nhl15() {
    let root = r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15";
    assert!(Path::new(root).is_dir(), "{root} missing (fails, never skips)");
    let reg = Registry::default();
    let id = open(&reg, "ps3", "nhl15");
    let players = reg.list_players(id).unwrap();
    let mut ann_ids = std::collections::BTreeSet::new();
    let mut ann_called = std::collections::BTreeSet::new();
    let mut pbp_ids = std::collections::BTreeSet::new();
    let mut pbp_keyless = std::collections::BTreeSet::new();
    let mut with_pbp_name = 0usize;
    for p in &players {
        let c = reg.player_name_calls(id, p.player_id, root).unwrap();
        assert_eq!((c.first_name.as_str(), c.last_name.as_str()), (p.first_name.trim(), p.last_name.trim()));
        assert_eq!(c.pbp_player_tag, Some(2), "NHL 15 pbp player_ID is tag 2");
        assert_eq!(c.announcer_tag, Some(2163));
        let a = c.announcer_audio_id.expect("bio audioid");
        if a != 0 {
            ann_ids.insert(a);
            if !c.announcer_tables.is_empty() {
                ann_called.insert(a);
                assert_eq!(c.announcer_bank_resolved_distinct, c.announcer_sample_ids_distinct, "{}: announcer samples in the bank", p.last_name);
            } else {
                assert!(c.notes.iter().any(|n| n.contains("no announcer table")), "{}: {:?}", p.last_name, c.notes);
            }
        }
        if !c.pbp_names.is_empty() {
            with_pbp_name += 1;
        }
        for n in &c.pbp_names {
            assert!(n.name.eq_ignore_ascii_case(if n.is_first_name { &p.first_name } else { &p.last_name }));
            pbp_ids.insert(n.audio_id);
            if n.tables.is_empty() {
                pbp_keyless.insert(n.audio_id);
            } else {
                assert_eq!(n.bank_resolved_distinct, n.sample_ids_distinct, "{}: pbp samples in the bank", n.name);
            }
        }
    }
    eprintln!("players {} announcer ids {} called {} | pbp name rows matched: players {} ids {} keyless {:?}", players.len(), ann_ids.len(), ann_called.len(), with_pbp_name, pbp_ids.len(), pbp_keyless);
    assert_eq!(ann_ids.len(), 5311, "distinct non-zero bio audio ids (research 5 311)");
    assert_eq!(ann_called.len(), 4464, "announcer playerid keys (research 4 464)");
    // Keyless = the research's three ids without a table, plus audio id 0 ("name in the catalogue,
    // no recording" — the same sentinel convention as the team ids).
    assert!(pbp_keyless.iter().all(|k| [0u64, 33465, 33469, 33484].contains(k)), "the keyless pbp ids are the research's three (+ 0): {pbp_keyless:?}");
    assert!(with_pbp_name * 2 > players.len(), "most roster players have a recorded first or last name ({with_pbp_name} / {})", players.len());
}

/// Phase 1.5 / 2.7 (21 Sep 2026): the context listing through the registry on the real NHL 15
/// tree — the research's worked example (2026-09-21 handoff 01 §2): record 1
/// `Cup_End_Cup_Presentation_NHL` on `trigger_CUP_ENDINGS` with the conditions
/// `cup_type_Stanley_Cup`, `+_Context_RUN_ONCE`, `+Group_Cup_Sequence_5_run_once` and one speech
/// item `CUP_END_CUP_NHL` with an immediate 0; record 52 `Playoff_End_Stanley_Cup_Presentation`
/// with two items (`CUP_END_CUP_NHL`, `SILENCE_HALF_SECOND`); the totals 3 866 / 153 / 1 980 /
/// 4 007 + 515; the query narrows; NHL 11 refuses with the v3.1 reason.
#[test]
#[ignore = "real Xbox 360 installs under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn context_listing_reproduces_the_research_worked_example_on_nhl15() {
    let root = r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15";
    assert!(Path::new(root).is_dir(), "{root} missing (fails, never skips)");
    let reg = Registry::default();
    let id = open(&reg, "xbox", "nhl15");
    let all = reg.context_catalogue(id, root, "", 5000).unwrap();
    assert_eq!((all.records, all.triggers, all.condition_names, all.speech_items, all.context_items), (3866, 153, 1980, 4007, 515));
    assert_eq!(all.contexts.len(), 3866);
    let r1 = all.contexts.iter().find(|c| c.name == "Cup_End_Cup_Presentation_NHL").expect("record 1");
    assert_eq!(r1.trigger.as_deref(), Some("trigger_CUP_ENDINGS"));
    assert_eq!(r1.conditions, vec!["cup_type_Stanley_Cup", "+_Context_RUN_ONCE", "+Group_Cup_Sequence_5_run_once"]);
    assert_eq!(r1.items.len(), 1);
    assert_eq!((r1.items[0].kind.as_str(), r1.items[0].name.as_str(), r1.items[0].params.as_slice()), ("speech", "CUP_END_CUP_NHL", &["0".to_string()][..]));
    let r52 = all.contexts.iter().find(|c| c.name == "Playoff_End_Stanley_Cup_Presentation").expect("record 52");
    assert_eq!(r52.items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["CUP_END_CUP_NHL", "SILENCE_HALF_SECOND"]);
    assert!(r52.conditions.iter().any(|c| c == "playoff_cup_type_Stanley_Cup"));
    let cup = reg.context_catalogue(id, root, "cup_type", 10).unwrap();
    assert!(cup.matching >= 2 && cup.matching < 3866 && cup.contexts.len() == 10 && cup.truncated, "{} / {}", cup.matching, cup.contexts.len());
    assert!(all.notes.iter().any(|n| n.contains("51 contexts have no trigger")), "{:?}", all.notes);
    // NHL 11 lists through the v3.1 reader (research 2026-09-21 handoff 16): 2 833 records, 2 796
    // speech items + 3 raised triggers (+ 34 other-module items), 11 702 conditions, 134 without
    // a trigger; trigger-slot parameters read as text.
    let root11 = r"X:\Games\Emulation\Roms\XBOX360\NHL 11\NHL 11";
    let id11 = open(&reg, "xbox", "nhl11");
    let c11 = reg.context_catalogue(id11, root11, "", 5000).unwrap();
    assert_eq!((c11.records, c11.speech_items, c11.context_items, c11.contexts.iter().map(|c| c.conditions.len()).sum::<usize>()), (2833, 2796, 3, 11702));
    assert!(c11.notes.iter().any(|n| n.contains("134 contexts have no trigger")), "{:?}", c11.notes);
    assert!(c11.contexts.iter().any(|c| c.items.iter().any(|i| i.kind == "other-module")), "the injury-story items are typed other-module");
    assert!(c11.contexts.iter().any(|c| c.items.iter().any(|i| i.params.iter().any(|p| p.starts_with("trigger slot")))), "slot parameters read as text");
}

/// P2 (22 Sep 2026) — the **PS4** context listing through the same registry seam and the same
/// view, on the real extracted NHL 21 and NHL 16 trees (research 2026-09-21 pm handoff 15). The
/// counts are the research census; a record worked through by hand checks the join end to end;
/// the query narrows; the PS4-specific note names the file version and what is not decoded.
/// This is the parity step that takes the Contexts card from gen7 11–Legacy to PS4 15–21.
#[test]
#[ignore = "real extracted PS4 trees under H:/Emulators/ShadPS4/Games (read only): select with --ignored"]
fn the_ps4_context_listing_reproduces_the_research_census_through_the_registry() {
    let reg = Registry::default();
    for (game, cusa, records, triggers_min, speech, ctxitems, conds, version) in [
        ("nhl21", "CUSA18182", 2153usize, 1usize, 2147usize, 479usize, 4313usize, 4u32),
        ("nhl16", "CUSA01909", 4396, 1, 4534, 650, 18565, 3),
    ] {
        let root = format!(r"H:\Emulators\ShadPS4\Games\{cusa}\databigs\Extracted");
        assert!(Path::new(&root).is_dir(), "{root} missing (fails, never skips)");
        let id = open_ps4(&reg, game);
        let all = reg.context_catalogue(id, &root, "", 20_000).unwrap_or_else(|e| panic!("{game}: {e}"));
        assert_eq!(all.platform, "ps4", "{game}");
        assert_eq!((all.records, all.speech_items, all.context_items, all.condition_names), (records, speech, ctxitems, conds), "{game}");
        assert!(all.triggers >= triggers_min && all.triggers < records, "{game}: {} distinct triggers", all.triggers);
        assert_eq!(all.contexts.len(), records, "{game}: every record is listed under the cap");
        assert!(all.notes.iter().any(|n| n.contains(&format!("version {version}"))), "{game}: {:?}", all.notes);
        assert!(all.notes.iter().any(|n| n.contains("not decoded")), "{game}: the undecoded parts are named");
        // Every listed record names itself and every item is typed by the MOIR namespace.
        assert!(all.contexts.iter().all(|c| !c.name.is_empty()), "{game}");
        assert!(all.contexts.iter().flat_map(|c| &c.items).all(|i| i.kind == "speech" || i.kind == "context"), "{game}: an item is untyped");
        // The query narrows and the cap truncates.
        let narrowed = reg.context_catalogue(id, &root, "cup", 10).unwrap();
        assert!(narrowed.matching > 0 && narrowed.matching < records, "{game}: `cup` matched {}", narrowed.matching);
        assert!(narrowed.contexts.len() <= 10, "{game}");
        if narrowed.matching > 10 {
            assert!(narrowed.truncated, "{game}");
        }
    }
    // The worked NHL 21 record: the same shape the gen7 card shows.
    let root = r"H:\Emulators\ShadPS4\Games\CUSA18182\databigs\Extracted";
    let id = open_ps4(&reg, "nhl21");
    let all = reg.context_catalogue(id, root, "CUP_ENDINGS_SEQ5_TROPHY_HOIST_EASTERN_CONFERENCE_NHL", 5).unwrap();
    let r = all.contexts.first().expect("the hoist record");
    assert_eq!(r.name, "CUP_ENDINGS_SEQ5_TROPHY_HOIST_EASTERN_CONFERENCE_NHL");
    assert_eq!(r.trigger.as_deref(), Some("trigger_CUP_ENDINGS"));
    assert_eq!(r.conditions.len(), 1, "{:?}", r.conditions);
    assert!(!r.items.is_empty() && r.items.iter().all(|i| !i.name.is_empty()));
}

/// A PS4 workspace on the committed anchor, for the registry seam.
fn open_ps4(reg: &Registry, game: &str) -> u32 {
    let db = repo(&format!("anchors/ps4/{game}/databases/nhl_game.db"));
    reg.open_workspace_from_paths(Some(game), studio_api::parse_platform("ps4").unwrap(), &s(&db), None, &s(&repo(&format!("anchors/ps4/{game}/databases/nhl_game-meta.xml"))), &s(&repo("defs")))
        .unwrap()
        .project_id
}
