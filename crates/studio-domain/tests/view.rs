//! Phase 3.1 — view-model foundation verified against the NHL Legacy anchor: strings read, enum
//! labels resolve, sign-decoding stays in the metadata's legal range.

use std::path::{Path, PathBuf};

use studio_domain::{
    arena_summaries, arena_view, career_stats, entity_fields, lines_view, player_summaries,
    player_view, probe_main_team_encoding, team_summaries, team_view, FieldKind, MainTeamEncoding,
};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;
use tdb_relations::{PlayerKind, Position, RosterModel};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

const DB: &str = "anchors/ps3/nhllegacy/databases/nhlng.db";
const META: &str = "anchors/ps3/nhllegacy/databases/nhlng-meta.xml";
const DEFS: &str = "defs";

fn fixtures() -> (Vec<u8>, FieldMetaIndex, EnumRegistry) {
    let bytes = std::fs::read(repo(DB)).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(META)).unwrap()).unwrap();
    let enums = EnumRegistry::from_dir(repo(DEFS).join("enums")).unwrap();
    (bytes, meta, enums)
}

/// Inline name strings read, the position field resolves to an enum label, and `firstname` is
/// reported editable (its metadata is updatable).
#[test]
fn ubpc_fields_decode_strings_and_enums() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let bio = db.table_canonical("ubPc").unwrap();
    let n = (bio.current_records as u32).min(300);

    let mut found_name = false;
    let mut found_pos_label = false;
    let mut checked_editable = false;

    for i in 0..n {
        let fields = entity_fields(&db, &meta, &enums, "ubPc", i);
        assert!(!fields.is_empty(), "ubPc record {i} has fields");

        if let Some(last) = fields.iter().find(|f| f.name == "lastname") {
            assert_eq!(last.kind, FieldKind::String);
            assert!(last.is_string);
            if !last.display.is_empty() {
                found_name = true;
            }
        }
        if let Some(pos) = fields.iter().find(|f| f.name == "position") {
            assert_eq!(pos.kind, FieldKind::Integer);
            if let Some(label) = &pos.enum_label {
                assert!(!label.is_empty(), "position enum label non-empty");
                found_pos_label = true;
            }
        }
        if let Some(first) = fields.iter().find(|f| f.name == "firstname") {
            assert!(first.editable, "firstname is updatable per metadata");
            checked_editable = true;
        }
    }

    assert!(found_name, "no non-empty lastname found in first {n} bios");
    assert!(found_pos_label, "position never produced an enum label");
    assert!(checked_editable, "firstname field was present and checked");
}

/// Every integer field with a declared range decodes within that range on vanilla anchor data —
/// proves sign/sentinel decoding (`FieldMeta::decode_raw`) against the metadata across a whole table.
#[test]
fn integer_fields_decode_within_declared_range() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();

    // The team table is small and fully populated — a tight, fast invariant check.
    let team = db.table_canonical("kOtt").unwrap();
    let n = team.current_records as u32;
    assert!(n > 0, "kOtt has records");

    for i in 0..n {
        for fv in entity_fields(&db, &meta, &enums, "kOtt", i) {
            if let (Some(d), Some(lo), Some(hi)) = (fv.decoded, fv.range_low, fv.range_high) {
                // Skip degenerate metadata ranges (rangelow==rangehigh==0 marks "unconstrained").
                if lo == 0 && hi == 0 {
                    continue;
                }
                assert!(
                    d >= lo && d <= hi,
                    "kOtt rec {i} field {} ({}) decoded {d} outside [{lo}, {hi}]",
                    fv.studio_field,
                    fv.name
                );
            }
        }
    }
}

/// The player master list covers every modelled player and carries readable names + position labels.
#[test]
fn player_summaries_cover_the_model() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    let summaries = player_summaries(&db, &meta, &enums, &model);
    assert_eq!(summaries.len(), model.players.len(), "one row per player");

    let named = summaries.iter().filter(|s| !s.last_name.is_empty()).count();
    assert!(named > summaries.len() / 2, "most players have a last name ({named}/{})", summaries.len());
    assert!(
        summaries.iter().any(|s| !s.position_label.is_empty()),
        "position labels resolve"
    );
}

