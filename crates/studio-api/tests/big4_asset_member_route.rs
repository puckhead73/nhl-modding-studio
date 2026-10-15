//! C4 (overnight 2026-09-20) — the archive-member texture route on the **BIG4 + `.bh`** archives
//! of NHL 07–10 (Xbox 360 `data0.big` / `fe.big` with the packages RefPack-wrapped whole; PS3 08–10
//! raw): the package is rewritten by the span-preserving re-plan, re-wrapped as stored, and placed
//! by the BIG4 stager — fit-in-slot against the verified-zero slack, else append-and-retarget with
//! the directory entry, the header length, the `.bh` record and the `.bh` trailer patched — through
//! the ordinary copied-target save as **two artifacts of one change set** (`archive-asset-n` and
//! `archive-asset-n.bh`), restored together. NHL 07 `data0.big` refuses growth (its `.bh` is
//! mirrored inside `boot.big`). Synthetic pairs around real fixture packages run by default when
//! the fixtures are present (`--ignored` otherwise); real `fe.big` copies run `--ignored`.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};
use studio_domain::tdb_txn::FileIdentity;
use tdb_archive_big::big4::{synth::build, BoundBig4};
use tdb_archive_big::replan::PackageLayout;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-c4-big4-{tag}-{}", std::process::id()));
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

fn fixture_root() -> PathBuf {
    PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the private logo fixtures cannot be resolved"))
}

fn mad(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(x, y)| (*x as i32 - *y as i32).abs() as f64).sum::<f64>() / a.len().max(1) as f64
}

/// A marked variant (magenta band) of the decoded stored package (RefPack-wrapped or raw).
fn marked_source(stored: &[u8], seed: u8, dir: &Path) -> (PathBuf, studio_domain::DecodedImage) {
    let tmp = dir.join(format!("pkg-{seed}.big"));
    std::fs::write(&tmp, stored).unwrap();
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

/// A noise image of the package's dimensions: RefPack cannot compress it, so the wrapped member
/// (X-A whole-package wrapper, P-B inner texture) grows past its stock slot — the append case.
fn noisy_source(stored: &[u8], seed: u32, dir: &Path) -> (PathBuf, studio_domain::DecodedImage) {
    let tmp = dir.join(format!("pkg-noise-{seed}.big"));
    std::fs::write(&tmp, stored).unwrap();
    let (_, img) = studio_domain::decode_texture_file(&tmp, None).unwrap();
    let (w, h) = (img.width, img.height);
    let mut state = seed.wrapping_mul(2654435761).wrapping_add(12345);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        for _ in 0..3 {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            rgba.push((state >> 24) as u8);
        }
        rgba.push(255);
    }
    let edited = studio_domain::DecodedImage { width: w, height: h, rgba };
    let src = dir.join(format!("noise-{seed}.png"));
    std::fs::write(&src, edited.to_png().unwrap()).unwrap();
    (src, edited)
}

const LOGO: &str = "fe/ion/artassets/leaguelogos/l0.big";
const SMALL: &str = "fe/ion/artassets/leaguelogossmall/l0.big";
const WIDE: &str = "fe/ion/artassets/leaguelogoswide/l0.big";

fn anchors_dir(platform: &str) -> &'static str {
    if platform == "xbox" { "xbox360" } else { "ps3" }
}

/// NHL 07 ships its main database as `NHL07NG.db` (+ `NHL07NG-meta.xml`).
fn db_stem(game: &str) -> &'static str {
    if game == "nhl07" { "NHL07NG" } else { "nhlng" }
}

fn open(reg: &Registry, platform: &str, game: &str, db: &Path) -> u32 {
    reg.open_workspace_from_paths(
        Some(game),
        parse_platform(platform).unwrap(),
        &s(db),
        None,
        &s(&repo(&format!("anchors/{}/{game}/databases/{}-meta.xml", anchors_dir(platform), db_stem(game)))),
        &s(&repo("defs")),
    )
    .unwrap()
    .project_id
}

/// `<root>/<archive>` + `.bh` around the three stored packages and two unrelated members.
fn build_root(tmp: &Tmp, platform: &str, game: &str, archive_name: &str, logo: &[u8], small: &[u8], wide: &[u8]) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let root = tmp.0.join("copy");
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let db = root.join(format!("{}.db", db_stem(game)));
    std::fs::copy(repo(&format!("anchors/{}/{game}/databases/{}.db", anchors_dir(platform), db_stem(game))), &db).unwrap();
    let dec = |b: &[u8]| if tdb_archive_codec::is_refpack(b) { tdb_archive_codec::refpack_decompress(b).unwrap().len() as u32 } else { 0 };
    let members: Vec<(&str, &[u8], u32)> = vec![
        ("fe/data/first.bin", b"FIRST-MEMBER", 0),
        (LOGO, logo, dec(logo)),
        (SMALL, small, dec(small)),
        (WIDE, wide, dec(wide)),
        ("fe/data/last.bin", &[0xABu8; 64], 0),
    ];
    let (big, bh) = build(&members, &[0, 16, 0, 48, 0]);
    let archive = root.join(archive_name);
    let sidecar = root.join(studio_domain::asset_member_edit::big4_sidecar_relative(archive_name));
    std::fs::write(&archive, &big).unwrap();
    std::fs::write(&sidecar, &bh).unwrap();
    (root, db, archive, sidecar)
}

