//! League identity — target-specific resolution of league codes and league rows.
//!
//! Two shapes exist across the catalog (measured in L1.1-A,
//! `docs/plans/league-identity-parity-matrix.md`):
//!
//! * **PS4 (NHL 15–21)** carry a real `stockleagues` (`fsLK`) table keyed by `leagueid`. Each
//!   team's `kOtt.xMjj` is a foreign key into it. The row's `name` is a *localization key*, not
//!   display text; without a decoded dictionary it is shown as a key.
//! * **gen6/gen7 (NHL 07–15, Legacy, Recompiled)** carry only the per-team code. Its meaning is
//!   read from a `defs/leagues/*.toml` vocabulary that Studio selects by **schema fingerprint plus
//!   game id** — never by a global enum or by bit width alone. When the evidence does not select
//!   exactly one vocabulary, codes stay raw and the catalog says why.
//!
//! The catalog is the single resolver behind team headers, lists, filters and Transactions, so a
//! league label never depends on which page rendered it. Selection keys are always the native code
//! (`leagueid` / `xMjj`), never the label — duplicate labels (three `CUSTOM_TEAMS` rows on NHL 21)
//! are legitimate.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::Deserialize;
use tdb_core::{Table, TdbFile};
use tdb_defs::FieldMetaIndex;

use crate::view::FieldView;

/// A localization dictionary attached to a session for label resolution: the target's own file,
/// identified so a label can never be attributed to a neighbouring title's dictionary.
#[derive(Clone, Debug)]
pub struct AttachedDictionary {
    pub path: String,
    /// Locale token from the file name (`eng_us`), for display.
    pub locale: String,
    pub sha256: String,
    pub dictionary: std::sync::Arc<tdb_loc::LocDictionary>,
    /// The file bytes exactly as attached (the input of the LOC-W1 re-point and the identity a
    /// copied-target save expects to find at the destination).
    pub bytes: std::sync::Arc<Vec<u8>>,
}

impl AttachedDictionary {
    /// Parse and attach the dictionary at `path` (either byte order). CRC scopes are verified
    /// first; a mismatch is an error so a damaged dictionary never labels anything.
    pub fn load(path: &str) -> Result<Self, crate::DomainError> {
        let bytes = std::fs::read(path).map_err(|source| crate::DomainError::Io {
            path: path.to_string(),
            source,
        })?;
        Self::from_bytes(path, bytes)
    }

    /// Attach `bytes` as the dictionary at `path` (the path is the identity's home, not read).
    pub fn from_bytes(path: &str, bytes: Vec<u8>) -> Result<Self, crate::DomainError> {
        let checks = tdb_loc::verify(&bytes)
            .map_err(|e| crate::DomainError::Sync(format!("{path}: {e}")))?;
        if let Some(bad) = checks.iter().find(|c| !c.matches()) {
            return Err(crate::DomainError::Sync(format!(
                "{path}: dictionary CRC scope {} does not verify (stored {:08x}, computed {:08x})",
                bad.name, bad.stored, bad.computed
            )));
        }
        let dictionary = tdb_loc::LocDictionary::parse(&bytes)
            .map_err(|e| crate::DomainError::Sync(format!("{path}: {e}")))?;
        let file = Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let locale = file.strip_prefix("nhl_").unwrap_or(&file).to_string();
        Ok(AttachedDictionary {
            path: path.to_string(),
            locale,
            sha256: tdb_txn::FileIdentity::of_bytes(&bytes).sha256,
            dictionary: std::sync::Arc::new(dictionary),
            bytes: std::sync::Arc::new(bytes),
        })
    }
}

/// The `stockleagues` table (studio id) on PS4 titles.
pub const STOCK_LEAGUES: &str = "fsLK";
/// `stockleagues.leagueid` — the primary key.
pub const STOCK_LEAGUE_ID: &str = "aQrQ";
/// `stockleagues.name` — the localization key (inline string, 25 bytes on NHL 21).
pub const STOCK_LEAGUE_NAME: &str = "vIys";
/// The team-side league code (`league`) on every fat team table and on NHL 07's stock table.
pub const TEAM_LEAGUE_FIELD: &str = "xMjj";

// ----------------------------------------------------------------------------- vocabularies

/// How well a label is established for the target it is shown on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LabelConfidence {
    /// Read from the target's own `stockleagues` row (still a key, not display text).
    Native,
    /// The same code layout is named by a PS4 `stockleagues` table on an identical distribution.
    Corroborated,
    /// Inferred from member-team abbreviations on the committed anchor only.
    Candidate,
    /// Present in the target executable's league enum (a traced code → key table) with no team row
    /// on the anchors; the label is the traced key's text in the title's own dictionary.
    Enum,
}

impl LabelConfidence {
    pub fn as_str(self) -> &'static str {
        match self {
            LabelConfidence::Native => "native",
            LabelConfidence::Corroborated => "corroborated",
            LabelConfidence::Candidate => "candidate",
            LabelConfidence::Enum => "enum",
        }
    }
}

/// Schema evidence a vocabulary requires before it may be applied.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct SchemaFingerprint {
    /// Studio id of the table carrying the per-team league code (`kOtt`, or `HqzU` on NHL 07).
    pub league_table: String,
    /// Studio id of the league-code field on that table.
    pub league_field: String,
    /// Exact bit width the code field must have.
    pub league_width: u32,
    /// `table.field` studio ids that must exist.
    #[serde(default)]
    pub required_fields: Vec<String>,
    /// `table.field` studio ids that must not exist.
    #[serde(default)]
    pub forbidden_fields: Vec<String>,
    /// Studio table ids that must exist.
    #[serde(default)]
    pub required_tables: Vec<String>,
    /// Studio table ids that must not exist.
    #[serde(default)]
    pub forbidden_tables: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct VocabSource {
    pub kind: String,
    #[serde(default)]
    pub description: String,
}

