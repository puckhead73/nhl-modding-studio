//! The R71 vectors (24: NHL 15 / 16 / 17 PS4, 8 each) through `tdb-snd`: the `.sbr` bank's largest
//! DSET yields the records, the `.sbs` block stream at each `.OFF` hashes to the research
//! `sns_sha256`, the contract parser accepts the unmodified sample with the research frame counts
//! and refuses the dropped-block / truncation mutations, and the Symphonia decode yields
//! `frames × 1152` samples whose first-above-1-LSB index and RMS profile match the research
//! measurements (decoder-independent checks — the research hashes are ffmpeg's fixed / float
//! outputs and are recorded, not required). Candidate A is assembled and labelled diagnostic.
//!
//! Read-only over the extracted PS4 banks on `H:` (paths from the research JSON); nothing written.
//! Fails loudly when a bank is missing — never a silent skip.
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tdb_snd::{contract_parse, decode_layer3, render_candidate_a, walk_sample, Rendering, Sbr};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn read_window(path: &Path, off: u64, len: usize) -> Vec<u8> {
    let mut f = File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    f.seek(SeekFrom::Start(off)).unwrap();
    let mut buf = vec![0u8; len];
    let n = f.read(&mut buf).unwrap();
    buf.truncate(n);
    buf
}

fn rms(x: &[i16]) -> f64 {
    if x.is_empty() {
        return 0.0;
    }
    (x.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / x.len() as f64).sqrt()
}

