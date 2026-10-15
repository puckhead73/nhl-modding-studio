//! D8 — a texture member inside a packed PS4 archive replaced through the [`Registry`] seam:
//! preview (plan held under a ticket), apply as a pending replacement, the pending review listing
//! it, undo, and the ordinary copied-target review staging the archive itself (`archive-asset-0`)
//! — a deduplicated member takes a **private copy** (appended; only its row re-pointed; the sibling
//! keeps the shared bytes) — then apply, reopen and restore byte-exact. A synthetic archive in the
//! stager's qualified layout with PNG members (the NHL 15 shape); the real GNF member runs
//! `--ignored` on the private fixture. Nothing here is evidence a game accepts the result.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};
use studio_domain::tdb_txn::FileIdentity;
use tdb_archive_big::eb::FILE_TABLE_OFFSET;
use tdb_archive_big::stage::synth::build_archive;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-d8-api-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn png(w: u32, h: u32, seed: u8) -> Vec<u8> {
    let rgba: Vec<u8> = (0..w * h).flat_map(|i| [(i % w) as u8 ^ seed, (i / w) as u8, seed, 255]).collect();
    studio_domain::DecodedImage { width: w, height: h, rgba }.to_png().unwrap()
}

const L0: &str = "nhl/ui/assets/leaguelogos/size64/l0.png";
const L5: &str = "nhl/ui/assets/leaguelogos/size64/l5.png";
const L100: &str = "nhl/ui/assets/leaguelogos/size64/l100.png";

/// A copied-target root with the NHL 15 anchor database and a `databigs/inxp10.big` holding three
/// raw PNG logo members, `l100` pointed at `l5`'s block (a shared span, as shipped).
fn build_root(tmp: &Tmp) -> (PathBuf, PathBuf, PathBuf) {
    let root = tmp.0.join("copy");
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let db_dir = root.join("nhl/data/common/gamemodes/database/t3db");
    std::fs::create_dir_all(&db_dir).unwrap();
    let db = db_dir.join("nhl_game.db");
    std::fs::copy(repo("anchors/ps4/nhl15/databases/nhl_game.db"), &db).unwrap();
    let files: Vec<(u16, &str, Vec<u8>)> = vec![
        (1, "first.bin", b"FIRST".to_vec()),
        (0, "l0.png", png(64, 64, 1)),
        (0, "l5.png", png(64, 64, 5)),
        (0, "l100.png", png(64, 64, 100)),
        (1, "last.bin", vec![0xAB; 64]),
    ];
    let mut bytes = build_archive(&["nhl/ui/assets/leaguelogos/size64", "nhl/data/other"], &files);
    // Row 3 (l100) takes row 2's block and size: one shared span.
    let r2 = FILE_TABLE_OFFSET + 2 * 16;
    let r3 = FILE_TABLE_OFFSET + 3 * 16;
    let row2 = bytes[r2..r2 + 12].to_vec();
    bytes[r3..r3 + 12].copy_from_slice(&row2);
    std::fs::create_dir_all(root.join("databigs")).unwrap();
    let archive = root.join("databigs/inxp10.big");
    std::fs::write(&archive, &bytes).unwrap();
    (root, db, archive)
}

fn open(reg: &Registry, db: &Path) -> u32 {
    reg.open_workspace_from_paths(
        Some("nhl15"),
        parse_platform("ps4").unwrap(),
        &s(db),
        None,
        &s(&repo("anchors/ps4/nhl15/databases/nhl_game-meta.xml")),
        &s(&repo("defs")),
    )
    .unwrap()
    .project_id
}

fn member_png(archive: &Path, path: &str) -> Vec<u8> {
    let arc = tdb_archive_big::eb::EbArchive::parse(&std::fs::read(archive).unwrap()).unwrap();
    let e = arc.entries.iter().find(|e| e.path == path).unwrap();
    let all = std::fs::read(archive).unwrap();
    all[e.offset as usize..e.offset as usize + e.size as usize].to_vec()
}

