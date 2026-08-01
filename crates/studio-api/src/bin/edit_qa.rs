//! `edit-qa` — a headless edit-QA runner.
//!
//! Drives the **exact `studio_api::Registry` seam the app's UI calls** across the committed anchor
//! matrix (NHL 07–21 × PS3 / Xbox360 / PS4), applies a capability-tolerant smoke set (or an
//! operator-supplied JSON spec), and:
//!   * deposits the **edited `.db` / roster save** for each cell under `<out>/artifacts/` — mirroring
//!     the anchor layout so they can be applied in-game;
//!   * writes a **human-readable markdown report** per cell (resolved names, before→after, round-trip
//!     / CRC verdicts) under `<out>/reports/`.
//!
//! Anchors are **read-only sources** — every edit happens on a copy in the output tree; the anchor
//! files are never written. Nothing here touches Tauri: the whole flow is plain Rust over `Registry`.
//!
//! ```text
//! cargo run -p studio-api --bin edit-qa -- \
//!   --anchors-root ./anchors --defs-root ./defs \
//!   --games all --platforms all [--spec edits.json] --out ./qa-out
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use studio_api::{
    CapabilitiesDto, FieldViewDto, PlayerSummaryDto, PlayerViewDto, ProjectSummaryDto, Registry,
};
use studio_domain::{FieldValue, Platform};

// ----------------------------------------------------------------------------- args

struct Args {
    anchors_root: PathBuf,
    defs_root: PathBuf,
    out: PathBuf,
    games: Option<Vec<String>>,     // None = all
    platforms: Option<Vec<String>>, // None = all
    spec: Option<PathBuf>,
}

fn parse_args() -> Args {
    let mut a = Args {
        anchors_root: PathBuf::from("anchors"),
        defs_root: PathBuf::from("defs"),
        out: PathBuf::from("qa-out"),
        games: None,
        platforms: None,
        spec: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut val = || it.next().unwrap_or_default();
        match flag.as_str() {
            "--anchors-root" => a.anchors_root = PathBuf::from(val()),
            "--defs-root" => a.defs_root = PathBuf::from(val()),
            "--out" => a.out = PathBuf::from(val()),
            "--spec" => a.spec = Some(PathBuf::from(val())),
            "--games" => {
                let v = val();
                a.games = (v != "all").then(|| csv(&v));
            }
            "--platforms" => {
                let v = val();
                a.platforms = (v != "all").then(|| csv(&v));
            }
            "-h" | "--help" => {
                eprintln!(
                    "edit-qa --anchors-root <dir> --defs-root <dir> --out <dir> \
                     [--games all|nhl14,nhl15] [--platforms all|ps3,ps4] [--spec edits.json]"
                );
                std::process::exit(0);
            }
            other => eprintln!("warning: ignoring unknown flag {other:?}"),
        }
    }
    a
}

fn csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|t| t.trim().to_ascii_lowercase())
        .filter(|t| !t.is_empty())
        .collect()
}

// ----------------------------------------------------------------------------- matrix discovery

/// One (title, platform) anchor cell resolved to concrete files.
struct Cell {
    game: String,
    platform_dir: String, // "ps3" | "xbox360" | "ps4"
    platform: Platform,
    /// Anchor main DB (nhlng.db / NHL07NG.db / nhl_game.db).
    db: PathBuf,
    /// Metadata XML (may live under the sibling xbox360 tree when a PS3 cell ships none).
    meta: PathBuf,
    /// Anchor roster save dir (PS3/Xbox only), if one is present.
    roster_dir: Option<PathBuf>,
}

fn platform_of(dir: &str) -> Option<Platform> {
    match dir {
        "ps3" => Some(Platform::Ps3),
        "xbox360" => Some(Platform::Xbox),
        "ps4" => Some(Platform::Ps4),
        _ => None,
    }
}

