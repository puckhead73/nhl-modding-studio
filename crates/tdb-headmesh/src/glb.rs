//! Minimal glTF-binary (GLB) reader — just enough for head-component ingest.
//!
//! Reads the NHL24 Frosty-exported head/hair component GLBs (and any similar skinned
//! glTF 2.0 binary): per-primitive POSITION / TEXCOORD_0 / JOINTS_0 / WEIGHTS_0 +
//! indices, with the primitive named by its material (the `mat_head` / `mat_eyes` /
//! `mat_eyelashes` convention). Deliberately not a general glTF implementation: no
//! sparse accessors, no interleaved fallbacks beyond byteStride, no extensions, single
//! binary buffer only. UVs are returned in the file's native glTF convention
//! ([`crate::ir::UvConvention::GlSourced`]).

use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum GlbError {
    #[error("not a GLB file (bad magic)")]
    BadMagic,
    #[error("unsupported GLB container version {0}")]
    Version(u32),
    #[error("truncated GLB: {0}")]
    Truncated(&'static str),
    #[error("GLB JSON chunk is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported glTF construct: {0}")]
    Unsupported(String),
    #[error("malformed glTF: {0}")]
    Malformed(String),
}

/// One primitive, expanded to plain arrays.
#[derive(Debug, Clone)]
pub struct GlbPrimitive {
    /// Material name when present, else mesh name, else `primitive_<n>`.
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// Per-vertex joint indices into the glTF skin's joint list; empty when unskinned.
    pub joints: Vec<[u16; 4]>,
    /// Per-vertex weights, parallel to `joints`; empty when unskinned.
    pub weights: Vec<[f32; 4]>,
    pub triangles: Vec<[u32; 3]>,
}

#[derive(Debug, Clone)]
pub struct GlbMesh {
    pub primitives: Vec<GlbPrimitive>,
    /// Node names of the first skin's joints, in glTF joint order (what `joints`
    /// indices refer to). Empty when the file has no skin.
    pub joint_names: Vec<String>,
    /// Per-joint **skinning matrix at bind time**: `globalJointTransform × inverseBindMatrix`,
    /// row-major. This is the matrix the renderer applies to a vertex fully weighted to
    /// that joint while the rig sits in its authored pose.
    ///
    /// It is identity for every joint when the exporter already baked the scene
    /// transform into the vertex data (Blender's CC5 head exports do — the armature's
    /// 0.01 scale is cancelled by the 100× inverse-bind). When it is *not* identity the
    /// raw POSITION stream is in some other space and must be skinned before use; see
    /// [`crate::extract::bake_bind_pose`]. Empty when the file has no skin.
    pub joint_bind_matrices: Vec<[[f32; 4]; 4]>,
    /// Each joint's origin in model space (the translation column of its global
    /// transform). Distinct origins are what make a rig *articulated*; a stub rig whose
    /// joints all sit on one point carries no retargetable information.
    pub joint_model_positions: Vec<[f32; 3]>,
}

/// Row-major 4×4 multiply.
fn mat_mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0f32; 4]; 4];
    for (r, row) in out.iter_mut().enumerate() {
        for (c, cell) in row.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[r][k] * b[k][c]).sum();
        }
    }
    out
}

const IDENTITY4: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// A node's local transform: either an explicit `matrix` (glTF stores column-major) or
/// the TRS triple, returned row-major.
fn node_local_matrix(node: &Value) -> [[f32; 4]; 4] {
    if let Some(m) = node.get("matrix").and_then(Value::as_array) {
        if m.len() == 16 {
            let v: Vec<f32> = m.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect();
            // glTF matrices are column-major: element (row r, col c) is v[c * 4 + r].
            let mut out = [[0f32; 4]; 4];
            for (r, row) in out.iter_mut().enumerate() {
                for (c, cell) in row.iter_mut().enumerate() {
                    *cell = v[c * 4 + r];
                }
            }
            return out;
        }
    }
    let f3 = |key: &str, dflt: [f32; 3]| -> [f32; 3] {
        node.get(key)
            .and_then(Value::as_array)
            .filter(|a| a.len() == 3)
            .map(|a| {
                let mut o = [0f32; 3];
                for (i, s) in o.iter_mut().enumerate() {
                    *s = a[i].as_f64().unwrap_or(0.0) as f32;
                }
                o
            })
            .unwrap_or(dflt)
    };
    let t = f3("translation", [0.0; 3]);
    let s = f3("scale", [1.0; 3]);
    let q = node
        .get("rotation")
        .and_then(Value::as_array)
        .filter(|a| a.len() == 4)
        .map(|a| {
            let mut o = [0f32; 4];
            for (i, slot) in o.iter_mut().enumerate() {
                *slot = a[i].as_f64().unwrap_or(0.0) as f32;
            }
            o
        })
        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let [x, y, z, w] = q;
    let rot = [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - z * w),
            2.0 * (x * z + y * w),
        ],
        [
            2.0 * (x * y + z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - x * w),
        ],
        [
            2.0 * (x * z - y * w),
            2.0 * (y * z + x * w),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ];
    let mut out = IDENTITY4;
    for r in 0..3 {
        for c in 0..3 {
            out[r][c] = rot[r][c] * s[c];
        }
        out[r][3] = t[r];
    }
    out
}

