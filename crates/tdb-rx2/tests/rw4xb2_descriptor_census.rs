//! P2-B (21 Sep 2026) — the `RW4xb2` descriptor rule of research 2026-09-21 handoff 11 §2,
//! reproduced by Studio's `patch_descriptor_for_resize` on every loose `rendering/**` texlib of
//! the real Xbox 360 Legacy / NHL 11 / NHL 09 trees (read only): for each tiled 2D DXT raster of
//! at least 128 × 128 texels, re-deriving the descriptor from (width, height, format, mips) must
//! give the shipped bytes back. The research's exceptions (the 16 × 256 `ice/texlib_0.rx2`
//! strip, tiny rasters, non-tiled FE rasters, cubes) are outside the gate by construction; any
//! other mismatch is a defect. A missing tree fails, never skips.

use std::path::{Path, PathBuf};

use tdb_rx2::texlib::{descriptor_mip_count, parse, patch_descriptor_for_resize, DESCRIPTOR_LEN};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("rx2")) {
            out.push(p);
        }
    }
}

#[test]
#[ignore = "real Xbox 360 installs under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn every_tiled_dxt_descriptor_of_128_texels_or_more_is_reproduced() {
    for (title, root, min_textures) in [
        ("legacy", r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\rendering", 18_000usize),
        ("nhl11", r"X:\Games\Emulation\Roms\XBOX360\NHL 11\NHL 11\rendering", 12_000),
        ("nhl09", r"X:\Games\Emulation\Roms\XBOX360\NHL 09\NHL 09 (USA)\rendering", 9_000),
    ] {
        let root = Path::new(root);
        assert!(root.is_dir(), "{title}: {} missing (fails, never skips)", root.display());
        let mut files = Vec::new();
        walk(root, &mut files);
        let (mut texlibs, mut gated, mut reproduced, mut refused, mut older) = (0usize, 0usize, 0usize, 0usize, 0usize);
        let mut mismatches: Vec<String> = Vec::new();
        // The `DW8` join (research 2026-09-21 pm handoff R42 §1) over **every** descriptor of the
        // tree, not only the gated ones: the word's upper field must be the file-table index of
        // the texture's own pixel-data entry. Studio's write path refuses a container that
        // disagrees, so this counts how many real containers that refusal would reject.
        let (mut join_checked, mut join_ok) = (0usize, 0usize);
        let mut join_bad: Vec<String> = Vec::new();
        for f in &files {
            let Ok(bytes) = std::fs::read(f) else { continue };
            if !bytes.starts_with(b"\x89RW4xb2") {
                continue;
            }
            let Ok(lib) = parse(&bytes) else { continue };
            texlibs += 1;
            for (i, t) in lib.textures.iter().enumerate() {
                let off = t.descriptor_offset();
                let orig: [u8; DESCRIPTOR_LEN] = bytes[off..off + DESCRIPTOR_LEN].try_into().unwrap();
                join_checked += 1;
                let dw8_index = tdb_rx2::texlib::descriptor_data_entry_index(&orig);
                if dw8_index == t.data_entry_index() {
                    join_ok += 1;
                } else if join_bad.len() < 12 {
                    join_bad.push(format!("{} #{i} {}x{} {:?}: DW8 index {dw8_index}, data entry {}", f.strip_prefix(root).unwrap().display(), t.width, t.height, t.format, t.data_entry_index()));
                }
                if !t.tiled || !t.format.is_dxt() || t.width < 128 || t.height < 128 {
                    continue;
                }
                // Cubes: the slot is 6 × the 2D chain — skip by the research's own key (the slot
                // capacity against the 2D chain size).
                let mips = descriptor_mip_count(&orig);
                let chain: usize = (0..mips).map(|l| tdb_rx2::texlib::tiled_buffer_size((t.width >> l).max(1), (t.height >> l).max(1), t.format.bytes_per_block())).sum();
                if t.buffer_capacity() >= 6 * chain && chain > 0 {
                    continue;
                }
                gated += 1;
                match patch_descriptor_for_resize(&orig, t.width, t.height, t.format, mips) {
                    Ok(d) if d == orig => reproduced += 1,
                    Ok(d) => {
                        if mismatches.len() < 12 {
                            let diff: Vec<String> = (0..14).filter(|w| d[w * 4..w * 4 + 4] != orig[w * 4..w * 4 + 4]).map(|w| format!("DW{w}: {:02x?} vs {:02x?}", &orig[w * 4..w * 4 + 4], &d[w * 4..w * 4 + 4])).collect();
                            mismatches.push(format!("{} #{i} {}x{} {:?} mips {mips}: {}", f.strip_prefix(root).unwrap().display(), t.width, t.height, t.format, diff.join(", ")));
                        }
                    }
                    Err(tdb_rx2::texlib::DescriptorRuleRefusal::CubeMap) => refused += 1,
                    // NHL 09 carries a subset of descriptors in the older `DW0 = 3` layout (243 of
                    // 13 059 gated rasters — hands, FE blockers …): the rule refuses them by the
                    // generation gate rather than guessing; not a mismatch.
                    Err(tdb_rx2::texlib::DescriptorRuleRefusal::UnmeasuredGeneration { .. }) => older += 1,
                    Err(e) => {
                        if mismatches.len() < 12 {
                            mismatches.push(format!("{} #{i}: refused: {e}", f.strip_prefix(root).unwrap().display()));
                        }
                    }
                }
            }
        }
        eprintln!("{title}: {} rx2 files, {texlibs} texlibs, {gated} gated rasters, {reproduced} reproduced, {refused} cube maps refused, {older} older-layout (DW0 = 3) refused, {} mismatches", files.len(), gated - reproduced - refused - older);
        for m in &mismatches {
            eprintln!("  {m}");
        }
        eprintln!("{title}: DW8 slot join {join_ok} / {join_checked} descriptors address their own pixel-data entry");
        for m in &join_bad {
            eprintln!("  join: {m}");
        }
        assert_eq!(join_ok, join_checked, "{title}: every descriptor's DW8 addresses its paired pixel-data entry (handoff R42 §1; the write path refuses the rest)");
        assert!(gated >= min_textures, "{title}: gate covered {gated} rasters");
        assert_eq!(gated - refused - older, reproduced, "{title}: every gated 2D descriptor of the measured layout is reproduced by the rule (first mismatches above)");
    }
}

