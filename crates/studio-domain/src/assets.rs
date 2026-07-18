//! Player **asset extraction** — portraits, head meshes, head textures.
//!
//! Turns the two asset-id fields on the player-bio table into real files on
//! disk, named by the player they belong to rather than by EA's numeric ids
//! (the human-readable-surfaces rule — see `CLAUDE.md`).
//!
//! ## The link (verified 2026-07-17 against an Xbox 360 install)
//!
//! Two distinct fields on `ubPc` point at art:
//!
//! | Field  | Meaning        | Resolves to                                          |
//! | ------ | -------------- | ---------------------------------------------------- |
//! | `lOnr` | portrait id    | `fe/ion/artassets/playerheads/p{bucket}/p{lOnr}.big`  |
//! | `pPaD` | head model id  | `rendering/player/player_{pPaD}.rx2` + `texlib_{pPaD}.rx2` |
//!
//! `kOtt.lOnr` is a *different* field (team-logo art id) — same raw id, other
//! table. Don't conflate them.
//!
//! **A player has a bespoke modelled head iff `pPaD == lOnr`.** Everyone else
//! shares a generic pool head (ids `>= 60000`, see [`POOL_HEAD_MIN`]) that
//! belongs to no one player and is therefore not extractable *as* that player.
//! On the reference Xbox 360 anchor this selects 246 of 5632 players, and all
//! 246 resolve to a mesh, a texlib **and** a portrait present on disk — the
//! rule is exact, not a heuristic.
//!
//! **Portraits are decoupled from heads**: 4259 of those same 5632 players have
//! a portrait. So the portrait set is ~17× the head set, and the two are scanned
//! independently rather than behind one filter.
//!
//! ## Why the portrait chain has three layers
//!
//! `p{id}.big` is an EA BIGF archive holding ~6 entries, most of them Scaleform
//! (`Apt.Data`, `Apt.cons`) or empty `sg1`/`sg2` stubs. Exactly one is the real
//! raster: RefPack-compressed, wrapping an Xbox-tiled `XPR2` package. We pick
//! the largest by pixel area rather than the first, because the small auxiliary
//! rasters would otherwise win by position.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use tdb_core::dev_key::dev_player_key;
use tdb_core::{FieldDescriptor, Table, TdbFile};
use tdb_defs::FieldMetaIndex;
use tdb_relations::entity::ids;

use crate::{DomainError, Result};

/// Head ids at or above this are shared generic "pool" heads — one model reused
/// by many players, belonging to none of them. A Legacy-era convention: NHL 10
/// has no pool heads at all (every `pPaD` there is below this).
pub const POOL_HEAD_MIN: u64 = 60_000;

/// How a title stores `ubPc`'s birth month/day.
///
/// **This drifts across titles and the metadata does not say which.** Both NHL 10
/// and Legacy declare identical `rangelow="0" rangehigh="15"` descriptors, but
/// the stored values differ:
///
/// | Title  | raw month | raw day | Meaning        |
/// | ------ | --------- | ------- | -------------- |
/// | Legacy | `0..11`   | `0..30` | 0-indexed, `+1` to read |
/// | NHL 10 | `1..12`   | `1..31` | already 1-indexed       |
///
/// Verified over 5417 Legacy and 3933 NHL 10 records. Applying Legacy's `+1` to
/// NHL 10 shifts every date a day and a month forward — Jordan Staal (born
/// 1988-09-10) reads as 1988-10-11 — which silently corrupts every export name
/// and breaks the historical-corpus join.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BirthdateEncoding {
    /// `JKLp + 1` / `Kswi + 1` — NHL 11→21, every platform.
    ZeroIndexed,
    /// Stored as-is — NHL 10 only (PS3 and Xbox 360 both).
    OneIndexed,
}

impl BirthdateEncoding {
    /// What to add to a stored month/day to get the real calendar number. The
    /// single place the offset is defined, so the read display, the edit form
    /// and the export slug can't drift apart.
    pub fn offset(self) -> i64 {
        match self {
            BirthdateEncoding::ZeroIndexed => 1,
            BirthdateEncoding::OneIndexed => 0,
        }
    }
}

/// Probe a database for its birthdate convention rather than assuming it from
/// the title id — the same evidence-first rule `Capabilities::probe` follows.
///
/// The two schemes are mutually exclusive, which makes this decidable from the
/// values alone: month `12` cannot occur 0-indexed, and month `0` cannot occur
/// 1-indexed. Either marker settles it, so this returns on the first one seen —
/// both are common enough (roughly a twelfth of players each) that it reads a
/// handful of records in practice, cheap enough to call per field read.
///
/// Falls back to `ZeroIndexed` when neither marker appears — a database too
/// small to tell. That matches 19 of the 21 committed anchors (NHL 11→21 on
/// PS3/PS4/Xbox 360); only NHL 10 is 1-indexed, on both its platforms.
pub fn probe_birthdate_encoding(db: &TdbFile, bio: &Table) -> BirthdateEncoding {
    let Ok(f_month) = field_by_studio_id(db, bio, F_BIRTH_MONTH) else {
        return BirthdateEncoding::ZeroIndexed;
    };
    for record in 0..bio.current_records as u32 {
        match db.read_u64(bio, record, f_month) {
            Some(0) => return BirthdateEncoding::ZeroIndexed,
            Some(12) => return BirthdateEncoding::OneIndexed,
            _ => {}
        }
    }
    BirthdateEncoding::ZeroIndexed
}

/// `ubPc` portrait-id field (studio id) — EA's metadata calls it `artid`.
/// Distinct from `kOtt.lOnr`, the team-logo art id: same raw id, other table.
const F_PORTRAIT_ID: &str = "lOnr";
/// `ubPc` head-model-id field (studio id) — EA's metadata calls it `headid`.
const F_HEAD_ID: &str = "pPaD";
/// Birthdate components (studio ids), stored offset: `qFnd + 1900`, `JKLp + 1`,
/// `Kswi + 1`. [`dev_player_key`] applies those offsets itself. Addressed by id
/// rather than by their metadata names (`year`/`month`/`day`), which are generic
/// enough to recur across unrelated tables.
const F_BIRTH_YEAR: &str = "qFnd";
const F_BIRTH_MONTH: &str = "JKLp";
const F_BIRTH_DAY: &str = "Kswi";

