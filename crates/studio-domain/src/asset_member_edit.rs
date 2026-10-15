//! P1-F (PS4 side, D8) — replace a texture **member inside a packed EB archive** through the
//! copied-target route: the PS4 league logos and presentation imagery live in `databigs/inxp_10.big`
//! (chunkzip'd GNF; NHL 15 raw PNG in `inxp10.big`), never loose, so a replaced logo only reaches the
//! game when the archive member is rewritten. The plan re-uses the loose-file texture writer on the
//! decoded member, re-encodes the member in its stored form (native-framed chunkzip or raw), and
//! records the identities the stager binds to. Deduplicated members (research 02 §1.4: a league
//! logo, its French twin and a scouting-region logo can share one block) take a **private copy**
//! — appended, only the chosen row retargeted — and the siblings are named for the review.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tdb_archive_big::eb::{self, EbHeader, Payload};

use crate::settings_csv_edit::CsvMemberEdit;

/// Why a member edit was refused (`kind` is stable for the UI).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetMemberRefusal {
    pub kind: &'static str,
    pub reason: String,
}

fn refuse(kind: &'static str, reason: String) -> AssetMemberRefusal {
    AssetMemberRefusal { kind, reason }
}

/// The BIG4 + `.bh` facts of a member that lives in an NHL 07–10 archive (C4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Big4MemberInfo {
    /// The sidecar index relative to the install root (`fe.bh` beside `fe.big`).
    pub sidecar_relative: String,
    /// The `.bh` record's decoded-length word for the new member (RefPack declared length, or 0
    /// for a raw member).
    pub decoded_len: u32,
    /// When set, a replacement that does not fit its slot is refused with this reason instead of
    /// being appended (NHL 07 `dataN.big`: the `.bh` is mirrored inside `boot.big`).
    pub growth_refusal: Option<String>,
    /// The placement this preview predicted and the apply must reproduce: `fit` (the member is
    /// placed in its slot; a copy where it no longer fits is refused rather than grown),
    /// `reflow` (the stock shape: the slot grows by a 64-byte multiple and every later entry
    /// moves up — research 2026-09-20 handoff 03 §2, reproduced byte-for-byte on the six stock
    /// archives), or `append` (only when the gap after the slot carries non-zero bytes a reflow
    /// would erase: appended past the EOF and re-pointed — a shape no stock archive exhibits).
    pub placement: Big4Placement,
    /// NHL 07 `dataN.big` only: the `.bh` is mirrored (RefPack'd, decode-equal) inside `boot.big`
    /// (research 2026-09-20 handoff 03 §4), so a changed sidecar cascades — the re-encoded index
    /// is placed into `boot.big` (fit-in-slot against its verified-zero slack, else ordered
    /// reflow) and `boot.bh` follows, as part of the same copied-target transaction. Bound at
    /// preview to the boot pair's identities; `None` on every other archive.
    pub cascade: Option<Big4Cascade>,
}

/// The NHL 07 two-level cascade a preview bound (see [`Big4MemberInfo::cascade`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Big4Cascade {
    /// `boot.big` / `boot.bh` relative to the install root.
    pub boot_relative: String,
    pub boot_sidecar_relative: String,
    /// The `boot.big` member that mirrors this archive's sidecar (`data0.bh`) and its index.
    pub boot_member_path: String,
    pub boot_member_index: usize,
    /// Stored bytes of that member (hex SHA-256) and the whole `boot.big` / `boot.bh` identities
    /// the apply re-checks before placing anything.
    pub boot_member_stored_sha256: String,
    pub boot_sha256: String,
    pub boot_sidecar_sha256: String,
    /// The member's slot (stored length + verified-zero slack) — the fit-in-slot capacity.
    pub boot_slot_capacity: u64,
}

/// The BIG4 placement a review predicted (see [`Big4MemberInfo::placement`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Big4Placement {
    Fit,
    Reflow,
    Append,
}

impl Big4Placement {
    pub fn as_str(self) -> &'static str {
        match self {
            Big4Placement::Fit => "fit",
            Big4Placement::Reflow => "reflow",
            Big4Placement::Append => "append",
        }
    }
}