/// One code → label entry with its own evidence.
#[derive(Clone, Debug, Deserialize)]
pub struct VocabEntry {
    pub raw: u64,
    pub label: String,
    pub confidence: LabelConfidence,
    #[serde(default)]
    pub evidence: String,
    /// The localization-dictionary key the game's executable binds this code to, when known.
    /// `key_evidence` says how: `traced` (read out of an executable's static table — so far only
    /// NHL Legacy Xbox 360's) or `hypothesis` (the key family is present in the title's own
    /// dictionary, the binding itself is untraced). Never a label source on its own.
    #[serde(default)]
    pub dictionary_key: Option<String>,
    #[serde(default)]
    pub key_evidence: Option<String>,
}

/// A group-id label (`OwTu` conference / `CGMK` division) scoped to one league code.
#[derive(Clone, Debug, Deserialize)]
pub struct GroupEntry {
    /// Studio field id on the team table (`OwTu`, `CGMK`, `iIBl`).
    pub field: String,
    /// The league code the label is anchored under.
    pub league_raw: u64,
    pub raw: u64,
    pub label: String,
    pub confidence: LabelConfidence,
}

/// One `defs/leagues/*.toml` file.
#[derive(Clone, Debug, Deserialize)]
pub struct LeagueVocabulary {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Studio game ids this layout was measured for.
    pub applies_to: Vec<String>,
    pub schema: SchemaFingerprint,
    pub source: Option<VocabSource>,
    /// Where the entries' `traced` key evidence was actually read (R1 F5). A vocabulary shared
    /// by several targets carries the mapping once; the evidence class is resolved per target
    /// against this witness, so only the witnessed (game, platform) shows `traced`.
    #[serde(default)]
    pub key_witness: Option<KeyWitness>,
    #[serde(default)]
    pub entries: Vec<VocabEntry>,
    #[serde(default)]
    pub groups: Vec<GroupEntry>,
}

/// The executable(s) a vocabulary's code → dictionary-key table was traced on.
#[derive(Clone, Debug, Deserialize)]
pub struct KeyWitness {
    /// The targets whose own executable was inspected.
    pub traced_on: Vec<WitnessTarget>,
    /// Plain-language description of the witness (file, region, method).
    #[serde(default)]
    pub source: String,
    /// The evidence class every other target sharing this vocabulary gets for a `traced` entry
    /// (`by-reference`: the mapping is inherited from the witness, this target's executable was
    /// not inspected).
    #[serde(default = "default_otherwise")]
    pub otherwise: String,
}

fn default_otherwise() -> String {
    KEY_EVIDENCE_BY_REFERENCE.to_string()
}

/// One witnessed target: studio game id + platform token (`ps3` / `xbox` / `ps4` / `recomp`).
#[derive(Clone, Debug, Deserialize)]
pub struct WitnessTarget {
    pub game: String,
    pub platform: String,
}

/// The evidence classes of a code → key binding as it applies to one target.
pub const KEY_EVIDENCE_TRACED: &str = "traced";
pub const KEY_EVIDENCE_HYPOTHESIS: &str = "hypothesis";
pub const KEY_EVIDENCE_BY_REFERENCE: &str = "by-reference";

impl LeagueVocabulary {
    pub fn from_toml(text: &str) -> Result<Self, LeagueError> {
        toml::from_str(text).map_err(|e| LeagueError::Parse(e.to_string()))
    }

    fn entry(&self, raw: u64) -> Option<&VocabEntry> {
        self.entries.iter().find(|e| e.raw == raw)
    }

    /// Resolve an entry's `key_evidence` for one target. A `traced` claim holds only on a target
    /// the vocabulary's `[key_witness]` names (game **and** platform); every other target sharing
    /// the vocabulary gets the witness's `otherwise` class (`by-reference`), and a `traced` claim
    /// with no witness at all — or an unknown target — is never shown as traced. `hypothesis` and
    /// other classes pass through unchanged. The second value is the witness note to show.
    pub fn key_evidence_for(
        &self,
        entry: &VocabEntry,
        game_id: Option<&str>,
        platform: Option<crate::Platform>,
    ) -> (Option<String>, Option<String>) {
        let Some(claimed) = entry.key_evidence.as_deref() else {
            return (None, None);
        };
        if claimed != KEY_EVIDENCE_TRACED {
            return (Some(claimed.to_string()), None);
        }
        let Some(w) = self.key_witness.as_ref() else {
            return (
                Some(KEY_EVIDENCE_BY_REFERENCE.to_string()),
                Some("the vocabulary claims a traced binding but names no witness".to_string()),
            );
        };
        let witnessed = match (game_id, platform) {
            (Some(g), Some(p)) => w
                .traced_on
                .iter()
                .any(|t| t.game == g && t.platform == p.as_str()),
            _ => false,
        };
        if witnessed {
            (Some(KEY_EVIDENCE_TRACED.to_string()), Some(w.source.clone()))
        } else {
            let on = w
                .traced_on
                .iter()
                .map(|t| format!("{} {}", t.game, t.platform))
                .collect::<Vec<_>>()
                .join(", ");
            (
                Some(w.otherwise.clone()),
                Some(format!(
                    "traced on {on} ({}); this target's own executable was not inspected",
                    w.source
                )),
            )
        }
    }

    /// `true` iff every schema requirement holds on `db`.
    pub fn fingerprint_matches(&self, db: &TdbFile) -> bool {
        let s = &self.schema;
        let Some(t) = db.table_canonical(&s.league_table) else {
            return false;
        };
        let Some(f) = field_by_studio(db, t, &s.league_field) else {
            return false;
        };
        if f.bit_width != s.league_width {
            return false;
        }
        let has_field = |spec: &str| -> bool {
            let Some((table, field)) = spec.split_once('.') else {
                return false;
            };
            db.table_canonical(table)
                .and_then(|t| field_by_studio(db, t, field))
                .is_some()
        };
        s.required_fields.iter().all(|x| has_field(x))
            && !s.forbidden_fields.iter().any(|x| has_field(x))
            && s.required_tables
                .iter()
                .all(|x| db.table_canonical(x).is_some())
            && !s
                .forbidden_tables
                .iter()
                .any(|x| db.table_canonical(x).is_some())
    }
}

