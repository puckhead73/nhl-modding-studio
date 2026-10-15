//! C1 (overnight 2026-09-20) — the **loose-file** texture route (`Registry::import_texture`, what
//! the League logo / presentation imagery "Replace…" dialog dispatches) exercised with a **visibly
//! different** image on every real league-logo package and PS4 member of the private fixture set,
//! not the same-pixel codec round trip of receipt 06. Each case: the package at its install-relative
//! path in a temp root → `texture_preview` (the dialog's current-texture query) → `import_texture`
//! with backup → the `.bak` equals the original → `texture_preview` of the written file shows the
//! new pixels (exact for the lossless layouts, bounded for DXT5/BC7, and the drawn mark is present
//! in every case) → the package shape is preserved (span-preserving re-plan: same members, same
//! trailer and tail, every other member byte-identical) → a second write with another image
//! succeeds and the first `.bak` is kept. A synthetic BIGF/RW4ps3 case runs by default; the real
//! sweep runs `--ignored` under `NHL_STUDIO_FIXTURES`. Nothing here is evidence a game accepts the
//! result; runtime precedence (loose vs archive) stays a user card.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use studio_api::Registry;
use tdb_archive_big::replan::PackageLayout;

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-c1-loose-{tag}-{}", std::process::id()));
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

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// Plan, then write under that review — the dialog's two-step dispatch (24 Sep 2026 review F1).
fn reviewed_import(reg: &Registry, install_dir: &str, rel: &str, src: &str, backup: bool, idx: Option<u32>) -> Result<studio_api::ImportTextureSummaryDto, studio_api::ApiError> {
    let plan = reg.texture_replace_plan(install_dir, rel, src, idx)?;
    reg.import_texture(install_dir, rel, src, backup, idx, plan.review_id)
}

/// Mean absolute per-channel difference.
fn mad(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(x, y)| (*x as i32 - *y as i32).abs() as f64).sum::<f64>() / a.len().max(1) as f64
}

/// A visibly edited variant of `img`: a solid magenta band across the middle third and an
/// inverted top-left quadrant — unmistakable against any shipped logo.
fn marked(img: &studio_domain::DecodedImage, seed: u8) -> studio_domain::DecodedImage {
    let (w, h) = (img.width, img.height);
    let mut rgba = img.rgba.clone();
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            if y >= h / 3 && y < 2 * h / 3 {
                rgba[i..i + 4].copy_from_slice(&[255, seed, 255, 255]);
            } else if x < w / 2 && y < h / 3 {
                rgba[i] = 255 - rgba[i];
                rgba[i + 1] = 255 - rgba[i + 1];
                rgba[i + 2] = 255 - rgba[i + 2];
                rgba[i + 3] = 255;
            }
        }
    }
    studio_domain::DecodedImage { width: w, height: h, rgba }
}

/// The band's mean colour in `rgba` — the mark-presence check.
fn band_mean(img: &studio_domain::DecodedImage) -> [f64; 3] {
    let (w, h) = (img.width, img.height);
    let mut acc = [0f64; 3];
    let mut n = 0f64;
    for y in h / 3..2 * h / 3 {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            for c in 0..3 {
                acc[c] += img.rgba[i + c] as f64;
            }
            n += 1.0;
        }
    }
    [acc[0] / n, acc[1] / n, acc[2] / n]
}

fn decode_data_url(url: &str) -> studio_domain::DecodedImage {
    use base64::Engine;
    let b64 = url.split_once(",").unwrap().1;
    let png = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
    studio_domain::decode_source_image_bytes(&png).unwrap()
}

/// Package shape facts that a loose write must keep: member names/sizes of the untouched
/// members, their bytes, the table trailer and the tail.
fn package_shape(bytes: &[u8]) -> Option<(PackageLayout, Vec<u8>)> {
    let pkg = if tdb_archive_codec::is_refpack(bytes) { tdb_archive_codec::refpack_decompress(bytes).ok()? } else { bytes.to_vec() };
    if !pkg.starts_with(b"BIGF") && !pkg.starts_with(b"BIG4") {
        return None;
    }
    let layout = PackageLayout::parse(&pkg).ok()?;
    Some((layout, pkg))
}