/// One held member replacement.
#[derive(Debug, Clone)]
pub struct AssetMemberPlan {
    /// The archive relative to the install root (`databigs/inxp_10.big`).
    pub archive_relative: String,
    pub archive_path: PathBuf,
    pub member_path: String,
    pub entry_index: usize,
    /// What the member decoded as (`gnf` / `png` / …) and its dimensions (unchanged).
    pub container: String,
    pub width: u32,
    pub height: u32,
    /// Other entries sharing the member's stored span; non-empty ⇒ private copy unless
    /// `whole_group`.
    pub shared_with: Vec<String>,
    /// The explicit scope of a shared member: `false` = this member only (a private copy is
    /// appended, the siblings keep the shipped bytes); `true` = the whole deduplicated group
    /// follows the new bytes (in place when they fit, else appended once with every row
    /// re-pointed). Never widened silently — the caller chose it (C7).
    pub whole_group: bool,
    pub placement_prediction: String,
    /// One line for the review / pending card.
    pub description: String,
    /// The member record the copied-target route installs (identities + new bytes).
    pub member: CsvMemberEdit,
    /// `Some` when the archive is a BIG4 + `.bh` pair (NHL 07–10), `None` for EB archives.
    pub big4: Option<Big4MemberInfo>,
    /// Receipt 67 (C37, PS3 board pairs): the loose copy of the same package, written with the member in the **same**
    /// transaction (both outputs persist or both originals stay recoverable). `None` on every other plan.
    pub paired_loose: Option<PairedLoose>,
    /// Receipt 67 (C37, PS3 archive-only titles): the root-relative loose path that must stay **absent** — a loose file of
    /// that name appearing is another topology, refused at the save. `None` on every other plan.
    pub loose_must_be_absent: Option<String>,
    /// Further disclosures for the review (the materials a board raster change affects, identities, runtime caveats).
    pub notes: Vec<String>,
}

/// The loose half of a PS3 board pair (receipt 67).
#[derive(Debug, Clone)]
pub struct PairedLoose {
    /// Root-relative path (`rendering/stadium/stadium_321_container_0.rpsgl`).
    pub relative: String,
    /// The loose file's identity when the plan was made (it equalled the unwrapped member): the save's expected prior.
    pub prior_len: u64,
    pub prior_sha256: String,
    /// The new package bytes (= the member's new unwrapped bytes).
    pub new_bytes: Arc<Vec<u8>>,
}

fn hex(b: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(b.as_ref()))
}

