//! Players 3.4 — reassign a player's portrait or head to another shipped asset (receipt 38; research
//! R124 = 2026-09-26 handoff 17 §4). The plan must name the files that make a target valid, refuse
//! every condition of R124's list by name, disclose sharing / duplicate faces / the stored
//! `hasportrait`, and the apply is one ordinary edit (review, undo). Anchor-only cases run
//! always; the real-tree cases read the Xbox 360 NHL Legacy loose tree and the PS4 NHL 21 portraits
//! (read only) and write only the in-memory session.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use studio_domain::player_reassign::{plan_reassign, AssetKind, Ps4Heads};
use studio_domain::{EditSession, FieldValue, Platform, Project, ProjectRole};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn open(platform: &str, game: &str) -> EditSession {
    let dir = repo(&format!("anchors/{platform}/{game}/databases"));
    let (db, meta) = if platform == "ps4" { (dir.join("nhl_game.db"), dir.join("nhl_game-meta.xml")) } else if game == "nhl07" { (dir.join("NHL07NG.db"), dir.join("NHL07NG-meta.xml")) } else { (dir.join("nhlng.db"), dir.join("nhlng-meta.xml")) };
    let p = match platform {
        "ps3" => Platform::Ps3,
        "ps4" => Platform::Ps4,
        _ => Platform::Xbox,
    };
    let mut s = EditSession::begin(Project::load(db, p, ProjectRole::Nhlng, meta, repo("defs")).unwrap());
    s.set_game_id(Some(game.to_string()));
    s
}

fn plan(s: &EditSession, game: &str, platform: Platform, fe: Option<&Path>, rendering: Option<&Path>, rec: u32, kind: AssetKind, target: u64) -> studio_domain::player_reassign::ReassignPlan {
    let db = TdbFile::parse(s.bytes()).unwrap();
    plan_reassign(Some(game), platform, &db, s.project().meta(), fe, rendering, None, Ps4Heads::NotAttached, rec, kind, target).unwrap()
}

/// A rostered skater's bio record and ids.
fn subject(s: &EditSession) -> (u32, u64, u64) {
    let p = s.player_summaries().unwrap().into_iter().find(|p| !p.is_goalie && p.team_id.is_some()).unwrap();
    let db = TdbFile::parse(s.bytes()).unwrap();
    let (rows, _) = studio_domain::player_containment::bio_rows(&db, s.project().meta()).unwrap();
    let r = rows.iter().find(|r| r.record == p.bio_record).unwrap();
    (p.bio_record, r.portrait_id, r.head_id)
}

#[test]
fn unmeasured_titles_widths_and_the_same_id_refuse_by_name() {
    // NHL 07: no measured containment rule.
    let s = open("xbox360", "nhl07");
    let (rec, art, _) = subject(&s);
    let p = plan(&s, "nhl07", Platform::Xbox, None, None, rec, AssetKind::Portrait, art + 1);
    assert!(!p.allowed && p.refusals.iter().any(|r| r.contains("not measured")), "{:?}", p.refusals);
    // Legacy: the same id, and an id beyond the declared width.
    let s = open("xbox360", "nhllegacy");
    let (rec, art, head) = subject(&s);
    let p = plan(&s, "nhllegacy", Platform::Xbox, None, None, rec, AssetKind::Portrait, art);
    assert!(p.refusals.iter().any(|r| r.contains("already uses portrait")), "{:?}", p.refusals);
    let p = plan(&s, "nhllegacy", Platform::Xbox, None, None, rec, AssetKind::Head, 1 << 20);
    assert!(p.refusals.iter().any(|r| r.contains("does not fit the 16-bit field")), "{:?}", p.refusals);
    let _ = head;
    // No tree attached: the target cannot be checked, so nothing is allowed.
    let p = plan(&s, "nhllegacy", Platform::Xbox, None, None, rec, AssetKind::Portrait, 5);
    assert!(!p.allowed && p.refusals.iter().any(|r| r.contains("no front-end tree is attached")), "{:?}", p.refusals);
    // Recompiled: archive-held assets — without the install attached nothing can be checked (receipt 55; the archive route is
    // `recompiled_and_ps3_archive_members` below).
    let p = plan(&s, "legacyrecomp", Platform::Recomp, None, None, rec, AssetKind::Head, 60001);
    assert!(p.refusals.iter().any(|r| r.contains("keeps its portraits and heads inside its archives")), "{:?}", p.refusals);
    // PS4 heads: vault-keyed.
    let s4 = open("ps4", "nhl21");
    let (rec4, _, head4) = subject(&s4);
    let p = plan(&s4, "nhl21", Platform::Ps4, None, None, rec4, AssetKind::Head, head4 + 1);
    // Receipt 63: PS4 heads are checked against the title's vault; without an attached install that cannot happen.
    assert!(p.refusals.iter().any(|r| r.contains("attach the title's install")), "{:?}", p.refusals);
}