/// One loose replace on `stored` placed at `rel` under a fresh root; returns the written bytes.
fn replace_once(reg: &Registry, root: &Path, rel: &str, stored: &[u8], seed: u8, label: &str, block_bound: f64) -> Vec<u8> {
    let target = root.join(rel);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, stored).unwrap();
    let bak = PathBuf::from(format!("{}.bak", s(&target)));
    let _ = std::fs::remove_file(&bak);
    let norm = |p: &str| p.replace('\\', "/");
    let before = reg.texture_preview(&s(root), rel, 0, None).unwrap_or_else(|e| panic!("{label}: preview: {e}"));
    let original = decode_data_url(&before.data_url);
    assert_eq!((original.width, original.height), (before.width, before.height));
    let edited = marked(&original, seed);
    let src = root.join(format!("edited-{seed}.png"));
    std::fs::write(&src, edited.to_png().unwrap()).unwrap();
    let r = reviewed_import(&reg, &s(root), rel, &s(&src), true, None).unwrap_or_else(|e| panic!("{label}: import: {e}"));
    assert_eq!(r.backup_path.as_deref().map(norm), Some(norm(&s(&bak))), "{label}");
    assert_eq!(std::fs::read(&bak).unwrap(), stored, "{label}: the .bak is the original");
    assert!(r.notes.is_empty(), "{label}: writer notes {:?}", r.notes);
    let written = std::fs::read(&target).unwrap();
    assert_eq!(r.bytes_written as usize, written.len());
    let after = reg.texture_preview(&s(root), rel, 0, None).unwrap_or_else(|e| panic!("{label}: re-preview: {e}"));
    assert_eq!((after.width, after.height), (before.width, before.height), "{label}: dims");
    assert_eq!(after.container, before.container, "{label}: container");
    let got = decode_data_url(&after.data_url);
    let err = mad(&got.rgba, &edited.rgba);
    assert!(err <= block_bound, "{label}: written pixels differ from the edited image (MAD {err} > {block_bound})");
    let far = mad(&got.rgba, &original.rgba);
    assert!(far > 20.0, "{label}: the written file still looks like the original (MAD {far})");
    let m = band_mean(&got);
    assert!(m[0] > 200.0 && m[2] > 200.0 && m[1] < 90.0 + seed as f64, "{label}: the magenta band is not there ({m:?})");
    // Package shape.
    if let Some((l0, p0)) = package_shape(stored) {
        let (l1, p1) = package_shape(&written).unwrap_or_else(|| panic!("{label}: written package does not parse"));
        assert_eq!(l1.members.len(), l0.members.len(), "{label}");
        assert_eq!(l1.data_offset, l0.data_offset, "{label}");
        assert_eq!(l1.tail_len, l0.tail_len, "{label}: tail");
        assert_eq!(&p1[p1.len() - l1.tail_len..], &p0[p0.len() - l0.tail_len..], "{label}: tail bytes");
        assert_eq!(&p1[l1.table_end..l1.data_offset as usize], &p0[l0.table_end..l0.data_offset as usize], "{label}: trailer");
        // Exactly one member (the texture — not necessarily the largest: a `tblank` placeholder's
        // texture is smaller than its Apt data) changes; every member placed before it keeps its
        // offset, every other member keeps its bytes.
        let changed: Vec<usize> = l0
            .members
            .iter()
            .zip(&l1.members)
            .enumerate()
            .filter(|(_, (a, b))| {
                assert_eq!(a.name, b.name, "{label}");
                p0[a.offset as usize..(a.offset + a.size) as usize] != p1[b.offset as usize..(b.offset + b.size) as usize]
            })
            .map(|(i, _)| i)
            .collect();
        assert_eq!(changed.len(), 1, "{label}: exactly the texture member changed ({changed:?})");
        let tex = &l0.members[changed[0]];
        for (a, b) in l0.members.iter().zip(&l1.members) {
            if a.name != tex.name && a.offset <= tex.offset {
                assert_eq!(a.offset, b.offset, "{label}: member {} moved", a.name);
            }
        }
    }
    written
}

// ------------------------------------------------------------------ synthetic (default)

