//! D4 / P1-F — the native-framed `chunkzip` encoder: framing rules on synthetic inputs (chunk
//! count, prefix alignment, store-if-not-smaller, no trailing bytes, decode equality) and, with
//! private fixtures (`--ignored`), byte-for-byte reproduction of EA's stored members: the 65 NHL 16–21
//! `setting_csv` members (53 chunkzip + 12 raw) through the manifest the fixture extraction wrote,
//! and the two NHL 21 `overlays_21.big` members whose last chunk is stored. Byte identity is the
//! reproduction target, not a loader rule; the write path relies on framing + decode equality.

use std::path::PathBuf;

use sha2::{Digest, Sha256};
use tdb_archive_codec::{compress_chunkzip, compress_chunkzip_native, decompress_chunkzip, is_native_chunkzip_framing, CHUNKZIP_NATIVE_BACKEND, CHUNKZIP_NATIVE_WINDOW, MAGIC_CHUNKZIP};

fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}

fn be(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// Deterministic pseudo-random bytes (incompressible) without a dependency.
fn noise(n: usize, seed: u64) -> Vec<u8> {
    let mut x = seed | 1;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 24) as u8
        })
        .collect()
}

fn text(n: usize) -> Vec<u8> {
    let line = b"3,72,1,4,9,1,3\r\n";
    line.iter().cycle().take(n).copied().collect()
}

#[test]
fn native_framing_header_chunk_count_alignment_and_no_trailing_bytes() {
    for (n, expect_chunks) in [(0usize, 0usize), (1, 1), (100, 1), (CHUNKZIP_NATIVE_WINDOW - 1, 1), (CHUNKZIP_NATIVE_WINDOW, 1), (CHUNKZIP_NATIVE_WINDOW + 1, 2), (3 * CHUNKZIP_NATIVE_WINDOW + 17, 4)] {
        let plain = text(n);
        let stored = compress_chunkzip_native(&plain);
        assert_eq!(&stored[..8], MAGIC_CHUNKZIP);
        assert_eq!((be(&stored, 8), be(&stored, 12) as usize, be(&stored, 16) as usize, be(&stored, 20) as usize, be(&stored, 24)), (2, n, CHUNKZIP_NATIVE_WINDOW, expect_chunks, 0x10), "n = {n}");
        assert!(stored[28..40].iter().all(|b| *b == 0));
        is_native_chunkzip_framing(&stored).unwrap_or_else(|e| panic!("n = {n}: {e}"));
        // Walk the chunks: each prefix at ≡ 8 (mod 16), flag 1 (compressible text), ends exactly.
        let mut off = 40;
        for i in 0..expect_chunks {
            while off % 16 != 8 {
                assert_eq!(stored[off], 0, "padding byte");
                off += 1;
            }
            assert_eq!(off % 16, 8, "chunk {i} prefix alignment");
            let zsize = be(&stored, off) as usize;
            // A tiny chunk (1 byte of input, or the 1-byte tail of n = window + 1) deflates to more
            // than its length → stored by the rule; a longer text chunk deflates.
            let chunk_len = (n - i * CHUNKZIP_NATIVE_WINDOW).min(CHUNKZIP_NATIVE_WINDOW);
            let flag = be(&stored, off + 4);
            assert!(flag == 1 || flag == 4, "chunk {i} flag {flag} (n = {n})");
            if chunk_len >= 64 {
                assert_eq!(flag, 1, "chunk {i} of repeating text deflates (n = {n})");
            }
            off += 8 + zsize;
        }
        assert_eq!(off, stored.len(), "no trailing bytes (n = {n})");
        if n > 0 {
            assert_eq!(decompress_chunkzip(&stored).unwrap(), plain, "round trip n = {n}");
        }
    }
}