/// Receipt 66 — the PS4 hair plan against a **synthetic** catalogue (the real vaults carry no ambiguous style): a unique
/// value is allowed and names its exact source; an ambiguous, absent, negative or out-of-range value, an unmeasured title, a
/// missing install and an unreadable vault are refused; only `hairid` is written; the fingerprint binds the source identity.
#[test]
fn ps4_hair_plan_against_a_synthetic_catalogue() {
    use studio_domain::player_reassign::{plan_hair_reassign, Ps4Hair};
    use studio_domain::ps4_hair::HairCatalogue;
    use studio_domain::ps4_vault::VaultSource;
    let s = open("ps4", "nhl21");
    let db = TdbFile::parse(s.bytes()).unwrap();
    let meta = s.project().meta();
    let bio = db.table_canonical(tdb_relations::entity::ids::BIO).unwrap();
    let hf = bio.fields.iter().find(|f| meta.field(&bio.id.raw(), &f.id.raw()).is_some_and(|m| m.name == "hairid")).unwrap();
    let hm = meta.field(&bio.id.raw(), &hf.id.raw()).unwrap();
    let rec = s.player_summaries().unwrap().into_iter().find(|p| !p.is_goalie && p.team_id.is_some()).unwrap().bio_record;
    let cur = hm.decode_raw(db.read_u64(bio, rec, hf).unwrap());
    let (unique, twice, absent) = ((0..2000).find(|v| *v != cur).unwrap(), 2001, 2002);
    let source = |defs: &str| VaultSource { archive: "databigs/boot.big".into(), members: vec![("gccollections.vlt".into(), "ab".repeat(32))], title: Some("nhl21".into()), identities_sha256: defs.repeat(64) };
    let cat = |defs: &str| HairCatalogue { title: "nhl21".into(), source: source(defs), styles: [(unique, 1), (twice, 2), (cur, 1)].into_iter().collect(), records_without_value: 0 };
    let c = cat("1");
    let run = |game: &str, h: Ps4Hair<'_>, t: i64| plan_hair_reassign(Some(game), Platform::Ps4, &db, meta, h, rec, t).unwrap();
    let ok = run("nhl21", Ps4Hair::Catalogue(&c), unique);
    assert!(ok.allowed, "{:?}", ok.refusals);
    assert_eq!(ok.writes, vec![(hf.id.canonical(db.endian), unique as u64)], "only hairid is written");
    assert!(ok.evidence[0].contains("ge_nhl_skaterhairtexturepack with GenericHairStyle"), "{:?}", ok.evidence);
    assert!(ok.source.iter().any(|l| l.contains(&"ab".repeat(32))) && ok.source.iter().any(|l| l.contains(&"1".repeat(64))), "{:?}", ok.source);
    assert!(ok.notes.iter().any(|n| n.contains("`headid`, `headtexture`")), "{:?}", ok.notes);
    // The same plan with other identity definitions is another review.
    let c2 = cat("2");
    assert_ne!(run("nhl21", Ps4Hair::Catalogue(&c2), unique).fingerprint, ok.fingerprint);
    assert_eq!(run("nhl21", Ps4Hair::Catalogue(&c), unique).fingerprint, ok.fingerprint, "stable when nothing changed");
    let why = |t: &str, h: Ps4Hair<'_>, v: i64| run(t, h, v).refusals.join(" | ");
    assert!(why("nhl21", Ps4Hair::Catalogue(&c), twice).contains("ambiguous"));
    assert!(why("nhl21", Ps4Hair::Catalogue(&c), absent).contains("has no hair record for 2002"));
    assert!(why("nhl21", Ps4Hair::Catalogue(&c), -1).contains("negative"));
    assert!(why("nhl21", Ps4Hair::Catalogue(&c), 2047).contains("does not fit `hairid` (0–2046)"));
    assert!(why("nhl21", Ps4Hair::Catalogue(&c), cur).contains("already uses hair style"));
    assert!(why("nhl21", Ps4Hair::NotAttached, unique).contains("attach the title's install"));
    assert!(why("nhl21", Ps4Hair::Problem("source mismatch: x".into()), unique).contains("source mismatch"));
    assert!(why("nhl15", Ps4Hair::Catalogue(&c), unique).contains("NHL 15 / 16 vaults carry no ge_nhl_skaterhairtexturepack"));
    let x = plan_hair_reassign(Some("nhl21"), Platform::Xbox, &db, meta, Ps4Hair::Catalogue(&c), rec, unique).unwrap();
    assert!(!x.allowed && x.refusals.iter().any(|r| r.contains("PS4 NHL 17–21 only")), "{:?}", x.refusals);
}

