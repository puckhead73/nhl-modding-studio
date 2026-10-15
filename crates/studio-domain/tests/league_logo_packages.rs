//! P1-E — league-logo package preview and same-dimension replace on every shipped chain
//! (research 16 X-A/X-B/X-C, P-A/P-B/P-C, S-A/S-B): the texture surface (`decode_texture_file` →
//! `replace_texture_in_file`) on the index-0 package of every family of every gen7 source and on
//! the PS4 `l0` members. Synthetic PS3 `RW4ps3` packages run by default; the 77 real packages and
//! 34 PS4 members (`<root>/logos/**`, identities in `logos/manifest.json`) run `--ignored` under
//! `NHL_STUDIO_FIXTURES`. Each real case: identity check, decode, a no-op re-encode of the decoded
//! image written through the same path, re-parse, pixels compared (exact for the lossless
//! ARGB8/PNG layouts, bounded for DXT5/BC7 re-encodes), and the PS3 decode compared with the Xbox
//! decode of the same art.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use studio_domain::{decode_texture_file, replace_texture_in_file, TextureContainer};

fn tmp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("nhlms-logos-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// Mean absolute per-channel difference between two RGBA buffers.
fn mad(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len(), "pixel buffers differ in length");
    a.iter().zip(b).map(|(x, y)| (*x as i32 - *y as i32).abs() as f64).sum::<f64>() / a.len().max(1) as f64
}

// ------------------------------------------------------------------ synthetic RW4ps3 packages