/// A logo-layout RW4ps3 ARGB8 texture inside an X-C-shaped package (as `league_logo_packages.rs`
/// builds it) through the API route.
#[test]
fn a_synthetic_ps3_logo_package_takes_a_marked_image_through_import_texture() {
    let tmp = Tmp::new("synthetic");
    let (w, h) = (64u32, 32u32);
    // RW4ps3 container: header 0x200, one ARGB8 slot.
    let hdr = 0x200usize;
    let slot: Vec<u8> = (0..w * h).flat_map(|i| [255u8, (i % 64) as u8 * 4, (i / 64) as u8 * 8, 0x40]).collect();
    let mut t = vec![0u8; hdr + slot.len()];
    t[..7].copy_from_slice(b"\x89RW4ps3");
    t[0x20..0x24].copy_from_slice(&2u32.to_be_bytes());
    t[0x30..0x34].copy_from_slice(&0x1B4u32.to_be_bytes());
    t[0x44..0x48].copy_from_slice(&(hdr as u32).to_be_bytes());
    let d = 0x17C;
    for (k, sg) in [0x6007u32, 0x6007, 0x8035].iter().enumerate() {
        t[d + 32 + k * 4..d + 36 + k * 4].copy_from_slice(&sg.to_be_bytes());
    }
    t[d + 48..d + 50].copy_from_slice(&(w as u16).to_be_bytes());
    t[d + 50..d + 52].copy_from_slice(&(h as u16).to_be_bytes());
    t[d + 52] = 1;
    let tb = 0x1B4;
    for (k, v) in [0u32, 0, slot.len() as u32, 0x80, 1].iter().enumerate() {
        t[tb + k * 4..tb + k * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    t[tb + 20..tb + 24].copy_from_slice(&[0, 1, 0, 0x30]);
    for (k, v) in [d as u32, 0, 0x38, 4, 0xA].iter().enumerate() {
        t[tb + 24 + k * 4..tb + 28 + k * 4].copy_from_slice(&v.to_be_bytes());
    }
    t[tb + 44..tb + 48].copy_from_slice(&[0, 2, 0, 3]);
    t[hdr..].copy_from_slice(&slot);
    // Package: 0, 3, sg1, 2, sg2, 1 with stride 0, trailer, 16-byte opaque tail.
    let names = ["0", "3", "sg1", "2", "sg2", "1"];
    let sizes = [240u32, 288, 0, t.len() as u32, 0, 32];
    let table_end = 16 + names.iter().map(|n| 8 + n.len() + 1).sum::<usize>();
    let data_offset = table_end + 8;
    let a64 = |x: usize| x.div_ceil(64) * 64;
    let o0 = a64(data_offset);
    let o3 = a64(o0 + 240);
    let sg1 = a64(o3 + 288);
    let o2 = sg1;
    let sg2 = a64(o2 + t.len());
    let o1 = sg2;
    let last = o1 + 32;
    let offs = [o0, o3, sg1, o2, sg2, o1];
    let mut pkg = vec![0u8; last + 16];
    pkg[..4].copy_from_slice(b"BIGF");
    pkg[4..8].copy_from_slice(&(last as u32).to_le_bytes());
    pkg[8..12].copy_from_slice(&6u32.to_be_bytes());
    pkg[12..16].copy_from_slice(&(data_offset as u32).to_be_bytes());
    let mut c = 16;
    for (k, n) in names.iter().enumerate() {
        pkg[c..c + 4].copy_from_slice(&(offs[k] as u32).to_be_bytes());
        pkg[c + 4..c + 8].copy_from_slice(&sizes[k].to_be_bytes());
        c += 8;
        pkg[c..c + n.len()].copy_from_slice(n.as_bytes());
        c += n.len() + 1;
    }
    pkg[table_end..table_end + 8].copy_from_slice(b"L286\x15\x05\x00\x01");
    pkg[o0..o0 + 14].copy_from_slice(b"Apt Data:1:5:4");
    pkg[o3..o3 + 4].copy_from_slice(&[0, 0, 1, 0x20]);
    pkg[o2..o2 + t.len()].copy_from_slice(&t);
    pkg[o1..o1 + 17].copy_from_slice(b"Apt constant file");
    for (k, b) in pkg[last..].iter_mut().enumerate() {
        *b = 0xD0 | k as u8;
    }
    let reg = Registry::default();
    let rel = "USRDIR/fe/ion/artassets/leaguelogos/l0.big";
    let first = replace_once(&reg, &tmp.0, rel, &pkg, 10, "synthetic ps3", 0.0);
    // A second write on the written file: the .bak stays the original.
    let target = tmp.0.join(rel);
    let second = replace_once_no_reset(&reg, &tmp.0, rel, 200, "synthetic ps3 #2");
    assert_ne!(first, second);
    assert_eq!(std::fs::read(format!("{}.bak", s(&target))).unwrap(), pkg, "the first .bak is kept");
}

/// A further write on an already-written file (no reset of the target or `.bak`).
fn replace_once_no_reset(reg: &Registry, root: &Path, rel: &str, seed: u8, label: &str) -> Vec<u8> {
    let before = reg.texture_preview(&s(root), rel, 0, None).unwrap();
    let edited = marked(&decode_data_url(&before.data_url), seed);
    let src = root.join(format!("edited-{seed}.png"));
    std::fs::write(&src, edited.to_png().unwrap()).unwrap();
    let r = reviewed_import(&reg, &s(root), rel, &s(&src), true, None).unwrap_or_else(|e| panic!("{label}: import: {e}"));
    assert!(r.backup_path.is_some());
    let after = decode_data_url(&reg.texture_preview(&s(root), rel, 0, None).unwrap().data_url);
    assert!(mad(&after.rgba, &edited.rgba) <= 12.0, "{label}");
    std::fs::read(root.join(rel)).unwrap()
}

// ------------------------------------------------------------------ R1 F1: refusal through the public route

/// A solid-colour PNG of `w × h` on disk, for the resize requests.
fn flat_png(dir: &Path, name: &str, w: u32, h: u32, rgb: [u8; 3]) -> PathBuf {
    let rgba: Vec<u8> = (0..(w as usize * h as usize)).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect();
    let png = studio_domain::DecodedImage { width: w, height: h, rgba }.to_png().unwrap();
    let p = dir.join(name);
    std::fs::write(&p, png).unwrap();
    p
}

/// A refused resize through `Registry::import_texture` (the loose "Replace…" route) leaves the
/// target's bytes and the backup state untouched, and the refusal names the requested value —
/// never a masked one. Then a legitimate changed-dimension replace on the same target succeeds and
/// reads back at the requested size. Coordinator review R1 F1 (21 Sep 2026): the GNF T# stores
/// 14-bit dimensions (16 400 was masked to 16), the `RW4xb2` descriptor 13-bit ones (8 196 to 4).
#[test]
fn an_unrepresentable_resize_is_refused_through_import_texture_and_the_target_is_untouched() {
    let tmp = Tmp::new("r1-f1");
    let reg = Registry::default();
    let root = &tmp.0;
    let norm = |p: &str| p.replace('\\', "/");

    // ---- GNF (PS4): a rule-conformant 32 × 32 BC1 source.
    let base: Vec<u8> = (0..32u32 * 32 / 16 * 8).map(|i| (i * 7) as u8).collect();
    let gnf = tdb_rx2::gnf::encode_gnf(tdb_rx2::gnf::BcFormat::Bc1, 32, 32, &base);
    assert_eq!(tdb_rx2::gnf::check_resize_rule(&gnf).unwrap(), (32, 32, tdb_rx2::gnf::BcFormat::Bc1, 1));
    let gnf_rel = "data/textures/t0.gnf";
    let gnf_target = root.join(gnf_rel);
    std::fs::create_dir_all(gnf_target.parent().unwrap()).unwrap();
    std::fs::write(&gnf_target, &gnf).unwrap();
    let gnf_bak = PathBuf::from(format!("{}.bak", s(&gnf_target)));

    let wide = flat_png(root, "wide-16400x4.png", 16_400, 4, [200, 30, 30]);
    let err = match reviewed_import(&reg, &s(root), gnf_rel, &s(&wide), true, None) {
        Ok(r) => panic!("16 400 wide must be refused, got {}x{}", r.width, r.height),
        Err(e) => e,
    };
    let text = err.to_string();
    assert!(text.contains("16400") && text.contains("16384"), "the refusal names the request and the bound: {text}");
    assert_eq!(std::fs::read(&gnf_target).unwrap(), gnf, "GNF target bytes untouched by the refusal");
    assert!(!gnf_bak.exists(), "no backup is written for a refused import");
    let still = reg.texture_preview(&s(root), gnf_rel, 0, None).unwrap();
    assert_eq!((still.width, still.height), (32, 32));

    // The boundary itself is accepted (16 384 wide, 4 high) and reads back exactly …
    let edge = flat_png(root, "edge-16384x4.png", 16_384, 4, [10, 200, 10]);
    let r = reviewed_import(&reg, &s(root), gnf_rel, &s(&edge), true, None).expect("16 384 wide is representable");
    assert_eq!((r.width, r.height), (16_384, 4));
    assert_eq!(r.backup_path.as_deref().map(norm), Some(norm(&s(&gnf_bak))));
    assert_eq!(std::fs::read(&gnf_bak).unwrap(), gnf, "the .bak is the pristine 32 × 32 source");
    let written = std::fs::read(&gnf_target).unwrap();
    assert_eq!(tdb_rx2::gnf::check_resize_rule(&written).unwrap(), (16_384, 4, tdb_rx2::gnf::BcFormat::Bc1, 1));
    let after = reg.texture_preview(&s(root), gnf_rel, 0, None).unwrap();
    assert_eq!((after.width, after.height), (16_384, 4));
    let got = decode_data_url(&after.data_url);
    let m = band_mean(&got);
    assert!(m[1] > 180.0 && m[0] < 40.0 && m[2] < 40.0, "the written pixels are the green source ({m:?})");
    // … and a meaningful ordinary change of dimensions (64 × 16 from 16 384 × 4) too.
    let mid = flat_png(root, "mid-64x16.png", 64, 16, [20, 20, 220]);
    let r = reviewed_import(&reg, &s(root), gnf_rel, &s(&mid), true, None).expect("64 × 16");
    assert_eq!((r.width, r.height), (64, 16));
    assert_eq!(std::fs::read(&gnf_bak).unwrap(), gnf, "the first .bak is kept");
    let after = reg.texture_preview(&s(root), gnf_rel, 0, None).unwrap();
    assert_eq!((after.width, after.height), (64, 16));
    let m = band_mean(&decode_data_url(&after.data_url));
    assert!(m[2] > 180.0 && m[0] < 40.0 && m[1] < 40.0, "{m:?}");

    // ---- RW4xb2 (Xbox 360): a synthetic two-raster texlib, 256 × 256 DXT1 + 128 × 128 DXT1.
    use tdb_rx2::texlib::{base_mip_byte_size, PixelFormat};
    let a = vec![0u8; base_mip_byte_size(PixelFormat::Dxt1, 256, 256)];
    let b: Vec<u8> = (0..base_mip_byte_size(PixelFormat::Dxt1, 128, 128)).map(|i| 0x30 | (i as u8 & 0xF)).collect();
    let lib = tdb_rx2::texlib::encode_synthetic_texlib(&[(256, 256, PixelFormat::Dxt1, &a), (128, 128, PixelFormat::Dxt1, &b)]);
    let lib_rel = "rendering/jersey/texlib_r1.rx2";
    let lib_target = root.join(lib_rel);
    std::fs::create_dir_all(lib_target.parent().unwrap()).unwrap();
    std::fs::write(&lib_target, &lib).unwrap();
    let lib_bak = PathBuf::from(format!("{}.bak", s(&lib_target)));

    let wide = flat_png(root, "wide-8196x128.png", 8196, 128, [200, 30, 30]);
    let err = match reviewed_import(&reg, &s(root), lib_rel, &s(&wide), true, Some(0)) {
        Ok(r) => panic!("8 196 wide must be refused, got {}x{}", r.width, r.height),
        Err(e) => e,
    };
    let text = err.to_string();
    assert!(text.contains("8196") && text.contains("8192"), "the refusal names the request and the bound: {text}");
    assert_eq!(std::fs::read(&lib_target).unwrap(), lib, "texlib target bytes untouched by the refusal");
    assert!(!lib_bak.exists(), "no backup is written for a refused import");

    // The boundary (8 192 × 128) reflows and reads back; the other raster is byte-identical.
    let edge = flat_png(root, "edge-8192x128.png", 8192, 128, [10, 200, 10]);
    let r = reviewed_import(&reg, &s(root), lib_rel, &s(&edge), true, Some(0)).expect("8 192 wide is representable");
    assert_eq!((r.width, r.height), (8192, 128));
    assert_eq!(std::fs::read(&lib_bak).unwrap(), lib, "the .bak is the pristine source");
    let re = tdb_rx2::texlib::parse(&std::fs::read(&lib_target).unwrap()).unwrap();
    assert_eq!((re.textures[0].width, re.textures[0].height), (8192, 128));
    assert_eq!(re.textures[1].base_mip_linear, tdb_rx2::texlib::parse(&lib).unwrap().textures[1].base_mip_linear, "the other raster is untouched");
    let after = reg.texture_preview(&s(root), lib_rel, 0, Some(0)).unwrap();
    assert_eq!((after.width, after.height), (8192, 128));
    let m = band_mean(&decode_data_url(&after.data_url));
    assert!(m[1] > 180.0 && m[0] < 40.0 && m[2] < 40.0, "{m:?}");
    // An ordinary change of dimensions (512 × 256) on the same target.
    let mid = flat_png(root, "mid-512x256.png", 512, 256, [20, 20, 220]);
    let r = reviewed_import(&reg, &s(root), lib_rel, &s(&mid), true, Some(0)).expect("512 × 256");
    assert_eq!((r.width, r.height), (512, 256));
    let re = tdb_rx2::texlib::parse(&std::fs::read(&lib_target).unwrap()).unwrap();
    assert_eq!((re.textures[0].width, re.textures[0].height), (512, 256));
    assert_eq!(re.textures[1].base_mip_linear, tdb_rx2::texlib::parse(&lib).unwrap().textures[1].base_mip_linear);
    assert_eq!(std::fs::read(&lib_bak).unwrap(), lib, "the first .bak is kept");
}

// ------------------------------------------------------------------ F1 (receipts 10–17 review): the Xbox 360 write boundary

/// Coordinator review of receipts 10–17, F1 (22 Sep 2026): an ordinary **changed-size** write into
/// a **mipped** Xbox 360 raster is refused through the public route — before backup, write or any
/// other mutation — because a resized chain would address packed-tail levels under a layout
/// nothing independent supports, and the chain is not shortened to dodge that. The pre-Apply plan
/// carries the same refusal. At the shipped size the write is a **partial** replacement: the plan
/// and the result disclose it in the same words, the `.bak` is the original, the bytes outside the
/// written large levels are the shipped ones, and the base reads back. A single-level raster keeps
/// its changed-size route (one level re-emitted) with its own disclosure. Nothing here has a
/// pending / history state: the loose route writes files directly, and a refusal leaves none.
#[test]
fn a_changed_size_write_into_a_mipped_xbox_raster_is_refused_before_any_mutation_and_the_same_size_write_is_partial() {
    use tdb_rx2::texlib::{base_mip_byte_size, parse, tiled_buffer_size, LinearMip, PixelFormat};
    let tmp = Tmp::new("f1-boundary");
    let reg = Registry::default();
    let root = &tmp.0;
    let norm = |p: &str| p.replace('\\', "/");

    // A two-raster texlib: 512 × 512 DXT1 grown to a 10-level chain through the codec-level resize
    // (outside the Replace flow — the only way to get a mipped synthetic raster), plus a 128 × 128
    // single-level raster.
    let a = vec![0u8; base_mip_byte_size(PixelFormat::Dxt1, 512, 512)];
    let b: Vec<u8> = (0..base_mip_byte_size(PixelFormat::Dxt1, 128, 128)).map(|i| 0x30 | (i as u8 & 0xF)).collect();
    let flat = tdb_rx2::texlib::encode_synthetic_texlib(&[(512, 512, PixelFormat::Dxt1, &a), (128, 128, PixelFormat::Dxt1, &b)]);
    let lib = parse(&flat).unwrap();
    let chain: Vec<(u32, u32, Vec<u8>)> = (0..10u32)
        .map(|l| {
            let (w, h) = ((512u32 >> l).max(1), (512u32 >> l).max(1));
            (w, h, vec![0x10 + l as u8; base_mip_byte_size(PixelFormat::Dxt1, w, h)])
        })
        .collect();
    let mips: Vec<LinearMip<'_>> = chain.iter().map(|(w, h, d)| LinearMip { width: *w, height: *h, linear_dxt: d }).collect();
    let stored = lib.replace_with_resize(0, PixelFormat::Dxt1, &mips).unwrap();
    let lib = parse(&stored).unwrap();
    assert_eq!(lib.declared_mip_count(0), 10);
    assert_eq!(lib.declared_mip_count(1), 1);
    let slot_start = lib.header_size() as usize + lib.textures[0].data_offset();
    let slot_len = lib.textures[0].buffer_capacity();
    let large_bytes: usize = (0..3).map(|l| tiled_buffer_size(512 >> l, 512 >> l, 8)).sum();

    let rel = "rendering/jersey/texlib_f1.rx2";
    let target = root.join(rel);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, &stored).unwrap();
    let bak = PathBuf::from(format!("{}.bak", s(&target)));

    // ---- Changed size on the mipped raster: the plan refuses …
    let src = flat_png(root, "half-256.png", 256, 256, [200, 30, 30]);
    let plan = reg.texture_replace_plan(&s(root), rel, &s(&src), Some(0)).unwrap();
    assert_eq!(plan.container.as_deref(), Some("texlib"));
    assert!(plan.checked && !plan.allowed, "{plan:?}");
    let reason = plan.refusal.clone().expect("a refusal reason");
    assert!(reason.contains("declares 10 mip levels") && reason.contains("Nothing was written") && reason.contains("512×512"), "{reason}");
    assert!(plan.notes.is_empty());
    // … and both public writers refuse with the same reason, before any mutation.
    let err = match reg.import_texture(&s(root), rel, &s(&src), true, Some(0), plan.review_id) { Ok(_) => panic!("a changed size on a mipped raster must be refused"), Err(e) => e };
    assert!(err.to_string().contains(&reason), "{err}");
    let png = std::fs::read(&src).unwrap();
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    let bplan = reg.texture_replace_plan_bytes(&s(root), rel, &b64, Some(0)).unwrap();
    assert!(!bplan.allowed, "{bplan:?}");
    let err = match reg.import_texture_bytes(&s(root), rel, true, Some(0), bplan.review_id) { Ok(_) => panic!("the bytes twin refuses too"), Err(e) => e };
    assert!(err.to_string().contains("declares 10 mip levels"), "{err}");
    assert_eq!(std::fs::read(&target).unwrap(), stored, "the source bytes are untouched by the refusal");
    assert!(!bak.exists(), "no backup is written for a refused import");
    let still = reg.texture_preview(&s(root), rel, 0, Some(0)).unwrap();
    assert_eq!((still.width, still.height), (512, 512), "the preview still reads the shipped raster");
    // A larger size is refused the same way (the chain is never shortened to fit the rule).
    let big = flat_png(root, "big-1024.png", 1024, 1024, [200, 30, 30]);
    let plan = reg.texture_replace_plan(&s(root), rel, &s(&big), Some(0)).unwrap();
    assert!(!plan.allowed && plan.refusal.as_deref().unwrap().contains("1024×1024 chain"), "{plan:?}");

    // ---- Same size on the mipped raster: a partial replacement, disclosed before and after.
    let same = flat_png(root, "same-512.png", 512, 512, [10, 200, 10]);
    let plan = reg.texture_replace_plan(&s(root), rel, &s(&same), Some(0)).unwrap();
    assert!(plan.checked && plan.allowed && plan.refusal.is_none(), "{plan:?}");
    assert_eq!(plan.notes.len(), 1, "{:?}", plan.notes);
    let note = &plan.notes[0];
    assert!(note.contains("Partial replacement") && note.contains("the base and 2 smaller levels above 64 texels") && note.contains("7 packed-tail levels") && note.contains("not a full-image replacement"), "{note}");
    assert!(!note.contains("gameplay distance"), "no runtime prediction is stated as an observation: {note}");
    let r = reg.import_texture(&s(root), rel, &s(&same), true, Some(0), plan.review_id).expect("same size");
    assert_eq!(r.notes, plan.notes, "the result repeats the plan's disclosure word for word");
    assert_eq!(r.backup_path.as_deref().map(norm), Some(norm(&s(&bak))));
    assert_eq!(std::fs::read(&bak).unwrap(), stored, "the .bak is the original");
    let written = std::fs::read(&target).unwrap();
    assert_eq!(written.len(), stored.len(), "a partial replacement never changes the length");
    // Byte-write mask: everything before the slot, everything after the three large levels of the
    // slot (the seven packed-tail levels and the rest of the file, including raster 1) is shipped.
    assert_eq!(&written[..slot_start], &stored[..slot_start], "bytes before the slot");
    assert_eq!(&written[slot_start + large_bytes..], &stored[slot_start + large_bytes..], "the packed tail and every later byte are the shipped bytes");
    assert_ne!(&written[slot_start..slot_start + large_bytes], &stored[slot_start..slot_start + large_bytes], "the base and large levels changed");
    let re = parse(&written).unwrap();
    assert_eq!(re.declared_mip_count(0), 10, "mip declarations unchanged");
    assert_eq!((re.textures[0].width, re.textures[0].height, re.textures[0].format), (512, 512, PixelFormat::Dxt1), "dimensions and format unchanged");
    assert_eq!(re.textures[0].buffer_capacity(), slot_len, "slot identity unchanged");
    assert_eq!(re.textures[1].base_mip_linear, lib.textures[1].base_mip_linear, "the other raster is untouched");
    let after = reg.texture_preview(&s(root), rel, 0, Some(0)).unwrap();
    let m = band_mean(&decode_data_url(&after.data_url));
    assert!(m[1] > 180.0 && m[0] < 40.0 && m[2] < 40.0, "the base reads back as the green source ({m:?})");

    // ---- A single-level raster keeps its changed-size route, with its own disclosure. (A lone
    // raster: the synthetic builder's second slot carries a `DW8` join the codec rule refuses,
    // which is a fixture limitation, not the route.)
    let single = tdb_rx2::texlib::encode_synthetic_texlib(&[(128, 128, PixelFormat::Dxt1, &b)]);
    let single_rel = "rendering/jersey/texlib_f1_single.rx2";
    std::fs::write(root.join(single_rel), &single).unwrap();
    let small = flat_png(root, "grown-256.png", 256, 256, [20, 20, 220]);
    let plan = reg.texture_replace_plan(&s(root), single_rel, &s(&small), None).unwrap();
    assert!(plan.allowed && plan.notes.len() == 1 && plan.notes[0].contains("declares a single level") && plan.notes[0].contains("no smaller level is written"), "{plan:?}");
    let r = reg.import_texture(&s(root), single_rel, &s(&small), true, None, plan.review_id).expect("single-level resize");
    assert_eq!(r.notes, plan.notes);
    assert_eq!((r.width, r.height), (256, 256));
    let re = parse(&std::fs::read(root.join(single_rel)).unwrap()).unwrap();
    assert_eq!((re.textures[0].width, re.textures[0].height), (256, 256));
    assert_eq!(re.declared_mip_count(0), 1, "one level declared: nothing below the base is addressed");
    assert_eq!(std::fs::read(&bak).unwrap(), stored, "the mipped raster's .bak is kept");

    // ---- A stale plan never governs a write (R4 review F1; rebound 24 Sep 2026 review F1): the plan
    // for the single-level file said "allowed"; the file at that path is then replaced by the mipped
    // raster; the write named that review, so the review's byte binding refuses before any writer
    // rule runs, leaving file and backup state alone.
    let stale_rel = "rendering/jersey/texlib_f1_stale.rx2";
    std::fs::write(root.join(stale_rel), &single).unwrap();
    let plan = reg.texture_replace_plan(&s(root), stale_rel, &s(&small), None).unwrap();
    assert!(plan.allowed, "{plan:?}");
    std::fs::write(root.join(stale_rel), &stored).unwrap();
    let stale_bak = PathBuf::from(format!("{}.bak", s(&root.join(stale_rel))));
    let err = match reg.import_texture(&s(root), stale_rel, &s(&small), true, None, plan.review_id) {
        Ok(_) => panic!("the write must be decided from the current bytes, not the earlier plan"),
        Err(e) => e,
    };
    assert!(err.to_string().contains("changed since the plan was shown"), "{err}");
    assert_eq!(std::fs::read(root.join(stale_rel)).unwrap(), stored, "the replaced file is untouched");
    assert!(!stale_bak.exists(), "no backup for a refused write");
    let replan = reg.texture_replace_plan(&s(root), stale_rel, &s(&small), None).unwrap();
    assert!(!replan.allowed, "a fresh plan reads the current bytes: {replan:?}");

    // ---- PS4 GNF plans are the GNF writer's own policy (P0-1, 13 Oct 2026): a rule-conformant
    // 32 × 32 file takes a 64 × 64 resize, so the plan is checked, allowed and carries no disclosure.
    let gnf =tdb_rx2::gnf::encode_gnf(tdb_rx2::gnf::BcFormat::Bc1, 32, 32, &(0..32u32 * 32 / 16 * 8).map(|i| (i * 7) as u8).collect::<Vec<u8>>());
    let gnf_rel = "data/textures/plan.gnf";
    std::fs::create_dir_all(root.join(gnf_rel).parent().unwrap()).unwrap();
    std::fs::write(root.join(gnf_rel), &gnf).unwrap();
    let g64 = flat_png(root, "gnf-64.png", 64, 64, [10, 200, 10]);
    let plan = reg.texture_replace_plan(&s(root), gnf_rel, &s(&g64), None).unwrap();
    assert_eq!(plan.container.as_deref(), Some("gnf"));
    assert!(plan.checked && plan.allowed && plan.notes.is_empty() && plan.refusal.is_none(), "{plan:?}");
}

// ------------------------------------------------------------------ real fixtures (opt-in)

fn fixture_root() -> PathBuf {
    PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the private logo fixtures cannot be resolved"))
}

fn manifest(root: &Path) -> serde_json::Value {
    let p = root.join("logos").join("manifest.json");
    serde_json::from_slice(&std::fs::read(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))).unwrap()
}