/// Where a title keeps its art: `rendering/` (meshes, texlibs) and `fe/` (the
/// front-end tree — portraits, logos, jerseys). Both sit beside the executable,
/// which is not always the folder the user pointed at — see [`AssetRoots::resolve`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetRoots {
    /// `<anchor>/rendering` — player meshes and texlibs.
    pub rendering: String,
    /// `<anchor>/fe` — the front-end tree (portraits, logos, jerseys).
    pub fe: String,
}

/// Bounded depth for the executable search. Recomp nests the real install one
/// level down (`<root>/game/default.xex`); the cap keeps a mis-picked root from
/// walking a whole drive.
const ANCHOR_SEARCH_DEPTH: usize = 3;

impl AssetRoots {
    /// Infer the art roots from an install dir. The prior tooling made these
    /// operator-configured only; we infer and let the caller override, so the
    /// common path needs no setup.
    ///
    /// Art sits beside the game executable, which is **not** always the folder
    /// the user picked. On Xbox the picked root *is* the `default.xex` folder,
    /// but a Recomp install keeps it one level down (`<root>/game/default.xex`)
    /// — so anchoring on the install dir would name `<root>/rendering` for a
    /// tree whose art actually extracts to `<root>/game/rendering`. Anchor on
    /// the executable's own directory instead, falling back to the install dir
    /// (PS3, whose `USRDIR` holds the art directly and has no `.xex`).
    pub fn resolve(install_dir: &Path) -> AssetRoots {
        let anchor = anchor_dir(install_dir);
        AssetRoots {
            rendering: anchor.join("rendering").to_string_lossy().into_owned(),
            fe: anchor.join("fe").to_string_lossy().into_owned(),
        }
    }

    /// Apply optional operator overrides over an inferred pair. An override that
    /// is `None` or empty leaves the inferred value alone.
    pub fn with_overrides(mut self, rendering: Option<&str>, fe: Option<&str>) -> AssetRoots {
        if let Some(r) = rendering.filter(|s| !s.trim().is_empty()) {
            self.rendering = r.to_string();
        }
        if let Some(f) = fe.filter(|s| !s.trim().is_empty()) {
            self.fe = f.to_string();
        }
        self
    }

    /// Whether both roots are present — i.e. this install has extracted art.
    /// A packed install (`.big` archives, nothing loose) reports `false`, which
    /// is the signal to run extraction first.
    pub fn complete(&self) -> bool {
        self.rendering_dir().is_dir() && self.fe_dir().is_dir()
    }

    fn rendering_dir(&self) -> PathBuf {
        PathBuf::from(&self.rendering)
    }

    fn fe_dir(&self) -> PathBuf {
        PathBuf::from(&self.fe)
    }

    /// The head **mesh** container, across the two naming schemes EA used
    /// (verified on real installs):
    ///
    /// * **NHL 10 → Legacy** — `player/player_{head_id}.rx2`, one container bundling
    ///   every submesh (head, hair, eyes, mouth).
    /// * **NHL 09 (and 08)** — `player/head_{head_id}.rx2`, the face only; hair is a
    ///   *separate* mesh file (see [`AssetRoots::extra_head_mesh_paths`]).
    /// * **NHL 07** — nested one folder per head: `player/player_{head_id}/head_{head_id}.rx2`,
    ///   with the eye meshes/textures beside it (verified on a real Xbox 360 install).
    ///
    /// The schemes are disjoint (no install carries a head in more than one form),
    /// so probing each in turn resolves every title correctly. The bare
    /// `head_{id}.rx2` never collides with NHL 10's loose `head_{id}_cm.rx2`
    /// textures — those always carry a map suffix.
    pub fn mesh_path(&self, head_id: u64) -> Option<PathBuf> {
        let dir = self.rendering_dir().join("player");
        let nested = dir.join(format!("player_{head_id}"));
        [
            dir.join(format!("player_{head_id}.rx2")), // NHL 10 → Legacy bundle
            dir.join(format!("head_{head_id}.rx2")),   // NHL 08/09 loose
            nested.join(format!("head_{head_id}.rx2")), // NHL 07 nested per-head folder
        ]
        .into_iter()
        .find(|p| p.is_file())
    }

    /// Companion meshes that render *with* the head but live in their own files.
    /// On NHL 09 the hairstyle is a separate `hair_{head_id}_0_0_0.rx2` mesh
    /// (the `_0_0_0` variant infix is the default style/length/colour), where
    /// NHL 10 → Legacy carry hair as a `haircap_`/`cards*` submesh inside the one
    /// `player_` container. Empty when none apply, so the caller can always merge
    /// the returned set into the preview unconditionally.
    pub fn extra_head_mesh_paths(&self, head_id: u64) -> Vec<PathBuf> {
        let hair = self
            .rendering_dir()
            .join("player")
            .join(format!("hair_{head_id}_0_0_0.rx2"));
        if hair.is_file() {
            vec![hair]
        } else {
            Vec::new()
        }
    }

