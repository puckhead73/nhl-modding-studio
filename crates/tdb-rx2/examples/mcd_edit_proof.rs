//! `mcd_edit_proof` — the N19-H1 offline writer proof for one PS4 `.mcd` container.
//!
//! ```text
//! cargo run -p tdb-rx2 --example mcd_edit_proof -- <donor.mcd> <out-dir>
//! ```
//!
//! Runs, in order:
//!
//! 1. **Structural report** — declared ownership (submesh → format → streams → index
//!    buffer), from the container's own records rather than the preview decoder's
//!    range-validation heuristic.
//! 2. **No-op proof** — reserialize with no edits and require byte-for-byte equality
//!    with the input. The output is rebuilt from the region tiling, so equality is
//!    evidence the tiling is exhaustive and the typed regions round-trip.
//! 3. **Positional patch** — displace a small, explicitly bounded exterior patch of
//!    `geo_headShape` along +Z, refresh that submesh's tail AABB, reserialize, then
//!    prove every differing byte falls inside an allowlisted writable span and that
//!    every protected submesh's positions are bit-identical.
//! 4. **Reparse** — reload the patched bytes and check counts, ownership and the
//!    intended displacement survived.
//!
//! Writes `noop.mcd` and `patched.mcd` into `<out-dir>`. Reads the donor read-only.

use std::collections::BTreeSet;

use tdb_rx2::mcd_write::{fingerprint, McdContainer};

