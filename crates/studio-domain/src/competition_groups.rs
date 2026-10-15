//! Competition groups (Phase 1.2): the per-title table the NHL 12 → Legacy executables index with
//! the team row's global group ids `leaguegroup` / `conferencegroup` / `divisiongroup`, and — since
//! P1-C — the PS4 NHL 15 → 21 executables' pointer array of dictionary keys that the same three
//! team-row fields index. The tables are baked in `defs/leagues/groups/<game>.toml` (gen7, research
//! handoff P1.2-X) and `defs/leagues/groups/ps4-<game>.toml` (PS4, handoff P1.2-PS4; both
//! 2026-09-18) by the repository tool `competition-groups` (`crates/studio-api/src/bin`). Each
//! title has its own numbering, so a group id never transfers between titles, and a membership
//! change must pick the three ids from *this* title's table.
//!
//! Two evidence layers live in a table and are kept apart:
//!
//! * the **key lookup** — which dictionary key each id names, read from the executable
//!   (`evidence`: `traced-static` / `inferred` / `unknown` per row);
//! * the **witnessed structure** — which (conference, division) pairs each league block's stock
//!   rows carry (`[[pairs]]`, derived by the tool from the witness database and marked as such).
//!   Membership validation accepts only a witnessed *regular* arrangement; exceptional stock
//!   arrangements (an All-Star squad carrying a conference id in its division slot, a custom row
//!   pointing into another block) are recorded with the rows that carry them, browse as they are,
//!   and are never offered as a choice.
//!
//! Selection is bound to provenance (R1 F2): a table names the league `source` it was witnessed
//! on and its platforms, records the **identities** of the databases it was measured on (base and
//! known patches) and a **structural signature** (table set and capacities, which no edit changes).
//! A recorded identity verifies the title; otherwise the signature must identify it, and an
//! ambiguous or contradicting structure never gains another title's labels. Row agreement
//! (`FitReport`) is reported as compatibility information only.
//!
//! Labels: with the title's own dictionary attached the label is the dictionary text, else the
//! observed English `text`, always with the key disclosed. A group with no key is shown as an
//! unnamed group with its id; its `hypothesis_key` is a hint for progressive disclosure, never the
//! name. Which screens read these ids is runtime evidence, not established here.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use tdb_core::TdbFile;

use crate::leagues::{field_by_studio, resolve_text, AttachedDictionary, LeagueError, LeagueSource, Resolved};

/// Per-row evidence classes (the research vocabulary, carried verbatim).
pub const EVIDENCE_TRACED_STATIC: &str = "traced-static";
pub const EVIDENCE_INFERRED: &str = "inferred";
pub const EVIDENCE_UNKNOWN: &str = "unknown";

pub const ARRANGEMENT_REGULAR: &str = "regular";
pub const ARRANGEMENT_EXCEPTIONAL: &str = "exceptional";

/// `defs/leagues/groups/<game>.toml`.
#[derive(Clone, Debug, Deserialize)]
pub struct GroupTable {
    pub game: String,
    pub applies_to: Vec<String>,
    /// Platforms the table applies to (`ps3` / `xbox` / `ps4` / `recomp`); empty = any. A PS4
    /// table shares its game id with the Xbox/PS3 table of the same title, so this is what keeps
    /// them apart when the platform is known.
    #[serde(default)]
    pub platforms: Vec<String>,
    /// The league source the table was witnessed on: `stockleagues` (PS4) or `derived-codes`
    /// (gen7, the default). Bound to the open database's schema, so a PS4 table never applies to
    /// a gen7 database of the same title and vice versa even without a platform.
    #[serde(default = "default_source")]
    pub source: String,
    pub fields: GroupFields,
    pub witness: GroupWitness,
    /// Structural signature of the witness database (title identity independent of edits).
    #[serde(default)]
    pub signature: Option<Signature>,
    /// Database identities known to belong to this title (base witness, known patches).
    #[serde(default)]
    pub known_databases: Vec<KnownDatabase>,
    pub groups: Vec<GroupDef>,
    /// The (conference, division) pairs the witness rows carry per league block.
    #[serde(default)]
    pub pairs: Vec<PairDef>,
}

