//! Phase 1.2 — the mode-setup **settings catalogue**, read-only (D2 / P1-BS-2).
//!
//! Two representations of the same catalogue exist on PS4 and Studio reads both into one model:
//!
//! * **NHL 15** stores it in five database tables (`stocksettings`, `stocksettingdefaults`,
//!   `stocksettingcontext`, `stocksettinggroups`, `stocksettingdependencies`), decoded through the
//!   declaration-scoped offset-binary sites of P1-BS-1 (`tdb-defs`). Context and group **names**,
//!   and the code → label arrays, are not in the database: they come from `defs/settings/ps4-nhl15.toml`
//!   (NHL 16/21 lineage names, the executable's label arrays) and are always marked **inferred**.
//! * **NHL 16–21** keep it as plain CSV members of `databigs/inxp_00.big` in three schema variants
//!   (`setting.csv` columns decide): NHL 16 (one global cell, placeholder choices → NHL 15 arrays by
//!   lineage, inferred), NHL 17 (global = the reserved `_DEFAULT_` context row, explicit choice
//!   lists) and NHL 18–21 (global stored twice — `setting.csv` and the `_DEFAULT_` row — shown as
//!   a pair with its consistency, never collapsed into one cell).
//!
//! Row identity is the **legacy id** (`stocksettings.legacyid` = `setting.csv id`, the same
//! numbering on every PS4 title). A shared *setting identity* (`stocksettings.id` / NHL 17
//! `legacy_id`) is informational: siblings are listed, never targets. Nothing here says which
//! default the game offers or applies, nor which of two stored copies it reads (handoff 22 §9).
//! Gen7 / PS3 / Recompiled / NHL 07–09 have no data-side representation in the inspected sources
//! and are reported as blocked with the reason.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use tdb_archive_big::eb::{self, EbHeader, Payload};
use tdb_core::TdbFile;
use tdb_defs::{FieldMetaIndex, OffsetEvidence};

use crate::leagues::{resolve_text, AttachedDictionary};
use crate::DomainError;

/// The 21 legacy ids of the accepted stored-default contract (handoff 22 rev. 3 §2.1; the
/// P1-BS-R1 acceptance lists them explicitly — 67 and 85 share a displayed row).
pub const ELIGIBLE_LEGACY_IDS: [i64; 21] = [
    67, 68, 69, 70, 71, 72, 73, 77, 83, 85, 86, 87, 88, 89, 116, 117, 118, 119, 120, 123, 212,
];

/// Archive path prefix of the NHL 16–21 catalogue members.
pub const CSV_MEMBER_PREFIX: &str = "nhl/data/common/gamemodes/setting_csv/";

/// The reserved NHL 17–21 context that carries the global default.
pub const DEFAULT_CONTEXT: i64 = -2;

// ============================================================================ defs

