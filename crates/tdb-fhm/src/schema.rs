//! The versioned corpus schema. `tdb-fhm-build` executes [`SCHEMA_SQL`] to create the corpus;
//! `tdb-fhm-store` checks [`SCHEMA_VERSION`] in the `meta` table before reading. Bump the version
//! whenever the shape changes so a stale data pack fails fast instead of mis-reading.
//!
//! Grain: the corpus is **person-centric**. `person` is one row per human (unified across seasons by
//! the [`crate::slug`] rule); `player_season` / `ratings_season` carry the per-season snapshots;
//! `career_stat` is the deduped career; `team_season` / `league_season` / `conference_season` /
//! `division_season` are per-season reference tables so namespaced ids resolve to names.

/// Corpus schema version — written into `meta(schema_version)` at build, verified at open.
/// v2 adds `person.position`; v3 adds the person-level `contract` table; v4 brings the corpus to
/// full parity with the source DB: cross-season team identity (`team`, `team_season.team_key`),
/// affiliates (`team_parent`), jersey numbers, captaincy, per-season contracts + contract years,
/// player rights, the schedule, and draft names — everything the prior `.db` carried. v5 adds the
/// NHL franchise lineage (`franchise`, `franchise_member`) that consolidates relocations/renames
/// into one entry per franchise with an era-specific display name (ported from the Hockey-Legends
/// lineage, mapped onto our `team_key`s). v6 makes `draft_info` **multi-row per person** (every league
/// draft a player was selected in, not just the first one encountered), nulls FHM's `10000` pick
/// sentinel at ingest, and resolves the era-appropriate drafting-team `name` via the franchise lineage.
pub const SCHEMA_VERSION: i64 = 6;

/// Full DDL for a fresh corpus. Executed as one batch by the build binary.
pub const SCHEMA_SQL: &str = r#"
PRAGMA journal_mode = OFF;
PRAGMA synchronous = OFF;

CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- One row per human, unified across seasons by `slug` = normalized last|first|dob.
CREATE TABLE person (
    person_id     INTEGER PRIMARY KEY,
    slug          TEXT NOT NULL UNIQUE,
    first_name    TEXT NOT NULL,
    last_name     TEXT NOT NULL,
    nick_name     TEXT,
    dob           TEXT NOT NULL,          -- YYYY-MM-DD (or '?')
    birth_year    INTEGER,
    height        INTEGER,                -- inches, from the person's latest season
    weight        INTEGER,                -- pounds, from the person's latest season
    hand          TEXT,
    nationality   TEXT,                   -- primary nationality, latest season
    birthcity     TEXT,
    birthstate    TEXT,
    retired       INTEGER NOT NULL DEFAULT 0,
    first_year    INTEGER,                -- earliest season the person appears
    last_year     INTEGER,                -- latest season the person appears
    season_count  INTEGER NOT NULL DEFAULT 0,
    is_goalie     INTEGER NOT NULL DEFAULT 0,
    position      TEXT                    -- derived primary position: C/LW/RW/D/G (null if no ratings)
);
CREATE INDEX idx_person_last ON person(last_name, first_name);
CREATE INDEX idx_person_year ON person(last_year);

-- Per-season snapshot of a person's roster row (maps a person back to a season export's raw ids).
CREATE TABLE player_season (
    person_id     INTEGER NOT NULL REFERENCES person(person_id),
    season_year   INTEGER NOT NULL,       -- end year, e.g. 2024 for 2023-2024
    src_player_id INTEGER NOT NULL,
    team_id       INTEGER,                -- namespaced to season_year (-1 = none)
    franchise_id  INTEGER,
    height        INTEGER,
    weight        INTEGER,
    birthcity     TEXT,
    birthstate    TEXT,
    nat_one       TEXT,
    nat_two       TEXT,
    nat_three     TEXT,
    retired       INTEGER,
    hand          TEXT,
    rookie        INTEGER,
    PRIMARY KEY (person_id, season_year)
);
CREATE INDEX idx_pseason_season ON player_season(season_year);
CREATE INDEX idx_pseason_team ON player_season(season_year, team_id);

