//! P2-A (21 Sep 2026) — the BIG4 + `.bh` writer against the research placement oracle
//! (`docs/research/overnight-2026-09-20/data/r10-placement-oracle.json`, handoff 03 §2): for each
//! of the six stock archives the oracle's three outcomes — fit-in-slot (N = slack), append and
//! **ordered reflow** (N = slack + 64) — are planned on the real, read-only install pair with the
//! oracle's own replacement input (`stock payload ++ N zero bytes`) and the staged `.big` / `.bh`
//! are hashed **in memory** (no file is written for the whole-file hashes) and compared with the
//! oracle's expected SHA-256. One archive is additionally staged to disk and run through `verify`.
//! The reflow variant is absent from the oracle for the two 2 GB `data0.big` (not computed there).
//! A missing install fails; nothing under `X:` is written.

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tdb_archive_big::big4::{plan, stage, stage_files, Big4Placement, Big4Replacement, BoundBig4, Growth};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

struct HashSink(Sha256, u64);
impl Write for HashSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.update(buf);
        self.1 += buf.len() as u64;
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn hex32(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
#[ignore = "real Xbox 360 / PS3 installs under X:/Games/Emulation/Roms (read only) + the research oracle JSON: select with --ignored; hashes ~5 GB of staged output in memory"]
fn every_oracle_outcome_is_reproduced_byte_for_byte_on_the_real_archives() {
    let oracle: serde_json::Value = serde_json::from_slice(&std::fs::read(repo("docs/research/overnight-2026-09-20/data/r10-placement-oracle.json")).expect("oracle JSON")).unwrap();
    let cases = oracle["big4_outer"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    let mut verified_on_disk = false;
    let mut outcomes = 0usize;
    for c in cases {
        let big_path = PathBuf::from(c["source"].as_str().unwrap());
        let bh_path = PathBuf::from(c["bh"]["path"].as_str().unwrap());
        assert!(big_path.is_file() && bh_path.is_file(), "{}: install missing (fails, never skips)", big_path.display());
        let t0 = std::time::Instant::now();
        let bh = std::fs::read(&bh_path).unwrap();
        let mut f = std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(&big_path).unwrap());
        let bound = BoundBig4::bind(&mut f, &bh).unwrap_or_else(|e| panic!("{}: {e}", big_path.display()));
        assert_eq!(hex32(&bound.identity.sha256), c["source_sha256"].as_str().unwrap(), "{}: source identity", big_path.display());
        assert_eq!(hex32(&bound.identity.sidecar_sha256), c["bh_sha256"].as_str().unwrap());
        let i = c["entry"]["i"].as_u64().unwrap() as usize;
        let e = &bound.entries[i];
        assert_eq!(e.path, c["entry"]["path"].as_str().unwrap());
        assert_eq!(e.offset as u64, c["entry"]["offset"].as_u64().unwrap());
        f.seek(SeekFrom::Start(e.offset as u64)).unwrap();
        let mut stock = vec![0u8; e.size as usize];
        f.read_exact(&mut stock).unwrap();
        let stored_sha: [u8; 32] = Sha256::digest(&stock).into();
        assert_eq!(hex32(&stored_sha), c["entry"]["stored_sha256"].as_str().unwrap());
        let decoded_len = bound.sidecar.records[i][2];
        eprintln!("{}: bound {} entries in {:?}; entry {i} `{}` S={} slack={}", big_path.display(), bound.entries.len(), t0.elapsed(), e.path, e.size, c["slot"]["slack"]);
        for (variant, growth, placement) in [
            ("fit_N=slack", Growth::Refuse("fit only".into()), "fit"),
            ("append_N=slack+64", Growth::Append, "append"),
            ("reflow_N=slack+64", Growth::Reflow, "reflow"),
        ] {
            let Some(v) = c["variants"].get(variant) else {
                eprintln!("  {variant}: not in the oracle for this archive (skipped by the oracle, not by this test)");
                continue;
            };
            let n = v["N"].as_u64().unwrap() as usize;
            let mut payload = stock.clone();
            payload.extend(std::iter::repeat_n(0u8, n));
            assert_eq!(payload.len() as u64, v["L"].as_u64().unwrap());
            let req = Big4Replacement { entry_index: i, expected_path: e.path.clone(), expected_stored_sha256: stored_sha, new_stored: payload, decoded_len, growth: None };
            let t1 = std::time::Instant::now();
            let p = plan(&bound, &mut f, vec![req], growth).unwrap_or_else(|err| panic!("{variant}: {err}"));
            match (placement, &p.edits[0].placement) {
                ("fit", Big4Placement::FitInSlot { .. }) | ("append", Big4Placement::AppendAndRetarget { .. }) | ("reflow", Big4Placement::Reflow { .. }) => {}
                (want, got) => panic!("{variant}: planned {got:?}, oracle expects {want}"),
            }
            if let Some(nl) = v.get("new_length").and_then(|x| x.as_u64()) {
                assert_eq!(p.new_len, nl, "{variant}: new length");
            }
            if let Big4Placement::Reflow { delta, .. } = p.edits[0].placement {
                assert_eq!(delta, v["delta"].as_u64().unwrap(), "{variant}: delta");
            }
            let mut sink = HashSink(Sha256::new(), 0);
            let (sidecar, written) = stage(&bound, &p, &mut f, &mut sink).unwrap_or_else(|err| panic!("{variant}: {err}"));
            assert_eq!(written, p.new_len);
            assert_eq!(sink.1, p.new_len);
            let big_sha: [u8; 32] = sink.0.finalize().into();
            let bh_sha: [u8; 32] = Sha256::digest(&sidecar).into();
            assert_eq!(hex32(&big_sha), v["expected_big_sha256"].as_str().unwrap(), "{}: {variant}: .big hash", big_path.display());
            assert_eq!(hex32(&bh_sha), v["expected_bh_sha256"].as_str().unwrap(), "{}: {variant}: .bh hash", big_path.display());
            eprintln!("  {variant}: N={n} -> {} bytes, both hashes reproduced ({:?})", p.new_len, t1.elapsed());
            outcomes += 1;
            // One on-disk staging + verify (the smallest archive, reflow: the segment coverage
            // and the shifted-copy comparison run for real).
            if placement == "reflow" && !verified_on_disk && bound.identity.len < 300_000_000 {
                let dir = std::env::temp_dir().join(format!("nhlms-big4-oracle-{}", std::process::id()));
                std::fs::create_dir_all(&dir).unwrap();
                let out_big = dir.join("staged.big");
                let out_bh = dir.join("staged.bh");
                let report = stage_files(&bound, &p, &big_path, &out_big, &out_bh).unwrap_or_else(|err| panic!("{variant}: stage_files: {err}"));
                assert_eq!(hex32(&report.output_sha256), v["expected_big_sha256"].as_str().unwrap());
                assert_eq!(hex32(&report.output_sidecar_sha256), v["expected_bh_sha256"].as_str().unwrap());
                assert_eq!(report.output_len, p.new_len);
                // The reflowed archive re-binds in the stock shape.
                let staged_bh = std::fs::read(&out_bh).unwrap();
                let mut g = std::io::BufReader::new(std::fs::File::open(&out_big).unwrap());
                let rebound = BoundBig4::bind(&mut g, &staged_bh).unwrap();
                assert!(rebound.index_order_is_offset_order, "reflow keeps the stock order");
                assert_eq!(rebound.entries[i].size as u64, v["L"].as_u64().unwrap());
                std::fs::remove_dir_all(&dir).unwrap();
                verified_on_disk = true;
                eprintln!("  {variant}: staged to disk, verified, re-bound in stock order, removed");
            }
        }
    }
    assert_eq!(outcomes, 16, "6 fit + 6 append + 4 reflow outcomes");
    assert!(verified_on_disk);
}