#[derive(Debug, Deserialize)]
struct DefsFile {
    game: String,
    platform: String,
    #[serde(default)]
    applies_to: Vec<String>,
    witness: WitnessDef,
    #[serde(default)]
    contexts: Vec<ContextDef>,
    #[serde(default)]
    groups: Vec<GroupDef>,
    #[serde(default)]
    labels: Vec<LabelDef>,
    #[serde(default)]
    eligible: Vec<EligibleDef>,
    #[serde(default)]
    excluded: Vec<ExcludedDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WitnessDef {
    pub cusa: String,
    pub executable_sha256: String,
    pub base_database_sha256: String,
    pub patch_database_sha256: String,
    pub metadata_sha256: String,
    pub descriptor_table_va: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ContextDef {
    id: i64,
    #[serde(default)]
    nhl16_key: String,
    #[serde(default)]
    nhl21_name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GroupDef {
    id: i64,
    #[serde(default)]
    nhl16_name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct LabelDef {
    legacyid: i64,
    #[serde(default)]
    exe_name: String,
    exe_id: i64,
    keys: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct EligibleDef {
    legacyid: i64,
    role: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ExcludedDef {
    legacyid: i64,
    reason: String,
}

/// One title's support data (`defs/settings/<platform>-<game>.toml`).
#[derive(Debug, Clone)]
pub struct TitleSettingsDefs {
    pub game: String,
    pub platform: String,
    pub applies_to: Vec<String>,
    pub witness: WitnessDef,
    contexts: BTreeMap<i64, (String, String)>,
    groups: BTreeMap<i64, String>,
    labels: BTreeMap<i64, LabelDef>,
    eligible: BTreeMap<i64, String>,
    excluded: BTreeMap<i64, String>,
}

impl TitleSettingsDefs {
    fn from_toml(text: &str) -> Result<Self, DomainError> {
        let f: DefsFile = toml::from_str(text).map_err(|e| DomainError::Validation(format!("defs/settings: {e}")))?;
        Ok(Self {
            game: f.game,
            platform: f.platform,
            applies_to: f.applies_to,
            witness: f.witness,
            contexts: f.contexts.into_iter().map(|c| (c.id, (c.nhl16_key, c.nhl21_name))).collect(),
            groups: f.groups.into_iter().map(|g| (g.id, g.nhl16_name)).collect(),
            labels: f.labels.into_iter().map(|l| (l.legacyid, l)).collect(),
            eligible: f.eligible.into_iter().map(|e| (e.legacyid, e.role)).collect(),
            excluded: f.excluded.into_iter().map(|e| (e.legacyid, e.reason)).collect(),
        })
    }

    /// The lineage name of an NHL 15 context: the NHL 16 key, else the NHL 21 name.
    pub fn context_lineage_name(&self, id: i64) -> Option<String> {
        let (k16, n21) = self.contexts.get(&id)?;
        if !k16.is_empty() {
            Some(k16.clone())
        } else if !n21.is_empty() {
            Some(n21.clone())
        } else {
            None
        }
    }

    pub fn group_lineage_name(&self, id: i64) -> Option<&str> {
        self.groups.get(&id).map(String::as_str).filter(|s| !s.is_empty())
    }

    /// The executable's label keys for a legacy id (array order = code), if the record has any.
    pub fn label_keys(&self, legacyid: i64) -> Option<&[String]> {
        self.labels.get(&legacyid).map(|l| l.keys.as_slice())
    }

    /// The executable descriptor record's own name and setting id for a legacy id (the witness
    /// of the offset reading), if the record carries labels.
    pub fn executable_record(&self, legacyid: i64) -> Option<(&str, i64)> {
        self.labels.get(&legacyid).map(|l| (l.exe_name.as_str(), l.exe_id))
    }

    pub fn eligible_role(&self, legacyid: i64) -> Option<&str> {
        self.eligible.get(&legacyid).map(String::as_str)
    }

    pub fn excluded_reason(&self, legacyid: i64) -> Option<&str> {
        self.excluded.get(&legacyid).map(String::as_str)
    }
}

/// Every title's settings support data (`defs/settings/*.toml`); empty when the directory is absent.
#[derive(Debug, Clone, Default)]
pub struct SettingsDefs {
    titles: Vec<TitleSettingsDefs>,
}

impl SettingsDefs {
    pub fn load_dir(dir: impl AsRef<Path>) -> Result<Self, DomainError> {
        let dir = dir.as_ref();
        let mut titles = Vec::new();
        let Ok(rd) = std::fs::read_dir(dir) else {
            return Ok(Self::default());
        };
        let mut paths: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "toml")).collect();
        paths.sort();
        for p in paths {
            let text = std::fs::read_to_string(&p).map_err(|source| DomainError::Io { path: p.display().to_string(), source })?;
            titles.push(TitleSettingsDefs::from_toml(&text).map_err(|e| DomainError::Validation(format!("{}: {e}", p.display())))?);
        }
        Ok(Self { titles })
    }

    pub fn from_toml_texts(texts: &[&str]) -> Result<Self, DomainError> {
        Ok(Self { titles: texts.iter().map(|t| TitleSettingsDefs::from_toml(t)).collect::<Result<_, _>>()? })
    }

    /// The defs whose `applies_to` names `game` on `platform`.
    pub fn for_title(&self, game: &str, platform: &str) -> Option<&TitleSettingsDefs> {
        self.titles.iter().find(|t| t.platform == platform && (t.game == game || t.applies_to.iter().any(|g| g == game)))
    }

    /// The NHL 15 PS4 defs (also the lineage/label source for NHL 16, whose executable arrays
    /// are identical — handoff 22 §3).
    pub fn nhl15_ps4(&self) -> Option<&TitleSettingsDefs> {
        self.for_title("nhl15", "ps4")
    }

    pub fn is_empty(&self) -> bool {
        self.titles.is_empty()
    }
}

// ============================================================================ model

/// Where the catalogue was read from.
#[derive(Debug, Clone)]
pub enum CatalogueSource {
    /// NHL 15 PS4: the five tables of the open database.
    Database {
        /// `base 01.00` / `patch 01.06` (recorded identities) or `unrecorded` (an edited or
        /// unknown build; the shape still matched).
        copy: String,
        database_sha256: String,
        tables: Vec<TableShape>,
    },
    /// NHL 16–21 PS4: the `setting_csv` members.
    SettingCsv {
        /// The archive the members were read from, or the directory (fixtures).
        origin: String,
        variant: CsvVariant,
        members: Vec<MemberInfo>,
    },
}

#[derive(Debug, Clone)]
pub struct TableShape {
    pub name: String,
    pub raw_id: String,
    pub rows: u16,
    pub max_records: u16,
}

#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub path: String,
    pub plain_len: usize,
    pub plain_sha256: String,
    /// `chunkzip` / `raw` (archive), `file` (directory of plain members).
    pub stored_form: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvVariant {
    Nhl16,
    Nhl17,
    Nhl18Plus,
}

impl CsvVariant {
    pub fn as_str(self) -> &'static str {
        match self {
            CsvVariant::Nhl16 => "nhl16",
            CsvVariant::Nhl17 => "nhl17",
            CsvVariant::Nhl18Plus => "nhl18-21",
        }
    }
}

/// How a displayed name was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameSource {
    /// Read from this title's own data.
    Explicit,
    /// Carried by id lineage from another title (NHL 16 keys / NHL 21 names for NHL 15 contexts).
    Lineage,
    /// Inferred from the executable's label arrays (array order = code).
    Inferred,
    None,
}

impl NameSource {
    pub fn as_str(self) -> &'static str {
        match self {
            NameSource::Explicit => "explicit",
            NameSource::Lineage => "lineage",
            NameSource::Inferred => "inferred",
            NameSource::None => "none",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContextInfo {
    pub id: i64,
    pub name: Option<String>,
    pub name_source: NameSource,
    /// Stored per-context defaults for this context.
    pub defaults_rows: usize,
    /// Layout rows (groups shown on the screen).
    pub layout_rows: usize,
    /// The reserved `_DEFAULT_` context (NHL 17–21 global store) — never an ordinary screen.
    pub reserved: bool,
}

#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub id: i64,
    pub name: Option<String>,
    pub name_source: NameSource,
    pub parent: Option<i64>,
    pub members: usize,
}

#[derive(Debug, Clone)]
pub struct Choice {
    pub code: i64,
    pub key: String,
    /// The dictionary text for `key`, when a dictionary is attached and resolves it.
    pub text: Option<String>,
}

/// One stored default: the global one (one cell, or the NHL 18–21 pair) or a per-context one.
#[derive(Debug, Clone)]
pub struct StoredDefault {
    /// `None` = the cell is empty (a null stored default).
    pub code: Option<i64>,
    pub label: Option<String>,
    /// Exact cell references (table/record/field or member/line/column), one per stored copy.
    pub cells: Vec<String>,
    /// NHL 18–21 global: the two copies' values; `consistent` says whether they agree.
    pub values: Vec<Option<i64>>,
    pub paired: bool,
    pub consistent: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ContextDefault {
    pub context: i64,
    pub context_name: Option<String>,
    pub stored: StoredDefault,
    /// The row's own domain when it differs from the setting's (NHL 17–21 `potential_values`
    /// per context); `None` = same as the setting's choices.
    pub choices: Option<Vec<Choice>>,
    pub domain_note: Option<String>,
    pub enabled: Option<bool>,
    pub visible: Option<bool>,
    pub anomaly: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eligibility {
    /// One of the 21 contract ids, present on this title.
    Eligible { role: String },
    /// Named in the contract's exclusion list, with the reason shown to the user.
    Excluded { reason: String },
    /// Not a league/competition setting of the contract.
    OutOfScope,
}

#[derive(Debug, Clone)]
pub struct SettingRow {
    /// Row identity: `stocksettings.legacyid` / `setting.csv id`.
    pub legacy_id: i64,
    /// `stringid` (NHL 15) / `key` (NHL 16) / `name` (NHL 17–21).
    pub name: String,
    /// The dictionary key of the setting's label (`stringid` / `string_id` / `name_loc`) and its
    /// resolved text when a dictionary is attached.
    pub label_key: Option<String>,
    pub label_text: Option<String>,
    /// Shared setting identity (`stocksettings.id` / NHL 17 `legacy_id`), when the title stores one.
    pub setting_id: Option<i64>,
    pub siblings: Vec<i64>,
    pub is_choice: Option<bool>,
    pub style: Option<i64>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub size: Option<i64>,
    pub global: Option<StoredDefault>,
    pub context_defaults: Vec<ContextDefault>,
    pub choices: Vec<Choice>,
    pub label_source: NameSource,
    pub groups: Vec<i64>,
    pub dependents: Vec<i64>,
    pub eligibility: Eligibility,
    pub anomalies: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SettingsCatalogue {
    pub game: String,
    pub platform: String,
    pub source: CatalogueSource,
    pub contexts: Vec<ContextInfo>,
    pub groups: Vec<GroupInfo>,
    pub settings: Vec<SettingRow>,
    /// Contract ids that this title's catalogue does not carry (NHL 17–21: 83, 89, 118, 119, 123).
    pub absent_eligible: Vec<i64>,
    /// Pre-existing anomalies of the source, reported once and preserved.
    pub anomalies: Vec<String>,
    /// Evidence and limitation notes shown with the catalogue.
    pub notes: Vec<String>,
    /// The attached dictionary used for label text, if any.
    pub dictionary: Option<String>,
}

impl SettingsCatalogue {
    pub fn setting(&self, legacy_id: i64) -> Option<&SettingRow> {
        self.settings.iter().find(|s| s.legacy_id == legacy_id)
    }
    pub fn context(&self, id: i64) -> Option<&ContextInfo> {
        self.contexts.iter().find(|c| c.id == id)
    }
}

/// Why no catalogue can be shown for this project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsBlocker {
    /// `unsupported-title` / `no-install-root` / `archive-missing` / `declaration-mismatch` /
    /// `member-refused` / `parse`.
    pub kind: &'static str,
    pub reason: String,
}

fn blocker(kind: &'static str, reason: impl Into<String>) -> SettingsBlocker {
    SettingsBlocker { kind, reason: reason.into() }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn dict_text(dict: Option<&AttachedDictionary>, key: &str) -> Option<String> {
    dict.and_then(|d| resolve_text(d, key).map(|(t, _)| t.to_string()))
}

fn choices_from_keys(keys: &[String], dict: Option<&AttachedDictionary>) -> Vec<Choice> {
    keys.iter()
        .enumerate()
        .map(|(i, k)| Choice { code: i as i64, key: k.clone(), text: dict_text(dict, k) })
        .collect()
}

fn label_for(choices: &[Choice], code: Option<i64>) -> Option<String> {
    let code = code?;
    let c = choices.iter().find(|c| c.code == code)?;
    Some(c.text.clone().unwrap_or_else(|| c.key.clone()))
}

fn eligibility_for(defs: Option<&TitleSettingsDefs>, legacy_id: i64, own_role: Option<&str>) -> Eligibility {
    if ELIGIBLE_LEGACY_IDS.contains(&legacy_id) {
        let role = own_role
            .map(str::to_string)
            .or_else(|| defs.and_then(|d| d.eligible_role(legacy_id)).map(str::to_string))
            .unwrap_or_default();
        return Eligibility::Eligible { role };
    }
    if let Some(reason) = defs.and_then(|d| d.excluded_reason(legacy_id)) {
        return Eligibility::Excluded { reason: reason.to_string() };
    }
    Eligibility::OutOfScope
}

// ============================================================================ NHL 15 database

const T_SETTINGS: &str = "fRvu";
const T_DEFAULTS: &str = "gZtX";
const T_CONTEXT: &str = "HwRX";
const T_GROUPS: &str = "Oreg";
const T_DEPS: &str = "FzWx";

/// True iff the database carries the five NHL 15 catalogue tables.
pub fn database_has_catalogue(db: &TdbFile) -> bool {
    [T_SETTINGS, T_DEFAULTS, T_CONTEXT, T_GROUPS, T_DEPS].iter().all(|t| db.table(t).is_some())
}

struct Reader<'a> {
    db: &'a TdbFile<'a>,
    meta: &'a FieldMetaIndex,
}

impl<'a> Reader<'a> {
    fn int(&self, table: &tdb_core::Table, rec: u32, raw_field: &str) -> Option<i64> {
        let f = table.field(raw_field)?;
        let m = self.meta.field(&table.id.raw(), raw_field)?;
        Some(m.decode_raw(self.db.read_u64(table, rec, f)?))
    }
    fn text(&self, table: &tdb_core::Table, rec: u32, raw_field: &str) -> Option<String> {
        let f = table.field(raw_field)?;
        Some(String::from_utf8_lossy(self.db.read_inline_str(table, rec, f)?).into_owned())
    }
}

/// Read the NHL 15 PS4 catalogue from the five tables of `db`.
pub fn read_database(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    defs: Option<&TitleSettingsDefs>,
    dict: Option<&AttachedDictionary>,
) -> Result<SettingsCatalogue, SettingsBlocker> {
    if !database_has_catalogue(db) {
        return Err(blocker("unsupported-title", "this database carries no settings-catalogue tables (stocksettings …)"));
    }
    // The declaration scope of P1-BS-1: without it the four columns would be misread.
    for (t, f) in [(T_SETTINGS, "SbEu"), (T_SETTINGS, "CkjP"), (T_CONTEXT, "QHRR"), (T_CONTEXT, "NVBd")] {
        match meta.field(t, f) {
            Some(m) if m.offset_binary => {}
            Some(_) => {
                return Err(blocker(
                    "declaration-mismatch",
                    format!("{}.{} is declared differently from the NHL 15 PS4 catalogue shape; its values would be misread, so the catalogue is not shown", meta.table_name(t).unwrap_or(t), m_name(meta, t, f)),
                ))
            }
            None => return Err(blocker("declaration-mismatch", format!("{t}.{f} is missing from the metadata"))),
        }
    }
    let r = Reader { db, meta };
    let settings_t = db.table(T_SETTINGS).unwrap();
    let defaults_t = db.table(T_DEFAULTS).unwrap();
    let context_t = db.table(T_CONTEXT).unwrap();
    let groups_t = db.table(T_GROUPS).unwrap();
    let deps_t = db.table(T_DEPS).unwrap();

    let mut anomalies = Vec::new();
    let mut notes = vec![
        "NHL 15 stores no context or group names: the names shown are the NHL 16 / NHL 21 names carried by the same numbering (lineage), not text observed on NHL 15.".to_string(),
        "Choice labels are the executable's label arrays in array order (inferred, UI code not traced); the raw code is the stored value.".to_string(),
        "Whether a creation screen offers a stored default, and whether a context default or the global default wins when both exist, is not established.".to_string(),
    ];
    if let Some(m) = meta.field(T_CONTEXT, "NVBd") {
        if m.offset_evidence == Some(OffsetEvidence::StructuralOnly) {
            notes.push("`subgrouporder` is read with a structurally inferred offset interpretation (no external witness); `subgroupparent` by lineage; `min`/`id` are witnessed by the executable's copy.".to_string());
        }
    }

    // --- stocksettings
    struct Raw {
        record: u32,
        legacyid: i64,
        stringid: String,
        id: i64,
        style: i64,
        min: i64,
        max: i64,
        size: i64,
        default: i64,
    }
    let mut raws = Vec::new();
    for rec in 0..settings_t.current_records as u32 {
        raws.push(Raw {
            record: rec,
            legacyid: r.int(settings_t, rec, "jjHq").unwrap_or(-1),
            stringid: r.text(settings_t, rec, "sAhV").unwrap_or_default(),
            id: r.int(settings_t, rec, "CkjP").unwrap_or(-1),
            style: r.int(settings_t, rec, "cVLR").unwrap_or(0),
            min: r.int(settings_t, rec, "SbEu").unwrap_or(0),
            max: r.int(settings_t, rec, "KODr").unwrap_or(0),
            size: r.int(settings_t, rec, "qrwM").unwrap_or(0),
            default: r.int(settings_t, rec, "npHF").unwrap_or(0),
        });
    }
    let mut legacy_counts: BTreeMap<i64, usize> = BTreeMap::new();
    for x in &raws {
        *legacy_counts.entry(x.legacyid).or_default() += 1;
    }
    for (l, n) in &legacy_counts {
        if *n > 1 {
            anomalies.push(format!("stocksettings: legacyid {l} occurs {n} times (ambiguous row identity)"));
        }
    }
    let by_id: BTreeMap<i64, Vec<i64>> = raws.iter().filter(|x| x.id >= 0).fold(BTreeMap::new(), |mut m, x| {
        m.entry(x.id).or_default().push(x.legacyid);
        m
    });
    let known: BTreeSet<i64> = raws.iter().map(|x| x.legacyid).collect();

    // --- stocksettingdefaults
    let mut defaults: BTreeMap<(i64, i64), Vec<(u32, i64)>> = BTreeMap::new();
    let mut defaults_per_context: BTreeMap<i64, usize> = BTreeMap::new();
    for rec in 0..defaults_t.current_records as u32 {
        let ctx = r.int(defaults_t, rec, "mmiA").unwrap_or(-1);
        let lid = r.int(defaults_t, rec, "jjHq").unwrap_or(-1);
        let d = r.int(defaults_t, rec, "npHF").unwrap_or(0);
        defaults.entry((ctx, lid)).or_default().push((rec, d));
        *defaults_per_context.entry(ctx).or_default() += 1;
    }
    let mut dangling: Vec<String> = Vec::new();
    for ((ctx, lid), rows) in &defaults {
        if !known.contains(lid) {
            dangling.push(format!("(context {ctx}, legacyid {lid})"));
        }
        if rows.len() > 1 {
            anomalies.push(format!("stocksettingdefaults: (context {ctx}, legacyid {lid}) stored {} times (ambiguous)", rows.len()));
        }
    }
    if !dangling.is_empty() {
        anomalies.push(format!("stocksettingdefaults: {} row(s) reference a legacyid with no stocksettings row: {}", dangling.len(), dangling.join(", ")));
    }

    // --- stocksettingcontext
    let mut layout_per_context: BTreeMap<i64, Vec<(i64, i64, i64, i64)>> = BTreeMap::new();
    for rec in 0..context_t.current_records as u32 {
        let ctx = r.int(context_t, rec, "mmiA").unwrap_or(-1);
        let group = r.int(context_t, rec, "OAja").unwrap_or(-1);
        let parent = r.int(context_t, rec, "QHRR").unwrap_or(-1);
        let order = r.int(context_t, rec, "ORkb").unwrap_or(0);
        let sub_order = r.int(context_t, rec, "NVBd").unwrap_or(-1);
        layout_per_context.entry(ctx).or_default().push((group, parent, order, sub_order));
    }

    // --- stocksettinggroups
    let mut group_members: BTreeMap<i64, Vec<(i64, i64)>> = BTreeMap::new();
    for rec in 0..groups_t.current_records as u32 {
        let g = r.int(groups_t, rec, "OAja").unwrap_or(-1);
        let lid = r.int(groups_t, rec, "jjHq").unwrap_or(-1);
        let order = r.int(groups_t, rec, "ORkb").unwrap_or(0);
        group_members.entry(g).or_default().push((order, lid));
    }
    for (g, m) in &group_members {
        let mut orders: BTreeMap<i64, usize> = BTreeMap::new();
        for (o, _) in m {
            *orders.entry(*o).or_default() += 1;
        }
        for (o, n) in orders {
            if n > 1 {
                anomalies.push(format!("stocksettinggroups: group {g} uses order {o} on {n} rows"));
            }
        }
    }
    let mut setting_groups: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
    for (g, m) in &group_members {
        for (_, lid) in m {
            setting_groups.entry(*lid).or_default().push(*g);
        }
    }
    // Groups a screen lists, and the container parents those rows hang under.
    let referenced_groups: BTreeSet<i64> = layout_per_context
        .values()
        .flatten()
        .flat_map(|(g, p, ..)| [Some(*g), (*p >= 0).then_some(*p)])
        .flatten()
        .collect();
    let empty_layout_groups: Vec<String> = referenced_groups.iter().filter(|g| !group_members.contains_key(g)).map(|g| g.to_string()).collect();
    if !empty_layout_groups.is_empty() {
        anomalies.push(format!("stocksettingcontext references group(s) with no member rows (containers): {}", empty_layout_groups.join(", ")));
    }
    let no_layout_but_defaults: Vec<String> = defaults_per_context.keys().filter(|c| !layout_per_context.contains_key(c)).map(|c| c.to_string()).collect();
    if !no_layout_but_defaults.is_empty() {
        anomalies.push(format!("context(s) with stored defaults but no layout rows: {}", no_layout_but_defaults.join(", ")));
    }

    // --- dependencies
    let mut dependents: BTreeMap<i64, Vec<(i64, i64)>> = BTreeMap::new();
    for rec in 0..deps_t.current_records as u32 {
        let lid = r.int(deps_t, rec, "jjHq").unwrap_or(-1);
        let dep = r.int(deps_t, rec, "ylMD").unwrap_or(-1);
        let order = r.int(deps_t, rec, "ORkb").unwrap_or(0);
        dependents.entry(lid).or_default().push((order, dep));
        if !known.contains(&dep) || !known.contains(&lid) {
            anomalies.push(format!("stocksettingdependencies: ({lid} -> {dep}) references an unknown legacyid"));
        }
    }

    // --- contexts / groups
    let all_contexts: BTreeSet<i64> = layout_per_context.keys().chain(defaults_per_context.keys()).copied().collect();
    let contexts: Vec<ContextInfo> = all_contexts
        .into_iter()
        .map(|id| {
            let name = defs.and_then(|d| d.context_lineage_name(id));
            ContextInfo {
                id,
                name_source: if name.is_some() { NameSource::Lineage } else { NameSource::None },
                name,
                defaults_rows: defaults_per_context.get(&id).copied().unwrap_or(0),
                layout_rows: layout_per_context.get(&id).map_or(0, Vec::len),
                reserved: false,
            }
        })
        .collect();
    let parent_of: BTreeMap<i64, i64> = layout_per_context.values().flatten().filter(|(_, p, ..)| *p >= 0).map(|(g, p, ..)| (*g, *p)).collect();
    let all_groups: BTreeSet<i64> = group_members.keys().chain(referenced_groups.iter()).copied().collect();
    let groups: Vec<GroupInfo> = all_groups
        .into_iter()
        .map(|id| {
            let name = defs.and_then(|d| d.group_lineage_name(id)).map(str::to_string);
            GroupInfo {
                id,
                name_source: if name.is_some() { NameSource::Lineage } else { NameSource::None },
                name,
                parent: parent_of.get(&id).copied(),
                members: group_members.get(&id).map_or(0, Vec::len),
            }
        })
        .collect();
    let context_name = |id: i64| defs.and_then(|d| d.context_lineage_name(id));

    // --- settings
    let mut settings = Vec::with_capacity(raws.len());
    for x in &raws {
        let mut row_anomalies = Vec::new();
        let keys = defs.and_then(|d| d.label_keys(x.legacyid));
        let choices = keys.map(|k| choices_from_keys(k, dict)).unwrap_or_default();
        if let Some(k) = keys {
            if x.size > 0 && k.len() as i64 != x.size {
                row_anomalies.push(format!("label array has {} entries but the row declares size {} (codes beyond the array are unlabelled)", k.len(), x.size));
            }
        }
        if let Some((_, exe_id)) = defs.and_then(|d| d.executable_record(x.legacyid)) {
            if exe_id != x.id {
                row_anomalies.push(format!("the executable's descriptor record carries setting id {exe_id}, the database {}", x.id));
            }
        }
        if x.size > 0 && x.size != x.max - x.min + 1 {
            row_anomalies.push(format!("size {} ≠ max − min + 1 ({}..{})", x.size, x.min, x.max));
        }
        if x.default < x.min || x.default > x.max {
            row_anomalies.push(format!("global default {} lies outside the row's own range {}..{} (observed on stock)", x.default, x.min, x.max));
        }
        let global = StoredDefault {
            code: Some(x.default),
            label: label_for(&choices, Some(x.default)),
            cells: vec![format!("stocksettings record {} (legacyid {}) field default", x.record, x.legacyid)],
            values: vec![Some(x.default)],
            paired: false,
            consistent: None,
        };
        let mut context_defaults = Vec::new();
        for ((ctx, lid), rows) in &defaults {
            if *lid != x.legacyid {
                continue;
            }
            for (rec, d) in rows {
                let anomaly = if *d < x.min || *d > x.max { Some(format!("stored context default {d} outside {}..{}", x.min, x.max)) } else { None };
                context_defaults.push(ContextDefault {
                    context: *ctx,
                    context_name: context_name(*ctx),
                    stored: StoredDefault {
                        code: Some(*d),
                        label: label_for(&choices, Some(*d)),
                        cells: vec![format!("stocksettingdefaults record {rec} (context {ctx}, legacyid {lid}) field default")],
                        values: vec![Some(*d)],
                        paired: false,
                        consistent: None,
                    },
                    choices: None,
                    domain_note: None,
                    enabled: None,
                    visible: None,
                    anomaly,
                });
            }
        }
        let siblings: Vec<i64> = by_id.get(&x.id).map(|v| v.iter().copied().filter(|l| *l != x.legacyid).collect()).unwrap_or_default();
        let mut deps: Vec<(i64, i64)> = dependents.get(&x.legacyid).cloned().unwrap_or_default();
        deps.sort();
        settings.push(SettingRow {
            legacy_id: x.legacyid,
            name: x.stringid.clone(),
            label_key: (!x.stringid.is_empty()).then(|| x.stringid.clone()),
            label_text: dict_text(dict, &x.stringid),
            setting_id: Some(x.id),
            siblings,
            is_choice: Some(x.size > 0),
            style: Some(x.style),
            min: Some(x.min),
            max: Some(x.max),
            size: Some(x.size),
            global: Some(global),
            context_defaults,
            label_source: if keys.is_some() { NameSource::Inferred } else { NameSource::None },
            choices,
            groups: setting_groups.get(&x.legacyid).cloned().unwrap_or_default(),
            dependents: deps.into_iter().map(|(_, d)| d).collect(),
            eligibility: eligibility_for(defs, x.legacyid, None),
            anomalies: row_anomalies,
        });
    }
    let in_no_group = settings.iter().filter(|s| s.groups.is_empty()).count();
    if in_no_group > 0 {
        anomalies.push(format!("{in_no_group} setting row(s) belong to no group"));
    }
    let absent_eligible: Vec<i64> = ELIGIBLE_LEGACY_IDS.iter().copied().filter(|l| !known.contains(l)).collect();

    let database_sha256 = hex(&Sha256::digest(db.bytes()));
    let copy = match defs {
        Some(d) if d.witness.base_database_sha256 == database_sha256 => "base 01.00".to_string(),
        Some(d) if d.witness.patch_database_sha256 == database_sha256 => "patch 01.06".to_string(),
        _ => "unrecorded (edited or unknown build; the declaration shape matched)".to_string(),
    };
    let tables = [(T_SETTINGS, settings_t), (T_DEFAULTS, defaults_t), (T_CONTEXT, context_t), (T_GROUPS, groups_t), (T_DEPS, deps_t)]
        .iter()
        .map(|(raw, t)| TableShape {
            name: meta.table_name(raw).unwrap_or(raw).to_string(),
            raw_id: raw.to_string(),
            rows: t.current_records,
            max_records: t.max_records,
        })
        .collect();
    notes.push("NHL 15 PS4 ships two database copies (inxp00.big base, patch_inxp00.big patch 01.06); which one the installed game reads is unresolved. The copy shown is the one this project opened.".to_string());
    Ok(SettingsCatalogue {
        game: "nhl15".to_string(),
        platform: "ps4".to_string(),
        source: CatalogueSource::Database { copy, database_sha256, tables },
        contexts,
        groups,
        settings,
        absent_eligible,
        anomalies,
        notes,
        dictionary: dict.map(|d| d.path.clone()),
    })
}

fn m_name(meta: &FieldMetaIndex, t: &str, f: &str) -> String {
    meta.field(t, f).map(|m| m.name.clone()).unwrap_or_else(|| f.to_string())
}

// ============================================================================ NHL 16–21 CSV

/// The plain text of the `setting_csv` members, keyed by file name (`setting.csv`, …).
#[derive(Debug, Clone, Default)]
pub struct SettingCsvMembers {
    pub origin: String,
    pub members: BTreeMap<String, (Vec<u8>, String)>,
}

impl SettingCsvMembers {
    /// Read the members out of a PS4 `EB\0\3` archive (streaming: front tables, then only the
    /// catalogue entries).
    pub fn from_eb_archive(path: &Path) -> Result<Self, SettingsBlocker> {
        let io = |e: std::io::Error| blocker("archive-missing", format!("{}: {e}", path.display()));
        let mut file = std::fs::File::open(path).map_err(io)?;
        let mut header_buf = vec![0u8; eb::HEADER_LEN];
        file.read_exact(&mut header_buf).map_err(io)?;
        let header = EbHeader::parse(&header_buf).map_err(|e| blocker("parse", format!("{}: {e}", path.display())))?;
        let mut table_buf = vec![0u8; header.table_end()];
        file.seek(SeekFrom::Start(0)).map_err(io)?;
        file.read_exact(&mut table_buf).map_err(io)?;
        let archive = eb::EbArchive::parse(&table_buf).map_err(|e| blocker("parse", format!("{}: {e}", path.display())))?;
        drop(table_buf);
        let mut members = BTreeMap::new();
        for entry in archive.entries.iter().filter(|e| e.path.starts_with(CSV_MEMBER_PREFIX)) {
            let name = entry.path[CSV_MEMBER_PREFIX.len()..].to_string();
            if name.contains('/') {
                continue;
            }
            let mut raw = vec![0u8; entry.size as usize];
            file.seek(SeekFrom::Start(entry.offset)).map_err(io)?;
            file.read_exact(&mut raw).map_err(io)?;
            let form = if raw.starts_with(tdb_archive_codec::MAGIC_CHUNKZIP) { "chunkzip" } else { "raw" };
            let data = match eb::decode_payload(raw) {
                Ok(Payload::Data(d)) => d,
                Ok(Payload::UnsupportedLzx) => return Err(blocker("member-refused", format!("{}: chunklzx is not decodable", entry.path))),
                Err(e) => return Err(blocker("member-refused", format!("{}: {e}", entry.path))),
            };
            if members.insert(name.clone(), (data, form.to_string())).is_some() {
                return Err(blocker("member-refused", format!("{}: the archive lists this member twice (ambiguous)", entry.path)));
            }
        }
        if members.is_empty() {
            return Err(blocker("archive-missing", format!("{}: no {CSV_MEMBER_PREFIX} member", path.display())));
        }
        Ok(Self { origin: path.display().to_string(), members })
    }

    /// Read plain members from a directory (private fixtures / tests).
    pub fn from_dir(dir: &Path) -> Result<Self, SettingsBlocker> {
        let rd = std::fs::read_dir(dir).map_err(|e| blocker("archive-missing", format!("{}: {e}", dir.display())))?;
        let mut members = BTreeMap::new();
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "csv") {
                let data = std::fs::read(&p).map_err(|e| blocker("archive-missing", format!("{}: {e}", p.display())))?;
                members.insert(p.file_name().unwrap().to_string_lossy().into_owned(), (data, "file".to_string()));
            }
        }
        if members.is_empty() {
            return Err(blocker("archive-missing", format!("{}: no .csv member", dir.display())));
        }
        Ok(Self { origin: dir.display().to_string(), members })
    }

    pub fn from_texts(origin: &str, texts: &[(&str, &str)]) -> Self {
        Self { origin: origin.to_string(), members: texts.iter().map(|(n, t)| (n.to_string(), (t.as_bytes().to_vec(), "text".to_string()))).collect() }
    }

    fn infos(&self) -> Vec<MemberInfo> {
        self.members
            .iter()
            .map(|(n, (d, f))| MemberInfo { path: format!("{CSV_MEMBER_PREFIX}{n}"), plain_len: d.len(), plain_sha256: hex(&Sha256::digest(d)), stored_form: f.clone() })
            .collect()
    }
}

/// A parsed member: header names and rows (1-based line numbers of the plain text).
#[derive(Debug, Clone)]
pub struct CsvTable {
    pub name: String,
    pub header: Vec<String>,
    pub rows: Vec<CsvRow>,
    /// Records whose column count differs from the header (kept out of `rows`, reported).
    pub ragged: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct CsvRow {
    pub line: usize,
    pub cells: Vec<String>,
}

impl CsvTable {
    fn col(&self, name: &str) -> Option<usize> {
        self.header.iter().position(|h| h == name)
    }
    fn get<'a>(&self, row: &'a CsvRow, name: &str) -> Option<&'a str> {
        self.col(name).and_then(|i| row.cells.get(i)).map(String::as_str)
    }
    fn int(&self, row: &CsvRow, name: &str) -> Option<i64> {
        self.get(row, name).and_then(|s| s.trim().parse().ok())
    }
}

/// Parse one member within the measured subset (handoff 01 §3.4): ASCII, no BOM, no `"`, comma
/// delimiter, CRLF or LF records; a required header named twice is refused.
pub fn parse_csv(name: &str, bytes: &[u8]) -> Result<CsvTable, SettingsBlocker> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Err(blocker("member-refused", format!("{name}: a byte-order mark is outside the supported subset")));
    }
    if let Some(pos) = bytes.iter().position(|b| *b == b'"') {
        return Err(blocker("member-refused", format!("{name}: a quote character at byte {pos} — quoting is not interpreted, the member is browse-only refused")));
    }
    if bytes.iter().any(|b| *b >= 0x80 || *b == 0) {
        return Err(blocker("member-refused", format!("{name}: non-ASCII or NUL bytes are outside the supported subset")));
    }
    let text = std::str::from_utf8(bytes).map_err(|e| blocker("member-refused", format!("{name}: {e}")))?;
    let mut lines: Vec<&str> = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    let Some(head) = lines.first() else {
        return Err(blocker("member-refused", format!("{name}: empty member")));
    };
    let header: Vec<String> = head.split(',').map(|s| s.trim().to_string()).collect();
    let mut seen = BTreeSet::new();
    for h in &header {
        if !seen.insert(h.clone()) {
            return Err(blocker("member-refused", format!("{name}: header column `{h}` appears twice (ambiguous binding)")));
        }
    }
    let mut rows = Vec::with_capacity(lines.len());
    let mut ragged = Vec::new();
    for (i, l) in lines.iter().enumerate().skip(1) {
        if l.is_empty() {
            continue;
        }
        let cells: Vec<String> = l.split(',').map(|s| s.to_string()).collect();
        if cells.len() != header.len() {
            ragged.push(i + 1);
            continue;
        }
        rows.push(CsvRow { line: i + 1, cells });
    }
    Ok(CsvTable { name: name.to_string(), header, rows, ragged })
}