/// The depth-chart summary fields resolve on anchor data: derived Overalls land on the display
/// scale for essentially everyone, archetype labels resolve, both active states occur, jerseys
/// read, and the NHL-rights decode stays inside the team id space.
#[test]
fn player_summaries_carry_depth_chart_fields() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    let summaries = player_summaries(&db, &meta, &enums, &model);
    let n = summaries.len();

    // Overall: the formula (skaters) / stored rating (goalies) must produce a plausible EA value
    // for nearly every player with a companion. Elite players must clear the first-liner tier —
    // an all-36 or all-zero read would fail this.
    let overalls: Vec<i64> = summaries.iter().filter_map(|s| s.overall).collect();
    assert!(overalls.len() > n * 9 / 10, "overalls resolve ({}/{n})", overalls.len());
    assert!(overalls.iter().all(|&o| (1..=99).contains(&o)), "overalls clamp to 1..99");
    let elite = overalls.iter().filter(|&&o| o >= 80).count();
    assert!(elite > 50, "the league has first-liners ({elite} rated 80+)");

    // Goalie Overall is the stored `overall` rating on the display (+36) scale, not the formula.
    let goalie = model
        .players
        .iter()
        .find(|p| p.kind == PlayerKind::Goalie && p.goalie_ai.is_some())
        .expect("anchor has goalies");
    let gs = summaries.iter().find(|s| s.player_id == goalie.player_id).unwrap();
    let stored = player_view(&db, &meta, &enums, &model, goalie.player_id)
        .unwrap()
        .ratings
        .iter()
        .find(|f| f.name == "c_overall")
        .and_then(|f| f.decoded)
        .unwrap();
    assert_eq!(gs.overall, Some(stored + 36), "goalie overall = stored eWUF + 36");

    // Archetypes, jerseys, active flag, rights.
    assert!(
        summaries.iter().filter(|s| s.style_label.is_some()).count() > n / 2,
        "style labels resolve for most players"
    );
    assert!(
        summaries.iter().filter(|s| s.jersey_number.is_some()).count() > n * 9 / 10,
        "jersey numbers read"
    );
    assert!(summaries.iter().any(|s| s.is_active == Some(true)), "active players exist");
    let team_count = model.teams.len() as u64;
    let rights: Vec<u64> = summaries.iter().filter_map(|s| s.rights_team_id).collect();
    assert!(!rights.is_empty(), "some players carry NHL rights");
    assert!(
        rights.iter().all(|&t| t < team_count),
        "rights decode into the canonical team id space"
    );
}

