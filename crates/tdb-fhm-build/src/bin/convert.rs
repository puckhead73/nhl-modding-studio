//! `tdb-fhm-convert` — convert the prior app's normalized `historical.db` into our person-centric
//! corpus schema. The prior DB holds the same FHM data (name+DOB slug identity) in a differently-
//! shaped, more complete form (more rated players + richer tables), so this reads its tables and
//! emits our schema — leaving `tdb-fhm-store`, the formula engine, and the UI unchanged.
//!
//! Usage: `tdb-fhm-convert <prior historical.db> <out.db>`
//!
//! Mapping (prior → ours): `players`→`person` (bio from newest `season_players`), `season_players`→
//! `player_season`, `season_player_ratings` grades + `season_player_raw_ratings` (via `rating_columns`)
//! → `ratings_season` (grades injected into `attrs_json`), `season_player_career_stats`→`career_stat`,
//! `season_teams`+`teams`/`leagues`→`team_season`/`league_season`, contracts/draft/awards likewise.
//! Careers/draft/contract/awards are sourced once per person from their newest season (like the CSV
//! ingest). Positions use the same "highest grade (20) wins; ties G>C>LW>RW>D" rule + goalie fallback.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::{Path, PathBuf};

use csv::StringRecord;
use rusqlite::{params, Connection};

use tdb_fhm::{person_slug, DRAFT_TEAM_NAME_SQL, SCHEMA_SQL, SCHEMA_VERSION};

type Res<T> = Result<T, Box<dyn Error>>;

struct Person {
    slug: String,
    first: String,
    last: String,
    nick: String,
    dob: String,
    birth_year: Option<i64>,
    height: Option<i64>,
    weight: Option<i64>,
    hand: String,
    nationality: String,
    birthcity: String,
    birthstate: String,
    retired: bool,
    is_goalie: bool,
    position: String,
    first_year: i64,
    last_year: i64,
    season_count: i64,
    bio_done: bool,
}

/// Per-(season,player) ratings assembled from the prior DB's grade + raw-rating tables.
struct Ratings {
    g: i64,
    ld: i64,
    rd: i64,
    lw: i64,
    c: i64,
    rw: i64,
    ability: Option<f64>,
    potential: Option<f64>,
    attrs_json: String,
    is_goalie: bool,
    position: String,
}

// ---------------------------------------------------------------------------
// NHL franchise lineage — ported from the Hockey-Legends `seed_franchise_lineage.py` and mapped
// onto our `team_key`s (validated: 0 cross-franchise season collisions). See `schema::franchise`.
// ---------------------------------------------------------------------------

/// Canonical franchises: `(name, display_name, abbr, is_defunct, notes)`.
const FRANCHISES: &[(&str, &str, &str, i64, &str)] = &[
    ("Anaheim Ducks", "Anaheim Ducks", "ANA", 0, ""),
    ("Boston Bruins", "Boston Bruins", "BOS", 0, ""),
    ("Buffalo Sabres", "Buffalo Sabres", "BUF", 0, ""),
    (
        "Calgary Flames org",
        "Calgary Flames",
        "CGY",
        0,
        "Includes Atlanta Flames",
    ),
    (
        "Carolina Hurricanes org",
        "Carolina Hurricanes",
        "CAR",
        0,
        "Includes Hartford Whalers (WHA+NHL)",
    ),
    (
        "Chicago Blackhawks",
        "Chicago Blackhawks",
        "CHI",
        0,
        "Includes Black Hawks era",
    ),
    (
        "Colorado Avalanche org",
        "Colorado Avalanche",
        "COL",
        0,
        "Includes Quebec Nordiques (WHA+NHL)",
    ),
    (
        "Columbus Blue Jackets",
        "Columbus Blue Jackets",
        "CBJ",
        0,
        "",
    ),
    (
        "Dallas Stars org",
        "Dallas Stars",
        "DAL",
        0,
        "Includes Seals, Barons, Minnesota North Stars",
    ),
    (
        "Detroit Red Wings",
        "Detroit Red Wings",
        "DET",
        0,
        "Includes Cougars and Falcons eras",
    ),
    (
        "Edmonton Oilers",
        "Edmonton Oilers",
        "EDM",
        0,
        "Includes WHA era",
    ),
    ("Florida Panthers", "Florida Panthers", "FLA", 0, ""),
    ("Los Angeles Kings", "Los Angeles Kings", "LAK", 0, ""),
    ("Minnesota Wild", "Minnesota Wild", "MIN", 0, ""),
    ("Montreal Canadiens", "Montreal Canadiens", "MTL", 0, ""),
    ("Nashville Predators", "Nashville Predators", "NSH", 0, ""),
    (
        "New Jersey Devils org",
        "New Jersey Devils",
        "NJD",
        0,
        "Includes Kansas City Scouts, Colorado Rockies",
    ),
    ("New York Islanders", "New York Islanders", "NYI", 0, ""),
    ("New York Rangers", "New York Rangers", "NYR", 0, ""),
    (
        "Ottawa Senators org",
        "Ottawa Senators",
        "OTT",
        0,
        "Includes original Senators, St. Louis Eagles",
    ),
    ("Philadelphia Flyers", "Philadelphia Flyers", "PHI", 0, ""),
    ("Pittsburgh Penguins", "Pittsburgh Penguins", "PIT", 0, ""),
    ("San Jose Sharks", "San Jose Sharks", "SJS", 0, ""),
    ("Seattle Kraken", "Seattle Kraken", "SEA", 0, ""),
    ("St. Louis Blues", "St. Louis Blues", "STL", 0, ""),
    ("Tampa Bay Lightning", "Tampa Bay Lightning", "TBL", 0, ""),
    (
        "Toronto Maple Leafs",
        "Toronto Maple Leafs",
        "TOR",
        0,
        "Includes Arenas and St. Patricks eras",
    ),
    ("Utah Mammoth", "Utah Mammoth", "UTA", 0, ""),
    ("Vancouver Canucks", "Vancouver Canucks", "VAN", 0, ""),
    ("Vegas Golden Knights", "Vegas Golden Knights", "VGK", 0, ""),
    ("Washington Capitals", "Washington Capitals", "WSH", 0, ""),
    (
        "Winnipeg Jets org",
        "Winnipeg Jets",
        "WPG",
        0,
        "Includes Atlanta Thrashers",
    ),
    (
        "Arizona Coyotes org",
        "Arizona Coyotes",
        "ARI",
        1,
        "Winnipeg Jets (WHA+NHL) -> Phoenix -> Arizona",
    ),
    (
        "Montreal Wanderers",
        "Montreal Wanderers",
        "MWN",
        1,
        "1917-18",
    ),
    ("Montreal Maroons", "Montreal Maroons", "MMR", 1, "1924-38"),
    (
        "Brooklyn Americans org",
        "Brooklyn Americans",
        "BRA",
        1,
        "Quebec Bulldogs -> Hamilton -> NY/Brooklyn Americans",
    ),
    (
        "Philadelphia Quakers org",
        "Philadelphia Quakers",
        "PHQ",
        1,
        "Pittsburgh Pirates -> Philadelphia Quakers",
    ),
];