fn default_source() -> String {
    "derived-codes".to_string()
}

#[derive(Clone, Debug, Deserialize)]
pub struct GroupFields {
    /// Studio field id on the team table carrying the league group id.
    pub league: String,
    pub conference: String,
    pub division: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GroupWitness {
    pub game: String,
    pub platform: String,
    pub source: String,
    /// PS4: the install the table was read from and the files it was cross-checked against.
    #[serde(default)]
    pub cusa: Option<String>,
    #[serde(default)]
    pub executable_sha256: Option<String>,
    #[serde(default)]
    pub database_sha256: Option<String>,
    #[serde(default)]
    pub dictionary_sha256: Option<String>,
    #[serde(default)]
    pub param_sfo_versions: Vec<String>,
    /// Provenance caveats (patch databases, anchor identity, key wording).
    #[serde(default)]
    pub notes: Vec<String>,
}

/// What identifies a title's database structurally: the set of tables and the capacities of the
/// team (and, on PS4, the league) table. None of these change under Studio's edits (no row
/// creation on measured titles), and they differ between every measured title.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Signature {
    pub tables: usize,
    /// SHA-256 of the sorted canonical table ids joined by `,`.
    pub table_set_sha256: String,
    pub team_max_records: u32,
    #[serde(default)]
    pub stockleagues_max_records: Option<u32>,
}

impl Signature {
    /// Measure `db`.
    pub fn of(db: &TdbFile) -> Option<Self> {
        let mut ids: Vec<String> = db.tables.iter().map(|t| t.id.canonical(db.endian)).collect();
        ids.sort();
        let team = db.table_canonical(tdb_relations::entity::ids::TEAM)?;
        Some(Signature {
            tables: db.tables.len(),
            table_set_sha256: hex(&Sha256::digest(ids.join(",").as_bytes())),
            team_max_records: team.max_records as u32,
            stockleagues_max_records: db.table_canonical(crate::leagues::STOCK_LEAGUES).map(|t| t.max_records as u32),
        })
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Clone, Debug, Deserialize)]
pub struct KnownDatabase {
    /// `base` / `patch` (identities of this title) / `excluded` (a recorded file that is not).
    pub role: String,
    pub sha256: String,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GroupDef {
    pub id: u64,
    /// Localization key; empty when unknown (`evidence = "unknown"`).
    #[serde(default)]
    pub key: String,
    /// `league` / `conference` / `division` / `pool` / `tournament` / `unused`. A `pool` is a
    /// tournament group the title carries in both the conference and the division slot.
    pub kind: String,
    /// The first league code (team-table `xMjj`) this group belongs to; `-1` when none.
    pub league_code: i64,
    /// Every league code whose rows carry this group's block (several `stockleagues` rows share
    /// one block on PS4: the Custom Teams leagues, Alumni / All-Time). Defaults to
    /// `[league_code]`.
    #[serde(default)]
    pub leagues: Option<Vec<i64>>,
    #[serde(default)]
    pub logo_index: u64,
    /// English text observed in the title's own dictionary at research time.
    #[serde(default)]
    pub text: String,
    /// Whether any witnessed team row carries the id.
    pub used: bool,
    /// `traced-static` / `inferred` / `unknown`; absent on gen7 tables = `traced-static`.
    #[serde(default)]
    pub evidence: Option<String>,
    /// Why the row is inferred or unknown, in the research's words.
    #[serde(default)]
    pub note: Option<String>,
    /// Unknown rows only: the research's candidate key — a hint, never the label.
    #[serde(default)]
    pub hypothesis_key: Option<String>,
}

impl GroupDef {
    pub fn league_codes(&self) -> Vec<u64> {
        match &self.leagues {
            Some(v) => v.iter().filter_map(|&c| u64::try_from(c).ok()).collect(),
            None => u64::try_from(self.league_code).ok().into_iter().collect(),
        }
    }

