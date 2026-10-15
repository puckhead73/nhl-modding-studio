//! Receipt 49 — the read-only venue-art gallery on real trees (read only): X360 NHL Legacy `stadium_1` (Winnipeg
//! materials) lists board-ad materials with their rasters and previews them; a changed file is refused by its hash;
//! PS3 NHL Legacy lists sizes without previews; an unmeasured title parses nothing.
use std::path::PathBuf;

use studio_domain::venue_art::{venue_art, venue_raster_png};
use studio_domain::Platform;

const X360_LEGACY: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\rendering";
const PS3_LEGACY: &str = r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL Legacy - Vanilla\PS3_GAME\USRDIR\rendering";

#[test]
#[ignore = "real X360 / PS3 NHL Legacy trees under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn legacy_venue_art_lists_board_ads_and_previews_only_unchanged_files() {
    let root = PathBuf::from(X360_LEGACY);
    assert!(root.join("stadium").is_dir(), "{}: missing (fails, never skips)", root.display());
    let v = venue_art(Some("nhllegacy"), Platform::Xbox, Some(1), Some(&root));
    assert!(v.measured, "{:?}", v.notes);
    assert!(!v.packages.is_empty(), "{:?}", v.notes);
    let p = &v.packages[0];
    assert_eq!(p.file, "stadium/stadium_1_container_0.rx2");
    let wpg = p.materials.iter().find(|m| m.name == "adbd_WPG").expect("adbd_WPG");
    assert_eq!(wpg.class, "Board ad");
    let r = &p.rasters[wpg.slots[0].raster.expect("slot 0 names a raster")];
    assert_eq!((r.width, r.height, r.format.as_str(), r.previewable), (512, 1024, "DXT1", true), "R127 §6 example");
    assert!(r.used_by.iter().any(|u| u == "adbd_WPG (slot 0)"));
    let classes: std::collections::BTreeSet<&str> = p.materials.iter().map(|m| m.class.as_str()).collect();
    eprintln!("stadium_1: {} rasters, {} presentation materials {:?}, {} other materials", p.rasters.len(), p.materials.len(), classes, p.other_materials);

    // Preview: a PNG no larger than asked, decoded back to the requested size class.
    let (w, h, png) = venue_raster_png(std::path::Path::new(&p.path), &p.sha256, r.ordinal, 256).unwrap();
    assert_eq!((w, h), (512, 1024));
    if let Ok(d) = std::env::var("NHLMS_DUMP_PREVIEW") {
        std::fs::write(PathBuf::from(d).join("adbd_WPG.png"), &png).unwrap();
    }
    let img = image::load_from_memory(&png).unwrap();
    assert_eq!((img.width(), img.height()), (128, 256));
    // A different hash (the file changed since listing): refused.
    let e = venue_raster_png(std::path::Path::new(&p.path), &"0".repeat(64), r.ordinal, 256).unwrap_err().to_string();
    assert!(e.contains("changed since it was listed"), "{e}");

    // PS3 (receipt 58): previewable through the descriptor's plain-chain data entry, and — research R137 §1 — the PS3
    // base level holds exactly the Xbox 360 base level's blocks once those are untiled and byte-swapped.
    let ps3 = venue_art(Some("nhllegacy"), Platform::Ps3, Some(1), Some(&PathBuf::from(PS3_LEGACY)));
    let q = &ps3.packages[0];
    let qa = q.materials.iter().find(|m| m.name == "adbd_WPG").unwrap();
    let qr = &q.rasters[qa.slots[0].raster.unwrap()];
    assert_eq!((qr.width, qr.height, qr.format.as_str(), qr.previewable), (512, 1024, "DXT1", true), "{:?}", qr.note);
    let xb = std::fs::read(&p.path).unwrap();
    let pb = std::fs::read(&q.path).unwrap();
    let (xp, pp) = (tdb_rx2::venue::read(&xb).unwrap(), tdb_rx2::venue::read(&pb).unwrap());
    let x_lin = tdb_rx2::venue::raster_linear(&xb, &xp, r.ordinal).unwrap();
    let p_lin = tdb_rx2::venue::raster_linear(&pb, &pp, qr.ordinal).unwrap();
    assert_eq!((x_lin.0, x_lin.1, x_lin.2), (p_lin.0, p_lin.1, p_lin.2));
    assert!(x_lin.3 == p_lin.3, "adbd_WPG: PS3 base blocks = X360 base blocks (R137 §1)");
    let (w3, h3, png3) = venue_raster_png(std::path::Path::new(&q.path), &q.sha256, qr.ordinal, 256).unwrap();
    assert_eq!((w3, h3), (512, 1024));
    assert!(!png3.is_empty());
    // Every PS3 raster of the package either previews or says why; no DXT raster of this package is refused.
    let refused: Vec<_> = q.rasters.iter().filter(|r| !r.previewable).map(|r| (r.ordinal, r.format.clone(), r.note.clone())).collect();
    eprintln!("PS3 stadium_1: {} rasters, {} not previewable {:?}", q.rasters.len(), refused.len(), refused);
    assert!(refused.iter().all(|(_, f, _)| !f.starts_with("DXT")), "{refused:?}");

    // Unmeasured titles parse nothing.
    let u = venue_art(Some("nhl09"), Platform::Xbox, Some(1), Some(&root));
    assert!(!u.measured && u.packages.is_empty() && u.notes[0].contains("not measured"));
    let rec = venue_art(Some("legacyrecomp"), Platform::Recomp, Some(1), Some(&root));
    assert!(!rec.measured, "Recompiled needs its own source identity");
}