/// A logo-layout RW4ps3 container (header 0x200, one slot, one descriptor) around `slot`.
fn synth_rw4ps3(argb8: bool, w: u16, h: u16, mips: u8, slot: &[u8]) -> Vec<u8> {
    let hdr = 0x200usize;
    let mut b = vec![0u8; hdr + slot.len()];
    b[..7].copy_from_slice(b"\x89RW4ps3");
    b[0x20..0x24].copy_from_slice(&2u32.to_be_bytes());
    b[0x30..0x34].copy_from_slice(&0x1B4u32.to_be_bytes());
    b[0x44..0x48].copy_from_slice(&(hdr as u32).to_be_bytes());
    let d = 0x17C;
    let sig: [u32; 3] = if argb8 { [0x6007, 0x6007, 0x8035] } else { [0x83F3, 0x83F3, 0xFFFF] };
    for (k, s) in sig.iter().enumerate() {
        b[d + 32 + k * 4..d + 36 + k * 4].copy_from_slice(&s.to_be_bytes());
    }
    b[d + 48..d + 50].copy_from_slice(&w.to_be_bytes());
    b[d + 50..d + 52].copy_from_slice(&h.to_be_bytes());
    b[d + 52] = mips;
    let t = 0x1B4;
    for (k, v) in [0u32, 0, slot.len() as u32, 0x80, 1].iter().enumerate() {
        b[t + k * 4..t + k * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    b[t + 20..t + 24].copy_from_slice(&[0, 1, 0, 0x30]);
    for (k, v) in [d as u32, 0, 0x38, 4, 0xA].iter().enumerate() {
        b[t + 24 + k * 4..t + 28 + k * 4].copy_from_slice(&v.to_be_bytes());
    }
    b[t + 44..t + 48].copy_from_slice(&[0, 2, 0, 3]);
    b[hdr..].copy_from_slice(slot);
    b
}

/// The six-member BIGF layout of a league-logo package with the texture as member `2`.
fn synth_package(texture_member: Vec<u8>) -> Vec<u8> {
    let mut b = tdb_archive_big::ArchiveBuilder::new();
    b.add_entry("0", b"Apt Data:".to_vec());
    b.add_entry("3", vec![0, 0, 1, 0x20]);
    b.add_entry("sg1", Vec::new());
    b.add_entry("2", texture_member);
    b.add_entry("sg2", Vec::new());
    b.add_entry("1", b"Apt constant file".to_vec());
    b.build().unwrap()
}

fn gradient(w: u32, h: u32, seed: u8) -> Vec<u8> {
    (0..w * h)
        .flat_map(|i| {
            let (x, y) = ((i % w) as u8, (i / w) as u8);
            [x.wrapping_mul(29).wrapping_add(seed), y.wrapping_mul(53), x ^ y, 255]
        })
        .collect()
}

#[test]
fn ps3_argb8_package_previews_and_replaces_raw_and_refpacked_members() {
    let dir = tmp_dir("synth-argb");
    let (w, h) = (8u32, 4u32);
    let before = gradient(w, h, 1);
    let slot: Vec<u8> = before.chunks_exact(4).flat_map(|p| [p[3], p[0], p[1], p[2]]).collect();
    let rw4 = synth_rw4ps3(true, w as u16, h as u16, 1, &slot);
    for (wrapped, tag) in [(false, "raw"), (true, "refpack")] {
        let member = if wrapped { tdb_archive_codec::refpack_compress(&rw4) } else { rw4.clone() };
        let path = dir.join(format!("l0-{tag}.big"));
        write(&path, &synth_package(member));
        let (container, img) = decode_texture_file(&path, None).unwrap();
        assert_eq!(container, TextureContainer::Portrait);
        assert_eq!((img.width, img.height), (w, h));
        assert_eq!(img.rgba, before, "{tag}: ARGB8 decodes losslessly");

        let after = gradient(w, h, 77);
        let out = replace_texture_in_file(std::fs::read(&path).unwrap(), &after, w, h, None).unwrap();
        assert_eq!(out.container, TextureContainer::Portrait);
        write(&path, &out.bytes);
        let (_, img2) = decode_texture_file(&path, None).unwrap();
        assert_eq!(img2.rgba, after, "{tag}: replaced pixels read back");
        // The member kept its own wrapping and the RW4 inside kept its length and header.
        let arc = tdb_archive_big::Archive::parse(out.bytes.clone()).unwrap();
        let raw = arc.read_entry_raw("2").unwrap();
        assert_eq!(tdb_archive_codec::is_refpack(&raw), wrapped, "{tag}: wrapping preserved");
        let inner = if wrapped { tdb_archive_codec::refpack_decompress(&raw).unwrap() } else { raw };
        assert_eq!(inner.len(), rw4.len());
        assert_eq!(&inner[..0x200], &rw4[..0x200], "{tag}: header/table/descriptor untouched");
        assert_eq!(arc.entries.len(), 6);
        // Different dimensions are refused with the limit named.
        let e = replace_texture_in_file(out.bytes.clone(), &gradient(4, 4, 1), 4, 4, None).unwrap_err();
        assert!(e.to_string().contains("span-preserving package layout"), "{e}");
    }
    // The X-A shape: the whole package RefPack-wrapped (as the BIG4 archive stores it).
    let path = dir.join("l0-wrapped-package.big");
    write(&path, &tdb_archive_codec::refpack_compress(&synth_package(rw4.clone())));
    let (_, img) = decode_texture_file(&path, None).unwrap();
    assert_eq!(img.rgba, before);
    let after = gradient(w, h, 5);
    let out = replace_texture_in_file(std::fs::read(&path).unwrap(), &after, w, h, None).unwrap();
    assert!(tdb_archive_codec::is_refpack(&out.bytes), "the package stays a RefPack stream");
    write(&path, &out.bytes);
    assert_eq!(decode_texture_file(&path, None).unwrap().1.rgba, after);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn ps3_dxt5_slot_is_re_encoded_at_the_same_length_with_its_mip_chain() {
    let dir = tmp_dir("synth-dxt5");
    let (w, h) = (16u32, 8u32);
    // Three levels: 16×8 (128 B) + 8×4 (32 B) + 4×2 (16 B, the floor) = 176 B.
    let sizes = tdb_rx2::texlib_ps3::mip_chain_sizes(tdb_rx2::texlib_ps3::Ps3Format::Dxt5, w, h, 3);
    assert_eq!(sizes, vec![128, 32, 16]);
    let flat = vec![0u8; 176];
    let rw4 = synth_rw4ps3(false, w as u16, h as u16, 3, &flat);
    let path = dir.join("texlib_0.rpsgl");
    write(&path, &rw4);
    let (container, img) = decode_texture_file(&path, None).unwrap();
    assert_eq!(container, TextureContainer::Texlib);
    assert_eq!((img.width, img.height), (w, h));
    // A solid colour survives DXT5 exactly.
    let solid: Vec<u8> = (0..w * h).flat_map(|_| [200u8, 40, 90, 255]).collect();
    let out = replace_texture_in_file(rw4.clone(), &solid, w, h, None).unwrap();
    assert_eq!(out.bytes.len(), rw4.len());
    let lib = tdb_rx2::texlib_ps3::parse(&out.bytes).unwrap();
    assert_eq!(lib.textures[0].mips, 3);
    write(&path, &out.bytes);
    let (_, img2) = decode_texture_file(&path, None).unwrap();
    assert!(mad(&img2.rgba, &solid) < 1.0, "solid colour round trip");
    // The smaller levels carry the colour too (not zeroed).
    let slot = lib.slot_bytes(0).unwrap();
    assert!(slot[128..].iter().any(|b| *b != 0), "mip levels regenerated");
    // Odd dimensions are refused for a block format.
    let e = replace_texture_in_file(rw4.clone(), &gradient(6, 8, 1)[..], 6, 8, None).unwrap_err();
    assert!(e.to_string().contains("divisible by 4"), "{e}");
    // P2-B (research 2026-09-21 handoff 11 §1): a loose PS3 container resizes — the slot's own
    // level count (3) at 32×16 = 512 + 128 + 32 bytes, words 12 / 13 rewritten, the file longer
    // by the chain delta, the pixels read back at the new size.
    let big: Vec<u8> = (0..32 * 16).flat_map(|_| [10u8, 200, 30, 255]).collect();
    let out2 = replace_texture_in_file(rw4.clone(), &big, 32, 16, None).unwrap();
    assert_eq!(out2.bytes.len(), rw4.len() - 176 + 672);
    let lib2 = tdb_rx2::texlib_ps3::parse(&out2.bytes).unwrap();
    assert_eq!((lib2.textures[0].width, lib2.textures[0].height, lib2.textures[0].mips), (32, 16, 3));
    let q = dir.join("resized.rpsgl");
    write(&q, &out2.bytes);
    let (_, back) = decode_texture_file(&q, None).unwrap();
    assert_eq!((back.width, back.height), (32, 16));
    assert!(mad(&back.rgba, &big) < 1.0, "resized solid colour round trip");
    std::fs::remove_dir_all(&dir).ok();
}

// ------------------------------------------------------------------ real fixtures (opt-in)

fn fixture_root() -> PathBuf {
    PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the private logo fixtures cannot be resolved"))
}

fn manifest(root: &Path) -> serde_json::Value {
    let p = root.join("logos").join("manifest.json");
    serde_json::from_slice(&std::fs::read(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))).unwrap()
}

fn read_checked(root: &Path, rec: &serde_json::Value) -> (PathBuf, Vec<u8>) {
    let path = root.join(rec["fixture"].as_str().unwrap());
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    assert!(!bytes.is_empty(), "{}", path.display());
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), rec["sha256"].as_str().unwrap(), "{}: fixture identity", path.display());
    (path, bytes)
}

/// Decode a package, write the decoded image back through the replace path (a no-op edit) on a
/// temp copy, re-decode and compare. Returns the decoded image and the MAD of the round trip.
fn round_trip(dir: &Path, label: &str, bytes: &[u8]) -> (studio_domain::DecodedImage, f64, TextureContainer) {
    let src = dir.join(format!("{}.bin", label.replace(['/', '\\', ' '], "_")));
    write(&src, bytes);
    let (container, img) = decode_texture_file(&src, None).unwrap_or_else(|e| panic!("{label}: decode: {e}"));
    assert!(img.width > 0 && img.height > 0, "{label}");
    assert_eq!(img.rgba.len(), (img.width * img.height * 4) as usize, "{label}");
    let out = replace_texture_in_file(bytes.to_vec(), &img.rgba, img.width, img.height, None)
        .unwrap_or_else(|e| panic!("{label}: replace: {e}"));
    assert_eq!(out.container, container, "{label}");
    write(&src, &out.bytes);
    let (_, back) = decode_texture_file(&src, None).unwrap_or_else(|e| panic!("{label}: re-decode: {e}"));
    assert_eq!((back.width, back.height), (img.width, img.height), "{label}: dims preserved");
    let err = mad(&img.rgba, &back.rgba);
    (img, err, container)
}