#[derive(Debug)]
pub enum LeagueError {
    Io(std::io::Error),
    Parse(String),
}

impl std::fmt::Display for LeagueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LeagueError::Io(e) => write!(f, "league vocabulary io: {e}"),
            LeagueError::Parse(e) => write!(f, "league vocabulary parse: {e}"),
        }
    }
}
impl std::error::Error for LeagueError {}

/// Every loaded vocabulary (`defs/leagues`).
#[derive(Clone, Debug, Default)]
pub struct LeagueVocabularies {
    vocabs: Vec<LeagueVocabulary>,
    /// Competition group tables (`groups/<game>.toml`), Phase 1.2.
    group_tables: Vec<crate::competition_groups::GroupTable>,
    /// League logo index tables (`logos/<game>.toml`), Phase 1.4.
    logo_tables: Vec<crate::league_logos::LogoTable>,
}

impl LeagueVocabularies {
    /// Load every `*.toml` under `dir`, in name order. A missing directory yields an empty set
    /// (older defs trees), never an error.
    pub fn from_dir(dir: impl AsRef<Path>) -> Result<Self, LeagueError> {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            return Ok(Self::default());
        }
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .map_err(LeagueError::Io)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        paths.sort();
        let mut vocabs = Vec::with_capacity(paths.len());
        for p in paths {
            let text = std::fs::read_to_string(&p).map_err(LeagueError::Io)?;
            vocabs.push(LeagueVocabulary::from_toml(&text)?);
        }
        let group_tables = crate::competition_groups::GroupTable::load_dir(&dir.join("groups"))?;
        let logo_tables = crate::league_logos::LogoTable::load_dir(&dir.join("logos"))?;
        Ok(LeagueVocabularies {
            vocabs,
            group_tables,
            logo_tables,
        })
    }

    pub fn from_vocabs(vocabs: Vec<LeagueVocabulary>) -> Self {
        LeagueVocabularies {
            vocabs,
            group_tables: Vec::new(),
            logo_tables: Vec::new(),
        }
    }

    pub fn logo_tables(&self) -> &[crate::league_logos::LogoTable] {
        &self.logo_tables
    }

    /// The competition group table that applies to `game_id`, if one is baked in (any platform
    /// or source; the first in file order). Prefer [`LeagueVocabularies::group_table_for`].
    pub fn group_table(&self, game_id: &str) -> Option<&crate::competition_groups::GroupTable> {
        self.group_tables.iter().find(|t| t.applies(game_id))
    }

    /// The competition group table bound to this target: game id, the league source the open
    /// database presents (`stockleagues` on PS4, derived codes on gen7) and, when known, the
    /// platform. The PS4 and Xbox/PS3 tables of one title share a game id and differ in both.
    pub fn group_table_for(
        &self,
        game_id: &str,
        source: LeagueSource,
        platform: Option<crate::Platform>,
    ) -> Option<&crate::competition_groups::GroupTable> {
        self.group_tables
            .iter()
            .find(|t| t.applies_to_target(game_id, source, platform))
    }

    pub fn group_tables(&self) -> &[crate::competition_groups::GroupTable] {
        &self.group_tables
    }

    pub fn vocabularies(&self) -> &[LeagueVocabulary] {
        &self.vocabs
    }

    /// Select the vocabulary for `db`: fingerprint candidates narrowed by `game_id`.
    pub fn select(&self, db: &TdbFile, game_id: Option<&str>) -> VocabularySelection {
        let candidates: Vec<&LeagueVocabulary> = self
            .vocabs
            .iter()
            .filter(|v| v.fingerprint_matches(db))
            .collect();
        if candidates.is_empty() {
            return VocabularySelection::Unresolved {
                reason: "no league vocabulary matches this database's schema".to_string(),
                candidates: Vec::new(),
            };
        }
        let names: Vec<String> = candidates.iter().map(|v| v.name.clone()).collect();
        match game_id {
            Some(g) => {
                let hits: Vec<&LeagueVocabulary> = candidates
                    .iter()
                    .copied()
                    .filter(|v| v.applies_to.iter().any(|a| a == g))
                    .collect();
                match hits.as_slice() {
                    [one] => VocabularySelection::Selected(Box::new((*one).clone())),
                    [] => VocabularySelection::Unresolved {
                        reason: format!(
                            "the schema matches {} but none was measured for game '{g}'",
                            names.join(", ")
                        ),
                        candidates: names,
                    },
                    _ => VocabularySelection::Unresolved {
                        reason: format!(
                            "several league vocabularies claim game '{g}' on this schema: {}",
                            hits.iter()
                                .map(|v| v.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        candidates: names,
                    },
                }
            }
            None => match candidates.as_slice() {
                [one] => VocabularySelection::Selected(Box::new((*one).clone())),
                _ => VocabularySelection::Unresolved {
                    reason: format!(
                        "the schema matches several league layouts ({}) and no game id is known",
                        names.join(", ")
                    ),
                    candidates: names,
                },
            },
        }
    }
}

/// Outcome of vocabulary selection.
#[derive(Clone, Debug)]
pub enum VocabularySelection {
    Selected(Box<LeagueVocabulary>),
    Unresolved {
        reason: String,
        candidates: Vec<String>,
    },
}

// ----------------------------------------------------------------------------- catalog

/// How a dictionary key resolved to text.
pub enum Resolved {
    Exact,
    /// Through the game's case-folded hash, to the single row with that hash (its key).
    ByHash(String),
}

/// The text for `key` in `d`: the exact row, else the unique row carrying `loc_hash(key)` (the
/// game evidently looks keys up by hash — `AHL` resolves through the row keyed `ahl`). A hash
/// carried by several rows is ambiguous and resolves to nothing.
pub fn resolve_text<'a>(d: &'a AttachedDictionary, key: &str) -> Option<(&'a str, Resolved)> {
    if let Some(t) = d.dictionary.text(key) {
        return Some((t, Resolved::Exact));
    }
    let h = tdb_loc::loc_hash(key);
    let mut hits = d.dictionary.rows.iter().filter(|r| r.hash == h);
    let first = hits.next()?;
    if hits.next().is_some() {
        return None;
    }
    Some((first.text.as_deref()?, Resolved::ByHash(first.key.clone().unwrap_or_default())))
}