    pub fn evidence(&self) -> &str {
        self.evidence.as_deref().unwrap_or(EVIDENCE_TRACED_STATIC)
    }
}

/// One (conference, division) arrangement the witness rows carry under a league block.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PairDef {
    /// The block's league group id (the rows' `leaguegroup`).
    pub head: u64,
    pub conference: u64,
    pub division: u64,
    /// Witness rows carrying exactly this triple.
    pub rows: u32,
    /// `regular` (a choosable arrangement) / `exceptional` (recorded, never offered).
    pub arrangement: String,
    /// League codes of the rows carrying it.
    #[serde(default)]
    pub leagues: Vec<u64>,
    /// Exceptional only: the team ids carrying it.
    #[serde(default)]
    pub teams: Vec<u64>,
    #[serde(default)]
    pub note: Option<String>,
}

impl GroupTable {
    pub fn from_toml(text: &str) -> Result<Self, LeagueError> {
        let t: GroupTable = toml::from_str(text).map_err(|e| LeagueError::Parse(e.to_string()))?;
        for g in &t.groups {
            if let Some(e) = g.evidence.as_deref() {
                if ![EVIDENCE_TRACED_STATIC, EVIDENCE_INFERRED, EVIDENCE_UNKNOWN].contains(&e) {
                    return Err(LeagueError::Parse(format!(
                        "{}: group {} has evidence {e:?} (expected traced-static / inferred / unknown)",
                        t.game, g.id
                    )));
                }
            }
            if g.key.is_empty() && g.evidence() != EVIDENCE_UNKNOWN && g.kind != "unused" {
                return Err(LeagueError::Parse(format!(
                    "{}: group {} has no key but is not marked unknown",
                    t.game, g.id
                )));
            }
        }
        for p in &t.pairs {
            if ![ARRANGEMENT_REGULAR, ARRANGEMENT_EXCEPTIONAL].contains(&p.arrangement.as_str()) {
                return Err(LeagueError::Parse(format!(
                    "{}: pair {}/{}/{} has arrangement {:?}",
                    t.game, p.head, p.conference, p.division, p.arrangement
                )));
            }
        }
        Ok(t)
    }

    /// Load every `*.toml` under `dir` (a missing directory yields none).
    pub fn load_dir(dir: &Path) -> Result<Vec<GroupTable>, LeagueError> {
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .map_err(LeagueError::Io)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        paths.sort();
        let mut out = Vec::new();
        for p in paths {
            let text = std::fs::read_to_string(&p).map_err(LeagueError::Io)?;
            out.push(Self::from_toml(&text)?);
        }
        Ok(out)
    }

    /// Whether the table is for `game_id` at all (any platform / source).
    pub fn applies(&self, game_id: &str) -> bool {
        self.applies_to.iter().any(|g| g == game_id)
    }

    /// Whether the table applies to this target: game id, the league source the open database
    /// presents, and (when known) the platform.
    pub fn applies_to_target(
        &self,
        game_id: &str,
        source: LeagueSource,
        platform: Option<crate::Platform>,
    ) -> bool {
        self.applies(game_id) && self.source_and_platform_ok(source, platform)
    }

    pub fn source_and_platform_ok(&self, source: LeagueSource, platform: Option<crate::Platform>) -> bool {
        let source_ok = match source {
            LeagueSource::StockLeagues => self.source == "stockleagues",
            LeagueSource::DerivedCodes => self.source != "stockleagues",
        };
        let platform_ok = match platform {
            Some(p) => self.platforms.is_empty() || self.platforms.iter().any(|x| x == p.as_str()),
            None => true,
        };
        source_ok && platform_ok
    }
}

// ---------------------------------------------------------------------------------------------
// Witnessed structure: derived from a database's team rows by the bake tool (and by tests).
// ---------------------------------------------------------------------------------------------

