//! `tdb-fhm-build` — ingest the FHM season-export CSV tree into the person-centric SQLite corpus.
//!
//! Usage: `tdb-fhm-build <export-root> <out.db>`
//!   * `<export-root>` holds the `YYYY-YYYY.lg` season folders (each with the 23-file CSV set).
//!   * `<out.db>` is (re)created with the [`tdb_fhm::SCHEMA_SQL`] schema.
//!
//! Model: ids in each `.lg` folder are namespaced to that season, so a human is unified across
//! seasons by the [`tdb_fhm::person_slug`] rule (`last|first|dob`). We walk seasons **newest→oldest**
//! so the first time a slug is seen is its canonical (latest) bio; a person's career is sourced once,
//! from the newest export in which they carry career rows (fullest history, one id namespace).

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::{Path, PathBuf};

use csv::StringRecord;
use rusqlite::{params, Connection};

use tdb_fhm::{person_slug, SCHEMA_SQL, SCHEMA_VERSION};

type Res<T> = Result<T, Box<dyn Error>>;

/// An accumulating person record (indexed by assigned person_id = position in a Vec).
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
    /// Derived primary position (C/LW/RW/D/G); empty until a ratings row is seen for this person.
    position: String,
    first_year: i64,
    last_year: i64,
    season_count: i64,
}

fn main() -> Res<()> {
    let mut args = std::env::args().skip(1);
    let root = args
        .next()
        .ok_or("usage: tdb-fhm-build <export-root> <out.db>")?;
    let out = args
        .next()
        .ok_or("usage: tdb-fhm-build <export-root> <out.db>")?;
    let root = PathBuf::from(root);

    let mut seasons = discover_seasons(&root)?;
    seasons.sort_by_key(|s| std::cmp::Reverse(s.0)); // newest first
    if seasons.is_empty() {
        return Err(format!(
            "no YYYY-YYYY.lg season folders found under {}",
            root.display()
        )
        .into());
    }
    eprintln!(
        "found {} seasons ({}..={})",
        seasons.len(),
        seasons.last().unwrap().0,
        seasons.first().unwrap().0
    );

    // Fresh corpus.
    let _ = std::fs::remove_file(&out);
    let mut conn = Connection::open(&out)?;
    // We insert child rows (player_season, career_stat, …) during the pass and flush the parent
    // `person` rows at the end, so FK enforcement (on by default in rusqlite) must be off for the
    // bulk build. The corpus is read-only downstream; the FK declarations stay as documentation.
    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
    conn.execute_batch(SCHEMA_SQL)?;

    // Per-season reference maps for name resolution (built as we ingest the reference tables).
    // (season, team_id) -> (name, abbr, league_id) ; (season, league_id) -> abbr
    let mut team_ref: HashMap<(i64, i64), (String, String, i64)> = HashMap::new();
    let mut league_ref: HashMap<(i64, i64), String> = HashMap::new();

    // Person accumulation.
    let mut persons: Vec<Person> = Vec::new();
    let mut slug_to_id: HashMap<String, i64> = HashMap::new();
    let mut career_done: HashSet<i64> = HashSet::new();
    let mut draft_done: HashSet<i64> = HashSet::new();
    let mut award_done: HashSet<i64> = HashSet::new();
    let mut contract_done: HashSet<i64> = HashSet::new();

    let tx = conn.transaction()?;
    for (year, dir) in &seasons {
        ingest_reference(&tx, *year, dir, &mut team_ref, &mut league_ref)?;
        ingest_season(
            &tx,
            *year,
            dir,
            &team_ref,
            &league_ref,
            &mut persons,
            &mut slug_to_id,
            &mut career_done,
            &mut draft_done,
            &mut award_done,
            &mut contract_done,
        )?;
        eprintln!(
            "  {year}: {} persons so far ({} with career)",
            persons.len(),
            career_done.len()
        );
    }

    // Fallback: ratings-less people who nonetheless have a goalie career line are goalies. (Skaters
    // without ratings have no position source in the data, so they stay unset.)
    {
        let mut stmt =
            tx.prepare("SELECT DISTINCT person_id FROM career_stat WHERE kind='goalie'")?;
        let goalie_ids: HashSet<i64> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<Result<HashSet<_>, _>>()?;
        for (pid, p) in persons.iter_mut().enumerate() {
            if p.position.is_empty() && goalie_ids.contains(&(pid as i64)) {
                p.position = "G".to_string();
                p.is_goalie = true;
            }
        }
    }

    // Flush the accumulated persons.
    {
        let mut ins = tx.prepare(
            "INSERT INTO person(person_id, slug, first_name, last_name, nick_name, dob, birth_year,
                height, weight, hand, nationality, birthcity, birthstate, retired, first_year,
                last_year, season_count, is_goalie, position)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
        )?;
        for (pid, p) in persons.iter().enumerate() {
            ins.execute(params![
                pid as i64,
                p.slug,
                p.first,
                p.last,
                opt_str(&p.nick),
                p.dob,
                p.birth_year,
                p.height,
                p.weight,
                opt_str(&p.hand),
                opt_str(&p.nationality),
                opt_str(&p.birthcity),
                opt_str(&p.birthstate),
                p.retired as i64,
                p.first_year,
                p.last_year,
                p.season_count,
                p.is_goalie as i64,
                opt_str(&p.position),
            ])?;
        }
    }

    // Corpus metadata.
    write_meta(&tx, "schema_version", &SCHEMA_VERSION.to_string())?;
    write_meta(&tx, "source_root", &root.display().to_string())?;
    write_meta(&tx, "season_count", &seasons.len().to_string())?;
    write_meta(&tx, "person_count", &persons.len().to_string())?;
    write_meta(&tx, "season_min", &seasons.last().unwrap().0.to_string())?;
    write_meta(&tx, "season_max", &seasons.first().unwrap().0.to_string())?;

    tx.commit()?;
    // Indices are declared in the schema; a final ANALYZE helps the planner.
    conn.execute_batch("ANALYZE;")?;

    eprintln!("done: {} persons -> {}", persons.len(), out);
    Ok(())
}