/// A real multi-raster Legacy texlib (read only) resized through `replace_with_resize`: the
/// target raster halves (a synthetic chain), its descriptor equals the rule's output with `DW8`
/// unchanged, the other rasters' descriptors and de-tiled base mips are byte-identical, the
/// container re-parses, and the data-region reflow keeps every non-target slot intact.
#[test]
#[ignore = "real Xbox 360 install under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn a_real_legacy_texlib_resizes_through_the_rule_and_keeps_its_other_rasters() {
    use tdb_rx2::texlib::{base_mip_byte_size, LinearMip};
    let root = Path::new(r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\rendering\jersey");
    assert!(root.is_dir(), "{} missing (fails, never skips)", root.display());
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    let mut done = 0usize;
    for f in files.iter().filter(|f| f.file_name().unwrap().to_string_lossy().starts_with("texlib")).take(400) {
        let bytes = std::fs::read(f).unwrap();
        if !bytes.starts_with(b"\x89RW4xb2") {
            continue;
        }
        let Ok(lib) = parse(&bytes) else { continue };
        let Some(target) = lib.textures.iter().position(|t| t.tiled && t.format.is_dxt() && t.width >= 256 && t.height >= 256 && t.width.is_power_of_two() && t.height.is_power_of_two()) else { continue };
        let t = &lib.textures[target];
        let off = t.descriptor_offset();
        let orig: [u8; DESCRIPTOR_LEN] = bytes[off..off + DESCRIPTOR_LEN].try_into().unwrap();
        if patch_descriptor_for_resize(&orig, t.width, t.height, t.format, descriptor_mip_count(&orig)).ok() != Some(orig) {
            continue; // a cube or an exception — the census covers those
        }
        let (nw, nh) = (t.width / 2, t.height / 2);
        let levels: Vec<(u32, u32, Vec<u8>)> = (0..).map(|l| ((nw >> l).max(1), (nh >> l).max(1))).take_while(|&(w, h)| w >= 4 && h >= 4).map(|(w, h)| (w, h, vec![0x5Au8; base_mip_byte_size(t.format, w, h)])).collect();
        let mips: Vec<LinearMip<'_>> = levels.iter().map(|(w, h, d)| LinearMip { width: *w, height: *h, linear_dxt: d }).collect();
        let out = lib.replace_with_resize(target, t.format, &mips).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        let lib2 = parse(&out).unwrap_or_else(|e| panic!("{}: re-parse: {e}", f.display()));
        assert_eq!(lib2.textures.len(), lib.textures.len());
        let t2 = &lib2.textures[target];
        assert_eq!((t2.width, t2.height, t2.format), (nw, nh, t.format), "{}", f.display());
        let new_desc: [u8; DESCRIPTOR_LEN] = out[t2.descriptor_offset()..t2.descriptor_offset() + DESCRIPTOR_LEN].try_into().unwrap();
        assert_eq!(new_desc, patch_descriptor_for_resize(&orig, nw, nh, t.format, mips.len() as u32).unwrap(), "{}: descriptor by the rule", f.display());
        assert_eq!(&new_desc[32..36], &orig[32..36], "{}: DW8 (slot reference) unchanged", f.display());
        assert_eq!(descriptor_mip_count(&new_desc), mips.len() as u32);
        for (i, (a, b)) in lib.textures.iter().zip(&lib2.textures).enumerate() {
            if i == target {
                continue;
            }
            assert_eq!((a.width, a.height, a.format, a.tiled), (b.width, b.height, b.format, b.tiled), "{} #{i}", f.display());
            assert_eq!(a.base_mip_linear, b.base_mip_linear, "{} #{i}: base mip intact after the reflow", f.display());
            assert_eq!(&bytes[a.descriptor_offset()..a.descriptor_offset() + DESCRIPTOR_LEN], &out[b.descriptor_offset()..b.descriptor_offset() + DESCRIPTOR_LEN], "{} #{i}: descriptor intact", f.display());
        }
        done += 1;
        eprintln!("{}: raster #{target} {}x{} → {nw}x{nh}, {} mips, {} → {} bytes", f.strip_prefix(root).unwrap().display(), t.width, t.height, mips.len(), bytes.len(), out.len());
        if done == 3 {
            break;
        }
    }
    assert_eq!(done, 3, "three multi-raster jersey texlibs resized");
}

