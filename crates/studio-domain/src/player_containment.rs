//! Phase 3 (Players, 22 Sep 2026) — **portrait / head containment**: which of the roster's art ids
//! and head ids the attached install actually ships, the `hasportrait` flag shown as the database
//! flag it is (never derived from a file, never written from one), and the shipped-but-unreferenced
//! pools a reassignment could draw from.
//!
//! ## Gen7 (NHL 09 → Legacy, Xbox 360 / PS3 / Recompiled) — research 2026-09-21 pm handoff 11 (R53)
//!
//! ```text
//! artid   → fe/ion/artassets/playerheads/p<bucket>/p<artid>.big     (buckets 0–4 000 / 4 001–8 000 / 8 001–12 000)
//! headid  → rendering/player/player_<headid>.rx2                    (NHL 10 → Legacy; `head_<headid>` on 09 / 10; `.rpsgl` on PS3)
//! ```
//!
//! A player is **bespoke** when `headid == artid` (R53's definition; 246–270 per title), otherwise
//! in the **pool** (10 000+ on 09–13, 60 000+ on 14+; the floor is measured per file, receipt 06).
//! On NHL 10 → Legacy every bespoke head ships; NHL 09 lacks five bespoke heads and five portraits
//! ("shipped without asset"). The `hasportrait` flag contradicts the files on 13 / Legacy (93 / 127
//! players flagged 0 with a file; 2 Legacy players flagged 1 without one), so it is displayed as
//! data and cross-tabulated, never reconciled by Studio.
//!
//! ## PS4 (NHL 15 → 21) — handoff 20 (R63)
//!
//! `artid` → `playerportraits/p<artid>.gnf` (`assets/portraits/p<artid>.png` on NHL 15). No row is
//! ever flagged 1 without a file on any PS4 title; the reverse direction is a surplus. Heads on PS4
//! are name-keyed vault data and are **not** measured here.
//!
//! Nothing here writes; the flag is never changed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::project::Platform;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ContainmentContract {
    /// Gen7 file-name rules (R53). `head_prefixes` lists the stems the title uses.
    Gen7 { extension: String, head_prefixes: Vec<String>, source: String },
    /// PS4 portrait rule (R63); heads not measured.
    Ps4 { source: String },
    /// Nothing measured (NHL 07 / 08, unknown titles).
    Unmeasured { reason: String },
}

impl ContainmentContract {
    pub fn describe(&self) -> String {
        match self {
            Self::Gen7 { extension, head_prefixes, source } => format!(
                "portrait `fe/ion/artassets/playerheads/p<bucket>/p<artid>.big`; head `rendering/player/{}<headid>.{extension}`; bespoke = head id equals art id; pool floor measured per file ({source})",
                head_prefixes.join("|")
            ),
            Self::Ps4 { source } => format!("portrait `playerportraits/p<artid>.gnf` (NHL 15: `assets/portraits/p<artid>.png`); heads are name-keyed vault data, not measured here ({source})"),
            Self::Unmeasured { reason } => reason.clone(),
        }
    }
}

/// The contract for a title / platform, decided from identity alone.
pub fn containment_contract(game_id: Option<&str>, platform: Platform) -> ContainmentContract {
    if platform == Platform::Ps4 {
        return ContainmentContract::Ps4 { source: "research 2026-09-21 pm handoff R63: 0 rows flagged without a file on all seven titles".into() };
    }
    let extension = match platform {
        Platform::Ps3 => "rpsgl",
        _ => "rx2",
    };
    let source = "research 2026-09-21 pm handoff R53 §1–2 (NHL 09 → Legacy, Xbox 360; PS3 15 / Legacy identical)";
    match game_id {
        Some("nhl09") => ContainmentContract::Gen7 { extension: extension.into(), head_prefixes: vec!["head_".into()], source: source.into() },
        Some("nhl10") => ContainmentContract::Gen7 { extension: extension.into(), head_prefixes: vec!["player_".into(), "head_".into()], source: source.into() },
        Some("nhl11" | "nhl12" | "nhl13" | "nhl14" | "nhl15" | "nhllegacy") => ContainmentContract::Gen7 { extension: extension.into(), head_prefixes: vec!["player_".into()], source: source.into() },
        // Receipt 55: Legacy Recompiled keeps the X360 Legacy files as byte-identical archive members (research R128 / 2026-09-28
        // handoff 04, R138); its own archives are what a check reads.
        Some("legacyrecomp") => ContainmentContract::Gen7 {
            extension: "rx2".into(),
            head_prefixes: vec!["player_".into()],
            source: "research R128 / R138 / R146: Recompiled archive members byte-identical to the Xbox 360 Legacy tree (R53 rules); with the install attached, the card checks its own archive members".into(),
        },
        Some(other) => ContainmentContract::Unmeasured { reason: format!("the portrait / head containment of `{other}` is not measured (research R53 covers NHL 09 → Legacy, R63 the PS4 titles); nothing is joined") },
        None => ContainmentContract::Unmeasured { reason: "the open project has no recorded title, so the containment rules cannot be chosen; nothing is joined".into() },
    }
}