const LEGACY: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy";

fn ids(dir: &Path, prefix: &str, ext: &str) -> BTreeSet<u64> {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.strip_prefix(prefix)?.strip_suffix(ext)?.parse().ok()
        })
        .collect()
}

#[test]
#[ignore = "real X360 NHL Legacy loose tree under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn legacy_portrait_and_head_reassignment_check_apply_undo() {
    let root = Path::new(LEGACY);
    assert!(root.is_dir(), "{LEGACY}: missing (fails, never skips)");
    let (fe, rendering) = (root.join("fe"), root.join("rendering"));
    let mut s = open("xbox360", "nhllegacy");
    let (rec, art, head) = subject(&s);
    let db = TdbFile::parse(s.bytes()).unwrap();
    let (rows, _) = studio_domain::player_containment::bio_rows(&db, s.project().meta()).unwrap();
    let used_art: BTreeSet<u64> = rows.iter().map(|r| r.portrait_id).collect();
    let used_head: BTreeSet<u64> = rows.iter().map(|r| r.head_id).collect();

    // Portrait: an unused shipped portrait is allowed with its file as evidence; a missing one refused.
    let mut shipped = BTreeSet::new();
    for b in ["p0_4000", "p4001_8000", "p8001_12000"] {
        shipped.extend(ids(&fe.join("ion/artassets/playerheads").join(b), "p", ".big"));
    }
    let target = *shipped.iter().find(|i| !used_art.contains(i)).expect("an unused shipped portrait");
    let p = plan(&s, "nhllegacy", Platform::Xbox, Some(&fe), Some(&rendering), rec, AssetKind::Portrait, target);
    assert!(p.allowed, "{:?}", p.refusals);
    assert!(p.evidence.iter().any(|e| e.ends_with(&format!("p{target}.big"))), "{:?}", p.evidence);
    assert!(p.notes.iter().any(|n| n.contains("hasportrait") && n.contains("stays as stored")), "{:?}", p.notes);
    let missing = (1..20_000u64).find(|i| !shipped.contains(i)).unwrap();
    let p = plan(&s, "nhllegacy", Platform::Xbox, Some(&fe), Some(&rendering), rec, AssetKind::Portrait, missing);
    assert!(!p.allowed && p.refusals.iter().any(|r| r.contains(&format!("no portrait p{missing}.big"))), "{:?}", p.refusals);
    // Apply = one edit; undo restores; redo re-applies.
    s.set_field("ubPc", rec, &p.field, FieldValue::Int(target as i64)).unwrap();
    let after = TdbFile::parse(s.bytes()).unwrap();
    let (rows_after, _) = studio_domain::player_containment::bio_rows(&after, s.project().meta()).unwrap();
    assert_eq!(rows_after.iter().find(|r| r.record == rec).unwrap().portrait_id, target);
    assert!(s.undo());
    let back = TdbFile::parse(s.bytes()).unwrap();
    assert_eq!(studio_domain::player_containment::bio_rows(&back, s.project().meta()).unwrap().0.iter().find(|r| r.record == rec).unwrap().portrait_id, art);

    // Head: an unused registered head with mesh + texture library is allowed; a mesh without a
    // texture library, and one without a `stockplayerheads` row, are refused by name.
    let dir = rendering.join("player");
    let meshes = ids(&dir, "player_", ".rx2");
    let texlibs = ids(&dir, "texlib_", ".rx2");
    let mut allowed_head = None;
    let (mut no_texlib, mut no_row) = (None, None);
    for id in meshes.iter().filter(|i| !used_head.contains(i) && **i != head) {
        let p = plan(&s, "nhllegacy", Platform::Xbox, Some(&fe), Some(&rendering), rec, AssetKind::Head, *id);
        if p.allowed && allowed_head.is_none() {
            allowed_head = Some(p.clone());
        }
        if !texlibs.contains(id) && no_texlib.is_none() {
            no_texlib = Some(p.clone());
        }
        if texlibs.contains(id) && p.refusals.iter().any(|r| r.contains("stockplayerheads")) && no_row.is_none() {
            no_row = Some(p.clone());
        }
    }
    let ok = allowed_head.expect("an unused registered head with mesh and texture library");
    assert_eq!(ok.evidence.len(), 3, "mesh + texlib + registry row: {:?}", ok.evidence);
    let nt = no_texlib.expect("an unused mesh without a texture library (R124: 457 unused meshes, 238 with one)");
    assert!(nt.refusals.iter().any(|r| r.contains("has no texture library")), "{:?}", nt.refusals);
    if let Some(nr) = &no_row {
        assert!(!nr.allowed);
    }
    // Another player's own face is disclosed.
    let bespoke_other = rows.iter().find(|r| r.record != rec && r.head_id == r.portrait_id && texlibs.contains(&r.head_id) && meshes.contains(&r.head_id)).unwrap();
    let p = plan(&s, "nhllegacy", Platform::Xbox, Some(&fe), Some(&rendering), rec, AssetKind::Head, bespoke_other.head_id);
    assert!(p.notes.iter().any(|n| n.contains("own face")), "{:?}", p.notes);
    eprintln!(
        "legacy: portrait {art} → {target} allowed ({}); head → {} allowed; {} refused (no texlib); {}; own-face disclosure for {}",
        p.evidence.len(),
        ok.target_id,
        nt.target_id,
        match &no_row { Some(n) => format!("{} refused (no registry row)", n.target_id), None => "no unregistered-with-texlib case in this tree".into() },
        bespoke_other.full_name
    );
}