/// The 77 gen7 packages: X-A (07–10 Xbox, RefPack'd package: RW4xb2 raw inside), X-B (11 Xbox:
/// RW4xb2 RefPack'd inside a loose BIGF), X-C (12–Legacy Xbox: XPR2), P-A (08 PS3: RW4ps3 raw),
/// P-B (09–11 PS3: RW4ps3 RefPack'd), P-C (12–Legacy PS3 + Legacy cache: DDS).
#[test]
#[ignore = "private fixtures logos/<platform>-<title>/<family>/l0.big + logos/manifest.json: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_gen7_league_logo_package_previews_and_round_trips() {
    let root = fixture_root();
    let m = manifest(&root);
    let dir = tmp_dir("gen7");
    let mut decoded: std::collections::BTreeMap<(String, String), (u32, u32, Vec<u8>)> = Default::default();
    let mut n = 0;
    let mut by_chain: std::collections::BTreeMap<String, usize> = Default::default();
    for rec in m["gen7"].as_array().unwrap() {
        let (path, bytes) = read_checked(&root, rec);
        let label = format!("{} {} {}", rec["platform"].as_str().unwrap(), rec["title"].as_str().unwrap(), rec["family"].as_str().unwrap());
        let (img, err, container) = round_trip(&dir, &label, &bytes);
        assert_eq!(container, TextureContainer::Portrait, "{label}: {}", path.display());
        // Lossless layouts (ARGB8 / 8_8_8_8) read back exactly; block formats within re-encode noise.
        let family = rec["family"].as_str().unwrap();
        let bound = if family == "leaguelogossmall" { 6.0 } else { 0.0 };
        assert!(err <= bound, "{label}: round-trip MAD {err} > {bound}");
        *by_chain.entry(format!("{} {}", rec["platform"].as_str().unwrap(), rec["outer"].as_str().unwrap())).or_default() += 1;
        decoded.insert((rec["platform"].as_str().unwrap().to_string(), format!("{} {family}", rec["title"].as_str().unwrap())), (img.width, img.height, img.rgba));
        n += 1;
    }
    assert_eq!(n, 77, "the survey: 77 index-0 packages");
    // PS3 vs Xbox: the same art decodes to the same pixels (research 18 §2, MAD 0 on the survey).
    let mut compared = 0;
    for ((plat, key), (w, h, px)) in &decoded {
        if plat != "ps3" {
            continue;
        }
        if let Some((xw, xh, xp)) = decoded.get(&("x360".to_string(), key.clone())) {
            if (xw, xh) == (w, h) {
                let d = mad(px, xp);
                assert_eq!(d, 0.0, "{key}: PS3 vs Xbox MAD {d}");
                compared += 1;
            }
        }
    }
    assert!(compared >= 20, "cross-platform comparisons: {compared}");
    eprintln!("gen7 packages: {n}; chains {by_chain:?}; ps3-vs-x360 same-dims comparisons: {compared}");
    std::fs::remove_dir_all(&dir).ok();
}

/// The PS4 members: S-A (NHL 15 PNG) and S-B (NHL 16–21 GNF BC7, decoded from chunkzip).
#[test]
#[ignore = "private fixtures logos/ps4-<title>/<size>/l0[_fr].{png,gnf} + logos/manifest.json: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_ps4_league_logo_member_previews_and_round_trips() {
    let root = fixture_root();
    let m = manifest(&root);
    let dir = tmp_dir("ps4");
    let (mut png, mut gnf) = (0, 0);
    for rec in m["ps4"].as_array().unwrap() {
        let (_, bytes) = read_checked(&root, rec);
        let label = format!("ps4 {} {}", rec["title"].as_str().unwrap(), rec["path"].as_str().unwrap());
        let (img, err, container) = round_trip(&dir, &label, &bytes);
        match container {
            TextureContainer::Png => {
                assert_eq!(err, 0.0, "{label}");
                png += 1;
            }
            TextureContainer::Gnf => {
                assert!(err <= 6.0, "{label}: BC7 round-trip MAD {err}");
                gnf += 1;
            }
            other => panic!("{label}: unexpected container {other:?}"),
        }
        let size: u32 = rec["path"].as_str().unwrap().split("/size").nth(1).unwrap().split('/').next().unwrap().parse().unwrap();
        assert_eq!((img.width, img.height), (size, size), "{label}: member size matches its folder");
    }
    assert_eq!((png, gnf), (2, 32), "the survey: 2 PNG + 32 GNF l0 members");
    std::fs::remove_dir_all(&dir).ok();
}

// ------------------------------------------------------------------ span-preserving re-plan (research 02 §4.2, V-PKG)

