//! Receipt 48 — the shipped groups of a league schedule table, offered only by exact identity (research R113 /
//! R113b, consumed under R123 Q9 / Q10, accepted 28 Sep 2026 for single-phase generation).
//!
//! `defs/schedules/stock-partitions.json` lists complete single-phase stock partitions, each tied to a database
//! SHA-256, a table and the R113 mapping fingerprint (SHA-256 of the canonical JSON `[[rank, index, artabbr], …]`,
//! rank = the league slot). Studio returns a table's groups only when the file it opened, the table and the
//! current membership all equal a row: anything else fails closed and the user enters groups instead. Groups are
//! never guessed from team order, a neighbouring title or a similar schema. The Austrian two-phase table is not
//! listed (Q10) and stays refused by the tagged-row rule.
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{DomainError, Result};
use crate::schedules::SlotMap;

#[derive(Debug, Clone, Deserialize)]
pub struct StockQuotas {
    pub within: u32,
    pub across: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StockContract {
    pub source: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub db_sha256: String,
    pub table: String,
    pub quotas: StockQuotas,
    pub n_members: usize,
    pub games: u64,
    /// Groups as team `index` values (the key the slot map is ordered by).
    pub groups_by_index: Vec<Vec<u64>>,
    pub mapping_fingerprint: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StockPartitionDefs {
    #[serde(default)]
    pub contracts: Vec<StockContract>,
}

impl StockPartitionDefs {
    /// Load `defs/schedules/stock-partitions.json`; a missing file is an empty set (nothing offered).
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.is_file() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path).map_err(|source| DomainError::Io { path: path.display().to_string(), source })?;
        serde_json::from_str(&text).map_err(|e| DomainError::Validation(format!("{}: {e}", path.display())))
    }
}

/// R113's mapping fingerprint of a league's current membership.
pub fn r113_fingerprint(slots: &SlotMap) -> String {
    let rows: Vec<(u64, u64, &str)> = slots.entries.iter().map(|e| (e.slot, e.global_index, e.art_abbrev.as_str())).collect();
    let json = serde_json::to_string(&rows).expect("plain values serialise");
    format!("{:x}", Sha256::digest(json.as_bytes()))
}

/// The shipped groups, as team ids, with their quotas and where they come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockGroups {
    pub groups: Vec<Vec<u64>>,
    pub within: u32,
    pub across: u32,
    pub source: String,
    pub games: u64,
}

/// The groups for `table` when the opened file (`loaded_sha256`), the table and the membership all match a
/// contract exactly; otherwise the reason, in plain words.
pub fn stock_groups(defs: &StockPartitionDefs, table: &str, loaded_sha256: Option<&str>, slots: &SlotMap) -> std::result::Result<StockGroups, String> {
    let for_table: Vec<&StockContract> = defs.contracts.iter().filter(|c| c.table == table).collect();
    if for_table.is_empty() {
        return Err(format!("no shipped single-phase groups are recorded for {table}; enter the groups and game counts yourself"));
    }
    let Some(sha) = loaded_sha256 else {
        return Err("this project was not opened from a file, so its source cannot be matched to a recorded shipped table; enter the groups yourself".into());
    };
    let Some(c) = for_table.iter().find(|c| c.db_sha256.eq_ignore_ascii_case(sha)) else {
        return Err(format!(
            "the opened file is not one of the recorded sources for {table} (it may be edited, saved, patched or another title); Studio does not reuse another source's groups. Enter the groups yourself"
        ));
    };
    let fp = r113_fingerprint(slots);
    if fp != c.mapping_fingerprint {
        return Err(format!(
            "the league's membership or order differs from the recorded {} table (mapping fingerprint {} ≠ {}); enter the groups yourself",
            c.source,
            &fp[..12],
            &c.mapping_fingerprint[..12.min(c.mapping_fingerprint.len())]
        ));
    }
    if slots.entries.len() != c.n_members {
        return Err(format!("the league has {} members, the recorded table {}", slots.entries.len(), c.n_members));
    }
    let mut groups = Vec::new();
    for g in &c.groups_by_index {
        let mut ids = Vec::new();
        for idx in g {
            match slots.entries.iter().find(|e| e.global_index == *idx) {
                Some(e) => ids.push(e.team_id),
                None => return Err(format!("recorded group member index {idx} is not in the league now")),
            }
        }
        groups.push(ids);
    }
    Ok(StockGroups { groups, within: c.quotas.within, across: c.quotas.across, source: c.source.clone(), games: c.games })
}
