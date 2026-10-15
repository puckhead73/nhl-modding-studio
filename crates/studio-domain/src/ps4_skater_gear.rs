//! Equipment 4.1–4.4 (receipt 52; research 2026-09-28 handoff 02b / R136): PS4 skater helmet, gloves, skates, stick and
//! visor numbers checked against the title's own catalogue, read-only.
//!
//! On NHL 17–21 each family has a Creation Zone list (`nhl/data/common/gamemodes/creation_zone_csv/*.csv` in
//! `databigs/inxp_00.big`): a row per offered item with its number, a `$`-prefixed dictionary **key** for its name and a
//! thumbnail named by the number (`…/creationzone/equipment/<dir>/<dir><N>.gnf`). The vault catalogue carries the same
//! numbers under the family's enum (`HelmetBrand`, `GloveBrand`, `SkateBrand`, `StickBrand`, `HelmetVisorType`). A stored
//! value is reported as: a Creation Zone row (with its key behind disclosure and its thumbnail), a catalogue value
//! without a row, or outside both (e.g. helmet 99 on 17–21) — reported, never refused. NHL 15 / 16 ship no rows: the
//! enum check only. Names are keys, not display text (the dictionary route is not established); nothing says what the
//! game draws. Identity first: the rows are read only when the member's SHA-256 equals the recorded one for the open
//! project's title, and the vault only when it is the title's own (`ps4_vault`).
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::ps4_vault::{self, Schema, Vault};
use crate::{DomainError, Result};