/// Every gen7 package (outer RefPack unwrapped where the chain wraps it): the span-preserving
/// re-plan with the texture member's own bytes reproduces the package byte for byte, and a
/// re-plan with a member of `len ± 1 / ± 63 / ± 64` moves only the followers (`sg2` / `1` /
/// `size_le` / tail), keeps every other member's bytes and the trailer, and the followers keep the
/// package's own stride — on all 77 shipped shapes (X-A 4-member, X-B/P-B stride 64, X-C/P-C/P-A
/// stride 0, with and without the 16-byte tail).
#[test]
#[ignore = "private fixtures logos/<platform>-<title>/<family>/l0.big + logos/manifest.json: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_gen7_package_replans_identically_and_shifts_only_its_followers() {
    use tdb_archive_big::replan::{replan_member, PackageLayout};
    let root = fixture_root();
    let m = manifest(&root);
    let mut n = 0;
    let mut shapes: std::collections::BTreeMap<String, usize> = Default::default();
    for rec in m["gen7"].as_array().unwrap() {
        let (_, stored) = read_checked(&root, rec);
        let label = format!("{} {} {}", rec["platform"].as_str().unwrap(), rec["title"].as_str().unwrap(), rec["family"].as_str().unwrap());
        let pkg = if tdb_archive_codec::is_refpack(&stored) { tdb_archive_codec::refpack_decompress(&stored).unwrap() } else { stored.clone() };
        let layout = PackageLayout::parse(&pkg).unwrap_or_else(|e| panic!("{label}: {e}"));
        // The texture member is the largest one.
        let tex = layout.members.iter().max_by_key(|m| m.size).unwrap().clone();
        let tex_bytes = pkg[tex.offset as usize..(tex.offset + tex.size) as usize].to_vec();
        let id = replan_member(&pkg, &tex.name, &tex_bytes).unwrap_or_else(|e| panic!("{label}: identity re-plan: {e}"));
        assert!(id.is_identity(), "{label}");
        assert_eq!(id.bytes, pkg, "{label}: identity re-plan is byte-exact");
        let followers: Vec<_> = layout.members.iter().filter(|m| m.offset > tex.offset || (m.offset == tex.offset && m.index > tex.index)).collect();
        let stride = layout
            .members
            .iter()
            .find(|m| m.size == 0 && m.offset <= tex.offset)
            .map(|sg| tex.offset - sg.offset);
        shapes
            .entry(format!("{} members, stride {:?}, tail {}", layout.members.len(), stride, layout.tail_len))
            .and_modify(|c| *c += 1)
            .or_insert(1);
        for delta in [1i64, -1, 63, -63, 64, -64, 1024, -1024] {
            let new_len = tex.size as i64 + delta;
            if new_len <= 0 {
                continue;
            }
            let new_tex: Vec<u8> = (0..new_len as usize).map(|i| (i % 251) as u8 ^ 0x5A).collect();
            let r = replan_member(&pkg, &tex.name, &new_tex).unwrap_or_else(|e| panic!("{label} {delta:+}: {e}"));
            let out = PackageLayout::parse(&r.bytes).unwrap_or_else(|e| panic!("{label} {delta:+}: output: {e}"));
            assert_eq!(out.tail_len, layout.tail_len, "{label} {delta:+}: tail");
            assert_eq!(out.data_offset, layout.data_offset);
            assert_eq!(&r.bytes[layout.table_end..layout.data_offset as usize], &pkg[layout.table_end..layout.data_offset as usize], "{label}: trailer");
            assert_eq!(&r.bytes[..4], &pkg[..4]);
            // Members before the texture: same offsets and bytes; the texture: new bytes at its offset.
            for (a, b) in layout.members.iter().zip(&out.members) {
                assert_eq!(a.name, b.name);
                if a.index == tex.index {
                    assert_eq!(b.offset, a.offset, "{label} {delta:+}: texture keeps its offset");
                    assert_eq!(&r.bytes[b.offset as usize..(b.offset + b.size) as usize], &new_tex[..], "{label} {delta:+}");
                } else {
                    assert_eq!(&r.bytes[b.offset as usize..(b.offset + b.size) as usize], &pkg[a.offset as usize..(a.offset + a.size) as usize], "{label} {delta:+}: member {}", a.name);
                    if !followers.iter().any(|f| f.index == a.index) {
                        assert_eq!(b.offset, a.offset, "{label} {delta:+}: earlier member {} keeps its offset", a.name);
                    }
                }
            }
            // Followers: align64(previous end) + the source residual; the stride is preserved.
            let mut prev_end = tex.offset as u64 + new_len as u64;
            for f in &followers {
                let b = out.members.iter().find(|m| m.name == f.name).unwrap();
                let old_prev_end = {
                    // previous member in offset order in the source
                    let mut prev = tex.offset as u64 + tex.size as u64;
                    for g in &followers {
                        if g.offset < f.offset || (g.offset == f.offset && g.index < f.index) {
                            prev = g.offset as u64 + g.size as u64;
                        }
                    }
                    prev
                };
                let residual = f.offset as u64 - old_prev_end.div_ceil(64) * 64;
                assert_eq!(b.offset as u64, prev_end.div_ceil(64) * 64 + residual, "{label} {delta:+}: follower {}", f.name);
                prev_end = b.offset as u64 + b.size as u64;
            }
            assert_eq!(out.size_le, out.last_end);
            assert_eq!(r.bytes.len(), out.last_end as usize + layout.tail_len);
            // Only 16-multiples when the source was.
            if pkg.len() % 16 == 0 {
                assert_eq!(r.bytes.len() % 16, 0, "{label} {delta:+}: 16-byte multiple kept");
            }
        }
        n += 1;
    }
    assert_eq!(n, 77);
    eprintln!("shapes: {shapes:?}");
}

// ------------------------------------------------------------------ dimension change (C6, overnight 2026-09-20)

/// Read the Apt geometry facts of a package: the `<n+1>` record's six quad vertices and member 0's
/// size words, checked independently of the writer.
fn apt_geometry(pkg: &[u8]) -> Option<(u32, u32, Vec<(f32, f32)>, (u32, u32), (f32, f32))> {
    use tdb_archive_big::replan::PackageLayout;
    let l = PackageLayout::parse(pkg).ok()?;
    let tex = l.members.iter().max_by_key(|m| m.size)?;
    let n: u32 = tex.name.parse().ok()?;
    let rec = l.members.iter().find(|m| m.name == (n + 1).to_string() && m.size == 288)?;
    let m0 = l.members.iter().find(|m| m.name == "0")?;
    let b = &pkg[rec.offset as usize..(rec.offset + 288) as usize];
    let f = |at: usize| f32::from_be_bytes(b[at..at + 4].try_into().unwrap());
    let verts: Vec<(f32, f32)> = (0..6).map(|i| (f(96 + i * 32), f(100 + i * 32))).collect();
    let z = &pkg[m0.offset as usize..(m0.offset + m0.size) as usize];
    let u = |at: usize| u32::from_be_bytes(z[at..at + 4].try_into().unwrap());
    let g = |at: usize| f32::from_be_bytes(z[at..at + 4].try_into().unwrap());
    // Member 0's object table starts at the offset stored at +16; the size words sit at table +20
    // (integers) and table +96 (floats) on every measured variant (240 / 256 / 264 / 440 bytes).
    let table = 16 + u(16) as usize;
    let w = verts[4].0 as u32;
    let h = verts[4].1 as u32;
    Some((w, h, verts, (u(table + 20), u(table + 24)), (g(table + 96), g(table + 100))))
}

