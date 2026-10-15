//! Xbox 360 `RW4xb2` **mip placement audit** (22 Sep 2026, coordinator dispatch item 4 / research
//! R65 F1; classifier restated after the coordinator review of receipts 10–17, F2): does the
//! sub-level layout Studio's chain writers assume match where EA actually put each level? The
//! withdrawn R60 measurement (last non-zero byte inside the slot) cannot answer that — an
//! equal-sized region says nothing about *placement* — so this test uses **content**: it reads
//! each level at the offset Studio's model predicts, de-tiles and decodes it, and compares the
//! picture against a box-downscale of the base level. Two deliberately wrong offsets (one 4 KiB
//! page earlier and later) are decoded the same way as controls.
//!
//! **Declared classifier, applied uniformly to every level** (no class is relabelled afterwards):
//!
//! | class | rule |
//! |---|---|
//! | `matched-decisive` | mean absolute RGB difference `d ≤ MATCH_THRESHOLD` **and** every control differs by more than `CONTROL_MARGIN` beyond `d` |
//! | `matched-nondecisive` | `d ≤ MATCH_THRESHOLD` but a control is within the margin (flat or repetitive content: the test cannot tell the predicted offset from a neighbour) |
//! | `unmatched` | `d > MATCH_THRESHOLD` — the box-filtered base is **not** what decodes at the predicted offset. Whether the level beat both controls is recorded as data beside the class, never used to move it |
//! | `zero` | the level bytes at the predicted offset are all zero |
//!
//! What the classes mean, no more: a *matched-decisive* level is content-located at the predicted
//! offset under an approximate box-filter comparison; *matched-nondecisive* is not positive
//! placement proof; *unmatched* means the model's offset does not hold a box-filtered copy of the
//! base — it does not say where the level is, and a legitimate authoring filter could in principle
//! produce a large `d` on a correctly placed level, so the strength is "not matched by this
//! comparison", not "misplaced by the GPU's reading". **RGB only**: DXT3 / DXT5 alpha is not
//! compared. Decision values are printed unrounded (three decimals) so the boundary is visible.
//!
//! Studio's model (`replace_with_mip_chain` / `replace_with_resize`): level `k` starts at
//! `Σ_{i<k} tiled_buffer_size(w_i, h_i, bpb)` — each level tiled on its own 32 × 32-block macro-tile
//! grid and concatenated. That is a *model*; nothing outside this test establishes it for any level
//! but the base (whose tiling is validated by every preview Studio draws). The audit record is
//! `docs/validation/xbox360-mip-layout-audit-2026-09-22.md`.
//!
//! Read-only over `X:`; a missing tree fails, never skips.

use std::path::{Path, PathBuf};

use tdb_rx2::texlib::{descriptor_mip_count, parse, swap16_in_place, tiled_buffer_size, untile_dxt, PixelFormat, DESCRIPTOR_LEN};

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

// --- a minimal BC1 / BC2 / BC3 colour decoder (RGB only; alpha is not compared) -------------

fn rgb565(c: u16) -> [f32; 3] {
    [((c >> 11) & 31) as f32 * (255.0 / 31.0), ((c >> 5) & 63) as f32 * (255.0 / 63.0), (c & 31) as f32 * (255.0 / 31.0)]
}

/// Decode a linear (little-endian) DXT block stream to RGB f32 triples, row-major.
fn decode_rgb(linear: &[u8], w: u32, h: u32, format: PixelFormat) -> Vec<[f32; 3]> {
    let bpb = format.bytes_per_block();
    let (bw, bh) = (w.div_ceil(4).max(1) as usize, h.div_ceil(4).max(1) as usize);
    let mut out = vec![[0.0f32; 3]; (w * h) as usize];
    for by in 0..bh {
        for bx in 0..bw {
            let blk = &linear[(by * bw + bx) * bpb..(by * bw + bx + 1) * bpb];
            // BC2 / BC3 carry 8 alpha bytes before the colour block.
            let col = if bpb == 16 { &blk[8..16] } else { blk };
            let c0 = u16::from_le_bytes([col[0], col[1]]);
            let c1 = u16::from_le_bytes([col[2], col[3]]);
            let (p0, p1) = (rgb565(c0), rgb565(c1));
            let (p2, p3) = if c0 > c1 || bpb == 16 {
                ([(2.0 * p0[0] + p1[0]) / 3.0, (2.0 * p0[1] + p1[1]) / 3.0, (2.0 * p0[2] + p1[2]) / 3.0], [(p0[0] + 2.0 * p1[0]) / 3.0, (p0[1] + 2.0 * p1[1]) / 3.0, (p0[2] + 2.0 * p1[2]) / 3.0])
            } else {
                ([(p0[0] + p1[0]) / 2.0, (p0[1] + p1[1]) / 2.0, (p0[2] + p1[2]) / 2.0], [0.0, 0.0, 0.0])
            };
            let palette = [p0, p1, p2, p3];
            let idx = u32::from_le_bytes([col[4], col[5], col[6], col[7]]);
            for ty in 0..4 {
                for tx in 0..4 {
                    let (x, y) = (bx * 4 + tx, by * 4 + ty);
                    if x < w as usize && y < h as usize {
                        let sel = (idx >> (2 * (ty * 4 + tx))) & 3;
                        out[y * w as usize + x] = palette[sel as usize];
                    }
                }
            }
        }
    }
    out
}