fn entry_bytes(big: &[u8], bh: &[u8], path: &str) -> (u32, Vec<u8>, [u32; 4]) {
    let b = BoundBig4::bind(&mut std::io::Cursor::new(big), bh).unwrap();
    let e = b.entries.iter().find(|e| e.path == path).unwrap();
    (e.offset, big[e.offset as usize..(e.offset + e.size) as usize].to_vec(), b.sidecar.records[e.index])
}

/// The full route on one synthetic pair: three planned members (one fits, the DXT5 small one
/// grows → append), one copied-target save with the two artifacts, table/sidecar facts, decoded
/// pixels, restore of both files byte-exact.
fn run_synthetic(platform: &str, game: &str, archive_name: &str, plat_dir: &str) {
    let fx = fixture_root();
    let logo = std::fs::read(fx.join(format!("logos/{plat_dir}-{game}/leaguelogos/l0.big"))).unwrap();
    let small = std::fs::read(fx.join(format!("logos/{plat_dir}-{game}/leaguelogossmall/l0.big"))).unwrap();
    let wide = std::fs::read(fx.join(format!("logos/{plat_dir}-{game}/leaguelogoswide/l0.big"))).unwrap();
    let tmp = Tmp::new(&format!("{platform}-{game}"));
    let (root, db, archive, sidecar) = build_root(&tmp, platform, game, archive_name, &logo, &small, &wide);
    let (orig_big, orig_bh) = (std::fs::read(&archive).unwrap(), std::fs::read(&sidecar).unwrap());
    let original_db = FileIdentity::of_file(&db).unwrap();
    let reg = Registry::default();
    let id = open(&reg, platform, game, &db);
    let root_s = s(&root);

    // The listing binds the packages to this archive (and needs the sidecar beside it).
    let ll = reg.league_logo(id, 0, Some(&root_s)).unwrap().expect("league 0 resolves");
    assert!(ll.variants.iter().all(|v| v.archive.as_deref() == Some(archive_name)), "{platform} {game}");
    assert!(ll.notes.iter().any(|n| n.contains(".bh")));

    let mut planned = Vec::new();
    // LOGO / WIDE: marked images (compress well → fit); SMALL: noise (RefPack cannot shrink it →
    // the wrapped member grows → append on the RefPack chains; P-A's raw inner texture keeps its
    // fixed length and fits).
    for (path, stored, seed, bound) in [(LOGO, &logo, 10u8, 0.5f64), (SMALL, &small, 77, -1.0), (WIDE, &wide, 200, 0.5)] {
        let (src, edit) = if bound < 0.0 { noisy_source(stored, seed as u32, &tmp.0) } else { marked_source(stored, seed, &tmp.0) };
        let p = reg.asset_member_preview(id, &root_s, archive_name, path, &s(&src)).unwrap();
        let plan = p.plan.unwrap_or_else(|| panic!("{platform} {game} {path}: {:?} {:?}", p.refusal_kind, p.refusal));
        assert_eq!(plan.stored_form, if platform == "xbox" { "refpack" } else { "raw" }, "{platform} {game}");
        assert!(plan.description.contains(".bh"), "{}", plan.description);
        eprintln!("{platform} {game} {path}: {}", plan.placement_prediction);
        reg.asset_member_apply_review(id, &plan.ticket).unwrap();
        planned.push((path, stored.clone(), edit, bound));
    }
    assert_eq!(reg.pending_changes(id, 100).unwrap().asset_members.unwrap().len(), 3);
    let review = reg.copied_target_review(id, &root_s).unwrap();
    let keys: Vec<&str> = review.artifacts.iter().map(|a| a.key.as_str()).collect();
    assert!(keys.contains(&"archive-asset-0") && keys.contains(&"archive-asset-0.bh"), "{keys:?}");
    let apply = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(apply.state, "committed", "{}", apply.state);

    let (new_big, new_bh) = (std::fs::read(&archive).unwrap(), std::fs::read(&sidecar).unwrap());
    let rb = BoundBig4::bind(&mut std::io::Cursor::new(&new_big), &new_bh).unwrap();
    assert_eq!(rb.size_le as usize, new_big.len());
    assert_eq!(rb.sidecar.trailer_len() as usize, new_big.len());
    // P2-A (21 Sep): a grown member reflows (the stock shape) — it keeps its offset, its slot grows
    // by Δ and every later entry moves up by Δ with its bytes and `.bh` record (offset word aside).
    let delta = new_big.len() as u32 - orig_big.len() as u32;
    for (other, after_grown) in [("fe/data/first.bin", false), ("fe/data/last.bin", true)] {
        let (o_off, o_bytes, o_rec) = entry_bytes(&orig_big, &orig_bh, other);
        let (n_off, n_bytes, n_rec) = entry_bytes(&new_big, &new_bh, other);
        let shift = if after_grown { delta } else { 0 };
        assert_eq!((o_off + shift, &o_bytes, &o_rec[1..]), (n_off, &n_bytes, &n_rec[1..]), "{other}");
        assert_eq!(n_rec[0], o_rec[0] + shift, "{other}: .bh offset word");
    }
    let mut grown = 0;
    let mut running_shift = 0u32;
    for (path, stored, edit, bound) in &planned {
        let (o_off, o_bytes, _) = entry_bytes(&orig_big, &orig_bh, path);
        let (n_off, n_bytes, n_rec) = entry_bytes(&new_big, &new_bh, path);
        assert_eq!(n_off, o_off + running_shift, "{path}: the entry keeps its offset (shifted only by earlier reflows)");
        assert!((n_off as usize) < orig_big.len() + delta as usize, "{path}: nothing is appended past the EOF");
        if n_bytes.len() > o_bytes.len() {
            grown += 1;
            let old_slot_end = (o_off as u64 + o_bytes.len() as u64).div_ceil(64) * 64;
            let new_slot_end = (n_off as u64 + n_bytes.len() as u64).div_ceil(64) * 64;
            if new_slot_end > old_slot_end + running_shift as u64 {
                running_shift += (new_slot_end - (old_slot_end + running_shift as u64)) as u32;
            }
        }
        let plain = if tdb_archive_codec::is_refpack(&n_bytes) {
            assert_eq!(n_rec[2] as usize, tdb_archive_codec::refpack_decompress(&n_bytes).unwrap().len(), "{path}: .bh decoded length");
            tdb_archive_codec::refpack_decompress(&n_bytes).unwrap()
        } else {
            assert_eq!(n_rec[2], 0);
            n_bytes.clone()
        };
        assert_eq!(n_rec[1] as usize, n_bytes.len(), "{path}: .bh stored size");
        let src_plain = if tdb_archive_codec::is_refpack(stored) { tdb_archive_codec::refpack_decompress(stored).unwrap() } else { stored.clone() };
        let (l0, l1) = (PackageLayout::parse(&src_plain).unwrap(), PackageLayout::parse(&plain).unwrap());
        assert_eq!(&plain[l1.table_end..l1.data_offset as usize], &src_plain[l0.table_end..l0.data_offset as usize], "{path}: trailer");
        assert_eq!(&plain[plain.len() - l1.tail_len..], &src_plain[src_plain.len() - l0.tail_len..], "{path}: tail");
        let t = tmp.0.join("installed.big");
        std::fs::write(&t, &n_bytes).unwrap();
        let (_, img) = studio_domain::decode_texture_file(&t, None).unwrap();
        if *bound >= 0.0 {
            assert!(mad(&img.rgba, &edit.rgba) <= *bound, "{path}: pixels");
        } else {
            // Noise through DXT5: not the source pixels, but certainly not the shipped logo.
            std::fs::write(&t, stored).unwrap();
            let (_, orig) = studio_domain::decode_texture_file(&t, None).unwrap();
            assert!(mad(&img.rgba, &orig.rgba) > 20.0, "{path}: the noise did not land");
        }
    }
    let refpack_chain = tdb_archive_codec::is_refpack(&small) || {
        let plain = PackageLayout::parse(&small).unwrap();
        let tex = plain.members.iter().max_by_key(|m| m.size).unwrap();
        tdb_archive_codec::is_refpack(&small[tex.offset as usize..(tex.offset + tex.size) as usize])
    };
    if refpack_chain {
        assert!(grown >= 1, "{platform} {game}: the noisy small logo grows past its slot on a RefPack chain");
        assert!(delta > 0 && delta % 64 == 0, "{platform} {game}: the reflow grew the archive by a 64-byte multiple ({delta})");
        assert!(rb.index_order_is_offset_order, "the reflowed pair keeps the stock offset order");
    } else {
        assert_eq!(grown, 0, "{platform} {game}: raw inner textures keep their length (P-A)");
        assert_eq!(delta, 0);
    }
    assert_eq!(FileIdentity::of_file(&db).unwrap(), original_db);
    assert!(!reg.summary(id).unwrap().dirty);
    // A further plan on the installed (reflowed) pair binds fine.
    let (src, _) = marked_source(&logo, 33, &tmp.0);
    let p = reg.asset_member_preview(id, &root_s, archive_name, LOGO, &s(&src)).unwrap();
    assert!(p.plan.is_some(), "{:?}", p.refusal);
    // Restore both files byte-exact.
    let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(std::fs::read(&archive).unwrap(), orig_big);
    assert_eq!(std::fs::read(&sidecar).unwrap(), orig_bh);
}

