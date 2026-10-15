//! Teams 2.7 / Leagues 1.5 (gen7) — the **diagnostic EALayer3 v1 preview** of one speech sample
//! (overnight 2026-09-24, package P4.3; research R77 as narrowed by R79, extended by R80).
//!
//! Inputs: the title, the bank stem (`pbp_eng` play-by-play, `ann_eng` announcer) and a sample id
//! as the commentary / name-call listings show them. The route is offered **only on the measured
//! tuples** ([`tdb_snd::ealayer3::MEASURED`]); everything else refuses with the tuple named. The
//! bank (`audio/speech/<stem>_samples.sbr`, gen7 `SBKR`) gives the sample's `.sbs` offset. Bank and
//! stream are read **loose first**; a bank that ships only inside an audio archive (PS3 09 / 10
//! `data0.big` / `audio.big` / `audioboot.big`, PS3 12 / 13 `nocache.big` / `audioboot.big` — the
//! research R80 sources) is read from the archive's file table and decoded from its stored form
//! (`chunkzip`, RefPack or stored); a stream that ships only as an archive member (X360 15 / Legacy
//! `nocache.big`, PS3 11 `audiostream.big` — R77) is read at the member's offset without copying
//! the archive. The SNS blocks are walked; the granule grammar accepts or refuses by name; the
//! granules are rebuilt into standard frames through the bit reservoir and decoded by Symphonia;
//! the WAV carries the decoded stream **as is** (granules × 576 samples) under the fixed label
//! *"diagnostic preview — start handling unresolved (two warm-up granules + 47 PCM samples; not
//! game-verified)"*. No trimming rule is applied to the start or the end: which samples the game
//! discards is Q-R77-1, so the preview shows everything the stream carries and says so.
//!
//! Music banks (`music_samples`, 44.1 kHz **stereo**, R80) use the same `SBKR` table under
//! `audio/music/` (measured 24 Sep 2026 on the X360 / PS3 NHL 15 files: `SBKR`, 263 entries), so they
//! preview through the same route with two channels; the stereo rebuild carries both channels'
//! side info as read (R80 §2).
//!
//! Xbox 360 NHL 09 / 10 speech and NHL 09–12 music are **EA-XMA** (R84; receipt 33): the SNS
//! blocks are unwrapped into XMA2 packets (`tdb_snd::eaxma`), wrapped as RIFF XMA2 and decoded by the
//! external helper (`tdb_snd::helper`, Studio's own minimal LGPL FFmpeg build — receipt 32). The
//! listing says when no helper is installed and marks nothing previewable then; the preview looks the
//! helper up again (it may have gone since the listing), can be cancelled, and refuses unless the
//! decoded count equals the SNR count.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{DomainError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Gen7SamplePreview {
    pub game_id: String,
    pub platform: String,
    pub bank_stem: String,
    pub bank_path: String,
    pub bank_len: u64,
    pub bank_sha256: String,
    pub stream_path: String,
    pub stream_len: u64,
    pub sample_id: u32,
    pub offset: u64,
    pub sns_len: u64,
    pub sns_sha256: String,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub num_samples: u32,
    pub blocks: usize,
    pub granules: usize,
    pub granules_per_block: Vec<usize>,
    pub rebuilt_frames: usize,
    pub max_main_data_begin: u32,
    pub pcm_frame_samples: u16,
    /// `granules × 576 + 47 − num_samples` (R77: 1 152 + the last block's trim).
    pub surplus_samples: i64,
    pub decoded_samples_per_channel: usize,
    pub first_sample_over_1_lsb: Option<usize>,
    pub label: String,
    /// The EA-Speex route's diagnostic line (frames per block, trims, decoder); `None` on the
    /// EALayer3 route, whose granule fields above carry it.
    pub route_detail: Option<String>,
    pub tuple_source: String,
    pub wav: Vec<u8>,
    pub notes: Vec<String>,
}