#[test]
fn incompressible_chunks_are_stored_with_flag_4_and_still_decode() {
    let plain = noise(CHUNKZIP_NATIVE_WINDOW + 5000, 7);
    let stored = compress_chunkzip_native(&plain);
    is_native_chunkzip_framing(&stored).unwrap();
    assert_eq!(be(&stored, 20), 2);
    // First chunk: stored (zsize == window, flag 4) — deflate of noise is never shorter.
    assert_eq!((be(&stored, 40) as usize, be(&stored, 44)), (CHUNKZIP_NATIVE_WINDOW, 4));
    let second = 40 + 8 + CHUNKZIP_NATIVE_WINDOW;
    let second = (second - 8 + 15) / 16 * 16 + 8;
    assert_eq!((be(&stored, second) as usize, be(&stored, second + 4)), (5000, 4));
    assert_eq!(stored.len(), second + 8 + 5000);
    assert_eq!(decompress_chunkzip(&stored).unwrap(), plain);
    // A mixed input: compressible first chunk, noisy second.
    let mut mixed = text(CHUNKZIP_NATIVE_WINDOW);
    mixed.extend(noise(3000, 9));
    let stored = compress_chunkzip_native(&mixed);
    is_native_chunkzip_framing(&stored).unwrap();
    assert_eq!(be(&stored, 44), 1);
    let z0 = be(&stored, 40) as usize;
    let p1 = (40 + 8 + z0 - 8 + 15) / 16 * 16 + 8;
    assert_eq!(be(&stored, p1 + 4), 4);
    assert_eq!(decompress_chunkzip(&stored).unwrap(), mixed);
}

#[test]
fn the_framing_check_rejects_the_legacy_single_chunk_layout_and_tampering() {
    let plain = text(300_000);
    // The pre-existing substitution encoder: single chunk, window word = fullsize → not native.
    let legacy = compress_chunkzip(&plain);
    let e = is_native_chunkzip_framing(&legacy).unwrap_err();
    assert!(e.contains("window"), "{e}");
    assert_eq!(decompress_chunkzip(&legacy).unwrap(), plain, "the legacy output still decodes");
    let native = compress_chunkzip_native(&plain);
    is_native_chunkzip_framing(&native).unwrap();
    // Trailing bytes.
    let mut t = native.clone();
    t.extend_from_slice(&[0, 0, 0, 0]);
    assert!(is_native_chunkzip_framing(&t).unwrap_err().contains("trailing"));
    // Non-zero padding.
    let mut p = native.clone();
    let z0 = be(&p, 40) as usize;
    let pad_at = 48 + z0;
    if pad_at % 16 != 8 {
        p[pad_at] = 1;
        assert!(is_native_chunkzip_framing(&p).unwrap_err().contains("padding"));
    }
    // Wrong chunk count word.
    let mut c = native.clone();
    c[20..24].copy_from_slice(&1u32.to_be_bytes());
    assert!(is_native_chunkzip_framing(&c).unwrap_err().contains("chunks"));
    // A foreign flag.
    let mut f = native.clone();
    f[44..48].copy_from_slice(&7u32.to_be_bytes());
    assert!(is_native_chunkzip_framing(&f).unwrap_err().contains("flag"));
    assert!(is_native_chunkzip_framing(b"chunklzx").is_err());
}

#[test]
fn the_backend_is_recorded() {
    assert!(CHUNKZIP_NATIVE_BACKEND.contains("zlib 1.3.2") && CHUNKZIP_NATIVE_BACKEND.contains("level 9"));
}

// ---------------------------------------------------------------------------------------------
// Private fixtures (--ignored): reproduction of EA's stored bytes
// ---------------------------------------------------------------------------------------------

fn fixture_root() -> PathBuf {
    PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the private chunkzip fixtures cannot be resolved"))
}

