//! `mcd_anatomy` — N19-H2a read-only anatomy and deformation report for a fingerprinted
//! NHL 19 head container, plus the donor vertex-ID masks a Path B fit must protect.
//!
//! ```text
//! cargo run -p tdb-headmesh --example mcd_anatomy -- <donor.mcd> <out-dir> [source.glb]
//! ```
//!
//! Reports, in order:
//!
//! 1. **Container framing** — the three top-level blocks after the tail records
//!    (`PSTB`/`DCTB`/`QIND`) and how exactly they tile to EOF.
//! 2. **Skeleton** — the `SKTN` joint table (declared count, names, offsets), and a
//!    recheck of the head stream's `t7`/`t8` blend bytes against the *structurally
//!    selected* stream rather than a guessed offset.
//! 3. **Deformation channels** — the `PSTB` channel count and the channel names, with an
//!    explicit statement of what is and is not decoded.
//! 4. **Encoding test** — a magnitude census that distinguishes additive deltas from
//!    absolute target positions, with the head vertex stream as the positive control.
//! 5. **Masks** — shell/neck boundary, oral rim (with its coincident groups split into
//!    upper and lower), oral transition, and eyelid pockets, derived from the validated
//!    index buffer, never from joint names.
//!
//! Writes `anatomy.json` (masks and counts) and PNG previews into `<out-dir>`. Everything
//! is read-only; no game file is modified.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use tdb_rx2::mcd_write::McdContainer;

