//! Receipt 63 — PS4 head reassignment through the Registry (research R15 / R124 row 78), on disposable copies of the genuine
//! NHL 20 database (private fixture) and the NHL 21 and NHL 15 anchors, each checked against its own install's vault (read only).
//!
//! Covered: a shared-pool target and another player's bespoke target (disclosed as a duplicate face), an absent target, a
//! foreign vault (an NHL 19 project with the NHL 20 install), a missing `headtexture` field (metadata copy without it: refused,
//! nothing written), review invalidation (the fingerprint), the paired `headid` + `headtexture` write as one undo step with
//! redo, save to a copy and reopen, and NHL 15 (no `headtexture`: `headid` only; its 10391 has no vault record).
use std::path::{Path, PathBuf};

use studio_api::dto::{AssetRootsArgsDto, PlayerReassignPlanDto};
use studio_api::{parse_platform, Registry};
use studio_domain::FieldValue;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
fn install(cusa: &str) -> AssetRootsArgsDto {
    AssetRootsArgsDto { install_dir: format!(r"H:\Emulators\ShadPS4\Games\{cusa}"), platform: Some("ps4".into()), rendering_override: None, fe_override: None }
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-ps4-heads-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Tmp(d)
    }
    /// A disposable copy of a database (and its metadata, optionally rewritten).
    fn copy(&self, dir: &Path, rewrite_meta: Option<(&str, &str)>) -> (PathBuf, PathBuf) {
        let (db, meta) = (self.0.join("nhl_game.db"), self.0.join("nhl_game-meta.xml"));
        std::fs::copy(dir.join("nhl_game.db"), &db).unwrap();
        let m = std::fs::read_to_string(dir.join("nhl_game-meta.xml")).unwrap();
        std::fs::write(&meta, match rewrite_meta {
            Some((a, b)) => m.replace(a, b),
            None => m,
        })
        .unwrap();
        (db, meta)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn open(reg: &Registry, game: &str, db: &Path, meta: &Path) -> u32 {
    reg.open_workspace_from_paths(Some(game), parse_platform("ps4").unwrap(), &s(db), None, &s(meta), &s(&repo("defs"))).unwrap().project_id
}

/// (bio record, headid, artid, headtexture) of every skater row, through the public player view.
fn rows(reg: &Registry, id: u32) -> Vec<(u64, u32, i64, i64, Option<i64>)> {
    reg.list_players(id)
        .unwrap()
        .into_iter()
        .filter(|p| !p.is_goalie)
        .take(150)
        .map(|p| {
            let v = reg.player_view(id, p.player_id).unwrap().unwrap();
            let get = |n: &str| v.all_fields.iter().find(|f| f.name == n).and_then(|f| f.decoded);
            (p.player_id, v.bio_record, get("headid").unwrap(), get("artid").unwrap(), get("headtexture"))
        })
        .collect()
}

fn plan(reg: &Registry, id: u32, args: &AssetRootsArgsDto, rec: u32, target: u64) -> PlayerReassignPlanDto {
    reg.player_reassign(id, args, rec, "head", target as i64, false, None, &repo("defs")).unwrap()
}
fn apply(reg: &Registry, id: u32, args: &AssetRootsArgsDto, rec: u32, target: u64, fp: Option<&str>) -> Result<PlayerReassignPlanDto, String> {
    reg.player_reassign(id, args, rec, "head", target as i64, true, fp, &repo("defs")).map_err(|e| e.to_string())
}
fn refused(r: Result<PlayerReassignPlanDto, String>) -> String {
    match r {
        Ok(_) => panic!("expected a refusal"),
        Err(e) => e,
    }
}
fn cells(reg: &Registry, id: u32, player: u64) -> (i64, Option<i64>) {
    let v = reg.player_view(id, player).unwrap().unwrap();
    let get = |n: &str| v.all_fields.iter().find(|f| f.name == n).and_then(|f| f.decoded);
    (get("headid").unwrap(), get("headtexture"))
}

#[test]
#[ignore = "genuine NHL 20 fixture (NHL_STUDIO_FIXTURES) and PS4 installs under H:/Emulators/ShadPS4/Games (read only): select with --ignored"]
fn ps4_heads_reassign_against_the_titles_own_vault() {
    let fx = PathBuf::from(std::env::var("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES (fails, never skips)")).join("databases/nhl20-ps4-cusa15439/inxp_00.big");
    for (game, dbdir, cusa) in [("nhl20", fx.clone(), "CUSA15439"), ("nhl21", repo("anchors/ps4/nhl21/databases"), "CUSA18182")] {
        let t = Tmp::new(game);
        let (db, meta) = t.copy(&dbdir, None);
        let reg = Registry::default();
        let id = open(&reg, game, &db, &meta);
        let args = install(cusa);
        let all = rows(&reg, id);
        let exceptions = all.iter().filter(|r| r.4 != Some(r.2)).count();
        let (player, rec, head, art, _) = *all.iter().find(|r| r.2 == r.3 && r.4 == Some(r.2)).expect("a bespoke skater with the stock pairing");
        // Another player's bespoke head, and a pool head, both shipped by this title's vault.
        let owner = all.iter().find(|r| r.1 != rec && r.2 == r.3 && r.2 < 60_000 && r.2 != head).expect("another bespoke skater");
        let p = plan(&reg, id, &args, rec, owner.2 as u64);
        assert!(p.allowed, "{game}: {:?}", p.refusals);
        assert_eq!(p.written_fields.len(), 2, "{game}: headid and headtexture together");
        assert!(p.evidence[0].contains("SkaterNameAttribute") && p.evidence[0].contains(&format!("{game} vault")), "{:?}", p.evidence);
        assert!(p.notes.iter().any(|n| n.contains("own face — two players would share it")), "{:?}", p.notes);
        assert!(p.notes.iter().any(|n| n.contains("'s own bespoke head")), "{:?}", p.notes);
        // Pool and absent targets picked from the title's own catalogue (the same identity-checked vault the plan reads).
        let cat = studio_domain::ps4_heads::HeadCatalogue::load(Path::new(&args.install_dir), &repo("defs"), game).unwrap();
        let pool = *cat.heads.keys().find(|h| **h >= 60_000).expect("a shipped pool head") as u64;
        assert!(plan(&reg, id, &args, rec, pool).allowed);
        let pp = plan(&reg, id, &args, rec, pool);
        assert!(pp.notes.iter().any(|n| n.contains("shared pool head")) && pp.notes.iter().any(|n| n.contains("becomes not bespoke")), "{:?}", pp.notes);
        // Absent: a number the vault does not carry.
        let absent = (pool + 1..65_535).find(|h| !cat.heads.contains_key(&(*h as i64))).unwrap();
        let pa = plan(&reg, id, &args, rec, absent);
        assert!(pa.refusals.iter().any(|r| r.contains("has no head record")), "{:?}", pa.refusals);
        assert!(refused(apply(&reg, id, &args, rec, absent, Some(&pa.fingerprint))).contains("not applied"));

        // Apply needs the reviewed fingerprint; a changed row invalidates it.
        assert!(refused(apply(&reg, id, &args, rec, pool, None)).contains("check the reassignment first"));
        let tex_field = reg.player_view(id, player).unwrap().unwrap().all_fields.iter().find(|f| f.name == "headtexture").unwrap().studio_field.clone();
        reg.set_field(id, "ubPc", rec, &tex_field, FieldValue::Int(art + 1)).unwrap();
        let e = refused(apply(&reg, id, &args, rec, pool, Some(&pp.fingerprint)));
        assert!(e.contains("differs from the one reviewed") && e.contains("Nothing was written"), "{e}");
        assert_eq!(cells(&reg, id, player), (head, Some(art + 1)), "the stale apply wrote nothing");
        assert!(reg.undo(id).unwrap());
        // The exceptional row is disclosed, not silently normalized.
        reg.set_field(id, "ubPc", rec, &tex_field, FieldValue::Int(art + 1)).unwrap();
        assert!(plan(&reg, id, &args, rec, pool).notes.iter().any(|n| n.contains("an exception to the stock pairing")));
        assert!(reg.undo(id).unwrap());

        // The reviewed apply: both cells, one undo step, redo, save / reopen.
        let pp = plan(&reg, id, &args, rec, pool);
        let r = apply(&reg, id, &args, rec, pool, Some(&pp.fingerprint)).unwrap();
        assert!(r.applied.is_some());
        assert_eq!(cells(&reg, id, player), (pool as i64, Some(pool as i64)));
        assert!(reg.undo(id).unwrap());
        assert_eq!(cells(&reg, id, player), (head, Some(head)), "one undo restores both");
        assert!(reg.redo(id).unwrap());
        assert_eq!(cells(&reg, id, player), (pool as i64, Some(pool as i64)));
        let out = t.0.join("saved.db");
        reg.save(id, &s(&out)).unwrap();
        let id2 = open(&reg, game, &out, &meta);
        assert_eq!(cells(&reg, id2, player), (pool as i64, Some(pool as i64)), "both survive save / reopen");
        eprintln!("{game}: {} skaters read, {exceptions} with headtexture ≠ headid; {} bespoke → pool {pool}; bespoke target {} ({}), absent {absent}", all.len(), head, owner.2, p.player);
    }

    // Missing paired field: an NHL 20 copy whose metadata does not name `headtexture` — refused, nothing written.
    let t = Tmp::new("nofield");
    let (db, meta) = t.copy(&fx, Some(("name=\"headtexture\"", "name=\"headtexture_unnamed\"")));
    let reg = Registry::default();
    let id = open(&reg, "nhl20", &db, &meta);
    let args = install("CUSA15439");
    let (player, rec, head, _, _) = rows(&reg, id).into_iter().find(|r| r.2 == r.3).unwrap();
    let p = plan(&reg, id, &args, rec, 60_001);
    assert!(p.refusals.iter().any(|r| r.contains("no `headtexture` field")), "{:?}", p.refusals);
    assert!(apply(&reg, id, &args, rec, 60_001, Some(&p.fingerprint)).is_err());
    assert_eq!(cells(&reg, id, player).0, head);
    assert!(!reg.pending_changes(id, 10).unwrap().dirty);

    // Foreign vault: an NHL 19 project with the NHL 20 install is refused as a source mismatch.
    let t = Tmp::new("foreign");
    let (db, meta) = t.copy(&repo("anchors/ps4/nhl19/databases"), None);
    let reg = Registry::default();
    let id = open(&reg, "nhl19", &db, &meta);
    let (_, rec, head, _, _) = rows(&reg, id)[0];
    let p = plan(&reg, id, &install("CUSA15439"), rec, head as u64 + 1);
    assert!(p.refusals.iter().any(|r| r.contains("source mismatch")), "{:?}", p.refusals);

    // NHL 15: no `headtexture` — `headid` only; 10391 has no head record in its vault (R15).
    let t = Tmp::new("nhl15");
    let (db, meta) = t.copy(&repo("anchors/ps4/nhl15/databases"), None);
    let reg = Registry::default();
    let id = open(&reg, "nhl15", &db, &meta);
    let args = install("CUSA00561");
    let all = rows(&reg, id);
    let (player, rec, head, _, tex) = all.iter().copied().find(|r| r.2 != 10_391).unwrap();
    assert_eq!(tex, None, "NHL 15 has no headtexture field");
    let p = plan(&reg, id, &args, rec, 10_391);
    assert!(p.refusals.iter().any(|r| r.contains("has no head record for 10391")), "{:?}", p.refusals);
    let cat = studio_domain::ps4_heads::HeadCatalogue::load(Path::new(&args.install_dir), &repo("defs"), "nhl15").unwrap();
    assert_eq!(cat.records_for(10_391), 0, "R15: NHL 15 ships no head 10391");
    let target = all.iter().map(|r| r.2 as u64).find(|h| *h != head as u64 && cat.records_for(*h as i64) == 1).expect("a shipped head");
    let p = plan(&reg, id, &args, rec, target);
    assert_eq!(p.written_fields.len(), 1, "headid only");
    apply(&reg, id, &args, rec, target, Some(&p.fingerprint)).unwrap();
    assert_eq!(cells(&reg, id, player).0, target as i64);
    if let Some(r) = all.iter().find(|r| r.2 == 10_391) {
        let n = plan(&reg, id, &args, r.1, target).notes;
        assert!(n.iter().any(|x| x.contains("has no record in this title's vault")), "{n:?}");
    }
}
