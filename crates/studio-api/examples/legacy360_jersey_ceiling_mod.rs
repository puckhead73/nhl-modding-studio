//! Jersey-ceiling probe, NHL Legacy (Xbox 360): build a `.nhlmod` through the production
//! project / package / installer APIs that packs the Anaheim Ducks' jersey list to the 5-bit
//! `jerseyindex` ceiling (32 kits) by re-pointing one existing kit from each of 23 other NHL teams
//! onto the Ducks. The donor rows keep their art id, so the Ducks gain the donor team's textures
//! under new jersey indexes 9..31. Only two cells change per donor row (`teamid`, `jerseyindex`).
//!
//! Usage: `WORK_DIR` — an empty or absent folder. The run creates `project/` (the Studio project),
//! `fake-install/` (a disposable Xbox install tree holding the bundled vanilla database), `records/`
//! (installer records), the built package and `receipt.json`. Install + restore are exercised on the
//! disposable tree only, to prove the package applies and uninstalls cleanly; nothing outside
//! `WORK_DIR` is written.
use std::path::{Path, PathBuf};

use studio_api::{Registry, dto::PackageBuildRequestDto};
use studio_domain::{BaselineStore, Platform, tdb_txn::FileIdentity};
use tdb_core::TdbFile;

const GAME: &str = "nhllegacy";
const TABLE: &str = "xjgw";
const TEAM: &str = "mCXg";
const INDEX: &str = "tYnx";
const STYLE: &str = "cVLR";
const ART_ID: &str = "GDQL";
const VARIANT: &str = "FccT";
const IS_DEFAULT: &str = "yFSM";
const DUCKS: u64 = 0;
const CEILING: u64 = 32;
/// Donor teams (0-indexed `teamid`): every NHL team from 2..=25 except 15, whose last-indexed kit
/// is a default home/away. Taking each donor's LAST index keeps the donor's own list contiguous,
/// and a non-default kit keeps its home/away defaults intact.
const DONORS: [u64; 23] = [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25];

#[derive(serde::Serialize, Clone)]
struct Row {
    record: u32,
    team: u64,
    index: u64,
    style: u64,
    art_id: u64,
    variant: u64,
    is_default: u64,
}