fn read_checked(root: &Path, rec: &serde_json::Value) -> Vec<u8> {
    let path = root.join(rec["fixture"].as_str().unwrap());
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    assert!(!bytes.is_empty(), "{}", path.display());
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), rec["sha256"].as_str().unwrap(), "{}: fixture identity", path.display());
    bytes
}

/// The 77 gen7 packages (X-A RefPack'd RW4xb2, X-B RefPack'd-inside RW4xb2, X-C XPR2, P-A raw
/// RW4ps3, P-B RefPack'd RW4ps3, P-C uncompressed DDS / DXT5) through the loose route with a
/// marked image, twice each.
#[test]
#[ignore = "private fixtures logos/<platform>-<title>/<family>/l0.big + logos/manifest.json: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_gen7_league_logo_package_takes_a_marked_image_through_the_loose_route() {
    let root = fixture_root();
    let m = manifest(&root);
    let tmp = Tmp::new("gen7");
    let reg = Registry::default();
    let mut n = 0;
    for rec in m["gen7"].as_array().unwrap() {
        let stored = read_checked(&root, rec);
        let (plat, title, family) = (rec["platform"].as_str().unwrap(), rec["title"].as_str().unwrap(), rec["family"].as_str().unwrap());
        let label = format!("{plat} {title} {family}");
        let rel = format!("{plat}-{title}/{}", rec["member"].as_str().unwrap());
        let bound = if family == "leaguelogossmall" { 14.0 } else { 0.0 };
        let first = replace_once(&reg, &tmp.0, &rel, &stored, 10, &label, bound);
        let second = replace_once_no_reset(&reg, &tmp.0, &rel, 200, &format!("{label} #2"));
        assert_ne!(first, second, "{label}");
        assert_eq!(std::fs::read(tmp.0.join(format!("{rel}.bak"))).unwrap(), stored, "{label}: first .bak kept");
        n += 1;
    }
    assert_eq!(n, 77);
}

