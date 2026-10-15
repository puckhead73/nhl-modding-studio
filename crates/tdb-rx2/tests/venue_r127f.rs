//! Receipt 49 — Studio's venue-package reader agrees with research R127f (handoff 20 §7) on every measured package:
//! X360 NHL 10 / 13 / 15 / Legacy and PS3 NHL 15 / Legacy loose trees (read only). For each package, the
//! presentation-class materials (board ad, bench ad, jumbotron, power ring, glass) in chunk order, their slot-0 /
//! slot-1 raster (descriptor chunk, ordinal, width, height, format) or raw word — exactly as research listed.
//! Selected with `--ignored`; a missing tree or package fails, never skips.
use std::path::{Path, PathBuf};

use tdb_rx2::venue::{self, Slot, VenuePackage};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn tree_root(tree: &str) -> PathBuf {
    let x = r"X:\Games\Emulation\Roms\XBOX360";
    let p = r"X:\Games\Emulation\Roms\Sony Playstation 3";
    PathBuf::from(match tree {
        "x360-nhl10" => format!(r"{x}\NHL 10\NHL 10"),
        "x360-nhl13" => format!(r"{x}\NHL 13\NHL 13"),
        "x360-nhl15" => format!(r"{x}\NHL 15\NHL 15"),
        "x360-legacy" => format!(r"{x}\NHL Legacy\NHL Legacy"),
        "ps3-nhl15" => format!(r"{p}\NHL 15\PS3_GAME\USRDIR"),
        "ps3-legacy" => format!(r"{p}\NHL Legacy - Vanilla\PS3_GAME\USRDIR"),
        other => panic!("unknown tree {other}"),
    })
}

fn presentation(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    ["adbd", "boardad", "benchad", "jumbotron", "powerring", "rinkglass", "glass"].iter().any(|p| l.starts_with(p))
}

fn slot_json(p: &VenuePackage, s: &Slot) -> serde_json::Value {
    match s {
        Slot::Raster(o) => {
            let r = &p.rasters[*o];
            // Research writes an unnamed PS3 signature as `sig:a/b/c`; Studio's label says "PS3 signature a/b/c".
            // Receipt 58: R137 names PS3 `83f1` / `83f2` as DXT1 / DXT3, which R127f still writes as signatures.
            let fmt = match (p.platform, r.format.as_str()) {
                (venue::VenuePlatform::Ps3, "DXT1") => "sig:83f1/83f1/ffff".to_string(),
                (venue::VenuePlatform::Ps3, "DXT3") => "sig:83f2/83f2/ffff".to_string(),
                _ => r.format.strip_prefix("PS3 signature ").map(|s| format!("sig:{s}")).unwrap_or_else(|| r.format.clone()),
            };
            serde_json::json!({"descriptor_chunk": r.chunk, "ordinal": o, "w": r.width, "h": r.height, "fmt": fmt})
        }
        Slot::Other(w) => serde_json::json!({"raw": format!("{w:#x}")}),
        Slot::Unreadable(r) => serde_json::json!({"unreadable": r}),
    }
}

#[test]
#[ignore = "real X360 / PS3 loose trees under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn every_measured_package_matches_r127f() {
    let data: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(repo("docs/research/overnight-2026-09-26/data/r127f-venue-art-slot-list.json")).unwrap()).unwrap();
    let (mut packages, mut materials, mut previewable, mut board_ads, mut past_block_end, mut x360_rasters) = (0, 0, 0, 0, 0, 0);
    for (tree, pk) in data["trees"].as_object().unwrap() {
        let dir = tree_root(tree).join("rendering").join("stadium");
        assert!(dir.is_dir(), "{}: missing (fails, never skips)", dir.display());
        for (file, want) in pk.as_object().unwrap() {
            let b = std::fs::read(dir.join(file)).unwrap_or_else(|e| panic!("{tree} {file}: {e}"));
            let p = venue::read(&b).unwrap_or_else(|e| panic!("{tree} {file}: {e}"));
            assert_eq!(p.rasters.len() as u64, want["rasters"].as_u64().unwrap(), "{tree} {file}: raster count");
            // R127f lists a presentation material only when its preceding chunk is the parameter block.
            let got: Vec<serde_json::Value> = p
                .materials
                .iter()
                .filter(|m| presentation(&m.name) && !matches!(m.slots[0], Slot::Unreadable(_)))
                .map(|m| serde_json::json!({"material": m.name, "slot0": slot_json(&p, &m.slots[0]), "slot1": slot_json(&p, &m.slots[1])}))
                .collect();
            let want_m = want["materials"].as_array().unwrap();
            assert_eq!(got.len(), want_m.len(), "{tree} {file}: presentation materials");
            for (g, w) in got.iter().zip(want_m) {
                // Receipt 49 finding: where the parameter block is shorter than 292 bytes (PS3), research's probe read the
                // slot-1 word past the block's end. Studio refuses to read outside the chunk. Accepted only when research's
                // out-of-block word is not a raster (so no image is lost), and counted.
                if g["slot1"].get("unreadable").is_some() && w["slot1"].get("raw").is_some() {
                    assert_eq!((&g["material"], &g["slot0"]), (&w["material"], &w["slot0"]), "{tree} {file}");
                    past_block_end += 1;
                    continue;
                }
                assert_eq!(g, w, "{tree} {file}");
            }
            if tree.starts_with("x360") {
                previewable += p.rasters.iter().filter(|r| r.pixel_chunk.is_some()).count();
                x360_rasters += p.rasters.len();
            }
            board_ads += got.iter().filter(|m| m["material"].as_str().unwrap().to_ascii_lowercase().starts_with(&"adbd"[..]) || m["material"].as_str().unwrap().to_ascii_lowercase().starts_with("boardad")).count();
            packages += 1;
            materials += got.len();
        }
    }
    eprintln!("R127f agreement: {packages} packages, {materials} presentation materials ({board_ads} board ads); X360 rasters with a validated pixel entry: {previewable} of {x360_rasters}; slot-1 words research read past a short parameter block (not rasters): {past_block_end}");
    // The handoff text says 360 packages; its JSON lists 361 (the count asserted here) and 4 116 materials.
    assert_eq!((packages, materials), (361, 4116));
}
