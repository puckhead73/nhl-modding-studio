//! PS4 save-data `DATA` container codec (verified read/extract; structural repack).
//!
//! A PS4 save is a directory; the editable payload is the `DATA` file — a stream of records:
//!
//! ```text
//! [u32 LE payload-len] [12-byte header] [payload-len bytes payload]   (repeated)
//! ```
//!
//! The 12-byte header is `<5 bytes> <per-record id> <6-byte tag shared across the save's records>`
//! (its exact byte values vary by title — NHL19 starts `00 70 00…`, NHL16 starts `07 00 00…` — so it
//! is preserved verbatim, never interpreted). One record (the roster) has a payload that is an
//! **LZ4 Frame**, optionally preceded by a `01` codec byte: NHL17–19+ use `[01][frame]`, **NHL16 uses
//! a bare `[frame]`** (no codec byte). The frame decompresses to a **little-endian TDB v6** (the live
//! roster). Other records are small JSON/version blobs. See `docs/formats/ps4-save-container.md`
//! (verified against the nhl19 + nhl16 anchors).
//!
//! Repack preserves every record/header **verbatim** and only swaps the roster record's payload
//! (recompressing the new inner TDB), updating its length prefix. Write-back framing is verified
//! structurally; in-game/real-console validation of the recompressed frame + header tag is pending
//! (the doc marks write-back deferred — no PFD encryption was observed on emulator exports).

use std::io::{Read, Write};

use crate::SaveError;

/// LZ4 Frame magic (`04 22 4D 18`).
const LZ4_MAGIC: [u8; 4] = [0x04, 0x22, 0x4D, 0x18];
/// Codec id byte preceding the LZ4 frame in the roster record's payload.
const CODEC_LZ4: u8 = 0x01;
/// Per-record header length.
const HEADER_LEN: usize = 12;

/// One `DATA` record: its 12-byte header (preserved verbatim) and payload bytes.
struct Record {
    header: [u8; HEADER_LEN],
    payload: Vec<u8>,
}

/// Walk the `DATA` record stream. Cheap — no decompression.
fn parse_records(data: &[u8]) -> Result<Vec<Record>, SaveError> {
    let mut records = Vec::new();
    let mut pos = 0usize;
    while pos < data.len() {
        if pos + 4 + HEADER_LEN > data.len() {
            return Err(SaveError::Ps4("truncated record header".into()));
        }
        let len = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;
        let mut header = [0u8; HEADER_LEN];
        header.copy_from_slice(&data[pos..pos + HEADER_LEN]);
        pos += HEADER_LEN;
        if pos + len > data.len() {
            return Err(SaveError::Ps4("record payload exceeds DATA length".into()));
        }
        records.push(Record {
            header,
            payload: data[pos..pos + len].to_vec(),
        });
        pos += len;
    }
    if records.is_empty() {
        return Err(SaveError::Ps4("empty DATA stream".into()));
    }
    Ok(records)
}

/// How the roster record wraps its LZ4 frame within the payload.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Framing {
    /// Payload is the bare LZ4 frame (NHL16).
    Bare,
    /// Payload is `[01][LZ4 frame]` (NHL17–19+).
    Codec01,
}

impl Framing {
    /// Bytes the frame is offset by within the payload.
    fn prefix_len(self) -> usize {
        match self {
            Framing::Bare => 0,
            Framing::Codec01 => 1,
        }
    }
}

/// Find the roster record — the one whose payload is an LZ4 frame (optionally `01`-prefixed) — and
/// how it is framed (so repack can reproduce the exact wrapping).
fn roster_record(records: &[Record]) -> Option<(usize, Framing)> {
    records.iter().enumerate().find_map(|(i, r)| {
        if r.payload.get(0..4) == Some(&LZ4_MAGIC[..]) {
            Some((i, Framing::Bare))
        } else if r.payload.first() == Some(&CODEC_LZ4) && r.payload.get(1..5) == Some(&LZ4_MAGIC[..]) {
            Some((i, Framing::Codec01))
        } else {
            None
        }
    })
}

fn serialize_records(records: &[Record]) -> Vec<u8> {
    let mut out = Vec::new();
    for r in records {
        out.extend_from_slice(&(r.payload.len() as u32).to_le_bytes());
        out.extend_from_slice(&r.header);
        out.extend_from_slice(&r.payload);
    }
    out
}

/// `true` if `data` parses as a PS4 save stream containing an LZ4 roster record.
pub fn is_ps4_save(data: &[u8]) -> bool {
    parse_records(data)
        .map(|r| roster_record(&r).is_some())
        .unwrap_or(false)
}

/// Extract the inner little-endian roster TDB from a PS4 save `DATA` stream.
pub fn extract_roster(data: &[u8]) -> Result<Vec<u8>, SaveError> {
    let records = parse_records(data)?;
    let (idx, framing) =
        roster_record(&records).ok_or_else(|| SaveError::Ps4("no LZ4 roster record in DATA".into()))?;
    lz4_decompress(&records[idx].payload[framing.prefix_len()..])
}

