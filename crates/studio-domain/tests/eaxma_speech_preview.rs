//! EA-XMA sample-bank previews through the domain (receipt 33) on the real Xbox 360 trees, read
//! only: the eight SNS tuples research R84 measured — announcer / play-by-play on NHL 09 / 10 (24 kHz
//! mono) and music on NHL 09–12 (44.1 kHz stereo). Per tuple: the listing marks the EA-XMA rows
//! previewable when a helper is installed; the first sample by stream offset has the research SNS
//! hash, previews to exactly its SNR count, and its PCM equals the research decode (R84 hash). Then
//! the refusals: a missing stream, a truncated stream, a corrupted sub-block length, no helper at
//! listing time, a helper that disappears after the listing, and a cancelled decode. The helper is the
//! file named by `NHL_STUDIO_XMA2_HELPER` (Studio's own build, receipt 32); a missing variable fails.
//! Nothing is written outside a disposable temporary root.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use sha2::{Digest, Sha256};
use studio_domain::gen7_speech_preview::{coverage, list_bank_samples, preview_sample, preview_sample_cancellable};
use studio_domain::Platform;

const ENV: &str = "NHL_STUDIO_XMA2_HELPER";

fn root(game: &str) -> PathBuf {
    PathBuf::from(match game {
        "nhl09" => r"X:\Games\Emulation\Roms\XBOX360\NHL 09\NHL 09 (USA)",
        "nhl10" => r"X:\Games\Emulation\Roms\XBOX360\NHL 10\NHL 10",
        "nhl11" => r"X:\Games\Emulation\Roms\XBOX360\NHL 11\NHL 11",
        "nhl12" => r"X:\Games\Emulation\Roms\XBOX360\NHL 12\NHL 12",
        g => panic!("{g}"),
    })
}

const TUPLES: &[(&str, &str, &str)] = &[
    ("nhl09", "ann_eng", "x360-nhl09:ann_eng"),
    ("nhl09", "pbp_eng", "x360-nhl09:pbp_eng"),
    ("nhl10", "ann_eng", "x360-nhl10:ann_eng"),
    ("nhl10", "pbp_eng", "x360-nhl10:pbp_eng"),
    ("nhl09", "music_samples", "x360-nhl09:music"),
    ("nhl10", "music_samples", "x360-nhl10:music"),
    ("nhl11", "music_samples", "x360-nhl11:music"),
    ("nhl12", "music_samples", "x360-nhl12:music"),
];

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

/// Research R84 per family: offset → (SNS SHA-256, decoded PCM SHA-256).
fn research() -> BTreeMap<String, BTreeMap<u64, (String, String)>> {
    let j: serde_json::Value = serde_json::from_slice(&std::fs::read(repo("docs/research/overnight-2026-09-24/data/r84-ea-xma-preview.json")).unwrap()).unwrap();
    let mut out = BTreeMap::new();
    for (family, f) in j["families"].as_object().unwrap() {
        let mut m = BTreeMap::new();
        for v in f["vectors"].as_array().unwrap() {
            if let (Some(off), Some(sns), Some(pcm)) = (v["off"].as_u64(), v["sns_sha256"].as_str(), v["r84"]["decode"]["sha256"].as_str()) {
                m.insert(off, (sns.to_string(), pcm.to_string()));
            }
        }
        out.insert(family.clone(), m);
    }
    out
}

#[test]
fn the_eight_r84_tuples_are_admitted_and_nothing_else_is() {
    for (game, stem, _) in TUPLES {
        let t = coverage(game, Platform::Xbox, stem).unwrap();
        assert_eq!(t.source, "R84", "{game} {stem}");
    }
    assert_eq!(coverage("nhl11", Platform::Xbox, "ann_eng").unwrap().source, "R83", "X360 11 speech stays EA-Speex");
    assert_eq!(coverage("nhl13", Platform::Xbox, "music_samples").unwrap().source, "R80", "X360 13 music stays EALayer3 v1");
    assert!(coverage("nhl11", Platform::Xbox, "pbp_eng").unwrap().source == "R83");
    assert!(coverage("nhllegacy-recompiled", Platform::Recomp, "music_samples").is_ok(), "Recompiled music is the Legacy EALayer3 tuple, not EA-XMA");
    assert_eq!(coverage("nhllegacy-recompiled", Platform::Recomp, "music_samples").unwrap().source, "R80");
    assert_eq!(coverage("nhl10", Platform::Ps3, "music_samples").unwrap().source, "R82", "PS3 10 music: EALayer3 v1 (receipt 36), not EA-XMA");
}