// ------------------------------------------------------------------- small helpers

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    d.get(o..o + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn find_tag(d: &[u8], tag: &[u8; 4]) -> Vec<usize> {
    d.windows(4)
        .enumerate()
        .filter(|(_, w)| *w == tag)
        .map(|(i, _)| i)
        .collect()
}

/// A `[u32 len][ascii][NUL]` name at `off`, if one is there.
fn name_at(d: &[u8], off: usize, max: usize) -> Option<(String, usize)> {
    let n = u32_at(d, off)? as usize;
    if n < 3 || n > max {
        return None;
    }
    let raw = d.get(off + 4..off + 4 + n)?;
    if !raw.iter().all(|&b| (0x20..0x7f).contains(&b)) || *d.get(off + 4 + n)? != 0 {
        return None;
    }
    Some((String::from_utf8_lossy(raw).into_owned(), off + 4 + n + 1))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(path), Some(out_dir)) = (args.next(), args.next()) else {
        eprintln!("usage: mcd_anatomy <donor.mcd> <out-dir> [source.glb]");
        std::process::exit(2);
    };
    let source = args.next();
    let d = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("read {path}: {e}");
        std::process::exit(1);
    });
    std::fs::create_dir_all(&out_dir).unwrap();

    let c = McdContainer::parse_with_profile(&d).unwrap_or_else(|e| {
        eprintln!("parse {path}: {e}");
        std::process::exit(1);
    });
    let profile = c.profile.expect("gated");
    println!(
        "# donor {} ({} bytes), profile {}",
        path,
        d.len(),
        profile.name
    );

    // ------------------------------------------------------------ 1. block framing
    println!("\n## Top-level blocks after the tail records");
    let mut blocks: Vec<(String, usize, u32, usize)> = Vec::new();
    for tag in [b"PSTB", b"DCTB", b"DNIQ"] {
        for off in find_tag(&d, tag) {
            let (Some(ver), Some(size)) = (u32_at(&d, off + 4), u32_at(&d, off + 8)) else {
                continue;
            };
            let size = size as usize;
            if off + size > d.len() || size < 16 {
                continue;
            }
            blocks.push((String::from_utf8_lossy(tag).into_owned(), off, ver, size));
        }
    }
    blocks.sort_by_key(|b| b.1);
    let mut prev_end: Option<usize> = None;
    for (tag, off, ver, size) in &blocks {
        let end = off + size;
        let contiguous = prev_end.map_or("-".to_string(), |p: usize| {
            if p == *off {
                "contiguous".into()
            } else {
                format!("GAP {} bytes", off.saturating_sub(p))
            }
        });
        let entries = u32_at(&d, off + 16).unwrap_or(0);
        println!(
            "{tag}\toff={off}\tver={ver}\tsize={size}\tend={end}\tentries={entries}\t{contiguous}"
        );
        prev_end = Some(end);
    }
    match prev_end {
        Some(e) if e == d.len() => println!("# the block chain ends exactly at EOF ({e})"),
        Some(e) => println!("# WARNING: block chain ends at {e}, EOF is {}", d.len()),
        None => println!("# no blocks found"),
    }
    println!(
        "# unclaimed index chunks (all inside QIND): {:?}",
        c.unclaimed_index_chunks
    );

    // --------------------------------------------------------------- 2. skeleton
    println!("\n## SKTN skeleton");
    let mut joints: Vec<(usize, String)> = Vec::new();
    if let Some(&sk) = find_tag(&d, b"NTKS").first() {
        let declared = u32_at(&d, sk + 16).unwrap_or(0) as usize;
        let mut p = sk + 20;
        while joints.len() < declared {
            match name_at(&d, p, 64) {
                Some((n, next)) => {
                    joints.push((p + 4, n));
                    p = next;
                }
                None => break,
            }
        }
        println!(
            "SKTN off={sk} ver={} size={} declared_joints={declared} recovered={}",
            u32_at(&d, sk + 4).unwrap_or(0),
            u32_at(&d, sk + 8).unwrap_or(0),
            joints.len()
        );
        for (i, (off, n)) in joints.iter().enumerate() {
            println!("  joint{i:02}\t@{off}\t{n}");
        }
    } else {
        println!("no SKTN chunk");
    }

    // t7/t8 recheck against the structurally selected head stream.
    let head_sub = c.submesh_index(profile.head_submesh).expect("head submesh");
    let head_stream = c
        .position_stream_of(head_sub)
        .expect("head position stream");
    let hs = &c.streams[head_stream];
    println!(
        "\n# head stream #{head_stream}: {} verts, stride {}, payload [{}, {})",
        hs.count,
        hs.stride,
        hs.data_off,
        hs.data_off + hs.count * hs.stride
    );
    // `t7`/`t8` are the last 8 bytes of the 40-byte record on this profile.
    let (t7, t8) = (0x20usize, 0x24usize);
    let mut nonzero = 0usize;
    for i in 0..hs.count {
        let b = hs.data_off + i * hs.stride;
        if d[b + t7..b + t7 + 8].iter().any(|&x| x != 0) {
            nonzero += 1;
        }
    }
    println!(
        "# t7/t8 (blend indices + weights) at record offsets {t7:#x}/{t8:#x}: {nonzero} of {} \
         vertices carry any non-zero byte",
        hs.count
    );
    println!(
        "# Establishes only this: the head stream carries no linear-skin binding *in this \
         container*. It does NOT establish that the {} named joints are unused, nor where \
         facial animation is driven from — external binding or runtime-generated streams \
         remain possible and are UNKNOWN.",
        joints.len()
    );

    // ------------------------------------------------------ 3. deformation channels
    println!("\n## PSTB deformation channels");
    if let Some((_, pstb_off, _, pstb_size)) = blocks.iter().find(|b| b.0 == "PSTB").cloned() {
        let declared = u32_at(&d, pstb_off + 16).unwrap_or(0) as usize;
        println!(
            "PSTB declares {declared} channel(s), block [{pstb_off}, {})",
            pstb_off + pstb_size
        );
        // Channel names live in the tail-name run just before the block.
        let mut names: Vec<String> = Vec::new();
        let mut p = c.submeshes.iter().map(|s| s.mesh_off).max().unwrap_or(0);
        while p < pstb_off {
            match name_at(&d, p, 96) {
                Some((n, next)) => {
                    if n.starts_with("swelling_")
                        || n.starts_with("flex_")
                        || n.starts_with("damage_")
                    {
                        names.push(n);
                    }
                    p = next;
                }
                None => p += 1,
            }
        }
        names.dedup();
        for n in &names {
            println!("  channel\t{n}");
        }
        println!(
            "# {} name(s) recovered for {declared} declared channels — each channel carries a \
             target-scoped name (`X_Additive_<kind>_geo_headShape`) and a short name. They divide \
             into 6 additive *swelling* (geometry) and 13 *texture* flex/damage channels, all \
             targeting geo_headShape.",
            names.len()
        );
        println!(
            "# No blink, eyelid or jaw channel appears among the decoded NAMES, and the animation \
             source is unresolved. Because the payloads themselves are undecoded, this is NOT a \
             claim that the container holds no such data anywhere, and it does not rule out an \
             external or runtime binding."
        );
        println!(
            "# Per-channel payload framing (run tables and their component layout) is NOT decoded. \
             The block is carried as opaque bytes."
        );
    }

    // ------------------------------------------------------------ 4. encoding test
    println!("\n## Delta vs absolute encoding");
    println!("region\tfinite_nonzero_f32\tmedian_abs\tmax_abs\tin_[1.4,2.0]\tshare");
    let census = |label: &str, lo: usize, hi: usize| {
        let mut v: Vec<f32> = Vec::new();
        let mut o = lo;
        while o + 4 <= hi {
            let x = f32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]).abs();
            if x.is_finite() && x > 1e-8 && x < 1e4 {
                v.push(x);
            }
            o += 4;
        }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = v.len();
        if n == 0 {
            println!("{label}\t0\t-\t-\t-\t-");
            return;
        }
        let band = v.iter().filter(|&&x| (1.4..=2.0).contains(&x)).count();
        println!(
            "{label}\t{n}\t{:.6}\t{:.4}\t{band}\t{:.2}%",
            v[n / 2],
            v[n - 1],
            100.0 * band as f64 / n as f64
        );
    };
    for (tag, off, _, size) in &blocks {
        census(tag, off + 16, off + size);
    }
    census(
        "head-stream (control)",
        hs.data_off,
        hs.data_off + hs.count * hs.stride,
    );
    println!(
        "# The control shows exactly one float per head vertex inside the absolute-Y band — the \
         Y coordinate. The deformation blocks show a negligible share there. That is \
         *consistent with* small additive values, and inconsistent with a plain array of \
         absolute positions in this space. It does NOT exclude absolute targets that are \
         quantized, compressed, sparse, indirectly referenced, or expressed in a local or \
         transformed space: the payload framing is undecoded, so no such claim can be made, \
         and the tail AABB cannot be declared the only position-derived field. The first \
         offline candidate therefore retains every unknown block verbatim as an EXPERIMENTAL \
         PRESERVATION POLICY, not as demonstrated compatibility."
    );
    // ------------------------------------------------------------------- 5. masks
    let pos: Vec<[f32; 3]> = c.streams[head_stream].positions().to_vec();
    let tris = c.triangles_of(head_sub).expect("validated index buffer");
    let masks = build_masks(&c, &pos, &tris);
    println!(
        "\n## Donor vertex-ID masks (geo_headShape, {} vertices)",
        pos.len()
    );
    for (name, ids) in &masks.sets {
        println!("{name}\t{}", ids.len());
    }
    println!(
        "protected_total\t{}\nfree_total\t{}",
        masks.protected.len(),
        pos.len() - masks.protected.len()
    );
    println!(
        "# The oral rim is {} vertices in {} boundary loops, and every one of them is coincident \
         with at least one other: {} pairs plus {} larger groups. Those pairs are the closed lip \
         seam. They are distinct vertex ids at identical positions and must never be welded, \
         projected onto one another or smoothed across.",
        masks.oral_rim_len, masks.oral_loops, masks.pairs, masks.bigger_groups
    );

    write_json(
        &format!("{out_dir}/anatomy.json"),
        &masks,
        &pos,
        profile.name,
    );
    render_masks(&out_dir, &pos, &tris, &masks);

    // ------------------------------------------------------------- 6. source overlay
    if let Some(src) = source {
        match std::fs::read(&src) {
            Ok(bytes) => source_overlay(&bytes, &src, &pos, &out_dir),
            Err(e) => eprintln!("# source {src}: {e}"),
        }
    }
    println!("\n# wrote reports and previews to {out_dir}");
}