#[test]
fn a_member_replacement_flows_through_preview_apply_pending_and_the_copied_target_save_with_a_private_copy() {
    let tmp = Tmp::new("flow");
    let (root, db, archive) = build_root(&tmp);
    let original_archive = std::fs::read(&archive).unwrap();
    let original_db = FileIdentity::of_file(&db).unwrap();
    let source = tmp.0.join("new-l0.png");
    std::fs::write(&source, png(64, 64, 77)).unwrap();
    let source_shared = tmp.0.join("new-l100.png");
    std::fs::write(&source_shared, png(64, 64, 42)).unwrap();
    let reg = Registry::default();
    let id = open(&reg, &db);
    let root_s = s(&root);

    // Preview l0 (its own slot) and apply.
    let p = reg.asset_member_preview(id, &root_s, "databigs/inxp10.big", L0, &s(&source)).unwrap();
    let plan = p.plan.unwrap_or_else(|| panic!("{:?} {:?}", p.refusal_kind, p.refusal));
    assert_eq!((plan.container.as_str(), plan.width, plan.height, plan.stored_form.as_str()), ("png", 64, 64, "raw"));
    assert!(plan.shared_with.is_empty());
    assert!(plan.placement_prediction.contains("fit") || plan.placement_prediction.contains("appended"), "{}", plan.placement_prediction);
    let applied = reg.asset_member_apply_review(id, &plan.ticket).unwrap();
    assert_eq!(applied.pending, 1);
    assert!(applied.summary.dirty);
    assert!(reg.asset_member_apply_review(id, &plan.ticket).is_err(), "consumed");
    // The shared member: a private copy is predicted, the sibling named.
    let p = reg.asset_member_preview(id, &root_s, "databigs/inxp10.big", L100, &s(&source_shared)).unwrap();
    let plan2 = p.plan.unwrap();
    assert_eq!(plan2.shared_with, vec![L5.to_string()]);
    assert!(plan2.placement_prediction.contains("private copy"), "{}", plan2.placement_prediction);
    reg.asset_member_apply_review(id, &plan2.ticket).unwrap();
    let pending = reg.pending_changes(id, 100).unwrap();
    assert!(pending.dirty);
    assert_eq!(pending.primary.total_cells, 0);
    let am = pending.asset_members.as_ref().expect("pending asset members");
    assert_eq!(am.len(), 2);
    assert_eq!((am[0].member_path.as_str(), am[1].member_path.as_str()), (L0, L100));
    // Undo the last and re-apply through a fresh preview.
    assert!(reg.asset_member_undo(id).unwrap());
    assert_eq!(reg.pending_changes(id, 100).unwrap().asset_members.unwrap().len(), 1);
    let p = reg.asset_member_preview(id, &root_s, "databigs/inxp10.big", L100, &s(&source_shared)).unwrap();
    reg.asset_member_apply_review(id, &p.plan.unwrap().ticket).unwrap();
    // A second plan on the same member supersedes the pending one (still two pending).
    let p = reg.asset_member_preview(id, &root_s, "databigs/inxp10.big", L100, &s(&source_shared)).unwrap();
    reg.asset_member_apply_review(id, &p.plan.unwrap().ticket).unwrap();
    assert_eq!(reg.pending_changes(id, 100).unwrap().asset_members.unwrap().len(), 2);

    // The copied-target review stages the archive itself.
    let review = reg.copied_target_review(id, &root_s).unwrap();
    let keys: Vec<&str> = review.artifacts.iter().map(|a| a.key.as_str()).collect();
    assert!(keys.contains(&"archive-asset-0"), "{keys:?}");
    let apply = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(apply.state, "committed", "{}", apply.state);
    // Installed: l0 decodes to the new image; l100 is a private copy; l5 keeps the shared bytes.
    let l0 = studio_domain::decode_source_image_bytes(&member_png(&archive, L0)).unwrap();
    let want = studio_domain::decode_source_image(&source).unwrap();
    assert_eq!(l0.rgba, want.rgba);
    let l100 = studio_domain::decode_source_image_bytes(&member_png(&archive, L100)).unwrap();
    assert_eq!(l100.rgba, studio_domain::decode_source_image(&source_shared).unwrap().rgba);
    assert_eq!(member_png(&archive, L5), png(64, 64, 5), "the sibling keeps the shared bytes");
    let arc = tdb_archive_big::eb::EbArchive::parse(&std::fs::read(&archive).unwrap()).unwrap();
    let (e5, e100) = (arc.entries.iter().find(|e| e.path == L5).unwrap(), arc.entries.iter().find(|e| e.path == L100).unwrap());
    assert_ne!(e5.offset, e100.offset, "l100 was re-pointed at a private copy");
    assert_eq!(e100.offset, original_archive.len() as u64, "appended at the old end");
    assert_eq!(FileIdentity::of_file(&db).unwrap(), original_db);
    // The session adopted the install: clean, nothing pending; a further replacement plans on
    // the installed archive (l100 is no longer shared).
    assert!(!reg.summary(id).unwrap().dirty);
    assert!(reg.pending_changes(id, 100).unwrap().asset_members.is_none());
    let p = reg.asset_member_preview(id, &root_s, "databigs/inxp10.big", L100, &s(&source)).unwrap();
    assert!(p.plan.unwrap().shared_with.is_empty());
    // Restore: byte-exact.
    let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
    assert_eq!(restored.state, "restored", "{}", restored.state);
    assert_eq!(std::fs::read(&archive).unwrap(), original_archive);
}