/// Plan the replacement of `member_path` inside `<install_root>/<archive_relative>` with `rgba`
/// (`w`×`h`). Reads the member, rewrites it with the shared texture writer (same container, same
/// dimensions), re-encodes it as stored, and predicts the placement. Nothing is written.
pub fn plan_asset_member_edit(
    game_id: Option<&str>,
    install_root: &Path,
    archive_relative: &str,
    member_path: &str,
    rgba: &[u8],
    w: u32,
    h: u32,
    whole_group: bool,
) -> Result<AssetMemberPlan, AssetMemberRefusal> {
    let archive_path = crate::assets::resolve_install_path(install_root, archive_relative)
        .ok_or_else(|| refuse("invalid", format!("{archive_relative} escapes the install root")))?;
    let io = |e: std::io::Error| refuse("archive-missing", format!("{}: {e}", archive_path.display()));
    let mut file = std::fs::File::open(&archive_path).map_err(io)?;
    let len = file.metadata().map_err(io)?.len();
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).map_err(io)?;
    file.seek(SeekFrom::Start(0)).map_err(io)?;
    if &magic == b"BIG4" {
        return plan_big4_member_edit(game_id, install_root, archive_relative, &archive_path, member_path, rgba, w, h);
    }
    let mut header_buf = vec![0u8; eb::HEADER_LEN];
    file.read_exact(&mut header_buf).map_err(io)?;
    let header = EbHeader::parse(&header_buf).map_err(|e| refuse("parse", format!("{}: {e}", archive_path.display())))?;
    let mut table_buf = vec![0u8; header.table_end()];
    file.seek(SeekFrom::Start(0)).map_err(io)?;
    file.read_exact(&mut table_buf).map_err(io)?;
    let archive = eb::EbArchive::parse(&table_buf).map_err(|e| refuse("parse", format!("{}: {e}", archive_path.display())))?;
    drop(table_buf);

    let mut matches = archive.entries.iter().enumerate().filter(|(_, e)| e.path == member_path);
    let Some((entry_index, entry)) = matches.next() else {
        return Err(refuse("not-found", format!("{} has no member {member_path}", archive_path.display())));
    };
    if matches.next().is_some() {
        return Err(refuse("ambiguous", format!("{} lists {member_path} twice", archive_path.display())));
    }
    let (off, end) = (entry.offset, entry.offset + entry.size as u64);
    let shared_with: Vec<String> = archive
        .entries
        .iter()
        .enumerate()
        .filter(|(i, o)| *i != entry_index && o.offset < end.max(off + 1) && o.offset + (o.size as u64).max(1) > off)
        .map(|(_, o)| o.path.clone())
        .collect();
    let slot_end = {
        let mut starts: Vec<u64> = archive.entries.iter().map(|e| e.offset).collect();
        starts.sort_unstable();
        starts.iter().copied().find(|s| *s >= end).unwrap_or(len)
    };

    let mut raw = vec![0u8; entry.size as usize];
    file.seek(SeekFrom::Start(entry.offset)).map_err(io)?;
    file.read_exact(&mut raw).map_err(io)?;
    // A `chunklzx` member (decoder promoted 21 Sep 2026) is written back through the **store**
    // route of research 2026-09-21 handoff 06: the same `chunklzx` container with LZX uncompressed
    // blocks — codec-preserving, no compression, so the member grows to its plaintext size. The
    // console loader's acceptance of such blocks is a runtime question (card R30-LZX-1); the
    // prediction says so. No `chunkzip` substitution is ever made.
    let stored_form = if eb::is_chunklzx(&raw) {
        "chunklzx-store"
    } else if raw.starts_with(tdb_archive_codec::MAGIC_CHUNKZIP) {
        "chunkzip"
    } else {
        "raw"
    };
    let source_stored_sha256 = hex(&raw);
    let source_stored_len = raw.len();
    let plain = match eb::decode_payload(raw) {
        Ok(Payload::Data(d)) => d,
        Ok(Payload::UnsupportedLzx) => return Err(refuse("member-refused", format!("{member_path}: chunklzx is not decodable"))),
        Err(e) => return Err(refuse("member-refused", format!("{member_path}: {e}"))),
    };
    if tdb_archive_codec::is_refpack(&plain) {
        return Err(refuse("member-refused", format!("{member_path}: a whole-member RefPack wrapper (NHL 07–10 BIG4 archives) is not an EB member route; the BIG4 + `.bh` writer is separate")));
    }
    // A gen7 BIGF package member (Xbox 360 NHL 11 `fe.big`, NHL 12–Legacy / PS3 12–Legacy
    // `cache.big`): the package is rewritten in place by the span-preserving re-plan
    // (`tdb_archive_big::replan`) — the texture member, its size word, the followers' offsets and
    // `size_le` change; header, trailer, other members and the opaque tail stay. A package whose
    // shape the re-plan does not recognise would be *rebuilt* by the generic builder on the loose
    // route; inside an archive that is refused instead (the note names the reason).
    let is_package = plain.starts_with(b"BIGF") || plain.starts_with(b"BIG4");

    let written = crate::assets::replace_texture_in_file(plain.clone(), rgba, w, h, None)
        .map_err(|e| refuse("texture", format!("{member_path}: {e}")))?;
    if is_package && !written.notes.is_empty() {
        return Err(refuse("package-shape", format!("{member_path}: {}", written.notes.join("; "))));
    }
    let new_plain = written.bytes;
    let new_stored = match stored_form {
        "chunkzip" => tdb_archive_codec::compress_chunkzip_native(&new_plain),
        "chunklzx-store" => tdb_archive_codec::compress_chunklzx_store(&new_plain).map_err(|e| refuse("encode", format!("{member_path}: {e}")))?,
        _ => new_plain.clone(),
    };
    if stored_form != "raw" {
        let back = match stored_form {
            "chunkzip" => tdb_archive_codec::decompress_chunkzip(&new_stored),
            _ => tdb_archive_codec::decompress_chunklzx(&new_stored),
        }
        .map_err(|e| refuse("encode", format!("{member_path}: the re-encoded member does not decode: {e}")))?;
        if back != new_plain {
            return Err(refuse("encode", format!("{member_path}: the re-encoded member decodes to different bytes")));
        }
    }
    let slack = slot_end - end;
    if whole_group && !shared_with.is_empty() {
        // The group must share the exact span (the stager refuses otherwise): say so here.
        if let Some(o) = archive.entries.iter().enumerate().find(|(i, o)| *i != entry_index && shared_with.contains(&o.path) && (o.offset != entry.offset || o.size != entry.size)) {
            return Err(refuse("group-irregular", format!("{member_path}: `{}` overlaps this member's block with a different offset or size; a whole-group replacement is not offered on this group", o.1.path)));
        }
    }
    let lzx_note = if stored_form == "chunklzx-store" {
        format!(
            "; stored as chunklzx: written back as LZX uncompressed blocks inside the same container (the store route of research 2026-09-21 handoff 06 — no LZX compressor exists, so the member grows from {source_stored_len} to {} stored bytes); whether the console's decoder accepts uncompressed blocks in this member family is NOT established — card R30-LZX-1",
            new_stored.len()
        )
    } else {
        String::new()
    };
    let placement_prediction = if !shared_with.is_empty() && !whole_group {
        format!(
            "the member shares its stored bytes with {} — a private copy is appended at the end of the archive and only this row is re-pointed ({} stay on the shared bytes)",
            shared_with.len(),
            shared_with.join(", ")
        )
    } else if !shared_with.is_empty() {
        if new_stored.len() as u64 <= entry.size as u64 + slack {
            format!("whole group: {} bytes fit the shared block ({} + {slack} slack) — the block is rewritten in place and all {} paths ({member_path}, {}) show the new image", new_stored.len(), entry.size, shared_with.len() + 1, shared_with.join(", "))
        } else {
            format!("whole group: {} bytes exceed the shared block ({} + {slack} slack) — the new bytes are appended once and all {} paths ({member_path}, {}) are re-pointed at them", new_stored.len(), entry.size, shared_with.len() + 1, shared_with.join(", "))
        }
    } else if new_stored.len() as u64 <= entry.size as u64 + slack {
        format!("{} bytes fit the member's slot ({} + {slack} slack)", new_stored.len(), entry.size)
    } else {
        format!("{} bytes exceed the member's slot ({} + {slack} slack): the member will be appended at the end of the archive and its row re-pointed", new_stored.len(), entry.size)
    } + &lzx_note;
    let description = format!(
        "{archive_relative}: replace {member_path} ({} {w}×{h}, {stored_form}; {} → {} stored bytes)",
        written.container.display(),
        source_stored_len,
        new_stored.len()
    );
    let member = CsvMemberEdit {
        member: member_path.rsplit('/').next().unwrap_or(member_path).to_string(),
        member_path: member_path.to_string(),
        stored_form: stored_form.to_string(),
        source_plain_sha256: hex(&plain),
        source_plain_len: plain.len(),
        source_stored_sha256: Some(source_stored_sha256),
        source_stored_len: Some(source_stored_len),
        new_plain_sha256: hex(&new_plain),
        new_plain: Arc::new(new_plain),
        new_stored_sha256: hex(&new_stored),
        new_stored: Arc::new(new_stored),
        cells: Vec::new(),
        placement_prediction: Some(placement_prediction.clone()),
    };
    Ok(AssetMemberPlan {
        archive_relative: archive_relative.to_string(),
        archive_path,
        member_path: member_path.to_string(),
        entry_index,
        container: written.container.label().to_string(),
        width: w,
        height: h,
        whole_group: whole_group && !shared_with.is_empty(),
        shared_with,
        placement_prediction,
        description,
        member,
        big4: None,
        paired_loose: None,
        loose_must_be_absent: None,
        notes: Vec::new(),
    })
}