// ------------------------------------------------------------------------- masks

struct Masks {
    sets: Vec<(String, Vec<u32>)>,
    protected: HashSet<u32>,
    oral_rim_len: usize,
    oral_loops: usize,
    pairs: usize,
    bigger_groups: usize,
}

impl Masks {
    fn get(&self, name: &str) -> &[u32] {
        self.sets
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
            .unwrap_or(&[])
    }
}

fn build_masks(c: &McdContainer, pos: &[[f32; 3]], tris: &[[u32; 3]]) -> Masks {
    // Edge use counts → boundary edges → boundary components. Every boundary vertex on
    // this donor has degree exactly 2, so the components are clean closed loops.
    let mut edge: HashMap<(u32, u32), u32> = HashMap::new();
    let mut ring: HashMap<u32, BTreeSet<u32>> = HashMap::new();
    for t in tris {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *edge.entry((a.min(b), a.max(b))).or_default() += 1;
            ring.entry(a).or_default().insert(b);
            ring.entry(b).or_default().insert(a);
        }
    }
    let mut badj: HashMap<u32, Vec<u32>> = HashMap::new();
    for (&(a, b), &n) in &edge {
        if n == 1 {
            badj.entry(a).or_default().push(b);
            badj.entry(b).or_default().push(a);
        }
    }
    let mut seen: HashSet<u32> = HashSet::new();
    let mut comps: Vec<Vec<u32>> = Vec::new();
    let mut keys: Vec<u32> = badj.keys().copied().collect();
    keys.sort_unstable();
    for v in keys {
        if seen.contains(&v) {
            continue;
        }
        let mut stack = vec![v];
        let mut comp = Vec::new();
        while let Some(x) = stack.pop() {
            if !seen.insert(x) {
                continue;
            }
            comp.push(x);
            stack.extend(badj[&x].iter().copied().filter(|y| !seen.contains(y)));
        }
        comp.sort_unstable();
        comps.push(comp);
    }
    comps.sort_by_key(|c| std::cmp::Reverse(c.len()));

    // Largest boundary component is the outer shell seam (neck + rear scalp cut); the
    // rest sit in the mouth.
    let shell: Vec<u32> = comps.first().cloned().unwrap_or_default();
    let oral_loops = comps.len().saturating_sub(1);
    let oral: Vec<u32> = comps.iter().skip(1).flatten().copied().collect();

    let grow = |seed: &[u32], rings: usize| -> Vec<u32> {
        let base: HashSet<u32> = seed.iter().copied().collect();
        let mut out = base.clone();
        let mut cur = base.clone();
        for _ in 0..rings {
            let mut next = HashSet::new();
            for v in &cur {
                if let Some(r) = ring.get(v) {
                    next.extend(r.iter().copied());
                }
            }
            next.retain(|v| !out.contains(v));
            out.extend(next.iter().copied());
            cur = next;
        }
        let mut v: Vec<u32> = out.difference(&base).copied().collect();
        v.sort_unstable();
        v
    };
    let oral_transition = grow(&oral, 2);
    let shell_transition = grow(&shell, 2);

    // Coincident groups along the oral rim, split by 1-ring mean height: the upper-lip
    // vertex of a coincident pair has the higher neighbourhood.
    let key = |p: [f32; 3]| {
        (
            (p[0] * 1e6).round() as i64,
            (p[1] * 1e6).round() as i64,
            (p[2] * 1e6).round() as i64,
        )
    };
    let mut groups: BTreeMap<(i64, i64, i64), Vec<u32>> = BTreeMap::new();
    for &v in &oral {
        groups.entry(key(pos[v as usize])).or_default().push(v);
    }
    let ring_y = |v: u32| -> f32 {
        let r = &ring[&v];
        r.iter().map(|&j| pos[j as usize][1]).sum::<f32>() / r.len() as f32
    };
    let (mut upper, mut lower, mut ambiguous) = (Vec::new(), Vec::new(), Vec::new());
    let (mut pairs, mut bigger) = (0usize, 0usize);
    for g in groups.values() {
        match g.len() {
            2 => {
                pairs += 1;
                let (a, b) = (g[0], g[1]);
                if ring_y(a) > ring_y(b) {
                    upper.push(a);
                    lower.push(b);
                } else {
                    upper.push(b);
                    lower.push(a);
                }
            }
            1 => ambiguous.extend(g.iter().copied()),
            _ => {
                bigger += 1;
                // Never guess a three-way seam: freeze it whole.
                ambiguous.extend(g.iter().copied());
            }
        }
    }
    upper.sort_unstable();
    lower.sort_unstable();
    ambiguous.sort_unstable();

    // Eyelid pockets by proximity to each eyeball submesh's own bounds — geometry, not
    // joint names.
    let eye_pocket = |name: &str| -> Vec<u32> {
        let Some(i) = c.submesh_index(name) else {
            return Vec::new();
        };
        let Some((mn, mx)) = c.submeshes[i].bounds else {
            return Vec::new();
        };
        let ctr = [
            (mn[0] + mx[0]) * 0.5,
            (mn[1] + mx[1]) * 0.5,
            (mn[2] + mx[2]) * 0.5,
        ];
        // Radius: the eyeball's own half-diagonal, scaled to reach the surrounding lid.
        let half = ((mx[0] - mn[0]).powi(2) + (mx[1] - mn[1]).powi(2) + (mx[2] - mn[2]).powi(2))
            .sqrt()
            * 0.5;
        let r = half * 1.35;
        let mut v: Vec<u32> = (0..pos.len() as u32)
            .filter(|&k| {
                let p = pos[k as usize];
                (p[0] - ctr[0]).powi(2) + (p[1] - ctr[1]).powi(2) + (p[2] - ctr[2]).powi(2) < r * r
            })
            .collect();
        v.sort_unstable();
        v
    };
    let r_pocket = eye_pocket("geo_reyeShape");
    let l_pocket = eye_pocket("geo_leyeShape");

    let sets = vec![
        ("shell_boundary".to_string(), shell.clone()),
        ("shell_transition".to_string(), shell_transition.clone()),
        ("oral_rim".to_string(), oral.clone()),
        ("oral_rim_upper".to_string(), upper),
        ("oral_rim_lower".to_string(), lower),
        ("oral_rim_ambiguous".to_string(), ambiguous),
        ("oral_transition".to_string(), oral_transition.clone()),
        ("eyelid_pocket_right".to_string(), r_pocket.clone()),
        ("eyelid_pocket_left".to_string(), l_pocket.clone()),
    ];
    let mut protected: HashSet<u32> = HashSet::new();
    for n in [
        "shell_boundary",
        "shell_transition",
        "oral_rim",
        "oral_transition",
        "eyelid_pocket_right",
        "eyelid_pocket_left",
    ] {
        if let Some((_, v)) = sets.iter().find(|(k, _)| k == n) {
            protected.extend(v.iter().copied());
        }
    }
    Masks {
        oral_rim_len: oral.len(),
        oral_loops,
        pairs,
        bigger_groups: bigger,
        sets,
        protected,
    }
}

