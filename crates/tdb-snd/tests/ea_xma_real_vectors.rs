//! EA-XMA (codec `0x03`) through `tdb-snd::eaxma` + the helper runner on the research R84 vectors
//! (`docs/research/overnight-2026-09-24/data/r84-ea-xma-preview.json`): SNS speech (X360 NHL 09 / 10
//! `ann_eng` / `pbp_eng`), SNS music (X360 NHL 09–12, stereo) and loose raw `.abk` ram segments
//! (X360 NHL 10 → Legacy `000_goal` / `000_period` / `000_chant` / `sfx`). The NHL 09 archived banks
//! are not read here (their source path stays refused in Studio).
//!
//! Per vector: the source bytes hash to the research value (identity before decoding); the refusing
//! extraction accepts them; the RIFF XMA2 file goes through `helper::decode_xma2` (explicit path, no
//! shell, bounded); the decoded sample count per channel equals the declared count; the PCM hash is
//! compared with the research oracle's (R84: FFmpeg 8.1.1) and must be **bit-identical** — no
//! tolerance. **The helper is whatever `NHL_STUDIO_XMA2_HELPER` names**; its SHA-256 is printed so the
//! log names the exact binary. Since 27 Sep that is Studio's own minimal LGPL helper built by
//! `tools/xma2-helper/build-helper.sh` (FFmpeg 8.1.3); the installed GPLv3 full build is never bundled
//! and does not validate the helper. A different FFmpeg release that changes PCM fails here and must be
//! investigated, not tolerated. Exact counts and hashes are decoder agreement, not audible fidelity.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tdb_snd::helper::{decode_xma2, locate, Limits};
use tdb_snd::{eaxma, walk_sample};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn read_window(path: &Path, off: u64, len: usize) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
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

fn speech_root(game: &str) -> PathBuf {
    match game {
        "nhl09" => PathBuf::from(r"X:\Games\Emulation\Roms\XBOX360\NHL 09\NHL 09 (USA)"),
        "nhl10" => PathBuf::from(r"X:\Games\Emulation\Roms\XBOX360\NHL 10\NHL 10"),
        g => panic!("{g}"),
    }
}

fn pcm_sha(pcm: &[i16]) -> String {
    format!("{:x}", Sha256::digest(pcm.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>()))
}

