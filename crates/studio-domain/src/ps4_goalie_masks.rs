//! Equipment 4.5 **G3** (receipt 51; research 2026-09-28 handoff 02 / R135): PS4 NHL 15–21 goalie mask pairs.
//!
//! On PS4 a goalie's mask is chosen by two stored numbers together — `maskid` (the design) and `masktype` (the helmet
//! type) — and the title's own vault catalogue (`ge_nhl_goaliehelmettexturepack`: `GoalieMaskTexture`,
//! `GoalieHelmetBrand`, four typed `ge_texture` references) has at most one pack per pair on every title. This module
//! classifies a stored pair against that catalogue: **unique**, **not a catalogue helmet type** (99 on NHL 19–21),
//! **only under other helmet types**, **absent**, or **ambiguous** (never measured; kept as a class). The last four are
//! reported, never refused. A team's mask paint slot has no helmet type of its own, so a paint is listed with the helmet
//! types that carry it. Nothing here says which pack the game draws or whether it rejects a pair.
//!
//! Identity first (contract §6.1): the catalogue is read only from the attached install's `boot.big`, and only when its
//! four vault members match the recorded identity of the **open project's title** (`defs/ps4/vault-identities.json`);
//! another title's vault is refused as a source mismatch.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::ps4_vault::{self, Schema, Value, Vault, VaultSource};
use crate::{DomainError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaskPack {
    pub maskid: i64,
    pub helmet_type: i64,
    /// The pack's `DebugName` (an internal asset name — behind disclosure only).
    pub debug_name: Option<String>,
    /// The colour map's `FileRef` (`…_cm`), when a typed reference resolves to one.
    pub colour_map: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MaskCatalogue {
    pub title: String,
    pub source: VaultSource,
    pub packs: Vec<MaskPack>,
    /// Helmet types (brands) the catalogue carries.
    pub helmet_types: BTreeSet<i64>,
    by_pair: BTreeMap<(i64, i64), Vec<usize>>,
    by_mask: BTreeMap<i64, BTreeSet<i64>>,
    /// Receipt 54 (contract §6.6): a thumbnail per catalogue pair — from a Creation Zone list row (NHL 17–21), or on NHL 19–21
    /// by the name convention `mask<helmet type × 1000 + mask>` (a **candidate**, labelled). NHL 15 / 16: none.
    pub thumbnails: BTreeMap<(i64, i64), (crate::ps4_skater_gear::Thumb, ThumbBasis)>,
    /// Receipt 57: `(maskid, helmet type) → the list row's name` in the title's own dictionary (R141; list rows only).
    pub names: BTreeMap<(i64, i64), String>,
    /// Why names are not shown.
    pub names_problem: Option<String>,
}

/// How a mask thumbnail was joined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbBasis {
    /// A `goaliemaskinfo.csv` row names this pair and its `assetid` thumbnail.
    CreationZoneRow,
    /// NHL 19–21: `mask<brand × 1000 + maskid>` exists — the list's own rule applied beyond its rows (a name join).
    NameConvention,
}

impl ThumbBasis {
    pub fn as_str(self) -> &'static str {
        match self {
            ThumbBasis::CreationZoneRow => "creation_zone_row",
            ThumbBasis::NameConvention => "name_convention_candidate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairClass {
    Unique(MaskPack),
    /// The stored helmet type is no catalogue helmet type (e.g. 99).
    NotCatalogueHelmetType,
    /// The design exists only for these other helmet types.
    OnlyOtherHelmetTypes(Vec<i64>),
    /// No pack carries the design.
    Absent,
    /// Several packs carry the pair (never observed; none is chosen).
    Ambiguous(Vec<MaskPack>),
}

impl PairClass {
    pub fn code(&self) -> &'static str {
        match self {
            PairClass::Unique(_) => "unique",
            PairClass::NotCatalogueHelmetType => "not_a_catalogue_helmet_type",
            PairClass::OnlyOtherHelmetTypes(_) => "only_other_helmet_types",
            PairClass::Absent => "absent",
            PairClass::Ambiguous(_) => "ambiguous",
        }
    }

    /// Plain words (contract §6.2); numbers as stored, no brand names.
    pub fn describe(&self, maskid: i64, helmet_type: i64) -> String {
        match self {
            PairClass::Unique(_) => format!("Mask {maskid} · helmet type {helmet_type}: one catalogue mask"),
            PairClass::NotCatalogueHelmetType => format!("stored helmet type {helmet_type}; no catalogue helmet has that type"),
            PairClass::OnlyOtherHelmetTypes(t) => format!(
                "mask {maskid} exists only for helmet type{} {}, not {helmet_type}",
                if t.len() == 1 { "" } else { "s" },
                t.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ")
            ),
            PairClass::Absent => format!("mask {maskid}: no catalogue record on this title"),
            PairClass::Ambiguous(p) => format!("{} catalogue masks carry mask {maskid} · helmet type {helmet_type}; none is chosen", p.len()),
        }
    }
}

impl MaskCatalogue {
    /// Build from parsed vault parts (the identity is the caller's).
    pub fn from_vault(title: &str, source: VaultSource, vault: &Vault, schema: &Schema) -> Result<Self> {
        let class = ps4_vault::key("ge_nhl_goaliehelmettexturepack");
        // The two numbers are found by their **enum type** (`…Enums::GoalieMaskTexture` / `…::GoalieHelmetBrand`), as
        // research R135 reads them: the design attribute's own name is not `GoalieMaskTexture` (its key is unresolved).
        let by_type = |suffix: &str| {
            schema.classes.get(&class).and_then(|attrs| attrs.iter().find(|a| schema.type_name(a.type_key).ends_with(suffix)).map(|a| a.key))
        };
        let (Some(k_mask), Some(k_brand)) = (by_type("Enums::GoalieMaskTexture::GoalieMaskTexture"), by_type("Enums::GoalieHelmetBrand::GoalieHelmetBrand")) else {
            return Err(DomainError::Validation("PS4 vault: the goalie mask pack class carries no mask / helmet-type enum".into()));
        };
        let k_debug = ps4_vault::key("DebugName");
        let tex = ps4_vault::key("ge_texture");
        let mut packs = Vec::new();
        for &(off, size) in vault.records_of(class) {
            let attrs = vault.decode(schema, off, size)?;
            let int = |k: u32| attrs.iter().find_map(|(a, v)| match (*a == k, v) {
                (true, Value::Int(i)) => Some(*i),
                _ => None,
            });
            let (Some(maskid), Some(helmet_type)) = (int(k_mask), int(k_brand)) else {
                return Err(DomainError::Validation(format!("PS4 vault: a goalie mask pack at {off:#x} carries no mask / helmet-type value")));
            };
            let debug_name = attrs.iter().find_map(|(a, v)| match (*a == k_debug, v) {
                (true, Value::Text(t)) => t.clone(),
                _ => None,
            });
            let mut colour_map = None;
            for (_, v) in &attrs {
                if let Value::Ref { class, key, resolved: true } = v {
                    if *class == tex {
                        if let Some(fr) = vault.texture_file_ref(schema, *class, *key)? {
                            if fr.to_ascii_lowercase().ends_with("_cm") {
                                colour_map = Some(fr);
                            }
                        }
                    }
                }
            }
            packs.push(MaskPack { maskid, helmet_type, debug_name, colour_map });
        }
        if packs.is_empty() {
            return Err(DomainError::Validation("PS4 vault: no goalie mask packs (ge_nhl_goaliehelmettexturepack)".into()));
        }
        let mut by_pair: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
        let mut by_mask: BTreeMap<i64, BTreeSet<i64>> = BTreeMap::new();
        for (i, p) in packs.iter().enumerate() {
            by_pair.entry((p.maskid, p.helmet_type)).or_default().push(i);
            by_mask.entry(p.maskid).or_default().insert(p.helmet_type);
        }
        let helmet_types = packs.iter().map(|p| p.helmet_type).collect();
        Ok(MaskCatalogue { title: title.to_string(), source, packs, helmet_types, by_pair, by_mask, thumbnails: BTreeMap::new(), names: BTreeMap::new(), names_problem: None })
    }

    /// Read the attached install's vault for `project_title` (identity checked) and build the catalogue.
    pub fn load(install_root: &Path, defs_root: &Path, project_title: &str) -> Result<Self> {
        let (source, [v, b, sv, sb]) = ps4_vault::read_install_vault(install_root, defs_root)?;
        match source.title.as_deref() {
            Some(t) if t == project_title => {}
            Some(t) => {
                return Err(DomainError::Validation(format!(
                    "source mismatch: the attached install's vault is {t}'s, but the open project is {project_title}; Studio never borrows another title's catalogue"
                )))
            }
            None => {
                return Err(DomainError::Validation(format!(
                    "the attached install's vault ({}) matches no recorded PS4 title identity; the catalogue is not read",
                    source.members.iter().map(|(n, s)| format!("{n} {}", &s[..12])).collect::<Vec<_>>().join(", ")
                )))
            }
        }
        let schema = Schema::parse(&sv, &sb)?;
        let vault = Vault::parse(v, b)?;
        let mut cat = Self::from_vault(project_title, source, &vault, &schema)?;
        let (thumbnails, rows) = mask_thumbnails(install_root, defs_root, project_title, &cat)?;
        cat.thumbnails = thumbnails;
        if !rows.is_empty() {
            match crate::ps4_cz_names::CzNames::load(install_root, defs_root, project_title) {
                Ok(Some(n)) => {
                    for (pair, (name, style, pattern)) in rows {
                        if let Some(t) = n.mask_name(&name, &style, &pattern).filter(|t| !t.is_empty()) {
                            cat.names.entry(pair).or_insert(t);
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => cat.names_problem = Some(e.to_string()),
            }
        }
        Ok(cat)
    }

    /// The list row's name of a pair in the title's own dictionary (receipt 57).
    pub fn name(&self, maskid: i64, helmet_type: i64) -> Option<&str> {
        self.names.get(&(maskid, helmet_type)).map(String::as_str)
    }

    /// The thumbnail of a catalogue pair and how it was joined.
    pub fn thumbnail(&self, maskid: i64, helmet_type: i64) -> Option<&(crate::ps4_skater_gear::Thumb, ThumbBasis)> {
        self.thumbnails.get(&(maskid, helmet_type))
    }

    pub fn classify(&self, maskid: i64, helmet_type: i64) -> PairClass {
        match self.by_pair.get(&(maskid, helmet_type)).map(Vec::as_slice) {
            Some([i]) => PairClass::Unique(self.packs[*i].clone()),
            Some(many) if many.len() > 1 => PairClass::Ambiguous(many.iter().map(|i| self.packs[*i].clone()).collect()),
            _ if !self.helmet_types.contains(&helmet_type) => PairClass::NotCatalogueHelmetType,
            _ => match self.by_mask.get(&maskid) {
                Some(types) => PairClass::OnlyOtherHelmetTypes(types.iter().copied().collect()),
                None => PairClass::Absent,
            },
        }
    }

    /// The helmet types a team slot's paint exists for (empty = no catalogue record).
    pub fn paint_helmet_types(&self, paint: i64) -> Vec<i64> {
        self.by_mask.get(&paint).map(|s| s.iter().copied().collect()).unwrap_or_default()
    }
}

/// One `exhibitiongoalieequipment` row's stored pair and its class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalieRowPair {
    pub record: u32,
    pub maskid: i64,
    pub helmet_type: i64,
    pub class: PairClass,
}

/// Every goalie equipment row of the working database, classified (the census research R135 §3 reports).
pub fn goalie_rows(db: &tdb_core::TdbFile, meta: &tdb_defs::FieldMetaIndex, cat: &MaskCatalogue) -> Result<Vec<GoalieRowPair>> {
    let t = crate::team_reid::table_by_name(db, meta, "exhibitiongoalieequipment")
        .ok_or_else(|| DomainError::Validation("no exhibitiongoalieequipment table on this title".into()))?;
    let (fm, ft) = (
        crate::view::descriptor_by_name(meta, t, "maskid").ok_or_else(|| DomainError::Validation("no maskid field".into()))?,
        crate::view::descriptor_by_name(meta, t, "masktype").ok_or_else(|| DomainError::Validation("no masktype field".into()))?,
    );
    Ok((0..t.current_records as u32)
        .filter_map(|r| {
            let (m, h) = (db.read_u64(t, r, fm)? as i64, db.read_u64(t, r, ft)? as i64);
            Some(GoalieRowPair { record: r, maskid: m, helmet_type: h, class: cat.classify(m, h) })
        })
        .collect())
}

impl crate::EditSession {
    /// G3 (receipt 51): every goalie row's mask pair classified against `cat`.
    pub fn ps4_goalie_rows(&self, cat: &MaskCatalogue) -> Result<Vec<GoalieRowPair>> {
        let db = tdb_core::TdbFile::parse(self.bytes())?;
        goalie_rows(&db, self.project().meta(), cat)
    }
}

#[derive(serde::Deserialize)]
struct MaskCsvIds {
    titles: BTreeMap<String, MaskCsvId>,
}
#[derive(serde::Deserialize)]
struct MaskCsvId {
    member: String,
    sha256: String,
    assetid_rule: String,
}

/// Receipt 54: mask thumbnails for `title` (contract §6.6). The list is read only when it hashes to the recorded value.
type MaskRowKeys = BTreeMap<(i64, i64), (String, String, String)>;

/// Also returns each list row's `(name, stylename, patternname)` keys by pair (receipt 57).
fn mask_thumbnails(install_root: &Path, defs_root: &Path, title: &str, cat: &MaskCatalogue) -> Result<(BTreeMap<(i64, i64), (crate::ps4_skater_gear::Thumb, ThumbBasis)>, MaskRowKeys)> {
    let mut out = BTreeMap::new();
    let mut keys = MaskRowKeys::new();
    let ids: MaskCsvIds = serde_json::from_str(&std::fs::read_to_string(defs_root.join("ps4/goalie-mask-csv.json")).map_err(|e| DomainError::Validation(format!("goalie-mask list identities: {e}")))?)
        .map_err(|e| DomainError::Validation(format!("goalie-mask list identities: {e}")))?;
    let Some(want) = ids.titles.get(title) else { return Ok((out, keys)) }; // NHL 15 / 16: no list, no join
    let databigs = install_root.join("databigs");
    let thumbs = crate::ps4_skater_gear::creationzone_thumbnails(&databigs)?;
    let inxp = databigs.join("inxp_00.big");
    let entries = ps4_vault::eb_index(&inxp)?;
    let bytes = ps4_vault::eb_member(&inxp, &entries, &want.member)?;
    use sha2::Digest;
    let sha = format!("{:x}", sha2::Sha256::digest(&bytes));
    if !sha.eq_ignore_ascii_case(&want.sha256) {
        return Err(DomainError::Validation(format!("{}: SHA-256 {} is not the recorded {title} list; no mask thumbnails are joined", want.member, &sha[..12])));
    }
    let table = crate::ps4_skater_gear::csv_table(&String::from_utf8_lossy(&bytes));
    let header = table.first().cloned().unwrap_or_default();
    let col = |n: &str| header.iter().position(|h| h.eq_ignore_ascii_case(n));
    let (Some(ac), Some(sc), Some(pc)) = (col("assetid"), col("stylerenderassetid"), col("patternrenderassetid")) else {
        return Err(DomainError::Validation(format!("{}: missing assetid / style / pattern columns", want.member)));
    };
    let (nc, snc, pnc) = (col("name"), col("stylename"), col("patternname"));
    for r in table.iter().skip(1) {
        let (Some(a), Some(st), Some(pa)) = (r.get(ac).and_then(|x| x.parse::<i64>().ok()), r.get(sc).and_then(|x| x.parse::<i64>().ok()), r.get(pc).and_then(|x| x.parse::<i64>().ok())) else { continue };
        let cell = |c: Option<usize>| c.and_then(|c| r.get(c)).cloned().unwrap_or_default();
        keys.entry((pa, st)).or_insert((cell(nc), cell(snc), cell(pnc)));
        if let Some(t) = thumbs.get(&("mask".to_string(), a)) {
            out.insert((pa, st), (t.clone(), ThumbBasis::CreationZoneRow));
        }
    }
    if want.assetid_rule == "style_x1000_plus_pattern" {
        for p in &cat.packs {
            let key = (p.maskid, p.helmet_type);
            if out.contains_key(&key) {
                continue;
            }
            if let Some(t) = thumbs.get(&("mask".to_string(), p.helmet_type * 1000 + p.maskid)) {
                out.insert(key, (t.clone(), ThumbBasis::NameConvention));
            }
        }
    }
    Ok((out, keys))
}