/// Diagnostic (kept): the raster shapes of the first jersey texlibs, so the resize case's target
/// filter can be checked against what the tree holds.
#[test]
#[ignore = "real Xbox 360 install under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn jersey_texlib_shapes() {
    let root = Path::new(r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\rendering\jersey");
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    for f in files.iter().filter(|f| f.file_name().unwrap().to_string_lossy().starts_with("texlib")).take(3) {
        let bytes = std::fs::read(f).unwrap();
        match parse(&bytes) {
            Ok(lib) => eprintln!("{}: {:?}", f.file_name().unwrap().to_string_lossy(), lib.textures.iter().map(|t| (t.name.clone(), t.width, t.height, t.format, t.tiled)).collect::<Vec<_>>()),
            Err(e) => eprintln!("{}: {e}", f.display()),
        }
    }
}

/// P2.1 (22 Sep 2026) — the **NHL 07 / 08 generation** (`DW0 = 0x00000003`), whose descriptors
/// `patch_descriptor_for_resize` refused as `UnmeasuredGeneration` until research 2026-09-21 pm
/// handoff R42 §2 measured every dimension-dependent word on their full populations (NHL 07 pitch
/// 10 666 / 10 666, `DW12` 9 554 / 9 555; NHL 08 14 066 / 14 066 after the RefPack wrapper comes
/// off). With the gate lifted the rule must **reproduce** those shipped descriptors exactly, and
/// the `DW8` join must hold, on both real trees — NHL 08 read through its wrapper.
///
/// The container writer is a separate question and stays refused for NHL 07's table-at-EOF layout;
/// that refusal is asserted here too, so lifting the descriptor gate cannot be mistaken for
/// lifting the write gate.
#[test]
#[ignore = "real Xbox 360 installs under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn the_nhl_07_and_08_generation_is_reproduced_by_the_same_rule() {
    for (title, root, wrapped, min_desc) in [
        ("nhl07", r"X:\Games\Emulation\Roms\XBOX360\NHL 07\NHL 07 (USA)\rendering", false, 6_000usize),
        ("nhl08", r"X:\Games\Emulation\Roms\XBOX360\NHL 08\NHL 08 (USA)\rendering", true, 9_000),
    ] {
        let root = Path::new(root);
        assert!(root.is_dir(), "{title}: {} missing (fails, never skips)", root.display());
        let mut files = Vec::new();
        walk(root, &mut files);
        let (mut containers, mut undecodable, mut gated, mut reproduced) = (0usize, 0usize, 0usize, 0usize);
        let (mut join_checked, mut join_ok, mut null_format) = (0usize, 0usize, 0usize);
        let (mut refused_small, mut refused_cube, mut refused_other) = (0usize, 0usize, 0usize);
        let mut mismatches: Vec<String> = Vec::new();
        for f in &files {
            let Ok(raw) = std::fs::read(f) else { continue };
            // NHL 08 wraps every loose `.rx2` in a RefPack stream (handoff R42 §3). 110 jersey
            // home / away libraries carry an undecodable stream and are counted, not asserted.
            let bytes = if wrapped && tdb_archive_codec::is_refpack(&raw) {
                match tdb_archive_codec::refpack_decompress(&raw) {
                    Ok(b) => b,
                    Err(_) => {
                        undecodable += 1;
                        continue;
                    }
                }
            } else {
                raw
            };
            if !bytes.starts_with(b"\x89RW4xb2") {
                continue;
            }
            let Ok(lib) = parse(&bytes) else { continue };
            containers += 1;
            for t in lib.textures.iter() {
                let off = t.descriptor_offset();
                if off + DESCRIPTOR_LEN > bytes.len() {
                    continue;
                }
                let orig: [u8; DESCRIPTOR_LEN] = bytes[off..off + DESCRIPTOR_LEN].try_into().unwrap();
                if u32::from_be_bytes(orig[0..4].try_into().unwrap()) != 0x0000_0003 {
                    continue;
                }
                join_checked += 1;
                if tdb_rx2::texlib::descriptor_data_entry_index(&orig) == t.data_entry_index() {
                    join_ok += 1;
                }
                if !t.tiled || !t.format.is_dxt() || t.width < 128 || t.height < 128 {
                    continue;
                }
                let mips = descriptor_mip_count(&orig);
                let chain: usize = (0..mips).map(|l| tdb_rx2::texlib::tiled_buffer_size((t.width >> l).max(1), (t.height >> l).max(1), t.format.bytes_per_block())).sum();
                if chain > 0 && t.buffer_capacity() >= 6 * chain {
                    continue; // cube map, by the research's own key
                }
                gated += 1;
                match patch_descriptor_for_resize(&orig, t.width, t.height, t.format, mips) {
                    Ok(d) if d == orig => reproduced += 1,
                    Ok(d) => {
                        if mismatches.len() < 12 {
                            let diff: Vec<String> = (0..14).filter(|w| d[w * 4..w * 4 + 4] != orig[w * 4..w * 4 + 4]).map(|w| format!("DW{w}: {:02x?} vs {:02x?}", &orig[w * 4..w * 4 + 4], &d[w * 4..w * 4 + 4])).collect();
                            mismatches.push(format!("{} {}x{} {:?} mips {mips}: {}", f.strip_prefix(root).unwrap().display(), t.width, t.height, t.format, diff.join(", ")));
                        }
                    }
                    Err(tdb_rx2::texlib::DescriptorRuleRefusal::NullFormatDescriptor) => null_format += 1,
                    Err(tdb_rx2::texlib::DescriptorRuleRefusal::TooSmall { .. }) => refused_small += 1,
                    // The `DW12` dimension bits say cube; the capacity pre-filter above only
                    // catches the ones whose slot is a clean 6 × the 2D chain (NHL 07's two
                    // `environment/common/*` cubes are not). Research R42 §2 lists 2 / 2 for
                    // NHL 07 and 5 / 5 for NHL 08 — the same count this arm sees.
                    Err(tdb_rx2::texlib::DescriptorRuleRefusal::CubeMap) => refused_cube += 1,
                    Err(tdb_rx2::texlib::DescriptorRuleRefusal::UnmeasuredGeneration { dw0 }) => {
                        panic!("{title}: the lifted gate still refuses DW0 = {dw0:#010x}");
                    }
                    Err(e) => {
                        refused_other += 1;
                        if mismatches.len() < 12 {
                            mismatches.push(format!("{} {}x{} {:?} mips {mips}: REFUSED: {e}", f.strip_prefix(root).unwrap().display(), t.width, t.height, t.format));
                        }
                    }
                }
            }
        }
        eprintln!("{title}: {} files, {containers} containers, {undecodable} undecodable RefPack, {join_ok}/{join_checked} DW8 join, {gated} gated, {reproduced} reproduced, {null_format} null-format, {refused_small} too-small, {refused_cube} cube maps, {refused_other} other refusals, {} unreproduced", files.len(), gated - reproduced - refused_cube);
        for m in &mismatches {
            eprintln!("  {m}");
        }
        assert!(join_checked >= min_desc, "{title}: only {join_checked} DW0 = 3 descriptors seen");
        assert_eq!(join_ok, join_checked, "{title}: every DW0 = 3 descriptor's DW8 addresses its own pixel-data entry");
        assert_eq!(refused_other, 0, "{title}: an unexpected refusal class (listed above)");
        assert_eq!(reproduced, gated - refused_cube, "{title}: the lifted rule reproduces every gated 2D shipped descriptor (cube maps are refused by design; listed above)");
        if wrapped {
            // Handoff R42 §3: exactly the 110 jersey home / away libraries fail to decode.
            assert_eq!(undecodable, 110, "nhl08: the undecodable RefPack set is the measured 110 jersey libraries");
        }
    }
}