    /// Every RX2 container holding a raster for this head, across both layouts
    /// EA used (verified on real installs):
    ///
    /// * **Legacy / 14 / 15** — one bundled `texlib_{head_id}.rx2` carrying
    ///   every map (685 of them beside 904 meshes, so plenty of heads have none).
    /// * **NHL 10** — no texlib at all; each raster is its own single-raster
    ///   container, `head_{head_id}_cm.rx2`, `hair_{head_id}_cm.rx2`, …
    ///
    /// Both decode through `texlib::parse`; only the packaging differs.
    /// * **NHL 07** — no texlib; the head's rasters live in its own
    ///   `player/player_{head_id}/` folder (`head_{id}_0_0_0_cm.rx2`, `eyes_{id}_0_cm.rx2`, …).
    pub fn texture_paths(&self, head_id: u64) -> Vec<PathBuf> {
        let player_dir = self.rendering_dir().join("player");
        let bundled = player_dir.join(format!("texlib_{head_id}.rx2"));
        if bundled.is_file() {
            return vec![bundled];
        }
        // Loose rasters live either directly in `player/` (NHL 10) or in the head's own nested
        // `player_{id}/` folder (NHL 07). Scan whichever exists; `is_loose_raster_for` keys on the id
        // token so unrelated files are ignored.
        let nested = player_dir.join(format!("player_{head_id}"));
        let mut loose: Vec<PathBuf> = [player_dir, nested]
            .into_iter()
            .filter_map(|d| std::fs::read_dir(d).ok())
            .flat_map(|entries| entries.flatten().map(|e| e.path()))
            .filter(|p| is_loose_raster_for(p, head_id))
            .collect();
        loose.sort(); // stable order across runs
        loose
    }

    /// The portrait `.big` under `fe/ion/artassets/{dir}/`, if present. `dir` is
    /// [`PORTRAIT_DIR_LARGE`] or [`PORTRAIT_DIR_SMALL`].
    ///
    /// * **NHL 08 → Legacy** — bucketed into id-range subfolders, `p{bucket}/p{id}.big`.
    /// * **NHL 07** — flat, `p{id}.big` directly under the dir (verified on a real install).
    ///
    /// Probe flat first, then the bucketed layout; existence-gated, so neither yields a false match.
    pub fn portrait_path(&self, portrait_id: u64, dir: &str) -> Option<PathBuf> {
        let base = self.fe_dir().join("ion").join("artassets").join(dir);
        let file = format!("p{portrait_id}.big");
        let flat = base.join(&file);
        if flat.is_file() {
            return Some(flat);
        }
        let bucketed = base.join(portrait_bucket(portrait_id)?).join(&file);
        bucketed.is_file().then_some(bucketed)
    }
}

/// The directory the game's art hangs off: whichever folder holds `default.xex`,
/// searched from `install_dir` down to [`ANCHOR_SEARCH_DEPTH`]. Falls back to
/// `install_dir` when there's no executable to anchor on (PS3), which is also
/// the right answer there.
///
/// Breadth-first, so the shallowest match wins — a backup copy nested deeper
/// never outranks the real install.
fn anchor_dir(install_dir: &Path) -> PathBuf {
    let mut queue = std::collections::VecDeque::from([(install_dir.to_path_buf(), 0usize)]);
    while let Some((dir, depth)) = queue.pop_front() {
        if dir.join("default.xex").is_file() {
            return dir;
        }
        if depth < ANCHOR_SEARCH_DEPTH {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                let mut children: Vec<PathBuf> = entries
                    .flatten()
                    .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                    .map(|e| e.path())
                    .collect();
                children.sort(); // deterministic across runs
                for child in children {
                    queue.push_back((child, depth + 1));
                }
            }
        }
    }
    install_dir.to_path_buf()
}

/// Texture-map suffixes a loose raster's filename ends in: colour, normal,
/// specular, detail, tint, ambient. A `.rx2` that ends in one of these is a
/// texture; one that doesn't is a mesh. This is what tells NHL 09's hair
/// **texture** `hair_10000_cm.rx2` apart from its hair **mesh**
/// `hair_10000_0_0_0.rx2` — same part+id, but only the former ends in a map.
const MAP_SUFFIXES: &[&str] = &["cm", "nm", "sm", "dm", "tm", "am"];

/// Is `path` a loose per-raster texture container for `head_id`?
///
/// Matches `{part}_{head_id}_…_{map}.rx2`, e.g. NHL 10's `head_10000_cm.rx2` and
/// NHL 09's `head_10000_0_0_0_cm.rx2` — the id as a whole `_`-token so head 1000
/// never picks up head 11000's files, and a **map suffix** as the last token so
/// a same-named mesh (NHL 09's `hair_<id>_0_0_0.rx2`) is not mistaken for one.
/// `player_*` / `head_*` bare meshes are excluded by the missing map suffix.
fn is_loose_raster_for(path: &Path, head_id: u64) -> bool {
    if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case("rx2")) {
        return false;
    }
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    let mut tokens = stem.split('_');
    let Some(part) = tokens.next() else {
        return false;
    };
    if part.eq_ignore_ascii_case("player") || part.eq_ignore_ascii_case("texlib") {
        return false;
    }
    // Need at least `{part}_{id}_{map}`: id token first, a known map suffix last.
    let rest: Vec<&str> = tokens.collect();
    rest.len() >= 2
        && rest[0].parse::<u64>() == Ok(head_id)
        && MAP_SUFFIXES
            .iter()
            .any(|m| rest.last().is_some_and(|last| last.eq_ignore_ascii_case(m)))
}

/// Menu/detail-size portraits.
pub const PORTRAIT_DIR_LARGE: &str = "playerheads";
/// In-game HUD-size portraits — the same ids at a lower resolution.
pub const PORTRAIT_DIR_SMALL: &str = "playerheadssmall";

/// The bucket folder a portrait id lives in. Ids outside `1..=12000` have no
/// shipped portrait at all, so they resolve to `None` before any disk touch.
fn portrait_bucket(portrait_id: u64) -> Option<&'static str> {
    match portrait_id {
        1..=4000 => Some("p0_4000"),
        4001..=8000 => Some("p4001_8000"),
        8001..=12000 => Some("p8001_12000"),
        _ => None,
    }
}

