//! PS3 board-ad raster **writer admission and chain writer** (receipt 67; research 2026-09-28 handoff 10 §8.1 — R142b, the
//! coordinator-accepted C37 contract of research receipt 7).
//!
//! A PS3 venue package (`rendering/stadium/stadium_<id>_container_<n>.rpsgl`) names its board-ad materials (`0xEF0005`, dictionary
//! name `^(adbd|boardad)`, case-insensitive). The material's preceding `0xEF0001` block holds at +256 the file-table index of its
//! slot-0 raster descriptor (`0x20003`); the descriptor's first u32 names its `0x10030` data entry, which holds the **plain** DXT chain
//! (base first, every declared level concatenated, one-block floor per axis). [`admit`] follows that edge only when every check of
//! §8.1 holds, in order; [`discover`] reports formats for census and admits nothing. [`replace_chain`] rewrites exactly the admitted
//! data entry from a same-size RGBA image, regenerating every declared level; nothing else of the file changes.
//!
//! Admitted: DXT1 (`83f1`) and DXT3 (`83f2`) only. DXT5 (`83f3`) is discovered but refused, as is any other format. No resizing.
//! Nothing here reads or writes an install, or says which copy the console loads or how the game draws it.
use std::collections::BTreeMap;

use crate::texlib::PixelFormat;

pub const TAG_NAMES: u32 = 0x00EC_0010;
pub const TAG_BLOCK: u32 = 0x00EF_0001;
pub const TAG_MATERIAL: u32 = 0x00EF_0005;
pub const TAG_DESCRIPTOR: u32 = 0x0002_0003;
pub const TAG_DATA: u32 = 0x0001_0030;
/// The defensive dimension bound of §8.1 step 4 (the measured maximum is 1024).
pub const MAX_DIM: u32 = 8192;

/// One file-table entry: `(offset, _, size, align, type index, tag)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub offset: u32,
    pub size: u32,
    pub tag: u32,
}

/// One declared level of a plain chain: `(level, width, height, offset in the entry, bytes)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    pub level: u32,
    pub width: u32,
    pub height: u32,
    pub offset: usize,
    pub len: usize,
}

/// An admitted board-ad raster edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardEdge {
    pub material: String,
    pub material_chunk: usize,
    pub descriptor: usize,
    pub entry: usize,
    pub format: PixelFormat,
    pub width: u32,
    pub height: u32,
    pub levels: u32,
    /// The data entry's absolute span `[start, start + len)` — the only bytes a replacement may change.
    pub start: usize,
    pub len: usize,
    pub chain: Vec<Level>,
}

fn be32(b: &[u8], o: usize) -> Option<u32> {
    b.get(o..o.checked_add(4)?).map(|s| u32::from_be_bytes(s.try_into().unwrap()))
}
fn be16(b: &[u8], o: usize) -> Option<u16> {
    b.get(o..o.checked_add(2)?).map(|s| u16::from_be_bytes(s.try_into().unwrap()))
}

fn is_board_name(n: &str) -> bool {
    let l = n.to_ascii_lowercase();
    l.starts_with("adbd") || l.starts_with("boardad")
}

/// `1 + floor(log2(max(w, h)))` for `max >= 1`.
pub fn max_levels(w: u32, h: u32) -> u32 {
    32 - w.max(h).leading_zeros()
}

/// The declared plain chain: every level `max(w >> l, 1) × max(h >> l, 1)` with a one-block floor, base first.
pub fn chain(format: PixelFormat, w: u32, h: u32, levels: u32) -> (Vec<Level>, usize) {
    let bpb = format.bytes_per_block();
    let (mut out, mut o) = (Vec::with_capacity(levels as usize), 0usize);
    for l in 0..levels {
        let (lw, lh) = ((w >> l).max(1), (h >> l).max(1));
        let n = lw.div_ceil(4) as usize * lh.div_ceil(4) as usize * bpb;
        out.push(Level { level: l, width: lw, height: lh, offset: o, len: n });
        o += n;
    }
    (out, o)
}