/// The sidecar beside a BIG4 archive: `data0.bh` for `data0.big`.
pub fn big4_sidecar_relative(archive_relative: &str) -> String {
    match archive_relative.rsplit_once('.') {
        Some((stem, ext)) if ext.eq_ignore_ascii_case("big") => format!("{stem}.bh"),
        _ => format!("{archive_relative}.bh"),
    }
}

/// Whether this archive's sidecar is mirrored inside `boot.big`: **NHL 07 only** — its
/// `data0.big` … `data5.big` carry RefPack'd copies of their `.bh` inside `boot.big` (research
/// 2026-09-20 handoff 03 §4: 8/8 decode-equal to the loose files, 227–2 036 zero slack bytes after
/// each; NHL 08/09/10 `boot.big` carry none). A `.bh` change there is the two-level cascade
/// [`Big4Cascade`] plans; `growth_refusal` is no longer raised for it.
pub fn big4_sidecar_is_mirrored_in_boot(game_id: Option<&str>, archive_relative: &str) -> bool {
    let name = archive_relative.rsplit(['/', '\\']).next().unwrap_or(archive_relative).to_ascii_lowercase();
    let is_data_n = name.starts_with("data") && name.ends_with(".big") && name[4..name.len() - 4].chars().all(|c| c.is_ascii_digit()) && name.len() > 8;
    is_data_n && game_id == Some("nhl07")
}