/// Where the catalog's league identities come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeagueSource {
    /// PS4: rows of `stockleagues`, joined on `leagueid`.
    StockLeagues,
    /// gen6/gen7: distinct per-team codes, labelled by a selected vocabulary.
    DerivedCodes,
}

impl LeagueSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            LeagueSource::StockLeagues => "stockleagues",
            LeagueSource::DerivedCodes => "derived-codes",
        }
    }
}

/// One league identity as the target stores it.
#[derive(Clone, Debug)]
pub struct LeagueIdentity {
    /// The native code: `stockleagues.leagueid` on PS4, the team-side `xMjj` code elsewhere.
    pub code: u64,
    /// `stockleagues` record index (PS4 only) — the handle a field edit takes.
    pub record: Option<u32>,
    /// `stockleagues.name` — a localization key, shown as a key until a dictionary resolves it.
    pub name_key: Option<String>,
    /// Resolved display text, when one is established for this target.
    pub label: Option<String>,
    pub confidence: Option<LabelConfidence>,
    /// gen7: the dictionary key the executable is believed to bind this code to (from the
    /// vocabulary), with `dictionary_key_evidence` resolved **for this target**: `traced` only on
    /// the witnessed (game, platform), `by-reference` on targets that inherit the witnessed
    /// mapping, `hypothesis` where only the key family is known. PS4: `None` (the key is
    /// `name_key`).
    pub dictionary_key: Option<String>,
    pub dictionary_key_evidence: Option<String>,
    /// Plain-language provenance of `label` / `name_key`.
    pub evidence: String,
    /// Teams whose league code equals `code` (every team-table row, including inactive ones).
    pub member_teams: usize,
    /// Presentation fields on the PS4 row (semantics not established — surfaced raw).
    pub art_id: Option<u64>,
    pub render_id: Option<u64>,
    pub order_id: Option<u64>,
    pub primary_color: Option<u64>,
    pub secondary_color: Option<u64>,
}

impl LeagueIdentity {
    /// What to print for this league: the label, else the key shown as a key, else the raw code.
    pub fn display(&self) -> String {
        if let Some(l) = &self.label {
            return l.clone();
        }
        if let Some(k) = &self.name_key {
            return k.clone();
        }
        format!("League code {}", self.code)
    }
}

/// The dictionary a catalog resolved labels through, for disclosure.
#[derive(Clone, Debug)]
pub struct DictionaryRef {
    pub path: String,
    pub locale: String,
    pub sha256: String,
    pub rows: usize,
}

/// The per-session league catalog.
#[derive(Clone, Debug)]
pub struct LeagueCatalog {
    pub source: LeagueSource,
    /// The attached dictionary used for PS4 labels, when one was.
    pub dictionary: Option<DictionaryRef>,
    /// Studio id of the table carrying the per-team league code.
    pub team_league_table: String,
    /// The vocabulary that labelled derived codes, if one was selected.
    pub vocabulary: Option<String>,
    /// Why no vocabulary applies (derived codes stay raw), when that is the case.
    pub unresolved_reason: Option<String>,
    pub leagues: Vec<LeagueIdentity>,
    groups: Vec<GroupEntry>,
    by_code: HashMap<u64, usize>,
    /// The title's competition group table, resolved for this session (NHL 12 → Legacy family;
    /// PS4 NHL 15 → 21 since P1-C) — only when a table is baked for this target and the open
    /// database's team rows agree with it.
    pub competition: Option<crate::competition_groups::ResolvedGroups>,
    /// Why no group table applies (none baked for the target, no game id, schema without the
    /// group fields, or a table the rows contradict), when `competition` is `None` although the
    /// team row carries group ids.
    pub competition_blocker: Option<String>,
}

impl LeagueCatalog {
    /// Build the catalog over `db`. `game_id` is the session's studio game id when known.
    pub fn build(
        db: &TdbFile,
        meta: &FieldMetaIndex,
        vocabs: &LeagueVocabularies,
        game_id: Option<&str>,
    ) -> LeagueCatalog {
        Self::build_for_target(db, meta, vocabs, game_id, None, None)
    }

    /// As [`LeagueCatalog::build`], resolving PS4 name keys through the target's own attached
    /// dictionary when one is given. Derived-code titles never consult a dictionary here: their
    /// code → key binding is not stored in the database. Without a platform, no binding can be
    /// shown as `traced` (see [`LeagueVocabulary::key_evidence_for`]).
    pub fn build_with_dictionary(
        db: &TdbFile,
        meta: &FieldMetaIndex,
        vocabs: &LeagueVocabularies,
        game_id: Option<&str>,
        dictionary: Option<&AttachedDictionary>,
    ) -> LeagueCatalog {
        Self::build_for_target(db, meta, vocabs, game_id, None, dictionary)
    }