struct Table {
    chunks: Vec<Entry>,
    hdr: usize,
    names: BTreeMap<usize, String>,
}

fn table(b: &[u8]) -> Result<Table, String> {
    if b.len() < 0x50 {
        return Err("file shorter than the header".into());
    }
    let (fc, fto, hdr) = (be32(b, 0x20).unwrap() as usize, be32(b, 0x30).unwrap() as usize, be32(b, 0x44).unwrap() as usize);
    if fc.checked_mul(24).and_then(|n| n.checked_add(fto)).is_none_or(|end| end > b.len()) || hdr > b.len() {
        return Err("file table or hdr_size outside the file".into());
    }
    let chunks: Vec<Entry> = (0..fc)
        .map(|i| {
            let e = fto + 24 * i;
            Entry { offset: be32(b, e).unwrap(), size: be32(b, e + 8).unwrap(), tag: be32(b, e + 20).unwrap() }
        })
        .collect();
    // Names: only dictionary entries inside the dictionary chunk and the header region are read.
    let mut names = BTreeMap::new();
    for c in chunks.iter().filter(|c| c.tag == TAG_NAMES) {
        let off = c.offset as usize;
        if off + 8 > hdr {
            continue;
        }
        let n = (be32(b, off).unwrap() & 0x00FF_FFFF) as usize;
        let limit = (off + c.size as usize).min(hdr);
        for k in 0..n {
            if off + 20 + 12 * k > limit {
                break;
            }
            let (sp, oi) = (be32(b, off + 8 + 12 * k).unwrap() as usize, be32(b, off + 16 + 12 * k).unwrap() as usize);
            if let Some(end) = b.get(sp..).and_then(|s| s.iter().position(|x| *x == 0)) {
                names.insert(oi, b[sp..sp + end].iter().map(|c| *c as char).collect());
            }
        }
    }
    Ok(Table { chunks, hdr, names })
}

/// §8.1 step 1 (R142 checks 1–3): the material's descriptor, or why not.
fn material_descriptor(b: &[u8], t: &Table, mi: usize) -> Result<usize, String> {
    let Some(pi) = mi.checked_sub(1) else { return Err("no predecessor chunk".into()) };
    let p = t.chunks[pi];
    if p.tag != TAG_BLOCK {
        return Err(format!("predecessor type {:#x} (not 0xEF0001)", p.tag));
    }
    if p.offset as usize + p.size as usize > t.hdr {
        return Err("parameter block outside the header region".into());
    }
    if p.size < 260 {
        return Err(format!("short parameter block ({} B): slot word at +256 lies outside it", p.size));
    }
    let di = be32(b, p.offset as usize + 256).unwrap() as usize;
    let Some(d) = t.chunks.get(di) else { return Err(format!("slot word {di} out of range ({} chunks)", t.chunks.len())) };
    if d.tag != TAG_DESCRIPTOR {
        return Err(format!("slot names chunk {di} of type {:#x} (not a 0x20003 descriptor)", d.tag));
    }
    if d.size < 0x38 || d.offset as usize + d.size as usize > t.hdr {
        return Err(format!("descriptor {di}: size {} / extent outside the header region", d.size));
    }
    Ok(di)
}

/// Format discovery for census and preview: every board material → its descriptor's format name (`DXT1` / `DXT3` / `DXT5` / the
/// raw code) or the reason no descriptor is reached. Admits nothing.
pub fn discover(b: &[u8]) -> Result<Vec<(String, String)>, String> {
    let t = table(b)?;
    let mut out = Vec::new();
    for (mi, c) in t.chunks.iter().enumerate() {
        let Some(name) = t.names.get(&mi).filter(|n| c.tag == TAG_MATERIAL && is_board_name(n)) else { continue };
        out.push((
            name.clone(),
            match material_descriptor(b, &t, mi) {
                Err(why) => why,
                Ok(di) => match be16(b, t.chunks[di].offset as usize + 34).unwrap() {
                    0x83F1 => "DXT1".into(),
                    0x83F2 => "DXT3".into(),
                    0x83F3 => "DXT5".into(),
                    other => format!("{other:04x}"),
                },
            },
        ));
    }
    Ok(out)
}