/// The 34 PS4 members (NHL 15 PNG, 16–21 GNF BC7) as loose extraction-tree files.
#[test]
#[ignore = "private fixtures logos/ps4-<title>/<size>/l0[_fr].{png,gnf} + logos/manifest.json: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_ps4_league_logo_member_takes_a_marked_image_through_the_loose_route() {
    let root = fixture_root();
    let m = manifest(&root);
    let tmp = Tmp::new("ps4");
    let reg = Registry::default();
    let mut n = 0;
    for rec in m["ps4"].as_array().unwrap() {
        let stored = read_checked(&root, rec);
        let title = rec["title"].as_str().unwrap();
        let label = format!("ps4 {title} {}", rec["path"].as_str().unwrap());
        let rel = format!("{title}/{}", rec["path"].as_str().unwrap());
        let bound = if stored.starts_with(b"\x89PNG") { 0.0 } else { 14.0 };
        let first = replace_once(&reg, &tmp.0, &rel, &stored, 10, &label, bound);
        let second = replace_once_no_reset(&reg, &tmp.0, &rel, 200, &format!("{label} #2"));
        assert_ne!(first, second, "{label}");
        n += 1;
    }
    assert_eq!(n, 34);
}

// ------------------------------------------------------------------ non-logo league imagery (P1-G, opt-in)