#[test]
#[ignore = "private fixtures logos/x360-nhl10/**: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn xbox_nhl10_packages_in_a_synthetic_fe_big_pair_flow_through_the_archive_route() {
    run_synthetic("xbox", "nhl10", "fe.big", "x360");
}

#[test]
#[ignore = "private fixtures logos/x360-nhl08/**: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn xbox_nhl08_packages_in_a_synthetic_data0_big_pair_flow_through_the_archive_route() {
    run_synthetic("xbox", "nhl08", "data0.big", "x360");
}

#[test]
#[ignore = "private fixtures logos/ps3-nhl10/**: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn ps3_nhl10_packages_in_a_synthetic_fe_big_pair_flow_through_the_archive_route() {
    run_synthetic("ps3", "nhl10", "fe.big", "ps3");
}

#[test]
#[ignore = "private fixtures logos/ps3-nhl08/**: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn ps3_nhl08_packages_in_a_synthetic_data0_big_pair_flow_through_the_archive_route() {
    run_synthetic("ps3", "nhl08", "data0.big", "ps3");
}

/// NHL 07 `data0.big` (P2-A, 21 Sep 2026 — the boot.big cascade, research 2026-09-20 handoff 03
/// §4): a replacement that changes the `.bh` record is no longer refused; the review names the
/// cascade, and the copied-target save writes **four** files as one transaction — `data0.big` +
/// `data0.bh` (reflow / fit as elsewhere) and `boot.big` + `boot.bh`, where the boot.big member
/// `data0.bh` is the RefPack'd re-encoded index (fit-in-slot against its verified-zero slack here,
/// as the six stock members with 227–2 036 bytes of slack; else reflow). Afterwards the loose
/// `data0.bh` and the decoded boot.big copy are equal (the cascade's invariant), everything else
/// in boot.big is verbatim, and restore puts all four files back byte-exact. Sidecar faults still
/// refuse before anything is planned; a missing boot pair refuses with the cascade named.
#[test]
#[ignore = "private fixtures logos/x360-nhl07/**: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn nhl07_data0_growth_cascades_into_boot_big_and_sidecar_faults_refuse_before_planning() {
    let fx = fixture_root();
    let logo = std::fs::read(fx.join("logos/x360-nhl07/leaguelogos/l0.big")).unwrap();
    let small = std::fs::read(fx.join("logos/x360-nhl07/leaguelogossmall/l0.big")).unwrap();
    let wide = std::fs::read(fx.join("logos/x360-nhl07/leaguelogoswide/l0.big")).unwrap();
    let tmp = Tmp::new("nhl07");
    let (root, db, archive, sidecar) = build_root(&tmp, "xbox", "nhl07", "data0.big", &logo, &small, &wide);
    let reg = Registry::default();
    let id = open(&reg, "xbox", "nhl07", &db);
    let root_s = s(&root);
    let ll = reg.league_logo(id, 0, Some(&root_s)).unwrap().unwrap();
    assert!(ll.notes.iter().any(|n| n.contains("boot.big")), "{:?}", ll.notes);
    // No boot pair yet: a `.bh`-changing replacement is refused with the cascade named.
    let (src_logo, edit_logo) = marked_source(&logo, 10, &tmp.0);
    let p = reg.asset_member_preview(id, &root_s, "data0.big", LOGO, &s(&src_logo)).unwrap();
    assert_eq!(p.refusal_kind.as_deref(), Some("sidecar-cascade"), "{:?}", p.plan.map(|p| p.placement_prediction));
    assert!(p.refusal.as_deref().unwrap().contains("boot.big"), "{:?}", p.refusal);
    // The boot pair in the measured shape: `data0.bh` = RefPack(loose data0.bh) with zero slack
    // after it, between two unrelated members.
    let bh0 = std::fs::read(&sidecar).unwrap();
    let boot_member = tdb_archive_codec::refpack_compress(&bh0);
    let (boot_big, boot_bh) = build(&[("gamedata/first.bin", b"BOOT-FIRST", 0), ("data0.bh", &boot_member, bh0.len() as u32), ("gamedata/last.bin", &[0xCDu8; 40], 0)], &[0, 1024, 0]);
    std::fs::write(root.join("boot.big"), &boot_big).unwrap();
    std::fs::write(root.join("boot.bh"), &boot_bh).unwrap();
    // A wrong boot copy (not decode-equal to the loose sidecar) refuses.
    {
        let other = tdb_archive_codec::refpack_compress(&[0x11u8; 64]);
        let (bb, bbh) = build(&[("gamedata/first.bin", b"BOOT-FIRST", 0), ("data0.bh", &other, 64), ("gamedata/last.bin", &[0xCDu8; 40], 0)], &[0, 1024, 0]);
        std::fs::write(root.join("boot.big"), &bb).unwrap();
        std::fs::write(root.join("boot.bh"), &bbh).unwrap();
        let p = reg.asset_member_preview(id, &root_s, "data0.big", LOGO, &s(&src_logo)).unwrap();
        assert_eq!(p.refusal_kind.as_deref(), Some("sidecar-cascade"));
        assert!(p.refusal.as_deref().unwrap().contains("decode-equal"), "{:?}", p.refusal);
        std::fs::write(root.join("boot.big"), &boot_big).unwrap();
        std::fs::write(root.join("boot.bh"), &boot_bh).unwrap();
    }
    // Two members: the marked logo (fits its slot; the `.bh` size word changes) and the noisy small
    // logo (grows → ordered reflow of data0.big).
    let p = reg.asset_member_preview(id, &root_s, "data0.big", LOGO, &s(&src_logo)).unwrap();
    let plan_logo = p.plan.unwrap_or_else(|| panic!("{:?} {:?}", p.refusal_kind, p.refusal));
    assert!(plan_logo.placement_prediction.contains("boot.big") && plan_logo.placement_prediction.contains("data0.bh"), "{}", plan_logo.placement_prediction);
    reg.asset_member_apply_review(id, &plan_logo.ticket).unwrap();
    let (src_small, _) = noisy_source(&small, 77, &tmp.0);
    let p = reg.asset_member_preview(id, &root_s, "data0.big", SMALL, &s(&src_small)).unwrap();
    let plan_small = p.plan.unwrap_or_else(|| panic!("{:?} {:?}", p.refusal_kind, p.refusal));
    assert!(plan_small.placement_prediction.contains("ordered reflow"), "{}", plan_small.placement_prediction);
    reg.asset_member_apply_review(id, &plan_small.ticket).unwrap();
    let pending = reg.pending_changes(id, 100).unwrap();
    assert_eq!(pending.asset_members.as_ref().map(|m| m.len()), Some(2));

    let orig_big = std::fs::read(&archive).unwrap();
    let review = reg.copied_target_review(id, &root_s).unwrap();
    let keys: Vec<&str> = review.artifacts.iter().map(|a| a.key.as_str()).collect();
    for k in ["archive-asset-0", "archive-asset-0.bh", "archive-asset-boot", "archive-asset-boot.bh"] {
        assert!(keys.contains(&k), "{keys:?}");
    }
    let apply = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(apply.state, "committed");

    // data0 pair: reflowed (grew by a 64-byte multiple), both members installed.
    let new_big = std::fs::read(&archive).unwrap();
    let new_bh = std::fs::read(&sidecar).unwrap();
    assert!(new_big.len() > orig_big.len() && (new_big.len() - orig_big.len()) % 64 == 0);
    assert_ne!(new_bh, bh0, "the sidecar changed");
    let rb = BoundBig4::bind(&mut std::io::Cursor::new(&new_big), &new_bh).unwrap();
    assert!(rb.index_order_is_offset_order);
    let (_, installed_logo, _) = entry_bytes(&new_big, &new_bh, LOGO);
    let t = tmp.0.join("installed.big");
    std::fs::write(&t, &installed_logo).unwrap();
    let (_, img) = studio_domain::decode_texture_file(&t, None).unwrap();
    assert!(mad(&img.rgba, &edit_logo.rgba) <= 0.5, "installed logo pixels");
    // boot pair: the `data0.bh` member decodes to the new loose sidecar; everything else verbatim.
    let boot_big2 = std::fs::read(root.join("boot.big")).unwrap();
    let boot_bh2 = std::fs::read(root.join("boot.bh")).unwrap();
    let bb = BoundBig4::bind(&mut std::io::Cursor::new(&boot_big2), &boot_bh2).unwrap();
    let m = bb.entries.iter().find(|e| e.path == "data0.bh").unwrap();
    let stored = &boot_big2[m.offset as usize..(m.offset + m.size) as usize];
    assert!(tdb_archive_codec::is_refpack(stored));
    assert_eq!(tdb_archive_codec::refpack_decompress(stored).unwrap(), new_bh, "boot.big's copy of data0.bh equals the loose data0.bh");
    assert_eq!(bb.sidecar.records[m.index][2] as usize, new_bh.len(), "boot.bh decoded-length word");
    let (o_first, first_bytes, first_rec) = entry_bytes(&boot_big, &boot_bh, "gamedata/first.bin");
    let (n_first, first_bytes2, first_rec2) = entry_bytes(&boot_big2, &boot_bh2, "gamedata/first.bin");
    assert_eq!((o_first, first_bytes, first_rec), (n_first, first_bytes2, first_rec2));
    let (_, last_bytes, _) = entry_bytes(&boot_big, &boot_bh, "gamedata/last.bin");
    let (_, last_bytes2, _) = entry_bytes(&boot_big2, &boot_bh2, "gamedata/last.bin");
    assert_eq!(last_bytes, last_bytes2);
    assert_eq!(bb.size_le as usize, boot_big2.len());
    assert_eq!(bb.sidecar.trailer_len() as usize, boot_big2.len());
    eprintln!("nhl07 cascade: data0.big {} -> {} B; boot.big {} -> {} B (member {} -> {} B, slot capacity 1 KB slack)", orig_big.len(), new_big.len(), boot_big.len(), boot_big2.len(), boot_member.len(), m.size);
    assert!(!reg.summary(id).unwrap().dirty);
    // Restore all four files byte-exact.
    let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(std::fs::read(&archive).unwrap(), orig_big);
    assert_eq!(std::fs::read(&sidecar).unwrap(), bh0);
    assert_eq!(std::fs::read(root.join("boot.big")).unwrap(), boot_big);
    assert_eq!(std::fs::read(root.join("boot.bh")).unwrap(), boot_bh);
    // Sidecar faults still refuse before anything is planned.
    let mut bad = bh0.clone();
    bad[16 + 16 + 5] ^= 1;
    std::fs::write(&sidecar, &bad).unwrap();
    let p = reg.asset_member_preview(id, &root_s, "data0.big", LOGO, &s(&src_logo)).unwrap();
    assert_eq!(p.refusal_kind.as_deref(), Some("parse"), "{:?}", p.refusal);
    assert!(p.refusal.as_deref().unwrap().contains("sidecar"));
    std::fs::remove_file(&sidecar).unwrap();
    let p = reg.asset_member_preview(id, &root_s, "data0.big", LOGO, &s(&src_logo)).unwrap();
    assert_eq!(p.refusal_kind.as_deref(), Some("sidecar-missing"));
    let ll = reg.league_logo(id, 0, Some(&root_s)).unwrap().unwrap();
    assert!(ll.variants.iter().all(|v| v.archive.is_none()), "no .bh beside the archive → no archive route offered");
    std::fs::write(&sidecar, &bh0).unwrap();
}