/// `(franchise name, team_key, era_start (exclusive), era_end (inclusive; 0 = open), era_name)`.
const FRANCHISE_MEMBERS: &[(&str, &str, i64, i64, &str)] = &[
    (
        "Calgary Flames org",
        "nhl__atl",
        1972,
        1980,
        "Atlanta Flames",
    ),
    ("Calgary Flames org", "nhl__cgy", 1980, 0, "Calgary Flames"),
    (
        "Carolina Hurricanes org",
        "wha__new",
        1972,
        1979,
        "New England Whalers",
    ),
    (
        "Carolina Hurricanes org",
        "nhl__har",
        1979,
        1997,
        "Hartford Whalers",
    ),
    (
        "Carolina Hurricanes org",
        "nhl__car",
        1997,
        0,
        "Carolina Hurricanes",
    ),
    (
        "Colorado Avalanche org",
        "wha__que",
        1972,
        1979,
        "Quebec Nordiques (WHA)",
    ),
    (
        "Colorado Avalanche org",
        "nhl__que",
        1979,
        1995,
        "Quebec Nordiques",
    ),
    (
        "Colorado Avalanche org",
        "nhl__col",
        1995,
        0,
        "Colorado Avalanche",
    ),
    ("Dallas Stars org", "nhl__oak", 1967, 1970, "Oakland Seals"),
    (
        "Dallas Stars org",
        "nhl__cal",
        1970,
        1976,
        "California Golden Seals",
    ),
    (
        "Dallas Stars org",
        "nhl__cle",
        1976,
        1978,
        "Cleveland Barons",
    ),
    (
        "Dallas Stars org",
        "nhl__min",
        1967,
        1993,
        "Minnesota North Stars",
    ),
    ("Dallas Stars org", "nhl__dal", 1993, 0, "Dallas Stars"),
    (
        "New Jersey Devils org",
        "nhl__kcs",
        1974,
        1976,
        "Kansas City Scouts",
    ),
    (
        "New Jersey Devils org",
        "nhl__col",
        1976,
        1982,
        "Colorado Rockies",
    ),
    (
        "New Jersey Devils org",
        "nhl__njd",
        1982,
        0,
        "New Jersey Devils",
    ),
    (
        "Ottawa Senators org",
        "nha__ott",
        1900,
        1917,
        "Ottawa Senators (original)",
    ),
    (
        "Ottawa Senators org",
        "nhl__ott",
        1917,
        1934,
        "Ottawa Senators (original)",
    ),
    (
        "Ottawa Senators org",
        "nhl__stl",
        1934,
        1935,
        "St. Louis Eagles",
    ),
    (
        "Ottawa Senators org",
        "nhl__ott",
        1991,
        0,
        "Ottawa Senators",
    ),
    (
        "Winnipeg Jets org",
        "nhl__atl",
        1999,
        2011,
        "Atlanta Thrashers",
    ),
    ("Winnipeg Jets org", "nhl__wpg", 2011, 0, "Winnipeg Jets"),
    (
        "Arizona Coyotes org",
        "wha__wpg",
        1972,
        1979,
        "Winnipeg Jets (WHA)",
    ),
    (
        "Arizona Coyotes org",
        "nhl__wpg",
        1979,
        1996,
        "Winnipeg Jets (original)",
    ),
    (
        "Arizona Coyotes org",
        "nhl__phx",
        1996,
        2014,
        "Phoenix Coyotes",
    ),
    (
        "Arizona Coyotes org",
        "nhl__arz",
        2014,
        2024,
        "Arizona Coyotes",
    ),
    (
        "Brooklyn Americans org",
        "nha__que",
        1900,
        1920,
        "Quebec Bulldogs",
    ),
    (
        "Brooklyn Americans org",
        "nhl__que",
        1919,
        1920,
        "Quebec Bulldogs",
    ),
    (
        "Brooklyn Americans org",
        "nhl__ham",
        1920,
        1925,
        "Hamilton Tigers",
    ),
    (
        "Brooklyn Americans org",
        "nhl__nya",
        1925,
        1941,
        "New York Americans",
    ),
    (
        "Brooklyn Americans org",
        "nhl__bkn",
        1941,
        1942,
        "Brooklyn Americans",
    ),
    (
        "Philadelphia Quakers org",
        "nhl__pit",
        1925,
        1930,
        "Pittsburgh Pirates",
    ),
    (
        "Philadelphia Quakers org",
        "nhl__phi",
        1930,
        1931,
        "Philadelphia Quakers",
    ),
    (
        "Chicago Blackhawks",
        "nhl__chi",
        1900,
        1986,
        "Chicago Black Hawks",
    ),
    (
        "Chicago Blackhawks",
        "nhl__chi",
        1986,
        0,
        "Chicago Blackhawks",
    ),
    (
        "Detroit Red Wings",
        "nhl__det",
        1926,
        1930,
        "Detroit Cougars",
    ),
    (
        "Detroit Red Wings",
        "nhl__det",
        1930,
        1932,
        "Detroit Falcons",
    ),
    (
        "Detroit Red Wings",
        "nhl__det",
        1932,
        0,
        "Detroit Red Wings",
    ),
    (
        "Montreal Wanderers",
        "nha__mtw",
        1900,
        1917,
        "Montreal Wanderers",
    ),
    (
        "Montreal Wanderers",
        "nhl__mtw",
        1917,
        1918,
        "Montreal Wanderers",
    ),
    (
        "Montreal Maroons",
        "nhl__mtm",
        1924,
        1938,
        "Montreal Maroons",
    ),
    ("Anaheim Ducks", "nhl__ana", 1993, 0, "Anaheim Ducks"),
    ("Boston Bruins", "nhl__bos", 1924, 0, "Boston Bruins"),
    ("Buffalo Sabres", "nhl__buf", 1970, 0, "Buffalo Sabres"),
    (
        "Columbus Blue Jackets",
        "nhl__cbj",
        2000,
        0,
        "Columbus Blue Jackets",
    ),
    (
        "Columbus Blue Jackets",
        "nhl__cbs",
        2000,
        0,
        "Columbus Blue Jackets",
    ),
    (
        "Columbus Blue Jackets",
        "nhl__columbus",
        2000,
        0,
        "Columbus Blue Jackets",
    ),
    (
        "Edmonton Oilers",
        "wha__edm",
        1972,
        1979,
        "Edmonton Oilers (WHA)",
    ),
    ("Edmonton Oilers", "nhl__edm", 1979, 0, "Edmonton Oilers"),
    ("Florida Panthers", "nhl__fla", 1993, 0, "Florida Panthers"),
    (
        "Los Angeles Kings",
        "nhl__lak",
        1967,
        0,
        "Los Angeles Kings",
    ),
    ("Minnesota Wild", "nhl__min", 2000, 0, "Minnesota Wild"),
    (
        "Montreal Canadiens",
        "nha__mtl",
        1900,
        1917,
        "Montreal Canadiens",
    ),
    (
        "Montreal Canadiens",
        "nhl__mtl",
        1917,
        0,
        "Montreal Canadiens",
    ),
    (
        "Nashville Predators",
        "nhl__nas",
        1998,
        0,
        "Nashville Predators",
    ),
    (
        "Nashville Predators",
        "nhl__nsh",
        1998,
        0,
        "Nashville Predators",
    ),
    (
        "New York Islanders",
        "nhl__nyi",
        1972,
        0,
        "New York Islanders",
    ),
    ("New York Rangers", "nhl__nyr", 1926, 0, "New York Rangers"),
    (
        "Philadelphia Flyers",
        "nhl__phi",
        1967,
        0,
        "Philadelphia Flyers",
    ),
    (
        "Pittsburgh Penguins",
        "nhl__pit",
        1967,
        0,
        "Pittsburgh Penguins",
    ),
    ("San Jose Sharks", "nhl__sjs", 1991, 0, "San Jose Sharks"),
    ("Seattle Kraken", "nhl__sea", 2021, 0, "Seattle Kraken"),
    ("St. Louis Blues", "nhl__stl", 1967, 0, "St. Louis Blues"),
    (
        "Tampa Bay Lightning",
        "nhl__tbl",
        1992,
        0,
        "Tampa Bay Lightning",
    ),
    (
        "Toronto Maple Leafs",
        "nhl__tor",
        1917,
        0,
        "Toronto Maple Leafs",
    ),
    ("Utah Mammoth", "nhl__uta", 2024, 0, "Utah Mammoth"),
    (
        "Vancouver Canucks",
        "nhl__van",
        1970,
        0,
        "Vancouver Canucks",
    ),
    (
        "Vegas Golden Knights",
        "nhl__vgk",
        2017,
        0,
        "Vegas Golden Knights",
    ),
    (
        "Washington Capitals",
        "nhl__was",
        1974,
        0,
        "Washington Capitals",
    ),
];