/// The team-subtab views (Strategies / Draft Picks / Retired Numbers / Jerseys / Equipment) all
/// resolve on anchor data with the shapes the reference decode predicts.
#[test]
fn team_tab_views_resolve_on_anchor() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let team_id = 0u64; // Anaheim in the Legacy anchor — a stock NHL franchise.

    // Strategies: 12 tactics + 24 forward biases + 18 defense biases + 6 team biases, with the
    // tactic vocabularies resolving through the enum registry.
    let strat = studio_domain::team_strategies(&db, &meta, &enums, team_id)
        .expect("EeYo row for team 0");
    assert_eq!(strat.tactics.len(), 12);
    assert_eq!(strat.forward_biases.len(), 24);
    assert_eq!(strat.defense_biases.len(), 18);
    assert_eq!(strat.team_biases.len(), 6);
    let forecheck = strat
        .tactics
        .iter()
        .find(|c| c.field.studio_field == "RBGh")
        .unwrap();
    assert_eq!(forecheck.field.enum_options.len(), 4, "forecheck vocab bound");
    assert!(forecheck.field.enum_label.is_some(), "stored value is a member");

    // Draft picks: the stock grid is 6 years × 7 rounds per team, minus/plus real trades (the
    // anchor ships the 2015 trade ledger — team 0 owns 38). Traded picks resolve their origin.
    let picks = studio_domain::team_draft_picks(&db, &meta, &model, team_id);
    assert!(
        (30..=50).contains(&picks.len()),
        "a stock team owns roughly 6×7 picks ({})",
        picks.len()
    );
    assert!(picks.iter().all(|p| (1..=7).contains(&p.round)));
    assert!(picks.windows(2).all(|w| (w[0].year, w[0].round) <= (w[1].year, w[1].round)));
    assert!(
        picks
            .iter()
            .filter(|p| p.via_trade)
            .all(|p| p.original_owner_name.is_some()),
        "traded picks resolve the origin team's name"
    );

    // Retired numbers: #99 is retired league-wide (one wcSL row per franchise).
    let team = team_view(&db, &meta, &enums, &model, team_id).unwrap();
    assert!(team.retired_numbers.contains(&99), "Gretzky's 99 is retired");

    // Jerseys: every NHL team registers at least a home + away; zones decode 9-wide.
    let jerseys = studio_domain::team_jerseys(&db, team_id);
    assert!(jerseys.len() >= 2, "team 0 has jerseys ({})", jerseys.len());
    let equipment = studio_domain::team_equipment(&db, team_id);
    assert!(!equipment.is_empty(), "team 0 has stock equipment");
    assert!(equipment.iter().all(|e| e.zones.len() == 9));

    // Lines: the slot catalog carries the named situational flags with their studio ids.
    let lines = lines_view(&db, &meta, &enums, &model, team_id).unwrap();
    assert!(lines.slot_fields.len() >= 70, "full slot catalog ({})", lines.slot_fields.len());
    for name in ["l1c", "l4rw", "pp4_2ld", "pk3_1rd", "ot_3c", "s5", "x2", "g1"] {
        assert!(
            lines.slot_fields.iter().any(|s| s.name == name),
            "slot catalog carries {name}"
        );
    }
    assert!(
        !lines.slot_fields.iter().any(|s| s.name == "rosterstatus"),
        "the 1-bit rosterstatus flag is not a situational slot"
    );
    // Entries' set slots reference catalog studio ids.
    let ids: std::collections::HashSet<&str> =
        lines.slot_fields.iter().map(|s| s.studio_field.as_str()).collect();
    assert!(lines
        .entries
        .iter()
        .flat_map(|e| e.slots.iter())
        .all(|f| ids.contains(f.as_str())));
}

/// The Player detail assembles curated identity, the kind-correct ratings companion, memberships,
/// and the full bio field list.
#[test]
fn player_view_assembles_detail() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    // A skater that has a roster slot — exercises ratings + memberships.
    let pid = model
        .slots
        .iter()
        .filter_map(|s| s.player_id)
        .find(|&id| model.player(id).is_some_and(|p| p.skater_ai.is_some()))
        .expect("a slotted skater exists");

    let view = player_view(&db, &meta, &enums, &model, pid).expect("player resolves");
    assert_eq!(view.kind, "skater");
    assert!(view.identity.iter().any(|f| f.name == "position"));
    assert!(view.identity.iter().any(|f| f.name == "firstname"));
    assert!(!view.ratings.is_empty(), "skater has rating fields (dSvy)");
    assert!(!view.all_fields.is_empty(), "bio all-fields populated");
    assert!(!view.memberships.is_empty(), "slotted player has a membership");

    assert!(player_view(&db, &meta, &enums, &model, 0xFFFF_FFFF).is_none());
}

/// Player Style (`dSvy.QgFs`) locks its vocabulary by position: defensemen see the defenseman
/// roles (raw 1..=4), forwards the forward roles (raw 5..=10) — never the other sub-range, except
/// (at most) the player's own stored value.
#[test]
fn player_style_options_lock_by_position() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    let mut checked_defense = false;
    let mut checked_forward = false;
    for p in &model.players {
        if matches!(p.kind, PlayerKind::Goalie) {
            continue;
        }
        let defense = p.position == Position::Defense;
        if if defense { checked_defense } else { checked_forward } {
            continue;
        }
        let view = player_view(&db, &meta, &enums, &model, p.player_id).unwrap();
        let Some(style) = view.ratings.iter().find(|f| f.studio_field == "QgFs") else {
            continue; // broken cluster without a dSvy companion
        };
        let eligible = if defense { 1..=4 } else { 5..=10 };
        let raws: Vec<i64> = style.enum_options.iter().map(|(r, _)| *r).collect();
        for r in eligible.clone() {
            assert!(raws.contains(&r), "eligible style {r} missing for {}", view.header.full_name);
        }
        for r in &raws {
            assert!(
                eligible.contains(r) || Some(*r) == style.decoded,
                "ineligible style option {r} offered to {}",
                view.header.full_name
            );
        }
        if defense {
            checked_defense = true;
        } else {
            checked_forward = true;
        }
        if checked_defense && checked_forward {
            break;
        }
    }
    assert!(checked_defense && checked_forward, "anchor yields both a defenseman and a forward");
}

