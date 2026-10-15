//! Minimal Wavefront OBJ reader for the wrap+weight-transfer pipeline.
//!
//! Handles the subset of OBJ we actually meet in NHL24 exports:
//! `v / vn / vt / f` lines, triangles only. Face vertices may use
//! either the parallel-indexed form (`f a/a/a b/b/b c/c/c`, common
//! in clean exports) or the de-duplicated form (`f p/t/n p'/t'/n' …`
//! where pos/uv/normal indices disagree, which Blender emits when
//! faces were edited and the position list compacted but the UV
//! list did not). In the de-duplicated form we **expand** every
//! unique `(pos, uv, normal)` triple into its own logical vertex so
//! downstream code sees parallel arrays; texture coords stay pinned
//! to the original atlas locations the modeller authored.
//! Everything else (`g / o / mtllib / usemtl / s` and comments)
//! is silently skipped.

use std::convert::TryInto;
use std::fs;
use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObjError {
    #[error("reading {0}: {1}")]
    Io(String, std::io::Error),
    #[error("line {line}: malformed `{kind}` directive: {detail}")]
    Malformed {
        line: usize,
        kind: String,
        detail: String,
    },
    #[error("line {line}: non-triangle face with {verts} vertices — only triangles supported")]
    NonTriangleFace { line: usize, verts: usize },
    /// Dead variant — kept for SemVer compatibility. The parser
    /// previously rejected de-duplicated face indices; it now expands
    /// each unique `(pos, uv, normal)` triple into its own vertex,
    /// so this is never returned.
    #[error(
        "line {line}: face vertex `{token}` mixes position/uv/normal indices that don't match"
    )]
    #[doc(hidden)]
    MisalignedFaceIndices { line: usize, token: String },
    #[error("line {line}: face index {idx} out of range (have {len} positions)")]
    IndexOutOfRange { line: usize, idx: usize, len: usize },
}

#[derive(Debug, Clone, Default)]
pub struct ObjMesh {
    pub positions: Vec<[f32; 3]>,
    /// Empty if the OBJ has no `vn` lines.
    pub normals: Vec<[f32; 3]>,
    /// Empty if the OBJ has no `vt` lines.
    pub uvs: Vec<[f32; 2]>,
    /// 0-based triangle indices into `positions` (and `normals` /
    /// `uvs` when populated, since we require parallel indexing).
    pub triangles: Vec<[u32; 3]>,
    /// Parallel to `triangles` — group name each face was declared
    /// under (from the most recent `g <name>` line). Empty when the
    /// OBJ has no `g` markers. NHL24 head exports use this to
    /// separate `mat_head`, `mat_eyes`, and `mat_eyelashes`.
    pub triangle_groups: Vec<String>,
}

pub fn read_obj(path: &Path) -> Result<ObjMesh, ObjError> {
    let text = fs::read_to_string(path).map_err(|e| ObjError::Io(path.display().to_string(), e))?;
    parse_obj(&text)
}