/// Resolve the main DB file inside a `databases/` dir, preferring the modern name.
fn find_db(db_dir: &Path) -> Option<PathBuf> {
    for name in ["nhlng.db", "nhl_game.db", "NHL07NG.db"] {
        let p = db_dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// `<stem>-meta.xml` beside the DB; if absent (e.g. PS3 NHL10 ships none), fall back to the
/// byte-identical xbox360 sibling's meta.
fn find_meta(db: &Path, anchors_root: &Path, game: &str) -> Option<PathBuf> {
    let stem = db.file_stem()?.to_string_lossy().into_owned();
    let local = db.with_file_name(format!("{stem}-meta.xml"));
    if local.is_file() {
        return Some(local);
    }
    let sibling = anchors_root
        .join("xbox360")
        .join(game)
        .join("databases")
        .join(format!("{stem}-meta.xml"));
    sibling.is_file().then_some(sibling)
}

/// The roster save dir under `savefiles/` (classified here by folder name — anchors are
/// conventionally named; discovery-by-content is the app's job, not this smoke harness's).
fn find_roster_dir(save_root: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(save_root).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir()
            && p.file_name()
                .map(|n| n.to_string_lossy().to_ascii_uppercase().contains("ROSTER"))
                .unwrap_or(false)
        {
            return Some(p);
        }
    }
    None
}

/// The container file inside a roster save dir: `SYS-DATA` (PS3), `DATA` (PS4), or the single inner
/// file (Xbox).
fn roster_container(dir: &Path) -> Option<PathBuf> {
    for name in ["SYS-DATA", "DATA"] {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    (files.len() == 1).then(|| files.remove(0))
}

fn discover(args: &Args) -> Vec<Cell> {
    let mut cells = Vec::new();
    let plat_ok = |d: &str| args.platforms.as_ref().is_none_or(|v| v.iter().any(|p| p == d));
    let game_ok = |g: &str| args.games.as_ref().is_none_or(|v| v.iter().any(|x| x == g));

    let Ok(plats) = fs::read_dir(&args.anchors_root) else {
        return cells;
    };
    for plat in plats.flatten().filter(|e| e.path().is_dir()) {
        let platform_dir = plat.file_name().to_string_lossy().into_owned();
        let Some(platform) = platform_of(&platform_dir) else { continue };
        if !plat_ok(&platform_dir) {
            continue;
        }
        let Ok(games) = fs::read_dir(plat.path()) else { continue };
        for g in games.flatten().filter(|e| e.path().is_dir()) {
            let game = g.file_name().to_string_lossy().into_owned();
            if !game_ok(&game) {
                continue;
            }
            let db_dir = g.path().join("databases");
            let Some(db) = find_db(&db_dir) else { continue };
            let Some(meta) = find_meta(&db, &args.anchors_root, &game) else {
                eprintln!("skip {platform_dir}/{game}: no metadata XML found");
                continue;
            };
            // Roster save on any platform: PS3/Xbox mirror `nhlng.db`; PS4 is a standalone LZ4 save.
            let roster_dir = find_roster_dir(&g.path().join("savefiles"));
            cells.push(Cell {
                game,
                platform_dir: platform_dir.clone(),
                platform,
                db,
                meta,
                roster_dir,
            });
        }
    }
    cells.sort_by_key(|c| (c.platform_dir.clone(), c.game.clone()));
    cells
}

// ----------------------------------------------------------------------------- fs helpers

fn copy_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src, dst)?;
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let from = e.path();
        let to = dst.join(e.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------------- edit steps

#[derive(Clone, Copy, PartialEq)]
enum Status {
    Ok,
    Skipped,
    Rejected, // an edit correctly refused (e.g. out-of-range) — a *pass* for that step
    Failed,
}

struct Step {
    name: String,
    status: Status,
    detail: String,
    /// Raw studio ids, surfaced only in the details affordance.
    technical: Option<String>,
}

impl Step {
    fn ok(name: &str, detail: String) -> Self {
        Self { name: name.into(), status: Status::Ok, detail, technical: None }
    }
    fn skipped(name: &str, detail: String) -> Self {
        Self { name: name.into(), status: Status::Skipped, detail, technical: None }
    }
    fn rejected(name: &str, detail: String) -> Self {
        Self { name: name.into(), status: Status::Rejected, detail, technical: None }
    }
    fn failed(name: &str, detail: String) -> Self {
        Self { name: name.into(), status: Status::Failed, detail, technical: None }
    }
    fn tech(mut self, t: String) -> Self {
        self.technical = Some(t);
        self
    }
    fn glyph(&self) -> &'static str {
        match self.status {
            Status::Ok => "✓",
            Status::Skipped => "—",
            Status::Rejected => "✓ (rejected as expected)",
            Status::Failed => "✗",
        }
    }
}

/// The bits of a chosen field the harness needs — a small owned snapshot so we don't hold a borrow
/// of the (serialize-only) DTO across an edit.
struct Picked {
    table: String,
    record: u32,
    field: String,
    name: String,
    range_high: Option<i64>,
    display: String,
    text: Option<String>,
}

fn snapshot(f: &FieldViewDto, table: String, record: u32) -> Picked {
    Picked {
        table,
        record,
        field: f.studio_field.clone(),
        name: f.name.clone(),
        range_high: f.range_high,
        display: f.display.clone(),
        text: f.text.clone(),
    }
}

/// Pick an editable, bounded integer field on the target player; prefer a rating (visible), then a
/// bio field.
fn pick_int_field(view: &PlayerViewDto) -> Option<Picked> {
    let usable = |f: &FieldViewDto| {
        f.editable
            && !f.is_string
            && f.kind == "integer"
            && matches!((f.range_low, f.range_high), (Some(lo), Some(hi)) if hi > lo)
    };
    if let Some(rec) = view.ratings_record {
        if let Some(f) = view.ratings.iter().find(|f| usable(f)) {
            return Some(snapshot(f, view.ratings_table.clone(), rec));
        }
    }
    view.identity
        .iter()
        .find(|f| usable(f))
        .map(|f| snapshot(f, f.studio_table.clone(), view.bio_record))
}

/// Pick an editable inline-string field on the target player (bio identity), for the string path.
fn pick_string_field(view: &PlayerViewDto) -> Option<Picked> {
    view.identity
        .iter()
        .find(|f| f.editable && f.is_string)
        .map(|f| snapshot(f, f.studio_table.clone(), view.bio_record))
}

/// Re-read one field's display value after an edit (matched by studio ids).
fn read_field_display(view: &PlayerViewDto, table: &str, field: &str) -> Option<String> {
    view.all_fields
        .iter()
        .chain(view.identity.iter())
        .chain(view.ratings.iter())
        .find(|f| f.studio_table == table && f.studio_field == field)
        .map(|f| f.display.clone())
}

/// The built-in, capability-tolerant smoke set. Every op is try-and-report: a domain refusal marks
/// the step skipped/failed and the cell keeps going, so the harness works across the whole matrix.
fn run_smoke(reg: &Registry, id: u32) -> Vec<Step> {
    let mut steps = Vec::new();

    let players = match reg.list_players(id) {
        Ok(p) => p,
        Err(e) => {
            steps.push(Step::failed("Load players", e.to_string()));
            return steps;
        }
    };
    if players.is_empty() {
        steps.push(Step::skipped("Load players", "no players in this database".into()));
        return steps;
    }

    // Clean-up ops first — the app's on-load workflow. Clearing a pre-existing duplicate jersey
    // matters because the transaction harness re-audits after every edit and rolls back if *any*
    // violation (even a pre-existing one) is present, so a dirty anchor would otherwise block all
    // subsequent generic edits.
    match reg.autocorrect_jerseys(id) {
        Ok(s) => steps.push(Step::ok(
            "Autocorrect jerseys",
            format!("{} correction(s)", s.corrections.len()),
        )),
        Err(e) => steps.push(Step::skipped("Autocorrect jerseys", e.to_string())),
    }
    match reg.repair_roster_blocks(id) {
        Ok(s) => steps.push(Step::ok(
            "Repair roster blocks",
            format!("{} slot(s) moved", s.moved_slots),
        )),
        Err(e) => steps.push(Step::skipped("Repair roster blocks", e.to_string())),
    }

    // Target: the first player who occupies a roster slot (so team-scoped ops are valid).
    let target: &PlayerSummaryDto = players
        .iter()
        .find(|p| p.team_id.is_some())
        .unwrap_or(&players[0]);
    let pid = target.player_id;
    let team = target.team_id;
    let pname = format!("{} {}", target.first_name, target.last_name).trim().to_string();

    let view = reg.player_view(id, pid).ok().flatten();

    // 1) Scalar bounds edit + out-of-range rejection.
    if let Some(view) = view.as_ref() {
        if let Some(p) = pick_int_field(view) {
            let hi = p.range_high.unwrap();
            match reg.set_field(id, &p.table, p.record, &p.field, FieldValue::Int(hi)) {
                Ok(_) => {
                    let after = reg
                        .player_view(id, pid)
                        .ok()
                        .flatten()
                        .and_then(|v| read_field_display(&v, &p.table, &p.field))
                        .unwrap_or_else(|| hi.to_string());
                    steps.push(
                        Step::ok(
                            "Scalar edit (to max)",
                            format!("{pname} · {}: {} → {after}", p.name, p.display),
                        )
                        .tech(format!("{}.{}  record {}  = {hi}", p.table, p.field, p.record)),
                    );
                }
                Err(e) => steps.push(Step::failed("Scalar edit (to max)", format!("{}: {e}", p.name))),
            }
            // Rejection probe.
            if let Some(over) = hi.checked_add(1) {
                match reg.set_field(id, &p.table, p.record, &p.field, FieldValue::Int(over)) {
                    Ok(_) => steps.push(Step::failed(
                        "Out-of-range rejection",
                        format!("{}: value {over} was ACCEPTED (expected rejection)", p.name),
                    )),
                    Err(e) => steps.push(Step::rejected(
                        "Out-of-range rejection",
                        format!("{}: {e}", p.name),
                    )),
                }
            }
        } else {
            steps.push(Step::skipped("Scalar edit (to max)", "no bounded integer field found".into()));
        }

        // 2) Inline-string edit.
        if let Some(p) = pick_string_field(view) {
            let before = p.text.clone().unwrap_or_default();
            match reg.set_field(id, &p.table, p.record, &p.field, FieldValue::Text("QA".into())) {
                Ok(_) => steps.push(
                    Step::ok(
                        "Inline-string edit",
                        format!("{}: {before:?} → \"QA\"", p.name),
                    )
                    .tech(format!("{}.{}  record {}", p.table, p.field, p.record)),
                ),
                Err(e) => steps.push(Step::failed("Inline-string edit", format!("{}: {e}", p.name))),
            }
        } else {
            steps.push(Step::skipped("Inline-string edit", "no editable string field found".into()));
        }
    } else {
        steps.push(Step::skipped("Scalar/string edit", "player detail unavailable".into()));
    }

    // 3) Jersey number set (team-scoped).
    if let Some(t) = team {
        match reg.set_jersey_number(id, t, pid, 98) {
            Ok(o) => steps.push(Step::ok(
                "Set jersey number",
                format!("{pname} → #98 ({} edit(s))", o.edits),
            )),
            Err(e) => steps.push(Step::skipped("Set jersey number", e.to_string())),
        }
        // 4) Captaincy set.
        match reg.set_captaincy(id, t, pid, 1) {
            Ok(o) => steps.push(Step::ok(
                "Set captaincy",
                format!("{pname} → captain role ({} edit(s))", o.edits),
            )),
            Err(e) => steps.push(Step::skipped("Set captaincy", e.to_string())),
        }
    } else {
        steps.push(Step::skipped("Jersey / captaincy", "target player is not on a team".into()));
    }

    // Free-agent slot ops (last: they move players). Release the target from its team — which
    // frees a slot there — then sign a *guaranteed* free agent into that now-free slot. Two players,
    // but both ops run with satisfied preconditions (release needs an on-roster player; sign needs a
    // free agent + a team with a free slot), and the net roster count is unchanged.
    if let Some(t) = team {
        match reg.release_to_fa(id, t, pid) {
            Ok(_) => {
                // Skip the wBIz==0 sentinel row some rosters carry in the FA pool.
                let fa = reg
                    .list_free_agents(id)
                    .ok()
                    .and_then(|v| v.into_iter().find(|p| p.player_id != 0));
                match fa {
                    Some(fa) => {
                        let faname = format!("{} {}", fa.first_name, fa.last_name).trim().to_string();
                        match reg.sign_from_fa(id, fa.player_id, t) {
                            Ok(_) => steps.push(Step::ok(
                                "FA release + sign",
                                format!("released {pname}; signed free agent {faname} into the slot"),
                            )),
                            Err(e) => steps.push(Step::failed(
                                "FA release + sign",
                                format!("released {pname}, but sign failed: {e}"),
                            )),
                        }
                    }
                    None => steps.push(Step::ok(
                        "FA release + sign",
                        format!("released {pname}; no free agent available to sign back"),
                    )),
                }
            }
            Err(e) => steps.push(Step::skipped("FA release + sign", e.to_string())),
        }
    }

    steps
}

// ----------------------------------------------------------------------------- spec mode

/// Run an operator JSON spec instead of the smoke set. Entities resolve by resolved name
/// (`player:First Last`) or raw id (`id:123`); field names match the metadata `name`.
fn run_spec(reg: &Registry, id: u32, spec: &serde_json::Value) -> Vec<Step> {
    let mut steps = Vec::new();
    let players = reg.list_players(id).unwrap_or_default();
    let teams = reg.list_teams(id).unwrap_or_default();

    let resolve_player = |e: &str| -> Option<u64> {
        if let Some(idn) = e.strip_prefix("id:") {
            return idn.trim().parse().ok();
        }
        let name = e.strip_prefix("player:").unwrap_or(e).trim().to_ascii_lowercase();
        players
            .iter()
            .find(|p| format!("{} {}", p.first_name, p.last_name).trim().to_ascii_lowercase() == name)
            .map(|p| p.player_id)
    };
    let resolve_team = |e: &str| -> Option<u64> {
        if let Some(idn) = e.strip_prefix("id:") {
            return idn.trim().parse().ok();
        }
        let name = e.trim().to_ascii_lowercase();
        teams
            .iter()
            .find(|t| t.name.to_ascii_lowercase() == name || t.abbrev.to_ascii_lowercase() == name)
            .map(|t| t.team_id)
    };

    let Some(ops) = spec.as_array() else {
        steps.push(Step::failed("Spec", "top-level JSON is not an array of ops".into()));
        return steps;
    };

    for (i, op) in ops.iter().enumerate() {
        let kind = op.get("op").and_then(|v| v.as_str()).unwrap_or("");
        let label = format!("Spec[{i}] {kind}");
        match kind {
            "set_field" => {
                let entity = op.get("entity").and_then(|v| v.as_str()).unwrap_or("");
                let fname = op.get("field").and_then(|v| v.as_str()).unwrap_or("");
                let Some(pid) = resolve_player(entity) else {
                    steps.push(Step::failed(&label, format!("could not resolve entity {entity:?}")));
                    continue;
                };
                let Some(view) = reg.player_view(id, pid).ok().flatten() else {
                    steps.push(Step::failed(&label, "player detail unavailable".into()));
                    continue;
                };
                let field = view
                    .all_fields
                    .iter()
                    .chain(view.identity.iter())
                    .chain(view.ratings.iter())
                    .find(|f| f.name.eq_ignore_ascii_case(fname) || f.studio_field == fname);
                let Some(field) = field else {
                    steps.push(Step::failed(&label, format!("no field named {fname:?}")));
                    continue;
                };
                let record = if field.studio_table == view.ratings_table {
                    view.ratings_record
                } else {
                    Some(view.bio_record)
                };
                let Some(record) = record else {
                    steps.push(Step::failed(&label, "no companion record for this field".into()));
                    continue;
                };
                let value = match op.get("value") {
                    Some(v) if v.is_string() => FieldValue::Text(v.as_str().unwrap().to_string()),
                    Some(v) if v.is_i64() => FieldValue::Int(v.as_i64().unwrap()),
                    Some(v) if v.is_u64() => FieldValue::Int(v.as_u64().unwrap() as i64),
                    _ => {
                        steps.push(Step::failed(&label, "missing/invalid \"value\"".into()));
                        continue;
                    }
                };
                let table = field.studio_table.clone();
                let sfield = field.studio_field.clone();
                match reg.set_field(id, &table, record, &sfield, value) {
                    Ok(o) => steps.push(Step::ok(&label, format!("{fname} set ({} edit(s))", o.edits))),
                    Err(e) => steps.push(Step::failed(&label, e.to_string())),
                }
            }
            "set_jersey_number" => {
                let (Some(pid), Some(num)) = (
                    op.get("entity").and_then(|v| v.as_str()).and_then(&resolve_player),
                    op.get("value").and_then(|v| v.as_u64()),
                ) else {
                    steps.push(Step::failed(&label, "need entity + numeric value".into()));
                    continue;
                };
                let team = players.iter().find(|p| p.player_id == pid).and_then(|p| p.team_id);
                match team {
                    Some(t) => match reg.set_jersey_number(id, t, pid, num) {
                        Ok(o) => steps.push(Step::ok(&label, format!("→ #{num} ({} edit(s))", o.edits))),
                        Err(e) => steps.push(Step::failed(&label, e.to_string())),
                    },
                    None => steps.push(Step::failed(&label, "player not on a team".into())),
                }
            }
            "set_captaincy" => {
                let pid = op.get("player").and_then(|v| v.as_str()).and_then(&resolve_player);
                let tid = op.get("team").and_then(|v| v.as_str()).and_then(&resolve_team);
                let role = op.get("role").and_then(|v| v.as_u64()).unwrap_or(1);
                match (pid, tid) {
                    (Some(p), Some(t)) => match reg.set_captaincy(id, t, p, role) {
                        Ok(o) => steps.push(Step::ok(&label, format!("role {role} ({} edit(s))", o.edits))),
                        Err(e) => steps.push(Step::failed(&label, e.to_string())),
                    },
                    _ => steps.push(Step::failed(&label, "could not resolve team/player".into())),
                }
            }
            other => steps.push(Step::skipped(&label, format!("unsupported op {other:?}"))),
        }
    }
    steps
}

// ----------------------------------------------------------------------------- verification

struct Verify {
    files_written: Option<u64>,
    db_crc_ok: Option<bool>,
    db_size_ok: Option<bool>,
    roster_checksums_ok: Option<bool>,
    baseline_identical: Option<bool>,
    error: Option<String>,
}

/// Re-parse the saved main-DB artifact and check CRC/size; re-parse the roster container's checksums.
fn verify_artifacts(db: &Path, roster_container: Option<&Path>) -> Verify {
    let mut v = Verify {
        files_written: None,
        db_crc_ok: None,
        db_size_ok: None,
        roster_checksums_ok: None,
        baseline_identical: None,
        error: None,
    };
    match fs::read(db) {
        Ok(bytes) => match tdb_core::TdbFile::parse(&bytes) {
            Ok(f) => {
                v.db_crc_ok = Some(f.header_crc_ok());
                v.db_size_ok = Some(f.size_ok());
            }
            Err(e) => v.error = Some(format!("main DB re-parse failed: {e}")),
        },
        Err(e) => v.error = Some(format!("main DB unreadable: {e}")),
    }
    if let Some(rc) = roster_container {
        if let Ok(bytes) = fs::read(rc) {
            v.roster_checksums_ok = tdb_savedata::SaveContainer::parse(&bytes)
                .ok()
                .map(|c| c.checksums_valid());
        }
    }
    v
}

/// No-op baseline: open a fresh copy through the app path, save with zero edits, and assert the
/// written main DB is byte-identical to the anchor (the golden round-trip gate). Roster capacity
/// normalization can legitimately transform a packed roster save, so this is main-DB only.
fn baseline_identical(reg: &Registry, cell: &Cell, tmp: &Path, defs_root: &Path) -> Option<bool> {
    let anchor_bytes = fs::read(&cell.db).ok()?;
    let work = tmp.join(format!("{}-{}-baseline.db", cell.platform_dir, cell.game));
    fs::create_dir_all(tmp).ok()?;
    fs::copy(&cell.db, &work).ok()?;
    let summary = reg
        .open_workspace_from_paths(
            Some(&cell.game),
            cell.platform,
            &work.to_string_lossy(),
            None, // main DB only for the byte-equality gate
            &cell.meta.to_string_lossy(),
            &defs_root.to_string_lossy(),
        )
        .ok()?;
    let out = tmp.join(format!("{}-{}-baseline-out.db", cell.platform_dir, cell.game));
    reg.save(summary.project_id, &out.to_string_lossy()).ok()?;
    let written = fs::read(&out).ok()?;
    let _ = fs::remove_file(&work);
    let _ = fs::remove_file(&out);
    Some(written == anchor_bytes)
}

impl Verify {
    fn empty() -> Self {
        Verify {
            files_written: None,
            db_crc_ok: None,
            db_size_ok: None,
            roster_checksums_ok: None,
            baseline_identical: None,
            error: None,
        }
    }
}

/// Re-parse a saved PS4 roster `DATA` save: extract its inner LZ4 TDB and check CRC/size.
fn verify_ps4(data_path: &Path) -> Verify {
    let mut v = Verify::empty();
    match fs::read(data_path) {
        Ok(bytes) => match tdb_savedata::ps4_extract_roster(&bytes) {
            Ok(inner) => match tdb_core::TdbFile::parse(&inner) {
                Ok(f) => {
                    v.db_crc_ok = Some(f.header_crc_ok());
                    v.db_size_ok = Some(f.size_ok());
                }
                Err(e) => v.error = Some(format!("PS4 inner TDB re-parse failed: {e}")),
            },
            Err(e) => v.error = Some(format!("PS4 roster extract failed: {e}")),
        },
        Err(e) => v.error = Some(format!("PS4 DATA unreadable: {e}")),
    }
    v
}

/// PS4 semantic baseline: open a fresh copy of the anchor `DATA`, save with no edits, and confirm the
/// re-extracted inner TDB is byte-identical to the original (LZ4 recompression means the *container*
/// won't be byte-identical, so we compare the inner roster DB instead).
fn baseline_ps4(reg: &Registry, cell: &Cell, data_anchor: &Path, tmp: &Path, defs_root: &Path) -> Option<bool> {
    let orig_inner = tdb_savedata::ps4_extract_roster(&fs::read(data_anchor).ok()?).ok()?;
    fs::create_dir_all(tmp).ok()?;
    let work = tmp.join(format!("{}-{}-ps4-baseline-DATA", cell.platform_dir, cell.game));
    fs::copy(data_anchor, &work).ok()?;
    let summary = reg
        .open_workspace_from_paths(
            Some(&cell.game),
            cell.platform,
            &work.to_string_lossy(),
            None,
            &cell.meta.to_string_lossy(),
            &defs_root.to_string_lossy(),
        )
        .ok()?;
    let out = tmp.join(format!("{}-{}-ps4-baseline-out", cell.platform_dir, cell.game));
    reg.save(summary.project_id, &out.to_string_lossy()).ok()?;
    let out_inner = tdb_savedata::ps4_extract_roster(&fs::read(&out).ok()?).ok()?;
    let _ = fs::remove_file(&work);
    let _ = fs::remove_file(&out);
    Some(out_inner == orig_inner)
}

// ----------------------------------------------------------------------------- per-cell driver

struct CellResult {
    game: String,
    platform_dir: String,
    summary: Option<ProjectSummaryDto>,
    steps: Vec<Step>,
    verify: Verify,
    artifact_db: PathBuf,
    artifact_roster: Option<PathBuf>,
    roster_note: Option<String>,
    open_error: Option<String>,
}

/// PS4 with a writable roster save: edit the **standalone** LZ4 `DATA` save directly (no mirror; the
/// template `nhl_game.db` is used only for its metadata). Returns `None` if the DATA isn't a codec-
/// recognised PS4 save, so the caller falls back to opening the template db.
fn process_ps4_roster(
    reg: &Registry,
    cell: &Cell,
    args: &Args,
    tmp: &Path,
    roster_dir: &Path,
) -> Option<CellResult> {
    let cell_out = args.out.join("artifacts").join(&cell.platform_dir).join(&cell.game);
    let dir_name = roster_dir.file_name()?.to_string_lossy().into_owned();
    let dst_dir = cell_out.join("savefiles").join(&dir_name);
    copy_dir(roster_dir, &dst_dir).ok()?;
    let data = roster_container(&dst_dir)?;
    if !tdb_savedata::is_ps4_save(&fs::read(&data).ok()?) {
        return None; // not a recognised PS4 roster save — fall back to the template db.
    }

    let mut result = CellResult {
        game: cell.game.clone(),
        platform_dir: cell.platform_dir.clone(),
        summary: None,
        steps: Vec::new(),
        verify: Verify::empty(),
        artifact_db: data.clone(),
        artifact_roster: None,
        roster_note: None,
        open_error: None,
    };

    // Open the DATA save as the single edit session (auto-detected as a PS4 save; standalone).
    let opened = reg.open_workspace_from_paths(
        Some(&cell.game),
        cell.platform,
        &data.to_string_lossy(),
        None,
        &cell.meta.to_string_lossy(),
        &args.defs_root.to_string_lossy(),
    );
    let summary = match opened {
        Ok(s) => s,
        Err(e) => {
            result.open_error = Some(e.to_string());
            return Some(result);
        }
    };
    let id = summary.project_id;

    result.steps = match args.spec.as_ref().and_then(|p| fs::read_to_string(p).ok()) {
        Some(txt) => match serde_json::from_str::<serde_json::Value>(&txt) {
            Ok(spec) => run_spec(reg, id, &spec),
            Err(e) => vec![Step::failed("Spec parse", e.to_string())],
        },
        None => run_smoke(reg, id),
    };

    match reg.save_workspace(id) {
        Ok(n) => {
            result.verify = verify_ps4(&data);
            result.verify.files_written = Some(n);
        }
        Err(e) => result.verify.error = Some(format!("save failed: {e}")),
    }
    if let Some(anchor_data) = roster_container(roster_dir) {
        result.verify.baseline_identical = baseline_ps4(reg, cell, &anchor_data, tmp, &args.defs_root);
    }
    result.summary = Some(summary);
    Some(result)
}

fn process_cell(reg: &Registry, cell: &Cell, args: &Args, tmp: &Path) -> CellResult {
    // PS4: prefer editing the standalone roster DATA save when one is present.
    if cell.platform == Platform::Ps4 {
        if let Some(dir) = &cell.roster_dir {
            if let Some(res) = process_ps4_roster(reg, cell, args, tmp, dir) {
                return res;
            }
        }
        // No recognised roster save → fall through to the template nhl_game.db (browse/edit template).
    }

    let cell_out = args
        .out
        .join("artifacts")
        .join(&cell.platform_dir)
        .join(&cell.game);
    let db_name = cell.db.file_name().unwrap().to_string_lossy().into_owned();
    let artifact_db = cell_out.join("databases").join(&db_name);

    let mut result = CellResult {
        game: cell.game.clone(),
        platform_dir: cell.platform_dir.clone(),
        summary: None,
        steps: Vec::new(),
        verify: Verify {
            files_written: None,
            db_crc_ok: None,
            db_size_ok: None,
            roster_checksums_ok: None,
            baseline_identical: None,
            error: None,
        },
        artifact_db: artifact_db.clone(),
        artifact_roster: None,
        roster_note: None,
        open_error: None,
    };

    // Stage the editable files into the output tree (anchors stay pristine).
    if let Err(e) = copy_file(&cell.db, &artifact_db) {
        result.open_error = Some(format!("copy main DB: {e}"));
        return result;
    }
    let mut roster_container_out = None;
    if let Some(dir) = &cell.roster_dir {
        let dir_name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let dst_dir = cell_out.join("savefiles").join(&dir_name);
        if copy_dir(dir, &dst_dir).is_ok() {
            // Mirror the container-file resolution on the *copied* dir.
            if let Some(container) = roster_container(&dst_dir) {
                // Attach the roster mirror only when its container is *writable* — the exact gate
                // the app applies. NHL10/11 ship read-only v1 containers, and NHL07–09 ship
                // containers the codec doesn't recognise; in both cases we edit the main DB alone.
                let writable = fs::read(&container)
                    .ok()
                    .map(|b| {
                        tdb_savedata::SaveContainer::parse(&b)
                            .map(|c| c.writeable())
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);
                if writable {
                    roster_container_out = Some(container.clone());
                    result.artifact_roster = Some(container);
                } else {
                    result.roster_note = Some(
                        "a roster save is present but its container is read-only / unrecognised \
                         (NHL10/11 v1 or NHL07–09) — edited the main DB only, matching the app."
                            .to_string(),
                    );
                }
            }
        }
    }

    // Open through the exact app seam.
    let opened = reg.open_workspace_from_paths(
        Some(&cell.game),
        cell.platform,
        &artifact_db.to_string_lossy(),
        roster_container_out.as_ref().map(|p| p.to_string_lossy().into_owned()).as_deref(),
        &cell.meta.to_string_lossy(),
        &args.defs_root.to_string_lossy(),
    );
    let summary = match opened {
        Ok(s) => s,
        Err(e) => {
            result.open_error = Some(e.to_string());
            return result;
        }
    };
    let id = summary.project_id;

    // Apply edits (spec overrides the smoke set when supplied).
    result.steps = match args.spec.as_ref().and_then(|p| fs::read_to_string(p).ok()) {
        Some(txt) => match serde_json::from_str::<serde_json::Value>(&txt) {
            Ok(spec) => run_spec(reg, id, &spec),
            Err(e) => vec![Step::failed("Spec parse", e.to_string())],
        },
        None => run_smoke(reg, id),
    };

    // Save the edited artifacts in place, then verify.
    match reg.save_workspace(id) {
        Ok(n) => {
            result.verify = verify_artifacts(&artifact_db, roster_container_out.as_deref());
            result.verify.files_written = Some(n);
        }
        Err(e) => {
            result.verify.error = Some(format!("save failed: {e}"));
        }
    }
    result.verify.baseline_identical = baseline_identical(reg, cell, tmp, &args.defs_root);
    result.summary = Some(summary);
    result
}

// ----------------------------------------------------------------------------- reports

fn caps_line(c: &CapabilitiesDto) -> String {
    let mut on = Vec::new();
    let flag = |v: bool, name: &str| v.then(|| name.to_string());
    on.extend(flag(c.arena_link, "arena-link"));
    on.extend(flag(c.player_style, "player-style"));
    on.extend(flag(c.year_ufa, "year-UFA"));
    on.extend(flag(c.draft_team || c.draft_year || c.draft_round || c.draft_pick, "draft-fields"));
    on.extend(flag(c.has_skater_equip, "skater-equip"));
    on.extend(flag(c.has_goalie_equip, "goalie-equip"));
    format!(
        "ratings: {}/{} skater labelled, {}/{} goalie labelled · main-team offset {} · features: {}",
        c.skater_ratings_labelled,
        c.skater_ratings_total,
        c.goalie_ratings_labelled,
        c.goalie_ratings_total,
        c.main_team_offset,
        if on.is_empty() { "—".into() } else { on.join(", ") },
    )
}

fn tri(b: Option<bool>) -> &'static str {
    match b {
        Some(true) => "✓",
        Some(false) => "✗",
        None => "—",
    }
}

fn write_cell_report(dir: &Path, r: &CellResult, args: &Args) -> std::io::Result<PathBuf> {
    let mut md = String::new();
    let title = format!("{} — {}", r.game.to_uppercase(), r.platform_dir);
    md.push_str(&format!("# {title}\n\n"));

    if let Some(err) = &r.open_error {
        md.push_str(&format!("**Could not open this cell:** {err}\n"));
        let path = dir.join(format!("{}-{}.md", r.game, r.platform_dir));
        fs::write(&path, md)?;
        return Ok(path);
    }

    if let Some(s) = &r.summary {
        md.push_str(&format!(
            "- Source: `{}` · endian `{}` · {} tables · {} bytes\n",
            s.source, s.endian, s.table_count, s.db_size
        ));
        md.push_str(&format!(
            "- Roster save: {} (aligned: {})\n",
            if s.roster_attached { "attached" } else { "none" },
            if s.roster_aligned { "yes" } else { "n/a" }
        ));
        if let Some(note) = &r.roster_note {
            md.push_str(&format!("- Note: {note}\n"));
        }
        if s.write_experimental {
            md.push_str(
                "- ⚠️ **Experimental write path** — the edited save is structurally valid and round-trips, \
                 but its on-disk integrity tag is un-reversed (NHL10/11 `tag_1c`) or its on-console load \
                 is unverified (PS4). Confirm in-game before relying on it.\n",
            );
        }
        md.push_str(&format!("- Capabilities: {}\n\n", caps_line(&s.capabilities)));
    }

    md.push_str("## Edits\n\n");
    for st in &r.steps {
        md.push_str(&format!("- **{}** {} — {}\n", st.name, st.glyph(), st.detail));
        if let Some(t) = &st.technical {
            md.push_str(&format!("  <sub>ids: {t}</sub>\n"));
        }
    }

    md.push_str("\n## Verification\n\n");
    let v = &r.verify;
    if let Some(e) = &v.error {
        md.push_str(&format!("- **Error:** {e}\n"));
    }
    md.push_str(&format!(
        "- Files written: {}\n",
        v.files_written.map(|n| n.to_string()).unwrap_or_else(|| "—".into())
    ));
    md.push_str(&format!(
        "- Edited main DB: CRC {} · size {}\n",
        tri(v.db_crc_ok),
        tri(v.db_size_ok)
    ));
    md.push_str(&format!("- Roster container checksums: {}\n", tri(v.roster_checksums_ok)));
    md.push_str(&format!(
        "- Baseline no-op save byte-identical to anchor (main DB): {}\n",
        tri(v.baseline_identical)
    ));

    md.push_str("\n## Artifacts to apply\n\n");
    let rel = |p: &Path| {
        p.strip_prefix(&args.out)
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/")
    };
    md.push_str(&format!("- Primary edited file: `{}`\n", rel(&r.artifact_db)));
    if let Some(rp) = &r.artifact_roster {
        md.push_str(&format!("- Mirrored roster save: `{}`\n", rel(rp)));
    }
    md.push_str(
        "\n<sub>Anchors are read-only sources; these copies carry the edits above. \
         Apply them over your game install to verify in-engine.</sub>\n",
    );

    let path = dir.join(format!("{}-{}.md", r.game, r.platform_dir));
    fs::write(&path, md)?;
    Ok(path)
}

fn write_index(dir: &Path, results: &[CellResult]) -> std::io::Result<()> {
    let mut md = String::from("# Edit-QA matrix\n\n");
    md.push_str(&format!("{} cell(s) processed.\n\n", results.len()));
    md.push_str("| Title | Platform | Opened | Edits ok | Main DB CRC | Baseline == anchor | Report |\n");
    md.push_str("|---|---|---|---|---|---|---|\n");
    // Stable grouping by platform then title.
    let mut sorted: Vec<&CellResult> = results.iter().collect();
    sorted.sort_by(|a, b| {
        (a.platform_dir.as_str(), a.game.as_str()).cmp(&(b.platform_dir.as_str(), b.game.as_str()))
    });
    for r in sorted {
        let opened = if r.open_error.is_some() { "✗" } else { "✓" };
        let ok = r.steps.iter().filter(|s| matches!(s.status, Status::Ok | Status::Rejected)).count();
        let total = r.steps.len();
        let report = format!("{}-{}.md", r.game, r.platform_dir);
        md.push_str(&format!(
            "| {} | {} | {} | {}/{} | {} | {} | [{}]({}) |\n",
            r.game.to_uppercase(),
            r.platform_dir,
            opened,
            ok,
            total,
            tri(r.verify.db_crc_ok),
            tri(r.verify.baseline_identical),
            report,
            report,
        ));
    }
    fs::write(dir.join("index.md"), md)
}

// ----------------------------------------------------------------------------- main

fn main() {
    let args = parse_args();
    let cells = discover(&args);
    if cells.is_empty() {
        eprintln!(
            "No anchor cells matched under {:?} (games={:?}, platforms={:?}).",
            args.anchors_root, args.games, args.platforms
        );
        std::process::exit(1);
    }

    let reports_dir = args.out.join("reports");
    if let Err(e) = fs::create_dir_all(&reports_dir) {
        eprintln!("cannot create output dir {:?}: {e}", reports_dir);
        std::process::exit(1);
    }
    let baseline_tmp = args.out.join(".baseline-tmp");

    let reg = Registry::default();
    let mut results = Vec::new();
    let counts = cells.len();
    for (i, cell) in cells.iter().enumerate() {
        eprintln!("[{}/{counts}] {} / {}", i + 1, cell.platform_dir, cell.game);
        let r = process_cell(&reg, cell, &args, &baseline_tmp);
        match write_cell_report(&reports_dir, &r, &args) {
            Ok(p) => eprintln!("    report: {}", p.display()),
            Err(e) => eprintln!("    report write failed: {e}"),
        }
        results.push(r);
    }
    let _ = fs::remove_dir_all(&baseline_tmp);

    if let Err(e) = write_index(&reports_dir, &results) {
        eprintln!("index write failed: {e}");
    }

    let opened = results.iter().filter(|r| r.open_error.is_none()).count();
    eprintln!(
        "\nDone. {opened}/{counts} cells opened. Reports: {}  Artifacts: {}",
        reports_dir.display(),
        args.out.join("artifacts").display()
    );
}
