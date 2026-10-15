//! P4.3 (overnight 2026-09-24) — the gen7 EALayer3 v1 diagnostic preview through the domain on the
//! real loose trees (read only): X360 NHL 15 `pbp_eng` (44.1 kHz mono, R77) and PS3 NHL 15 `ann_eng`
//! (24 kHz mono, R80). The bank's first sample previews; the coverage gate refuses a tuple outside
//! the measured set and a bank stem the speech table does not index. Nothing is written.
use std::path::Path;

use studio_domain::gen7_speech_preview::{bank_entries, coverage, list_bank_samples, preview_sample};
use studio_domain::Platform;

#[test]
#[ignore = "real gen7 trees under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn real_x360_nhl15_pbp_and_ps3_nhl15_ann_samples_preview_through_the_domain() {
    for (game, platform, root, stem) in [
        ("nhl15", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15", "pbp_eng"),
        ("nhl15", Platform::Ps3, r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 15\PS3_GAME\USRDIR", "ann_eng"),
        // Banks that ship only inside archives (R80 sources): PS3 13 pbp in audioboot.big (chunkzip), PS3 09 ann in data0.big (RefPack).
        ("nhl13", Platform::Ps3, r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 13\PS3_GAME\USRDIR", "pbp_eng"),
        ("nhl09", Platform::Ps3, r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 09\PS3_GAME\USRDIR", "ann_eng"),
        // Music banks (R80: 44.1 kHz stereo; SBKR table under audio/music, measured 24 Sep 2026).
        ("nhl15", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15", "music_samples"),
        ("nhl15", Platform::Ps3, r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 15\PS3_GAME\USRDIR", "music_samples"),
    ] {
        let root = Path::new(root);
        assert!(root.is_dir(), "{}: missing (fails, never skips)", root.display());
        coverage(game, platform, stem).unwrap();
        // The bank's first sample by stream offset, through the route's own resolution (loose or archived).
        let (source, bank, _) = bank_entries(game, platform, root, stem).unwrap_or_else(|e| panic!("{game} {platform:?} {stem}: {e}"));
        eprintln!("{game} {platform:?} {stem}: bank source {} — {} samples", source.describe(), bank.samples.len());
        let loose = if stem == "music_samples" { root.join("audio/music/music_samples.sbr") } else { root.join("audio/speech").join(format!("{stem}_samples.sbr")) };
        if !loose.is_file() {
            assert!(matches!(source, studio_domain::gen7_speech_preview::SourceLocation::ArchiveMember { .. }), "{}", source.describe());
        }
        let first_id = bank.samples.iter().min_by_key(|s| s.sbs_offset).unwrap().id;
        let p = preview_sample(game, platform, root, stem, first_id).unwrap_or_else(|e| panic!("{game} {platform:?} {stem} sample {first_id}: {e}"));
        eprintln!("{game} {platform:?} {stem}: bank {} · stream {}", p.bank_path, p.stream_path);
        eprintln!("{game} {platform:?} {stem} sample {}: {} Hz {} ch, {} blocks, {} granules {:?}, {} frames (mdb ≤ {}), SNR {} samples, decoded {} / channel, surplus {}, first>1LSB {:?}, wav {} B — {}", p.sample_id, p.sample_rate, p.channels, p.blocks, p.granules, p.granules_per_block, p.rebuilt_frames, p.max_main_data_begin, p.num_samples, p.decoded_samples_per_channel, p.surplus_samples, p.first_sample_over_1_lsb, p.wav.len(), p.label);
        assert_eq!(p.codec, "ealayer3-v1");
        let (expect_rate, expect_ch) = match (game, platform, stem) {
            (_, _, "music_samples") => (44100, 2),
            ("nhl15", Platform::Xbox, _) => (44100, 1),
            _ => (24000, 1),
        };
        assert_eq!((p.sample_rate, p.channels), (expect_rate, expect_ch));
        assert_eq!(p.pcm_frame_samples, 47);
        assert_eq!(p.decoded_samples_per_channel, p.granules * 576);
        assert_eq!(p.wav.len(), 44 + 2 * p.decoded_samples_per_channel * p.channels as usize);
        assert!(p.label.contains("not game-verified"));
        // An id the bank does not carry refuses by name.
        let e = preview_sample(game, platform, root, stem, u32::MAX).unwrap_err().to_string();
        assert!(e.contains("is not in the bank"), "{e}");
    }
    // The listing: every music sample of the X360 NHL 15 bank with its header; census from the headers.
    let l = list_bank_samples("nhl15", Platform::Xbox, Path::new(r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15"), "music_samples", "", 1000).unwrap();
    eprintln!("music listing: {} total, census {:?}, first rows {:?}", l.total, l.codec_census, l.rows.iter().take(3).map(|r| (r.sample_id, r.offset, r.codec.clone(), r.channels, r.previewable)).collect::<Vec<_>>());
    assert_eq!(l.total, 263, "R80: 263 music entries on NHL 15");
    assert!(l.rows.iter().all(|r| r.problem.is_none()), "{:?}", l.rows.iter().filter(|r| r.problem.is_some()).take(3).collect::<Vec<_>>());
    assert!(l.codec_census.iter().any(|(c, _)| c == "ealayer3-v1"));
    let f = list_bank_samples("nhl15", Platform::Xbox, Path::new(r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15"), "music_samples", "634", 10).unwrap();
    assert!(f.rows.iter().any(|r| r.sample_id == 634) && f.matching < l.total);
    // X360 NHL 11 and PS3 NHL 14 play-by-play are EA-Speex, admitted since receipt 29 (R83; their own
    // cases are in `easpeex_preview.rs`); outside every measured tuple: refused, never guessed.
    assert_eq!(coverage("nhl11", Platform::Xbox, "pbp_eng").unwrap().source, "R83");
    assert_eq!(coverage("nhl14", Platform::Ps3, "pbp_eng").unwrap().source, "R83");
    // X360 NHL 09 / 10 speech and 09–12 music are EA-XMA, admitted since receipt 33 (R84; their own
    // cases are in `eaxma_speech_preview.rs`).
    assert_eq!(coverage("nhl10", Platform::Xbox, "pbp_eng").unwrap().source, "R84", "X360 NHL 10 speech is EA-XMA");
    assert_eq!(coverage("nhl11", Platform::Xbox, "music_samples").unwrap().source, "R84", "X360 09–12 music is EA-XMA");
}