/// One player's art, resolved against a real install.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerAssets {
    /// Record index into `ubPc`.
    pub bio_record: u32,
    /// "First Last" — the display name.
    pub full_name: String,
    /// `lastname_firstname_dd_mm_yyyy`, the export filename stem and the bridge
    /// key to the historical corpus. `None` when the bio lacks a usable name or
    /// a plausible birthdate.
    pub slug: Option<String>,
    /// `ubPc.lOnr`.
    pub portrait_id: u64,
    /// `ubPc.pPaD`.
    pub head_id: u64,
    /// `pPaD == lOnr` — this head model is this player's own.
    pub bespoke_head: bool,
    /// `pPaD >= 60000` — a shared generic head.
    pub pool_head: bool,
    /// Resolved paths, `None` when the file isn't on disk.
    pub portrait_path: Option<String>,
    pub portrait_small_path: Option<String>,
    pub mesh_path: Option<String>,
    /// Every RX2 container holding one of this head's textures — a single
    /// bundled texlib on Legacy/14/15, or several loose per-raster files on
    /// NHL 10. Empty when none ship.
    pub texture_paths: Vec<String>,
}

impl PlayerAssets {
    /// Anything worth exporting for this player.
    pub fn has_any(&self) -> bool {
        self.portrait_path.is_some() || self.mesh_path.is_some() || !self.texture_paths.is_empty()
    }
}

/// Scan every `ubPc` record and resolve its art against `roots`.
///
/// This touches the filesystem once per candidate path, so it's a scan, not a
/// hot path — call it when a project opens or the roots change, then filter the
/// result in memory.
pub fn scan_player_assets(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    roots: &AssetRoots,
) -> Result<Vec<PlayerAssets>> {
    let bio = db
        .table_canonical(ids::BIO)
        .ok_or_else(|| DomainError::Extract(format!("player-bio table {} not found", ids::BIO)))?;

    let f_portrait = field_by_studio_id(db, bio, F_PORTRAIT_ID)?;
    let f_head = field_by_studio_id(db, bio, F_HEAD_ID)?;
    // Birthdate is optional: without it a player simply gets no slug, which the
    // export reports rather than treating as a hard failure.
    let f_year = field_by_studio_id(db, bio, F_BIRTH_YEAR).ok();
    let f_month = field_by_studio_id(db, bio, F_BIRTH_MONTH).ok();
    let f_day = field_by_studio_id(db, bio, F_BIRTH_DAY).ok();
    // `dev_player_key` composes the year as `qFnd + 1900`, but NHL 07 stores the birth year in a 6-bit
    // field based at 1959 (see `view::birth_year_base`). Bias the raw year by `base - 1900` so the slug
    // (and its FHM identity match) carries the real calendar year, not e.g. 1924 for a 1983 birth.
    let year_bias: u64 = match f_year {
        Some(f) if f.bit_width <= 6 => 1959 - 1900,
        _ => 0,
    };
    // NHL 10 stores month/day already 1-indexed; Legacy stores them 0-indexed.
    // `dev_player_key` applies Legacy's `+1`, so pre-decrement NHL 10's values
    // to hand it the encoding it expects.
    let encoding = probe_birthdate_encoding(db, bio);

    let mut out = Vec::with_capacity(bio.current_records as usize);
    for record in 0..bio.current_records as u32 {
        let portrait_id = db.read_u64(bio, record, f_portrait).unwrap_or(0);
        let head_id = db.read_u64(bio, record, f_head).unwrap_or(0);

        let first = read_str_by_name(db, meta, bio, record, "firstname");
        let last = read_str_by_name(db, meta, bio, record, "lastname");
        let full_name = match (first.trim(), last.trim()) {
            ("", l) => l.to_string(),
            (f, "") => f.to_string(),
            (f, l) => format!("{f} {l}"),
        };

        // `dev_player_key` bakes in the 0-indexed `+1`, so hand it values in that
        // form. `checked_sub` rather than `- 1`: a stray 0 in 1-indexed data is
        // "no date", and must stay out of range so the slug is refused rather
        // than wrapping to a bogus December 31st.
        let to_zero_indexed = |v: Option<u64>| match encoding.offset() {
            1 => v,
            _ => v.and_then(|v| v.checked_sub(1)),
        };
        let slug = dev_player_key(
            Some(first.trim()),
            Some(last.trim()),
            f_year.and_then(|f| db.read_u64(bio, record, f)).map(|y| y + year_bias),
            to_zero_indexed(f_month.and_then(|f| db.read_u64(bio, record, f))),
            to_zero_indexed(f_day.and_then(|f| db.read_u64(bio, record, f))),
        );

        let bespoke_head = head_id == portrait_id;
        out.push(PlayerAssets {
            bio_record: record,
            full_name,
            slug,
            portrait_id,
            head_id,
            bespoke_head,
            pool_head: head_id >= POOL_HEAD_MIN,
            portrait_path: path_str(roots.portrait_path(portrait_id, PORTRAIT_DIR_LARGE)),
            portrait_small_path: path_str(roots.portrait_path(portrait_id, PORTRAIT_DIR_SMALL)),
            // Only a bespoke head belongs to this player. A pool head resolves
            // to a real file too, but naming it after this player would be a
            // lie — 5385 players share those models.
            mesh_path: path_str(bespoke_head.then(|| roots.mesh_path(head_id)).flatten()),
            texture_paths: if bespoke_head {
                roots
                    .texture_paths(head_id)
                    .into_iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect()
            } else {
                Vec::new()
            },
        });
    }
    Ok(out)
}

fn path_str(p: Option<PathBuf>) -> Option<String> {
    p.map(|p| p.to_string_lossy().into_owned())
}

/// Resolve a field on `table` by its **studio** id (canonical — handles the
/// big-endian reversal), erroring with the id when absent.
fn field_by_studio_id<'a>(
    db: &TdbFile,
    table: &'a Table,
    studio: &str,
) -> Result<&'a FieldDescriptor> {
    table
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == studio)
        .ok_or_else(|| {
            DomainError::Extract(format!(
                "field {studio} not present on {} — this title predates the asset-id fields",
                ids::BIO
            ))
        })
}