/// Team summaries cover the model with readable names; the detail resolves identity, arena, roster.
#[test]
fn team_views_resolve_identity_and_arena() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    let teams = team_summaries(&db, &meta, &enums, &model);
    assert_eq!(teams.len(), model.teams.len());
    assert!(teams.iter().any(|t| !t.name.is_empty()), "teams have names");
    assert!(teams.iter().any(|t| !t.abbrev.is_empty()), "teams have abbreviations");
    assert!(
        teams.iter().any(|t| t.league_label.as_deref() == Some("NHL")),
        "league labels resolve"
    );

    let view = team_view(&db, &meta, &enums, &model, teams[0].team_id).expect("team resolves");
    assert!(view.identity.iter().any(|f| f.name == "fullname"));
    assert_eq!(view.roster_count, model.team_roster(teams[0].team_id).len());
    // At least one team in the league resolves its home arena via kOtt.buDa -> StEO.vfEq.
    assert!(
        teams
            .iter()
            .filter_map(|t| team_view(&db, &meta, &enums, &model, t.team_id))
            .any(|v| v.arena.is_some()),
        "at least one team resolves an arena"
    );

    assert!(team_view(&db, &meta, &enums, &model, 0xFFFF_FFFF).is_none());
}

/// Arena summaries list the rinks; the detail resolves identity (name/city).
#[test]
fn arena_views_resolve() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();

    let arenas = arena_summaries(&db, &meta);
    assert!(!arenas.is_empty(), "anchor has arenas");
    assert!(arenas.iter().any(|a| !a.name.is_empty()), "arenas have names");

    let view = arena_view(&db, &meta, &enums, arenas[0].arena_id).expect("arena resolves");
    assert!(view.identity.iter().any(|f| f.name == "arenaname"));
    assert!(!view.all_fields.is_empty());
}

/// The depth chart lists a team's roster with resolved names and situational slot labels.
#[test]
fn lines_view_builds_depth_chart() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    // A team that actually has a roster.
    let team_id = model
        .teams
        .iter()
        .map(|t| t.team_id)
        .find(|&id| !model.team_roster(id).is_empty())
        .expect("a team with a roster");

    let lines = lines_view(&db, &meta, &enums, &model, team_id).expect("lines resolve");
    assert_eq!(lines.entries.len(), model.team_roster(team_id).len());
    assert!(lines.entries.iter().any(|e| !e.name.is_empty()), "names resolve");
    // Situational flag labels are metadata names (e.g. line/pp/pk slots) when set.
    assert!(
        lines.entries.iter().flat_map(|e| &e.slots).any(|s| !s.is_empty()),
        "at least one roster entry has a labelled situational slot"
    );

    assert!(lines_view(&db, &meta, &enums, &model, 0xFFFF_FFFF).is_none());
}

/// Find a player id by name in the anchor.
fn player_id_by_name(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    model: &RosterModel,
    first: &str,
    last: &str,
) -> u64 {
    player_summaries(db, meta, enums, model)
        .iter()
        .find(|p| p.first_name == first && p.last_name == last)
        .unwrap_or_else(|| panic!("{first} {last} not found in the anchor"))
        .player_id
}

/// A cell's display string by column label in a career section row.
fn cell_display(section: &studio_domain::CareerSection, row: &studio_domain::CareerRow, label: &str) -> String {
    let idx = section
        .columns
        .iter()
        .position(|c| c.label == label)
        .unwrap_or_else(|| panic!("no {label} column"));
    row.cells[idx]
        .as_ref()
        .map(|f| f.display.clone())
        .unwrap_or_default()
}