/// R1 F2: superseding a pending replacement and undoing the later one leaves the **earlier** one
/// pending, and the copied-target save installs exactly that (the effective latest per member is
/// the only thing staged), with a byte-exact restore.
#[test]
fn undoing_a_superseding_replacement_saves_the_earlier_one() {
    let tmp = Tmp::new("supersede-undo");
    let (root, db, archive) = build_root(&tmp);
    let original_archive = std::fs::read(&archive).unwrap();
    let image_a = tmp.0.join("a.png");
    std::fs::write(&image_a, png(64, 64, 42)).unwrap();
    let image_b = tmp.0.join("b.png");
    std::fs::write(&image_b, png(64, 64, 77)).unwrap();
    let reg = Registry::default();
    let id = open(&reg, &db);
    let root_s = s(&root);

    // A then B on the same member: one effective pending entry carrying B.
    let a = reg.asset_member_preview(id, &root_s, "databigs/inxp10.big", L0, &s(&image_a)).unwrap().plan.unwrap();
    assert_eq!(reg.asset_member_apply_review(id, &a.ticket).unwrap().pending, 1);
    let b = reg.asset_member_preview(id, &root_s, "databigs/inxp10.big", L0, &s(&image_b)).unwrap().plan.unwrap();
    assert_ne!(a.new_stored_len, 0);
    let applied_b = reg.asset_member_apply_review(id, &b.ticket).unwrap();
    assert_eq!(applied_b.pending, 1, "coalesced per member");
    assert!(applied_b.summary.dirty);
    let pending = reg.pending_changes(id, 100).unwrap();
    let am = pending.asset_members.as_ref().unwrap();
    assert_eq!(am.len(), 1);
    assert_eq!(am[0].description, b.description);

    // Undo B: A is pending again, the project still dirty (was: clean, nothing pending).
    assert!(reg.asset_member_undo(id).unwrap());
    let summary = reg.summary(id).unwrap();
    assert!(summary.dirty, "A is still pending after undoing B");
    let pending = reg.pending_changes(id, 100).unwrap();
    assert!(pending.dirty);
    let am = pending.asset_members.as_ref().expect("A pending");
    assert_eq!(am.len(), 1);
    assert_eq!((am[0].member_path.as_str(), am[0].description.as_str(), am[0].new_stored_len), (L0, a.description.as_str(), a.new_stored_len));

    // The copied-target save installs A.
    let review = reg.copied_target_review(id, &root_s).unwrap();
    assert!(review.artifacts.iter().any(|x| x.key == "archive-asset-0"));
    let apply = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(apply.state, "committed", "{}", apply.state);
    let installed = studio_domain::decode_source_image_bytes(&member_png(&archive, L0)).unwrap();
    assert_eq!(installed.rgba, studio_domain::decode_source_image(&image_a).unwrap().rgba, "A, not B");
    assert_ne!(installed.rgba, studio_domain::decode_source_image(&image_b).unwrap().rgba);
    assert_eq!(member_png(&archive, L5), png(64, 64, 5));
    assert_eq!(member_png(&archive, L100), png(64, 64, 5), "the non-target shared row still reads the shared bytes");
    assert!(!reg.summary(id).unwrap().dirty);
    assert!(!reg.asset_member_undo(id).unwrap(), "the log is closed by the commit");
    let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(std::fs::read(&archive).unwrap(), original_archive);
}

#[test]
fn refusals_name_the_member_the_dimensions_and_the_missing_archive() {
    let tmp = Tmp::new("refusals");
    let (root, db, archive) = build_root(&tmp);
    let reg = Registry::default();
    let id = open(&reg, &db);
    let root_s = s(&root);
    let wrong = tmp.0.join("wrong.png");
    std::fs::write(&wrong, png(32, 32, 1)).unwrap();
    // A PNG member takes any size (the game reads the file's own size) — so a wrong-size source
    // is a plan, not a refusal, and the dims are the source's.
    let p = reg.asset_member_preview(id, &root_s, "databigs/inxp10.big", L0, &s(&wrong)).unwrap();
    assert_eq!((p.plan.as_ref().unwrap().width, p.plan.as_ref().unwrap().height), (32, 32));
    // Missing member; escaping path; a member that is not a texture.
    let p = reg.asset_member_preview(id, &root_s, "databigs/inxp10.big", "nhl/ui/assets/leaguelogos/size64/l9.png", &s(&wrong)).unwrap();
    assert_eq!(p.refusal_kind.as_deref(), Some("not-found"));
    let p = reg.asset_member_preview(id, &root_s, "../outside.big", L0, &s(&wrong)).unwrap();
    assert_eq!(p.refusal_kind.as_deref(), Some("invalid"));
    let p = reg.asset_member_preview(id, &root_s, "databigs/inxp10.big", "nhl/data/other/first.bin", &s(&wrong)).unwrap();
    assert_eq!(p.refusal_kind.as_deref(), Some("texture"), "{:?}", p.refusal);
    // A pending replacement whose archive vanished from the copied root refuses the review and
    // keeps the pending entry.
    let good = tmp.0.join("good.png");
    std::fs::write(&good, png(64, 64, 9)).unwrap();
    let p = reg.asset_member_preview(id, &root_s, "databigs/inxp10.big", L0, &s(&good)).unwrap();
    reg.asset_member_apply_review(id, &p.plan.unwrap().ticket).unwrap();
    std::fs::remove_file(&archive).unwrap();
    let e = match reg.copied_target_review(id, &root_s) {
        Ok(_) => panic!("expected a refusal"),
        Err(e) => e.to_string(),
    };
    assert!(e.contains("has no databigs/inxp10.big"), "{e}");
    assert_eq!(reg.pending_changes(id, 100).unwrap().asset_members.unwrap().len(), 1);
}