/// The EA-Speex route (R83; receipt 29): the refusing grammar, then libspeex UWB with the enhancer
/// on, the 509-sample start and the tail trim dropped → exactly the SNR sample count.
#[allow(clippy::too_many_arguments)]
fn speex_preview(
    game_id: &str,
    platform: crate::Platform,
    bank_stem: &str,
    sample_id: u32,
    tuple: &tdb_snd::ealayer3::Tuple,
    bank_source: &SourceLocation,
    bank_bytes: &[u8],
    stream_source: &SourceLocation,
    stream_len: u64,
    off: u64,
    raw: &[u8],
    snr: &tdb_snd::SnrHeader,
    blocks: &[tdb_snd::Block],
) -> Result<Gen7SamplePreview> {
    let s = tdb_snd::easpeex::parse(snr, blocks).map_err(|e| DomainError::Validation(format!("sample {sample_id}: EA-Speex grammar refused: {e}")))?;
    let (pcm, decoded) = tdb_snd::easpeex::decode(&s).map_err(|e| DomainError::Validation(format!("sample {sample_id}: EA-Speex decode refused: {e}")))?;
    let first = pcm.iter().position(|&x| x.abs() > 1);
    Ok(Gen7SamplePreview {
        game_id: game_id.to_string(),
        platform: platform_key(platform).to_string(),
        bank_stem: bank_stem.to_string(),
        bank_path: bank_source.describe(),
        bank_len: bank_bytes.len() as u64,
        bank_sha256: format!("{:x}", Sha256::digest(bank_bytes)),
        stream_path: stream_source.describe(),
        stream_len,
        sample_id,
        offset: off,
        sns_len: raw.len() as u64,
        sns_sha256: format!("{:x}", Sha256::digest(raw)),
        codec: snr.codec_label().to_string(),
        sample_rate: snr.sample_rate,
        channels: 1,
        num_samples: snr.num_samples,
        blocks: blocks.len(),
        granules: 0,
        granules_per_block: Vec::new(),
        rebuilt_frames: 0,
        max_main_data_begin: 0,
        pcm_frame_samples: 0,
        surplus_samples: 0,
        decoded_samples_per_channel: pcm.len(),
        first_sample_over_1_lsb: first,
        label: tdb_snd::easpeex::LABEL.to_string(),
        route_detail: Some(format!(
            "{} Speex frames ({}) × 640 = {} decoded; first 509 and tail {} dropped → {} samples · libspeex {} ultra-wideband, enhancer on",
            s.frames.len(),
            s.frames_per_block.iter().map(|k| k.to_string()).collect::<Vec<_>>().join("/"),
            decoded,
            s.tail_trim,
            pcm.len(),
            tdb_snd::easpeex::decoder_version()
        )),
        tuple_source: tuple.source.to_string(),
        wav: tdb_snd::wav::encode_pcm16(snr.sample_rate, 1, &pcm),
        notes: vec![
            format!("Measured tuple {} / {} / {} (32 000 Hz mono) from research {}; the block grammar (u8 length | frame, consuming each block) and every count rule accepted the stream before decoding.", game_id, platform_key(platform), bank_stem, tuple.source),
            "Regression: every R83 vector of this family decodes to the declared count and lies within 1 LSB of the libspeex reference path (receipt 29); that is decoder agreement, not a listening or game result.".into(),
            "Replacement is not supported: no encoder, nothing is written.".into(),
        ],
    })
}

fn platform_key(p: crate::Platform) -> &'static str {
    match p {
        crate::Platform::Xbox | crate::Platform::Recomp => "xbox",
        crate::Platform::Ps3 => "ps3",
        crate::Platform::Ps4 => "ps4",
    }
}

fn read_window(path: &Path, off: u64, len: usize) -> Result<Vec<u8>> {
    let mut f = File::open(path).map_err(|e| DomainError::Validation(format!("{}: {e}", path.display())))?;
    f.seek(SeekFrom::Start(off)).map_err(|e| DomainError::Validation(format!("{}: seek {off}: {e}", path.display())))?;
    let mut buf = vec![0u8; len];
    let mut got = 0;
    while got < len {
        let n = f.read(&mut buf[got..]).map_err(|e| DomainError::Validation(format!("{}: read: {e}", path.display())))?;
        if n == 0 {
            break;
        }
        got += n;
    }
    buf.truncate(got);
    Ok(buf)
}

/// Is the gen7 EALayer3 preview offered for this title / platform / bank stem? `Err` names why not.
pub fn coverage(game_id: &str, platform: crate::Platform, bank_stem: &str) -> Result<&'static tdb_snd::ealayer3::Tuple> {
    if !matches!(bank_stem, "pbp_eng" | "ann_eng" | "music_samples") {
        return Err(DomainError::Validation(format!("bank `{bank_stem}`: only `pbp_eng` / `ann_eng` (speech) and `music_samples` (music) are previewed through the SBKR sample table")));
    }
    let key = platform_key(platform);
    let game = if matches!(game_id, "nhllegacy-recompiled" | "legacyrecomp") { "nhllegacy" } else { game_id };
    // EA-Speex (R83; receipt 29) and SNS EA-XMA (R84; receipt 33) tuples — never on Legacy
    // Recompiled, which has no measured vector of either.
    if platform != crate::Platform::Recomp {
        if let Some(t) = tdb_snd::easpeex::measured(key, game, bank_stem) {
            return Ok(t);
        }
        if let Some(t) = tdb_snd::eaxma::measured(key, game, bank_stem) {
            return Ok(t);
        }
    }
    tdb_snd::ealayer3::measured(key, game, bank_stem).ok_or_else(|| {
        DomainError::Validation(format!(
            "{game_id} / {key} / {bank_stem}: not a measured EALayer3 v1 (research R77 / R80), EA-Speex (R83) or EA-XMA (R84) tuple; the codec there is unmeasured or another class — no preview is offered"
        ))
    })
}