/// Derive the witnessed (conference, division) arrangements per league block from `db`'s team
/// rows, classified against `groups` (the table's rows). Deterministic: pairs come out ordered by
/// (head, conference, division).
///
/// A pair is **regular** when the conference slot holds one of the block's conferences or pools
/// (or the block's own id: a league without conferences) and the division slot one of its
/// divisions or pools (or the own id, then in both slots), with a pool matching in both slots;
/// anything else — a conference id in the division slot, a group of another block, an unknown
/// id — is **exceptional** and recorded with its rows.
pub fn witness_pairs(db: &TdbFile, fields: &GroupFields, groups: &[GroupDef]) -> Option<Vec<PairDef>> {
    let table = db.table_canonical(tdb_relations::entity::ids::TEAM)?;
    let f = |studio: &str| field_by_studio(db, table, studio);
    let (league_f, team_f) = (f(crate::leagues::TEAM_LEAGUE_FIELD)?, f(tdb_relations::entity::ids::TEAM_KEY)?);
    let (lg_f, conf_f, div_f) = (f(&fields.league)?, f(&fields.conference)?, f(&fields.division)?);
    let by_id: HashMap<u64, &GroupDef> = groups.iter().map(|g| (g.id, g)).collect();
    #[derive(Default)]
    struct Acc {
        rows: u32,
        leagues: BTreeSet<u64>,
        teams: Vec<u64>,
    }
    let mut acc: BTreeMap<(u64, u64, u64), Acc> = BTreeMap::new();
    for rec in 0..table.current_records as u32 {
        let (Some(code), Some(head), Some(conf), Some(div)) = (
            db.read_u64(table, rec, league_f),
            db.read_u64(table, rec, lg_f),
            db.read_u64(table, rec, conf_f),
            db.read_u64(table, rec, div_f),
        ) else {
            continue;
        };
        let a = acc.entry((head, conf, div)).or_default();
        a.rows += 1;
        a.leagues.insert(code);
        if let Some(t) = db.read_u64(table, rec, team_f) {
            a.teams.push(t);
        }
    }
    let mut out = Vec::new();
    for ((head, conf, div), a) in acc {
        let block: Vec<u64> = by_id.get(&head).map(|g| g.league_codes()).unwrap_or_default();
        let in_block = |id: u64| id == head || by_id.get(&id).is_some_and(|g| !block.is_empty() && g.league_codes().iter().any(|c| block.contains(c)));
        let kind = |id: u64| by_id.get(&id).map(|g| g.kind.as_str()).unwrap_or("");
        let head_is_league = kind(head) == "league";
        let conf_ok = conf == head || (in_block(conf) && matches!(kind(conf), "conference" | "pool"));
        let div_ok = div == head || (in_block(div) && matches!(kind(div), "division" | "pool"));
        // Pools match in both slots; the own id in the division slot needs the own id in the
        // conference slot too; the own id in the conference slot with one of the block's
        // divisions is a division-only league (QMJHL on every title) and is regular.
        let consistent = if kind(conf) == "pool" || kind(div) == "pool" || div == head { conf == div } else { true };
        let regular = head_is_league && conf_ok && div_ok && consistent;
        let note = if regular {
            None
        } else if !head_is_league {
            Some(format!("league group {head} is not a league-kind group of this table"))
        } else if kind(div) == "conference" {
            Some("a conference id carried in the division slot (All-Star / special squads)".to_string())
        } else if !in_block(conf) || !in_block(div) {
            Some("a group of another league's block".to_string())
        } else {
            Some("an arrangement the block's structure does not describe".to_string())
        };
        out.push(PairDef {
            head,
            conference: conf,
            division: div,
            rows: a.rows,
            arrangement: if regular { ARRANGEMENT_REGULAR } else { ARRANGEMENT_EXCEPTIONAL }.to_string(),
            leagues: a.leagues.into_iter().collect(),
            teams: if regular { Vec::new() } else { a.teams },
            note,
        });
    }
    Some(out)
}