fn main() -> Res<()> {
    let mut args = std::env::args().skip(1);
    let src_path = args
        .next()
        .ok_or("usage: tdb-fhm-convert <historical.db> <out.db> [fhm-export-root]")?;
    let out_path = args
        .next()
        .ok_or("usage: tdb-fhm-convert <historical.db> <out.db> [fhm-export-root]")?;
    // Optional: the FHM season-export root. The prior DB collapses each player to a single draft pick
    // (often a junior/import one), so when the CSV root is given we source drafts from the authoritative
    // per-season `draft_info.csv` instead — every league draft, joined to our persons by name+DOB slug.
    let fhm_root = args.next();

    let src = Connection::open_with_flags(&src_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    if std::path::Path::new(&out_path).exists() {
        std::fs::remove_file(&out_path).map_err(|e| {
            format!("cannot overwrite {out_path}: {e} — close the app if it has this corpus open")
        })?;
    }
    let mut out = Connection::open(&out_path)?;
    out.execute_batch("PRAGMA foreign_keys = OFF;")?;
    out.execute_batch(SCHEMA_SQL)?;

    // ---- global lookups -------------------------------------------------
    // seasons: season_id -> end_year, newest first
    let mut seasons: Vec<(i64, String)> = src
        .prepare("SELECT end_year, season_id FROM seasons")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<_, _>>()?;
    seasons.sort_by_key(|s| std::cmp::Reverse(s.0));
    if seasons.is_empty() {
        return Err("prior DB has no seasons".into());
    }

    // rating_columns: id -> name
    let rating_cols: HashMap<i64, String> = src
        .prepare("SELECT column_id, name FROM rating_columns")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<_, _>>()?;

    // teams: team_key -> (name, nickname, abbr, league_key)
    let teams: HashMap<String, (String, String, String, String)> = src
        .prepare("SELECT team_key, name, nickname, abbreviation, league_key FROM teams")?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                (
                    r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    r.get::<_, Option<String>>(4)?.unwrap_or_default(),
                ),
            ))
        })?
        .collect::<Result<_, _>>()?;
    // leagues: league_key -> (name, abbr)
    let leagues: HashMap<String, (String, String)> = src
        .prepare("SELECT league_key, name, abbreviation FROM leagues")?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                (
                    r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                ),
            ))
        })?
        .collect::<Result<_, _>>()?;

    // players -> assign person_id + canonical name/dob/retired/nationality
    let mut persons: Vec<Person> = Vec::new();
    let mut slug_to_id: HashMap<String, i64> = HashMap::new();
    // Parallel index keyed by OUR person_slug (name+DOB), so FHM-CSV draft rows — which carry no prior
    // -app slug — can be matched back to these persons. Built from the same name/dob we store.
    let mut our_slug_to_id: HashMap<String, i64> = HashMap::new();
    {
        let mut stmt = src.prepare(
            "SELECT slug, first_name, last_name, date_of_birth, is_retired, latest_nationality FROM players",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<i64>>(4)?.unwrap_or(0),
                r.get::<_, Option<String>>(5)?.unwrap_or_default(),
            ))
        })?;
        for row in rows {
            let (slug, first, last, dob, retired, nat) = row?;
            let dob = dob
                .filter(|d| !d.is_empty())
                .unwrap_or_else(|| "?".to_string());
            let birth_year = dob.split('-').next().and_then(|y| y.parse().ok());
            let pid = persons.len() as i64;
            // Source-data mojibake fix-up (see tdb_fhm::fixups) — only applies to the handful of
            // names we've verified against a clean record elsewhere (an earlier season / a
            // denormalized jersey-number or captaincy row).
            let (first, last) = tdb_fhm::known_name_fixup(&slug)
                .map(|(f, l)| (f.to_string(), l.to_string()))
                .unwrap_or((first, last));
            slug_to_id.insert(slug.clone(), pid);
            our_slug_to_id
                .entry(person_slug(&last, &first, &dob))
                .or_insert(pid);
            persons.push(Person {
                slug,
                first,
                last,
                nick: String::new(),
                dob,
                birth_year,
                height: None,
                weight: None,
                hand: String::new(),
                nationality: nat,
                birthcity: String::new(),
                birthstate: String::new(),
                retired: retired != 0,
                is_goalie: false,
                position: String::new(),
                first_year: i64::MAX,
                last_year: 0,
                season_count: 0,
                bio_done: false,
            });
        }
    }
    eprintln!("{} seasons, {} persons", seasons.len(), persons.len());

    let tx = out.transaction()?;
    // (end_year, fhm_team_id) -> (name, abbr, league_id) ; (end_year, fhm_league_id) -> abbr
    let mut team_ref: HashMap<(i64, i64), (String, String, i64)> = HashMap::new();
    let mut league_ref: HashMap<(i64, i64), String> = HashMap::new();
    let mut career_done: HashSet<i64> = HashSet::new();
    let mut draft_done: HashSet<i64> = HashSet::new();
    let mut contract_done: HashSet<i64> = HashSet::new();
    let mut award_done: HashSet<i64> = HashSet::new();

    // -- global cross-season team identity (source `teams` + `leagues`) --
    {
        let mut ins = tx.prepare(
            "INSERT OR IGNORE INTO team(team_key, name, nickname, abbr, league_key, league_abbr, league_name)
             VALUES(?1,?2,?3,?4,?5,?6,?7)",
        )?;
        for (tkey, (name, nick, abbr, lkey)) in &teams {
            let (lname, labbr) = leagues.get(lkey).cloned().unwrap_or_default();
            ins.execute(params![
                tkey,
                opt(name),
                opt(nick),
                opt(abbr),
                opt(lkey),
                opt(&labbr),
                opt(&lname)
            ])?;
        }
    }

    // -- NHL franchise lineage (consolidation layer) --
    {
        let mut ins_f = tx.prepare(
            "INSERT INTO franchise(franchise_id, name, display_name, abbr, is_defunct, notes)
             VALUES(?1,?2,?3,?4,?5,?6)",
        )?;
        let mut fid: HashMap<&str, i64> = HashMap::new();
        for (i, &(name, disp, abbr, defunct, notes)) in FRANCHISES.iter().enumerate() {
            let id = i as i64;
            ins_f.execute(params![id, name, disp, abbr, defunct, opt(notes)])?;
            fid.insert(name, id);
        }
        let mut ins_m = tx.prepare(
            "INSERT OR IGNORE INTO franchise_member(franchise_id, team_key, era_start, era_end, era_name)
             VALUES(?1,?2,?3,?4,?5)",
        )?;
        for &(fr, key, e0, e1, nm) in FRANCHISE_MEMBERS {
            let Some(&id) = fid.get(fr) else { continue };
            let end = if e1 == 0 { None } else { Some(e1) };
            ins_m.execute(params![id, key, e0, end, nm])?;
        }
    }

    for (year, sid) in &seasons {
        let year = *year;
        // -- leagues (+ reverse league_key -> fhm_league_id for this season) --
        let mut league_key_to_id: HashMap<String, i64> = HashMap::new();
        {
            let mut ins = tx.prepare(
                "INSERT OR IGNORE INTO league_season(season_year, src_league_id, name, abbr) VALUES(?1,?2,?3,?4)",
            )?;
            let mut stmt = src.prepare(
                "SELECT fhm_league_id, league_key FROM season_leagues WHERE season_id=?1",
            )?;
            let rows = stmt.query_map(params![sid], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?))
            })?;
            for row in rows {
                let (lid, lkey) = row?;
                let lkey = lkey.unwrap_or_default();
                league_key_to_id.insert(lkey.clone(), lid);
                let (name, abbr) = leagues.get(&lkey).cloned().unwrap_or_default();
                ins.execute(params![year, lid, opt(&name), opt(&abbr)])?;
                league_ref.insert((year, lid), if abbr.is_empty() { name } else { abbr });
            }
        }
        // -- teams --
        {
            let mut ins = tx.prepare(
                "INSERT OR IGNORE INTO team_season(season_year, src_team_id, team_key, league_id, name,
                    nickname, abbr, primary_color, secondary_color, text_color, conference_id, division_id)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            )?;
            let mut stmt = src.prepare(
                "SELECT fhm_team_id, team_key, primary_color, secondary_color, text_color, conference_id, division_id
                 FROM season_teams WHERE season_id=?1",
            )?;
            let rows = stmt.query_map(params![sid], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                    r.get::<_, Option<i64>>(6)?,
                ))
            })?;
            for row in rows {
                let (tid, tkey, pc, sc, tc, conf, div) = row?;
                let tkey = tkey.unwrap_or_default();
                let (name, nick, abbr, lkey) = teams.get(&tkey).cloned().unwrap_or_default();
                let lid = league_key_to_id.get(&lkey).copied().unwrap_or(-1);
                ins.execute(params![
                    year,
                    tid,
                    opt(&tkey),
                    lid,
                    opt(&name),
                    opt(&nick),
                    opt(&abbr),
                    pc,
                    sc,
                    tc,
                    conf,
                    div
                ])?;
                let disp = if abbr.is_empty() { name.clone() } else { abbr };
                team_ref.insert((year, tid), (name, disp, lid));
            }
        }
        // -- conferences / divisions --
        {
            let mut ins = tx.prepare(
                "INSERT OR IGNORE INTO conference_season(season_year, league_id, conference_id, name) VALUES(?1,?2,?3,?4)",
            )?;
            let mut stmt = src.prepare("SELECT fhm_league_id, fhm_conference_id, name FROM season_conferences WHERE season_id=?1")?;
            let rows = stmt.query_map(params![sid], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })?;
            for row in rows {
                let (lid, cid, name) = row?;
                ins.execute(params![year, lid, cid, name])?;
            }
        }
        {
            let mut ins = tx.prepare(
                "INSERT OR IGNORE INTO division_season(season_year, league_id, conference_id, division_id, name) VALUES(?1,?2,?3,?4,?5)",
            )?;
            let mut stmt = src.prepare("SELECT fhm_league_id, fhm_conference_id, fhm_division_id, name FROM season_divisions WHERE season_id=?1")?;
            let rows = stmt.query_map(params![sid], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            })?;
            for row in rows {
                let (lid, cid, did, name) = row?;
                ins.execute(params![year, lid, cid, did, name])?;
            }
        }

        // -- assemble ratings per player for this season --
        let ratings = load_season_ratings(&src, sid, &rating_cols)?;

        // -- contract-year counts per player (contract term) --
        let mut term: HashMap<i64, i64> = HashMap::new();
        {
            let mut stmt = src.prepare(
                "SELECT fhm_player_id, COUNT(*) FROM season_player_contract_years WHERE season_id=?1 GROUP BY fhm_player_id",
            )?;
            let rows = stmt.query_map(params![sid], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (pid, n) = row?;
                term.insert(pid, n);
            }
        }

        // -- season_players -> person / player_season / ratings_season --
        let mut season_pid: HashMap<i64, i64> = HashMap::new();
        {
            let mut ins_ps = tx.prepare(
                "INSERT OR IGNORE INTO player_season(person_id, season_year, src_player_id, team_id,
                    franchise_id, height, weight, birthcity, birthstate, nat_one, nat_two, nat_three,
                    retired, hand, rookie) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            )?;
            let mut ins_rs = tx.prepare(
                "INSERT OR IGNORE INTO ratings_season(person_id, season_year, g, ld, rd, lw, c, rw,
                    ability, potential, attrs_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            )?;
            let mut stmt = src.prepare(
                "SELECT fhm_player_id, slug, nick_name, fhm_team_id, fhm_franchise_id, height_in, weight_lb,
                        birth_city, birth_state, nationality_one, nationality_two, nationality_three,
                        retired, hand, rookie FROM season_players WHERE season_id=?1",
            )?;
            let mut rows = stmt.query(params![sid])?;
            while let Some(r) = rows.next()? {
                let src_id: i64 = r.get(0)?;
                let slug: Option<String> = r.get(1)?;
                let Some(slug) = slug else { continue };
                let Some(&pid) = slug_to_id.get(&slug) else {
                    continue;
                };
                season_pid.insert(src_id, pid);
                let rat = ratings.get(&src_id);

                // person span + canonical bio (first sighting = newest, since seasons are desc)
                {
                    let p = &mut persons[pid as usize];
                    p.first_year = p.first_year.min(year);
                    p.last_year = p.last_year.max(year);
                    p.season_count += 1;
                    if !p.bio_done {
                        p.nick = r.get::<_, Option<String>>(2)?.unwrap_or_default();
                        p.height = r.get(5)?;
                        p.weight = r.get(6)?;
                        p.birthcity = r.get::<_, Option<String>>(7)?.unwrap_or_default();
                        p.birthstate = r.get::<_, Option<String>>(8)?.unwrap_or_default();
                        let n1: Option<String> = r.get(9)?;
                        if let Some(n) = n1.filter(|s| !s.is_empty()) {
                            p.nationality = n;
                        }
                        p.hand = r.get::<_, Option<String>>(13)?.unwrap_or_default();
                        p.retired = r.get::<_, Option<i64>>(12)?.unwrap_or(0) != 0;
                        p.bio_done = true;
                    }
                    // position/goalie from the first season with ratings
                    if p.position.is_empty() {
                        if let Some(rat) = rat {
                            p.position = rat.position.clone();
                            p.is_goalie = rat.is_goalie;
                        }
                    }
                }

                ins_ps.execute(params![
                    pid,
                    year,
                    src_id,
                    r.get::<_, Option<i64>>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                    r.get::<_, Option<i64>>(6)?,
                    r.get::<_, Option<String>>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, Option<String>>(9)?,
                    r.get::<_, Option<String>>(10)?,
                    r.get::<_, Option<String>>(11)?,
                    r.get::<_, Option<i64>>(12)?,
                    r.get::<_, Option<String>>(13)?,
                    r.get::<_, Option<i64>>(14)?,
                ])?;
                if let Some(rat) = rat {
                    ins_rs.execute(params![
                        pid,
                        year,
                        rat.g,
                        rat.ld,
                        rat.rd,
                        rat.lw,
                        rat.c,
                        rat.rw,
                        rat.ability,
                        rat.potential,
                        rat.attrs_json
                    ])?;
                }
            }
        }

        ingest_careers(
            &tx,
            year,
            sid,
            &src,
            &season_pid,
            &team_ref,
            &league_ref,
            &mut career_done,
        )?;
        // Drafts: from the prior DB only when no FHM CSV root was given (its picks are collapsed to one
        // per player). With the CSV root, drafts + draft names come from `ingest_drafts_from_csv` below.
        if fhm_root.is_none() {
            ingest_draft(
                &tx,
                year,
                sid,
                &src,
                &season_pid,
                &team_ref,
                &mut draft_done,
            )?;
            ingest_season_drafts(&tx, year, sid, &src)?;
        }
        ingest_contract(
            &tx,
            sid,
            &src,
            &season_pid,
            &team_ref,
            &term,
            &mut contract_done,
        )?;
        ingest_awards(&tx, sid, &src, &season_pid, &league_ref, &mut award_done)?;
        // Full-parity per-season entities (no per-person dedup — every season is kept).
        ingest_season_contracts(&tx, year, sid, &src, &season_pid)?;
        ingest_player_rights(&tx, year, sid, &src, &season_pid)?;
        ingest_team_parents(&tx, year, sid, &src)?;
        ingest_schedule(&tx, year, sid, &src)?;

        eprintln!("  {year}: {} persons seen", career_done.len());
    }

    // Authoritative drafts from the FHM CSV export (all of a person's league drafts), matched to our
    // persons by name+DOB slug. team_ref is fully built now, so drafting teams resolve.
    if let Some(root) = &fhm_root {
        ingest_drafts_from_csv(&tx, root, &our_slug_to_id, &team_ref, &mut draft_done)?;
    }

    // -- global, source-resolved tables (keyed by slug + team_key, not per-season ids) --
    ingest_jersey_numbers(&tx, &src, &slug_to_id)?;
    ingest_captaincy(&tx, &src, &slug_to_id)?;

    // goalie-career fallback for ratings-less people
    {
        let mut stmt =
            tx.prepare("SELECT DISTINCT person_id FROM career_stat WHERE kind='goalie'")?;
        let goalies: HashSet<i64> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        for (pid, p) in persons.iter_mut().enumerate() {
            if p.position.is_empty() && goalies.contains(&(pid as i64)) {
                p.position = "G".to_string();
                p.is_goalie = true;
            }
        }
    }

    // flush persons
    {
        let mut ins = tx.prepare(
            "INSERT INTO person(person_id, slug, first_name, last_name, nick_name, dob, birth_year,
                height, weight, hand, nationality, birthcity, birthstate, retired, first_year,
                last_year, season_count, is_goalie, position)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
        )?;
        for (pid, p) in persons.iter().enumerate() {
            if !p.bio_done {
                continue; // a `players` row with no season_players — no usable data, skip
            }
            ins.execute(params![
                pid as i64,
                p.slug,
                p.first,
                p.last,
                opt(&p.nick),
                p.dob,
                p.birth_year,
                p.height,
                p.weight,
                opt(&p.hand),
                opt(&p.nationality),
                opt(&p.birthcity),
                opt(&p.birthstate),
                p.retired as i64,
                if p.first_year == i64::MAX {
                    None
                } else {
                    Some(p.first_year)
                },
                if p.last_year == 0 {
                    None
                } else {
                    Some(p.last_year)
                },
                p.season_count,
                p.is_goalie as i64,
                opt(&p.position),
            ])?;
        }
    }

    // Resolve each draft row's era-appropriate drafting-team name now that team_season + the franchise
    // lineage are populated (falls back to the raw abbr where no lineage covers the pick).
    tx.execute_batch(DRAFT_TEAM_NAME_SQL)?;

    write_meta(&tx, "schema_version", &SCHEMA_VERSION.to_string())?;
    write_meta(&tx, "source", &format!("converted from {src_path}"))?;
    write_meta(&tx, "season_count", &seasons.len().to_string())?;
    write_meta(
        &tx,
        "person_count",
        &persons.iter().filter(|p| p.bio_done).count().to_string(),
    )?;
    write_meta(&tx, "season_min", &seasons.last().unwrap().0.to_string())?;
    write_meta(&tx, "season_max", &seasons.first().unwrap().0.to_string())?;

    tx.commit()?;
    out.execute_batch("ANALYZE;")?;
    eprintln!("done -> {out_path}");
    Ok(())
}