/// One player's containment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerContainment {
    pub bio_record: u32,
    pub full_name: String,
    pub portrait_id: u64,
    pub head_id: u64,
    /// `hasportrait` as stored; `None` when the title's bio table has no such column.
    pub has_portrait_flag: Option<bool>,
    /// `bespoke` (head id equals art id, R53), `pool` (in the measured pool range), `other`.
    pub class: String,
    /// `present` / `missing` / `unknown` (no tree, or not measured on this platform).
    pub portrait: crate::kit_files::ComponentStatus,
    pub head: crate::kit_files::ComponentStatus,
    /// A plain-language note when the row and the files disagree, or a bespoke asset is missing.
    pub note: Option<String>,
}

/// The title-level cross-tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainmentSummary {
    pub contract: ContainmentContract,
    pub players: usize,
    pub bespoke: usize,
    pub bespoke_head_present: usize,
    pub bespoke_portrait_present: usize,
    pub pool_players: usize,
    pub pool_head_present: usize,
    pub pool_floor: Option<u64>,
    /// `hasportrait` × portrait file: flagged with file, flagged without, clear with file, clear without.
    pub flag1_file1: usize,
    pub flag1_file0: usize,
    pub flag0_file1: usize,
    pub flag0_file0: usize,
    pub has_flag_column: bool,
    /// Shipped portrait / head ids and the ones no roster row references.
    pub shipped_portraits: Option<usize>,
    pub shipped_heads: Option<usize>,
    pub unused_portraits: Option<usize>,
    pub unused_heads: Option<usize>,
    /// Bounded name lists (first 16) for the residuals the research lists.
    pub bespoke_missing_head: Vec<String>,
    pub bespoke_missing_portrait: Vec<String>,
    pub flagged_without_file: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainmentView {
    pub summary: ContainmentSummary,
    pub players: Vec<PlayerContainment>,
}

// ---------------------------------------------------------------------------

/// Ids named by `<prefix><id><suffix>` files in a directory (recursing one level into the portrait
/// buckets). `None` when the directory could not be listed.
fn ids_in_dir(dir: &Path, prefix: &str, exts: &[&str], recurse: bool) -> Option<BTreeSet<u64>> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut out = BTreeSet::new();
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            // The portrait buckets are `p0_4000` / `p4001_8000` / `p8001_12000`; the sibling
            // `silhouettes/` holds the pool heads' stand-ins (R53) and is not a portrait bucket.
            let bucket = e.file_name().to_string_lossy().starts_with('p');
            if recurse && bucket {
                if let Some(sub) = ids_in_dir(&p, prefix, exts, false) {
                    out.extend(sub);
                }
            }
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        let Some((stem, ext)) = name.rsplit_once('.') else { continue };
        if !exts.contains(&ext) {
            continue;
        }
        if let Some(num) = stem.strip_prefix(prefix) {
            if !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()) && !(num.len() > 1 && num.starts_with('0')) {
                if let Ok(id) = num.parse::<u64>() {
                    out.insert(id);
                }
            }
        }
    }
    Some(out)
}

/// The database side: every bio row's `(record, name, artid, headid, hasportrait)`.
pub struct BioRow {
    pub record: u32,
    pub full_name: String,
    pub portrait_id: u64,
    pub head_id: u64,
    pub has_portrait: Option<bool>,
}