// ---------------------------------------------------------------------------------------------
// Resolution for a session.
// ---------------------------------------------------------------------------------------------

/// One group as resolved for a session: label through the attached dictionary when possible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedGroup {
    pub id: u64,
    /// Empty when the key is unknown.
    pub key: String,
    pub kind: String,
    /// The first league code of the block (`league_codes[0]`), for display.
    pub league_code: Option<u64>,
    /// Every league code whose rows carry this block.
    pub league_codes: Vec<u64>,
    pub label: String,
    /// `dictionary` (text from the attached dictionary), `observed` (research text), `key`,
    /// `unknown` (no key: the label names the id only).
    pub label_source: &'static str,
    pub used: bool,
    /// `traced-static` / `inferred` / `unknown`.
    pub evidence: String,
    pub note: Option<String>,
    pub hypothesis_key: Option<String>,
}

impl ResolvedGroup {
    /// The label with its evidence caveat for text surfaces ("Spengler Cup (inferred)").
    pub fn label_with_evidence(&self) -> String {
        match self.evidence.as_str() {
            EVIDENCE_TRACED_STATIC => self.label.clone(),
            e => format!("{} ({e})", self.label),
        }
    }
}

/// How the open database's team rows agree with a table — compatibility information, reported
/// beside the provenance; never the basis of title identity.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FitReport {
    /// Rows whose `leaguegroup` is a league group of the table for the row's league code.
    pub fit: usize,
    /// Rows whose `leaguegroup` is a league group of the table, but of other league codes.
    pub conflict: usize,
    /// Rows whose `leaguegroup` is not in the table at all.
    pub unknown: usize,
    /// Rows whose league group is in the table but is not a league-kind group.
    pub misplaced: usize,
    /// League codes (with row counts) the table has no league group for.
    pub codes_without_group: Vec<(u64, usize)>,
}

impl FitReport {
    pub fn total(&self) -> usize {
        self.fit + self.conflict + self.unknown + self.misplaced
    }

    pub fn summary(&self) -> String {
        format!(
            "{} of {} team rows carry their league's group from this table ({} name another league's group, {} an id the table lacks)",
            self.fit,
            self.total(),
            self.conflict + self.misplaced,
            self.unknown
        )
    }
}

/// How the open database was tied to the table's title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Provenance {
    /// The database bytes are a recorded identity of this title (`role`, sha256 prefix).
    Recorded { role: String, sha256: String },
    /// Not a recorded identity (edited or unknown build), but the structural signature matches
    /// this title and no other baked title of the same source.
    Structural,
    /// The signature matches this title and the named other titles too: the claim cannot be
    /// verified from the file alone.
    Ambiguous { also: Vec<String> },
}

impl Provenance {
    pub fn describe(&self) -> String {
        match self {
            Provenance::Recorded { role, sha256 } => format!("recorded identity ({role} database {}…)", &sha256[..12.min(sha256.len())]),
            Provenance::Structural => "structure identifies this title (table set and capacities); file identity not recorded — an edited or unrecorded build".to_string(),
            Provenance::Ambiguous { also } => format!(
                "structure matches this title and also {}; the title claim is not verified by the file",
                also.join(", ")
            ),
        }
    }
}

/// The group table resolved for one session.
#[derive(Clone, Debug)]
pub struct ResolvedGroups {
    pub game: String,
    pub fields: GroupFields,
    /// Evidence class for this target: `traced` on the witnessed (game, platform) with a verified
    /// title, `by-reference` elsewhere (same database fields; executable not inspected, or the
    /// title not verified from the file).
    pub evidence: String,
    pub witness: String,
    /// Provenance caveats from the witness.
    pub witness_notes: Vec<String>,
    /// How the open database was tied to the title (set at attach time).
    pub provenance: Option<Provenance>,
    /// How the open database's rows agree with the table, when checked.
    pub fit: Option<FitReport>,
    pairs: Vec<PairDef>,
    by_id: HashMap<u64, ResolvedGroup>,
    ordered: Vec<u64>,
}