/// Build `fhm_player_id -> Ratings` for one season: grades from `season_player_ratings`, attributes
/// (incl. Ability/Potential) from `season_player_raw_ratings`; grades are also injected into
/// `attrs_json` so it matches the CSV-built corpus (the UI reads position pills from there).
fn load_season_ratings(
    src: &Connection,
    sid: &str,
    rating_cols: &HashMap<i64, String>,
) -> Res<HashMap<i64, Ratings>> {
    // grades
    let mut grades: HashMap<i64, (i64, i64, i64, i64, i64, i64)> = HashMap::new();
    {
        let mut stmt = src.prepare("SELECT fhm_player_id, g, ld, rd, lw, c, rw FROM season_player_ratings WHERE season_id=?1")?;
        let rows = stmt.query_map(params![sid], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                (
                    r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(3)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(4)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(5)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(6)?.unwrap_or(0),
                ),
            ))
        })?;
        for row in rows {
            let (pid, g) = row?;
            grades.insert(pid, g);
        }
    }
    // raw attrs
    let mut attrs: HashMap<i64, serde_json::Map<String, serde_json::Value>> = HashMap::new();
    {
        let mut stmt = src.prepare("SELECT fhm_player_id, column_id, value FROM season_player_raw_ratings WHERE season_id=?1")?;
        let mut rows = stmt.query(params![sid])?;
        while let Some(r) = rows.next()? {
            let pid: i64 = r.get(0)?;
            let col: i64 = r.get(1)?;
            let val: Option<String> = r.get(2)?;
            let Some(name) = rating_cols.get(&col) else {
                continue;
            };
            attrs
                .entry(pid)
                .or_default()
                .insert(name.clone(), json_num(val.as_deref().unwrap_or("")));
        }
    }

    let mut out = HashMap::new();
    // Union of players that have grades OR attrs.
    let ids: HashSet<i64> = grades.keys().chain(attrs.keys()).copied().collect();
    for pid in ids {
        let (g, ld, rd, lw, c, rw) = grades.get(&pid).copied().unwrap_or((0, 0, 0, 0, 0, 0));
        let mut map = attrs.remove(&pid).unwrap_or_default();
        let ability = map.get("Ability").and_then(|v| v.as_f64());
        let potential = map.get("Potential").and_then(|v| v.as_f64());
        // inject position grades so attrs_json matches the CSV corpus
        for (k, v) in [
            ("G", g),
            ("LD", ld),
            ("RD", rd),
            ("LW", lw),
            ("C", c),
            ("RW", rw),
        ] {
            map.insert(k.to_string(), serde_json::Value::from(v));
        }
        // primary position: highest grade wins (FHM rates natural position 20); ties G>C>LW>RW>D
        let cand = [
            ("G", g),
            ("C", c),
            ("LW", lw),
            ("RW", rw),
            ("D", ld.max(rd)),
        ];
        let maxv = cand.iter().map(|(_, v)| *v).max().unwrap_or(0);
        let position = if maxv <= 0 {
            String::new()
        } else {
            cand.iter()
                .find(|(_, v)| *v == maxv)
                .map(|(l, _)| l.to_string())
                .unwrap_or_default()
        };
        let is_goalie = position == "G";
        out.insert(
            pid,
            Ratings {
                g,
                ld,
                rd,
                lw,
                c,
                rw,
                ability,
                potential,
                attrs_json: serde_json::to_string(&map).unwrap_or_else(|_| "{}".into()),
                is_goalie,
                position,
            },
        );
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn ingest_careers(
    tx: &Connection,
    year: i64,
    sid: &str,
    src: &Connection,
    season_pid: &HashMap<i64, i64>,
    team_ref: &HashMap<(i64, i64), (String, String, i64)>,
    league_ref: &HashMap<(i64, i64), String>,
    career_done: &mut HashSet<i64>,
) -> Res<()> {
    struct Row {
        pid: i64,
        kind: &'static str,
        phase: &'static str,
        year: i64,
        team_id: Option<i64>,
        league_id: Option<i64>,
        gp: Option<i64>,
        stats_json: String,
    }
    let mut pending: Vec<Row> = Vec::new();
    let mut fresh: HashSet<i64> = HashSet::new();

    let mut stmt = src.prepare(
        "SELECT fhm_player_id, kind, year, fhm_team_id, fhm_league_id, games_played, goals, assists,
                plus_minus, penalty_minutes, wins, losses, goals_against, shots_against, shutouts
         FROM season_player_career_stats WHERE season_id=?1",
    )?;
    let mut rows = stmt.query(params![sid])?;
    while let Some(r) = rows.next()? {
        let src_id: i64 = r.get(0)?;
        let pid = match season_pid.get(&src_id) {
            Some(&p) if !career_done.contains(&p) => p,
            _ => continue,
        };
        let kind_raw: String = r.get::<_, Option<String>>(1)?.unwrap_or_default();
        let (kind, phase) = match kind_raw.as_str() {
            "SkaterRegular" => ("skater", "rs"),
            "SkaterPlayoff" => ("skater", "po"),
            "GoalieRegular" => ("goalie", "rs"),
            "GoaliePlayoff" => ("goalie", "po"),
            _ => continue,
        };
        let gp: Option<i64> = r.get(5)?;
        let mut map = serde_json::Map::new();
        let put = |m: &mut serde_json::Map<String, serde_json::Value>, k: &str, v: Option<i64>| {
            if let Some(v) = v {
                m.insert(k.to_string(), serde_json::Value::from(v));
            }
        };
        put(&mut map, "GP", gp);
        if kind == "skater" {
            put(&mut map, "G", r.get(6)?);
            put(&mut map, "A", r.get(7)?);
            put(&mut map, "+/-", r.get(8)?);
            put(&mut map, "PIM", r.get(9)?);
        } else {
            put(&mut map, "W", r.get(10)?);
            put(&mut map, "L", r.get(11)?);
            put(&mut map, "GA", r.get(12)?);
            put(&mut map, "SA", r.get(13)?);
            put(&mut map, "SO", r.get(14)?);
        }
        fresh.insert(pid);
        pending.push(Row {
            pid,
            kind,
            phase,
            year: r.get(2)?,
            team_id: r.get(3)?,
            league_id: r.get(4)?,
            gp,
            stats_json: serde_json::to_string(&map)?,
        });
    }

    let mut ins = tx.prepare(
        "INSERT INTO career_stat(person_id, kind, phase, year, src_season, team_id, league_id,
            team_abbr, league_abbr, gp, stats_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
    )?;
    for row in pending {
        let team_abbr = row
            .team_id
            .and_then(|t| team_ref.get(&(year, t)).map(|x| x.1.clone()));
        let league_abbr = row
            .league_id
            .and_then(|l| league_ref.get(&(year, l)).cloned());
        ins.execute(params![
            row.pid,
            row.kind,
            row.phase,
            row.year,
            year,
            row.team_id,
            row.league_id,
            team_abbr,
            league_abbr,
            row.gp,
            row.stats_json
        ])?;
    }
    career_done.extend(fresh);
    Ok(())
}

fn ingest_draft(
    tx: &Connection,
    year: i64,
    sid: &str,
    src: &Connection,
    season_pid: &HashMap<i64, i64>,
    team_ref: &HashMap<(i64, i64), (String, String, i64)>,
    draft_done: &mut HashSet<i64>,
) -> Res<()> {
    let mut ins = tx.prepare(
        "INSERT OR IGNORE INTO draft_info(person_id, year, round, pick, overall, src_season, team_id,
            team_abbr, draft_id, picked_from) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
    )?;
    let mut stmt = src.prepare(
        "SELECT fhm_player_id, year, round, pick, overall, fhm_team_id, draft_id, picked_from
         FROM season_draft_picks WHERE season_id=?1",
    )?;
    let mut rows = stmt.query(params![sid])?;
    // Keep EVERY draft row a person has in this snapshot (NHL + junior/import), not just the first.
    // Finalize per person only AFTER the snapshot so all their rows land from one id namespace; a
    // person already sourced from a newer season is skipped entirely.
    let mut seen: HashSet<i64> = HashSet::new();
    while let Some(r) = rows.next()? {
        let src_id: i64 = r.get(0)?;
        let pid = match season_pid.get(&src_id) {
            Some(&p) if !draft_done.contains(&p) => p,
            _ => continue,
        };
        let team_id: Option<i64> = r.get(5)?;
        let team_abbr = team_id.and_then(|t| team_ref.get(&(year, t)).map(|x| x.1.clone()));
        ins.execute(params![
            pid,
            r.get::<_, Option<i64>>(1)?,
            r.get::<_, Option<i64>>(2)?,
            sentinel_pick(r.get(3)?),
            r.get::<_, Option<i64>>(4)?,
            year,
            team_id,
            team_abbr,
            r.get::<_, Option<i64>>(6)?,
            r.get::<_, Option<i64>>(7)?
        ])?;
        seen.insert(pid);
    }
    draft_done.extend(seen);
    Ok(())
}