/// The audio archives beside a gen7 root that may carry speech banks / streams (research r28 / R80 /
/// R77 sources), in the order the research searched them.
const BANK_ARCHIVES: &[&str] = &["audioboot.big", "audio.big", "data0.big", "audioboot2.big", "nocache.big"];
const STREAM_ARCHIVES: &[&str] = &["nocache.big", "audiostream.big", "data0.big", "audio.big"];

/// Where the bytes of a bank or stream come from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceLocation {
    Loose { path: String, len: u64 },
    /// An archive member: `archive` beside the root, the member path, its stored offset / size and
    /// the stored form the bank was decoded from (`stored`, `chunkzip`, `refpack`).
    ArchiveMember { archive: String, member: String, offset: u64, size: u64, stored_form: String },
}

impl SourceLocation {
    pub fn describe(&self) -> String {
        match self {
            SourceLocation::Loose { path, len } => format!("{path} ({len} B, loose)"),
            SourceLocation::ArchiveMember { archive, member, offset, size, stored_form } => format!("{archive}:{member} (member at {offset}, {size} B, {stored_form})"),
        }
    }
}

/// Find `member_path` in an EB or BIG4 archive: `(offset, size)` of the stored bytes.
pub(crate) fn locate_member(archive: &Path, member_path: &str) -> Option<(u64, u64)> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(archive).ok()?;
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic).ok()?;
    let wanted = member_path.replace('\\', "/").to_ascii_lowercase();
    if &magic == b"EB\0\x03" {
        use tdb_archive_big::eb::{self, EbHeader};
        let mut hb = vec![0u8; eb::HEADER_LEN];
        f.seek(SeekFrom::Start(0)).ok()?;
        f.read_exact(&mut hb).ok()?;
        let header = EbHeader::parse(&hb).ok()?;
        let mut tb = vec![0u8; header.table_end()];
        f.seek(SeekFrom::Start(0)).ok()?;
        f.read_exact(&mut tb).ok()?;
        let table = eb::EbArchive::parse(&tb).ok()?;
        let e = table.entries.iter().find(|e| e.path.replace('\\', "/").to_ascii_lowercase() == wanted)?;
        return Some((e.offset, e.size as u64));
    }
    if &magic == b"BIG4" {
        let sidecar = std::fs::read(archive.with_extension("bh")).ok()?;
        // Directory only: no whole-archive hash per lookup (the checks are bind's).
        let entries = tdb_archive_big::big4::BoundBig4::read_directory(&mut f, &sidecar).ok()?;
        let e = entries.iter().find(|e| e.path.replace('\\', "/").to_ascii_lowercase() == wanted)?;
        return Some((e.offset as u64, e.size as u64));
    }
    None
}

/// Every member of an EB or BIG4 archive as `(path with forward slashes, stored offset, stored
/// size)` — the same tables [`locate_member`] reads. `None` when the archive is neither format or
/// its table (or BIG4 `.bh` sidecar) is unreadable.
pub(crate) fn list_members(archive: &Path) -> Option<Vec<(String, u64, u64)>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(archive).ok()?;
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic).ok()?;
    if &magic == b"EB\0\x03" {
        use tdb_archive_big::eb::{self, EbHeader};
        let mut hb = vec![0u8; eb::HEADER_LEN];
        f.seek(SeekFrom::Start(0)).ok()?;
        f.read_exact(&mut hb).ok()?;
        let header = EbHeader::parse(&hb).ok()?;
        let mut tb = vec![0u8; header.table_end()];
        f.seek(SeekFrom::Start(0)).ok()?;
        f.read_exact(&mut tb).ok()?;
        let table = eb::EbArchive::parse(&tb).ok()?;
        return Some(table.entries.iter().map(|e| (e.path.replace('\\', "/"), e.offset, e.size as u64)).collect());
    }
    if &magic == b"BIG4" {
        let sidecar = std::fs::read(archive.with_extension("bh")).ok()?;
        let entries = tdb_archive_big::big4::BoundBig4::read_directory(&mut f, &sidecar).ok()?;
        return Some(entries.iter().map(|e| (e.path.replace('\\', "/"), e.offset as u64, e.size as u64)).collect());
    }
    None
}

/// The archive `name` beside the root: `<root>/<name>`, or — Legacy Recompiled attached as its install folder — `<root>/game/<name>`
/// (receipt 67; research 2026-09-28 handoff 16 §2, as `gen7_sfx_banks::layout`). Returns the file and its label.
pub(crate) fn archive_beside(install_root: &Path, name: &str) -> Option<(PathBuf, String)> {
    let direct = install_root.join(name);
    if direct.is_file() {
        return Some((direct, name.to_string()));
    }
    let game = install_root.join("game").join(name);
    game.is_file().then(|| (game, format!("game/{name}")))
}

