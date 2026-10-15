//! Receipt 50 — research 2026-09-28 handoff 05 (R139 / R139b / R139c) consumed: X360 NHL 10 → Legacy EA-XMA ram records
//! that Studio refused now preview through the shipped helper, each segment / chunk at exactly its declared count:
//!   X1 the (frame count 0, offset 0x7FFF) "no frame begins here" packet (the defect: a valid marker was refused);
//!   X2 two-segment looping stereo records (intro + loop body, decoded separately, loop at segment 0's end);
//!   X3 single-segment looping stereo / four-channel records (played once);
//!   X4 four- / six-channel chains of stereo chunks (one pair at a time; no speaker mapping).
//! Real loose trees, read only; the helper is named by `NHL_STUDIO_XMA2_HELPER` (the pinned `dc3533ed…` build).
use std::path::PathBuf;

use studio_domain::gen7_sfx_banks::{bank_tree, list_banks, preview_part, RECOMP_PREVIEW_NOTE};
use studio_domain::Platform;

fn x(d: &str) -> PathBuf {
    PathBuf::from(format!(r"X:\Games\Emulation\Roms\XBOX360\{d}"))
}

#[test]
#[ignore = "real X360 trees under X:/Games/Emulation/Roms and an XMA2 helper named by NHL_STUDIO_XMA2_HELPER: select with --ignored"]
fn research_05_vectors_decode_to_their_declared_counts() {
    assert!(std::env::var_os("NHL_STUDIO_XMA2_HELPER").is_some(), "NHL_STUDIO_XMA2_HELPER names the helper for this case");
    let r10 = x(r"NHL 10\NHL 10");
    let leg = x(r"NHL Legacy\NHL Legacy");
    assert!(r10.is_dir() && leg.is_dir(), "missing trees (fails, never skips)");

    // X3: a single-segment looping stereo record, played once.
    let p = preview_part("nhl10", Platform::Xbox, &r10, "000_chant.abk", 1, None).unwrap();
    assert_eq!((p.channels, p.num_samples, p.decoded_frames_per_channel, p.loop_start), (2, 48_888, 48_888, None));
    assert!(p.notes.iter().any(|n| n.contains("played once")), "{:?}", p.notes);

    // X2: 4 437 + 48 138 = 52 575, loop at 4 437; the segments decode separately (the concatenated-packet decode
    // research measured gives 53 258 and is not used).
    let p = preview_part("nhl10", Platform::Xbox, &r10, "073_chant.abk", 0, None).unwrap();
    assert_eq!((p.channels, p.num_samples, p.decoded_frames_per_channel, p.loop_start), (2, 52_575, 52_575, Some(4_437)));
    assert!(p.source.contains("2 segment(s)"), "{}", p.source);

    // X4: channel config 3 = two stereo pairs of 90 452; pair 3 does not exist.
    for k in 0..2u16 {
        let p = preview_part("nhl10", Platform::Xbox, &r10, "002_chant.abk", 1, Some(k)).unwrap();
        assert_eq!((p.channels, p.sub_stream, p.sub_streams, p.decoded_frames_per_channel), (2, Some(k), 2, 90_452));
        assert!(p.notes.iter().any(|n| n.contains("which speakers a pair feeds is not established")));
    }
    assert!(preview_part("nhl10", Platform::Xbox, &r10, "002_chant.abk", 1, Some(2)).unwrap_err().to_string().contains("pair 3 of 2"));

    // X1 inside X4: `002_chant.abk` #0 pair 2 has a (0, 0x7FFF) packet (packet 81) — refused before, exact now.
    for (root, game) in [(&r10, "nhl10"), (&leg, "nhllegacy")] {
        let p = preview_part(game, Platform::Xbox, root, "002_chant.abk", 0, Some(1)).unwrap_or_else(|e| panic!("{game}: {e}"));
        assert_eq!(p.decoded_frames_per_channel, 266_690, "{game}");
    }
    // X1 on a single-stream record: `002_goal.abk` #0 (stereo, R139b: 352 323 with the marker kept).
    let p = preview_part("nhl10", Platform::Xbox, &r10, "002_goal.abk", 0, None).unwrap();
    assert_eq!(p.decoded_frames_per_channel, 352_323);
}