-- Per-season ratings. Position grades + headline ability/potential are columns (for sort/filter);
-- the full ~50-attribute line is JSON (name -> value) so the schema is stable as attributes drift.
CREATE TABLE ratings_season (
    person_id     INTEGER NOT NULL REFERENCES person(person_id),
    season_year   INTEGER NOT NULL,
    g INTEGER, ld INTEGER, rd INTEGER, lw INTEGER, c INTEGER, rw INTEGER,
    ability       REAL,
    potential     REAL,
    attrs_json    TEXT NOT NULL,
    PRIMARY KEY (person_id, season_year)
);

-- Deduped career (across active + retired export files). team/league resolved to names within the
-- source season's namespace at build time (human-readable), raw ids kept for reference.
CREATE TABLE career_stat (
    person_id     INTEGER NOT NULL REFERENCES person(person_id),
    kind          TEXT NOT NULL,          -- 'skater' | 'goalie'
    phase         TEXT NOT NULL,          -- 'rs' | 'po'
    year          INTEGER NOT NULL,
    src_season    INTEGER NOT NULL,       -- export the row was taken from (id namespace)
    team_id       INTEGER,
    league_id     INTEGER,
    team_abbr     TEXT,
    league_abbr   TEXT,
    gp            INTEGER,
    stats_json    TEXT NOT NULL
);
CREATE INDEX idx_career_person ON career_stat(person_id, kind, phase, year);

-- Stable cross-season team/franchise identity (source `teams` + `leagues`). `team_key` unifies a
-- club across seasons the way `person.slug` unifies a human, backing the all-seasons team browse.
CREATE TABLE team (
    team_key      TEXT PRIMARY KEY,       -- "<league_key>__<team_slug>"
    name          TEXT,
    nickname      TEXT,
    abbr          TEXT,
    league_key    TEXT,
    league_abbr   TEXT,
    league_name   TEXT
);

-- Per-season reference tables (id -> name), namespaced by season_year.
CREATE TABLE team_season (
    season_year   INTEGER NOT NULL,
    src_team_id   INTEGER NOT NULL,
    team_key      TEXT,                   -- cross-season identity (-> team.team_key)
    league_id     INTEGER,
    name          TEXT,
    nickname      TEXT,
    abbr          TEXT,
    primary_color TEXT,
    secondary_color TEXT,
    text_color    TEXT,
    conference_id INTEGER,
    division_id   INTEGER,
    PRIMARY KEY (season_year, src_team_id)
);
CREATE INDEX idx_team_season ON team_season(season_year);
CREATE INDEX idx_team_season_key ON team_season(team_key);

-- Team affiliates: a team's parent club(s) that season (source `season_team_parents`).
CREATE TABLE team_parent (
    season_year    INTEGER NOT NULL,
    src_team_id    INTEGER NOT NULL,
    parent_team_id INTEGER NOT NULL,      -- season-namespaced team id
    PRIMARY KEY (season_year, src_team_id, parent_team_id)
);

CREATE TABLE league_season (
    season_year   INTEGER NOT NULL,
    src_league_id INTEGER NOT NULL,
    name          TEXT,
    abbr          TEXT,
    PRIMARY KEY (season_year, src_league_id)
);

CREATE TABLE conference_season (
    season_year   INTEGER NOT NULL,
    league_id     INTEGER NOT NULL,
    conference_id INTEGER NOT NULL,
    name          TEXT,
    PRIMARY KEY (season_year, league_id, conference_id)
);

CREATE TABLE division_season (
    season_year   INTEGER NOT NULL,
    league_id     INTEGER NOT NULL,
    conference_id INTEGER NOT NULL,
    division_id   INTEGER NOT NULL,
    name          TEXT,
    PRIMARY KEY (season_year, league_id, conference_id, division_id)
);

