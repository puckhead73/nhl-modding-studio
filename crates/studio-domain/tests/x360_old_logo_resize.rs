//! League 1.4 / Teams 2.4 — logo resizing on the Xbox 360 NHL 08 → 11 package form (receipt 37;
//! research R132 = 2026-09-26 handoff 25).
//!
//! 1. **The descriptor rule** (R132 §3): for every single-level Xbox-tiled `8_8_8_8` raster of the loose
//!    logo families of NHL 08 → 11, re-deriving the shipped descriptor at its own size with Studio's
//!    rule (`patch_descriptor_for_resize`) must reproduce it byte for byte, and the payload must be
//!    align(w, 32) × align(h, 32) × 4 bytes. That is the evidence the resize writer stands on.
//! 2. **The route**: one team logo per title (NHL 08: whole-file RefPack package; NHL 09 / 10 plain; NHL 11
//!    RefPack texture member) is resized on a disposable copy: it re-opens at the new size with the
//!    imported pixels and keeps its whole-file wrapper; a calendar package (a display crop) refuses by
//!    name. The Apt quad / member `0` size words and the other members' byte identity are asserted per
//!    package by the private-fixture sweep `team_logo_dimension_sweep.rs`. Nothing about what the
//!    console draws is claimed (card).
use std::path::{Path, PathBuf};

use studio_domain::{decode_texture_file, replace_texture_in_file};

fn tree(game: &str) -> PathBuf {
    let n = &game[3..];
    let inner = if matches!(n, "07" | "08" | "09") { format!("NHL {n} (USA)") } else { format!("NHL {n}") };
    PathBuf::from(format!(r"X:\Games\Emulation\Roms\XBOX360\NHL {n}\{inner}\fe\ion\artassets"))
}

fn unwrap(b: &[u8]) -> Vec<u8> {
    if tdb_archive_codec::is_refpack(b) {
        tdb_archive_codec::refpack_decompress(b).unwrap()
    } else {
        b.to_vec()
    }
}

const FAMILIES: &[&str] = &["teamlogos", "teamlogossmall", "teamlogoswide", "teamlogosdynasty", "teamlogoscalendar", "leaguelogos", "leaguelogossmall", "leaguelogoswide"];

#[test]
#[ignore = "real X360 NHL 08-11 trees under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn the_r132_argb8_descriptor_rule_reproduces_every_shipped_logo_descriptor() {
    use tdb_rx2::texlib::{patch_descriptor_for_resize, PixelFormat, DESCRIPTOR_LEN};
    let mut total = 0usize;
    let mut failures = Vec::new();
    for game in ["nhl07", "nhl08", "nhl09", "nhl10", "nhl11"] {
        let mut per_title = 0usize;
        for fam in FAMILIES {
            let dir = tree(game).join(fam);
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for f in rd.flatten() {
                let path = f.path();
                if path.extension().and_then(|e| e.to_str()) != Some("big") {
                    continue;
                }
                let pkg = unwrap(&std::fs::read(&path).unwrap());
                let Ok(archive) = tdb_archive_big::Archive::parse(pkg) else { continue };
                for e in archive.iter_entries() {
                    let Ok(raw) = archive.read_entry_raw(&e.name) else { continue };
                    let member = unwrap(&raw);
                    if !member.starts_with(tdb_rx2::texlib::MAGIC) {
                        continue;
                    }
                    let lib = tdb_rx2::texlib::parse(&member).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
                    for (i, t) in lib.textures.iter().enumerate() {
                        if t.format != PixelFormat::Argb8888 || !t.tiled || lib.declared_mip_count(i) != 1 {
                            continue;
                        }
                        let off = t.descriptor_offset();
                        let orig: [u8; DESCRIPTOR_LEN] = member[off..off + DESCRIPTOR_LEN].try_into().unwrap();
                        let again = patch_descriptor_for_resize(&orig, t.width, t.height, PixelFormat::Argb8888, 1);
                        let payload_ok = t.buffer_capacity() == (t.width as usize).div_ceil(32) * 32 * (t.height as usize).div_ceil(32) * 32 * 4;
                        match again {
                            Ok(d) if d == orig && payload_ok => {}
                            Ok(d) => failures.push(format!("{} raster {i} {}x{}: descriptor {} payload {} (slot {} B)", path.display(), t.width, t.height, if d == orig { "=" } else { "≠" }, if payload_ok { "=" } else { "≠" }, t.buffer_capacity())),
                            Err(err) => failures.push(format!("{} raster {i} {}x{}: {err}", path.display(), t.width, t.height)),
                        }
                        per_title += 1;
                    }
                }
            }
        }
        eprintln!("{game}: {per_title} single-level tiled 8_8_8_8 logo rasters checked");
        total += per_title;
    }
    eprintln!("{total} rasters; failures: {}", failures.len());
    for f in failures.iter().take(20) {
        eprintln!("  {f}");
    }
    assert!(total > 2_000, "the logo families were not found ({total})");
    assert!(failures.is_empty(), "{} descriptor(s) the rule does not reproduce", failures.len());
}

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn first_package(dir: &Path) -> PathBuf {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().and_then(|e| e.to_str()) == Some("big")).collect();
    v.sort();
    v.into_iter().next().unwrap_or_else(|| panic!("{}: no package", dir.display()))
}

