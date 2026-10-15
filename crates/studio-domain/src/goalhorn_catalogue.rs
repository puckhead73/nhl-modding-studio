//! Phase 2.7 (PS4) — the goal-horn **catalogue**, so an arena's `goalhorn` cell reads as a name
//! instead of a bare number.
//!
//! Research 2026-09-21 pm handoff R50 (`research-r50-ps4-goalhorn-catalogue`) measured the join on
//! NHL 17–21: the catalogue is `nhl/data/common/gamemodes/creation_zone_csv/goalhorns.csv`, and the
//! database's value resolves to a row by this rule (§2, "combined reading"):
//!
//! * `1 ≤ v ≤ 31` → the **premade club horn** of team index `v − 1`, i.e. the row whose `audioid`
//!   is `v − 1`. Measured: 30 / 30 NHL-club arenas name their own club on every one of the five
//!   titles;
//! * any other value → the row whose `audioid` is `v` (the generic horns, and the per-title
//!   default: 31 on NHL 17, 41 on 18+);
//! * `690`–`693` → a sentinel: **no row**, and Studio says so rather than inventing one.
//!
//! Two shipped gaps are carried as refusals, not papered over: the **New York Rangers** row is
//! absent from the catalogue on every title (their arena's `v = 20` resolves to nothing), and the
//! sentinel values resolve to nothing by design.
//!
//! **Not claimed:** which sound file a catalogue row plays (handoff R50 §3 leaves catalogue → bank
//! open), and anything about what the console does with an edited value. This module names a row;
//! it does not establish a runtime effect, and it is not a writer.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{DomainError, Result};

/// One catalogue row, as the CSV carries it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HornRow {
    pub index: i64,
    /// `AnaheimDucks`, `HornElement_12`, `None` …
    pub identifier: String,
    pub artid: i64,
    pub audioid: i64,
    /// A dictionary key (`$NHLTeamName_ANA`, `$TXT_HORNFLAIR_…`) — resolved for display by the
    /// caller when a dictionary is attached; shown as the key otherwise.
    pub name: String,
    /// `-1` where the title's CSV has no `tone` column (NHL 17 / 18).
    pub tone: i64,
    /// The row's class, from the columns the title carries.
    pub class: HornClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HornClass {
    /// The `None` rows.
    NoHorn,
    /// A premade club horn (`artid` = the club's art id).
    Team,
    /// A generic horn (`artid` 31).
    Generic,
    /// `HornElement_n` — a building block of a custom horn (NHL 19+).
    Element,
    /// `HornFlair_n` (NHL 19+).
    Flair,
}

impl HornClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoHorn => "none",
            Self::Team => "team",
            Self::Generic => "generic",
            Self::Element => "element",
            Self::Flair => "flair",
        }
    }
}

/// The parsed catalogue of one title.
#[derive(Debug, Clone, Default)]
pub struct GoalhornCatalogue {
    rows: Vec<HornRow>,
    by_audioid: BTreeMap<i64, usize>,
    pub source: String,
}

/// How a database value resolved against the catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HornResolution {
    /// The raw `goalhorn` / `periodhorn` cell.
    pub value: i64,
    /// The row it names, when one exists.
    pub row: Option<HornRow>,
    /// Which arm of the measured rule applied — shown behind disclosure so the resolution is
    /// auditable rather than magic.
    pub rule: String,
    /// Present when nothing resolved: the reason, in the user's words.
    pub refusal: Option<String>,
}

impl GoalhornCatalogue {
    /// The catalogue path under a PS4 install root.
    pub fn path_for(install_root: &Path) -> PathBuf {
        crate::assets::ps4_kettle_root(install_root)
            .parent()
            .map(|nhl_data| nhl_data.join("common").join("gamemodes").join("creation_zone_csv").join("goalhorns.csv"))
            .unwrap_or_else(|| install_root.join("goalhorns.csv"))
    }

    /// Read and parse the catalogue. A missing file is an error: an empty catalogue that silently
    /// resolved nothing would look like "this title has no horns".
    pub fn load(install_root: &Path) -> Result<Self> {
        let path = Self::path_for(install_root);
        let text = std::fs::read_to_string(&path).map_err(|e| DomainError::Validation(format!("{}: {e}", path.display())))?;
        Self::parse(&text, &path.display().to_string())
    }