/// Writer admission (§8.1, in order): `(edges, refusals)` for every board material of the package.
pub fn admit(b: &[u8]) -> Result<(Vec<BoardEdge>, Vec<(String, String)>), String> {
    let t = table(b)?;
    let mut owners: BTreeMap<usize, usize> = BTreeMap::new();
    for c in t.chunks.iter().filter(|c| c.tag == TAG_DESCRIPTOR && c.size >= 4 && c.offset as usize + 4 <= t.hdr) {
        *owners.entry(be32(b, c.offset as usize).unwrap() as usize).or_default() += 1;
    }
    let spans: Vec<(usize, usize, usize)> = t
        .chunks
        .iter()
        .enumerate()
        .filter(|(_, c)| c.tag == TAG_DATA && c.size > 0)
        .map(|(i, c)| (i, t.hdr + c.offset as usize, t.hdr + c.offset as usize + c.size as usize))
        .collect();
    let (mut edges, mut refusals) = (Vec::new(), Vec::new());
    for (mi, c) in t.chunks.iter().enumerate() {
        let Some(name) = t.names.get(&mi).filter(|nm| c.tag == TAG_MATERIAL && is_board_name(nm)) else { continue };
        let di = match material_descriptor(b, &t, mi) {
            Ok(di) => di,
            Err(why) => {
                refusals.push((name.clone(), why));
                continue;
            }
        };
        let d = t.chunks[di].offset as usize;
        let (s0, s1, w, h, lv) = (be16(b, d + 34).unwrap(), be16(b, d + 38).unwrap(), be16(b, d + 48).unwrap() as u32, be16(b, d + 50).unwrap() as u32, b[d + 52] as u32);
        let no = |r: String| (name.clone(), format!("descriptor {di}: {r}"));
        if s1 != s0 {
            refusals.push(no(format!("format words differ ({s0:04x} / {s1:04x})")));
            continue;
        }
        let format = match s0 {
            0x83F1 => PixelFormat::Dxt1,
            0x83F2 => PixelFormat::Dxt3,
            other => {
                let label = if other == 0x83F3 { "DXT5" } else { "unknown" };
                refusals.push(no(format!("format {other:04x} ({label}) not admitted for writing (DXT1 / DXT3 only)")));
                continue;
            }
        };
        if w == 0 || h == 0 {
            refusals.push(no(format!("zero dimension ({w} x {h})")));
            continue;
        }
        if w > MAX_DIM || h > MAX_DIM {
            refusals.push(no(format!("dimension above {MAX_DIM} ({w} x {h})")));
            continue;
        }
        if lv == 0 {
            refusals.push(no("zero level count".into()));
            continue;
        }
        if lv > max_levels(w, h) {
            refusals.push(no(format!("level count {lv} exceeds {} possible for {w} x {h}", max_levels(w, h))));
            continue;
        }
        let e = be32(b, d).unwrap() as usize;
        let Some(ent) = t.chunks.get(e).copied() else {
            refusals.push(no(format!("entry word {e} out of range")));
            continue;
        };
        if ent.tag != TAG_DATA {
            refusals.push(no(format!("entry {e} of type {:#x} (not a 0x10030 data entry)", ent.tag)));
            continue;
        }
        let (s, tend) = (t.hdr + ent.offset as usize, t.hdr + ent.offset as usize + ent.size as usize);
        if tend > b.len() {
            refusals.push(no(format!("data entry {e} extends past the file")));
            continue;
        }
        let (levels, want) = chain(format, w, h, lv);
        if ent.size as usize != want {
            let cube = if ent.size as usize == 6 * want { " (= 6 x chain: cube map)" } else { "" };
            refusals.push(no(format!("data entry {e}: {} B != declared plain chain {want} B{cube}", ent.size)));
            continue;
        }
        let hit: Vec<usize> = spans.iter().filter(|(i, a, z)| *i != e && *a < tend && s < *z).map(|(i, ..)| *i).collect();
        if !hit.is_empty() {
            refusals.push(no(format!("data entry {e} [{s:#x}, {tend:#x}) intersects data entr{} {:?}", if hit.len() == 1 { "y" } else { "ies" }, &hit[..hit.len().min(4)])));
            continue;
        }
        let owned = owners.get(&e).copied().unwrap_or(0);
        if owned > 1 {
            refusals.push(no(format!("data entry {e} owned by {owned} descriptors")));
            continue;
        }
        edges.push(BoardEdge { material: name.clone(), material_chunk: mi, descriptor: di, entry: e, format, width: w, height: h, levels: lv, start: s, len: want, chain: levels });
    }
    Ok((edges, refusals))
}