/// Every NHL 10 bank: the tree admits the measured shapes; for each admitted ram shape class the first record
/// previews at exactly its declared count; the six-channel records preview per pair; an unmeasured title (NHL 09)
/// keeps its own rules.
#[test]
#[ignore = "real X360 NHL 10 tree under X:/Games/Emulation/Roms and an XMA2 helper named by NHL_STUDIO_XMA2_HELPER: select with --ignored"]
fn nhl10_sweep_admits_the_measured_shapes_and_each_class_decodes() {
    assert!(std::env::var_os("NHL_STUDIO_XMA2_HELPER").is_some(), "NHL_STUDIO_XMA2_HELPER names the helper for this case");
    let r10 = x(r"NHL 10\NHL 10");
    let banks = list_banks("nhl10", Platform::Xbox, &r10).unwrap();
    let mut first: std::collections::BTreeMap<(String, u8), (String, u16, Option<usize>, u32)> = Default::default();
    let (mut ram, mut admitted, mut refused) = (0, 0, std::collections::BTreeMap::<String, usize>::new());
    for b in &banks.banks {
        let t = bank_tree("nhl10", Platform::Xbox, &r10, &b.name).unwrap();
        for e in t.pickers.iter().flat_map(|p| &p.entries).filter(|e| e.kind.starts_with("ram") && e.codec_code == 0x03) {
            ram += 1;
            if e.previewable {
                admitted += 1;
                first.entry((e.kind.clone(), e.channel_config)).or_insert((b.name.clone(), e.record, e.sub_streams, e.num_samples));
            } else {
                let why = e.preview_refusal.clone().unwrap_or_default();
                *refused.entry(why.split(" — ").next().unwrap_or("").chars().take(90).collect()).or_default() += 1;
            }
        }
    }
    eprintln!("NHL 10 EA-XMA ram entries (picker entries): {ram}, admitted {admitted}; refusal classes {refused:#?}");
    assert!(first.contains_key(&("ram".into(), 5)), "a six-channel record is admitted: {first:?}");
    for ((kind, cc), (bank, record, subs, n)) in &first {
        let pairs = subs.unwrap_or(1).max(1) as u16;
        for k in 0..pairs {
            let pick = if *cc == 3 || *cc == 5 { Some(k) } else { None };
            let p = preview_part("nhl10", Platform::Xbox, &r10, bank, *record, pick).unwrap_or_else(|e| panic!("{kind} cc {cc} {bank} #{record} pair {k}: {e}"));
            assert_eq!(p.decoded_frames_per_channel as u32, *n, "{kind} cc {cc} {bank} #{record}");
            if pick.is_none() {
                break;
            }
        }
        eprintln!("{kind} cc {cc}: {bank} #{record} ({n} samples) exact");
    }
}

