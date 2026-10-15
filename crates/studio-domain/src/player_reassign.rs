//! Players 3.4 — **reassign a player's portrait or head to another shipped asset** (27 Sep 2026,
//! receipt 38; research R124 = 2026-09-26 handoff 17 §4, the refusal list).
//!
//! A player row names its portrait by `artid` (`ubPc.lOnr`) and its head by `headid` (`ubPc.pPaD`);
//! on every walked title each roster number points at a file of exactly that name (R53 / R63). A
//! reassignment is therefore an ordinary database edit of one of those numbers — reviewed, undoable,
//! saved like any edit — **guarded** so it can only point at an asset the title's own tree ships:
//!
//! * portrait: the target must have its `p<id>` file (gen7 `fe/ion/artassets/playerheads/p<bucket>/
//!   p<id>.big`; PS4 `playerportraits/p<id>.gnf`, NHL 15 `assets/portraits/p<id>.png`) and fit the
//!   declared width; `hasportrait` is shown as stored and never flipped; sharing with other rows is
//!   disclosed (stock data already shares a few portrait ids); on PS3 Legacy a `patch.big` copy may
//!   differ (C0-AS7) — disclosed;
//! * head (gen7 only): a mesh of that id (`head_` on 09 / 10, `player_` from 10; `.rx2` / `.rpsgl`); from
//!   NHL 13 also its `texlib_<id>` (every roster head has one); from NHL 12 also a `stockplayerheads`
//!   row; ids within the declared width; taking another player's own (bespoke) head is disclosed as a
//!   duplicate face; the bespoke / pool class change is disclosed.
//!
//! * head on PS4 (receipt 63; R15 / R124 row 78): the target must be a `SkaterNameAttribute` value of exactly one
//!   `ge_nhl_skaterhead` record in the **open title's** identity-checked vault; on NHL 17–21 `headtexture` is written with
//!   `headid` as one change (both fields present and in range, or nothing is written); other PS4 titles write `headid` only.
//! * hair on PS4 NHL 17–21 (receipt 66; R15): [`plan_hair_reassign`] — the target must be the `GenericHairStyle` value of
//!   exactly one `ge_nhl_skaterhairtexturepack` record in the open title's identity-checked vault; only `hairid` is written.
//!
//! Receipt 66 (review F2): PS4 plans carry the exact vault identity they were checked against (`source`), and the review
//! fingerprint binds it together with every disclosure, so a changed vault or identity definition invalidates a review.
//!
//! Refused by name: titles and platforms whose trees research did not walk (X360 07 / 08), and new assets (the external
//! head-conversion workstream).
//! Nothing here claims what the game draws: that is card P34-1.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::player_containment::{bio_rows, containment_contract, ContainmentContract};
use crate::project::Platform;
use crate::{DomainError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Portrait,
    Head,
    /// PS4 NHL 17–21 `hairid` (receipt 66; planned by [`plan_hair_reassign`]).
    Hair,
}

impl AssetKind {
    fn field(self) -> &'static str {
        match self {
            AssetKind::Portrait => "lOnr",
            AssetKind::Head => "pPaD",
            // Resolved by metadata name (`hairid`) per title; this is only the placeholder before resolution.
            AssetKind::Hair => "hairid",
        }
    }
    fn label(self) -> &'static str {
        match self {
            AssetKind::Portrait => "portrait",
            AssetKind::Head => "head",
            AssetKind::Hair => "hair style",
        }
    }
}

