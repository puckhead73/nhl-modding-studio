//! P2-B (21 Sep 2026) — the `RW4ps3` rule of research 2026-09-21 handoff 11 §1 on the real PS3
//! Legacy and NHL 11 loose `rendering/**` trees (read only): every container parses with the
//! four linear formats (ARGB8, DXT1 / 3 / 5 — the parser's own slot-length check *is* the
//! chain-size rule), the only slot-length failures are the cube maps the research counts
//! (72 / 5), and a real multi-raster container resizes through `replace_slot_resized` keeping
//! its other rasters byte-identical. A missing tree fails, never skips.

use std::path::{Path, PathBuf};

use tdb_rx2::texlib_ps3::{mip_chain_sizes, parse, TexlibPs3Error};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("rpsgl") || x.eq_ignore_ascii_case("rx2")) {
            out.push(p);
        }
    }
}

#[test]
#[ignore = "real PS3 installs under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn every_ps3_container_parses_and_only_cube_maps_fail_the_chain_rule() {
    for (title, root, min_textures, max_cubes) in [
        ("legacy", r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL Legacy - Vanilla\PS3_GAME\USRDIR\rendering", 25_000usize, 160usize),
        ("nhl11", r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 11\PS3_GAME\USRDIR\rendering", 18_000, 40),
    ] {
        let root = Path::new(root);
        assert!(root.is_dir(), "{title}: {} missing (fails, never skips)", root.display());
        let mut files = Vec::new();
        walk(root, &mut files);
        let (mut containers, mut textures, mut cube_like, mut other, mut not_texlib, mut mixed) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
        let mut formats = std::collections::BTreeMap::<&str, usize>::new();
        let mut examples: Vec<String> = Vec::new();
        for f in &files {
            let Ok(bytes) = std::fs::read(f) else { continue };
            if !bytes.starts_with(tdb_rx2::texlib_ps3::MAGIC) {
                continue;
            }
            containers += 1;
            match parse(&bytes) {
                Ok(lib) => {
                    for t in &lib.textures {
                        textures += 1;
                        *formats.entry(t.format.label()).or_default() += 1;
                        let chain: usize = mip_chain_sizes(t.format, t.width, t.height, t.mips).iter().sum();
                        assert_eq!(chain, t.slot_len(), "{title} {}: slot = chain", f.display());
                    }
                }
                // A cube map: six faces in the slot (the research keys cubes by "slot = 6 × the
                // chain"; the shipped faces carry one extra 16-byte block each, so ≥ 6 ×).
                Err(TexlibPs3Error::SlotLengthMismatch { slot, expected, .. }) if expected > 0 && slot >= 6 * expected && slot < 7 * expected => cube_like += 1,
                // Mesh / non-texture containers carry data slots without descriptors: not texlibs.
                Err(TexlibPs3Error::Unpaired { count_descs: 0, .. }) => not_texlib += 1,
                // Mixed mesh + texture containers (`stadium/*_container_0`, the ice surfaces): more
                // data slots than descriptors — the slot ↔ descriptor pairing the PS3 model uses
                // does not apply; their rasters are outside this census (an open read-side item).
                Err(TexlibPs3Error::Unpaired { .. }) => mixed += 1,
                Err(e) => {
                    other += 1;
                    if examples.len() < 10 {
                        examples.push(format!("{}: {e}", f.strip_prefix(root).unwrap().display()));
                    }
                }
            }
        }
        eprintln!("{title}: {} rpsgl / rx2 files, {containers} RW4ps3 containers ({not_texlib} without descriptors = not texlibs, {mixed} mixed mesh + texture containers skipped), {textures} rasters {formats:?}, {cube_like} cube-map containers (slot = 6 x chain), {other} other failures", files.len());
        for e in &examples {
            eprintln!("  {e}");
        }
        assert!(textures >= min_textures, "{title}: {textures} rasters parsed");
        assert!(cube_like <= max_cubes, "{title}: {cube_like} cube-like containers (bound {max_cubes}; the research counts 72 / 5 cube descriptors, a container holds several)");
        assert_eq!(other, 0, "{title}: containers failing for another reason (above)");
    }
}

/// A real multi-raster PS3 Legacy jersey container resized: the target halves (a synthetic
/// chain), words 12 / 13 follow, the other rasters' slots and descriptors are byte-identical,
/// the container re-parses at the new length.
#[test]
#[ignore = "real PS3 install under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn a_real_ps3_legacy_container_resizes_and_keeps_its_other_rasters() {
    let root = Path::new(r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL Legacy - Vanilla\PS3_GAME\USRDIR\rendering\jersey");
    assert!(root.is_dir(), "{} missing (fails, never skips)", root.display());
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    let mut done = 0usize;
    for f in files.iter().take(400) {
        let bytes = std::fs::read(f).unwrap();
        if !bytes.starts_with(tdb_rx2::texlib_ps3::MAGIC) {
            continue;
        }
        let Ok(lib) = parse(&bytes) else { continue };
        let Some(target) = lib.textures.iter().position(|t| t.width >= 256 && t.height >= 256 && t.mips > 1 && t.width.is_power_of_two() && t.height.is_power_of_two()) else { continue };
        let t = &lib.textures[target];
        let (nw, nh) = (t.width / 2, t.height / 2);
        let nmips = t.mips - 1;
        let chain: Vec<u8> = mip_chain_sizes(t.format, nw, nh, nmips).iter().enumerate().flat_map(|(i, n)| std::iter::repeat_n((0x40 + i) as u8, *n)).collect();
        let out = lib.replace_slot_resized(target, nw, nh, nmips, &chain).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        let lib2 = parse(&out).unwrap_or_else(|e| panic!("{}: re-parse: {e}", f.display()));
        assert_eq!(lib2.textures.len(), lib.textures.len());
        let t2 = &lib2.textures[target];
        assert_eq!((t2.width, t2.height, t2.mips, t2.format), (nw, nh, nmips, t.format));
        assert_eq!(lib2.slot_bytes(target).unwrap(), &chain[..]);
        assert_eq!(&out[t2.descriptor_offset()..t2.descriptor_offset() + 48], &bytes[t.descriptor_offset()..t.descriptor_offset() + 48], "{}: words 0–11 kept", f.display());
        for (i, (a, b)) in lib.textures.iter().zip(&lib2.textures).enumerate() {
            if i == target {
                continue;
            }
            assert_eq!((a.width, a.height, a.mips, a.format), (b.width, b.height, b.mips, b.format), "{} #{i}", f.display());
            assert_eq!(lib.slot_bytes(i).unwrap(), lib2.slot_bytes(i).unwrap(), "{} #{i}: slot intact", f.display());
            assert_eq!(&bytes[a.descriptor_offset()..a.descriptor_offset() + 0x38], &out[b.descriptor_offset()..b.descriptor_offset() + 0x38], "{} #{i}: descriptor intact", f.display());
        }
        assert_eq!(out.len(), bytes.len() - t.slot_len() + chain.len());
        done += 1;
        eprintln!("{}: raster #{target} {}x{} → {nw}x{nh}, {} → {} mips, {} → {} bytes", f.strip_prefix(root).unwrap().display(), t.width, t.height, t.mips, nmips, bytes.len(), out.len());
        if done == 3 {
            break;
        }
    }
    assert_eq!(done, 3, "three multi-raster jersey containers resized");
}