/// Read an inline-string field by its metadata `name`.
fn read_str_by_name(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    table: &Table,
    index: u32,
    name: &str,
) -> String {
    let Some(f) = table.fields.iter().find(|f| {
        meta.field(&table.id.raw(), &f.id.raw())
            .map(|m| m.name == name)
            .unwrap_or(false)
    }) else {
        return String::new();
    };
    db.read_inline_str(table, index, f)
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .unwrap_or_default()
}

// ---------------------------------------------------------------- decoding

/// A decoded image, ready to hand to the UI or write as a PNG.
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    /// Tightly packed RGBA8.
    pub rgba: Vec<u8>,
}

impl DecodedImage {
    /// Crop to the bounding box of pixels with alpha above `threshold`.
    ///
    /// Returns `self` unchanged when the image is fully transparent — there is
    /// no meaningful box to crop to, and returning a zero-sized image would
    /// just fail later at encode time.
    pub fn cropped_to_opaque(self, threshold: u8) -> DecodedImage {
        let (w, h) = (self.width as usize, self.height as usize);
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0usize, 0usize);
        for y in 0..h {
            for x in 0..w {
                if self.rgba[(y * w + x) * 4 + 3] > threshold {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
        if min_x > max_x || min_y > max_y {
            return self;
        }

        let (cw, ch) = (max_x - min_x + 1, max_y - min_y + 1);
        let mut rgba = Vec::with_capacity(cw * ch * 4);
        for y in min_y..=max_y {
            let row = (y * w + min_x) * 4;
            rgba.extend_from_slice(&self.rgba[row..row + cw * 4]);
        }
        DecodedImage {
            width: cw as u32,
            height: ch as u32,
            rgba,
        }
    }

    /// Encode as a PNG.
    pub fn to_png(&self) -> Result<Vec<u8>> {
        let buf = image::RgbaImage::from_raw(self.width, self.height, self.rgba.clone())
            .ok_or_else(|| DomainError::Extract("decoded image buffer size mismatch".into()))?;
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(buf)
            .write_to(&mut out, image::ImageFormat::Png)
            .map_err(|e| DomainError::Extract(format!("png encode failed: {e}")))?;
        Ok(out.into_inner())
    }
}

/// One raster pulled out of a portrait archive, normalised across the container
/// kinds so the caller can compare and decode them uniformly.
struct Raster {
    width: u32,
    height: u32,
    format: tdb_rx2::texlib::PixelFormat,
    /// Untiled, linear pixel bytes for the base mip.
    linear: Vec<u8>,
}

impl Raster {
    fn area(&self) -> u64 {
        (self.width as u64) * (self.height as u64)
    }
}

/// Decode a portrait `.big` to RGBA.
///
/// Keeps the largest raster by pixel area rather than the first: the archive
/// also carries Scaleform blobs, empty `sg*` stubs, and small auxiliary rasters
/// that would win a first-match scan.
pub fn decode_portrait(path: &Path) -> Result<DecodedImage> {
    let bytes = std::fs::read(path).map_err(|e| DomainError::Io {
        path: path.to_string_lossy().into_owned(),
        source: e,
    })?;
    let archive = tdb_archive_big::Archive::parse(bytes)
        .map_err(|e| DomainError::Extract(format!("{}: {e}", path.display())))?;

    let mut best: Option<Raster> = None;
    for entry in &archive.entries {
        let Ok(raw) = archive.read_entry_raw(&entry.name) else {
            continue;
        };
        let payload = if tdb_archive_codec::is_refpack(&raw) {
            match tdb_archive_codec::refpack_decompress(&raw) {
                Ok(p) => p,
                Err(_) => continue,
            }
        } else {
            raw
        };
        for cand in rasters_in(payload) {
            if best.as_ref().is_none_or(|b| cand.area() > b.area()) {
                best = Some(cand);
            }
        }
    }

    let raster = best.ok_or_else(|| {
        DomainError::Extract(format!(
            "{}: no portrait raster in archive ({} entries — neither an XPR2 nor an RX2 container)",
            path.display(),
            archive.entries.len()
        ))
    })?;
    Ok(DecodedImage {
        width: raster.width,
        height: raster.height,
        rgba: raster_to_rgba(&raster)?,
    })
}

/// Every raster one archive entry carries.
///
/// EA shipped portraits in **two different containers**, verified against real
/// installs:
///
/// * NHL Legacy / 14 / 15 — a RefPack-compressed, Xbox-tiled `XPR2` package.
/// * **NHL 10** — an uncompressed `\x89RW4xb2` RX2 container, the same format
///   the `rendering/` texlibs use. No RefPack, no XPR2.
///
/// Anything else (Scaleform `Apt Data` / `Apt constant file` blobs, the empty
/// `sg1`/`sg2` stubs) yields nothing, which is why this returns a list rather
/// than erroring per entry.
fn rasters_in(payload: Vec<u8>) -> Vec<Raster> {
    // XPR2 first: it validates a 4-byte magic, so it can't swallow an RX2.
    if let Ok(tex) = tdb_rx2::xpr::Xpr2Texture::parse(payload.clone()) {
        let format = tex
            .format
            .as_dxt()
            .unwrap_or(tdb_rx2::texlib::PixelFormat::Argb8888);
        return vec![Raster {
            width: tex.width,
            height: tex.height,
            linear: tex.decode_base_mip_linear(),
            format,
        }];
    }
    if let Ok(lib) = tdb_rx2::texlib::parse(&payload) {
        return lib
            .textures
            .into_iter()
            .map(|t| Raster {
                width: t.width,
                height: t.height,
                format: t.format,
                linear: t.base_mip_linear,
            })
            .collect();
    }
    Vec::new()
}

/// Turn an untiled base mip into RGBA8.
fn raster_to_rgba(r: &Raster) -> Result<Vec<u8>> {
    linear_to_rgba(r.format, r.width, r.height, &r.linear)
}

/// Turn an untiled, linear base mip into tightly packed RGBA8. Shared by the
/// portrait path (XPR2/RX2 rasters) and the head-preview path (texlib rasters).
pub(crate) fn linear_to_rgba(
    format: tdb_rx2::texlib::PixelFormat,
    width: u32,
    height: u32,
    linear: &[u8],
) -> Result<Vec<u8>> {
    use tdb_rx2::texlib::PixelFormat;

    match format {
        // Stored `[A,R,G,B]` per texel; RGBA8 wants the channels rotated, not
        // just copied.
        PixelFormat::Argb8888 => {
            let expected = (width as usize) * (height as usize) * 4;
            if linear.len() < expected {
                return Err(DomainError::Extract(format!(
                    "uncompressed raster short: {} bytes, need {expected}",
                    linear.len()
                )));
            }
            let mut rgba = Vec::with_capacity(expected);
            for px in linear[..expected].chunks_exact(4) {
                rgba.extend_from_slice(&[px[1], px[2], px[3], px[0]]);
            }
            Ok(rgba)
        }
        // Round-tripping through a 1-mip DDS reuses the block decoder rather
        // than duplicating BC1/2/3 decompression here.
        fmt => {
            let dds = tdb_archive_codec::encode_dds(dds_format_of(fmt)?, width, height, 1, linear);
            let decoded = tdb_archive_codec::decode_dds_to_rgba(&dds)
                .map_err(|e| DomainError::Extract(format!("dds decode failed: {e}")))?;
            Ok(decoded.rgba)
        }
    }
}

fn dds_format_of(fmt: tdb_rx2::texlib::PixelFormat) -> Result<tdb_archive_codec::DdsFormat> {
    use tdb_archive_codec::DdsFormat;
    use tdb_rx2::texlib::PixelFormat;
    Ok(match fmt {
        PixelFormat::Dxt1 => DdsFormat::Dxt1,
        PixelFormat::Dxt3 => DdsFormat::Dxt3,
        PixelFormat::Dxt5 => DdsFormat::Dxt5,
        other => {
            return Err(DomainError::Extract(format!(
                "unsupported pixel format {other:?}"
            )))
        }
    })
}

// ---------------------------------------------------------------- export

/// What to include in an export run.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ExportOptions {
    /// Portraits as PNG (the menu-size raster).
    pub portraits: bool,
    /// Head mesh as GLB — full skin, joints and weights.
    pub meshes_glb: bool,
    /// The untouched source `.rx2` containers alongside the decoded output, so
    /// later compatibility work can go back to the original bytes.
    pub raw_rx2: bool,
    /// Head textures as DDS, one per map (`_cm`, `_nm`, …).
    pub textures: bool,
    /// Crop each portrait to its opaque bounds.
    ///
    /// EA ships portraits as a fixed 512×512 canvas with the subject in the
    /// top-left and the rest fully transparent — the front-end crops via UVs at
    /// draw time. Off by default so the export stays byte-faithful to what
    /// shipped; turn it on when the goal is a tidy portrait library rather than
    /// a reproduction of the game's canvas.
    pub crop_portraits: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            portraits: true,
            meshes_glb: true,
            raw_rx2: true,
            textures: true,
            crop_portraits: false,
        }
    }
}