    /// The full form: `platform` lets the code → key evidence resolve per target (R1 F5).
    pub fn build_for_target(
        db: &TdbFile,
        meta: &FieldMetaIndex,
        vocabs: &LeagueVocabularies,
        game_id: Option<&str>,
        platform: Option<crate::Platform>,
        dictionary: Option<&AttachedDictionary>,
    ) -> LeagueCatalog {
        // Member counts from the team-side code, whichever table carries it.
        let (team_league_table, counts) = team_code_counts(db);

        if let Some(fs) = db.table_canonical(STOCK_LEAGUES) {
            let mut leagues = Vec::with_capacity(fs.current_records as usize);
            let key = field_by_studio(db, fs, STOCK_LEAGUE_ID);
            let name = field_by_studio(db, fs, STOCK_LEAGUE_NAME);
            let by_name = |n: &str| descriptor_by_name(meta, fs, n);
            let art = by_name("artid");
            let render = by_name("renderid");
            let order = by_name("orderid");
            let pc = by_name("primarycolor");
            let sc = by_name("secondarycolor");
            for rec in 0..fs.current_records as u32 {
                let Some(code) = key.and_then(|f| db.read_u64(fs, rec, f)) else {
                    continue;
                };
                let name_key = name
                    .and_then(|f| db.read_inline_str(fs, rec, f))
                    .map(|b| String::from_utf8_lossy(b).into_owned());
                let rd =
                    |f: Option<&tdb_core::FieldDescriptor>| f.and_then(|f| db.read_u64(fs, rec, f));
                let (label, evidence) = match (dictionary, name_key.as_deref()) {
                    (Some(d), Some(k)) => match resolve_text(d, k) {
                        Some((text, Resolved::Exact)) => (
                            Some(text.to_string()),
                            format!(
                                "stockleagues row; key {k} resolved in this target's {} dictionary ({}…)",
                                d.locale,
                                &d.sha256[..12]
                            ),
                        ),
                        Some((text, Resolved::ByHash(row_key))) => (
                            Some(text.to_string()),
                            format!(
                                "stockleagues row; key {k} is not an exact row but its hash matches the single row keyed {row_key:?} in this target's {} dictionary ({}…) — the game's lookup is by hash",
                                d.locale,
                                &d.sha256[..12]
                            ),
                        ),
                        None => (
                            None,
                            format!(
                                "stockleagues row; key {k} is not in this target's {} dictionary (exact or by hash) — shown as a key (the game's own fallback is unverified)",
                                d.locale
                            ),
                        ),
                    },
                    _ => (
                        None,
                        "stockleagues row; the name is a localization key, not display text (no dictionary attached)"
                            .to_string(),
                    ),
                };
                leagues.push(LeagueIdentity {
                    code,
                    record: Some(rec),
                    label,
                    confidence: Some(LabelConfidence::Native),
                    dictionary_key: None,
                    dictionary_key_evidence: None,
                    evidence,
                    member_teams: counts.get(&code).copied().unwrap_or(0),
                    name_key,
                    art_id: rd(art),
                    render_id: rd(render),
                    order_id: rd(order),
                    primary_color: rd(pc),
                    secondary_color: rd(sc),
                });
            }
            // Team codes that reference no stockleagues row still need an identity (reported raw).
            for (&code, &n) in &counts {
                if !leagues.iter().any(|l| l.code == code) {
                    leagues.push(LeagueIdentity {
                        code,
                        record: None,
                        name_key: None,
                        label: None,
                        confidence: None,
                        dictionary_key: None,
                        dictionary_key_evidence: None,
                        evidence: "team-side league code with no stockleagues row".to_string(),
                        member_teams: n,
                        art_id: None,
                        render_id: None,
                        order_id: None,
                        primary_color: None,
                        secondary_color: None,
                    });
                }
            }
            leagues.sort_by_key(|l| l.code);
            let mut cat = Self::finish(
                LeagueSource::StockLeagues,
                team_league_table,
                None,
                None,
                leagues,
                Vec::new(),
            );
            cat.dictionary = dictionary.map(|d| DictionaryRef {
                path: d.path.clone(),
                locale: d.locale.clone(),
                sha256: d.sha256.clone(),
                rows: d.dictionary.len(),
            });
            Self::attach_competition(&mut cat, db, vocabs, game_id, platform, dictionary);
            return cat;
        }

        let (vocab, reason) = match vocabs.select(db, game_id) {
            VocabularySelection::Selected(v) => (Some(*v), None),
            VocabularySelection::Unresolved { reason, .. } => (None, Some(reason)),
        };
        // Every code a team carries, plus the codes the title's executable enum knows without a
        // stock team (`confidence = "enum"` entries, P1.1-X) — shown with zero members so they can
        // be browsed and chosen as a membership target.
        let mut all_codes: BTreeMap<u64, usize> = counts.clone();
        if let Some(v) = &vocab {
            for e in v.entries.iter().filter(|e| e.confidence == LabelConfidence::Enum) {
                all_codes.entry(e.raw).or_insert(0);
            }
        }
        let mut leagues: Vec<LeagueIdentity> = all_codes
            .iter()
            .map(|(&code, &n)| {
                let entry = vocab.as_ref().and_then(|v| v.entry(code));
                let mut evidence = match (&vocab, entry) {
                    (Some(v), Some(e)) => {
                        format!("{} ({}): {}", v.name, e.confidence.as_str(), e.evidence)
                    }
                    (Some(v), None) => format!("{}: code not catalogued", v.name),
                    (None, _) => reason.clone().unwrap_or_default(),
                };
                let mut label = entry.map(|e| e.label.clone());
                let dictionary_key = entry.and_then(|e| e.dictionary_key.clone());
                let (key_evidence, witness_note) = match (&vocab, entry) {
                    (Some(v), Some(e)) => v.key_evidence_for(e, game_id, platform),
                    _ => (None, None),
                };
                if let (Some(_), Some(note)) = (dictionary_key.as_deref(), witness_note.as_deref())
                {
                    evidence = format!(
                        "{evidence}; code -> key binding {}: {note}",
                        key_evidence.as_deref().unwrap_or("unstated")
                    );
                }
                // With the title's own dictionary attached and a key on record, show the game's
                // text for that key; the vocabulary label stays the fallback and the evidence says
                // whether the code -> key binding is traced or a hypothesis.
                if let (Some(d), Some(k)) = (dictionary, dictionary_key.as_deref()) {
                    match resolve_text(d, k) {
                        Some((text, how)) => {
                            let via = match how {
                                Resolved::Exact => String::new(),
                                Resolved::ByHash(rk) => format!(" (by hash, row keyed {rk:?})"),
                            };
                            evidence = format!(
                                "{evidence}; dictionary text for key {k}{via} in this target's {} dictionary ({}…); code -> key binding: {}",
                                d.locale,
                                &d.sha256[..12],
                                key_evidence.as_deref().unwrap_or("unstated")
                            );
                            label = Some(text.to_string());
                        }
                        None => {
                            evidence = format!(
                                "{evidence}; key {k} is not in this target's {} dictionary — vocabulary label shown",
                                d.locale
                            );
                        }
                    }
                }
                LeagueIdentity {
                    code,
                    record: None,
                    name_key: None,
                    label,
                    confidence: entry.map(|e| e.confidence),
                    dictionary_key,
                    dictionary_key_evidence: key_evidence,
                    evidence,
                    member_teams: n,
                    art_id: None,
                    render_id: None,
                    order_id: None,
                    primary_color: None,
                    secondary_color: None,
                }
            })
            .collect();
        leagues.sort_by_key(|l| l.code);
        let mut groups = vocab.as_ref().map(|v| v.groups.clone()).unwrap_or_default();
        // NHL07's local group identities are established by stock membership; resolve their
        // labels through this project's own dictionary instead of fixed English text.
        if game_id == Some("nhl07") {
            if let Some(dictionary) = dictionary {
                for group in &mut groups {
                    let key = match (group.field.as_str(), group.raw) {
                        ("rasz", 0) => Some("WesternConference"), ("rasz", 1) => Some("EasternConference"),
                        ("xgeX", 0) => Some("NortheastDivision"), ("xgeX", 1) => Some("AtlanticDivision"),
                        ("xgeX", 2) => Some("SoutheastDivision"), ("xgeX", 3) => Some("CentralDivision"),
                        ("xgeX", 4) => Some("NorthwestDivision"), ("xgeX", 5) => Some("PacificDivision"),
                        _ => None,
                    };
                    if group.league_raw == 0 {
                        if let Some(text) = key.and_then(|key| dictionary.dictionary.text(key)) { group.label = text.to_string(); }
                    }
                }
            }
        }
        let mut cat = Self::finish(
            LeagueSource::DerivedCodes,
            team_league_table,
            vocab.map(|v| v.name),
            reason,
            leagues,
            groups,
        );
        cat.dictionary = dictionary.map(|d| DictionaryRef {
            path: d.path.clone(),
            locale: d.locale.clone(),
            sha256: d.sha256.clone(),
            rows: d.dictionary.len(),
        });
        Self::attach_competition(&mut cat, db, vocabs, game_id, platform, dictionary);
        cat
    }