pub fn bio_rows(db: &tdb_core::TdbFile, meta: &tdb_defs::FieldMetaIndex) -> Option<(Vec<BioRow>, bool)> {
    use crate::view::{read_str_by_name, read_u64_by_studio};
    let bio = db.table_canonical(tdb_relations::entity::ids::BIO)?;
    let flag = bio.fields.iter().find(|f| meta.field(&bio.id.raw(), &f.id.raw()).map(|m| m.name == "hasportrait").unwrap_or(false));
    let mut rows = Vec::with_capacity(bio.current_records as usize);
    for rec in 0..bio.current_records as u32 {
        let first = read_str_by_name(db, meta, bio, rec, "firstname");
        let last = read_str_by_name(db, meta, bio, rec, "lastname");
        let full_name = match (first.trim(), last.trim()) {
            ("", l) => l.to_string(),
            (f, "") => f.to_string(),
            (f, l) => format!("{f} {l}"),
        };
        rows.push(BioRow {
            record: rec,
            full_name,
            portrait_id: read_u64_by_studio(db, bio, rec, "lOnr").unwrap_or(0),
            head_id: read_u64_by_studio(db, bio, rec, "pPaD").unwrap_or(0),
            has_portrait: flag.and_then(|f| db.read_u64(bio, rec, f)).map(|v| v == 1),
        });
    }
    Some((rows, flag.is_some()))
}

/// Ids named by `<dir>/[<bucket>/]<prefix><id>.<ext>` members of an archive index (the member-path form of [`ids_in_dir`]).
fn ids_in_index(idx: &crate::archive_index::ArchiveIndex, dir: &str, prefix: &str, exts: &[&str], buckets: bool) -> BTreeSet<u64> {
    let mut out = BTreeSet::new();
    for path in idx.members.keys() {
        let Some(rest) = path.strip_prefix(dir) else { continue };
        let leaf = match rest.split_once('/') {
            None => rest,
            // One bucket level (`p0_4000/`), never `silhouettes/` or deeper folders.
            Some((b, leaf)) if buckets && b.starts_with('p') && !leaf.contains('/') => leaf,
            Some(_) => continue,
        };
        let Some((stem, ext)) = leaf.rsplit_once('.') else { continue };
        if !exts.contains(&ext) {
            continue;
        }
        if let Some(num) = stem.strip_prefix(prefix) {
            if !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()) && !(num.len() > 1 && num.starts_with('0')) {
                if let Ok(id) = num.parse::<u64>() {
                    out.insert(id);
                }
            }
        }
    }
    out
}

/// Where a shipped-id set came from (receipt 67).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Source {
    Loose,
    /// The install's archive members; `complete` when every archive was listed (a missing id is then missing, not unknown).
    Archives { complete: bool },
}

/// Build the containment view. `fe_root` is the front-end root (`<anchor>/fe` on gen7, `<nhl>/ui`
/// on PS4); `rendering_root` the rendering tree.
pub fn player_containment(rows: &[BioRow], has_flag_column: bool, contract: ContainmentContract, fe_root: Option<&Path>, rendering_root: Option<&Path>) -> ContainmentView {
    player_containment_with_archives(rows, has_flag_column, contract, fe_root, rendering_root, None)
}