pub fn parse_obj(text: &str) -> Result<ObjMesh, ObjError> {
    // Two-stage parse:
    //   1. Accumulate raw `v / vn / vt` arrays exactly as authored,
    //      plus raw face-corner triples `(pos1, uv1, nrm1)` (1-based).
    //   2. Walk the triples; the first time we see a unique
    //      `(pos, uv, nrm)` combination, emit a new logical vertex
    //      with the corresponding position + uv + normal pulled from
    //      the raw arrays. Re-uses indices on duplicates. This both
    //      handles the parallel-indexed form (every triple is unique
    //      by definition) and the de-duplicated form (one position
    //      can appear with multiple UVs — each gets its own logical
    //      vertex so the UV array stays parallel to positions).
    let mut raw_pos: Vec<[f32; 3]> = Vec::new();
    let mut raw_nrm: Vec<[f32; 3]> = Vec::new();
    let mut raw_uv: Vec<[f32; 2]> = Vec::new();
    // Per-face raw triples (1-based; 0 means "absent").
    let mut face_triples: Vec<[(u32, u32, u32); 3]> = Vec::new();
    let mut face_groups: Vec<String> = Vec::new();

    let mut current_group: String = String::new();
    for (i, raw) in text.lines().enumerate() {
        let line_no = i + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(kind) = parts.next() else { continue };
        if kind == "g" || kind == "o" {
            current_group = parts.collect::<Vec<_>>().join(" ");
            continue;
        }
        match kind {
            "v" => raw_pos.push(parse_vec3(parts, line_no, "v")?),
            "vn" => raw_nrm.push(parse_vec3(parts, line_no, "vn")?),
            "vt" => raw_uv.push(parse_vec2(parts, line_no, "vt")?),
            "f" => {
                let toks: Vec<&str> = parts.collect();
                if toks.len() != 3 {
                    return Err(ObjError::NonTriangleFace {
                        line: line_no,
                        verts: toks.len(),
                    });
                }
                let mut triple = [(0u32, 0u32, 0u32); 3];
                for (slot, tok) in toks.iter().enumerate() {
                    let comps: Vec<&str> = tok.split('/').collect();
                    let p = comps[0].parse::<u32>().map_err(|_| ObjError::Malformed {
                        line: line_no,
                        kind: "f".into(),
                        detail: format!("bad position index in `{tok}`"),
                    })?;
                    if p == 0 || (p as usize) > raw_pos.len() {
                        return Err(ObjError::IndexOutOfRange {
                            line: line_no,
                            idx: p as usize,
                            len: raw_pos.len(),
                        });
                    }
                    let parse_opt = |idx: usize, max: usize, what: &str| -> Result<u32, ObjError> {
                        let Some(raw) = comps.get(idx) else {
                            return Ok(0);
                        };
                        if raw.is_empty() {
                            return Ok(0);
                        }
                        let v = raw.parse::<u32>().map_err(|_| ObjError::Malformed {
                            line: line_no,
                            kind: "f".into(),
                            detail: format!("bad {what} index in `{tok}`"),
                        })?;
                        if v == 0 || (v as usize) > max {
                            return Err(ObjError::IndexOutOfRange {
                                line: line_no,
                                idx: v as usize,
                                len: max,
                            });
                        }
                        Ok(v)
                    };
                    let t = parse_opt(1, raw_uv.len(), "uv")?;
                    let n = parse_opt(2, raw_nrm.len(), "normal")?;
                    triple[slot] = (p, t, n);
                }
                face_triples.push(triple);
                face_groups.push(current_group.clone());
            }
            _ => {} // ignore mtllib / usemtl / s / etc.
        }
    }

    // Stage 2: dedupe (pos, uv, nrm) triples into parallel-indexed
    // vertex arrays. If the OBJ used parallel indexing already this
    // is a 1:1 pass through (each triple is unique by construction).
    // If it used de-duplicated indexing this fissions positions that
    // appear with multiple UVs/normals into separate vertices.
    let mut dedup: std::collections::HashMap<(u32, u32, u32), u32> =
        std::collections::HashMap::new();
    let mut mesh = ObjMesh::default();
    let have_uv = !raw_uv.is_empty();
    let have_nrm = !raw_nrm.is_empty();
    let mut intern = |triple: (u32, u32, u32)| -> u32 {
        if let Some(&existing) = dedup.get(&triple) {
            return existing;
        }
        let new_idx = mesh.positions.len() as u32;
        let (p, t, n) = triple;
        mesh.positions.push(raw_pos[(p - 1) as usize]);
        if have_uv {
            // Absent uv on a face that has some UVs declared: fall
            // back to (0, 0) so downstream code still sees parallel
            // arrays. Rare in practice — Blender always emits UVs
            // consistently per-face.
            if t == 0 {
                mesh.uvs.push([0.0, 0.0]);
            } else {
                mesh.uvs.push(raw_uv[(t - 1) as usize]);
            }
        }
        if have_nrm {
            if n == 0 {
                mesh.normals.push([0.0, 0.0, 1.0]);
            } else {
                mesh.normals.push(raw_nrm[(n - 1) as usize]);
            }
        }
        dedup.insert(triple, new_idx);
        new_idx
    };
    for (tri, group) in face_triples.iter().zip(face_groups) {
        let idxs = [intern(tri[0]), intern(tri[1]), intern(tri[2])];
        mesh.triangles.push(idxs);
        mesh.triangle_groups.push(group);
    }
    Ok(mesh)
}