/// Receipt 53 — research 2026-09-28 handoff 06 (R140): NHL 11 / 13 / 14 `.ast` stream records (and NHL 12 loops / pairs) preview
/// at exactly their declared counts: S1 non-looping mono / stereo, S2 looping stereo (stub + body), S3 four- / six-channel pairs.
#[test]
#[ignore = "real X360 trees under X:/Games/Emulation/Roms and an XMA2 helper named by NHL_STUDIO_XMA2_HELPER: select with --ignored"]
fn research_06_stream_vectors_decode_to_their_declared_counts() {
    assert!(std::env::var_os("NHL_STUDIO_XMA2_HELPER").is_some(), "NHL_STUDIO_XMA2_HELPER names the helper for this case");
    let r11 = x(r"NHL 11\NHL 11");
    let r13 = x(r"NHL 13\NHL 13");
    let r14 = x(r"NHL 14\NHL 14");
    // S1
    let p = preview_part("nhl11", Platform::Xbox, &r11, "colors1.abk", 0, None).unwrap();
    assert_eq!((p.channels, p.decoded_frames_per_channel), (1, 54_000));
    for (game, root) in [("nhl13", &r13), ("nhl14", &r14)] {
        let p = preview_part(game, Platform::Xbox, root, "ambientskatesandpunches.abk", 0, None).unwrap_or_else(|e| panic!("{game}: {e}"));
        assert_eq!((p.channels, p.decoded_frames_per_channel), (2, 28_496), "{game}");
        let p = preview_part(game, Platform::Xbox, root, "ambientskatesandpunches.abk", 3, None).unwrap();
        assert_eq!((p.channels, p.decoded_frames_per_channel), (1, 17_068), "{game}");
    }
    // S2: stub + body.
    let p = preview_part("nhl11", Platform::Xbox, &r11, "crowds.abk", 6, None).unwrap();
    assert_eq!((p.loop_start, p.decoded_frames_per_channel), (Some(26), 26 + 113_266));
    let p = preview_part("nhl13", Platform::Xbox, &r13, "ambientskatesandpunches.abk", 207, None).unwrap();
    assert_eq!((p.loop_start, p.decoded_frames_per_channel), (Some(32), 32 + 815_616));
    // S3: pairs.
    for k in 0..2u16 {
        let p = preview_part("nhl11", Platform::Xbox, &r11, "crowds.abk", 13, Some(k)).unwrap();
        assert_eq!((p.channels, p.sub_streams, p.decoded_frames_per_channel), (2, 2, 952_896));
        let p = preview_part("nhl11", Platform::Xbox, &r11, "crowds.abk", 14, Some(k)).unwrap();
        assert_eq!((p.loop_start, p.decoded_frames_per_channel), (Some(24), 24 + 691_200));
    }
    for k in 0..3u16 {
        let p = preview_part("nhl13", Platform::Xbox, &r13, "sfx.abk", 206, Some(k)).unwrap_or_else(|e| panic!("pair {k}: {e}"));
        assert_eq!((p.channels, p.sub_streams, p.decoded_frames_per_channel), (2, 3, 455_519));
    }
}

/// Receipt 56 — research 2026-09-28 handoff 06 §4 addendum (R140b): NHL 10 / 15 / Legacy stream shapes beyond mono / stereo
/// (stereo loop, four-channel with and without a loop, six-channel on 15 / Legacy) preview at their declared counts.
#[test]
#[ignore = "real X360 trees under X:/Games/Emulation/Roms and an XMA2 helper named by NHL_STUDIO_XMA2_HELPER: select with --ignored"]
fn research_06_addendum_stream_vectors_decode_to_their_declared_counts() {
    assert!(std::env::var_os("NHL_STUDIO_XMA2_HELPER").is_some(), "NHL_STUDIO_XMA2_HELPER names the helper for this case");
    let r10 = x(r"NHL 10\NHL 10");
    assert!(r10.is_dir(), "{}: missing (fails, never skips)", r10.display());
    // Stereo loop, stub + body.
    let p = preview_part("nhl10", Platform::Xbox, &r10, "crowds.abk", 198, None).unwrap();
    assert_eq!((p.channels, p.loop_start, p.decoded_frames_per_channel), (2, Some(23), 23 + 100_681));
    for k in 0..2u16 {
        // Four-channel loop and four-channel, one stereo pair at a time.
        let p = preview_part("nhl10", Platform::Xbox, &r10, "crowds.abk", 121, Some(k)).unwrap();
        assert_eq!((p.sub_streams, p.loop_start, p.decoded_frames_per_channel), (2, Some(21), 21 + 500_650));
        let p = preview_part("nhl10", Platform::Xbox, &r10, "crowds.abk", 144, Some(k)).unwrap();
        assert_eq!((p.sub_streams, p.decoded_frames_per_channel), (2, 847_018));
    }
    for (game, root) in [("nhl15", x(r"NHL 15\NHL 15")), ("nhllegacy", x(r"NHL Legacy\NHL Legacy"))] {
        assert!(root.is_dir(), "{}: missing (fails, never skips)", root.display());
        let p = preview_part(game, Platform::Xbox, &root, "ambientskatesandpunches.abk", 132, None).unwrap_or_else(|e| panic!("{game}: {e}"));
        assert_eq!((p.channels, p.loop_start, p.decoded_frames_per_channel), (2, Some(32), 32 + 815_616), "{game}");
        for k in 0..2u16 {
            let p = preview_part(game, Platform::Xbox, &root, "crowds.abk", 0, Some(k)).unwrap();
            assert_eq!((p.sub_streams, p.loop_start, p.decoded_frames_per_channel), (2, Some(24), 24 + 563_232), "{game}");
            let p = preview_part(game, Platform::Xbox, &root, "crowds.abk", 83, Some(k)).unwrap();
            assert_eq!((p.sub_streams, p.decoded_frames_per_channel), (2, 952_896), "{game}");
        }
        for k in 0..3u16 {
            let p = preview_part(game, Platform::Xbox, &root, "sfx.abk", 441, Some(k)).unwrap_or_else(|e| panic!("{game} pair {k}: {e}"));
            assert_eq!((p.channels, p.sub_streams, p.decoded_frames_per_channel), (2, 3, 455_519), "{game}");
        }
    }
}