pub fn parse_glb(data: &[u8]) -> Result<GlbMesh, GlbError> {
    if data.len() < 12 {
        return Err(GlbError::Truncated("header"));
    }
    if &data[0..4] != b"glTF" {
        return Err(GlbError::BadMagic);
    }
    let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if version != 2 {
        return Err(GlbError::Version(version));
    }
    // Chunk walk: chunk 0 must be JSON; the BIN chunk (if any) follows.
    let mut offset = 12usize;
    let mut json: Option<&[u8]> = None;
    let mut bin: &[u8] = &[];
    while offset + 8 <= data.len() {
        let len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = &data[offset + 4..offset + 8];
        let body = data
            .get(offset + 8..offset + 8 + len)
            .ok_or(GlbError::Truncated("chunk body"))?;
        match kind {
            b"JSON" => json = Some(body),
            b"BIN\0" => bin = body,
            _ => {}
        }
        offset += 8 + len;
    }
    let json: Value = serde_json::from_slice(json.ok_or(GlbError::Truncated("JSON chunk"))?)?;
    let doc = Doc { json: &json, bin };

    let meshes = doc.array("meshes");
    let mut primitives = Vec::new();
    for (mi, mesh) in meshes.iter().enumerate() {
        let mesh_name = mesh.get("name").and_then(Value::as_str);
        for (pi, prim) in mesh
            .get("primitives")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .iter()
            .enumerate()
        {
            // mode 4 (TRIANGLES) is the default; anything else is out of scope.
            if prim.get("mode").and_then(Value::as_u64).unwrap_or(4) != 4 {
                return Err(GlbError::Unsupported("non-triangle primitive mode".into()));
            }
            let attrs = prim
                .get("attributes")
                .ok_or_else(|| GlbError::Malformed("primitive without attributes".into()))?;
            let acc = |name: &str| attrs.get(name).and_then(Value::as_u64).map(|i| i as usize);

            let positions = match acc("POSITION") {
                Some(a) => doc.read_vec3_f32(a)?,
                None => return Err(GlbError::Malformed("primitive without POSITION".into())),
            };
            let uvs = match acc("TEXCOORD_0") {
                Some(a) => doc.read_vec2_f32(a)?,
                None => Vec::new(),
            };
            let joints = match acc("JOINTS_0") {
                Some(a) => doc.read_vec4_u16(a)?,
                None => Vec::new(),
            };
            let weights = match acc("WEIGHTS_0") {
                Some(a) => doc.read_vec4_f32(a)?,
                None => Vec::new(),
            };
            let triangles = match prim.get("indices").and_then(Value::as_u64) {
                Some(a) => {
                    let flat = doc.read_scalar_u32(a as usize)?;
                    if flat.len() % 3 != 0 {
                        return Err(GlbError::Malformed(
                            "index count not a multiple of 3".into(),
                        ));
                    }
                    flat.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect()
                }
                None => (0..positions.len() as u32 / 3)
                    .map(|t| [t * 3, t * 3 + 1, t * 3 + 2])
                    .collect(),
            };
            let name = prim
                .get("material")
                .and_then(Value::as_u64)
                .and_then(|mat| doc.array("materials").get(mat as usize))
                .and_then(|m| m.get("name"))
                .and_then(Value::as_str)
                .or(mesh_name)
                .map(str::to_owned)
                .unwrap_or_else(|| format!("primitive_{mi}_{pi}"));
            primitives.push(GlbPrimitive {
                name,
                positions,
                uvs,
                joints,
                weights,
                triangles,
            });
        }
    }

    // Joint names + bind-time skinning matrices from the first skin (NHL24 and CC5
    // components both carry exactly one).
    let mut joint_names = Vec::new();
    let mut joint_bind_matrices = Vec::new();
    let mut joint_model_positions = Vec::new();
    if let Some(skin) = doc.array("skins").first() {
        let nodes = doc.array("nodes");
        // child → parent, so a joint's global transform can be walked up the scene graph.
        let mut parent_of = vec![usize::MAX; nodes.len()];
        for (i, node) in nodes.iter().enumerate() {
            for c in node
                .get("children")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[])
            {
                if let Some(ci) = c.as_u64().map(|v| v as usize) {
                    if ci < parent_of.len() {
                        parent_of[ci] = i;
                    }
                }
            }
        }
        let ibms = match skin.get("inverseBindMatrices").and_then(Value::as_u64) {
            Some(a) => doc.read_mat4_f32(a as usize)?,
            None => Vec::new(),
        };
        let joints = skin
            .get("joints")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        for (ji, j) in joints.iter().enumerate() {
            let idx = j
                .as_u64()
                .map(|v| v as usize)
                .filter(|i| *i < nodes.len())
                .ok_or_else(|| GlbError::Malformed("skin joint references missing node".into()))?;
            joint_names.push(
                nodes[idx]
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            );
            // Global transform = parents (root-first) × local.
            let mut global = node_local_matrix(&nodes[idx]);
            let mut walk = parent_of[idx];
            let mut guard = 0usize;
            while walk != usize::MAX && guard <= nodes.len() {
                global = mat_mul(&node_local_matrix(&nodes[walk]), &global);
                walk = parent_of[walk];
                guard += 1;
            }
            joint_model_positions.push([global[0][3], global[1][3], global[2][3]]);
            let ibm = ibms.get(ji).copied().unwrap_or(IDENTITY4);
            joint_bind_matrices.push(mat_mul(&global, &ibm));
        }
    }

    Ok(GlbMesh {
        primitives,
        joint_names,
        joint_bind_matrices,
        joint_model_positions,
    })
}