/// Serialise parallel vertex arrays + triangles into Wavefront OBJ
/// text. Counterpart to [`parse_obj`].
///
/// Emits `v` lines for every position, `vt` lines when `uvs` is
/// non-empty, and `vn` lines when `normals` is non-empty. Face lines
/// reference whichever channels are present using the standard
/// `p`, `p/t`, `p//n`, or `p/t/n` corner syntax (1-based indices).
/// `uvs` / `normals`, when non-empty, must be parallel to `positions`
/// (one entry per vertex); shorter arrays are treated as absent for
/// the missing tail. `object_name`, when `Some`, is written as a
/// leading `o <name>` marker.
pub fn write_obj(
    positions: &[[f32; 3]],
    uvs: &[[f32; 2]],
    normals: &[[f32; 3]],
    triangles: &[[u32; 3]],
    object_name: Option<&str>,
) -> String {
    let have_uv = uvs.len() == positions.len() && !uvs.is_empty();
    let have_nrm = normals.len() == positions.len() && !normals.is_empty();
    let mut s = String::with_capacity(positions.len() * 48 + triangles.len() * 24);
    if let Some(name) = object_name {
        s.push_str(&format!("o {name}\n"));
    }
    for p in positions {
        s.push_str(&format!("v {:.6} {:.6} {:.6}\n", p[0], p[1], p[2]));
    }
    if have_uv {
        for uv in uvs {
            s.push_str(&format!("vt {:.6} {:.6}\n", uv[0], uv[1]));
        }
    }
    if have_nrm {
        for n in normals {
            s.push_str(&format!("vn {:.6} {:.6} {:.6}\n", n[0], n[1], n[2]));
        }
    }
    for t in triangles {
        s.push('f');
        for &idx in t {
            let i = idx + 1; // OBJ is 1-based.
            match (have_uv, have_nrm) {
                (true, true) => s.push_str(&format!(" {i}/{i}/{i}")),
                (true, false) => s.push_str(&format!(" {i}/{i}")),
                (false, true) => s.push_str(&format!(" {i}//{i}")),
                (false, false) => s.push_str(&format!(" {i}")),
            }
        }
        s.push('\n');
    }
    s
}

fn parse_vec3<'a, I>(parts: I, line: usize, kind: &str) -> Result<[f32; 3], ObjError>
where
    I: IntoIterator<Item = &'a str>,
{
    let v: Vec<f32> = parts
        .into_iter()
        .filter_map(|p| p.parse::<f32>().ok())
        .collect();
    if v.len() < 3 {
        return Err(ObjError::Malformed {
            line,
            kind: kind.into(),
            detail: format!("need 3 floats, got {}", v.len()),
        });
    }
    Ok(v[..3].try_into().expect("len checked"))
}

fn parse_vec2<'a, I>(parts: I, line: usize, kind: &str) -> Result<[f32; 2], ObjError>
where
    I: IntoIterator<Item = &'a str>,
{
    let v: Vec<f32> = parts
        .into_iter()
        .filter_map(|p| p.parse::<f32>().ok())
        .collect();
    if v.len() < 2 {
        return Err(ObjError::Malformed {
            line,
            kind: kind.into(),
            detail: format!("need 2 floats, got {}", v.len()),
        });
    }
    Ok(v[..2].try_into().expect("len checked"))
}

/// One connected component of an [`ObjMesh`] — verts share at least
/// one triangle edge transitively within the component, and no edge
/// at all with verts outside.
#[derive(Debug, Clone)]
pub struct Component {
    /// Vertex indices belonging to this component (sorted ascending).
    pub vert_indices: Vec<u32>,
    /// Triangles belonging to this component (subset of the parent
    /// mesh's `triangles`).
    pub triangles: Vec<[u32; 3]>,
}

impl Component {
    pub fn bounds(&self, positions: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for &v in &self.vert_indices {
            let p = positions[v as usize];
            for c in 0..3 {
                if p[c] < min[c] {
                    min[c] = p[c];
                }
                if p[c] > max[c] {
                    max[c] = p[c];
                }
            }
        }
        (min, max)
    }
}