    /// Parse the CSV text. The header names the columns; the titles differ (NHL 17 / 18 have no
    /// `ispremade` / `tone`), so every column is looked up by name and a missing one is a default,
    /// never a positional guess.
    pub fn parse(text: &str, source: &str) -> Result<Self> {
        let mut lines = text.lines().filter(|l| !l.trim().is_empty());
        let header = lines.next().ok_or_else(|| DomainError::Validation(format!("{source}: empty catalogue")))?;
        let cols: Vec<&str> = header.split(',').map(str::trim).collect();
        let col = |n: &str| cols.iter().position(|c| *c == n);
        let (Some(c_index), Some(c_ident), Some(c_artid), Some(c_audio), Some(c_name)) =
            (col("index"), col("identifier"), col("artid"), col("audioid"), col("name"))
        else {
            return Err(DomainError::Validation(format!("{source}: the catalogue header lacks one of index / identifier / artid / audioid / name (found: {header})")));
        };
        let c_tone = col("tone");
        let mut rows = Vec::new();
        let mut by_audioid = BTreeMap::new();
        for (n, line) in lines.enumerate() {
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            let get = |i: usize| f.get(i).copied().unwrap_or("");
            let num = |i: usize| -> i64 { get(i).parse().unwrap_or(0) };
            let identifier = get(c_ident).to_string();
            let artid = num(c_artid);
            let audioid = num(c_audio);
            let class = if identifier.eq_ignore_ascii_case("none") {
                HornClass::NoHorn
            } else if identifier.starts_with("HornElement") {
                HornClass::Element
            } else if identifier.starts_with("HornFlair") {
                HornClass::Flair
            } else if artid == 31 {
                HornClass::Generic
            } else {
                HornClass::Team
            };
            let row = HornRow {
                index: num(c_index),
                identifier,
                artid,
                audioid,
                name: get(c_name).to_string(),
                tone: c_tone.map(num).unwrap_or(-1),
                class,
            };
            // The first row wins a duplicated `audioid` (the `None` rows repeat −1); recording the
            // first keeps the lookup deterministic.
            by_audioid.entry(row.audioid).or_insert(n);
            rows.push(row);
        }
        if rows.is_empty() {
            return Err(DomainError::Validation(format!("{source}: the catalogue has a header but no rows")));
        }
        Ok(Self { rows, by_audioid, source: source.to_string() })
    }