/// How a league's block is arranged on a title, from its witnessed regular pairs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeagueShape {
    /// The league's own id fills all three fields (the only witnessed arrangement).
    Single { league_group: u64 },
    /// Several witnessed arrangements: the choosable (conference, division) pairs.
    Structured { league_group: u64, pairs: Vec<(u64, u64)> },
}

impl ResolvedGroups {
    pub fn resolve(
        table: &GroupTable,
        game_id: &str,
        platform: Option<crate::Platform>,
        dictionary: Option<&AttachedDictionary>,
    ) -> Self {
        let witnessed = platform.is_some_and(|p| {
            table.witness.game == game_id && table.witness.platform == p.as_str()
        });
        let evidence = if witnessed {
            crate::leagues::KEY_EVIDENCE_TRACED
        } else {
            crate::leagues::KEY_EVIDENCE_BY_REFERENCE
        };
        let mut by_id = HashMap::new();
        let mut ordered = Vec::new();
        for g in &table.groups {
            let (label, label_source) = if g.key.is_empty() {
                (format!("Unnamed group {}", g.id), "unknown")
            } else {
                match dictionary.and_then(|d| resolve_text(d, &g.key)) {
                    Some((t, Resolved::Exact)) | Some((t, Resolved::ByHash(_))) => {
                        (t.to_string(), "dictionary")
                    }
                    None if !g.text.is_empty() => (g.text.clone(), "observed"),
                    None => (g.key.clone(), "key"),
                }
            };
            let league_codes = g.league_codes();
            ordered.push(g.id);
            by_id.insert(
                g.id,
                ResolvedGroup {
                    id: g.id,
                    key: g.key.clone(),
                    kind: g.kind.clone(),
                    league_code: league_codes.first().copied(),
                    league_codes,
                    label,
                    label_source,
                    used: g.used,
                    evidence: g.evidence().to_string(),
                    note: g.note.clone(),
                    hypothesis_key: g.hypothesis_key.clone(),
                },
            );
        }
        ResolvedGroups {
            game: table.game.clone(),
            fields: table.fields.clone(),
            evidence: evidence.to_string(),
            witness: table.witness.source.clone(),
            witness_notes: table.witness.notes.clone(),
            provenance: None,
            fit: None,
            pairs: table.pairs.clone(),
            by_id,
            ordered,
        }
    }

    /// P1-C / research 2026-09-21 handoff 08 §3: a `league`-kind group with no key — the PS4
    /// ids 77 (NHL Alumni + All-Time) and 78 (Pro-Am template teams), single flat pseudo-groups
    /// shared by their leagues — is labelled **by rule** from the leagues that carry it
    /// ("<league> / <league> (single group)"), never "unknown". `display` gives a league code's
    /// display text. Evidence, note and hypothesis stay as recorded; `label_source` becomes
    /// `derived-from-leagues`. Groups of other kinds keep the unnamed label.
    pub fn derive_unknown_league_group_labels(&mut self, display: impl Fn(u64) -> Option<String>) {
        for g in self.by_id.values_mut() {
            if !g.key.is_empty() || g.kind != "league" || g.league_codes.is_empty() {
                continue;
            }
            let mut names: Vec<String> = Vec::new();
            for code in &g.league_codes {
                let Some(n) = display(*code) else { continue };
                if !names.contains(&n) {
                    names.push(n);
                }
            }
            if names.is_empty() {
                continue;
            }
            g.label = format!("{} (single group)", names.join(" / "));
            g.label_source = "derived-from-leagues";
        }
    }

    /// Demote the evidence class (an unverified title claim never shows as `traced`).
    pub fn demote_evidence(&mut self) {
        self.evidence = crate::leagues::KEY_EVIDENCE_BY_REFERENCE.to_string();
    }