/// The NHL 07 container layout (`data_size` 0, buffers in the header region at absolute offsets)
/// is refused by the **writer** with its own reason — lifting the descriptor gate did not open a
/// write path whose reflow is not implemented for that layout.
#[test]
#[ignore = "real Xbox 360 install under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn the_nhl_07_container_layout_is_still_refused_by_the_writer() {
    let root = Path::new(r"X:\Games\Emulation\Roms\XBOX360\NHL 07\NHL 07 (USA)\rendering");
    assert!(root.is_dir(), "{} missing (fails, never skips)", root.display());
    let mut files = Vec::new();
    walk(root, &mut files);
    let mut tried = 0usize;
    for f in &files {
        let Ok(bytes) = std::fs::read(f) else { continue };
        if !bytes.starts_with(b"\x89RW4xb2") {
            continue;
        }
        let Ok(lib) = parse(&bytes) else { continue };
        let Some((i, t)) = lib.textures.iter().enumerate().find(|(_, t)| t.format.is_dxt() && t.width >= 256 && t.height >= 256) else { continue };
        let (w, h) = (t.width / 2, t.height / 2);
        let linear = vec![0u8; tdb_rx2::texlib::base_mip_byte_size(t.format, w, h)];
        let err = lib
            .replace_with_resize(i, t.format, &[tdb_rx2::texlib::LinearMip { width: w, height: h, linear_dxt: &linear }])
            .expect_err("the NHL 07 layout must refuse a resize");
        let text = err.to_string();
        assert!(text.contains("table-at-EOF") || text.contains("absolute offsets"), "unexpected refusal: {text}");
        assert_eq!(std::fs::read(f).unwrap(), bytes, "the source file is untouched");
        tried += 1;
        if tried >= 20 {
            break;
        }
    }
    assert!(tried >= 5, "only {tried} NHL 07 containers exercised");
}