struct TempRoot(PathBuf);
impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A disposable root holding a copy of a bank and a prefix of its stream (`None` = no stream).
fn temp_root(tag: &str, game: &str, stem: &str, stream_prefix: Option<usize>) -> TempRoot {
    let dir = std::env::temp_dir().join(format!("nhlms-xma-speech-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let rel = if stem == "music_samples" { "audio/music" } else { "audio/speech" };
    let file = if stem == "music_samples" { "music_samples".to_string() } else { format!("{stem}_samples") };
    std::fs::create_dir_all(dir.join(rel)).unwrap();
    std::fs::copy(root(game).join(rel).join(format!("{file}.sbr")), dir.join(rel).join(format!("{file}.sbr"))).unwrap();
    if let Some(n) = stream_prefix {
        use std::io::Read;
        let mut buf = vec![0u8; n];
        std::fs::File::open(root(game).join(rel).join(format!("{file}.sbs"))).unwrap().read_exact(&mut buf).unwrap();
        std::fs::write(dir.join(rel).join(format!("{file}.sbs")), &buf).unwrap();
    }
    TempRoot(dir)
}

#[test]
#[ignore = "real X360 trees under X:/Games/Emulation/Roms (read only) and the XMA2 helper named by NHL_STUDIO_XMA2_HELPER: select with --ignored"]
fn every_r84_tuple_lists_and_previews_with_research_identity_and_pcm_then_refuses_by_name() {
    let helper = std::env::var_os(ENV).map(PathBuf::from).unwrap_or_else(|| panic!("{ENV} must name the XMA2 helper (fails, never skips)"));
    assert!(helper.is_file(), "{}", helper.display());
    let research = research();
    let mut failed = Vec::new();
    for (game, stem, family) in TUPLES {
        let r = root(game);
        assert!(r.is_dir(), "{}: missing (fails, never skips)", r.display());
        let result = (|| -> Result<String, String> {
            let l = list_bank_samples(game, Platform::Xbox, &r, stem, "", 5000).map_err(|e| e.to_string())?;
            if l.preview_unavailable.is_some() {
                return Err(format!("helper reported unavailable: {:?}", l.preview_unavailable));
            }
            let first = &l.rows[0];
            if first.codec.as_deref() != Some("ea-xma") || !first.previewable {
                return Err(format!("first row {:?} previewable {}", first.codec, first.previewable));
            }
            let not_xma = l.rows.iter().filter(|x| x.codec.as_deref().is_some_and(|c| c != "ea-xma")).count();
            let p = preview_sample(game, Platform::Xbox, &r, stem, first.sample_id).map_err(|e| e.to_string())?;
            let want = research.get(*family).and_then(|m| m.get(&p.offset)).ok_or(format!("no research vector at offset {}", p.offset))?;
            if p.sns_sha256 != want.0 {
                return Err(format!("SNS hash {} ≠ research {}", p.sns_sha256, want.0));
            }
            let pcm_sha = format!("{:x}", Sha256::digest(&p.wav[44..]));
            if pcm_sha != want.1 {
                return Err(format!("PCM {pcm_sha} ≠ research {}", want.1));
            }
            if p.decoded_samples_per_channel as u32 != p.num_samples || p.codec != "ea-xma" || p.tuple_source != "R84" || !p.label.contains("not game-verified") {
                return Err(format!("{} decoded vs {} declared; codec {}; tuple {}", p.decoded_samples_per_channel, p.num_samples, p.codec, p.tuple_source));
            }
            let problems = l.rows.iter().filter(|x| x.problem.is_some()).count();
            Ok(format!(
                "{} samples ({} shown, {} not EA-XMA, {} with a problem); sample {} @ {}: {} Hz {} ch, {} samples, SNS + PCM = research · {}",
                l.total,
                l.rows.len(),
                not_xma,
                problems,
                p.sample_id,
                p.offset,
                p.sample_rate,
                p.channels,
                p.num_samples,
                p.route_detail.unwrap_or_default()
            ))
        })();
        match result {
            Ok(line) => eprintln!("{game} {stem}: {line}"),
            Err(e) => {
                eprintln!("{game} {stem}: FAIL {e}");
                failed.push(format!("{game} {stem}: {e}"));
            }
        }
    }

    // The NHL 11 loose music stream is shorter than its bank's offsets (R84 skipped one vector for
    // this): those rows carry a problem and their preview refuses by name.
    let r11 = root("nhl11");
    let l = list_bank_samples("nhl11", Platform::Xbox, &r11, "music_samples", "", 5000).unwrap();
    let beyond: Vec<_> = l.rows.iter().filter(|x| x.problem.is_some()).collect();
    eprintln!("nhl11 music: {} of {} rows beyond / unreadable in the loose stream", beyond.len(), l.rows.len());
    if let Some(b) = beyond.first() {
        assert!(!b.previewable);
        let e = preview_sample("nhl11", Platform::Xbox, &r11, "music_samples", b.sample_id).unwrap_err().to_string();
        eprintln!("  sample {} @ {}: {e}", b.sample_id, b.offset);
    }

    // Disposable copies: a missing stream, a truncated stream, a corrupted sub-block length word.
    let t = temp_root("missing", "nhl10", "ann_eng", None);
    let e = list_bank_samples("nhl10", Platform::Xbox, &t.0, "ann_eng", "", 10).unwrap_err().to_string();
    assert!(e.contains("not loose, and no `audio/speech/ann_eng_samples.sbs` member"), "{e}");
    drop(t);
    let t = temp_root("truncated", "nhl10", "ann_eng", Some(1 << 20));
    let l = list_bank_samples("nhl10", Platform::Xbox, &t.0, "ann_eng", "", 5000).unwrap();
    let inside = l.rows.iter().find(|x| x.previewable).expect("a sample inside the first MiB");
    let outside = l.rows.iter().find(|x| x.offset > (1 << 20)).expect("a sample beyond the first MiB");
    assert!(outside.problem.as_deref().is_some_and(|p| p.contains("no SNS header block")) && !outside.previewable, "{:?}", outside.problem);
    let e = preview_sample("nhl10", Platform::Xbox, &t.0, "ann_eng", outside.sample_id).unwrap_err().to_string();
    assert!(e.contains(&format!("sample {}", outside.sample_id)), "{e}");
    preview_sample("nhl10", Platform::Xbox, &t.0, "ann_eng", inside.sample_id).expect("inside the prefix still previews");
    let sbs = t.0.join("audio/speech/ann_eng_samples.sbs");
    let mut bytes = std::fs::read(&sbs).unwrap();
    let body = inside.offset as usize + 8; // SNS header block is 8 bytes long here; the first data block follows
    assert_eq!(bytes[inside.offset as usize], 0x48);
    let data_block = body + u32::from_be_bytes([0, bytes[inside.offset as usize + 1], bytes[inside.offset as usize + 2], bytes[inside.offset as usize + 3]]) as usize - 8;
    // Corrupt the first sub-block's length word (w0) inside the first data block.
    bytes[data_block + 8 + 3] ^= 0x04;
    std::fs::write(&sbs, &bytes).unwrap();
    let e = preview_sample("nhl10", Platform::Xbox, &t.0, "ann_eng", inside.sample_id).unwrap_err().to_string();
    assert!(e.contains("EA-XMA packets refused"), "{e}");
    eprintln!("corrupted sub-block length: {e}");
    drop(t);

    // No helper at listing time: the variable names a missing file (no fallback to a helpers folder).
    let saved = std::env::var_os(ENV);
    std::env::set_var(ENV, std::env::temp_dir().join("nhlms-no-such-helper.exe"));
    let r10 = root("nhl10");
    let l = list_bank_samples("nhl10", Platform::Xbox, &r10, "ann_eng", "", 20).unwrap();
    let why = l.preview_unavailable.clone().expect("no helper → unavailable");
    assert!(why.contains("not a file") && l.rows.iter().all(|x| !x.previewable), "{why}");
    let e = preview_sample("nhl10", Platform::Xbox, &r10, "ann_eng", l.rows[0].sample_id).unwrap_err().to_string();
    assert!(e.contains("not a file"), "{e}");
    // A helper present at listing time that has gone by preview time refuses; it does not reuse a result.
    std::env::set_var(ENV, &helper);
    let l = list_bank_samples("nhl10", Platform::Xbox, &r10, "ann_eng", "", 20).unwrap();
    assert!(l.preview_unavailable.is_none() && l.rows[0].previewable);
    std::env::set_var(ENV, std::env::temp_dir().join("nhlms-no-such-helper.exe"));
    let e = preview_sample("nhl10", Platform::Xbox, &r10, "ann_eng", l.rows[0].sample_id).unwrap_err().to_string();
    assert!(e.contains("not a file"), "{e}");
    // Cancelled: the helper run is stopped and the preview refuses with the cancellation text.
    std::env::set_var(ENV, &helper);
    let cancel = AtomicBool::new(true);
    let e = preview_sample_cancellable("nhl10", Platform::Xbox, &r10, "ann_eng", l.rows[0].sample_id, Some(&cancel)).unwrap_err().to_string();
    assert!(e.contains("cancelled"), "{e}");
    match saved {
        Some(v) => std::env::set_var(ENV, v),
        None => std::env::remove_var(ENV),
    }
    assert!(failed.is_empty(), "{failed:#?}");
}
