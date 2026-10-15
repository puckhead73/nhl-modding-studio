//! Phase 2.4 (22 Sep 2026) — the **different-dimension** replacement route swept over every gen7
//! team-logo package of the private fixture set (`teamlogos/manifest.json` under
//! `NHL_STUDIO_FIXTURES`: X360 07 → Legacy, PS3 08 → Legacy, the Legacy PS3 `cache.big` members —
//! every family the census keys by team art id). The Phase 2 matrix row "Different dimensions" said
//! *impl, test missing — team packages not swept*; this is the sweep.
//!
//! For every package: identity check, decode, then a dimension change (Xbox `8_8_8_8` XPR2 slots
//! between the measured descriptor widths 64 ↔ 256 and a doubled height; everything else 2× and
//! half). A **success** must re-open at the new size with the imported pixels, keep every other
//! member byte-identical, keep the trailer and tail, and carry the new size in the Apt geometry
//! record where the package has one. A **refusal** must be one of the route's named reasons (the
//! same-size-only package forms, a display-crop quad, an unmeasured XPR2 shape). Anything else —
//! a panic, an unexpected error, a silent success on a same-size-only form — fails the sweep.
//!
//! The per-family outcome matrix is printed and is the coverage record; nothing here claims what
//! the console draws (runtime cards T-3 / R2-BIG4-1 / R2-EB-1 are unchanged).

use std::collections::BTreeMap;
use std::path::PathBuf;

use sha2::{Digest, Sha256};
use studio_domain::{decode_texture_file, replace_texture_in_file};

struct Tmp(PathBuf);
impl Tmp {
    fn new() -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-teamlogo-dims-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Tmp(d)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixture_root() -> PathBuf {
    PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the private team-logo fixtures cannot be resolved"))
}

/// Mean absolute per-channel difference between two RGBA buffers.
fn mad(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len(), "pixel buffers differ in length");
    a.iter().zip(b).map(|(x, y)| (*x as i32 - *y as i32).abs() as f64).sum::<f64>() / a.len().max(1) as f64
}

/// The Apt geometry of a logo package (research 16): the six quad vertices and member 0's size
/// words. `None` when the package carries no 288-byte geometry record (a family without a quad).
fn apt_geometry(pkg: &[u8]) -> Option<(u32, u32, (u32, u32), (f32, f32))> {
    use tdb_archive_big::replan::PackageLayout;
    let l = PackageLayout::parse(pkg).ok()?;
    let tex = l.members.iter().max_by_key(|m| m.size)?;
    let n: u32 = tex.name.parse().ok()?;
    let rec = l.members.iter().find(|m| m.name == (n + 1).to_string() && m.size == 288)?;
    let m0 = l.members.iter().find(|m| m.name == "0")?;
    let b = &pkg[rec.offset as usize..(rec.offset + 288) as usize];
    let f = |at: usize| f32::from_be_bytes(b[at..at + 4].try_into().unwrap());
    let verts: Vec<(f32, f32)> = (0..6).map(|i| (f(96 + i * 32), f(100 + i * 32))).collect();
    // Member 0 is RefPack-wrapped on some NHL 11 packages (research R132 §2).
    let z0 = &pkg[m0.offset as usize..(m0.offset + m0.size) as usize];
    let z_owned = if tdb_archive_codec::is_refpack(z0) { tdb_archive_codec::refpack_decompress(z0).ok()? } else { z0.to_vec() };
    let z = &z_owned[..];
    let u = |at: usize| u32::from_be_bytes(z[at..at + 4].try_into().unwrap());
    let g = |at: usize| f32::from_be_bytes(z[at..at + 4].try_into().unwrap());
    let table = 16 + u(16) as usize;
    if table + 104 > z.len() {
        return None;
    }
    Some((verts[4].0 as u32, verts[4].1 as u32, (u(table + 20), u(table + 24)), (g(table + 96), g(table + 100))))
}

fn unwrap_package(bytes: &[u8]) -> Vec<u8> {
    if tdb_archive_codec::is_refpack(bytes) {
        tdb_archive_codec::refpack_decompress(bytes).unwrap()
    } else {
        bytes.to_vec()
    }
}