/// What a reassignment would do, decided from the database and the attached tree before anything
/// is written. `allowed == false` carries the refusals; the notes are shown either way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReassignPlan {
    pub kind: AssetKind,
    pub bio_record: u32,
    pub player: String,
    /// The stored value, decoded (`-1` = the field's "none" sentinel where the field declares one).
    pub current_id: i64,
    pub target_id: i64,
    /// Studio id of the bio field that is written (`lOnr` / `pPaD`).
    pub field: String,
    pub allowed: bool,
    pub refusals: Vec<String>,
    /// The files that make the target valid (relative to the attached roots).
    pub evidence: Vec<String>,
    /// Disclosures: sharing, duplicate faces, class change, the stored flag, residual caveats.
    pub notes: Vec<String>,
    /// Every bio cell the apply writes, as one change: `(studio field id, value)` — the primary field first; PS4 NHL 17–21
    /// heads add `headtexture` (receipt 63).
    pub writes: Vec<(String, u64)>,
    /// Receipt 66: the exact source identity the check read (PS4: every vault member's full SHA-256 and the identity
    /// definitions' SHA-256); empty where the evidence is file presence by path (gen7 / portraits), re-checked at apply.
    pub source: Vec<String>,
    /// A digest of everything the reviewer was shown and everything the write depends on: title, platform, record, player
    /// name, target, every written field as stored, the evidence, the exact source identity, the refusals and the
    /// disclosures. An apply names the fingerprint it reviewed and is refused when the plan re-derived at apply differs
    /// (receipt 63; widened in receipt 66). Unrelated fields of the row that change nothing shown are not bound.
    pub fingerprint: String,
}

/// Where a PS4 vault-checked target is checked (receipt 63 / 66): the open title's catalogue, a reason it could not be read,
/// or nothing attached.
pub enum Ps4Source<'a, T> {
    NotAttached,
    Problem(String),
    Catalogue(&'a T),
}

/// Where a PS4 head target is checked (receipt 63).
pub type Ps4Heads<'a> = Ps4Source<'a, crate::ps4_heads::HeadCatalogue>;
/// Where a PS4 hair target is checked (receipt 66).
pub type Ps4Hair<'a> = Ps4Source<'a, crate::ps4_hair::HairCatalogue>;

fn exists_any(dir: &Path, stems: &[String], exts: &[&str]) -> Option<String> {
    for s in stems {
        for e in exts {
            let p = dir.join(format!("{s}.{e}"));
            if p.is_file() {
                return Some(p.to_string_lossy().into_owned());
            }
        }
    }
    None
}

/// The portrait file of `id` in a gen7 `playerheads` tree (any `p*` bucket directory).
fn gen7_portrait_file(fe_root: &Path, id: u64) -> Option<String> {
    let base = fe_root.join("ion").join("artassets").join("playerheads");
    let rd = std::fs::read_dir(&base).ok()?;
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() && e.file_name().to_string_lossy().starts_with('p') {
            let f = p.join(format!("p{id}.big"));
            if f.is_file() {
                return Some(f.to_string_lossy().into_owned());
            }
        }
    }
    None
}

/// Whether the title's `stockplayerheads` table (NHL 12 →) carries a row for `head`.
fn stock_head_row(db: &tdb_core::TdbFile, meta: &tdb_defs::FieldMetaIndex, head: u64) -> Option<bool> {
    let t = db.tables.iter().find(|t| meta.table_name(&t.id.raw()) == Some("stockplayerheads"))?;
    let f = t.fields.iter().find(|f| meta.field(&t.id.raw(), &f.id.raw()).is_some_and(|m| m.name == "headid"))?;
    Some((0..t.current_records as u32).any(|r| db.read_u64(t, r, f) == Some(head)))
}

