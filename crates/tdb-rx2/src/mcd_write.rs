//! Bounded, fail-closed **editor** for the PS4 `.mcd` (RSF) player-mesh container.
//!
//! [`crate::mcd`] is a *display* decoder: it locates geometry by scanning for chunk
//! tags and binds index buffers by trying each one until the indices happen to fall in
//! range. That is fine for preview and unsafe for writing — on the real NHL 19
//! `jack_eichel.mcd` it gives the left eye a 56-triangle buffer belonging to an
//! entirely different structure, when the container itself declares 100.
//!
//! This module reads the container's own declarations, validates them, and refuses
//! anything it cannot account for. It is deliberately not a general RSF serializer.
//!
//! # The model
//!
//! [`McdContainer::parse`] tiles the file into an ordered, gapless, non-overlapping
//! sequence of [`Region`]s covering every byte of `[0, len)`. Two region kinds are
//! typed — vertex-stream payloads and tail bounding boxes — and everything else is
//! [`Region::Opaque`]. [`McdContainer::serialize`] walks that tiling and re-emits each
//! region, so a no-op serialize reproducing the input byte-for-byte is evidence that
//! the tiling is exhaustive and that the typed regions round-trip.
//!
//! **What the no-op proof does and does not cover.** Vertex payloads are held as raw
//! bytes *plus* a decoded `f32` position vector; serialize re-emits the raw bytes and
//! then overwrites the position field from the decoded vector. So the proof covers
//! region tiling, chunk framing, `count × stride` arithmetic and the position
//! round-trip. It deliberately does **not** decode normals, tangents, UVs or blend
//! data: those ride along verbatim, which is what the edit contract requires anyway
//! (their `f16`/`u8n` encodings could not be re-emitted bit-exactly through `f32`).
//!
//! # What "ownership" here does and does not mean
//!
//! Submesh → format → vertex-stream binding **is** declaration-driven: each `HSEM`
//! record names a triangle count, a format index and a stream count; the format indexes
//! the `TMFV` array, whose attribute tokens are `name:byteOffset(hex):streamIndex:…`;
//! and the stream counts partition the `MRTS` chunks in order. All of that is checked
//! against the chunks actually present, including attribute extents against the stride
//! and equal vertex counts across a submesh's split streams.
//!
//! Submesh → **index buffer** binding is *not* declaration-driven, because no record in
//! the container names an index chunk. This module matches by declared element count
//! (`3 × triangles`), validates the selected payload's encoding, extent and every index
//! against the submesh's vertex count, and **refuses ambiguity**: two candidate buffers
//! with the same count is an error, not a coin flip. Count equality alone still cannot
//! prove a shared buffer, so for real work use [`McdContainer::parse_with_profile`],
//! which additionally pins each buffer's verified offset for a fingerprinted donor.
//!
//! Not every `XDNI` chunk is a triangle list. The NHL 19 head container carries a
//! second group of seven index-like chunks whose element counts (132, 2348, 168, 2256,
//! 200, 12640, 200) match no submesh — 2348 is not even divisible by three. They are
//! claimed by nothing, reported in [`McdContainer::unclaimed_index_chunks`], carried as
//! opaque bytes, and their purpose is **unidentified**.
//!
//! # Refusals
//!
//! Parsing fails closed on: a container that is not a single RSF (the extracted
//! `jack_eichel.mcd` in a shadPS4 `Extracted` tree is three concatenated copies),
//! missing or truncated declaration words, overlapping or out-of-order stream payloads,
//! an absent format or stream, an attribute running past its stride, a VFMT naming a
//! stream slot the submesh does not have, split streams with unequal vertex counts,
//! duplicate submesh names, a position that is not `3f32`, a malformed or empty VFMT
//! token, an ambiguous/truncated/out-of-range index buffer, and non-finite replacement
//! coordinates.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::mcd::MAGIC_RSF;

// Reversed (little-endian) FourCC tags, as they appear on disk.
const TAG_VFMT: &[u8; 4] = b"TMFV";
const TAG_STRM: &[u8; 4] = b"MRTS";
const TAG_INDX: &[u8; 4] = b"XDNI";
const TAG_MESH: &[u8; 4] = b"HSEM";
const TAG_GEOM: &[u8; 4] = b"MOEG";

/// STRM/INDX chunks carry a 1-byte pad after the tag, so their `u32` fields sit at odd
/// offsets: version at +5, element count at +9.
const CHUNK_COUNT_OFF: usize = 0x09;
/// A STRM chunk's stride `u32`.
const STRM_STRIDE_OFF: usize = 0x0D;
/// A STRM chunk's interleaved vertex bytes.
const STRM_DATA_OFF: usize = 0x15;
/// An INDX chunk's reversed `ID16`/`ID32` element-width token.
const INDX_TOKEN_OFF: usize = 0x0D;
/// An INDX chunk's element bytes.
const INDX_DATA_OFF: usize = 0x11;

/// Offsets of the `HSEM`/`TSLT` record's declaration words.
const MESH_TRIS_OFF: usize = 20;
const MESH_FORMAT_OFF: usize = 24;
const MESH_STREAMS_OFF: usize = 32;

#[derive(Debug, Error)]
pub enum McdWriteError {
    #[error("input is shorter than the RSF magic ({0} bytes)")]
    TooShort(usize),
    #[error("magic mismatch — expected `RSF\\0`")]
    BadMagic,
    #[error(
        "input holds {count} concatenated RSF containers (extra magic at {offsets:?}); that is \
         the shape of an `Extracted`-tree dump, not a native asset — decode a single archive \
         entry instead"
    )]
    ConcatenatedContainers { count: usize, offsets: Vec<usize> },
    #[error("no MESH (HSEM) submesh records found")]
    NoSubmeshes,
    #[error("{what} at {offset} is missing or truncated")]
    MissingDeclaration { what: &'static str, offset: usize },
    #[error("submesh {sub}: declares format {format} but only {have} VFMT chunks are present")]
    FormatOutOfRange {
        sub: usize,
        format: usize,
        have: usize,
    },
    #[error("submesh {sub}: declares {want} vertex stream(s) but only {have} remain unassigned")]
    StreamsExhausted {
        sub: usize,
        want: usize,
        have: usize,
    },
    #[error(
        "submesh {sub}: stream slot {slot} attribute {attr:?} spans [{lo}, {hi}) but the stride \
         is {stride}"
    )]
    AttributeOutsideStride {
        sub: usize,
        slot: usize,
        attr: String,
        lo: usize,
        hi: usize,
        stride: usize,
    },
    #[error(
        "submesh {sub}: VFMT declares stream slot {slot} but the submesh declares only {slots} \
         slot(s)"
    )]
    StreamSlotOutOfRange {
        sub: usize,
        slot: usize,
        slots: usize,
    },
    #[error(
        "submesh {sub}: split streams disagree on vertex count (slot 0 has {first}, slot {slot} \
         has {other})"
    )]
    SplitStreamCountMismatch {
        sub: usize,
        slot: usize,
        first: usize,
        other: usize,
    },
    #[error("submesh {sub}: no index buffer declares {want} elements")]
    NoIndexBuffer { sub: usize, want: usize },
    #[error(
        "submesh {sub}: {n} index buffers declare {want} elements ({offsets:?}); element count \
         alone cannot decide between them"
    )]
    AmbiguousIndexBuffer {
        sub: usize,
        want: usize,
        n: usize,
        offsets: Vec<usize>,
    },
    #[error("index buffer at {offset}: unsupported element-width token {token:?}")]
    UnsupportedIndexEncoding { offset: usize, token: [u8; 4] },
    #[error(
        "index buffer at {offset}: needs {needed} bytes for {count} × {width}-byte elements, \
         only {have} remain"
    )]
    TruncatedIndexBuffer {
        offset: usize,
        count: usize,
        width: usize,
        needed: usize,
        have: usize,
    },
    #[error(
        "index buffer at {offset}: element {slot} is {value}, out of range for {vertices} vertices"
    )]
    IndexOutOfRange {
        offset: usize,
        slot: usize,
        value: u32,
        vertices: usize,
    },
    #[error("index buffer at {offset}: {count} elements is not a whole number of triangles")]
    IndexCountNotTriangles { offset: usize, count: usize },
    #[error("submesh {sub}: VFMT declares no position (`p0`) attribute")]
    NoPosition { sub: usize },
    #[error(
        "submesh {sub}: position is {count}×{comp} — this editor writes only `3f32` positions"
    )]
    UnsupportedPositionEncoding {
        sub: usize,
        count: usize,
        comp: &'static str,
    },
    #[error("submesh {sub}: declares more than one position attribute in slot {slot}")]
    DuplicatePosition { sub: usize, slot: usize },
    #[error("two submeshes share the name {0:?}")]
    DuplicateSubmeshName(String),
    #[error("chunk at {offset} runs past end of file (needs {needed} bytes, have {have})")]
    Truncated {
        offset: usize,
        needed: usize,
        have: usize,
    },
    #[error("size arithmetic overflowed for the chunk at {offset}")]
    SizeOverflow { offset: usize },
    #[error("stream payloads are not in strictly increasing, non-overlapping order at {offset}")]
    OverlappingStreams { offset: usize },
    #[error("malformed VFMT at {offset}: token {token:?}")]
    BadVfmt { offset: usize, token: String },
    #[error("{0} vertex stream(s) are declared by no submesh")]
    OrphanStreams(usize),
    #[error(
        "region tiling is not exhaustive: {covered} of {total} bytes covered — an internal \
         invariant failure, not a property of the input"
    )]
    TilingIncomplete { covered: usize, total: usize },
    #[error("stream {stream}: expected {want} positions, got {got}")]
    PositionCountMismatch {
        stream: usize,
        want: usize,
        got: usize,
    },
    #[error("stream {stream}: position {index} component {axis} is not finite ({value})")]
    NonFinitePosition {
        stream: usize,
        index: usize,
        axis: usize,
        value: f32,
    },
    #[error("stream {stream} carries no position attribute and cannot be patched")]
    StreamHasNoPositions { stream: usize },
    #[error(
        "donor is not a recognised profile: sha256 {actual}, {len} bytes. This editor's measured \
         layout and protected regions are specific to a fingerprinted donor; a same-count file \
         is not the same mesh."
    )]
    UnknownDonor { actual: String, len: usize },
    #[error(
        "donor matches profile {profile:?} but its structure does not: {detail}. Refusing rather \
         than editing an unverified layout."
    )]
    ProfileMismatch {
        profile: &'static str,
        detail: String,
    },
}