/// The real thing: an NHL 21 `l0.gnf` (BC7 256², chunkzip'd as shipped) in a synthetic archive
/// shared with its `_fr` twin — the private copy carries a re-encoded GNF the game's decoder
/// shape accepts (round trip within BC7 noise), the twin keeps the shipped bytes.
#[test]
#[ignore = "private fixture logos/ps4-nhl21/size256/l0.gnf: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn a_real_gnf_member_takes_a_private_copy_in_its_stored_form() {
    let fixtures = PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set"));
    let gnf = std::fs::read(fixtures.join("logos/ps4-nhl21/size256/l0.gnf")).unwrap();
    assert!(!gnf.is_empty());
    let tmp = Tmp::new("gnf");
    let root = tmp.0.join("copy");
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let db_dir = root.join("nhl/data/common/gamemodes/database/t3db");
    std::fs::create_dir_all(&db_dir).unwrap();
    let db = db_dir.join("nhl_game.db");
    std::fs::copy(repo("anchors/ps4/nhl21/databases/nhl_game.db"), &db).unwrap();
    let stored = tdb_archive_codec::compress_chunkzip_native(&gnf);
    let files: Vec<(u16, &str, Vec<u8>)> = vec![(1, "first.bin", b"FIRST".to_vec()), (0, "l0.gnf", stored.clone()), (0, "l0_fr.gnf", stored), (1, "last.bin", vec![0xAB; 64])];
    let mut bytes = build_archive(&["nhl/ui/kettle_assets/leaguelogos/size256", "nhl/data/other"], &files);
    let r1 = FILE_TABLE_OFFSET + 16;
    let r2 = FILE_TABLE_OFFSET + 2 * 16;
    let row1 = bytes[r1..r1 + 12].to_vec();
    bytes[r2..r2 + 12].copy_from_slice(&row1);
    std::fs::create_dir_all(root.join("databigs")).unwrap();
    let archive = root.join("databigs/inxp_10.big");
    std::fs::write(&archive, &bytes).unwrap();
    let original = std::fs::read(&archive).unwrap();

    let reg = Registry::default();
    let id = reg
        .open_workspace_from_paths(Some("nhl21"), parse_platform("ps4").unwrap(), &s(&db), None, &s(&repo("anchors/ps4/nhl21/databases/nhl_game-meta.xml")), &s(&repo("defs")))
        .unwrap()
        .project_id;
    let root_s = s(&root);
    let source = tmp.0.join("new.png");
    std::fs::write(&source, png(256, 256, 3)).unwrap();
    let member = "nhl/ui/kettle_assets/leaguelogos/size256/l0.gnf";
    let p = reg.asset_member_preview(id, &root_s, "databigs/inxp_10.big", member, &s(&source)).unwrap();
    let plan = p.plan.unwrap_or_else(|| panic!("{:?}", p.refusal));
    assert_eq!((plan.container.as_str(), plan.stored_form.as_str(), plan.width, plan.height), ("gnf", "chunkzip", 256, 256));
    assert_eq!(plan.shared_with, vec!["nhl/ui/kettle_assets/leaguelogos/size256/l0_fr.gnf".to_string()]);
    reg.asset_member_apply_review(id, &plan.ticket).unwrap();
    let review = reg.copied_target_review(id, &root_s).unwrap();
    let apply = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(apply.state, "committed");
    let all = std::fs::read(&archive).unwrap();
    let arc = tdb_archive_big::eb::EbArchive::parse(&all).unwrap();
    let e0 = arc.entries.iter().find(|e| e.path == member).unwrap();
    let e1 = arc.entries.iter().find(|e| e.path.ends_with("l0_fr.gnf")).unwrap();
    assert_eq!(e0.offset, original.len() as u64);
    assert_eq!(&all[e1.offset as usize..e1.offset as usize + e1.size as usize], &original[e1.offset as usize..e1.offset as usize + e1.size as usize]);
    let new_stored = &all[e0.offset as usize..e0.offset as usize + e0.size as usize];
    tdb_archive_codec::is_native_chunkzip_framing(new_stored).unwrap();
    let new_gnf = tdb_archive_codec::decompress_chunkzip(new_stored).unwrap();
    let out = tmp.0.join("out.gnf");
    std::fs::write(&out, &new_gnf).unwrap();
    let (container, img) = studio_domain::decode_texture_file(&out, None).unwrap();
    assert_eq!(container, studio_domain::TextureContainer::Gnf);
    let want = studio_domain::decode_source_image(&source).unwrap();
    let mad = img.rgba.iter().zip(&want.rgba).map(|(a, b)| (*a as i32 - *b as i32).abs() as f64).sum::<f64>() / img.rgba.len() as f64;
    assert!(mad < 6.0, "BC7 round trip MAD {mad}");
    let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(std::fs::read(&archive).unwrap(), original);
}

// ------------------------------------------------------------------ real PS4 archives (opt-in, C2 overnight 2026-09-20)

/// Streaming SHA-256 of a (multi-GB) file.
fn file_sha256(p: &Path) -> String {
    use sha2::Digest;
    use std::io::Read;
    let mut f = std::fs::File::open(p).unwrap();
    let mut h = sha2::Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    format!("{:x}", h.finalize())
}