    pub fn rows(&self) -> &[HornRow] {
        &self.rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// How many rows of each class — the card's summary line.
    pub fn class_counts(&self) -> Vec<(&'static str, usize)> {
        let mut out = Vec::new();
        for c in [HornClass::NoHorn, HornClass::Team, HornClass::Generic, HornClass::Element, HornClass::Flair] {
            let n = self.rows.iter().filter(|r| r.class == c).count();
            if n > 0 {
                out.push((c.as_str(), n));
            }
        }
        out
    }

    fn by_audio(&self, audioid: i64) -> Option<&HornRow> {
        self.by_audioid.get(&audioid).and_then(|i| self.rows.get(*i))
    }

    /// Resolve an arena's `goalhorn` value by the measured rule (R50 §2).
    pub fn resolve(&self, value: i64) -> HornResolution {
        // The sentinel band: measured as "no row" on every title.
        if (690..=693).contains(&value) {
            return HornResolution {
                value,
                row: None,
                rule: "sentinel 690–693".into(),
                refusal: Some(format!("{value} is the \"no horn selected\" sentinel — it names no catalogue row")),
            };
        }
        if (1..=31).contains(&value) {
            if let Some(r) = self.by_audio(value - 1).filter(|r| r.class == HornClass::Team) {
                return HornResolution { value, row: Some(r.clone()), rule: format!("premade club horn: audioid {} = value − 1", value - 1), refusal: None };
            }
            // The Rangers row is absent from every shipped catalogue (R50 §2).
            if value == 20 {
                return HornResolution {
                    value,
                    row: None,
                    rule: "premade club horn: audioid 19 = value − 1".into(),
                    refusal: Some("the New York Rangers row is absent from the shipped catalogue on every measured title, so this club horn has no entry to name".into()),
                };
            }
        }
        match self.by_audio(value) {
            Some(r) => HornResolution { value, row: Some(r.clone()), rule: format!("audioid {value}"), refusal: None },
            None => HornResolution {
                value,
                row: None,
                rule: format!("audioid {value}"),
                refusal: Some(format!("no catalogue row carries audioid {value} on this title")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A miniature catalogue in the NHL 19+ shape.
    const CSV: &str = "index,identifier,artid,audioid,name,ispremade,ishornelement,isflare,tone\n\
        -1,None,30,-1,$None,2,0,0,-1\n\
        0,AnaheimDucks,0,0,$NHLTeamName_ANA,1,0,0,-1\n\
        1,ArizonaCoyotes,22,22,$NHLTeamName_PHX,1,0,0,-1\n\
        40,GenericHorn1,31,41,$TXT_HORN_ONE,0,0,0,-1\n\
        100,HornElement_1,31,501,$TXT_HORNFLAIR_A,0,1,0,1\n\
        200,HornFlair_1,31,600,$TXT_HORNFLAIR_B,0,0,1,2\n";

    #[test]
    fn the_measured_rule_resolves_clubs_generics_sentinels_and_the_missing_rangers_row() {
        let c = GoalhornCatalogue::parse(CSV, "test").unwrap();
        assert_eq!(c.len(), 6);
        assert_eq!(c.class_counts(), vec![("none", 1), ("team", 2), ("generic", 1), ("element", 1), ("flair", 1)]);

        // 1 ≤ v ≤ 31 → the team row whose audioid is v − 1.
        let anaheim = c.resolve(1);
        assert_eq!(anaheim.row.as_ref().unwrap().identifier, "AnaheimDucks");
        assert!(anaheim.rule.contains("value − 1") && anaheim.refusal.is_none());

        // A generic value resolves by audioid directly.
        let generic = c.resolve(41);
        assert_eq!(generic.row.as_ref().unwrap().identifier, "GenericHorn1");
        assert_eq!(generic.rule, "audioid 41");

        // The sentinel band names nothing, and says so.
        for v in [690, 691, 693] {
            let s = c.resolve(v);
            assert!(s.row.is_none() && s.refusal.as_deref().is_some_and(|r| r.contains("sentinel")), "{v}: {s:?}");
        }

        // The shipped Rangers gap is a named refusal, not a wrong row.
        let rangers = c.resolve(20);
        assert!(rangers.row.is_none());
        assert!(rangers.refusal.as_deref().is_some_and(|r| r.contains("New York Rangers")), "{rangers:?}");

        // An id no row carries refuses rather than guessing.
        let missing = c.resolve(999);
        assert!(missing.row.is_none() && missing.refusal.as_deref().is_some_and(|r| r.contains("999")));

        // Elements and flairs are classified, and reachable by their own audioid.
        assert_eq!(c.resolve(501).row.unwrap().class, HornClass::Element);
        assert_eq!(c.resolve(600).row.unwrap().class, HornClass::Flair);
    }

    /// The NHL 17 / 18 shape has neither `ispremade` nor `tone`; columns are found by name and the
    /// missing ones default rather than shifting the parse.
    #[test]
    fn the_earlier_column_set_parses_without_positional_guessing() {
        let csv = "index,identifier,artid,audioid,name\n0,AnaheimDucks,0,0,$NHLTeamName_ANA\n31,Gen,31,31,$TXT_G\n";
        let c = GoalhornCatalogue::parse(csv, "test").unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c.resolve(1).row.unwrap().identifier, "AnaheimDucks");
        assert_eq!(c.rows()[0].tone, -1, "a title without a tone column reports -1, not a shifted field");
        assert_eq!(c.resolve(31).row.unwrap().identifier, "Gen", "NHL 17's default resolves by audioid");
    }

    #[test]
    fn a_catalogue_without_the_measured_columns_is_refused() {
        assert!(GoalhornCatalogue::parse("a,b,c\n1,2,3\n", "test").is_err());
        assert!(GoalhornCatalogue::parse("", "test").is_err());
        assert!(GoalhornCatalogue::parse("index,identifier,artid,audioid,name\n", "test").is_err());
    }
}