fn imagery_manifest(root: &Path) -> serde_json::Value {
    let p = root.join("imagery").join("manifest.json");
    serde_json::from_slice(&std::fs::read(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))).unwrap()
}

/// The non-logo league imagery packages research 03 keyed by league (`divisionlogos/d0`,
/// `trophies/t0`, `trophiesbroadcast/t0`, `mudleaguelogos/l0`) on every gen7 source: preview and a
/// marked replace through the loose route, package shape preserved. These files were never
/// decoded on a real file before (receipt 06 §5 / R1 F3).
#[test]
#[ignore = "private fixtures imagery/<platform>-<title>/<family>/* + imagery/manifest.json: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_gen7_league_imagery_package_previews_and_takes_a_marked_image() {
    let root = fixture_root();
    let m = imagery_manifest(&root);
    let tmp = Tmp::new("imagery-gen7");
    let reg = Registry::default();
    let mut by_family: std::collections::BTreeMap<String, usize> = Default::default();
    let mut dims: std::collections::BTreeMap<String, std::collections::BTreeSet<(u32, u32)>> = Default::default();
    for rec in m["gen7"].as_array().unwrap() {
        let stored = read_checked(&root, rec);
        let (plat, title, family) = (rec["platform"].as_str().unwrap(), rec["title"].as_str().unwrap(), rec["family"].as_str().unwrap());
        let label = format!("{plat} {title} {family}");
        let rel = format!("{plat}-{title}/{}", rec["member"].as_str().unwrap());
        // Block-compressed layouts (DXT5 where a family ships it) get the bounded comparison;
        // decide from the preview's container + the round trip rather than assuming per family.
        let target = tmp.0.join(&rel);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, &stored).unwrap();
        let before = reg.texture_preview(&s(&tmp.0), &rel, 0, None).unwrap_or_else(|e| panic!("{label}: preview: {e}"));
        let img = decode_data_url(&before.data_url);
        let out = studio_domain::replace_texture_in_file(stored.clone(), &img.rgba, img.width, img.height, None).unwrap_or_else(|e| panic!("{label}: probe: {e}"));
        std::fs::write(&target, &out.bytes).unwrap();
        let back = decode_data_url(&reg.texture_preview(&s(&tmp.0), &rel, 0, None).unwrap().data_url);
        let lossless = mad(&back.rgba, &img.rgba) == 0.0;
        let bound = if lossless { 0.0 } else { 14.0 };
        let first = replace_once(&reg, &tmp.0, &rel, &stored, 10, &label, bound);
        let second = replace_once_no_reset(&reg, &tmp.0, &rel, 200, &format!("{label} #2"));
        assert_ne!(first, second, "{label}");
        assert_eq!(std::fs::read(tmp.0.join(format!("{rel}.bak"))).unwrap(), stored, "{label}: first .bak kept");
        *by_family.entry(family.to_string()).or_default() += 1;
        dims.entry(family.to_string()).or_default().insert((before.width, before.height));
    }
    let n: usize = by_family.values().sum();
    assert_eq!(n, 64, "{by_family:?}");
    eprintln!("gen7 imagery: {by_family:?}; dims {dims:?}");
}