#[test]
#[ignore = "real PS4 banks under H:\\Emulators\\ShadPS4\\Games\\CUSA*\\databigs\\Extracted (read only): select with --ignored"]
fn the_r71_vectors_parse_hash_decode_and_refuse_mutations() {
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(repo("docs/research/overnight-2026-09-22-pm/data/r71-ea-mp3-embedded-pcm.json")).unwrap()).unwrap();
    let mut checked = 0;
    let mut fixed_exact = 0;
    let mut float_exact = 0;
    for (title, t) in json["titles"].as_object().unwrap() {
        let sbr_path = PathBuf::from(t["bank"].as_str().unwrap());
        let sbs_path = sbr_path.with_extension("sbs");
        assert!(sbr_path.is_file() && sbs_path.is_file(), "{title}: {} / {} missing (fails, never skips)", sbr_path.display(), sbs_path.display());
        let bank = Sbr::parse(&std::fs::read(&sbr_path).unwrap()).unwrap_or_else(|e| panic!("{title}: {e}"));
        assert!(bank.little_endian, "{title}: PS4 banks are SBle");
        let dset = bank.largest_dset().expect("a DSET");
        let sids = dset.column(".SID").expect(".SID decodes");
        let offs = dset.column(".OFF").expect(".OFF decodes");
        eprintln!("{title}: bank {} — {} DSETs, largest {} records, stream path {:?}", sbr_path.file_name().unwrap().to_string_lossy(), bank.dsets.len(), dset.records, bank.stream_path);
        for v in t["vectors"].as_array().unwrap() {
            let sid = v["sid"].as_u64().unwrap();
            let off = v["off"].as_u64().unwrap();
            let i = sids.iter().position(|&s| s == sid).unwrap_or_else(|| panic!("{title}: sid {sid} not in the bank"));
            assert_eq!(offs[i], off, "{title} {sid}: the bank's .OFF is the research offset");
            let window = read_window(&sbs_path, off, 8 << 20);
            let (snr, blocks, end) = walk_sample(&window, 0).unwrap_or_else(|e| panic!("{title} {sid}: {e}"));
            let raw = &window[..end as usize];
            let sha = format!("{:x}", Sha256::digest(raw));
            assert_eq!(sha, v["sns_sha256"].as_str().unwrap(), "{title} {sid}: SNS bytes");
            assert_eq!(snr.codec, 0x0b);
            assert_eq!((snr.sample_rate, snr.channel_config), (48000, 0), "{title} {sid}");
            assert_eq!(snr.num_samples as u64, v["num_samples"].as_u64().unwrap());
            assert_eq!(blocks.len() as u64, v["blocks"].as_u64().unwrap());
            tdb_snd::eamp3::check_header(&snr).unwrap();
            let refs: Vec<(u32, &[u8])> = blocks.iter().map(|b| (b.samples, b.payload.as_slice())).collect();
            let parsed = contract_parse(&refs, snr.num_samples).unwrap_or_else(|e| panic!("{title} {sid}: {e}"));
            assert_eq!(parsed.total_frames as u64, v["total_frames"].as_u64().unwrap(), "{title} {sid}: frames");
            assert_eq!(parsed.trim as u64, v["trim"].as_u64().unwrap(), "{title} {sid}: trim");
            assert_eq!(parsed.embedded_pcm.len(), 1152);
            assert_eq!(parsed.mpeg.len() as u64, v["robustness"]["unmodified"]["mpeg_bytes"].as_u64().unwrap(), "{title} {sid}: MPEG bytes");
            // Mutations that the research refused: a dropped middle block (when applicable), a truncated last block.
            if refs.len() > 2 {
                let dropped: Vec<(u32, &[u8])> = refs.iter().copied().enumerate().filter(|(k, _)| *k != 1).map(|(_, r)| r).collect();
                let e = contract_parse(&dropped, snr.num_samples).unwrap_err();
                assert!(matches!(e, tdb_snd::EaMp3Refusal::BlockSamples { .. }), "{title} {sid}: {e}");
            }
            let mut trunc = refs.clone();
            let (n, last) = trunc.pop().unwrap();
            trunc.push((n, &last[..last.len() - 3]));
            assert!(contract_parse(&trunc, snr.num_samples).is_err(), "{title} {sid}: truncated last block must refuse");

            // Decode.
            let dec = decode_layer3(&parsed.mpeg).unwrap_or_else(|e| panic!("{title} {sid}: decode: {e}"));
            assert_eq!((dec.sample_rate, dec.channels, dec.frames), (48000, 1, parsed.total_frames));
            assert_eq!(dec.pcm.len(), parsed.total_frames * 1152);
            let first = dec.pcm.iter().position(|&x| x.abs() > 1).map(|p| p as i64);
            let research_first = v["first_sample_over_1_lsb"].as_i64();
            if let (Some(a), Some(b)) = (first, research_first) {
                // A 1-LSB threshold crossing is decoder-rounding sensitive (RMS agrees within 0.1 %); the bound is loose.
                assert!((a - b).abs() <= 128, "{title} {sid}: first sample over 1 LSB {a} vs research {b}");
            }
            // The research RMS figures are over 96-sample windows at 0 / 1152 / 2304 (probe `onset`).
            let t3 = &v["T3"];
            let r0 = rms(&dec.pcm[..96]);
            let r1 = rms(&dec.pcm[1152..1248]);
            let r2 = rms(&dec.pcm[2304..2400.min(dec.pcm.len())]);
            for (mine, key) in [(r0, "rms_first_win"), (r1, "rms_at_1152"), (r2, "rms_at_2304")] {
                if let Some(theirs) = t3[key].as_f64() {
                    let tol = (theirs * 0.05).max(2.0);
                    assert!((mine - theirs).abs() <= tol, "{title} {sid}: {key} {mine:.1} vs research {theirs:.1}");
                }
            }
            let emb_rms = rms(&parsed.embedded_pcm);
            if let Some(theirs) = t3["embedded_rms"].as_f64() {
                assert!((emb_rms - theirs).abs() <= 0.5, "{title} {sid}: embedded RMS {emb_rms:.2} vs {theirs:.2}");
            }
            // The research hashes are ffmpeg's; recorded as informational matches only.
            let mine = {
                let mut h = Sha256::new();
                for s in &dec.pcm {
                    h.update(s.to_le_bytes());
                }
                format!("{:x}", h.finalize())
            };
            if Some(mine.as_str()) == v["decode"]["mp3"]["pcm_sha256"].as_str() {
                fixed_exact += 1;
            }
            if Some(mine.as_str()) == v["decode"]["mp3float"]["pcm_sha256"].as_str() {
                float_exact += 1;
            }
            let a = render_candidate_a(&dec, &parsed.embedded_pcm, Rendering::CandidateA, snr.num_samples).unwrap();
            assert_eq!(a.len() as u32, snr.num_samples);
            assert_eq!(&a[..1152], &parsed.embedded_pcm[..]);
            let m = render_candidate_a(&dec, &parsed.embedded_pcm, Rendering::MpegOnly, snr.num_samples).unwrap();
            assert_eq!(m.len() as u32, snr.num_samples);
            eprintln!("{title} {sid}: {} blocks, {} frames, trim {}, first>1LSB {:?} (research {:?}), rms@1152 {:.1} / {:.1}", blocks.len(), parsed.total_frames, parsed.trim, first, research_first, r1, t3["rms_at_1152"].as_f64().unwrap_or(0.0));
            checked += 1;
        }
    }
    eprintln!("checked {checked} vectors; PCM bit-exact with ffmpeg mp3 (fixed): {fixed_exact}, with mp3float: {float_exact} (informational)");
    assert_eq!(checked, 24);
}