/// Every material of the package (any name) whose slot 0 or slot 1 names `descriptor` — the disclosure of what a replacement of
/// that raster changes. Read through [`crate::venue::read`]'s validated slots.
pub fn materials_using(b: &[u8], descriptor: usize) -> Vec<String> {
    let Ok(p) = crate::venue::read(b) else { return Vec::new() };
    let Some(ordinal) = p.rasters.iter().find(|r| r.chunk == descriptor).map(|r| r.ordinal) else { return Vec::new() };
    p.materials.iter().filter(|m| m.slots.iter().any(|s| *s == crate::venue::Slot::Raster(ordinal))).map(|m| m.name.clone()).collect()
}

/// Every declared level from a base RGBA image (`w × h`), each a 2 × 2 box average of the previous one.
pub fn levels_from_rgba(w: u32, h: u32, levels: u32, rgba: &[u8]) -> Vec<(u32, u32, Vec<u8>)> {
    let mut out = vec![(w, h, rgba.to_vec())];
    for _ in 1..levels {
        let (pw, ph, prev) = out.last().unwrap();
        let (nw, nh) = ((pw / 2).max(1), (ph / 2).max(1));
        let mut next = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                for ch in 0..4 {
                    let (mut sum, mut cnt) = (0u32, 0u32);
                    for dy in 0..2 {
                        for dx in 0..2 {
                            let (sx, sy) = ((x * 2 + dx).min(pw - 1), (y * 2 + dy).min(ph - 1));
                            sum += prev[((sy * pw + sx) * 4 + ch) as usize] as u32;
                            cnt += 1;
                        }
                    }
                    next[((y * nw + x) * 4 + ch) as usize] = ((sum + cnt / 2) / cnt) as u8;
                }
            }
        }
        out.push((nw, nh, next));
    }
    out
}

/// The package with the admitted edge's data entry replaced by the full declared chain encoded from `rgba` (`width × height` of
/// the raster, unchanged). Only `[start, start + len)` differs from `b`; the encoded chain must be exactly `len` bytes.
pub fn replace_chain(b: &[u8], edge: &BoardEdge, rgba: &[u8], encode: impl Fn(PixelFormat, u32, u32, &[u8]) -> Vec<u8>) -> Result<Vec<u8>, String> {
    if rgba.len() != edge.width as usize * edge.height as usize * 4 {
        return Err(format!("the replacement is {} bytes of RGBA, not {} × {} (resizing is not offered)", rgba.len(), edge.width, edge.height));
    }
    let mut data = Vec::with_capacity(edge.len);
    for ((lw, lh, px), level) in levels_from_rgba(edge.width, edge.height, edge.levels, rgba).iter().zip(&edge.chain) {
        let enc = encode(edge.format, *lw, *lh, px);
        if enc.len() != level.len {
            return Err(format!("level {} encoded to {} bytes, not the declared {}", level.level, enc.len(), level.len));
        }
        data.extend_from_slice(&enc);
    }
    if data.len() != edge.len || edge.start + edge.len > b.len() {
        return Err(format!("the chain is {} bytes, not the entry's {}", data.len(), edge.len));
    }
    let mut out = b.to_vec();
    out[edge.start..edge.start + edge.len].copy_from_slice(&data);
    Ok(out)
}