/// Receipt 58 — research 2026-09-28 handoff 03 §1 / §8 (R137): every PS3 NHL 14 / 15 / Legacy loose venue descriptor names a
/// `0x10030` entry of exactly its plain chain, so every one previews; X360 NHL 11 / 12 / 14 are listed with R137's package
/// counts and their board-ad materials reach rasters.
#[test]
#[ignore = "real X360 / PS3 trees under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn r137_trees_list_and_ps3_descriptors_preview() {
    let x = r"X:\Games\Emulation\Roms\XBOX360";
    let s = r"X:\Games\Emulation\Roms\Sony Playstation 3";
    // (tree, game, platform, root, packages, descriptors) — R137 §1 / §8.
    let cases = [
        ("PS3 NHL 14", "nhl14", Platform::Ps3, format!(r"{s}\NHL 14\PS3_GAME\USRDIR\rendering"), 64, 3033),
        ("PS3 NHL 15", "nhl15", Platform::Ps3, format!(r"{s}\NHL 15\PS3_GAME\USRDIR\rendering"), 64, 3033),
        ("PS3 Legacy", "nhllegacy", Platform::Ps3, PS3_LEGACY.to_string(), 64, 3035),
        ("X360 NHL 11", "nhl11", Platform::Xbox, format!(r"{x}\NHL 11\NHL 11\rendering"), 51, 2060),
        ("X360 NHL 12", "nhl12", Platform::Xbox, format!(r"{x}\NHL 12\NHL 12\rendering"), 57, 2567),
        ("X360 NHL 14", "nhl14", Platform::Xbox, format!(r"{x}\NHL 14\NHL 14\rendering"), 64, 3033),
    ];
    for (tree, game, platform, root, packages, descriptors) in cases {
        let dir = PathBuf::from(&root).join("stadium");
        assert!(dir.is_dir(), "{}: missing (fails, never skips)", dir.display());
        let ext = if platform == Platform::Ps3 { "rpsgl" } else { "rx2" };
        let files: Vec<PathBuf> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|e| e == ext) && p.file_name().unwrap().to_string_lossy().starts_with("stadium_")).collect();
        let (mut n, mut previewable, mut board_edges) = (0usize, 0usize, 0usize);
        for f in &files {
            let b = std::fs::read(f).unwrap();
            let p = tdb_rx2::venue::read(&b).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
            n += p.rasters.len();
            previewable += p.rasters.iter().filter(|r| r.pixel_chunk.is_some()).count();
            board_edges += p.materials.iter().filter(|m| m.name.to_ascii_lowercase().starts_with("adbd") && matches!(m.slots[0], tdb_rx2::venue::Slot::Raster(_))).count();
        }
        assert_eq!((files.len(), n), (packages, descriptors), "{tree}");
        assert!(board_edges > 0, "{tree}: board-ad materials reach rasters");
        if platform == Platform::Ps3 {
            assert_eq!(previewable, descriptors, "{tree}: every descriptor names its plain chain (R137)");
        }
        // The gallery lists the title (arena art id 1).
        let v = venue_art(Some(game), platform, Some(1), Some(&PathBuf::from(&root)));
        assert!(v.measured && !v.packages.is_empty(), "{tree}: {:?}", v.notes);
        eprintln!("{tree}: {} packages, {n} descriptors, {previewable} previewable, {board_edges} adbd material edges", files.len());
    }
}
