//! P4 (overnight 2026-09-24) — the PS4 speech bank listing and the diagnostic EA-MP3 preview on a
//! synthetic extracted install: a synthetic `SBle` bank + `.sbs` stream holding one EA-MP3 sample
//! (two blocks, 12 synthetic frames, 1 152 embedded samples), one EA-Opus-labelled sample and one
//! sample whose grammar is broken. The listing measures the codec census from the headers; the
//! preview renders the EA-MP3 sample in both renderings, labelled diagnostic; the other two refuse by
//! name; nothing is written to the install. The real R71 vectors run in `tdb-snd` (`--ignored`).
use std::path::{Path, PathBuf};

use studio_domain::ps4_speech_bank::{list_banks, list_samples, preview_sample, PreviewRendering};
use studio_domain::Platform;
use tdb_snd::eamp3::{synthetic_ea_frame, synthetic_frame};
use tdb_snd::sns::{encode_sample, SnrHeader};

struct Tmp(PathBuf);
impl Tmp {
    fn new() -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-ps4snd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Tmp(d)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn ea_mp3_sample(num_samples: u32) -> Vec<u8> {
    let pcm: Vec<i16> = (0..1152).map(|i| ((i % 64) as i16 - 32) * 100).collect();
    let mut b0 = synthetic_ea_frame(&synthetic_frame(64, 0), Some(&pcm));
    for _ in 0..8 {
        b0.extend(synthetic_ea_frame(&synthetic_frame(96, 100), None));
    }
    let mut b1 = Vec::new();
    for _ in 0..3 {
        b1.extend(synthetic_ea_frame(&synthetic_frame(128, 200), None));
    }
    let h = SnrHeader { version: 1, codec: 0x0b, channel_config: 0, sample_rate: 48000, stream_type: 1, looped: false, num_samples };
    encode_sample(&h, &[(10368, &b0), (num_samples - 10368, &b1)])
}

#[test]
fn lists_banks_and_samples_with_a_measured_codec_census_and_previews_only_ea_mp3() {
    let tmp = Tmp::new();
    // The kettle root walk: `<root>/nhl/data/kettle/audio/speech/eng` (as the context listing resolves it).
    let root = tmp.0.join("install");
    let dir = studio_domain::ps4_speech_bank::ps4_speech_dir(&root);
    std::fs::create_dir_all(&dir).unwrap();
    // Three samples in the stream: EA-MP3 (good), EA-Opus-labelled, EA-MP3 with a dropped block.
    let good = ea_mp3_sample(10368 + 3456 - 400);
    let opus_h = SnrHeader { version: 1, codec: 0x0c, channel_config: 0, sample_rate: 48000, stream_type: 1, looped: false, num_samples: 5000 };
    let opus = encode_sample(&opus_h, &[(5000, &[0xfc, 0xff, 0xfe, 0x00])]);
    let broken = {
        let full = ea_mp3_sample(10368 + 3456 - 400);
        // Re-encode with only the first block declared: Σ block samples ≠ num_samples.
        let (h, blocks, _) = tdb_snd::walk_sample(&full, 0).unwrap();
        encode_sample(&h, &[(blocks[0].samples, &blocks[0].payload)])
    };
    let mut sbs = vec![0u8; 32]; // the .sbs 'data' header the research notes (32 bytes)
    sbs[..4].copy_from_slice(b"data");
    let off_good = sbs.len() as u32;
    sbs.extend(&good);
    let off_opus = sbs.len() as u32;
    sbs.extend(&opus);
    let off_broken = sbs.len() as u32;
    sbs.extend(&broken);
    let bank = tdb_snd::sbr::synth::build_bank(&[(332, off_good, 1.6f32.to_bits()), (360, off_opus, 0.1f32.to_bits()), (5587, off_broken, 0.3f32.to_bits())]);
    std::fs::write(dir.join("pbp_samples.sbr"), &bank).unwrap();
    std::fs::write(dir.join("pbp_samples.sbs"), &sbs).unwrap();
    std::fs::write(dir.join("junk.sbr"), b"NOTABANK").unwrap();
    let before_bank = std::fs::read(dir.join("pbp_samples.sbr")).unwrap();
    let before_sbs = std::fs::read(dir.join("pbp_samples.sbs")).unwrap();

    // Banks.
    let l = list_banks("nhl17", Platform::Ps4, &root).unwrap();
    assert_eq!(l.banks.len(), 2, "{:?}", l.banks);
    let junk = l.banks.iter().find(|b| b.file == "junk.sbr").unwrap();
    assert!(junk.note.as_deref().unwrap().contains("not read"), "{:?}", junk.note);
    let b = l.banks.iter().find(|b| b.file == "pbp_samples.sbr").unwrap();
    assert_eq!((b.dsets, b.samples, b.sbs_present, b.sbs_len), (1, 3, true, Some(sbs.len() as u64)));
    assert_eq!(b.sbr_len, bank.len() as u64);
    assert!(list_banks("nhl17", Platform::Xbox, &root).is_err());
    assert!(list_banks("nhl17", Platform::Ps4, &tmp.0.join("nowhere")).is_err());

    // Samples + census.
    let s = list_samples("nhl17", Platform::Ps4, &root, "pbp_samples.sbr", "", 100).unwrap();
    assert_eq!((s.total, s.matching, s.truncated), (3, 3, false));
    assert_eq!(s.codec_census, vec![("ea-mp3".to_string(), 2), ("ea-opus".to_string(), 1)]);
    let row = |sid: u64| s.rows.iter().find(|r| r.sid == sid).unwrap();
    assert_eq!((row(332).codec.as_deref(), row(332).previewable, row(332).duration_ms, row(332).num_samples), (Some("ea-mp3"), true, Some(1600), Some(10368 + 3456 - 400)));
    assert_eq!((row(360).codec.as_deref(), row(360).previewable), (Some("ea-opus"), false));
    assert!(row(5587).previewable, "the header of the broken sample is still EA-MP3; the grammar refuses it at preview time");
    let f = list_samples("nhl17", Platform::Ps4, &root, "pbp_samples.sbr", "55", 100).unwrap();
    assert_eq!((f.matching, f.rows.len(), f.rows[0].sid), (1, 1, 5587));
    let t = list_samples("nhl17", Platform::Ps4, &root, "pbp_samples.sbr", "", 1).unwrap();
    assert!(t.truncated && t.rows.len() == 1);
    assert!(list_samples("nhl17", Platform::Ps4, &root, "../pbp_samples.sbr", "", 10).is_err());
    assert!(list_samples("nhl17", Platform::Ps4, &root, "missing.sbr", "", 10).is_err());

    // Preview: both renderings, labelled diagnostic, trimmed to num_samples.
    for (r, label_part) in [(PreviewRendering::CandidateA, "candidate A"), (PreviewRendering::MpegOnly, "MPEG frames only")] {
        let p = preview_sample("nhl17", Platform::Ps4, &root, "pbp_samples.sbr", 332, r).unwrap();
        assert_eq!((p.codec.as_str(), p.sample_rate, p.channels, p.blocks, p.frames, p.trim, p.embedded_pcm_samples), ("ea-mp3", 48000, 1, 2, 12, 400, 1152));
        assert_eq!(p.frames_per_block, vec![9, 3]);
        assert_eq!(p.num_samples, 10368 + 3456 - 400);
        assert!(p.label.contains(label_part) && p.label.contains("not game-verified"), "{}", p.label);
        assert_eq!(&p.wav[..4], b"RIFF");
        let data_len = u32::from_le_bytes([p.wav[40], p.wav[41], p.wav[42], p.wav[43]]);
        assert_eq!(data_len, p.num_samples * 2, "trimmed to the SNR sample count");
        assert_eq!(p.sns_len, good.len() as u64);
        let first = i16::from_le_bytes([p.wav[44], p.wav[45]]);
        if r == PreviewRendering::CandidateA {
            assert_eq!(first, -3200, "candidate A starts with the embedded PCM");
        } else {
            assert!(first.abs() <= 1, "MPEG-only starts with the decoded (silent) frame 0");
        }
        assert!(p.notes.iter().any(|n| n.contains("Replacement is not supported")));
    }
    // Refusals name the codec / the grammar violation; an unknown id and a duplicate id refuse.
    let e = preview_sample("nhl17", Platform::Ps4, &root, "pbp_samples.sbr", 360, PreviewRendering::CandidateA).unwrap_err().to_string();
    assert!(e.contains("ea-opus") && e.contains("no preview route"), "{e}");
    let e = preview_sample("nhl17", Platform::Ps4, &root, "pbp_samples.sbr", 5587, PreviewRendering::CandidateA).unwrap_err().to_string();
    assert!(e.contains("grammar refused") && e.contains("sum of declared block samples"), "{e}");
    assert!(preview_sample("nhl17", Platform::Ps4, &root, "pbp_samples.sbr", 999, PreviewRendering::CandidateA).unwrap_err().to_string().contains("no sample 999"));
    // Nothing was written.
    assert_eq!(std::fs::read(dir.join("pbp_samples.sbr")).unwrap(), before_bank);
    assert_eq!(std::fs::read(dir.join("pbp_samples.sbs")).unwrap(), before_sbs);
    let _ = Path::new("");
}

/// The real NHL 17 PS4 extracted tree (research R71's bank): every bank of the speech directory
/// lists, the play-by-play bank's census is measured from the headers, and the first R71 vector
/// previews in both renderings. Read only; the tree is never written.
#[test]
#[ignore = "real PS4 NHL 17 extracted tree under H:/Emulators/ShadPS4/Games/CUSA04326/databigs/Extracted (read only): select with --ignored"]
fn real_nhl17_banks_list_and_the_r71_vector_previews() {
    let root = Path::new(r"H:\Emulators\ShadPS4\Games\CUSA04326\databigs\Extracted");
    assert!(root.is_dir(), "{} missing (fails, never skips)", root.display());
    let l = list_banks("nhl17", Platform::Ps4, root).unwrap();
    eprintln!("{}: {} banks", l.speech_dir, l.banks.len());
    for b in &l.banks {
        eprintln!("  {} — {} DSETs, {} samples, sbs {:?}{}", b.file, b.dsets, b.samples, b.sbs_len, b.note.as_deref().map(|n| format!(" ({n})")).unwrap_or_default());
    }
    let pbp = l.banks.iter().find(|b| b.file == "pbp_samples.sbr").expect("pbp_samples.sbr");
    assert!(pbp.samples >= 3863, "{pbp:?}");
    let s = list_samples("nhl17", Platform::Ps4, root, "pbp_samples.sbr", "", 5000).unwrap();
    eprintln!("census: {:?} (total {}, truncated {})", s.codec_census, s.total, s.truncated);
    assert!(s.codec_census.iter().any(|(c, n)| c == "ea-mp3" && *n > 0));
    for r in [PreviewRendering::CandidateA, PreviewRendering::MpegOnly] {
        let p = preview_sample("nhl17", Platform::Ps4, root, "pbp_samples.sbr", 15360, r).unwrap();
        eprintln!("15360 {:?}: {} blocks, {} frames {:?}, trim {}, first>1LSB {:?}, wav {} bytes — {}", r, p.blocks, p.frames, p.frames_per_block, p.trim, p.first_sample_over_1_lsb, p.wav.len(), p.label);
        assert_eq!((p.sample_rate, p.channels, p.embedded_pcm_samples), (48000, 1, 1152));
        assert_eq!(p.wav.len() as u32, 44 + 2 * p.num_samples);
    }
}