fn detect_variant(setting: &CsvTable) -> Result<CsvVariant, SettingsBlocker> {
    let has = |c: &str| setting.col(c).is_some();
    if has("key") && has("remap_id") && has("default_value") && has("min_value") {
        Ok(CsvVariant::Nhl16)
    } else if has("legacy_id") && !has("default_value") {
        Ok(CsvVariant::Nhl17)
    } else if has("default_value") && has("potential_values") && has("is_choice") {
        Ok(CsvVariant::Nhl18Plus)
    } else {
        Err(blocker("parse", format!("setting.csv: unknown schema (columns {})", setting.header.join(","))))
    }
}

/// Read the NHL 16–21 catalogue from the `setting_csv` members. `game` names the title the
/// project opened as; the schema variant is detected from the members themselves.
pub fn read_setting_csv(
    game: &str,
    members: &SettingCsvMembers,
    nhl15_defs: Option<&TitleSettingsDefs>,
    dict: Option<&AttachedDictionary>,
) -> Result<SettingsCatalogue, SettingsBlocker> {
    let parsed = |n: &str| -> Result<CsvTable, SettingsBlocker> {
        let (bytes, _) = members.members.get(n).ok_or_else(|| blocker("archive-missing", format!("{n}: member not found under {CSV_MEMBER_PREFIX}")))?;
        parse_csv(n, bytes)
    };
    let setting = parsed("setting.csv")?;
    let variant = detect_variant(&setting)?;
    let context = parsed("context.csv")?;
    let contextsetting = parsed("contextsetting.csv")?;
    let group = parsed("group.csv")?;
    let contextgroup = parsed("contextgroup.csv")?;
    let membership = match variant {
        CsvVariant::Nhl16 => parsed("settinggroup.csv").ok(),
        _ => parsed("groupsetting.csv").ok(),
    };
    let deps = parsed("stocksettingdependencies.csv").ok();
    let (choice, choicelist) = match variant {
        CsvVariant::Nhl16 => (None, None),
        _ => (Some(parsed("choice.csv")?), Some(parsed("choicelist.csv")?)),
    };

    let mut anomalies = Vec::new();
    let mut notes = vec![
        "Whether a creation screen offers a stored default, which context a screen reads, and what `is_enabled` / `is_visible` / `usage` mean are not established.".to_string(),
    ];
    for t in [&setting, &context, &contextsetting, &group, &contextgroup] {
        if !t.ragged.is_empty() {
            anomalies.push(format!("{}: {} record(s) with a column count different from the header (lines {:?}) were skipped", t.name, t.ragged.len(), &t.ragged[..t.ragged.len().min(8)]));
        }
    }

    // --- contexts
    let mut contexts: Vec<ContextInfo> = Vec::new();
    let mut context_names: BTreeMap<i64, String> = BTreeMap::new();
    for row in &context.rows {
        let Some(id) = context.int(row, "id") else { continue };
        let name = match variant {
            CsvVariant::Nhl16 => context.get(row, "key").unwrap_or("").to_string(),
            _ => context.get(row, "name").unwrap_or("").to_string(),
        };
        if context_names.insert(id, name.clone()).is_some() {
            anomalies.push(format!("context.csv: id {id} occurs twice"));
        }
        contexts.push(ContextInfo { id, name: (!name.is_empty()).then_some(name), name_source: NameSource::Explicit, defaults_rows: 0, layout_rows: 0, reserved: id == DEFAULT_CONTEXT });
    }
    let mut layout_per_context: BTreeMap<i64, usize> = BTreeMap::new();
    for row in &contextgroup.rows {
        if let Some(c) = contextgroup.int(row, "context_id") {
            *layout_per_context.entry(c).or_default() += 1;
        }
    }

    // --- groups
    let mut group_members: BTreeMap<i64, usize> = BTreeMap::new();
    let mut setting_groups: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
    if let Some(m) = &membership {
        for row in &m.rows {
            if let (Some(g), Some(s)) = (m.int(row, "group_id"), m.int(row, "setting_id")) {
                *group_members.entry(g).or_default() += 1;
                setting_groups.entry(s).or_default().push(g);
            }
        }
    }
    let groups: Vec<GroupInfo> = group
        .rows
        .iter()
        .filter_map(|row| {
            let id = group.int(row, "id")?;
            let name = group.get(row, "name").unwrap_or("").to_string();
            Some(GroupInfo {
                id,
                name: (!name.is_empty()).then_some(name),
                name_source: NameSource::Explicit,
                parent: group.int(row, "parent_id"),
                members: group_members.get(&id).copied().unwrap_or(0),
            })
        })
        .collect();

    // --- choice lists (17+)
    let mut lists: BTreeMap<i64, (String, Vec<Choice>, bool)> = BTreeMap::new();
    if let (Some(choice), Some(choicelist)) = (&choice, &choicelist) {
        for row in &choicelist.rows {
            if let Some(id) = choicelist.int(row, "id") {
                lists.insert(id, (choicelist.get(row, "name").unwrap_or("").to_string(), Vec::new(), true));
            }
        }
        for row in &choice.rows {
            let (Some(list), Some(cid), Some(value)) = (choice.int(row, "choicelist_id"), choice.int(row, "id"), choice.int(row, "value")) else { continue };
            let key = choice.get(row, "name_loc").filter(|s| !s.is_empty()).or(choice.get(row, "name")).unwrap_or("").to_string();
            let e = lists.entry(list).or_insert_with(|| (String::new(), Vec::new(), true));
            if cid != value {
                e.2 = false;
            }
            e.1.push(Choice { code: value, key: key.clone(), text: dict_text(dict, &key) });
        }
        for (_, v, _) in lists.values_mut() {
            v.sort_by_key(|c| c.code);
        }
    }

    // --- dependencies (16/17 only)
    let mut dependents: BTreeMap<i64, Vec<(i64, i64)>> = BTreeMap::new();
    if let Some(d) = &deps {
        for row in &d.rows {
            if let (Some(l), Some(o), Some(dep)) = (d.int(row, "LegacyID"), d.int(row, "Order"), d.int(row, "DependentLegacyID")) {
                dependents.entry(l).or_default().push((o, dep));
            }
        }
    }

    // --- contextsetting index
    let mut cs_index: BTreeMap<(i64, i64), Vec<&CsvRow>> = BTreeMap::new();
    let mut defaults_per_context: BTreeMap<i64, usize> = BTreeMap::new();
    let mut empty_defaults = 0usize;
    let known_settings: BTreeSet<i64> = setting.rows.iter().filter_map(|r| setting.int(r, "id")).collect();
    let mut dangling = 0usize;
    for row in &contextsetting.rows {
        let (Some(c), Some(s)) = (contextsetting.int(row, "context_id"), contextsetting.int(row, "setting_id")) else { continue };
        cs_index.entry((c, s)).or_default().push(row);
        *defaults_per_context.entry(c).or_default() += 1;
        if contextsetting.get(row, "default_value").is_some_and(|v| v.trim().is_empty()) {
            empty_defaults += 1;
        }
        if !known_settings.contains(&s) {
            dangling += 1;
        }
        if !context_names.contains_key(&c) {
            anomalies.push(format!("contextsetting.csv line {}: context {c} is not in context.csv", row.line));
        }
    }
    for ((c, s), rows) in &cs_index {
        if rows.len() > 1 {
            anomalies.push(format!("contextsetting.csv: (context {c}, setting {s}) stored {} times (ambiguous)", rows.len()));
        }
    }
    if empty_defaults > 0 {
        anomalies.push(format!("contextsetting.csv: {empty_defaults} row(s) store an empty default (null; never offered as a target)"));
    }
    if dangling > 0 {
        anomalies.push(format!("contextsetting.csv: {dangling} row(s) reference a setting id with no setting.csv row"));
    }
    for c in contexts.iter_mut() {
        c.defaults_rows = defaults_per_context.get(&c.id).copied().unwrap_or(0);
        c.layout_rows = layout_per_context.get(&c.id).copied().unwrap_or(0);
    }
    let context_name = |id: i64| context_names.get(&id).cloned().filter(|s| !s.is_empty());

    // --- shared identities (17: legacy_id)
    let mut by_identity: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
    if variant == CsvVariant::Nhl17 {
        for row in &setting.rows {
            if let (Some(id), Some(l)) = (setting.int(row, "id"), setting.int(row, "legacy_id")) {
                by_identity.entry(l).or_default().push(id);
            }
        }
    }

    // --- settings
    let mut settings = Vec::with_capacity(setting.rows.len());
    let mut seen_ids: BTreeMap<i64, usize> = BTreeMap::new();
    for row in &setting.rows {
        let Some(id) = setting.int(row, "id") else { continue };
        *seen_ids.entry(id).or_default() += 1;
        let mut row_anomalies = Vec::new();
        let name = match variant {
            CsvVariant::Nhl16 => setting.get(row, "key").unwrap_or("").to_string(),
            _ => setting.get(row, "name").unwrap_or("").to_string(),
        };
        let label_key = match variant {
            CsvVariant::Nhl16 => setting.get(row, "string_id"),
            _ => setting.get(row, "name_loc"),
        }
        .filter(|s| !s.is_empty())
        .map(str::to_string);
        let is_choice = setting.int(row, "is_choice").map(|v| v == 1);
        // NHL 17+: the reserved `_DEFAULT_` row and the choice list the setting's global uses.
        let default_row = if variant == CsvVariant::Nhl16 { None } else { cs_index.get(&(DEFAULT_CONTEXT, id)).and_then(|v| v.first().copied()) };
        let list_id: Option<i64> = if variant == CsvVariant::Nhl16 {
            None
        } else {
            default_row.and_then(|r| contextsetting.int(r, "potential_values")).or_else(|| setting.int(row, "potential_values"))
        };
        let (min, max, size, choices, label_source, global, setting_id) = match variant {
            CsvVariant::Nhl16 => {
                let min = setting.int(row, "min_value");
                let max = setting.int(row, "max_value");
                let size = match (min, max) {
                    (Some(lo), Some(hi)) => Some(hi - lo + 1),
                    _ => None,
                };
                // Placeholder choice.csv on NHL 16: the NHL 15 executable arrays by lineage (the
                // NHL 16 executable carries identical arrays — handoff 22 §3), inferred.
                let keys = nhl15_defs.and_then(|d| d.label_keys(id));
                let choices = match (keys, size) {
                    (Some(k), Some(n)) if k.len() as i64 == n && min == Some(0) => choices_from_keys(k, dict),
                    (Some(k), Some(n)) => {
                        row_anomalies.push(format!("NHL 15 label array has {} entries but this row spans {n} values — labels not applied", k.len()));
                        Vec::new()
                    }
                    _ => Vec::new(),
                };
                let src = if choices.is_empty() { NameSource::None } else { NameSource::Inferred };
                let dv = setting.get(row, "default_value").map(str::trim);
                let code = dv.filter(|s| !s.is_empty()).and_then(|s| s.parse::<i64>().ok());
                if dv.is_some_and(|s| !s.is_empty()) && code.is_none() {
                    row_anomalies.push(format!("setting.csv default_value `{}` is not an integer", dv.unwrap_or("")));
                }
                let g = StoredDefault {
                    code,
                    label: label_for(&choices, code),
                    cells: vec![format!("setting.csv line {} column default_value", row.line)],
                    values: vec![code],
                    paired: false,
                    consistent: None,
                };
                (min, max, size, choices, src, Some(g), None)
            }
            CsvVariant::Nhl17 | CsvVariant::Nhl18Plus => {
                let (choices, src) = match list_id.and_then(|l| lists.get(&l)) {
                    Some((_, ch, true)) => (ch.clone(), NameSource::Explicit),
                    Some((n, _, false)) => {
                        row_anomalies.push(format!("choice list `{n}` numbers its rows differently from their values (ambiguous binding); labels not applied"));
                        (Vec::new(), NameSource::None)
                    }
                    None => (Vec::new(), NameSource::None),
                };
                let (min, max, size) = if choices.is_empty() {
                    (None, None, None)
                } else {
                    (choices.iter().map(|c| c.code).min(), choices.iter().map(|c| c.code).max(), Some(choices.len() as i64))
                };
                let ctx_code = default_row.and_then(|r| contextsetting.get(r, "default_value")).map(str::trim).filter(|s| !s.is_empty()).and_then(|s| s.parse::<i64>().ok());
                let global = if variant == CsvVariant::Nhl17 {
                    default_row.map(|r| StoredDefault {
                        code: ctx_code,
                        label: label_for(&choices, ctx_code),
                        cells: vec![format!("contextsetting.csv line {} (_DEFAULT_ context {DEFAULT_CONTEXT}, setting {id}) column default_value", r.line)],
                        values: vec![ctx_code],
                        paired: false,
                        consistent: None,
                    })
                } else {
                    let set_code = setting.get(row, "default_value").map(str::trim).filter(|s| !s.is_empty()).and_then(|s| s.parse::<i64>().ok());
                    let set_list = setting.int(row, "potential_values");
                    let mut cells = vec![format!("setting.csv line {} column default_value", row.line)];
                    let mut values = vec![set_code];
                    let mut consistent = None;
                    if let Some(r) = default_row {
                        cells.push(format!("contextsetting.csv line {} (_DEFAULT_ context {DEFAULT_CONTEXT}, setting {id}) column default_value", r.line));
                        values.push(ctx_code);
                        let same_list = contextsetting.int(r, "potential_values") == set_list;
                        consistent = Some(set_code == ctx_code && same_list);
                        if set_code != ctx_code {
                            row_anomalies.push(format!("the two stored global copies disagree: setting.csv {:?} vs _DEFAULT_ row {:?}", set_code, ctx_code));
                        }
                        if !same_list {
                            row_anomalies.push("the two stored global copies name different choice lists".to_string());
                        }
                    } else {
                        row_anomalies.push("no _DEFAULT_ row: the global default is stored once, in setting.csv only".to_string());
                    }
                    Some(StoredDefault { code: set_code, label: label_for(&choices, set_code), cells, values, paired: default_row.is_some(), consistent })
                };
                let identity = if variant == CsvVariant::Nhl17 { setting.int(row, "legacy_id") } else { None };
                (min, max, size, choices, src, global, identity)
            }
        };
        if let Some(c) = setting.int(row, "choice_to_range") {
            if c != 0 {
                row_anomalies.push("range-based setting (choice_to_range ≠ 0): not a choice row".to_string());
            }
        }
        // per-context rows (17+: every context except the reserved global; 16: all)
        let mut context_defaults = Vec::new();
        for ((c, s), rows) in &cs_index {
            if *s != id || *c == DEFAULT_CONTEXT {
                continue;
            }
            for r in rows {
                let dv = contextsetting.get(r, "default_value").map(str::trim);
                let code = dv.filter(|s| !s.is_empty()).and_then(|s| s.parse::<i64>().ok());
                let mut anomaly = None;
                if dv.is_some_and(|s| !s.is_empty()) && code.is_none() {
                    anomaly = Some(format!("default_value `{}` is not an integer", dv.unwrap_or("")));
                }
                let (row_choices, domain_note) = match variant {
                    CsvVariant::Nhl16 => {
                        let over = ["min_value", "max_value", "choices"].iter().any(|c| contextsetting.get(r, c).is_some_and(|v| !v.trim().is_empty()));
                        (None, over.then(|| "per-context min/max/choices override present — not interpreted".to_string()))
                    }
                    _ => {
                        let row_list = contextsetting.int(r, "potential_values");
                        let global_list = list_id;
                        if row_list.is_some() && row_list != global_list {
                            match row_list.and_then(|l| lists.get(&l)) {
                                Some((n, ch, true)) => (Some(ch.clone()), Some(format!("this screen uses choice list `{n}` (differs from the setting's)"))),
                                Some((n, _, false)) => (None, Some(format!("choice list `{n}` numbers its rows differently from their values (ambiguous)"))),
                                None => (None, Some(format!("choice list {} not found", row_list.unwrap_or(-1)))),
                            }
                        } else {
                            (None, None)
                        }
                    }
                };
                let effective = row_choices.as_deref().unwrap_or(&choices);
                if let Some(code) = code {
                    if !effective.is_empty() && !effective.iter().any(|ch| ch.code == code) {
                        anomaly = Some(format!("stored default {code} is not a value of the row's choice list"));
                    }
                    if variant == CsvVariant::Nhl16 {
                        if let (Some(lo), Some(hi)) = (min, max) {
                            if code < lo || code > hi {
                                anomaly = Some(format!("stored default {code} outside {lo}..{hi}"));
                            }
                        }
                    }
                }
                context_defaults.push(ContextDefault {
                    context: *c,
                    context_name: context_name(*c),
                    stored: StoredDefault {
                        code,
                        label: label_for(effective, code),
                        cells: vec![format!("contextsetting.csv line {} (context {c}, setting {id}) column default_value", r.line)],
                        values: vec![code],
                        paired: false,
                        consistent: None,
                    },
                    choices: row_choices,
                    domain_note,
                    enabled: contextsetting.int(r, "is_enabled").map(|v| v == 1),
                    visible: contextsetting.int(r, "is_visible").map(|v| v == 1),
                    anomaly,
                });
            }
        }
        let siblings: Vec<i64> = setting_id.and_then(|l| by_identity.get(&l)).map(|v| v.iter().copied().filter(|x| *x != id).collect()).unwrap_or_default();
        let mut deps: Vec<(i64, i64)> = dependents.get(&id).cloned().unwrap_or_default();
        deps.sort();
        settings.push(SettingRow {
            legacy_id: id,
            label_text: label_key.as_deref().and_then(|k| dict_text(dict, k)),
            label_key,
            setting_id,
            siblings,
            is_choice,
            style: None,
            min,
            max,
            size,
            global,
            context_defaults,
            label_source,
            choices,
            groups: setting_groups.get(&id).cloned().unwrap_or_default(),
            dependents: deps.into_iter().map(|(_, d)| d).collect(),
            eligibility: eligibility_for(nhl15_defs, id, Some(&name)),
            anomalies: row_anomalies,
            name,
        });
    }
    for (id, n) in &seen_ids {
        if *n > 1 {
            anomalies.push(format!("setting.csv: id {id} occurs {n} times (ambiguous row identity)"));
        }
    }
    let absent_eligible: Vec<i64> = ELIGIBLE_LEGACY_IDS.iter().copied().filter(|l| !seen_ids.contains_key(l)).collect();
    match variant {
        CsvVariant::Nhl16 => notes.push("NHL 16's choice.csv is a placeholder (CHOICE_n); labels are the NHL 15 executable arrays carried by the same numbering (inferred). The global default is one cell of setting.csv.".to_string()),
        CsvVariant::Nhl17 => notes.push("The global default is the reserved `_DEFAULT_` context row (-2) of contextsetting.csv; choice labels are explicit (choice.csv name_loc).".to_string()),
        CsvVariant::Nhl18Plus => notes.push("The global default is stored twice (setting.csv and the `_DEFAULT_` row of contextsetting.csv); stock copies agree on every eligible row, which is an observation, not an engine requirement — an edit must change both as one paired operation.".to_string()),
    }
    if variant != CsvVariant::Nhl17 {
        notes.push("This title stores no shared setting identity column; sibling rows are not listed.".to_string());
    }
    Ok(SettingsCatalogue {
        game: game.to_string(),
        platform: "ps4".to_string(),
        source: CatalogueSource::SettingCsv { origin: members.origin.clone(), variant, members: members.infos() },
        contexts,
        groups,
        settings,
        absent_eligible,
        anomalies,
        notes,
        dictionary: dict.map(|d| d.path.clone()),
    })
}

