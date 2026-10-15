//! EA-Speex (codec `0x09`) through `tdb-snd` + pinned libspeex (`tdb-speex`) on the research R83
//! vectors: 25 families (X360 NHL 11–Legacy `ann_eng`, X360 NHL 11–14 `pbp_eng`, PS3 NHL 14
//! `pbp_eng`, PS4 NHL 15–21 `rink_ann_samples[_inxp]`), up to four vectors each, from
//! `docs/research/overnight-2026-09-24/data/r83-codec09-speex.json`.
//!
//! Per vector: the SNS bytes hash to the research `sns_sha256` (source identity before decoding);
//! the grammar accepts the unmodified stream with the research frame count and tail trim and refuses
//! the four R83 mutations by name; the decode yields frames × 640 samples and, after the 509-sample
//! start and the tail trim, exactly `num_samples`. **PCM comparison:** the untrimmed decode is compared
//! sample by sample with FFmpeg's `libspeex` decoder (the research oracle, invoked here only as an
//! external reference program — it is not part of Studio) on the same frames rewrapped losslessly in
//! Ogg Speex (80-byte header, one frame per packet, enhancer default on); max |diff|, identical share
//! and correlation are printed per vector and per family. The tolerance asserted — max |diff| ≤ 2 LSB
//! and correlation ≥ 0.9999 — is what the admitted families must meet; failures are listed, not hidden.
//! Nothing is listened to; nothing is written outside an owned temporary directory.
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use tdb_snd::easpeex::{self, SpeexRefusal};
use tdb_snd::walk_sample;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn read_window(path: &Path, off: u64, len: usize) -> Vec<u8> {
    let mut f = File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    f.seek(SeekFrom::Start(off)).unwrap();
    let mut buf = vec![0u8; len];
    let mut got = 0;
    while got < len {
        let n = f.read(&mut buf[got..]).unwrap();
        if n == 0 {
            break;
        }
        got += n;
    }
    buf.truncate(got);
    buf
}

/// The gen7 tree root of a research family key (`x360-nhl11`, `ps3-nhl14`, `x360-legacy`).
fn gen7_root(key: &str) -> PathBuf {
    let (plat, game) = key.split_once('-').unwrap();
    let title = match game {
        "legacy" => "NHL Legacy".to_string(),
        g => format!("NHL {}", &g[3..]),
    };
    match plat {
        "x360" => PathBuf::from(format!(r"X:\Games\Emulation\Roms\XBOX360\{title}\{title}")),
        "ps3" => PathBuf::from(format!(r"X:\Games\Emulation\Roms\Sony Playstation 3\{title}\PS3_GAME\USRDIR")),
        _ => unreachable!(),
    }
}

/// `(stream file, base offset)` for a vector offset: the loose `.sbs` when it covers the offset, else
/// the archive member the research names.
fn stream_for(family: &str, f: &serde_json::Value, off: u64) -> (PathBuf, u64) {
    let (key, stem) = family.split_once(':').unwrap();
    if key.starts_with("ps4") {
        let sbr = PathBuf::from(f["source"]["sbr"]["path"].as_str().unwrap());
        return (sbr.with_extension("sbs"), 0);
    }
    let root = gen7_root(key);
    let loose = root.join("audio/speech").join(format!("{stem}_samples.sbs"));
    if loose.is_file() && std::fs::metadata(&loose).unwrap().len() > off + 16 {
        return (loose, 0);
    }
    let m = &f["source"]["sbs"]["archive_member"];
    assert!(m.is_object(), "{family}: no loose stream covering {off} and no archive member");
    (root.join(m["archive"].as_str().unwrap()), m["offset"].as_u64().unwrap())
}

// --- Ogg Speex rewrap (for the FFmpeg oracle only) ---------------------------------------------

fn ogg_crc(data: &[u8]) -> u32 {
    let mut crc: u32 = 0;
    for &b in data {
        crc ^= (b as u32) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 { (crc << 1) ^ 0x04c1_1db7 } else { crc << 1 };
        }
    }
    crc
}