/// Box-downscale an RGB image by `2^k`.
fn downscale(img: &[[f32; 3]], w: u32, h: u32, k: u32) -> (Vec<[f32; 3]>, u32, u32) {
    let (nw, nh) = ((w >> k).max(1), (h >> k).max(1));
    let f = 1u32 << k;
    let mut out = vec![[0.0f32; 3]; (nw * nh) as usize];
    for y in 0..nh {
        for x in 0..nw {
            let mut acc = [0.0f32; 3];
            let mut n = 0.0;
            for dy in 0..f {
                for dx in 0..f {
                    let (sx, sy) = (x * f + dx, y * f + dy);
                    if sx < w && sy < h {
                        let p = img[(sy * w + sx) as usize];
                        acc[0] += p[0];
                        acc[1] += p[1];
                        acc[2] += p[2];
                        n += 1.0;
                    }
                }
            }
            out[(y * nw + x) as usize] = [acc[0] / n, acc[1] / n, acc[2] / n];
        }
    }
    (out, nw, nh)
}

fn mean_abs_diff(a: &[[f32; 3]], b: &[[f32; 3]]) -> f32 {
    let n = a.len().min(b.len()).max(1) as f32;
    a.iter().zip(b).map(|(p, q)| ((p[0] - q[0]).abs() + (p[1] - q[1]).abs() + (p[2] - q[2]).abs()) / 3.0).sum::<f32>() / n
}

/// Decode level `k` of a slot as if it started at `off` (Studio's tiling for that level's own
/// dimensions). `None` when the bytes run past the slot.
fn level_rgb_at(slot: &[u8], off: usize, w: u32, h: u32, format: PixelFormat) -> Option<Vec<[f32; 3]>> {
    let bpb = format.bytes_per_block();
    let size = tiled_buffer_size(w, h, bpb);
    let mut bytes = slot.get(off..off + size)?.to_vec();
    swap16_in_place(&mut bytes);
    let linear = untile_dxt(&bytes, w, h, bpb);
    Some(decode_rgb(&linear, w, h, format))
}

#[derive(Default, Debug, Clone)]
struct LevelTally {
    /// Levels examined.
    n: usize,
    matched_decisive: usize,
    matched_nondecisive: usize,
    zero: usize,
    unmatched: usize,
    /// Of the unmatched levels, how many still differed less from the reference than both
    /// controls did (data beside the class; it does not change the class).
    unmatched_beating_controls: usize,
    diffs: Vec<f32>,
}

impl LevelTally {
    fn add(&mut self, other: &LevelTally) {
        self.n += other.n;
        self.matched_decisive += other.matched_decisive;
        self.matched_nondecisive += other.matched_nondecisive;
        self.zero += other.zero;
        self.unmatched += other.unmatched;
        self.unmatched_beating_controls += other.unmatched_beating_controls;
        self.diffs.extend_from_slice(&other.diffs);
    }

    fn summary(&self) -> String {
        let mut d = self.diffs.clone();
        d.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let pct = |p: f32| d.get(((d.len() as f32 * p) as usize).min(d.len().saturating_sub(1))).copied().unwrap_or(f32::NAN);
        format!(
            "{} levels: {} matched-decisive, {} matched-nondecisive, {} zero, {} unmatched (of which {} beat both controls); d p50 {:.3} p90 {:.3} max {:.3}",
            self.n, self.matched_decisive, self.matched_nondecisive, self.zero, self.unmatched, self.unmatched_beating_controls, pct(0.5), pct(0.9), pct(1.0)
        )
    }
}

/// `d ≤ MATCH_THRESHOLD` counts as matched. 16 / 255 is generous to EA's downsampling filter and
/// DXT re-quantisation (the p50 of matched levels is 1–4); it is a declared constant, not tuned
/// per family.
const MATCH_THRESHOLD: f32 = 16.0;
/// A control must differ by more than this beyond `d` for the match to be decisive.
const CONTROL_MARGIN: f32 = 8.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Class {
    MatchedDecisive,
    MatchedNondecisive,
    Unmatched,
    Zero,
}

