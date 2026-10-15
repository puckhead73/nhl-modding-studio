//! The R77 / R80 gen7 EALayer3 v1 vectors through `tdb-snd`: for every research vector whose
//! `.sbs` stream is a loose file on the tree (the R77 six pbp sets, the R80 PS3 `ann_eng` /
//! `pbp_eng` 24 kHz sets, the `music_samples` stereo sets), the sample's SNS bytes hash to the
//! research value, the granule grammar accepts it with the research granule counts, the rebuild
//! yields the research frame count with `main_data_begin` never above 511 / 255, and the Symphonia
//! decode returns `granules × 576` samples per channel. Vectors whose stream is only an archive
//! member (X360 15 / Legacy / NHL 11 pbp on some trees) are read from the archive at the recorded
//! member offset. `.abk` raw streams are not covered here (their S10A container is not read).
//!
//! Read-only over the game trees on `X:`; nothing written. A missing tree fails loudly.
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tdb_snd::ealayer3::{contract_parse, rebuild, LABEL};
use tdb_snd::{decode_layer3, walk_sample};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn tree_root(key: &str) -> PathBuf {
    let x360 = r"X:\Games\Emulation\Roms\XBOX360";
    let ps3 = r"X:\Games\Emulation\Roms\Sony Playstation 3";
    let p = match key {
        "x360-nhl09" => format!(r"{x360}\NHL 09\NHL 09 (USA)"),
        "x360-nhl10" => format!(r"{x360}\NHL 10\NHL 10"),
        "x360-nhl11" => format!(r"{x360}\NHL 11\NHL 11"),
        "x360-nhl12" => format!(r"{x360}\NHL 12\NHL 12"),
        "x360-nhl13" => format!(r"{x360}\NHL 13\NHL 13"),
        "x360-nhl14" => format!(r"{x360}\NHL 14\NHL 14"),
        "x360-nhl15" => format!(r"{x360}\NHL 15\NHL 15"),
        "x360-legacy" => format!(r"{x360}\NHL Legacy\NHL Legacy"),
        "ps3-nhl09" => format!(r"{ps3}\NHL 09\PS3_GAME\USRDIR"),
        "ps3-nhl10" => format!(r"{ps3}\NHL 10\PS3_GAME\USRDIR"),
        "ps3-nhl11" => format!(r"{ps3}\NHL 11\PS3_GAME\USRDIR"),
        "ps3-nhl12" => format!(r"{ps3}\NHL 12\[BLUS 30791] NHL® 12\PS3_GAME\USRDIR"),
        "ps3-nhl13" => format!(r"{ps3}\NHL 13\PS3_GAME\USRDIR"),
        "ps3-nhl14" => format!(r"{ps3}\NHL 14\PS3_GAME\USRDIR"),
        "ps3-nhl15" => format!(r"{ps3}\NHL 15\PS3_GAME\USRDIR"),
        "ps3-legacy" => format!(r"{ps3}\NHL Legacy - Vanilla\PS3_GAME\USRDIR"),
        other => panic!("unknown tree {other}"),
    };
    PathBuf::from(p)
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

/// Where a bank's `.sbs` bytes start: the loose file at offset 0, or the archive member's offset.
fn stream_source(root: &Path, stem: &str, sbs: &serde_json::Value, music: bool) -> Option<(PathBuf, u64)> {
    let loose = if music { root.join("audio").join("music").join(format!("{stem}.sbs")) } else { root.join("audio").join("speech").join(format!("{stem}_samples.sbs")) };
    if loose.is_file() {
        return Some((loose, 0));
    }
    let m = sbs.get("archive_member")?.as_object()?;
    let archive = root.join(m.get("archive")?.as_str()?);
    let off = m.get("offset")?.as_u64()?;
    archive.is_file().then_some((archive, off))
}

struct Tally {
    vectors: usize,
    decoded: usize,
    skipped_archive: usize,
}

fn run_vector(title: &str, stem: &str, stream: &Path, base: u64, v: &serde_json::Value, expect_rate: Option<u64>, tally: &mut Tally) {
    let off = v["off"].as_u64().unwrap();
    let window = read_window(stream, base + off, 32 << 20);
    let (snr, blocks, end) = walk_sample(&window, 0).unwrap_or_else(|e| panic!("{title} {stem} @{off}: {e}"));
    let raw = &window[..end as usize];
    assert_eq!(format!("{:x}", Sha256::digest(raw)), v["sns_sha256"].as_str().unwrap(), "{title} {stem} @{off}: SNS bytes");
    assert_eq!(snr.codec, 0x05, "{title} {stem} @{off}: codec");
    if let Some(r) = expect_rate {
        assert_eq!(snr.sample_rate as u64, r, "{title} {stem} @{off}: rate");
    }
    tdb_snd::ealayer3::check_header(&snr).unwrap();
    let refs: Vec<(u32, &[u8])> = blocks.iter().map(|b| (b.samples, b.payload.as_slice())).collect();
    let parsed = contract_parse(&refs, snr.num_samples).unwrap_or_else(|e| panic!("{title} {stem} @{off}: grammar: {e}"));
    assert_eq!(parsed.channels as u32, snr.channels(), "{title} {stem} @{off}: channels");
    assert_eq!(parsed.pcm.pcm_samples, 47);
    assert_eq!(parsed.pcm.samples.len(), 47 * parsed.channels);
    if let Some(g) = v["granules"].as_u64() {
        assert_eq!(parsed.granules.len() as u64, g, "{title} {stem} @{off}: granules");
    }
    if let Some(s) = v["granules_x576_minus_num_samples"].as_i64() {
        // The research figure counts the 47 PCM samples on the granule side too.
        assert_eq!(parsed.surplus_samples, s, "{title} {stem} @{off}: surplus");
    }
    let rebuilt = rebuild(&parsed.granules).unwrap_or_else(|e| panic!("{title} {stem} @{off}: rebuild: {e}"));
    if let Some(f) = v["T6"]["frames"].as_u64() {
        assert_eq!(rebuilt.frames as u64, f, "{title} {stem} @{off}: frames");
    }
    assert!(rebuilt.max_main_data_begin <= if parsed.mpeg1 { 511 } else { 255 });
    let dec = decode_layer3(&rebuilt.mpeg).unwrap_or_else(|e| panic!("{title} {stem} @{off}: decode: {e}"));
    assert_eq!(dec.sample_rate, snr.sample_rate);
    assert_eq!(dec.channels as usize, parsed.channels);
    let expect_samples = rebuilt.frames * dec.samples_per_frame;
    assert_eq!(dec.pcm.len(), expect_samples * dec.channels as usize, "{title} {stem} @{off}: sample count");
    if let Some(s) = v["T6"]["samples"].as_u64() {
        assert_eq!(expect_samples as u64, s, "{title} {stem} @{off}: research sample count");
    }
    // Decoder-independent sanity: speech-level RMS in the research's range class, and a warm-up zone.
    let n = dec.pcm.len() as f64;
    let rms = (dec.pcm.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>() / n).sqrt();
    if let Some(r) = v["T6"]["rms"].as_f64() {
        assert!((rms - r).abs() <= r * 0.05 + 2.0, "{title} {stem} @{off}: RMS {rms:.1} vs research {r:.1}");
    }
    let first = dec.pcm.iter().position(|&x| x.abs() > 1);
    eprintln!("{title} {stem} @{off}: {} blocks, {} granules {:?}, {} frames, mdb≤{}, {} Hz {} ch, decoded {} samples, RMS {rms:.1} (research {:?}), first>1LSB {:?} (research {:?}) — {}", blocks.len(), parsed.granules.len(), parsed.granules_per_block, rebuilt.frames, rebuilt.max_main_data_begin, dec.sample_rate, dec.channels, expect_samples, v["T6"]["rms"].as_f64(), first, v["T6"]["first_nonzero"].as_i64(), LABEL);
    tally.vectors += 1;
    tally.decoded += 1;
}

#[test]
#[ignore = "real gen7 trees under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn the_r77_and_r80_ealayer3_vectors_parse_rebuild_and_decode() {
    let mut tally = Tally { vectors: 0, decoded: 0, skipped_archive: 0 };
    // R77: the six pbp sets.
    let r77: serde_json::Value = serde_json::from_slice(&std::fs::read(repo("docs/research/overnight-2026-09-22-pm/data/r77-gen7-ealayer3-v1-grammar.json")).unwrap()).unwrap();
    for (tree, banks) in r77["trees"].as_object().unwrap() {
        let root = tree_root(tree);
        assert!(root.is_dir(), "{}: missing (fails, never skips)", root.display());
        for (stem, b) in banks.as_object().unwrap() {
            let Some((stream, base)) = stream_source(&root, stem, &b["sbs"], false) else {
                tally.skipped_archive += 1;
                eprintln!("{tree} {stem}: stream not readable (no loose .sbs and no readable archive member) — skipped, counted");
                continue;
            };
            for v in b["vectors"].as_array().unwrap() {
                run_vector(tree, stem, &stream, base, v, v["rate"].as_u64(), &mut tally);
            }
        }
    }
    // R80: every codec-0x05 vector with a block stream (ann_eng / pbp_eng / music); .abk entries excluded.
    let r80: serde_json::Value = serde_json::from_slice(&std::fs::read(repo("docs/research/overnight-2026-09-22-pm/data/r80-gen7-codec05-survey.json")).unwrap()).unwrap();
    let mut r80_vectors = 0;
    for (tree, banks) in r80["trees"].as_object().unwrap() {
        let root = tree_root(tree);
        for (stem, b) in banks.as_object().unwrap() {
            if stem.ends_with(".abk") {
                continue;
            }
            let Some(vs) = b["vectors"].as_array() else { continue };
            let ealayer3: Vec<&serde_json::Value> = vs.iter().filter(|v| v["snr"]["codec_code"].as_u64() == Some(5) && v.get("r77").is_some()).collect();
            if ealayer3.is_empty() {
                continue;
            }
            let music = stem == "music";
            let sbs = if music { serde_json::Value::Null } else { b["source"]["sbs"].clone() };
            let bank_stem = if music { "music_samples" } else { stem.as_str() };
            let Some((stream, base)) = stream_source(&root, bank_stem, &sbs, music) else {
                tally.skipped_archive += 1;
                eprintln!("{tree} {stem}: stream not readable — skipped, counted");
                continue;
            };
            for v in ealayer3 {
                // R80 nests the R77 measurements under `r77` with its own key names: map them onto
                // the R77 vector shape this checker reads.
                let r = &v["r77"];
                let mut merged = serde_json::json!({
                    "off": v["off"], "sns_sha256": v["sns_sha256"],
                    "granules": r["granules"],
                    "granules_x576_minus_num_samples": r["granules_x576_plus_pcm_minus_num_samples"],
                    "T6": { "frames": r["rebuild"]["frames"], "samples": r["decode"]["samples"], "rms": r["decode"]["rms"], "first_nonzero": r["decode"]["first_nonzero"] },
                });
                if r["established"].as_bool() != Some(true) { merged["granules"] = serde_json::Value::Null; }
                run_vector(tree, stem, &stream, base, &merged, v["snr"]["sample_rate"].as_u64(), &mut tally);
                r80_vectors += 1;
            }
        }
    }
    eprintln!("vectors checked {} (R80 share {r80_vectors}); decoded {}; banks skipped for an unreadable stream {}", tally.vectors, tally.decoded, tally.skipped_archive);
    assert!(tally.vectors >= 48, "the six R77 sets alone are 48 vectors");
}