    /// Check the table against the open database's team rows (see [`FitReport`]). `None` when
    /// the team table lacks the group fields, which is a schema mismatch by itself.
    pub fn check_fit(&self, db: &TdbFile, team_table: &str) -> Option<FitReport> {
        let table = db.table_canonical(team_table)?;
        let field = |studio: &str| field_by_studio(db, table, studio);
        let league_field = field(crate::leagues::TEAM_LEAGUE_FIELD)?;
        let group_field = field(&self.fields.league)?;
        field(&self.fields.conference)?;
        field(&self.fields.division)?;
        let mut report = FitReport::default();
        let mut missing: BTreeMap<u64, usize> = BTreeMap::new();
        for rec in 0..table.current_records as u32 {
            let (Some(code), Some(lg)) = (db.read_u64(table, rec, league_field), db.read_u64(table, rec, group_field)) else {
                continue;
            };
            match self.group(lg) {
                None => report.unknown += 1,
                Some(g) if g.kind != "league" => report.misplaced += 1,
                Some(g) if g.league_codes.contains(&code) => report.fit += 1,
                Some(_) => report.conflict += 1,
            }
            if self.league_group(code).is_none() {
                *missing.entry(code).or_default() += 1;
            }
        }
        report.codes_without_group = missing.into_iter().collect();
        Some(report)
    }

    pub fn group(&self, id: u64) -> Option<&ResolvedGroup> {
        self.by_id.get(&id)
    }

    /// Every group in table order.
    pub fn all(&self) -> impl Iterator<Item = &ResolvedGroup> {
        self.ordered.iter().filter_map(|id| self.by_id.get(id))
    }

    /// Groups of the table whose key is unknown (shown raw with the id).
    pub fn unknown(&self) -> Vec<&ResolvedGroup> {
        self.all().filter(|g| g.evidence == EVIDENCE_UNKNOWN).collect()
    }

    /// The groups of one league, in table order, by kind.
    pub fn for_league(&self, code: u64, kind: &str) -> Vec<&ResolvedGroup> {
        self.all()
            .filter(|g| g.league_codes.contains(&code) && g.kind == kind)
            .collect()
    }

    /// The league's own group id (the value `leaguegroup` carries for its teams).
    pub fn league_group(&self, code: u64) -> Option<&ResolvedGroup> {
        self.for_league(code, "league").into_iter().next()
    }

    /// Every witnessed arrangement of a block, regular and exceptional.
    pub fn pairs_of(&self, head: u64) -> impl Iterator<Item = &PairDef> {
        self.pairs.iter().filter(move |p| p.head == head)
    }

    /// The choosable (conference, division) pairs of a league's block, in table order of the
    /// conference then the division.
    pub fn regular_pairs(&self, code: u64) -> Vec<(u64, u64)> {
        let Some(lg) = self.league_group(code) else {
            return Vec::new();
        };
        let order = |id: u64| self.ordered.iter().position(|&x| x == id).unwrap_or(usize::MAX);
        let mut v: Vec<(u64, u64)> = self
            .pairs_of(lg.id)
            .filter(|p| p.arrangement == ARRANGEMENT_REGULAR)
            .map(|p| (p.conference, p.division))
            .collect();
        v.sort_by_key(|&(c, d)| (order(c), order(d)));
        v.dedup();
        v
    }

    /// The exceptional arrangements recorded for a league's block.
    pub fn exceptional_pairs(&self, code: u64) -> Vec<&PairDef> {
        let Some(lg) = self.league_group(code) else {
            return Vec::new();
        };
        self.pairs_of(lg.id).filter(|p| p.arrangement == ARRANGEMENT_EXCEPTIONAL).collect()
    }

    /// Whether the table carries witnessed pairs at all (older files do not: then structure is
    /// unresolved and no choice is offered beyond the own-id arrangement).
    pub fn has_pairs(&self) -> bool {
        !self.pairs.is_empty()
    }