/// Plan a reassignment of player `bio_record`'s portrait or head to `target`. Read-only.
#[allow(clippy::too_many_arguments)]
pub fn plan_reassign(
    game_id: Option<&str>,
    platform: Platform,
    db: &tdb_core::TdbFile,
    meta: &tdb_defs::FieldMetaIndex,
    fe_root: Option<&Path>,
    rendering_root: Option<&Path>,
    archives: Option<&crate::archive_index::ArchiveIndex>,
    ps4_heads: Ps4Heads<'_>,
    bio_record: u32,
    kind: AssetKind,
    target: u64,
) -> Result<ReassignPlan> {
    let (rows, has_flag) = bio_rows(db, meta).ok_or_else(|| DomainError::Validation("this database has no player bio table".into()))?;
    let row = rows.iter().find(|r| r.record == bio_record).ok_or_else(|| DomainError::Validation(format!("bio record {bio_record} does not exist")))?;
    let current = match kind {
        AssetKind::Portrait => row.portrait_id,
        AssetKind::Head => row.head_id,
        AssetKind::Hair => return Err(DomainError::Validation("hair reassignment is planned by plan_hair_reassign".into())),
    };
    let mut plan = ReassignPlan {
        kind,
        bio_record,
        player: row.full_name.clone(),
        current_id: current as i64,
        target_id: target as i64,
        field: kind.field().to_string(),
        allowed: false,
        refusals: Vec::new(),
        evidence: Vec::new(),
        notes: Vec::new(),
        writes: vec![(kind.field().to_string(), target)],
        source: Vec::new(),
        fingerprint: String::new(),
    };
    let refuse = |p: &mut ReassignPlan, why: String| p.refusals.push(why);
    if target == current {
        refuse(&mut plan, format!("{} already uses {} {target}", row.full_name, kind.label()));
    }
    // Declared width of the bio field.
    let bio = db.table_canonical(tdb_relations::entity::ids::BIO).expect("bio rows came from it");
    if let Some(fd) = bio.fields.iter().find(|f| f.id.canonical(db.endian) == kind.field()) {
        let max = meta.field(&bio.id.raw(), &fd.id.raw()).and_then(|m| m.range_high).map(|h| h as u64).unwrap_or_else(|| if fd.bit_width >= 64 { u64::MAX } else { (1u64 << fd.bit_width) - 1 });
        if target > max {
            refuse(&mut plan, format!("{} id {target} does not fit the {}-bit field (0–{max})", kind.label(), fd.bit_width));
        }
    }
    let contract = containment_contract(game_id, platform);
    // Receipt 55: archive-held sources — PS3 NHL 09 → 14 (research R134) and Legacy Recompiled (R138) — are checked against the
    // install's own archive members when no loose file exists. The Legacy-era rules (texture library, head registry) apply
    // to Recompiled's identical bytes.
    let archive_held = (platform == Platform::Ps3 && matches!(game_id, Some("nhl09" | "nhl10" | "nhl11" | "nhl12" | "nhl13" | "nhl14"))) || platform == Platform::Recomp;
    let legacy_rules = matches!(game_id, Some("nhl13" | "nhl14" | "nhl15" | "nhllegacy" | "legacyrecomp"));
    let registry_rules = matches!(game_id, Some("nhl12" | "nhl13" | "nhl14" | "nhl15" | "nhllegacy" | "legacyrecomp"));
    if archive_held && archives.is_none() && rendering_root.is_none() && fe_root.is_none() {
        refuse(&mut plan, "this install keeps its portraits and heads inside its archives; attach the install so its archive members can be checked".into());
    }
    if let Some(idx) = archives.filter(|_| archive_held) {
        if !idx.unreadable.is_empty() {
            plan.notes.push(format!(
                "{} archive(s) of this install could not be listed and were not searched: {}",
                idx.unreadable.len(),
                idx.unreadable.iter().map(|(a, _)| a.as_str()).take(4).collect::<Vec<_>>().join(", ")
            ));
        }
    }
    let member = |rel: &str| -> Option<String> { archives.filter(|_| archive_held).and_then(|i| i.find(rel).map(|a| format!("{a}: {rel}"))) };
    let others: Vec<&str> = rows
        .iter()
        .filter(|r| r.record != bio_record && match kind {
            AssetKind::Portrait => r.portrait_id == target,
            AssetKind::Head => r.head_id == target,
            AssetKind::Hair => false,
        })
        .map(|r| r.full_name.as_str())
        .collect();
    match (&contract, kind) {
        (ContainmentContract::Unmeasured { reason }, _) => refuse(&mut plan, format!("{reason}; reassignment needs the title's measured file rule")),
        (_, AssetKind::Hair) => refuse(&mut plan, "hair reassignment is planned by plan_hair_reassign".into()),
        (ContainmentContract::Ps4 { .. }, AssetKind::Head) => {
            // R124 row 78: on NHL 17–21 `headtexture` equals `headid` on every stock row, and the two are written as one change.
            let paired_title = matches!(game_id, Some("nhl17" | "nhl18" | "nhl19" | "nhl20" | "nhl21"));
            let texture = bio.fields.iter().find(|f| meta.field(&bio.id.raw(), &f.id.raw()).is_some_and(|m| m.name == "headtexture"));
            match (paired_title, texture) {
                (true, None) => refuse(&mut plan, "this NHL 17–21 player table has no `headtexture` field, so the paired write research R124 measured (headtexture = headid) cannot be made; nothing is written".into()),
                (true, Some(fd)) => {
                    let max = meta.field(&bio.id.raw(), &fd.id.raw()).and_then(|m| m.range_high).map(|h| h as u64).unwrap_or_else(|| if fd.bit_width >= 64 { u64::MAX } else { (1u64 << fd.bit_width) - 1 });
                    if target > max {
                        refuse(&mut plan, format!("head id {target} does not fit `headtexture` (0–{max}); nothing is written"));
                    }
                    let stored = db.read_u64(bio, bio_record, fd);
                    if stored != Some(row.head_id) {
                        plan.notes.push(format!(
                            "this row's `headtexture` is {} while its `headid` is {} (an exception to the stock pairing); the reassignment writes both as {target}",
                            stored.map(|v| v.to_string()).unwrap_or_else(|| "unreadable".into()),
                            row.head_id
                        ));
                    }
                    plan.writes.push((fd.id.canonical(db.endian), target));
                }
                (false, Some(_)) => plan.notes.push("this title's player table has a `headtexture` field, but research measured its pairing with `headid` on NHL 17–21 only; it is left as stored".into()),
                (false, None) => {}
            }
            match ps4_heads {
                Ps4Heads::NotAttached => refuse(&mut plan, "attach the title's install so the target can be checked against its own head catalogue (its vault)".into()),
                Ps4Heads::Problem(p) => refuse(&mut plan, p),
                Ps4Heads::Catalogue(cat) => {
                    plan.source = cat.source.identity_lines();
                    match cat.records_for(target as i64) {
                        0 => refuse(&mut plan, format!("the {} vault has no head record for {target} (a ge_nhl_skaterhead with that SkaterNameAttribute value); only shipped heads are offered", cat.title)),
                        1 => plan.evidence.push(format!("ge_nhl_skaterhead with SkaterNameAttribute {target} — {}", cat.describe())),
                        n => refuse(&mut plan, format!("{n} head records of the {} vault carry {target}; the target is ambiguous and not offered", cat.title)),
                    }
                    if row.head_id != 0 && cat.records_for(row.head_id as i64) == 0 {
                        plan.notes.push(format!("the current head {} has no record in this title's vault (as shipped, e.g. NHL 15's 10391); it is shown as stored", row.head_id));
                    }
                    if cat.records_without_value > 0 {
                        plan.notes.push(format!("{} head record(s) of this vault carry no readable SkaterNameAttribute and are not offered", cat.records_without_value));
                    }
                }
            }
        }
        (ContainmentContract::Ps4 { .. }, AssetKind::Portrait) => match fe_root {
            None => refuse(&mut plan, "no front-end tree is attached, so the target portrait cannot be checked".into()),
            Some(fe) => match exists_any(&fe.join("kettle_assets").join("playerportraits"), &[format!("p{target}")], &["gnf"]).or_else(|| exists_any(&fe.join("assets").join("portraits"), &[format!("p{target}")], &["png"])) {
                Some(f) => plan.evidence.push(f),
                None => refuse(&mut plan, format!("no portrait file p{target} ships in this title's tree (playerportraits/p{target}.gnf or assets/portraits/p{target}.png)")),
            },
        },
        (ContainmentContract::Gen7 { extension, head_prefixes, .. }, AssetKind::Portrait) => {
            let _ = (extension, head_prefixes);
            let loose = fe_root.and_then(|fe| gen7_portrait_file(fe, target));
            let packed = archives.filter(|_| archive_held).and_then(|i| i.portrait(target).map(|(p, a)| format!("{a}: {p}")));
            match loose.or(packed) {
                Some(f) => plan.evidence.push(f),
                None if fe_root.is_none() && !(archive_held && archives.is_some()) => refuse(&mut plan, "no front-end tree is attached, so the target portrait cannot be checked".into()),
                None => refuse(&mut plan, format!("no portrait p{target}.big ships in this title's playerheads{}", if archive_held { " (loose tree or archive members)" } else { " tree" })),
            }
            if platform == Platform::Ps3 && game_id == Some("nhllegacy") {
                plan.notes.push("PS3 NHL Legacy: `patch.big` carries some portrait members that differ from the loose files (C0-AS7, 23 ids); which copy the game loads is not observed".into());
            }
        }
        (ContainmentContract::Gen7 { extension, head_prefixes, .. }, AssetKind::Head) => {
            let exts: [&str; 3] = [extension.as_str(), "rx2", "rpsgl"];
            let stems: Vec<String> = head_prefixes.iter().map(|p| format!("{p}{target}")).collect();
            // A loose file, else (archive-held titles) an archive member of the same relative path.
            let find = |stem: &str| -> Option<String> {
                rendering_root
                    .and_then(|r| exists_any(&r.join("player"), &[stem.to_string()], &exts))
                    .or_else(|| exts.iter().find_map(|e| member(&format!("rendering/player/{stem}.{e}"))))
            };
            if rendering_root.is_none() && !(archive_held && archives.is_some()) {
                refuse(&mut plan, "no rendering tree is attached, so the target head cannot be checked".into());
            } else {
                match stems.iter().find_map(|s| find(s)) {
                    Some(f) => plan.evidence.push(f),
                    None => refuse(&mut plan, format!("no head mesh {} ships in this title's rendering/player{}", stems.join(" / "), if archive_held { " (loose tree or archive members)" } else { " tree" })),
                }
                if legacy_rules {
                    match find(&format!("texlib_{target}")) {
                        Some(f) => plan.evidence.push(f),
                        None => refuse(&mut plan, format!("head {target} has no texture library texlib_{target} — from NHL 13 every roster head has one (research R124), so a head without it is outside what the game ships")),
                    }
                }
                // PS3 NHL 10 → 12 ship per-head colour textures `head_<id>_cm` (R134 §9): disclosed, not required — 370–432
                // roster heads have one, and whether the others fall back is runtime.
                if platform == Platform::Ps3 && matches!(game_id, Some("nhl10" | "nhl11" | "nhl12")) {
                    match find(&format!("head_{target}_cm")) {
                        Some(f) => plan.notes.push(format!("head {target} has its PS3 per-head colour texture ({f})")),
                        None => plan.notes.push(format!("head {target} has no PS3 per-head colour texture head_{target}_cm; some shipped roster heads also have none, and what the game draws without it is not observed (R134 §9)")),
                    }
                }
            }
            if registry_rules {
                match stock_head_row(db, meta, target) {
                    Some(true) => plan.evidence.push(format!("stockplayerheads row for headid {target}")),
                    Some(false) => refuse(&mut plan, format!("head {target} has no `stockplayerheads` row (the head registry of NHL 12 →, research R124); only registered heads are offered")),
                    None => {}
                }
            }
        }
    }
    // Head class (gen7 and PS4): bespoke = head id equals the player's portrait id (R53); pool from the title's floor; duplicate
    // faces disclosed.
    if kind == AssetKind::Head && !matches!(contract, ContainmentContract::Unmeasured { .. }) {
        let pool_min = if matches!(contract, ContainmentContract::Ps4 { .. }) { crate::assets::POOL_HEAD_MIN } else { crate::assets::POOL_HEAD_MIN_EARLY_GEN7 };
        let owners: Vec<&str> = rows.iter().filter(|r| r.record != bio_record && r.head_id == target && r.portrait_id == target).map(|r| r.full_name.as_str()).collect();
        let class = if target >= pool_min {
            format!("a shared pool head ({pool_min}+)")
        } else if target == row.portrait_id {
            "this player's own bespoke head (head id = portrait id)".to_string()
        } else if !owners.is_empty() {
            format!("{}'s own bespoke head", owners.join(", "))
        } else {
            "a head no roster row owns as its bespoke face".to_string()
        };
        plan.notes.push(format!("target head {target}: {class}"));
        let bespoke_now = row.head_id == row.portrait_id;
        let bespoke_after = target == row.portrait_id;
        if bespoke_now != bespoke_after {
            plan.notes.push(format!("the head becomes {} (bespoke = head id equals portrait id, R53)", if bespoke_after { "bespoke" } else { "not bespoke" }));
        }
        if !owners.is_empty() {
            plan.notes.push(format!("head {target} is {}'s own face — two players would share it", owners.join(", ")));
        }
    }
    if !others.is_empty() && !(kind == AssetKind::Head && target >= crate::assets::POOL_HEAD_MIN_EARLY_GEN7) {
        let shown: Vec<&str> = others.iter().take(6).copied().collect();
        plan.notes.push(format!("{} {target} is also used by {}{}", kind.label(), shown.join(", "), if others.len() > shown.len() { format!(" and {} more", others.len() - shown.len()) } else { String::new() }));
    }
    if kind == AssetKind::Portrait && has_flag {
        plan.notes.push(format!("`hasportrait` stays as stored ({}); Studio never derives or flips it from a file", match row.has_portrait {
            Some(true) => "1",
            Some(false) => "0",
            None => "—",
        }));
    }
    plan.notes.push("what the game draws for the new id is not observed (card P34-1); this is a database edit of one number, reviewed and undoable like any edit".into());
    plan.allowed = plan.refusals.is_empty();
    plan.fingerprint = fingerprint(game_id, platform, db, bio, bio_record, &plan);
    Ok(plan)
}