/// Ingest a season's reference tables (leagues, teams, conferences, divisions) and populate the
/// in-memory resolution maps.
fn ingest_reference(
    tx: &Connection,
    year: i64,
    dir: &Path,
    team_ref: &mut HashMap<(i64, i64), (String, String, i64)>,
    league_ref: &mut HashMap<(i64, i64), String>,
) -> Res<()> {
    // Leagues
    if let Some((h, rows)) = read_table(&dir.join("league_data.csv"))? {
        let mut ins = tx.prepare(
            "INSERT OR IGNORE INTO league_season(season_year, src_league_id, name, abbr)
             VALUES(?1,?2,?3,?4)",
        )?;
        for r in &rows {
            let id = i0(g(&h, r, "LeagueId"));
            let name = g(&h, r, "Name").to_string();
            let abbr = g(&h, r, "Abbr").to_string();
            ins.execute(params![year, id, opt_str(&name), opt_str(&abbr)])?;
            league_ref.insert((year, id), if abbr.is_empty() { name } else { abbr });
        }
    }
    // Conferences
    if let Some((h, rows)) = read_table(&dir.join("conferences.csv"))? {
        let mut ins = tx.prepare(
            "INSERT OR IGNORE INTO conference_season(season_year, league_id, conference_id, name)
             VALUES(?1,?2,?3,?4)",
        )?;
        for r in &rows {
            ins.execute(params![
                year,
                i0(g(&h, r, "League Id")),
                i0(g(&h, r, "Conference Id")),
                opt_str(g(&h, r, "Name")),
            ])?;
        }
    }
    // Divisions
    if let Some((h, rows)) = read_table(&dir.join("divisions.csv"))? {
        let mut ins = tx.prepare(
            "INSERT OR IGNORE INTO division_season(season_year, league_id, conference_id, division_id, name)
             VALUES(?1,?2,?3,?4,?5)",
        )?;
        for r in &rows {
            ins.execute(params![
                year,
                i0(g(&h, r, "League Id")),
                i0(g(&h, r, "Conference Id")),
                i0(g(&h, r, "Division Id")),
                opt_str(g(&h, r, "Name")),
            ])?;
        }
    }
    // Teams
    if let Some((h, rows)) = read_table(&dir.join("team_data.csv"))? {
        let mut ins = tx.prepare(
            "INSERT OR IGNORE INTO team_season(season_year, src_team_id, league_id, name, nickname,
                abbr, primary_color, secondary_color, text_color, conference_id, division_id)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        )?;
        for r in &rows {
            let id = i0(g(&h, r, "TeamId"));
            let league = i0(g(&h, r, "LeagueId"));
            let name = g(&h, r, "Name").to_string();
            let abbr = g(&h, r, "Abbr").to_string();
            ins.execute(params![
                year,
                id,
                league,
                opt_str(&name),
                opt_str(g(&h, r, "Nickname")),
                opt_str(&abbr),
                opt_str(g(&h, r, "Primary Color")),
                opt_str(g(&h, r, "Secondary Color")),
                opt_str(g(&h, r, "Text Color")),
                i0(g(&h, r, "Conference Id")),
                i0(g(&h, r, "Division Id")),
            ])?;
            let disp_abbr = if abbr.is_empty() { name.clone() } else { abbr };
            team_ref.insert((year, id), (name, disp_abbr, league));
        }
    }
    Ok(())
}