impl ObjMesh {
    /// Walk the triangle graph and return every connected component
    /// (groups of verts that share triangle edges). Useful for
    /// peeling disconnected mesh islands out of an OBJ — e.g. NHL24
    /// head exports include sclera/iris as separate small islands
    /// from the head proper, and we drop them via this analysis.
    pub fn connected_components(&self) -> Vec<Component> {
        let n = self.positions.len();
        let mut parent: Vec<u32> = (0..n as u32).collect();
        fn find(parent: &mut [u32], x: u32) -> u32 {
            let mut r = x;
            while parent[r as usize] != r {
                r = parent[r as usize];
            }
            // Path compression.
            let mut cur = x;
            while parent[cur as usize] != r {
                let next = parent[cur as usize];
                parent[cur as usize] = r;
                cur = next;
            }
            r
        }
        // Standard union helper — re-finds inside so we never use a
        // stale root after a previous union in the same triangle.
        fn union(parent: &mut [u32], a: u32, b: u32) {
            let ra = find(parent, a);
            let rb = find(parent, b);
            if ra != rb {
                parent[ra as usize] = rb;
            }
        }
        for t in &self.triangles {
            union(&mut parent, t[0], t[1]);
            union(&mut parent, t[1], t[2]);
        }
        // Bucket verts by final root.
        let mut groups: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
        for v in 0..n as u32 {
            let r = find(&mut parent, v);
            groups.entry(r).or_default().push(v);
        }
        // Assign each triangle to its root component.
        let mut tri_buckets: std::collections::HashMap<u32, Vec<[u32; 3]>> =
            std::collections::HashMap::new();
        for t in &self.triangles {
            let r = find(&mut parent, t[0]);
            tri_buckets.entry(r).or_default().push(*t);
        }
        let mut out: Vec<Component> = groups
            .into_iter()
            .map(|(root, mut vs)| {
                vs.sort_unstable();
                Component {
                    vert_indices: vs,
                    triangles: tri_buckets.remove(&root).unwrap_or_default(),
                }
            })
            .collect();
        // Largest first so callers can `.first()` to get the head proper.
        out.sort_by_key(|c| std::cmp::Reverse(c.vert_indices.len()));
        out
    }

    /// Build a new mesh containing only the verts/triangles of the
    /// largest connected component. Useful for keeping the main head
    /// while dropping eye-island sub-meshes from NHL24 exports.
    pub fn keep_largest_component(&self) -> ObjMesh {
        let comps = self.connected_components();
        let Some(biggest) = comps.into_iter().next() else {
            return self.clone();
        };
        self.subset(&biggest.vert_indices, &biggest.triangles)
    }

    /// Distinct group names present in `triangle_groups`, in order of
    /// first appearance.
    pub fn distinct_groups(&self) -> Vec<String> {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut out: Vec<String> = Vec::new();
        for g in &self.triangle_groups {
            if seen.insert(g.clone()) {
                out.push(g.clone());
            }
        }
        out
    }

    /// Build a new mesh containing only triangles whose group name
    /// satisfies the predicate. Triangles with no group (legacy
    /// OBJs without `g` markers) are kept iff `predicate("")` is
    /// true. Verts that become unreferenced are pruned and indices
    /// renumbered.
    pub fn filter_by_group<F: Fn(&str) -> bool>(&self, predicate: F) -> ObjMesh {
        if self.triangle_groups.is_empty() {
            // No group info — predicate decides whether we keep all
            // or drop all.
            if predicate("") {
                return self.clone();
            } else {
                return ObjMesh::default();
            }
        }
        let mut kept_tris: Vec<[u32; 3]> = Vec::new();
        let mut kept_groups: Vec<String> = Vec::new();
        for (t, g) in self.triangles.iter().zip(self.triangle_groups.iter()) {
            if predicate(g) {
                kept_tris.push(*t);
                kept_groups.push(g.clone());
            }
        }
        // Find unique vertex indices referenced by the kept triangles.
        let mut keep_verts: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        for t in &kept_tris {
            keep_verts.insert(t[0]);
            keep_verts.insert(t[1]);
            keep_verts.insert(t[2]);
        }
        let keep_verts_vec: Vec<u32> = keep_verts.into_iter().collect();
        let mut subset = self.subset(&keep_verts_vec, &kept_tris);
        subset.triangle_groups = kept_groups;
        subset
    }