// ------------------------------------------------------------------ donor profiles

/// One submesh as measured on a fingerprinted donor.
#[derive(Debug, Clone, Copy)]
pub struct ProfileSubmesh {
    pub name: &'static str,
    pub triangles: usize,
    pub format: usize,
    /// `(vertex_count, stride)` per declared stream slot, in order.
    pub streams: &'static [(usize, usize)],
    /// The verified `XDNI` chunk offset this submesh draws from.
    pub index_chunk: usize,
    /// Whether that buffer is also claimed by an earlier submesh.
    pub shares_index_buffer: bool,
}

/// A donor whose layout has actually been measured, keyed by content hash.
///
/// The profile exists because a vertex-count match cannot establish vertex *order* or
/// protected-region identity. Editing entry points require one.
#[derive(Debug, Clone, Copy)]
pub struct DonorProfile {
    pub name: &'static str,
    /// Lowercase hex SHA-256 of the whole container.
    pub sha256: &'static str,
    pub len: usize,
    pub submeshes: &'static [ProfileSubmesh],
    /// Submesh a head fit targets.
    pub head_submesh: &'static str,
    /// Measured exterior forehead selection for the positional diagnostic, as
    /// `(y_lo, y_hi, z_min, abs_x_max)` in container units. Chosen clear of eyelids,
    /// lips, mouth interior and the neck seam.
    pub forehead_probe: (f32, f32, f32, f32),
}

/// The 7 submeshes of NHL 19 `jack_eichel.mcd`. Both known revisions share this layout
/// exactly; they differ only inside the `SKTN` joint-name region.
const EICHEL_SUBMESHES: &[ProfileSubmesh] = &[
    ProfileSubmesh {
        name: "geo_cardsAShape",
        triangles: 66,
        format: 0,
        streams: &[(106, 36)],
        index_chunk: 426409,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_cardsBShape",
        triangles: 1174,
        format: 0,
        streams: &[(1597, 36)],
        index_chunk: 426822,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_eyelashesShape",
        triangles: 84,
        format: 1,
        streams: &[(64, 32)],
        index_chunk: 433883,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_haircapShape",
        triangles: 1078,
        format: 1,
        streams: &[(606, 32)],
        index_chunk: 434404,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_reyeShape",
        triangles: 100,
        format: 2,
        streams: &[(66, 12), (66, 24)],
        index_chunk: 440889,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_headShape",
        triangles: 6014,
        format: 5,
        streams: &[(3157, 40)],
        index_chunk: 441506,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_leyeShape",
        triangles: 100,
        format: 2,
        streams: &[(66, 12), (66, 24)],
        // Genuinely the same buffer as `geo_reyeShape`: 66 mirrored vertices, 100
        // triangles, one index list between them.
        index_chunk: 440889,
        shares_index_buffer: true,
    },
];

/// NHL 19 `jack_eichel.mcd` as shipped in `game_90-008.big` #62195 and, byte-identically,
/// `patch_game_93.big` #3036.
pub const NHL19_EICHEL_BASE: DonorProfile = DonorProfile {
    name: "nhl19-jack_eichel-base",
    sha256: "4da8f5d5c9eaa64f3aaa38b7e9e4cb0b0aea74b91a1884ff863bf0cc61243195",
    len: 1_043_168,
    submeshes: EICHEL_SUBMESHES,
    head_submesh: "geo_headShape",
    forehead_probe: (1.895, 1.930, 0.045, 0.055),
};

/// The superseded `patch_game_91.big` #2608 revision. Geometry is identical; only the
/// `SKTN` joint-name ordering differs, and the semantics of that ordering are **not**
/// established — so this profile exists to be inspected and compared, not to be
/// interchanged with the base one.
pub const NHL19_EICHEL_PATCH91: DonorProfile = DonorProfile {
    name: "nhl19-jack_eichel-patch91",
    sha256: "3adde6df160db75240fc9ff907af8d05f4be41cd6866150aeb08435f8761932d",
    len: 1_043_168,
    submeshes: EICHEL_SUBMESHES,
    head_submesh: "geo_headShape",
    forehead_probe: (1.895, 1.930, 0.045, 0.055),
};

/// Every donor this editor will write to.
const GABORIK_SUBMESHES: &[ProfileSubmesh] = &[
    ProfileSubmesh {
        name: "geo_cardsAShape",
        triangles: 92,
        format: 0,
        streams: &[(98, 36)],
        index_chunk: 345672,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_cardsBShape",
        triangles: 408,
        format: 0,
        streams: &[(434, 36)],
        index_chunk: 346241,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_haircapShape",
        triangles: 1040,
        format: 1,
        streams: &[(580, 32)],
        index_chunk: 348706,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_reyeShape",
        triangles: 100,
        format: 2,
        streams: &[(66, 12), (66, 24)],
        index_chunk: 354963,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_headShape",
        triangles: 6014,
        format: 5,
        streams: &[(3285, 40)],
        index_chunk: 355580,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_leyeShape",
        triangles: 100,
        format: 2,
        streams: &[(66, 12), (66, 24)],
        index_chunk: 354963,
        shares_index_buffer: true,
    },
];
/// Measured NHL19 Gaborik donor; head includes native split lash records.
pub const NHL19_GABORIK_BASE: DonorProfile = DonorProfile {
    name: "nhl19-marian_gaborik-base",
    sha256: "e9e316e70a870b2116b8777f51f9c58038a99ce71be11f0fbf29365afca11b1b",
    len: 978768,
    submeshes: GABORIK_SUBMESHES,
    head_submesh: "geo_headShape",
    forehead_probe: (1.895, 1.930, 0.045, 0.055),
};
/// Separate revision binding; opaque rig/name ordering remains revision-specific.
pub const NHL19_GABORIK_PATCH91: DonorProfile = DonorProfile {
    name: "nhl19-marian_gaborik-patch91",
    sha256: "22b823733bd9a4c508677831000c59fc896938fe41da7fbf542f913e5e161203",
    ..NHL19_GABORIK_BASE
};
// Measured from all three original Bouwmeester entries. Both eyes share INDX.
const BOUWMEESTER_SUBMESHES: &[ProfileSubmesh] = &[
    ProfileSubmesh {
        name: "geo_haircapShape",
        triangles: 1142,
        format: 0,
        streams: &[(677, 32)],
        index_chunk: 313759,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_reyeShape",
        triangles: 100,
        format: 1,
        streams: &[(66, 12), (66, 24)],
        index_chunk: 320628,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_headShape",
        triangles: 6014,
        format: 4,
        streams: &[(3285, 40)],
        index_chunk: 321245,
        shares_index_buffer: false,
    },
    ProfileSubmesh {
        name: "geo_leyeShape",
        triangles: 100,
        format: 1,
        streams: &[(66, 12), (66, 24)],
        index_chunk: 320628,
        shares_index_buffer: true,
    },
];
pub const NHL19_BOUWMEESTER_BASE: DonorProfile = DonorProfile {
    name: "nhl19-jay_bouwmeester-base",
    sha256: "33a2f6dc5808dc6d787b65f3d22f5b4a61d909304208de82da6096a4ce26449b",
    len: 942448,
    submeshes: BOUWMEESTER_SUBMESHES,
    head_submesh: "geo_headShape",
    forehead_probe: (1.895, 1.930, 0.045, 0.055),
};
pub const NHL19_BOUWMEESTER_PATCH91: DonorProfile = DonorProfile {
    name: "nhl19-jay_bouwmeester-patch91",
    sha256: "9241944ec3c836c2e3b7f28271d80f66123bfe091a8daace21837919bae7dd10",
    ..NHL19_BOUWMEESTER_BASE
};
pub const KNOWN_DONORS: &[DonorProfile] = &[
    NHL19_EICHEL_BASE,
    NHL19_EICHEL_PATCH91,
    NHL19_GABORIK_BASE,
    NHL19_GABORIK_PATCH91,
    NHL19_BOUWMEESTER_BASE,
    NHL19_BOUWMEESTER_PATCH91,
];

