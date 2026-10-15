//! Receipt 67 — **PS3 board-ad raster replacement** (C37; research 2026-09-28 handoff 10 §5 / §8, accepted by the coordinator's
//! research-receipt-7 review): one board raster of a PS3 venue package replaced by a same-size, same-format DXT1 / DXT3 image,
//! every declared level regenerated, nothing else of the package changed.
//!
//! Topology is per title (handoff 10 §3 census) and decides what one reviewed change writes:
//!
//! | Titles | Package source | Written together |
//! |---|---|---|
//! | PS3 NHL 14 / 15 / Legacy | loose `rendering/stadium/<pkg>` **and** the `cacherender.big` member (a pair) | both, as one transaction |
//! | PS3 NHL 12 / 13 | `cacherender.big` member only | the member; a loose file of that name is refused (another topology) |
//! | PS3 NHL 11 | `stadium.big` member only | as NHL 12 / 13 |
//!
//! A pair must be one package: the unwrapped member equals the loose file byte for byte, else it is refused as diverged. Updating
//! only one copy is never offered (which copy the console loads is unknown). The member keeps its stored form (native-framed
//! chunkzip, re-wrapped; its stored length changes, so the archive always goes through the stager). The plan is an ordinary held
//! [`AssetMemberPlan`] — the pending review, undo and the copied-target save carry it — with the loose half in
//! [`AssetMemberPlan::paired_loose`] and the archive-only absence in [`AssetMemberPlan::loose_must_be_absent`].
//!
//! Refused: every other title / platform (Xbox 360 board gates, archive re-key and resizing stay withheld), DXT5 and any other
//! format, a raster the §8.1 admission does not admit, an image of another size. Nothing here says how the game draws the result.
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;

use sha2::{Digest, Sha256};
use tdb_archive_big::eb::{self, EbHeader, Payload};
use tdb_rx2::texlib::PixelFormat;
use tdb_rx2::venue_board::{self, BoardEdge};

use crate::asset_member_edit::{AssetMemberPlan, AssetMemberRefusal, PairedLoose};
use crate::project::Platform;
use crate::settings_csv_edit::CsvMemberEdit;

fn refuse(kind: &'static str, reason: String) -> AssetMemberRefusal {
    AssetMemberRefusal { kind, reason }
}
fn hex(b: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(b.as_ref()))
}

/// The measured PS3 topology of a title: `(archive, pair)` (handoff 10 §3 / §5).
pub fn ps3_board_topology(game_id: Option<&str>, platform: Platform) -> Result<(&'static str, bool), AssetMemberRefusal> {
    match (platform, game_id) {
        (Platform::Ps3, Some("nhl14" | "nhl15" | "nhllegacy")) => Ok(("cacherender.big", true)),
        (Platform::Ps3, Some("nhl12" | "nhl13")) => Ok(("cacherender.big", false)),
        (Platform::Ps3, Some("nhl11")) => Ok(("stadium.big", false)),
        (Platform::Xbox | Platform::Recomp, _) => Err(refuse(
            "withheld",
            "Xbox 360 board rasters are tiled with a kept packed tail and chunklzx members (research handoff 03 §5, card R30-LZX-1); that writer stays withheld — only PS3 NHL 11–15 / Legacy are admitted".into(),
        )),
        (Platform::Ps3, other) => Err(refuse("unmeasured", format!("PS3 {} board packages were not measured for this writer (research handoff 10 covers NHL 11–15 and Legacy)", other.unwrap_or("(no title)")))),
        (Platform::Ps4, _) => Err(refuse("unsupported", "PS4 arenas have no venue-package board rasters on this route".into())),
    }
}

fn valid_package_name(p: &str) -> bool {
    let Some(rest) = p.strip_prefix("stadium_").and_then(|r| r.strip_suffix(".rpsgl")) else { return false };
    let Some((a, b)) = rest.split_once("_container_") else { return false };
    !a.is_empty() && !b.is_empty() && a.bytes().all(|c| c.is_ascii_digit()) && b.bytes().all(|c| c.is_ascii_digit())
}

