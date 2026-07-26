//! Re-inject a (possibly edited) inner `.db` back into its container and recompute both checksums.
//!
//! The new payload is reconstructed as `new_db ++ original_trailer`, so any trailing non-DB bytes the
//! original carried (verified to exist even on ROSTER saves) are **preserved** — a repack is never
//! lossy. Then per variant:
//!
//! * **RawV2** — preserve the 44-byte header, write the payload tight, refresh
//!   `uncompressed_size@0x24`, recompute `checksum_b`→`checksum_a`.
//! * **ZlibV4** — deflate the payload (zlib level 6), preserve the 48-byte header, **zero-pad back to
//!   the original file allocation**, refresh `uncompressed_size@0x24` + `compressed_size@0x2C` (LE),
//!   recompute the checksums.
//! * **RawV3** — gen6 (NHL07-09) ROSTER: preserve the 28-byte header, write the payload tight,
//!   recompute **only** `checksum_a` (no `checksum_b` slot — `0x28` is inside the body).
//! * **RawV1** — NHL10/11: preserve the 32-byte header (including the un-reversed `tag_1c` verbatim),
//!   write the payload tight, recompute **only** `checksum_a`. **Experimental**: `tag_1c` is not
//!   recomputed, so an edited body carries a stale `tag_1c` — on-console load is unverified.

use crate::checksum;
use crate::extract::{inner_db_len, payload};
use crate::header::{ContainerVersion, SaveHeader};
use crate::SaveError;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

fn write_be(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_be_bytes());
}

fn write_le(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

/// Reconstruct the new payload as `new_db` followed by the original's trailing non-DB bytes.
fn rebuild_payload(original_file: &[u8], new_db: &[u8]) -> Result<Vec<u8>, SaveError> {
    let orig = payload(original_file)?;
    let orig_db_len = inner_db_len(&orig)?;
    let mut out = new_db.to_vec();
    out.extend_from_slice(&orig[orig_db_len..]); // preserved trailer (empty for v2)
    Ok(out)
}

/// Repack `new_db` into the same container shape as `original_file`.
pub fn repack_db(original_file: &[u8], new_db: &[u8]) -> Result<Vec<u8>, SaveError> {
    let header = SaveHeader::parse(original_file)?;
    match header.version {
        // Gen6 (RawV3) and NHL10/11 (RawV1) share a single-checksum, tight-body write. RawV1
        // additionally carries `tag_1c` in its 0x20 header, preserved verbatim by copying the
        // original header (its algorithm is un-reversed; see the experimental caveat above).
        ContainerVersion::RawV3 | ContainerVersion::RawV1 => {
            let payload = rebuild_payload(original_file, new_db)?;
            let hlen = header.version.header_len();
            let mut out = original_file[..hlen].to_vec();
            out.extend_from_slice(&payload);
            checksum::recompute_a_only(&mut out);
            Ok(out)
        }

        ContainerVersion::RawV2 => {
            let payload = rebuild_payload(original_file, new_db)?;
            let hlen = ContainerVersion::RawV2.header_len();
            let mut out = original_file[..hlen].to_vec();
            out.extend_from_slice(&payload);
            write_be(&mut out, 0x24, payload.len() as u32);
            checksum::recompute(&mut out);
            Ok(out)
        }

        ContainerVersion::ZlibV4 => {
            let payload = rebuild_payload(original_file, new_db)?;
            let hlen = ContainerVersion::ZlibV4.header_len();
            let original_len = original_file.len();

            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(6));
            encoder
                .write_all(&payload)
                .map_err(|e| SaveError::Deflate(e.to_string()))?;
            let compressed = encoder
                .finish()
                .map_err(|e| SaveError::Deflate(e.to_string()))?;

            if hlen + compressed.len() > original_len {
                return Err(SaveError::DeflatedTooLarge {
                    needed: hlen + compressed.len(),
                    allocated: original_len,
                });
            }

            let mut out = original_file[..hlen].to_vec();
            out.extend_from_slice(&compressed);
            out.resize(original_len, 0); // zero-pad back to the original allocation
            write_be(&mut out, 0x24, payload.len() as u32);
            write_le(&mut out, 0x2C, compressed.len() as u32);
            checksum::recompute(&mut out);
            Ok(out)
        }
    }
}