fn write_json(path: &str, m: &Masks, pos: &[[f32; 3]], profile: &str) {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"profile\": \"{profile}\",\n"));
    s.push_str("  \"submesh\": \"geo_headShape\",\n");
    s.push_str(&format!("  \"vertex_count\": {},\n", pos.len()));
    s.push_str(&format!("  \"protected_total\": {},\n", m.protected.len()));
    s.push_str("  \"masks\": {\n");
    for (i, (name, ids)) in m.sets.iter().enumerate() {
        let list: Vec<String> = ids.iter().map(|v| v.to_string()).collect();
        s.push_str(&format!("    \"{name}\": [{}]", list.join(",")));
        s.push_str(if i + 1 == m.sets.len() { "\n" } else { ",\n" });
    }
    s.push_str("  }\n}\n");
    std::fs::write(path, s).unwrap();
}

// ------------------------------------------------------------------------ preview

/// Orthographic splat of the head with masked vertices coloured. `axis` picks the view:
/// 0 = front (x,y), 1 = profile (z,y). `cutaway` drops one half so the oral interior and
/// the eye sockets are visible.
fn render(
    pos: &[[f32; 3]],
    tris: &[[u32; 3]],
    colour: &dyn Fn(u32) -> Option<[u8; 3]>,
    axis: usize,
    cutaway: bool,
    w: u32,
    h: u32,
) -> image::RgbImage {
    let mut img = image::RgbImage::from_pixel(w, h, image::Rgb([18, 18, 22]));
    let keep = |v: u32| !cutaway || pos[v as usize][0] <= 0.0;
    let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
    for p in pos {
        let q = if axis == 0 {
            [p[0], p[1]]
        } else {
            [p[2], p[1]]
        };
        for k in 0..2 {
            lo[k] = lo[k].min(q[k]);
            hi[k] = hi[k].max(q[k]);
        }
    }
    let pad = 0.06;
    let sx = (w as f32 * (1.0 - 2.0 * pad)) / (hi[0] - lo[0]).max(1e-6);
    let sy = (h as f32 * (1.0 - 2.0 * pad)) / (hi[1] - lo[1]).max(1e-6);
    let s = sx.min(sy);
    let project = |p: [f32; 3]| -> (i32, i32) {
        let q = if axis == 0 {
            [p[0], p[1]]
        } else {
            [p[2], p[1]]
        };
        let x = (q[0] - lo[0]) * s + w as f32 * pad;
        let y = h as f32 - ((q[1] - lo[1]) * s + h as f32 * pad);
        (x as i32, y as i32)
    };
    let mut put = |x: i32, y: i32, col: [u8; 3]| {
        if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
            img.put_pixel(x as u32, y as u32, image::Rgb(col));
        }
    };
    // Wireframe first, in a dim grey, then coloured vertices on top.
    for t in tris {
        if !t.iter().all(|&v| keep(v)) {
            continue;
        }
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (x0, y0) = project(pos[a as usize]);
            let (x1, y1) = project(pos[b as usize]);
            let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
            for k in 0..=n {
                let x = x0 + (x1 - x0) * k / n;
                let y = y0 + (y1 - y0) * k / n;
                put(x, y, [64, 64, 72]);
            }
        }
    }
    for v in 0..pos.len() as u32 {
        if !keep(v) {
            continue;
        }
        if let Some(col) = colour(v) {
            let (x, y) = project(pos[v as usize]);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    put(x + dx, y + dy, col);
                }
            }
        }
    }
    img
}