/// One member's stored bytes unwrapped (receipt 67): `stored`, `chunkzip`, `chunklzx` (research 2026-09-28 handoff 16 §2 — never
/// passed on as stored) or `refpack`. `Err` names the member when its wrapper does not decode. `Ok(None)` = not a member.
pub(crate) fn read_member_unwrapped(archive: &Path, member_path: &str) -> std::result::Result<Option<(Vec<u8>, &'static str, u64, u64)>, String> {
    let Some((offset, size)) = locate_member(archive, member_path) else { return Ok(None) };
    let raw = read_window(archive, offset, size as usize).map_err(|e| format!("{}: {member_path}: {e}", archive.display()))?;
    if raw.len() as u64 != size {
        return Err(format!("{}: {member_path}: the member runs past the archive", archive.display()));
    }
    let bad = |form: &str, e: String| format!("{}: {member_path}: {form} member does not decode ({e}); it is not read as stored bytes", archive.display());
    let (bytes, form) = if tdb_archive_big::eb::is_chunklzx(&raw) {
        (tdb_archive_codec::decompress_chunklzx(&raw).map_err(|e| bad("chunklzx", e.to_string()))?, "chunklzx")
    } else if raw.starts_with(tdb_archive_codec::MAGIC_CHUNKZIP) {
        (tdb_archive_codec::decompress_chunkzip(&raw).map_err(|e| bad("chunkzip", e.to_string()))?, "chunkzip")
    } else if tdb_archive_codec::is_refpack(&raw) {
        (tdb_archive_codec::refpack_decompress(&raw).map_err(|e| bad("refpack", e.to_string()))?, "refpack")
    } else {
        (raw, "stored")
    };
    Ok(Some((bytes, form, offset, size)))
}

/// Read and decode a bank that ships inside one of the audio archives beside the root.
fn bank_from_archives(install_root: &Path, member_path: &str) -> std::result::Result<Option<(Vec<u8>, SourceLocation)>, String> {
    for name in BANK_ARCHIVES {
        let Some((archive, label)) = archive_beside(install_root, name) else { continue };
        let Some((bytes, form, offset, size)) = read_member_unwrapped(&archive, member_path)? else { continue };
        return Ok(Some((bytes, SourceLocation::ArchiveMember { archive: label, member: member_path.to_string(), offset, size, stored_form: form.to_string() })));
    }
    Ok(None)
}

/// Locate a stream that ships as an archive member: the file to read and the member's base offset.
fn stream_from_archives(install_root: &Path, member_path: &str) -> Option<(PathBuf, SourceLocation)> {
    for name in STREAM_ARCHIVES {
        let Some((archive, label)) = archive_beside(install_root, name) else { continue };
        if let Some((offset, size)) = locate_member(&archive, member_path) {
            return Some((archive, SourceLocation::ArchiveMember { archive: label, member: member_path.to_string(), offset, size, stored_form: "stored (streamed in place)".to_string() }));
        }
    }
    None
}

/// The bank's directory (relative to the root) and its file stem: speech banks are
/// `audio/speech/<stem>_samples.*`, the music bank is `audio/music/music_samples.*`.
fn bank_files(bank_stem: &str) -> (&'static str, String) {
    if bank_stem == "music_samples" { ("audio/music", "music_samples".to_string()) } else { ("audio/speech", format!("{bank_stem}_samples")) }
}

/// The resolved bank of a measured tuple: where it came from and its `(sample id, .sbs offset)`
/// table — loose first, else decoded from the audio archives. Read-only.
pub fn bank_entries(game_id: &str, platform: crate::Platform, install_root: &Path, bank_stem: &str) -> Result<(SourceLocation, tdb_speech::Bank, Vec<u8>)> {
    coverage(game_id, platform, bank_stem)?;
    let (rel_dir, file_stem) = bank_files(bank_stem);
    let dir: PathBuf = crate::assets::resolve_install_path(install_root, rel_dir).ok_or_else(|| DomainError::Validation(format!("{rel_dir} escapes the install root")))?;
    let bank_path = dir.join(format!("{file_stem}.sbr"));
    let member_sbr = format!("{rel_dir}/{file_stem}.sbr");
    let (bank_bytes, bank_source) = match std::fs::read(&bank_path) {
        Ok(b) => {
            let len = b.len() as u64;
            (b, SourceLocation::Loose { path: bank_path.to_string_lossy().into_owned(), len })
        }
        Err(_) => bank_from_archives(install_root, &member_sbr).map_err(DomainError::Validation)?.ok_or_else(|| {
            DomainError::Validation(format!(
                "{}: not loose, and no `{member_sbr}` member in {} beside the root or under its `game` folder",
                bank_path.display(),
                BANK_ARCHIVES.join(" / ")
            ))
        })?,
    };
    let bank = tdb_speech::parse_bank(&bank_bytes).map_err(|e| DomainError::Validation(format!("{}: {e}", bank_source.describe())))?;
    Ok((bank_source, bank, bank_bytes))
}