/// `(member path, stored SHA-256, stored length)` of every catalogue entry of the archive, read
/// from the front tables and the entries' stored bytes only.
pub fn csv_stored_identities(path: &Path) -> Result<Vec<(String, String, usize)>, SettingsBlocker> {
    let io = |e: std::io::Error| blocker("archive-missing", format!("{}: {e}", path.display()));
    let mut file = std::fs::File::open(path).map_err(io)?;
    let mut header_buf = vec![0u8; eb::HEADER_LEN];
    file.read_exact(&mut header_buf).map_err(io)?;
    let header = EbHeader::parse(&header_buf).map_err(|e| blocker("parse", format!("{}: {e}", path.display())))?;
    let mut table_buf = vec![0u8; header.table_end()];
    file.seek(SeekFrom::Start(0)).map_err(io)?;
    file.read_exact(&mut table_buf).map_err(io)?;
    let archive = eb::EbArchive::parse(&table_buf).map_err(|e| blocker("parse", format!("{}: {e}", path.display())))?;
    let mut out = Vec::new();
    for entry in archive.entries.iter().filter(|e| e.path.starts_with(CSV_MEMBER_PREFIX)) {
        let mut raw = vec![0u8; entry.size as usize];
        file.seek(SeekFrom::Start(entry.offset)).map_err(io)?;
        file.read_exact(&mut raw).map_err(io)?;
        out.push((entry.path.clone(), hex(&Sha256::digest(&raw)), raw.len()));
    }
    Ok(out)
}

/// The archive a PS4 16–21 install keeps the catalogue in, under its install root.
pub fn csv_archive_path(install_root: &Path) -> PathBuf {
    install_root.join("databigs").join("inxp_00.big")
}

/// Why a non-PS4 (or NHL 07–09) target has no catalogue to show (handoff 22 §3).
pub fn unsupported_reason(platform: crate::Platform, game_id: Option<&str>) -> String {
    let g = game_id.unwrap_or("this title");
    match platform {
        crate::Platform::Ps4 => format!("{g} on PS4: no settings catalogue representation is known for this title"),
        crate::Platform::Xbox | crate::Platform::Recomp => match game_id {
            Some("nhl07") | Some("nhl08") | Some("nhl09") => format!("{g}: no settings catalogue was found in the executable or the loose files that were searched (handoff 22 §3)"),
            _ => format!("{g} on Xbox 360: the setting definitions live only in a compiled table of the executable (no database or data-file copy; per-screen defaults were not located). A read-only executable reader is not part of this checkpoint."),
        },
        crate::Platform::Ps3 => format!("{g} on PS3: the database carries no catalogue tables and the executable is encrypted; known only by reference to the Xbox 360 executable table"),
    }
}