/// `(family, database field name, thumbnail folder, id column, name column, vault class, enum)`.
pub const FAMILIES: [(&str, &str, &str, &str, &str, &str, &str); 5] = [
    ("helmet", "helmetbrand", "helmet", "assetid", "itemname", "ge_nhl_skaterhelmettexturepack", "HelmetBrand"),
    ("gloves", "gloves", "gloves", "assetid", "itemname", "ge_nhl_skaterglovestexturepack", "GloveBrand"),
    ("skates", "skatebrand", "skate", "skateassetid", "skatename", "ge_nhl_skaterskates", "SkateBrand"),
    ("stick", "stick", "stick", "stickassetid", "stickname", "ge_nhl_skatersticktexturepack", "StickBrand"),
    ("visor", "visortype", "visor", "assetid", "itemname", "ge_nhl_skaterhelmet", "HelmetVisorType"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GearRow {
    pub id: i64,
    /// The row's `$` name key (behind disclosure).
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumb {
    pub archive: String,
    pub member: String,
}

#[derive(Debug, Clone)]
pub struct FamilyCatalogue {
    pub family: String,
    pub field: String,
    /// `None` on NHL 15 / 16 (no Creation Zone list ships).
    pub rows: Option<Vec<GearRow>>,
    /// `(member, SHA-256)` of the list read.
    pub rows_source: Option<(String, String)>,
    pub enum_values: BTreeSet<i64>,
    pub thumbnails: BTreeMap<i64, Thumb>,
}

#[derive(Debug, Clone)]
pub struct SkaterGearCatalogue {
    pub title: String,
    pub families: Vec<FamilyCatalogue>,
    pub vault: ps4_vault::VaultSource,
    /// Receipt 57: the title's own dictionary for the list keys (R141); `None` on NHL 15 / 16 or when it could not be read.
    pub names: Option<crate::ps4_cz_names::CzNames>,
    /// Why names are not shown (a dictionary that is not the recorded one, or unreadable).
    pub names_problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GearCheck {
    pub family: String,
    pub value: i64,
    /// `row` / `catalogue_only` / `outside` / `enum_only_title` (15 / 16: in the catalogue) / `not_in_catalogue` (15 / 16).
    pub class: String,
    pub text: String,
    pub key: Option<String>,
    /// Receipt 57: the key's text in the title's own dictionary.
    pub name: Option<String>,
    pub thumbnail: Option<Thumb>,
}

#[derive(serde::Deserialize)]
struct CsvIds {
    titles: BTreeMap<String, BTreeMap<String, CsvId>>,
}
#[derive(serde::Deserialize)]
struct CsvId {
    member: Option<String>,
    sha256: Option<String>,
}

/// A small CSV reader (quoted fields, BOM) — enough for the shipped Creation Zone lists.
fn csv_rows(text: &str) -> Vec<Vec<String>> {
    let text = text.trim_start_matches('\u{feff}');
    let mut rows = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let mut cells = Vec::new();
        let (mut cur, mut quoted) = (String::new(), false);
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match (c, quoted) {
                ('"', true) if chars.peek() == Some(&'"') => {
                    cur.push('"');
                    chars.next();
                }
                ('"', _) => quoted = !quoted,
                (',', false) => cells.push(std::mem::take(&mut cur)),
                _ => cur.push(c),
            }
        }
        cells.push(cur);
        rows.push(cells.into_iter().map(|c| c.trim().to_string()).collect());
    }
    rows
}

impl SkaterGearCatalogue {
    pub fn load(install_root: &Path, defs_root: &Path, title: &str) -> Result<Self> {
        let (vsrc, [v, b, sv, sb]) = ps4_vault::read_install_vault(install_root, defs_root)?;
        match vsrc.title.as_deref() {
            Some(t) if t == title => {}
            Some(t) => return Err(DomainError::Validation(format!("source mismatch: the attached install's vault is {t}'s, but the open project is {title}"))),
            None => return Err(DomainError::Validation("the attached install's vault matches no recorded PS4 title identity".into())),
        }
        let schema = Schema::parse(&sv, &sb)?;
        let vault = Vault::parse(v, b)?;
        let ids: CsvIds = serde_json::from_str(&std::fs::read_to_string(defs_root.join("ps4/skater-gear-csv.json")).map_err(|e| DomainError::Validation(format!("skater-gear identities: {e}")))?)
            .map_err(|e| DomainError::Validation(format!("skater-gear identities: {e}")))?;
        let want = ids.titles.get(title).ok_or_else(|| DomainError::Validation(format!("no skater-gear record for {title}")))?;
        let databigs = install_root.join("databigs");
        let inxp = databigs.join("inxp_00.big");
        // NHL 15 / 16 record no lists (and NHL 15 names its archives differently): the archive is opened only when needed.
        let inxp_entries = if want.values().any(|c| c.member.is_some()) { ps4_vault::eb_index(&inxp)? } else { Vec::new() };
        let thumbs = creationzone_thumbnails(&databigs)?;
        let mut families = Vec::new();
        for (fam, field, dir, idc, namec, class, en) in FAMILIES {
            let enum_values = ps4_vault::enum_values(&vault, &schema, class, en)?;
            let (rows, rows_source) = match want.get(fam) {
                Some(CsvId { member: Some(m), sha256: Some(h) }) => {
                    let bytes = ps4_vault::eb_member(&inxp, &inxp_entries, m)?;
                    let sha = format!("{:x}", Sha256::digest(&bytes));
                    if !sha.eq_ignore_ascii_case(h) {
                        return Err(DomainError::Validation(format!("{m}: SHA-256 {} is not the recorded {title} list ({}…); the rows are not read", &sha[..12], &h[..12])));
                    }
                    let table = csv_rows(&String::from_utf8_lossy(&bytes));
                    let header = table.first().cloned().unwrap_or_default();
                    let col = |n: &str| header.iter().position(|h| h.eq_ignore_ascii_case(n));
                    let (Some(ic), Some(nc)) = (col(idc), col(namec)) else {
                        return Err(DomainError::Validation(format!("{m}: no `{idc}` / `{namec}` column")));
                    };
                    let rows = table[1..].iter().filter_map(|r| Some(GearRow { id: r.get(ic)?.parse().ok()?, key: r.get(nc).cloned().unwrap_or_default() })).collect();
                    (Some(rows), Some((m.clone(), sha)))
                }
                _ => (None, None),
            };
            let thumbnails = if rows.is_some() {
                thumbs.iter().filter(|((d, _), _)| d == dir).map(|((_, n), t)| (*n, t.clone())).collect()
            } else {
                BTreeMap::new() // NHL 15 / 16: thumbnail numbers are not joined (R136 §2)
            };
            families.push(FamilyCatalogue { family: fam.into(), field: field.into(), rows, rows_source, enum_values, thumbnails });
        }
        // Receipt 57: names through the title's own dictionary; a problem there hides the names, never the check.
        let (names, names_problem) = if families.iter().any(|f| f.rows.is_some()) {
            match crate::ps4_cz_names::CzNames::load(install_root, defs_root, title) {
                Ok(n) => (n, None),
                Err(e) => (None, Some(e.to_string())),
            }
        } else {
            (None, None)
        };
        Ok(SkaterGearCatalogue { title: title.to_string(), families, vault: vsrc, names, names_problem })
    }

    pub fn check(&self, family: &str, value: i64) -> Option<GearCheck> {
        let f = self.families.iter().find(|f| f.family == family)?;
        let in_enum = f.enum_values.contains(&value);
        let (class, text, key, thumbnail) = match &f.rows {
            Some(rows) => match rows.iter().find(|r| r.id == value) {
                Some(r) => ("row", format!("{family} {value}: a Creation Zone item"), Some(r.key.clone()), f.thumbnails.get(&value).cloned()),
                None if in_enum => ("catalogue_only", format!("{family} {value}: in the catalogue but not a Creation Zone item"), None, None),
                None => ("outside", format!("stored {value}; not in the catalogue"), None, None),
            },
            None if in_enum => ("enum_only_title", format!("{family} {value}: in the catalogue (this title ships no Creation Zone list)"), None, None),
            None => ("not_in_catalogue", format!("stored {value}; not in the catalogue enum"), None, None),
        };
        let name = key.as_deref().and_then(|k| self.names.as_ref()?.resolve(k)).map(|(t, _)| t.trim().to_string()).filter(|t| !t.is_empty());
        Some(GearCheck { family: family.into(), value, class: class.into(), text, key, name, thumbnail })
    }
}

/// Every Creation Zone equipment thumbnail of an install: `(folder, number) → member`, from every `databigs/*.big` directory
/// (`…/creationzone/equipment/<dir>/<dir><N>.gnf`; the `_xl` copies are not listed). The first archive in name order wins.
pub fn creationzone_thumbnails(databigs: &Path) -> Result<BTreeMap<(String, i64), Thumb>> {
    let mut thumbs: BTreeMap<(String, i64), Thumb> = BTreeMap::new();
    let mut archives: Vec<PathBuf> = std::fs::read_dir(databigs)
        .map_err(|e| DomainError::Validation(format!("{}: {e}", databigs.display())))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("big")))
        .collect();
    archives.sort();
    for a in &archives {
        let Ok(entries) = ps4_vault::eb_index(a) else { continue };
        for e in entries {
            let l = e.path.to_ascii_lowercase();
            let Some(rest) = l.split("/creationzone/equipment/").nth(1) else { continue };
            let Some((dir, file)) = rest.split_once('/') else { continue };
            let Some(n) = file.strip_prefix(dir).and_then(|f| f.strip_suffix(".gnf")).and_then(|n| n.parse::<i64>().ok()) else { continue };
            thumbs.entry((dir.to_string(), n)).or_insert(Thumb { archive: a.file_name().unwrap().to_string_lossy().into_owned(), member: e.path.clone() });
        }
    }
    Ok(thumbs)
}

