//! Presentation 5.2 (receipt 49) — a **read-only** gallery of a gen7 arena's venue art: the arena's venue packages
//! (`rendering/stadium/stadium_<arenaartid>_container_<n>`, joined by `arenaartid` as the arena-assets card already
//! does), each package's rasters, and the board-ad / bench-ad / jumbotron / power-ring / glass materials with the
//! raster their parameter block names (research R127 §6–§7; parsed data, validated by `tdb_rx2::venue`).
//!
//! What this does **not** claim: that the game draws a raster because a material of that name references it
//! (runtime selection is unobserved), or that anything may be replaced — no board-ad write, archive re-key or
//! resize follows from this listing. Measured families only: Xbox 360 NHL 10 / 13 / 15 / Legacy and PS3 NHL 15 /
//! Legacy (R127f), plus Xbox 360 NHL 11 / 12 / 14 and PS3 NHL 14 (research 2026-09-28 handoff 03 §8, R137, receipt 58);
//! every other title is named as not measured and nothing is parsed. Xbox 360 rasters preview when their `DW8` pixel
//! entry validates, PS3 rasters when their descriptor's data entry holds exactly the plain chain (receipt 58). A preview
//! names the package by its SHA-256 from the listing and is refused when the file changed.
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tdb_rx2::venue::{self, Slot, VenuePlatform};

use crate::{DomainError, Platform, Result};