/// FHM writes `10000` for "pick-within-round unknown" (the `overall` position is still reliable).
/// Null it so it never surfaces as a literal pick number.
fn sentinel_pick(pick: Option<i64>) -> Option<i64> {
    pick.filter(|&p| p < 10000)
}

/// Source drafts from the FHM season-export CSVs (the prior DB keeps only one collapsed pick per
/// player). Walks seasons newest→oldest, matching each `draft_info.csv` row to our persons by the
/// name+DOB slug from that season's `player_master.csv`; a person's drafts come from their newest
/// appearance so one draft-id namespace applies. Also fills `season_draft` names from `draft_index.csv`.
/// `team_ref` (fully built) resolves the drafting-team abbr; the era-name pass runs afterwards.
fn ingest_drafts_from_csv(
    tx: &Connection,
    fhm_root: &str,
    our_slug_to_id: &HashMap<String, i64>,
    team_ref: &HashMap<(i64, i64), (String, String, i64)>,
    draft_done: &mut HashSet<i64>,
) -> Res<()> {
    let mut seasons = discover_seasons(Path::new(fhm_root))?;
    seasons.sort_by_key(|s| std::cmp::Reverse(s.0)); // newest first
    let mut ins = tx.prepare(
        "INSERT OR IGNORE INTO draft_info(person_id, year, round, pick, overall, src_season, team_id,
            team_abbr, draft_id, picked_from) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
    )?;
    let mut ins_sd = tx.prepare(
        "INSERT OR IGNORE INTO season_draft(season_year, draft_id, name) VALUES(?1,?2,?3)",
    )?;
    let mut total = 0i64;
    for (year, dir) in &seasons {
        // Draft-league names for this season (draft_index.csv can carry a duplicated header row — a
        // non-numeric DraftId — which `opt_i` drops).
        if let Some((h, rows)) = read_table(&dir.join("draft_index.csv"))? {
            for r in &rows {
                if let Some(id) = opt_i(g(&h, r, "DraftId")) {
                    ins_sd.execute(params![year, id, g(&h, r, "Draft Name")])?;
                }
            }
        }
        // PlayerId -> our person slug for this season.
        let Some((mh, mrows)) = read_table(&dir.join("player_master.csv"))? else {
            continue;
        };
        let mut pid_slug: HashMap<i64, String> = HashMap::new();
        for r in &mrows {
            pid_slug.insert(
                i0(g(&mh, r, "PlayerId")),
                person_slug(
                    g(&mh, r, "Last Name"),
                    g(&mh, r, "First Name"),
                    g(&mh, r, "Date Of Birth"),
                ),
            );
        }
        let Some((dh, drows)) = read_table(&dir.join("draft_info.csv"))? else {
            continue;
        };
        // Insert every draft row for persons not already sourced from a newer season; finalize after so
        // all of this season's rows for a person are kept before older seasons are skipped.
        let mut seen: HashSet<i64> = HashSet::new();
        for r in &drows {
            let Some(slug) = pid_slug.get(&i0(g(&dh, r, "PlayerId"))) else {
                continue;
            };
            let pid = match our_slug_to_id.get(slug) {
                Some(&p) if !draft_done.contains(&p) => p,
                _ => continue,
            };
            let team_id = opt_i(g(&dh, r, "Team"));
            let team_abbr = team_id.and_then(|t| team_ref.get(&(*year, t)).map(|x| x.1.clone()));
            ins.execute(params![
                pid,
                opt_i(g(&dh, r, "Year")),
                opt_i(g(&dh, r, "Round")),
                sentinel_pick(opt_i(g(&dh, r, "Pick"))),
                opt_i(g(&dh, r, "Overall")),
                year,
                team_id,
                team_abbr,
                opt_i(g(&dh, r, "DraftId")),
                opt_i(g(&dh, r, "Picked From")),
            ])?;
            seen.insert(pid);
            total += 1;
        }
        draft_done.extend(seen);
    }
    eprintln!(
        "  drafts (FHM CSV): {total} rows across {} persons",
        draft_done.len()
    );
    Ok(())
}