/// Plan a PS4 NHL 17–21 reassignment of player `bio_record`'s `hairid` to `target` (receipt 66; research R15): the target must
/// be the `GenericHairStyle` value of exactly one `ge_nhl_skaterhairtexturepack` record in the open title's identity-checked
/// vault. Only `hairid` is written; `headid`, `headtexture` and every other appearance field stay as stored. Read-only.
#[allow(clippy::too_many_arguments)]
pub fn plan_hair_reassign(game_id: Option<&str>, platform: Platform, db: &tdb_core::TdbFile, meta: &tdb_defs::FieldMetaIndex, hair: Ps4Hair<'_>, bio_record: u32, target: i64) -> Result<ReassignPlan> {
    let (rows, _) = bio_rows(db, meta).ok_or_else(|| DomainError::Validation("this database has no player bio table".into()))?;
    let row = rows.iter().find(|r| r.record == bio_record).ok_or_else(|| DomainError::Validation(format!("bio record {bio_record} does not exist")))?;
    let bio = db.table_canonical(tdb_relations::entity::ids::BIO).expect("bio rows came from it");
    let fd = bio.fields.iter().find(|f| meta.field(&bio.id.raw(), &f.id.raw()).is_some_and(|m| m.name == "hairid"));
    let fm = fd.and_then(|f| meta.field(&bio.id.raw(), &f.id.raw()));
    let current = match (fd, fm) {
        (Some(f), Some(m)) => db.read_u64(bio, bio_record, f).map(|raw| m.decode_raw(raw)),
        _ => None,
    };
    let field = fd.map(|f| f.id.canonical(db.endian)).unwrap_or_else(|| AssetKind::Hair.field().to_string());
    let mut plan = ReassignPlan {
        kind: AssetKind::Hair,
        bio_record,
        player: row.full_name.clone(),
        current_id: current.unwrap_or(-1),
        target_id: target,
        field: field.clone(),
        allowed: false,
        refusals: Vec::new(),
        evidence: Vec::new(),
        notes: Vec::new(),
        writes: vec![(field, target.max(0) as u64)],
        source: Vec::new(),
        fingerprint: String::new(),
    };
    let measured = platform == Platform::Ps4 && game_id.is_some_and(|g| crate::ps4_hair::HAIR_TITLES.contains(&g));
    if !measured {
        plan.refusals.push(match (platform, game_id) {
            (Platform::Ps4, Some("nhl15" | "nhl16")) => "the NHL 15 / 16 vaults carry no ge_nhl_skaterhairtexturepack class (research R15), so a hair id cannot be checked; nothing is written".to_string(),
            _ => "hair reassignment is measured on PS4 NHL 17–21 only (research R15); nothing is written".to_string(),
        });
    }
    match (fd, fm) {
        (None, _) | (_, None) => plan.refusals.push("this player table has no `hairid` field; nothing is written".into()),
        (Some(f), Some(m)) => {
            if current.is_none() {
                plan.refusals.push("this row's `hairid` cannot be read; nothing is written".into());
            }
            let hi = m.range_high.unwrap_or(if f.bit_width >= 63 { i64::MAX } else { (1i64 << f.bit_width) - 1 });
            if target < 0 {
                plan.refusals.push(format!("hair style {target} is negative; a reassignment names a shipped style (0–{hi}), never the field's \"none\" value"));
            } else if target > hi {
                plan.refusals.push(format!("hair style {target} does not fit `hairid` (0–{hi})"));
            }
            if !m.updatable {
                plan.refusals.push("`hairid` is not updatable in this title's metadata".into());
            }
        }
    }
    if current == Some(target) {
        plan.refusals.push(format!("{} already uses hair style {target}", row.full_name));
    }
    if measured {
        match hair {
            Ps4Hair::NotAttached => plan.refusals.push("attach the title's install so the target can be checked against its own hair catalogue (its vault)".into()),
            Ps4Hair::Problem(p) => plan.refusals.push(p),
            Ps4Hair::Catalogue(cat) => {
                plan.source = cat.source.identity_lines();
                let c = crate::ps4_hair::HAIR_CLASS;
                match cat.records_for(target) {
                    0 => plan.refusals.push(format!("the {} vault has no hair record for {target} (a {c} with that GenericHairStyle value); only shipped styles are offered", cat.title)),
                    1 => plan.evidence.push(format!("{c} with GenericHairStyle {target} — {}", cat.describe())),
                    n => plan.refusals.push(format!("{n} hair records of the {} vault carry {target}; the target is ambiguous and not offered", cat.title)),
                }
                if let Some(cur) = current.filter(|c| *c >= 0) {
                    if cat.records_for(cur) != 1 {
                        plan.notes.push(format!("the current hair style {cur} has {} record(s) in this title's vault (a stock exception); it is shown as stored, not normalized", cat.records_for(cur)));
                    }
                }
                if cat.records_without_value > 0 {
                    plan.notes.push(format!("{} hair record(s) of this vault carry no readable GenericHairStyle and are not offered", cat.records_without_value));
                }
            }
        }
    }
    if current == Some(-1) {
        plan.notes.push("this row's `hairid` is -1 (the field's \"none\" value); what the game draws for it is not observed".into());
    }
    if target >= 0 {
        let sharing = rows.iter().filter(|r| r.record != bio_record).filter(|r| {
            match (fd, fm) {
                (Some(f), Some(m)) => db.read_u64(bio, r.record, f).map(|raw| m.decode_raw(raw)) == Some(target),
                _ => false,
            }
        }).count();
        plan.notes.push(format!("hair style {target} is used by {sharing} other player row(s); styles are shared, not a player's own"));
    }
    plan.notes.push("only `hairid` is written: `headid`, `headtexture` and the other appearance fields stay as stored".into());
    plan.notes.push("the binding names a hair texture pack; its colour, geometry, helmet compatibility and what the game draws are not derived or observed — a database edit of one number, reviewed and undoable like any edit".into());
    plan.allowed = plan.refusals.is_empty();
    plan.fingerprint = fingerprint(game_id, platform, db, bio, bio_record, &plan);
    Ok(plan)
}

