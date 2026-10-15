//! Teams 2.7 / Leagues 1.5 (PS4 NHL 15–21) — the **PS4 speech sample banks** and the **diagnostic
//! EA-MP3 preview** (overnight 2026-09-24, package P4; research R67 / R71 contract rev. 2, R75
//! corrections).
//!
//! What is listed: every `*.sbr` bank in the extracted kettle speech directory
//! (`<nhl>/data/kettle/audio/speech/eng`), its DSET record tables aggregated into one sample list
//! (`.SID`, `.OFF` into the bank's `.sbs`, `.DUR`), and per sample the SNR header of its block
//! stream (codec label, rate, channels, sample count, block count). The codec census is measured
//! from the headers, never assumed from the file name.
//!
//! What is previewed: a codec-`0x0b` sample decoded through `tdb-snd` — the sized-frame grammar
//! (every refusal by name), the concatenated MPEG frames through Symphonia, and the rendering the
//! caller chose: **MPEG only** or **candidate A** (the embedded 1 152 samples replace decoded frame
//! 0). Every preview is labelled *diagnostic, not game-verified*; candidate A is the research's
//! statically supported candidate (20 / 24 vectors), not a listening-verified fact. Other codecs
//! (EA-Opus `0x0c`, `0x09`, …) refuse with their label. Nothing here writes, encodes or claims
//! replacement support.
//!
//! Identity: every reply carries the bank file's length + SHA-256 and the `.sbs` length, so a UI
//! can bind a playing preview to the exact source it came from.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{DomainError, Result};

/// The PS4 speech directory, shared with the context listing.
pub fn ps4_speech_dir(install_root: &Path) -> PathBuf {
    crate::assets::ps4_kettle_root(install_root).join("audio").join("speech").join("eng")
}

/// The PS4 rink-announcer directory (research R83: `rink_announcer/eng/rink_ann_samples[_inxp]`).
pub fn ps4_rink_announcer_dir(install_root: &Path) -> PathBuf {
    crate::assets::ps4_kettle_root(install_root).join("audio").join("rink_announcer").join("eng")
}

/// The rink-announcer banks, admitted for the EA-Speex route (R83 families on PS4 NHL 15–21; receipt
/// 29 regression). They are named by file name like the speech banks and resolve to their own
/// directory.
pub const RINK_ANNOUNCER_BANKS: &[&str] = &["rink_ann_samples.sbr", "rink_ann_samples_inxp.sbr"];

fn is_rink_bank(bank_file: &str) -> bool {
    RINK_ANNOUNCER_BANKS.iter().any(|b| b.eq_ignore_ascii_case(bank_file))
}

/// R93: the NHL 21 base `rink_ann_samples` bank holds one single-block record whose decoded output is
/// near-silent under both reference decoders; the preview discloses it.
const R93_NOTE: &str = "Research R93: this bank's single record (12 frames, one block) decodes to near-silent output under both reference decoders; the cause is unidentified, so this preview says nothing about what the game plays here.";

