//! Receipt 57 — PS4 NHL 17–21 Creation Zone **names** (research 2026-09-28 handoff 08, R141). The Creation Zone lists name
//! their items by `$` dictionary keys (`$bauer4500`, `$TXT_VISOR_AVIATOR`); each key is a string of the title's own English
//! dictionary `nhl/data/kettle/loc/eng_us.db`. About one key in five differs from the stored key only in letter case, so a
//! key is looked up as the game's key hash does: the `$` stripped, the exact key first, else the row whose **stored** hash
//! equals `tdb_loc::loc_hash(key)` (case-insensitive for ASCII). A key that resolves to nothing (the NHL 19+ placeholder
//! words `GoalieMaskPattern` / `GoalieMaskBrandAbbr`) shows nothing — never the placeholder.
//!
//! Identity first: the dictionary is read only from the archive recorded for the open project's title in
//! `defs/ps4/cz-dictionary.json`, and only when its bytes hash to the recorded value. Every string shown is the title's own
//! text; no brand name is invented. Read-only.
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::{ps4_vault, DomainError, Result};

#[derive(serde::Deserialize)]
struct DictIds {
    titles: BTreeMap<String, DictId>,
}
#[derive(serde::Deserialize)]
struct DictId {
    archive: String,
    member: String,
    sha256: String,
}

/// How a key was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Exact,
    /// Through the stored key hash (the key differs only in letter case, or the row's key string is not the cell's).
    Hash,
}

#[derive(Debug, Clone)]
pub struct CzNames {
    pub title: String,
    pub archive: String,
    pub sha256: String,
    dict: tdb_loc::LocDictionary,
    by_hash: HashMap<u32, usize>,
}

impl CzNames {
    /// The open project's title dictionary, identity-checked. `Ok(None)` for a title with no recorded dictionary
    /// (NHL 15 / 16, which ship no Creation Zone lists).
    pub fn load(install_root: &Path, defs_root: &Path, title: &str) -> Result<Option<Self>> {
        let ids: DictIds = serde_json::from_str(&std::fs::read_to_string(defs_root.join("ps4/cz-dictionary.json")).map_err(|e| DomainError::Validation(format!("dictionary identities: {e}")))?)
            .map_err(|e| DomainError::Validation(format!("dictionary identities: {e}")))?;
        let Some(want) = ids.titles.get(title) else { return Ok(None) };
        let archive = install_root.join("databigs").join(&want.archive);
        let entries = ps4_vault::eb_index(&archive)?;
        let bytes = ps4_vault::eb_member(&archive, &entries, &want.member)?;
        let sha = format!("{:x}", Sha256::digest(&bytes));
        if !sha.eq_ignore_ascii_case(&want.sha256) {
            return Err(DomainError::Validation(format!(
                "{} in {}: SHA-256 {}… is not the recorded {title} dictionary ({}…); names are not shown",
                want.member,
                want.archive,
                &sha[..12],
                &want.sha256[..12]
            )));
        }
        Ok(Some(Self::from_bytes(title, &want.archive, &sha, &bytes)?))
    }

    pub fn from_bytes(title: &str, archive: &str, sha256: &str, bytes: &[u8]) -> Result<Self> {
        let dict = tdb_loc::LocDictionary::parse(bytes).map_err(|e| DomainError::Validation(format!("{title} dictionary: {e}")))?;
        let mut by_hash = HashMap::new();
        for (i, r) in dict.rows.iter().enumerate() {
            by_hash.entry(r.hash).or_insert(i); // the first row with a stored hash, as the research probe reads it
        }
        Ok(CzNames { title: title.into(), archive: archive.into(), sha256: sha256.into(), dict, by_hash })
    }

    /// The title's text for a list cell (`$key`, or a bare key on the mask display list).
    pub fn resolve(&self, cell: &str) -> Option<(String, Resolution)> {
        let cell = cell.trim();
        let key = cell.strip_prefix('$').unwrap_or(cell);
        if key.is_empty() {
            return None;
        }
        if self.dict.contains_key(key) {
            return self.dict.text(key).map(|t| (t.to_string(), Resolution::Exact));
        }
        self.by_hash.get(&tdb_loc::loc_hash(key)).and_then(|&i| self.dict.rows[i].text.clone()).map(|t| (t, Resolution::Hash))
    }