#[test]
#[ignore = "real X360 trees under X:/Games/Emulation/Roms (read only) and an XMA2 helper named by NHL_STUDIO_XMA2_HELPER: select with --ignored"]
fn the_r84_vectors_decode_to_their_declared_counts_through_the_helper() {
    // Receipt 43: with `NHLMS_RELEASE_ROOT` set (an extracted release archive), the helper must be the
    // archive's own, found beside the executable exactly as Studio finds it — no developer override.
    let helper = match std::env::var_os("NHLMS_RELEASE_ROOT") {
        Some(root) => {
            assert!(std::env::var_os(tdb_snd::helper::XMA2_HELPER_ENV).is_none(), "release check: {} must not be set", tdb_snd::helper::XMA2_HELPER_ENV);
            let root = PathBuf::from(root);
            assert!(root.join("nhl-studio.exe").is_file(), "{}: no nhl-studio.exe (not an extracted release root)", root.display());
            let h = locate(Some(&root)).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(h, root.join("helpers").join(tdb_snd::helper::XMA2_HELPER_FILE));
            eprintln!("release root: {} (cwd {})", root.display(), std::env::current_dir().unwrap().display());
            h
        }
        None => locate(None).unwrap_or_else(|e| panic!("{e}")),
    };
    let helper_sha = format!("{:x}", Sha256::digest(std::fs::read(&helper).unwrap()));
    eprintln!("helper: {} sha256 {helper_sha}", helper.display());
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(repo("docs/research/overnight-2026-09-24/data/r84-ea-xma-preview.json")).unwrap()).unwrap();
    let mut table: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new(); // (vectors, exact count, oracle hash equal)
    let mut failures = Vec::new();
    let mut first: Option<(Vec<u8>, String)> = None; // (RIFF, research PCM hash) for the helpers-folder route
    for (family, f) in json["families"].as_object().unwrap() {
        let (key, what) = family.split_once(':').unwrap();
        let game = key.trim_start_matches("x360-");
        let is_abk = what.ends_with(".abk");
        if is_abk && f["source"]["source"].as_str() != Some("loose") {
            continue; // NHL 09 archived banks: source path refused in Studio; not read here.
        }
        for v in f["vectors"].as_array().unwrap() {
            if !is_abk && !v["sns_sha256"].is_string() {
                eprintln!("{family} {}: no research SNS hash (R84 skipped it: beyond the loose stream)", v["off"]);
                continue;
            }
            let tag = format!("{family} {}", v.get("off").or(v.get("entry")).unwrap());
            let result = (|| -> Result<(bool, bool), String> {
                let (riff, channels, declared, research_sha) = if is_abk {
                    let path = PathBuf::from(f["source"]["path"].as_str().unwrap());
                    let bank = tdb_snd::abk::parse_bank(&std::fs::read(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                    let rec = bank.record(v["entry"].as_u64().unwrap() as u16).ok_or("no such record")?;
                    let tdb_snd::abk::RecordBody::Ram { segments, .. } = &rec.body else { return Err("not a ram record".into()) };
                    let seg = &segments[v["segment"].as_u64().unwrap() as usize];
                    if format!("{:x}", Sha256::digest(&seg.bytes)) != v["sha256"].as_str().unwrap() {
                        return Err("segment bytes differ from the research hash".into());
                    }
                    let p = eaxma::raw_segment_packets(&seg.bytes).map_err(|e| e.to_string())?;
                    let h = rec.header;
                    let ch = h.channels() as u16;
                    (eaxma::riff_xma2(&p.packets, seg.samples, ch, h.sample_rate), ch, seg.samples, v["r84"]["decode"]["sha256"].as_str().unwrap_or("").to_string())
                } else {
                    let off = v["off"].as_u64().unwrap();
                    let sbs = if what == "music" {
                        PathBuf::from(f["source"]["sbr"].as_str().unwrap()).with_extension("sbs")
                    } else {
                        speech_root(game).join("audio/speech").join(format!("{what}_samples.sbs"))
                    };
                    let w = read_window(&sbs, off, 64 << 20);
                    let (snr, blocks, end) = walk_sample(&w, 0).map_err(|e| e.to_string())?;
                    if format!("{:x}", Sha256::digest(&w[..end as usize])) != v["sns_sha256"].as_str().unwrap() {
                        return Err("SNS bytes differ from the research hash".into());
                    }
                    let p = eaxma::sns_packets(&snr, &blocks).map_err(|e| e.to_string())?;
                    let ch = snr.channels() as u16;
                    (eaxma::riff_xma2(&p.packets, snr.num_samples, ch, snr.sample_rate), ch, snr.num_samples, v["r84"]["decode"]["sha256"].as_str().unwrap_or("").to_string())
                };
                let pcm = decode_xma2(&helper, &riff, Limits::default(), None).map_err(|e| e.to_string())?;
                if first.is_none() && !research_sha.is_empty() {
                    first = Some((riff.clone(), research_sha.clone()));
                }
                let per_channel = pcm.len() / channels as usize;
                eprintln!("{tag}: {channels} ch, declared {declared}, decoded {per_channel}{}", if pcm_sha(&pcm) == research_sha { " (PCM = research oracle hash)" } else { "" });
                Ok((per_channel as u32 == declared, pcm_sha(&pcm) == research_sha))
            })();
            let e = table.entry(family.clone()).or_default();
            e.0 += 1;
            match result {
                Ok((exact, same)) => {
                    e.1 += exact as usize;
                    e.2 += same as usize;
                    if !exact {
                        failures.push(format!("{tag}: count differs"));
                    }
                    if !same {
                        failures.push(format!("{tag}: PCM differs from the research oracle"));
                    }
                }
                Err(err) => failures.push(format!("{tag}: {err}")),
            }
        }
    }
    eprintln!("\n| family | vectors | decoded = declared | PCM = research hash |\n|---|---|---|---|");
    for (k, (n, exact, same)) in &table {
        eprintln!("| {k} | {n} | {exact} | {same} |");
    }
    let total: usize = table.values().map(|t| t.0).sum();
    eprintln!("{total} vectors; failures: {failures:#?}");
    assert!(total >= 60, "the R84 loose / SNS vectors ({total})");
    assert!(failures.is_empty());

    // The executable-adjacent route: the same binary copied to `<dir>/helpers/`, found by `locate`
    // with the variable removed (this is the only test in this binary, so nothing races on it).
    let (riff, research_sha) = first.expect("one decoded vector");
    let dir = std::env::temp_dir().join(format!("nhlms-xma-helpers-route-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("helpers")).unwrap();
    std::fs::copy(&helper, dir.join("helpers").join(tdb_snd::helper::XMA2_HELPER_FILE)).unwrap();
    let saved = std::env::var_os(tdb_snd::helper::XMA2_HELPER_ENV);
    std::env::remove_var(tdb_snd::helper::XMA2_HELPER_ENV);
    let adjacent = locate(Some(&dir));
    let missing = locate(Some(&std::env::temp_dir().join("nhlms-no-such-exe-dir")));
    if let Some(v) = saved {
        std::env::set_var(tdb_snd::helper::XMA2_HELPER_ENV, v);
    }
    let adjacent = adjacent.unwrap();
    assert_eq!(adjacent, dir.join("helpers").join(tdb_snd::helper::XMA2_HELPER_FILE));
    let pcm = decode_xma2(&adjacent, &riff, Limits::default(), None).unwrap();
    assert_eq!(pcm_sha(&pcm), research_sha, "helpers-folder route decodes the same PCM");
    assert!(missing.unwrap_err().to_string().contains("no XMA2 decoder helper is installed"));
    eprintln!("helpers-folder route: {} decoded {} samples = research hash", adjacent.display(), pcm.len());
    let _ = std::fs::remove_dir_all(&dir);
}