/// [`player_containment`] with the install's archive index (receipt 67; research 2026-09-28 handoffs 12 §2 / 17 §2): on gen7, a
/// loose directory that is absent falls back to the archive members of the same relative path. A readable, complete index
/// without the member reads **missing**; one with unreadable archives reads **unknown** for ids it does not hold; no index,
/// or an index with no archive, keeps the loose result (unknown when the directory is absent).
pub fn player_containment_with_archives(
    rows: &[BioRow],
    has_flag_column: bool,
    contract: ContainmentContract,
    fe_root: Option<&Path>,
    rendering_root: Option<&Path>,
    archives: Option<&crate::archive_index::ArchiveIndex>,
) -> ContainmentView {
    use crate::kit_files::ComponentStatus as S;
    let mut notes = Vec::new();
    let archives = archives.filter(|i| !i.archives.is_empty());
    let archive_source = archives.map(|i| Source::Archives { complete: i.unreadable.is_empty() });
    let (mut portrait_src, mut head_src) = (Source::Loose, Source::Loose);

    // Shipped id sets, by contract.
    let (portraits, heads): (Option<BTreeSet<u64>>, Option<BTreeSet<u64>>) = match (&contract, fe_root, rendering_root) {
        (ContainmentContract::Gen7 { extension, head_prefixes, .. }, fe, rendering) => {
            let exts = [extension.as_str(), "rx2", "rpsgl"];
            let mut portraits = fe.and_then(|fe| ids_in_dir(&fe.join("ion").join("artassets").join("playerheads"), "p", &["big"], true));
            let mut heads = rendering.and_then(|r| {
                let dir = r.join("player");
                let mut all = BTreeSet::new();
                let mut any = false;
                for prefix in head_prefixes {
                    if let Some(s) = ids_in_dir(&dir, prefix, &exts, false) {
                        any = true;
                        all.extend(s);
                    }
                }
                any.then_some(all)
            });
            if let (Some(idx), Some(src)) = (archives, archive_source) {
                if portraits.is_none() {
                    portraits = Some(ids_in_index(idx, "fe/ion/artassets/playerheads/", "p", &["big"], true));
                    portrait_src = src;
                }
                if heads.is_none() {
                    let mut all = BTreeSet::new();
                    for prefix in head_prefixes {
                        all.extend(ids_in_index(idx, "rendering/player/", prefix, &exts, false));
                    }
                    heads = Some(all);
                    head_src = src;
                }
            }
            (portraits, heads)
        }
        (ContainmentContract::Ps4 { .. }, fe, _) => {
            let portraits = fe.and_then(|fe| {
                let gnf = ids_in_dir(&fe.join("kettle_assets").join("playerportraits"), "p", &["gnf"], false);
                let png = ids_in_dir(&fe.join("assets").join("portraits"), "p", &["png"], false);
                match (gnf, png) {
                    (None, None) => None,
                    (a, b) => Some(a.unwrap_or_default().union(&b.unwrap_or_default()).copied().collect()),
                }
            });
            (portraits, None)
        }
        (ContainmentContract::Unmeasured { reason }, _, _) => {
            notes.push(reason.clone());
            (None, None)
        }
    };
    if let (Some(idx), true) = (archives, portrait_src != Source::Loose || head_src != Source::Loose) {
        let which = match (portrait_src != Source::Loose, head_src != Source::Loose) {
            (true, true) => "portraits and heads",
            (true, false) => "portraits",
            _ => "heads",
        };
        let listed: usize = idx.archives.iter().map(|(_, n)| n).sum();
        notes.push(format!(
            "{which} were checked against this install's own archive members (no loose tree): {} archive(s) listed, {listed} member paths; only directories were read, so a member is shipped by path, its bytes are not checked",
            idx.archives.len()
        ));
        if !idx.unreadable.is_empty() {
            notes.push(format!(
                "{} archive(s) could not be listed ({}): an id without a member is unknown, not proven absent",
                idx.unreadable.len(),
                idx.unreadable.iter().map(|(a, _)| a.as_str()).take(4).collect::<Vec<_>>().join(", ")
            ));
        }
    }
    if matches!(contract, ContainmentContract::Gen7 { .. } | ContainmentContract::Ps4 { .. }) {
        if portrait_src != Source::Loose {
        } else if fe_root.is_none() {
            notes.push("no front-end tree is attached, so portraits were not looked for".into());
        } else if portraits.is_none() {
            notes.push("the portrait directory could not be listed under the attached front-end tree: portraits are unknown, not proven absent".into());
        }
        if matches!(contract, ContainmentContract::Gen7 { .. }) {
            if head_src != Source::Loose {
            } else if rendering_root.is_none() {
                notes.push("no rendering tree is attached, so head files were not looked for".into());
            } else if heads.is_none() {
                notes.push("the `rendering/player` directory could not be listed: heads are unknown, not proven absent".into());
            }
        } else {
            notes.push("PS4 heads are name-keyed vault data (research 2026-09-20 handoff 08) and are not measured by this card".into());
        }
    }
    if !has_flag_column {
        notes.push("this title's bio table carries no `hasportrait` column".into());
    }

    // The pool floor, measured per file exactly as `scan_player_assets` does (receipt 06).
    let pool_floor = match &contract {
        ContainmentContract::Gen7 { .. } | ContainmentContract::Ps4 { .. } => {
            Some(if rows.iter().any(|r| r.head_id >= crate::assets::POOL_HEAD_MIN) { crate::assets::POOL_HEAD_MIN } else { crate::assets::POOL_HEAD_MIN_EARLY_GEN7 })
        }
        ContainmentContract::Unmeasured { .. } => None,
    };
    let status = |set: &Option<BTreeSet<u64>>, src: Source, id: u64| match (set, src) {
        (Some(s), _) if s.contains(&id) => S::Present,
        (Some(_), Source::Loose | Source::Archives { complete: true }) => S::Missing,
        (Some(_), Source::Archives { complete: false }) | (None, _) => S::Unknown,
    };

    let mut summary = ContainmentSummary {
        contract: contract.clone(),
        players: rows.len(),
        bespoke: 0,
        bespoke_head_present: 0,
        bespoke_portrait_present: 0,
        pool_players: 0,
        pool_head_present: 0,
        pool_floor,
        flag1_file1: 0,
        flag1_file0: 0,
        flag0_file1: 0,
        flag0_file0: 0,
        has_flag_column,
        shipped_portraits: portraits.as_ref().map(BTreeSet::len),
        shipped_heads: heads.as_ref().map(BTreeSet::len),
        unused_portraits: None,
        unused_heads: None,
        bespoke_missing_head: Vec::new(),
        bespoke_missing_portrait: Vec::new(),
        flagged_without_file: Vec::new(),
        notes: Vec::new(),
    };
    let mut players = Vec::with_capacity(rows.len());
    let mut used_portraits = BTreeSet::new();
    let mut used_heads = BTreeSet::new();
    for r in rows {
        used_portraits.insert(r.portrait_id);
        used_heads.insert(r.head_id);
        let measured = !matches!(contract, ContainmentContract::Unmeasured { .. });
        let class = if !measured {
            "other"
        } else if r.head_id != 0 && r.head_id == r.portrait_id {
            "bespoke"
        } else if pool_floor.is_some_and(|f| r.head_id >= f) {
            "pool"
        } else {
            "other"
        };
        let portrait = if measured { status(&portraits, portrait_src, r.portrait_id) } else { S::Unknown };
        let head = if matches!(contract, ContainmentContract::Gen7 { .. }) { status(&heads, head_src, r.head_id) } else { S::Unknown };
        let mut note = None;
        match class {
            "bespoke" => {
                summary.bespoke += 1;
                if head == S::Present {
                    summary.bespoke_head_present += 1;
                } else if head == S::Missing {
                    if summary.bespoke_missing_head.len() < 16 {
                        summary.bespoke_missing_head.push(r.full_name.clone());
                    }
                    note = Some(format!("shipped without asset: this player's own head (id {}) has no file in the rendering tree", r.head_id));
                }
                if portrait == S::Present {
                    summary.bespoke_portrait_present += 1;
                } else if portrait == S::Missing {
                    if summary.bespoke_missing_portrait.len() < 16 {
                        summary.bespoke_missing_portrait.push(r.full_name.clone());
                    }
                    note = Some(match note {
                        Some(n) => format!("{n}; and no portrait file ships for art id {}", r.portrait_id),
                        None => format!("shipped without asset: no portrait file ships for this bespoke player's art id {}", r.portrait_id),
                    });
                }
            }
            "pool" => {
                summary.pool_players += 1;
                if head == S::Present {
                    summary.pool_head_present += 1;
                }
            }
            _ => {}
        }
        if let Some(flag) = r.has_portrait {
            match (flag, portrait) {
                (true, S::Present) => summary.flag1_file1 += 1,
                (true, S::Missing) => {
                    summary.flag1_file0 += 1;
                    if summary.flagged_without_file.len() < 16 {
                        summary.flagged_without_file.push(r.full_name.clone());
                    }
                    note.get_or_insert_with(|| format!("`hasportrait` is set but no portrait file ships for art id {} — the flag is shown as stored, not corrected", r.portrait_id));
                }
                (false, S::Present) => {
                    summary.flag0_file1 += 1;
                    note.get_or_insert_with(|| "a portrait file ships for this art id but `hasportrait` is clear — shown as stored; what the game displays is runtime".to_string());
                }
                (false, S::Missing) => summary.flag0_file0 += 1,
                (_, S::Unknown) => {}
            }
        }
        players.push(PlayerContainment {
            bio_record: r.record,
            full_name: r.full_name.clone(),
            portrait_id: r.portrait_id,
            head_id: r.head_id,
            has_portrait_flag: r.has_portrait,
            class: class.to_string(),
            portrait,
            head,
            note,
        });
    }
    // An incomplete archive index cannot say what is unreferenced.
    let complete = |src: Source| src != Source::Archives { complete: false };
    summary.unused_portraits = portraits.as_ref().filter(|_| complete(portrait_src)).map(|p| p.difference(&used_portraits).count());
    summary.unused_heads = heads.as_ref().filter(|_| complete(head_src)).map(|h| h.difference(&used_heads).count());
    if matches!(contract, ContainmentContract::Gen7 { .. } | ContainmentContract::Ps4 { .. }) {
        summary.notes.push(format!(
            "{} players: {} bespoke (head id = art id; {} with a head file, {} with a portrait file), {} pool (floor {}; {} with a head file)",
            summary.players,
            summary.bespoke,
            summary.bespoke_head_present,
            summary.bespoke_portrait_present,
            summary.pool_players,
            pool_floor.map(|f| f.to_string()).unwrap_or_else(|| "—".into()),
            summary.pool_head_present
        ));
        if has_flag_column {
            summary.notes.push(format!(
                "`hasportrait` vs the shipped file: {} flagged with a file, {} flagged without, {} clear with a file, {} clear without — the flag is data, not derived from the files and never written from them",
                summary.flag1_file1, summary.flag1_file0, summary.flag0_file1, summary.flag0_file0
            ));
        }
        if let (Some(sp), Some(up)) = (summary.shipped_portraits, summary.unused_portraits) {
            summary.notes.push(format!("{sp} shipped portrait files, {up} referenced by no roster row (the reassignment pool)"));
        }
        if let (Some(sh), Some(uh)) = (summary.shipped_heads, summary.unused_heads) {
            summary.notes.push(format!("{sh} shipped head files, {uh} referenced by no roster row"));
        }
    }
    summary.notes.extend(notes);
    summary.notes.push("read-only: no id, flag or file is changed here; a reassignment needs its own write contract validated against the unused shipped ids".into());
    ContainmentView { summary, players }
}