/// Kept for callers of the earlier policy: no archive refuses growth by title any more (NHL 07
/// plans its cascade instead), so this is always `None`.
pub fn big4_growth_refusal(_game_id: Option<&str>, _archive_relative: &str) -> Option<String> {
    None
}

/// Bind the NHL 07 cascade for `archive_relative`'s sidecar: `boot.big` + `boot.bh` under the
/// root, the member named like the sidecar (`data0.bh`), decode-equal to the loose sidecar bytes.
fn bind_boot_cascade(install_root: &Path, archive_relative: &str, sidecar: &[u8]) -> Result<Big4Cascade, AssetMemberRefusal> {
    use tdb_archive_big::big4::BoundBig4;
    let boot_relative = "boot.big".to_string();
    let boot_sidecar_relative = "boot.bh".to_string();
    let boot_path = crate::assets::resolve_install_path(install_root, &boot_relative).ok_or_else(|| refuse("invalid", "boot.big escapes the install root".to_string()))?;
    let boot_bh_path = crate::assets::resolve_install_path(install_root, &boot_sidecar_relative).ok_or_else(|| refuse("invalid", "boot.bh escapes the install root".to_string()))?;
    let boot_bh = std::fs::read(&boot_bh_path).map_err(|e| refuse("sidecar-cascade", format!("{archive_relative}: on NHL 07 the `.bh` index is mirrored inside boot.big, so a changed index is placed there too — but {} is not present: {e}", boot_bh_path.display())))?;
    let mut boot = std::io::BufReader::new(std::fs::File::open(&boot_path).map_err(|e| refuse("sidecar-cascade", format!("{archive_relative}: on NHL 07 the `.bh` index is mirrored inside boot.big, so a changed index is placed there too — but {} is not present: {e}", boot_path.display())))?);
    let bound = BoundBig4::bind(&mut boot, &boot_bh).map_err(|e| refuse("parse", format!("{}: {e}", boot_path.display())))?;
    let want = big4_sidecar_relative(archive_relative);
    let want_name = want.rsplit(['/', '\\']).next().unwrap_or(&want).to_ascii_lowercase();
    let Some(entry) = bound.entries.iter().find(|e| e.path.rsplit(['/', '\\']).next().unwrap_or(&e.path).eq_ignore_ascii_case(&want_name)).cloned() else {
        return Err(refuse("sidecar-cascade", format!("{archive_relative}: boot.big carries no member {want_name} to mirror the index into; the measured NHL 07 layout has one (research 2026-09-20 handoff 03 §4) — this install is not in that shape")));
    };
    boot.seek(SeekFrom::Start(entry.offset as u64)).map_err(|e| refuse("archive-missing", e.to_string()))?;
    let mut stored = vec![0u8; entry.size as usize];
    boot.read_exact(&mut stored).map_err(|e| refuse("archive-missing", e.to_string()))?;
    let decoded = if tdb_archive_codec::is_refpack(&stored) { tdb_archive_codec::refpack_decompress(&stored).map_err(|e| refuse("parse", format!("boot.big {}: RefPack: {e}", entry.path)))? } else { stored.clone() };
    if decoded != sidecar {
        return Err(refuse("sidecar-cascade", format!("{archive_relative}: the boot.big copy of {want_name} does not decode to the loose {want_name}; the cascade's precondition (decode-equal copies, measured 8/8) does not hold on this install, so nothing is placed")));
    }
    let old_end = entry.offset as u64 + entry.size as u64;
    let slot_end = bound.slot_end(entry.index);
    let mut gap = vec![0u8; (slot_end - old_end) as usize];
    boot.seek(SeekFrom::Start(old_end)).map_err(|e| refuse("archive-missing", e.to_string()))?;
    boot.read_exact(&mut gap).map_err(|e| refuse("archive-missing", e.to_string()))?;
    let zero_slack = gap.iter().position(|b| *b != 0).unwrap_or(gap.len()) as u64;
    Ok(Big4Cascade {
        boot_relative,
        boot_sidecar_relative,
        boot_member_path: entry.path.clone(),
        boot_member_index: entry.index,
        boot_member_stored_sha256: hex(&stored),
        boot_sha256: hex_bytes(&bound.identity.sha256),
        boot_sidecar_sha256: hex_bytes(&bound.identity.sidecar_sha256),
        boot_slot_capacity: entry.size as u64 + zero_slack,
    })
}