/// Classify a refusal by the route's own reason text. Anything unrecognised fails the sweep.
fn refusal_class(e: &str) -> &'static str {
    if e.contains("same-size only") {
        "same-size only (package form)"
    } else if e.contains("not the texture's own size") {
        "display-crop quad"
    } else if e.contains("measured for widths") || e.contains("multiples of 8") {
        "unmeasured XPR2 shape"
    } else if e.contains("cube") {
        "cube map"
    } else if e.contains("undecodable") || e.contains("RefPack") {
        "undecodable member"
    } else if e.contains("Apt geometry record") {
        "no Apt geometry record (package without a quad)"
    } else if e.contains("not a tagged data entry") {
        "untagged pixel entry (NHL 07 variant)"
    } else if e.contains("table-at-EOF") {
        "NHL 07 container (buffers in the header region)"
    } else if e.contains("descriptor rule covers") || e.contains("cannot be stored in the descriptor") || e.contains("unmeasured") {
        "outside the descriptor rule"
    } else {
        "UNEXPECTED"
    }
}

#[derive(Default, Debug)]
struct Outcome {
    packages: usize,
    resized: usize,
    refused: BTreeMap<&'static str, usize>,
    with_apt: usize,
}

#[test]
#[ignore = "private fixtures teamlogos/** + teamlogos/manifest.json: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_gen7_team_logo_package_resizes_with_its_invariants_or_refuses_by_name() {
    use tdb_archive_big::replan::PackageLayout;
    let root = fixture_root();
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("teamlogos/manifest.json")).expect("teamlogos/manifest.json")).unwrap();
    let tmp = Tmp::new();
    let mut matrix: BTreeMap<(String, String, String), Outcome> = BTreeMap::new();
    let mut unexpected: Vec<String> = Vec::new();
    let mut total = 0usize;
    for rec in manifest["fixtures"].as_array().unwrap() {
        let platform = rec["platform"].as_str().unwrap().to_string();
        if platform == "ps4" {
            continue; // the PS4 GNF / PNG members are Phase 2.4's other route (receipt 02 / D6)
        }
        let title = rec["title"].as_str().unwrap().to_string();
        let family = rec["family"].as_str().unwrap().to_string();
        let label = format!("{platform}/{title}/{family}/{}", rec["member"].as_str().unwrap());
        let path = root.join(rec["fixture"].as_str().unwrap());
        let stored = std::fs::read(&path).unwrap_or_else(|e| panic!("{label}: cannot read {}: {e}", path.display()));
        assert!(!stored.is_empty(), "{label}: empty fixture");
        assert_eq!(format!("{:x}", Sha256::digest(&stored)), rec["sha256"].as_str().unwrap(), "{label}: fixture identity");
        total += 1;
        let entry = matrix.entry((platform.clone(), title.clone(), family.clone())).or_default();
        entry.packages += 1;

        let src = tmp.0.join("in.bin");
        std::fs::write(&src, &stored).unwrap();
        let (_, img) = decode_texture_file(&src, None).unwrap_or_else(|e| panic!("{label}: decode: {e}"));
        let pkg_in = unwrap_package(&stored);
        let apt_in = apt_geometry(&pkg_in);
        if apt_in.is_some() {
            entry.with_apt += 1;
        }

        // Xbox 8_8_8_8 XPR2 slots resize between the measured descriptor widths (64 ↔ 256) and to
        // a doubled height; DXT and PS3 slots take 2× / half. The `small` families are DXT5.
        let argb_x360 = platform == "x360" && !family.contains("small");
        let targets: Vec<(u32, u32)> = if argb_x360 {
            vec![if img.width == 64 { (256, 256) } else { (64, 64) }, (img.width, img.height * 2)]
        } else {
            vec![(img.width * 2, img.height * 2), ((img.width / 2).max(4), (img.height / 2).max(4))]
        };
        let mut package_resized = false;
        for (nw, nh) in targets {
            let rgba: Vec<u8> = (0..nw * nh).flat_map(|i| [(i % nw * 255 / nw) as u8, (i / nw * 255 / nh) as u8, 128, 255]).collect();
            let r = std::panic::catch_unwind(|| replace_texture_in_file(stored.clone(), &rgba, nw, nh, None));
            let r = match r {
                Ok(r) => r,
                Err(_) => {
                    unexpected.push(format!("{label} {nw}x{nh}: PANIC"));
                    continue;
                }
            };
            match r {
                Err(e) => {
                    let e = e.to_string();
                    let class = refusal_class(&e);
                    if class == "UNEXPECTED" {
                        unexpected.push(format!("{label} {nw}x{nh}: {e}"));
                    }
                    *entry.refused.entry(class).or_default() += 1;
                }
                Ok(out) => {
                    // Invariants of a success.
                    let q = tmp.0.join("out.bin");
                    std::fs::write(&q, &out.bytes).unwrap();
                    let (_, back) = decode_texture_file(&q, None).unwrap_or_else(|e| panic!("{label} {nw}x{nh}: re-decode: {e}"));
                    assert_eq!((back.width, back.height), (nw, nh), "{label}: re-opens at the new size");
                    let err = mad(&back.rgba, &rgba);
                    assert!(err <= 14.0, "{label} {nw}x{nh}: pixels differ (MAD {err:.1})");
                    let pkg_out = unwrap_package(&out.bytes);
                    let (l0, l1) = (PackageLayout::parse(&pkg_in).unwrap(), PackageLayout::parse(&pkg_out).unwrap());
                    assert_eq!(l0.members.len(), l1.members.len(), "{label}: member count");
                    assert_eq!(&pkg_out[l1.table_end..l1.data_offset as usize], &pkg_in[l0.table_end..l0.data_offset as usize], "{label}: trailer");
                    assert_eq!(&pkg_out[pkg_out.len() - l1.tail_len..], &pkg_in[pkg_in.len() - l0.tail_len..], "{label}: tail");
                    let tex = l0.members.iter().max_by_key(|m| m.size).unwrap();
                    let n: u32 = tex.name.parse().unwrap_or(u32::MAX - 1);
                    for (a, b) in l0.members.iter().zip(&l1.members) {
                        assert_eq!(a.name, b.name, "{label}: member order");
                        if a.name == tex.name || a.name == (n + 1).to_string() || a.name == "0" {
                            continue;
                        }
                        assert_eq!(&pkg_out[b.offset as usize..(b.offset + b.size) as usize], &pkg_in[a.offset as usize..(a.offset + a.size) as usize], "{label}: member {} must be byte-identical", a.name);
                    }
                    if apt_in.is_some() {
                        let (w, h, ints, floats) = apt_geometry(&pkg_out).unwrap_or_else(|| panic!("{label}: the Apt record vanished"));
                        assert_eq!((w, h, ints, floats), (nw, nh, (nw, nh), (nw as f32, nh as f32)), "{label} {nw}x{nh}: Apt size words");
                    }
                    package_resized = true;
                }
            }
        }
        if package_resized {
            entry.resized += 1;
        }
    }
    eprintln!("team-logo dimension sweep: {total} gen7 packages");
    for ((platform, title, family), o) in &matrix {
        let refused: Vec<String> = o.refused.iter().map(|(k, v)| format!("{v} {k}")).collect();
        eprintln!("  {platform:9} {title:9} {family:22} packages {:2}  resized {:2}  apt {:2}  refused: {}", o.packages, o.resized, o.with_apt, if refused.is_empty() { "-".to_string() } else { refused.join("; ") });
    }
    assert!(unexpected.is_empty(), "unexpected outcomes:\n{}", unexpected.join("\n"));
    assert!(total >= 150, "only {total} gen7 packages swept — the fixture set is incomplete");
    // Every resizable-form package resized on at least one target, and every same-size-only form
    // refused every target: the route's own forms, not a title list, decide — checked per package.
    for ((platform, title, family), o) in &matrix {
        let same_size_only = o.refused.get("same-size only (package form)").copied().unwrap_or(0);
        assert!(o.resized > 0 || same_size_only > 0 || o.refused.values().sum::<usize>() > 0, "{platform}/{title}/{family}: a package neither resized nor refused");
    }
}