-- Person-linked draft origins — EVERY league draft the person was selected in (NHL entry, plus junior
-- /import drafts like the QMJHL/OHL/CHL Import), all sourced from their newest appearance so one draft
-- id namespace applies. `pick` has FHM's `10000` "pick-within-round unknown" sentinel nulled at ingest
-- (`overall` carries the true position). `team_abbr` is the snapshot-era abbr (raw fallback); `team_name`
-- is the era-resolved drafting-team name (franchise lineage at the draft year), NULL when unresolved.
CREATE TABLE draft_info (
    person_id     INTEGER NOT NULL REFERENCES person(person_id),
    year          INTEGER,
    round         INTEGER,
    pick          INTEGER,
    overall       INTEGER,
    src_season    INTEGER,
    team_id       INTEGER,
    team_abbr     TEXT,
    team_name     TEXT,                   -- era-resolved drafting-team name (else NULL -> use team_abbr)
    draft_id      INTEGER,                -- which draft (-> season_draft.draft_id)
    picked_from   INTEGER,                -- team the pick was acquired from (trade origin)
    PRIMARY KEY (person_id, draft_id)
);
CREATE INDEX idx_draft_person ON draft_info(person_id);

-- Draft names, namespaced by season export (source `season_drafts`).
CREATE TABLE season_draft (
    season_year   INTEGER NOT NULL,
    draft_id      INTEGER NOT NULL,
    name          TEXT,
    PRIMARY KEY (season_year, draft_id)
);

-- Person-linked contract (one per person, from their latest appearance).
CREATE TABLE contract (
    person_id     INTEGER PRIMARY KEY REFERENCES person(person_id),
    team_abbr     TEXT,
    ntc           INTEGER,               -- no-trade clause (bool)
    nmc           INTEGER,               -- no-movement clause (bool)
    elc           INTEGER,               -- entry-level contract (bool)
    ufa           INTEGER,               -- unrestricted free agent flag (bool)
    scholarship   TEXT,
    avg_salary    INTEGER,               -- FHM "Average Salary" (thousands)
    length        INTEGER,               -- years remaining (count of Major years with a salary)
    src_season    INTEGER
);

-- Person-linked awards (many per person).
CREATE TABLE award (
    person_id     INTEGER NOT NULL REFERENCES person(person_id),
    year          INTEGER,
    league_abbr   TEXT,
    name          TEXT,
    description   TEXT
);
CREATE INDEX idx_award_person ON award(person_id);

-- Jersey numbers a person wore for a franchise in a season (source `player_jersey_numbers`,
-- resolved to person + team_key). A player can carry more than one number in a season.
CREATE TABLE jersey_number (
    person_id     INTEGER NOT NULL REFERENCES person(person_id),
    team_key      TEXT,
    season_year   INTEGER,
    number        INTEGER NOT NULL,
    PRIMARY KEY (person_id, team_key, season_year, number)
);
CREATE INDEX idx_jersey_team ON jersey_number(team_key, season_year);

-- Captain / alternate designations (source `team_captaincy`, resolved to person + team_key).
CREATE TABLE captaincy (
    person_id     INTEGER NOT NULL REFERENCES person(person_id),
    team_key      TEXT,
    season_year   INTEGER,
    role          TEXT NOT NULL           -- 'C' | 'A'
);
CREATE INDEX idx_captaincy_team ON captaincy(team_key, season_year);

-- Per-season contract snapshot (source `season_player_contracts`). Unlike the person-level
-- `contract` (latest only), this keeps every season's contract so a team's cap sheet resolves.
CREATE TABLE season_contract (
    person_id     INTEGER NOT NULL REFERENCES person(person_id),
    season_year   INTEGER NOT NULL,
    team_id       INTEGER,                -- season-namespaced team id
    ntc           INTEGER,
    nmc           INTEGER,
    elc           INTEGER,
    ufa           INTEGER,
    scholarship   TEXT,
    avg_salary    INTEGER,                -- thousands
    PRIMARY KEY (person_id, season_year)
);
CREATE INDEX idx_season_contract_team ON season_contract(season_year, team_id);

-- Per-year salary breakdown within a season's contract (source `season_player_contract_years`).
CREATE TABLE season_contract_year (
    person_id     INTEGER NOT NULL REFERENCES person(person_id),
    season_year   INTEGER NOT NULL,
    year          INTEGER NOT NULL,
    major_salary  INTEGER,
    minor_salary  INTEGER,
    PRIMARY KEY (person_id, season_year, year)
);