/// Submeshes whose positions must come back bit-identical.
const PROTECTED: [&str; 3] = ["geo_reyeShape", "geo_leyeShape", "geo_eyelashesShape"];
/// Displacement applied to the selected patch, in container units (metres).
const BUMP: f32 = 0.010;

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(path), Some(out_dir)) = (args.next(), args.next()) else {
        eprintln!("usage: mcd_edit_proof <donor.mcd> <out-dir>");
        std::process::exit(2);
    };
    let data = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("read {path}: {e}");
        std::process::exit(1);
    });
    std::fs::create_dir_all(&out_dir).unwrap();

    // Profile-gated: the measured forehead coordinates and protected-region identities
    // below are specific to a fingerprinted donor, so an arbitrary parseable `.mcd` — even
    // one with matching vertex counts — must not reach this diagnostic.
    let container = McdContainer::parse_with_profile(&data).unwrap_or_else(|e| {
        eprintln!("parse {path}: {e}");
        std::process::exit(1);
    });
    let profile = container
        .profile
        .expect("parse_with_profile sets a profile");
    let target = profile.head_submesh;
    println!(
        "# donor profile: {} (sha256 {})",
        profile.name, profile.sha256
    );
    assert_eq!(
        fingerprint(&data),
        profile.sha256,
        "profile gate is by content hash"
    );

    // ---------------------------------------------------------------- 1. structure
    println!("# {path}: {} bytes", data.len());
    let (opaque, vertex, bounds) = container.region_counts();
    println!("# regions: {opaque} opaque, {vertex} vertex, {bounds} bounds");
    println!(
        "# unclaimed index chunks (purpose unidentified, carried opaque): {:?}",
        container.unclaimed_index_chunks
    );
    println!("sub\tname\ttris\tfmt\tstreams(count x stride)\tindx@\tshared\taabb");
    for (i, sm) in container.submeshes.iter().enumerate() {
        let streams: Vec<String> = sm
            .streams
            .iter()
            .map(|&s| {
                let st = &container.streams[s];
                format!(
                    "#{s}({}x{}{})",
                    st.count,
                    st.stride,
                    st.pos_off.map_or(String::new(), |p| format!(",p@{p:#x}"))
                )
            })
            .collect();
        let aabb = sm.bounds.map_or("-".into(), |(mn, mx)| {
            format!(
                "[{:.4},{:.4},{:.4}]..[{:.4},{:.4},{:.4}]",
                mn[0], mn[1], mn[2], mx[0], mx[1], mx[2]
            )
        });
        println!(
            "{i}\t{}\t{}\t{}\t{}\t{}\t{}\t{aabb}",
            sm.name,
            sm.triangles,
            sm.format,
            streams.join("+"),
            sm.index_chunk,
            sm.shares_index_buffer
        );
    }

    // ------------------------------------------------------------------ 2. no-op
    let noop = container.serialize();
    let noop_ok = noop == data;
    println!(
        "\n# NO-OP: {} ({} bytes out)",
        if noop_ok {
            "byte-identical"
        } else {
            "MISMATCH"
        },
        noop.len()
    );
    std::fs::write(format!("{out_dir}/noop.mcd"), &noop).unwrap();
    if !noop_ok {
        let first = noop
            .iter()
            .zip(&data)
            .position(|(a, b)| a != b)
            .unwrap_or(noop.len().min(data.len()));
        eprintln!("# first differing byte at {first}");
        std::process::exit(1);
    }

    // ------------------------------------------------------------------ 3. patch
    let mut patched = container.clone();
    let sub = patched.submesh_index(target).unwrap_or_else(|| {
        eprintln!("# no {target} submesh");
        std::process::exit(1);
    });
    let stream = patched.position_stream_of(sub).unwrap();

    // Select a bounded exterior patch: upper-forehead, front-facing, off the midline
    // seam and well clear of eyes, lids, lips, mouth interior and the neck boundary.
    // Bounds are stated in absolute container coordinates so they can be checked by
    // hand against the submesh AABB printed above.
    let (lo_y, hi_y, min_z, max_abs_x) = profile.forehead_probe;

    let before: Vec<[f32; 3]> = patched.streams[stream].positions().to_vec();
    let mut after = before.clone();
    let mut touched = Vec::new();
    for (i, p) in after.iter_mut().enumerate() {
        if p[1] >= lo_y && p[1] <= hi_y && p[2] >= min_z && p[0].abs() <= max_abs_x {
            p[2] += BUMP;
            touched.push(i);
        }
    }
    println!(
        "\n# PATCH: {target} stream #{stream}, {} of {} vertices displaced +{BUMP} in Z",
        touched.len(),
        before.len()
    );
    println!(
        "# selection: y in [{lo_y},{hi_y}], z >= {min_z}, |x| <= {max_abs_x}  (first ids: {:?})",
        &touched[..touched.len().min(8)]
    );
    if touched.is_empty() {
        eprintln!("# selection matched no vertices — refusing to claim a patch");
        std::process::exit(1);
    }

    patched.set_positions(stream, &after).unwrap();
    let refreshed = patched.refresh_bounds(sub);
    println!(
        "# tail AABB refreshed: {refreshed} -> {:?}",
        patched.submeshes[sub].bounds.unwrap()
    );
    let out = patched.serialize();
    std::fs::write(format!("{out_dir}/patched.mcd"), &out).unwrap();
    println!("# wrote {} bytes", out.len());

    // ------------------------------------------------- allowlisted byte-span diff
    let spans = container.writable_spans();
    let diffs: Vec<usize> = out
        .iter()
        .zip(&data)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();
    let outside: Vec<usize> = diffs
        .iter()
        .copied()
        .filter(|&i| !spans.iter().any(|&(s, e)| i >= s && i < e))
        .collect();
    println!(
        "\n# DIFF: {} bytes differ; {} outside the writable allowlist ({} spans)",
        diffs.len(),
        outside.len(),
        spans.len()
    );
    let ranges = contiguous(&diffs);
    for (s, e) in ranges.iter().take(12) {
        let label = spans
            .iter()
            .find(|&&(a, b)| *s >= a && *s < b)
            .map_or("OUTSIDE ALLOWLIST", |_| "writable");
        println!("  {s}..{e} ({} bytes) {label}", e - s);
    }
    if ranges.len() > 12 {
        println!("  … {} more ranges", ranges.len() - 12);
    }
    if !outside.is_empty() {
        eprintln!(
            "# FAIL: {} bytes changed outside the allowlist",
            outside.len()
        );
        std::process::exit(1);
    }

    // ------------------------------------------------------- protected assertions
    let mut protected_ok = true;
    for name in PROTECTED {
        let Some(pi) = container.submesh_index(name) else {
            println!("# protected {name}: absent");
            continue;
        };
        let ps = container.position_stream_of(pi).unwrap();
        let a = container.streams[ps].positions();
        let b = patched.streams[ps].positions();
        let same = a.len() == b.len()
            && a.iter().zip(b).all(|(x, y)| {
                x[0].to_bits() == y[0].to_bits()
                    && x[1].to_bits() == y[1].to_bits()
                    && x[2].to_bits() == y[2].to_bits()
            });
        println!(
            "# protected {name}: {} positions, bit-identical={same}",
            a.len()
        );
        protected_ok &= same;
    }

    // Every untouched vertex of the target stream must also be bit-identical.
    let touched_set: BTreeSet<usize> = touched.iter().copied().collect();
    let untouched_same = before.iter().enumerate().all(|(i, p)| {
        touched_set.contains(&i)
            || (0..3).all(|k| p[k].to_bits() == patched.streams[stream].positions()[i][k].to_bits())
    });
    println!("# untouched {target} vertices bit-identical: {untouched_same}");

    // ----------------------------------------------------------------- 4. reparse
    let re = McdContainer::parse(&out).unwrap_or_else(|e| {
        eprintln!("# reparse failed: {e}");
        std::process::exit(1);
    });
    let re_stream = re
        .position_stream_of(re.submesh_index(target).unwrap())
        .unwrap();
    let rp = re.streams[re_stream].positions();
    let moved = touched
        .iter()
        .all(|&i| (rp[i][2] - before[i][2] - BUMP).abs() < 1e-6);
    let counts_ok = re.submeshes.len() == container.submeshes.len()
        && re
            .submeshes
            .iter()
            .zip(&container.submeshes)
            .all(|(a, b)| a.triangles == b.triangles && a.streams == b.streams);
    println!(
        "\n# REPARSE: submeshes={} counts_and_ownership_preserved={counts_ok} displacement_present={moved}",
        re.submeshes.len()
    );

    if protected_ok && untouched_same && counts_ok && moved {
        println!("\n# RESULT: PASS");
    } else {
        eprintln!("\n# RESULT: FAIL");
        std::process::exit(1);
    }
}

/// Collapse sorted indices into contiguous `[start, end)` ranges.
fn contiguous(idx: &[usize]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for &i in idx {
        match out.last_mut() {
            Some(last) if last.1 == i => last.1 = i + 1,
            _ => out.push((i, i + 1)),
        }
    }
    out
}