fn ps4_only(game_id: &str, platform: crate::Platform) -> Result<()> {
    match (game_id, platform) {
        ("nhl15" | "nhl16" | "nhl17" | "nhl18" | "nhl19" | "nhl20" | "nhl21", crate::Platform::Ps4) => Ok(()),
        _ => Err(DomainError::Validation(format!(
            "{game_id} on {platform:?}: the SND10 bank listing covers PS4 NHL 15–21 only (gen7 speech banks are `SBKR` files read by the commentary catalogue; their EALayer3 samples are a different codec)"
        ))),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BankSummary {
    pub file: String,
    pub sbr_len: u64,
    pub sbr_sha256: String,
    pub sbs_present: bool,
    pub sbs_len: Option<u64>,
    pub dsets: usize,
    /// Records carrying both `.SID` and `.OFF` across every DSET.
    pub samples: usize,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BankListing {
    pub game_id: String,
    pub speech_dir: String,
    pub banks: Vec<BankSummary>,
    pub notes: Vec<String>,
}

/// List the banks of the title's speech directory. Reads each `.sbr` in full (they are small);
/// the `.sbs` streams are only stat'ed.
pub fn list_banks(game_id: &str, platform: crate::Platform, install_root: &Path) -> Result<BankListing> {
    ps4_only(game_id, platform)?;
    let dir = ps4_speech_dir(install_root);
    if !dir.is_dir() {
        return Err(DomainError::Validation(format!("{}: no extracted kettle speech directory under this root", dir.display())));
    }
    let mut names: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| DomainError::Validation(format!("{}: {e}", dir.display())))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sbr")))
        .collect();
    // The rink-announcer banks (EA-Speex, R83) sit in their own directory; listed by file name.
    for b in RINK_ANNOUNCER_BANKS {
        let p = ps4_rink_announcer_dir(install_root).join(b);
        if p.is_file() {
            names.push(p);
        }
    }
    names.sort_by_key(|p| p.file_name().map(|n| n.to_ascii_lowercase()));
    let mut banks = Vec::new();
    for p in names {
        let bytes = std::fs::read(&p).map_err(|e| DomainError::Validation(format!("{}: {e}", p.display())))?;
        let sbs = p.with_extension("sbs");
        let sbs_len = std::fs::metadata(&sbs).ok().map(|m| m.len());
        let file = p.file_name().unwrap().to_string_lossy().into_owned();
        match tdb_snd::Sbr::parse(&bytes) {
            Ok(b) => {
                let samples = b.dsets.iter().filter(|d| d.column(".SID").is_some() && d.column(".OFF").is_some()).map(|d| d.records).sum();
                banks.push(BankSummary { file, sbr_len: bytes.len() as u64, sbr_sha256: format!("{:x}", Sha256::digest(&bytes)), sbs_present: sbs_len.is_some(), sbs_len, dsets: b.dsets.len(), samples, note: None });
            }
            Err(e) => banks.push(BankSummary { file, sbr_len: bytes.len() as u64, sbr_sha256: format!("{:x}", Sha256::digest(&bytes)), sbs_present: sbs_len.is_some(), sbs_len, dsets: 0, samples: 0, note: Some(format!("not read: {e}")) }),
        }
    }
    Ok(BankListing {
        game_id: game_id.to_string(),
        speech_dir: dir.to_string_lossy().into_owned(),
        banks,
        notes: vec![
            "SND10 `SBle` banks read by the layout of research probe `snd10_sbr.py` (2026-09-20), as re-used by R61 / R67 / R71; sample counts are records carrying `.SID` and `.OFF`.".into(),
            "The rink-announcer banks (`audio/rink_announcer/eng`) are listed beside the speech banks; their EA-Speex samples preview through pinned libspeex (R83).".into(),
            "Listing and diagnostic preview only: no encoder, no writer, no claim of replacement support (research R67 §4).".into(),
        ],
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SampleRow {
    pub sid: u64,
    pub offset: u64,
    /// `.DUR` seconds × 1000, when the field decodes.
    pub duration_ms: Option<u32>,
    pub codec: Option<String>,
    pub codec_code: Option<u8>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub num_samples: Option<u32>,
    pub blocks: Option<u32>,
    /// Why the header could not be read, when it could not.
    pub problem: Option<String>,
    pub previewable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SampleListing {
    pub bank: BankSummary,
    pub total: usize,
    pub matching: usize,
    pub truncated: bool,
    /// Codec label → count over every record whose header was read.
    pub codec_census: Vec<(String, usize)>,
    pub rows: Vec<SampleRow>,
    pub notes: Vec<String>,
}

fn bank_path(install_root: &Path, bank_file: &str) -> Result<PathBuf> {
    if bank_file.contains(['/', '\\']) || bank_file.contains("..") || !bank_file.to_ascii_lowercase().ends_with(".sbr") {
        return Err(DomainError::Validation(format!("{bank_file}: a bank is named by its `.sbr` file name inside the speech directory")));
    }
    if is_rink_bank(bank_file) {
        return Ok(ps4_rink_announcer_dir(install_root).join(bank_file));
    }
    Ok(ps4_speech_dir(install_root).join(bank_file))
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

fn summary_of(p: &Path, bytes: &[u8], bank: &tdb_snd::Sbr) -> BankSummary {
    let sbs = p.with_extension("sbs");
    let sbs_len = std::fs::metadata(&sbs).ok().map(|m| m.len());
    BankSummary {
        file: p.file_name().unwrap().to_string_lossy().into_owned(),
        sbr_len: bytes.len() as u64,
        sbr_sha256: format!("{:x}", Sha256::digest(bytes)),
        sbs_present: sbs_len.is_some(),
        sbs_len,
        dsets: bank.dsets.len(),
        samples: bank.dsets.iter().filter(|d| d.column(".SID").is_some() && d.column(".OFF").is_some()).map(|d| d.records).sum(),
        note: None,
    }
}

/// All `(sid, off, dur)` records of a bank, in DSET order.
fn records(bank: &tdb_snd::Sbr) -> Vec<(u64, u64, Option<u64>)> {
    let mut out = Vec::new();
    for d in &bank.dsets {
        let (Some(sids), Some(offs)) = (d.column(".SID"), d.column(".OFF")) else { continue };
        let durs = d.column(".DUR");
        for i in 0..d.records.min(sids.len()).min(offs.len()) {
            out.push((sids[i], offs[i], durs.and_then(|v| v.get(i).copied())));
        }
    }
    out
}

/// List a bank's samples with their SNR headers (one 12-byte read per row, up to `limit` rows
/// after the `query` filter on the sample id).
pub fn list_samples(game_id: &str, platform: crate::Platform, install_root: &Path, bank_file: &str, query: &str, limit: usize) -> Result<SampleListing> {
    ps4_only(game_id, platform)?;
    let p = bank_path(install_root, bank_file)?;
    let bytes = std::fs::read(&p).map_err(|e| DomainError::Validation(format!("{}: {e}", p.display())))?;
    let bank = tdb_snd::Sbr::parse(&bytes).map_err(|e| DomainError::Validation(format!("{}: {e}", p.display())))?;
    let summary = summary_of(&p, &bytes, &bank);
    let sbs = p.with_extension("sbs");
    let all = records(&bank);
    let q = query.trim();
    let matching: Vec<&(u64, u64, Option<u64>)> = all.iter().filter(|(sid, _, _)| q.is_empty() || sid.to_string().contains(q)).collect();
    let mut rows = Vec::new();
    let mut census: std::collections::BTreeMap<String, usize> = Default::default();
    for (sid, off, dur) in matching.iter().take(limit.max(1)) {
        let mut row = SampleRow { sid: *sid, offset: *off, duration_ms: dur.map(|d| (tdb_snd::sbr::duration_seconds(d) * 1000.0).round().max(0.0) as u32), codec: None, codec_code: None, sample_rate: None, channels: None, num_samples: None, blocks: None, problem: None, previewable: false };
        if !summary.sbs_present {
            row.problem = Some("the bank's .sbs stream file is missing".into());
        } else {
            match read_window(&sbs, *off, 16).and_then(|w| {
                if w.len() < 16 || w[0] != 0x48 {
                    return Err(DomainError::Validation(format!("no SNS header block at .sbs offset {off}")));
                }
                tdb_snd::SnrHeader::parse(&w[4..12]).ok_or_else(|| DomainError::Validation("short SNR header".into()))
            }) {
                Ok(h) => {
                    row.codec = Some(h.codec_label().to_string());
                    row.codec_code = Some(h.codec);
                    row.sample_rate = Some(h.sample_rate);
                    row.channels = Some(h.channels());
                    row.num_samples = Some(h.num_samples);
                    row.previewable = tdb_snd::eamp3::check_header(&h).is_ok() || (is_rink_bank(bank_file) && h.codec == tdb_snd::easpeex::CODEC && h.sample_rate == tdb_snd::easpeex::SAMPLE_RATE && h.channels() == 1);
                    *census.entry(h.codec_label().to_string()).or_default() += 1;
                }
                Err(e) => row.problem = Some(e.to_string()),
            }
        }
        rows.push(row);
    }
    Ok(SampleListing {
        total: all.len(),
        matching: matching.len(),
        truncated: matching.len() > rows.len(),
        codec_census: census.into_iter().collect(),
        rows,
        bank: summary,
        notes: vec!["Codec labels are the community's names for the SNR codec nibble (research `snd10_sbr.py`); EA-MP3 (0x0b, 48 kHz mono) previews in the speech banks and EA-Speex (0x09, 32 kHz mono) in the rink-announcer banks.".into()],
    })
}

/// Which rendering of the first 24 ms the preview uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreviewRendering {
    MpegOnly,
    CandidateA,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SamplePreview {
    pub bank: BankSummary,
    pub sid: u64,
    pub offset: u64,
    pub sns_len: u64,
    pub sns_sha256: String,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub num_samples: u32,
    pub blocks: usize,
    pub frames: usize,
    pub frames_per_block: Vec<usize>,
    pub trim: usize,
    pub embedded_pcm_samples: usize,
    /// The first decoded sample above 1 LSB (the decoder warm-up zone), when any.
    pub first_sample_over_1_lsb: Option<usize>,
    pub rendering: PreviewRendering,
    /// The label every preview must carry (contract rev. 2).
    pub label: String,
    /// A canonical 16-bit PCM WAV of the rendering, trimmed to `num_samples`.
    pub wav: Vec<u8>,
    pub notes: Vec<String>,
}

/// Decode one sample as a diagnostic preview. Refuses non-EA-MP3 samples by codec label and every
/// grammar / decoder violation by name; writes nothing.
pub fn preview_sample(game_id: &str, platform: crate::Platform, install_root: &Path, bank_file: &str, sid: u64, rendering: PreviewRendering) -> Result<SamplePreview> {
    ps4_only(game_id, platform)?;
    let p = bank_path(install_root, bank_file)?;
    let bytes = std::fs::read(&p).map_err(|e| DomainError::Validation(format!("{}: {e}", p.display())))?;
    let bank = tdb_snd::Sbr::parse(&bytes).map_err(|e| DomainError::Validation(format!("{}: {e}", p.display())))?;
    let summary = summary_of(&p, &bytes, &bank);
    let all = records(&bank);
    let hits: Vec<_> = all.iter().filter(|(s, _, _)| *s == sid).collect();
    let (_, off, _) = match hits.as_slice() {
        [one] => **one,
        [] => return Err(DomainError::Validation(format!("{bank_file}: no sample {sid}"))),
        many => return Err(DomainError::Validation(format!("{bank_file}: sample id {sid} appears {} times; the preview needs one record", many.len()))),
    };
    let sbs = p.with_extension("sbs");
    if !summary.sbs_present {
        return Err(DomainError::Validation(format!("{}: missing", sbs.display())));
    }
    // Speech samples are well under this window; the walk stops at the end block.
    let window = read_window(&sbs, off, 16 << 20)?;
    let (snr, blocks, end) = tdb_snd::walk_sample(&window, 0).map_err(|e| DomainError::Validation(format!("{bank_file} sample {sid} at {off}: {e}")))?;
    let raw = &window[..end as usize];
    if snr.codec == tdb_snd::easpeex::CODEC && is_rink_bank(bank_file) {
        let s = tdb_snd::easpeex::parse(&snr, &blocks).map_err(|e| DomainError::Validation(format!("{bank_file} sample {sid}: EA-Speex grammar refused: {e}")))?;
        let (pcm, decoded) = tdb_snd::easpeex::decode(&s).map_err(|e| DomainError::Validation(format!("{bank_file} sample {sid}: EA-Speex decode refused: {e}")))?;
        let mut notes = vec![
            format!("Diagnostic preview: the EA-Speex block grammar (R83) and every count rule accepted this sample; {} frames × 640 = {decoded} decoded by libspeex {} (ultra-wideband, enhancer on); the first 509 and the tail {} dropped → {} samples.", s.frames.len(), tdb_snd::easpeex::decoder_version(), s.tail_trim, pcm.len()),
            "Regression: every R83 vector of the PS4 rink-announcer families decodes to the declared count within 1 LSB of the libspeex reference path (receipt 29); decoder agreement, not a listening or game result.".into(),
            "Replacement is not supported: no encoder, nothing is written.".into(),
        ];
        if game_id == "nhl21" && bank_file.eq_ignore_ascii_case("rink_ann_samples.sbr") {
            notes.insert(0, R93_NOTE.into());
        }
        return Ok(SamplePreview {
            bank: summary,
            sid,
            offset: off,
            sns_len: raw.len() as u64,
            sns_sha256: format!("{:x}", Sha256::digest(raw)),
            codec: snr.codec_label().to_string(),
            sample_rate: snr.sample_rate,
            channels: 1,
            num_samples: snr.num_samples,
            blocks: blocks.len(),
            frames: s.frames.len(),
            frames_per_block: s.frames_per_block.iter().map(|k| *k as usize).collect(),
            trim: s.tail_trim as usize,
            embedded_pcm_samples: 0,
            first_sample_over_1_lsb: pcm.iter().position(|&x| x.abs() > 1),
            rendering,
            label: tdb_snd::easpeex::LABEL.to_string(),
            wav: tdb_snd::wav::encode_pcm16(snr.sample_rate, 1, &pcm),
            notes,
        });
    }
    tdb_snd::eamp3::check_header(&snr).map_err(|e| DomainError::Validation(format!("{bank_file} sample {sid}: {e}; no preview route exists for this codec")))?;
    let refs: Vec<(u32, &[u8])> = blocks.iter().map(|b| (b.samples, b.payload.as_slice())).collect();
    let parsed = tdb_snd::contract_parse(&refs, snr.num_samples).map_err(|e| DomainError::Validation(format!("{bank_file} sample {sid}: EA-MP3 grammar refused: {e}")))?;
    let decoded = tdb_snd::decode_layer3(&parsed.mpeg).map_err(|e| DomainError::Validation(format!("{bank_file} sample {sid}: decoder refused: {e}")))?;
    let r = match rendering {
        PreviewRendering::MpegOnly => tdb_snd::Rendering::MpegOnly,
        PreviewRendering::CandidateA => tdb_snd::Rendering::CandidateA,
    };
    let pcm = tdb_snd::render_candidate_a(&decoded, &parsed.embedded_pcm, r, snr.num_samples).map_err(|e| DomainError::Validation(format!("{bank_file} sample {sid}: {e}")))?;
    let first = decoded.pcm.iter().position(|&x| x.abs() > 1);
    Ok(SamplePreview {
        bank: summary,
        sid,
        offset: off,
        sns_len: raw.len() as u64,
        sns_sha256: format!("{:x}", Sha256::digest(raw)),
        codec: snr.codec_label().to_string(),
        sample_rate: decoded.sample_rate,
        channels: decoded.channels,
        num_samples: snr.num_samples,
        blocks: blocks.len(),
        frames: parsed.total_frames,
        frames_per_block: parsed.frames_per_block.clone(),
        trim: parsed.trim,
        embedded_pcm_samples: parsed.embedded_pcm.len(),
        first_sample_over_1_lsb: first,
        rendering,
        label: r.label().to_string(),
        wav: tdb_snd::wav::encode_pcm16(decoded.sample_rate, decoded.channels as u16, &pcm),
        notes: vec![
            "Diagnostic preview: the sized-frame grammar (R67) and the reservoir / block-count checks (R71) accepted this sample; the MPEG frames were decoded by Symphonia (pure Rust); nothing here is the game's own output.".into(),
            match rendering {
                PreviewRendering::CandidateA => "Start: the 1 152 embedded samples replace decoded frame 0 (candidate A — statically supported on 20 / 24 research vectors, four inconclusive; not listening-verified, card A-MP3-1).".into(),
                PreviewRendering::MpegOnly => "Start: the decoder's own frame 0 (degraded by the codec delay); the embedded 1 152 samples are not used.".into(),
            },
            "Replacement is not supported: no encoder, and the bit reservoir crosses blocks so a block cannot be spliced.".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_titles_outside_ps4_and_bad_bank_names() {
        assert!(list_banks("nhl15", crate::Platform::Xbox, Path::new("x")).is_err());
        assert!(list_banks("nhl14", crate::Platform::Ps4, Path::new("x")).is_err());
        assert!(bank_path(Path::new("x"), "../evil.sbr").is_err());
        assert!(bank_path(Path::new("x"), "sub/bank.sbr").is_err());
        assert!(bank_path(Path::new("x"), "bank.sbs").is_err());
        assert!(bank_path(Path::new("x"), "pbp_samples.sbr").is_ok());
        assert!(bank_path(Path::new("x"), "rink_ann_samples.sbr").unwrap().to_string_lossy().contains("rink_announcer"));
    }
}