/// The plan's review digest (receipt 63; widened in receipt 66 review F2): title, platform, record, player name, target, every
/// written field as currently stored (raw bits), the evidence, the exact source identity, the refusals and the disclosures.
/// Two plans with the same digest show the reviewer the same thing and write the same cells from the same stored values.
fn fingerprint(game_id: Option<&str>, platform: Platform, db: &tdb_core::TdbFile, bio: &tdb_core::Table, bio_record: u32, plan: &ReassignPlan) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(format!("{}|{}|{bio_record}|{:?}|{}|{}\n", game_id.unwrap_or("-"), platform.as_str(), plan.kind, plan.target_id, plan.player));
    for (f, v) in &plan.writes {
        let stored = bio.fields.iter().find(|d| d.id.canonical(db.endian) == *f).and_then(|d| db.read_u64(bio, bio_record, d));
        h.update(format!("write {f}={stored:?}->{v}\n"));
    }
    for (tag, lines) in [("evidence", &plan.evidence), ("source", &plan.source), ("refusal", &plan.refusals), ("note", &plan.notes)] {
        for l in lines {
            h.update(format!("{tag} {l}\n"));
        }
    }
    h.update(format!("allowed={}", plan.refusals.is_empty()));
    format!("{:x}", h.finalize())[..32].to_string()
}