/// The EB header + file table of an archive (not the payloads).
fn table_of(p: &Path) -> (tdb_archive_big::eb::EbArchive, Vec<u8>, u32) {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(p).unwrap();
    let mut hdr = vec![0u8; tdb_archive_big::eb::HEADER_LEN];
    f.read_exact(&mut hdr).unwrap();
    let h = tdb_archive_big::eb::EbHeader::parse(&hdr).unwrap();
    let mut table = vec![0u8; h.table_end()];
    f.seek(SeekFrom::Start(0)).unwrap();
    f.read_exact(&mut table).unwrap();
    (tdb_archive_big::eb::EbArchive::parse(&table).unwrap(), table, h.version)
}

fn read_span(p: &Path, off: u64, len: usize) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(p).unwrap();
    f.seek(SeekFrom::Start(off)).unwrap();
    let mut b = vec![0u8; len];
    f.read_exact(&mut b).unwrap();
    b
}

/// A visibly edited variant of a decoded member (magenta band across the middle third).
fn marked_png(member_plain: &[u8], seed: u8, dir: &Path) -> (PathBuf, studio_domain::DecodedImage) {
    let tmp = dir.join(format!("member-{seed}.bin"));
    std::fs::write(&tmp, member_plain).unwrap();
    let (_, img) = studio_domain::decode_texture_file(&tmp, None).unwrap();
    let (w, h) = (img.width, img.height);
    let mut rgba = img.rgba.clone();
    for y in h / 3..2 * h / 3 {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&[255, seed, 255, 255]);
        }
    }
    let edited = studio_domain::DecodedImage { width: w, height: h, rgba };
    let src = dir.join(format!("edited-{seed}.png"));
    std::fs::write(&src, edited.to_png().unwrap()).unwrap();
    (src, edited)
}

fn mad(a: &[u8], b: &[u8]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (*x as i32 - *y as i32).abs() as f64).sum::<f64>() / a.len().max(1) as f64
}