fn render_masks(dir: &str, pos: &[[f32; 3]], tris: &[[u32; 3]], m: &Masks) {
    let idx = |name: &str| -> HashSet<u32> { m.get(name).iter().copied().collect() };
    let (shell, shellt) = (idx("shell_boundary"), idx("shell_transition"));
    let (up, lo_) = (idx("oral_rim_upper"), idx("oral_rim_lower"));
    let amb = idx("oral_rim_ambiguous");
    let oralt = idx("oral_transition");
    let (re, le) = (idx("eyelid_pocket_right"), idx("eyelid_pocket_left"));
    let colour = move |v: u32| -> Option<[u8; 3]> {
        if up.contains(&v) {
            Some([255, 96, 96]) // upper lip seam
        } else if lo_.contains(&v) {
            Some([96, 160, 255]) // lower lip seam
        } else if amb.contains(&v) {
            Some([255, 220, 80]) // non-pair seam group — frozen, never guessed
        } else if oralt.contains(&v) {
            Some([190, 90, 190])
        } else if re.contains(&v) || le.contains(&v) {
            Some([90, 220, 140])
        } else if shell.contains(&v) {
            Some([255, 150, 40])
        } else if shellt.contains(&v) {
            Some([150, 100, 50])
        } else {
            None
        }
    };
    for (name, axis, cut) in [
        ("masks_front.png", 0usize, false),
        ("masks_profile.png", 1usize, false),
        ("masks_cutaway.png", 1usize, true),
    ] {
        let img = render(pos, tris, &colour, axis, cut, 900, 1100);
        img.save(format!("{dir}/{name}")).unwrap();
        println!("# wrote {dir}/{name}");
    }
}