/// What an export run produced. Partial results are the norm — one unreadable
/// texlib must not sink 246 players' worth of work — so failures are counted
/// and described rather than propagated.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AssetExportSummary {
    pub players_considered: usize,
    pub portraits_written: usize,
    pub meshes_written: usize,
    pub textures_written: usize,
    pub raw_copied: usize,
    /// Players skipped because their bio yields no usable slug.
    pub skipped_no_slug: usize,
    /// Per-asset failures, already human-readable (player name + reason).
    pub failures: Vec<String>,
    pub bytes_written: u64,
}

/// Export art for `players` into `out_dir`, naming every file by the player's
/// slug.
///
/// Layout is one folder per asset class, flat within it:
///
/// ```text
/// <out>/portraits/<slug>.png
/// <out>/heads/<slug>.glb
/// <out>/heads/<slug>.rx2          (raw, when enabled)
/// <out>/textures/<slug>_<map>.dds
/// <out>/textures/<slug>_texlib.rx2 (raw, when enabled)
/// ```
pub fn export_player_assets(
    players: &[PlayerAssets],
    out_dir: &Path,
    opts: ExportOptions,
) -> Result<AssetExportSummary> {
    let mut summary = AssetExportSummary::default();

    // Slugs carry a birthdate, so a collision means two bio records really do
    // describe the same person — worth reporting, not silently overwriting.
    let mut seen: HashMap<&str, &str> = HashMap::new();

    for p in players {
        summary.players_considered += 1;
        let Some(slug) = p.slug.as_deref() else {
            summary.skipped_no_slug += 1;
            summary.failures.push(format!(
                "{}: no slug (bio lacks a usable name or birthdate)",
                p.full_name
            ));
            continue;
        };
        if let Some(prev) = seen.insert(slug, &p.full_name) {
            summary.failures.push(format!(
                "{}: slug {slug} already used by {prev} — skipped to avoid overwriting",
                p.full_name
            ));
            continue;
        }

        if opts.portraits {
            if let Some(src) = p.portrait_path.as_deref() {
                match decode_portrait(Path::new(src))
                    .map(|img| {
                        if opts.crop_portraits {
                            img.cropped_to_opaque(0)
                        } else {
                            img
                        }
                    })
                    .and_then(|img| img.to_png())
                {
                    Ok(png) => {
                        let dst = out_dir.join("portraits").join(format!("{slug}.png"));
                        match write_file(&dst, &png) {
                            Ok(n) => {
                                summary.portraits_written += 1;
                                summary.bytes_written += n;
                            }
                            Err(e) => summary.failures.push(format!("{}: {e}", p.full_name)),
                        }
                    }
                    Err(e) => summary
                        .failures
                        .push(format!("{}: portrait — {e}", p.full_name)),
                }
            }
        }

        if let Some(src) = p.mesh_path.as_deref().filter(|_| opts.meshes_glb) {
            match std::fs::read(src)
                .map_err(|e| DomainError::Extract(format!("read {src}: {e}")))
                .and_then(|b| {
                    tdb_rx2::playermesh::parse_player_mesh(&b)
                        .map_err(|e| DomainError::Extract(format!("{src}: {e}")))
                }) {
                Ok(asset) => {
                    let glb = tdb_rx2::playermesh::write_glb(&asset);
                    let dst = out_dir.join("heads").join(format!("{slug}.glb"));
                    match write_file(&dst, &glb) {
                        Ok(n) => {
                            summary.meshes_written += 1;
                            summary.bytes_written += n;
                        }
                        Err(e) => summary.failures.push(format!("{}: {e}", p.full_name)),
                    }
                }
                Err(e) => summary.failures.push(format!("{}: mesh — {e}", p.full_name)),
            }
        }

        if opts.textures {
            for src in &p.texture_paths {
                let src = src.as_str();
                match std::fs::read(src)
                    .map_err(|e| DomainError::Extract(format!("read {src}: {e}")))
                    .and_then(|b| {
                        tdb_rx2::texlib::parse(&b)
                            .map_err(|e| DomainError::Extract(format!("{src}: {e}")))
                    }) {
                    Ok(lib) => {
                        for tex in &lib.textures {
                            let map = map_suffix(&tex.name, p.head_id);
                            match dds_format_of(tex.format).map(|f| {
                                tdb_archive_codec::encode_dds(
                                    f,
                                    tex.width,
                                    tex.height,
                                    1,
                                    &tex.base_mip_linear,
                                )
                            }) {
                                Ok(dds) => {
                                    let dst = out_dir
                                        .join("textures")
                                        .join(format!("{slug}_{map}.dds"));
                                    match write_file(&dst, &dds) {
                                        Ok(n) => {
                                            summary.textures_written += 1;
                                            summary.bytes_written += n;
                                        }
                                        Err(e) => {
                                            summary.failures.push(format!("{}: {e}", p.full_name))
                                        }
                                    }
                                }
                                Err(e) => summary
                                    .failures
                                    .push(format!("{} [{}]: {e}", p.full_name, tex.name)),
                            }
                        }
                    }
                    Err(e) => summary
                        .failures
                        .push(format!("{}: texlib — {e}", p.full_name)),
                }
            }
        }

        if opts.raw_rx2 {
            // Keep each source container's own filename after the slug, so a
            // Legacy bundle lands as `<slug>_texlib.rx2` while NHL 10's loose
            // rasters stay distinguishable as `<slug>_head_cm.rx2` etc.
            let mut sources: Vec<(&str, PathBuf)> = Vec::new();
            if let Some(mesh) = p.mesh_path.as_deref() {
                sources.push((mesh, out_dir.join("heads").join(format!("{slug}.rx2"))));
            }
            for src in &p.texture_paths {
                let stem = Path::new(src)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let tag = raw_source_tag(&stem, p.head_id);
                sources.push((
                    src.as_str(),
                    out_dir.join("textures").join(format!("{slug}_{tag}.rx2")),
                ));
            }
            for (src, dst) in sources {
                match std::fs::read(src).map_err(|e| format!("read {src}: {e}")) {
                    Ok(bytes) => match write_file(&dst, &bytes) {
                        Ok(n) => {
                            summary.raw_copied += 1;
                            summary.bytes_written += n;
                        }
                        Err(e) => summary.failures.push(format!("{}: {e}", p.full_name)),
                    },
                    Err(e) => summary.failures.push(format!("{}: {e}", p.full_name)),
                }
            }
        }
    }

    Ok(summary)
}

