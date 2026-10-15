//! C3 (overnight 2026-09-20) — the archive-member texture route on the **gen7 EB archives**
//! (Xbox 360 NHL 11 `fe.big`, NHL 12–Legacy / PS3 12–Legacy `cache.big`, Recompiled by identity):
//! the member is a BIGF logo / imagery *package*, rewritten in place by the span-preserving re-plan
//! (`tdb_archive_big::replan`) and placed by the S2-A stager (fit-in-slot when the re-planned
//! package fits its slot + verified-zero slack, else append-and-retarget) through the ordinary
//! copied-target save. The default case uses a synthetic 16-byte-row EB archive (the shipped
//! `0x00000400` layout) around **real** fixture packages (Xbox Legacy XPR2 + `divisionlogos`,
//! PS3 Legacy DDS); the real `cache.big` copies run `--ignored`. Nothing here is evidence a game
//! accepts the result; loose-vs-archive precedence stays a user card.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};
use studio_domain::tdb_txn::FileIdentity;
use tdb_archive_big::replan::PackageLayout;
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
        let dir = std::env::temp_dir().join(format!("nhlms-c3-gen7-{tag}-{}", std::process::id()));
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

/// A marked variant (magenta band) of the decoded package.
fn marked_source(pkg: &[u8], seed: u8, dir: &Path) -> (PathBuf, studio_domain::DecodedImage) {
    let tmp = dir.join(format!("pkg-{seed}.big"));
    std::fs::write(&tmp, pkg).unwrap();
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

fn member_bytes(archive: &Path, path: &str) -> (u64, Vec<u8>) {
    let all = std::fs::read(archive).unwrap();
    let arc = tdb_archive_big::eb::EbArchive::parse(&all).unwrap();
    let e = arc.entries.iter().find(|e| e.path == path).unwrap();
    (e.offset, all[e.offset as usize..e.offset as usize + e.size as usize].to_vec())
}

const LOGO: &str = "fe/ion/artassets/leaguelogos/l0.big";
const DIV: &str = "fe/ion/artassets/divisionlogos/d0.big";
const SMALL: &str = "fe/ion/artassets/leaguelogossmall/l0.big";

/// Build `<root>/<archive>` (16-byte rows) with the three packages plus two unrelated members;
/// `slack_after_logo` extra zero bytes after the logo so a fit-in-slot case can be forced.
fn build_root(tmp: &Tmp, platform: &str, archive_name: &str, logo: &[u8], small: &[u8], div: &[u8]) -> (PathBuf, PathBuf, PathBuf) {
    let root = tmp.0.join("copy");
    studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let anchors = if platform == "xbox" { "xbox360" } else { platform };
    let db = root.join("nhlng.db");
    std::fs::copy(repo(&format!("anchors/{anchors}/nhllegacy/databases/nhlng.db")), &db).unwrap();
    let files: Vec<(u16, &str, Vec<u8>)> = vec![
        (3, "first.bin", b"FIRST-MEMBER".to_vec()),
        (0, "l0.big", logo.to_vec()),
        (1, "l0.big", small.to_vec()),
        (2, "d0.big", div.to_vec()),
        (3, "last.bin", vec![0xAB; 64]),
    ];
    let bytes = build_archive(&["fe/ion/artassets/leaguelogos", "fe/ion/artassets/leaguelogossmall", "fe/ion/artassets/divisionlogos", "fe/data/other"], &files);
    let archive = root.join(archive_name);
    std::fs::write(&archive, &bytes).unwrap();
    (root, db, archive)
}

fn open(reg: &Registry, platform: &str, db: &Path) -> u32 {
    let anchors = if platform == "xbox" { "xbox360" } else { platform };
    reg.open_workspace_from_paths(
        Some("nhllegacy"),
        parse_platform(platform).unwrap(),
        &s(db),
        None,
        &s(&repo(&format!("anchors/{anchors}/nhllegacy/databases/nhlng-meta.xml"))),
        &s(&repo("defs")),
    )
    .unwrap()
    .project_id
}

/// One title/platform: three real packages in a synthetic `cache.big` → the logo (XPR2 / DDS
/// 8_8_8_8, lossless) and the `divisionlogos` package replaced with marked images and the small
/// (DXT5) logo with a same-pixel round trip, one copied-target save; the installed members are
/// stock-shaped packages (re-plan: trailer + tail + other members verbatim) that decode to the
/// marked pixels; the unrelated members and the DB are untouched; restore byte-exact.
fn run_synthetic(platform: &str, tag: &str) {
    let root_fx = fixture_root();
    let plat_dir = if platform == "xbox" { "x360" } else { "ps3" };
    let logo = std::fs::read(root_fx.join(format!("logos/{plat_dir}-nhllegacy/leaguelogos/l0.big"))).unwrap();
    let small = std::fs::read(root_fx.join(format!("logos/{plat_dir}-nhllegacy/leaguelogossmall/l0.big"))).unwrap();
    let div = std::fs::read(root_fx.join(format!("imagery/{plat_dir}-nhllegacy/divisionlogos/d0.big"))).unwrap();
    assert!(logo.starts_with(b"BIGF") && small.starts_with(b"BIGF") && div.starts_with(b"BIGF"));
    let tmp = Tmp::new(tag);
    let (root, db, archive) = build_root(&tmp, platform, "cache.big", &logo, &small, &div);
    let original = std::fs::read(&archive).unwrap();
    let original_db = FileIdentity::of_file(&db).unwrap();
    let reg = Registry::default();
    let id = open(&reg, platform, &db);
    let root_s = s(&root);

    // The listing binds the packages to the archive on this title/platform.
    let logo_listing = reg.league_logo(id, 0, Some(&root_s)).unwrap();
    let ll = logo_listing.expect("NHL league (code 0) resolves on Legacy");
    assert!(ll.variants.iter().all(|v| v.archive.as_deref() == Some("cache.big")), "{:?}", ll.variants.iter().map(|v| v.archive.clone()).collect::<Vec<_>>());
    assert!(ll.notes.iter().any(|n| n.contains("cache.big")));

    let (logo_src, logo_edit) = marked_source(&logo, 10, &tmp.0);
    let (div_src, div_edit) = marked_source(&div, 200, &tmp.0);
    let p = reg.asset_member_preview(id, &root_s, "cache.big", LOGO, &s(&logo_src)).unwrap();
    let plan = p.plan.unwrap_or_else(|| panic!("{:?} {:?}", p.refusal_kind, p.refusal));
    assert_eq!((plan.container.as_str(), plan.stored_form.as_str()), ("portrait", "raw"));
    assert_eq!((plan.width, plan.height), (logo_edit.width, logo_edit.height));
    assert!(plan.shared_with.is_empty());
    eprintln!("{platform} logo: {}", plan.placement_prediction);
    reg.asset_member_apply_review(id, &plan.ticket).unwrap();
    let p = reg.asset_member_preview(id, &root_s, "cache.big", DIV, &s(&div_src)).unwrap();
    let plan = p.plan.unwrap_or_else(|| panic!("{:?} {:?}", p.refusal_kind, p.refusal));
    eprintln!("{platform} div: {}", plan.placement_prediction);
    reg.asset_member_apply_review(id, &plan.ticket).unwrap();
    // The DXT5 small logo with its own decoded pixels: a same-content re-encode of a lossy slot
    // still changes the stream (the encoder is not EA's), so it is a third planned member.
    let (small_src, _) = {
        let t = tmp.0.join("small.big");
        std::fs::write(&t, &small).unwrap();
        let (_, img) = studio_domain::decode_texture_file(&t, None).unwrap();
        let src = tmp.0.join("small.png");
        std::fs::write(&src, img.to_png().unwrap()).unwrap();
        (src, img)
    };
    let p = reg.asset_member_preview(id, &root_s, "cache.big", SMALL, &s(&small_src)).unwrap();
    let plan = p.plan.unwrap_or_else(|| panic!("{:?} {:?}", p.refusal_kind, p.refusal));
    eprintln!("{platform} small: {}", plan.placement_prediction);
    reg.asset_member_apply_review(id, &plan.ticket).unwrap();
    assert_eq!(reg.pending_changes(id, 100).unwrap().asset_members.unwrap().len(), 3);

    let review = reg.copied_target_review(id, &root_s).unwrap();
    let keys: Vec<&str> = review.artifacts.iter().map(|a| a.key.as_str()).collect();
    assert!(keys.contains(&"archive-asset-0"), "{keys:?}");
    let apply = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(apply.state, "committed", "{}", apply.state);

    // Installed packages: stock-shaped, decode to the marked pixels.
    for (path, source_pkg, edit, bound) in [(LOGO, &logo, Some(&logo_edit), 0.0), (DIV, &div, Some(&div_edit), 0.0), (SMALL, &small, None, 6.0)] {
        let (_, installed) = member_bytes(&archive, path);
        let l0 = PackageLayout::parse(source_pkg).unwrap();
        let l1 = PackageLayout::parse(&installed).unwrap_or_else(|e| panic!("{platform} {path}: {e}"));
        assert_eq!(l1.members.len(), l0.members.len());
        assert_eq!(&installed[l1.table_end..l1.data_offset as usize], &source_pkg[l0.table_end..l0.data_offset as usize], "{path}: trailer");
        assert_eq!(&installed[installed.len() - l1.tail_len..], &source_pkg[source_pkg.len() - l0.tail_len..], "{path}: tail");
        let tex = l0.members.iter().max_by_key(|m| m.size).unwrap();
        for (a, b) in l0.members.iter().zip(&l1.members) {
            if a.name != tex.name {
                assert_eq!(&installed[b.offset as usize..(b.offset + b.size) as usize], &source_pkg[a.offset as usize..(a.offset + a.size) as usize], "{path}: member {}", a.name);
            }
        }
        let t = tmp.0.join("installed.big");
        std::fs::write(&t, &installed).unwrap();
        let (_, img) = studio_domain::decode_texture_file(&t, None).unwrap();
        if let Some(edit) = edit {
            assert!(mad(&img.rgba, &edit.rgba) <= bound, "{platform} {path}: pixels");
        }
    }
    // Unrelated members untouched; DB untouched; clean session; restore exact.
    assert_eq!(member_bytes(&archive, "fe/data/other/first.bin").1, b"FIRST-MEMBER");
    assert_eq!(member_bytes(&archive, "fe/data/other/last.bin").1, vec![0xAB; 64]);
    assert_eq!(FileIdentity::of_file(&db).unwrap(), original_db);
    assert!(!reg.summary(id).unwrap().dirty);
    let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(std::fs::read(&archive).unwrap(), original);
}

#[test]
#[ignore = "private fixtures logos/x360-nhllegacy/** + imagery/x360-nhllegacy/**: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn xbox_legacy_packages_in_a_synthetic_cache_big_flow_through_the_archive_route() {
    run_synthetic("xbox", "xbox");
}