    /// Attach the title's competition group table when one is baked for this target and the
    /// open database's team rows agree with it; otherwise record why not (only when the team
    /// row has group fields at all — a title without them has nothing to label).
    fn attach_competition(
        cat: &mut LeagueCatalog,
        db: &TdbFile,
        vocabs: &LeagueVocabularies,
        game_id: Option<&str>,
        platform: Option<crate::Platform>,
        dictionary: Option<&AttachedDictionary>,
    ) {
        use crate::competition_groups::{Provenance, ResolvedGroups, Signature};
        use sha2::Digest;
        let has_group_fields = db
            .table_canonical(&cat.team_league_table)
            .is_some_and(|t| field_by_studio(db, t, "iIBl").is_some());
        if !has_group_fields {
            return;
        }
        let Some(g) = game_id else {
            cat.competition_blocker = Some(
                "the session has no game id, so no title's competition group table can be selected; group ids are shown raw".to_string(),
            );
            return;
        };
        let Some(table) = vocabs.group_table_for(g, cat.source, platform) else {
            cat.competition_blocker = Some(format!(
                "no competition group table is baked for {g}{} ({} league source); group ids are shown raw",
                platform.map(|p| format!(" on {}", p.as_str())).unwrap_or_default(),
                cat.source.as_str()
            ));
            return;
        };
        let mut resolved = ResolvedGroups::resolve(table, g, platform, dictionary);
        resolved.derive_unknown_league_group_labels(|code| cat.league(code).map(|l| l.display()));
        let Some(fit) = resolved.check_fit(db, &cat.team_league_table) else {
            cat.competition_blocker = Some(format!(
                "the {} group table expects the team-row fields {} / {} / {}, which this database lacks; group ids are shown raw",
                table.game, table.fields.league, table.fields.conference, table.fields.division
            ));
            return;
        };
        // Title provenance (R1 F2): a recorded database identity verifies the claim; otherwise
        // the structural signature must identify the title, and a structure that identifies
        // another baked title — or several — never gains this table's labels as verified.
        let provenance = match &table.signature {
            None => {
                // Older tables without a signature: the row agreement is all there is, and it
                // is compatibility, not identity — say so and keep the evidence by reference.
                if fit.fit == 0 || fit.fit <= fit.conflict + fit.misplaced {
                    cat.competition_blocker = Some(format!(
                        "the {} group table does not describe this database: {}; the database was opened as {g} but its team rows do not use that title's group ids, so they are shown raw",
                        table.game,
                        fit.summary()
                    ));
                    return;
                }
                resolved.demote_evidence();
                Provenance::Ambiguous { also: vec!["(no structural signature recorded for this table)".to_string()] }
            }
            Some(sig) => {
                let identity = crate::competition_groups::hex(&sha2::Sha256::digest(db.bytes()));
                if let Some(k) = table.known_databases.iter().find(|k| k.sha256 == identity && k.role == "excluded") {
                    // A recorded identity known NOT to be this title (the repository nhl20 anchor
                    // is an NHL 19 base copy): refused by name, never labelled.
                    cat.competition_blocker = Some(format!(
                        "this database is a recorded file that is not {}: {}; group ids are shown raw",
                        table.game, k.source
                    ));
                    return;
                }
                if let Some(k) = table.known_databases.iter().find(|k| k.sha256 == identity) {
                    Provenance::Recorded { role: k.role.clone(), sha256: k.sha256.clone() }
                } else {
                    let Some(measured) = Signature::of(db) else {
                        cat.competition_blocker = Some("this database has no team table; group ids are shown raw".to_string());
                        return;
                    };
                    // Every baked title (same source) whose signature the database matches.
                    let matching: Vec<String> = vocabs
                        .group_tables()
                        .iter()
                        .filter(|t| t.source_and_platform_ok(cat.source, platform))
                        .filter(|t| t.signature.as_ref() == Some(&measured))
                        .map(|t| t.game.clone())
                        .collect();
                    if *sig != measured {
                        cat.competition_blocker = Some(format!(
                            "this database was opened as {g}, but its structure ({} tables, team capacity {}{}) is not {}'s{}; group ids are shown raw rather than labelled with another title's names",
                            measured.tables,
                            measured.team_max_records,
                            measured.stockleagues_max_records.map(|n| format!(", {n} league rows")).unwrap_or_default(),
                            table.game,
                            if matching.is_empty() {
                                " and matches no baked title".to_string()
                            } else {
                                format!(" — it identifies as {}", matching.join(" / "))
                            }
                        ));
                        return;
                    }
                    let also: Vec<String> = matching.into_iter().filter(|m| *m != table.game).collect();
                    if also.is_empty() {
                        Provenance::Structural
                    } else {
                        resolved.demote_evidence();
                        Provenance::Ambiguous { also }
                    }
                }
            }
        };
        resolved.provenance = Some(provenance);
        resolved.fit = Some(fit);
        cat.competition = Some(resolved);
    }