/// The bank's stream: `(file to read, base offset, source)` — loose first, else an archive member.
fn resolve_stream(install_root: &Path, bank_stem: &str) -> Result<(PathBuf, u64, SourceLocation)> {
    let (rel_dir, file_stem) = bank_files(bank_stem);
    let dir: PathBuf = crate::assets::resolve_install_path(install_root, rel_dir).ok_or_else(|| DomainError::Validation(format!("{rel_dir} escapes the install root")))?;
    let stream_path = dir.join(format!("{file_stem}.sbs"));
    let member_sbs = format!("{rel_dir}/{file_stem}.sbs");
    match std::fs::metadata(&stream_path) {
        Ok(m) => Ok((stream_path.clone(), 0, SourceLocation::Loose { path: stream_path.to_string_lossy().into_owned(), len: m.len() })),
        Err(_) => {
            let (archive, loc) = stream_from_archives(install_root, &member_sbs).ok_or_else(|| {
                DomainError::Validation(format!("{}: not loose, and no `{member_sbs}` member in {} beside the root", stream_path.display(), STREAM_ARCHIVES.join(" / ")))
            })?;
            let base = match &loc {
                SourceLocation::ArchiveMember { offset, .. } => *offset,
                _ => 0,
            };
            Ok((archive, base, loc))
        }
    }
}

/// One row of a gen7 bank listing: the sample id, its stream offset and the SNR header read there.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Gen7SampleRow {
    pub sample_id: u32,
    pub offset: u64,
    pub codec: Option<String>,
    pub codec_code: Option<u8>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub num_samples: Option<u32>,
    pub problem: Option<String>,
    /// `true` when the header is EALayer3 v1 on this tuple's rate / channel class.
    pub previewable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Gen7BankListing {
    pub game_id: String,
    pub platform: String,
    pub bank_stem: String,
    pub tuple_source: String,
    pub bank: String,
    pub bank_len: u64,
    pub bank_sha256: String,
    pub stream: String,
    pub total: usize,
    pub matching: usize,
    pub truncated: bool,
    pub codec_census: Vec<(String, usize)>,
    pub rows: Vec<Gen7SampleRow>,
    /// Why no row of this bank can be previewed right now although its tuple is measured — the
    /// EA-XMA tuples without an installed XMA2 helper; `None` when previews are available.
    pub preview_unavailable: Option<String>,
    pub notes: Vec<String>,
}

/// List a measured gen7 bank's samples (id filter on the id text, capped) with the SNR header read
/// from each sample's stream position — 16 bytes per row; read-only.
pub fn list_bank_samples(game_id: &str, platform: crate::Platform, install_root: &Path, bank_stem: &str, query: &str, limit: usize) -> Result<Gen7BankListing> {
    let tuple = coverage(game_id, platform, bank_stem)?;
    let (bank_source, bank, bank_bytes) = bank_entries(game_id, platform, install_root, bank_stem)?;
    let (stream_file, stream_base, stream_source) = resolve_stream(install_root, bank_stem)?;
    let codec = tuple_codec(tuple);
    let preview_unavailable = if codec == tdb_snd::eaxma::CODEC { tdb_snd::helper::locate(exe_dir().as_deref()).err().map(|e| e.to_string()) } else { None };
    let q = query.trim();
    let mut samples: Vec<&tdb_speech::BankSample> = bank.samples.iter().filter(|s| q.is_empty() || s.id.to_string().contains(q)).collect();
    samples.sort_by_key(|s| (s.sbs_offset, s.id));
    let matching = samples.len();
    let mut rows = Vec::new();
    let mut census: std::collections::BTreeMap<String, usize> = Default::default();
    for s in samples.iter().take(limit.max(1)) {
        let mut row = Gen7SampleRow { sample_id: s.id, offset: s.sbs_offset as u64, codec: None, codec_code: None, sample_rate: None, channels: None, num_samples: None, problem: None, previewable: false };
        match read_window(&stream_file, stream_base + s.sbs_offset as u64, 16).and_then(|w| {
            if w.len() < 16 || w[0] != 0x48 {
                return Err(DomainError::Validation(format!("no SNS header block at stream offset {}", s.sbs_offset)));
            }
            tdb_snd::SnrHeader::parse(&w[4..12]).ok_or_else(|| DomainError::Validation("short SNR header".into()))
        }) {
            Ok(h) => {
                row.codec = Some(h.codec_label().to_string());
                row.codec_code = Some(h.codec);
                row.sample_rate = Some(h.sample_rate);
                row.channels = Some(h.channels());
                row.num_samples = Some(h.num_samples);
                row.previewable = preview_unavailable.is_none() && h.codec == codec && h.sample_rate == tuple.sample_rate && h.channels() as usize == tuple.channels && !(codec == tdb_snd::eaxma::CODEC && h.looped);
                if codec == tdb_snd::eaxma::CODEC && h.codec == codec && h.looped {
                    row.problem = Some("looping EA-XMA stream — loop handling is not measured; framed only".into());
                }
                *census.entry(h.codec_label().to_string()).or_default() += 1;
            }
            Err(e) => row.problem = Some(e.to_string()),
        }
        rows.push(row);
    }
    Ok(Gen7BankListing {
        game_id: game_id.to_string(),
        platform: platform_key(platform).to_string(),
        bank_stem: bank_stem.to_string(),
        tuple_source: tuple.source.to_string(),
        bank: bank_source.describe(),
        bank_len: bank_bytes.len() as u64,
        bank_sha256: format!("{:x}", Sha256::digest(&bank_bytes)),
        stream: stream_source.describe(),
        total: bank.samples.len(),
        matching,
        truncated: matching > rows.len(),
        codec_census: census.into_iter().collect(),
        rows,
        preview_unavailable,
        notes: vec![
            format!(
                "Measured tuple {} / {} / {} ({} Hz, {} ch) from research {}; codec labels are the community's names for the SNR nibble; only {} rows on this tuple's class preview.",
                game_id,
                platform_key(platform),
                bank_stem,
                tuple.sample_rate,
                tuple.channels,
                tuple.source,
                match codec {
                    0x09 => "EA-Speex",
                    0x03 => "EA-XMA (through the XMA2 helper)",
                    _ => "EALayer3 v1",
                }
            ),
            "Listing and diagnostic preview only: no writer, no encoder, no claim about which sample the game plays.".into(),
        ],
    })
}