/// Every gen7 fixture package (77 logos + 64 imagery): the Apt geometry layout is the measured one
/// (six vertices `(0,H) (0,0) (W,0) (W,0) (W,H) (0,H)`; member 0's size words agree), and the quad
/// is the texture's own size on the logo families (a dimension change is offered) while the
/// display-crop families (trophies / broadcast trophies, NHL 07's oversized textures) are the
/// refused cases.
#[test]
#[ignore = "private fixtures logos/** + imagery/**: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_gen7_package_carries_the_measured_apt_geometry() {
    let root = fixture_root();
    let mut recs: Vec<(String, Vec<u8>)> = Vec::new();
    for (m, key) in [("logos", "gen7"), ("imagery", "gen7")] {
        let mf: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join(m).join("manifest.json")).unwrap()).unwrap();
        for rec in mf[key].as_array().unwrap() {
            let (_, stored) = read_checked(&root, rec);
            let pkg = if tdb_archive_codec::is_refpack(&stored) { tdb_archive_codec::refpack_decompress(&stored).unwrap() } else { stored };
            recs.push((format!("{} {} {}", rec["platform"].as_str().unwrap(), rec["title"].as_str().unwrap(), rec["family"].as_str().unwrap()), pkg));
        }
    }
    assert_eq!(recs.len(), 141);
    let (mut same, mut crop) = (0, 0);
    let mut crops: std::collections::BTreeSet<String> = Default::default();
    let tmp = tmp_dir("apt");
    for (label, pkg) in &recs {
        let (w, h, verts, ints, floats) = apt_geometry(pkg).unwrap_or_else(|| panic!("{label}: no Apt geometry"));
        let (wf, hf) = (w as f32, h as f32);
        assert_eq!(verts, vec![(0.0, hf), (0.0, 0.0), (wf, 0.0), (wf, 0.0), (wf, hf), (0.0, hf)], "{label}: vertices");
        assert_eq!(ints, (w, h), "{label}: member 0 ints");
        assert_eq!(floats, (wf, hf), "{label}: member 0 floats");
        let p = tmp.join("pkg.big");
        write(&p, pkg);
        let (_, img) = decode_texture_file(&p, None).unwrap();
        if (img.width, img.height) == (w, h) {
            same += 1;
        } else {
            crop += 1;
            crops.insert(format!("{} quad {w}x{h} on {}x{}", label.rsplit(' ').next().unwrap(), img.width, img.height));
            // The quad-is-smaller-than-texture families: every league logo family has quad == texture.
            assert!(!label.contains(" leaguelogos") || label.contains("nhl07"), "{label}: quad {w}x{h} on a {}x{} texture", img.width, img.height);
        }
    }
    std::fs::remove_dir_all(&tmp).ok();
    eprintln!("Apt geometry: {same} packages with quad == texture, {crop} with a smaller quad: {crops:?}");
    // 95 packages (every league-logo family of 08-Legacy, divisionlogos 08-Legacy, mudleaguelogos on
    // Xbox) draw the texture 1:1; 46 draw a smaller or even transposed quad (trophies 225x375 on a
    // 376x228 texture) - their texture-coordinate rule is not established, so they refuse a resize.
    // (Was 94 / 47 until 21 Sep 2026: the RW4xb2 height read was 8-bit and rounded a height that
    // is not a multiple of 8 down — e.g. the 560x404 division logo read as 560x400 — so one
    // package whose quad equals its texture looked cropped; the 13-bit read of research handoff
    // 11 corrected it.)
    assert_eq!((same, crop), (95, 46));
}