fn hex_bytes(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The BIG4 + `.bh` branch of [`plan_asset_member_edit`] (Xbox 360 NHL 07–10 `data0.big` /
/// `fe.big`, PS3 NHL 08–10): the member is the whole logo package — RefPack-wrapped on the Xbox
/// chain (X-A), raw on PS3 (P-A / P-B) — rewritten by the span-preserving re-plan and re-wrapped
/// as stored; placement predicted against the verified-zero slack the way the BIG4 stager decides
/// it; the `.bh` record and trailer follow at staging.
fn plan_big4_member_edit(
    game_id: Option<&str>,
    install_root: &Path,
    archive_relative: &str,
    archive_path: &Path,
    member_path: &str,
    rgba: &[u8],
    w: u32,
    h: u32,
) -> Result<AssetMemberPlan, AssetMemberRefusal> {
    use tdb_archive_big::big4::BoundBig4;
    let sidecar_relative = big4_sidecar_relative(archive_relative);
    let sidecar_path = crate::assets::resolve_install_path(install_root, &sidecar_relative)
        .ok_or_else(|| refuse("invalid", format!("{sidecar_relative} escapes the install root")))?;
    let sidecar = std::fs::read(&sidecar_path).map_err(|e| refuse("sidecar-missing", format!("{}: the BIG4 archive's `.bh` index is required beside it: {e}", sidecar_path.display())))?;
    let io = |e: std::io::Error| refuse("archive-missing", format!("{}: {e}", archive_path.display()));
    let mut file = std::io::BufReader::new(std::fs::File::open(archive_path).map_err(io)?);
    let bound = BoundBig4::bind(&mut file, &sidecar).map_err(|e| refuse("parse", format!("{}: {e}", archive_path.display())))?;
    let mut matches = bound.entries.iter().filter(|e| e.path.eq_ignore_ascii_case(member_path));
    let Some(entry) = matches.next().cloned() else {
        return Err(refuse("not-found", format!("{} has no member {member_path}", archive_path.display())));
    };
    if matches.next().is_some() {
        return Err(refuse("ambiguous", format!("{} lists {member_path} twice", archive_path.display())));
    }
    if !bound.index_order_is_offset_order {
        // A previous Studio append is in this archive: fine to plan on, but say so.
    }
    let mut raw = vec![0u8; entry.size as usize];
    file.seek(SeekFrom::Start(entry.offset as u64)).map_err(io)?;
    file.read_exact(&mut raw).map_err(io)?;
    let source_stored_sha256 = hex(&raw);
    let source_stored_len = raw.len();
    let wrapped = tdb_archive_codec::is_refpack(&raw);
    let plain = if wrapped {
        tdb_archive_codec::refpack_decompress(&raw).map_err(|e| refuse("member-refused", format!("{member_path}: RefPack: {e}")))?
    } else {
        raw.clone()
    };
    let old_decoded = bound.sidecar.records[entry.index][2];
    if wrapped && old_decoded as usize != plain.len() {
        return Err(refuse("sidecar-mismatch", format!("{member_path}: the `.bh` decoded length {old_decoded} is not the member's decoded length {}", plain.len())));
    }
    if !wrapped && old_decoded != 0 {
        return Err(refuse("sidecar-mismatch", format!("{member_path}: a raw member with a non-zero `.bh` decoded length {old_decoded}")));
    }
    let is_package = plain.starts_with(b"BIGF") || plain.starts_with(b"BIG4");
    let written = crate::assets::replace_texture_in_file(plain.clone(), rgba, w, h, None)
        .map_err(|e| refuse("texture", format!("{member_path}: {e}")))?;
    if is_package && !written.notes.is_empty() {
        return Err(refuse("package-shape", format!("{member_path}: {}", written.notes.join("; "))));
    }
    let new_plain = written.bytes;
    let (new_stored, decoded_len) = if wrapped {
        if new_plain.len() > 0xFF_FFFF {
            return Err(refuse("member-refused", format!("{member_path}: the rewritten member ({} bytes) exceeds the RefPack 3-byte length field", new_plain.len())));
        }
        let s = tdb_archive_codec::refpack_compress(&new_plain);
        let back = tdb_archive_codec::refpack_decompress(&s).map_err(|e| refuse("encode", format!("{member_path}: the re-wrapped member does not decode: {e}")))?;
        if back != new_plain {
            return Err(refuse("encode", format!("{member_path}: the re-wrapped member decodes to different bytes")));
        }
        (s, new_plain.len() as u32)
    } else {
        (new_plain.clone(), 0)
    };
    // Placement as the stager decides it: verified-zero slack up to the next entry.
    let old_end = entry.offset as u64 + entry.size as u64;
    let slot_end = bound.slot_end(entry.index);
    let mut gap = vec![0u8; (slot_end - old_end) as usize];
    file.seek(SeekFrom::Start(old_end)).map_err(io)?;
    file.read_exact(&mut gap).map_err(io)?;
    let zero_slack = gap.iter().position(|b| *b != 0).unwrap_or(gap.len()) as u64;
    let growth_refusal = big4_growth_refusal(game_id, archive_relative);
    let mirrored = big4_sidecar_is_mirrored_in_boot(game_id, archive_relative);
    let unchanged = new_stored == raw && decoded_len == old_decoded;
    // NHL 07: a `.bh` change (any size or decoded-length change) cascades into boot.big; bind the
    // cascade now so the review names it and the apply can re-check the boot pair.
    let cascade = if mirrored && !unchanged && (new_stored.len() != entry.size as usize || decoded_len != old_decoded) {
        Some(bind_boot_cascade(install_root, archive_relative, &sidecar)?)
    } else {
        None
    };
    let fits = entry.offset as u64 + new_stored.len() as u64 <= old_end + zero_slack;
    let gap_all_zero = zero_slack as usize == gap.len();
    let mut placement = Big4Placement::Fit;
    let placement_prediction = if unchanged {
        "the member is unchanged (identical bytes): nothing is written".to_string()
    } else if fits {
        format!("{} bytes fit the member's slot ({} + {zero_slack} verified-zero slack); the directory size word and the `.bh` record are patched, nothing moves", new_stored.len(), entry.size)
    } else if let Some(reason) = &growth_refusal {
        return Err(refuse("growth-refused", format!("{member_path}: the {}-byte replacement does not fit the {}-byte slot (+{zero_slack} verified-zero slack) — {reason}", new_stored.len(), entry.size)));
    } else if gap_all_zero {
        placement = Big4Placement::Reflow;
        let delta = (entry.offset as u64 + new_stored.len() as u64).div_ceil(64) * 64 - slot_end;
        format!("{} bytes exceed the member's slot ({} + {zero_slack} verified-zero slack): ordered reflow — the slot grows by {delta} bytes and every later entry of {archive_relative} moves up by {delta} with its `.bh` record, the header length and the `.bh` trailer following (the stock shape, reproduced byte-for-byte on the six stock archives against the research oracle; loader acceptance is card R2-BIG4-1)", new_stored.len(), entry.size)
    } else {
        placement = Big4Placement::Append;
        format!("{} bytes exceed the member's slot ({} + {zero_slack} verified-zero slack) and the gap after the slot carries non-zero bytes a reflow would erase: the member is appended at the end of {archive_relative} and its directory entry, the header length, the `.bh` record and the `.bh` trailer are re-pointed — the directory's offset order is broken by this entry, a shape no stock archive exhibits (research 02 §4.3; card R2-BIG4-1)", new_stored.len(), entry.size)
    };
    let placement_prediction = match &cascade {
        Some(c) => format!(
            "{placement_prediction}. NHL 07: {sidecar_relative} is mirrored inside {} as `{}` (RefPack'd, decode-equal), so the re-encoded index is placed there too — fit-in-slot when it is at most {} bytes (the member's slot plus its verified-zero slack), else an ordered reflow of {} — and {} follows; decided from the actual re-encoded bytes at save, as one transaction with this archive and its `.bh`",
            c.boot_relative, c.boot_member_path, c.boot_slot_capacity, c.boot_relative, c.boot_sidecar_relative
        ),
        None => placement_prediction,
    };
    let description = format!(
        "{archive_relative} (+ {sidecar_relative}{}): replace {member_path} ({} {w}×{h}, {}; {} → {} stored bytes)",
        if cascade.is_some() { " + boot.big + boot.bh" } else { "" },
        written.container.display(),
        if wrapped { "RefPack" } else { "raw" },
        source_stored_len,
        new_stored.len()
    );
    let member = CsvMemberEdit {
        member: member_path.rsplit('/').next().unwrap_or(member_path).to_string(),
        member_path: entry.path.clone(),
        stored_form: if wrapped { "refpack".to_string() } else { "raw".to_string() },
        source_plain_sha256: hex(&plain),
        source_plain_len: plain.len(),
        source_stored_sha256: Some(source_stored_sha256),
        source_stored_len: Some(source_stored_len),
        new_plain_sha256: hex(&new_plain),
        new_plain: Arc::new(new_plain),
        new_stored_sha256: hex(&new_stored),
        new_stored: Arc::new(new_stored),
        cells: Vec::new(),
        placement_prediction: Some(placement_prediction.clone()),
    };
    Ok(AssetMemberPlan {
        archive_relative: archive_relative.to_string(),
        archive_path: archive_path.to_path_buf(),
        member_path: entry.path.clone(),
        entry_index: entry.index,
        container: written.container.label().to_string(),
        width: w,
        height: h,
        shared_with: Vec::new(),
        whole_group: false,
        placement_prediction,
        description,
        member,
        big4: Some(Big4MemberInfo { sidecar_relative, decoded_len, growth_refusal, placement, cascade }),
        paired_loose: None,
        loose_must_be_absent: None,
        notes: Vec::new(),
    })
}