/// Receipt 59 — research 2026-09-28 handoff 04 (R138): a Legacy Recompiled project (catalogue id `legacyrecomp`) lists its banks from
/// its own archives — the same banks, records and stream-id checks as the X360 Legacy loose tree — and a ram, a stream, a looping
/// stream and a surround pair decode to exactly the X360 Legacy PCM, labelled. The install is only read.
#[test]
#[ignore = "the Recompiled install under H:/Emulators/games/XBOX, the X360 Legacy tree under X:/Games/Emulation/Roms and an XMA2 helper named by NHL_STUDIO_XMA2_HELPER: select with --ignored"]
fn recompiled_banks_are_the_legacy_banks_from_its_own_archives() {
    assert!(std::env::var_os("NHL_STUDIO_XMA2_HELPER").is_some(), "NHL_STUDIO_XMA2_HELPER names the helper for this case");
    let rec = PathBuf::from(r"H:\Emulators\games\XBOX\NHL Legacy Decomp Test");
    let leg = x(r"NHL Legacy\NHL Legacy");
    assert!(rec.join("game").is_dir() && leg.is_dir(), "missing trees (fails, never skips)");
    let lr = list_banks("legacyrecomp", Platform::Recomp, &rec).unwrap();
    let ll = list_banks("nhllegacy", Platform::Xbox, &leg).unwrap();
    let shape = |l: &studio_domain::gen7_sfx_banks::Gen7SfxBankList| -> Vec<(String, usize, usize, usize, Option<bool>, bool)> {
        let mut v: Vec<_> = l.banks.iter().map(|b| (b.name.to_ascii_lowercase(), b.records, b.stream_records, b.ram_records, b.ast_id_matches, b.problem.is_none())).collect();
        v.sort();
        v
    };
    assert_eq!(shape(&lr), shape(&ll), "the same banks, records and stream ids");
    assert!(lr.banks.iter().all(|b| b.path.contains(".big:")), "every bank is an archive member");
    // The install's `game` folder attached directly lists the same banks.
    assert_eq!(list_banks("legacyrecomp", Platform::Recomp, &rec.join("game")).unwrap().banks.len(), lr.banks.len());
    for (bank, record, pair) in [
        ("playercalls.abk", 0u16, None),
        ("002_chant.abk", 0, Some(1u16)),
        ("ambientskatesandpunches.abk", 132, None),
        ("crowds.abk", 0, Some(1)),
        ("sfx.abk", 441, Some(2)),
    ] {
        let a = preview_part("legacyrecomp", Platform::Recomp, &rec, bank, record, pair).unwrap_or_else(|e| panic!("recompiled {bank} #{record}: {e}"));
        let b = preview_part("nhllegacy", Platform::Xbox, &leg, bank, record, pair).unwrap();
        assert!(a.wav == b.wav, "{bank} #{record} {pair:?}: Recompiled PCM = X360 Legacy PCM");
        assert_eq!(a.bank_sha256, b.bank_sha256, "{bank}: the bank bytes are the X360 Legacy file's");
        assert!(a.notes.iter().any(|n| n == RECOMP_PREVIEW_NOTE), "{:?}", a.notes);
        eprintln!("{bank} #{record} {pair:?}: {} frames, {} — identical", a.decoded_frames_per_channel, a.source);
    }
    // The Recompiled id on another platform is not a population.
    assert!(list_banks("legacyrecomp", Platform::Xbox, &leg).is_err());
}