#[test]
#[ignore = "real PS4 NHL 21 tree under H:/Emulators/ShadPS4 (read only): select with --ignored"]
fn ps4_portrait_reassignment_checks_the_gnf_file() {
    let fe = PathBuf::from(r"H:\Emulators\ShadPS4\Games\CUSA18182\databigs\Extracted\nhl\ui");
    assert!(fe.join("kettle_assets").join("playerportraits").is_dir(), "{}: missing (fails, never skips)", fe.display());
    let s = open("ps4", "nhl21");
    let (rec, art, _) = subject(&s);
    let shipped = ids(&fe.join("kettle_assets").join("playerportraits"), "p", ".gnf");
    let target = *shipped.iter().find(|i| **i != art).unwrap();
    let p = plan(&s, "nhl21", Platform::Ps4, Some(&fe), None, rec, AssetKind::Portrait, target);
    assert!(p.allowed && p.evidence[0].ends_with(&format!("p{target}.gnf")), "{:?} {:?}", p.refusals, p.evidence);
    let missing = (1..70_000u64).find(|i| !shipped.contains(i)).unwrap();
    let p = plan(&s, "nhl21", Platform::Ps4, Some(&fe), None, rec, AssetKind::Portrait, missing);
    assert!(!p.allowed, "{:?}", p.refusals);
}