/// Career stats join by the 1-indexed `wBIz` and decode to a real, known stat line: Sidney
/// Crosby's 2006-07 season in the NHL Legacy stock data (79 GP, 36 G, 84 A, 120 PTS, +10) —
/// this pins the `player_id + 1` join, the `year + 1900` season label, and the offset-binary
/// plus/minus in one check against real data.
#[test]
fn career_stats_decode_crosby_2006_07() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let pid = player_id_by_name(&db, &meta, &enums, &model, "Sidney", "Crosby");

    let cs = career_stats(&db, &meta, &enums, pid);
    assert_eq!(cs.sections.len(), 4, "four career sections");
    let season = cs.sections.iter().find(|s| s.kind == "skater_season").unwrap();
    assert!(!season.columns.is_empty(), "skater season columns resolve");
    assert!(
        season.rows.len() >= 8,
        "Crosby has a career's worth of season rows (got {})",
        season.rows.len()
    );
    // Newest-first ordering.
    assert!(
        season.rows.first().unwrap().season_label >= season.rows.last().unwrap().season_label,
        "rows sort newest-first"
    );

    let rookie = season
        .rows
        .iter()
        .find(|r| r.season_label == "2006-07")
        .expect("Crosby's 2006-07 season row");
    assert_eq!(rookie.league_label, "NHL");
    assert_eq!(cell_display(season, rookie, "GP"), "79");
    assert_eq!(cell_display(season, rookie, "G"), "36");
    assert_eq!(cell_display(season, rookie, "A"), "84");
    assert_eq!(cell_display(season, rookie, "PTS"), "120");
    assert_eq!(cell_display(season, rookie, "+/-"), "+10");
}

/// The goalie sections decode the fixed-point SV% (×0.001) and GAA (×0.01) columns: Carey Price's
/// 2014-15 season (44 W, .933, 1.96) in the NHL Legacy stock data.
#[test]
fn career_stats_decode_price_2014_15() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let pid = player_id_by_name(&db, &meta, &enums, &model, "Carey", "Price");

    let cs = career_stats(&db, &meta, &enums, pid);
    let season = cs.sections.iter().find(|s| s.kind == "goalie_season").unwrap();
    assert!(!season.rows.is_empty(), "Price has goalie season rows");
    let y1415 = season
        .rows
        .iter()
        .find(|r| r.season_label == "2014-15")
        .expect("Price's 2014-15 season row");
    assert_eq!(cell_display(season, y1415, "W"), "44");
    assert_eq!(cell_display(season, y1415, "SV%"), ".933");
    assert_eq!(cell_display(season, y1415, "GAA"), "1.96");
}

/// Structural sanity across many players: rows are rectangular, the league flag resolves to its
/// vocabulary, plus/minus decodes within its legal (offset) range, and season labels are sane.
#[test]
fn career_stats_shape_across_players() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    let mut rows_seen = 0usize;
    for p in model.players.iter().take(2000) {
        let cs = career_stats(&db, &meta, &enums, p.player_id);
        for s in &cs.sections {
            for r in &s.rows {
                rows_seen += 1;
                assert_eq!(r.cells.len(), s.columns.len(), "rectangular ({})", s.kind);
                assert!(
                    r.league_label == "NHL" || r.league_label == "AHL",
                    "league resolves (got {:?})",
                    r.league_label
                );
                assert!(!r.team.is_empty(), "team code non-empty");
                if let Some(idx) = s.columns.iter().position(|c| c.label == "+/-") {
                    let d = r.cells[idx].as_ref().and_then(|f| f.decoded).unwrap();
                    assert!((-64..=127).contains(&d), "+/- {d} in legal range");
                }
            }
        }
    }
    assert!(rows_seen > 1000, "the anchor carries plenty of career rows ({rows_seen})");
}