    /// Build a subset mesh from a chosen vertex set + triangle set.
    /// Re-numbers triangle indices to the new (compact) vertex order.
    pub fn subset(&self, keep_verts: &[u32], keep_tris: &[[u32; 3]]) -> ObjMesh {
        let mut remap: std::collections::HashMap<u32, u32> =
            std::collections::HashMap::with_capacity(keep_verts.len());
        let mut positions = Vec::with_capacity(keep_verts.len());
        let mut normals = Vec::with_capacity(keep_verts.len());
        let mut uvs = Vec::with_capacity(keep_verts.len());
        for (new_idx, &old) in keep_verts.iter().enumerate() {
            remap.insert(old, new_idx as u32);
            positions.push(self.positions[old as usize]);
            if let Some(n) = self.normals.get(old as usize) {
                normals.push(*n);
            }
            if let Some(u) = self.uvs.get(old as usize) {
                uvs.push(*u);
            }
        }
        let triangles: Vec<[u32; 3]> = keep_tris
            .iter()
            .filter_map(|t| Some([*remap.get(&t[0])?, *remap.get(&t[1])?, *remap.get(&t[2])?]))
            .collect();
        ObjMesh {
            positions,
            normals,
            uvs,
            triangles,
            triangle_groups: Vec::new(),
        }
    }

    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut it = self.positions.iter();
        let first = *it.next()?;
        let mut min = first;
        let mut max = first;
        for &p in it {
            for c in 0..3 {
                if p[c] < min[c] {
                    min[c] = p[c];
                }
                if p[c] > max[c] {
                    max[c] = p[c];
                }
            }
        }
        Some((min, max))
    }

    /// Smooth per-vertex normals (face-normal-weighted average,
    /// normalized) computed from `positions` + `triangles`. Used as a
    /// fallback for OBJs authored without `vn` lines so downstream
    /// renderers always have a well-formed normal attribute. Mirrors
    /// `tdb_rx2::Mesh::compute_smooth_normals`.
    pub fn smooth_normals(&self) -> Vec<[f32; 3]> {
        let mut acc = vec![[0f64; 3]; self.positions.len()];
        for tri in &self.triangles {
            let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
            let (v0, v1, v2) = (self.positions[a], self.positions[b], self.positions[c]);
            let e1 = [
                (v1[0] - v0[0]) as f64,
                (v1[1] - v0[1]) as f64,
                (v1[2] - v0[2]) as f64,
            ];
            let e2 = [
                (v2[0] - v0[0]) as f64,
                (v2[1] - v0[1]) as f64,
                (v2[2] - v0[2]) as f64,
            ];
            let n = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            for &i in &[a, b, c] {
                acc[i][0] += n[0];
                acc[i][1] += n[1];
                acc[i][2] += n[2];
            }
        }
        acc.into_iter()
            .map(|n| {
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                let inv = if len > 0.0 { 1.0 / len } else { 1.0 };
                [
                    (n[0] * inv) as f32,
                    (n[1] * inv) as f32,
                    (n[2] * inv) as f32,
                ]
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_triangle() {
        let text = "\
# comment
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
";
        let m = parse_obj(text).unwrap();
        assert_eq!(m.positions.len(), 3);
        assert_eq!(m.triangles, vec![[0, 1, 2]]);
    }

    #[test]
    fn parses_aligned_pos_uv_normal_triples() {
        let text = "\
v 0 0 0
v 1 0 0
v 0 1 0
vn 0 0 1
vn 0 0 1
vn 0 0 1
vt 0 0
vt 1 0
vt 0 1
f 1/1/1 2/2/2 3/3/3
";
        let m = parse_obj(text).unwrap();
        assert_eq!(m.normals.len(), 3);
        assert_eq!(m.uvs.len(), 3);
        assert_eq!(m.triangles, vec![[0, 1, 2]]);
    }

    #[test]
    fn expands_misaligned_face_indices_into_unique_verts() {
        // Same position used by two faces but with different UVs —
        // Blender emits this after face deletions compact positions
        // without compacting UVs. Each unique (pos, uv) triple
        // becomes its own logical vertex so downstream arrays stay
        // parallel and UV atlas locations are preserved.
        let text = "\
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
vt 0.5 0.5
f 1/1 2/2 3/3
f 1/4 2/2 3/3
";
        let m = parse_obj(text).unwrap();
        // Six distinct face corners but verts (2/2) and (3/3) are
        // reused across both faces — so 4 unique (pos, uv) triples:
        // (1,1), (2,2), (3,3), (1,4).
        assert_eq!(m.positions.len(), 4);
        assert_eq!(m.uvs.len(), 4);
        assert_eq!(m.triangles.len(), 2);
        // Second face's first corner shares position with first
        // face's first corner but has a different UV → a different
        // logical vertex.
        assert_ne!(m.triangles[0][0], m.triangles[1][0]);
        assert_eq!(m.positions[m.triangles[0][0] as usize], [0.0, 0.0, 0.0]);
        assert_eq!(m.positions[m.triangles[1][0] as usize], [0.0, 0.0, 0.0]);
        assert_eq!(m.uvs[m.triangles[0][0] as usize], [0.0, 0.0]);
        assert_eq!(m.uvs[m.triangles[1][0] as usize], [0.5, 0.5]);
    }

    #[test]
    fn connected_components_splits_two_disjoint_triangles() {
        // Two triangles, no shared verts → two components.
        let text = "\
v 0 0 0
v 1 0 0
v 0 1 0
v 10 0 0
v 11 0 0
v 10 1 0
f 1 2 3
f 4 5 6
";
        let m = parse_obj(text).unwrap();
        let comps = m.connected_components();
        assert_eq!(comps.len(), 2);
        // Both components are 3 verts.
        assert_eq!(comps[0].vert_indices.len(), 3);
        assert_eq!(comps[1].vert_indices.len(), 3);
    }

    #[test]
    fn keep_largest_component_picks_the_bigger_island() {
        // A 5-vert quad (split into 2 triangles sharing edge 2-3)
        // + a separate 3-vert triangle.
        let text = "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
v 0.5 0.5 1
v 10 0 0
v 11 0 0
v 10 1 0
f 1 2 3
f 1 3 4
f 1 3 5
f 6 7 8
";
        let m = parse_obj(text).unwrap();
        let pruned = m.keep_largest_component();
        // First 5 verts are one component (5 verts, 3 tris); last 3
        // are another component (3 verts, 1 tri). We keep the larger.
        assert_eq!(pruned.positions.len(), 5);
        assert_eq!(pruned.triangles.len(), 3);
    }

    #[test]
    fn write_obj_round_trips_through_parser() {
        let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let uvs = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        let normals = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        let triangles = vec![[0u32, 1, 2]];
        let text = write_obj(&positions, &uvs, &normals, &triangles, Some("head"));
        let m = parse_obj(&text).unwrap();
        assert_eq!(m.positions, positions);
        assert_eq!(m.uvs, uvs);
        assert_eq!(m.normals, normals);
        assert_eq!(m.triangles, triangles);
    }

    #[test]
    fn write_obj_positions_only_when_no_uv_normal() {
        let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let triangles = vec![[0u32, 1, 2]];
        let text = write_obj(&positions, &[], &[], &triangles, None);
        assert!(!text.contains("vt "));
        assert!(!text.contains("vn "));
        assert!(text.contains("f 1 2 3"));
        let m = parse_obj(&text).unwrap();
        assert_eq!(m.positions, positions);
        assert!(m.uvs.is_empty());
        assert!(m.normals.is_empty());
    }

    #[test]
    fn rejects_quad_face() {
        let text = "\
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
f 1 2 3 4
";
        let err = parse_obj(text).unwrap_err();
        assert!(matches!(err, ObjError::NonTriangleFace { verts: 4, .. }));
    }
}