/// The route on the **real** `inxp_10*.big` of each installed PS4 title, on a disposable copy under
/// `%TEMP%`: a league logo member (`size256/l0`, a shared span on 16-21 -> private copy) and a non-logo
/// imagery member (`large_league_logo` / `memorial_cup` / `trophy_broadcast` per title) replaced with
/// marked images in one copied-target save; the table afterwards differs from the original only in
/// the two rows and the archive-size word; the siblings' bytes and every other payload are untouched
/// (the stager's verify + a direct check of the shared block); the installed members decode to the
/// marked pixels in native chunkzip framing; restore is byte-exact (streaming hash).
/// Titles come from `NHL_STUDIO_PS4_TITLES` (comma list; default all seven) so one title can be run
/// alone; a missing install is a failure, not a skip.
#[test]
#[ignore = "real PS4 installs under H:/Emulators/ShadPS4/Games + private fixtures (logos/imagery manifests): set NHL_STUDIO_FIXTURES and select with --ignored; copies up to 3.8 GB per title into %TEMP%"]
fn every_real_ps4_inxp10_archive_takes_a_logo_and_an_imagery_member_through_the_copied_target_save() {
    let fixtures = PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set"));
    let logos: serde_json::Value = serde_json::from_slice(&std::fs::read(fixtures.join("logos/manifest.json")).unwrap()).unwrap();
    let imagery: serde_json::Value = serde_json::from_slice(&std::fs::read(fixtures.join("imagery/manifest.json")).unwrap()).unwrap();
    let titles: Vec<String> = std::env::var("NHL_STUDIO_PS4_TITLES")
        .map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_else(|_| ["nhl15", "nhl16", "nhl17", "nhl18", "nhl19", "nhl20", "nhl21"].iter().map(|s| s.to_string()).collect());
    for title in &titles {
        let t0 = std::time::Instant::now();
        let logo = logos["ps4"].as_array().unwrap().iter().find(|r| r["title"] == title.as_str() && r["path"].as_str().unwrap().contains("size256") && r["path"].as_str().unwrap().ends_with(if title == "nhl15" { "l0.png" } else { "l0.gnf" })).unwrap_or_else(|| panic!("{title}: no size256 l0 logo record"));
        let img_rec = imagery["ps4"].as_array().unwrap().iter().find(|r| r["title"] == title.as_str() && r["type"] != "gmc_league_icon").unwrap_or_else(|| panic!("{title}: no imagery record"));
        let source_archive = PathBuf::from(logo["archive"].as_str().unwrap());
        assert!(source_archive.is_file(), "{title}: install archive {} is not present (a missing install fails, it does not skip)", source_archive.display());
        let archive_relative = format!("databigs/{}", source_archive.file_name().unwrap().to_string_lossy());
        assert_eq!(img_rec["archive_relative"].as_str().unwrap(), archive_relative, "{title}: both members in the same archive");
        let logo_path = logo["path"].as_str().unwrap();
        let img_path = img_rec["path"].as_str().unwrap();
        let logo_plain = std::fs::read(fixtures.join(logo["fixture"].as_str().unwrap())).unwrap();
        let img_plain = std::fs::read(fixtures.join(img_rec["fixture"].as_str().unwrap())).unwrap();

        // Disposable copied target: the archive copied from the install (read-only source).
        let tmp = Tmp::new(&format!("real-{title}"));
        let root = tmp.0.join("copy");
        studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
        let db_dir = root.join("nhl/data/common/gamemodes/database/t3db");
        std::fs::create_dir_all(&db_dir).unwrap();
        let db = db_dir.join("nhl_game.db");
        let anchor_title = if title == "nhl20" { "nhl19" } else { title.as_str() }; // the nhl20 anchor is the NHL 19 base copy; the DB is not edited here
        std::fs::copy(repo(&format!("anchors/ps4/{anchor_title}/databases/nhl_game.db")), &db).unwrap();
        std::fs::create_dir_all(root.join("databigs")).unwrap();
        let archive = root.join(&archive_relative);
        std::fs::copy(&source_archive, &archive).unwrap();
        let original_len = std::fs::metadata(&archive).unwrap().len();
        let original_sha = file_sha256(&archive);
        let (orig_arc, orig_table, version) = table_of(&archive);
        eprintln!("{title}: copied {} ({original_len} B, {} entries) in {:?}", archive_relative, orig_arc.entries.len(), t0.elapsed());

        let reg = Registry::default();
        let id = reg
            .open_workspace_from_paths(Some(title), parse_platform("ps4").unwrap(), &s(&db), None, &s(&repo(&format!("anchors/ps4/{anchor_title}/databases/nhl_game-meta.xml"))), &s(&repo("defs")))
            .unwrap()
            .project_id;
        let root_s = s(&root);
        let (logo_src, logo_edit) = marked_png(&logo_plain, 10, &tmp.0);
        let (img_src, img_edit) = marked_png(&img_plain, 200, &tmp.0);

        // Plan both.
        let p = reg.asset_member_preview(id, &root_s, &archive_relative, logo_path, &s(&logo_src)).unwrap();
        let plan_logo = p.plan.unwrap_or_else(|| panic!("{title}: logo refused {:?} {:?}", p.refusal_kind, p.refusal));
        let e_logo = orig_arc.entries.iter().find(|e| e.path == logo_path).unwrap();
        let expect_shared: Vec<String> = orig_arc.entries.iter().filter(|o| o.path != logo_path && o.offset < e_logo.offset + e_logo.size as u64 && o.offset + o.size as u64 > e_logo.offset).map(|o| o.path.clone()).collect();
        assert_eq!(plan_logo.shared_with, expect_shared, "{title}: siblings named from the real table");
        assert_eq!((plan_logo.width, plan_logo.height), (256, 256), "{title}");
        assert_eq!(plan_logo.stored_form, if title == "nhl15" { "raw" } else { "chunkzip" }, "{title}");
        reg.asset_member_apply_review(id, &plan_logo.ticket).unwrap();
        let p = reg.asset_member_preview(id, &root_s, &archive_relative, img_path, &s(&img_src)).unwrap();
        let plan_img = p.plan.unwrap_or_else(|| panic!("{title}: imagery refused {:?} {:?}", p.refusal_kind, p.refusal));
        reg.asset_member_apply_review(id, &plan_img.ticket).unwrap();
        let pending = reg.pending_changes(id, 100).unwrap();
        assert_eq!(pending.asset_members.as_ref().map(|v| v.len()), Some(2), "{title}");
        eprintln!("{title}: planned logo ({}; {}) + imagery ({}) in {:?}", plan_logo.placement_prediction, plan_logo.shared_with.join(", "), plan_img.placement_prediction, t0.elapsed());

        // Copied-target save on the real archive.
        let review = reg.copied_target_review(id, &root_s).unwrap_or_else(|e| panic!("{title}: review: {e}"));
        let keys: Vec<&str> = review.artifacts.iter().map(|a| a.key.as_str()).collect();
        assert!(keys.contains(&"archive-asset-0"), "{title}: {keys:?}");
        eprintln!("{title}: staged in {:?}", t0.elapsed());
        let apply = reg.copied_target_apply(id, review.handle).unwrap_or_else(|e| panic!("{title}: apply: {e}"));
        assert_eq!(apply.state, "committed", "{title}: {}", apply.state);
        eprintln!("{title}: committed in {:?}", t0.elapsed());

        // The table: only the two rows and the archive-size word may differ.
        let (new_arc, new_table, _) = table_of(&archive);
        assert_eq!(new_table.len(), orig_table.len(), "{title}: table length");
        let row_len = if version == 0x0011_0400 { 20 } else { 16 };
        let changed_rows: Vec<usize> = (0..orig_arc.entries.len()).filter(|i| orig_table[FILE_TABLE_OFFSET + i * row_len..FILE_TABLE_OFFSET + (i + 1) * row_len] != new_table[FILE_TABLE_OFFSET + i * row_len..FILE_TABLE_OFFSET + (i + 1) * row_len]).collect();
        let (i_logo, i_img) = (
            orig_arc.entries.iter().position(|e| e.path == logo_path).unwrap(),
            orig_arc.entries.iter().position(|e| e.path == img_path).unwrap(),
        );
        let mut expect_rows = vec![i_logo, i_img];
        expect_rows.sort_unstable();
        assert_eq!(changed_rows, expect_rows, "{title}: exactly the two rows changed");
        // Header: bytes 0..0x30 equal except the archive-size word when the archive grew.
        for (i, (a, b)) in orig_table[..FILE_TABLE_OFFSET].iter().zip(&new_table[..FILE_TABLE_OFFSET]).enumerate() {
            if a != b {
                assert!((0x18..0x20).contains(&i), "{title}: header byte {i} changed");
            }
        }
        // The shared block's siblings: rows and bytes untouched (the stager verified every
        // untouched byte; check the shared block directly too).
        for sib in &expect_shared {
            let o = orig_arc.entries.iter().find(|e| e.path == *sib).unwrap();
            let n = new_arc.entries.iter().find(|e| e.path == *sib).unwrap();
            assert_eq!((o.offset, o.size), (n.offset, n.size), "{title}: sibling {sib} row");
            assert_eq!(read_span(&archive, n.offset, n.size as usize), read_span(&source_archive, o.offset, o.size as usize), "{title}: sibling {sib} bytes");
        }
        let n_logo = new_arc.entries.iter().find(|e| e.path == logo_path).unwrap();
        if !expect_shared.is_empty() {
            assert!(n_logo.offset >= original_len, "{title}: private copy appended past the original EOF");
        }
        // The installed members decode to the marked pixels.
        for (path, edit, bound) in [(logo_path, &logo_edit, 8.0), (img_path, &img_edit, 8.0)] {
            let e = new_arc.entries.iter().find(|e| e.path == path).unwrap();
            let stored = read_span(&archive, e.offset, e.size as usize);
            let plain = if stored.starts_with(tdb_archive_codec::MAGIC_CHUNKZIP) {
                tdb_archive_codec::is_native_chunkzip_framing(&stored).unwrap();
                tdb_archive_codec::decompress_chunkzip(&stored).unwrap()
            } else {
                stored
            };
            let out = tmp.0.join("installed.bin");
            std::fs::write(&out, &plain).unwrap();
            let (_, img) = studio_domain::decode_texture_file(&out, None).unwrap();
            let d = mad(&img.rgba, &edit.rgba);
            assert!(d <= bound, "{title}: {path}: installed pixels MAD {d}");
        }
        // Clean session; restore byte-exact.
        assert!(!reg.summary(id).unwrap().dirty, "{title}");
        assert!(reg.pending_changes(id, 100).unwrap().asset_members.is_none(), "{title}");
        let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
        assert_eq!(restored.state, "restored", "{title}: {}", restored.state);
        assert_eq!(std::fs::metadata(&archive).unwrap().len(), original_len, "{title}");
        assert_eq!(file_sha256(&archive), original_sha, "{title}: restore byte-exact");
        eprintln!("{title}: restored in {:?}", t0.elapsed());
    }
}