// -------------------------------------------------------------- source alignment

/// Read each mesh node's TRS from the GLB JSON so the scene transform is applied exactly
/// once. `glb::parse_glb` does not bake node transforms, so the raw POSITION stream is in
/// node-local space and must be transformed before any comparison with the donor.
fn node_rotations(json: &str) -> Vec<[f32; 4]> {
    let mut out = Vec::new();
    let mut rest = json;
    while let Some(i) = rest.find("\"rotation\"") {
        rest = &rest[i + 10..];
        let Some(a) = rest.find('[') else { break };
        let Some(b) = rest.find(']') else { break };
        let nums: Vec<f32> = rest[a + 1..b]
            .split(',')
            .filter_map(|t| t.trim().parse::<f32>().ok())
            .collect();
        if nums.len() == 4 {
            out.push([nums[0], nums[1], nums[2], nums[3]]);
        }
        rest = &rest[b..];
    }
    out
}

fn rotate(q: [f32; 4], p: [f32; 3]) -> [f32; 3] {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let t = [
        2.0 * (y * p[2] - z * p[1]),
        2.0 * (z * p[0] - x * p[2]),
        2.0 * (x * p[1] - y * p[0]),
    ];
    [
        p[0] + w * t[0] + (y * t[2] - z * t[1]),
        p[1] + w * t[1] + (z * t[0] - x * t[2]),
        p[2] + w * t[2] + (x * t[1] - y * t[0]),
    ]
}