#[test]
#[ignore = "real X360 NHL 08-11 trees under X:/Games/Emulation/Roms (read only; writes only to a temp copy): select with --ignored"]
fn a_team_logo_resizes_on_each_old_title_and_a_calendar_crop_refuses() {
    let tmp = Tmp(std::env::temp_dir().join(format!("nhlms-oldlogo-{}", std::process::id())));
    let _ = std::fs::remove_dir_all(&tmp.0);
    std::fs::create_dir_all(&tmp.0).unwrap();
    for game in ["nhl08", "nhl09", "nhl10", "nhl11"] {
        let src = first_package(&tree(game).join("teamlogos"));
        let stored = std::fs::read(&src).unwrap();
        let copy = tmp.0.join(format!("{game}.big"));
        std::fs::write(&copy, &stored).unwrap();
        let (_, img) = decode_texture_file(&copy, None).unwrap_or_else(|e| panic!("{game}: decode {}: {e}", src.display()));
        let (nw, nh) = (img.width + 64, img.height / 2);
        let rgba: Vec<u8> = (0..nw * nh).flat_map(|i| [(i % nw * 255 / nw) as u8, (i / nw * 255 / nh) as u8, 90, 255]).collect();
        let out = replace_texture_in_file(stored.clone(), &rgba, nw, nh, None).unwrap_or_else(|e| panic!("{game} {}: resize {}x{} → {nw}x{nh}: {e}", src.display(), img.width, img.height));
        std::fs::write(&copy, &out.bytes).unwrap();
        let (_, back) = decode_texture_file(&copy, None).unwrap();
        assert_eq!((back.width, back.height), (nw, nh), "{game}: re-opens at the new size");
        let mad = back.rgba.iter().zip(&rgba).map(|(a, b)| (*a as i32 - *b as i32).abs() as f64).sum::<f64>() / rgba.len() as f64;
        assert!(mad < 1.0, "{game}: uncompressed pixels round-trip (MAD {mad:.2})");
        assert_eq!(tdb_archive_codec::is_refpack(&out.bytes), tdb_archive_codec::is_refpack(&stored), "{game}: the whole-file wrapper is kept");
        eprintln!("{game}: {} {}x{} → {nw}x{nh} ({} → {} bytes) · {:?}", src.file_name().unwrap().to_string_lossy(), img.width, img.height, stored.len(), out.bytes.len(), out.notes);
        // A calendar package: the quad is a display crop → refused by name, nothing written.
        let cal = first_package(&tree(game).join("teamlogoscalendar"));
        let cal_bytes = std::fs::read(&cal).unwrap();
        let cal_copy = tmp.0.join(format!("{game}-cal.big"));
        std::fs::write(&cal_copy, &cal_bytes).unwrap();
        let (_, cimg) = decode_texture_file(&cal_copy, None).unwrap();
        let (cw, ch) = (cimg.width + 32, cimg.height);
        let crgba = vec![200u8; (cw * ch * 4) as usize];
        let e = replace_texture_in_file(cal_bytes, &crgba, cw, ch, None).unwrap_err().to_string();
        assert!(e.contains("not the texture's own size"), "{game} calendar: {e}");
    }
}