#[cfg(test)]
mod tests {
    //! The 29 admission vectors of research 2026-09-28 handoff 10 §8.2 (R142's 14 + R142b's 15), transferred from the research
    //! builder (`synth`) byte for byte into Rust, and run against this module's [`admit`] / [`discover`].
    use super::*;

    const DATA: u32 = TAG_DATA;

    fn put32(b: &mut [u8], o: usize, v: u32) {
        b[o..o + 4].copy_from_slice(&v.to_be_bytes());
    }
    fn put16(b: &mut [u8], o: usize, v: u16) {
        b[o..o + 2].copy_from_slice(&v.to_be_bytes());
    }

    #[derive(Clone)]
    struct V {
        block: u32,
        block_type: u32,
        desc_size: u32,
        a_off: u32,
        a_size: u32,
        a_type: u32,
        slot: u32,
        a_entry: u32,
        b_entry: u32,
        b_off: u32,
        fmt: u16,
    }
    impl Default for V {
        fn default() -> Self {
            V { block: 320, block_type: TAG_BLOCK, desc_size: 0x38, a_off: 0, a_size: 40, a_type: DATA, slot: 3, a_entry: 4, b_entry: 6, b_off: 0x100, fmt: 0x83F1 }
        }
    }

    /// R142 `synth`: 0 dictionary, 1 EF0001 block, 2 EF0005 `adbd_TEST`, 3 descriptor A → 4, 4 data A, 5 descriptor B → 6, 6 data B.
    fn synth(v: V) -> Vec<u8> {
        let (hdr, fto) = (0x400usize, 0x80usize);
        let want = 40u32; // 8 × 8 DXT1, 2 levels: 32 + 8
        let ent: [(u32, u32, u32); 7] = [
            (0x120, 0x40, TAG_NAMES),
            (0x160, v.block, v.block_type),
            (0x2C0, 0x40, TAG_MATERIAL),
            (0x300, v.desc_size, TAG_DESCRIPTOR),
            (v.a_off, v.a_size, v.a_type),
            (0x340, 0x38, TAG_DESCRIPTOR),
            (v.b_off, want, DATA),
        ];
        let mut b = vec![0u8; hdr + 0x200];
        put32(&mut b, 0x20, 7);
        put32(&mut b, 0x30, fto as u32);
        put32(&mut b, 0x44, hdr as u32);
        put32(&mut b, 0x4C, 0x200);
        for (i, (off, size, tag)) in ent.iter().enumerate() {
            put32(&mut b, fto + 24 * i, *off);
            put32(&mut b, fto + 24 * i + 8, *size);
            put32(&mut b, fto + 24 * i + 20, *tag);
        }
        put32(&mut b, 0x120, 1);
        put32(&mut b, 0x128, 0x140);
        put32(&mut b, 0x12C, 0);
        put32(&mut b, 0x130, 2);
        b[0x140..0x14A].copy_from_slice(b"adbd_TEST\0");
        put32(&mut b, 0x160 + 256, if v.block < 260 { 3 } else { v.slot });
        for (base, target, fmt) in [(0x300usize, v.a_entry, v.fmt), (0x340, v.b_entry, 0x83F1)] {
            put32(&mut b, base, target);
            put16(&mut b, base + 34, fmt);
            put16(&mut b, base + 38, fmt);
            put16(&mut b, base + 42, 0xFFFF);
            put16(&mut b, base + 48, 8);
            put16(&mut b, base + 50, 8);
            b[base + 52] = 2;
        }
        b
    }