/// Discover `YYYY-YYYY.lg` season folders under an FHM export root, keyed by end year.
fn discover_seasons(root: &Path) -> Res<Vec<(i64, PathBuf)>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let stem = name.strip_suffix(".lg").unwrap_or(&name);
        if let Some((_, end)) = stem.split_once('-') {
            if let Ok(y) = end.trim().parse::<i64>() {
                out.push((y, entry.path()));
            }
        }
    }
    Ok(out)
}

/// A parsed semicolon CSV: the header's column-name -> index map, plus its data rows.
type CsvTable = (HashMap<String, usize>, Vec<StringRecord>);

/// Open a semicolon CSV; `None` if missing or header-only.
fn read_table(path: &Path) -> Res<Option<CsvTable>> {
    if !path.exists() {
        return Ok(None);
    }
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .flexible(true)
        .from_path(path)?;
    let headers: HashMap<String, usize> = rdr
        .headers()?
        .iter()
        .enumerate()
        .map(|(i, h)| (h.trim().to_string(), i))
        .collect();
    let rows: Vec<StringRecord> = rdr.records().filter_map(Result::ok).collect();
    if rows.is_empty() {
        return Ok(None);
    }
    Ok(Some((headers, rows)))
}

/// Field accessor by header name (trimmed; "" when absent).
fn g<'a>(h: &HashMap<String, usize>, r: &'a StringRecord, name: &str) -> &'a str {
    h.get(name).and_then(|&i| r.get(i)).unwrap_or("").trim()
}