/// The growth policies in isolation (no fixtures): a plan-wide `Refuse` refuses growth, `Append`
/// and `Reflow` place it; NHL 07's `data0.big`–`data5.big` are recognised as mirrored in boot.big
/// (the cascade), and no archive refuses growth by title any more.
#[test]
fn nhl07_growth_policy_is_per_archive_name() {
    use tdb_archive_big::big4::{plan, Growth};
    let (big, bh) = build(&[("fe/a.big", &[1u8; 100], 0), ("fe/b.big", &[2u8; 70], 0)], &[0, 0]);
    let b = BoundBig4::bind(&mut std::io::Cursor::new(&big), &bh).unwrap();
    let e = &b.entries[0];
    let req = |new: &[u8]| tdb_archive_big::big4::Big4Replacement {
        entry_index: 0,
        expected_path: e.path.clone(),
        expected_stored_sha256: { use sha2::Digest; sha2::Sha256::digest(&big[e.offset as usize..(e.offset + e.size) as usize]).into() },
        new_stored: new.to_vec(),
        decoded_len: 0,
        growth: None,
    };
    let refuse = Growth::Refuse("data0.big: mirrored in boot.big".into());
    assert!(plan(&b, &mut std::io::Cursor::new(&big), vec![req(&[9u8; 100])], refuse.clone()).is_ok());
    assert!(plan(&b, &mut std::io::Cursor::new(&big), vec![req(&[9u8; 200])], refuse).is_err());
    assert!(plan(&b, &mut std::io::Cursor::new(&big), vec![req(&[9u8; 200])], Growth::Append).is_ok());
    assert!(plan(&b, &mut std::io::Cursor::new(&big), vec![req(&[9u8; 200])], Growth::Reflow).is_ok());
    use studio_domain::asset_member_edit::{big4_growth_refusal, big4_sidecar_is_mirrored_in_boot};
    assert!(big4_sidecar_is_mirrored_in_boot(Some("nhl07"), "data0.big"));
    assert!(big4_sidecar_is_mirrored_in_boot(Some("nhl07"), "data5.big"));
    assert!(!big4_sidecar_is_mirrored_in_boot(Some("nhl07"), "boot.big"));
    assert!(!big4_sidecar_is_mirrored_in_boot(Some("nhl08"), "data0.big"));
    assert!(!big4_sidecar_is_mirrored_in_boot(Some("nhl10"), "fe.big"));
    assert_eq!(big4_growth_refusal(Some("nhl07"), "data0.big"), None);
}