/// C7: the **whole-group** scope of a shared block, chosen explicitly at preview: the plan names
/// the scope, the copied-target save rewrites the shared block in place when the bytes fit (both
/// rows keep their offset and take the new size) or appends once with every row re-pointed when
/// they do not; a private-copy plan on another shared member in the same save stays private;
/// restore is byte-exact; an irregular group refuses the whole-group scope.
#[test]
fn a_whole_group_replacement_rewrites_every_path_of_the_block_and_restores() {
    let tmp = Tmp::new("group");
    let (root, db, archive) = build_root(&tmp);
    let original = std::fs::read(&archive).unwrap();
    let reg = Registry::default();
    let id = open(&reg, &db);
    let root_s = s(&root);
    // A tiny image (fits the shared 64x64 block) with the whole-group scope.
    let small = tmp.0.join("small.png");
    std::fs::write(&small, png(64, 64, 9)).unwrap();
    let p = reg.asset_member_preview_scoped(id, &root_s, "databigs/inxp10.big", L100, &s(&small), true).unwrap();
    let plan = p.plan.unwrap_or_else(|| panic!("{:?} {:?}", p.refusal_kind, p.refusal));
    assert_eq!(plan.scope, "whole-group");
    assert_eq!(plan.shared_with, vec![L5.to_string()]);
    assert!(plan.placement_prediction.starts_with("whole group"), "{}", plan.placement_prediction);
    // The default scope on the same member is a private copy (a different plan).
    let p2 = reg.asset_member_preview(id, &root_s, "databigs/inxp10.big", L100, &s(&small)).unwrap();
    assert_eq!(p2.plan.as_ref().unwrap().scope, "this-member");
    reg.asset_member_apply_review(id, &plan.ticket).unwrap();
    let review = reg.copied_target_review(id, &root_s).unwrap();
    let apply = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(apply.state, "committed");
    let arc = tdb_archive_big::eb::EbArchive::parse(&std::fs::read(&archive).unwrap()).unwrap();
    let (e5, e100) = (arc.entries.iter().find(|e| e.path == L5).unwrap(), arc.entries.iter().find(|e| e.path == L100).unwrap());
    assert_eq!((e5.offset, e5.size), (e100.offset, e100.size), "still one block");
    let orig = tdb_archive_big::eb::EbArchive::parse(&original).unwrap();
    let o5 = orig.entries.iter().find(|e| e.path == L5).unwrap();
    assert_eq!(e5.offset, o5.offset, "rewritten in place");
    let want = studio_domain::decode_source_image(&small).unwrap();
    assert_eq!(studio_domain::decode_source_image_bytes(&member_png(&archive, L5)).unwrap().rgba, want.rgba, "the sibling shows the new image too");
    assert_eq!(std::fs::metadata(&archive).unwrap().len(), original.len() as u64, "nothing appended");
    let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(std::fs::read(&archive).unwrap(), original);

    // A large image: appended once, both rows re-pointed; the old block stays.
    let big = tmp.0.join("big.png");
    let noisy: Vec<u8> = (0..64 * 64 * 4).map(|i| ((i as u32).wrapping_mul(2654435761) >> 13) as u8).collect();
    std::fs::write(&big, studio_domain::DecodedImage { width: 64, height: 64, rgba: noisy }.to_png().unwrap()).unwrap();
    let p = reg.asset_member_preview_scoped(id, &root_s, "databigs/inxp10.big", L5, &s(&big), true).unwrap();
    let plan = p.plan.unwrap();
    assert!(plan.placement_prediction.contains("appended once"), "{}", plan.placement_prediction);
    reg.asset_member_apply_review(id, &plan.ticket).unwrap();
    let review = reg.copied_target_review(id, &root_s).unwrap();
    let apply = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(apply.state, "committed");
    let all = std::fs::read(&archive).unwrap();
    let arc = tdb_archive_big::eb::EbArchive::parse(&all).unwrap();
    let (e5, e100) = (arc.entries.iter().find(|e| e.path == L5).unwrap(), arc.entries.iter().find(|e| e.path == L100).unwrap());
    assert_eq!((e5.offset, e5.size), (e100.offset, e100.size));
    assert_eq!(e5.offset, original.len() as u64, "appended at the old end");
    assert_eq!(&all[o5.offset as usize..o5.offset as usize + o5.size as usize], &original[o5.offset as usize..o5.offset as usize + o5.size as usize], "old block left in place");
    let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(std::fs::read(&archive).unwrap(), original);
}