// ---------- accessor plumbing ----------

const COMP_U8: u64 = 5121;
const COMP_I16: u64 = 5122;
const COMP_U16: u64 = 5123;
const COMP_U32: u64 = 5125;
const COMP_F32: u64 = 5126;

struct Doc<'a> {
    json: &'a Value,
    bin: &'a [u8],
}

/// A resolved accessor: raw bytes + layout.
struct Accessor<'a> {
    data: &'a [u8],
    count: usize,
    component: u64,
    components: usize,
    stride: usize,
    normalized: bool,
}

impl<'a> Doc<'a> {
    fn array(&self, key: &str) -> &'a [Value] {
        self.json
            .get(key)
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn accessor(&self, index: usize, expect_type: &str) -> Result<Accessor<'a>, GlbError> {
        let acc = self
            .array("accessors")
            .get(index)
            .ok_or_else(|| GlbError::Malformed(format!("missing accessor {index}")))?;
        if acc.get("sparse").is_some() {
            return Err(GlbError::Unsupported("sparse accessor".into()));
        }
        let ty = acc.get("type").and_then(Value::as_str).unwrap_or("");
        if ty != expect_type {
            return Err(GlbError::Malformed(format!(
                "accessor {index} is {ty}, expected {expect_type}"
            )));
        }
        let components = match ty {
            "SCALAR" => 1,
            "VEC2" => 2,
            "VEC3" => 3,
            "VEC4" => 4,
            "MAT4" => 16,
            other => return Err(GlbError::Unsupported(format!("accessor type {other}"))),
        };
        let component = acc
            .get("componentType")
            .and_then(Value::as_u64)
            .ok_or_else(|| GlbError::Malformed("accessor without componentType".into()))?;
        let count = acc.get("count").and_then(Value::as_u64).unwrap_or(0) as usize;
        let acc_offset = acc.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let view_index = acc
            .get("bufferView")
            .and_then(Value::as_u64)
            .ok_or_else(|| GlbError::Unsupported("accessor without bufferView".into()))?
            as usize;
        let view = self
            .array("bufferViews")
            .get(view_index)
            .ok_or_else(|| GlbError::Malformed(format!("missing bufferView {view_index}")))?;
        if view.get("buffer").and_then(Value::as_u64) != Some(0) {
            return Err(GlbError::Unsupported("non-BIN buffer".into()));
        }
        let view_offset = view.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let view_len = view.get("byteLength").and_then(Value::as_u64).unwrap_or(0) as usize;
        let comp_size = match component {
            COMP_U8 => 1,
            COMP_I16 | COMP_U16 => 2,
            COMP_U32 | COMP_F32 => 4,
            other => return Err(GlbError::Unsupported(format!("componentType {other}"))),
        };
        let elem_size = comp_size * components;
        let stride = view
            .get("byteStride")
            .and_then(Value::as_u64)
            .map(|s| s as usize)
            .unwrap_or(elem_size);
        let start = view_offset + acc_offset;
        let need = if count == 0 {
            0
        } else {
            (count - 1) * stride + elem_size
        };
        let data = self
            .bin
            .get(start..view_offset + view_len)
            .filter(|d| d.len() >= need)
            .ok_or(GlbError::Truncated("accessor data"))?;
        Ok(Accessor {
            data,
            count,
            component,
            components,
            stride,
            normalized: acc
                .get("normalized")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }

    fn read_vec3_f32(&self, index: usize) -> Result<Vec<[f32; 3]>, GlbError> {
        let a = self.accessor(index, "VEC3")?;
        read_f32s(&a).map(|v| v.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
    }

    fn read_vec2_f32(&self, index: usize) -> Result<Vec<[f32; 2]>, GlbError> {
        let a = self.accessor(index, "VEC2")?;
        read_f32s(&a).map(|v| v.chunks_exact(2).map(|c| [c[0], c[1]]).collect())
    }

    fn read_vec4_f32(&self, index: usize) -> Result<Vec<[f32; 4]>, GlbError> {
        let a = self.accessor(index, "VEC4")?;
        read_f32s(&a).map(|v| {
            v.chunks_exact(4)
                .map(|c| [c[0], c[1], c[2], c[3]])
                .collect()
        })
    }

    fn read_vec4_u16(&self, index: usize) -> Result<Vec<[u16; 4]>, GlbError> {
        let a = self.accessor(index, "VEC4")?;
        let mut out = Vec::with_capacity(a.count);
        for i in 0..a.count {
            let e = &a.data[i * a.stride..];
            let mut v = [0u16; 4];
            for (c, slot) in v.iter_mut().enumerate() {
                *slot = match a.component {
                    COMP_U8 => u16::from(e[c]),
                    COMP_U16 => u16::from_le_bytes(e[c * 2..c * 2 + 2].try_into().unwrap()),
                    other => {
                        return Err(GlbError::Unsupported(format!(
                            "JOINTS componentType {other}"
                        )))
                    }
                };
            }
            out.push(v);
        }
        Ok(out)
    }

    /// MAT4 accessor → row-major matrices (glTF stores them column-major).
    fn read_mat4_f32(&self, index: usize) -> Result<Vec<[[f32; 4]; 4]>, GlbError> {
        let a = self.accessor(index, "MAT4")?;
        let flat = read_f32s(&a)?;
        Ok(flat
            .chunks_exact(16)
            .map(|m| {
                let mut out = [[0f32; 4]; 4];
                for (r, row) in out.iter_mut().enumerate() {
                    for (c, cell) in row.iter_mut().enumerate() {
                        *cell = m[c * 4 + r];
                    }
                }
                out
            })
            .collect())
    }

    fn read_scalar_u32(&self, index: usize) -> Result<Vec<u32>, GlbError> {
        let a = self.accessor(index, "SCALAR")?;
        let mut out = Vec::with_capacity(a.count);
        for i in 0..a.count {
            let e = &a.data[i * a.stride..];
            out.push(match a.component {
                COMP_U8 => u32::from(e[0]),
                COMP_U16 => u32::from(u16::from_le_bytes(e[0..2].try_into().unwrap())),
                COMP_U32 => u32::from_le_bytes(e[0..4].try_into().unwrap()),
                other => {
                    return Err(GlbError::Unsupported(format!(
                        "index componentType {other}"
                    )))
                }
            });
        }
        Ok(out)
    }
}

/// Flattened float components, denormalizing u8/u16-normalized storage.
fn read_f32s(a: &Accessor<'_>) -> Result<Vec<f32>, GlbError> {
    let mut out = Vec::with_capacity(a.count * a.components);
    for i in 0..a.count {
        let e = &a.data[i * a.stride..];
        for c in 0..a.components {
            out.push(match a.component {
                COMP_F32 => f32::from_le_bytes(e[c * 4..c * 4 + 4].try_into().unwrap()),
                COMP_U8 if a.normalized => f32::from(e[c]) / 255.0,
                COMP_U16 if a.normalized => {
                    f32::from(u16::from_le_bytes(e[c * 2..c * 2 + 2].try_into().unwrap())) / 65535.0
                }
                COMP_I16 if a.normalized => {
                    let raw = i16::from_le_bytes(e[c * 2..c * 2 + 2].try_into().unwrap());
                    (f32::from(raw) / 32767.0).max(-1.0)
                }
                other => {
                    return Err(GlbError::Unsupported(format!(
                        "float attribute componentType {other} (normalized={})",
                        a.normalized
                    )))
                }
            });
        }
    }
    Ok(out)
}

/// Named morph-target POSITION deltas for one primitive, selected by material name.
///
/// CC5 exports the full ExPlus expression set on `Std_Skin_Head` — 349 targets on the
/// reference character. Two of them, `Eye_Blink_L` / `Eye_Blink_R`, displace exactly the
/// vertices of the corresponding upper eyelid, which is the authored answer to a question
/// geometry can only approximate: *which vertices are the eyelid, and how much does each
/// participate?* The normalized delta magnitude is directly usable as a lid weight.
///
/// Only the requested targets are decoded. The reference export is 78 MB and decoding all
/// 349 would be pure waste when six are wanted.
pub fn read_morph_targets(
    data: &[u8],
    material_name: &str,
    wanted: &[&str],
) -> Result<std::collections::BTreeMap<String, Vec<[f32; 3]>>, GlbError> {
    let (json, bin) = split_chunks(data)?;
    let json: Value = serde_json::from_slice(json)?;
    let doc = Doc { json: &json, bin };
    let mut out = std::collections::BTreeMap::new();

    for mesh in doc.array("meshes") {
        // `targetNames` lives on the mesh, `targets` on each primitive, index-aligned.
        let names: Vec<&str> = mesh
            .get("extras")
            .and_then(|e| e.get("targetNames"))
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if names.is_empty() {
            continue;
        }
        for prim in mesh
            .get("primitives")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[])
        {
            let mat_name = prim
                .get("material")
                .and_then(Value::as_u64)
                .and_then(|m| doc.array("materials").get(m as usize))
                .and_then(|m| m.get("name"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            if mat_name != material_name {
                continue;
            }
            // Dense length = the primitive's own vertex count, so sparse targets expand
            // to one delta per vertex and stay index-aligned with the geometry.
            let base_count = prim
                .get("attributes")
                .and_then(|a| a.get("POSITION"))
                .and_then(Value::as_u64)
                .and_then(|i| doc.array("accessors").get(i as usize).cloned())
                .and_then(|a| a.get("count").and_then(Value::as_u64))
                .unwrap_or(0) as usize;
            let targets = prim
                .get("targets")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            for want in wanted {
                let Some(ti) = names.iter().position(|n| n == want) else {
                    continue;
                };
                let Some(acc) = targets.get(ti).and_then(|t| t.get("POSITION")) else {
                    continue;
                };
                let Some(acc) = acc.as_u64() else { continue };
                out.insert(
                    (*want).to_string(),
                    doc.read_vec3_f32_sparse(acc as usize, base_count)?,
                );
            }
            return Ok(out);
        }
    }
    Ok(out)
}

/// Split a GLB into its JSON and BIN chunks.
fn split_chunks(data: &[u8]) -> Result<(&[u8], &[u8]), GlbError> {
    if data.len() < 12 || &data[0..4] != b"glTF" {
        return Err(GlbError::BadMagic);
    }
    let mut offset = 12usize;
    let mut json: Option<&[u8]> = None;
    let mut bin: &[u8] = &[];
    while offset + 8 <= data.len() {
        let len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = &data[offset + 4..offset + 8];
        let body = data
            .get(offset + 8..offset + 8 + len)
            .ok_or(GlbError::Truncated("chunk body"))?;
        match kind {
            b"JSON" => json = Some(body),
            b"BIN\0" => bin = body,
            _ => {}
        }
        offset += 8 + len;
    }
    Ok((json.ok_or(GlbError::Truncated("JSON chunk"))?, bin))
}

impl Doc<'_> {
    /// VEC3 f32 accessor with **sparse** support, densified to one entry per vertex.
    ///
    /// glTF stores morph targets sparsely: a blink moves a few hundred of a head's four
    /// thousand vertices, so only those are written, as an (indices, values) pair over a
    /// base of zeros. The dense reader rejects sparse accessors outright, which is right
    /// for geometry streams and wrong here — every CC5 expression target is sparse.
    fn read_vec3_f32_sparse(
        &self,
        index: usize,
        dense_len: usize,
    ) -> Result<Vec<[f32; 3]>, GlbError> {
        let acc = self
            .array("accessors")
            .get(index)
            .cloned()
            .ok_or_else(|| GlbError::Malformed(format!("missing accessor {index}")))?;
        let count = acc.get("count").and_then(Value::as_u64).unwrap_or(0) as usize;
        let mut out = vec![[0f32; 3]; dense_len.max(count)];

        // Base values, when the accessor has a buffer view at all.
        if acc.get("bufferView").is_some() {
            // Re-read through the dense path by temporarily treating it as non-sparse:
            // build an Accessor directly rather than going through `accessor()`, which
            // refuses anything carrying a `sparse` key.
            let bv_idx = acc.get("bufferView").and_then(Value::as_u64).unwrap_or(0) as usize;
            let bv = self
                .array("bufferViews")
                .get(bv_idx)
                .cloned()
                .ok_or_else(|| GlbError::Malformed("morph base bufferView".into()))?;
            let start = bv.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize
                + acc.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
            let stride = bv
                .get("byteStride")
                .and_then(Value::as_u64)
                .map(|s| s as usize)
                .unwrap_or(12);
            for (i, vertex) in out.iter_mut().enumerate().take(count) {
                let o = start + i * stride;
                if o + 12 > self.bin.len() {
                    break;
                }
                for (c, axis) in vertex.iter_mut().enumerate() {
                    *axis =
                        f32::from_le_bytes(self.bin[o + c * 4..o + c * 4 + 4].try_into().unwrap());
                }
            }
        }

        let Some(sparse) = acc.get("sparse") else {
            out.truncate(dense_len.max(count));
            return Ok(out);
        };
        let n = sparse.get("count").and_then(Value::as_u64).unwrap_or(0) as usize;
        let idx_desc = sparse
            .get("indices")
            .ok_or_else(|| GlbError::Malformed("sparse without indices".into()))?;
        let val_desc = sparse
            .get("values")
            .ok_or_else(|| GlbError::Malformed("sparse without values".into()))?;

        let slice_of = |desc: &Value| -> Result<usize, GlbError> {
            let bv_idx = desc.get("bufferView").and_then(Value::as_u64).unwrap_or(0) as usize;
            let bv = self
                .array("bufferViews")
                .get(bv_idx)
                .cloned()
                .ok_or_else(|| GlbError::Malformed("sparse bufferView".into()))?;
            Ok(
                bv.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize
                    + desc.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize,
            )
        };
        let idx_off = slice_of(idx_desc)?;
        let val_off = slice_of(val_desc)?;
        let comp = idx_desc
            .get("componentType")
            .and_then(Value::as_u64)
            .unwrap_or(COMP_U16);
        let isz = match comp {
            COMP_U8 => 1,
            COMP_U16 => 2,
            COMP_U32 => 4,
            other => {
                return Err(GlbError::Unsupported(format!(
                    "sparse index componentType {other}"
                )))
            }
        };
        for k in 0..n {
            let io = idx_off + k * isz;
            if io + isz > self.bin.len() {
                break;
            }
            let vi = match isz {
                1 => usize::from(self.bin[io]),
                2 => usize::from(u16::from_le_bytes(self.bin[io..io + 2].try_into().unwrap())),
                _ => u32::from_le_bytes(self.bin[io..io + 4].try_into().unwrap()) as usize,
            };
            let vo = val_off + k * 12;
            if vo + 12 > self.bin.len() || vi >= out.len() {
                continue;
            }
            for (c, axis) in out[vi].iter_mut().enumerate() {
                *axis =
                    f32::from_le_bytes(self.bin[vo + c * 4..vo + c * 4 + 4].try_into().unwrap());
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tiny one-triangle skinned GLB in memory.
    fn tiny_glb() -> Vec<u8> {
        let mut bin: Vec<u8> = Vec::new();
        // POSITION: 3 × vec3 f32
        for p in [[0.0f32, 1.7, 0.1], [0.1, 1.8, 0.1], [-0.1, 1.8, 0.1]] {
            for c in p {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        let uv_off = bin.len();
        for uv in [[0.0f32, 0.0], [1.0, 0.0], [0.5, 1.0]] {
            for c in uv {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        let joints_off = bin.len();
        for j in [[0u8, 1, 0, 0]; 3] {
            bin.extend_from_slice(&j);
        }
        let weights_off = bin.len();
        for w in [[0.75f32, 0.25, 0.0, 0.0]; 3] {
            for c in w {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        let idx_off = bin.len();
        for i in [0u16, 1, 2] {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }

        let json = serde_json::json!({
            "asset": {"version": "2.0"},
            "materials": [{"name": "mat_head"}],
            "meshes": [{
                "name": "head_mesh",
                "primitives": [{
                    "attributes": {"POSITION": 0, "TEXCOORD_0": 1, "JOINTS_0": 2, "WEIGHTS_0": 3},
                    "indices": 4,
                    "material": 0
                }]
            }],
            "nodes": [{"name": "Head"}, {"name": "Jaw"}],
            "skins": [{"joints": [0, 1]}],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
                {"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2"},
                {"bufferView": 2, "componentType": 5121, "count": 3, "type": "VEC4"},
                {"bufferView": 3, "componentType": 5126, "count": 3, "type": "VEC4"},
                {"bufferView": 4, "componentType": 5123, "count": 3, "type": "SCALAR"}
            ],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": uv_off},
                {"buffer": 0, "byteOffset": uv_off, "byteLength": joints_off - uv_off},
                {"buffer": 0, "byteOffset": joints_off, "byteLength": weights_off - joints_off},
                {"buffer": 0, "byteOffset": weights_off, "byteLength": idx_off - weights_off},
                {"buffer": 0, "byteOffset": idx_off, "byteLength": 6}
            ],
            "buffers": [{"byteLength": bin.len()}]
        });
        let mut json_bytes = serde_json::to_vec(&json).unwrap();
        while !json_bytes.len().is_multiple_of(4) {
            json_bytes.push(b' ');
        }

        let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
        let mut glb = Vec::with_capacity(total);
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total as u32).to_le_bytes());
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"JSON");
        glb.extend_from_slice(&json_bytes);
        glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"BIN\0");
        glb.extend_from_slice(&bin);
        glb
    }

    #[test]
    fn parses_skinned_primitive_named_by_material() {
        let mesh = parse_glb(&tiny_glb()).unwrap();
        assert_eq!(mesh.primitives.len(), 1);
        let p = &mesh.primitives[0];
        assert_eq!(p.name, "mat_head");
        assert_eq!(p.positions.len(), 3);
        assert_eq!(p.uvs[2], [0.5, 1.0]);
        assert_eq!(p.joints[0], [0, 1, 0, 0]);
        assert!((p.weights[0][0] - 0.75).abs() < 1e-6);
        assert_eq!(p.triangles, vec![[0, 1, 2]]);
        assert_eq!(
            mesh.joint_names,
            vec!["Head".to_string(), "Jaw".to_string()]
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(parse_glb(b"nope"), Err(GlbError::Truncated(_))));
        assert!(matches!(
            parse_glb(b"XXXX\x02\x00\x00\x00\x0c\x00\x00\x00"),
            Err(GlbError::BadMagic)
        ));
    }
}