/// Lowercase hex SHA-256 of `data`.
pub fn fingerprint(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// The profile matching `data`'s content hash, if any.
pub fn identify_donor(data: &[u8]) -> Option<&'static DonorProfile> {
    let fp = fingerprint(data);
    KNOWN_DONORS.iter().find(|p| p.sha256 == fp)
}

// ------------------------------------------------------------------------ regions

/// One span of the container in serialize order. The regions of an [`McdContainer`]
/// tile `[0, len)` exactly: gapless, non-overlapping, ascending.
#[derive(Debug, Clone)]
pub enum Region {
    /// Bytes carried through verbatim.
    Opaque { start: usize, end: usize },
    /// A STRM payload: raw interleaved records plus decoded `f32` positions.
    Vertices {
        start: usize,
        end: usize,
        /// Index into [`McdContainer::streams`].
        stream: usize,
    },
    /// A tail `MOEG` record's 6-float axis-aligned bounding box (min xyz, max xyz).
    Bounds {
        start: usize,
        end: usize,
        /// Index into [`McdContainer::submeshes`].
        sub: usize,
    },
}

impl Region {
    fn span(&self) -> (usize, usize) {
        match *self {
            Region::Opaque { start, end }
            | Region::Vertices { start, end, .. }
            | Region::Bounds { start, end, .. } => (start, end),
        }
    }
}

/// The per-component storage of one VFMT attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompType {
    F32,
    F16,
    S16N,
    U8,
}

impl CompType {
    fn size(self) -> usize {
        match self {
            CompType::F32 => 4,
            CompType::F16 | CompType::S16N => 2,
            CompType::U8 => 1,
        }
    }

    fn label(self) -> &'static str {
        match self {
            CompType::F32 => "f32",
            CompType::F16 => "f16",
            CompType::S16N => "s16n",
            CompType::U8 => "u8n",
        }
    }
}

/// One VFMT attribute, with its component type and count retained.
#[derive(Debug, Clone)]
struct Attr {
    name: String,
    offset: usize,
    stream: usize,
    count: usize,
    comp: CompType,
}

impl Attr {
    fn size(&self) -> usize {
        self.count * self.comp.size()
    }
}

/// One vertex stream's payload and layout.
#[derive(Debug, Clone)]
pub struct VertexStream {
    /// Offset of the `MRTS` tag.
    pub chunk_off: usize,
    /// Offset of the first vertex record.
    pub data_off: usize,
    pub count: usize,
    pub stride: usize,
    /// Byte offset of a `3f32` `p0` within a record, when this stream carries one.
    pub pos_off: Option<usize>,
    /// Interleaved records, verbatim.
    bytes: Vec<u8>,
    /// Decoded positions; empty when `pos_off` is `None`.
    positions: Vec<[f32; 3]>,
}

impl VertexStream {
    pub fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }
}

/// One declared submesh.
#[derive(Debug, Clone)]
pub struct Submesh {
    /// Tail name, e.g. `geo_headShape`.
    pub name: String,
    /// `HSEM` record offset.
    pub mesh_off: usize,
    pub triangles: usize,
    /// Index into the VFMT array.
    pub format: usize,
    /// Indices into [`McdContainer::streams`], in declaration order.
    pub streams: Vec<usize>,
    /// Offset of the validated `XDNI` chunk supplying this submesh's indices.
    pub index_chunk: usize,
    /// True when `index_chunk` is also claimed by an earlier submesh.
    pub shares_index_buffer: bool,
    /// Tail AABB, when the tail record was found.
    pub bounds: Option<([f32; 3], [f32; 3])>,
    bounds_off: Option<usize>,
}