    fn put_entry(b: &mut [u8], i: usize, off: u32, size: u32, tag: u32) {
        put32(b, 0x80 + 24 * i, off);
        put32(b, 0x80 + 24 * i + 8, size);
        put32(b, 0x80 + 24 * i + 20, tag);
    }
    fn resize(b: &mut [u8], i: usize, size: u32) {
        put32(b, 0x80 + 24 * i + 8, size);
    }

    fn outcome(b: &[u8]) -> (&'static str, Option<String>) {
        let (e, r) = admit(b).unwrap();
        (if e.is_empty() { "refuse" } else { "edge" }, r.first().map(|x| x.1.clone()))
    }

    #[test]
    fn the_29_research_vectors_have_their_expected_outcomes() {
        let d = V::default;
        let r142: Vec<(&str, V, &str)> = vec![
            ("baseline", d(), "edge"),
            ("short block (240 B)", V { block: 240, ..d() }, "refuse"),
            ("predecessor not 0xEF0001", V { block_type: 0xEF0002, ..d() }, "refuse"),
            ("block past header region", V { block: 0x400, ..d() }, "refuse"),
            ("slot word out of range", V { slot: 99, ..d() }, "refuse"),
            ("slot names a material (foreign type)", V { slot: 2, ..d() }, "refuse"),
            ("descriptor shorter than 0x38", V { desc_size: 0x20, ..d() }, "refuse"),
            ("format 6007 (texel)", V { fmt: 0x6007, ..d() }, "refuse"),
            ("entry word out of range", V { a_entry: 50, ..d() }, "refuse"),
            ("entry word names a non-data chunk", V { a_entry: 1, ..d() }, "refuse"),
            ("entry past file end", V { a_off: 0x1F0, ..d() }, "refuse"),
            ("entry size = 6 x chain (cube)", V { a_size: 240, ..d() }, "refuse"),
            ("entry owned by two descriptors", V { b_entry: 4, ..d() }, "refuse"),
            ("entry overlaps another data entry", V { b_off: 0x10, ..d() }, "refuse"),
        ];
        let mut results = Vec::new();
        for (label, v, expect) in r142 {
            results.push((label.to_string(), outcome(&synth(v)), expect));
        }
        let with = |v: V, f: &dyn Fn(&mut Vec<u8>)| {
            let mut b = synth(v);
            f(&mut b);
            b
        };
        let new: Vec<(&str, Vec<u8>, &str)> = vec![
            (
                "review: enclosing span, selected inside it beyond a contained span",
                with(V { a_off: 0x80, b_off: 0, ..d() }, &|b| {
                    put_entry(b, 5, 0x40, 0x10, DATA);
                    resize(b, 6, 0x180);
                }),
                "refuse",
            ),
            ("equal start, different size", with(V { b_off: 0, ..d() }, &|b| resize(b, 6, 80)), "refuse"),
            ("duplicate span (equal start and size)", synth(V { b_off: 0, ..d() }), "refuse"),
            ("partial overlap on the right", synth(V { b_off: 0x20, ..d() }), "refuse"),
            ("boundary-touching disjoint span", synth(V { b_off: 40, ..d() }), "edge"),
            ("empty data span inside the selected span", with(d(), &|b| put_entry(b, 5, 0x10, 0, DATA)), "edge"),
            ("DXT3 8 x 8, 2 levels (80 B)", synth(V { fmt: 0x83F2, a_size: 80, ..d() }), "edge"),
            ("DXT5 8 x 8, 2 levels (80 B): discovered, not admitted", synth(V { fmt: 0x83F3, a_size: 80, ..d() }), "refuse"),
            ("zero width (review: a_size 24)", with(V { a_size: 24, ..d() }, &|b| put16(b, 0x300 + 48, 0)), "refuse"),
            ("zero height", with(V { a_size: 24, ..d() }, &|b| put16(b, 0x300 + 50, 0)), "refuse"),
            ("zero level count (review: a_size 32)", with(V { a_size: 32, ..d() }, &|b| b[0x300 + 52] = 0), "refuse"),
            ("level count 5 > 4 possible for 8 x 8", with(V { a_size: 48, ..d() }, &|b| b[0x300 + 52] = 5), "refuse"),
            ("format words differ (83f1 / 83f2)", with(d(), &|b| put16(b, 0x300 + 38, 0x83F2)), "refuse"),
            ("declared partial chain: 8 x 8 with 2 of 4 levels (baseline)", synth(d()), "edge"),
            ("declared single level 8 x 8 (32 B)", with(V { a_size: 32, ..d() }, &|b| b[0x300 + 52] = 1), "edge"),
        ];
        for (label, b, expect) in &new {
            results.push((label.to_string(), outcome(b), expect));
            if label.starts_with("DXT5") {
                assert_eq!(discover(b).unwrap(), vec![("adbd_TEST".to_string(), "DXT5".to_string())], "discovered, not admitted");
            }
        }
        assert_eq!(results.len(), 29);
        let failed: Vec<_> = results.iter().filter(|(_, (got, _), expect)| got != expect).collect();
        assert!(failed.is_empty(), "{failed:#?}");
        // Specific reasons the research recorded.
        let reason = |label: &str| results.iter().find(|r| r.0 == label).and_then(|r| r.1 .1.clone()).unwrap();
        assert!(reason("short block (240 B)").contains("short parameter block (240 B)"));
        assert!(reason("format 6007 (texel)").contains("not admitted for writing"));
        assert!(reason("entry size = 6 x chain (cube)").contains("cube map"));
        assert!(reason("review: enclosing span, selected inside it beyond a contained span").contains("intersects"));
        assert!(reason("level count 5 > 4 possible for 8 x 8").contains("exceeds 4 possible"));
    }