/// Parse an int, treating "", "-1", "-" as absent.
fn opt_i(s: &str) -> Option<i64> {
    let t = s.trim();
    if t.is_empty() || t == "-1" || t == "-" {
        None
    } else {
        t.parse().ok()
    }
}

/// Parse an int id, defaulting absent to 0 (0 is a real season id namespace value).
fn i0(s: &str) -> i64 {
    let t = s.trim();
    if t.is_empty() || t == "-" {
        return 0;
    }
    t.parse().unwrap_or(0)
}

/// Per-season contract snapshot + per-year salary breakdown (no per-person dedup).
fn ingest_season_contracts(
    tx: &Connection,
    year: i64,
    sid: &str,
    src: &Connection,
    season_pid: &HashMap<i64, i64>,
) -> Res<()> {
    {
        let mut ins = tx.prepare(
            "INSERT OR IGNORE INTO season_contract(person_id, season_year, team_id, ntc, nmc, elc, ufa,
                scholarship, avg_salary) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        )?;
        let mut stmt = src.prepare(
            "SELECT fhm_player_id, fhm_team_id, no_trade_clause, no_movement_clause, entry_level, ufa,
                    scholarship, average_salary FROM season_player_contracts WHERE season_id=?1",
        )?;
        let mut rows = stmt.query(params![sid])?;
        while let Some(r) = rows.next()? {
            let Some(&pid) = season_pid.get(&r.get::<_, i64>(0)?) else {
                continue;
            };
            ins.execute(params![
                pid,
                year,
                r.get::<_, Option<i64>>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, Option<i64>>(3)?,
                r.get::<_, Option<i64>>(4)?,
                r.get::<_, Option<i64>>(5)?,
                r.get::<_, Option<String>>(6)?
                    .filter(|s| s != "-" && !s.is_empty()),
                r.get::<_, Option<i64>>(7)?
            ])?;
        }
    }
    {
        let mut ins = tx.prepare(
            "INSERT OR IGNORE INTO season_contract_year(person_id, season_year, year, major_salary, minor_salary)
             VALUES(?1,?2,?3,?4,?5)",
        )?;
        let mut stmt = src.prepare(
            "SELECT fhm_player_id, year, major_salary, minor_salary FROM season_player_contract_years WHERE season_id=?1",
        )?;
        let mut rows = stmt.query(params![sid])?;
        while let Some(r) = rows.next()? {
            let Some(&pid) = season_pid.get(&r.get::<_, i64>(0)?) else {
                continue;
            };
            ins.execute(params![
                pid,
                year,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, Option<i64>>(3)?
            ])?;
        }
    }
    Ok(())
}

/// Per-season rights holder for each player.
fn ingest_player_rights(
    tx: &Connection,
    year: i64,
    sid: &str,
    src: &Connection,
    season_pid: &HashMap<i64, i64>,
) -> Res<()> {
    let mut ins = tx.prepare(
        "INSERT OR IGNORE INTO player_rights(person_id, season_year, league_id, team_id) VALUES(?1,?2,?3,?4)",
    )?;
    let mut stmt = src.prepare(
        "SELECT fhm_player_id, fhm_league_id, fhm_team_id FROM season_player_rights WHERE season_id=?1",
    )?;
    let mut rows = stmt.query(params![sid])?;
    while let Some(r) = rows.next()? {
        let Some(&pid) = season_pid.get(&r.get::<_, i64>(0)?) else {
            continue;
        };
        ins.execute(params![
            pid,
            year,
            r.get::<_, Option<i64>>(1)?,
            r.get::<_, Option<i64>>(2)?
        ])?;
    }
    Ok(())
}

/// Team affiliates (parent clubs) that season.
fn ingest_team_parents(tx: &Connection, year: i64, sid: &str, src: &Connection) -> Res<()> {
    let mut ins = tx.prepare(
        "INSERT OR IGNORE INTO team_parent(season_year, src_team_id, parent_team_id) VALUES(?1,?2,?3)",
    )?;
    let mut stmt = src.prepare(
        "SELECT fhm_team_id, parent_team_id FROM season_team_parents WHERE season_id=?1",
    )?;
    let mut rows = stmt.query(params![sid])?;
    while let Some(r) = rows.next()? {
        ins.execute(params![year, r.get::<_, i64>(0)?, r.get::<_, i64>(1)?])?;
    }
    Ok(())
}

/// Draft names for the season export.
fn ingest_season_drafts(tx: &Connection, year: i64, sid: &str, src: &Connection) -> Res<()> {
    let mut ins = tx.prepare(
        "INSERT OR IGNORE INTO season_draft(season_year, draft_id, name) VALUES(?1,?2,?3)",
    )?;
    let mut stmt = src.prepare("SELECT draft_id, name FROM season_drafts WHERE season_id=?1")?;
    let mut rows = stmt.query(params![sid])?;
    while let Some(r) = rows.next()? {
        ins.execute(params![
            year,
            r.get::<_, i64>(0)?,
            r.get::<_, Option<String>>(1)?
        ])?;
    }
    Ok(())
}