-- Rights holder for a person that season (source `season_player_rights`).
CREATE TABLE player_rights (
    person_id     INTEGER NOT NULL REFERENCES person(person_id),
    season_year   INTEGER NOT NULL,
    league_id     INTEGER,
    team_id       INTEGER,
    PRIMARY KEY (person_id, season_year)
);

-- League schedule + results (source `season_schedule_games`).
CREATE TABLE schedule_game (
    season_year   INTEGER NOT NULL,
    league_id     INTEGER,
    game_date     TEXT,
    home_team_id  INTEGER,
    score_home    INTEGER,
    away_team_id  INTEGER,
    score_away    INTEGER,
    game_type     TEXT,
    played        INTEGER,
    overtime      INTEGER,
    shootout      INTEGER,
    game_id       INTEGER
);
CREATE INDEX idx_schedule_season ON schedule_game(season_year, league_id);

-- NHL franchise lineage: consolidates relocations/renames into one canonical entry. Ported from the
-- Hockey-Legends `seed_franchise_lineage.py` and mapped onto our `team_key`s. Only NHL-lineage teams
-- are members; every other team stays standalone (no franchise row).
CREATE TABLE franchise (
    franchise_id  INTEGER PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE,   -- canonical key, e.g. "Calgary Flames org"
    display_name  TEXT NOT NULL,          -- "Calgary Flames"
    abbr          TEXT,
    is_defunct    INTEGER NOT NULL DEFAULT 0,
    notes         TEXT
);

-- Which of our teams (by `team_key`) belong to a franchise, in which era, under which era-name.
-- A team-season belongs to the franchise when `era_start < season_year <= era_end` (era_end NULL =
-- open). `era_name` is the historical name for that era (our corpus can't supply it). Multiple
-- members can cover one season (concurrent constituents, e.g. Seals + North Stars).
CREATE TABLE franchise_member (
    franchise_id  INTEGER NOT NULL REFERENCES franchise(franchise_id),
    team_key      TEXT NOT NULL,
    era_start     INTEGER,                -- exclusive lower bound (season start year)
    era_end       INTEGER,                -- inclusive upper bound; NULL = open
    era_name      TEXT,
    PRIMARY KEY (franchise_id, team_key, era_start)
);
CREATE INDEX idx_franchise_member_key ON franchise_member(team_key);
"#;

/// Post-build pass: fill `draft_info.team_name` with the era-appropriate drafting-team name via the
/// franchise lineage. FHM records the draft team as the *snapshot-season* id — i.e. the franchise's
/// modern club — so the raw abbr shows the current identity (Quebec's 1989 picks read "COL"). We
/// recover the historical name in two steps: **(m1)** pin the franchise by the member era covering the
/// SNAPSHOT season (this disambiguates city `team_key`s reused by unrelated franchises — e.g. `nhl__col`
/// belongs to both the Colorado Rockies→Devils lineage and the Avalanche lineage), then **(m2)** take
/// the era name of that same franchise covering the DRAFT year. Rows with no franchise lineage
/// (non-NHL drafts, or the CSV-path corpus with no `franchise_member` rows) are left NULL → the store
/// falls back to `team_abbr`. Run after `team_season` and `franchise_member` are populated.
pub const DRAFT_TEAM_NAME_SQL: &str = r#"
UPDATE draft_info SET team_name = (
    SELECT m2.era_name
    FROM team_season ts
    JOIN franchise_member m1 ON m1.team_key = ts.team_key
        AND (m1.era_start IS NULL OR ts.season_year > m1.era_start)
        AND (m1.era_end   IS NULL OR ts.season_year <= m1.era_end)
    JOIN franchise_member m2 ON m2.franchise_id = m1.franchise_id
        AND (m2.era_start IS NULL OR draft_info.year > m2.era_start)
        AND (m2.era_end   IS NULL OR draft_info.year <= m2.era_end)
    WHERE ts.season_year = draft_info.src_season
      AND ts.src_team_id = draft_info.team_id
    ORDER BY m2.era_name
    LIMIT 1
)
WHERE team_id IS NOT NULL AND year IS NOT NULL;
"#;
