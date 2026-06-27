//! CRC-32/MPEG-2 — the EA TDB inner-database hash (`crc32_tdb`).
//!
//! Params: poly `0x04C11DB7`, init `0xFFFFFFFF`, no input/output reflection, XorOut 0.
//! Verified against the anchors: `crc32_tdb(file[0x00..0x14])` equals the stored header hash
//! for all 7 PS3 games (and matches the known answer `crc32_tdb("123456789") == 0x0376E6E7`).

const POLY: u32 = 0x04C1_1DB7;

const TABLE: [u32; 256] = build_table();

const fn build_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = (i as u32) << 24;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ POLY
            } else {
                crc << 1
            };
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

/// CRC-32/MPEG-2 over `data` (the EA TDB inner-DB hash).
pub fn crc32_mpeg2(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc = (crc << 8) ^ TABLE[(((crc >> 24) as u8) ^ b) as usize];
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_answer() {
        assert_eq!(crc32_mpeg2(b"123456789"), 0x0376_E6E7);
    }
}