/// A parsed, editable container.
#[derive(Debug, Clone)]
pub struct McdContainer {
    len: usize,
    regions: Vec<Region>,
    pub streams: Vec<VertexStream>,
    pub submeshes: Vec<Submesh>,
    /// `XDNI` chunks no submesh claims: `(offset, declared_count)`. Their content is
    /// unidentified and carried as opaque bytes.
    pub unclaimed_index_chunks: Vec<(usize, usize)>,
    /// Set when the container was matched against a [`DonorProfile`].
    pub profile: Option<&'static DonorProfile>,
    original: Vec<u8>,
}

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    d.get(o..o + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn u32_req(d: &[u8], o: usize, what: &'static str) -> Result<u32, McdWriteError> {
    u32_at(d, o).ok_or(McdWriteError::MissingDeclaration { what, offset: o })
}

fn f32_at(d: &[u8], o: usize) -> Option<f32> {
    d.get(o..o + 4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn find_all(hay: &[u8], needle: &[u8; 4], from: usize) -> Vec<usize> {
    if from >= hay.len() {
        return Vec::new();
    }
    hay[from..]
        .windows(4)
        .enumerate()
        .filter(|(_, w)| *w == needle)
        .map(|(i, _)| i + from)
        .collect()
}

/// Parse a VFMT `countType` token such as `3f32`, `4f16`, `4u8n` into `(count, type)`.
/// Rejects an empty or malformed token rather than panicking on it.
fn parse_comp(tok: &str) -> Option<(usize, CompType)> {
    let mut chars = tok.chars();
    let count = chars.next()?.to_digit(10)? as usize;
    if count == 0 || count > 4 {
        return None;
    }
    let comp = match chars.as_str() {
        "f32" => CompType::F32,
        "f16" => CompType::F16,
        "s16n" => CompType::S16N,
        "u8n" | "u8" => CompType::U8,
        _ => return None,
    };
    Some((count, comp))
}

/// Parse one VFMT chunk's attribute list.
fn parse_vfmt(data: &[u8], off: usize) -> Result<Vec<Attr>, McdWriteError> {
    let len = u32_req(data, off + 4, "VFMT length")? as usize;
    let body = data
        .get(off + 8..off + 8 + len)
        .ok_or(McdWriteError::MissingDeclaration {
            what: "VFMT body",
            offset: off,
        })?;
    let text = std::str::from_utf8(body).map_err(|_| McdWriteError::BadVfmt {
        offset: off,
        token: "<non-utf8>".into(),
    })?;
    let mut attrs = Vec::new();
    for tok in text.split_whitespace() {
        let bad = || McdWriteError::BadVfmt {
            offset: off,
            token: tok.to_string(),
        };
        let parts: Vec<&str> = tok.split(':').collect();
        if parts.len() < 3 || parts[0].is_empty() {
            return Err(bad());
        }
        let offset = usize::from_str_radix(parts[1], 16).map_err(|_| bad())?;
        let stream = usize::from_str_radix(parts[2], 16).map_err(|_| bad())?;
        let (count, comp) = parse_comp(parts[parts.len() - 1]).ok_or_else(bad)?;
        attrs.push(Attr {
            name: parts[0].to_string(),
            offset,
            stream,
            count,
            comp,
        });
    }
    Ok(attrs)
}

/// Validate one `XDNI` chunk as a triangle list over `vertices` vertices.
fn validate_index_buffer(
    data: &[u8],
    off: usize,
    count: usize,
    vertices: usize,
) -> Result<(), McdWriteError> {
    if count == 0 || !count.is_multiple_of(3) {
        return Err(McdWriteError::IndexCountNotTriangles { offset: off, count });
    }
    let token: [u8; 4] = data
        .get(off + INDX_TOKEN_OFF..off + INDX_TOKEN_OFF + 4)
        .and_then(|s| s.try_into().ok())
        .ok_or(McdWriteError::MissingDeclaration {
            what: "index width token",
            offset: off,
        })?;
    // Reversed ASCII: `61DI` is `ID16`, `23DI` is `ID32`.
    let width = match &token {
        b"61DI" => 2usize,
        b"23DI" => 4usize,
        _ => return Err(McdWriteError::UnsupportedIndexEncoding { offset: off, token }),
    };
    let start = off + INDX_DATA_OFF;
    let needed = count
        .checked_mul(width)
        .ok_or(McdWriteError::SizeOverflow { offset: off })?;
    let end = start
        .checked_add(needed)
        .ok_or(McdWriteError::SizeOverflow { offset: off })?;
    let bytes = data
        .get(start..end)
        .ok_or(McdWriteError::TruncatedIndexBuffer {
            offset: off,
            count,
            width,
            needed,
            have: data.len().saturating_sub(start),
        })?;
    for (slot, c) in bytes.chunks_exact(width).enumerate() {
        let v = if width == 2 {
            u16::from_le_bytes([c[0], c[1]]) as u32
        } else {
            u32::from_le_bytes([c[0], c[1], c[2], c[3]])
        };
        if v as usize >= vertices {
            return Err(McdWriteError::IndexOutOfRange {
                offset: off,
                slot,
                value: v,
                vertices,
            });
        }
    }
    Ok(())
}

impl McdContainer {
    /// Parse `data` into an editable model, or refuse it.
    ///
    /// This does **not** establish that `data` is any particular mesh. Use
    /// [`Self::parse_with_profile`] before editing.
    pub fn parse(data: &[u8]) -> Result<Self, McdWriteError> {
        Self::parse_inner(data, None)
    }

    /// Parse and require the container to match a fingerprinted, measured donor.
    ///
    /// The profile is selected by content hash, so a same-count or same-shaped file that
    /// is not the measured donor is refused: vertex counts cannot establish vertex order
    /// or protected-region identity.
    pub fn parse_with_profile(data: &[u8]) -> Result<Self, McdWriteError> {
        let profile = identify_donor(data).ok_or_else(|| McdWriteError::UnknownDonor {
            actual: fingerprint(data),
            len: data.len(),
        })?;
        let c = Self::parse_inner(data, Some(profile))?;
        c.check_profile(profile)?;
        Ok(c)
    }

    /// Every declared fact must match what was measured for this donor.
    fn check_profile(&self, p: &'static DonorProfile) -> Result<(), McdWriteError> {
        let fail = |detail: String| McdWriteError::ProfileMismatch {
            profile: p.name,
            detail,
        };
        if self.len != p.len {
            return Err(fail(format!("length {}, expected {}", self.len, p.len)));
        }
        if self.submeshes.len() != p.submeshes.len() {
            return Err(fail(format!(
                "{} submeshes, expected {}",
                self.submeshes.len(),
                p.submeshes.len()
            )));
        }
        for (i, (got, want)) in self.submeshes.iter().zip(p.submeshes).enumerate() {
            if got.name != want.name {
                return Err(fail(format!(
                    "submesh {i} is {:?}, expected {:?}",
                    got.name, want.name
                )));
            }
            if got.triangles != want.triangles || got.format != want.format {
                return Err(fail(format!(
                    "{:?}: {} tris / format {}, expected {} / {}",
                    got.name, got.triangles, got.format, want.triangles, want.format
                )));
            }
            if got.index_chunk != want.index_chunk
                || got.shares_index_buffer != want.shares_index_buffer
            {
                return Err(fail(format!(
                    "{:?}: index chunk {} (shared {}), expected {} (shared {})",
                    got.name,
                    got.index_chunk,
                    got.shares_index_buffer,
                    want.index_chunk,
                    want.shares_index_buffer
                )));
            }
            let got_streams: Vec<(usize, usize)> = got
                .streams
                .iter()
                .map(|&s| (self.streams[s].count, self.streams[s].stride))
                .collect();
            if got_streams != want.streams {
                return Err(fail(format!(
                    "{:?}: streams {:?}, expected {:?}",
                    got.name, got_streams, want.streams
                )));
            }
        }
        Ok(())
    }

    fn parse_inner(
        data: &[u8],
        profile: Option<&'static DonorProfile>,
    ) -> Result<Self, McdWriteError> {
        if data.len() < 16 {
            return Err(McdWriteError::TooShort(data.len()));
        }
        if &data[..4] != MAGIC_RSF {
            return Err(McdWriteError::BadMagic);
        }
        // A native asset holds exactly one container; concatenated copies are the
        // signature of an `Extracted`-tree dump.
        let extra = find_all(data, MAGIC_RSF, 1);
        if !extra.is_empty() {
            return Err(McdWriteError::ConcatenatedContainers {
                count: extra.len() + 1,
                offsets: extra,
            });
        }

        let vfmts: Vec<Vec<Attr>> = find_all(data, TAG_VFMT, 0x10)
            .iter()
            .map(|&o| parse_vfmt(data, o))
            .collect::<Result<_, _>>()?;

        // ---- STRM chunks, in file order.
        let mut streams = Vec::new();
        let mut last_end = 0usize;
        for off in find_all(data, TAG_STRM, 0x10) {
            let count = u32_req(data, off + CHUNK_COUNT_OFF, "STRM vertex count")? as usize;
            let stride = u32_req(data, off + STRM_STRIDE_OFF, "STRM stride")? as usize;
            let data_off = off + STRM_DATA_OFF;
            let need = count
                .checked_mul(stride)
                .ok_or(McdWriteError::SizeOverflow { offset: off })?;
            let end = data_off
                .checked_add(need)
                .ok_or(McdWriteError::SizeOverflow { offset: off })?;
            if end > data.len() {
                return Err(McdWriteError::Truncated {
                    offset: off,
                    needed: need,
                    have: data.len().saturating_sub(data_off),
                });
            }
            if data_off < last_end {
                return Err(McdWriteError::OverlappingStreams { offset: off });
            }
            last_end = end;
            streams.push(VertexStream {
                chunk_off: off,
                data_off,
                count,
                stride,
                pos_off: None,
                bytes: data[data_off..end].to_vec(),
                positions: Vec::new(),
            });
        }

        // ---- INDX chunks: offset → declared element count.
        let index_chunks: Vec<(usize, usize)> = find_all(data, TAG_INDX, 0x10)
            .into_iter()
            .map(|off| -> Result<(usize, usize), McdWriteError> {
                Ok((
                    off,
                    u32_req(data, off + CHUNK_COUNT_OFF, "INDX element count")? as usize,
                ))
            })
            .collect::<Result<_, _>>()?;

        let tail = tail_records(data);

        // ---- MESH records drive everything.
        let mesh_offs = find_all(data, TAG_MESH, 0x10);
        if mesh_offs.is_empty() {
            return Err(McdWriteError::NoSubmeshes);
        }
        let mut submeshes: Vec<Submesh> = Vec::new();
        let mut next_stream = 0usize;
        let mut used_index: Vec<usize> = Vec::new();
        for (sub, &mesh_off) in mesh_offs.iter().enumerate() {
            let tris = u32_req(data, mesh_off + MESH_TRIS_OFF, "MESH triangle count")? as usize;
            let format = u32_req(data, mesh_off + MESH_FORMAT_OFF, "MESH format index")? as usize;
            let n_streams =
                u32_req(data, mesh_off + MESH_STREAMS_OFF, "MESH stream count")? as usize;
            if n_streams == 0 {
                return Err(McdWriteError::MissingDeclaration {
                    what: "MESH stream count",
                    offset: mesh_off + MESH_STREAMS_OFF,
                });
            }

            let attrs = vfmts.get(format).ok_or(McdWriteError::FormatOutOfRange {
                sub,
                format,
                have: vfmts.len(),
            })?;
            if next_stream + n_streams > streams.len() {
                return Err(McdWriteError::StreamsExhausted {
                    sub,
                    want: n_streams,
                    have: streams.len() - next_stream,
                });
            }
            // Every attribute must name a slot this submesh actually has.
            for a in attrs {
                if a.stream >= n_streams {
                    return Err(McdWriteError::StreamSlotOutOfRange {
                        sub,
                        slot: a.stream,
                        slots: n_streams,
                    });
                }
            }

            let mut mine = Vec::with_capacity(n_streams);
            let first_count = streams[next_stream].count;
            for slot in 0..n_streams {
                let si = next_stream + slot;
                let stride = streams[si].stride;
                if streams[si].count != first_count {
                    return Err(McdWriteError::SplitStreamCountMismatch {
                        sub,
                        slot,
                        first: first_count,
                        other: streams[si].count,
                    });
                }
                for a in attrs.iter().filter(|a| a.stream == slot) {
                    let hi = a
                        .offset
                        .checked_add(a.size())
                        .ok_or(McdWriteError::SizeOverflow { offset: mesh_off })?;
                    if hi > stride {
                        return Err(McdWriteError::AttributeOutsideStride {
                            sub,
                            slot,
                            attr: a.name.clone(),
                            lo: a.offset,
                            hi,
                            stride,
                        });
                    }
                }
                let mut positions = attrs.iter().filter(|a| a.name == "p0" && a.stream == slot);
                if let Some(p) = positions.next() {
                    if positions.next().is_some() {
                        return Err(McdWriteError::DuplicatePosition { sub, slot });
                    }
                    if p.count != 3 || p.comp != CompType::F32 {
                        return Err(McdWriteError::UnsupportedPositionEncoding {
                            sub,
                            count: p.count,
                            comp: p.comp.label(),
                        });
                    }
                    streams[si].pos_off = Some(p.offset);
                }
                mine.push(si);
            }
            let Some(&pos_stream) = mine.iter().find(|&&s| streams[s].pos_off.is_some()) else {
                return Err(McdWriteError::NoPosition { sub });
            };
            next_stream += n_streams;

            // Index buffer: matched by declared element count, then *validated*, and
            // ambiguity refused. With a profile the offset is additionally pinned.
            let want = tris * 3;
            let vertices = streams[pos_stream].count;
            let index_chunk = if let Some(p) = profile.and_then(|p| p.submeshes.get(sub)) {
                p.index_chunk
            } else {
                let free: Vec<usize> = index_chunks
                    .iter()
                    .filter(|(o, c)| *c == want && !used_index.contains(o))
                    .map(|(o, _)| *o)
                    .collect();
                match free.len() {
                    1 => free[0],
                    0 => {
                        let shared: Vec<usize> = index_chunks
                            .iter()
                            .filter(|(o, c)| *c == want && used_index.contains(o))
                            .map(|(o, _)| *o)
                            .collect();
                        match shared.len() {
                            1 => shared[0],
                            0 => return Err(McdWriteError::NoIndexBuffer { sub, want }),
                            n => {
                                return Err(McdWriteError::AmbiguousIndexBuffer {
                                    sub,
                                    want,
                                    n,
                                    offsets: shared,
                                })
                            }
                        }
                    }
                    n => {
                        return Err(McdWriteError::AmbiguousIndexBuffer {
                            sub,
                            want,
                            n,
                            offsets: free,
                        })
                    }
                }
            };
            let declared = index_chunks
                .iter()
                .find(|(o, _)| *o == index_chunk)
                .map(|(_, c)| *c)
                .ok_or(McdWriteError::NoIndexBuffer { sub, want })?;
            if declared != want {
                return Err(McdWriteError::NoIndexBuffer { sub, want });
            }
            validate_index_buffer(data, index_chunk, declared, vertices)?;
            let shared = used_index.contains(&index_chunk);
            used_index.push(index_chunk);

            let rec = tail.get(&sub);
            let name = rec.map(|r| r.0.clone()).unwrap_or_default();
            if !name.is_empty() && submeshes.iter().any(|s| s.name == name) {
                return Err(McdWriteError::DuplicateSubmeshName(name));
            }
            submeshes.push(Submesh {
                name,
                mesh_off,
                triangles: tris,
                format,
                streams: mine,
                index_chunk,
                shares_index_buffer: shared,
                bounds: rec.map(|r| (r.2, r.3)),
                bounds_off: rec.map(|r| r.1),
            });
        }
        if next_stream != streams.len() {
            return Err(McdWriteError::OrphanStreams(streams.len() - next_stream));
        }

        let unclaimed_index_chunks: Vec<(usize, usize)> = index_chunks
            .iter()
            .copied()
            .filter(|(o, _)| !used_index.contains(o))
            .collect();

        // ---- Decode positions.
        for s in streams.iter_mut() {
            let Some(po) = s.pos_off else { continue };
            let mut v = Vec::with_capacity(s.count);
            for i in 0..s.count {
                let b = i * s.stride + po;
                v.push([
                    f32_at(&s.bytes, b).unwrap_or(0.0),
                    f32_at(&s.bytes, b + 4).unwrap_or(0.0),
                    f32_at(&s.bytes, b + 8).unwrap_or(0.0),
                ]);
            }
            s.positions = v;
        }

        // ---- Build the exhaustive region tiling.
        let mut typed: Vec<Region> = Vec::new();
        for (i, s) in streams.iter().enumerate() {
            typed.push(Region::Vertices {
                start: s.data_off,
                end: s.data_off + s.bytes.len(),
                stream: i,
            });
        }
        for (i, sm) in submeshes.iter().enumerate() {
            if let Some(off) = sm.bounds_off {
                typed.push(Region::Bounds {
                    start: off,
                    end: off + 24,
                    sub: i,
                });
            }
        }
        typed.sort_by_key(|r| r.span().0);

        let mut regions = Vec::new();
        let mut cursor = 0usize;
        for r in typed {
            let (start, end) = r.span();
            if start < cursor {
                return Err(McdWriteError::OverlappingStreams { offset: start });
            }
            if start > cursor {
                regions.push(Region::Opaque {
                    start: cursor,
                    end: start,
                });
            }
            regions.push(r);
            cursor = end;
        }
        if cursor < data.len() {
            regions.push(Region::Opaque {
                start: cursor,
                end: data.len(),
            });
        }

        let covered: usize = regions.iter().map(|r| r.span().1 - r.span().0).sum();
        if covered != data.len() {
            return Err(McdWriteError::TilingIncomplete {
                covered,
                total: data.len(),
            });
        }

        Ok(Self {
            len: data.len(),
            regions,
            streams,
            submeshes,
            unclaimed_index_chunks,
            profile,
            original: data.to_vec(),
        })
    }

    /// Submesh index by tail name.
    pub fn submesh_index(&self, name: &str) -> Option<usize> {
        self.submeshes.iter().position(|s| s.name == name)
    }

    /// The stream carrying `sub`'s positions.
    pub fn position_stream_of(&self, sub: usize) -> Option<usize> {
        self.submeshes
            .get(sub)?
            .streams
            .iter()
            .copied()
            .find(|&s| self.streams[s].pos_off.is_some())
    }

    /// Replace one stream's positions. The count must match exactly and every coordinate
    /// must be finite: this editor never changes vertex order, count, topology or any
    /// other attribute.
    pub fn set_positions(
        &mut self,
        stream: usize,
        positions: &[[f32; 3]],
    ) -> Result<(), McdWriteError> {
        let s = self
            .streams
            .get_mut(stream)
            .ok_or(McdWriteError::StreamHasNoPositions { stream })?;
        if s.pos_off.is_none() {
            return Err(McdWriteError::StreamHasNoPositions { stream });
        }
        if positions.len() != s.count {
            return Err(McdWriteError::PositionCountMismatch {
                stream,
                want: s.count,
                got: positions.len(),
            });
        }
        for (index, p) in positions.iter().enumerate() {
            for (axis, v) in p.iter().enumerate() {
                if !v.is_finite() {
                    return Err(McdWriteError::NonFinitePosition {
                        stream,
                        index,
                        axis,
                        value: *v,
                    });
                }
            }
        }
        s.positions.copy_from_slice(positions);
        Ok(())
    }

    /// Recompute a submesh's tail AABB from its current positions.
    pub fn refresh_bounds(&mut self, sub: usize) -> bool {
        let Some(stream) = self.position_stream_of(sub) else {
            return false;
        };
        if self.submeshes.get(sub).and_then(|s| s.bounds_off).is_none() {
            return false;
        }
        let p = &self.streams[stream].positions;
        if p.is_empty() {
            return false;
        }
        let (mut min, mut max) = (p[0], p[0]);
        for v in p {
            for k in 0..3 {
                min[k] = min[k].min(v[k]);
                max[k] = max[k].max(v[k]);
            }
        }
        self.submeshes[sub].bounds = Some((min, max));
        true
    }

    /// Re-emit the container from the region model.
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.len);
        for r in &self.regions {
            match *r {
                Region::Opaque { start, end } => out.extend_from_slice(&self.original[start..end]),
                Region::Vertices { stream, .. } => {
                    let s = &self.streams[stream];
                    let base = out.len();
                    out.extend_from_slice(&s.bytes);
                    if let Some(po) = s.pos_off {
                        for (i, p) in s.positions.iter().enumerate() {
                            let b = base + i * s.stride + po;
                            out[b..b + 4].copy_from_slice(&p[0].to_le_bytes());
                            out[b + 4..b + 8].copy_from_slice(&p[1].to_le_bytes());
                            out[b + 8..b + 12].copy_from_slice(&p[2].to_le_bytes());
                        }
                    }
                }
                Region::Bounds { start, end, sub } => match self.submeshes[sub].bounds {
                    Some((min, max)) => {
                        for v in min.iter().chain(max.iter()) {
                            out.extend_from_slice(&v.to_le_bytes());
                        }
                    }
                    None => out.extend_from_slice(&self.original[start..end]),
                },
            }
        }
        out
    }

    /// Byte spans this model may rewrite: every vertex payload and every tail AABB.
    /// Anything outside them must be identical between input and output — the
    /// allowlist a byte-span diff is checked against.
    pub fn writable_spans(&self) -> Vec<(usize, usize)> {
        self.regions
            .iter()
            .filter_map(|r| match r {
                Region::Vertices { .. } | Region::Bounds { .. } => Some(r.span()),
                Region::Opaque { .. } => None,
            })
            .collect()
    }

    /// Region-kind counts, for reporting: `(opaque, vertices, bounds)`.
    pub fn region_counts(&self) -> (usize, usize, usize) {
        let mut c = (0, 0, 0);
        for r in &self.regions {
            match r {
                Region::Opaque { .. } => c.0 += 1,
                Region::Vertices { .. } => c.1 += 1,
                Region::Bounds { .. } => c.2 += 1,
            }
        }
        c
    }

    /// Decoded triangle list for one submesh, read from its validated index buffer.
    /// Used by inspection/export; the editor itself never rewrites indices.
    pub fn triangles_of(&self, sub: usize) -> Option<Vec<[u32; 3]>> {
        let sm = self.submeshes.get(sub)?;
        let off = sm.index_chunk;
        let count = sm.triangles * 3;
        let token: [u8; 4] = self
            .original
            .get(off + INDX_TOKEN_OFF..off + INDX_TOKEN_OFF + 4)?
            .try_into()
            .ok()?;
        let width = match &token {
            b"61DI" => 2usize,
            b"23DI" => 4usize,
            _ => return None,
        };
        let start = off + INDX_DATA_OFF;
        let bytes = self.original.get(start..start + count * width)?;
        let idx: Vec<u32> = bytes
            .chunks_exact(width)
            .map(|c| {
                if width == 2 {
                    u16::from_le_bytes([c[0], c[1]]) as u32
                } else {
                    u32::from_le_bytes([c[0], c[1], c[2], c[3]])
                }
            })
            .collect();
        Some(idx.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect())
    }
}

/// Tail `MOEG` records: submesh ordinal → (name, aabb offset, min, max).
///
/// Each record is `MOEG | u32 | u32 name_len | name | NUL | 6 × f32`, appearing in
/// submesh order after the first MESH record.
type TailRecord = (String, usize, [f32; 3], [f32; 3]);

fn tail_records(data: &[u8]) -> BTreeMap<usize, TailRecord> {
    let mut out = BTreeMap::new();
    let Some(first_mesh) = find_all(data, TAG_MESH, 0x10).first().copied() else {
        return out;
    };
    let mut ord = 0usize;
    for off in find_all(data, TAG_GEOM, first_mesh) {
        let Some(len) = u32_at(data, off + 8) else {
            continue;
        };
        let len = len as usize;
        if len == 0 || len > 64 {
            continue;
        }
        let Some(raw) = data.get(off + 12..off + 12 + len) else {
            continue;
        };
        let Ok(name) = std::str::from_utf8(raw) else {
            continue;
        };
        if !name.starts_with("geo_") || !raw.iter().all(|&b| (0x20..0x7f).contains(&b)) {
            continue;
        }
        let fo = off + 12 + len + 1;
        let mut v = [0f32; 6];
        let mut ok = true;
        for (k, slot) in v.iter_mut().enumerate() {
            match f32_at(data, fo + k * 4) {
                Some(f) if f.is_finite() => *slot = f,
                _ => ok = false,
            }
        }
        if !ok {
            continue;
        }
        out.insert(
            ord,
            (name.to_string(), fo, [v[0], v[1], v[2]], [v[3], v[4], v[5]]),
        );
        ord += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic submesh declaration.
    struct Sub {
        name: &'static str,
        tris: usize,
        format: usize,
        /// One `(count, stride)` per vertex stream the submesh consumes.
        streams: Vec<(usize, usize)>,
    }

    /// Assemble a minimal but structurally faithful RSF container: header, VFMT array,
    /// STRM payloads, INDX chunks, MESH records, then tail name/AABB records.
    ///
    /// Deliberately synthetic — no EA bytes are committed. The layout mirrors the field
    /// positions the parser reads, so these cases exercise the real code paths.
    ///
    /// Each entry of `indices` is `(declared_count, payload)`: the declared element count
    /// written into the chunk header and the `u16` elements actually stored. Letting the
    /// two disagree is how the truncation and out-of-range cases are built.
    fn build_ex(vfmts: &[&str], subs: &[Sub], indices: &[(usize, Vec<u16>)]) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(MAGIC_RSF);
        d.extend_from_slice(&[0u8; 12]); // header padding to 0x10

        for v in vfmts {
            d.extend_from_slice(TAG_VFMT);
            d.extend_from_slice(&(v.len() as u32).to_le_bytes());
            d.extend_from_slice(v.as_bytes());
        }

        for (si, (count, stride)) in subs.iter().flat_map(|s| s.streams.iter()).enumerate() {
            d.extend_from_slice(TAG_STRM);
            d.push(0);
            d.extend_from_slice(&2u32.to_le_bytes()); // version
            d.extend_from_slice(&(*count as u32).to_le_bytes());
            d.extend_from_slice(&(*stride as u32).to_le_bytes());
            d.extend_from_slice(&(si as u32).to_le_bytes());
            // Deterministic payload; positions are real f32s so the decode/encode round
            // trip is actually exercised rather than trivially preserved.
            for i in 0..*count {
                let mut rec = vec![0u8; *stride];
                if *stride >= 12 {
                    rec[0..4].copy_from_slice(&(i as f32 * 0.5 + si as f32).to_le_bytes());
                    rec[4..8].copy_from_slice(&(1.5 + i as f32 * 0.25).to_le_bytes());
                    rec[8..12].copy_from_slice(&(-0.125 * i as f32).to_le_bytes());
                }
                for (k, b) in rec.iter_mut().enumerate().skip(12) {
                    *b = ((i * 7 + k * 13 + si) % 251) as u8;
                }
                d.extend_from_slice(&rec);
            }
        }

        for (declared, payload) in indices {
            d.extend_from_slice(TAG_INDX);
            d.push(0);
            d.extend_from_slice(&0u32.to_le_bytes());
            d.extend_from_slice(&(*declared as u32).to_le_bytes());
            d.extend_from_slice(b"61DI"); // reversed `ID16`
            for e in payload {
                d.extend_from_slice(&e.to_le_bytes());
            }
        }

        for (i, s) in subs.iter().enumerate() {
            d.extend_from_slice(TAG_MESH);
            d.extend_from_slice(&[0u8; 4]);
            d.extend_from_slice(b"TSLT");
            d.extend_from_slice(&[0u8; 8]);
            d.extend_from_slice(&(s.tris as u32).to_le_bytes()); // +20
            d.extend_from_slice(&(s.format as u32).to_le_bytes()); // +24
            d.extend_from_slice(&1u32.to_le_bytes()); // +28
            d.extend_from_slice(&(s.streams.len() as u32).to_le_bytes()); // +32
            d.extend_from_slice(&(i as u32).to_le_bytes()); // +36
        }

        for (i, s) in subs.iter().enumerate() {
            d.extend_from_slice(TAG_GEOM);
            d.extend_from_slice(&[0u8; 4]);
            d.extend_from_slice(&(s.name.len() as u32).to_le_bytes());
            d.extend_from_slice(s.name.as_bytes());
            d.push(0);
            for k in 0..6 {
                d.extend_from_slice(&((i as f32) * 0.1 + k as f32).to_le_bytes());
            }
        }
        d
    }

    /// `build_ex` with well-formed all-zero index payloads of the declared length.
    fn build(vfmts: &[&str], subs: &[Sub], index_counts: &[usize]) -> Vec<u8> {
        let ix: Vec<(usize, Vec<u16>)> = index_counts.iter().map(|&c| (c, vec![0u16; c])).collect();
        build_ex(vfmts, subs, &ix)
    }

    /// One position-bearing stream, stride 16 (`p0` plus a UV).
    const VF_SIMPLE: &str = "p0:00:00:0001:0000:00:3f32 t0:0C:00:0001:0000:00:2f16";
    /// Two streams: positions in slot 0 (stride 12), shading in slot 1 (stride 16) —
    /// the shape the real eyes use.
    const VF_SPLIT: &str =
        "p0:00:00:0001:0000:00:3f32 n0:00:01:0001:0000:00:4f16 t0:08:01:0001:0000:00:2f16";

    fn one_sub() -> Vec<Sub> {
        vec![Sub {
            name: "geo_testShape",
            tris: 4,
            format: 0,
            streams: vec![(6, 16)],
        }]
    }

    fn two_eyes() -> Vec<Sub> {
        vec![
            Sub {
                name: "geo_reyeShape",
                tris: 4,
                format: 0,
                streams: vec![(6, 16)],
            },
            Sub {
                name: "geo_leyeShape",
                tris: 4,
                format: 0,
                streams: vec![(6, 16)],
            },
        ]
    }

    // ------------------------------------------------------------- happy paths

    #[test]
    fn synthetic_container_round_trips_byte_for_byte() {
        let d = build(&[VF_SIMPLE], &one_sub(), &[12]);
        let c = McdContainer::parse(&d).expect("parse");
        assert_eq!(c.submeshes.len(), 1);
        assert_eq!(c.streams.len(), 1);
        assert_eq!(c.streams[0].pos_off, Some(0));
        assert_eq!(c.streams[0].positions().len(), 6);
        assert_eq!(c.region_counts().1, 1);
        assert_eq!(c.serialize(), d, "no-op serialize must be byte-identical");
    }

    #[test]
    fn split_stream_submesh_binds_positions_to_the_declared_slot() {
        let subs = vec![Sub {
            name: "geo_eyeShape",
            tris: 2,
            format: 0,
            streams: vec![(5, 12), (5, 16)],
        }];
        let d = build(&[VF_SPLIT], &subs, &[6]);
        let c = McdContainer::parse(&d).expect("parse");
        assert_eq!(c.streams.len(), 2);
        assert_eq!(c.streams[0].pos_off, Some(0), "slot 0 carries p0");
        assert_eq!(c.streams[1].pos_off, None, "slot 1 carries no position");
        assert_eq!(c.position_stream_of(0), Some(0));
        assert_eq!(c.serialize(), d);
    }

    #[test]
    fn two_submeshes_with_equal_triangle_counts_share_one_index_buffer() {
        // One index buffer for two 4-triangle submeshes: the second must be recognised as
        // sharing it, not handed some other buffer that happens to range-validate.
        let d = build(&[VF_SIMPLE], &two_eyes(), &[12]);
        let c = McdContainer::parse(&d).expect("parse");
        assert_eq!(c.submeshes[0].index_chunk, c.submeshes[1].index_chunk);
        assert!(!c.submeshes[0].shares_index_buffer);
        assert!(c.submeshes[1].shares_index_buffer);
        assert_eq!(c.submeshes[1].triangles, 4);
    }

    #[test]
    fn an_index_buffer_no_submesh_claims_is_reported_not_silently_dropped() {
        // 6 elements matches no submesh here; it must surface as unclaimed. This is the
        // synthetic stand-in for NHL 19's second XDNI group, whose 132/2348/168/2256/
        // 200/12640/200 counts belong to no submesh and whose purpose is unidentified.
        let d = build(&[VF_SIMPLE], &one_sub(), &[6, 12]);
        let c = McdContainer::parse(&d).expect("parse");
        assert_eq!(c.submeshes[0].triangles, 4);
        assert_eq!(c.unclaimed_index_chunks.len(), 1);
        assert_eq!(c.unclaimed_index_chunks[0].1, 6);
        let first = find_all(&d, TAG_INDX, 0x10)[0];
        assert_ne!(
            c.submeshes[0].index_chunk, first,
            "must not take the first validating buffer"
        );
    }

    // ------------------------------------------------------- index-buffer refusals

    #[test]
    fn rejects_two_unclaimed_buffers_with_the_same_element_count() {
        // Element count alone cannot decide between them, so refuse rather than guess.
        let d = build(&[VF_SIMPLE], &one_sub(), &[12, 12]);
        match McdContainer::parse(&d) {
            Err(McdWriteError::AmbiguousIndexBuffer { want, n, .. }) => {
                assert_eq!((want, n), (12, 2));
            }
            other => panic!("expected AmbiguousIndexBuffer, got {other:?}"),
        }
    }

    #[test]
    fn rejects_ambiguous_sharing_between_equal_count_buffers() {
        // Three equal-count buffers and two submeshes: after the first two are claimed the
        // third submesh would have to pick among used buffers. Build that shape directly.
        let mut subs = two_eyes();
        subs.push(Sub {
            name: "geo_thirdShape",
            tris: 4,
            format: 0,
            streams: vec![(6, 16)],
        });
        let d = build(&[VF_SIMPLE], &subs, &[12, 12]);
        // Two free buffers for three equal-count submeshes: the first claim is already
        // ambiguous, which is the point — ambiguity is refused at the earliest opportunity.
        assert!(matches!(
            McdContainer::parse(&d),
            Err(McdWriteError::AmbiguousIndexBuffer { .. })
        ));
    }

    #[test]
    fn rejects_an_index_buffer_whose_payload_is_truncated() {
        // Declares 300000 elements but stores 12.
        let subs = vec![Sub {
            name: "geo_testShape",
            tris: 100_000,
            format: 0,
            streams: vec![(6, 16)],
        }];
        let d = build_ex(&[VF_SIMPLE], &subs, &[(300_000, vec![0u16; 12])]);
        match McdContainer::parse(&d) {
            Err(McdWriteError::TruncatedIndexBuffer { count, width, .. }) => {
                assert_eq!((count, width), (300_000, 2));
            }
            other => panic!("expected TruncatedIndexBuffer, got {other:?}"),
        }
    }

    #[test]
    fn rejects_an_index_out_of_range_for_its_vertex_count() {
        let mut payload = vec![0u16; 12];
        payload[11] = 999; // only 6 vertices exist
        let d = build_ex(&[VF_SIMPLE], &one_sub(), &[(12, payload)]);
        match McdContainer::parse(&d) {
            Err(McdWriteError::IndexOutOfRange {
                slot,
                value,
                vertices,
                ..
            }) => {
                assert_eq!((slot, value, vertices), (11, 999, 6));
            }
            other => panic!("expected IndexOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn rejects_an_element_count_that_is_not_whole_triangles() {
        // A submesh cannot declare a fractional triangle, so drive it through a buffer
        // whose stored count is checked directly.
        let d = build(&[VF_SIMPLE], &one_sub(), &[12]);
        let off = find_all(&d, TAG_INDX, 0x10)[0];
        assert!(matches!(
            validate_index_buffer(&d, off, 11, 6),
            Err(McdWriteError::IndexCountNotTriangles { count: 11, .. })
        ));
    }

    #[test]
    fn rejects_an_unsupported_index_width_token() {
        let mut d = build(&[VF_SIMPLE], &one_sub(), &[12]);
        let off = find_all(&d, TAG_INDX, 0x10)[0];
        d[off + INDX_TOKEN_OFF..off + INDX_TOKEN_OFF + 4].copy_from_slice(b"ZZZZ");
        match McdContainer::parse(&d) {
            Err(McdWriteError::UnsupportedIndexEncoding { token, .. }) => {
                assert_eq!(&token, b"ZZZZ");
            }
            other => panic!("expected UnsupportedIndexEncoding, got {other:?}"),
        }
    }

    #[test]
    fn rejects_a_missing_index_buffer() {
        let d = build(&[VF_SIMPLE], &one_sub(), &[99]);
        assert!(matches!(
            McdContainer::parse(&d),
            Err(McdWriteError::NoIndexBuffer { want: 12, .. })
        ));
    }

    // ------------------------------------------------------------- VFMT refusals

    #[test]
    fn rejects_a_position_that_is_not_three_f32() {
        let vf = "p0:00:00:0001:0000:00:3f16 t0:0C:00:0001:0000:00:2f16";
        let d = build(&[vf], &one_sub(), &[12]);
        match McdContainer::parse(&d) {
            Err(McdWriteError::UnsupportedPositionEncoding { count, comp, .. }) => {
                assert_eq!((count, comp), (3, "f16"));
            }
            other => panic!("expected UnsupportedPositionEncoding, got {other:?}"),
        }
    }

    #[test]
    fn rejects_malformed_and_empty_vfmt_tokens_without_panicking() {
        for vf in [
            "p0:00:00:0001:0000:00:",     // empty component token
            "p0:00:00:0001:0000:00:9zz",  // unknown component type
            "p0:00",                      // too few fields
            ":00:00:0001:0000:00:3f32",   // empty attribute name
            "p0:zz:00:0001:0000:00:3f32", // non-hex offset
            "p0:00:zz:0001:0000:00:3f32", // non-hex stream slot
            "p0:00:00:0001:0000:00:0f32", // zero component count
        ] {
            let d = build(&[vf], &one_sub(), &[12]);
            assert!(
                matches!(McdContainer::parse(&d), Err(McdWriteError::BadVfmt { .. })),
                "token {vf:?} should be refused as malformed"
            );
        }
    }

    #[test]
    fn rejects_an_attribute_running_past_its_stride() {
        let subs = vec![Sub {
            name: "geo_testShape",
            tris: 4,
            format: 0,
            streams: vec![(6, 8)], // VF_SIMPLE needs 16
        }];
        let d = build(&[VF_SIMPLE], &subs, &[12]);
        match McdContainer::parse(&d) {
            Err(McdWriteError::AttributeOutsideStride {
                attr, hi, stride, ..
            }) => {
                assert_eq!((attr.as_str(), hi, stride), ("p0", 12, 8));
            }
            other => panic!("expected AttributeOutsideStride, got {other:?}"),
        }
    }

    #[test]
    fn rejects_a_vfmt_naming_a_stream_slot_the_submesh_does_not_have() {
        // VF_SPLIT declares slot 1, but this submesh declares a single stream.
        let subs = vec![Sub {
            name: "geo_testShape",
            tris: 2,
            format: 0,
            streams: vec![(5, 12)],
        }];
        let d = build(&[VF_SPLIT], &subs, &[6]);
        assert!(matches!(
            McdContainer::parse(&d),
            Err(McdWriteError::StreamSlotOutOfRange {
                slot: 1,
                slots: 1,
                ..
            })
        ));
    }

    #[test]
    fn rejects_split_streams_with_unequal_vertex_counts() {
        let subs = vec![Sub {
            name: "geo_eyeShape",
            tris: 2,
            format: 0,
            streams: vec![(5, 12), (4, 16)], // 5 vs 4
        }];
        let d = build(&[VF_SPLIT], &subs, &[6]);
        match McdContainer::parse(&d) {
            Err(McdWriteError::SplitStreamCountMismatch { first, other, .. }) => {
                assert_eq!((first, other), (5, 4));
            }
            other => panic!("expected SplitStreamCountMismatch, got {other:?}"),
        }
    }

    // ----------------------------------------------------------- structural refusals

    #[test]
    fn rejects_concatenated_containers() {
        let d = build(&[VF_SIMPLE], &one_sub(), &[12]);
        let mut two = d.clone();
        two.extend_from_slice(&d);
        match McdContainer::parse(&two) {
            Err(McdWriteError::ConcatenatedContainers { count, offsets }) => {
                assert_eq!(count, 2);
                assert_eq!(offsets, vec![d.len()]);
            }
            other => panic!("expected ConcatenatedContainers, got {other:?}"),
        }
    }

    #[test]
    fn rejects_a_format_index_out_of_range() {
        let subs = vec![Sub {
            name: "geo_testShape",
            tris: 4,
            format: 9,
            streams: vec![(6, 16)],
        }];
        let d = build(&[VF_SIMPLE], &subs, &[12]);
        assert!(matches!(
            McdContainer::parse(&d),
            Err(McdWriteError::FormatOutOfRange {
                format: 9,
                have: 1,
                ..
            })
        ));
    }

    #[test]
    fn rejects_streams_no_submesh_claims() {
        let d = build(&[VF_SIMPLE], &one_sub(), &[12]);
        let mesh_at = find_all(&d, TAG_MESH, 0x10)[0];
        let mut extra = Vec::new();
        extra.extend_from_slice(TAG_STRM);
        extra.push(0);
        extra.extend_from_slice(&2u32.to_le_bytes());
        extra.extend_from_slice(&3u32.to_le_bytes());
        extra.extend_from_slice(&16u32.to_le_bytes());
        extra.extend_from_slice(&0u32.to_le_bytes());
        extra.extend_from_slice(&[0u8; 48]);
        let mut spliced = d.clone();
        spliced.splice(mesh_at..mesh_at, extra);
        assert!(matches!(
            McdContainer::parse(&spliced),
            Err(McdWriteError::OrphanStreams(1))
        ));
    }

    #[test]
    fn rejects_a_stream_running_past_end_of_file() {
        let d = build(&[VF_SIMPLE], &one_sub(), &[12]);
        let strm = find_all(&d, TAG_STRM, 0x10)[0];
        let mut bad = d.clone();
        bad[strm + CHUNK_COUNT_OFF..strm + CHUNK_COUNT_OFF + 4]
            .copy_from_slice(&100_000u32.to_le_bytes());
        assert!(matches!(
            McdContainer::parse(&bad),
            Err(McdWriteError::Truncated { .. })
        ));
    }

    #[test]
    fn rejects_a_declaration_word_that_runs_off_the_end() {
        let d = build(&[VF_SIMPLE], &one_sub(), &[12]);
        // Cut the file inside the last MESH record's declaration words.
        let mesh = *find_all(&d, TAG_MESH, 0x10).last().unwrap();
        let cut = &d[..mesh + MESH_TRIS_OFF + 2];
        assert!(matches!(
            McdContainer::parse(cut),
            Err(McdWriteError::MissingDeclaration { .. })
        ));
    }

    #[test]
    fn rejects_bad_magic_and_short_input() {
        assert!(matches!(
            McdContainer::parse(&[0u8; 4]),
            Err(McdWriteError::TooShort(4))
        ));
        let mut d = build(&[VF_SIMPLE], &one_sub(), &[12]);
        d[0] = b'X';
        assert!(matches!(
            McdContainer::parse(&d),
            Err(McdWriteError::BadMagic)
        ));
    }

    // ------------------------------------------------------------------ editing

    #[test]
    fn set_positions_enforces_the_exact_vertex_count() {
        let d = build(&[VF_SIMPLE], &one_sub(), &[12]);
        let mut c = McdContainer::parse(&d).expect("parse");
        assert!(matches!(
            c.set_positions(0, &[[0.0; 3]; 5]),
            Err(McdWriteError::PositionCountMismatch {
                want: 6,
                got: 5,
                ..
            })
        ));
        assert!(matches!(
            c.set_positions(0, &[[0.0; 3]; 7]),
            Err(McdWriteError::PositionCountMismatch {
                want: 6,
                got: 7,
                ..
            })
        ));
        assert_eq!(
            c.serialize(),
            d,
            "a refused edit must leave the model intact"
        );
    }

    #[test]
    fn set_positions_refuses_non_finite_coordinates() {
        let d = build(&[VF_SIMPLE], &one_sub(), &[12]);
        let c = McdContainer::parse(&d).expect("parse");
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut edited = c.clone();
            let mut p = edited.streams[0].positions().to_vec();
            p[3][2] = bad;
            match edited.set_positions(0, &p) {
                Err(McdWriteError::NonFinitePosition { index, axis, .. }) => {
                    assert_eq!((index, axis), (3, 2));
                }
                other => panic!("expected NonFinitePosition for {bad}, got {other:?}"),
            }
            assert_eq!(
                edited.serialize(),
                d,
                "a refused edit must not reach the output"
            );
        }
    }

    #[test]
    fn set_positions_refuses_a_stream_without_positions() {
        let subs = vec![Sub {
            name: "geo_eyeShape",
            tris: 2,
            format: 0,
            streams: vec![(5, 12), (5, 16)],
        }];
        let d = build(&[VF_SPLIT], &subs, &[6]);
        let mut c = McdContainer::parse(&d).expect("parse");
        assert!(matches!(
            c.set_positions(1, &[[0.0; 3]; 5]),
            Err(McdWriteError::StreamHasNoPositions { stream: 1 })
        ));
    }

    #[test]
    fn a_position_edit_touches_only_allowlisted_spans() {
        let d = build(&[VF_SIMPLE], &one_sub(), &[12]);
        let c = McdContainer::parse(&d).expect("parse");
        let mut edited = c.clone();
        let mut p = edited.streams[0].positions().to_vec();
        p[2][1] += 0.5;
        edited.set_positions(0, &p).expect("same count");
        let out = edited.serialize();

        assert_eq!(out.len(), d.len(), "an edit never changes the length");
        let spans = c.writable_spans();
        let changed: Vec<usize> = out
            .iter()
            .zip(&d)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, _)| i)
            .collect();
        assert!(!changed.is_empty(), "the edit must actually change bytes");
        for i in &changed {
            assert!(
                spans.iter().any(|&(s, e)| *i >= s && *i < e),
                "byte {i} changed outside the writable allowlist"
            );
        }
        let re = McdContainer::parse(&out).expect("reparse");
        for (i, (a, b)) in c.streams[0]
            .positions()
            .iter()
            .zip(re.streams[0].positions())
            .enumerate()
        {
            if i == 2 {
                continue;
            }
            assert_eq!(a[0].to_bits(), b[0].to_bits());
            assert_eq!(a[1].to_bits(), b[1].to_bits());
            assert_eq!(a[2].to_bits(), b[2].to_bits());
        }
    }

    #[test]
    fn refresh_bounds_recomputes_from_the_current_positions() {
        let d = build(&[VF_SIMPLE], &one_sub(), &[12]);
        let c = McdContainer::parse(&d).expect("parse");
        let mut edited = c.clone();
        assert!(edited.refresh_bounds(0));
        let (min, max) = edited.submeshes[0].bounds.expect("bounds");
        let p = c.streams[0].positions();
        for k in 0..3 {
            let lo = p.iter().map(|v| v[k]).fold(f32::INFINITY, f32::min);
            let hi = p.iter().map(|v| v[k]).fold(f32::NEG_INFINITY, f32::max);
            assert_eq!(min[k], lo);
            assert_eq!(max[k], hi);
        }
        assert_eq!(edited.serialize().len(), d.len());
    }

    #[test]
    fn triangles_of_reads_the_validated_buffer() {
        let payload: Vec<u16> = vec![0, 1, 2, 3, 4, 5, 0, 2, 4, 1, 3, 5];
        let d = build_ex(&[VF_SIMPLE], &one_sub(), &[(12, payload)]);
        let c = McdContainer::parse(&d).expect("parse");
        let tris = c.triangles_of(0).expect("triangles");
        assert_eq!(tris.len(), 4);
        assert_eq!(tris[0], [0, 1, 2]);
        assert_eq!(tris[3], [1, 3, 5]);
    }

    // ------------------------------------------------------------- donor profiles

    #[test]
    fn an_unrecognised_donor_is_refused_for_editing() {
        // Structurally valid, but it is not a measured donor — so it must not be editable
        // through the profile-gated entry point. Same vertex counts would not help.
        let d = build(&[VF_SIMPLE], &one_sub(), &[12]);
        assert!(McdContainer::parse(&d).is_ok(), "inspection still works");
        match McdContainer::parse_with_profile(&d) {
            Err(McdWriteError::UnknownDonor { actual, len }) => {
                assert_eq!(actual, fingerprint(&d));
                assert_eq!(len, d.len());
            }
            other => panic!("expected UnknownDonor, got {other:?}"),
        }
    }

    #[test]
    fn known_donor_profiles_are_self_consistent() {
        for p in KNOWN_DONORS {
            assert_eq!(
                p.sha256.len(),
                64,
                "{}: sha256 must be 64 hex chars",
                p.name
            );
            assert!(
                p.sha256
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
                "{}: sha256 must be lowercase hex",
                p.name
            );
            assert!(
                p.submeshes.iter().any(|s| s.name == p.head_submesh),
                "{}: head submesh {:?} is not in the profile",
                p.name,
                p.head_submesh
            );
            let (lo, hi, _, _) = p.forehead_probe;
            assert!(lo < hi, "{}: forehead probe band is inverted", p.name);
            // A shared index buffer must actually be shared with an earlier submesh.
            for (i, s) in p.submeshes.iter().enumerate() {
                if s.shares_index_buffer {
                    assert!(
                        p.submeshes[..i]
                            .iter()
                            .any(|e| e.index_chunk == s.index_chunk),
                        "{}: {:?} claims sharing but no earlier submesh uses {}",
                        p.name,
                        s.name,
                        s.index_chunk
                    );
                }
            }
        }
    }

    #[test]
    fn fingerprint_is_lowercase_hex_sha256() {
        let fp = fingerprint(b"");
        assert_eq!(
            fp,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