/// Dimension change on the resizable containers: Xbox 12-Legacy XPR2 (8_8_8_8 and DXT5) and PS3
/// 12-Legacy DDS (uncompressed and DXT5) packages take a 2x and a half-size replacement: the
/// texture, the Apt record's vertices and member 0's size words all read the new size; everything
/// else in the package (trailer, tail, other members) is verbatim; the written pixels decode. The
/// RW4 containers (07-11) and a display-crop package refuse with their reasons.
#[test]
#[ignore = "private fixtures logos/** + imagery/**: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn dimension_change_rewrites_texture_and_apt_geometry_on_the_resizable_families() {
    use tdb_archive_big::replan::PackageLayout;
    let root = fixture_root();
    let dir = tmp_dir("dims");
    let cases = [
        ("logos/x360-nhllegacy/leaguelogos/l0.big", true),
        ("logos/x360-nhllegacy/leaguelogossmall/l0.big", true),
        ("logos/x360-nhl12/leaguelogosbig/l0.big", true),
        ("logos/ps3-nhllegacy/leaguelogos/l0.big", true),
        ("logos/ps3-nhllegacy/leaguelogossmall/l0.big", true),
        ("imagery/ps3-nhl15/divisionlogos/d0.big", true),
        ("logos/x360-nhl10/leaguelogos/l0.big", false),
        ("logos/x360-nhl11/leaguelogos/l0.big", false),
        ("logos/ps3-nhl10/leaguelogos/l0.big", false),
        ("logos/ps3-nhl08/leaguelogos/l0.big", false),
    ];
    for (rel, resizable) in cases {
        let stored = std::fs::read(root.join(rel)).unwrap();
        let p = dir.join("in.big");
        write(&p, &stored);
        let (_, img) = decode_texture_file(&p, None).unwrap();
        // Xbox 8_8_8_8 XPR2 slots resize between the measured descriptor widths (64 <-> 256);
        // DXT and PS3 DDS slots take 2x / half sizes.
        let argb_x360 = rel.starts_with("logos/x360") && !rel.contains("small");
        let targets: Vec<(u32, u32)> = if argb_x360 {
            vec![if img.width == 64 { (256, 256) } else { (64, 64) }, (img.width, img.height * 2)]
        } else {
            vec![(img.width * 2, img.height * 2), ((img.width / 2).max(4), (img.height / 2).max(4))]
        };
        for (nw, nh) in targets {
            let rgba: Vec<u8> = (0..nw * nh).flat_map(|i| [(i % nw * 255 / nw) as u8, (i / nw * 255 / nh) as u8, 128, 255]).collect();
            let r = replace_texture_in_file(stored.clone(), &rgba, nw, nh, None);
            if !resizable {
                let e = r.err().map(|e| e.to_string()).unwrap_or_default();
                assert!(e.contains("same-size only"), "{rel} {nw}x{nh}: {e}");
                continue;
            }
            let out = r.unwrap_or_else(|e| panic!("{rel} {nw}x{nh}: {e}"));
            assert!(out.notes.is_empty(), "{rel}: {:?}", out.notes);
            let q = dir.join("out.big");
            write(&q, &out.bytes);
            let (_, back) = decode_texture_file(&q, None).unwrap();
            assert_eq!((back.width, back.height), (nw, nh), "{rel}: texture dims");
            assert!(mad(&back.rgba, &rgba) <= 14.0, "{rel} {nw}x{nh}: pixels");
            let pkg_out = if tdb_archive_codec::is_refpack(&out.bytes) { tdb_archive_codec::refpack_decompress(&out.bytes).unwrap() } else { out.bytes.clone() };
            let pkg_in = if tdb_archive_codec::is_refpack(&stored) { tdb_archive_codec::refpack_decompress(&stored).unwrap() } else { stored.clone() };
            // P1 audit (21 Sep): a PS3 DXT DDS member keeps its shipped level structure — the
            // `leaguelogossmall` DXT5 ships one level (`mips == 0`); the resized member must too,
            // with the header verbatim apart from the size words.
            if rel.starts_with("logos/ps3") && rel.contains("small") {
                let (li, lo) = (PackageLayout::parse(&pkg_in).unwrap(), PackageLayout::parse(&pkg_out).unwrap());
                let tex_in = li.members.iter().max_by_key(|m| m.size).unwrap();
                let tex_out = lo.members.iter().find(|m| m.name == tex_in.name).unwrap();
                let din = tdb_archive_codec::refpack_decompress(&pkg_in[tex_in.offset as usize..(tex_in.offset + tex_in.size) as usize]).unwrap();
                let dout = tdb_archive_codec::refpack_decompress(&pkg_out[tex_out.offset as usize..(tex_out.offset + tex_out.size) as usize]).unwrap();
                assert_eq!(&din[..4], b"DDS ");
                assert_eq!(u32::from_le_bytes(din[28..32].try_into().unwrap()), 0, "{rel}: source ships no mip chain");
                assert_eq!(u32::from_le_bytes(dout[28..32].try_into().unwrap()), 0, "{rel}: output ships no mip chain");
                assert_eq!(&dout[8..12], &din[8..12], "{rel}: flags verbatim");
                assert_eq!(&dout[32..128], &din[32..128], "{rel}: reserved / pixel format / caps verbatim");
                assert_eq!(dout.len(), 128 + ((nw / 4) * (nh / 4) * 16) as usize, "{rel}: base level only");
            }
            let (w, h, verts, ints, floats) = apt_geometry(&pkg_out).unwrap();
            assert_eq!((w, h, ints, floats), (nw, nh, (nw, nh), (nw as f32, nh as f32)), "{rel}: Apt size words");
            assert_eq!(verts[1], (0.0, 0.0));
            let (l0, l1) = (PackageLayout::parse(&pkg_in).unwrap(), PackageLayout::parse(&pkg_out).unwrap());
            assert_eq!(&pkg_out[l1.table_end..l1.data_offset as usize], &pkg_in[l0.table_end..l0.data_offset as usize], "{rel}: trailer");
            assert_eq!(&pkg_out[pkg_out.len() - l1.tail_len..], &pkg_in[pkg_in.len() - l0.tail_len..], "{rel}: tail");
            let tex = l0.members.iter().max_by_key(|m| m.size).unwrap();
            let n: u32 = tex.name.parse().unwrap();
            for (a, b) in l0.members.iter().zip(&l1.members) {
                if a.name == tex.name || a.name == (n + 1).to_string() || a.name == "0" {
                    continue;
                }
                assert_eq!(&pkg_out[b.offset as usize..(b.offset + b.size) as usize], &pkg_in[a.offset as usize..(a.offset + a.size) as usize], "{rel}: member {}", a.name);
            }
            // Member 0 differs only in its four size words.
            let (a0, b0) = (l0.members.iter().find(|m| m.name == "0").unwrap(), l1.members.iter().find(|m| m.name == "0").unwrap());
            let (ma, mb) = (&pkg_in[a0.offset as usize..(a0.offset + a0.size) as usize], &pkg_out[b0.offset as usize..(b0.offset + b0.size) as usize]);
            let table = 16 + u32::from_be_bytes(ma[16..20].try_into().unwrap()) as usize;
            for (i, (x, y)) in ma.iter().zip(mb).enumerate() {
                if x != y {
                    assert!((table + 20..table + 28).contains(&i) || (table + 96..table + 104).contains(&i), "{rel}: member 0 byte {i} changed");
                }
            }
        }
    }
    // A display-crop package refuses a dimension change with the reason.
    let stored = std::fs::read(root.join("imagery/x360-nhllegacy/trophies/t0.big")).unwrap();
    let rgba = vec![0u8; 64 * 256 * 4];
    let e = replace_texture_in_file(stored, &rgba, 64, 256, None).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("not the texture's own size"), "{e}");
    // An unmeasured Xbox XPR2 shape (a width that is not a measured power of two; a height that
    // is not a multiple of 8) refuses with the descriptor reason.
    let stored = std::fs::read(root.join("logos/x360-nhllegacy/leaguelogos/l0.big")).unwrap();
    let rgba = vec![0u8; 96 * 96 * 4];
    let e = replace_texture_in_file(stored.clone(), &rgba, 96, 96, None).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("measured for widths 32, 64, 128, 256, 512 and 1024"), "{e}");
    let rgba = vec![0u8; 64 * 60 * 4];
    let e = replace_texture_in_file(stored, &rgba, 64, 60, None).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(e.contains("multiples of 8"), "{e}");
    std::fs::remove_dir_all(&dir).ok();
}