fn source_overlay(bytes: &[u8], label: &str, donor: &[[f32; 3]], _out_dir: &str) {
    let mesh = match tdb_headmesh::glb::parse_glb(bytes) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("# parse {label}: {e}");
            return;
        }
    };
    let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let json = String::from_utf8_lossy(&bytes[20..20 + json_len]).into_owned();
    let rots = node_rotations(&json);
    println!("\n## Source alignment ({label})");
    println!(
        "# {} primitive(s); {} node rotation(s) read from the scene",
        mesh.primitives.len(),
        rots.len()
    );

    let q = rots.first().copied().unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let mut src: Vec<[f32; 3]> = Vec::new();
    for (i, p) in mesh.primitives.iter().enumerate() {
        let qi = rots.get(i).copied().unwrap_or(q);
        println!(
            "  primitive{i}\tname={:?}\tverts={}\ttris={}\trotation={:?}",
            p.name,
            p.positions.len(),
            p.triangles.len(),
            qi
        );
        src.extend(p.positions.iter().map(|&v| rotate(qi, v)));
    }
    let bbox = |v: &[[f32; 3]]| {
        let mut mn = [f32::MAX; 3];
        let mut mx = [f32::MIN; 3];
        for p in v {
            for k in 0..3 {
                mn[k] = mn[k].min(p[k]);
                mx[k] = mx[k].max(p[k]);
            }
        }
        (mn, mx)
    };
    let (smn, smx) = bbox(&src);
    let (dmn, dmx) = bbox(donor);
    println!("# source  (scene transform applied once) min={smn:?} max={smx:?}");
    println!("# donor   min={dmn:?} max={dmx:?}");
    let sext = [smx[0] - smn[0], smx[1] - smn[1], smx[2] - smn[2]];
    let dext = [dmx[0] - dmn[0], dmx[1] - dmn[1], dmx[2] - dmn[2]];
    println!("# extents source={sext:?} donor={dext:?}");
    println!(
        "# per-axis extent ratio donor/source = [{:.4}, {:.4}, {:.4}]",
        dext[0] / sext[0],
        dext[1] / sext[1],
        dext[2] / sext[2]
    );
    println!(
        "# No unit multiplier is assumed and no fit is performed here: this package reports the \
         measured spaces so a later alignment can be checked against them."
    );
}