fn ogg_page(out: &mut Vec<u8>, flags: u8, granule: u64, seq: u32, packet: &[u8]) {
    let mut lacing = Vec::new();
    let mut n = packet.len();
    while n >= 255 {
        lacing.push(255u8);
        n -= 255;
    }
    lacing.push(n as u8);
    let start = out.len();
    out.extend_from_slice(b"OggS");
    out.push(0);
    out.push(flags);
    out.extend_from_slice(&granule.to_le_bytes());
    out.extend_from_slice(&0x4e48_4c4du32.to_le_bytes()); // serial
    out.extend_from_slice(&seq.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.push(lacing.len() as u8);
    out.extend_from_slice(&lacing);
    out.extend_from_slice(packet);
    let crc = ogg_crc(&out[start..]);
    out[start + 22..start + 26].copy_from_slice(&crc.to_le_bytes());
}

fn ogg_speex(frames: &[Vec<u8>]) -> Vec<u8> {
    let mut h = Vec::new();
    h.extend_from_slice(b"Speex   ");
    let mut ver = [0u8; 20];
    ver[..11].copy_from_slice(b"speex-1.2.1");
    h.extend_from_slice(&ver);
    for v in [1i32, 80, 32000, 2, 4, 1, -1, 640, 1, 1, 0, 0, 0] {
        h.extend_from_slice(&v.to_le_bytes());
    }
    assert_eq!(h.len(), 80);
    let vendor = b"nhlms-oracle";
    let mut c = (vendor.len() as u32).to_le_bytes().to_vec();
    c.extend_from_slice(vendor);
    c.extend_from_slice(&0u32.to_le_bytes());
    let mut out = Vec::new();
    ogg_page(&mut out, 0x02, 0, 0, &h);
    ogg_page(&mut out, 0x00, 0, 1, &c);
    for (i, f) in frames.iter().enumerate() {
        let flags = if i + 1 == frames.len() { 0x04 } else { 0 };
        ogg_page(&mut out, flags, (i as u64 + 1) * 640, i as u32 + 2, f);
    }
    out
}

fn ffmpeg_libspeex(tmp: &Path, tag: &str, frames: &[Vec<u8>]) -> Vec<i16> {
    let ogg = tmp.join(format!("{tag}.ogg"));
    let raw = tmp.join(format!("{tag}.s16"));
    std::fs::write(&ogg, ogg_speex(frames)).unwrap();
    let st = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-c:a", "libspeex", "-i"])
        .arg(&ogg)
        .args(["-f", "s16le", "-ac", "1", "-ar", "32000"])
        .arg(&raw)
        .status()
        .expect("ffmpeg (the research oracle) must be on PATH for this diagnostic case");
    assert!(st.success(), "{tag}: ffmpeg libspeex failed");
    let b = std::fs::read(&raw).unwrap();
    let _ = std::fs::remove_file(&ogg);
    let _ = std::fs::remove_file(&raw);
    b.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect()
}

#[derive(Default, Clone)]
struct Cmp {
    n: usize,
    identical: usize,
    max_abs: i32,
    sxy: f64,
    sxx: f64,
    syy: f64,
    sx: f64,
    sy: f64,
}

impl Cmp {
    fn add(&mut self, a: &[i16], b: &[i16]) {
        for (&x, &y) in a.iter().zip(b) {
            let (x, y) = (x as f64, y as f64);
            self.n += 1;
            self.identical += (x == y) as usize;
            self.max_abs = self.max_abs.max((x - y).abs() as i32);
            self.sxy += x * y;
            self.sxx += x * x;
            self.syy += y * y;
            self.sx += x;
            self.sy += y;
        }
    }
    fn corr(&self) -> f64 {
        let n = self.n as f64;
        let cov = self.sxy - self.sx * self.sy / n;
        let vx = self.sxx - self.sx * self.sx / n;
        let vy = self.syy - self.sy * self.sy / n;
        if vx <= 0.0 || vy <= 0.0 {
            return if self.max_abs == 0 { 1.0 } else { 0.0 };
        }
        cov / (vx * vy).sqrt()
    }
    fn line(&self) -> String {
        format!("{} samples, max |diff| {} LSB, identical {:.1} %, corr {:.6}", self.n, self.max_abs, 100.0 * self.identical as f64 / self.n.max(1) as f64, self.corr())
    }
}

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "real gen7 trees under X:/Games/Emulation/Roms and PS4 banks under H:/Emulators/ShadPS4 (read only), plus ffmpeg on PATH as the research oracle: select with --ignored"]
fn the_r83_vectors_parse_hash_decode_and_match_the_libspeex_oracle() {
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(repo("docs/research/overnight-2026-09-24/data/r83-codec09-speex.json")).unwrap()).unwrap();
    let tmp = Tmp(std::env::temp_dir().join(format!("nhlms-easpeex-oracle-{}", std::process::id())));
    std::fs::create_dir_all(&tmp.0).unwrap();
    let mut per_family: BTreeMap<String, Cmp> = BTreeMap::new();
    let mut failures = Vec::new();
    let (mut vectors, mut skipped) = (0, Vec::new());
    for (family, f) in json["families"].as_object().unwrap() {
        for v in f["vectors"].as_array().unwrap() {
            let off = v["off"].as_u64().unwrap();
            let Some(want_sha) = v["sns_sha256"].as_str() else {
                skipped.push(format!("{family} @{off}: no research SNS hash (R83 skipped it)"));
                continue;
            };
            let (file, base) = stream_for(family, f, off);
            assert!(file.is_file(), "{family}: {} missing (fails, never skips)", file.display());
            let window = read_window(&file, base + off, 4 << 20);
            let (snr, blocks, end) = walk_sample(&window, 0).unwrap_or_else(|e| panic!("{family} @{off}: {e}"));
            let raw = &window[..end as usize];
            assert_eq!(format!("{:x}", Sha256::digest(raw)), want_sha, "{family} @{off}: source identity");
            let r83 = &v["r83"];
            let s = easpeex::parse(&snr, &blocks).unwrap_or_else(|e| panic!("{family} @{off}: {e}"));
            assert_eq!(s.frames.len() as u64, r83["frames"].as_u64().unwrap(), "{family} @{off}: frames");
            assert_eq!(s.tail_trim as u64, r83["tail_trim"].as_u64().unwrap(), "{family} @{off}: tail trim");
            // R83's four mutations refuse by name.
            let mut b = blocks.clone();
            let l = b.last().unwrap().payload.len();
            b.last_mut().unwrap().payload.truncate(l - 3);
            assert!(matches!(easpeex::parse(&snr, &b), Err(SpeexRefusal::FrameOverrun { .. } | SpeexRefusal::LastBlockCount { .. } | SpeexRefusal::SingleBlockCount { .. })), "{family} @{off}: truncation");
            let mut b = blocks.clone();
            b[0].payload[0] = 0;
            assert!(matches!(easpeex::parse(&snr, &b), Err(SpeexRefusal::ZeroLength { block: 0, at: 0 })), "{family} @{off}: length 0");
            let mut b = blocks.clone();
            b[0].payload[0] = 0xff;
            assert!(easpeex::parse(&snr, &b).is_err(), "{family} @{off}: length 0xff");
            let mut b = blocks.clone();
            b[0].samples += 1;
            assert!(easpeex::parse(&snr, &b).is_err(), "{family} @{off}: block samples + 1");
            // Decode through pinned libspeex.
            let (pcm, decoded) = easpeex::decode(&s).unwrap_or_else(|e| panic!("{family} @{off}: {e}"));
            assert_eq!(decoded, s.frames.len() * 640, "{family} @{off}: frames × 640");
            assert_eq!(pcm.len() as u32, snr.num_samples, "{family} @{off}: 509 + num_samples + trim");
            let mut full = Vec::with_capacity(decoded);
            let mut dec = tdb_speex::UwbDecoder::new(true).unwrap();
            for fr in &s.frames {
                full.extend(dec.decode(fr).unwrap());
            }
            assert_eq!(&full[509..509 + pcm.len()], &pcm[..], "{family} @{off}: trim is a slice of the full decode");
            let oracle = ffmpeg_libspeex(&tmp.0, &format!("{}-{off}", family.replace(':', "-")), &s.frames);
            assert_eq!(oracle.len(), full.len(), "{family} @{off}: oracle length");
            let mut c = Cmp::default();
            c.add(&full, &oracle);
            let research_pcm = r83["decode"]["libspeex"]["sha256"].as_str().unwrap_or("");
            let oracle_sha = format!("{:x}", Sha256::digest(oracle.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>()));
            eprintln!("{family} @{off}: {} blocks, {} frames, trim {} — vs ffmpeg libspeex: {}{}", blocks.len(), s.frames.len(), s.tail_trim, c.line(), if oracle_sha == research_pcm { " (oracle = R83 hash)" } else { " (oracle ≠ R83 hash)" });
            if c.max_abs > 2 || c.corr() < 0.9999 {
                failures.push(format!("{family} @{off}: {}", c.line()));
            }
            per_family.entry(family.clone()).or_default().add(&full, &oracle);
            vectors += 1;
        }
    }
    eprintln!("\n| family | vectors compared | result vs ffmpeg libspeex |\n|---|---|---|");
    for (fam, c) in &per_family {
        let n = json["families"][fam]["vectors"].as_array().unwrap().iter().filter(|v| v["sns_sha256"].is_string()).count();
        eprintln!("| {fam} | {n} | {} |", c.line());
    }
    eprintln!("{vectors} vectors decoded and compared; skipped (no research vector): {skipped:?}");
    assert!(vectors >= 96, "every R83 vector ({vectors})");
    assert!(failures.is_empty(), "outside the 2-LSB / 0.9999 tolerance: {failures:#?}");
}
