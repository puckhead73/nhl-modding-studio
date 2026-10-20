//! Re-inject a (possibly edited) inner `.db` back into its container and recompute both checksums.
//!
//! The new payload is reconstructed as `new_db ++ original_trailer`, so any trailing non-DB bytes the
//! original carried (verified to exist even on ROSTER saves) are **preserved** — a repack is never
//! lossy. Then per variant:
//!
//! * **RawV2** — preserve the 44-byte header, write the payload tight, refresh
//!   `uncompressed_size@0x24`, recompute `checksum_b`→`checksum_a`.
//! * **ZlibV4** — deflate the payload exactly as the games do (real zlib, level 6, memLevel 9),
//!   preserve the 48-byte header, **zero-pad the payload region** (`0x2C + uncompressed_size`,
//!   which follows the new payload size on a roster save and stays as declared on a mode save),
//!   append a mode save's sections verbatim, refresh `uncompressed_size@0x24` +
//!   `compressed_size@0x2C` (LE), recompute the checksums.
//! * **RawV3** — gen6 (NHL07-09) ROSTER: preserve the 28-byte header, write the payload tight,
//!   recompute **only** `checksum_a` (no `checksum_b` slot — `0x28` is inside the body).
//! * **RawV1** — NHL10/11: preserve the 32-byte header (including the un-reversed `tag_1c` verbatim),
//!   write the payload tight, recompute **only** `checksum_a`. **Experimental**: `tag_1c` is not
//!   recomputed, so an edited body carries a stale `tag_1c` — on-console load is unverified.

use crate::checksum;
use crate::extract::{inner_db_len, payload};
use crate::header::{ContainerVersion, SaveHeader};
use crate::SaveError;

/// Deflate `payload` exactly as the games do (measured 2026-10-19 on every v4 roster anchor,
/// PS3 and Xbox 360, NHL 14 / 15 / Legacy): a zlib stream at level 6, window 15, **memLevel 9**,
/// default strategy. With real zlib these parameters reproduce the games' streams byte for byte;
/// memLevel 9 is why this goes through `deflateInit2_` rather than `flate2` (which pins 8).
fn zlib_deflate_as_game(payload: &[u8]) -> Result<Vec<u8>, SaveError> {
    use libz_sys::*;
    use std::ffi::{c_char, c_int};
    let fail = |what: &str, code: c_int| SaveError::Deflate(format!("{what} returned {code}"));
    // zlib's allocator hooks: the bindings declare them non-nullable, so the standard Rust
    // allocator is handed over with the allocation size kept in a 16-byte header (zfree gets no
    // size).
    unsafe extern "C" fn zalloc(_: voidpf, items: uInt, size: uInt) -> voidpf {
        let total = (items as usize).saturating_mul(size as usize).saturating_add(16);
        let Ok(layout) = std::alloc::Layout::from_size_align(total, 16) else {
            return std::ptr::null_mut();
        };
        // SAFETY: a non-zero-sized, 16-aligned layout; the size is stored in the header the
        // matching zfree reads back.
        unsafe {
            let p = std::alloc::alloc(layout);
            if p.is_null() {
                return std::ptr::null_mut();
            }
            (p as *mut usize).write(total);
            p.add(16) as voidpf
        }
    }
    unsafe extern "C" fn zfree(_: voidpf, address: voidpf) {
        if address.is_null() {
            return;
        }
        // SAFETY: `address` came from `zalloc` above, so the header 16 bytes before it holds the
        // layout size.
        unsafe {
            let p = (address as *mut u8).sub(16);
            let total = (p as *mut usize).read();
            std::alloc::dealloc(p, std::alloc::Layout::from_size_align_unchecked(total, 16));
        }
    }
    // SAFETY: the stream is zero-initialised except for the allocator hooks set before use;
    // every pointer handed to zlib stays valid for the call it is used in, and the output buffer
    // is sized by deflateBound and only its filled prefix is read back.
    unsafe {
        let mut uninit = std::mem::MaybeUninit::<z_stream>::zeroed();
        let s = uninit.as_mut_ptr();
        std::ptr::addr_of_mut!((*s).zalloc).write(zalloc);
        std::ptr::addr_of_mut!((*s).zfree).write(zfree);
        let mut strm = uninit.assume_init();
        let version = zlibVersion();
        let rc = deflateInit2_(
            &mut strm,
            6,
            Z_DEFLATED,
            15,
            9,
            Z_DEFAULT_STRATEGY,
            version as *const c_char,
            std::mem::size_of::<z_stream>() as c_int,
        );
        if rc != Z_OK {
            return Err(fail("deflateInit2", rc));
        }
        let bound = deflateBound(&mut strm, payload.len() as uLong) as usize;
        let mut out = vec![0u8; bound];
        strm.next_in = payload.as_ptr() as *mut _;
        strm.avail_in = payload.len() as uInt;
        strm.next_out = out.as_mut_ptr();
        strm.avail_out = bound as uInt;
        let rc = deflate(&mut strm, Z_FINISH);
        let produced = strm.total_out as usize;
        deflateEnd(&mut strm);
        if rc != Z_STREAM_END {
            return Err(fail("deflate(Z_FINISH)", rc));
        }
        out.truncate(produced);
        Ok(out)
    }
}

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
            // `uncompressed_size` declares the database alone; a mode save's further sections
            // follow it (inside `rebuild_payload`'s preserved trailer) and are covered only by
            // `checksum_a`.
            let payload = rebuild_payload(original_file, new_db)?;
            let hlen = ContainerVersion::RawV2.header_len();
            let mut out = original_file[..hlen].to_vec();
            out.extend_from_slice(&payload);
            write_be(&mut out, 0x24, new_db.len() as u32);
            checksum::recompute(&mut out);
            Ok(out)
        }

        ContainerVersion::ZlibV4 => {
            // Layout (measured on every v4 anchor): header, the zlib stream, zero padding up to
            // `0x2C + uncompressed_size` (the payload region, also `checksum_b`'s scope), then —
            // mode saves only — further sections verbatim. A roster save is exactly
            // `0x2C + uncompressed_size` long with the region equal to its inflated payload, so
            // its allocation follows the new payload size. A mode save declares a region of
            // half its inflated payload (meaning unknown), so its region is kept as the game
            // wrote it and the stream must still fit inside it.
            let payload = rebuild_payload(original_file, new_db)?;
            let hlen = ContainerVersion::ZlibV4.header_len();
            let tail_start = checksum::checksum_b_region_end(original_file);
            let tail = &original_file[tail_start..];
            let declared = header.uncompressed_size.unwrap_or(0) as usize;
            let region_end = if tail.is_empty() {
                checksum::CHECKSUM_B_REGION_START + payload.len()
            } else {
                tail_start
            };
            let declared = if tail.is_empty() { payload.len() } else { declared };

            let compressed = zlib_deflate_as_game(&payload)?;

            if hlen + compressed.len() > region_end {
                return Err(SaveError::DeflatedTooLarge {
                    needed: hlen + compressed.len(),
                    allocated: region_end,
                });
            }

            let mut out = original_file[..hlen].to_vec();
            out.extend_from_slice(&compressed);
            out.resize(region_end, 0); // zero-pad the payload region
            out.extend_from_slice(tail);
            write_be(&mut out, 0x24, declared as u32);
            write_le(&mut out, 0x2C, compressed.len() as u32);
            checksum::recompute(&mut out);
            Ok(out)
        }
    }
}