/// The 65 `setting_csv` members of the six NHL 16–21 installs: plain text under
/// `databases/<title>-ps4-<cusa>/inxp_00.big/setting_csv/`, identities in
/// `databases/ps4-setting-csv-manifest.json` (archive, entry, stored form, stored/plain SHA-256).
#[test]
#[ignore = "private fixtures databases/ps4-setting-csv-manifest.json + databases/nhl{16..21}-ps4-*/inxp_00.big/setting_csv: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn every_nhl16_to_21_setting_csv_member_re_encodes_to_ea_s_stored_bytes() {
    let root = fixture_root();
    let manifest_path = root.join("databases/ps4-setting-csv-manifest.json");
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest_path).unwrap_or_else(|e| panic!("cannot read {}: {e}", manifest_path.display()))).unwrap();
    let mut identical = 0;
    let mut raw = 0;
    let mut total = 0;
    let dirs = [("nhl16", "nhl16-ps4-cusa01909"), ("nhl17", "nhl17-ps4-cusa04326"), ("nhl18", "nhl18-ps4-cusa07569"), ("nhl19", "nhl19-ps4-cusa11126"), ("nhl20", "nhl20-ps4-cusa15439"), ("nhl21", "nhl21-ps4-cusa18182")];
    for (title, dir) in dirs {
        let members = manifest[title]["members"].as_array().unwrap_or_else(|| panic!("{title}: no members in the manifest"));
        for m in members {
            total += 1;
            let path = m["path"].as_str().unwrap();
            let name = path.rsplit('/').next().unwrap();
            let plain_path = root.join("databases").join(dir).join("inxp_00.big/setting_csv").join(name);
            let plain = std::fs::read(&plain_path).unwrap_or_else(|e| panic!("cannot read {}: {e}", plain_path.display()));
            assert!(!plain.is_empty(), "{}", plain_path.display());
            assert_eq!(sha(&plain), m["plain_sha256"].as_str().unwrap(), "{title} {name}: plain identity");
            let stored_sha = m["stored_sha256"].as_str().unwrap();
            let stored_size = m["stored_size"].as_u64().unwrap() as usize;
            match m["form"].as_str().unwrap() {
                "raw" => {
                    // Stored as plain text in the archive: nothing to wrap, identity holds trivially.
                    assert_eq!(sha(&plain), stored_sha, "{title} {name}: raw member identity");
                    raw += 1;
                }
                "chunkzip" => {
                    let re = compress_chunkzip_native(&plain);
                    is_native_chunkzip_framing(&re).unwrap();
                    assert_eq!(decompress_chunkzip(&re).unwrap(), plain, "{title} {name}: decode");
                    assert_eq!(re.len(), stored_size, "{title} {name}: stored length");
                    assert_eq!(sha(&re), stored_sha, "{title} {name}: byte identity with EA's stored member");
                    identical += 1;
                }
                other => panic!("{title} {name}: unknown form {other}"),
            }
        }
    }
    assert_eq!((total, identical, raw), (65, 53, 12), "the census: 65 members, 53 chunkzip, 12 raw");
    eprintln!("chunkzip native encoder: {identical}/{identical} NHL 16–21 members byte-identical ({CHUNKZIP_NATIVE_BACKEND})");
}

/// The two NHL 21 `overlays_21.big` members whose last chunk is stored (flag 4): the
/// store-if-not-smaller rule reproduces them byte-for-byte too.
#[test]
#[ignore = "private fixtures chunkzip/nhl21-ps4-cusa18182/overlays_21.big/guestintheboo*.gfx.chunkzip: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn nhl21_overlays_members_with_a_stored_final_chunk_re_encode_identically() {
    let root = fixture_root();
    for (name, stored_sha) in [("guestinthebootharrived", "2f310dbadd66a8ae767cb8f693a091c0ae65f17c1197c840d51e791744c7da20"), ("guestintheboothteaser", "2447f07144b2ddb3640e3e9f094fa64951f707107b05ed40196b5d8df1e738b9")] {
        let path = root.join(format!("chunkzip/nhl21-ps4-cusa18182/overlays_21.big/{name}.gfx.chunkzip"));
        let stored = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert!(!stored.is_empty());
        assert_eq!(sha(&stored), stored_sha, "{name}: fixture identity");
        is_native_chunkzip_framing(&stored).unwrap();
        let plain = decompress_chunkzip(&stored).unwrap();
        let re = compress_chunkzip_native(&plain);
        assert_eq!(re.len(), stored.len(), "{name}: length");
        assert_eq!(re, stored, "{name}: byte identity incl. the stored final chunk");
        // The last chunk really is stored.
        let n = be(&stored, 20) as usize;
        let mut off = 40;
        let mut last_flag = 0;
        for _ in 0..n {
            while off % 16 != 8 {
                off += 1;
            }
            last_flag = be(&stored, off + 4);
            off += 8 + be(&stored, off) as usize;
        }
        assert_eq!(last_flag, 4, "{name}: final chunk stored");
    }
}