/// The PS4 registry members (`large_league_logo` 0, `memorial_cup_logo`, `champions_logo`,
/// `gmc_league_icon` 0, `trophy_broadcast` 0) as loose extraction-tree files.
#[test]
#[ignore = "private fixtures imagery/ps4-<title>/<type>/* + imagery/manifest.json: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_ps4_league_imagery_member_previews_and_takes_a_marked_image() {
    let root = fixture_root();
    let m = imagery_manifest(&root);
    let tmp = Tmp::new("imagery-ps4");
    let reg = Registry::default();
    let mut n = 0;
    for rec in m["ps4"].as_array().unwrap() {
        let stored = read_checked(&root, rec);
        let title = rec["title"].as_str().unwrap();
        let label = format!("ps4 {title} {} {}", rec["type"].as_str().unwrap(), rec["path"].as_str().unwrap());
        let rel = format!("{title}/{}", rec["path"].as_str().unwrap());
        let bound = if stored.starts_with(b"\x89PNG") { 0.0 } else { 14.0 };
        let first = replace_once(&reg, &tmp.0, &rel, &stored, 10, &label, bound);
        let second = replace_once_no_reset(&reg, &tmp.0, &rel, 200, &format!("{label} #2"));
        assert_ne!(first, second, "{label}");
        n += 1;
    }
    assert_eq!(n, 22);
}