/// The SNR codec a measured tuple carries: EA-Speex (`0x09`) for the R83 tuples, EALayer3 v1
/// (`0x05`) otherwise.
fn tuple_codec(t: &tdb_snd::ealayer3::Tuple) -> u8 {
    // By identity within the `static` table (a `const` table has no stable address).
    if tdb_snd::easpeex::MEASURED.iter().any(|m| std::ptr::eq(m, t)) {
        tdb_snd::easpeex::CODEC
    } else if tdb_snd::eaxma::MEASURED.iter().any(|m| std::ptr::eq(m, t)) {
        tdb_snd::eaxma::CODEC
    } else {
        0x05
    }
}

/// The directory of the running executable (where a `helpers` folder may sit).
fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf))
}

/// The EA-XMA route (R84; receipts 30 / 32 / 33): the refusing SNS packet extraction, RIFF XMA2, the
/// helper looked up **now** (not trusted from the listing), a bounded and cancellable decode, and
/// the decoded count checked against the SNR count.
#[allow(clippy::too_many_arguments)]
fn xma_preview(
    game_id: &str,
    platform: crate::Platform,
    bank_stem: &str,
    sample_id: u32,
    tuple: &tdb_snd::ealayer3::Tuple,
    bank_source: &SourceLocation,
    bank_bytes: &[u8],
    stream_source: &SourceLocation,
    stream_len: u64,
    off: u64,
    raw: &[u8],
    snr: &tdb_snd::SnrHeader,
    blocks: &[tdb_snd::Block],
    cancel: Option<&AtomicBool>,
) -> Result<Gen7SamplePreview> {
    let err = |e: String| DomainError::Validation(format!("sample {sample_id}: {e}"));
    if snr.codec != tdb_snd::eaxma::CODEC {
        return Err(err(format!("codec {} is not EA-XMA; this tuple ({}) previews EA-XMA only", snr.codec_label(), tuple.source)));
    }
    if snr.looped {
        return Err(err("looping EA-XMA stream — loop handling is not measured; framed only".into()));
    }
    if snr.sample_rate != tuple.sample_rate || snr.channels() as usize != tuple.channels {
        return Err(err(format!("{} Hz, {} channel(s) — outside the measured tuple ({} Hz, {} ch); not previewed", snr.sample_rate, snr.channels(), tuple.sample_rate, tuple.channels)));
    }
    let p = tdb_snd::eaxma::sns_packets(snr, blocks).map_err(|e| err(format!("EA-XMA packets refused: {e}")))?;
    let ch = snr.channels() as u16;
    let riff = tdb_snd::eaxma::riff_xma2(&p.packets, snr.num_samples, ch, snr.sample_rate);
    let helper = tdb_snd::helper::locate(exe_dir().as_deref()).map_err(|e| err(e.to_string()))?;
    let helper_sha = std::fs::read(&helper).map(|b| format!("{:x}", Sha256::digest(&b))).map_err(|e| err(format!("{}: {e}", helper.display())))?;
    let pcm = tdb_snd::helper::decode_xma2(&helper, &riff, tdb_snd::helper::Limits::default(), cancel).map_err(|e| err(e.to_string()))?;
    let per_channel = pcm.len() / ch as usize;
    if per_channel as u32 != snr.num_samples {
        return Err(err(format!("the helper decoded {per_channel} samples per channel, the SNR header declares {} — refused", snr.num_samples)));
    }
    let first = pcm.iter().position(|&x| x.abs() > 1);
    let helper_name = helper.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(Gen7SamplePreview {
        game_id: game_id.to_string(),
        platform: platform_key(platform).to_string(),
        bank_stem: bank_stem.to_string(),
        bank_path: bank_source.describe(),
        bank_len: bank_bytes.len() as u64,
        bank_sha256: format!("{:x}", Sha256::digest(bank_bytes)),
        stream_path: stream_source.describe(),
        stream_len,
        sample_id,
        offset: off,
        sns_len: raw.len() as u64,
        sns_sha256: format!("{:x}", Sha256::digest(raw)),
        codec: snr.codec_label().to_string(),
        sample_rate: snr.sample_rate,
        channels: ch as u32,
        num_samples: snr.num_samples,
        blocks: blocks.len(),
        granules: 0,
        granules_per_block: Vec::new(),
        rebuilt_frames: 0,
        max_main_data_begin: 0,
        pcm_frame_samples: 0,
        surplus_samples: 0,
        decoded_samples_per_channel: per_channel,
        first_sample_over_1_lsb: first,
        label: tdb_snd::eaxma::LABEL.to_string(),
        route_detail: Some(format!(
            "{} XMA2 packets ({} frames) from {} SNS blocks → RIFF XMA2 → {} ({}…) → {} samples per channel = SNR count",
            p.packet_count,
            p.frame_count_sum,
            blocks.len(),
            helper_name,
            &helper_sha[..12],
            per_channel
        )),
        tuple_source: tuple.source.to_string(),
        wav: tdb_snd::wav::encode_pcm16(snr.sample_rate, ch, &pcm),
        notes: vec![
            format!("Measured tuple {} / {} / {} ({} Hz, {} ch) from research {}; every SNS sub-block's length word and packet header was checked before decoding.", game_id, platform_key(platform), bank_stem, tuple.sample_rate, tuple.channels, tuple.source),
            format!("Decoded by the XMA2 helper {} (SHA-256 {helper_sha}); regression: every R84 vector of this family decodes to its declared count with PCM equal to the research decode (receipts 30 / 32). That is decoder agreement, not a listening or game result.", helper.display()),
            "Replacement is not supported: no encoder, nothing is written.".into(),
        ],
    })
}

