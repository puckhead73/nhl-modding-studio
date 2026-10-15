//! Receipt 66 — PS4 hair reassignment through the Registry (research R15: `ge_nhl_skaterhairtexturepack`'s `GenericHairStyle`
//! value = the database `hairid`), on disposable copies of the genuine NHL 20 database (private fixture — **not** the
//! `anchors/ps4/nhl20` files, which are NHL 19 base) and the NHL 21 anchor, each checked against its own install's vault (read
//! only).
//!
//! Covered per title: a valid existing target (only `hairid` written; `headid` / `headtexture` untouched), an absent target, the
//! vault carrying no ambiguous value (the ambiguous refusal is the synthetic domain case), a negative and an out-of-range value,
//! review invalidation, one apply / undo / redo, save to a copy and reopen. Plus: a foreign vault (NHL 21 project, NHL 20
//! install), a missing `hairid` field (metadata copy without it) and NHL 15 (no measured hair class) — nothing written.
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
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
fn sha(p: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(p).unwrap()))
}

/// An exclusive per-run scratch folder (created new, never cleared from before), removed on drop.
struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("nhlms-r66-hair-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir(&d).expect("an exclusive scratch folder");
        Tmp(d)
    }
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
fn hair(reg: &Registry, id: u32, args: &AssetRootsArgsDto, rec: u32, target: i64, apply: Option<&str>) -> Result<PlayerReassignPlanDto, String> {
    reg.player_reassign(id, args, rec, "hair", target, apply.is_some(), apply, &repo("defs")).map_err(|e| e.to_string())
}
/// (bio record, headid, headtexture, hairid) of a player, through the public player view.
fn cells(reg: &Registry, id: u32, player: u64) -> (u32, i64, Option<i64>, Option<i64>) {
    let v = reg.player_view(id, player).unwrap().unwrap();
    let get = |n: &str| v.all_fields.iter().find(|f| f.name == n).and_then(|f| f.decoded);
    (v.bio_record, get("headid").unwrap(), get("headtexture"), get("hairid"))
}