/// The `dXSB` "main team" pointer is 1-indexed on the PS3 NHL Legacy anchor (raw = team id + 1,
/// 0 = free agent) — the probe recovers that from the roster ground truth. Guards the offset used
/// by the Player banner and the Main-Team picker; NHL 09/10 (0-indexed) are verified against their
/// external installs (not committed anchors). Every rostered player's resolved main team is also a
/// team they actually fill a slot on — the header never points at a team the player doesn't play for.
#[test]
fn main_team_encoding_is_one_indexed_on_legacy_anchor() {
    let (bytes, meta, enums) = fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    assert_eq!(
        probe_main_team_encoding(&db, &model),
        MainTeamEncoding::OneIndexed,
        "Legacy stores dXSB as team_id + 1"
    );

    // Spot-check the resolved header against roster ground truth for the first 500 rostered players.
    let mut checked = 0usize;
    for p in model.players.iter() {
        let memberships: Vec<u64> = model
            .slots
            .iter()
            .filter(|s| s.player_id == Some(p.player_id))
            .map(|s| s.team_id)
            .collect();
        if memberships.is_empty() {
            continue;
        }
        let pv = player_view(&db, &meta, &enums, &model, p.player_id).unwrap();
        if let Some(primary) = pv.header.primary_team_id {
            assert!(
                memberships.contains(&primary),
                "player {} main team {primary} must be a real membership {memberships:?}",
                p.player_id
            );
        }
        checked += 1;
        if checked >= 500 {
            break;
        }
    }
    assert!(checked >= 400, "exercised a healthy sample of rostered players ({checked})");
}

const NHL07_DB: &str = "anchors/xbox360/nhl07/databases/NHL07NG.db";
const NHL07_META: &str = "anchors/xbox360/nhl07/databases/NHL07NG-meta.xml";

fn nhl07_fixtures() -> (Vec<u8>, FieldMetaIndex, EnumRegistry) {
    let bytes = std::fs::read(repo(NHL07_DB)).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(NHL07_META)).unwrap()).unwrap();
    let enums = EnumRegistry::from_dir(repo(DEFS).join("enums")).unwrap();
    (bytes, meta, enums)
}

/// NHL07's `kOtt` (`exhibitionteams`) is a thin index that carries no name fields; team names live in
/// `stockteamtable`, reached via the `(table, teamid)` join. `team_summaries` must follow that join
/// so every team resolves to a real, human-readable name — not the empty string the fat-schema reader
/// produced. Verified team ids from the vanilla anchor (raw dXSB = team id, offset 0).
#[test]
fn nhl07_team_names_resolve_via_thin_index_join() {
    let (bytes, meta, enums) = nhl07_fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    let teams = team_summaries(&db, &meta, &enums, &model);
    assert_eq!(teams.len(), 93);
    assert!(
        teams.iter().all(|t| !t.name.trim().is_empty()),
        "every NHL07 team must resolve a name via the stock/custom join"
    );
    let by_id = |id: u64| teams.iter().find(|t| t.team_id == id).map(|t| t.name.as_str());
    assert_eq!(by_id(0), Some("Anaheim Ducks\u{2122}"));
    assert_eq!(by_id(9), Some("Dallas Stars\u{2122}"));

    // Team detail identity is sourced from the same join (not the thin kOtt row).
    let view = team_view(&db, &meta, &enums, &model, 2).unwrap();
    assert!(
        view.identity.iter().any(|f| f.display.contains("Boston")),
        "team_view identity should carry the resolved full name: {:?}",
        view.identity.iter().map(|f| &f.display).collect::<Vec<_>>()
    );
}

/// NHL07 stores the birth year in a 6-bit field with base 1959 (not the 8-bit, base-1900 encoding of
/// NHL 09→21) and month/day already 1-indexed. The width-aware base + the birthdate probe must
/// compose the correct calendar date through the real player-view path. Daniel Alfredsson (wBIz=232)
/// was born 1972-12-11.
#[test]
fn nhl07_birthdate_uses_probed_base_and_offset() {
    let (bytes, meta, enums) = nhl07_fixtures();
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    let pv = player_view(&db, &meta, &enums, &model, 232).unwrap();
    assert_eq!(pv.header.full_name.trim(), "Daniel Alfredsson");
    assert_eq!(pv.header.birthdate.as_deref(), Some("1972-12-11"));
}