    /// How a league's teams are grouped on this title, by its witnessed regular arrangements.
    pub fn shape(&self, code: u64) -> Option<LeagueShape> {
        let lg = self.league_group(code)?;
        let pairs = self.regular_pairs(code);
        Some(if pairs.is_empty() || pairs == [(lg.id, lg.id)] {
            LeagueShape::Single { league_group: lg.id }
        } else {
            LeagueShape::Structured { league_group: lg.id, pairs }
        })
    }

    /// The choosable conference-slot values of a league (distinct, in table order); a league
    /// whose rows also carry the own-id arrangement lists its own id first.
    pub fn conference_choices(&self, code: u64) -> Vec<&ResolvedGroup> {
        let mut seen = Vec::new();
        for (c, _) in self.regular_pairs(code) {
            if !seen.contains(&c) {
                seen.push(c);
            }
        }
        seen.into_iter().filter_map(|id| self.group(id)).collect()
    }

    /// The choosable division-slot values of a league (distinct, in table order), optionally
    /// restricted to the pairs of one conference.
    pub fn division_choices(&self, code: u64, conference: Option<u64>) -> Vec<&ResolvedGroup> {
        let mut seen = Vec::new();
        for (c, d) in self.regular_pairs(code) {
            if conference.is_none_or(|x| x == c) && !seen.contains(&d) {
                seen.push(d);
            }
        }
        seen.into_iter().filter_map(|id| self.group(id)).collect()
    }

    /// Validate a (league, conference, division) triple against the witnessed structure: the
    /// triple must be a regular arrangement the title's stock rows carry for that league's block.
    /// Returns the plain-language problem, if any.
    pub fn validate_membership(
        &self,
        league_code: u64,
        conference: u64,
        division: u64,
    ) -> Result<(), String> {
        let Some(lg) = self.league_group(league_code) else {
            return Err(format!(
                "league code {league_code} has no group in this title's table; its teams cannot be given group ids from here"
            ));
        };
        let name = |id: u64| self.group(id).map(|g| g.label.clone()).unwrap_or_else(|| format!("group {id}"));
        if !self.has_pairs() {
            // No witnessed structure recorded: only the own-id arrangement is safe to write.
            return if conference == lg.id && division == lg.id {
                Ok(())
            } else {
                Err(format!(
                    "this title's table records no witnessed conference/division arrangements; only {}'s own id {} can be written in all three fields",
                    lg.label, lg.id
                ))
            };
        }
        let regular = self.regular_pairs(league_code);
        if regular.contains(&(conference, division)) {
            return Ok(());
        }
        if let Some(p) = self.pairs_of(lg.id).find(|p| p.conference == conference && p.division == division) {
            return Err(format!(
                "{} / {} under {} is an exceptional stock arrangement ({}; carried by {} row{}: team id{} {}) and is not offered for ordinary clubs",
                name(conference),
                name(division),
                lg.label,
                p.note.as_deref().unwrap_or("recorded as exceptional"),
                p.rows,
                if p.rows == 1 { "" } else { "s" },
                if p.teams.len() == 1 { "" } else { "s" },
                p.teams.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ")
            ));
        }
        if regular == [(lg.id, lg.id)] {
            return Err(format!(
                "{} is a single-group league on this title: its conference and division are its own group id {} (not {} / {})",
                lg.label, lg.id, conference, division
            ));
        }
        let with_conf: Vec<String> = regular
            .iter()
            .filter(|(c, _)| *c == conference)
            .map(|(_, d)| format!("{d} = {}", name(*d)))
            .collect();
        if with_conf.is_empty() {
            let confs: Vec<String> = self.conference_choices(league_code).iter().map(|g| format!("{} = {}", g.id, g.label)).collect();
            return Err(format!(
                "{} ({conference}) is not a conference {}'s teams carry on this title; witnessed: {}",
                name(conference),
                lg.label,
                confs.join(", ")
            ));
        }
        Err(format!(
            "{} ({division}) is not paired with {} under {} on this title; the witnessed divisions of that conference are {}",
            name(division),
            name(conference),
            lg.label,
            with_conf.join(", ")
        ))
    }
}