/// Repack an edited inner roster TDB into the `DATA` stream, preserving every other record and all
/// headers verbatim, and reproducing the roster record's exact framing (bare frame for NHL16, or the
/// `01` codec prefix for NHL17–19+).
pub fn repack_roster(original: &[u8], new_inner: &[u8]) -> Result<Vec<u8>, SaveError> {
    let mut records = parse_records(original)?;
    let (idx, framing) =
        roster_record(&records).ok_or_else(|| SaveError::Ps4("no LZ4 roster record in DATA".into()))?;

    let frame = lz4_compress(new_inner)?;
    let mut payload = Vec::with_capacity(frame.len() + framing.prefix_len());
    if framing == Framing::Codec01 {
        payload.push(CODEC_LZ4);
    }
    payload.extend_from_slice(&frame);
    records[idx].payload = payload;

    Ok(serialize_records(&records))
}

fn lz4_decompress(frame: &[u8]) -> Result<Vec<u8>, SaveError> {
    let mut dec = lz4_flex::frame::FrameDecoder::new(frame);
    let mut out = Vec::new();
    dec.read_to_end(&mut out)
        .map_err(|e| SaveError::Lz4(e.to_string()))?;
    Ok(out)
}

fn lz4_compress(inner: &[u8]) -> Result<Vec<u8>, SaveError> {
    let mut enc = lz4_flex::frame::FrameEncoder::new(Vec::new());
    enc.write_all(inner).map_err(|e| SaveError::Lz4(e.to_string()))?;
    enc.finish().map_err(|e| SaveError::Lz4(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tdb_core::TdbFile;

    fn read_anchor(rel: &str) -> Vec<u8> {
        std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)).unwrap()
    }
    /// NHL19 roster save (`[01][frame]` framing) — the doc's primary verified anchor.
    fn nhl19_data() -> Vec<u8> {
        read_anchor("anchors/ps4/nhl19/savefiles/RosterCustom0/DATA")
    }
    /// NHL16 roster save (bare `[frame]` framing — no codec byte).
    fn nhl16_data() -> Vec<u8> {
        read_anchor("anchors/ps4/nhl16/savefiles/84f8102f - ROSTER/DATA")
    }

    fn assert_extracts_le_roster(data: &[u8]) {
        assert!(is_ps4_save(data));
        let inner = extract_roster(data).unwrap();
        let db = TdbFile::parse(&inner).expect("inner roster parses as TDB");
        assert_eq!(db.endian, tdb_core::Endian::Little, "PS4 inner TDB is little-endian");
        assert!(db.header_crc_ok(), "extracted roster header CRC verifies");
        // A real roster subset (NHL16 ≈ 36 tables, NHL19 ≈ 54), not a stub.
        assert!(db.tables.len() > 20, "roster subset has many tables: {}", db.tables.len());
    }

    #[test]
    fn detects_and_extracts_codec01_framing_nhl19() {
        assert_eq!(roster_record(&parse_records(&nhl19_data()).unwrap()).unwrap().1, Framing::Codec01);
        assert_extracts_le_roster(&nhl19_data());
    }

    #[test]
    fn detects_and_extracts_bare_framing_nhl16() {
        assert_eq!(roster_record(&parse_records(&nhl16_data()).unwrap()).unwrap().1, Framing::Bare);
        assert_extracts_le_roster(&nhl16_data());
    }

    /// Repack preserves other records verbatim, round-trips the inner roster, and keeps the framing
    /// (codec byte presence) — checked for both the `01`-prefixed and bare variants.
    #[test]
    fn repack_round_trips_and_preserves_framing() {
        for data in [nhl19_data(), nhl16_data()] {
            let (_, framing) = roster_record(&parse_records(&data).unwrap()).unwrap();
            let inner = extract_roster(&data).unwrap();

            let repacked = repack_roster(&data, &inner).unwrap();
            assert_eq!(extract_roster(&repacked).unwrap(), inner, "inner roster survives repack");
            assert_eq!(
                roster_record(&parse_records(&repacked).unwrap()).unwrap().1,
                framing,
                "repack preserves the codec framing"
            );

            let orig = parse_records(&data).unwrap();
            let new = parse_records(&repacked).unwrap();
            assert_eq!(orig.len(), new.len());
            let roster = roster_record(&orig).unwrap().0;
            for (i, (a, b)) in orig.iter().zip(new.iter()).enumerate() {
                assert_eq!(a.header, b.header, "record {i} header preserved");
                if i != roster {
                    assert_eq!(a.payload, b.payload, "non-roster record {i} payload preserved");
                }
            }
        }
    }

    #[test]
    fn edited_roster_survives_repack() {
        for data in [nhl19_data(), nhl16_data()] {
            let mut inner = extract_roster(&data).unwrap();
            let probe = inner.len() / 2;
            inner[probe] ^= 0xFF;
            let repacked = repack_roster(&data, &inner).unwrap();
            assert_eq!(extract_roster(&repacked).unwrap(), inner);
        }
    }
}