/// P1 audit (21 Sep 2026) — the XPR2 geometry rule against real members. For every XPR2 member
/// of the gen7 league-logo / imagery / team-logo fixture packages (82 members: `8_8_8_8` at
/// 32×32, 64×64, 128×64, 128×128, 256×256, 256×512; DXT5 at 128×64): (a) a same-dimension
/// `replace_resized` reproduces the stored header bytes exactly (DW7 / DW9 / `data_size`
/// included), and (b) resizing any `8_8_8_8` member to each other measured shape yields the
/// DW7 + DW9 words a real member of that shape carries. The DXT5 member resized to 64 wide must
/// carry DW7's second byte `0x80` — the byte the pre-audit path left at its stored `0x00`.
#[test]
#[ignore = "private fixtures logos/** + imagery/** + teamlogos/**: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn xpr2_resize_reproduces_the_descriptor_words_of_real_members() {
    use tdb_archive_big::replan::PackageLayout;
    use tdb_rx2::xpr::Xpr2Texture;
    let root = fixture_root();
    // Real (DW7, DW9) words per (width, height) from the fixture census.
    let measured: std::collections::BTreeMap<(u32, u32), (u32, u32)> = [
        ((32, 32), (0x8040_0002, 0x0003_e01f)),
        ((64, 64), (0x8080_0002, 0x0007_e03f)),
        ((128, 64), (0x8100_0002, 0x0007_e07f)),
        ((128, 128), (0x8100_0002, 0x000f_e07f)),
        ((256, 256), (0x8200_0002, 0x001f_e0ff)),
        ((256, 512), (0x8200_0002, 0x003f_e0ff)),
    ]
    .into_iter()
    .collect();
    let words = |x: &[u8]| -> (u32, u32) {
        let data_size = u32::from_be_bytes(x[8..12].try_into().unwrap()) as usize;
        let ps = x.len() - data_size;
        (u32::from_be_bytes(x[ps - 0x18..ps - 0x14].try_into().unwrap()), u32::from_be_bytes(x[ps - 0x10..ps - 0x0C].try_into().unwrap()))
    };
    let mut seen: std::collections::BTreeMap<(u32, u32), usize> = Default::default();
    let mut members = 0usize;
    let mut dxt_checked = false;
    for sub in ["logos", "imagery", "teamlogos"] {
        for entry in walkdir(&root.join(sub)) {
            let rel = entry.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
            if !rel.contains("x360-") || !rel.ends_with(".big") {
                continue;
            }
            let stored = std::fs::read(&entry).unwrap();
            let pkg = if tdb_archive_codec::is_refpack(&stored) { tdb_archive_codec::refpack_decompress(&stored).unwrap() } else { stored };
            let Ok(layout) = PackageLayout::parse(&pkg) else { continue };
            for m in &layout.members {
                let raw = &pkg[m.offset as usize..(m.offset + m.size) as usize];
                let raw = if tdb_archive_codec::is_refpack(raw) { tdb_archive_codec::refpack_decompress(raw).unwrap() } else { raw.to_vec() };
                if !raw.starts_with(b"XPR2") {
                    continue;
                }
                members += 1;
                let tex = Xpr2Texture::parse(raw.clone()).unwrap_or_else(|e| panic!("{rel}:{}: {e}", m.name));
                let (w, h) = (tex.width, tex.height);
                *seen.entry((w, h)).or_default() += 1;
                let (dw7, dw9) = words(&raw);
                assert_eq!(measured.get(&(w, h)).copied(), Some((dw7, dw9)), "{rel}:{}: stored words at {w}x{h}", m.name);
                // (a) same dims: header byte-identical.
                let linear = tex.decode_base_mip_linear();
                let same = tex.replace_resized(w, h, &linear).unwrap_or_else(|e| panic!("{rel}:{}: {e}", m.name));
                let ps = raw.len() - u32::from_be_bytes(raw[8..12].try_into().unwrap()) as usize;
                assert_eq!(&same[..ps], &raw[..ps], "{rel}:{}: same-dims header", m.name);
                // (b) every other measured shape.
                let is_argb = tex.format == tdb_rx2::xpr::XprFormat::Argb8888;
                for (&(nw, nh), &(e7, e9)) in &measured {
                    if (nw, nh) == (w, h) {
                        continue;
                    }
                    let lin = if is_argb { vec![0u8; (nw * nh * 4) as usize] } else { vec![0u8; ((nw / 4) * (nh / 4) * 16) as usize] };
                    let out = tex.replace_resized(nw, nh, &lin).unwrap_or_else(|e| panic!("{rel}:{} -> {nw}x{nh}: {e}", m.name));
                    let (o7, o9) = words(&out);
                    // DW7's stored low half-word and the DXT5 data words are format-bound and kept
                    // from the source; only the width-derived upper half is compared for DXT.
                    if is_argb {
                        assert_eq!((o7, o9), (e7, e9), "{rel}:{} -> {nw}x{nh}", m.name);
                    } else {
                        assert_eq!((o7 >> 16, o9), (e7 >> 16, e9), "{rel}:{} -> {nw}x{nh} (DXT)", m.name);
                        dxt_checked = true;
                    }
                    let back = Xpr2Texture::parse(out).unwrap();
                    assert_eq!((back.width, back.height), (nw, nh));
                }
                // Unmeasured shapes refuse.
                assert!(tex.replace_resized(96, 96, &vec![0u8; if is_argb { 96 * 96 * 4 } else { 24 * 24 * 16 }]).is_err());
                assert!(tex.replace_resized(64, 60, &vec![0u8; if is_argb { 64 * 60 * 4 } else { 16 * 15 * 16 }]).is_err());
            }
        }
    }
    assert!(members >= 80, "{members} XPR2 members");
    assert!(dxt_checked, "a DXT5 member was resized");
    eprintln!("xpr2 resize oracle: {members} members; shapes {seen:?}");
}

fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walkdir(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}

/// P1 audit (21 Sep 2026) — PS3 DXT DDS members keep their shipped level structure. The NHL
/// 12–Legacy `leaguelogossmall` DXT5 logos ship one level (`mips == 0`, flags `0x81007`, caps
/// `0x1000`, body = base only; measured on 6 fixture members); a portrait ships a full chain. The
/// earlier writer emitted a full chain with the mip flags set for every DXT DDS. Now: same-size
/// and resized writes of a base-only source stay base-only with the header verbatim apart from
/// the size words; a full-chain source keeps a full chain for the new size.
#[test]
fn ps3_dxt_dds_member_keeps_its_shipped_mip_structure() {
    let dir = tmp_dir("dds-mips");
    let dxt5 = tdb_archive_codec::DdsFormat::Dxt5;
    // A base-only DDS in the measured shape: flags CAPS|HEIGHT|WIDTH|PIXELFORMAT|LINEARSIZE,
    // mips word 0, caps TEXTURE only, with the reserved words carrying a marker to prove the
    // header is copied, not regenerated.
    let base_only = |w: u32, h: u32| -> Vec<u8> {
        let mut d = tdb_archive_codec::encode_dds(dxt5, w, h, 1, &tdb_archive_codec::compress_bc(dxt5, w, h, &gradient(w, h, 1)));
        d[8..12].copy_from_slice(&0x0008_1007u32.to_le_bytes());
        d[28..32].copy_from_slice(&0u32.to_le_bytes());
        d[108..112].copy_from_slice(&0x1000u32.to_le_bytes());
        d[32..36].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes()); // reserved1[0]
        d
    };
    let src_dds = base_only(128, 64);
    let pkg = synth_package(tdb_archive_codec::refpack_compress(&src_dds));
    let member = |bytes: &[u8]| -> Vec<u8> {
        let a = tdb_archive_big::Archive::parse(bytes.to_vec()).unwrap();
        tdb_archive_codec::refpack_decompress(&a.read_entry_raw("2").unwrap()).unwrap()
    };
    // (The synthetic builder package is not in the span-preserving shape a dimension change
    // needs; the resized base-only case is asserted on the real PS3 fixture in
    // `dimension_change_rewrites_texture_and_apt_geometry_on_the_resizable_families`.)
    for (w, h) in [(128u32, 64u32)] {
        let out = replace_texture_in_file(pkg.clone(), &gradient(w, h, 7), w, h, None).unwrap_or_else(|e| panic!("{w}x{h}: {e}"));
        let dds = member(&out.bytes);
        assert_eq!(&dds[..4], b"DDS ");
        assert_eq!(u32::from_le_bytes(dds[8..12].try_into().unwrap()), 0x0008_1007, "{w}x{h}: flags verbatim");
        assert_eq!(u32::from_le_bytes(dds[12..16].try_into().unwrap()), h);
        assert_eq!(u32::from_le_bytes(dds[16..20].try_into().unwrap()), w);
        assert_eq!(u32::from_le_bytes(dds[20..24].try_into().unwrap()), (w / 4) * (h / 4) * 16, "{w}x{h}: linear size");
        assert_eq!(u32::from_le_bytes(dds[28..32].try_into().unwrap()), 0, "{w}x{h}: no mip count");
        assert_eq!(u32::from_le_bytes(dds[32..36].try_into().unwrap()), 0xDEAD_BEEF, "{w}x{h}: reserved words verbatim");
        assert_eq!(u32::from_le_bytes(dds[108..112].try_into().unwrap()), 0x1000, "{w}x{h}: caps verbatim");
        assert_eq!(&dds[76..108], &src_dds[76..108], "{w}x{h}: pixel format verbatim");
        assert_eq!(dds.len(), 128 + ((w / 4) * (h / 4) * 16) as usize, "{w}x{h}: base level only");
        let p = dir.join("out.big");
        write(&p, &out.bytes);
        let (_, back) = decode_texture_file(&p, None).unwrap();
        assert_eq!((back.width, back.height), (w, h));
        assert!(mad(&back.rgba, &gradient(w, h, 7)) <= 14.0, "{w}x{h}: pixels");
    }
    // A full-chain source: the chain follows the new size.
    let full = |w: u32, h: u32| -> Vec<u8> {
        let mut levels = Vec::new();
        let mut n = 0;
        let (mut mw, mut mh) = (w, h);
        loop {
            levels.extend(tdb_archive_codec::compress_bc(dxt5, mw, mh, &gradient(mw, mh, 3)));
            n += 1;
            if mw == 1 && mh == 1 {
                break;
            }
            mw = (mw / 2).max(1);
            mh = (mh / 2).max(1);
        }
        tdb_archive_codec::encode_dds(dxt5, w, h, n, &levels)
    };
    let src = full(64, 64);
    assert_eq!(u32::from_le_bytes(src[28..32].try_into().unwrap()), 7);
    let pkg = synth_package(tdb_archive_codec::refpack_compress(&src));
    for (w, h, want_levels) in [(64u32, 64u32, 7u32)] {
        let out = replace_texture_in_file(pkg.clone(), &gradient(w, h, 9), w, h, None).unwrap();
        let dds = member(&out.bytes);
        assert_eq!(u32::from_le_bytes(dds[28..32].try_into().unwrap()), want_levels, "{w}x{h}: full chain for the new size");
        assert_eq!(u32::from_le_bytes(dds[8..12].try_into().unwrap()), u32::from_le_bytes(src[8..12].try_into().unwrap()));
        let mut expect = 0usize;
        let (mut mw, mut mh) = (w, h);
        for _ in 0..want_levels {
            expect += ((mw.div_ceil(4)) * (mh.div_ceil(4)) * 16) as usize;
            mw = (mw / 2).max(1);
            mh = (mh / 2).max(1);
        }
        assert_eq!(dds.len(), 128 + expect, "{w}x{h}: chain length");
    }
    std::fs::remove_dir_all(&dir).ok();
}