/// The map suffix for a texlib raster: `head_67_cm.Raster` → `head_cm`. The id
/// in the middle is the head id we already know, so it carries no information in
/// the exported name — but a raster that doesn't follow the convention keeps its
/// own name rather than being force-fit.
fn map_suffix(raster_name: &str, head_id: u64) -> String {
    let stem = raster_name
        .split('.')
        .next()
        .unwrap_or(raster_name)
        .to_string();
    let needle = format!("_{head_id}_");
    match stem.split_once(&needle) {
        Some((part, map)) => format!("{part}_{map}"),
        None => stem,
    }
}

/// The name a copied source container gets after the player's slug.
/// `texlib_67` → `texlib` (Legacy's bundle); `head_10000_cm` → `head_cm`
/// (NHL 10's loose raster). The id is dropped either way — the slug already
/// identifies the player.
fn raw_source_tag(stem: &str, head_id: u64) -> String {
    if stem == format!("texlib_{head_id}") {
        return "texlib".to_string();
    }
    map_suffix(stem, head_id)
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<u64> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DomainError::Io {
            path: parent.to_string_lossy().into_owned(),
            source: e,
        })?;
    }
    std::fs::write(path, bytes).map_err(|e| DomainError::Io {
        path: path.to_string_lossy().into_owned(),
        source: e,
    })?;
    Ok(bytes.len() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portrait_bucket_covers_shipped_range_only() {
        assert_eq!(portrait_bucket(1), Some("p0_4000"));
        assert_eq!(portrait_bucket(4000), Some("p0_4000"));
        assert_eq!(portrait_bucket(4001), Some("p4001_8000"));
        assert_eq!(portrait_bucket(12000), Some("p8001_12000"));
        // 0 and beyond-12000 ship no portrait at all.
        assert_eq!(portrait_bucket(0), None);
        assert_eq!(portrait_bucket(12001), None);
        assert_eq!(portrait_bucket(POOL_HEAD_MIN), None);
    }

    #[test]
    fn loose_raster_matches_the_id_as_a_whole_token() {
        assert!(is_loose_raster_for(Path::new("head_10000_cm.rx2"), 10000));
        assert!(is_loose_raster_for(Path::new("hair_10000_cm.rx2"), 10000));
        // A prefix/suffix collision must not match: head 1000 never claims
        // head 11000's or 10000's files.
        assert!(!is_loose_raster_for(Path::new("head_11000_cm.rx2"), 1000));
        assert!(!is_loose_raster_for(Path::new("head_10000_cm.rx2"), 1000));
        // The mesh and the bundled texlib are not loose rasters.
        assert!(!is_loose_raster_for(Path::new("player_10000.rx2"), 10000));
        assert!(!is_loose_raster_for(Path::new("texlib_10000.rx2"), 10000));
        // Needs a trailing map token, and must be an .rx2.
        assert!(!is_loose_raster_for(Path::new("head_10000.rx2"), 10000));
        assert!(!is_loose_raster_for(Path::new("head_10000_cm.dds"), 10000));

        // NHL 09 layout: variant-infixed textures match on the map suffix,
        // but the same-named hair MESH (no map suffix) does not.
        assert!(is_loose_raster_for(Path::new("head_10000_0_0_0_cm.rx2"), 10000));
        assert!(is_loose_raster_for(Path::new("head_10000_0_0_0_nm.rx2"), 10000));
        assert!(!is_loose_raster_for(Path::new("hair_10000_0_0_0.rx2"), 10000)); // the hair mesh
    }

    #[test]
    fn raw_source_tag_names_both_layouts() {
        // Legacy's bundle, and NHL 10's loose rasters.
        assert_eq!(raw_source_tag("texlib_67", 67), "texlib");
        assert_eq!(raw_source_tag("head_10000_cm", 10000), "head_cm");
        assert_eq!(raw_source_tag("hair_10000_cm", 10000), "hair_cm");
    }

    #[test]
    fn map_suffix_strips_the_head_id() {
        assert_eq!(map_suffix("head_67_cm.Raster", 67), "head_cm");
        assert_eq!(map_suffix("hair_67_cm.Raster", 67), "hair_cm");
        assert_eq!(map_suffix("head_1292_nm.Raster", 1292), "head_nm");
    }

    #[test]
    fn map_suffix_keeps_unconventional_names_intact() {
        assert_eq!(map_suffix("something_else.Raster", 67), "something_else");
    }

    /// A 4×4 image with a single opaque pixel at (1, 2).
    fn one_dot() -> DecodedImage {
        let mut rgba = vec![0u8; 4 * 4 * 4];
        let idx = (2 * 4 + 1) * 4;
        rgba[idx..idx + 4].copy_from_slice(&[10, 20, 30, 255]);
        DecodedImage {
            width: 4,
            height: 4,
            rgba,
        }
    }

    #[test]
    fn crop_reduces_to_the_opaque_bounds() {
        let cropped = one_dot().cropped_to_opaque(0);
        assert_eq!((cropped.width, cropped.height), (1, 1));
        assert_eq!(cropped.rgba, vec![10, 20, 30, 255]);
    }

    #[test]
    fn crop_of_fully_transparent_image_is_a_no_op() {
        let blank = DecodedImage {
            width: 4,
            height: 4,
            rgba: vec![0u8; 4 * 4 * 4],
        };
        let out = blank.cropped_to_opaque(0);
        // Unchanged rather than zero-sized — a 0×0 image would fail to encode.
        assert_eq!((out.width, out.height), (4, 4));
    }

    #[test]
    fn crop_respects_the_alpha_threshold() {
        let mut img = one_dot();
        img.rgba[(2 * 4 + 1) * 4 + 3] = 8; // faint pixel
        let out = img.cropped_to_opaque(16);
        assert_eq!((out.width, out.height), (4, 4)); // below threshold => nothing opaque
    }

    /// Recomp keeps the real install one level down. Anchoring on the picked
    /// root would name `<root>/rendering` for a tree whose art extracts to
    /// `<root>/game/rendering` — the paths would be wrong *and* the "not
    /// extracted yet" message would point the user at a folder that never
    /// appears. Caught by driving the real app against an F:\Games\NHL Legacy
    /// recomp install.
    #[test]
    fn asset_roots_anchor_on_the_executable_not_the_picked_root() {
        let tmp = std::env::temp_dir().join("nhlms_anchor_test_recomp");
        let game = tmp.join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("default.xex"), b"stub").unwrap();

        let roots = AssetRoots::resolve(&tmp);
        assert_eq!(roots.rendering, game.join("rendering").to_string_lossy());
        assert_eq!(roots.fe, game.join("fe").to_string_lossy());

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// Xbox: the picked root *is* the executable's folder.
    #[test]
    fn asset_roots_anchor_on_the_root_when_the_exe_is_there() {
        let tmp = std::env::temp_dir().join("nhlms_anchor_test_xbox");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("default.xex"), b"stub").unwrap();

        let roots = AssetRoots::resolve(&tmp);
        assert_eq!(roots.rendering, tmp.join("rendering").to_string_lossy());

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// PS3 has no `.xex`; the install dir (`USRDIR`) holds the art directly.
    #[test]
    fn asset_roots_fall_back_to_the_install_dir_without_an_exe() {
        let tmp = std::env::temp_dir().join("nhlms_anchor_test_ps3");
        std::fs::create_dir_all(&tmp).unwrap();

        let roots = AssetRoots::resolve(&tmp);
        assert_eq!(roots.rendering, tmp.join("rendering").to_string_lossy());

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn asset_roots_overrides_ignore_empty_strings() {
        let roots = AssetRoots::resolve(Path::new("/install"));
        let rendering = roots.rendering.clone();
        let overridden = roots.with_overrides(Some("   "), Some("/custom/fe"));
        assert_eq!(overridden.rendering, rendering);
        assert_eq!(overridden.fe, "/custom/fe");
    }
}