#[test]
#[ignore = "genuine NHL 20 fixture (NHL_STUDIO_FIXTURES) and PS4 installs under H:/Emulators/ShadPS4/Games (read only): select with --ignored"]
fn ps4_hair_reassign_against_the_titles_own_vault() {
    let fx = PathBuf::from(std::env::var("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES (fails, never skips)")).join("databases/nhl20-ps4-cusa15439/inxp_00.big");
    for (game, dbdir, cusa) in [("nhl20", fx.clone(), "CUSA15439"), ("nhl21", repo("anchors/ps4/nhl21/databases"), "CUSA18182")] {
        let t = Tmp::new(game);
        let (db, meta) = t.copy(&dbdir, None);
        let db_sha = sha(&db);
        let reg = Registry::default();
        let id = open(&reg, game, &db, &meta);
        let args = install(cusa);
        let cat = studio_domain::ps4_hair::HairCatalogue::load(Path::new(&args.install_dir), &repo("defs"), game).unwrap();
        assert_eq!(cat.styles.values().filter(|n| **n > 1).count(), 0, "{game}: the shipped vault carries no ambiguous style");
        let player = reg.list_players(id).unwrap().into_iter().find(|p| !p.is_goalie && p.team_id.is_some()).unwrap().player_id;
        let (rec, head, tex, cur) = cells(&reg, id, player);
        let cur = cur.expect("a hairid field");
        let target = *cat.styles.keys().find(|h| **h != cur).unwrap();

        // Valid existing target: only hairid, evidence names the class, attribute and this title's vault; exact hashes.
        let p = hair(&reg, id, &args, rec, target, None).unwrap();
        assert!(p.allowed, "{game}: {:?}", p.refusals);
        assert_eq!(p.kind, "hair");
        assert_eq!(p.written_fields.len(), 1, "{game}: hairid alone");
        assert!(p.evidence[0].contains("ge_nhl_skaterhairtexturepack with GenericHairStyle") && p.evidence[0].contains(&format!("{game} vault")), "{:?}", p.evidence);
        assert_eq!(p.source_identity.len(), 5, "{:?}", p.source_identity);
        assert!(p.notes.iter().any(|n| n.contains("helmet compatibility")), "{:?}", p.notes);
        // Absent, negative and out-of-range values: refused, and an apply of them writes nothing.
        let absent = (0..=2046).find(|v| !cat.styles.contains_key(v)).unwrap();
        for (v, why) in [(absent, "has no hair record"), (-1, "negative"), (2047, "does not fit `hairid` (0–2046)")] {
            let r = hair(&reg, id, &args, rec, v, None).unwrap();
            assert!(!r.allowed && r.refusals.iter().any(|x| x.contains(why)), "{game} {v}: {:?}", r.refusals);
            assert!(hair(&reg, id, &args, rec, v, Some(&r.fingerprint)).err().expect("refused").contains("not applied"));
        }
        assert!(hair(&reg, id, &args, rec, target, None).is_ok_and(|r| r.fingerprint == p.fingerprint), "stable review");
        assert!(hair(&reg, id, &args, rec, target, Some("")).err().expect("refused").contains("differs from the one reviewed"));
        assert_eq!(cells(&reg, id, player), (rec, head, tex, Some(cur)));
        assert!(!reg.pending_changes(id, 10).unwrap().dirty);

        // Review invalidation: the written cell changes after review.
        let hair_field = reg.player_view(id, player).unwrap().unwrap().all_fields.iter().find(|f| f.name == "hairid").unwrap().studio_field.clone();
        reg.set_field(id, "ubPc", rec, &hair_field, FieldValue::Int(absent)).unwrap();
        assert!(hair(&reg, id, &args, rec, target, Some(&p.fingerprint)).err().expect("refused").contains("Nothing was written"));
        assert_eq!(cells(&reg, id, player).3, Some(absent), "the stale apply wrote nothing");
        // A stored value without a vault record is disclosed as a stock exception, not normalized.
        let q = hair(&reg, id, &args, rec, target, None).unwrap();
        assert!(q.notes.iter().any(|n| n.contains("stock exception")), "{:?}", q.notes);
        assert!(reg.undo(id).unwrap());

        // The reviewed apply: one cell, one undo step, redo, save / reopen; head fields untouched throughout.
        let p = hair(&reg, id, &args, rec, target, None).unwrap();
        let r = hair(&reg, id, &args, rec, target, Some(&p.fingerprint)).unwrap();
        assert!(r.applied.is_some());
        assert_eq!(cells(&reg, id, player), (rec, head, tex, Some(target)));
        assert!(reg.undo(id).unwrap());
        assert_eq!(cells(&reg, id, player), (rec, head, tex, Some(cur)), "one undo restores it");
        assert!(reg.redo(id).unwrap());
        let out = t.0.join("saved.db");
        reg.save(id, &s(&out)).unwrap();
        let id2 = open(&reg, game, &out, &meta);
        assert_eq!(cells(&reg, id2, player), (rec, head, tex, Some(target)), "survives save / reopen");
        let boot = Path::new(&args.install_dir).join("databigs").join("boot.big");
        eprintln!(
            "{game} PS4 {cusa}: db copy sha256 {db_sha}; boot.big sha256 {}; {} styles (0 ambiguous); {} hair {cur} → {target}; absent {absent}",
            sha(&boot),
            cat.styles.len(),
            p.player
        );
    }

    // Foreign vault: an NHL 21 project with the NHL 20 install.
    let t = Tmp::new("foreign");
    let (db, meta) = t.copy(&repo("anchors/ps4/nhl21/databases"), None);
    let reg = Registry::default();
    let id = open(&reg, "nhl21", &db, &meta);
    let player = reg.list_players(id).unwrap().into_iter().find(|p| !p.is_goalie).unwrap().player_id;
    let (rec, ..) = cells(&reg, id, player);
    let p = hair(&reg, id, &install("CUSA15439"), rec, 1, None).unwrap();
    assert!(p.refusals.iter().any(|r| r.contains("source mismatch") && r.contains("nhl20")), "{:?}", p.refusals);

    // Missing field: an NHL 20 copy whose metadata does not name `hairid`.
    let t = Tmp::new("nofield");
    let (db, meta) = t.copy(&fx, Some(("name=\"hairid\"", "name=\"hairid_unnamed\"")));
    let reg = Registry::default();
    let id = open(&reg, "nhl20", &db, &meta);
    let player = reg.list_players(id).unwrap().into_iter().find(|p| !p.is_goalie).unwrap().player_id;
    let (rec, ..) = cells(&reg, id, player);
    let p = hair(&reg, id, &install("CUSA15439"), rec, 1, None).unwrap();
    assert!(p.refusals.iter().any(|r| r.contains("no `hairid` field")), "{:?}", p.refusals);
    assert!(hair(&reg, id, &install("CUSA15439"), rec, 1, Some(&p.fingerprint)).is_err());
    assert!(!reg.pending_changes(id, 10).unwrap().dirty);

    // NHL 15: no measured hair class — refused without reading its vault.
    let t = Tmp::new("nhl15");
    let (db, meta) = t.copy(&repo("anchors/ps4/nhl15/databases"), None);
    let reg = Registry::default();
    let id = open(&reg, "nhl15", &db, &meta);
    let player = reg.list_players(id).unwrap().into_iter().find(|p| !p.is_goalie).unwrap().player_id;
    let (rec, ..) = cells(&reg, id, player);
    let p = hair(&reg, id, &install("CUSA00561"), rec, 1, None).unwrap();
    assert!(!p.allowed && p.refusals.iter().any(|r| r.contains("NHL 15 / 16")), "{:?}", p.refusals);
}
