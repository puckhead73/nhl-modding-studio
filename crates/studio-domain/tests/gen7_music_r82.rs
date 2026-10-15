//! PS3 NHL 09 / 10 / 12 / 13 menu-music previews (research R82 = 2026-09-24 handoff 01 §3, named ready by
//! 2026-09-26 handoff 19 A1; receipt 36) through the domain, read only. The bank ships only inside
//! `audioboot.big` (RefPack on 09 / 10, chunkzip on 12 / 13); the stream in `data0.big` (09), loose (10) or
//! `nocache.big` (12 / 13). Per title the listing resolves the bank from its archive with R82's decoded
//! SHA-256 and entry count, and each of R82's three vectors previews with R82's SNS SHA-256, granule
//! count and rebuilt-frame count, decoding to granules × 576 samples per channel (R82's decoder count).
//! R82's PCM hashes come from FFmpeg's decoder; Studio decodes with Symphonia, so PCM is not compared.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_domain::gen7_speech_preview::{coverage, list_bank_samples, preview_sample};
use studio_domain::Platform;

const PS3: &str = r"X:\Games\Emulation\Roms\Sony Playstation 3";

fn root(game: &str) -> PathBuf {
    Path::new(PS3).join(match game {
        "nhl09" => r"NHL 09\PS3_GAME\USRDIR",
        "nhl10" => r"NHL 10\PS3_GAME\USRDIR",
        "nhl12" => r"NHL 12\[BLUS 30791] NHL® 12\PS3_GAME\USRDIR",
        "nhl13" => r"NHL 13\PS3_GAME\USRDIR",
        g => panic!("{g}"),
    })
}

#[test]
fn the_four_r82_music_tuples_are_admitted() {
    for game in ["nhl09", "nhl10", "nhl12", "nhl13"] {
        let t = coverage(game, Platform::Ps3, "music_samples").unwrap();
        assert_eq!((t.source, t.sample_rate, t.channels), ("R82", 44_100, 2), "{game}");
    }
    assert!(coverage("nhl09", Platform::Xbox, "music_samples").unwrap().source == "R84", "X360 09 music is EA-XMA");
}

#[test]
#[ignore = "real PS3 trees under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn every_r82_music_vector_previews_from_its_archive_backed_bank() {
    let j: serde_json::Value = serde_json::from_slice(&std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/research/overnight-2026-09-24/data/r82b-gen7-archive-audio-coverage.json")).unwrap()).unwrap();
    let mut failed = Vec::new();
    for game in ["nhl09", "nhl10", "nhl12", "nhl13"] {
        let r = root(game);
        assert!(r.is_dir(), "{}: missing (fails, never skips)", r.display());
        let m = &j["trees"][format!("ps3-{game}")]["music"];
        let l = list_bank_samples(game, Platform::Ps3, &r, "music_samples", "", 5000).unwrap_or_else(|e| panic!("{game}: {e}"));
        assert_eq!(l.bank_sha256, m["sbr"]["sha256"].as_str().unwrap(), "{game}: decoded bank = R82");
        assert_eq!(l.total as u64, m["entries"].as_u64().unwrap(), "{game}: entries = R82");
        assert!(l.bank.contains("audioboot.big"), "{game}: {}", l.bank);
        let by_offset: BTreeMap<u64, u32> = l.rows.iter().map(|x| (x.offset, x.sample_id)).collect();
        for v in m["vectors"].as_array().unwrap() {
            let off = v["off"].as_u64().unwrap();
            let r77 = &v["r77"];
            let result = (|| -> Result<String, String> {
                let sid = *by_offset.get(&off).ok_or(format!("no bank entry at offset {off}"))?;
                let row = l.rows.iter().find(|x| x.sample_id == sid).unwrap();
                if !row.previewable {
                    return Err(format!("row {sid} not previewable: {:?}", row.problem));
                }
                let p = preview_sample(game, Platform::Ps3, &r, "music_samples", sid).map_err(|e| e.to_string())?;
                let want = (v["sns_sha256"].as_str().unwrap().to_string(), r77["granules"].as_u64().unwrap() as usize, r77["rebuild"]["frames"].as_u64().unwrap() as usize);
                let got = (p.sns_sha256.clone(), p.granules, p.rebuilt_frames);
                if got != want {
                    return Err(format!("(SNS, granules, frames) {got:?} ≠ R82 {want:?}"));
                }
                if p.decoded_samples_per_channel != p.granules * 576 || p.decoded_samples_per_channel as u64 != r77["decode"]["samples"].as_u64().unwrap() {
                    return Err(format!("decoded {} per channel vs granules × 576 = {}", p.decoded_samples_per_channel, p.granules * 576));
                }
                if (p.channels, p.sample_rate, p.tuple_source.as_str()) != (2, 44_100, "R82") {
                    return Err(format!("{} ch {} Hz tuple {}", p.channels, p.sample_rate, p.tuple_source));
                }
                Ok(format!("sample {sid} @ {off}: {} granules → {} frames → {} samples per channel · stream {}", p.granules, p.rebuilt_frames, p.decoded_samples_per_channel, p.stream_path))
            })();
            match result {
                Ok(line) => eprintln!("{game}: {line}"),
                Err(e) => {
                    eprintln!("{game} @ {off}: FAIL {e}");
                    failed.push(format!("{game} @ {off}: {e}"));
                }
            }
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}