/// R1 F1 (20 September 2026): through the Registry seam, a scope that contradicts a pending
/// replacement of the same block (whole-group `l100` pending, then a private copy or a whole-group
/// replacement of its sibling `l5`) is refused at preview with a stable `scope-conflict` kind naming
/// both members; the pending review, the summary and the undo depth are unchanged; the accepted
/// whole-group plan still stages, installs on every path of the block and restores byte-exact.
#[test]
fn a_contradictory_scope_on_a_pending_block_is_refused_and_the_accepted_plan_still_installs_and_restores() {
    let tmp = Tmp::new("scope-conflict");
    let (root, db, archive) = build_root(&tmp);
    let original = std::fs::read(&archive).unwrap();
    let reg = Registry::default();
    let id = open(&reg, &db);
    let root_s = s(&root);
    let img_a = tmp.0.join("a.png");
    std::fs::write(&img_a, png(64, 64, 21)).unwrap();
    let img_b = tmp.0.join("b.png");
    std::fs::write(&img_b, png(64, 64, 22)).unwrap();

    let p = reg.asset_member_preview_scoped(id, &root_s, "databigs/inxp10.big", L100, &s(&img_a), true).unwrap();
    let plan = p.plan.unwrap();
    assert_eq!(plan.scope, "whole-group");
    reg.asset_member_apply_review(id, &plan.ticket).unwrap();
    let before = reg.pending_changes(id, 100).unwrap();
    let am = before.asset_members.as_ref().unwrap();
    assert_eq!(am.len(), 1);
    assert_eq!(am[0].member_path, L100);

    for whole in [false, true] {
        let p = reg.asset_member_preview_scoped(id, &root_s, "databigs/inxp10.big", L5, &s(&img_b), whole).unwrap();
        assert!(p.plan.is_none(), "must refuse: {:?}", p.plan.map(|p| p.placement_prediction));
        assert_eq!(p.refusal_kind.as_deref(), Some("scope-conflict"));
        let reason = p.refusal.unwrap();
        assert!(reason.contains(L5) && reason.contains(L100), "{reason}");
        let after = reg.pending_changes(id, 100).unwrap();
        assert_eq!(after.asset_members.as_ref().unwrap().len(), 1);
        assert_eq!(after.asset_members.as_ref().unwrap()[0].member_path, L100);
        assert!(after.dirty);
    }
    // An unrelated member is still accepted beside the group plan.
    let p = reg.asset_member_preview(id, &root_s, "databigs/inxp10.big", L0, &s(&img_b)).unwrap();
    reg.asset_member_apply_review(id, &p.plan.unwrap().ticket).unwrap();
    assert_eq!(reg.pending_changes(id, 100).unwrap().asset_members.unwrap().len(), 2);

    // The accepted plans stage and install: both paths of the block show image A, l0 shows B.
    let review = reg.copied_target_review(id, &root_s).unwrap();
    let apply = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(apply.state, "committed");
    let want_a = studio_domain::decode_source_image(&img_a).unwrap().rgba;
    let want_b = studio_domain::decode_source_image(&img_b).unwrap().rgba;
    for path in [L5, L100] {
        assert_eq!(studio_domain::decode_source_image_bytes(&member_png(&archive, path)).unwrap().rgba, want_a, "{path}");
    }
    assert_eq!(studio_domain::decode_source_image_bytes(&member_png(&archive, L0)).unwrap().rgba, want_b);
    let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(std::fs::read(&archive).unwrap(), original);
}