#[test]
#[ignore = "private fixtures logos/ps3-nhllegacy/** + imagery/ps3-nhllegacy/**: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn ps3_legacy_packages_in_a_synthetic_cache_big_flow_through_the_archive_route() {
    run_synthetic("ps3", "ps3");
}

/// The gen7 archive binding per title/platform (no fixtures): NHL 11 Xbox → `fe.big`;
/// 12–Legacy Xbox / PS3 / Recompiled → `cache.big`; 07–10 (BIG4) and NHL 11 PS3 (loose-only) → none;
/// with a root, only when the archive file exists.
#[test]
fn gen7_archive_binding_per_title_and_platform() {
    use studio_domain::league_logos::gen7_logo_archive;
    use studio_domain::Platform;
    assert_eq!(gen7_logo_archive(Some("nhl11"), Some(Platform::Xbox)).as_deref(), Some("fe.big"));
    for g in ["nhl12", "nhl13", "nhl14", "nhl15", "nhllegacy"] {
        assert_eq!(gen7_logo_archive(Some(g), Some(Platform::Xbox)).as_deref(), Some("cache.big"), "{g}");
        assert_eq!(gen7_logo_archive(Some(g), Some(Platform::Ps3)).as_deref(), Some("cache.big"), "{g}");
        assert_eq!(gen7_logo_archive(Some(g), Some(Platform::Recomp)).as_deref(), Some("cache.big"), "{g}");
    }
    // Receipt 59: a Recompiled project carries the catalogue id `legacyrecomp`.
    assert_eq!(gen7_logo_archive(Some("legacyrecomp"), Some(Platform::Recomp)).as_deref(), Some("cache.big"));
    assert_eq!(gen7_logo_archive(Some("legacyrecomp"), Some(Platform::Xbox)), None);
    for (g, x, p) in [("nhl07", Some("data0.big"), None), ("nhl08", Some("data0.big"), Some("data0.big")), ("nhl09", Some("fe.big"), Some("fe.big")), ("nhl10", Some("fe.big"), Some("fe.big"))] {
        assert_eq!(gen7_logo_archive(Some(g), Some(Platform::Xbox)).as_deref(), x, "{g}");
        assert_eq!(gen7_logo_archive(Some(g), Some(Platform::Ps3)).as_deref(), p, "{g}");
        assert!(studio_domain::league_logos::gen7_logo_archive_is_big4(Some(g)));
    }
    assert_eq!(gen7_logo_archive(Some("nhl11"), Some(Platform::Ps3)), None);
    assert_eq!(gen7_logo_archive(Some("nhl21"), Some(Platform::Ps4)), None);
    // With a root: the archive must exist there.
    let tmp = Tmp::new("binding");
    let db = tmp.0.join("nhlng.db");
    std::fs::copy(repo("anchors/xbox360/nhllegacy/databases/nhlng.db"), &db).unwrap();
    let reg = Registry::default();
    let id = open(&reg, "xbox", &db);
    let ll = reg.league_logo(id, 0, Some(&s(&tmp.0))).unwrap().unwrap();
    assert!(ll.variants.iter().all(|v| v.archive.is_none()), "no cache.big under the root → no archive route");
    std::fs::write(tmp.0.join("cache.big"), b"EB\0\x03").unwrap();
    let ll = reg.league_logo(id, 0, Some(&s(&tmp.0))).unwrap().unwrap();
    assert!(ll.variants.iter().all(|v| v.archive.as_deref() == Some("cache.big")));
    let im = reg.league_imagery(id, Some(&s(&tmp.0))).unwrap().unwrap();
    assert!(im.families.iter().flat_map(|f| f.members.iter()).all(|m| m.archive.as_deref() == Some("cache.big")));
    assert!(im.notes.iter().any(|n| n.contains("research unknown")), "{:?}", im.notes);
}