// ------------------------------------------------------------------ real BIG4 archives (opt-in)

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

fn read_span(p: &Path, off: u64, len: usize) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(p).unwrap();
    f.seek(SeekFrom::Start(off)).unwrap();
    let mut b = vec![0u8; len];
    f.read_exact(&mut b).unwrap();
    b
}

struct Big4Target {
    key: &'static str,
    platform: &'static str,
    game_id: &'static str,
    archive: &'static str,
    archive_name: &'static str,
}

const BIG4_TARGETS: &[Big4Target] = &[
    Big4Target { key: "xbox-nhl07", platform: "xbox", game_id: "nhl07", archive: r"X:\Games\Emulation\Roms\XBOX360\NHL 07\NHL 07 (USA)\data0.big", archive_name: "data0.big" },
    Big4Target { key: "xbox-nhl08", platform: "xbox", game_id: "nhl08", archive: r"X:\Games\Emulation\Roms\XBOX360\NHL 08\NHL 08 (USA)\data0.big", archive_name: "data0.big" },
    Big4Target { key: "xbox-nhl09", platform: "xbox", game_id: "nhl09", archive: r"X:\Games\Emulation\Roms\XBOX360\NHL 09\NHL 09 (USA)\fe.big", archive_name: "fe.big" },
    Big4Target { key: "xbox-nhl10", platform: "xbox", game_id: "nhl10", archive: r"X:\Games\Emulation\Roms\XBOX360\NHL 10\NHL 10\fe.big", archive_name: "fe.big" },
    Big4Target { key: "ps3-nhl08", platform: "ps3", game_id: "nhl08", archive: r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 08\PS3_GAME\USRDIR\data0.big", archive_name: "data0.big" },
    Big4Target { key: "ps3-nhl09", platform: "ps3", game_id: "nhl09", archive: r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 09\PS3_GAME\USRDIR\fe.big", archive_name: "fe.big" },
    Big4Target { key: "ps3-nhl10", platform: "ps3", game_id: "nhl10", archive: r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 10\PS3_GAME\USRDIR\fe.big", archive_name: "fe.big" },
];

/// The route on the **real** BIG4 + `.bh` pair of each selected target (`NHL_STUDIO_BIG4_TARGETS`,
/// comma list of the keys above; default `xbox-nhl10,ps3-nhl10`), on a disposable copy of both
/// files: `leaguelogos/l0` (marked, fits) and `leaguelogossmall/l0` (noise, grows → append on the
/// RefPack chains; NHL 07 refuses growth and plans only the fitting member) in one copied-target
/// save; both files re-bind and cross-check; every other entry / record / payload untouched; the
/// installed packages decode; restore of both files byte-exact. A missing install fails.
#[test]
#[ignore = "real Xbox 360 / PS3 installs under X:\\Games\\Emulation\\Roms + private fixtures: set NHL_STUDIO_FIXTURES and select with --ignored; copies up to 2 GB per target into %TEMP%"]
fn every_real_big4_archive_takes_a_logo_package_through_the_copied_target_save_with_its_sidecar() {
    let selected: Vec<String> = std::env::var("NHL_STUDIO_BIG4_TARGETS")
        .map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_else(|_| vec!["xbox-nhl10".into(), "ps3-nhl10".into()]);
    for key in &selected {
        let t = BIG4_TARGETS.iter().find(|t| t.key == key).unwrap_or_else(|| panic!("unknown target {key}"));
        let t0 = std::time::Instant::now();
        let source = Path::new(t.archive);
        let source_bh = PathBuf::from(studio_domain::asset_member_edit::big4_sidecar_relative(t.archive));
        assert!(source.is_file() && source_bh.is_file(), "{key}: {} (+ .bh) is not present (a missing install fails, it does not skip)", source.display());
        let tmp = Tmp::new(&format!("real-{key}"));
        let root = tmp.0.join("copy");
        studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
        let db = root.join(format!("{}.db", db_stem(t.game_id)));
        std::fs::copy(repo(&format!("anchors/{}/{}/databases/{}.db", anchors_dir(t.platform), t.game_id, db_stem(t.game_id))), &db).unwrap();
        let archive = root.join(t.archive_name);
        let sidecar = root.join(studio_domain::asset_member_edit::big4_sidecar_relative(t.archive_name));
        std::fs::copy(source, &archive).unwrap();
        std::fs::copy(&source_bh, &sidecar).unwrap();
        // NHL 07: the boot pair that mirrors the sidecar joins the copy (the cascade's target).
        let boot_pair: Option<(Vec<u8>, Vec<u8>)> = if t.game_id == "nhl07" {
            let dir = source.parent().unwrap();
            std::fs::copy(dir.join("boot.big"), root.join("boot.big")).unwrap();
            std::fs::copy(dir.join("boot.bh"), root.join("boot.bh")).unwrap();
            Some((std::fs::read(root.join("boot.big")).unwrap(), std::fs::read(root.join("boot.bh")).unwrap()))
        } else {
            None
        };
        let original_len = std::fs::metadata(&archive).unwrap().len();
        let original_sha = file_sha256(&archive);
        let original_bh = std::fs::read(&sidecar).unwrap();
        let bound0 = {
            let mut f = std::io::BufReader::new(std::fs::File::open(&archive).unwrap());
            BoundBig4::bind(&mut f, &original_bh).unwrap()
        };
        eprintln!("{key}: copied {} ({original_len} B, {} entries; index order == offset order: {}) in {:?}", t.archive_name, bound0.entries.len(), bound0.index_order_is_offset_order, t0.elapsed());

        let reg = Registry::default();
        let id = open(&reg, t.platform, t.game_id, &db);
        let root_s = s(&root);
        let ll = reg.league_logo(id, 0, Some(&root_s)).unwrap().expect("league 0 resolves");
        assert!(ll.variants.iter().all(|v| v.archive.as_deref() == Some(t.archive_name)), "{key}");

        let mut planned: Vec<(&str, Vec<u8>, Option<studio_domain::DecodedImage>)> = Vec::new();
        for (path, noise) in [(LOGO, false), (SMALL, true)] {
            let e = bound0.entries.iter().find(|e| e.path == path).unwrap_or_else(|| panic!("{key}: {path} not in the archive"));
            let stored = read_span(&archive, e.offset as u64, e.size as usize);
            let (src, edit) = if noise { noisy_source(&stored, 77, &tmp.0) } else { marked_source(&stored, 10, &tmp.0) };
            let p = reg.asset_member_preview(id, &root_s, t.archive_name, path, &s(&src)).unwrap();
            match p.plan {
                Some(plan) => {
                    eprintln!("{key}: {path}: {}", plan.placement_prediction);
                    reg.asset_member_apply_review(id, &plan.ticket).unwrap();
                    planned.push((path, stored, if noise { None } else { Some(edit) }));
                }
                None => panic!("{key}: {path} refused: {:?} {:?}", p.refusal_kind, p.refusal),
            }
        }
        assert!(!planned.is_empty(), "{key}: nothing planned");
        let review = reg.copied_target_review(id, &root_s).unwrap_or_else(|e| panic!("{key}: review: {e}"));
        let keys: Vec<&str> = review.artifacts.iter().map(|a| a.key.as_str()).collect();
        assert!(keys.contains(&"archive-asset-0") && keys.contains(&"archive-asset-0.bh"), "{key}: {keys:?}");
        if boot_pair.is_some() {
            assert!(keys.contains(&"archive-asset-boot") && keys.contains(&"archive-asset-boot.bh"), "{key}: the NHL 07 cascade artifacts: {keys:?}");
        }
        eprintln!("{key}: staged in {:?}", t0.elapsed());
        let apply = reg.copied_target_apply(id, review.handle).unwrap_or_else(|e| panic!("{key}: apply: {e}"));
        assert_eq!(apply.state, "committed", "{key}: {}", apply.state);
        eprintln!("{key}: committed in {:?}", t0.elapsed());

        let new_bh = std::fs::read(&sidecar).unwrap();
        let bound1 = {
            let mut f = std::io::BufReader::new(std::fs::File::open(&archive).unwrap());
            BoundBig4::bind(&mut f, &new_bh).unwrap()
        };
        assert_eq!(bound1.entries.len(), bound0.entries.len());
        let planned_paths: Vec<&str> = planned.iter().map(|(p, _, _)| *p).collect();
        // P2-A: growth reflows — entries after a grown one move up by the archive's growth;
        // the shift at an entry is the growth contributed by grown entries that precede it.
        let mut by_offset: Vec<&str> = planned_paths.clone();
        by_offset.sort_by_key(|p| bound0.entries.iter().find(|e| e.path == *p).unwrap().offset);
        let mut grown_slots: Vec<(u64, u64)> = Vec::new(); // (old slot end, delta)
        for p in by_offset {
            let a = bound0.entries.iter().find(|e| e.path == p).unwrap();
            let b = bound1.entries.iter().find(|e| e.path == p).unwrap();
            let shift_before: u64 = grown_slots.iter().map(|(_, d)| *d).sum();
            let old_end = bound0.slot_end(a.index);
            let new_end = (b.offset as u64 + b.size as u64).div_ceil(64) * 64;
            if new_end > old_end + shift_before {
                grown_slots.push((old_end, new_end - old_end - shift_before));
            }
        }
        let total_growth = std::fs::metadata(&archive).unwrap().len() - original_len;
        assert_eq!(total_growth, grown_slots.iter().map(|(_, d)| *d).sum::<u64>(), "{key}: growth accounted by reflow deltas");
        assert!(bound1.index_order_is_offset_order, "{key}: the reflowed archive keeps the stock order");
        for (a, b) in bound0.entries.iter().zip(&bound1.entries) {
            if planned_paths.contains(&a.path.as_str()) {
                continue;
            }
            let shift: u64 = grown_slots.iter().filter(|(end, _)| a.offset as u64 >= *end).map(|(_, d)| *d).sum();
            assert_eq!((a.offset as u64 + shift, a.size, &a.path), (b.offset as u64, b.size, &b.path), "{key}: entry {} changed", a.path);
            let (ra, rb) = (bound0.sidecar.records[a.index], bound1.sidecar.records[a.index]);
            assert_eq!((ra[0] as u64 + shift, &ra[1..]), (rb[0] as u64, &rb[1..]), "{key}: record {} changed", a.path);
            if shift == 0 {
                assert_eq!(read_span(&archive, a.offset as u64, a.size.min(4096) as usize), read_span(source, a.offset as u64, a.size.min(4096) as usize), "{key}: entry {} bytes", a.path);
            }
        }
        for (path, stored, edit) in &planned {
            let e = bound1.entries.iter().find(|e| e.path == *path).unwrap();
            let installed = read_span(&archive, e.offset as u64, e.size as usize);
            assert_ne!(&installed, stored, "{key} {path}: the member changed");
            let r = bound1.sidecar.records[e.index];
            assert_eq!((r[0], r[1]), (e.offset, e.size), "{key} {path}: .bh record");
            if tdb_archive_codec::is_refpack(&installed) {
                assert_eq!(r[2] as usize, tdb_archive_codec::refpack_decompress(&installed).unwrap().len(), "{key} {path}: .bh decoded length");
            } else {
                assert_eq!(r[2], 0);
            }
            let tt = tmp.0.join("installed.big");
            std::fs::write(&tt, &installed).unwrap();
            let (_, img) = studio_domain::decode_texture_file(&tt, None).unwrap();
            if let Some(edit) = edit {
                assert!(mad(&img.rgba, &edit.rgba) <= 0.5, "{key} {path}: installed pixels");
            }
        }
        if let Some((boot_big0, boot_bh0)) = &boot_pair {
            // NHL 07: boot.big's `data0.bh` member decodes to the new loose sidecar; the other
            // 26 members are verbatim (fit-in-slot, 1 398 bytes of slack) or shifted (reflow).
            let boot_big1 = std::fs::read(root.join("boot.big")).unwrap();
            let boot_bh1 = std::fs::read(root.join("boot.bh")).unwrap();
            let b0 = BoundBig4::bind(&mut std::io::Cursor::new(boot_big0), boot_bh0).unwrap();
            let b1 = BoundBig4::bind(&mut std::io::Cursor::new(&boot_big1), &boot_bh1).unwrap();
            let m = b1.entries.iter().find(|e| e.path.eq_ignore_ascii_case("data0.bh")).unwrap();
            let stored = &boot_big1[m.offset as usize..(m.offset + m.size) as usize];
            assert_eq!(tdb_archive_codec::refpack_decompress(stored).unwrap(), new_bh, "{key}: boot.big's data0.bh copy equals the loose sidecar");
            assert_eq!(b1.sidecar.records[m.index][2] as usize, new_bh.len());
            let shift = boot_big1.len() as u64 - boot_big0.len() as u64;
            for (a, b) in b0.entries.iter().zip(&b1.entries) {
                if a.index == m.index {
                    continue;
                }
                let s = if a.offset > b0.entries[m.index].offset { shift } else { 0 };
                assert_eq!((a.offset as u64 + s, a.size, &a.path), (b.offset as u64, b.size, &b.path), "{key}: boot.big entry {}", a.path);
                assert_eq!(&boot_big1[b.offset as usize..(b.offset + b.size) as usize], &boot_big0[a.offset as usize..(a.offset + a.size) as usize], "{key}: boot.big entry {} bytes", a.path);
            }
            eprintln!("{key}: boot.big {} -> {} B (data0.bh member {} -> {} B; {})", boot_big0.len(), boot_big1.len(), b0.entries[m.index].size, m.size, if shift == 0 { "fit-in-slot" } else { "reflow" });
        }
        assert!(!reg.summary(id).unwrap().dirty);
        let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
        assert_eq!(restored.state, "restored", "{key}");
        if let Some((boot_big0, boot_bh0)) = &boot_pair {
            assert_eq!(&std::fs::read(root.join("boot.big")).unwrap(), boot_big0, "{key}: boot.big restored byte-exact");
            assert_eq!(&std::fs::read(root.join("boot.bh")).unwrap(), boot_bh0, "{key}: boot.bh restored byte-exact");
        }
        assert_eq!(std::fs::metadata(&archive).unwrap().len(), original_len);
        assert_eq!(file_sha256(&archive), original_sha, "{key}: archive restored byte-exact");
        assert_eq!(std::fs::read(&sidecar).unwrap(), original_bh, "{key}: sidecar restored byte-exact");
        eprintln!("{key}: restored in {:?}", t0.elapsed());
    }
}