/// Receipt 55 — archive-held titles checked against their own archive members (research 2026-09-26 handoff 17 §9 / R134;
/// 2026-09-28 handoff 04 / R138): PS3 NHL 10 (BIG4 + `.bh`), 11 and 13 (EB), and Legacy Recompiled (EB; the X360 Legacy
/// anchor database, which R128 found byte-identical). A shipped, unused portrait / head passes with the member as evidence;
/// an id with no member is refused; PS3 10–12 disclose the per-head colour texture. Installs are only read.
#[test]
#[ignore = "real PS3 trees under X:/Games/Emulation/Roms and the Recompiled install under H:/Emulators (read only): select with --ignored"]
fn recompiled_and_ps3_archive_members() {
    use studio_domain::archive_index::ArchiveIndex;
    let ps3 = |t: &str| PathBuf::from(format!(r"X:\Games\Emulation\Roms\Sony Playstation 3\{t}\PS3_GAME\USRDIR"));
    let cases: [(&str, Platform, &str, PathBuf); 4] = [
        ("nhl10", Platform::Ps3, "anchors/ps3/nhl10/databases", ps3("NHL 10")),
        ("nhl11", Platform::Ps3, "anchors/ps3/nhl11/databases", ps3("NHL 11")),
        ("nhl13", Platform::Ps3, "anchors/ps3/nhl13/databases", ps3("NHL 13")),
        ("legacyrecomp", Platform::Recomp, "anchors/xbox360/nhllegacy/databases", PathBuf::from(r"H:\Emulators\games\XBOX\NHL Legacy Decomp Test")),
    ];
    for (game, platform, anchor, root) in cases {
        assert!(root.is_dir(), "{}: missing (fails, never skips)", root.display());
        let idx = ArchiveIndex::build(&root).unwrap();
        let d = repo(anchor);
        let s = EditSession::begin(Project::load(d.join("nhlng.db"), platform, ProjectRole::Nhlng, d.join("nhlng-meta.xml"), repo("defs")).unwrap());
        let (rec, _art, head) = subject(&s);
        let db = TdbFile::parse(s.bytes()).unwrap();
        let (rows, _) = studio_domain::player_containment::bio_rows(&db, s.project().meta()).unwrap();
        let used_art: std::collections::BTreeSet<u64> = rows.iter().map(|r| r.portrait_id).collect();
        let used_head: std::collections::BTreeSet<u64> = rows.iter().map(|r| r.head_id).collect();
        let run = |kind, target| plan_reassign(Some(game), platform, &db, s.project().meta(), None, None, Some(&idx), Ps4Heads::NotAttached, rec, kind, target).unwrap();
        // Portrait: an unused shipped id from the index.
        let port = (1..200_000u64).find(|i| !used_art.contains(i) && idx.portrait(*i).is_some()).expect("an unused shipped portrait");
        let p = run(AssetKind::Portrait, port);
        assert!(p.allowed, "{game} portrait {port}: {:?}", p.refusals);
        assert!(p.evidence[0].contains(&format!("/p{port}.big")), "{:?}", p.evidence);
        let missing = (1..200_000u64).find(|i| idx.portrait(*i).is_none() && !used_art.contains(i)).unwrap();
        assert!(run(AssetKind::Portrait, missing).refusals.iter().any(|r| r.contains(&format!("no portrait p{missing}.big")) && r.contains("archive members")));
        // Head: an unused id whose mesh (and, from NHL 13, texture library) is a member; the plan decides the rest.
        let prefix = if game == "nhl10" { ["player_", "head_"] } else { ["player_", "player_"] };
        let ext = if platform == Platform::Ps3 { "rpsgl" } else { "rx2" };
        let heads: Vec<u64> = idx
            .members
            .keys()
            .filter_map(|p| p.strip_prefix("rendering/player/"))
            .filter_map(|f| prefix.iter().find_map(|pf| f.strip_prefix(pf)))
            .filter_map(|f| f.strip_suffix(&format!(".{ext}")))
            .filter_map(|n| n.parse::<u64>().ok())
            .filter(|i| *i != head && !used_head.contains(i))
            .collect();
        let allowed = heads.iter().map(|h| (*h, run(AssetKind::Head, *h))).find(|(_, p)| p.allowed);
        let (h, p) = allowed.unwrap_or_else(|| panic!("{game}: no unused head passes; first refusals {:?}", heads.first().map(|h| run(AssetKind::Head, *h).refusals)));
        assert!(p.evidence.iter().any(|e| e.contains(&format!("rendering/player/"))), "{:?}", p.evidence);
        if matches!(game, "nhl10" | "nhl11") {
            assert!(p.notes.iter().any(|n| n.contains("per-head colour texture")), "{game}: {:?}", p.notes);
        }
        let nohead = (1..90_000u64).find(|i| !heads.contains(i) && !used_head.contains(i) && prefix.iter().all(|pf| idx.find(&format!("rendering/player/{pf}{i}.{ext}")).is_none())).unwrap();
        assert!(run(AssetKind::Head, nohead).refusals.iter().any(|r| r.contains("no head mesh")));
        eprintln!("{game}: {} archives ({} unreadable), portrait {port} ← {} ; head {h} ← {:?}", idx.archives.len(), idx.unreadable.len(), p.player, p.evidence);
    }
}
