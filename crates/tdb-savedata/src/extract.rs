//! Unwrap a save container down to its inner payload / TDB.
//!
//! * **RawV1 / RawV2** — the body is the literal payload.
//! * **ZlibV4** — inflate the RFC-1950 zlib stream at `0x30` (the *container* zlib, separate from the
//!   DB-internal Huffman varchar codec).
//!
//! Two layers: [`payload`] returns the full unframed payload (the TDB **plus** any trailing
//! non-DB bytes — verified: even a ROSTER save carries a small trailer the DB's `db_size` excludes),
//! and [`extract_db`] trims it to the inner DB for editing. The writer ([`crate::pack`]) preserves
//! the trailer so a repack is never lossy.

use crate::header::{ContainerVersion, SaveHeader};
use crate::SaveError;
use flate2::read::ZlibDecoder;
use std::io::Read;

/// Reject absurd declared inflate sizes (defensive cap).
const MAX_INFLATE: usize = 128 * 1024 * 1024;

/// The inner TDB length from its own 24-byte header (`DB` magic, `db_size@0x08`).
pub(crate) fn inner_db_len(payload: &[u8]) -> Result<usize, SaveError> {
    if payload.len() < 0x0C || &payload[0..2] != b"DB" {
        return Err(SaveError::NotTdb);
    }
    let big = payload[4] == 1;
    let s = [payload[0x08], payload[0x09], payload[0x0A], payload[0x0B]];
    let len = if big {
        u32::from_be_bytes(s)
    } else {
        u32::from_le_bytes(s)
    } as usize;
    if len < 0x18 || len > payload.len() {
        return Err(SaveError::NotTdb);
    }
    Ok(len)
}

/// The full unframed payload (TDB + any trailing non-DB bytes), decompressed for v4.
pub(crate) fn payload(file: &[u8]) -> Result<Vec<u8>, SaveError> {
    let header = SaveHeader::parse(file)?;
    match header.version {
        ContainerVersion::RawV3 | ContainerVersion::RawV1 | ContainerVersion::RawV2 => {
            Ok(file[header.version.header_len()..].to_vec())
        }
        ContainerVersion::ZlibV4 => {
            let stream = &file[ContainerVersion::ZlibV4.header_len()..];
            let mut out = Vec::with_capacity(header.uncompressed_size.unwrap_or(0) as usize);
            ZlibDecoder::new(stream)
                .take(MAX_INFLATE as u64 + 1)
                .read_to_end(&mut out)
                .map_err(|e| SaveError::Inflate(e.to_string()))?;
            if out.len() > MAX_INFLATE {
                return Err(SaveError::Inflate("inflated stream exceeds cap".into()));
            }
            if let Some(declared) = header.uncompressed_size {
                let diff = (out.len() as i64 - declared as i64).unsigned_abs();
                if diff > 4 {
                    return Err(SaveError::Inflate(format!(
                        "inflated {} bytes, header declares {declared}",
                        out.len()
                    )));
                }
            }
            Ok(out)
        }
    }
}

/// Extract the inner `.db` bytes (the payload trimmed to the DB's own `db_size`).
pub fn extract_db(file: &[u8]) -> Result<Vec<u8>, SaveError> {
    let p = payload(file)?;
    let len = inner_db_len(&p)?;
    let mut p = p;
    p.truncate(len);
    Ok(p)
}