/// A small CSV reader shared with the goalie-mask list (receipt 54).
pub fn csv_table(text: &str) -> Vec<Vec<String>> {
    csv_rows(text)
}

/// A Creation Zone thumbnail as a PNG (read from the named archive member of the install).
pub fn thumbnail_png(install_root: &Path, thumb: &Thumb, max_edge: u32) -> Result<(u32, u32, Vec<u8>)> {
    if thumb.archive.contains(['/', '\\']) || !thumb.member.to_ascii_lowercase().contains("/creationzone/equipment/") {
        return Err(DomainError::Validation("not a Creation Zone equipment thumbnail".into()));
    }
    let archive = install_root.join("databigs").join(&thumb.archive);
    let entries = ps4_vault::eb_index(&archive)?;
    let bytes = ps4_vault::eb_member(&archive, &entries, &thumb.member)?;
    let tex = tdb_rx2::gnf::parse(&bytes).map_err(|e| DomainError::Extract(format!("{}: {e}", thumb.member)))?;
    let rgba = crate::assets::gnf_to_rgba(&tex)?;
    let img = crate::assets::DecodedImage { width: tex.width, height: tex.height, rgba }.thumbnail(max_edge);
    Ok((tex.width, tex.height, img.to_png_fast()?))
}

/// Every `exhibitionskaterequipment` row's class per family (the census research R136 §1 / §2 reports).
pub fn skater_census(db: &tdb_core::TdbFile, meta: &tdb_defs::FieldMetaIndex, cat: &SkaterGearCatalogue) -> Result<BTreeMap<(String, String), usize>> {
    let t = crate::team_reid::table_by_name(db, meta, "exhibitionskaterequipment")
        .ok_or_else(|| DomainError::Validation("no exhibitionskaterequipment table on this title".into()))?;
    let mut out = BTreeMap::new();
    for (fam, field, ..) in FAMILIES {
        let Some(f) = crate::view::descriptor_by_name(meta, t, field) else { continue };
        for r in 0..t.current_records as u32 {
            let Some(v) = db.read_u64(t, r, f) else { continue };
            if let Some(c) = cat.check(fam, v as i64) {
                *out.entry((fam.to_string(), c.class)).or_insert(0) += 1;
            }
        }
    }
    Ok(out)
}

impl crate::EditSession {
    /// Receipt 52: the skater-gear census of the working database.
    pub fn ps4_skater_gear_census(&self, cat: &SkaterGearCatalogue) -> Result<BTreeMap<(String, String), usize>> {
        let db = tdb_core::TdbFile::parse(self.bytes())?;
        skater_census(&db, self.project().meta(), cat)
    }
}