    fn finish(
        source: LeagueSource,
        team_league_table: String,
        vocabulary: Option<String>,
        unresolved_reason: Option<String>,
        leagues: Vec<LeagueIdentity>,
        groups: Vec<GroupEntry>,
    ) -> LeagueCatalog {
        let by_code = leagues
            .iter()
            .enumerate()
            .map(|(i, l)| (l.code, i))
            .collect();
        LeagueCatalog {
            source,
            dictionary: None,
            team_league_table,
            vocabulary,
            unresolved_reason,
            leagues,
            groups,
            by_code,
            competition: None,
            competition_blocker: None,
        }
    }

    pub fn league(&self, code: u64) -> Option<&LeagueIdentity> {
        self.by_code.get(&code).map(|&i| &self.leagues[i])
    }

    /// The text to show for a league code: label, else key, else `League code N`.
    pub fn display(&self, code: u64) -> String {
        self.league(code)
            .map(|l| l.display())
            .unwrap_or_else(|| format!("League code {code}"))
    }

    /// A group-id label (`OwTu` / `CGMK` / `iIBl`) anchored under `league_code`, if any.
    pub fn group_label(&self, field: &str, league_code: u64, raw: u64) -> Option<String> {
        // The title's competition group table (global ids) wins when it applies; the vocabulary's
        // per-league anchored labels are the fallback for titles without one.
        if let Some(c) = &self.competition {
            if field == c.fields.league || field == c.fields.conference || field == c.fields.division
            {
                return c.group(raw).map(|g| g.label.clone());
            }
        }
        self.groups
            .iter()
            .find(|g| g.field == field && g.league_raw == league_code && g.raw == raw)
            .map(|g| g.label.clone())
    }

    /// Attach league display text to the team-table `league` (`xMjj`) field view, without
    /// pretending it is an enum vocabulary.
    pub fn decorate_team_field(&self, f: &mut FieldView) {
        if f.studio_field != TEAM_LEAGUE_FIELD || f.studio_table != self.team_league_table {
            return;
        }
        if let Some(code) = f.decoded.and_then(|v| u64::try_from(v).ok()) {
            if let Some(l) = self.league(code) {
                if l.label.is_some() || l.name_key.is_some() {
                    f.display = l.display();
                    f.enum_label = Some(l.display());
                }
            }
        }
        f.enum_options = self
            .leagues
            .iter()
            .map(|l| (l.code as i64, l.display()))
            .collect();
    }
}

/// One league's detail: its identity, the editable `stockleagues` row (PS4) and its member teams.
#[derive(Clone, Debug)]
pub struct LeagueView {
    pub identity: LeagueIdentity,
    /// The `stockleagues` row's fields (PS4 only; empty on derived-code titles).
    pub fields: Vec<FieldView>,
    /// Studio id of the row's table when `fields` is populated (`fsLK`).
    pub row_table: Option<String>,
    /// Byte width of the `name` key field on this title (the edit limit), when the row exists.
    pub name_max_bytes: Option<u32>,
    /// Teams whose league code is this league, in team-table order.
    pub members: Vec<crate::view::TeamSummary>,
    /// Why the league's name cannot be edited on this title, when it cannot.
    pub name_edit_blocker: Option<String>,
    /// Phase 1.2: the league's competition structure on this title, when a group table applies.
    pub structure: Option<LeagueStructure>,
    /// Why no structure is shown although the team row carries group ids (P1-C): no table for
    /// this target, no game id, or a table the rows contradict.
    pub structure_blocker: Option<String>,
}

/// A league's groups as this title's table lists them (NHL 12 → Legacy family).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeagueStructure {
    /// `traced` / `by-reference` for this target.
    pub evidence: String,
    pub league_group: crate::competition_groups::ResolvedGroup,
    pub conferences: Vec<crate::competition_groups::ResolvedGroup>,
    pub divisions: Vec<crate::competition_groups::ResolvedGroup>,
    /// Tournament pools the league's rows carry in the conference and division slots (PS4:
    /// Spengler Cup, Champions Hockey League, the NHL 17 World Cup).
    pub pools: Vec<crate::competition_groups::ResolvedGroup>,
    /// Other groups in the league's block (gen7 tournament pools, playoffs).
    pub other: Vec<crate::competition_groups::ResolvedGroup>,
    /// Other league codes sharing this block (PS4 Custom Teams rows, Alumni / All-Time), with
    /// their display names.
    pub shared_with: Vec<(u64, String)>,
    /// The witness's provenance caveats (patch databases, anchor identity).
    pub witness_notes: Vec<String>,
    /// Dictionary keys this league's groups share with groups of other leagues (renaming the key
    /// renames every sharer): key → labels of the other leagues' groups.
    pub shared_keys: Vec<(String, Vec<String>)>,
    /// The witnessed regular (conference, division) arrangements of the block (R1 F1).
    pub pairs: Vec<(u64, u64)>,
    /// Exceptional stock arrangements, described with their rows (never offered).
    pub exceptional: Vec<String>,
    /// How the open database was tied to this title (R1 F2).
    pub provenance: Option<String>,
}