/// The declared classifier — the only place a class is decided.
fn classify(all_zero: bool, d: f32, controls: &[f32]) -> Class {
    if all_zero {
        return Class::Zero;
    }
    if d > MATCH_THRESHOLD {
        return Class::Unmatched;
    }
    if controls.iter().all(|c| *c > d + CONTROL_MARGIN) {
        Class::MatchedDecisive
    } else {
        Class::MatchedNondecisive
    }
}

#[test]
#[ignore = "real Xbox 360 installs under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn sub_levels_decode_to_the_shrunken_base_at_studios_predicted_offsets_or_the_model_is_recorded_wrong() {
    const X360: &str = r"X:\Games\Emulation\Roms\XBOX360";
    // (title, rendering root, family, raster cap). The first three are the 22 Sep population; the
    // rest were added for the classifier restatement so the sampled titles are stated exactly.
    let families: [(&str, String, &str, usize); 15] = [
        ("legacy", format!(r"{X360}\NHL Legacy\NHL Legacy\rendering"), "jersey", 400),
        ("legacy", format!(r"{X360}\NHL Legacy\NHL Legacy\rendering"), "player", 400),
        ("nhl11", format!(r"{X360}\NHL 11\NHL 11\rendering"), "jersey", 300),
        ("nhl11", format!(r"{X360}\NHL 11\NHL 11\rendering"), "player", 300),
        ("nhl09", format!(r"{X360}\NHL 09\NHL 09 (USA)\rendering"), "jersey", 300),
        ("nhl09", format!(r"{X360}\NHL 09\NHL 09 (USA)\rendering"), "player", 300),
        ("nhl10", format!(r"{X360}\NHL 10\NHL 10\rendering"), "jersey", 300),
        ("nhl10", format!(r"{X360}\NHL 10\NHL 10\rendering"), "player", 300),
        ("nhl12", format!(r"{X360}\NHL 12\NHL 12\rendering"), "jersey", 300),
        ("nhl12", format!(r"{X360}\NHL 12\NHL 12\rendering"), "player", 300),
        ("nhl13", format!(r"{X360}\NHL 13\NHL 13\rendering"), "jersey", 300),
        ("nhl13", format!(r"{X360}\NHL 13\NHL 13\rendering"), "player", 300),
        ("nhl14", format!(r"{X360}\NHL 14\NHL 14\rendering"), "jersey", 300),
        ("nhl14", format!(r"{X360}\NHL 14\NHL 14\rendering"), "player", 300),
        ("nhl15", format!(r"{X360}\NHL 15\NHL 15\rendering"), "jersey", 300),
    ];
    let mut grand_large = LevelTally::default();
    let mut grand_tail = LevelTally::default();
    for (title, root, family_dir, cap) in &families {
        let root = Path::new(root);
        let dir = root.join(family_dir);
        assert!(dir.is_dir(), "{title}/{family_dir}: {} missing (fails, never skips)", dir.display());
        let mut files = Vec::new();
        walk(&dir, &mut files);
        files.sort();
        // Large levels (both dims > 64 texels, i.e. > 16 blocks) vs the packed tail (≤ 16 blocks):
        // the split this audit found by content on 22 Sep 2026 — no other measurement is cited.
        let (mut large, mut tail) = (LevelTally::default(), LevelTally::default());
        // Per (format, base shape): large / tail tallies, so the sampled formats and shapes are
        // stated exactly rather than summarised.
        let mut by_shape: std::collections::BTreeMap<(String, u32, u32), (LevelTally, LevelTally, usize)> = Default::default();
        let (mut rasters, mut alloc_equal, mut alloc_studio_larger, mut alloc_studio_smaller) = (0usize, 0usize, 0usize, 0usize);
        let mut dw0_forms: std::collections::BTreeMap<u32, usize> = Default::default();
        let mut examples: Vec<String> = Vec::new();
        'files: for f in &files {
            let Ok(bytes) = std::fs::read(f) else { continue };
            if !bytes.starts_with(b"\x89RW4xb2") {
                continue;
            }
            let Ok(lib) = parse(&bytes) else { continue };
            for (i, t) in lib.textures.iter().enumerate() {
                if !t.tiled || !t.format.is_dxt() || t.width < 256 || t.height < 256 {
                    continue;
                }
                let off = t.descriptor_offset();
                let desc: [u8; DESCRIPTOR_LEN] = bytes[off..off + DESCRIPTOR_LEN].try_into().unwrap();
                let mips = descriptor_mip_count(&desc);
                if mips < 2 {
                    continue;
                }
                let Some(slot) = lib.slot_buffer_tiled(i) else { continue };
                let bpb = t.format.bytes_per_block();
                // Studio's chain allocation at the shipped shape vs the shipped slot.
                let chain: usize = (0..mips).map(|l| tiled_buffer_size((t.width >> l).max(1), (t.height >> l).max(1), bpb)).sum();
                let chain_padded = (chain + 0xFFF) & !0xFFF;
                if chain_padded == slot.len() {
                    alloc_equal += 1;
                } else if chain_padded > slot.len() {
                    alloc_studio_larger += 1;
                } else {
                    alloc_studio_smaller += 1;
                }
                // Cubes are outside this audit (6 chains per slot).
                if slot.len() >= 6 * chain {
                    continue;
                }
                rasters += 1;
                *dw0_forms.entry(u32::from_be_bytes([desc[0], desc[1], desc[2], desc[3]])).or_default() += 1;
                let shape_key = (format!("{:?}", t.format), t.width, t.height);
                let shape = by_shape.entry(shape_key).or_default();
                shape.2 += 1;
                let base = decode_rgb(&t.base_mip_linear, t.width, t.height, t.format);
                let mut offset = tiled_buffer_size(t.width, t.height, bpb);
                for k in 1..mips {
                    let (w, h) = ((t.width >> k).max(1), (t.height >> k).max(1));
                    if w < 8 || h < 8 {
                        break;
                    }
                    let (reference, _, _) = downscale(&base, t.width, t.height, k);
                    let is_large = w > 64 && h > 64;
                    let size = tiled_buffer_size(w, h, bpb);
                    let Some(level) = level_rgb_at(slot, offset, w, h, t.format) else { break };
                    let all_zero = slot[offset..offset + size].iter().all(|&b| b == 0);
                    let d = if all_zero { f32::NAN } else { mean_abs_diff(&level, &reference) };
                    let controls: Vec<f32> = if all_zero {
                        Vec::new()
                    } else {
                        [offset.checked_sub(0x1000), Some(offset + 0x1000)]
                            .into_iter()
                            .flatten()
                            .filter_map(|o| level_rgb_at(slot, o, w, h, t.format))
                            .map(|img| mean_abs_diff(&img, &reference))
                            .collect()
                    };
                    let class = classify(all_zero, d, &controls);
                    let beats = !all_zero && controls.iter().all(|c| *c > d + CONTROL_MARGIN);
                    for tally in [if is_large { &mut large } else { &mut tail }, if is_large { &mut shape.0 } else { &mut shape.1 }] {
                        tally.n += 1;
                        match class {
                            Class::Zero => tally.zero += 1,
                            Class::MatchedDecisive => tally.matched_decisive += 1,
                            Class::MatchedNondecisive => tally.matched_nondecisive += 1,
                            Class::Unmatched => {
                                tally.unmatched += 1;
                                if beats {
                                    tally.unmatched_beating_controls += 1;
                                }
                            }
                        }
                        if !all_zero {
                            tally.diffs.push(d);
                        }
                    }
                    if class == Class::Unmatched && (is_large || examples.len() < 10) {
                        examples.push(format!(
                            "unmatched ({}): {} #{i} {}x{} {:?} mips {mips} level {k} ({w}x{h}) at +0x{offset:x}: d {d:.3}, controls {:?}, beats controls {beats}",
                            if is_large { "large" } else { "tail" },
                            f.strip_prefix(root).unwrap().display(),
                            t.width,
                            t.height,
                            t.format,
                            controls.iter().map(|c| format!("{c:.3}")).collect::<Vec<_>>()
                        ));
                    }
                    offset += size;
                }
                if rasters >= *cap {
                    break 'files;
                }
            }
        }
        eprintln!("{title}/{family_dir}: {rasters} mipped 2D DXT rasters ≥ 256² (RGB compared; alpha not compared); DW0 forms {:?}", dw0_forms.iter().map(|(k, v)| format!("0x{k:08X}:{v}")).collect::<Vec<_>>());
        eprintln!("  large levels (> 16 blocks): {}", large.summary());
        eprintln!("  tail levels (≤ 16 blocks):  {}", tail.summary());
        eprintln!("  Studio chain allocation vs shipped slot: {alloc_equal} equal, {alloc_studio_larger} Studio larger, {alloc_studio_smaller} Studio smaller");
        for ((fmt, w, h), (l, t, n)) in &by_shape {
            eprintln!("  shape {fmt} {w}x{h} ({n} rasters): large {} | tail {}", l.summary(), t.summary());
        }
        for e in &examples {
            eprintln!("  {e}");
        }
        grand_large.add(&large);
        grand_tail.add(&tail);
        assert!(rasters >= 100, "{title}/{family_dir}: only {rasters} rasters examined — the audit needs a population");
        assert!(large.n > 0 && tail.n > 0, "{title}/{family_dir}: both level classes must be represented");
    }
    eprintln!("ALL families — large: {}", grand_large.summary());
    eprintln!("ALL families — tail:  {}", grand_tail.summary());
}