/// Ingest one season's people: player_master + player_ratings → person/player_season/ratings_season,
/// then career/draft/award sourcing (newest-first, once per person).
#[allow(clippy::too_many_arguments)]
fn ingest_season(
    tx: &Connection,
    year: i64,
    dir: &Path,
    team_ref: &HashMap<(i64, i64), (String, String, i64)>,
    league_ref: &HashMap<(i64, i64), String>,
    persons: &mut Vec<Person>,
    slug_to_id: &mut HashMap<String, i64>,
    career_done: &mut HashSet<i64>,
    draft_done: &mut HashSet<i64>,
    award_done: &mut HashSet<i64>,
    contract_done: &mut HashSet<i64>,
) -> Res<()> {
    // Ratings for this season, keyed by src_player_id.
    let ratings = load_ratings(dir)?;

    // player_master → persons + player_season. Also builds this season's src_player_id → person_id.
    let (mh, mrows) = match read_table(&dir.join("player_master.csv"))? {
        Some(t) => t,
        None => return Ok(()),
    };
    let mut season_pid: HashMap<i64, i64> = HashMap::with_capacity(mrows.len());

    let mut ins_ps = tx.prepare(
        "INSERT OR IGNORE INTO player_season(person_id, season_year, src_player_id, team_id,
            franchise_id, height, weight, birthcity, birthstate, nat_one, nat_two, nat_three,
            retired, hand, rookie)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
    )?;
    let mut ins_rs = tx.prepare(
        "INSERT OR IGNORE INTO ratings_season(person_id, season_year, g, ld, rd, lw, c, rw,
            ability, potential, attrs_json)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
    )?;

    for r in &mrows {
        let src_id = i0(g(&mh, r, "PlayerId"));
        let last = g(&mh, r, "Last Name");
        let first = g(&mh, r, "First Name");
        let dob = g(&mh, r, "Date Of Birth");
        let slug = person_slug(last, first, dob);
        let rat = ratings.get(&src_id);

        // Assign / update the person.
        let pid = match slug_to_id.get(&slug) {
            Some(&pid) => {
                // Older season for a known person: widen the span, bump count.
                let p = &mut persons[pid as usize];
                p.first_year = p.first_year.min(year);
                p.season_count += 1;
                // Backfill position/goalie from the first season that actually has ratings.
                if p.position.is_empty() {
                    if let Some(rat) = rat {
                        p.position = rat.position.clone();
                        p.is_goalie = rat.is_goalie;
                    }
                }
                pid
            }
            None => {
                // First (newest) sighting = canonical bio.
                let pid = persons.len() as i64;
                let is_goalie = rat.map(|x| x.is_goalie).unwrap_or(false);
                let position = rat.map(|x| x.position.clone()).unwrap_or_default();
                // Source-data mojibake fix-up (see tdb_fhm::fixups) — only applies to the handful of
                // names we've verified against a clean record elsewhere.
                let (first, last) = tdb_fhm::known_name_fixup(&slug).unwrap_or((first, last));
                persons.push(Person {
                    slug: slug.clone(),
                    first: first.to_string(),
                    last: last.to_string(),
                    nick: g(&mh, r, "Nick Name").to_string(),
                    dob: tdb_fhm::slug::normalize_dob(dob),
                    birth_year: birth_year_of(dob),
                    height: opt_i(g(&mh, r, "Height")),
                    weight: opt_i(g(&mh, r, "Weight")),
                    hand: g(&mh, r, "Hand").to_string(),
                    nationality: g(&mh, r, "Nationality_One").to_string(),
                    birthcity: g(&mh, r, "Birthcity").to_string(),
                    birthstate: g(&mh, r, "Birthstate").to_string(),
                    retired: i0(g(&mh, r, "Retired")) != 0,
                    is_goalie,
                    position,
                    first_year: year,
                    last_year: year,
                    season_count: 1,
                });
                slug_to_id.insert(slug, pid);
                pid
            }
        };
        season_pid.insert(src_id, pid);

        // Per-season snapshot rows.
        ins_ps.execute(params![
            pid,
            year,
            src_id,
            opt_i(g(&mh, r, "TeamId")),
            opt_i(g(&mh, r, "FranchiseId")),
            opt_i(g(&mh, r, "Height")),
            opt_i(g(&mh, r, "Weight")),
            opt_str(g(&mh, r, "Birthcity")),
            opt_str(g(&mh, r, "Birthstate")),
            opt_str(g(&mh, r, "Nationality_One")),
            opt_str(g(&mh, r, "Nationality_Two")),
            opt_str(g(&mh, r, "Nationality_Three")),
            opt_i(g(&mh, r, "Retired")),
            opt_str(g(&mh, r, "Hand")),
            opt_i(g(&mh, r, "Rookie")),
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
                rat.attrs_json,
            ])?;
        }
    }
    drop(ins_ps);
    drop(ins_rs);

    // Careers — source each person once, from the newest season they carry career rows.
    ingest_careers(
        tx,
        year,
        dir,
        &season_pid,
        team_ref,
        league_ref,
        career_done,
    )?;
    // Draft origin — once per person, newest-first.
    ingest_draft(tx, year, dir, &season_pid, team_ref, draft_done)?;
    // Contract — once per person, newest-first.
    ingest_contract(tx, year, dir, &season_pid, team_ref, contract_done)?;
    // Awards — once per person, newest-first.
    ingest_awards(tx, year, dir, &season_pid, league_ref, award_done)?;
    Ok(())
}