// ------------------------------------------------------------------ real gen7 archives (opt-in)

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

fn table_of(p: &Path) -> (tdb_archive_big::eb::EbArchive, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(p).unwrap();
    let mut hdr = vec![0u8; tdb_archive_big::eb::HEADER_LEN];
    f.read_exact(&mut hdr).unwrap();
    let h = tdb_archive_big::eb::EbHeader::parse(&hdr).unwrap();
    let mut table = vec![0u8; h.table_end()];
    f.seek(SeekFrom::Start(0)).unwrap();
    f.read_exact(&mut table).unwrap();
    (tdb_archive_big::eb::EbArchive::parse(&table).unwrap(), table)
}

fn read_span(p: &Path, off: u64, len: usize) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(p).unwrap();
    f.seek(SeekFrom::Start(off)).unwrap();
    let mut b = vec![0u8; len];
    f.read_exact(&mut b).unwrap();
    b
}

struct Gen7Target {
    key: &'static str,
    platform: &'static str,
    game_id: &'static str,
    archive: &'static str,
    archive_name: &'static str,
}

const GEN7_TARGETS: &[Gen7Target] = &[
    Gen7Target { key: "xbox-nhl11", platform: "xbox", game_id: "nhl11", archive: r"X:\Games\Emulation\Roms\XBOX360\NHL 11\NHL 11\fe.big", archive_name: "fe.big" },
    Gen7Target { key: "xbox-nhl12", platform: "xbox", game_id: "nhl12", archive: r"X:\Games\Emulation\Roms\XBOX360\NHL 12\NHL 12\cache.big", archive_name: "cache.big" },
    Gen7Target { key: "xbox-nhl13", platform: "xbox", game_id: "nhl13", archive: r"X:\Games\Emulation\Roms\XBOX360\NHL 13\NHL 13\cache.big", archive_name: "cache.big" },
    Gen7Target { key: "xbox-nhl14", platform: "xbox", game_id: "nhl14", archive: r"X:\Games\Emulation\Roms\XBOX360\NHL 14\NHL 14\cache.big", archive_name: "cache.big" },
    Gen7Target { key: "xbox-nhl15", platform: "xbox", game_id: "nhl15", archive: r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15\cache.big", archive_name: "cache.big" },
    Gen7Target { key: "xbox-nhllegacy", platform: "xbox", game_id: "nhllegacy", archive: r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\cache.big", archive_name: "cache.big" },
    Gen7Target { key: "ps3-nhl12", platform: "ps3", game_id: "nhl12", archive: "X:\\Games\\Emulation\\Roms\\Sony Playstation 3\\NHL 12\\[BLUS 30791] NHL\u{ae} 12\\PS3_GAME\\USRDIR\\cache.big", archive_name: "cache.big" },
    Gen7Target { key: "ps3-nhl15", platform: "ps3", game_id: "nhl15", archive: r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 15\PS3_GAME\USRDIR\cache.big", archive_name: "cache.big" },
    Gen7Target { key: "ps3-nhllegacy", platform: "ps3", game_id: "nhllegacy", archive: r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL Legacy - Vanilla\PS3_GAME\USRDIR\cache.big", archive_name: "cache.big" },
];

/// The route on the **real** gen7 EB archive of each selected target (`NHL_STUDIO_GEN7_TARGETS`,
/// comma list of the keys above; default `xbox-nhllegacy,ps3-nhllegacy`), on a disposable copy:
/// the `leaguelogos/l0`, `divisionlogos/d0` (lossless, fit-in-slot) and `leaguelogossmall/l0`
/// (DXT5, grows → append-and-retarget) packages replaced with marked images in one copied-target
/// save; only the three rows and (on growth) the archive-size word change in the
/// table; every other payload is untouched (the stager's verify); the installed packages are
/// stock-shaped and decode to the marked pixels; restore byte-exact. A missing archive fails.
#[test]
#[ignore = "real Xbox 360 / PS3 installs under X:\\Games\\Emulation\\Roms + private fixtures: set NHL_STUDIO_FIXTURES and select with --ignored; copies up to ~450 MB per target into %TEMP%"]
fn every_real_gen7_eb_archive_takes_a_logo_and_a_division_logo_package_through_the_copied_target_save() {
    let selected: Vec<String> = std::env::var("NHL_STUDIO_GEN7_TARGETS")
        .map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_else(|_| vec!["xbox-nhllegacy".into(), "ps3-nhllegacy".into()]);
    for key in &selected {
        let t = GEN7_TARGETS.iter().find(|t| t.key == key).unwrap_or_else(|| panic!("unknown target {key}"));
        let t0 = std::time::Instant::now();
        let source = Path::new(t.archive);
        assert!(source.is_file(), "{key}: {} is not present (a missing install fails, it does not skip)", source.display());
        let tmp = Tmp::new(&format!("real-{key}"));
        let root = tmp.0.join("copy");
        studio_domain::tdb_txn::engine::create_copied_target_root(&root).unwrap();
        let anchors = if t.platform == "xbox" { "xbox360" } else { "ps3" };
        let db = root.join("nhlng.db");
        std::fs::copy(repo(&format!("anchors/{anchors}/{}/databases/nhlng.db", t.game_id)), &db).unwrap();
        let archive = root.join(t.archive_name);
        std::fs::copy(source, &archive).unwrap();
        let original_len = std::fs::metadata(&archive).unwrap().len();
        let original_sha = file_sha256(&archive);
        let (orig_arc, orig_table) = table_of(&archive);
        eprintln!("{key}: copied {} ({original_len} B, {} entries) in {:?}", t.archive_name, orig_arc.entries.len(), t0.elapsed());

        let reg = Registry::default();
        let id = reg
            .open_workspace_from_paths(Some(t.game_id), parse_platform(t.platform).unwrap(), &s(&db), None, &s(&repo(&format!("anchors/{anchors}/{}/databases/nhlng-meta.xml", t.game_id))), &s(&repo("defs")))
            .unwrap()
            .project_id;
        let root_s = s(&root);
        // The listing binds the packages to this archive.
        let ll = reg.league_logo(id, 0, Some(&root_s)).unwrap().expect("league 0 resolves");
        assert!(ll.variants.iter().all(|v| v.archive.as_deref() == Some(t.archive_name)), "{key}");

        let mut planned = Vec::new();
        // The DXT5 small logo is the growth case (Studio's RefPack stream is longer than EA's and
        // the re-planned package grows): append-and-retarget on every measured slot.
        // `NHL_STUDIO_GEN7_MEMBERS` (comma list of `path[:bound]`, C9) selects other members of the
        // same shape — e.g. a team's `teamlogos/t<artid>.big` + `teamlogossmall/s<artid>.big`.
        let members: Vec<(String, u8, f64)> = match std::env::var("NHL_STUDIO_GEN7_MEMBERS") {
            Ok(v) => v
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .enumerate()
                .map(|(i, s)| {
                    let (path, bound) = s.trim().rsplit_once(':').filter(|(_, b)| b.parse::<f64>().is_ok()).map(|(p, b)| (p.to_string(), b.parse::<f64>().unwrap())).unwrap_or((s.trim().to_string(), 8.0));
                    (path, 10 + (i as u8) * 60, bound)
                })
                .collect(),
            Err(_) => vec![(LOGO.to_string(), 10u8, 0.5f64), (DIV.to_string(), 200u8, 0.5), (SMALL.to_string(), 77u8, 8.0)],
        };
        for (path, seed, bound) in members {
            let path = path.as_str();
            let e = orig_arc.entries.iter().find(|e| e.path == path).unwrap_or_else(|| panic!("{key}: {path} not in the archive"));
            let pkg = read_span(&archive, e.offset, e.size as usize);
            let (src, edit) = marked_source(&pkg, seed, &tmp.0);
            let p = reg.asset_member_preview(id, &root_s, t.archive_name, path, &s(&src)).unwrap();
            let plan = p.plan.unwrap_or_else(|| panic!("{key}: {path} refused {:?} {:?}", p.refusal_kind, p.refusal));
            assert_eq!(plan.stored_form, "raw", "{key}: {path}");
            assert!(plan.shared_with.is_empty(), "{key}: {path}: {:?}", plan.shared_with);
            eprintln!("{key}: {path}: {}", plan.placement_prediction);
            reg.asset_member_apply_review(id, &plan.ticket).unwrap();
            planned.push((path.to_string(), pkg, edit, bound));
        }
        let review = reg.copied_target_review(id, &root_s).unwrap_or_else(|e| panic!("{key}: review: {e}"));
        assert!(review.artifacts.iter().any(|a| a.key == "archive-asset-0"));
        eprintln!("{key}: staged in {:?}", t0.elapsed());
        let apply = reg.copied_target_apply(id, review.handle).unwrap_or_else(|e| panic!("{key}: apply: {e}"));
        assert_eq!(apply.state, "committed", "{key}: {}", apply.state);
        eprintln!("{key}: committed in {:?}", t0.elapsed());

        let (new_arc, new_table) = table_of(&archive);
        assert_eq!(new_table.len(), orig_table.len());
        let changed_rows: Vec<usize> = (0..orig_arc.entries.len()).filter(|i| orig_table[tdb_archive_big::eb::FILE_TABLE_OFFSET + i * 16..tdb_archive_big::eb::FILE_TABLE_OFFSET + (i + 1) * 16] != new_table[tdb_archive_big::eb::FILE_TABLE_OFFSET + i * 16..tdb_archive_big::eb::FILE_TABLE_OFFSET + (i + 1) * 16]).collect();
        let mut expect_rows: Vec<usize> = planned.iter().map(|(p, _, _, _)| orig_arc.entries.iter().position(|e| e.path == *p).unwrap()).collect();
        expect_rows.sort_unstable();
        assert_eq!(changed_rows, expect_rows, "{key}: exactly the planned rows changed");
        for (i, (a, b)) in orig_table[..tdb_archive_big::eb::FILE_TABLE_OFFSET].iter().zip(&new_table[..tdb_archive_big::eb::FILE_TABLE_OFFSET]).enumerate() {
            if a != b {
                assert!((0x18..0x20).contains(&i), "{key}: header byte {i} changed");
            }
        }
        for (path, pkg, edit, bound) in &planned {
            let e = new_arc.entries.iter().find(|e| e.path == *path).unwrap();
            let installed = read_span(&archive, e.offset, e.size as usize);
            let l0 = PackageLayout::parse(pkg).unwrap();
            let l1 = PackageLayout::parse(&installed).unwrap_or_else(|e| panic!("{key} {path}: {e}"));
            assert_eq!(&installed[l1.table_end..l1.data_offset as usize], &pkg[l0.table_end..l0.data_offset as usize], "{key} {path}: trailer");
            assert_eq!(&installed[installed.len() - l1.tail_len..], &pkg[pkg.len() - l0.tail_len..], "{key} {path}: tail");
            let tt = tmp.0.join("installed.big");
            std::fs::write(&tt, &installed).unwrap();
            let (_, img) = studio_domain::decode_texture_file(&tt, None).unwrap();
            let d = mad(&img.rgba, &edit.rgba);
            assert!(d <= *bound, "{key} {path}: installed pixels MAD {d}");
        }
        assert!(!reg.summary(id).unwrap().dirty);
        let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
        assert_eq!(restored.state, "restored", "{key}");
        assert_eq!(std::fs::metadata(&archive).unwrap().len(), original_len);
        assert_eq!(file_sha256(&archive), original_sha, "{key}: restore byte-exact");
        eprintln!("{key}: restored in {:?}", t0.elapsed());
    }
}

/// P3 (21 Sep 2026, research handoff 06 / r30): a **chunklzx-stored** package member (the form of
/// the gen7 Xbox 360 `rendering/*` and `attribdb/*` members) takes the store route — the
/// preview names the form, the growth and the R30-LZX-1 qualification; the installed member is a
/// `chunklzx` container of LZX uncompressed blocks that decodes to the marked package; no
/// `chunkzip` substitution; restore byte-exact. The source member is the real Legacy XPR2 logo
/// package, store-encoded (the decoder reads it exactly as it reads the shipped members).
#[test]
#[ignore = "private fixtures logos/x360-nhllegacy/**: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn a_chunklzx_member_is_written_back_through_the_store_route() {
    let root_fx = fixture_root();
    let logo = std::fs::read(root_fx.join("logos/x360-nhllegacy/leaguelogos/l0.big")).unwrap();
    let small = std::fs::read(root_fx.join("logos/x360-nhllegacy/leaguelogossmall/l0.big")).unwrap();
    let stored = studio_domain::tdb_archive_codec::compress_chunklzx_store(&logo).unwrap();
    assert!(stored.starts_with(b"chunklzx") && stored.len() > logo.len());
    let tmp = Tmp::new("lzx-store");
    let (root, db, archive) = build_root(&tmp, "xbox", "cache.big", &stored, &small, &small);
    let original = std::fs::read(&archive).unwrap();
    let reg = Registry::default();
    let id = open(&reg, "xbox", &db);
    let root_s = s(&root);
    let (logo_src, logo_edit) = marked_source(&logo, 33, &tmp.0);
    let p = reg.asset_member_preview(id, &root_s, "cache.big", LOGO, &s(&logo_src)).unwrap();
    let plan = p.plan.unwrap_or_else(|| panic!("{:?} {:?}", p.refusal_kind, p.refusal));
    assert_eq!(plan.stored_form, "chunklzx-store");
    assert!(plan.placement_prediction.contains("R30-LZX-1") && plan.placement_prediction.contains("uncompressed blocks"), "{}", plan.placement_prediction);
    assert!(!plan.placement_prediction.contains("chunkzip"));
    reg.asset_member_apply_review(id, &plan.ticket).unwrap();
    let review = reg.copied_target_review(id, &root_s).unwrap();
    let apply = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(apply.state, "committed", "{}", apply.state);
    let (_, installed_raw) = {
        let (a, table) = table_of(&archive);
        let _ = table;
        let e = a.entries.iter().find(|e| e.path == LOGO).unwrap();
        (e.offset, read_span(&archive, e.offset, e.size as usize))
    };
    assert!(installed_raw.starts_with(b"chunklzx"), "the installed member keeps the chunklzx container");
    let installed = studio_domain::tdb_archive_codec::decompress_chunklzx(&installed_raw).unwrap();
    assert_eq!(u32::from_be_bytes(installed_raw[12..16].try_into().unwrap()) as usize, installed.len());
    let l1 = PackageLayout::parse(&installed).unwrap();
    assert_eq!(l1.members.len(), PackageLayout::parse(&logo).unwrap().members.len());
    let t = tmp.0.join("installed.big");
    std::fs::write(&t, &installed).unwrap();
    let (_, img) = studio_domain::decode_texture_file(&t, None).unwrap();
    assert!(mad(&img.rgba, &logo_edit.rgba) <= 0.0, "pixels");
    // Unrelated members untouched; restore exact.
    assert_eq!(member_bytes(&archive, "fe/data/other/first.bin").1, b"FIRST-MEMBER");
    assert!(!reg.summary(id).unwrap().dirty);
    let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(std::fs::read(&archive).unwrap(), original);
}