/// (game id, platform) pairs research measured with loose venue packages: R127f, then R137 §8 (X360 NHL 11 / 12 / 14,
/// PS3 NHL 14). PS3 NHL 11–13 keep their packages only inside archives (R137 §9) and are not listed.
pub const MEASURED: [(&str, Platform); 10] = [
    ("nhl10", Platform::Xbox),
    ("nhl11", Platform::Xbox),
    ("nhl12", Platform::Xbox),
    ("nhl13", Platform::Xbox),
    ("nhl14", Platform::Xbox),
    ("nhl15", Platform::Xbox),
    ("nhllegacy", Platform::Xbox),
    ("nhl14", Platform::Ps3),
    ("nhl15", Platform::Ps3),
    ("nhllegacy", Platform::Ps3),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueSlotView {
    /// The raster's ordinal in its package when the stored word names a raster descriptor.
    pub raster: Option<usize>,
    /// The stored word when it names no raster (e.g. `0x400000` on emissive power-ring materials).
    pub raw: Option<u32>,
    /// Why the slot could not be read (outside the parameter block, no parameter block).
    pub unreadable: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueMaterialView {
    pub name: String,
    /// "Board ad", "Bench ad", "Jumbotron", "Power ring", "Glass" — by the material's name only.
    pub class: String,
    pub shader: Option<String>,
    pub slots: [VenueSlotView; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueRasterView {
    pub ordinal: usize,
    /// The raster's descriptor (its file-table index): what a PS3 board replacement names (receipt 67).
    pub descriptor: usize,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub previewable: bool,
    pub note: Option<String>,
    /// Presentation materials whose slot names this raster, as "name (slot k)".
    pub used_by: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenuePackageView {
    /// `stadium/<file>`.
    pub file: String,
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub platform: String,
    pub materials: Vec<VenueMaterialView>,
    /// Named materials of other kinds (seats, ice, geometry …), counted only.
    pub other_materials: usize,
    pub rasters: Vec<VenueRasterView>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueArtView {
    pub arena_art_id: Option<u64>,
    pub measured: bool,
    pub scope: String,
    pub packages: Vec<VenuePackageView>,
    pub notes: Vec<String>,
}

/// The presentation class of a material, by its name (research R127f's classes).
pub fn material_class(name: &str) -> Option<&'static str> {
    let l = name.to_ascii_lowercase();
    if l.starts_with("adbd") || l.starts_with("boardad") {
        Some("Board ad")
    } else if l.starts_with("benchad") {
        Some("Bench ad")
    } else if l.starts_with("jumbotron") {
        Some("Jumbotron")
    } else if l.starts_with("powerring") {
        Some("Power ring")
    } else if l.starts_with("rinkglass") || l.starts_with("glass") {
        Some("Glass")
    } else {
        None
    }
}

fn scope_text() -> String {
    "research measured the loose venue packages of Xbox 360 NHL 10 → 15 / Legacy and PS3 NHL 14 / 15 / Legacy (R127f, R137); PS3 NHL 11 → 13 keep them only inside archives".into()
}

/// The arena's venue packages under `rendering_root`, listed read-only.
pub fn venue_art(game_id: Option<&str>, platform: Platform, arena_art_id: Option<u64>, rendering_root: Option<&Path>) -> VenueArtView {
    let mut view = VenueArtView { arena_art_id, measured: false, scope: scope_text(), packages: Vec::new(), notes: Vec::new() };
    let Some(game) = game_id else {
        view.notes.push("the open project has no recorded title, so the venue-package layout cannot be chosen".into());
        return view;
    };
    if !MEASURED.contains(&(game, platform)) {
        view.notes.push(format!("the venue packages of {game} on {} are not measured ({}); nothing is parsed", platform.as_str(), scope_text()));
        return view;
    }
    view.measured = true;
    let Some(art) = arena_art_id else {
        view.notes.push("this arena row carries no `arenaartid`, so no venue package can be named".into());
        return view;
    };
    let Some(root) = rendering_root else {
        view.notes.push("attach the title's install to list its venue packages".into());
        return view;
    };
    let ext = if platform == Platform::Ps3 { "rpsgl" } else { "rx2" };
    let dir = root.join("stadium");
    let mut files: Vec<(u64, PathBuf)> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                let rest = n.strip_prefix(&format!("stadium_{art}_container_"))?.strip_suffix(&format!(".{ext}"))?;
                rest.parse::<u64>().ok().map(|k| (k, e.path()))
            })
            .collect(),
        Err(e) => {
            view.notes.push(format!("{}: {e}", dir.display()));
            return view;
        }
    };
    files.sort();
    if files.is_empty() {
        view.notes.push(format!("no `stadium_{art}_container_<n>.{ext}` ships in {}", dir.display()));
    }
    for (_, path) in files {
        match package_view(&path) {
            Ok(p) => view.packages.push(p),
            Err(e) => view.notes.push(format!("{}: {e}", path.display())),
        }
    }
    view
}

fn package_view(path: &Path) -> Result<VenuePackageView> {
    let b = std::fs::read(path).map_err(|source| DomainError::Io { path: path.display().to_string(), source })?;
    let sha256 = format!("{:x}", Sha256::digest(&b));
    let p = venue::read(&b).map_err(|e| DomainError::Extract(e.to_string()))?;
    let slot = |s: &Slot| match s {
        Slot::Raster(o) => VenueSlotView { raster: Some(*o), raw: None, unreadable: None },
        Slot::Other(w) => VenueSlotView { raster: None, raw: Some(*w), unreadable: None },
        Slot::Unreadable(r) => VenueSlotView { raster: None, raw: None, unreadable: Some(r.clone()) },
    };
    let mut rasters: Vec<VenueRasterView> = p
        .rasters
        .iter()
        .map(|r| VenueRasterView { ordinal: r.ordinal, descriptor: r.chunk, width: r.width, height: r.height, format: r.format.clone(), previewable: r.pixel_chunk.is_some(), note: r.pixels_note.clone(), used_by: Vec::new() })
        .collect();
    let mut materials = Vec::new();
    let mut other = 0;
    for m in &p.materials {
        let Some(class) = material_class(&m.name) else {
            other += 1;
            continue;
        };
        for (k, s) in m.slots.iter().enumerate() {
            if let Slot::Raster(o) = s {
                rasters[*o].used_by.push(format!("{} (slot {k})", m.name));
            }
        }
        materials.push(VenueMaterialView { name: m.name.clone(), class: class.into(), shader: m.shader.clone(), slots: [slot(&m.slots[0]), slot(&m.slots[1])] });
    }
    let mut notes = Vec::new();
    if !p.sizes_consistent {
        notes.push("the header's region sizes do not add up to the file length (reported only)".into());
    }
    if p.bad_names > 0 {
        notes.push(format!("{} name entries point outside the package and were skipped", p.bad_names));
    }
    Ok(VenuePackageView {
        file: format!("stadium/{}", path.file_name().unwrap().to_string_lossy()),
        path: path.display().to_string(),
        bytes: b.len() as u64,
        sha256,
        platform: match p.platform {
            VenuePlatform::Xbox360 => "xbox".into(),
            VenuePlatform::Ps3 => "ps3".into(),
        },
        materials,
        other_materials: other,
        rasters,
        notes,
    })
}

/// One raster of a listed package as a PNG (longest edge at most `max_edge`), only while the file still has the
/// SHA-256 the listing showed.
pub fn venue_raster_png(path: &Path, expected_sha256: &str, ordinal: usize, max_edge: u32) -> Result<(u32, u32, Vec<u8>)> {
    let b = std::fs::read(path).map_err(|source| DomainError::Io { path: path.display().to_string(), source })?;
    let sha = format!("{:x}", Sha256::digest(&b));
    if !sha.eq_ignore_ascii_case(expected_sha256) {
        return Err(DomainError::Validation(format!("{} changed since it was listed; list the arena again", path.display())));
    }
    let p = venue::read(&b).map_err(|e| DomainError::Extract(e.to_string()))?;
    let (format, w, h, linear) = venue::raster_linear(&b, &p, ordinal).map_err(|e| DomainError::Extract(e.to_string()))?;
    let rgba = crate::assets::linear_to_rgba(format, w, h, &linear)?;
    let img = crate::assets::DecodedImage { width: w, height: h, rgba }.thumbnail(max_edge);
    Ok((w, h, img.to_png_fast()?))
}