/// A planned board replacement: the held member plan (with its loose half or absence binding) and what the review shows.
#[derive(Debug, Clone)]
pub struct BoardPlan {
    pub plan: AssetMemberPlan,
    pub edge: BoardEdge,
    /// Every material of the package whose slot names the raster (all of them change).
    pub materials: Vec<String>,
    /// `pair` or `archive-only`.
    pub topology: &'static str,
}

/// Plan the replacement of raster `descriptor` (the package's `0x20003` chunk index) in `package` of the PS3 install at
/// `install_root` (its `USRDIR`) with `rgba` (`w × h`, the raster's own size). Reads the member (and the loose copy of a pair);
/// writes nothing.
#[allow(clippy::too_many_arguments)]
pub fn plan_ps3_board_replacement(
    game_id: Option<&str>,
    platform: Platform,
    install_root: &Path,
    package: &str,
    descriptor: usize,
    rgba: &[u8],
    w: u32,
    h: u32,
) -> Result<BoardPlan, AssetMemberRefusal> {
    let (archive_relative, pair) = ps3_board_topology(game_id, platform)?;
    if !valid_package_name(package) {
        return Err(refuse("invalid", format!("`{package}` is not a venue package name (stadium_<id>_container_<n>.rpsgl)")));
    }
    let loose_relative = format!("rendering/stadium/{package}");
    let archive_path = crate::assets::resolve_install_path(install_root, archive_relative).ok_or_else(|| refuse("invalid", format!("{archive_relative} escapes the install root")))?;
    let loose_path = crate::assets::resolve_install_path(install_root, &loose_relative).ok_or_else(|| refuse("invalid", format!("{loose_relative} escapes the install root")))?;

    // The member: exactly one entry of that path, its stored bytes and slot.
    let io = |e: std::io::Error| refuse("archive-missing", format!("{}: {e}", archive_path.display()));
    let mut file = std::fs::File::open(&archive_path).map_err(io)?;
    let len = file.metadata().map_err(io)?.len();
    let mut header_buf = vec![0u8; eb::HEADER_LEN];
    file.read_exact(&mut header_buf).map_err(io)?;
    let header = EbHeader::parse(&header_buf).map_err(|e| refuse("parse", format!("{}: {e}", archive_path.display())))?;
    let mut table_buf = vec![0u8; header.table_end()];
    file.seek(SeekFrom::Start(0)).map_err(io)?;
    file.read_exact(&mut table_buf).map_err(io)?;
    let archive = eb::EbArchive::parse(&table_buf).map_err(|e| refuse("parse", format!("{}: {e}", archive_path.display())))?;
    let hits: Vec<(usize, &eb::EbEntry)> = archive.entries.iter().enumerate().filter(|(_, e)| e.path.replace('\\', "/").eq_ignore_ascii_case(&loose_relative)).collect();
    let (entry_index, entry) = match hits.as_slice() {
        [one] => *one,
        [] => return Err(refuse("not-found", format!("{archive_relative} has no member {loose_relative}"))),
        _ => return Err(refuse("ambiguous", format!("{archive_relative} lists {loose_relative} {} times", hits.len()))),
    };
    let (off, end) = (entry.offset, entry.offset + entry.size as u64);
    if archive.entries.iter().enumerate().any(|(i, o)| i != entry_index && o.offset < end.max(off + 1) && o.offset + (o.size as u64).max(1) > off) {
        return Err(refuse("shared", format!("{loose_relative} shares its stored bytes with another member of {archive_relative}; a shared package is not offered")));
    }
    let slot_end = {
        let mut starts: Vec<u64> = archive.entries.iter().map(|e| e.offset).collect();
        starts.sort_unstable();
        starts.into_iter().find(|s| *s >= end).unwrap_or(len)
    };
    let mut raw = vec![0u8; entry.size as usize];
    file.seek(SeekFrom::Start(off)).map_err(io)?;
    file.read_exact(&mut raw).map_err(io)?;
    drop(file);
    let stored_form = if eb::is_chunklzx(&raw) {
        return Err(refuse("member-refused", format!("{loose_relative} is stored as chunklzx in {archive_relative}; PS3 packages are measured as chunkzip / stored and the LZX store route is not admitted here")));
    } else if raw.starts_with(tdb_archive_codec::MAGIC_CHUNKZIP) {
        "chunkzip"
    } else {
        "raw"
    };
    let (source_stored_sha256, source_stored_len) = (hex(&raw), raw.len());
    let plain = match eb::decode_payload(raw) {
        Ok(Payload::Data(d)) => d,
        Ok(Payload::UnsupportedLzx) => return Err(refuse("member-refused", format!("{loose_relative}: undecodable member form"))),
        Err(e) => return Err(refuse("member-refused", format!("{loose_relative}: {e}"))),
    };
    let plain_sha = hex(&plain);

    // Topology: the loose copy is the member (pair) or absent (archive-only).
    let loose_meta = std::fs::symlink_metadata(&loose_path).ok();
    let paired_prior = match (pair, &loose_meta) {
        (true, None) => return Err(refuse("topology", format!("{}: the loose copy of this PS3 pair is missing; updating only the archive member is not offered (which copy the console loads is unknown)", loose_path.display()))),
        (true, Some(m)) if !m.is_file() || m.file_type().is_symlink() => return Err(refuse("topology", format!("{} is not a regular file", loose_path.display()))),
        (true, Some(_)) => {
            let loose = std::fs::read(&loose_path).map_err(|e| refuse("topology", format!("{}: {e}", loose_path.display())))?;
            if loose != plain {
                return Err(refuse(
                    "diverged",
                    format!(
                        "the pair has diverged: {} ({}) is not the unwrapped {archive_relative} member ({plain_sha}); Studio writes a pair only when both copies are one package",
                        loose_path.display(),
                        hex(&loose)
                    ),
                ));
            }
            Some((loose.len() as u64, hex(&loose)))
        }
        (false, Some(_)) => {
            return Err(refuse(
                "topology",
                format!("{} exists, but this title ships the package only inside {archive_relative}; a loose file of that name is another topology and is not written or invented", loose_path.display()),
            ))
        }
        (false, None) => None,
    };

    // Admission (§8.1) and the raster.
    let (edges, refusals) = venue_board::admit(&plain).map_err(|e| refuse("parse", format!("{loose_relative}: {e}")))?;
    let Some(edge) = edges.iter().find(|e| e.descriptor == descriptor).cloned() else {
        let why: Vec<String> = refusals.iter().filter(|(_, r)| r.starts_with(&format!("descriptor {descriptor}:"))).map(|(m, r)| format!("{m}: {r}")).collect();
        return Err(refuse(
            "not-admitted",
            if why.is_empty() {
                format!("raster {descriptor} of {package} is not a board-ad raster the writer admits (no board material names it through a checked edge)")
            } else {
                format!("raster {descriptor} of {package} is refused by the admission check: {}", why.join("; "))
            },
        ));
    };
    if (w, h) != (edge.width, edge.height) {
        return Err(refuse("size", format!("the image is {w} × {h}; the raster is {} × {} — resizing is not offered", edge.width, edge.height)));
    }
    let dds = match edge.format {
        PixelFormat::Dxt1 => tdb_archive_codec::DdsFormat::Dxt1,
        PixelFormat::Dxt3 => tdb_archive_codec::DdsFormat::Dxt3,
        other => return Err(refuse("format", format!("{other:?} is not admitted for writing"))),
    };
    // DXT1: the image is encoded **opaque**. The encoder would otherwise use DXT1's 1-bit punch-through for translucent pixels
    // (decoding as transparent black) — a transparency the board did not declare. DXT3 keeps the image's alpha (explicit 4-bit).
    let dxt1_alpha_dropped = edge.format == PixelFormat::Dxt1 && rgba.chunks(4).any(|p| p[3] != 255);
    let source: std::borrow::Cow<[u8]> = if edge.format == PixelFormat::Dxt1 {
        let mut o = rgba.to_vec();
        o.chunks_mut(4).for_each(|p| p[3] = 255);
        std::borrow::Cow::Owned(o)
    } else {
        std::borrow::Cow::Borrowed(rgba)
    };
    let new_plain = venue_board::replace_chain(&plain, &edge, &source, |_, lw, lh, px| tdb_archive_codec::compress_bc(dds, lw, lh, px)).map_err(|e| refuse("encode", format!("{package}: {e}")))?;
    // Only the admitted span differs (asserted, not assumed).
    if new_plain.len() != plain.len() || new_plain[..edge.start] != plain[..edge.start] || new_plain[edge.start + edge.len..] != plain[edge.start + edge.len..] {
        return Err(refuse("encode", format!("{package}: the rewritten package differs outside raster {descriptor}'s data entry")));
    }
    let new_stored = if stored_form == "chunkzip" { tdb_archive_codec::compress_chunkzip_native(&new_plain) } else { new_plain.clone() };
    if stored_form == "chunkzip" && tdb_archive_codec::decompress_chunkzip(&new_stored).map_err(|e| refuse("encode", format!("{package}: the re-wrapped member does not decode: {e}")))? != new_plain {
        return Err(refuse("encode", format!("{package}: the re-wrapped member decodes to different bytes")));
    }
    let materials = venue_board::materials_using(&plain, descriptor);
    let slack = slot_end - end;
    let placement_prediction = if new_stored.len() as u64 <= entry.size as u64 + slack {
        format!("{} stored bytes fit the member's slot ({} + {slack} slack)", new_stored.len(), entry.size)
    } else {
        format!("{} stored bytes exceed the member's slot ({} + {slack} slack): the member is appended at the end of the archive and its row re-pointed", new_stored.len(), entry.size)
    };
    let format_label = if edge.format == PixelFormat::Dxt1 { "DXT1" } else { "DXT3" };
    let topology = if pair { "pair" } else { "archive-only" };
    let description = format!(
        "{archive_relative}{}: board raster {descriptor} of {package} ({format_label} {w}×{h}, {} levels; {} material(s)): {source_stored_len} → {} stored bytes",
        if pair { format!(" + {loose_relative}") } else { String::new() },
        edge.levels,
        materials.len(),
        new_stored.len()
    );
    let mut notes = vec![
        format!("this raster is used by {}: all of them show the new image", if materials.is_empty() { "no named material".to_string() } else { materials.join(", ") }),
        format!("only data entry {} of the package changes (bytes {}..{} of {}), every declared level regenerated from the image", edge.entry, edge.start, edge.start + edge.len, plain.len()),
        format!("source: {archive_relative}:{loose_relative} stored {stored_form} sha256 {source_stored_sha256} ({source_stored_len} B), unwrapped sha256 {plain_sha}"),
        "which copy the console loads and how the game draws the board are not observed; this is a same-size, same-format DXT replacement only (research handoff 10)".to_string(),
    ];
    if dxt1_alpha_dropped {
        notes.insert(1, "this raster is DXT1: the image's transparency is not carried — it is encoded opaque (no 1-bit punch-through is introduced)".to_string());
    }
    if let Some((l, s)) = &paired_prior {
        notes.insert(2, format!("pair: the loose {loose_relative} ({l} B, sha256 {s}) equals the unwrapped member; both are written as one transaction — both persist, or both originals stay recoverable"));
    } else {
        notes.insert(2, format!("archive-only: {archive_relative} holds the only copy; no loose file is invented, and a loose {loose_relative} appearing before the save refuses it"));
    }
    let member = CsvMemberEdit {
        member: package.to_string(),
        member_path: entry.path.clone(),
        stored_form: stored_form.to_string(),
        source_plain_sha256: plain_sha,
        source_plain_len: plain.len(),
        source_stored_sha256: Some(source_stored_sha256),
        source_stored_len: Some(source_stored_len),
        new_plain_sha256: hex(&new_plain),
        new_plain: Arc::new(new_plain.clone()),
        new_stored_sha256: hex(&new_stored),
        new_stored: Arc::new(new_stored),
        cells: Vec::new(),
        placement_prediction: Some(placement_prediction.clone()),
    };
    let plan = AssetMemberPlan {
        archive_relative: archive_relative.to_string(),
        archive_path,
        member_path: entry.path.clone(),
        entry_index,
        container: format!("ps3-venue-{}", format_label.to_ascii_lowercase()),
        width: w,
        height: h,
        shared_with: Vec::new(),
        whole_group: false,
        placement_prediction,
        description,
        member,
        big4: None,
        paired_loose: paired_prior.map(|(prior_len, prior_sha256)| PairedLoose { relative: loose_relative.clone(), prior_len, prior_sha256, new_bytes: Arc::new(new_plain) }),
        loose_must_be_absent: (!pair).then(|| loose_relative.clone()),
        notes,
    };
    Ok(BoardPlan { plan, edge, materials, topology })
}