/// Decode one gen7 speech sample as a diagnostic preview. Read-only.
pub fn preview_sample(game_id: &str, platform: crate::Platform, install_root: &Path, bank_stem: &str, sample_id: u32) -> Result<Gen7SamplePreview> {
    preview_sample_cancellable(game_id, platform, install_root, bank_stem, sample_id, None)
}

/// [`preview_sample`] with a cancellation flag, honoured by the EA-XMA helper route (the in-process
/// decoders finish in well under a second and do not poll it). Read-only.
pub fn preview_sample_cancellable(game_id: &str, platform: crate::Platform, install_root: &Path, bank_stem: &str, sample_id: u32, cancel: Option<&AtomicBool>) -> Result<Gen7SamplePreview> {
    let tuple = coverage(game_id, platform, bank_stem)?;
    let (bank_source, bank, bank_bytes) = bank_entries(game_id, platform, install_root, bank_stem)?;
    let entry = bank.sample(sample_id).ok_or_else(|| DomainError::Validation(format!("{}: sample {sample_id} is not in the bank ({} samples)", bank_source.describe(), bank.samples.len())))?;
    let (stream_file, stream_base, stream_source) = resolve_stream(install_root, bank_stem)?;
    let stream_len = match &stream_source {
        SourceLocation::Loose { len, .. } => *len,
        SourceLocation::ArchiveMember { size, .. } => *size,
    };
    let off = entry.sbs_offset as u64;
    let window = read_window(&stream_file, stream_base + off, 32 << 20)?;
    let (snr, blocks, end) = tdb_snd::walk_sample(&window, 0).map_err(|e| DomainError::Validation(format!("{} sample {sample_id} at {off}: {e}", stream_source.describe())))?;
    let raw = &window[..end as usize];
    if tuple_codec(tuple) == tdb_snd::easpeex::CODEC {
        return speex_preview(game_id, platform, bank_stem, sample_id, tuple, &bank_source, &bank_bytes, &stream_source, stream_len, off, raw, &snr, &blocks);
    }
    if tuple_codec(tuple) == tdb_snd::eaxma::CODEC {
        return xma_preview(game_id, platform, bank_stem, sample_id, tuple, &bank_source, &bank_bytes, &stream_source, stream_len, off, raw, &snr, &blocks, cancel);
    }
    tdb_snd::ealayer3::check_header(&snr).map_err(|e| DomainError::Validation(format!("sample {sample_id}: {e}; no preview route exists for this codec")))?;
    if snr.sample_rate != tuple.sample_rate || snr.channels() as usize != tuple.channels {
        return Err(DomainError::Validation(format!(
            "sample {sample_id}: {} Hz, {} channel(s) — outside the measured tuple ({} Hz, {} ch); not previewed",
            snr.sample_rate,
            snr.channels(),
            tuple.sample_rate,
            tuple.channels
        )));
    }
    let refs: Vec<(u32, &[u8])> = blocks.iter().map(|b| (b.samples, b.payload.as_slice())).collect();
    let parsed = tdb_snd::ealayer3::contract_parse(&refs, snr.num_samples).map_err(|e| DomainError::Validation(format!("sample {sample_id}: EALayer3 v1 grammar refused: {e}")))?;
    let rebuilt = tdb_snd::ealayer3::rebuild(&parsed.granules).map_err(|e| DomainError::Validation(format!("sample {sample_id}: rebuild refused: {e}")))?;
    let decoded = tdb_snd::decode_layer3(&rebuilt.mpeg).map_err(|e| DomainError::Validation(format!("sample {sample_id}: decoder refused: {e}")))?;
    let per_channel = decoded.pcm.len() / decoded.channels as usize;
    let first = decoded.pcm.iter().position(|&x| x.abs() > 1);
    Ok(Gen7SamplePreview {
        game_id: game_id.to_string(),
        platform: platform_key(platform).to_string(),
        bank_stem: bank_stem.to_string(),
        bank_path: bank_source.describe(),
        bank_len: bank_bytes.len() as u64,
        bank_sha256: format!("{:x}", Sha256::digest(&bank_bytes)),
        stream_path: stream_source.describe(),
        stream_len,
        sample_id,
        offset: off,
        sns_len: raw.len() as u64,
        sns_sha256: format!("{:x}", Sha256::digest(raw)),
        codec: snr.codec_label().to_string(),
        sample_rate: decoded.sample_rate,
        channels: decoded.channels,
        num_samples: snr.num_samples,
        blocks: blocks.len(),
        granules: parsed.granules.len(),
        granules_per_block: parsed.granules_per_block.clone(),
        rebuilt_frames: rebuilt.frames,
        max_main_data_begin: rebuilt.max_main_data_begin,
        pcm_frame_samples: parsed.pcm.pcm_samples,
        surplus_samples: parsed.surplus_samples,
        decoded_samples_per_channel: per_channel,
        first_sample_over_1_lsb: first,
        label: tdb_snd::ealayer3::LABEL.to_string(),
        route_detail: None,
        tuple_source: tuple.source.to_string(),
        wav: tdb_snd::wav::encode_pcm16(decoded.sample_rate, decoded.channels as u16, &decoded.pcm),
        notes: vec![
            format!("Measured tuple {} / {} / {} ({} Hz, {} ch) from research {}; the granule grammar (R77 §1) accepted every block, the granules were rebuilt into {} standard frames through the bit reservoir (max main_data_begin {}) and decoded by Symphonia.", game_id, platform_key(platform), bank_stem, tuple.sample_rate, tuple.channels, tuple.source, rebuilt.frames, rebuilt.max_main_data_begin),
            format!("The WAV carries the whole decoded stream ({} samples per channel = granules × 576); the SNR count is {} and the 47-sample PCM frame is not mixed in — which samples the game discards at the start and end is unresolved (Q-R77-1, card A-EAL3-1).", per_channel, snr.num_samples),
            "Replacement is not supported: no encoder, and the granule / reservoir layout is not rewritten.".into(),
        ],
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_follows_the_measured_tuples_only() {
        assert!(coverage("nhl15", crate::Platform::Xbox, "pbp_eng").is_ok());
        assert!(coverage("nhllegacy-recompiled", crate::Platform::Recomp, "pbp_eng").is_ok(), "Recompiled shares the Legacy Xbox lineage");
        // X360 11 speech is not EALayer3 v1; since receipt 29 it is admitted as EA-Speex (R83).
        assert_eq!(coverage("nhl11", crate::Platform::Xbox, "pbp_eng").unwrap().source, "R83");
        assert_eq!(coverage("nhl14", crate::Platform::Ps3, "pbp_eng").unwrap().source, "R83", "PS3 14 pbp is EA-Speex (codec 0x09), admitted by R83");
        assert!(coverage("nhllegacy-recompiled", crate::Platform::Recomp, "ann_eng").is_err(), "no R83 vector on Recompiled");
        assert!(coverage("nhl14", crate::Platform::Ps3, "ann_eng").is_ok());
        assert!(coverage("nhl15", crate::Platform::Ps4, "pbp_eng").is_err());
        assert!(coverage("nhl15", crate::Platform::Xbox, "music_samples").is_ok(), "music banks share the SBKR table (measured 24 Sep 2026)");
        // X360 09–12 music and 09 / 10 speech are EA-XMA (R80 / R84); admitted since receipt 33.
        assert_eq!(coverage("nhl11", crate::Platform::Xbox, "music_samples").unwrap().source, "R84");
        assert_eq!(coverage("nhl10", crate::Platform::Xbox, "ann_eng").unwrap().source, "R84");
        assert!(coverage("nhl13", crate::Platform::Xbox, "music_samples").unwrap().source == "R80", "X360 13 music stays EALayer3 v1");
        assert_eq!(coverage("nhl10", crate::Platform::Ps3, "music_samples").unwrap().source, "R82", "PS3 10 music is EALayer3 v1 (R82, receipt 36), never EA-XMA");
    }
}