/// Bounded map of `bio_record → note` for the callers that want only the exceptions.
pub fn notes_by_record(view: &ContainmentView) -> BTreeMap<u32, String> {
    view.players.iter().filter_map(|p| p.note.clone().map(|n| (p.bio_record, n))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kit_files::ComponentStatus as S;
    use std::path::PathBuf;

    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Self {
            let d = std::env::temp_dir().join(format!("nhlms-contain-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            Tmp(d)
        }
        fn put(&self, rel: &str) {
            let p = self.0.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn row(rec: u32, name: &str, art: u64, head: u64, flag: Option<bool>) -> BioRow {
        BioRow { record: rec, full_name: name.into(), portrait_id: art, head_id: head, has_portrait: flag }
    }

    #[test]
    fn gen7_cross_tabulates_the_flag_against_the_files_and_names_the_shipped_without_asset_cases() {
        let d = Tmp::new("gen7");
        d.put("fe/ion/artassets/playerheads/p0_4000/p12.big");
        d.put("fe/ion/artassets/playerheads/p0_4000/p13.big");
        d.put("fe/ion/artassets/playerheads/p4001_8000/p5000.big"); // unused
        d.put("rendering/player/player_12.rx2");
        d.put("rendering/player/player_60000.rx2");
        d.put("rendering/player/player_60001.rx2"); // unused
        let rows = [
            row(0, "Star One", 12, 12, Some(true)),      // bespoke, both present, flag agrees
            row(1, "Star Two", 13, 13, Some(false)),     // bespoke, head missing, portrait present with flag clear
            row(2, "Pool Guy", 99, 60000, Some(true)),   // pool; flagged but no portrait file
            row(3, "Pool Two", 98, 60000, Some(false)),  // pool; clear, no file
        ];
        let v = player_containment(&rows, true, containment_contract(Some("nhllegacy"), Platform::Xbox), Some(&d.0.join("fe")), Some(&d.0.join("rendering")));
        let s = &v.summary;
        assert_eq!((s.players, s.bespoke, s.bespoke_head_present, s.bespoke_portrait_present), (4, 2, 1, 2));
        assert_eq!((s.pool_players, s.pool_head_present, s.pool_floor), (2, 2, Some(60_000)));
        assert_eq!((s.flag1_file1, s.flag1_file0, s.flag0_file1, s.flag0_file0), (1, 1, 1, 1));
        assert_eq!((s.shipped_portraits, s.unused_portraits), (Some(3), Some(1)));
        assert_eq!((s.shipped_heads, s.unused_heads), (Some(3), Some(1)));
        assert_eq!(s.bespoke_missing_head, vec!["Star Two"]);
        assert!(s.bespoke_missing_portrait.is_empty());
        assert_eq!(s.flagged_without_file, vec!["Pool Guy"]);
        assert_eq!(v.players[1].head, S::Missing);
        assert!(v.players[1].note.as_deref().unwrap().starts_with("shipped without asset"), "{:?}", v.players[1].note);
        assert!(v.players[2].note.as_deref().unwrap().contains("`hasportrait` is set but no portrait file"));
        assert!(v.players[0].note.is_none());
        assert!(s.notes.iter().any(|n| n.contains("never written from them")));
    }

    #[test]
    fn nhl09_uses_the_head_prefix_and_an_unlisted_directory_is_unknown() {
        let d = Tmp::new("nhl09");
        d.put("fe/ion/artassets/playerheads/p0_4000/p7.big");
        d.put("rendering/player/head_7.rx2");
        let rows = [row(0, "Old Star", 7, 7, Some(true)), row(1, "Old Pool", 8, 10050, Some(false))];
        let v = player_containment(&rows, true, containment_contract(Some("nhl09"), Platform::Xbox), Some(&d.0.join("fe")), Some(&d.0.join("rendering")));
        assert_eq!(v.players[0].head, S::Present);
        assert_eq!(v.summary.pool_floor, Some(10_000));
        assert_eq!(v.players[1].class, "pool");
        assert_eq!(v.players[1].head, S::Missing);
        // No rendering tree: heads unknown, portraits still read.
        let v = player_containment(&rows, true, containment_contract(Some("nhl09"), Platform::Xbox), Some(&d.0.join("fe")), None);
        assert_eq!(v.players[0].head, S::Unknown);
        assert_eq!(v.players[0].portrait, S::Present);
        assert!(v.summary.shipped_heads.is_none() && v.summary.unused_heads.is_none());
        assert!(v.summary.notes.iter().any(|n| n.contains("no rendering tree")));
    }

    #[test]
    fn ps4_reads_portraits_by_art_id_and_leaves_heads_unmeasured() {
        let d = Tmp::new("ps4");
        d.put("ui/kettle_assets/playerportraits/p10000.gnf");
        d.put("ui/kettle_assets/playerportraits/p10001.gnf");
        let rows = [row(0, "Modern Star", 10000, 10000, Some(true)), row(1, "No Portrait", 20000, 60005, Some(false))];
        let v = player_containment(&rows, true, containment_contract(Some("nhl21"), Platform::Ps4), Some(&d.0.join("ui")), None);
        assert!(matches!(v.summary.contract, ContainmentContract::Ps4 { .. }));
        assert_eq!(v.players[0].portrait, S::Present);
        assert_eq!(v.players[0].head, S::Unknown);
        assert_eq!(v.players[1].portrait, S::Missing);
        assert_eq!((v.summary.shipped_portraits, v.summary.unused_portraits), (Some(2), Some(1)));
        assert!(v.summary.shipped_heads.is_none());
        assert!(v.summary.notes.iter().any(|n| n.contains("name-keyed vault")));
        // NHL 15's PNG layout.
        let d = Tmp::new("ps4png");
        d.put("ui/assets/portraits/p5.png");
        let v = player_containment(&[row(0, "Png Star", 5, 5, Some(true))], true, containment_contract(Some("nhl15"), Platform::Ps4), Some(&d.0.join("ui")), None);
        assert_eq!(v.players[0].portrait, S::Present);
    }

    /// Receipt 67: with no loose trees, a complete archive index decides present / missing; an index with an unreadable
    /// archive leaves ids it lacks unknown; no index (or one listing no archive) keeps everything unknown; loose stays first.
    #[test]
    fn archive_members_decide_present_missing_and_unknown() {
        use crate::archive_index::ArchiveIndex;
        let mut idx = ArchiveIndex::default();
        for (p, a) in [
            ("fe/ion/artassets/playerheads/p0_4000/p12.big", "nocache.big"),
            ("fe/ion/artassets/playerheads/silhouettes/p13.big", "nocache.big"), // not a portrait bucket
            ("fe/ion/artassets/playerheadssmall/p0_4000/p13.big", "cache.big"),  // the small set, not this rule
            ("rendering/player/player_12.rx2", "cacherender.big"),
            ("rendering/player/texlib_13.rx2", "cacherender.big"),              // a texture library is not a head
        ] {
            idx.members.insert(p.into(), a.into());
        }
        idx.archives = vec![("nocache.big".into(), 2), ("cache.big".into(), 1), ("cacherender.big".into(), 2)];
        let rows = [row(0, "Star One", 12, 12, Some(true)), row(1, "Star Two", 13, 13, Some(true))];
        let contract = || containment_contract(Some("legacyrecomp"), Platform::Recomp);
        let v = player_containment_with_archives(&rows, true, contract(), None, None, Some(&idx));
        assert_eq!((v.players[0].portrait, v.players[0].head), (S::Present, S::Present));
        assert_eq!((v.players[1].portrait, v.players[1].head), (S::Missing, S::Missing));
        assert_eq!((v.summary.shipped_portraits, v.summary.shipped_heads), (Some(1), Some(1)));
        assert!(v.summary.notes.iter().any(|n| n.contains("checked against this install's own archive members") && n.contains("3 archive(s) listed")), "{:?}", v.summary.notes);
        assert!(!v.summary.notes.iter().any(|n| n.contains("no front-end tree is attached")), "{:?}", v.summary.notes);
        // An unreadable archive: what the index lacks is unknown, and nothing is called unreferenced.
        let mut partial = idx.clone();
        partial.unreadable.push(("patch.big".into(), "truncated".into()));
        let v = player_containment_with_archives(&rows, true, contract(), None, None, Some(&partial));
        assert_eq!((v.players[0].portrait, v.players[1].portrait, v.players[1].head), (S::Present, S::Unknown, S::Unknown));
        assert!(v.summary.unused_portraits.is_none() && v.summary.notes.iter().any(|n| n.contains("could not be listed (patch.big)")));
        // No index, or an index with no archive: unknown, as before.
        for none in [None, Some(&ArchiveIndex::default())] {
            let v = player_containment_with_archives(&rows, true, contract(), None, None, none);
            assert_eq!((v.players[0].portrait, v.players[0].head), (S::Unknown, S::Unknown));
            assert!(v.summary.notes.iter().any(|n| n.contains("no front-end tree is attached")));
        }
        // Loose first: a listed loose directory wins over the index.
        let d = Tmp::new("loosefirst");
        d.put("fe/ion/artassets/playerheads/p0_4000/p13.big");
        d.put("rendering/player/player_13.rx2");
        let v = player_containment_with_archives(&rows, true, contract(), Some(&d.0.join("fe")), Some(&d.0.join("rendering")), Some(&idx));
        assert_eq!((v.players[0].portrait, v.players[1].portrait), (S::Missing, S::Present));
        assert!(!v.summary.notes.iter().any(|n| n.contains("archive members")));
    }

    #[test]
    fn unmeasured_titles_and_a_missing_flag_column_say_so() {
        let rows = [row(0, "Someone", 1, 1, None)];
        let v = player_containment(&rows, false, containment_contract(Some("nhl08"), Platform::Xbox), None, None);
        assert!(matches!(v.summary.contract, ContainmentContract::Unmeasured { .. }));
        assert_eq!(v.players[0].class, "other");
        assert_eq!((v.players[0].portrait, v.players[0].head), (S::Unknown, S::Unknown));
        assert!(v.summary.notes.iter().any(|n| n.contains("not measured")));
        assert!(v.summary.notes.iter().any(|n| n.contains("no `hasportrait` column")));
        assert_eq!(v.players[0].has_portrait_flag, None);
    }
}