impl LeagueCatalog {
    /// The league's competition structure from the title's group table, when one applies.
    pub fn structure(&self, code: u64) -> Option<LeagueStructure> {
        let c = self.competition.as_ref()?;
        let lg = c.league_group(code)?.clone();
        let take = |kind: &str| c.for_league(code, kind).into_iter().cloned().collect::<Vec<_>>();
        let other = c
            .all()
            .filter(|g| g.league_codes.contains(&code) && !matches!(g.kind.as_str(), "league" | "conference" | "division" | "pool"))
            .cloned()
            .collect();
        let mine: Vec<_> = c.all().filter(|g| g.league_codes.contains(&code)).collect();
        let shared_with: Vec<(u64, String)> = lg
            .league_codes
            .iter()
            .filter(|&&o| o != code)
            .map(|&o| (o, self.display(o)))
            .collect();
        let mut shared_keys = Vec::new();
        for g in &mine {
            if g.key.is_empty() {
                continue;
            }
            let others: Vec<String> = c
                .all()
                .filter(|o| o.key == g.key && !o.league_codes.contains(&code))
                .map(|o| {
                    let league = o
                        .league_code
                        .and_then(|lc| c.league_group(lc))
                        .map(|l| l.label.clone())
                        .unwrap_or_else(|| "?".into());
                    format!("{} ({league})", o.label)
                })
                .collect();
            if !others.is_empty() && !shared_keys.iter().any(|(k, _)| *k == g.key) {
                shared_keys.push((g.key.clone(), others));
            }
        }
        Some(LeagueStructure {
            evidence: c.evidence.clone(),
            league_group: lg,
            conferences: take("conference"),
            divisions: take("division"),
            pools: take("pool"),
            other,
            shared_keys,
            shared_with,
            witness_notes: c.witness_notes.clone(),
            pairs: c.regular_pairs(code),
            exceptional: c
                .exceptional_pairs(code)
                .iter()
                .map(|p| {
                    let name = |id: u64| c.group(id).map(|g| g.label.clone()).unwrap_or_else(|| format!("group {id}"));
                    format!(
                        "{} / {} — {} ({} row{}: team id{} {})",
                        name(p.conference),
                        name(p.division),
                        p.note.as_deref().unwrap_or("exceptional"),
                        p.rows,
                        if p.rows == 1 { "" } else { "s" },
                        if p.teams.len() == 1 { "" } else { "s" },
                        p.teams.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ")
                    )
                })
                .collect(),
            provenance: c.provenance.as_ref().map(|p| p.describe()),
        })
    }

    /// Why league names are not directly editable on this target, if they are not.
    pub fn name_edit_blocker(&self) -> Option<String> {
        match self.source {
            LeagueSource::StockLeagues => None,
            LeagueSource::DerivedCodes => Some(
                "This title stores no league name in its database: teams carry only a league code,                  and the displayed name lives in the localization dictionary (Phase 1.3). The code                  and its member teams are shown for reference."
                    .to_string(),
            ),
        }
    }
}

/// Count team rows per league code, reading whichever table carries the code: the fat `kOtt`
/// (NHL 08+) or NHL 07's stock team table.
fn team_code_counts(db: &TdbFile) -> (String, BTreeMap<u64, usize>) {
    let mut counts = BTreeMap::new();
    for table_id in [
        tdb_relations::entity::ids::TEAM,
        tdb_relations::entity::ids::STOCK_TEAM,
    ] {
        let Some(t) = db.table_canonical(table_id) else {
            continue;
        };
        let Some(f) = field_by_studio(db, t, TEAM_LEAGUE_FIELD) else {
            continue;
        };
        for rec in 0..t.current_records as u32 {
            if let Some(v) = db.read_u64(t, rec, f) {
                *counts.entry(v).or_insert(0) += 1;
            }
        }
        return (table_id.to_string(), counts);
    }
    (tdb_relations::entity::ids::TEAM.to_string(), counts)
}

/// The league code of one team record (`kOtt` row), following NHL 07's thin index when the fat
/// table has no `league` field.
pub fn team_league_code(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    kott: &Table,
    record: u32,
) -> Option<u64> {
    if let Some(f) = field_by_studio(db, kott, TEAM_LEAGUE_FIELD) {
        return db.read_u64(kott, record, f);
    }
    let src = crate::view::team_name_source(db, meta, kott, record);
    let f = field_by_studio(db, src.table, TEAM_LEAGUE_FIELD)?;
    db.read_u64(src.table, src.record, f)
}

pub fn field_by_studio<'a>(
    db: &TdbFile,
    table: &'a Table,
    studio_field: &str,
) -> Option<&'a tdb_core::FieldDescriptor> {
    table
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == studio_field)
}

fn descriptor_by_name<'a>(
    meta: &FieldMetaIndex,
    table: &'a Table,
    name: &str,
) -> Option<&'a tdb_core::FieldDescriptor> {
    table.fields.iter().find(|f| {
        meta.field(&table.id.raw(), &f.id.raw())
            .map(|m| m.name == name)
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocabulary_parses_and_labels() {
        let text = r#"
name = "t"
applies_to = ["nhl99"]
[schema]
league_table = "kOtt"
league_field = "xMjj"
league_width = 4
[[entries]]
raw = 0
label = "NHL"
confidence = "candidate"
[[groups]]
field = "OwTu"
league_raw = 0
raw = 1
label = "Western"
confidence = "candidate"
"#;
        let v = LeagueVocabulary::from_toml(text).unwrap();
        assert_eq!(v.entry(0).unwrap().label, "NHL");
        assert_eq!(v.entry(0).unwrap().confidence, LabelConfidence::Candidate);
        assert_eq!(v.groups[0].label, "Western");
    }
}
