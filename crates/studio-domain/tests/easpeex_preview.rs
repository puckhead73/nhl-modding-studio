//! EA-Speex diagnostic previews through the domain (receipt 29) on the real trees, read only: every
//! admitted gen7 tuple (X360 NHL 11–Legacy `ann_eng` — NHL 14 since receipt 31, once the SBKR reader
//! skipped its eight all-zero slots — X360 NHL 11–14 `pbp_eng`, PS3 NHL 14 `pbp_eng`)
//! and the PS4 NHL 15–21 rink-announcer banks. Per tuple: the listing marks EA-Speex rows
//! previewable; the first sample by stream offset decodes to exactly its SNR count at 32 kHz mono
//! under the fixed label; the NHL 21 base single-record bank carries R93's disclosure; Legacy
//! Recompiled and a non-admitted tuple refuse by name. The decoder-level PCM comparison with the
//! libspeex reference path is `tdb-snd/tests/ea_speex_real_vectors.rs`. Nothing is written.
use std::path::Path;

use studio_domain::gen7_speech_preview::{coverage, list_bank_samples, preview_sample as gen7_preview};
use studio_domain::ps4_speech_bank::{list_banks, list_samples, preview_sample as ps4_preview, PreviewRendering};
use studio_domain::Platform;

const GEN7: &[(&str, Platform, &str, &str)] = &[
    ("nhl11", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL 11\NHL 11", "ann_eng"),
    ("nhl12", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL 12\NHL 12", "ann_eng"),
    ("nhl13", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL 13\NHL 13", "ann_eng"),
    ("nhl14", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL 14\NHL 14", "ann_eng"),
    ("nhl15", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15", "ann_eng"),
    ("nhllegacy", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy", "ann_eng"),
    ("nhl11", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL 11\NHL 11", "pbp_eng"),
    ("nhl12", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL 12\NHL 12", "pbp_eng"),
    ("nhl13", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL 13\NHL 13", "pbp_eng"),
    ("nhl14", Platform::Xbox, r"X:\Games\Emulation\Roms\XBOX360\NHL 14\NHL 14", "pbp_eng"),
    ("nhl14", Platform::Ps3, r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 14\PS3_GAME\USRDIR", "pbp_eng"),
];

const PS4: &[(&str, &str)] = &[
    ("nhl15", "CUSA00561"),
    ("nhl16", "CUSA01909"),
    ("nhl17", "CUSA04326"),
    ("nhl18", "CUSA07569"),
    ("nhl19", "CUSA11126"),
    ("nhl20", "CUSA15439"),
    ("nhl21", "CUSA18182"),
];

#[test]
fn recompiled_and_unmeasured_tuples_refuse_by_name() {
    let e = coverage("nhllegacy-recompiled", Platform::Recomp, "ann_eng").unwrap_err().to_string();
    assert!(e.contains("EA-Speex (R83)"), "{e}");
    assert_eq!(coverage("nhl10", Platform::Xbox, "ann_eng").unwrap().source, "R84", "X360 NHL 10 speech is EA-XMA (R84), not EA-Speex");
    assert_eq!(coverage("nhl13", Platform::Xbox, "pbp_eng").unwrap().source, "R83");
    assert_eq!(coverage("nhl14", Platform::Xbox, "ann_eng").unwrap().source, "R83", "X360 NHL 14 ann_eng admitted since receipt 31");
}

#[test]
#[ignore = "real gen7 trees under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn every_admitted_gen7_speex_tuple_lists_and_previews_its_first_sample() {
    let mut failed = Vec::new();
    for (game, platform, root, stem) in GEN7 {
        let root = Path::new(root);
        assert!(root.is_dir(), "{}: missing (fails, never skips)", root.display());
        let r = std::panic::catch_unwind(|| {
            let l = list_bank_samples(game, *platform, root, stem, "", 1).unwrap_or_else(|e| panic!("{e}"));
            let first = &l.rows[0];
            assert_eq!(first.codec.as_deref(), Some("ea-speex"), "first row codec");
            assert!(first.previewable, "EA-Speex rows are previewable");
            let p = gen7_preview(game, *platform, root, stem, first.sample_id).unwrap_or_else(|e| panic!("sample {}: {e}", first.sample_id));
            assert_eq!((p.codec.as_str(), p.sample_rate, p.channels), ("ea-speex", 32000, 1));
            assert_eq!(p.decoded_samples_per_channel as u32, p.num_samples, "exactly the SNR count");
            assert_eq!(p.wav.len(), 44 + 2 * p.num_samples as usize);
            assert!(p.label.contains("not game-verified"));
            assert_eq!(p.tuple_source, "R83");
            let detail = p.route_detail.clone().unwrap();
            assert!(detail.contains("first 509") && detail.contains("speex-1.2.1"), "{detail}");
            format!("sample {} @ {}: {}", p.sample_id, p.offset, detail)
        });
        match r {
            Ok(line) => eprintln!("{game} {platform:?} {stem}: {line}"),
            Err(e) => {
                let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                eprintln!("{game} {platform:?} {stem}: FAIL {msg}");
                failed.push(format!("{game} {platform:?} {stem}"));
            }
        }
    }
    assert!(failed.is_empty(), "{failed:?}");
}

#[test]
#[ignore = "real PS4 extracted installs under H:/Emulators/ShadPS4/Games (read only): select with --ignored"]
fn ps4_rink_announcer_banks_list_and_preview_on_every_title() {
    for (game, cusa) in PS4 {
        let root = std::path::PathBuf::from(format!(r"H:\Emulators\ShadPS4\Games\{cusa}\databigs\Extracted"));
        assert!(root.is_dir(), "{}: missing (fails, never skips)", root.display());
        let banks = list_banks(game, Platform::Ps4, &root).unwrap_or_else(|e| panic!("{game}: {e}"));
        for bank in ["rink_ann_samples.sbr", "rink_ann_samples_inxp.sbr"] {
            let b = banks.banks.iter().find(|b| b.file == bank).unwrap_or_else(|| panic!("{game}: {bank} not listed"));
            assert!(b.sbs_present && b.samples > 0, "{game} {bank}: {b:?}");
            let l = list_samples(game, Platform::Ps4, &root, bank, "", 5).unwrap();
            let row = l.rows.iter().min_by_key(|r| r.offset).unwrap();
            assert_eq!(row.codec.as_deref(), Some("ea-speex"), "{game} {bank}");
            assert!(row.previewable, "{game} {bank}: previewable");
            let p = ps4_preview(game, Platform::Ps4, &root, bank, row.sid, PreviewRendering::MpegOnly).unwrap_or_else(|e| panic!("{game} {bank} {}: {e}", row.sid));
            assert_eq!((p.codec.as_str(), p.sample_rate, p.channels), ("ea-speex", 32000, 1));
            assert_eq!(p.wav.len(), 44 + 2 * p.num_samples as usize, "{game} {bank}: exactly the SNR count");
            assert!(p.label.contains("not game-verified"));
            let r93 = p.notes.iter().any(|n| n.contains("R93"));
            assert_eq!(r93, *game == "nhl21" && bank == "rink_ann_samples.sbr", "{game} {bank}: R93 disclosure only on the NHL 21 base single-record bank");
            eprintln!("{game} {bank} sid {} @ {}: {} frames {:?}, trim {}, {} samples{}", p.sid, p.offset, p.frames, p.frames_per_block, p.trim, p.num_samples, if r93 { " — R93 disclosure" } else { "" });
        }
    }
}
