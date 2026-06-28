//! The two verified save-container checksums (spec §3 routines B & C).
//!
//! Both are stored **big-endian** at fixed offsets in every v2/v4 container and were confirmed byte
//! for byte against the real anchors:
//!
//! * [`checksum_a`] @ `0x10` — **CRC-32/IEEE** (reflected) over `file[0x1C..EOF]`.
//! * [`checksum_b`] @ `0x28` — **CRC-32/BZIP2** (non-reflected, final complement) over
//!   `file[0x2C..EOF]`. This differs from `tdb-core`'s `crc32_mpeg2` (the DB-internal CRC, routine A)
//!   *only* by the final complement, so it is derived from it directly.

/// Stored big-endian at this offset.
pub const CHECKSUM_A_OFFSET: usize = 0x10;
/// `checksum_a` covers from here to EOF (includes the stored `checksum_b`).
pub const CHECKSUM_A_REGION_START: usize = 0x1C;
/// Stored big-endian at this offset.
pub const CHECKSUM_B_OFFSET: usize = 0x28;
/// `checksum_b` covers from here to EOF (after both checksum slots).
pub const CHECKSUM_B_REGION_START: usize = 0x2C;

/// CRC-32/IEEE (reflected, the standard zlib CRC) — `checksum_a` over `file[0x1C..]`.
pub fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// `checksum_a` = CRC-32/IEEE over `file[0x1C..]`.
pub fn checksum_a(file: &[u8]) -> u32 {
    crc32_ieee(&file[CHECKSUM_A_REGION_START..])
}

/// `checksum_b` = CRC-32/BZIP2 over `file[0x2C..]` (= `crc32_mpeg2` with the final complement).
pub fn checksum_b(file: &[u8]) -> u32 {
    !tdb_core::crc::crc32_mpeg2(&file[CHECKSUM_B_REGION_START..])
}

/// Recompute and store both checksums in place. Order matters: `checksum_b` first (its region starts
/// after both slots), then `checksum_a` (its region covers the now-written `checksum_b`).
pub fn recompute(file: &mut [u8]) {
    let b = checksum_b(file);
    file[CHECKSUM_B_OFFSET..CHECKSUM_B_OFFSET + 4].copy_from_slice(&b.to_be_bytes());
    let a = checksum_a(file);
    file[CHECKSUM_A_OFFSET..CHECKSUM_A_OFFSET + 4].copy_from_slice(&a.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ieee_known_vector() {
        // Standard CRC-32 of "123456789" is 0xCBF43926.
        assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn bzip2_is_mpeg2_complement() {
        let data = b"the quick brown fox";
        assert_eq!(
            !tdb_core::crc::crc32_mpeg2(data),
            // CRC-32/BZIP2 known relation: complement of MPEG-2 over the same bytes.
            !tdb_core::crc::crc32_mpeg2(data)
        );
        // CRC-32/BZIP2 of "123456789" is 0xFC891918.
        assert_eq!(!tdb_core::crc::crc32_mpeg2(b"123456789"), 0xFC89_1918);
    }
}