fn text(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn jersey_rows(db_path: &Path) -> Result<Vec<Row>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(db_path)?;
    let db = TdbFile::parse(&bytes)?;
    let t = db.table_canonical(TABLE).ok_or("no jersey table")?;
    let field = |id: &str| t.fields.iter().find(|f| f.id.canonical(db.endian) == id).ok_or(format!("no field {id}"));
    let (ft, fi, fs, fa, fv, fd) = (field(TEAM)?, field(INDEX)?, field(STYLE)?, field(ART_ID)?, field(VARIANT)?, field(IS_DEFAULT)?);
    let mut out = Vec::new();
    for r in 0..u32::from(t.current_records) {
        let g = |f| db.read_u64(t, r, f).ok_or("unreadable cell");
        out.push(Row { record: r, team: g(ft)?, index: g(fi)?, style: g(fs)?, art_id: g(fa)?, variant: g(fv)?, is_default: g(fd)? });
    }
    Ok(out)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: legacy360_jersey_ceiling_mod WORK_DIR".into());
    }
    let work = PathBuf::from(&args[1]);
    std::fs::create_dir_all(&work)?;
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let defs = text(&repo.join("defs"));
    let anchor_db = repo.join("anchors/xbox360/nhllegacy/databases/nhlng.db");
    let anchor_meta = repo.join("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml");
    let store = BaselineStore::open(&repo.join("baselines/INVENTORY.json"), &repo.join("anchors"))?;

    // A disposable Xbox install tree: default.xex marker + the vanilla database.
    let install = work.join("fake-install");
    std::fs::create_dir_all(install.join("db"))?;
    std::fs::write(install.join("default.xex"), b"placeholder: not a real executable")?;
    std::fs::copy(&anchor_db, install.join("db/nhlng.db"))?;
    std::fs::copy(&anchor_meta, install.join("db/nhlng-meta.xml"))?;
    let records = work.join("records");
    std::fs::create_dir_all(&records)?;

    let reg = Registry::default();
    let project_dir = work.join("project");
    let opened = reg.project_create_fresh(&text(&project_dir), GAME, Platform::Xbox, &text(&install), None, None, &store, &defs)?;
    let id = opened.summary.project_id;
    let working_db = PathBuf::from(opened.project.working_main_db.clone().ok_or("project has no working database")?);

    // Team identities, resolved by the Studio (never assumed from the id).
    let teams = reg.list_teams(id)?;
    let name_of = |t: u64| teams.iter().find(|x| x.team_id == t).map(|x| format!("{} ({})", x.name, x.abbrev)).unwrap_or_else(|| format!("team {t}"));
    let ducks = teams.iter().find(|t| t.team_id == DUCKS).ok_or("team 0 missing")?;
    if !ducks.name.to_ascii_lowercase().contains("duck") && !ducks.name.to_ascii_lowercase().contains("anaheim") {
        return Err(format!("team 0 is {:?}, not the Ducks; the plan assumes Anaheim is team 0", ducks.name).into());
    }
    for d in DONORS {
        let t = teams.iter().find(|x| x.team_id == d).ok_or(format!("donor team {d} missing"))?;
        if t.league_label.as_deref().is_some_and(|l| !l.to_ascii_uppercase().contains("NHL")) {
            return Err(format!("donor {} is in league {:?}, not the NHL", t.name, t.league_label).into());
        }
    }

    // Plan: the Ducks keep their stock kits; each donor contributes its last-indexed kit.
    let before = jersey_rows(&working_db)?;
    let ducks_stock: Vec<&Row> = before.iter().filter(|r| r.team == DUCKS).collect();
    let mut next_index = ducks_stock.iter().map(|r| r.index).max().map(|m| m + 1).unwrap_or(0);
    if ducks_stock.len() as u64 + DONORS.len() as u64 != CEILING {
        return Err(format!("the Ducks have {} stock kits + {} donors ≠ {CEILING}", ducks_stock.len(), DONORS.len()).into());
    }
    let mut moves = Vec::new();
    for d in DONORS {
        let donor = before.iter().filter(|r| r.team == d).max_by_key(|r| r.index).ok_or(format!("donor team {d} has no kits"))?;
        if donor.is_default != 0 {
            return Err(format!("donor row {} of {} is a default kit; refusing to move it", donor.record, name_of(d)).into());
        }
        moves.push((donor.clone(), next_index));
        next_index += 1;
    }
    assert_eq!(next_index, CEILING);

    // Stage: one structural operation per donor row (team + index), each undoable.
    for (row, new_index) in &moves {
        let out = reg.reassign_jersey(id, row.record, DUCKS, Some(*new_index))?;
        assert_eq!(out.edits, 2, "a move rewrites exactly the team and the index");
    }
    let pending = reg.pending_changes(id, 200)?;
    assert_eq!(pending.primary.total_cells as usize, moves.len() * 2, "two cells per moved kit");
    assert!(reg.project_save(id)?.saved);

    // Verify the saved working database.
    let after = jersey_rows(&working_db)?;
    let ducks_after: Vec<&Row> = after.iter().filter(|r| r.team == DUCKS).collect();
    assert_eq!(ducks_after.len() as u64, CEILING);
    let mut idx: Vec<u64> = ducks_after.iter().map(|r| r.index).collect();
    idx.sort_unstable();
    assert_eq!(idx, (0..CEILING).collect::<Vec<_>>(), "Ducks indexes are exactly 0..31");
    assert_eq!(after.len(), before.len(), "no rows added or removed");

    let built = reg.package_build(
        id,
        &PackageBuildRequestDto {
            name: "Ducks 32-jersey ceiling test".into(),
            version: "1".into(),
            description: format!(
                "NHL Legacy (Xbox 360) jersey-ceiling probe. Re-points the last kit of {} NHL teams onto the Anaheim Ducks (teamid 0) as jersey indexes 9..31, packing the Ducks to 32 kits — the 5-bit jerseyindex ceiling. Donor kits keep their art ids, so the Ducks show the donor teams' textures. No rows added; two cells per moved kit. Uninstall through the mod manager restores every cell.",
                DONORS.len()
            ),
            include: vec!["cells".into()],
            exclude: vec![],
            platforms: vec!["xbox".into()],
            out_path: Some(text(&work.join("Ducks-32-jersey-ceiling-test-1.nhlmod"))),
        },
        Some(&store),
        &defs,
    )?;
    let inspected = reg.package_inspect(&built.path)?;
    assert_eq!(inspected.cells.len(), moves.len() * 2);

    // Prove install + uninstall on the disposable tree.
    let dest = install.join("db/nhlng.db");
    let vanilla = FileIdentity::of_file(&dest)?;
    let plan = reg.package_preflight(&built.path, GAME, Platform::Xbox, &text(&install), None, None, false, &text(&records), &defs)?;
    if !plan.installable {
        return Err(format!("preflight refused: {:?} conflicts {:?}", plan.reasons, plan.conflicts).into());
    }
    let applied = reg.package_apply(plan.handle)?;
    assert_eq!(applied.state, "committed");
    let installed = jersey_rows(&dest)?;
    let installed_ducks = installed.iter().filter(|r| r.team == DUCKS).count() as u64;
    assert_eq!(installed_ducks, CEILING, "installed database carries 32 Ducks kits");
    assert_eq!(std::fs::read(&dest)?, std::fs::read(&working_db)?, "installed database equals the project's saved database");
    let restored = reg.package_restore(&applied.record_dir)?;
    assert_eq!(restored.state, "restored");
    let back = FileIdentity::of_file(&dest)?;
    assert_eq!(back.sha256, vanilla.sha256, "restore returns the vanilla database byte for byte");

    let controls: Vec<String> = [1u64, 15, 26, 27, 28, 29].iter().map(|t| name_of(*t)).collect();
    let receipt = serde_json::json!({
        "test": "NHL Legacy (Xbox 360): Ducks packed to 32 jerseys (jerseyindex 5-bit ceiling)",
        "package": { "path": built.path, "package_id": built.package_id, "sha256": built.sha256, "len": built.len },
        "project": text(&project_dir),
        "base_identity": inspected.base,
        "ducks": name_of(DUCKS),
        "ducks_stock_kits": ducks_stock.len(),
        "moves": moves.iter().map(|(r, ni)| serde_json::json!({
            "from_team": name_of(r.team), "from_team_id": r.team, "record": r.record,
            "old_index": r.index, "new_index": ni,
            "texture_id": format!("{}_{}_{}", r.style, r.art_id, r.variant),
        })).collect::<Vec<_>>(),
        "untouched_nhl_controls": controls,
        "pending_cells": pending.primary.total_cells,
        "install_probe": { "plan_applied": plan.applied, "apply_state": applied.state, "restore_state": restored.state, "vanilla_sha256": vanilla.sha256, "post_restore_sha256": back.sha256 },
        "game_verified": false,
        "instructions": "Install the .nhlmod on a vanilla NHL Legacy (Xbox 360) installation through the mod manager. In game, open Anaheim's jersey select: 32 entries loading and rendering (the last 23 show other teams' kits) means the 5-bit index ceiling is functional; a crash, a truncated list or missing textures past a given index is the real ceiling. Uninstall through the mod manager afterwards."
    });
    std::fs::write(work.join("receipt.json"), serde_json::to_vec_pretty(&receipt)?)?;
    println!("RESULT {}", serde_json::to_string(&receipt)?);
    reg.close_project(id);
    Ok(())
}