/// League schedule + results for the season.
fn ingest_schedule(tx: &Connection, year: i64, sid: &str, src: &Connection) -> Res<()> {
    let mut ins = tx.prepare(
        "INSERT INTO schedule_game(season_year, league_id, game_date, home_team_id, score_home,
            away_team_id, score_away, game_type, played, overtime, shootout, game_id)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
    )?;
    let mut stmt = src.prepare(
        "SELECT fhm_league_id, game_date, home_team_id, score_home, away_team_id, score_away,
                game_type, played, overtime, shootout, game_id FROM season_schedule_games WHERE season_id=?1",
    )?;
    let mut rows = stmt.query(params![sid])?;
    while let Some(r) = rows.next()? {
        ins.execute(params![
            year,
            r.get::<_, Option<i64>>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<i64>>(2)?,
            r.get::<_, Option<i64>>(3)?,
            r.get::<_, Option<i64>>(4)?,
            r.get::<_, Option<i64>>(5)?,
            r.get::<_, Option<String>>(6)?,
            r.get::<_, Option<i64>>(7)?,
            r.get::<_, Option<i64>>(8)?,
            r.get::<_, Option<i64>>(9)?,
            r.get::<_, Option<i64>>(10)?
        ])?;
    }
    Ok(())
}

/// Jersey numbers, resolved from the source's pre-matched slug + team_key (global, all seasons).
fn ingest_jersey_numbers(
    tx: &Connection,
    src: &Connection,
    slug_to_id: &HashMap<String, i64>,
) -> Res<()> {
    let mut ins = tx.prepare(
        "INSERT OR IGNORE INTO jersey_number(person_id, team_key, season_year, number) VALUES(?1,?2,?3,?4)",
    )?;
    let mut stmt = src.prepare(
        "SELECT matched_slug, matched_team_key, season_id, jersey_number
         FROM player_jersey_numbers WHERE matched_slug IS NOT NULL",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(r) = rows.next()? {
        let slug: String = r.get(0)?;
        let Some(&pid) = slug_to_id.get(&slug) else {
            continue;
        };
        let sy = end_year(&r.get::<_, String>(2)?);
        ins.execute(params![
            pid,
            r.get::<_, Option<String>>(1)?,
            sy,
            r.get::<_, i64>(3)?
        ])?;
    }
    Ok(())
}

/// Captain / alternate designations, resolved from the source's pre-matched slug + team_key.
fn ingest_captaincy(
    tx: &Connection,
    src: &Connection,
    slug_to_id: &HashMap<String, i64>,
) -> Res<()> {
    let mut ins = tx.prepare(
        "INSERT INTO captaincy(person_id, team_key, season_year, role) VALUES(?1,?2,?3,?4)",
    )?;
    let mut stmt = src.prepare(
        "SELECT matched_slug, matched_team_key, matched_season_id, role
         FROM team_captaincy WHERE matched_slug IS NOT NULL",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(r) = rows.next()? {
        let slug: String = r.get(0)?;
        let Some(&pid) = slug_to_id.get(&slug) else {
            continue;
        };
        let sy = r.get::<_, Option<String>>(2)?.map(|s| end_year(&s));
        ins.execute(params![
            pid,
            r.get::<_, Option<String>>(1)?,
            sy,
            r.get::<_, String>(3)?
        ])?;
    }
    Ok(())
}

/// End year of a `"YYYY-YYYY"` season id (0 when unparseable).
fn end_year(season_id: &str) -> i64 {
    season_id
        .split('-')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn ingest_contract(
    tx: &Connection,
    sid: &str,
    src: &Connection,
    season_pid: &HashMap<i64, i64>,
    team_ref: &HashMap<(i64, i64), (String, String, i64)>,
    term: &HashMap<i64, i64>,
    contract_done: &mut HashSet<i64>,
) -> Res<()> {
    // The season's end year (for team-abbr resolution) is parsed from the season id ("YYYY-YYYY").
    let mut ins = tx.prepare(
        "INSERT OR IGNORE INTO contract(person_id, team_abbr, ntc, nmc, elc, ufa, scholarship,
            avg_salary, length, src_season) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
    )?;
    let mut stmt = src.prepare(
        "SELECT fhm_player_id, fhm_team_id, no_trade_clause, no_movement_clause, entry_level, ufa,
                scholarship, average_salary FROM season_player_contracts WHERE season_id=?1",
    )?;
    // Resolve the season's end_year from the season id string ("YYYY-YYYY").
    let sy: i64 = sid
        .split('-')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut rows = stmt.query(params![sid])?;
    while let Some(r) = rows.next()? {
        let src_id: i64 = r.get(0)?;
        let pid = match season_pid.get(&src_id) {
            Some(&p) if !contract_done.contains(&p) => p,
            _ => continue,
        };
        let team_id: Option<i64> = r.get(1)?;
        let team_abbr = team_id.and_then(|t| team_ref.get(&(sy, t)).map(|x| x.1.clone()));
        ins.execute(params![
            pid,
            team_abbr,
            r.get::<_, Option<i64>>(2)?.unwrap_or(0),
            r.get::<_, Option<i64>>(3)?.unwrap_or(0),
            r.get::<_, Option<i64>>(4)?.unwrap_or(0),
            r.get::<_, Option<i64>>(5)?.unwrap_or(0),
            r.get::<_, Option<String>>(6)?
                .filter(|s| s != "-" && !s.is_empty()),
            r.get::<_, Option<i64>>(7)?,
            term.get(&src_id).copied(),
            sy,
        ])?;
        contract_done.insert(pid);
    }
    Ok(())
}

/// One award row as grouped per person: (league id, year, name, description).
type AwardRow = (Option<i64>, i64, Option<String>, Option<String>);

fn ingest_awards(
    tx: &Connection,
    sid: &str,
    src: &Connection,
    season_pid: &HashMap<i64, i64>,
    league_ref: &HashMap<(i64, i64), String>,
    award_done: &mut HashSet<i64>,
) -> Res<()> {
    let sy: i64 = sid
        .split('-')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    // group by person; source once per person (newest season)
    let mut by_pid: HashMap<i64, Vec<AwardRow>> = HashMap::new();
    let mut stmt = src.prepare(
        "SELECT fhm_player_id, fhm_league_id, year, award_name, award_description FROM season_league_awards WHERE season_id=?1",
    )?;
    let mut rows = stmt.query(params![sid])?;
    while let Some(r) = rows.next()? {
        let src_id: i64 = r.get(0)?;
        let pid = match season_pid.get(&src_id) {
            Some(&p) if !award_done.contains(&p) => p,
            _ => continue,
        };
        by_pid.entry(pid).or_default().push((
            r.get::<_, Option<i64>>(2)?,
            r.get::<_, i64>(1)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, Option<String>>(4)?,
        ));
    }
    let mut ins = tx.prepare(
        "INSERT INTO award(person_id, year, league_abbr, name, description) VALUES(?1,?2,?3,?4,?5)",
    )?;
    for (pid, list) in by_pid {
        for (yr, lid, name, desc) in list {
            let league_abbr = league_ref.get(&(sy, lid)).cloned();
            ins.execute(params![pid, yr, league_abbr, name, desc])?;
        }
        award_done.insert(pid);
    }
    Ok(())
}

// ---- helpers ---------------------------------------------------------------

fn opt(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

fn json_num(v: &str) -> serde_json::Value {
    let t = v.trim();
    if t.is_empty() || t == "-" {
        return serde_json::Value::Null;
    }
    if let Ok(i) = t.parse::<i64>() {
        return serde_json::Value::from(i);
    }
    if let Ok(f) = t.parse::<f64>() {
        return serde_json::json!(f);
    }
    serde_json::Value::from(t)
}

fn write_meta(tx: &Connection, key: &str, value: &str) -> Res<()> {
    tx.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES(?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}