    #[test]
    fn the_baseline_edge_and_a_replacement_touch_only_the_entry() {
        let b = synth(V::default());
        let (edges, _) = admit(&b).unwrap();
        let e = &edges[0];
        assert_eq!((e.descriptor, e.entry, e.format, e.width, e.height, e.levels, e.start, e.len), (3, 4, PixelFormat::Dxt1, 8, 8, 2, 0x400, 40));
        assert_eq!(e.chain.iter().map(|l| (l.width, l.height, l.len)).collect::<Vec<_>>(), vec![(8, 8, 32), (4, 4, 8)]);
        let rgba = vec![200u8; 8 * 8 * 4];
        let out = replace_chain(&b, e, &rgba, |_, w, h, _| vec![0xAB; (w.div_ceil(4) * h.div_ceil(4) * 8) as usize]).unwrap();
        assert_eq!(out.len(), b.len());
        assert!(out[..0x400] == b[..0x400] && out[0x400 + 40..] == b[0x400 + 40..] && out[0x400..0x428].iter().all(|x| *x == 0xAB));
        // Wrong size refused (no resizing); a wrongly sized encoder output refused.
        assert!(replace_chain(&b, e, &rgba[..16], |_, _, _, _| Vec::new()).unwrap_err().contains("resizing is not offered"));
        assert!(replace_chain(&b, e, &rgba, |_, _, _, _| vec![0; 3]).unwrap_err().contains("not the declared"));
    }

    #[test]
    fn levels_are_box_averages_down_to_one_texel() {
        let mut rgba = vec![0u8; 4 * 2 * 4];
        for (i, p) in rgba.chunks_mut(4).enumerate() {
            p.copy_from_slice(&[(i * 30) as u8, 0, 0, 255]);
        }
        let l = levels_from_rgba(4, 2, 3, &rgba);
        assert_eq!(l.iter().map(|(w, h, p)| (*w, *h, p.len())).collect::<Vec<_>>(), vec![(4, 2, 32), (2, 1, 8), (1, 1, 4)]);
        assert_eq!(l[1].2[0], ((0 + 30 + 120 + 150) as f32 / 4.0).round() as u8);
        assert_eq!(max_levels(512, 128), 10);
        assert_eq!(max_levels(1, 1), 1);
    }
}