    /// A mask row's display name (handoff 08 §2.2): a `%1s %2s` template is filled with the resolved style and pattern names
    /// only when both resolve; a named mask shows its own text. `None` = show the keys.
    pub fn mask_name(&self, name: &str, style: &str, pattern: &str) -> Option<String> {
        let (n, _) = self.resolve(name)?;
        if n.contains("%1s") || n.contains("%2s") {
            let (s, _) = self.resolve(style)?;
            let (p, _) = self.resolve(pattern)?;
            Some(n.replace("%1s", s.trim()).replace("%2s", p.trim()).trim().to_string())
        } else {
            Some(n.trim().to_string())
        }
    }
}

/// The Creation Zone lists and the columns R141 resolved (the mask display list's keys carry no `$`).
pub const NAME_COLUMNS: [(&str, &[&str]); 7] = [
    ("goaliemaskinfo.csv", &["name", "stylename", "patternname", "styleabbreviation"]),
    ("goaliemaskdisplayinfo.csv", &["displayname"]),
    ("skaterhelmetstyle.csv", &["itemname"]),
    ("skaterglovestyle.csv", &["itemname"]),
    ("skatedisplayinfo.csv", &["skatename"]),
    ("stickdisplayinfo.csv", &["stickname"]),
    ("skatervisortypes.csv", &["itemname"]),
];
pub const CZ_DIR: &str = "nhl/data/common/gamemodes/creation_zone_csv/";

/// Archive order as the research probe sorts it: `inxp*`, then `patch*`, then the rest, each by name.
fn archive_order(name: &str) -> (u8, String) {
    (if name.starts_with("inxp") { 0 } else if name.starts_with("patch") { 1 } else { 2 }, name.to_string())
}

/// Every name cell of the title's Creation Zone lists (the last archive copy of each list) resolved:
/// `(list, column, class) → cells`, class `exact` / `hash` / `unresolved`, and the unresolved keys. For the R141 comparison.
pub fn name_census(install_root: &Path, names: &CzNames) -> Result<(BTreeMap<(String, String, String), usize>, BTreeMap<String, usize>)> {
    let databigs = install_root.join("databigs");
    let mut bigs: Vec<String> = std::fs::read_dir(&databigs)
        .map_err(|e| DomainError::Validation(format!("{}: {e}", databigs.display())))?
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .filter(|n| n.ends_with(".big"))
        .collect();
    bigs.sort_by_key(|n| archive_order(n));
    let mut listings = Vec::new();
    for b in &bigs {
        let p = databigs.join(b);
        if let Ok(entries) = ps4_vault::eb_index(&p) {
            listings.push((p, entries));
        }
    }
    let mut counts = BTreeMap::new();
    let mut unresolved = BTreeMap::new();
    for (file, cols) in NAME_COLUMNS {
        let member = format!("{CZ_DIR}{file}");
        let Some((p, entries)) = listings.iter().rev().find(|(_, e)| e.iter().any(|x| x.path.eq_ignore_ascii_case(&member))) else { continue };
        let bytes = ps4_vault::eb_member(p, entries, &member)?;
        let table = crate::ps4_skater_gear::csv_table(&String::from_utf8_lossy(&bytes));
        let header = table.first().cloned().unwrap_or_default();
        for c in cols.iter() {
            let Some(ci) = header.iter().position(|h| h.eq_ignore_ascii_case(c)) else { continue };
            for row in table.iter().skip(1) {
                let v = row.get(ci).map(|s| s.trim()).unwrap_or("");
                if v.is_empty() || (file != "goaliemaskdisplayinfo.csv" && !v.starts_with('$')) {
                    continue;
                }
                let class = match names.resolve(v) {
                    Some((_, Resolution::Exact)) => "exact",
                    Some((_, Resolution::Hash)) => "hash",
                    None => {
                        *unresolved.entry(v.trim_start_matches('$').to_string()).or_default() += 1;
                        "unresolved"
                    }
                };
                *counts.entry((file.to_string(), c.to_string(), class.to_string())).or_default() += 1;
            }
        }
    }
    Ok((counts, unresolved))
}