fn teamlogo_manifest(root: &Path) -> serde_json::Value {
    let p = root.join("teamlogos").join("manifest.json");
    serde_json::from_slice(&std::fs::read(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))).unwrap()
}

/// C9 (Phase 2.4): one member of **every** team-logo family of every source (gen7 packages as
/// stored — RefPack'd or raw BIGF — and PS4 members decoded from chunkzip) previews and takes a
/// marked image through the loose route, twice, `.bak` kept. Families whose member is not a
/// texture Studio decodes are collected and reported, never skipped silently: the test asserts the
/// exact set of refusals so a new refusal is a visible change.
#[test]
#[ignore = "private fixtures teamlogos/<platform>-<title>/<family>/* + teamlogos/manifest.json: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_team_logo_family_member_previews_and_takes_a_marked_image() {
    let root = fixture_root();
    let m = teamlogo_manifest(&root);
    let tmp = Tmp::new("teamlogos");
    let reg = Registry::default();
    let mut ok: std::collections::BTreeMap<String, usize> = Default::default();
    let mut refused: Vec<(String, String)> = Vec::new();
    let mut dims: std::collections::BTreeMap<String, std::collections::BTreeSet<(u32, u32)>> = Default::default();
    for rec in m["fixtures"].as_array().unwrap() {
        let stored = read_checked(&root, rec);
        let (plat, title, family) = (rec["platform"].as_str().unwrap(), rec["title"].as_str().unwrap(), rec["family"].as_str().unwrap());
        let member = rec["member"].as_str().unwrap();
        let label = format!("{plat} {title} {family} {}", member.rsplit('/').next().unwrap());
        let rel = format!("{plat}-{title}/{member}");
        let target = tmp.0.join(&rel);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, &stored).unwrap();
        let before = match reg.texture_preview(&s(&tmp.0), &rel, 0, None) {
            Ok(b) => b,
            Err(e) => {
                refused.push((label, format!("preview: {e}")));
                continue;
            }
        };
        let img = decode_data_url(&before.data_url);
        let probe = match studio_domain::replace_texture_in_file(stored.clone(), &img.rgba, img.width, img.height, None) {
            Ok(o) => o,
            Err(e) => {
                refused.push((label, format!("write: {e}")));
                continue;
            }
        };
        std::fs::write(&target, &probe.bytes).unwrap();
        let back = decode_data_url(&reg.texture_preview(&s(&tmp.0), &rel, 0, None).unwrap().data_url);
        let lossless = mad(&back.rgba, &img.rgba) == 0.0;
        let bound = if lossless { 0.0 } else { 14.0 };
        let first = replace_once(&reg, &tmp.0, &rel, &stored, 10, &label, bound);
        let second = replace_once_no_reset(&reg, &tmp.0, &rel, 200, &format!("{label} #2"));
        assert_ne!(first, second, "{label}");
        assert_eq!(std::fs::read(tmp.0.join(format!("{rel}.bak"))).unwrap(), stored, "{label}: first .bak kept");
        *ok.entry(format!("{plat} {title}")).or_default() += 1;
        dims.entry(format!("{plat} {title} {family}")).or_default().insert((before.width, before.height));
    }
    let n: usize = ok.values().sum();
    eprintln!("team logos: {n} ok — {ok:?}");
    eprintln!("dims: {dims:?}");
    eprintln!("refused ({}): {refused:#?}", refused.len());
    // Placeholders (`tblank`, `tcustom`, `dblank`, `t.big`) and the PS4 `_fr` / rare variants are
    // all in the fixture set; whatever refuses is asserted here by exact label so it stays visible.
    // Measured 20 Sep: exactly one refusal — NHL 15's `capilano_assets` (Xbox One tree, shipped
    // inside the PS4 archive) DDS twin of `overlaylogo`, a container Studio does not decode and a
    // family the Team logo card does not offer (no Xbox One target).
    let refused_labels: Vec<&str> = refused.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(refused_labels, ["ps4 nhl15 overlaylogo t0.dds"], "refusals: {refused:#?}");
    assert_eq!(n + refused.len(), m["fixtures"].as_array().unwrap().len());
    assert_eq!(n, 284);
}