/// A parsed ratings row.
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
    /// Primary position label derived from the position grades (C/LW/RW/D/G).
    position: String,
}

fn load_ratings(dir: &Path) -> Res<HashMap<i64, Ratings>> {
    let mut out = HashMap::new();
    let (h, rows) = match read_table(&dir.join("player_ratings.csv"))? {
        Some(t) => t,
        None => return Ok(out),
    };
    // Precompute the header list once for the attrs JSON.
    let cols: Vec<(String, usize)> = h.iter().map(|(k, v)| (k.clone(), *v)).collect();
    for r in &rows {
        let src = i0(g(&h, r, "PlayerId"));
        let gg = i0(g(&h, r, "G"));
        let ld = i0(g(&h, r, "LD"));
        let rd = i0(g(&h, r, "RD"));
        let lw = i0(g(&h, r, "LW"));
        let c = i0(g(&h, r, "C"));
        let rw = i0(g(&h, r, "RW"));
        // Primary position = whichever position carries the highest grade (FHM rates the natural
        // position 20). Ties break by priority G > C > LW > RW > D; D folds LD/RD. is_goalie ⟺ G won.
        let cand = [
            ("G", gg),
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
        // attrs JSON: every rating column except the id, values as numbers where possible.
        let mut map = serde_json::Map::new();
        for (name, idx) in &cols {
            if name == "PlayerId" {
                continue;
            }
            let v = r.get(*idx).unwrap_or("").trim();
            map.insert(name.clone(), json_num(v));
        }
        out.insert(
            src,
            Ratings {
                g: gg,
                ld,
                rd,
                lw,
                c,
                rw,
                ability: opt_f(g(&h, r, "Ability")),
                potential: opt_f(g(&h, r, "Potential")),
                attrs_json: serde_json::to_string(&map)?,
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
    dir: &Path,
    season_pid: &HashMap<i64, i64>,
    team_ref: &HashMap<(i64, i64), (String, String, i64)>,
    league_ref: &HashMap<(i64, i64), String>,
    career_done: &mut HashSet<i64>,
) -> Res<()> {
    // Collect all career rows for this season, grouped by person, for persons not yet sourced.
    // (kind, phase, file)
    let files = [
        ("skater", "rs", "player_skater_career_stats_rs.csv"),
        ("skater", "po", "player_skater_career_stats_po.csv"),
        ("skater", "rs", "player_skater_retired_career_stats_rs.csv"),
        ("skater", "po", "player_skater_retired_career_stats_po.csv"),
        ("skater", "ps", "player_skater_retired_career_stats_ps.csv"),
        ("goalie", "rs", "player_goalie_career_stats_rs.csv"),
        ("goalie", "po", "player_goalie_career_stats_po.csv"),
        ("goalie", "rs", "player_goalie_retired_career_stats_rs.csv"),
        ("goalie", "po", "player_goalie_retired_career_stats_po.csv"),
        ("goalie", "ps", "player_goalie_retired_career_stats_ps.csv"),
    ];

    // person_id -> rows to insert
    struct Row {
        kind: &'static str,
        phase: &'static str,
        year: i64,
        team_id: Option<i64>,
        league_id: Option<i64>,
        gp: Option<i64>,
        stats_json: String,
    }
    let mut pending: HashMap<i64, Vec<Row>> = HashMap::new();

    for (kind, phase, file) in files {
        if phase == "ps" {
            continue; // preseason files are header-only in the exports
        }
        let (h, rows) = match read_table(&dir.join(file))? {
            Some(t) => t,
            None => continue,
        };
        // Active files use "Team Id"/"League Id"; retired files use "TeamId"/"LeagueId".
        let team_key = if h.contains_key("Team Id") {
            "Team Id"
        } else {
            "TeamId"
        };
        let league_key = if h.contains_key("League Id") {
            "League Id"
        } else {
            "LeagueId"
        };
        let cols: Vec<(String, usize)> = h.iter().map(|(k, v)| (k.clone(), *v)).collect();

        for r in &rows {
            let src = i0(g(&h, r, "PlayerId"));
            let pid = match season_pid.get(&src) {
                Some(&p) if !career_done.contains(&p) => p,
                _ => continue,
            };
            let mut map = serde_json::Map::new();
            for (name, idx) in &cols {
                if name == "PlayerId" || name == team_key || name == league_key || name == "Year" {
                    continue;
                }
                map.insert(name.clone(), json_num(r.get(*idx).unwrap_or("").trim()));
            }
            pending.entry(pid).or_default().push(Row {
                kind,
                phase,
                year: i0(g(&h, r, "Year")),
                team_id: opt_i(g(&h, r, team_key)),
                league_id: opt_i(g(&h, r, league_key)),
                gp: opt_i(g(&h, r, "GP")),
                stats_json: serde_json::to_string(&map)?,
            });
        }
    }

    let mut ins = tx.prepare(
        "INSERT INTO career_stat(person_id, kind, phase, year, src_season, team_id, league_id,
            team_abbr, league_abbr, gp, stats_json)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
    )?;
    for (pid, rows) in pending {
        for row in rows {
            let team_abbr = row
                .team_id
                .and_then(|t| team_ref.get(&(year, t)).map(|x| x.1.clone()));
            let league_abbr = row
                .league_id
                .and_then(|l| league_ref.get(&(year, l)).cloned());
            ins.execute(params![
                pid,
                row.kind,
                row.phase,
                row.year,
                year,
                row.team_id,
                row.league_id,
                team_abbr,
                league_abbr,
                row.gp,
                row.stats_json,
            ])?;
        }
        career_done.insert(pid);
    }
    Ok(())
}

fn ingest_draft(
    tx: &Connection,
    year: i64,
    dir: &Path,
    season_pid: &HashMap<i64, i64>,
    team_ref: &HashMap<(i64, i64), (String, String, i64)>,
    draft_done: &mut HashSet<i64>,
) -> Res<()> {
    let (h, rows) = match read_table(&dir.join("draft_info.csv"))? {
        Some(t) => t,
        None => return Ok(()),
    };
    let mut ins = tx.prepare(
        "INSERT OR IGNORE INTO draft_info(person_id, year, round, pick, overall, src_season,
            team_id, team_abbr, draft_id, picked_from) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
    )?;
    // Keep EVERY draft row a person carries in this snapshot (NHL + junior/import), not just the first;
    // finalize per person only after the snapshot so all rows share one id namespace. FHM's `10000`
    // pick sentinel ("within-round unknown") is nulled — `overall` holds the true position.
    let mut seen: HashSet<i64> = HashSet::new();
    for r in &rows {
        let src = i0(g(&h, r, "PlayerId"));
        let pid = match season_pid.get(&src) {
            Some(&p) if !draft_done.contains(&p) => p,
            _ => continue,
        };
        let team_id = opt_i(g(&h, r, "Team"));
        let team_abbr = team_id.and_then(|t| team_ref.get(&(year, t)).map(|x| x.1.clone()));
        let pick = opt_i(g(&h, r, "Pick")).filter(|&p| p < 10000);
        ins.execute(params![
            pid,
            opt_i(g(&h, r, "Year")),
            opt_i(g(&h, r, "Round")),
            pick,
            opt_i(g(&h, r, "Overall")),
            year,
            team_id,
            team_abbr,
            opt_i(g(&h, r, "DraftId")),
            opt_i(g(&h, r, "Picked From")),
        ])?;
        seen.insert(pid);
    }
    draft_done.extend(seen);
    Ok(())
}

fn ingest_contract(
    tx: &Connection,
    year: i64,
    dir: &Path,
    season_pid: &HashMap<i64, i64>,
    team_ref: &HashMap<(i64, i64), (String, String, i64)>,
    contract_done: &mut HashSet<i64>,
) -> Res<()> {
    let (h, rows) = match read_table(&dir.join("player_contract.csv"))? {
        Some(t) => t,
        None => return Ok(()),
    };
    // "Major <year>" columns hold per-season salary; count those with a real value as the term.
    let major_cols: Vec<usize> = h
        .iter()
        .filter(|(k, _)| k.starts_with("Major "))
        .map(|(_, i)| *i)
        .collect();
    let mut ins = tx.prepare(
        "INSERT OR IGNORE INTO contract(person_id, team_abbr, ntc, nmc, elc, ufa, scholarship,
            avg_salary, length, src_season) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
    )?;
    for r in &rows {
        let src = i0(g(&h, r, "PlayerId"));
        let pid = match season_pid.get(&src) {
            Some(&p) if !contract_done.contains(&p) => p,
            _ => continue,
        };
        let team_id = opt_i(g(&h, r, "Team"));
        let team_abbr = team_id.and_then(|t| team_ref.get(&(year, t)).map(|x| x.1.clone()));
        let length = major_cols
            .iter()
            .filter(|&&i| opt_i(r.get(i).unwrap_or("")).is_some())
            .count() as i64;
        ins.execute(params![
            pid,
            team_abbr,
            yes_no(g(&h, r, "NTC")),
            yes_no(g(&h, r, "NMC")),
            yes_no(g(&h, r, "ELC")),
            yes_no(g(&h, r, "UFA")),
            opt_str(g(&h, r, "Scholarship")).filter(|s| s != "-"),
            opt_i(g(&h, r, "Average Salary")),
            length,
            year,
        ])?;
        contract_done.insert(pid);
    }
    Ok(())
}

fn ingest_awards(
    tx: &Connection,
    year: i64,
    dir: &Path,
    season_pid: &HashMap<i64, i64>,
    league_ref: &HashMap<(i64, i64), String>,
    award_done: &mut HashSet<i64>,
) -> Res<()> {
    let (h, rows) = match read_table(&dir.join("league_awards.csv"))? {
        Some(t) => t,
        None => return Ok(()),
    };
    // Group by person so we only source a person's awards from their newest appearance.
    let mut by_pid: HashMap<i64, Vec<&StringRecord>> = HashMap::new();
    for r in &rows {
        let src = i0(g(&h, r, "PlayerId"));
        if let Some(&pid) = season_pid.get(&src) {
            if !award_done.contains(&pid) {
                by_pid.entry(pid).or_default().push(r);
            }
        }
    }
    let mut ins = tx.prepare(
        "INSERT INTO award(person_id, year, league_abbr, name, description)
         VALUES(?1,?2,?3,?4,?5)",
    )?;
    for (pid, recs) in by_pid {
        for r in recs {
            let league_id = i0(g(&h, r, "LeagueId"));
            let league_abbr = league_ref.get(&(year, league_id)).cloned();
            ins.execute(params![
                pid,
                opt_i(g(&h, r, "Year")),
                league_abbr,
                opt_str(g(&h, r, "Award Name")),
                opt_str(g(&h, r, "Award Description")),
            ])?;
        }
        award_done.insert(pid);
    }
    Ok(())
}

// ---- helpers ---------------------------------------------------------------

/// Discover `YYYY-YYYY.lg` season folders; the season's key is its **end** year.
fn discover_seasons(root: &Path) -> Res<Vec<(i64, PathBuf)>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let stem = name.strip_suffix(".lg").unwrap_or(&name);
        // "1923-1924" -> end year 1924
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

/// Open a semicolon CSV; return `None` if the file is missing or header-only.
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

/// Parse an int, defaulting absent to 0 (for id keys where 0 is a real namespace value).
fn i0(s: &str) -> i64 {
    let t = s.trim();
    if t.is_empty() || t == "-" {
        return 0;
    }
    t.parse().unwrap_or(0)
}

fn opt_f(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() || t == "-" {
        None
    } else {
        t.parse().ok()
    }
}

/// FHM "Yes"/"No" contract clause → integer bool (1/0).
fn yes_no(s: &str) -> i64 {
    if s.trim().eq_ignore_ascii_case("yes") {
        1
    } else {
        0
    }
}

/// Empty string → SQL NULL.
fn opt_str(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// A CSV cell as a JSON value: number when it parses, else string, else null for empty.
fn json_num(v: &str) -> serde_json::Value {
    if v.is_empty() || v == "-" {
        return serde_json::Value::Null;
    }
    if let Ok(i) = v.parse::<i64>() {
        return serde_json::Value::from(i);
    }
    if let Ok(f) = v.parse::<f64>() {
        return serde_json::json!(f);
    }
    serde_json::Value::from(v)
}

fn birth_year_of(dob: &str) -> Option<i64> {
    dob.split('-').next().and_then(|y| y.trim().parse().ok())
}

fn write_meta(tx: &Connection, key: &str, value: &str) -> Res<()> {
    tx.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES(?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}
