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
//! **A player has their own modelled head when a real, non-pool `player_{pPaD}`
//! mesh resolves on disk.** This is *file-truth*, not the old `pPaD == lOnr`
//! proxy: that equality was exact only on the Legacy calibration anchor (where
//! it picked 246 of 5632) and silently dropped every bespoke head whose head-id
//! space diverges from the portrait-id space — the bulk of the roster on most
//! titles (Legacy alone ships 1589 head files; see
//! `docs/audits/head-hair-census.md`). Pool ids (`>= 60000`, see
//! [`POOL_HEAD_MIN`]) are shared models belonging to no one player; they are
//! captured generically by the non-player sweep, not *as* a player. PS4 heads are
//! name-slug keyed (`{slug}.mcd`), so there the test is that slug file's presence.
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

use crate::{DomainError, Platform, Result};

/// Head ids at or above this are shared generic "pool" heads — one model reused
/// by many players, belonging to none of them. A Legacy-era convention: NHL 10
/// has no pool heads at all (every `pPaD` there is below this).
pub const POOL_HEAD_MIN: u64 = 60_000;

/// The pool-head floor of the **earlier** gen7 titles (NHL 09–13), whose shared heads live at
/// 10 000+ instead of 60 000+ (research 2026-09-21 pm handoff R53 §1: 10 000–10 099 on 09 / 10,
/// 10 000–10 181 on 11 / 13, to 30 100 on 12). Which floor applies is decided per file by
/// [`scan_player_assets`], not by a title list.
pub const POOL_HEAD_MIN_EARLY_GEN7: u64 = 10_000;

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
    /// Stored as-is — NHL 07 → 10 (the schema-family contract, `crate::encoding`; research R116 confirmed
    /// the declared ranges on 19 sources, 26 Sep 2026).
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

/// **Diagnostic since 25 Sep 2026.** Probe a database's cells for the birthdate convention they
/// look like. The display / edit / import contract is `crate::encoding` (schema family), not
/// this probe; `content_diagnostics` reports when the two disagree.
///
/// The two schemes are mutually exclusive, which makes this decidable from the
/// values alone: month `12` cannot occur 0-indexed, and month `0` cannot occur
/// 1-indexed. Either marker settles it, so this returns on the first one seen —
/// both are common enough (roughly a twelfth of players each) that it reads a
/// handful of records in practice, cheap enough to call per field read.
///
/// Falls back to `ZeroIndexed` when neither marker appears — a database too
/// small to tell. That matches 19 of the 21 committed anchors (NHL 11→21 on
/// PS3/PS4/Xbox 360); of the committed anchors NHL 10 is the 1-indexed case the probe sees (the
/// contract applies 1-indexing to NHL 07 → 10).
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

/// `ubPc` weight field (studio id): pounds minus a per-title base.
const F_WEIGHT: &str = "sNZW";

/// The base `ubPc.sNZW` subtracts from a player's weight in pounds: **150 on NHL 07 → NHL 10, 120
/// on NHL 11 → Legacy and every PS4 title** (Players 3.1, measured 25 Sep 2026 on all 27 anchors:
/// the non-zero raw values average 44–46 on the first family and 69–73 on the second; Lasse
/// Kukkonen, 190 lb, is raw 40 on NHL 08–10 and raw 70 on NHL 11–PS4 20). EA's metadata is the
/// same 8-bit `0..255` on every title, so the base is read off the stored distribution: the median
/// non-zero raw weight below 60 means the 150 base (NHL players do not average 165 lb), otherwise
/// 120. A table with no weights at all falls back to 120.
///
/// This is deliberately **not** the rating-scale probe (`sixbit_rating_offset`), which answers a
/// different question and misfired here: the PS4 titles carry no 6-bit rating field (→ "gen6" →
/// 150, 30 lb too heavy) and NHL 08's goalie table carries a 6-bit value above 49 (→ 120, 30 lb too
/// light). Both display and the mirrored edit offset follow this base.
/// **Diagnostic since 25 Sep 2026** — see [`crate::encoding`]: the contract decides the base;
/// this reports the base the current cell population looks like (an edited roster can move it).
pub fn probe_weight_base(db: &TdbFile, bio: &Table) -> i64 {
    let Ok(f_weight) = field_by_studio_id(db, bio, F_WEIGHT) else {
        return 120;
    };
    let mut raws: Vec<u64> = (0..bio.current_records as u32)
        .filter_map(|record| db.read_u64(bio, record, f_weight))
        .filter(|&w| w != 0)
        .collect();
    if raws.is_empty() {
        return 120;
    }
    raws.sort_unstable();
    let median = raws[raws.len() / 2];
    if median < 60 { 150 } else { 120 }
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
    /// Player meshes and texlibs. On Xbox/PS3/Recomp this is `<anchor>/rendering`
    /// (RenderWare `.rx2`); on **PS4** it is the "Kettle" tree
    /// `<game>/nhl/data/kettle/rendering` (EA `.mcd` meshes + `.gnf` textures).
    pub rendering: String,
    /// `<anchor>/fe` — the front-end tree (portraits, logos, jerseys). Empty on
    /// PS4, whose portraits are plain PNGs under `ui/assets/` (resolved separately).
    pub fe: String,
    /// The platform whose on-disk layout and naming these roots follow. Drives
    /// mesh/texture resolution — Xbox is head-id keyed under `player/`, PS4 is
    /// player-name-slug keyed under `characters/player/`.
    #[serde(default = "default_platform")]
    platform: Platform,
}

/// PS3/Xbox/Recomp share the RenderWare layout, so a missing `platform` (an old
/// serialized `AssetRoots`) decodes as Xbox — the pre-PS4 behavior.
fn default_platform() -> Platform {
    Platform::Xbox
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
    ///
    /// The **NHL Legacy Recompiled** port nests its art one level deeper still,
    /// under a `_compiled/` folder beside the `.xex` (`<anchor>/_compiled/rendering`
    /// rather than `<anchor>/rendering`). So each root prefers `<anchor>/_compiled/<name>`
    /// whenever that directory actually exists, and otherwise uses `<anchor>/<name>` —
    /// keyed on filesystem presence, so Xbox / PS3 installs (no `_compiled`) are
    /// unaffected.
    /// On **PS4** there is no executable to anchor on and a different tree: art
    /// lives under `<game>/nhl/data/kettle/rendering`. [`resolve_ps4`] derives it
    /// from the picked/discovered dir (which points at the database folder deep
    /// inside `nhl/data/...`) by walking up to the `nhl/data` ancestor.
    pub fn resolve(install_dir: &Path, platform: Platform) -> AssetRoots {
        if platform == Platform::Ps4 {
            return resolve_ps4(install_dir);
        }
        let anchor = anchor_dir(install_dir, platform);
        AssetRoots {
            rendering: art_root(&anchor, "rendering")
                .to_string_lossy()
                .into_owned(),
            fe: art_root(&anchor, "fe").to_string_lossy().into_owned(),
            platform,
        }
    }

    /// The platform these roots follow.
    pub fn platform(&self) -> Platform {
        self.platform
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

    /// Whether the art tree is present — i.e. this install has extracted art.
    /// A packed install (`.big` archives, nothing loose) reports `false`, which
    /// is the signal to run extraction first.
    ///
    /// PS4 has no separate `fe/` root (portraits are PNGs elsewhere), so it only
    /// checks the kettle mesh/texture dir exists.
    pub fn complete(&self) -> bool {
        if self.platform == Platform::Ps4 {
            return self.ps4_player_dir().is_dir();
        }
        self.rendering_dir().is_dir() && self.fe_dir().is_dir()
    }

    fn rendering_dir(&self) -> PathBuf {
        PathBuf::from(&self.rendering)
    }

    /// PS4 player character art dir: `<kettle>/rendering/characters/player`.
    fn ps4_player_dir(&self) -> PathBuf {
        self.rendering_dir().join("characters").join("player")
    }

    /// PS4 bespoke head mesh `<player>/{slug}.mcd`, if the slug resolves a file.
    fn ps4_bespoke_mesh(&self, name_slug: Option<&str>) -> Option<PathBuf> {
        let slug = name_slug?;
        let p = self.ps4_player_dir().join(format!("{slug}.mcd"));
        p.is_file().then_some(p)
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
    /// On **PS4** the head is a single EA `.mcd` (RSF) container under
    /// `characters/player/`, keyed by the player-name slug for bespoke heads
    /// (`{slug}.mcd`, e.g. `alexander_ovechkin.mcd`) and by the numeric id for
    /// shared pool heads (`generic_player_{head_id}.mcd`). Bespoke wins when its
    /// slug file exists; otherwise the generic mesh (so a pool head can still be
    /// inspected). `name_slug` is ignored on the RenderWare platforms.
    pub fn mesh_path(&self, head_id: u64, name_slug: Option<&str>) -> Option<PathBuf> {
        if self.platform == Platform::Ps4 {
            return self.ps4_bespoke_mesh(name_slug).or_else(|| {
                let generic = self
                    .ps4_player_dir()
                    .join(format!("generic_player_{head_id}.mcd"));
                generic.is_file().then_some(generic)
            });
        }
        let dir = self.rendering_dir().join("player");
        let nested = dir.join(format!("player_{head_id}"));
        let ext=if self.platform==Platform::Ps3 {"rpsgl"}else{"rx2"};
        [
            dir.join(format!("player_{head_id}.{ext}")), // NHL 10 → Legacy bundle
            dir.join(format!("head_{head_id}.{ext}")),   // NHL 08/09 loose
            nested.join(format!("head_{head_id}.{ext}")), // NHL 07 nested per-head folder
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
    pub fn extra_head_mesh_paths(&self, head_id: u64, _name_slug: Option<&str>) -> Vec<PathBuf> {
        // PS4 heads bundle hair (`geo_haircapShape`/`hair_cardsShape`) as
        // submeshes inside the one `.mcd`, so there are no companion meshes.
        if self.platform == Platform::Ps4 {
            return Vec::new();
        }
        let hair = self
            .rendering_dir()
            .join("player")
            .join(format!("hair_{head_id}_0_0_0.{}", if self.platform == Platform::Ps3 { "rpsgl" } else { "rx2" }));
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
    ///
    /// On **PS4** the head's maps are PS4 GNF textures under `characters/player/`,
    /// name-slug keyed: `head_{slug}_{map}.gnf`, `hair_{slug}_{map}.gnf`
    /// (`map` ∈ `cm`/`nm`/`sm`/`am`/`coeff`). (GNF *decode* lands with the texture
    /// work; this resolves the correct files today.)
    pub fn texture_paths(&self, head_id: u64, name_slug: Option<&str>) -> Vec<PathBuf> {
        if self.platform == Platform::Ps4 {
            let Some(slug) = name_slug else {
                return Vec::new();
            };
            let mut gnf: Vec<PathBuf> = std::fs::read_dir(self.ps4_player_dir())
                .into_iter()
                .flat_map(|entries| entries.flatten().map(|e| e.path()))
                .filter(|p| ps4_is_texture_for(p, slug))
                .collect();
            gnf.sort();
            return gnf;
        }
        let player_dir = self.rendering_dir().join("player");
        let ext=if self.platform==Platform::Ps3 {"rpsgl"}else{"rx2"};
        let bundled = player_dir.join(format!("texlib_{head_id}.{ext}"));
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
    ///
    /// On **PS4** portraits are a single size, so only the
    /// [`PORTRAIT_DIR_LARGE`] request resolves; the HUD-size request returns
    /// `None` (there is no separate small set). Two title-drifted layouts, both
    /// under `<nhl>/ui`, probed by existence:
    ///
    /// * **NHL 15** — flat PNGs, `assets/portraits/p{id}.png`.
    /// * **NHL 16+** — tiled BC7 GNFs, `kettle_assets/playerportraits/p{id}.gnf`.
    ///
    /// [`decode_portrait`] dispatches on the file's magic, so a mixed guess is safe.
    pub fn portrait_path(&self, portrait_id: u64, dir: &str) -> Option<PathBuf> {
        if self.platform == Platform::Ps4 {
            if dir != PORTRAIT_DIR_LARGE {
                return None;
            }
            let png = self
                .fe_dir()
                .join("assets")
                .join("portraits")
                .join(format!("p{portrait_id}.png"));
            if png.is_file() {
                return Some(png);
            }
            let gnf = self
                .fe_dir()
                .join("kettle_assets")
                .join("playerportraits")
                .join(format!("p{portrait_id}.gnf"));
            return gnf.is_file().then_some(gnf);
        }
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

/// Derive the PS4 art roots from a discovered/picked dir. Discovery points at
/// the database folder deep inside `nhl/data/common/gamemodes/database/t3db`, so
/// we walk up to the `nhl/data` ancestor and hang the kettle tree off it. Falls
/// back to a bounded downward search (the user pointed at the game root), and
/// finally to `<dir>/nhl/data/kettle/rendering` verbatim so a missing tree still
/// yields a stable, reportable path (`complete()` then reads `false`).
fn resolve_ps4(install_dir: &Path) -> AssetRoots {
    let nhl_data = ps4_art_data_root(install_dir);
    let rendering = nhl_data
        .join("kettle")
        .join("rendering")
        .to_string_lossy()
        .into_owned();
    // Portraits are plain PNGs under `<nhl>/ui/assets/portraits`, a sibling of
    // `<nhl>/data`. Use the `fe` slot for the `<nhl>/ui` front-end root (the PS4
    // analogue of the Xbox `fe/` tree).
    let fe = nhl_data
        .parent()
        .map(|nhl| nhl.join("ui").to_string_lossy().into_owned())
        .unwrap_or_default();
    AssetRoots {
        rendering,
        fe,
        platform: Platform::Ps4,
    }
}

/// Locate the `nhl/data` directory for a PS4 install, given a dir somewhere in
/// or above it. Checks each ancestor for a `nhl/data` child (the up-walk from a
/// deep db folder), then the dir itself, then a shallow downward search.
fn ps4_nhl_data_root(dir: &Path) -> PathBuf {
    // Up-walk: an ancestor whose path ends `…/nhl/data`, or one that has a
    // `nhl/data` child.
    for anc in dir.ancestors() {
        if anc.ends_with("nhl/data") || anc.ends_with("nhl\\data") {
            return anc.to_path_buf();
        }
        let cand = anc.join("nhl").join("data");
        if cand.is_dir() {
            return cand;
        }
    }
    // Downward (user picked the game root): bounded BFS for `nhl/data`.
    let mut queue = std::collections::VecDeque::from([(dir.to_path_buf(), 0usize)]);
    while let Some((d, depth)) = queue.pop_front() {
        let cand = d.join("nhl").join("data");
        if cand.is_dir() {
            return cand;
        }
        if depth < ANCHOR_SEARCH_DEPTH {
            if let Ok(entries) = std::fs::read_dir(&d) {
                let mut kids: Vec<PathBuf> = entries
                    .flatten()
                    .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                    .map(|e| e.path())
                    .collect();
                kids.sort();
                for k in kids {
                    queue.push_back((k, depth + 1));
                }
            }
        }
    }
    dir.join("nhl").join("data")
}

/// Depth bound for the downward search that hunts for the extracted art tree.
/// From the game root the art sits ~2 levels down (`databigs/Extracted/nhl/data`),
/// so a small cap keeps this from wandering into the multi-GB asset tree.
const PS4_ART_SEARCH_DEPTH: usize = 3;

/// Locate the `nhl/data` whose **extracted art** (`kettle/rendering`) is present.
///
/// The database and the extracted art can live in *different* `nhl/data` trees: the
/// db often ships loose under `<game>/nhl/data`, while the art extracts to a sibling
/// like `<game>/databigs/Extracted/nhl/data` (observed on NHL 19). Anchoring the art
/// root on the db's ancestor (what [`ps4_nhl_data_root`] does) then names an
/// art-less `<game>/nhl/data/kettle/rendering`, so `complete()` reads `false` and the
/// portraits/meshes never resolve. This finds the `nhl/data` that actually holds the
/// art, falling back to [`ps4_nhl_data_root`] (a stable, reportable path) when nothing
/// is extracted yet — which is exactly when `complete()` *should* read `false`.
/// The PS4 "kettle" tree (`<nhl>/data/kettle`) derived from a discovered or picked directory —
/// the parent of `rendering`, `audio`, `loc` and the rest. Shared by the art roots and by the
/// speech / context readers so one anchor walk serves both.
pub fn ps4_kettle_root(dir: &Path) -> PathBuf {
    ps4_art_data_root(dir).join("kettle")
}

fn ps4_art_data_root(dir: &Path) -> PathBuf {
    let has_art = |nhl_data: &Path| nhl_data.join("kettle").join("rendering").is_dir();
    // 1) Ancestors: an `…/nhl/data` (or `<anc>/nhl/data`) that already holds the art.
    for anc in dir.ancestors() {
        if (anc.ends_with("nhl/data") || anc.ends_with("nhl\\data")) && has_art(anc) {
            return anc.to_path_buf();
        }
        let cand = anc.join("nhl").join("data");
        if has_art(&cand) {
            return cand;
        }
    }
    // 2) Downward from the game root (the nearest ancestor with an `nhl` child): find
    //    any `nhl/data` holding the art — this catches `<game>/databigs/Extracted/…`.
    let game_root = dir
        .ancestors()
        .find(|a| a.join("nhl").is_dir())
        .unwrap_or(dir);
    if let Some(found) = ps4_find_art_data(game_root, has_art) {
        return found;
    }
    // 3) Nothing extracted yet — a stable path (`complete()` then reads false).
    ps4_nhl_data_root(dir)
}

/// Bounded, breadth-first search under `root` for an `nhl/data` directory whose art
/// tree is present (per `has_art`). Directories only, so the multi-GB asset payloads
/// are never touched; returns as soon as the first match is found.
fn ps4_find_art_data(root: &Path, has_art: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let mut queue = std::collections::VecDeque::from([(root.to_path_buf(), 0usize)]);
    while let Some((d, depth)) = queue.pop_front() {
        let cand = d.join("nhl").join("data");
        if has_art(&cand) {
            return Some(cand);
        }
        if depth < PS4_ART_SEARCH_DEPTH {
            if let Ok(entries) = std::fs::read_dir(&d) {
                let mut kids: Vec<PathBuf> = entries
                    .flatten()
                    .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                    .map(|e| e.path())
                    .collect();
                kids.sort();
                for k in kids {
                    queue.push_back((k, depth + 1));
                }
            }
        }
    }
    None
}

/// PS4 player-name slug for asset filenames: `First Last` → `first_last`. Keeps
/// ASCII letters/digits, maps whitespace to `_`, and drops other punctuation
/// (apostrophes/hyphens/accents are stripped in the shipped filenames). `None`
/// when nothing usable remains.
pub fn ps4_name_slug(first: &str, last: &str) -> Option<String> {
    let joined = format!("{} {}", first.trim(), last.trim());
    let mut slug = String::with_capacity(joined.len());
    for ch in joined.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() && !slug.ends_with('_') {
            slug.push('_');
        }
        // else: drop (apostrophes, hyphens, accents, …)
    }
    let slug = slug.trim_matches('_').to_string();
    (!slug.is_empty()).then_some(slug)
}

/// Is `path` a PS4 GNF texture for the head/hair of `name_slug`? Matches
/// `head_{slug}_{map}.gnf` / `hair_{slug}_{map}.gnf` — the slug as a whole run of
/// `_`-tokens (so `ovechkin` never picks up `ovechkinov`) and a known map suffix
/// last so a non-map `.gnf` isn't mistaken for one.
fn ps4_is_texture_for(path: &Path, name_slug: &str) -> bool {
    if path
        .extension()
        .is_none_or(|e| !e.eq_ignore_ascii_case("gnf"))
    {
        return false;
    }
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    let lower = stem.to_ascii_lowercase();
    for part in ["head_", "hair_"] {
        if let Some(rest) = lower.strip_prefix(part) {
            if let Some(map) = rest
                .strip_prefix(name_slug)
                .and_then(|r| r.strip_prefix('_'))
            {
                if PS4_MAP_SUFFIXES.contains(&map) {
                    return true;
                }
            }
        }
    }
    false
}

/// PS4 head texture map suffixes: colour, normal, specular, ambient, coefficient.
const PS4_MAP_SUFFIXES: &[&str] = &["cm", "nm", "sm", "am", "coeff"];

/// The directory the game's art hangs off: whichever folder holds `default.xex`,
/// searched from `install_dir` down to [`ANCHOR_SEARCH_DEPTH`]. Falls back to
/// `install_dir` when there's no executable to anchor on (PS3), which is also
/// the right answer there.
///
/// Breadth-first, so the shallowest match wins — a backup copy nested deeper
/// never outranks the real install.
fn anchor_dir(install_dir: &Path, platform: Platform) -> PathBuf {
    let mut queue = std::collections::VecDeque::from([(install_dir.to_path_buf(), 0usize)]);
    while let Some((dir, depth)) = queue.pop_front() {
        if (platform == Platform::Ps3 && (dir.join("EBOOT.BIN").is_file() || dir.join("eboot.bin").is_file()))
            || (platform != Platform::Ps3 && dir.join("default.xex").is_file()) {
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

/// Resolve one art root (`rendering` / `fe`) under `anchor`, preferring the
/// Recompiled port's `_compiled/` nesting when it's present on disk.
///
/// `<anchor>/_compiled/<name>` wins **only** when that directory exists, so this
/// is a no-op for Xbox / PS3 installs (which have no `_compiled/`) and for an
/// un-extracted Recomp tree — those fall back to `<anchor>/<name>`, which is also
/// the folder the "not extracted yet" flow should name.
fn art_root(anchor: &Path, name: &str) -> PathBuf {
    let compiled = anchor.join("_compiled").join(name);
    if compiled.is_dir() {
        compiled
    } else {
        anchor.join(name)
    }
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
    if path
        .extension()
        .is_none_or(|e| !e.eq_ignore_ascii_case("rx2") && !e.eq_ignore_ascii_case("rpsgl"))
    {
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
    /// A real, **non-pool** head mesh for this player resolves on disk — i.e.
    /// they have their own modelled head (== `mesh_path.is_some()`). Was the
    /// `pPaD == lOnr` proxy; now file-truth, so divergent-id bespoke heads count
    /// (see docs/audits/head-hair-census.md). PS4: the slug `.mcd` exists.
    pub bespoke_head: bool,
    /// The head id is in this title's **pool** range — a shared generic head, not this player's
    /// own. The floor is measured per file: 60 000+ on NHL 14–Legacy / PS4, 10 000+ on NHL 09–13
    /// (research 2026-09-21 pm handoff R53 §1). Titles that pack generics into *low* shared ids
    /// with no pool range at all are still caught by [`Self::head_shared`], which is the portable
    /// signal.
    pub pool_head: bool,
    /// This `pPaD` is referenced by more than one player — a shared generic head,
    /// so it is not attributed to this player even if it isn't in the pool range.
    /// The portable bespoke-vs-generic signal (see [`scan_player_assets`]).
    pub head_shared: bool,
    /// Resolved paths, `None` when the file isn't on disk.
    pub portrait_path: Option<String>,
    pub portrait_small_path: Option<String>,
    pub mesh_path: Option<String>,
    /// Every RX2 container holding one of this head's textures — a single
    /// bundled texlib on Legacy/14/15, or several loose per-raster files on
    /// NHL 10. Empty when none ship.
    pub texture_paths: Vec<String>,
    /// The resolved `portrait_path` is a **generic placeholder** shared by many
    /// un-portraited players (EA ships a blank plus a few default-face variants),
    /// not this player's own portrait. `false` until [`mark_generic_portraits`]
    /// has run over the full scan — a lone [`scan_player_assets`] can't tell a
    /// shared portrait from a unique one without seeing every player.
    pub generic_portrait: bool,
}

impl PlayerAssets {
    /// Anything worth exporting for this player. A generic placeholder portrait
    /// doesn't count — see [`mark_generic_portraits`].
    pub fn has_any(&self) -> bool {
        (self.portrait_path.is_some() && !self.generic_portrait)
            || self.mesh_path.is_some()
            || !self.texture_paths.is_empty()
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
    // `dev_player_key` composes the year as `qFnd + 1900`, but the early Xbox-360 titles pack the birth
    // year into a sub-8-bit field based at 1959 (NHL 07 = 6-bit, NHL 08 = 7-bit — see
    // `view::birth_year_base`). Bias the raw year by `base - 1900` so the slug (and its FHM identity
    // match) carries the real calendar year, not e.g. 1924 for a 1983 birth. Keyed on width, not title.
    let year_bias: u64 = match f_year {
        Some(f) if f.bit_width < 8 => 1959 - 1900,
        _ => 0,
    };
    // NHL 10 stores month/day already 1-indexed; Legacy stores them 0-indexed.
    // `dev_player_key` applies Legacy's `+1`, so pre-decrement NHL 10's values
    // to hand it the encoding it expects.
    // The family contract decides (`crate::encoding`); the population probe is only the fallback
    // for a schema no family claims, where the slug is diagnostic anyway.
    let encoding = crate::encoding::player_encoding(db, meta)
        .map(|e| e.birthdate)
        .unwrap_or_else(|| probe_birthdate_encoding(db, bio));

    // Distinct *people* per `pPaD` across the whole bio table. A head id used by
    // exactly one person is that person's own (bespoke); one used by two or more
    // different people is a generic/pool head — regardless of its numeric value.
    // This is the portable signal: the 60000 pool cutoff is a Legacy/NHL 15
    // convention (NHL 08–14 pack generics into LOW shared ids, with no 60000 range
    // at all — verified across the committed anchors), so uniqueness, not a fixed
    // threshold, separates bespoke from shared. Counting *people* rather than
    // records is deliberate: all-star / legends teams duplicate a star's bio, and
    // those extra entries must not demote their real head to "shared". The person
    // key is the raw name + birth fields — the same identity the export slug uses.
    // Mirrors the shared-vs-unique logic of [`mark_generic_portraits`].
    let person_key = |record: u32| -> String {
        let first = read_str_by_name(db, meta, bio, record, "firstname");
        let last = read_str_by_name(db, meta, bio, record, "lastname");
        let y = f_year
            .and_then(|f| db.read_u64(bio, record, f))
            .unwrap_or(0);
        let m = f_month
            .and_then(|f| db.read_u64(bio, record, f))
            .unwrap_or(0);
        let d = f_day.and_then(|f| db.read_u64(bio, record, f)).unwrap_or(0);
        format!(
            "{}|{}|{y}|{m}|{d}",
            first.trim().to_ascii_lowercase(),
            last.trim().to_ascii_lowercase()
        )
    };
    let mut head_persons: HashMap<u64, std::collections::HashSet<String>> = HashMap::new();
    for record in 0..bio.current_records as u32 {
        let hid = db.read_u64(bio, record, f_head).unwrap_or(0);
        head_persons
            .entry(hid)
            .or_default()
            .insert(person_key(record));
    }

    // The pool-head id floor of **this** file, measured rather than assumed. Research 2026-09-21 pm
    // handoff R53 §1 measured the gen7 pool ranges per title: 10 000–10 099 on NHL 09 / 10,
    // 10 000–10 181 on NHL 11 / 13, up to 30 100 on NHL 12, and **60 000+ from NHL 14 on**. Studio's
    // fixed `POOL_HEAD_MIN` (60 000) is the NHL 14+ convention and labelled every NHL 09–13 pool
    // head as non-pool.
    //
    // The two conventions are told apart by the file itself: a title using the 60 000+ space has
    // head ids there, and one using the 10 000+ space has none. So the floor is 60 000 when any
    // head id reaches it and 10 000 otherwise — no title parameter, and a patched or modified
    // database classifies itself. Bespoke head ids are portrait ids (buckets 0–12 000), so the
    // 10 000 floor is only ever consulted on files that have no 60 000+ ids at all.
    let pool_floor = if head_persons.keys().any(|id| *id >= POOL_HEAD_MIN) {
        POOL_HEAD_MIN
    } else {
        POOL_HEAD_MIN_EARLY_GEN7
    };

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
            f_year
                .and_then(|f| db.read_u64(bio, record, f))
                .map(|y| y + year_bias),
            to_zero_indexed(f_month.and_then(|f| db.read_u64(bio, record, f))),
            to_zero_indexed(f_day.and_then(|f| db.read_u64(bio, record, f))),
        );

        // The PS4 art filename slug (`first_last`) is distinct from the export
        // slug above (`last_first_dd_mm_yyyy`). Only needed on PS4.
        let ps4_slug = (roots.platform() == Platform::Ps4)
            .then(|| ps4_name_slug(first.trim(), last.trim()))
            .flatten();
        let pool_head = head_id >= pool_floor;
        let head_shared = head_persons
            .get(&head_id)
            .map(|people| people.len() > 1)
            .unwrap_or(false);
        // "This player's own head" = a **uniquely-referenced**, non-pool head that
        // resolves on disk. This replaces the old `pPaD == lOnr` proxy, which was
        // exact only on the Legacy calibration anchor and silently dropped every
        // bespoke head whose head-id space diverges from the portrait-id space
        // (Ovechkin et al. on the titles where they differ). A head id shared by
        // many players is generic — captured once by the Phase-4 sweep, never
        // named after one of the sharers. PS4 keeps its slug-file test: its heads
        // are name-keyed, and a generic there is a numeric `generic_player_*`, so a
        // resolved `{slug}.mcd` is inherently unique to that player.
        let own_mesh = match roots.platform() {
            Platform::Ps4 => roots.ps4_bespoke_mesh(ps4_slug.as_deref()),
            _ if head_id == 0 || pool_head || head_shared => None,
            _ => roots.mesh_path(head_id, None),
        };
        let bespoke_head = own_mesh.is_some();
        let texture_paths = if own_mesh.is_some() {
            roots
                .texture_paths(head_id, ps4_slug.as_deref())
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect()
        } else {
            Vec::new()
        };
        out.push(PlayerAssets {
            bio_record: record,
            full_name,
            slug,
            portrait_id,
            head_id,
            bespoke_head,
            pool_head,
            head_shared,
            portrait_path: path_str(roots.portrait_path(portrait_id, PORTRAIT_DIR_LARGE)),
            portrait_small_path: path_str(roots.portrait_path(portrait_id, PORTRAIT_DIR_SMALL)),
            mesh_path: path_str(own_mesh),
            texture_paths,
            // Filled by `mark_generic_portraits` (needs the whole set); a bare
            // scan can't know whether a portrait is shared.
            generic_portrait: false,
        });
    }
    Ok(out)
}

/// Flag every player whose `portrait_path` is a **generic placeholder** rather
/// than their own portrait, so the exporter can skip the blanks.
///
/// EA gives un-portraited players a shared stand-in — a blank plus a handful of
/// default-face variants — so hundreds of records point at a portrait file with
/// **byte-identical** content. A real portrait, by contrast, is unique to one
/// player. So the rule is exactly that: a portrait shared (same bytes) by more
/// than one player is generic. This is title-agnostic — a title where every
/// portrait is unique simply flags nothing — and needs the full player set,
/// which is why it's a separate pass rather than part of [`scan_player_assets`].
///
/// Each distinct portrait file is read once (they're small `.big`s); a file that
/// can't be read is treated as non-generic (fail open — better a stray blank
/// than a dropped real portrait).
pub fn mark_generic_portraits(players: &mut [PlayerAssets]) {
    use std::collections::hash_map::Entry;
    // portrait_path -> content fingerprint (len, fnv-1a hash), read once per file.
    let mut fingerprints: HashMap<String, Option<(usize, u64)>> = HashMap::new();
    let mut fp_of = |path: &str| -> Option<(usize, u64)> {
        match fingerprints.entry(path.to_string()) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let fp = std::fs::read(path).ok().map(|b| (b.len(), fnv1a(&b)));
                *e.insert(fp)
            }
        }
    };

    // How many players share each portrait content.
    let mut counts: HashMap<(usize, u64), u32> = HashMap::new();
    for p in players.iter() {
        if let Some(path) = p.portrait_path.as_deref() {
            if let Some(fp) = fp_of(path) {
                *counts.entry(fp).or_default() += 1;
            }
        }
    }

    for p in players.iter_mut() {
        p.generic_portrait = p
            .portrait_path
            .as_deref()
            .and_then(&mut fp_of)
            .map(|fp| counts.get(&fp).copied().unwrap_or(0) > 1)
            .unwrap_or(false);
    }
}

/// FNV-1a 64-bit — a cheap content fingerprint for spotting byte-identical
/// portrait files. Not cryptographic; collisions here would only mis-skip a
/// portrait, and the length pairing makes even that vanishingly unlikely.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
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

    /// Crop for the **bottom-anchored header portrait** framing.
    ///
    /// Vertically, keep from the top of the visible content down to the lowest
    /// *solid* row — the lowest row that still holds a fully-opaque pixel
    /// (alpha `>= SOLID`). Any faded / alpha-gradient bottom edge below that row
    /// (shoulders trailing off into transparency) is dropped, so the caller can
    /// pin the result's bottom to the tile and know it sits on real content, not
    /// a fade. Horizontally, crop to the content bounds over that kept span.
    ///
    /// Horizontally, crop to the content bounds over that kept span, then pad
    /// symmetrically with transparency so the **highest point** (the midpoint of
    /// the topmost content row — the crown of the head) is the exact horizontal
    /// centre of the result. A plain centre-in-tile then lands the crown in the
    /// middle, steadier than centring on the shoulder-driven bounding box.
    ///
    /// `threshold` bounds what counts as visible content for the horizontal and
    /// top extent (ignores near-transparent dust). Falls back to
    /// [`Self::cropped_to_opaque`] when nothing solid exists to anchor to.
    pub fn cropped_to_solid_bottom(self, threshold: u8) -> DecodedImage {
        /// Alpha at/above which a pixel counts as fully opaque ("no transparency").
        /// Just under 255 to tolerate codec quantization of a solid edge.
        const SOLID: u8 = 250;

        let (w, h) = (self.width as usize, self.height as usize);
        let mut top: Option<usize> = None;
        let mut solid_bottom: Option<usize> = None;
        for y in 0..h {
            let mut has_content = false;
            let mut has_solid = false;
            for x in 0..w {
                let a = self.rgba[(y * w + x) * 4 + 3];
                has_content |= a > threshold;
                has_solid |= a >= SOLID;
            }
            if has_content && top.is_none() {
                top = Some(y);
            }
            if has_solid {
                solid_bottom = Some(y);
            }
        }
        let (Some(top), Some(bottom)) = (top, solid_bottom) else {
            return self.cropped_to_opaque(threshold);
        };
        if bottom < top {
            return self.cropped_to_opaque(threshold);
        }

        // Horizontal bounds over the kept vertical span only.
        let (mut min_x, mut max_x) = (w, 0usize);
        for y in top..=bottom {
            for x in 0..w {
                if self.rgba[(y * w + x) * 4 + 3] > threshold {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                }
            }
        }
        if min_x > max_x {
            return self.cropped_to_opaque(threshold);
        }

        // Highest-point horizontal centre: the midpoint of the content span in the
        // topmost row (the crown of the head). We pad the crop symmetrically about
        // this x so a plain centre-in-tile lands the highest point in the middle —
        // steadier than centring on the (shoulder-driven) bounding box.
        let (mut hx_min, mut hx_max) = (max_x, min_x);
        for x in min_x..=max_x {
            if self.rgba[(top * w + x) * 4 + 3] > threshold {
                hx_min = hx_min.min(x);
                hx_max = hx_max.max(x);
            }
        }
        let cw = max_x - min_x + 1;
        let ch = bottom - top + 1;
        // Highest-point x within the crop, and the padding that centres it.
        let cx = (hx_min + hx_max) / 2 - min_x;
        let half = cx.max(cw - 1 - cx); // half-width of the centred output
        let left_pad = half - cx;
        let out_w = 2 * half + 1;

        let mut rgba = vec![0u8; out_w * ch * 4]; // transparent padding
        for (r, y) in (top..=bottom).enumerate() {
            let src = (y * w + min_x) * 4;
            let dst = (r * out_w + left_pad) * 4;
            rgba[dst..dst + cw * 4].copy_from_slice(&self.rgba[src..src + cw * 4]);
        }
        DecodedImage {
            width: out_w as u32,
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

    /// PNG-encode with fast (low-CPU) settings — `Fast` deflate + a single up-filter instead of the
    /// default adaptive per-row filter search. Files are a little larger but encoding is several
    /// times cheaper, which matters when a bulk export writes thousands of 1024² textures at once.
    pub fn to_png_fast(&self) -> Result<Vec<u8>> {
        use image::codecs::png::{CompressionType, FilterType, PngEncoder};
        use image::{ExtendedColorType, ImageEncoder};
        let mut out = Vec::new();
        PngEncoder::new_with_quality(&mut out, CompressionType::Fast, FilterType::Up)
            .write_image(
                &self.rgba,
                self.width,
                self.height,
                ExtendedColorType::Rgba8,
            )
            .map_err(|e| DomainError::Extract(format!("png encode failed: {e}")))?;
        Ok(out)
    }

    /// Downscale so the longest edge is at most `max_edge` (aspect preserved), for
    /// a lightweight list thumbnail. Returns `self` unchanged when already within
    /// bounds (never upscales) or `max_edge == 0`.
    pub fn thumbnail(self, max_edge: u32) -> DecodedImage {
        let long = self.width.max(self.height);
        if max_edge == 0 || long <= max_edge {
            return self;
        }
        let scale = max_edge as f64 / long as f64;
        let nw = ((self.width as f64 * scale).round() as u32).max(1);
        let nh = ((self.height as f64 * scale).round() as u32).max(1);
        let Some(buf) = image::RgbaImage::from_raw(self.width, self.height, self.rgba.clone())
        else {
            return self;
        };
        let scaled = image::imageops::thumbnail(&buf, nw, nh);
        DecodedImage {
            width: scaled.width(),
            height: scaled.height(),
            rgba: scaled.into_raw(),
        }
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

/// Read a player-asset file, transparently unwrapping an outer RefPack layer.
///
/// NHL 08 ships its loose head meshes (`head_<id>.rx2`) and portrait archives
/// (`p<id>.big`) as **whole-file RefPack streams** — the `\x89RW4xb2` / `BIGF`
/// container only appears after decompression. (NHL 07 and NHL 09→Legacy store
/// these files uncompressed; the portrait's *inner* DDS entries are RefPacked in
/// every title, handled separately in `decode_portrait`.) Any file that isn't a
/// RefPack stream is returned verbatim, so this is safe to call unconditionally.
pub(crate) fn read_asset_file(path: &Path) -> Result<Vec<u8>> {
    let bytes = std::fs::read(path).map_err(|e| DomainError::Io {
        path: path.to_string_lossy().into_owned(),
        source: e,
    })?;
    if tdb_archive_codec::is_refpack(&bytes) {
        if let Ok(inner) = tdb_archive_codec::refpack_decompress(&bytes) {
            return Ok(inner);
        }
    }
    Ok(bytes)
}

/// Parse a head mesh into the shared [`tdb_rx2::playermesh::PlayerAsset`],
/// dispatching on the container the path names: PS4 `.mcd` (EA RSF) vs the
/// Xbox 360 / PS3 `.rx2` (RenderWare RW4). Both decoders emit the same model,
/// so [`tdb_rx2::playermesh::write_glb`] and the head-preview path are shared.
///
/// `preview` selects the lenient RW4 parse (tolerates NHL 10's per-submesh
/// multi-skin rig — viewer only). The PS4 rig is external (in the `.skel`), so
/// `.mcd` has no such ambiguity and ignores the flag.
pub(crate) fn parse_head_mesh(
    path: &Path,
    bytes: &[u8],
    preview: bool,
) -> Result<tdb_rx2::playermesh::PlayerAsset> {
    let err = |e: String| DomainError::Extract(format!("{}: {e}", path.display()));
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mcd"))
    {
        return tdb_rx2::mcd::parse_player_mesh(bytes).map_err(|e| err(e.to_string()));
    }
    if preview {
        tdb_rx2::playermesh::parse_player_mesh_preview(bytes).map_err(|e| err(e.to_string()))
    } else {
        tdb_rx2::playermesh::parse_player_mesh(bytes).map_err(|e| err(e.to_string()))
    }
}

/// Decode a head mesh for **geometry-only** use (the appearance catalog): tolerates
/// any AnimationSkin layout, including none — so rigid meshes like the standalone
/// hair-style variants (`hair_*_aa.rx2`, no skin chunk) decode instead of being
/// refused. Positions/UVs/triangles only; the rig is not resolved.
pub(crate) fn parse_head_mesh_geometry(
    path: &Path,
    bytes: &[u8],
) -> Result<tdb_rx2::playermesh::PlayerAsset> {
    let err = |e: String| DomainError::Extract(format!("{}: {e}", path.display()));
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mcd"))
    {
        return tdb_rx2::mcd::parse_player_mesh(bytes).map_err(|e| err(e.to_string()));
    }
    tdb_rx2::playermesh::parse_player_mesh_geometry(bytes).map_err(|e| err(e.to_string()))
}

/// PNG magic — the head of a PS4 portrait (`p{id}.png`).
const PNG_MAGIC: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// Decode PNG bytes to a [`DecodedImage`] (RGBA8).
fn decode_png(bytes: &[u8], path: &Path) -> Result<DecodedImage> {
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|e| DomainError::Extract(format!("{}: png decode failed: {e}", path.display())))?
        .to_rgba8();
    Ok(DecodedImage {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    })
}

/// Decode a portrait to RGBA.
///
/// Dispatched by file magic:
/// * **PS4 / NHL 15** — plain PNG (`assets/portraits/p{id}.png`).
/// * **PS4 / NHL 16+** — tiled BC7 GNF (`kettle_assets/playerportraits/p{id}.gnf`).
/// * **Xbox / PS3** — EA `.big` archive: keep the largest raster by pixel area
///   rather than the first (the archive also carries Scaleform blobs, empty
///   `sg*` stubs, and small auxiliary rasters that would win a first-match scan).
pub fn decode_portrait(path: &Path) -> Result<DecodedImage> {
    let bytes = read_asset_file(path)?;
    if bytes.starts_with(PNG_MAGIC) {
        return decode_png(&bytes, path);
    }
    if bytes.starts_with(tdb_rx2::gnf::MAGIC_GNF) {
        let tex = tdb_rx2::gnf::parse(&bytes)
            .map_err(|e| DomainError::Extract(format!("{}: {e}", path.display())))?;
        let rgba = gnf_to_rgba(&tex)?;
        return Ok(DecodedImage {
            width: tex.width,
            height: tex.height,
            rgba,
        });
    }
    best_portrait_raster(bytes, path)
}

/// Pick and decode the largest raster out of a `BIGF` portrait/logo archive. The
/// entries are RefPack-unwrapped and every recognised raster kind (XPR2 / DDS /
/// RX2) is compared by area, so the "main" picture wins over any thumbnail slot.
/// Shared by [`decode_portrait`] and [`decode_texture_file`].
fn best_portrait_raster(bytes: Vec<u8>, path: &Path) -> Result<DecodedImage> {
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
            "{}: no image found in this package ({} entries); it may contain only interface drawing data",
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

/// The visible name stem of a texlib raster, dropping the `.Raster`/`.mip` tail EA appends
/// internally (`jersey_0_10_0_cm.Raster` → `jersey_0_10_0_cm`).
fn raster_name_stem(name: &str) -> &str {
    name.split('.').next().unwrap_or(name)
}

/// Rank a texlib raster by how likely it is the *visible colour picture*: a colour map (`_cm`) wins,
/// an obvious non-colour map (spec/normal/roughness/…) loses, anything else is neutral. EA's
/// jersey/equipment texlibs bundle a colour map **and a same-size spec map**, so plain
/// largest-by-area selection can land on the grayscale spec map (the "jersey previews as grayscale"
/// bug). Pairing this rank ahead of area picks the colour map; ties still break on larger area.
fn raster_colour_rank(name: &str) -> u8 {
    match raster_name_stem(name).rsplit('_').next() {
        Some("cm") => 2,
        Some(t) if MAP_TOKENS.contains(&t) => 0, // sm / nm / rm / mm / am / coeff / spec / gloss
        _ => 1,
    }
}

/// Pick the index of a texlib's primary (visible colour) raster: colour map first, then largest.
fn primary_raster_index(textures: &[tdb_rx2::texlib::Texture]) -> Option<usize> {
    textures
        .iter()
        .enumerate()
        .max_by_key(|(_, t)| {
            (
                raster_colour_rank(&t.name),
                t.width as u64 * t.height as u64,
            )
        })
        .map(|(i, _)| i)
}

/// The PS3 counterpart of [`primary_raster_index`]: colour map first, then largest.
fn primary_ps3_raster_index(lib: &tdb_rx2::texlib_ps3::TexlibPs3) -> Option<usize> {
    lib.textures
        .iter()
        .enumerate()
        .max_by_key(|(_, t)| (raster_colour_rank(&t.name), t.width as u64 * t.height as u64))
        .map(|(i, _)| i)
}

/// Decode an extracted texture file to RGBA for preview, tagging which container
/// it turned out to be. The read-side companion to [`replace_texture_in_file`]:
/// it recognises the same three extracted containers, so the file browser can
/// show a "before" image (and a row thumbnail) for anything it can also write
/// back.
///
/// * `GNF ` → the PS4 Kettle texture's base mip.
/// * `BIGF` → the largest raster in the portrait/logo archive (XPR2 / DDS slot).
/// * `\x89RW4` → the largest raster in the RW4 texlib (logo / uniform / equipment).
///
/// An outer whole-file RefPack layer (NHL 08's wrapped files) is unwrapped first
/// via [`read_asset_file`], so those decode too.
///
/// `texture_index` targets a specific raster inside a multi-texture RW4 texlib (a jersey bundle holds
/// the body colour map, the number `font` map, and their spec maps) — `None` auto-picks the primary
/// colour map. It only applies to the texlib container; GNF/portrait hold a single image.
pub fn decode_texture_file(
    path: &Path,
    texture_index: Option<usize>,
) -> Result<(TextureContainer, DecodedImage)> {
    let bytes = read_asset_file(path)?;
    decode_texture_bytes(bytes, path, texture_index)
}

/// Read one member of an `EB\0\3` archive (PS4, Xbox 360 Legacy, PS3 NHL 12 / 13 render archives)
/// by exact member path: the front tables are parsed, the member's stored bytes are streamed and
/// its payload decoded (`chunkzip` / `chunklzx` / raw). Nothing else of the archive is read; a
/// member the table does not carry, or a payload the codecs cannot decode, is an error naming it.
pub fn read_eb_member(archive: &Path, member: &str) -> Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    use tdb_archive_big::eb::{self, EbHeader, Payload};
    let label = archive.display().to_string();
    let io = |e: std::io::Error| DomainError::Io { path: label.clone(), source: e };
    let mut file = std::fs::File::open(archive).map_err(io)?;
    let mut header_buf = vec![0u8; eb::HEADER_LEN];
    file.read_exact(&mut header_buf).map_err(io)?;
    let header = EbHeader::parse(&header_buf).map_err(|e| DomainError::Extract(format!("{label}: {e}")))?;
    let mut table_buf = vec![0u8; header.table_end()];
    file.seek(SeekFrom::Start(0)).map_err(io)?;
    file.read_exact(&mut table_buf).map_err(io)?;
    let table = eb::EbArchive::parse(&table_buf).map_err(|e| DomainError::Extract(format!("{label}: {e}")))?;
    let wanted = member.replace('\\', "/");
    let entry = table
        .entries
        .iter()
        .find(|e| e.path.replace('\\', "/") == wanted)
        .ok_or_else(|| DomainError::Extract(format!("{label}: no member `{member}` in the archive's file table")))?;
    let mut raw = vec![0u8; entry.size as usize];
    file.seek(SeekFrom::Start(entry.offset)).map_err(io)?;
    file.read_exact(&mut raw).map_err(io)?;
    match eb::decode_payload(raw) {
        Ok(Payload::Data(d)) => Ok(d),
        Ok(Payload::UnsupportedLzx) => Err(DomainError::Extract(format!("{label}: member `{member}` is stored in an LZX form Studio does not decode"))),
        Err(e) => Err(DomainError::Extract(format!("{label}: member `{member}`: {e}"))),
    }
}

/// [`decode_texture_file`] for a texture that lives inside an `EB\0\3` archive (the PS3 NHL 12 /
/// 13 kit textures in `cacherender.big` / `nocacherender.big`): the member is read with
/// [`read_eb_member`] and decoded exactly as a loose file would be. Read-only.
pub fn decode_archive_member_texture(archive: &Path, member: &str, texture_index: Option<usize>) -> Result<(TextureContainer, DecodedImage)> {
    let bytes = read_eb_member(archive, member)?;
    let label = archive.join(member);
    decode_texture_bytes(bytes, &label, texture_index)
}

/// The container dispatch behind [`decode_texture_file`], on bytes already read; `path` labels
/// the errors only.
pub fn decode_texture_bytes(bytes: Vec<u8>, path: &Path, texture_index: Option<usize>) -> Result<(TextureContainer, DecodedImage)> {
    if bytes.starts_with(PNG_MAGIC) {
        return Ok((TextureContainer::Png, decode_png(&bytes, path)?));
    }
    if bytes.starts_with(tdb_rx2::gnf::MAGIC_GNF) {
        let tex = tdb_rx2::gnf::parse(&bytes)
            .map_err(|e| DomainError::Extract(format!("{}: {e}", path.display())))?;
        let rgba = gnf_to_rgba(&tex)?;
        return Ok((
            TextureContainer::Gnf,
            DecodedImage {
                width: tex.width,
                height: tex.height,
                rgba,
            },
        ));
    }
    if bytes.starts_with(b"BIGF") {
        return Ok((
            TextureContainer::Portrait,
            best_portrait_raster(bytes, path)?,
        ));
    }
    if bytes.starts_with(tdb_rx2::texlib_ps3::MAGIC) {
        // PS3 texlib (`.rpsgl` rink/ice/jersey bundles, NHL 08–11 logo members): plain slots.
        let lib = tdb_rx2::texlib_ps3::parse(&bytes)
            .map_err(|e| DomainError::Extract(format!("{}: RW4ps3 parse: {e}", path.display())))?;
        let idx = match texture_index {
            Some(i) if i < lib.textures.len() => i,
            Some(i) => {
                return Err(DomainError::Extract(format!(
                    "{}: texture index {i} out of range ({} rasters)",
                    path.display(),
                    lib.textures.len()
                )))
            }
            None => primary_ps3_raster_index(&lib).ok_or_else(|| {
                DomainError::Extract(format!("{}: texlib has no texture", path.display()))
            })?,
        };
        let t = &lib.textures[idx];
        let base = lib.base_mip_linear(idx).ok_or_else(|| {
            DomainError::Extract(format!("{}: raster {idx} slot is truncated", path.display()))
        })?;
        let rgba = linear_to_rgba(ps3_format_to_texlib(t.format), t.width, t.height, base)?;
        return Ok((
            TextureContainer::Texlib,
            DecodedImage {
                width: t.width,
                height: t.height,
                rgba,
            },
        ));
    }
    if bytes.starts_with(b"\x89RW4") {
        let lib = tdb_rx2::texlib::parse(&bytes)
            .map_err(|e| DomainError::Extract(format!("{}: texlib parse: {e}", path.display())))?;
        // An explicit index selects one raster (the jersey body vs. the number font); `None`
        // auto-picks the colour map — a bundle carries a same-size grayscale spec map, so plain
        // largest-area would show that instead of the colour.
        let idx = match texture_index {
            Some(i) if i < lib.textures.len() => i,
            Some(i) => {
                return Err(DomainError::Extract(format!(
                    "{}: texture index {i} out of range ({} rasters)",
                    path.display(),
                    lib.textures.len()
                )))
            }
            None => primary_raster_index(&lib.textures).ok_or_else(|| {
                DomainError::Extract(format!("{}: texlib has no texture", path.display()))
            })?,
        };
        let best = &lib.textures[idx];
        let rgba = linear_to_rgba(best.format, best.width, best.height, &best.base_mip_linear)?;
        return Ok((
            TextureContainer::Texlib,
            DecodedImage {
                width: best.width,
                height: best.height,
                rgba,
            },
        ));
    }
    Err(DomainError::Extract(format!(
        "{}: unrecognized texture container (expected a GNF, a BIGF portrait archive, or an RW4 texlib)",
        path.display()
    )))
}

/// A pickable portrait texture slot, tagged by the platform's inner encoding.
/// Xbox 360 front-ends wrap an `XPR2` (Xbox-tiled DXT); PS3 front-ends wrap a
/// plain little-endian `DDS ` (DXT1/3/5). Both sit inside the same `BIGF` +
/// RefPack envelope — only the inner texture differs — so the picker sniffs
/// which and the writer branches. Mirrors the prior app's `PortraitTarget`.
enum PortraitSlot {
    Xpr2 {
        entry: String,
        xpr: tdb_rx2::xpr::Xpr2Texture,
    },
    /// PS3 DXT DDS member (portraits ship a full mip chain; the NHL 12–Legacy `leaguelogossmall`
    /// DXT5 logos ship the base level only, `mips == 0`). The shipped 128-byte header is kept
    /// verbatim apart from the size words, and the output carries the shipped number of levels.
    Dds {
        entry: String,
        width: u32,
        height: u32,
        fmt: tdb_archive_codec::DdsFormat,
        header: Vec<u8>,
        /// Declared mip levels (0/1 = base only).
        mips: u32,
    },
    /// PS3 NHL 12–Legacy league-logo member: an uncompressed 32-bit DDS. The shipped 128-byte
    /// header is kept verbatim and the texels are re-written in its channel order.
    DdsArgb {
        entry: String,
        header: Vec<u8>,
        info: DdsArgb,
    },
    /// Xbox 360 NHL 08–11 league-logo member: an `RW4xb2` texlib (raw inside the RefPack'd
    /// package on 08–10, RefPack'd inside the loose package on 11). `wrapped` records which,
    /// so the rewrite keeps the member's own encoding.
    Rw4Xb2 {
        entry: String,
        wrapped: bool,
        lib: Box<tdb_rx2::texlib::Texlib>,
        idx: usize,
    },
    /// PS3 NHL 08–11 league-logo member: an `RW4ps3` texlib (raw on 08, RefPack'd on 09–11).
    Rw4Ps3 {
        entry: String,
        wrapped: bool,
        lib: Box<tdb_rx2::texlib_ps3::TexlibPs3>,
        idx: usize,
    },
}

impl PortraitSlot {
    fn area(&self) -> u64 {
        match self {
            PortraitSlot::Xpr2 { xpr, .. } => xpr.width as u64 * xpr.height as u64,
            PortraitSlot::Dds { width, height, .. } => *width as u64 * *height as u64,
            PortraitSlot::DdsArgb { info, .. } => info.width as u64 * info.height as u64,
            PortraitSlot::Rw4Xb2 { lib, idx, .. } => {
                let t = &lib.textures[*idx];
                t.width as u64 * t.height as u64
            }
            PortraitSlot::Rw4Ps3 { lib, idx, .. } => {
                let t = &lib.textures[*idx];
                t.width as u64 * t.height as u64
            }
        }
    }
}

/// Write an edited texture back into an extracted **portrait** archive (`p{id}.big` — the extracted
/// file itself, a `BIGF` holding a RefPack-wrapped texture). Picks the largest decodable slot and
/// re-encodes `rgba` (w×h RGBA8) into the slot's DXT format, splicing it back through the platform's
/// container → RefPack → BIGF and returning the rebuilt archive bytes ready to write over the file.
///
/// Two platform encodings, both mirrored from the prior app's proven `portrait_replace` pipeline
/// (verified there against real EA bytes):
/// * **Xbox 360 `XPR2`** — same dimensions keep the descriptor via
///   [`Xpr2Texture::replace_base_mip_linear`]; a size change regenerates it via
///   [`Xpr2Texture::replace_resized`].
/// * **PS3 `DDS `** — the shipped header is kept apart from the size words and the level
///   structure is preserved: a full chain regenerated for the new size where the source ships one
///   (portraits), the base level alone where it does not (the NHL 12–Legacy `leaguelogossmall`
///   DXT5 logos).
///
/// DXT slots require 4-aligned dimensions (4×4 block compression).
pub fn replace_portrait_texture(
    archive_bytes: Vec<u8>,
    rgba: &[u8],
    w: u32,
    h: u32,
) -> Result<Vec<u8>> {
    replace_portrait_texture_noted(archive_bytes, rgba, w, h).map(|(b, _)| b)
}

/// Rewrite the package's primary texture slot for `rgba`: returns the member name and its new
/// stored bytes (wrapped as the member was), without placing it back into the package.
fn rewrite_package_texture(
    archive: &tdb_archive_big::Archive,
    rgba: &[u8],
    w: u32,
    h: u32,
) -> Result<(String, Vec<u8>)> {
    use tdb_rx2::xpr::{Xpr2Texture, XPR2_MAGIC};

    // Largest decodable slot (RefPack-unwrapped), either encoding — matches the
    // read-side picker in `decode_portrait`/`rasters_in`.
    let mut best: Option<PortraitSlot> = None;
    for entry in archive.iter_entries() {
        if entry.name.is_empty() {
            continue;
        }
        let Ok(raw) = archive.read_entry_raw(&entry.name) else {
            continue;
        };
        let wrapped = tdb_archive_codec::is_refpack(&raw);
        let decoded = if wrapped {
            match tdb_archive_codec::refpack_decompress(&raw) {
                Ok(d) => d,
                Err(_) => continue,
            }
        } else {
            raw
        };
        let candidate = if decoded.len() >= 4 && &decoded[..4] == XPR2_MAGIC {
            Xpr2Texture::parse(decoded)
                .ok()
                .map(|xpr| PortraitSlot::Xpr2 {
                    entry: entry.name.clone(),
                    xpr,
                })
        } else if let Some((fmt, dw, dh, _)) = tdb_archive_codec::dds_base_mip(&decoded) {
            Some(PortraitSlot::Dds {
                entry: entry.name.clone(),
                width: dw,
                height: dh,
                fmt,
                header: decoded[..128].to_vec(),
                mips: u32::from_le_bytes(decoded[28..32].try_into().unwrap()),
            })
        } else if let Some(info) = dds_uncompressed_32(&decoded) {
            Some(PortraitSlot::DdsArgb {
                entry: entry.name.clone(),
                header: decoded[..128].to_vec(),
                info,
            })
        } else if decoded.starts_with(tdb_rx2::texlib_ps3::MAGIC) {
            tdb_rx2::texlib_ps3::parse(&decoded).ok().and_then(|lib| {
                let idx = primary_ps3_raster_index(&lib)?;
                Some(PortraitSlot::Rw4Ps3 {
                    entry: entry.name.clone(),
                    wrapped,
                    lib: Box::new(lib),
                    idx,
                })
            })
        } else if decoded.starts_with(tdb_rx2::texlib::MAGIC) {
            tdb_rx2::texlib::parse(&decoded).ok().and_then(|lib| {
                let idx = primary_raster_index(&lib.textures)?;
                Some(PortraitSlot::Rw4Xb2 {
                    entry: entry.name.clone(),
                    wrapped,
                    lib: Box::new(lib),
                    idx,
                })
            })
        } else {
            None
        };
        if let Some(candidate) = candidate {
            if best.as_ref().is_none_or(|b| candidate.area() > b.area()) {
                best = Some(candidate);
            }
        }
    }
    let slot = best.ok_or_else(|| {
        DomainError::Extract("no XPR2 or DDS texture in portrait archive to replace".into())
    })?;

    // The DXT encodings need 4×4 blocks; an uncompressed 8_8_8_8 XPR2 slot (the FE league logos,
    // Phase 1.4) has no such constraint.
    let is_dxt = match &slot {
        PortraitSlot::Xpr2 { xpr, .. } => xpr.format.as_dxt().is_some(),
        PortraitSlot::Dds { .. } => true,
        PortraitSlot::DdsArgb { .. } => false,
        PortraitSlot::Rw4Xb2 { lib, idx, .. } => lib.textures[*idx].format.is_dxt(),
        PortraitSlot::Rw4Ps3 { lib, idx, .. } => {
            lib.textures[*idx].format != tdb_rx2::texlib_ps3::Ps3Format::Argb8
        }
    };
    if is_dxt && (!w.is_multiple_of(4) || !h.is_multiple_of(4)) {
        return Err(DomainError::Extract(format!(
            "portrait is DXT (4×4 blocks); replacement {w}×{h} must have both dims divisible by 4"
        )));
    }

    let (entry_name, packed) = match slot {
        PortraitSlot::Xpr2 { entry, xpr } => {
            let linear = match xpr.format.as_dxt() {
                Some(dxt) => {
                    let dds_fmt = dds_format_of(dxt)?;
                    tdb_archive_codec::compress_bc(dds_fmt, w, h, rgba)
                }
                // Uncompressed 8_8_8_8: the codec's linear stream is row-major big-endian
                // `[A, R, G, B]` texels (verified against the PS3 DDS ground truth in `tdb-rx2`).
                None => {
                    if rgba.len() != (w as usize) * (h as usize) * 4 {
                        return Err(DomainError::Extract(format!(
                            "replacement pixel buffer is {} bytes, not {w}×{h}×4",
                            rgba.len()
                        )));
                    }
                    rgba.chunks_exact(4)
                        .flat_map(|p| [p[3], p[0], p[1], p[2]])
                        .collect()
                }
            };
            let new_xpr = if w == xpr.width && h == xpr.height {
                xpr.replace_base_mip_linear(&linear)
            } else {
                xpr.replace_resized(w, h, &linear)
            }
            .map_err(|e| DomainError::Extract(format!("XPR2 rewrite: {e}")))?;
            (entry, tdb_archive_codec::refpack_compress(&new_xpr))
        }
        // PS3: RefPack-wrapped little-endian DDS. The shipped header is kept verbatim apart
        // from height @12, width @16, linear size @20 (DDSD_LINEARSIZE is set on every measured
        // DXT member) and the mip count @28; the shipped level structure is preserved: base only
        // when the source declares no chain (the NHL 12–Legacy `leaguelogossmall` logos, measured
        // `mips == 0`, body = one level), a full chain for the new size when the source shipped a
        // full chain (portraits), else the declared count clipped to the new size. P1 audit
        // (21 Sep 2026): the earlier writer always emitted a full chain with the mip flags set,
        // a shape those logos do not ship.
        PortraitSlot::Dds { entry, fmt, header, mips, width: ow, height: oh } => {
            let levels = if mips <= 1 {
                1
            } else if mips == full_mip_count(ow, oh) {
                full_mip_count(w, h)
            } else {
                mips.min(full_mip_count(w, h))
            };
            let all = compress_bc_mip_levels(fmt, w, h, rgba);
            let mut dds = header;
            dds[12..16].copy_from_slice(&h.to_le_bytes());
            dds[16..20].copy_from_slice(&w.to_le_bytes());
            let flags = u32::from_le_bytes(dds[8..12].try_into().unwrap());
            if flags & 0x0008_0000 != 0 {
                dds[20..24].copy_from_slice(&fmt.linear_size(w, h).to_le_bytes());
            }
            if mips > 1 || flags & 0x0002_0000 != 0 {
                dds[28..32].copy_from_slice(&levels.to_le_bytes());
            }
            for (_, _, data) in all.iter().take(levels as usize) {
                dds.extend_from_slice(data);
            }
            (entry, tdb_archive_codec::refpack_compress(&dds))
        }
        // PS3 NHL 12–Legacy logos: uncompressed DDS, same dimensions (the shipped header is
        // reused verbatim, so the dimensions it declares must hold), texels in its channel order,
        // mip levels regenerated when the header declares them.
        PortraitSlot::DdsArgb {
            entry,
            header,
            info,
        } => {
            // A dimension change patches the standard DDS header words (height @12, width @16,
            // pitch @20 = width × 4, mip count @28 kept at the shipped count when it fits the new
            // size, else the full chain) and keeps every other header byte (flags, pixel format
            // masks, caps) as shipped.
            let header = if (w, h) != (info.width, info.height) {
                if w == 0 || h == 0 {
                    return Err(DomainError::Extract("replacement dimensions must be non-zero".into()));
                }
                let mut hdr = header;
                let mips = if info.mips <= full_mip_count(w, h) { info.mips } else { full_mip_count(w, h) };
                hdr[12..16].copy_from_slice(&h.to_le_bytes());
                hdr[16..20].copy_from_slice(&w.to_le_bytes());
                hdr[20..24].copy_from_slice(&(w * 4).to_le_bytes());
                hdr[28..32].copy_from_slice(&mips.to_le_bytes());
                hdr
            } else {
                header
            };
            let info = DdsArgb { width: w, height: h, mips: u32::from_le_bytes(header[28..32].try_into().unwrap()), order: info.order };
            if rgba.len() != (w as usize) * (h as usize) * 4 {
                return Err(DomainError::Extract(format!(
                    "replacement pixel buffer is {} bytes, not {w}×{h}×4",
                    rgba.len()
                )));
            }
            let Some(base) = image::RgbaImage::from_raw(w, h, rgba.to_vec()) else {
                return Err(DomainError::Extract("replacement pixel buffer is malformed".into()));
            };
            let mut dds = header;
            for level in 0..info.mips.max(1) {
                let (mw, mh) = ((w >> level).max(1), (h >> level).max(1));
                let img = if level == 0 {
                    base.clone()
                } else {
                    image::imageops::resize(&base, mw, mh, image::imageops::FilterType::Triangle)
                };
                for p in img.as_raw().chunks_exact(4) {
                    let mut t = [0u8; 4];
                    for (c, byte) in info.order.iter().enumerate() {
                        t[*byte] = p[c];
                    }
                    dds.extend_from_slice(&t);
                }
            }
            (entry, tdb_archive_codec::refpack_compress(&dds))
        }
        // Xbox 360 NHL 08–11: the RW4xb2 texlib, same dimensions only (the Xenos descriptor
        // is not regenerated here); the member keeps its own wrapping.
        PortraitSlot::Rw4Xb2 {
            entry,
            wrapped,
            lib,
            idx,
        } => {
            // Receipt 37 (research R132): a different size goes through the texlib writer's own
            // rules (ARGB8 single-level tiled art: the measured descriptor rule; DXT: the existing
            // single-level policy) and the caller rewrites the Apt geometry; the member keeps its
            // own wrapping.
            let _ = &lib.textures[idx];
            let new_lib = replace_texlib_texture(lib.original_bytes(), rgba, w, h, Some(idx))?;
            (entry, wrap_member(new_lib, wrapped))
        }
        // PS3 NHL 08–11: the RW4ps3 texlib, plain slot re-encoded at the same length.
        PortraitSlot::Rw4Ps3 {
            entry,
            wrapped,
            lib,
            idx,
        } => {
            // Receipt 37 (research R132c: the PS3 NHL 08–11 packages carry the same Apt layout):
            // the loose RW4ps3 writer, same size or resized (words 12 / 13 + the linear slot).
            let new_lib = replace_texlib_texture(lib.original_bytes(), rgba, w, h, Some(idx))?;
            (entry, wrap_member(new_lib, wrapped))
        }
    };
    Ok((entry_name, packed))
}

/// [`replace_portrait_texture`] that also returns the writer's note when the package had to be
/// rebuilt through the generic builder instead of the span-preserving re-plan.
///
/// **Dimension change** (C6, overnight 2026-09-20): when `w × h` differs from the slot's, the
/// package must be an Apt logo/imagery package whose geometry Studio can rewrite — a `<n+1>`
/// 288-byte record whose six quad vertices span exactly the texture's own size, and a member `0`
/// (`Apt Data`) whose size words agree (measured on all 141 fixture packages of every gen7 family:
/// the record's vertices at +96 with a 32-byte stride are `(0,H) (0,0) (W,0) (W,0) (W,H) (0,H)`,
/// member 0 carries `W, H` as integers and as floats at fixed offsets from its object table). Both are rewritten
/// to the new size, so the logo is displayed at its new size, 1:1. A package whose quad is not the
/// texture's size (trophies: a 225×375 quad on a 256×512 texture — a display crop whose UV rule
/// is not established) refuses a dimension change; so does a texture container that cannot be
/// resized (RW4 texlibs, whose GPU descriptor is not regenerated). Runtime acceptance of a
/// resized quad is not established (card).
pub fn replace_portrait_texture_noted(
    archive_bytes: Vec<u8>,
    rgba: &[u8],
    w: u32,
    h: u32,
) -> Result<(Vec<u8>, Option<String>)> {
    let archive = tdb_archive_big::Archive::parse(archive_bytes)
        .map_err(|e| DomainError::Extract(format!("portrait archive parse: {e}")))?;
    let source_dims = package_texture_dims(&archive);
    let (entry_name, packed) = rewrite_package_texture(&archive, rgba, w, h)?;
    let (bytes, note) = place_package_member(archive, &entry_name, packed)?;
    match source_dims {
        Some((ow, oh)) if (ow, oh) != (w, h) => {
            if note.is_some() {
                return Err(DomainError::Extract("a dimension change needs the span-preserving package layout; this package was rebuilt through the generic builder".into()));
            }
            let rewritten = rewrite_apt_geometry(&bytes, &entry_name, ow, oh, w, h)?;
            Ok((rewritten, None))
        }
        _ => Ok((bytes, note)),
    }
}

/// The primary texture's dimensions in the package (the slot [`rewrite_package_texture`] writes).
fn package_texture_dims(archive: &tdb_archive_big::Archive) -> Option<(u32, u32)> {
    let mut best: Option<(u64, u32, u32)> = None;
    for entry in archive.iter_entries() {
        let Ok(raw) = archive.read_entry_raw(&entry.name) else { continue };
        let decoded = if tdb_archive_codec::is_refpack(&raw) { tdb_archive_codec::refpack_decompress(&raw).ok()? } else { raw };
        let dims = if decoded.len() >= 4 && &decoded[..4] == tdb_rx2::xpr::XPR2_MAGIC {
            tdb_rx2::xpr::Xpr2Texture::parse(decoded).ok().map(|x| (x.width, x.height))
        } else if let Some((_, dw, dh, _)) = tdb_archive_codec::dds_base_mip(&decoded) {
            Some((dw, dh))
        } else if let Some(i) = dds_uncompressed_32(&decoded) {
            Some((i.width, i.height))
        } else if decoded.starts_with(tdb_rx2::texlib_ps3::MAGIC) {
            tdb_rx2::texlib_ps3::parse(&decoded).ok().and_then(|lib| primary_ps3_raster_index(&lib).map(|i| (lib.textures[i].width as u32, lib.textures[i].height as u32)))
        } else if decoded.starts_with(tdb_rx2::texlib::MAGIC) {
            tdb_rx2::texlib::parse(&decoded).ok().and_then(|lib| primary_raster_index(&lib.textures).map(|i| (lib.textures[i].width, lib.textures[i].height)))
        } else {
            None
        };
        if let Some((dw, dh)) = dims {
            let area = dw as u64 * dh as u64;
            if best.is_none_or(|b| area > b.0) {
                best = Some((area, dw, dh));
            }
        }
    }
    best.map(|b| (b.1, b.2))
}

/// Rewrite the Apt geometry of a logo/imagery package for a texture whose size changed from
/// `old` to `new` (see [`replace_portrait_texture_noted`]). Refuses when the measured layout is
/// not found exactly (the quad must span the old texture size; member 0's size words must agree).
fn rewrite_apt_geometry(package: &[u8], texture_member: &str, ow: u32, oh: u32, nw: u32, nh: u32) -> Result<Vec<u8>> {
    use tdb_archive_big::replan::{replan_member, PackageLayout};
    let layout = PackageLayout::parse(package).map_err(|e| DomainError::Extract(format!("package layout: {e}")))?;
    let tex = layout.members.iter().find(|m| m.name == texture_member).ok_or_else(|| DomainError::Extract("texture member vanished".into()))?;
    // `<n+1>`: the 288-byte record named by the texture member's number + 1.
    let n: u32 = texture_member.parse().map_err(|_| DomainError::Extract(format!("a dimension change needs the numbered Apt package layout; the texture member is `{texture_member}`")))?;
    let rec_name = (n + 1).to_string();
    let rec = layout.members.iter().find(|m| m.name == rec_name && m.size == 288).ok_or_else(|| DomainError::Extract(format!("a dimension change needs the 288-byte Apt geometry record `{rec_name}` beside the texture; not found")))?;
    let m0 = layout.members.iter().find(|m| m.name == "0").ok_or_else(|| DomainError::Extract("a dimension change needs the `0` Apt Data member; not found".into()))?;
    let _ = tex;
    let (ow_f, oh_f) = (ow as f32, oh as f32);
    let body = &package[rec.offset as usize..(rec.offset + rec.size) as usize];
    let f32_at = |b: &[u8], at: usize| f32::from_be_bytes(b[at..at + 4].try_into().unwrap());
    // Six vertices at +96, stride 32: (0,H) (0,0) (W,0) (W,0) (W,H) (0,H); everything else zero.
    let expected: [(f32, f32); 6] = [(0.0, oh_f), (0.0, 0.0), (ow_f, 0.0), (ow_f, 0.0), (ow_f, oh_f), (0.0, oh_f)];
    for (i, (ex, ey)) in expected.iter().enumerate() {
        let at = 96 + i * 32;
        if f32_at(body, at) != *ex || f32_at(body, at + 4) != *ey {
            return Err(DomainError::Extract(format!(
                "a dimension change is refused: the Apt quad of this package is not the texture's own size ({ow}×{oh}); vertex {i} reads ({}, {}) — a display crop whose texture-coordinate rule is not established",
                f32_at(body, at),
                f32_at(body, at + 4)
            )));
        }
    }
    // Member 0 (`Apt Data:1:5:4 `, 16-byte tag): the word at +16 is the offset of the object
    // table (28 / 44 / 52 / 228 on the measured 240 / 256 / 264 / 440-byte variants); the size
    // sits at table + 20 (two integers) and table + 96 (two floats) on all 141 fixture packages.
    // NHL 11 stores member `0` RefPack-wrapped on 172 packages (research R132 §2): unwrap to check
    // and rewrite, re-wrap to place.
    let m0_stored = &package[m0.offset as usize..(m0.offset + m0.size) as usize];
    let m0_wrapped = tdb_archive_codec::is_refpack(m0_stored);
    let m0_plain = if m0_wrapped { tdb_archive_codec::refpack_decompress(m0_stored).map_err(|e| DomainError::Extract(format!("Apt Data member RefPack: {e}")))? } else { m0_stored.to_vec() };
    let m0b = m0_plain.as_slice();
    let u32_at = |b: &[u8], at: usize| u32::from_be_bytes(b[at..at + 4].try_into().unwrap());
    if m0b.len() < 24 || !m0b.starts_with(b"Apt Data:") {
        return Err(DomainError::Extract("a dimension change is refused: member `0` is not an `Apt Data` object".into()));
    }
    let table = 16 + u32_at(m0b, 16) as usize;
    let (int_at, float_at) = (table + 20, table + 96);
    if m0b.len() < float_at + 8 || u32_at(m0b, int_at) != ow || u32_at(m0b, int_at + 4) != oh || f32_at(m0b, float_at) != ow_f || f32_at(m0b, float_at + 4) != oh_f {
        return Err(DomainError::Extract(format!("a dimension change is refused: the `0` Apt Data member does not carry the texture size ({ow}x{oh}) at the measured positions (table +20 integers, +96 floats)")));
    }
    // Rewrite both members through the re-plan (same lengths: only the bytes change).
    let (nw_f, nh_f) = (nw as f32, nh as f32);
    let mut new_rec = body.to_vec();
    let verts: [(f32, f32); 6] = [(0.0, nh_f), (0.0, 0.0), (nw_f, 0.0), (nw_f, 0.0), (nw_f, nh_f), (0.0, nh_f)];
    for (i, (x, y)) in verts.iter().enumerate() {
        let at = 96 + i * 32;
        new_rec[at..at + 4].copy_from_slice(&x.to_be_bytes());
        new_rec[at + 4..at + 8].copy_from_slice(&y.to_be_bytes());
    }
    let mut new_m0 = m0b.to_vec();
    new_m0[int_at..int_at + 4].copy_from_slice(&nw.to_be_bytes());
    new_m0[int_at + 4..int_at + 8].copy_from_slice(&nh.to_be_bytes());
    new_m0[float_at..float_at + 4].copy_from_slice(&nw_f.to_be_bytes());
    new_m0[float_at + 4..float_at + 8].copy_from_slice(&nh_f.to_be_bytes());
    let new_m0 = wrap_member(new_m0, m0_wrapped);
    // A member whose rewrite keeps its stored length is patched **in place** — nothing moves, so no
    // placement rule is involved (the NHL 07 → 10 Xbox packages page-align their members, a
    // placement the re-plan does not measure; receipt 37). Only a length change (NHL 11's
    // re-wrapped member `0`) goes through the span-preserving re-plan.
    let place = |pkg: &[u8], name: &str, member: &tdb_archive_big::replan::Member, bytes: &[u8], what: &str| -> Result<Vec<u8>> {
        if bytes.len() == member.size as usize {
            let mut out = pkg.to_vec();
            out[member.offset as usize..(member.offset + member.size) as usize].copy_from_slice(bytes);
            Ok(out)
        } else {
            replan_member(pkg, name, bytes).map(|r| r.bytes).map_err(|e| DomainError::Extract(format!("{what}: {e}")))
        }
    };
    let step1 = place(package, &rec_name, rec, &new_rec, "Apt record rewrite")?;
    let layout1 = PackageLayout::parse(&step1).map_err(|e| DomainError::Extract(format!("package layout: {e}")))?;
    let m0_now = layout1.members.iter().find(|m| m.name == "0").ok_or_else(|| DomainError::Extract("the `0` Apt Data member vanished".into()))?;
    place(&step1, "0", m0_now, &new_m0, "Apt Data rewrite")
}

/// Put the rewritten member back into its package. The **span-preserving re-plan**
/// (`tdb_archive_big::replan`, research 02 §4.2) is the writer: every other byte of the package
/// — header, table trailer, the other members, their alignment gaps, the opaque 16-byte tail —
/// stays as shipped and only the member, its size word, the followers' offsets and `size_le`
/// change (an identical member reproduces the package byte for byte; proven on all 77 fixture
/// packages). The generic [`tdb_archive_big::ArchiveBuilder`] rebuild — no alignment, no trailer,
/// a different `archive_size` rule — is kept only for a package whose *shape* the re-plan does not
/// recognise (an entry-pad variant, an unexplained `size_le`, an unmeasured follower residual);
/// that fallback is the prior app's pipeline and is reported as the write's note so it is never
/// silently a rebuilt layout. A re-plan refusal about the *content* (a non-zero gap that a
/// length change would erase) is an error, not a fallback.
fn place_package_member(
    archive: tdb_archive_big::Archive,
    entry_name: &str,
    packed: Vec<u8>,
) -> Result<(Vec<u8>, Option<String>)> {
    use tdb_archive_big::replan::{replan_member, ReplanError};
    let source = archive.bytes();
    match replan_member(source, entry_name, &packed) {
        Ok(r) => Ok((r.bytes, None)),
        Err(e @ (ReplanError::NonZeroGap { .. } | ReplanError::TooLarge(_) | ReplanError::Verification(_))) => {
            Err(DomainError::Extract(format!("package re-plan for `{entry_name}`: {e}")))
        }
        Err(shape) => {
            let note = format!(
                "package shape not recognised by the span-preserving re-plan ({shape}); the package was rebuilt through the generic builder (members packed without alignment, no table trailer, builder archive_size rule) — a layout no shipped package exhibits"
            );
            let mut builder = archive
                .to_builder()
                .map_err(|e| DomainError::Extract(format!("archive to_builder: {e}")))?;
            if !builder.replace_entry(entry_name, packed) {
                return Err(DomainError::Extract(format!(
                    "entry {entry_name} vanished before replace"
                )));
            }
            let bytes = builder
                .build()
                .map_err(|e| DomainError::Extract(format!("archive rebuild: {e}")))?;
            Ok((bytes, Some(note)))
        }
    }
}

/// Re-wrap a rebuilt member the way the package stored it: RefPack'd or raw.
fn wrap_member(bytes: Vec<u8>, wrapped: bool) -> Vec<u8> {
    if wrapped {
        tdb_archive_codec::refpack_compress(&bytes)
    } else {
        bytes
    }
}

/// Every raster one archive entry carries.
///
/// EA shipped portraits in **three different containers**, verified against real
/// installs:
///
/// * NHL Legacy / 14 (Xbox 360) — a RefPack-compressed, Xbox-tiled `XPR2` package.
/// * **NHL Legacy / 15 (PS3)** — a RefPack-compressed, little-endian `DDS `
///   (DXT1/3/5). Untiled, so the top-mip block stream is a linear base mip.
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
    // PS3 DDS: `DDS ` magic + a DXT fourcc. The base-mip block slice is already
    // untiled/linear, so it drops straight into a `Raster` (the same shape the
    // XPR2 branch produces after un-tiling). Uncompressed / DX10 DDS return
    // `None` here and fall through (unobserved for portraits).
    if let Some((fmt, w, h, base)) = tdb_archive_codec::dds_base_mip(&payload) {
        if let Some(format) = dds_to_texlib_format(fmt) {
            return vec![Raster {
                width: w,
                height: h,
                linear: base.to_vec(),
                format,
            }];
        }
    }
    // PS3 NHL 12–Legacy league logos: an uncompressed A8R8G8B8 DDS. Re-order each texel into
    // the shared `[A,R,G,B]` linear layout.
    if let Some(d) = dds_uncompressed_32(&payload) {
        let n = (d.width as usize) * (d.height as usize);
        let linear: Vec<u8> = payload[128..128 + n * 4]
            .chunks_exact(4)
            .flat_map(|p| [p[d.order[3]], p[d.order[0]], p[d.order[1]], p[d.order[2]]])
            .collect();
        return vec![Raster {
            width: d.width,
            height: d.height,
            linear,
            format: tdb_rx2::texlib::PixelFormat::Argb8888,
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
    // **PS3 NHL 08–11** — a `\x89RW4ps3` texlib (raw on 08, RefPack-wrapped on
    // 09–11 inside the same BIGF). Its slots are already plain (research 18), so
    // the base level drops straight into a `Raster` under the shared format tags.
    if let Ok(lib) = tdb_rx2::texlib_ps3::parse(&payload) {
        return (0..lib.textures.len())
            .filter_map(|i| {
                let t = &lib.textures[i];
                Some(Raster {
                    width: t.width,
                    height: t.height,
                    format: ps3_format_to_texlib(t.format),
                    linear: lib.base_mip_linear(i)?.to_vec(),
                })
            })
            .collect();
    }
    Vec::new()
}

/// The shared raster format tag of a PS3 texlib format: the `[A,R,G,B]` texel order and the
/// LE DXT5 block layout are the same bytes `linear_to_rgba` already decodes for Xbox slots.
fn ps3_format_to_texlib(f: tdb_rx2::texlib_ps3::Ps3Format) -> tdb_rx2::texlib::PixelFormat {
    match f {
        tdb_rx2::texlib_ps3::Ps3Format::Argb8 => tdb_rx2::texlib::PixelFormat::Argb8888,
        tdb_rx2::texlib_ps3::Ps3Format::Dxt5 => tdb_rx2::texlib::PixelFormat::Dxt5,
        tdb_rx2::texlib_ps3::Ps3Format::Dxt1 => tdb_rx2::texlib::PixelFormat::Dxt1,
        tdb_rx2::texlib_ps3::Ps3Format::Dxt3 => tdb_rx2::texlib::PixelFormat::Dxt3,
    }
}

/// Encode `rgba` (w×h) as the whole plain slot of PS3 texture `idx`: ARGB8 texels for a
/// one-level slot, or a DXT5 chain regenerated level by level (box-filtered, 16-byte floor)
/// so the slot length is preserved exactly.
fn encode_ps3_slot(
    lib: &tdb_rx2::texlib_ps3::TexlibPs3,
    idx: usize,
    rgba: &[u8],
    w: u32,
    h: u32,
) -> Result<Vec<u8>> {
    use tdb_rx2::texlib_ps3::Ps3Format;
    let t = &lib.textures[idx];
    // P2-B (research 2026-09-21 handoff 11 §1): a different size is a plain resize on PS3 — the
    // chain is re-emitted at the new dimensions with the slot's own level count (capped where
    // the new size runs out of halvings) and the caller rewrites descriptor words 12 / 13.
    let mips = if (w, h) == (t.width, t.height) { t.mips } else { ps3_resized_mip_count(t.mips, w, h) };
    if rgba.len() != (w as usize) * (h as usize) * 4 {
        return Err(DomainError::Extract(format!(
            "replacement pixel buffer is {} bytes, not {w}×{h}×4",
            rgba.len()
        )));
    }
    let sizes = tdb_rx2::texlib_ps3::mip_chain_sizes(t.format, w, h, mips);
    let mut chain = Vec::with_capacity(sizes.iter().sum());
    match t.format {
        Ps3Format::Argb8 => {
            let Some(base) = image::RgbaImage::from_raw(w, h, rgba.to_vec()) else {
                return Err(DomainError::Extract("replacement pixel buffer is malformed".into()));
            };
            for level in 0..sizes.len() as u32 {
                let (mw, mh) = ((w >> level).max(1), (h >> level).max(1));
                let img = if level == 0 {
                    base.clone()
                } else {
                    image::imageops::resize(&base, mw, mh, image::imageops::FilterType::Triangle)
                };
                chain.extend(img.as_raw().chunks_exact(4).flat_map(|p| [p[3], p[0], p[1], p[2]]));
            }
        }
        Ps3Format::Dxt5 | Ps3Format::Dxt1 | Ps3Format::Dxt3 => {
            if !w.is_multiple_of(4) || !h.is_multiple_of(4) {
                return Err(DomainError::Extract(format!(
                    "slot is {} (4×4 blocks); replacement {w}×{h} must have both dims divisible by 4",
                    t.format.label()
                )));
            }
            let dds = match t.format {
                Ps3Format::Dxt1 => tdb_archive_codec::DdsFormat::Dxt1,
                Ps3Format::Dxt3 => tdb_archive_codec::DdsFormat::Dxt3,
                _ => tdb_archive_codec::DdsFormat::Dxt5,
            };
            let levels = compress_bc_mip_levels(dds, w, h, rgba);
            for (i, size) in sizes.iter().enumerate() {
                let mut data = levels.get(i).map(|(_, _, d)| d.clone()).unwrap_or_default();
                // The 16-byte floor: a level smaller than one block still occupies one block.
                data.resize(*size, 0);
                chain.extend_from_slice(&data);
            }
        }
    }
    Ok(chain)
}

/// The level count of a resized PS3 raster: the slot's own count, capped at the number of
/// halvings the new size affords (down to 1 × 1).
fn ps3_resized_mip_count(original: u8, w: u32, h: u32) -> u8 {
    let max = full_mip_count(w, h).max(1) as u8;
    original.max(1).min(max)
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

/// Decode a de-tiled PS4 GNF texture to tightly packed RGBA8. BC1/BC2/BC3 reuse
/// the shared block decoder via [`linear_to_rgba`]; BC4/BC5 decode through the
/// dedicated single-/dual-channel paths in [`tdb_rx2::gnf`].
pub(crate) fn gnf_to_rgba(tex: &tdb_rx2::gnf::GnfTexture) -> Result<Vec<u8>> {
    use tdb_rx2::gnf::BcFormat;
    use tdb_rx2::texlib::PixelFormat;
    let (w, h) = (tex.width, tex.height);
    match tex.format {
        BcFormat::Bc1 => linear_to_rgba(PixelFormat::Dxt1, w, h, &tex.base_mip_linear),
        BcFormat::Bc2 => linear_to_rgba(PixelFormat::Dxt3, w, h, &tex.base_mip_linear),
        BcFormat::Bc3 => linear_to_rgba(PixelFormat::Dxt5, w, h, &tex.base_mip_linear),
        // BC7 (NHL 16+ player portraits) decodes cleanly through the pure-Rust
        // block decoder — verified against a real portrait (seamless, no drift).
        BcFormat::Bc7 => Ok(tdb_rx2::gnf::decode_bc7(&tex.base_mip_linear, w, h)),
        // BC4/BC5 (single/dual-channel — roughness / normal maps). The block decode
        // is correct; on large 128-bpp surfaces the GNM de-tile still leaves some
        // artefacts (a known limitation), but the maps are exported rather than
        // dropped so a bulk "all textures" export is complete.
        BcFormat::Bc4 => Ok(tdb_rx2::gnf::decode_bc4(&tex.base_mip_linear, w, h)),
        BcFormat::Bc5 => Ok(tdb_rx2::gnf::decode_bc5(&tex.base_mip_linear, w, h)),
    }
}

/// How a PS4 GNF replacement will be written, decided from the file's own bytes **before anything
/// is encoded**, and shared by [`replace_gnf_texture_noted`] and [`plan_texture_replace`] so the
/// Replace… dialog shows exactly the refusal or disclosure the write returns.
#[derive(Debug, Clone, PartialEq, Eq)]
enum GnfWritePolicy {
    /// Same size, rule-conformant chain of `levels` (> 1): every level re-encoded from the
    /// replacement, header kept verbatim.
    Chain { levels: u32 },
    /// Same size, base level only. `note` is `None` when the file has a single level (the base is
    /// the whole image) and carries the disclosure when lower levels exist but their layout is not
    /// the measured rule, so they keep the old image.
    BaseOnly { note: Option<String> },
    /// Changed size: the measured resize rule re-emits a chain of `levels`.
    Resize { levels: u32 },
}

/// The [`GnfWritePolicy`] for a `w`×`h` replacement of `gnf_bytes`; `Ok(Err(reason))` is a refusal.
fn gnf_write_policy(gnf_bytes: &[u8], w: u32, h: u32) -> Result<std::result::Result<GnfWritePolicy, String>> {
    let tex = tdb_rx2::gnf::parse(gnf_bytes).map_err(|e| DomainError::Extract(format!("GNF parse: {e}")))?;
    let rule = tdb_rx2::gnf::check_resize_rule(gnf_bytes);
    if (w, h) == (tex.width, tex.height) {
        return Ok(Ok(match rule {
            Ok((_, _, _, levels)) if levels > 1 => GnfWritePolicy::Chain { levels },
            Ok(_) => GnfWritePolicy::BaseOnly { note: None },
            Err(why) => match tdb_rx2::gnf::declared_levels(gnf_bytes).unwrap_or(1) {
                1 => GnfWritePolicy::BaseOnly { note: None },
                n => GnfWritePolicy::BaseOnly {
                    note: Some(format!(
                        "Only the full-size image was replaced. This texture's {} smaller mip level(s) keep the original image, because its layout is not one Studio has measured ({why}); the old image may show when the game draws it at a distance.",
                        n - 1
                    )),
                },
            },
        }));
    }
    // P2-B: the PS4 resize of research 2026-09-21 handoff 15 §3 — the shipped header must first
    // pass the "descriptor reproducible" gate (cubes, BC6H, volumes and unmeasured layouts refuse).
    // Runtime acceptance of a changed size is card R31-GNF-1; the registry's size directory rule
    // is the outer gate.
    let orig_levels = match rule {
        Ok((_, _, _, n)) => n,
        Err(why) => return Ok(Err(format!("GNF resize refused: {why}"))),
    };
    // The codec is the authority on representability (`replace_resized` refuses what its header
    // cannot store — coordinator review R1 F1); this early gate only spares encoding a chain that
    // will be refused.
    if w == 0 || h == 0 || w > tdb_rx2::gnf::GNF_MAX_DIM || h > tdb_rx2::gnf::GNF_MAX_DIM {
        return Ok(Err(format!(
            "GNF resize refused: {w}x{h} cannot be stored in the GNF header (1–{} texels on each axis)",
            tdb_rx2::gnf::GNF_MAX_DIM
        )));
    }
    // The file's own level count, capped at the halvings the new size affords.
    Ok(Ok(GnfWritePolicy::Resize { levels: orig_levels.min(full_mip_count(w, h)).max(1) }))
}

/// Write an edited texture back into an extracted **PS4 GNF** (`.gnf` — the
/// extracted Kettle asset itself). Re-encodes `rgba` (w×h RGBA8) into the GNF's
/// existing BCn format and re-tiles it into the GNM `tiling_index = 13` layout.
/// Disclosures are returned by [`replace_gnf_texture_noted`]; this wrapper drops them.
///
/// Every GNF BCn format is writable: BC1/BC2/BC3 via
/// [`tdb_archive_codec::compress_bc`] (texpresso), **BC7** via [`tdb_rx2::bc7`]
/// (mode-6 — NHL 16+ portraits), and **BC4/BC5** via [`tdb_rx2::bc45`]
/// (single/dual-channel roughness / normal maps). Unlike DXT-in-a-.big, GNF
/// carries partial edge blocks, so non-multiple-of-4 dimensions (e.g. the
/// 512×340 portraits) are fine.
pub fn replace_gnf_texture(gnf_bytes: &[u8], rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
    replace_gnf_texture_noted(gnf_bytes, rgba, w, h).map(|(bytes, _)| bytes)
}

/// [`replace_gnf_texture`] with its disclosures. What is written follows [`gnf_write_policy`]:
///
/// * **same size, measured chain** — every level re-encoded from the replacement (box-filtered
///   downsamples) via [`tdb_rx2::gnf::replace_chain_same_size`]; the 256-byte header is kept
///   verbatim. Rewriting only the base left the old image in the lower levels, visible at distance.
/// * **same size, single level** — the base is the whole image ([`tdb_rx2::gnf::replace_base_mip`]).
/// * **same size, unmeasured layout with lower levels** (multi-stream files, cubes, …) — the base
///   only, with a disclosure that the lower levels keep the original image.
/// * **changed size** — the measured resize rule ([`tdb_rx2::gnf::replace_resized`]).
pub fn replace_gnf_texture_noted(gnf_bytes: &[u8], rgba: &[u8], w: u32, h: u32) -> Result<(Vec<u8>, Vec<String>)> {
    use tdb_archive_codec::DdsFormat;
    use tdb_rx2::gnf::BcFormat;

    let policy = gnf_write_policy(gnf_bytes, w, h)?.map_err(DomainError::Extract)?;
    let format = tdb_rx2::gnf::parse(gnf_bytes).map_err(|e| DomainError::Extract(format!("GNF parse: {e}")))?.format;
    let encode = |mw: u32, mh: u32, px: &[u8]| -> Vec<u8> {
        match format {
            BcFormat::Bc1 => tdb_archive_codec::compress_bc(DdsFormat::Dxt1, mw, mh, px),
            BcFormat::Bc2 => tdb_archive_codec::compress_bc(DdsFormat::Dxt3, mw, mh, px),
            BcFormat::Bc3 => tdb_archive_codec::compress_bc(DdsFormat::Dxt5, mw, mh, px),
            BcFormat::Bc4 => tdb_rx2::bc45::encode_bc4(px, mw, mh),
            BcFormat::Bc5 => tdb_rx2::bc45::encode_bc5(px, mw, mh),
            BcFormat::Bc7 => tdb_rx2::bc7::encode(px, mw, mh),
        }
    };
    let chain = |levels_n: u32| -> Result<Vec<(u32, u32, Vec<u8>)>> {
        let Some(base) = image::RgbaImage::from_raw(w, h, rgba.to_vec()) else {
            return Err(DomainError::Extract("replacement pixel buffer is malformed".into()));
        };
        Ok((0..levels_n)
            .map(|level| {
                let (mw, mh) = ((w >> level).max(1), (h >> level).max(1));
                let px = if level == 0 { base.clone() } else { image::imageops::resize(&base, mw, mh, image::imageops::FilterType::Triangle) };
                (mw, mh, encode(mw, mh, px.as_raw()))
            })
            .collect())
    };
    fn as_levels(c: &[(u32, u32, Vec<u8>)]) -> Vec<tdb_rx2::gnf::LinearLevel<'_>> {
        c.iter().map(|(mw, mh, d)| tdb_rx2::gnf::LinearLevel { width: *mw, height: *mh, linear: d }).collect()
    }
    match policy {
        GnfWritePolicy::Chain { levels } => {
            let c = chain(levels)?;
            let bytes = tdb_rx2::gnf::replace_chain_same_size(gnf_bytes, &as_levels(&c))
                .map_err(|e| DomainError::Extract(format!("GNF mip-chain rewrite: {e}")))?;
            Ok((bytes, Vec::new()))
        }
        GnfWritePolicy::BaseOnly { note } => {
            let bytes = tdb_rx2::gnf::replace_base_mip(gnf_bytes, &encode(w, h, rgba))
                .map_err(|e| DomainError::Extract(format!("GNF base-mip rewrite: {e}")))?;
            Ok((bytes, note.into_iter().collect()))
        }
        GnfWritePolicy::Resize { levels } => {
            let c = chain(levels)?;
            let bytes = tdb_rx2::gnf::replace_resized(gnf_bytes, &as_levels(&c))
                .map_err(|e| DomainError::Extract(format!("GNF resize: {e}")))?;
            Ok((bytes, Vec::new()))
        }
    }
}

/// Write an edited texture into an extracted **RW4 texlib** (`.rx2` — the loose
/// logo/uniform/equipment rasters). Picks the largest texture by area (the same
/// slot preview/export show) and re-encodes `rgba` (w×h RGBA8) into that slot's
/// format.
///
/// What a DXT slot gets is decided by [`xb2_dxt_policy`] (the same rule the pre-Apply plan
/// reports): at the shipped size a **partial** replacement — the base and, inside the audited
/// population, the large levels, with the shipped packed tail kept; at a changed size a
/// single-level re-emit for a single-level raster and a **refusal** for a mipped one. The
/// disclosures are returned by [`replace_texlib_texture_noted`]; this wrapper drops them.
///
/// An uncompressed `Argb8888` slot is stored linear (no DXT mip tiling), so it
/// takes the RGBA re-ordered to the stored `[A,R,G,B]` byte layout and writes the
/// base level via [`tdb_rx2::texlib::replace_same_size`].
pub fn replace_texlib_texture(
    file_bytes: &[u8],
    rgba: &[u8],
    w: u32,
    h: u32,
    texture_index: Option<usize>,
) -> Result<Vec<u8>> {
    replace_texlib_texture_noted(file_bytes, rgba, w, h, texture_index).map(|(bytes, _)| bytes)
}

/// The Xbox 360 `RW4xb2` DXT write policy for one request, decided from the file's own bytes
/// **before anything is encoded or written** and shared by the writer and
/// [`plan_texture_replace`] (coordinator review of receipts 10–17, F1, 22 Sep 2026):
///
/// * **same size** — a *partial* replacement: the base is rewritten; inside the audited
///   population ([`tdb_rx2::texlib::LargeLevelSupport`]) the large levels too, at the sequential
///   offsets the mip placement audit matched by content; every packed-tail level (64 texels or
///   less) keeps the shipped bytes, because the console's packed-mip layout is not established.
///   Dimensions, format, mip declarations, slot identity and every byte outside the written
///   ranges are unchanged;
/// * **changed size, single-level raster** — one level is re-emitted under the descriptor rule;
///   no smaller level is addressed;
/// * **changed size, mipped raster** — **refused**: a resized chain would place packed-tail
///   levels under a layout nothing independent supports, and shortening the chain to dodge that
///   is not done either. The codec-level `replace_with_resize` stays available to callers outside
///   the Replace flow (the external headmesh workstream, the census tests); it is not qualified
///   coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Xb2DxtPolicy {
    SameSizePartial {
        support: tdb_rx2::texlib::LargeLevelSupport,
        /// Declared levels of the shipped raster.
        declared: u32,
        /// Base plus large levels that will be written (what fits the slot).
        levels_written: u32,
        /// Declared levels kept as shipped.
        tail_levels_kept: u32,
        /// Slot bytes after the last written level, kept as shipped.
        tail_bytes_kept: usize,
    },
    ResizeSingleLevel,
    ResizeRefused { reason: String },
}

/// Decide [`Xb2DxtPolicy`] for raster `idx` of `lib` and a `w`×`h` request. Pure: reads the
/// parsed descriptor and slot size only.
pub fn xb2_dxt_policy(lib: &tdb_rx2::texlib::Texlib, idx: usize, w: u32, h: u32) -> Result<Xb2DxtPolicy> {
    use tdb_rx2::texlib::{is_packed_tail_level, tiled_buffer_size, LargeLevelSupport};
    let t = &lib.textures[idx];
    let declared = lib.declared_mip_count(idx);
    if (w, h) != (t.width, t.height) {
        if declared < 2 {
            return Ok(Xb2DxtPolicy::ResizeSingleLevel);
        }
        let new_levels = full_mip_count(w, h);
        let tail = (0..new_levels).filter(|&l| is_packed_tail_level((w >> l).max(1), (h >> l).max(1))).count();
        return Ok(Xb2DxtPolicy::ResizeRefused {
            reason: format!(
                "Xbox 360 changed-size replacement refused: this raster declares {declared} mip levels, and a {w}×{h} chain would place {tail} packed-tail level{} (64 texels or less) under a layout that is not independently established (mip placement audit 2026-09-22; the packed-tail contract is open research question Q-R65-1). Nothing was written. Replace at the shipped size ({}×{}) instead: the base and the audited large levels are rewritten and the shipped tail is kept",
                if tail == 1 { "" } else { "s" },
                t.width,
                t.height
            ),
        });
    }
    let support = lib.large_level_support(idx).map_err(|e| DomainError::Extract(format!("texlib gate: {e}")))?;
    let bpb = t.format.bytes_per_block();
    let capacity = t.buffer_capacity();
    let mut cursor = 0usize;
    let mut levels_written = 0u32;
    for l in 0..declared {
        let (lw, lh) = ((t.width >> l).max(1), (t.height >> l).max(1));
        if l > 0 && (!matches!(support, LargeLevelSupport::LargeLevels { .. }) || is_packed_tail_level(lw, lh)) {
            break;
        }
        let size = tiled_buffer_size(lw, lh, bpb);
        if cursor + size > capacity {
            break;
        }
        cursor += size;
        levels_written += 1;
    }
    Ok(Xb2DxtPolicy::SameSizePartial {
        support,
        declared,
        levels_written,
        tail_levels_kept: declared.saturating_sub(levels_written),
        tail_bytes_kept: capacity.saturating_sub(cursor),
    })
}

/// The user-facing disclosures of a policy — identical before Apply (the plan) and in the result.
pub fn xb2_policy_notes(policy: &Xb2DxtPolicy, w: u32, h: u32) -> Vec<String> {
    use tdb_rx2::texlib::LargeLevelSupport;
    match policy {
        Xb2DxtPolicy::ResizeRefused { .. } => Vec::new(),
        Xb2DxtPolicy::ResizeSingleLevel => vec![format!(
            "Xbox 360 changed size: the shipped raster declares a single level, so one level is re-emitted at {w}×{h} under the measured descriptor rule and no smaller level is written. Whether the console accepts a changed size is not established (runtime card R32-XB2-1)"
        )],
        Xb2DxtPolicy::SameSizePartial { support, declared, levels_written, tail_levels_kept, tail_bytes_kept } => {
            if *declared < 2 {
                return Vec::new();
            }
            let plural = |n: u32| if n == 1 { "" } else { "s" };
            match support {
                LargeLevelSupport::LargeLevels { .. } => vec![format!(
                    "Partial replacement (Xbox 360 mip chain): the base and {} smaller level{} above 64 texels are rewritten at the offsets the mip placement audit of 2026-09-22 matched by content; the {} packed-tail level{} (64 texels or less, {} bytes) keep the shipped bytes and so still carry the previous image's content, because the console's packed-mip layout is not established. Dimensions, format, mip declarations and slot identity are unchanged; this is not a full-image replacement",
                    levels_written.saturating_sub(1),
                    plural(levels_written.saturating_sub(1)),
                    tail_levels_kept,
                    plural(*tail_levels_kept),
                    tail_bytes_kept,
                )],
                LargeLevelSupport::BaseOnly { reason } => vec![format!(
                    "Partial replacement (Xbox 360): only the base level is rewritten — {reason} — so the audited large-level placement does not apply; the {} smaller level{} ({} bytes) keep the shipped bytes and still carry the previous image's content. Dimensions, format, mip declarations and slot identity are unchanged; this is not a full-image replacement",
                    tail_levels_kept,
                    plural(*tail_levels_kept),
                    tail_bytes_kept,
                )],
            }
        }
    }
}

/// [`replace_texlib_texture`] plus the writer's disclosures. The Xbox 360 DXT behaviour is
/// [`Xb2DxtPolicy`], decided before encoding; the notes are [`xb2_policy_notes`].
pub fn replace_texlib_texture_noted(
    file_bytes: &[u8],
    rgba: &[u8],
    w: u32,
    h: u32,
    texture_index: Option<usize>,
) -> Result<(Vec<u8>, Vec<String>)> {
    use tdb_rx2::texlib::{LinearMip, PixelFormat};

    // PS3 `\x89RW4ps3`: plain slots, re-encoded at the same length (research 18 §3).
    if file_bytes.starts_with(tdb_rx2::texlib_ps3::MAGIC) {
        let lib = tdb_rx2::texlib_ps3::parse(file_bytes)
            .map_err(|e| DomainError::Extract(format!("RW4ps3 parse: {e}")))?;
        let idx = match texture_index {
            Some(i) if i < lib.textures.len() => i,
            Some(i) => {
                return Err(DomainError::Extract(format!(
                    "texture index {i} out of range ({} rasters)",
                    lib.textures.len()
                )))
            }
            None => primary_ps3_raster_index(&lib)
                .ok_or_else(|| DomainError::Extract("texlib has no texture to replace".into()))?,
        };
        let chain = encode_ps3_slot(&lib, idx, rgba, w, h)?;
        let t = &lib.textures[idx];
        if (w, h) == (t.width, t.height) {
            return lib
                .replace_slot(idx, &chain)
                .map_err(|e| DomainError::Extract(format!("RW4ps3 replace: {e}")))
                .map(|b| (b, Vec::new()));
        }
        // P2-B: the PS3 resize of research 2026-09-21 handoff 11 §1 (words 12 / 13 + the linear
        // slot re-sized; cubes refused). Runtime acceptance is card R33-PS3-1.
        let mips = ps3_resized_mip_count(t.mips, w, h);
        return lib
            .replace_slot_resized(idx, w, h, mips, &chain)
            .map_err(|e| DomainError::Extract(format!("RW4ps3 resize: {e}")))
            .map(|b| (b, Vec::new()));
    }

    let lib = tdb_rx2::texlib::parse(file_bytes)
        .map_err(|e| DomainError::Extract(format!("texlib parse: {e}")))?;
    // An explicit index targets one raster (the jersey body vs. the number font); `None` targets the
    // colour-map raster — writing into the same-size grayscale spec map would leave the jersey
    // unchanged and corrupt its spec map. Matches the read-side pick in `decode_texture_file`.
    let idx = match texture_index {
        Some(i) if i < lib.textures.len() => i,
        Some(i) => {
            return Err(DomainError::Extract(format!(
                "texture index {i} out of range ({} rasters)",
                lib.textures.len()
            )))
        }
        None => primary_raster_index(&lib.textures)
            .ok_or_else(|| DomainError::Extract("texlib has no texture to replace".into()))?,
    };
    let (tw, th, tf) = {
        let t = &lib.textures[idx];
        (t.width, t.height, t.format)
    };
    let resized = (w, h) != (tw, th);
    if resized && matches!(tf, PixelFormat::Argb8888) {
        if let Some(why) = argb_resize_refusal(&lib, idx, w, h) {
            return Err(DomainError::Extract(why));
        }
    }
    if resized && !matches!(tf, PixelFormat::Argb8888) && (!w.is_multiple_of(4) || !h.is_multiple_of(4)) {
        return Err(DomainError::Extract(format!("a DXT resize needs both dimensions divisible by 4, got {w}x{h}")));
    }
    // The codec is the authority on representability (`patch_descriptor_for_resize` refuses what
    // `DW9` cannot store — coordinator review R1 F1); this early gate only spares encoding a chain
    // that will be refused.
    if resized && (w > tdb_rx2::texlib::DESC_MAX_DIM || h > tdb_rx2::texlib::DESC_MAX_DIM) {
        return Err(DomainError::Extract(format!(
            "texlib resize refused: {w}x{h} cannot be stored in the descriptor (1–{} texels on each axis)",
            tdb_rx2::texlib::DESC_MAX_DIM
        )));
    }

    if matches!(tf, PixelFormat::Argb8888) {
        // Uncompressed slot: `[A,R,G,B]` per texel (tiled for FE art, linear for nameplates) —
        // rotate RGBA into that order and write the base level.
        let mut base = Vec::with_capacity(rgba.len());
        for px in rgba.chunks_exact(4) {
            base.extend_from_slice(&[px[3], px[0], px[1], px[2]]);
        }
        if resized {
            // Receipt 37 (research R132 §3): single-level tiled FE art, descriptor re-derived by
            // the measured rule, the data region reflowed; read back at the new size.
            let bytes = lib
                .replace_with_resize(idx, tf, &[LinearMip { width: w, height: h, linear_dxt: &base }])
                .map_err(|e| DomainError::Extract(format!("texlib resize: {e}")))?;
            return Ok((bytes, vec![format!("uncompressed (8_8_8_8) raster resized {tw}×{th} → {w}×{h} under the measured Xbox 360 descriptor rule (research R132: 6 926 / 6 926 NHL 08–11 rasters); runtime display not observed")]));
        }
        return lib
            .replace_same_size(idx, &base)
            .map_err(|e| DomainError::Extract(format!("texlib replace: {e}")))
            .map(|b| (b, Vec::new()));
    }

    // DXT slot: the policy is decided from the shipped descriptor before anything is encoded, so a
    // refusal costs no encoding and touches no bytes.
    let policy = xb2_dxt_policy(&lib, idx, w, h)?;
    let notes = xb2_policy_notes(&policy, w, h);
    match policy {
        Xb2DxtPolicy::ResizeRefused { reason } => Err(DomainError::Extract(reason)),
        Xb2DxtPolicy::ResizeSingleLevel => {
            // P2-B: the Xbox 360 resize of research 2026-09-21 handoff 11 §2 — the descriptor
            // re-derived from its shipped bytes (`DW8` join checked), one tiled level re-emitted,
            // the data region reflowed; the rule's own refusals (NHL 07 layout, linear rasters,
            // cubes, rasters under 128 texels) are returned as the reason.
            let base = tdb_archive_codec::compress_bc(dds_format_of(tf)?, w, h, rgba);
            let bytes = lib
                .replace_with_resize(idx, tf, &[LinearMip { width: w, height: h, linear_dxt: &base }])
                .map_err(|e| DomainError::Extract(format!("texlib resize: {e}")))?;
            Ok((bytes, notes))
        }
        Xb2DxtPolicy::SameSizePartial { levels_written, tail_levels_kept, tail_bytes_kept, .. } => {
            let levels = compress_bc_mip_levels(dds_format_of(tf)?, w, h, rgba);
            let mips: Vec<LinearMip<'_>> = levels
                .iter()
                .map(|(mw, mh, data)| LinearMip {
                    width: *mw,
                    height: *mh,
                    linear_dxt: data,
                })
                .collect();
            let written = lib
                .replace_large_levels_keep_tail(idx, &mips)
                .map_err(|e| DomainError::Extract(format!("texlib replace: {e}")))?;
            // The policy predicted the write exactly; a divergence would be a Studio defect, and
            // the disclosure must never describe a write that did not happen.
            if (written.levels_written, written.tail_levels_kept, written.tail_bytes_kept) != (levels_written, tail_levels_kept, tail_bytes_kept) {
                return Err(DomainError::Extract(format!(
                    "texlib replace: the writer wrote {} level(s) keeping {} level(s) / {} bytes, but the policy predicted {} / {} / {} — refused so the disclosure stays truthful",
                    written.levels_written, written.tail_levels_kept, written.tail_bytes_kept, levels_written, tail_levels_kept, tail_bytes_kept
                )));
            }
            Ok((written.bytes, notes))
        }
    }
}

/// Why an uncompressed (`8_8_8_8`) Xbox 360 raster cannot take a `w`×`h` replacement of a
/// different size, or `None` when it can (receipt 37, research R132 §3): only a **single-level,
/// Xbox-tiled** raster (front-end art — logos, portraits) whose descriptor the measured rule
/// re-derives; a linear raster (nameplates), a mip chain, the NHL 07 table-at-EOF container, or a
/// size the descriptor cannot hold stays refused with the reason. Pure: bytes are only read.
fn argb_resize_refusal(lib: &tdb_rx2::texlib::Texlib, idx: usize, w: u32, h: u32) -> Option<String> {
    let t = &lib.textures[idx];
    if !t.tiled {
        return Some(format!("texlib replace of a linear uncompressed (8_8_8_8) raster is same-size only: got {w}x{h}, slot is {}x{} (the resize rule covers Xbox-tiled front-end art only)", t.width, t.height));
    }
    if lib.declared_mip_count(idx) > 1 {
        return Some(format!("texlib replace of a mipped uncompressed (8_8_8_8) raster is same-size only: got {w}x{h}, slot is {}x{}", t.width, t.height));
    }
    if w < 32 || h < 32 || w > tdb_rx2::texlib::DESC_MAX_DIM || h > tdb_rx2::texlib::DESC_MAX_DIM {
        return Some(format!("uncompressed (8_8_8_8) resize refused: {w}x{h} is outside what the descriptor rule covers (32–{} texels on each axis)", tdb_rx2::texlib::DESC_MAX_DIM));
    }
    None
}

/// What a texture replacement would do, decided **before Apply** from the target file's bytes and
/// the source image's dimensions — so the Replace… dialog can show the same refusal or partial-
/// replacement disclosure the write itself returns (coordinator review of receipts 10–17, F1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureReplacePlan {
    /// The container the target bytes are recognised as, when they are.
    pub container: Option<TextureContainer>,
    /// `true` when this plan is the writer's own policy (Xbox 360 `RW4xb2` DXT rasters, PS4 GNF). `false`
    /// means "no pre-check exists for this container: the writer's rule decides at write time",
    /// and `allowed` / `notes` say nothing about it.
    pub checked: bool,
    /// `false` when the write is known to be refused; `refusal` then carries the reason.
    pub allowed: bool,
    pub refusal: Option<String>,
    /// The disclosures the result will carry, word for word.
    pub notes: Vec<String>,
}

pub fn plan_texture_replace(file_bytes: &[u8], w: u32, h: u32, texture_index: Option<usize>) -> Result<TextureReplacePlan> {
    use tdb_rx2::texlib::PixelFormat;
    // Review the same inner bytes the writer uses. Otherwise a wrapped NHL07/08
    // raster appears unchecked and Apply refuses its previously undisclosed mip policy.
    if tdb_archive_codec::is_refpack(file_bytes) {
        let inner = tdb_archive_codec::refpack_decompress(file_bytes)
            .map_err(|e| DomainError::Extract(format!("whole-file RefPack: {e}")))?;
        if tdb_archive_codec::is_refpack(&inner) {
            return Err(DomainError::Extract(
                "whole-file RefPack wraps another RefPack stream; not a texture container".into(),
            ));
        }
        return plan_texture_replace(&inner, w, h, texture_index);
    }
    let unchecked = |container: Option<TextureContainer>| TextureReplacePlan { container, checked: false, allowed: true, refusal: None, notes: Vec::new() };
    if file_bytes.starts_with(PNG_MAGIC) {
        return Ok(unchecked(Some(TextureContainer::Png)));
    }
    if file_bytes.starts_with(tdb_rx2::gnf::MAGIC_GNF) {
        // The GNF writer's own policy, so the dialog shows its refusal / disclosure word for word.
        return Ok(match gnf_write_policy(file_bytes, w, h)? {
            Ok(policy) => {
                let notes = match policy {
                    GnfWritePolicy::BaseOnly { note } => note.into_iter().collect(),
                    GnfWritePolicy::Chain { .. } | GnfWritePolicy::Resize { .. } => Vec::new(),
                };
                TextureReplacePlan { container: Some(TextureContainer::Gnf), checked: true, allowed: true, refusal: None, notes }
            }
            Err(reason) => TextureReplacePlan { container: Some(TextureContainer::Gnf), checked: true, allowed: false, refusal: Some(reason), notes: Vec::new() },
        });
    }
    if file_bytes.starts_with(b"BIGF") {
        return Ok(unchecked(Some(TextureContainer::Portrait)));
    }
    if file_bytes.starts_with(tdb_rx2::texlib_ps3::MAGIC) {
        return Ok(unchecked(Some(TextureContainer::Texlib)));
    }
    if !file_bytes.starts_with(b"\x89RW4") {
        return Ok(unchecked(None));
    }
    let refused = |reason: String| TextureReplacePlan { container: Some(TextureContainer::Texlib), checked: true, allowed: false, refusal: Some(reason), notes: Vec::new() };
    let lib = tdb_rx2::texlib::parse(file_bytes).map_err(|e| DomainError::Extract(format!("texlib parse: {e}")))?;
    let idx = match texture_index {
        Some(i) if i < lib.textures.len() => i,
        Some(i) => return Ok(refused(format!("texture index {i} out of range ({} rasters)", lib.textures.len()))),
        None => match primary_raster_index(&lib.textures) {
            Some(i) => i,
            None => return Ok(refused("texlib has no texture to replace".into())),
        },
    };
    let (tw, th, tf) = {
        let t = &lib.textures[idx];
        (t.width, t.height, t.format)
    };
    let resized = (w, h) != (tw, th);
    if resized && matches!(tf, PixelFormat::Argb8888) {
        if let Some(why) = argb_resize_refusal(&lib, idx, w, h) {
            return Ok(refused(why));
        }
    }
    if resized && !matches!(tf, PixelFormat::Argb8888) && (!w.is_multiple_of(4) || !h.is_multiple_of(4)) {
        return Ok(refused(format!("a DXT resize needs both dimensions divisible by 4, got {w}x{h}")));
    }
    if resized && (w > tdb_rx2::texlib::DESC_MAX_DIM || h > tdb_rx2::texlib::DESC_MAX_DIM) {
        return Ok(refused(format!(
            "texlib resize refused: {w}x{h} cannot be stored in the descriptor (1–{} texels on each axis)",
            tdb_rx2::texlib::DESC_MAX_DIM
        )));
    }
    if matches!(tf, PixelFormat::Argb8888) {
        let notes = if resized { vec![format!("uncompressed (8_8_8_8) raster resized {tw}×{th} → {w}×{h} under the measured Xbox 360 descriptor rule (research R132: 6 926 / 6 926 NHL 08–11 rasters); runtime display not observed")] } else { Vec::new() };
        return Ok(TextureReplacePlan { container: Some(TextureContainer::Texlib), checked: true, allowed: true, refusal: None, notes });
    }
    let policy = xb2_dxt_policy(&lib, idx, w, h)?;
    let notes = xb2_policy_notes(&policy, w, h);
    Ok(match policy {
        Xb2DxtPolicy::ResizeRefused { reason } => refused(reason),
        _ => TextureReplacePlan { container: Some(TextureContainer::Texlib), checked: true, allowed: true, refusal: None, notes },
    })
}

/// The extracted texture container a file is, as recognised by
/// [`replace_texture_in_file`] — surfaced so the UI can name what it wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureContainer {
    /// PS4 Kettle `.gnf`.
    Gnf,
    /// Xbox 360 / PS3 portrait `BIGF` archive (`p{id}.big`).
    Portrait,
    /// Loose RW4 `.rx2` texlib (logo / uniform / equipment raster).
    Texlib,
    /// Plain PNG (PS4 NHL 15 league logos and portraits).
    Png,
}

impl TextureContainer {
    /// Stable lower-case tag for DTOs / logs.
    pub fn label(self) -> &'static str {
        match self {
            TextureContainer::Gnf => "gnf",
            TextureContainer::Portrait => "portrait",
            TextureContainer::Texlib => "texlib",
            TextureContainer::Png => "png",
        }
    }

    /// Human-readable name for user-facing text (the `Portrait` tag names the BIGF package
    /// shape, which league logos, imagery and portraits all use).
    pub fn display(self) -> &'static str {
        match self {
            TextureContainer::Gnf => "PS4 GNF",
            TextureContainer::Portrait => "BIGF package",
            TextureContainer::Texlib => "RW4 texlib",
            TextureContainer::Png => "PNG",
        }
    }
}

/// The rebuilt bytes of an edited texture file plus what it turned out to be.
pub struct TextureWrite {
    pub container: TextureContainer,
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
    /// Writer disclosures the caller must surface (e.g. a package rebuilt through the generic
    /// builder because its shape was not the measured one). Empty on the ordinary path.
    pub notes: Vec<String>,
}

impl std::fmt::Debug for TextureWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Summarise the payload rather than dumping every byte.
        f.debug_struct("TextureWrite")
            .field("container", &self.container)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes", &format_args!("<{} bytes>", self.bytes.len()))
            .finish()
    }
}

/// Write an edited texture (RGBA `w`×`h`) back into an extracted asset file,
/// dispatching on the file's container magic. Returns the rebuilt file bytes
/// ready to write over the original — the single entry point the app seam calls
/// so the UI's export→edit→reimport loop works across every texture family.
///
/// Dispatch:
/// * `GNF ` → [`replace_gnf_texture`] (PS4 heads/portraits/maps).
/// * `BIGF` → [`replace_portrait_texture`] (Xbox 360 XPR2 / PS3 DDS portraits).
/// * `\x89RW4` → [`replace_texlib_texture`] (logos / uniforms / equipment).
///
/// A whole-file-RefPack container (NHL 08's wrapped `.big`) is rejected with a
/// clear message rather than mis-parsed — the confirmed extracted trees ship
/// these three containers unwrapped.
pub fn replace_texture_in_file(
    file_bytes: Vec<u8>,
    rgba: &[u8],
    w: u32,
    h: u32,
    texture_index: Option<usize>,
) -> Result<TextureWrite> {
    if file_bytes.starts_with(PNG_MAGIC) {
        // PS4 NHL 15: a plain PNG member; the game reads whatever dimensions the PNG declares.
        let img = DecodedImage {
            width: w,
            height: h,
            rgba: rgba.to_vec(),
        };
        return Ok(TextureWrite {
            container: TextureContainer::Png,
            width: w,
            height: h,
            bytes: img.to_png()?,
            notes: Vec::new(),
        });
    }
    if file_bytes.starts_with(tdb_rx2::gnf::MAGIC_GNF) {
        let (bytes, notes) = replace_gnf_texture_noted(&file_bytes, rgba, w, h)?;
        return Ok(TextureWrite {
            container: TextureContainer::Gnf,
            width: w,
            height: h,
            bytes,
            notes,
        });
    }
    if file_bytes.starts_with(b"BIGF") {
        let (bytes, note) = replace_portrait_texture_noted(file_bytes, rgba, w, h)?;
        return Ok(TextureWrite {
            container: TextureContainer::Portrait,
            width: w,
            height: h,
            bytes,
            notes: note.into_iter().collect(),
        });
    }
    if file_bytes.starts_with(b"\x89RW4") {
        let (bytes, notes) = replace_texlib_texture_noted(&file_bytes, rgba, w, h, texture_index)?;
        return Ok(TextureWrite {
            container: TextureContainer::Texlib,
            width: w,
            height: h,
            bytes,
            notes,
        });
    }
    // A whole-file RefPack wrapper (NHL 07–10 Xbox 360 league-logo packages as the BIG4 archive
    // stores them, NHL 08's wrapped player files): unwrap, rewrite the inner container, re-wrap.
    // The stream Studio's encoder produces generally differs in length from EA's, so placing
    // the result back inside a packed archive is the archive route's concern; the extracted
    // file itself is complete.
    if tdb_archive_codec::is_refpack(&file_bytes) {
        let inner = tdb_archive_codec::refpack_decompress(&file_bytes)
            .map_err(|e| DomainError::Extract(format!("whole-file RefPack: {e}")))?;
        if tdb_archive_codec::is_refpack(&inner) {
            return Err(DomainError::Extract(
                "whole-file RefPack wraps another RefPack stream; not a texture container".into(),
            ));
        }
        let mut written = replace_texture_in_file(inner, rgba, w, h, texture_index)?;
        written.bytes = tdb_archive_codec::refpack_compress(&written.bytes);
        return Ok(written);
    }
    Err(DomainError::Extract(
        "unrecognized texture container (expected a GNF, a BIGF portrait archive, or an RW4 texlib)"
            .into(),
    ))
}

/// Decode a team's **logo** to an RGBA image for the header preview.
///
/// The logo raster is the `rendering/logo/texlib_{artid}_*.rx2` texlib named by
/// the team's `kOtt.lOnr` (`artid`). Tries the known on-disk name variants,
/// parses the texlib, and returns its largest raster de-tiled to RGBA. `None`
/// when the title ships no such file or it doesn't decode — the caller then
/// shows the neutral placeholder.
pub fn decode_team_logo(roots: &AssetRoots, artid: u64) -> Option<DecodedImage> {
    // Primary source: the **front-end team crest** at
    // `fe/ion/artassets/teamlogos/t{artid}.big` — a BIGF archive holding the
    // full-colour 256×256 team logo, keyed by the same `kOtt.lOnr` artid. Verified
    // against the real install (t10 = Red Wings winged wheel, t1 = Jets crest).
    //
    // The `rendering/logo/texlib_{artid}_*.rx2` texlibs are NOT the primary logo:
    // they are the in-rink art — a wide 256×128 **wordmark** plus mostly-empty
    // crest slots — so reading them (and picking the largest raster) yielded
    // wordmarks/secondaries. They stay only as a last-ditch fallback below.
    if !roots.fe.is_empty() {
        let crest = Path::new(&roots.fe)
            .join("ion")
            .join("artassets")
            .join("teamlogos")
            .join(format!("t{artid}.big"));
        if crest.is_file() {
            if let Ok(img) = decode_portrait(&crest) {
                return Some(img);
            }
        }
    }

    // Fallback (titles without the fe crest tree): the rendering texlib. Prefer a
    // non-empty **square** raster (the crest) over the wide wordmark, and skip
    // empty slots — a better guess than largest-by-area, which is the wordmark.
    let logo_dir = Path::new(&roots.rendering).join("logo");
    let ext = if roots.platform == Platform::Ps3 { "rpsgl" } else { "rx2" };
    let candidates = [
        format!("texlib_{artid}_0_0.{ext}"),
        format!("texlib_{artid}.{ext}"),
        format!("texlib_{artid}_0.{ext}"),
    ];
    for name in candidates {
        let path = logo_dir.join(&name);
        if !path.is_file() {
            continue;
        }
        let Ok(bytes) = read_asset_file(&path) else {
            continue;
        };
        let Ok(textures) = read_render_rasters(&bytes) else {
            continue;
        };
        let best = textures
            .iter()
            .filter_map(|t| {
                let rgba = linear_to_rgba(t.format, t.width, t.height, &t.base_mip_linear).ok()?;
                let covered = rgba.chunks_exact(4).filter(|p| p[3] > 8).count();
                // Skip near-empty slots (the common empty crest rasters).
                if covered * 50 < (t.width * t.height) as usize {
                    return None; // < 2% covered
                }
                // Rank: most-square first (crest over wordmark), then larger.
                let squareness = t.width.min(t.height) as f64 / t.width.max(t.height).max(1) as f64;
                let score = (squareness * 1000.0) as u64 * 1_000_000 + covered as u64;
                Some((score, t, rgba))
            })
            .max_by_key(|(score, _, _)| *score);
        if let Some((_, t, rgba)) = best {
            return Some(DecodedImage {
                width: t.width,
                height: t.height,
                rgba,
            });
        }
    }
    None
}

/// The in-game **jersey-select thumbnail** for one kit, resolved from the FE tree.
pub struct JerseyThumbnail {
    pub image: DecodedImage,
    /// Absolute path of the archive the image was decoded from.
    pub path: PathBuf,
    /// `true` when this is the kit's own stock archive (`jersey_{style}_{art}_{variant}.big`) — the
    /// safe overwrite target. `false` = one of the shared `custom*.big` fallbacks the game shows for
    /// kits without stock art; those are shared by every custom kit, so writes must not target them.
    pub stock: bool,
}

/// Resolve + decode the in-game jersey-select preview image for a kit's
/// `{style}_{art_id}_{variant}` asset id: `fe/ion/artassets/jerseys/jersey_{id}.big`, falling back
/// through the game's own chain (`custom.big` → `customhome.big` → `customaway.big`) exactly as the
/// jersey-select menu does. Each is a BIGF archive holding a RefPack-wrapped DDS (PS3) or XPR2
/// (Xbox 360); [`decode_portrait`] handles both. `None` when the FE tree ships none of them.
pub fn jersey_thumbnail(
    roots: &AssetRoots,
    style: u32,
    art_id: u32,
    variant: u32,
) -> Option<JerseyThumbnail> {
    if roots.fe.is_empty() {
        return None;
    }
    let dir = Path::new(&roots.fe)
        .join("ion")
        .join("artassets")
        .join("jerseys");
    let candidates = [
        (format!("jersey_{style}_{art_id}_{variant}.big"), true),
        ("custom.big".to_string(), false),
        ("customhome.big".to_string(), false),
        ("customaway.big".to_string(), false),
    ];
    for (name, stock) in candidates {
        let path = dir.join(&name);
        if !path.is_file() {
            continue;
        }
        if let Ok(image) = decode_portrait(&path) {
            return Some(JerseyThumbnail { image, path, stock });
        }
    }
    None
}

/// One texture file cloned onto a new kit's asset id.
#[derive(Clone, Debug)]
pub struct ClonedKitFile {
    /// Source path (install-relative when under the install root, else absolute), forward slashes.
    pub src: String,
    /// Destination path written, same convention.
    pub dst: String,
    /// How many embedded asset-id occurrences were rewritten inside the copied bytes.
    pub rewrites: u32,
}

/// The outcome of cloning a kit's texture files onto a new asset id.
#[derive(Clone, Debug, Default)]
pub struct KitTextureClone {
    pub files: Vec<ClonedKitFile>,
    /// Destination files that already existed and were left untouched.
    pub skipped_existing: Vec<String>,
}

/// The leading run of numeric `_`-tokens in an outfit file stem, after the textual prefix
/// (`jersey`, `name`, `texlib_pant`, …): `("texlib_pant_0_19_3", …)` → tokens `[0, 19, 3]` starting
/// at split index 2. Stops at the first non-numeric token after the run (the `_cm` map suffix).
fn leading_numeric_tokens(stem: &str) -> Option<(usize, Vec<u64>)> {
    let toks: Vec<&str> = stem.split('_').collect();
    let start = toks.iter().position(|t| t.parse::<u64>().is_ok())?;
    let nums: Vec<u64> = toks[start..]
        .iter()
        .map_while(|t| t.parse::<u64>().ok())
        .collect();
    Some((start, nums))
}

/// Replace every digit-boundary-guarded occurrence of `src` with `dst` (same byte length — the
/// containers reference the asset id in fixed-offset name slots, so the substitution must not shift
/// bytes). The boundary guard (no ASCII digit directly before/after the match) keeps `1_19_3` from
/// rewriting inside `11_19_3` — stricter than the reference app's plain substring rewrite.
fn rewrite_asset_id_bytes(bytes: &mut [u8], src: &[u8], dst: &[u8]) -> u32 {
    debug_assert_eq!(src.len(), dst.len());
    let mut hits = 0u32;
    if src.is_empty() || bytes.len() < src.len() {
        return 0;
    }
    let mut i = 0;
    while i + src.len() <= bytes.len() {
        if &bytes[i..i + src.len()] == src {
            let before_digit = i > 0 && bytes[i - 1].is_ascii_digit();
            let after_digit = bytes.get(i + src.len()).is_some_and(|b| b.is_ascii_digit());
            if !before_digit && !after_digit {
                bytes[i..i + src.len()].copy_from_slice(dst);
                hits += 1;
                i += src.len();
                continue;
            }
        }
        i += 1;
    }
    hits
}

/// A path for the clone report: install-relative with forward slashes when under `install_dir`,
/// else the absolute path (an overridden rendering root outside the install tree).
fn clone_report_path(install_dir: &Path, path: &Path) -> String {
    match path.strip_prefix(install_dir) {
        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
        Err(_) => path.to_string_lossy().replace('\\', "/"),
    }
}

/// Clone a kit's texture files onto a **new asset id** — the file half of "Add New Jersey" (the
/// `xjgw` row is the DB half). Walks the outfit folders (`rendering/jersey|pant|sock`, the same
/// discovery as the Uniforms surface) for every texture whose leading numeric tokens equal the
/// source `{style}_{art}_{variant}`, copies each under the destination tokens, and rewrites the
/// embedded asset-id strings in place (same-length substitution — enforced up front). The FE
/// jersey-select thumbnail (`fe/ion/artassets/jerseys/jersey_{id}.big`) is cloned the same way when
/// it exists. Existing destination files are never clobbered (reported as skipped). Proven on the
/// reference app's PS3/Xbox pipelines; a whole-file-RefPack container (NHL 08) copies fine but its
/// compressed interior can't be string-rewritten (reported with 0 rewrites).
pub fn clone_kit_textures(
    install_dir: &Path,
    roots: &AssetRoots,
    src: (u32, u32, u32),
    dst: (u32, u32, u32),
) -> Result<KitTextureClone> {
    clone_kit_textures_to(install_dir, roots, src, dst, None)
}

/// [`clone_kit_textures`] with the outputs written under `out_root` at the same install-relative
/// paths (ONB-002: the project's overlay) instead of beside the sources. Sources are read from the
/// install; an output that already exists under `out_root` is skipped like an installed one.
pub fn clone_kit_textures_to(
    install_dir: &Path,
    roots: &AssetRoots,
    src: (u32, u32, u32),
    dst: (u32, u32, u32),
    out_root: Option<&Path>,
) -> Result<KitTextureClone> {
    let src_id = format!("{}_{}_{}", src.0, src.1, src.2);
    let dst_id = format!("{}_{}_{}", dst.0, dst.1, dst.2);
    if src_id == dst_id {
        return Err(DomainError::Extract(
            "the new kit's asset id equals the source's — nothing to clone".into(),
        ));
    }
    if src_id.len() != dst_id.len() {
        return Err(DomainError::Extract(format!(
            "asset id {dst_id} must have the same character length as the source {src_id} — the \
             texture containers reference the id in fixed-length name slots, so pick style/art/variant \
             digits of matching width"
        )));
    }
    let src_tokens = [src.0 as u64, src.1 as u64, src.2 as u64];

    // Candidate files: the outfit folders' texture files, plus the FE jersey-select thumbnail.
    let mut candidates: Vec<PathBuf> = Vec::new();
    let rendering = Path::new(&roots.rendering);
    for folder in ["jersey", "pant", "sock"] {
        let mut files: Vec<(PathBuf, String)> = Vec::new();
        collect_outfit_files(&rendering.join(folder), 0, &mut files);
        for (path, stem) in files {
            if leading_numeric_tokens(&stem)
                .is_some_and(|(_, nums)| nums.len() >= 3 && nums[..3] == src_tokens)
            {
                candidates.push(path);
            }
        }
    }
    if !roots.fe.is_empty() {
        let fe_thumb = Path::new(&roots.fe)
            .join("ion")
            .join("artassets")
            .join("jerseys")
            .join(format!("jersey_{src_id}.big"));
        if fe_thumb.is_file() {
            candidates.push(fe_thumb);
        }
    }

    let mut out = KitTextureClone::default();
    for path in candidates {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        // Destination filename: swap the first 3 numeric tokens for the destination id. The
        // extension is part of the last token's tail, so operate on the full name via the stem run.
        let (stem, ext) = match file_name.rsplit_once('.') {
            Some((s, e)) => (s.to_string(), Some(e.to_string())),
            None => (file_name.clone(), None),
        };
        let Some((start, _)) = leading_numeric_tokens(&stem) else {
            continue;
        };
        let mut toks: Vec<String> = stem.split('_').map(str::to_string).collect();
        toks[start] = dst.0.to_string();
        toks[start + 1] = dst.1.to_string();
        toks[start + 2] = dst.2.to_string();
        let dst_name = match &ext {
            Some(e) => format!("{}.{e}", toks.join("_")),
            None => toks.join("_"),
        };
        let beside = path.with_file_name(&dst_name);
        let dst_path = match out_root {
            Some(root) => match beside.strip_prefix(install_dir) {
                Ok(rel) => root.join(rel),
                Err(_) => {
                    return Err(DomainError::Extract(format!(
                        "{} lies outside the install root {}; its clone cannot be placed in the project",
                        beside.display(),
                        install_dir.display()
                    )))
                }
            },
            None => beside.clone(),
        };
        if dst_path.exists() || (out_root.is_some() && beside.exists()) {
            out.skipped_existing
                .push(clone_report_path(install_dir, &beside));
            continue;
        }
        if let Some(parent) = dst_path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| DomainError::Io {
                path: parent.display().to_string(),
                source,
            })?;
        }
        let mut bytes = std::fs::read(&path).map_err(|source| DomainError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let rewrites = rewrite_asset_id_bytes(&mut bytes, src_id.as_bytes(), dst_id.as_bytes());
        std::fs::write(&dst_path, &bytes).map_err(|source| DomainError::Io {
            path: dst_path.display().to_string(),
            source,
        })?;
        out.files.push(ClonedKitFile {
            src: clone_report_path(install_dir, &path),
            dst: clone_report_path(install_dir, &beside),
            rewrites,
        });
    }
    Ok(out)
}

/// Decode an operator-supplied source image (PNG/JPG/…) to RGBA8 for import.
/// The `image` crate sniffs the format, so the UI can hand over whatever the
/// operator exported and edited.
pub fn decode_source_image(path: &Path) -> Result<DecodedImage> {
    let img = image::open(path)
        .map_err(|e| DomainError::Extract(format!("{}: image decode failed: {e}", path.display())))?
        .to_rgba8();
    Ok(DecodedImage {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    })
}

/// Decode an in-memory source image (PNG/JPG/… bytes) to RGBA8 — the bytes twin of
/// [`decode_source_image`], for a texture the UI produced on the fly (e.g. the nameplate editor
/// bakes a fresh PNG in-canvas and hands over the bytes rather than a path).
pub fn decode_source_image_bytes(bytes: &[u8]) -> Result<DecodedImage> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| DomainError::Extract(format!("image decode failed: {e}")))?
        .to_rgba8();
    Ok(DecodedImage {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    })
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

/// Inverse of [`dds_format_of`] for the DXT variants a portrait DDS uses.
/// Returns `None` for `Rgba32` (uncompressed portraits are unobserved and go
/// through a different channel-order path, so we don't claim them here).
fn dds_to_texlib_format(fmt: tdb_archive_codec::DdsFormat) -> Option<tdb_rx2::texlib::PixelFormat> {
    use tdb_archive_codec::DdsFormat;
    use tdb_rx2::texlib::PixelFormat;
    match fmt {
        DdsFormat::Dxt1 => Some(PixelFormat::Dxt1),
        DdsFormat::Dxt3 => Some(PixelFormat::Dxt3),
        DdsFormat::Dxt5 => Some(PixelFormat::Dxt5),
        DdsFormat::Rgba32 => None,
    }
}

/// An uncompressed 32-bit DDS as the PS3 NHL 12–Legacy league logos ship it (`leaguelogos`,
/// `leaguelogosbig`, `leaguelogoswide`: flags RGB|ALPHAPIXELS, masks A8R8G8B8, no mip chain).
/// The byte position of each channel comes from its mask, so any 8-bit-per-channel order reads.
struct DdsArgb {
    width: u32,
    height: u32,
    /// Byte index (0–3) of R, G, B, A inside a texel, from the header masks.
    order: [usize; 4],
    /// Declared mip levels (0/1 = base only).
    mips: u32,
}

fn dds_uncompressed_32(bytes: &[u8]) -> Option<DdsArgb> {
    if bytes.len() < 128 || &bytes[..4] != b"DDS " {
        return None;
    }
    let le = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
    let (height, width) = (le(12), le(16));
    let mips = le(28);
    let pf_flags = le(80);
    let bits = le(88);
    // DDPF_RGB (0x40) with 32 bits per pixel and no fourcc (DDPF_FOURCC = 0x4).
    if width == 0 || height == 0 || pf_flags & 0x4 != 0 || pf_flags & 0x40 == 0 || bits != 32 {
        return None;
    }
    let byte_of = |mask: u32| -> Option<usize> {
        match mask {
            0x0000_00FF => Some(0),
            0x0000_FF00 => Some(1),
            0x00FF_0000 => Some(2),
            0xFF00_0000 => Some(3),
            _ => None,
        }
    };
    let order = [byte_of(le(92))?, byte_of(le(96))?, byte_of(le(100))?, byte_of(le(104))?];
    if bytes.len() < 128 + (width as usize) * (height as usize) * 4 {
        return None;
    }
    Some(DdsArgb {
        width,
        height,
        order,
        mips,
    })
}

/// Full DDS mip-chain length for a texture of the given dimensions:
/// `floor(log2(max(w, h))) + 1` (512×512 → 10, 256×128 → 9, 4×4 → 3, 1×1 → 1).
/// A real EA portrait DDS ships the full chain, so the PS3 write path
/// regenerates it to keep the file internally consistent.
fn full_mip_count(w: u32, h: u32) -> u32 {
    let mut max_dim = w.max(h).max(1);
    let mut count = 1u32;
    while max_dim > 1 {
        max_dim >>= 1;
        count += 1;
    }
    count
}

/// Compress `rgba` (w×h RGBA8) into a full DXT mip chain, returning one entry per
/// level `(width, height, linear DXT block stream)` — top mip first, each level a
/// box/triangle downsample of the source block-compressed with
/// [`tdb_archive_codec::compress_bc`]. Levels count matches [`full_mip_count`]; if
/// the source can't be wrapped as an image (length mismatch) only the base level is
/// returned. This is the per-level form the texlib mip-chain writer consumes; the DDS writer
/// concatenates the levels it keeps.
fn compress_bc_mip_levels(
    fmt: tdb_archive_codec::DdsFormat,
    w: u32,
    h: u32,
    rgba: &[u8],
) -> Vec<(u32, u32, Vec<u8>)> {
    let base = tdb_archive_codec::compress_bc(fmt, w, h, rgba);
    let Some(base_img) = image::RgbaImage::from_raw(w, h, rgba.to_vec()) else {
        return vec![(w, h, base)];
    };
    let mip_count = full_mip_count(w, h);
    let mut levels = Vec::with_capacity(mip_count as usize);
    levels.push((w, h, base));
    for level in 1..mip_count {
        let mw = (w >> level).max(1);
        let mh = (h >> level).max(1);
        let scaled =
            image::imageops::resize(&base_img, mw, mh, image::imageops::FilterType::Triangle);
        levels.push((
            mw,
            mh,
            tdb_archive_codec::compress_bc(fmt, mw, mh, scaled.as_raw()),
        ));
    }
    levels
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
    /// Portraits skipped because they're a generic placeholder shared by many
    /// un-portraited players, not the player's own — see [`mark_generic_portraits`].
    pub portraits_skipped_generic: usize,
    /// Players skipped because their bio yields no usable slug.
    pub skipped_no_slug: usize,
    /// Heads not written because `pPaD` is a shared pool id (`>= 60000`) — the
    /// mesh is real but belongs to no single player; captured generically by the
    /// Phase-4 sweep instead. Counted rather than treated as a failure.
    pub meshes_skipped_pool: usize,
    /// Heads not written because `pPaD` is shared by several players (a generic
    /// head packed into a low id, the NHL 08–14 convention) — captured generically
    /// by the Phase-4 sweep, never named after one of the sharers.
    pub meshes_skipped_shared: usize,
    /// Heads not written because the player has no head assigned (`pPaD == 0`).
    pub meshes_skipped_no_head: usize,
    /// Heads not written because `pPaD` is a real, non-pool id but no mesh file
    /// resolves on disk — an incomplete extraction or an unexpected naming scheme.
    /// This is the anomaly worth chasing (it used to hide behind the equality gate).
    pub meshes_skipped_missing: usize,
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
            // A generic placeholder portrait belongs to no one — skip it rather
            // than write the same blank under hundreds of player slugs.
            if p.generic_portrait {
                summary.portraits_skipped_generic += 1;
            } else if let Some(src) = p.portrait_path.as_deref() {
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

        if opts.meshes_glb {
            if let Some(src) = p.mesh_path.as_deref() {
                match read_asset_file(Path::new(src))
                    .and_then(|b| parse_head_mesh(Path::new(src), &b, false))
                {
                    Ok(asset) => {
                        let glb = tdb_rx2::playermesh::write_glb_geometry_only(&asset);
                        let dst = out_dir.join("heads").join(format!("{slug}.glb"));
                        match write_file(&dst, &glb) {
                            Ok(n) => {
                                summary.meshes_written += 1;
                                summary.bytes_written += n;
                            }
                            Err(e) => summary.failures.push(format!("{}: {e}", p.full_name)),
                        }
                    }
                    Err(e) => summary
                        .failures
                        .push(format!("{}: mesh — {e}", p.full_name)),
                }
            } else if p.pool_head {
                // Shared pool head — real mesh, but not this player's own.
                summary.meshes_skipped_pool += 1;
            } else if p.head_shared {
                // Generic head packed into a low, shared id (NHL 08–14).
                summary.meshes_skipped_shared += 1;
            } else if p.head_id == 0 {
                summary.meshes_skipped_no_head += 1;
            } else {
                // Non-pool `pPaD` that resolved no file: the anomaly the old
                // equality gate hid. Count it, and name the first few so the
                // operator can chase a naming-scheme or extraction gap.
                summary.meshes_skipped_missing += 1;
                if summary.meshes_skipped_missing <= 20 {
                    summary.failures.push(format!(
                        "{}: head {} — no mesh file resolved (non-pool id, missing on disk)",
                        p.full_name, p.head_id
                    ));
                }
            }
        }

        if opts.textures {
            for src in &p.texture_paths {
                let src = src.as_str();
                // PS4 GNF: de-tile + decode + write PNG (a `.gnf` holds one
                // texture, named by the file stem's map suffix).
                if Path::new(src)
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("gnf"))
                {
                    let map = Path::new(src)
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    match read_asset_file(Path::new(src))
                        .and_then(|b| {
                            tdb_rx2::gnf::parse(&b)
                                .map_err(|e| DomainError::Extract(format!("{src}: {e}")))
                        })
                        .and_then(|tex| gnf_to_rgba(&tex).map(|rgba| (tex.width, tex.height, rgba)))
                        .and_then(|(w, h, rgba)| {
                            DecodedImage {
                                width: w,
                                height: h,
                                rgba,
                            }
                            .to_png()
                        }) {
                        Ok(png) => {
                            let dst = out_dir.join("textures").join(format!("{map}.png"));
                            match write_file(&dst, &png) {
                                Ok(n) => {
                                    summary.textures_written += 1;
                                    summary.bytes_written += n;
                                }
                                Err(e) => summary.failures.push(format!("{}: {e}", p.full_name)),
                            }
                        }
                        // BC4/BC5/BC7 maps are skipped (decode pending) — not fatal.
                        Err(e) => summary.failures.push(format!("{}: {e}", p.full_name)),
                    }
                    continue;
                }
                match read_asset_file(Path::new(src)).and_then(|b| read_render_rasters(&b)) {
                    Ok(textures) => {
                        for tex in &textures {
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
                                    let dst =
                                        out_dir.join("textures").join(format!("{slug}_{map}.dds"));
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
                let ext = Path::new(mesh).extension().and_then(|e| e.to_str()).unwrap_or("rx2");
                sources.push((mesh, out_dir.join("heads").join(format!("{slug}.{ext}"))));
            }
            for src in &p.texture_paths {
                let stem = Path::new(src)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let tag = raw_source_tag(&stem, p.head_id);
                let ext = Path::new(src).extension().and_then(|e| e.to_str()).unwrap_or("rx2");
                sources.push((
                    src.as_str(),
                    out_dir.join("textures").join(format!("{slug}_{tag}.{ext}")),
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

// ================================================================ file browser + raw-file export
//
// The player-asset export above is *entity-driven*: it walks `ubPc` and resolves each player's art
// by EA's per-title naming convention. This section is *file-driven*: it lets the UI browse the
// extracted install tree and export any file (or a whole folder) the user picks, decoding recognized
// assets to the same glTF/PNG/DDS the player exporter produces. The two share every decoder; only the
// *selection* differs (a file path vs a bio record). Extraction of `.big` archives is done at
// onboarding — this operates purely on the already-loose tree rooted at the install dir.

/// The output a source file decodes to — drives the badge in the browser and which export toggle
/// governs it. `None` (never surfaced here, only as `Option`) means "not a recognized asset".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetKind {
    /// A head/body mesh (RenderWare `.rx2` geometry, or PS4 `.mcd`) → glTF `.glb`.
    Mesh,
    /// A portrait container (EA `.big`, PS4 `.png`/`.gnf`, or NHL 10's RX2 raster) → `.png`.
    Portrait,
    /// A texture bundle or map (`texlib_*.rx2`, loose `*_cm.rx2`, PS4 `.gnf`) → `.dds`/`.png`.
    Texture,
}

impl AssetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AssetKind::Mesh => "mesh",
            AssetKind::Portrait => "portrait",
            AssetKind::Texture => "texture",
        }
    }
}

/// The texture-map tokens EA appends to a raster stem (`head_67_cm` → `cm`). Used both to classify a
/// loose `.rx2` as a texture (not a mesh) and to name a texlib's individual DDS outputs.
const MAP_TOKENS: &[&str] = &["cm", "nm", "sm", "am", "rm", "mm", "coeff", "spec", "gloss"];

/// True when a file stem ends in one of EA's texture-map tokens (`head_10000_cm` → true). A mesh
/// stem (`player_10000`, `hair_10000`) has no such tail.
fn has_map_suffix(stem: &str) -> bool {
    stem.rsplit('_')
        .next()
        .is_some_and(|t| MAP_TOKENS.contains(&t))
}

/// Best-effort classification of a file *by name only* (no bytes read) — cheap enough to run over a
/// whole directory listing. The decode path ([`decode_asset_file`]) is content-driven and will still
/// succeed on an unconventionally-named file; this only drives the browser's badge and the
/// per-kind export toggles.
fn classify_asset_kind(path: &Path) -> Option<AssetKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "mcd" => Some(AssetKind::Mesh),
        "big" => Some(AssetKind::Portrait),
        "png" => Some(AssetKind::Portrait),
        // PS4: `p{id}.gnf` is a portrait; every other GNF is a texture map. Both decode to PNG.
        "gnf" => {
            let is_portrait = stem.len() > 1
                && stem.starts_with('p')
                && stem[1..].chars().all(|c| c.is_ascii_digit());
            Some(if is_portrait {
                AssetKind::Portrait
            } else {
                AssetKind::Texture
            })
        }
        // `.rx2` is overloaded: a texlib bundle, a loose raster, or a head mesh. Name first
        // (texture markers win over the `head`/`player` prefixes they share), else assume a mesh.
        "rx2" | "rpsgl" => {
            if stem.starts_with("texlib") || has_map_suffix(&stem) {
                Some(AssetKind::Texture)
            } else {
                Some(AssetKind::Mesh)
            }
        }
        _ => None,
    }
}

/// Join a forward-slash relative path onto `root`, refusing anything that would escape it (absolute
/// paths, drive prefixes, `..`). Mirrors [`crate::extract`]'s guard but permits an empty `rel` to
/// mean the root itself (the browser opens there). Returns `None` for an unsafe path.
fn safe_join_rel(root: &Path, rel: &str) -> Option<PathBuf> {
    let mut out = root.to_path_buf();
    for raw in rel.split(['/', '\\']) {
        if raw.is_empty() || raw == "." {
            continue;
        }
        match Path::new(raw).components().next() {
            Some(std::path::Component::Normal(c)) if raw != ".." => out.push(c),
            _ => return None,
        }
    }
    Some(out)
}

/// Resolve a file-browser selection (`rel`, a forward-slash path relative to the
/// install root) to an absolute path under `root`, refusing anything that would
/// escape the root (`..`, absolute paths, drive prefixes) → `None`. The public
/// entry point the app seam uses so a UI-supplied path can never write outside
/// the install tree — the write-side counterpart to [`browse_asset_dir`].
pub fn resolve_install_path(root: &Path, rel: &str) -> Option<PathBuf> {
    safe_join_rel(root, rel)
}

/// One entry in a browsed directory of the extracted install.
#[derive(Clone, Debug)]
pub struct BrowseEntry {
    pub name: String,
    /// Forward-slash path relative to the browse root.
    pub rel_path: String,
    pub is_dir: bool,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// The output kind this file decodes to; `None` for directories and unrecognized files.
    pub kind: Option<AssetKind>,
}

/// A single directory level of the extracted install, for the file browser.
#[derive(Clone, Debug)]
pub struct BrowseListing {
    /// The absolute browse root (the install dir).
    pub root: String,
    /// Forward-slash path of the listed directory relative to the root (`""` = the root itself).
    pub rel_path: String,
    /// The parent's rel path, or `None` at the root. Drives the "up" affordance.
    pub parent: Option<String>,
    pub entries: Vec<BrowseEntry>,
}

/// List one directory level under `root`, resolving `rel` safely (no escape). Directories first,
/// then files, each alphabetical. Cheap — classification is by name only, so no file is opened.
pub fn browse_asset_dir(root: &Path, rel: &str) -> Result<BrowseListing> {
    let rel = rel.trim_matches(['/', '\\']);
    let dir = safe_join_rel(root, rel)
        .ok_or_else(|| DomainError::Extract(format!("unsafe or invalid path: {rel:?}")))?;
    let read = std::fs::read_dir(&dir).map_err(|e| DomainError::Io {
        path: dir.to_string_lossy().into_owned(),
        source: e,
    })?;

    let mut entries = Vec::new();
    for ent in read.flatten() {
        let Ok(meta) = ent.metadata() else { continue };
        let name = ent.file_name().to_string_lossy().into_owned();
        let is_dir = meta.is_dir();
        let rel_path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let kind = if is_dir {
            None
        } else {
            classify_asset_kind(Path::new(&name))
        };
        entries.push(BrowseEntry {
            name,
            rel_path,
            is_dir,
            size: if is_dir { 0 } else { meta.len() },
            kind,
        });
    }
    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        })
    });

    let parent = (!rel.is_empty()).then(|| {
        rel.rsplit_once('/')
            .map(|(p, _)| p.to_string())
            .unwrap_or_default()
    });
    Ok(BrowseListing {
        root: root.to_string_lossy().into_owned(),
        rel_path: rel.to_string(),
        parent,
        entries,
    })
}

/// One decoded output ready to write: the extension to give it, an optional suffix that distinguishes
/// several outputs from one source (a texlib's rasters), and the bytes.
struct DecodedOutput {
    ext: &'static str,
    suffix: Option<String>,
    bytes: Vec<u8>,
}

/// A texture-map token from a raster's name, if it ends in one (`head_67_cm.Raster` → `cm`). Used to
/// name a texlib's DDS outputs when there's no player slug to key on (the file browser exports by
/// source name, not by player).
fn raster_suffix(raster_name: &str) -> Option<String> {
    let stem = raster_name
        .split('.')
        .next()
        .unwrap_or(raster_name)
        .to_ascii_lowercase();
    stem.rsplit('_')
        .next()
        .filter(|t| MAP_TOKENS.contains(t))
        .map(String::from)
}

fn mesh_output(path: &Path) -> Result<DecodedOutput> {
    let bytes = read_asset_file(path)?;
    let asset = parse_head_mesh(path, &bytes, false)?;
    Ok(DecodedOutput {
        ext: "glb",
        suffix: None,
        bytes: tdb_rx2::playermesh::write_glb_geometry_only(&asset),
    })
}

fn portrait_output(path: &Path, crop: bool) -> Result<DecodedOutput> {
    let img = decode_portrait(path)?;
    let img = if crop { img.cropped_to_opaque(0) } else { img };
    Ok(DecodedOutput {
        ext: "png",
        suffix: None,
        bytes: img.to_png()?,
    })
}

fn gnf_output(path: &Path) -> Result<DecodedOutput> {
    let bytes = read_asset_file(path)?;
    let tex = tdb_rx2::gnf::parse(&bytes)
        .map_err(|e| DomainError::Extract(format!("{}: {e}", path.display())))?;
    let rgba = gnf_to_rgba(&tex)?;
    let png = DecodedImage {
        width: tex.width,
        height: tex.height,
        rgba,
    }
    .to_png()?;
    Ok(DecodedOutput {
        ext: "png",
        suffix: None,
        bytes: png,
    })
}

/// Decode a texlib bundle to one **PNG** per raster (decoded, directly viewable). A raster whose
/// format we can't decode is skipped; the whole file only errors when nothing at all decodes.
/// Platform-neutral read view. Writers still use the original platform container.
pub(crate) struct RasterRead {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: tdb_rx2::texlib::PixelFormat,
    pub base_mip_linear: Vec<u8>,
}

pub(crate) fn read_render_rasters(bytes: &[u8]) -> Result<Vec<RasterRead>> {
    if bytes.starts_with(tdb_rx2::texlib_ps3::MAGIC) {
        let lib = tdb_rx2::texlib_ps3::parse(bytes)
            .map_err(|e| DomainError::Extract(format!("PS3 texture: {e}")))?;
        lib.textures.iter().enumerate().map(|(i, t)| {
            Ok(RasterRead {
                name: t.name.clone(), width: t.width, height: t.height,
                format: ps3_format_to_texlib(t.format),
                base_mip_linear: lib.base_mip_linear(i)
                    .ok_or_else(|| DomainError::Extract(format!("PS3 texture {i} is truncated")))?.to_vec(),
            })
        }).collect()
    } else {
        let lib = tdb_rx2::texlib::parse(bytes)
            .map_err(|e| DomainError::Extract(format!("Xbox texture: {e}")))?;
        Ok(lib.textures.into_iter().map(|t| RasterRead {
            name: t.name, width: t.width, height: t.height,
            format: t.format, base_mip_linear: t.base_mip_linear,
        }).collect())
    }
}

fn texlib_outputs(path: &Path) -> Result<Vec<DecodedOutput>> {
    let bytes = read_asset_file(path)?;
    let textures = read_render_rasters(&bytes)?;
    let mut out = Vec::new();
    let mut used = std::collections::HashSet::new();
    for (i, tex) in textures.iter().enumerate() {
        let mut suffix = raster_suffix(&tex.name).unwrap_or_else(|| format!("tex{i}"));
        // A head bundle can contain several colour maps; never overwrite the first.
        if !used.insert(suffix.clone()) { suffix = format!("{suffix}_{i}"); }
        if let Ok(rgba) = linear_to_rgba(tex.format, tex.width, tex.height, &tex.base_mip_linear) {
            if let Ok(png) = (DecodedImage {
                width: tex.width,
                height: tex.height,
                rgba,
            })
            .to_png()
            {
                out.push(DecodedOutput {
                    ext: "png",
                    suffix: Some(suffix),
                    bytes: png,
                });
            }
        }
    }
    if out.is_empty() {
        return Err(DomainError::Extract(format!(
            "{}: texlib holds no decodable textures",
            path.display()
        )));
    }
    Ok(out)
}

/// Decode an `.rx2`, which may be a mesh, a texlib, or (NHL 10) a portrait raster. Dispatch on the
/// name convention first, then fall back across the other decoders so an unconventionally-named file
/// still exports; the *last* attempt surfaces its real error rather than a generic one.
fn decode_rx2(path: &Path, crop: bool) -> Result<Vec<DecodedOutput>> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if stem.starts_with("texlib") || has_map_suffix(&stem) {
        if let Ok(o) = texlib_outputs(path) {
            return Ok(o);
        }
        if let Ok(o) = portrait_output(path, crop) {
            return Ok(vec![o]);
        }
        return texlib_outputs(path);
    }
    if let Ok(o) = mesh_output(path) {
        return Ok(vec![o]);
    }
    if let Ok(o) = texlib_outputs(path) {
        return Ok(o);
    }
    if let Ok(o) = portrait_output(path, crop) {
        return Ok(vec![o]);
    }
    mesh_output(path).map(|o| vec![o])
}

/// Decode one source asset file into its export outputs. Dispatches on extension, then on container
/// content for the ambiguous `.rx2`. `crop` trims portraits to their opaque bounds.
fn decode_asset_file(path: &Path, crop: bool) -> Result<Vec<DecodedOutput>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "mcd" => Ok(vec![mesh_output(path)?]),
        "gnf" => Ok(vec![gnf_output(path)?]),
        "png" | "big" => Ok(vec![portrait_output(path, crop)?]),
        "rx2" | "rpsgl" => decode_rx2(path, crop),
        other => Err(DomainError::Extract(format!(
            "{}: .{other} isn't a decodable asset",
            path.display()
        ))),
    }
}

/// What to include in a file-browser export run, and how to treat files that aren't recognized assets.
#[derive(Clone, Copy)]
pub struct FileExportOptions {
    /// Decode meshes (`.rx2`/`.mcd` geometry) to glTF `.glb`.
    pub meshes_glb: bool,
    /// Decode portraits to `.png`.
    pub portraits_png: bool,
    /// Decode textures to `.dds` (RenderWare texlibs) / `.png` (PS4 GNF).
    pub textures: bool,
    /// Copy files that aren't a recognized decodable asset through unchanged rather than skipping
    /// them (also the fallback when a recognized file fails to decode).
    pub copy_unrecognized: bool,
    /// Crop portraits to their opaque bounds (EA ships them on a mostly-empty canvas).
    pub crop_portraits: bool,
}

/// What a file-browser export run produced. Partial results are normal — `failures` carries the
/// already-human-readable reasons, mirroring [`AssetExportSummary`].
#[derive(Default, Debug)]
pub struct FileExportSummary {
    pub files_considered: usize,
    pub meshes_written: usize,
    pub portraits_written: usize,
    pub textures_written: usize,
    pub raw_copied: usize,
    /// Files skipped because they aren't a recognized asset and `copy_unrecognized` was off.
    pub skipped_unrecognized: usize,
    /// Selections refused for trying to escape the browse root.
    pub skipped_unsafe: usize,
    pub failures: Vec<String>,
    pub bytes_written: u64,
}

/// The bounded recursion depth for expanding a selected folder — deep enough for every EA layout,
/// a backstop against a pathological tree.
const MAX_EXPORT_DEPTH: usize = 12;

fn collect_files(
    dir: &Path,
    rel_dir: &str,
    depth: usize,
    out: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for ent in read.flatten() {
        let Ok(meta) = ent.metadata() else { continue };
        let name = ent.file_name().to_string_lossy().into_owned();
        let rel = if rel_dir.is_empty() {
            name
        } else {
            format!("{rel_dir}/{name}")
        };
        if meta.is_dir() {
            if depth < MAX_EXPORT_DEPTH {
                collect_files(&ent.path(), &rel, depth + 1, out, seen);
            }
        } else if meta.is_file() && seen.insert(rel.clone()) {
            out.push(rel);
        }
    }
}

/// `rel` without its final extension, keeping the forward-slash directory prefix.
fn strip_ext(rel: &str) -> &str {
    match rel.rsplit_once('.') {
        Some((base, ext)) if !ext.contains('/') && !base.is_empty() => base,
        _ => rel,
    }
}

/// Turn a forward-slash relative path into a native `PathBuf` for joining onto an output dir.
fn rel_to_native(rel: &str) -> PathBuf {
    let mut p = PathBuf::new();
    for c in rel.split('/') {
        if !c.is_empty() {
            p.push(c);
        }
    }
    p
}

fn copy_through(src: &Path, rel: &str, out_dir: &Path, summary: &mut FileExportSummary) {
    match std::fs::read(src) {
        Ok(bytes) => {
            let dst = out_dir.join(rel_to_native(rel));
            match write_file(&dst, &bytes) {
                Ok(n) => {
                    summary.raw_copied += 1;
                    summary.bytes_written += n;
                }
                Err(e) => summary.failures.push(format!("{rel}: {e}")),
            }
        }
        Err(e) => summary.failures.push(format!("{rel}: read failed: {e}")),
    }
}

fn export_one_file(
    src: &Path,
    rel: &str,
    out_dir: &Path,
    opts: FileExportOptions,
    summary: &mut FileExportSummary,
) {
    let kind = classify_asset_kind(src);
    let wanted = match kind {
        Some(AssetKind::Mesh) => opts.meshes_glb,
        Some(AssetKind::Portrait) => opts.portraits_png,
        Some(AssetKind::Texture) => opts.textures,
        None => false,
    };
    if !wanted {
        // Unrecognized files copy through (if asked) or are counted as skipped; a recognized file
        // the user simply didn't tick is skipped silently.
        if kind.is_none() {
            if opts.copy_unrecognized {
                copy_through(src, rel, out_dir, summary);
            } else {
                summary.skipped_unrecognized += 1;
            }
        }
        return;
    }

    match decode_asset_file(src, opts.crop_portraits) {
        Ok(outputs) => {
            let base = strip_ext(rel);
            for o in outputs {
                let name = match &o.suffix {
                    Some(s) => format!("{base}_{s}.{}", o.ext),
                    None => format!("{base}.{}", o.ext),
                };
                let dst = out_dir.join(rel_to_native(&name));
                match write_file(&dst, &o.bytes) {
                    Ok(n) => {
                        match kind {
                            Some(AssetKind::Mesh) => summary.meshes_written += 1,
                            Some(AssetKind::Portrait) => summary.portraits_written += 1,
                            Some(AssetKind::Texture) => summary.textures_written += 1,
                            None => {}
                        }
                        summary.bytes_written += n;
                    }
                    Err(e) => summary.failures.push(format!("{rel}: {e}")),
                }
            }
        }
        // A recognized asset that won't decode falls back to a raw copy when allowed, so the user
        // still gets the source bytes rather than nothing.
        Err(e) => {
            if opts.copy_unrecognized {
                copy_through(src, rel, out_dir, summary);
            } else {
                summary.failures.push(format!("{rel}: {e}"));
            }
        }
    }
}

/// Export a set of selected files/folders from the extracted install `root` into `out_dir`, decoding
/// recognized assets to glTF/PNG/DDS. Folders are walked recursively (bounded depth). The output
/// mirrors each source's path relative to the root, so a title's own folder layout is preserved —
/// the requirement that this "work with each game's unique file structures".
pub fn export_asset_files(
    root: &Path,
    rel_paths: &[String],
    out_dir: &Path,
    opts: FileExportOptions,
) -> Result<FileExportSummary> {
    let mut summary = FileExportSummary::default();

    // Expand folders to their files, de-duplicated by rel path (a folder and one of its files can
    // both be selected).
    let mut files: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for rel in rel_paths {
        let rel = rel.trim_matches(['/', '\\']);
        let Some(abs) = safe_join_rel(root, rel) else {
            summary.skipped_unsafe += 1;
            continue;
        };
        if abs.is_dir() {
            collect_files(&abs, rel, 0, &mut files, &mut seen);
        } else if abs.is_file() {
            let key = rel.replace('\\', "/");
            if seen.insert(key.clone()) {
                files.push(key);
            }
        }
        // A path that no longer exists is ignored — the tree may have changed since the UI listed it.
    }

    for rel in files {
        summary.files_considered += 1;
        let Some(src) = safe_join_rel(root, &rel) else {
            summary.skipped_unsafe += 1;
            continue;
        };
        export_one_file(&src, &rel, out_dir, opts, &mut summary);
    }
    Ok(summary)
}

// ================================================================ team outfit export
//
// A team-centric counterpart to the player-asset export: instead of one player's head art, this
// walks the RenderWare outfit trees under `rendering/` and writes every team's **jersey, pants and
// socks** out as PNGs named by the team (e.g. `detroit_red_wings_jersey_0_0_cm.png`). The three
// pieces live in the `jersey/`, `pant/` and `sock/` folders, keyed by the team id in each file's
// name. Two on-disk shapes are handled, both by the same code:
//
//   * **NHL 09/12/14** — flat texlib bundles: `rendering/jersey/texlib_{set}_{team}_{variant}.rx2`,
//     each holding several rasters (`jersey_*`, `font_*` / `pant_*`); socks are loose `sock_*_cm.rx2`.
//   * **NHL 07/08** — loose single-raster files in a per-team subfolder:
//     `rendering/jersey/jersey_{team}/jersey_{set}_{team}_{variant}_{map}.rx2` (+ `font_*`, `name_*`),
//     `pant/pant_{team}/pant_*_{map}.rx2`, `sock/sock_{team}/sock_*_{map}.rx2`.
//
// So each folder is walked **recursively**, and a file counts as an outfit texture if it's a texlib
// bundle or a loose raster (its name ends in a map suffix like `_cm`/`_sm`/`_nm`) — which excludes
// the mesh `.rx2` files that share the folder. The team-id token position varies by title (NHL 09
// socks are `sock_a_b_team_c_d_cm`, NHL 14's are shorter, NHL 07 jerseys are `jersey_set_team_v_cm`),
// so it's derived per folder at runtime by [`derive_team_index`].

/// The three outfit folders under `rendering/`, and the summary bucket each feeds.
const OUTFIT_FOLDERS: &[&str] = &["jersey", "pant", "sock"];

/// Bounded depth for walking a piece folder — NHL 07/08 nest one per-team subfolder deep; a couple of
/// extra levels is harmless slack.
const OUTFIT_WALK_DEPTH: usize = 3;

/// The underscore-split token index that holds the team token (a jersey art id or, on titles
/// without a registry, the team id), inferred from a folder's filenames: the position whose values
/// cover the most *distinct* known tokens. The real team index spans ~30 clubs, while the
/// style/set indices only ever take one or two values, so it wins decisively — and this adapts to
/// each title's filename shape instead of hard-coding a position. `None` when no position covers a
/// meaningful number of teams (this folder isn't team-keyed on this title).
fn derive_team_index(stems: &[String], by_id: &HashMap<u64, (u64, String)>) -> Option<usize> {
    const MAX_IDX: usize = 8;
    const MIN_TEAMS: usize = 4;
    let mut best: Option<(usize, usize)> = None;
    for idx in 1..MAX_IDX {
        let mut distinct = std::collections::HashSet::new();
        for stem in stems {
            if let Some(id) = team_id_at(stem, idx) {
                if by_id.contains_key(&id) {
                    distinct.insert(id);
                }
            }
        }
        let count = distinct.len();
        if count >= MIN_TEAMS && best.is_none_or(|(_, c)| count > c) {
            best = Some((idx, count));
        }
    }
    best.map(|(idx, _)| idx)
}

/// A team resolved for the outfit export. `slug` is the human-readable name slug (`detroit_red_wings`
/// — the RenderWare output name and, normalized, the key PS4 filenames are matched against); `abbrev`
/// is the lowercased team code (`det`, `nyr`), which PS4 uses for a few clubs (the New York teams).
#[derive(Clone, Debug)]
pub struct OutfitTeam {
    pub team_id: u64,
    pub slug: String,
    pub abbrev: String,
    /// The team's registered jersey art ids (`xjgw.GDQL`) — the id space the RenderWare outfit
    /// filenames actually key on. When ANY team carries these, matching runs on art ids
    /// exclusively (mixing raw team ids back in recreates cross-space collisions — Team Poland's
    /// art id equals the Portland Pirates' team id). Empty on titles without a jersey registry,
    /// where matching falls back to the team id itself.
    pub art_ids: Vec<u64>,
}

/// Filename-token → `(db team id, slug)` for the outfit matchers: keyed on the jersey art-id
/// space when the roster carries one (see [`OutfitTeam::art_ids`]), else on raw team ids.
fn outfit_token_map(teams: &[OutfitTeam]) -> HashMap<u64, (u64, String)> {
    let any_art = teams.iter().any(|t| !t.art_ids.is_empty());
    let mut map: HashMap<u64, (u64, String)> = HashMap::new();
    for t in teams {
        if any_art {
            for &a in &t.art_ids {
                map.entry(a).or_insert_with(|| (t.team_id, t.slug.clone()));
            }
        } else {
            map.entry(t.team_id)
                .or_insert_with(|| (t.team_id, t.slug.clone()));
        }
    }
    map
}

/// What a team-outfit export produced. Partial results are normal — `failures` carries the
/// already-human-readable reasons, mirroring [`AssetExportSummary`].
#[derive(Default, Debug)]
pub struct OutfitExportSummary {
    pub teams_considered: usize,
    pub jerseys_written: usize,
    pub pants_written: usize,
    pub socks_written: usize,
    pub failures: Vec<String>,
    pub bytes_written: u64,
    /// The art folder that was scanned for outfit textures — surfaced so a "found nothing" result is
    /// diagnosable (the UI can show where it looked).
    pub rendering_root: String,
}

/// The team id encoded in an underscore-split filename/raster stem at `idx`, if it parses.
fn team_id_at(stem: &str, idx: usize) -> Option<u64> {
    stem.split('_').nth(idx).and_then(|t| t.parse::<u64>().ok())
}

/// Build the exported raster name: the raster's own stem with its team-id token replaced by the
/// team slug, so `jersey_0_10_0_cm` for Detroit becomes `detroit_red_wings_jersey_0_0_cm`. The slug
/// leads (it's how the user finds the file); the remaining indices — jersey set and variant — are
/// kept so a team's several jerseys don't collide.
fn outfit_output_name(slug: &str, raster_stem: &str, team_idx: usize, team_id: u64) -> String {
    let mut toks: Vec<&str> = raster_stem.split('_').collect();
    // Only drop the token at `team_idx` when it really is this team's id — so a title whose raster
    // names differ from its filenames keeps a correct (if id-bearing) name rather than losing a digit.
    if toks.get(team_idx).and_then(|t| t.parse::<u64>().ok()) == Some(team_id) {
        toks.remove(team_idx);
    }
    format!("{slug}_{}", toks.join("_"))
}

impl OutfitExportSummary {
    fn merge(&mut self, other: OutfitExportSummary) {
        self.jerseys_written += other.jerseys_written;
        self.pants_written += other.pants_written;
        self.socks_written += other.socks_written;
        self.bytes_written += other.bytes_written;
        self.failures.extend(other.failures);
    }
}

/// One decode-and-write unit: a texlib/rx2 file, the team it belongs to, and which folder it's from.
struct OutfitWork {
    path: PathBuf,
    /// The file's own stem (no extension). For loose rasters this is the descriptive name
    /// (`jersey_0_10_0_cm`); for a `texlib_*` bundle it's not per-raster, so the raster's own name is
    /// used instead — see [`process_one_outfit`].
    file_stem: String,
    slug: String,
    team_id: u64,
    /// The raw filename token the team resolved from (a jersey art id, or the team id on titles
    /// without a registry). Kept because token→team is NOT one-to-one: alternate teams (Winter
    /// Classic) register kits under their parent club's art id, so a per-team listing must match
    /// on the requesting team's own token set, not on the exclusive attribution above.
    token: u64,
    team_idx: usize,
    folder: &'static str,
}

/// True when `fname` is an outfit texture `.rx2` — a texlib bundle (NHL 09+) or a loose raster whose
/// name ends in a map suffix (NHL 07/08). Mesh `.rx2` files (e.g. `pant_0_0_0_0_0_0.rx2`, ending in a
/// bare index) are excluded.
fn outfit_texture_file(fname: &str) -> bool {
    let lower = fname.to_ascii_lowercase();
    let Some(stem) = lower.strip_suffix(".rx2").or_else(|| lower.strip_suffix(".rpsgl")) else {
        return false;
    };
    stem.starts_with("texlib_") || has_map_suffix(&stem.to_ascii_lowercase())
}

/// Recursively collect outfit-texture files under `dir` (bounded depth), as `(path, filename-stem)`.
fn collect_outfit_files(dir: &Path, depth: usize, out: &mut Vec<(PathBuf, String)>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for ent in read.flatten() {
        let Ok(ft) = ent.file_type() else { continue };
        if ft.is_dir() {
            if depth < OUTFIT_WALK_DEPTH {
                collect_outfit_files(&ent.path(), depth + 1, out);
            }
        } else {
            let fname = ent.file_name().to_string_lossy().into_owned();
            if outfit_texture_file(&fname) {
                let stem = Path::new(&fname).file_stem().unwrap_or_default().to_string_lossy().into_owned();
                out.push((ent.path(), stem));
            }
        }
    }
}

/// Decode one outfit file's rasters to PNGs and write them, accumulating into `summary`.
fn process_one_outfit(item: &OutfitWork, out_dir: &Path, summary: &mut OutfitExportSummary) {
    let fname = item.path.file_name().unwrap_or_default().to_string_lossy();
    let bytes = match read_asset_file(&item.path) {
        Ok(b) => b,
        Err(e) => {
            summary.failures.push(format!("{fname}: {e}"));
            return;
        }
    };
    // Dispatch on the native container, just as preview/replacement do. PS3
    // outfits use linear PSGL rasters rather than the Xbox tiled texture layout.
    let textures = if bytes.starts_with(tdb_rx2::texlib_ps3::MAGIC) {
        let Ok(lib) = tdb_rx2::texlib_ps3::parse(&bytes) else { return; };
        lib.textures.iter().enumerate().filter_map(|(i, t)| {
            lib.base_mip_linear(i).map(|base| (t.name.clone(), ps3_format_to_texlib(t.format), t.width, t.height, base.to_vec()))
        }).collect::<Vec<_>>()
    } else {
        // Non-texlib files sharing the prefix (e.g. sock meshes) are skipped.
        let Ok(lib) = tdb_rx2::texlib::parse(&bytes) else { return; };
        lib.textures.into_iter().map(|t| (t.name, t.format, t.width, t.height, t.base_mip_linear)).collect::<Vec<_>>()
    };
    // A `texlib_*` bundle names each raster internally (`jersey_0_10_0_cm`) — use that. A loose
    // raster file carries the descriptive name in the *filename* and only a generic internal name
    // (`texture_0000`), so use the filename stem instead.
    let is_bundle = item.file_stem.starts_with("texlib_");
    for (texture_name, format, width, height, base) in textures {
        let raster_name = texture_name.split('.').next().unwrap_or(&texture_name);
        let raster_stem = if is_bundle {
            raster_name
        } else {
            item.file_stem.as_str()
        };
        match linear_to_rgba(format, width, height, &base).and_then(
            |rgba| {
                DecodedImage {
                    width,
                    height,
                    rgba,
                }
                .to_png_fast()
            },
        ) {
            Ok(png) => {
                let name = outfit_output_name(&item.slug, raster_stem, item.team_idx, item.team_id);
                let dst = out_dir.join(item.folder).join(format!("{name}.png"));
                match write_file(&dst, &png) {
                    Ok(n) => {
                        match item.folder {
                            "jersey" => summary.jerseys_written += 1,
                            "pant" => summary.pants_written += 1,
                            "sock" => summary.socks_written += 1,
                            _ => {}
                        }
                        summary.bytes_written += n;
                    }
                    Err(e) => summary.failures.push(format!("{name}: {e}")),
                }
            }
            Err(e) => summary
                .failures
                .push(format!("{fname} [{raster_stem}]: {e}")),
        }
    }
}

/// Export every team's jersey/pants/socks under `rendering_root` into `out_dir` as PNGs named by the
/// team. `teams` is resolved by the caller (the slug is the human-readable team name, e.g.
/// `detroit_red_wings`). Dispatches on the on-disk layout:
///
///   * **PS4** (kettle tree) — GNF textures under `characters/player`, filenames already team- and
///     variant-named (`jersey_rbk_detroit_home_cm.gnf`). See [`export_ps4_outfits`].
///   * **RenderWare** (NHL 07–14 / Legacy, Xbox 360 / PS3) — `jersey/`, `pant/`, `sock/` texlibs
///     keyed by a numeric team id in the filename.
///
/// A run decodes thousands of textures, so after the (cheap) folder scan the work is spread across
/// the machine's cores — serial decoding took minutes, which read as a hung export.
pub fn export_team_outfits(
    teams: &[OutfitTeam],
    rendering_root: &Path,
    out_dir: &Path,
) -> Result<OutfitExportSummary> {
    // PS4 kettle layout: the outfit textures live under `characters/player`.
    let ps4_player = rendering_root.join("characters").join("player");
    if ps4_player.is_dir() {
        return export_ps4_outfits(&ps4_player, teams, out_dir);
    }

    // 1. Walk each outfit folder, deriving the team token position per folder, collecting the work.
    let work = collect_outfit_work(rendering_root, teams);

    // 2. Decode + write in parallel across the cores, then merge the per-worker tallies.
    let mut summary = run_outfit_workers(&work, |item, s| process_one_outfit(item, out_dir, s));
    summary.teams_considered = teams.len();
    summary.rendering_root = rendering_root.to_string_lossy().into_owned();
    Ok(summary)
}

/// Walk the RenderWare `jersey/`/`pant/`/`sock/` folders and resolve each outfit texture to the team
/// it belongs to. The team-id token position varies per title, so it's derived per folder from the
/// filenames ([`derive_team_index`]) before matching. Shared by the bulk [`export_team_outfits`] and
/// the per-team [`list_team_outfits`].
fn collect_outfit_work(rendering_root: &Path, teams: &[OutfitTeam]) -> Vec<OutfitWork> {
    let tokens = outfit_token_map(teams);
    let mut work: Vec<OutfitWork> = Vec::new();
    for &folder in OUTFIT_FOLDERS {
        let mut files: Vec<(PathBuf, String)> = Vec::new();
        collect_outfit_files(&rendering_root.join(folder), 0, &mut files);
        if files.is_empty() {
            continue; // this title/platform doesn't ship the piece here
        }
        let stems: Vec<String> = files.iter().map(|(_, s)| s.clone()).collect();
        let Some(team_idx) = derive_team_index(&stems, &tokens) else {
            continue; // no team-keyed position here — don't guess
        };
        for (path, stem) in files {
            let Some(token) = team_id_at(&stem, team_idx) else {
                continue;
            };
            // Resolve the token to its DB team, or fall back to a generic `team_<token>` so
            // entries the DB doesn't carry (special/all-star slots, ids beyond the roster) stay
            // distinguishable.
            let (team_id, slug) = tokens
                .get(&token)
                .map(|(id, s)| (*id, s.clone()))
                .unwrap_or_else(|| (token, format!("team_{token}")));
            work.push(OutfitWork {
                path,
                file_stem: stem,
                slug,
                team_id,
                token,
                team_idx,
                folder,
            });
        }
    }
    work
}

/// One outfit texture file resolved to a team, for the per-team Uniforms surface. `abs_path` is the
/// source file (a texlib `.rx2` or PS4 `.gnf`); the caller maps it to an install-relative path for
/// preview/replace. `label` is a human-readable descriptor (piece + set/variant/map, team token
/// dropped since the surface is already team-scoped) — a resolved descriptor per the
/// human-readable-surfaces rule, not the raw id-bearing filename (kept in `file_name` for details).
#[derive(Clone, Debug)]
pub struct OutfitTexture {
    pub abs_path: PathBuf,
    /// The raster within the file this entry refers to — a jersey texlib bundle yields separate
    /// entries for its body colour map and its number `font` map (different `texture_index`, same
    /// file). Threaded to the preview/replace commands so each targets its own raster.
    pub texture_index: u32,
    /// The outfit piece: `jersey` / `font` / `pant` / `sock`, or `name` for a nameplate sheet.
    /// Derived from the raster's own name (a jersey bundle's `font_*` raster is piece `font`), so
    /// the body and the number font group apart.
    pub piece: String,
    pub label: String,
    pub file_name: String,
    /// A `name_*` sheet: a fully-opaque template of marker zones (name / number / patch positions),
    /// not a paintable texture — the game reads only *where* the zones are. Drives the nameplate
    /// (name/number/patch location) editor instead of a plain image replace.
    pub is_nameplate: bool,
}

/// The map-suffix codes an outfit raster stem can end in, mapped to a human word for the label.
fn outfit_map_word(stem: &str) -> Option<&'static str> {
    match stem.rsplit('_').next()? {
        "cm" => Some("color map"),
        "nm" => Some("normal map"),
        "sm" => Some("spec map"),
        "am" => Some("alpha map"),
        "lm" => Some("light map"),
        _ => None,
    }
}

/// Turn a piece + id-bearing stem into a readable, team-scoped descriptor: drop the team token and
/// the leading piece word, translate the trailing map code, and keep the set/variant digits.
/// `jersey_0_10_0_cm` (team 10 at idx 2) → `Jersey · set 0 · variant 0 · color map`.
fn outfit_label(piece: &str, stem: &str, team_idx: usize, team_id: u64) -> String {
    let mut toks: Vec<&str> = stem.split('_').collect();
    if toks.get(team_idx).and_then(|t| t.parse::<u64>().ok()) == Some(team_id) {
        toks.remove(team_idx);
    }
    // Drop a leading piece word (`jersey`/`pant`/`sock`/`texlib`) — it's the group heading already.
    if toks.first().is_some_and(|t| *t == piece || *t == "texlib") {
        toks.remove(0);
    }
    let map = outfit_map_word(stem);
    if map.is_some() {
        toks.pop(); // the map code is re-added as a word
    }
    let piece_title = {
        let mut c = piece.chars();
        c.next()
            .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
            .unwrap_or_default()
    };
    let digits = toks.join(" · ");
    match (digits.is_empty(), map) {
        (true, Some(m)) => format!("{piece_title} · {m}"),
        (true, None) => piece_title,
        (false, Some(m)) => format!("{piece_title} · {digits} · {m}"),
        (false, None) => format!("{piece_title} · {digits}"),
    }
}

/// List one team's outfit texture files (jersey/pants/socks — plus PS4 `font` sheets) from the
/// install's `rendering/` tree, for the per-team Uniforms surface. Same discovery as
/// [`export_team_outfits`] but targeted at one `team_id` and returning the source files (so the
/// caller can preview/replace them in place) rather than decoding to PNG.
///
/// `teams` is the full resolved roster (needed to derive the team-id token position); the result is
/// filtered to `team_id`. Sorted by piece then filename for a stable UI. Empty when this title ships
/// no team-keyed outfit textures.
pub fn list_team_outfits(
    rendering_root: &Path,
    teams: &[OutfitTeam],
    team_id: u64,
) -> Vec<OutfitTexture> {
    // PS4 kettle layout: outfit GNFs live under `characters/player`, filename-keyed by team name.
    let ps4_player = rendering_root.join("characters").join("player");
    if ps4_player.is_dir() {
        return list_ps4_team_outfits(&ps4_player, teams, team_id, false);
    }

    // The requested team's own filename tokens: its registered jersey art ids, else its team id.
    // Matching runs on the token SET — not the exclusive token→team attribution — because tokens
    // are shared across teams: Winter Classic teams register kits under their parent club's art id
    // (verified: Legacy team 216 "NY Rangers WC" registers 1_19_3 — art 19 is the Rangers'), so
    // the same files must resolve for both teams.
    let my_tokens: Vec<u64> = match teams.iter().find(|t| t.team_id == team_id) {
        Some(t) if !t.art_ids.is_empty() => t.art_ids.clone(),
        _ => vec![team_id],
    };
    let mut out: Vec<OutfitTexture> = Vec::new();
    for w in collect_outfit_work(rendering_root, teams)
        .into_iter()
        .filter(|w| my_tokens.contains(&w.token))
    {
        let file_name = w
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| w.file_stem.clone());
        outfit_entries_for_file(&w, &file_name, &mut out);
    }
    out.sort_by(|a, b| {
        (a.piece.as_str(), &a.file_name, a.texture_index).cmp(&(
            b.piece.as_str(),
            &b.file_name,
            b.texture_index,
        ))
    });
    out
}

/// Expand one outfit file into per-raster [`OutfitTexture`] entries, appended to `out`. A RW4 texlib
/// jersey bundle yields separate entries for each of its **colour-map** rasters — the body and the
/// number `font` — so both preview and replace independently; a `name_*` sheet or an unparseable file
/// yields a single entry (the whole file, primary raster). Non-colour rasters (spec/normal maps) are
/// skipped. The file is read only here (per matched team file), not during the folder scan.
fn outfit_entries_for_file(w: &OutfitWork, file_name: &str, out: &mut Vec<OutfitTexture>) {
    let single = |idx: u32, piece: String, is_nameplate: bool| OutfitTexture {
        abs_path: w.path.clone(),
        texture_index: idx,
        label: outfit_label(&piece, &w.file_stem, w.team_idx, w.team_id),
        piece,
        file_name: file_name.to_string(),
        is_nameplate,
    };
    // A `name_*` sheet is the nameplate template (marker zones), regardless of internal raster names.
    if w.file_stem.starts_with("name") {
        out.push(single(0, "name".to_string(), true));
        return;
    }
    // Parse the bundle and emit one entry per raster — colour map, spec, normal and the number `font`
    // maps alike — so every texture is exposed for replacement (not just the visible colour maps).
    // Piece is taken from the raster's own name (`font_*` → font, `jersey_*` → jersey); the label
    // carries the full raster stem (with its `_cm`/`_sm`/`_nm` suffix) so the map type reads clearly.
    // Falls back to a single folder-tagged entry when the file doesn't parse as a texlib (e.g. a PS4
    // GNF, or the empty fixtures in tests).
    match read_asset_file(&w.path).and_then(|b| {
        if b.starts_with(tdb_rx2::texlib_ps3::MAGIC) {
            tdb_rx2::texlib_ps3::parse(&b)
                .map(|lib| lib.textures.into_iter().map(|t| t.name).collect::<Vec<_>>())
                .map_err(|e| DomainError::Extract(format!("PS3 texlib parse: {e}")))
        } else {
            tdb_rx2::texlib::parse(&b)
                .map(|lib| lib.textures.into_iter().map(|t| t.name).collect::<Vec<_>>())
                .map_err(|e| DomainError::Extract(format!("texlib parse: {e}")))
        }
    }) {
        Ok(names) if !names.is_empty() => {
            for (i, name) in names.iter().enumerate() {
                // Early single-raster files have no embedded name. The parser's
                // synthetic name loses the font/jersey and colour/normal identity;
                // use the descriptive filename only when it denotes one raster.
                let stem = if names.len() == 1 && name == "texture_0000" {
                    w.file_stem.as_str()
                } else {
                    raster_name_stem(name)
                };
                let piece = raster_piece(stem, w.folder);
                out.push(OutfitTexture {
                    abs_path: w.path.clone(),
                    texture_index: i as u32,
                    label: outfit_label(&piece, stem, w.team_idx, w.team_id),
                    piece,
                    file_name: file_name.to_string(),
                    is_nameplate: false,
                });
            }
        }
        // Unparseable or empty (PS4 GNF, test fixtures) — surface a single entry so nothing drops.
        _ => out.push(single(0, raster_piece(&w.file_stem, w.folder), false)),
    }
}

/// The outfit piece a texlib raster belongs to, from its own name (`font_0_10_0_cm` → `font`,
/// `jersey_0_10_0_cm` → `jersey`), falling back to the containing `folder` for anything unrecognised.
fn raster_piece(raster_name: &str, folder: &str) -> String {
    match raster_name_stem(raster_name).split('_').next() {
        Some("font") => "font",
        Some("jersey") => "jersey",
        Some("name") => "name",
        Some("pant") => "pant",
        Some("sock") => "sock",
        _ => folder,
    }
    .to_string()
}

/// PS4 per-team outfit listing: GNFs under `characters/player`, resolved to a team by the same slug
/// resolver the export uses. Returns the files whose resolved team matches this team's slug.
fn list_ps4_team_outfits(
    player_dir: &Path,
    teams: &[OutfitTeam],
    team_id: u64,
    include_ambiguous: bool,
) -> Vec<OutfitTexture> {
    let Some(team) = teams.iter().find(|t| t.team_id == team_id) else {
        return Vec::new();
    };
    let resolver = build_ps4_team_resolver(teams);
    // Explicit browsing mode only: expose ambiguous filename candidates without
    // assigning ownership or changing the exporter's conservative resolver.
    let candidate_resolver = build_ps4_team_resolver(std::slice::from_ref(team));
    let mut out: Vec<OutfitTexture> = Vec::new();
    let Ok(read) = std::fs::read_dir(player_dir) else {
        return out;
    };
    for ent in read.flatten() {
        let fname = ent.file_name().to_string_lossy().into_owned();
        let Some(stem) = fname.strip_suffix(".gnf") else {
            continue;
        };
        let Some(piece) = PS4_OUTFIT_TYPES
            .iter()
            .find(|t| stem.starts_with(&format!("{t}_")))
            .copied()
        else {
            continue;
        };
        // The resolved output stem leads with the team slug when it matched a DB team.
        let matched = ps4_output_stem(stem, &resolver);
        let candidate = matched.is_none() && include_ambiguous;
        let Some(resolved) = matched.or_else(|| candidate.then(|| ps4_output_stem(stem, &candidate_resolver)).flatten()) else {
            continue;
        };
        // Include the piece delimiter: `anaheim_ducks_alumni_jersey_…`
        // must not match the shorter `anaheim_ducks` team name.
        if !resolved.starts_with(&format!("{}_{}_", team.slug, piece)) {
            continue;
        }
        out.push(OutfitTexture {
            texture_index: 0, // PS4 GNFs are single-texture files
            label: format!("{}{}", if candidate { "Unverified filename match · " } else { "" }, ps4_outfit_label(piece, stem, &team.slug)),
            piece: piece.to_string(),
            file_name: fname,
            is_nameplate: false, // PS4 font sheets are handled as textures; no nameplate zones yet
            abs_path: ent.path(),
        });
    }
    out.sort_by(|a, b| (a.piece.as_str(), &a.file_name).cmp(&(b.piece.as_str(), &b.file_name)));
    out
}

/// User-requested PS4 candidate browsing. Returned files are editable by their
/// explicit paths, but ambiguous names are not evidence of a team/kit binding.
pub fn list_team_outfit_candidates(rendering_root: &Path, teams: &[OutfitTeam], team_id: u64) -> Vec<OutfitTexture> {
    let player = rendering_root.join("characters/player");
    if player.is_dir() { list_ps4_team_outfits(&player, teams, team_id, true) }
    else { list_team_outfits(rendering_root, teams, team_id) }
}

/// PS4 label: strip the team words the resolver matched, keep era/variant/map. Falls back to the raw
/// stem when nothing strips cleanly.
fn ps4_outfit_label(piece: &str, stem: &str, _slug: &str) -> String {
    let piece_title = {
        let mut c = piece.chars();
        c.next()
            .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
            .unwrap_or_default()
    };
    // Keep every token after the leading piece word; the team words stay (PS4 uses city names, which
    // read fine), and the trailing map code is translated.
    let mut toks: Vec<&str> = stem.split('_').collect();
    if toks.first().is_some_and(|t| *t == piece) {
        toks.remove(0);
    }
    let map = outfit_map_word(stem);
    if map.is_some() {
        toks.pop();
    }
    let rest = toks.join(" · ");
    match (rest.is_empty(), map) {
        (true, Some(m)) => format!("{piece_title} · {m}"),
        (true, None) => piece_title,
        (false, Some(m)) => format!("{piece_title} · {rest} · {m}"),
        (false, None) => format!("{piece_title} · {rest}"),
    }
}

/// Fan `work` out across the machine's cores, folding each worker's tally into one. `process` decodes
/// and writes a single item into a thread-local summary; the merge is done once at the end.
fn run_outfit_workers<T: Sync>(
    work: &[T],
    process: impl Fn(&T, &mut OutfitExportSummary) + Sync,
) -> OutfitExportSummary {
    if work.is_empty() {
        return OutfitExportSummary::default();
    }
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(work.len());
    let chunk = work.len().div_ceil(threads);
    let process = &process;
    let partials: Vec<OutfitExportSummary> = std::thread::scope(|scope| {
        let handles: Vec<_> = work
            .chunks(chunk)
            .map(|items| {
                scope.spawn(move || {
                    let mut s = OutfitExportSummary::default();
                    for it in items {
                        process(it, &mut s);
                    }
                    s
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_default())
            .collect()
    });
    let mut merged = OutfitExportSummary::default();
    for p in partials {
        merged.merge(p);
    }
    merged
}

// ---------------------------------------------------------------- PS4 (kettle) outfits

/// The outfit texture types PS4 keeps under `characters/player`, and the summary bucket each feeds.
/// Files are `{type}_{era}_{team}_{variant}_{map}.gnf` — jersey/pant/sock plus the number/name
/// `font` sheet (grouped like the RenderWare export).
const PS4_OUTFIT_TYPES: &[&str] = &["jersey", "pant", "sock", "font"];

/// Normalize a slug for loose matching: drop every non-alphanumeric and lowercase. `new_jersey` and
/// PS4's squished `newjersey` both become `newjersey`.
fn normalize_slug(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Build the PS4 team resolver: a map from a normalized filename slug to the team's full name slug
/// (`detroit_red_wings`). Keys are every leading-word prefix of the name (so PS4's city-only
/// `detroit` / `losangeles` resolve) plus the team abbreviation (so `nyr` / `nyi` resolve). A key
/// that would point at two different teams is dropped as ambiguous rather than guessing.
fn build_ps4_team_resolver(teams: &[OutfitTeam]) -> HashMap<String, String> {
    let mut idx: HashMap<String, Option<String>> = HashMap::new();
    let mut add = |key: String, full: &str| {
        if key.is_empty() {
            return;
        }
        idx.entry(key)
            .and_modify(|e| {
                if e.as_deref() != Some(full) {
                    *e = None;
                }
            })
            .or_insert_with(|| Some(full.to_string()));
    };
    for t in teams {
        let words: Vec<&str> = t.slug.split('_').filter(|w| !w.is_empty()).collect();
        for k in 1..=words.len() {
            add(normalize_slug(&words[..k].join("_")), &t.slug);
        }
        add(normalize_slug(&t.abbrev), &t.slug);
    }
    idx.into_iter()
        .filter_map(|(k, v)| v.map(|full| (k, full)))
        .collect()
}

/// One PS4 GNF outfit file staged for decode: its path, the type folder it belongs in, and the output
/// name to write (team resolved to the DB name where possible, era/variant/map kept intact).
struct Ps4OutfitWork {
    path: PathBuf,
    folder: &'static str,
    out_stem: String,
}

/// Rename a PS4 outfit stem to lead with the resolved team, keeping the rest (era, variant, map). The
/// team token-window is found by testing each short run of tokens against the resolver — so no era or
/// variant vocabulary is hard-coded, and the *variant name* the caller asked to keep survives
/// untouched. `jersey_rbk_detroit_home_cm` → `detroit_red_wings_jersey_rbk_home_cm`. Returns `None`
/// when no window resolves to a team in this DB (a club the loaded database doesn't carry — the file
/// is then skipped, matching the RenderWare path which only exports the DB's own teams).
fn ps4_output_stem(stem: &str, resolver: &HashMap<String, String>) -> Option<String> {
    let toks: Vec<&str> = stem.split('_').collect();
    // Search token windows (after the type at index 0), preferring the longest that resolves.
    let mut best: Option<(usize, usize, &String)> = None;
    for start in 1..toks.len() {
        for end in (start + 1)..=toks.len().min(start + 3) {
            if let Some(full) = resolver.get(&normalize_slug(&toks[start..end].join("_"))) {
                if best.is_none_or(|(s, e, _)| (end - start) > (e - s)) {
                    best = Some((start, end, full));
                }
            }
        }
    }
    best.map(|(start, end, full)| {
        let rest: Vec<&str> = toks[..start].iter().chain(&toks[end..]).copied().collect();
        format!("{full}_{}", rest.join("_"))
    })
}

/// Export PS4 outfit GNFs from `player_dir`. Files are already team- and variant-named; we decode the
/// colour maps to PNG and lead each name with the resolved DB team. GNF maps we can't yet decode
/// correctly (BC4/BC5 normal/coefficient maps) are skipped silently rather than flooding failures.
fn export_ps4_outfits(
    player_dir: &Path,
    teams: &[OutfitTeam],
    out_dir: &Path,
) -> Result<OutfitExportSummary> {
    let resolver = build_ps4_team_resolver(teams);

    let mut work: Vec<Ps4OutfitWork> = Vec::new();
    if let Ok(read) = std::fs::read_dir(player_dir) {
        for ent in read.flatten() {
            let fname = ent.file_name().to_string_lossy().into_owned();
            if !fname.ends_with(".gnf") {
                continue;
            }
            let stem = fname.trim_end_matches(".gnf");
            let Some(folder) = PS4_OUTFIT_TYPES
                .iter()
                .find(|t| stem.starts_with(&format!("{t}_")))
                .copied()
            else {
                continue; // not a jersey/pant/sock/font texture
            };
            // Resolve to the DB team name when we can; otherwise keep the file's native name (a club
            // this DB doesn't carry — AHL/CHL/European) so it's still exported, just under its own
            // descriptive slug rather than a resolved team.
            let out_stem = ps4_output_stem(stem, &resolver).unwrap_or_else(|| stem.to_string());
            work.push(Ps4OutfitWork {
                path: ent.path(),
                folder,
                out_stem,
            });
        }
    }

    let mut summary = run_outfit_workers(&work, |item, s| {
        let bytes = match read_asset_file(&item.path) {
            Ok(b) => b,
            Err(_) => return,
        };
        let Ok(tex) = tdb_rx2::gnf::parse(&bytes) else {
            return;
        };
        // BC4/BC5 (normal / coefficient) decode isn't correct yet — skip quietly.
        let Ok(rgba) = gnf_to_rgba(&tex) else {
            return;
        };
        let png = match (DecodedImage {
            width: tex.width,
            height: tex.height,
            rgba,
        })
        .to_png_fast()
        {
            Ok(p) => p,
            Err(e) => {
                s.failures.push(format!("{}: {e}", item.out_stem));
                return;
            }
        };
        let dst = out_dir
            .join(item.folder)
            .join(format!("{}.png", item.out_stem));
        match write_file(&dst, &png) {
            Ok(n) => {
                match item.folder {
                    "pant" => s.pants_written += 1,
                    "sock" => s.socks_written += 1,
                    // jersey + font are both jersey-side art.
                    _ => s.jerseys_written += 1,
                }
                s.bytes_written += n;
            }
            Err(e) => s.failures.push(format!("{}: {e}", item.out_stem)),
        }
    });
    summary.teams_considered = teams.len();
    summary.rendering_root = player_dir.to_string_lossy().into_owned();
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mip_levels_cover_the_full_chain_with_halving_dims() {
        // 64×32 DXT5 → full_mip_count(64,32)=7 levels: 64x32,32x16,16x8,8x4,4x2,2x1,1x1.
        let (w, h) = (64u32, 32u32);
        let rgba = vec![0x80u8; (w * h * 4) as usize];
        let levels = compress_bc_mip_levels(tdb_archive_codec::DdsFormat::Dxt5, w, h, &rgba);
        assert_eq!(
            levels.len() as u32,
            full_mip_count(w, h),
            "one entry per mip level"
        );

        // Dims halve each level (min 1) and each level's payload is the exact BC block size.
        let bpb = 16u32; // DXT5 = 16 bytes / 4×4 block
        for (i, (mw, mh, data)) in levels.iter().enumerate() {
            assert_eq!(*mw, (w >> i).max(1), "level {i} width");
            assert_eq!(*mh, (h >> i).max(1), "level {i} height");
            let expected = mw.div_ceil(4) * mh.div_ceil(4) * bpb;
            assert_eq!(data.len() as u32, expected, "level {i} block-stream size");
        }
        // The base level is NOT all-zero for a non-zero source — i.e. a real chain, not padding.
        assert!(
            levels[0].2.iter().any(|&b| b != 0),
            "base mip must carry real block data"
        );

        // The contiguous chain a DDS writer emits is the levels concatenated.
        let chain: Vec<u8> = levels.iter().flat_map(|(_, _, d)| d.iter().copied()).collect();
        assert_eq!(
            chain.len(),
            levels.iter().map(|(_, _, d)| d.len()).sum::<usize>()
        );
    }

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
        assert!(is_loose_raster_for(
            Path::new("head_10000_0_0_0_cm.rx2"),
            10000
        ));
        assert!(is_loose_raster_for(
            Path::new("head_10000_0_0_0_nm.rx2"),
            10000
        ));
        assert!(!is_loose_raster_for(
            Path::new("hair_10000_0_0_0.rx2"),
            10000
        )); // the hair mesh
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

    /// The header framing (a) drops a faded bottom edge below the lowest solid
    /// row, and (b) centres the highest point (crown) horizontally by padding.
    #[test]
    fn solid_bottom_crop_drops_fade_and_centres_crown() {
        // 8×8. A head whose crown (topmost pixel) sits left of centre, widening to
        // a solid bottom row, with a faded row beneath it.
        let (w, h) = (8usize, 8usize);
        let mut rgba = vec![0u8; w * h * 4];
        let put = |rgba: &mut [u8], x: usize, y: usize, a: u8| {
            let i = (y * w + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[200, 100, 50, a]);
        };
        put(&mut rgba, 2, 1, 255); // crown (highest point), off-centre-left
        for x in 1..=3 {
            put(&mut rgba, x, 2, 255);
        }
        for x in 0..=6 {
            put(&mut rgba, x, 3, 255); // solid bottom (widest)
            put(&mut rgba, x, 4, 100); // fade below — must be dropped
        }
        let out = DecodedImage {
            width: 8,
            height: 8,
            rgba,
        }
        .cropped_to_solid_bottom(8);

        // Rows 1–3 kept (fade row 4 dropped) → height 3; no partial-alpha survives.
        assert_eq!(out.height, 3, "fade row dropped");
        assert!(
            out.rgba.chunks_exact(4).all(|p| p[3] == 0 || p[3] == 255),
            "no faded pixel survives (only opaque content + transparent padding)"
        );
        // The crown (top row's only opaque pixel) sits at the horizontal centre.
        let ow = out.width as usize;
        let centre = ow / 2;
        let top_alpha = |x: usize| out.rgba[x * 4 + 3];
        assert_eq!(top_alpha(centre), 255, "crown is centred");
        assert_eq!(
            top_alpha(0),
            0,
            "left edge of top row is transparent padding/space"
        );
        assert_eq!(top_alpha(ow - 1), 0, "right edge of top row is transparent");
    }

    /// With no solid pixel anywhere (all partial alpha), the header crop falls
    /// back to the plain opaque-bounds crop rather than returning nothing.
    #[test]
    fn solid_bottom_crop_falls_back_without_solids() {
        let mut rgba = vec![0u8; 4 * 4 * 4];
        let i = (4 + 1) * 4;
        rgba[i..i + 4].copy_from_slice(&[10, 20, 30, 120]); // one partial pixel
        let out = DecodedImage {
            width: 4,
            height: 4,
            rgba,
        }
        .cropped_to_solid_bottom(8);
        assert_eq!((out.width, out.height), (1, 1)); // fell back to opaque-bounds crop
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

        let roots = AssetRoots::resolve(&tmp, Platform::Xbox);
        assert_eq!(roots.rendering, game.join("rendering").to_string_lossy());
        assert_eq!(roots.fe, game.join("fe").to_string_lossy());

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn ps3_asset_roots_accept_disc_game_and_usrdir_folders() {
        let tmp = std::env::temp_dir().join("nhlms_ps3_disc_asset_roots");
        let usrdir = tmp.join("PS3_GAME/USRDIR");
        std::fs::create_dir_all(&usrdir).unwrap();
        std::fs::write(usrdir.join("EBOOT.BIN"), b"stub").unwrap();
        for picked in [&tmp, &tmp.join("PS3_GAME"), &usrdir] {
            let roots = AssetRoots::resolve(picked, Platform::Ps3);
            assert_eq!(Path::new(&roots.rendering), usrdir.join("rendering"));
            assert_eq!(Path::new(&roots.fe), usrdir.join("fe"));
        }
        std::fs::remove_dir_all(tmp).unwrap();
    }

    /// NHL Legacy Recompiled nests its art under `_compiled/` beside the `.xex`.
    /// When that folder exists, the roots must point inside it; the plain
    /// `<anchor>/rendering` would miss the art entirely. Caught on a real recomp
    /// install whose meshes live in `…\game\_compiled\rendering`.
    #[test]
    fn asset_roots_prefer_the_compiled_folder_when_present() {
        let tmp = std::env::temp_dir().join("nhlms_anchor_test_compiled");
        let game = tmp.join("game");
        let compiled = game.join("_compiled");
        std::fs::create_dir_all(compiled.join("rendering")).unwrap();
        std::fs::create_dir_all(compiled.join("fe")).unwrap();
        std::fs::write(game.join("default.xex"), b"stub").unwrap();

        let roots = AssetRoots::resolve(&tmp, Platform::Xbox);
        assert_eq!(
            roots.rendering,
            compiled.join("rendering").to_string_lossy()
        );
        assert_eq!(roots.fe, compiled.join("fe").to_string_lossy());

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// Xbox: the picked root *is* the executable's folder.
    #[test]
    fn asset_roots_anchor_on_the_root_when_the_exe_is_there() {
        let tmp = std::env::temp_dir().join("nhlms_anchor_test_xbox");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("default.xex"), b"stub").unwrap();

        let roots = AssetRoots::resolve(&tmp, Platform::Xbox);
        assert_eq!(roots.rendering, tmp.join("rendering").to_string_lossy());

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// PS3 has no `.xex`; the install dir (`USRDIR`) holds the art directly.
    #[test]
    fn asset_roots_fall_back_to_the_install_dir_without_an_exe() {
        let tmp = std::env::temp_dir().join("nhlms_anchor_test_ps3");
        std::fs::create_dir_all(&tmp).unwrap();

        let roots = AssetRoots::resolve(&tmp, Platform::Xbox);
        assert_eq!(roots.rendering, tmp.join("rendering").to_string_lossy());

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// The GNF de-tile + BC1 decode reconstruct a real head cm map to a
    /// plausible image (varied, skin-toned, not black/uniform). Guards the
    /// 1D-micro de-tile against regressions. Skipped when the tree isn't mounted.
    #[test]
    fn ps4_gnf_head_cm_decodes_to_a_plausible_image() {
        let path = r"H:\Emulators\ShadPS4\Games\CUSA00561\databigs\Extracted\nhl\data\kettle\rendering\characters\player\head_alexander_ovechkin_cm.gnf";
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("skipping: {path} not present");
            return;
        };
        let tex = tdb_rx2::gnf::parse(&bytes).expect("gnf parses");
        let rgba = gnf_to_rgba(&tex).expect("cm decodes");
        assert_eq!(rgba.len(), (tex.width * tex.height * 4) as usize);

        // Not uniform: a scrambled de-tile would still vary, but a broken decode
        // tends toward black/constant. Require real spread and some skin tones.
        let opaque: Vec<&[u8]> = rgba.chunks_exact(4).filter(|p| p[3] > 8).collect();
        assert!(opaque.len() > 1000, "should have opaque pixels");
        let mean_r: u32 = opaque.iter().map(|p| p[0] as u32).sum::<u32>() / opaque.len() as u32;
        let mean_g: u32 = opaque.iter().map(|p| p[1] as u32).sum::<u32>() / opaque.len() as u32;
        let mean_b: u32 = opaque.iter().map(|p| p[2] as u32).sum::<u32>() / opaque.len() as u32;
        // Skin-dominated atlas: red channel clearly the strongest.
        assert!(
            mean_r > mean_g && mean_g > mean_b,
            "skin tone: r>g>b, got {mean_r},{mean_g},{mean_b}"
        );
        assert!(mean_r > 90, "not too dark: mean_r={mean_r}");

        // Seamless de-tile: rows on a 32-texel micro-tile boundary must be no
        // more discontinuous than interior rows. A wrong within-tile order
        // (e.g. row-major instead of Z-order) leaves ~2× the gradient at every
        // boundary, which reads as washboard ripples on the lit 3D head.
        let (w, h) = (tex.width as usize, tex.height as usize);
        let luma = |x: usize, y: usize| {
            let o = (y * w + x) * 4;
            0.299 * rgba[o] as f64 + 0.587 * rgba[o + 1] as f64 + 0.114 * rgba[o + 2] as f64
        };
        let (mut b, mut bn, mut i, mut inn) = (0.0, 0usize, 0.0, 0usize);
        for y in 1..h {
            let boundary = y % 32 == 0;
            for x in 0..w {
                let d = (luma(x, y) - luma(x, y - 1)).abs();
                if boundary {
                    b += d;
                    bn += 1;
                } else {
                    i += d;
                    inn += 1;
                }
            }
        }
        let seam_ratio = (b / bn as f64) / (i / inn as f64);
        assert!(
            seam_ratio < 1.6,
            "tile-boundary seam ratio too high: {seam_ratio:.2}"
        );
    }

    #[test]
    fn ps4_name_slug_normalizes_names() {
        assert_eq!(
            ps4_name_slug("Alexander", "Ovechkin").as_deref(),
            Some("alexander_ovechkin")
        );
        // Punctuation dropped, spaces collapsed.
        assert_eq!(ps4_name_slug("T.J.", "Oshie").as_deref(), Some("tj_oshie"));
        assert_eq!(
            ps4_name_slug("Jean-Gabriel", "Pageau").as_deref(),
            Some("jeangabriel_pageau")
        );
        assert_eq!(ps4_name_slug("  ", "").as_deref(), None);
    }

    #[test]
    fn ps4_texture_match_keys_on_the_whole_slug() {
        let slug = "alexander_ovechkin";
        assert!(ps4_is_texture_for(
            Path::new("head_alexander_ovechkin_cm.gnf"),
            slug
        ));
        assert!(ps4_is_texture_for(
            Path::new("hair_alexander_ovechkin_coeff.gnf"),
            slug
        ));
        // Wrong prefix, wrong extension, unknown map, and a longer slug all miss.
        assert!(!ps4_is_texture_for(
            Path::new("body_alexander_ovechkin_cm.gnf"),
            slug
        ));
        assert!(!ps4_is_texture_for(
            Path::new("head_alexander_ovechkin_cm.dds"),
            slug
        ));
        assert!(!ps4_is_texture_for(
            Path::new("head_alexander_ovechkin_xx.gnf"),
            slug
        ));
        assert!(!ps4_is_texture_for(
            Path::new("head_alexander_ovechkinov_cm.gnf"),
            slug
        ));
    }

    /// PS4 resolves art off the kettle tree, derived by walking up from the
    /// (deep) database dir to `nhl/data`; meshes are name-slug keyed under
    /// `characters/player/`, with a numeric `generic_player_*` fallback.
    #[test]
    fn ps4_asset_roots_resolve_kettle_tree_and_bespoke_mesh() {
        let tmp = std::env::temp_dir().join(format!("nhlms_ps4_roots_{}", std::process::id()));
        let game = tmp.join("Games").join("CUSA00561");
        let nhl_data = game.join("nhl").join("data");
        let player = nhl_data
            .join("kettle")
            .join("rendering")
            .join("characters")
            .join("player");
        std::fs::create_dir_all(&player).unwrap();
        // Portraits live under <nhl>/ui/assets/portraits (sibling of nhl/data).
        let portraits = game.join("nhl").join("ui").join("assets").join("portraits");
        std::fs::create_dir_all(&portraits).unwrap();
        std::fs::write(portraits.join("p4242.png"), b"\x89PNG stub").unwrap();
        // The dir discovery hands us is deep inside nhl/data.
        let db_dir = nhl_data
            .join("common")
            .join("gamemodes")
            .join("database")
            .join("t3db");
        std::fs::create_dir_all(&db_dir).unwrap();
        // A bespoke head + its GNF maps, and a shared pool mesh.
        std::fs::write(player.join("alexander_ovechkin.mcd"), b"RSF\0").unwrap();
        std::fs::write(player.join("head_alexander_ovechkin_cm.gnf"), b"GNF ").unwrap();
        std::fs::write(player.join("hair_alexander_ovechkin_cm.gnf"), b"GNF ").unwrap();
        std::fs::write(player.join("generic_player_60046.mcd"), b"RSF\0").unwrap();

        let roots = AssetRoots::resolve(&db_dir, Platform::Ps4);
        assert_eq!(
            roots.rendering,
            nhl_data.join("kettle").join("rendering").to_string_lossy()
        );
        assert_eq!(roots.fe, game.join("nhl").join("ui").to_string_lossy());
        assert!(roots.complete(), "characters/player exists");

        // Bespoke: slug mesh wins.
        let slug = Some("alexander_ovechkin");
        assert_eq!(
            roots.mesh_path(99, slug),
            Some(player.join("alexander_ovechkin.mcd"))
        );
        // Its two GNF maps resolve.
        let texs = roots.texture_paths(99, slug);
        assert_eq!(texs.len(), 2, "head + hair cm maps");
        // Pool head: no slug mesh → numeric generic.
        assert_eq!(
            roots.mesh_path(60046, Some("nobody_here")),
            Some(player.join("generic_player_60046.mcd"))
        );
        // No slug and no generic → None.
        assert_eq!(roots.mesh_path(12345, None), None);

        // Portrait: flat p{id}.png, large-only (no HUD-size set).
        assert_eq!(
            roots.portrait_path(4242, PORTRAIT_DIR_LARGE),
            Some(portraits.join("p4242.png"))
        );
        assert_eq!(roots.portrait_path(4242, PORTRAIT_DIR_SMALL), None);
        assert_eq!(roots.portrait_path(9999, PORTRAIT_DIR_LARGE), None);

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// Regression (NHL 19, CUSA11126): the loose db lives at `<game>/nhl/data` but the
    /// art was extracted to a *different* tree, `<game>/databigs/Extracted/nhl/data`.
    /// Anchoring the art root on the db's ancestor names an art-less path; resolution
    /// must instead find the `nhl/data` that actually holds `kettle/rendering`.
    #[test]
    fn ps4_asset_roots_follow_extracted_art_in_a_separate_tree() {
        let tmp = std::env::temp_dir().join(format!("nhlms_ps4_split_{}", std::process::id()));
        let game = tmp.join("Games").join("CUSA11126");
        // Loose db tree (no art here — kettle has loc but no rendering).
        let db_dir = game
            .join("nhl")
            .join("data")
            .join("common")
            .join("gamemodes")
            .join("database")
            .join("t3db");
        std::fs::create_dir_all(&db_dir).unwrap();
        std::fs::create_dir_all(game.join("nhl").join("data").join("kettle").join("loc")).unwrap();
        // Extracted art tree, a sibling under databigs/Extracted.
        let art_data = game
            .join("databigs")
            .join("Extracted")
            .join("nhl")
            .join("data");
        let player = art_data
            .join("kettle")
            .join("rendering")
            .join("characters")
            .join("player");
        std::fs::create_dir_all(&player).unwrap();
        std::fs::create_dir_all(art_data.parent().unwrap().join("ui")).unwrap();

        // Resolving from the deep db dir must follow the art into the Extracted tree.
        let roots = AssetRoots::resolve(&db_dir, Platform::Ps4);
        assert_eq!(
            roots.rendering,
            art_data.join("kettle").join("rendering").to_string_lossy(),
            "art root anchors on the tree that holds kettle/rendering, not the db's tree"
        );
        assert!(
            roots.complete(),
            "characters/player under the Extracted tree is found"
        );

        // Resolving from the game root (what discovery's extract-needed check uses) too.
        let from_root = AssetRoots::resolve(&game, Platform::Ps4);
        assert!(
            from_root.complete(),
            "game-root resolution also finds the extracted art"
        );

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// Live regression against the operator's real NHL 19 install (CUSA11126),
    /// whose loose db and extracted art sit in *different* `nhl/data` trees.
    /// Skipped when the drive isn't mounted. Resolves from the deep loose-db
    /// folder exactly as discovery hands it to us.
    #[test]
    fn ps4_real_nhl19_resolves_extracted_art_in_separate_tree() {
        let game = Path::new(r"H:\Emulators\ShadPS4\Games\CUSA11126");
        if !game
            .join("databigs/Extracted/nhl/data/kettle/rendering")
            .is_dir()
        {
            eprintln!("skipping: real NHL19 extracted art not present");
            return;
        }
        let db_dir = game.join(r"nhl\data\common\gamemodes\database\t3db");
        let roots = AssetRoots::resolve(&db_dir, Platform::Ps4);
        assert!(
            roots.complete(),
            "NHL19 extracted art must resolve despite db/art living in split trees"
        );
    }

    /// A real PS4 portrait PNG decodes through the shared `decode_portrait`
    /// entry point. Skipped when the extracted tree isn't mounted.
    #[test]
    fn ps4_portrait_png_decodes() {
        let path = r"H:\Emulators\ShadPS4\Games\CUSA00561\databigs\Extracted\nhl\ui\assets\portraits\p10000.png";
        if !Path::new(path).is_file() {
            eprintln!("skipping: {path} not present");
            return;
        }
        let img = decode_portrait(Path::new(path)).expect("ps4 portrait png decodes");
        assert!(img.width > 0 && img.height > 0);
        assert_eq!(img.rgba.len(), (img.width * img.height * 4) as usize);
    }

    #[test]
    fn asset_roots_overrides_ignore_empty_strings() {
        let roots = AssetRoots::resolve(Path::new("/install"), Platform::Xbox);
        let rendering = roots.rendering.clone();
        let overridden = roots.with_overrides(Some("   "), Some("/custom/fe"));
        assert_eq!(overridden.rendering, rendering);
        assert_eq!(overridden.fe, "/custom/fe");
    }

    /// Generate + drift-guard the NHL Legacy Xbox 360 **team-logo identity map**
    /// (`docs/audits/data/assets/nhllegacy/xbox360/identities.csv`) from the
    /// committed anchor, and verify (hard-rule-3) that the on-disk logo texlib id
    /// is `kOtt.lOnr` (`artid`) — not the record index / team key.
    ///
    /// Evidence: the team resolving to "Red Wings" carries `lOnr == 10` (matching
    /// the eyeballed `texlib_10` → Red Wings wordmark), and the artid space is
    /// sparse and reaches far past the 252 team records (max > 20000 — an index
    /// could never), the same high range the on-disk `texlib_{id}` files occupy.
    ///
    /// Reads only `anchors/`, so it runs in CI. `QA_WRITE=1` rewrites the CSV;
    /// otherwise it asserts the committed file is byte-identical.
    #[test]
    fn legacy_x360_team_logo_identity_map() {
        use tdb_core::TdbFile;
        use tdb_defs::FieldMetaIndex;

        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo = manifest.join("..").join("..");
        let base = repo
            .join("anchors")
            .join("xbox360")
            .join("nhllegacy")
            .join("databases");
        let db_path = base.join("nhlng.db");
        if !db_path.is_file() {
            eprintln!("skipping: {} not present", db_path.display());
            return;
        }
        let bytes = std::fs::read(&db_path).expect("read anchor");
        let db = TdbFile::parse(&bytes).expect("parse anchor");
        let meta = FieldMetaIndex::from_xml(
            &std::fs::read_to_string(base.join("nhlng-meta.xml")).expect("meta"),
        )
        .expect("parse meta");
        let kott = db.table_canonical("kOtt").expect("team table");
        let f_artid = field_by_studio_id(&db, kott, "lOnr").expect("kOtt.lOnr");
        let f_key = field_by_studio_id(&db, kott, "vfEq").expect("kOtt.vfEq");

        // CSV escape for a field that may hold a comma or quote.
        let esc = |s: &str| {
            if s.contains([',', '"', '\n']) {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        };

        let mut csv = String::from(
            "team_record,logo_artid,logo_texlib_stem,fullname,shortname,abbrname,artabbr\n",
        );
        let mut red_wings_artid: Option<u64> = None;
        let mut max_artid = 0u64;
        let mut divergent = 0usize;
        for rec in 0..kott.current_records as u32 {
            let artid = db.read_u64(kott, rec, f_artid).unwrap_or(u64::MAX);
            let key = db.read_u64(kott, rec, f_key).unwrap_or(u64::MAX);
            let full = read_str_by_name(&db, &meta, kott, rec, "fullname");
            let short = read_str_by_name(&db, &meta, kott, rec, "shortname");
            let abbr = read_str_by_name(&db, &meta, kott, rec, "abbrname");
            let artabbr = read_str_by_name(&db, &meta, kott, rec, "artabbr");
            // Skip empty/placeholder rows so the map holds only real teams.
            if full.trim().is_empty() && short.trim().is_empty() {
                continue;
            }
            if artid != u64::MAX {
                max_artid = max_artid.max(artid);
                if artid != key {
                    divergent += 1;
                }
            }
            if full.contains("Red Wings") || short.contains("Red Wings") {
                red_wings_artid = Some(artid);
            }
            csv.push_str(&format!(
                "{rec},{artid},texlib_{artid},{},{},{},{}\n",
                esc(full.trim()),
                esc(short.trim()),
                esc(abbr.trim()),
                esc(artabbr.trim()),
            ));
        }

        // Hard-rule-3 assertions: logo id == artid (lOnr), not an index/key.
        assert_eq!(
            red_wings_artid,
            Some(10),
            "Red Wings logo artid should be 10 (matches eyeballed texlib_10)"
        );
        assert!(
            max_artid > kott.current_records as u64,
            "artid space (max {max_artid}) must exceed the {} team records — proves it is not a \
             record index",
            kott.current_records
        );
        assert!(
            divergent > 0,
            "artid must diverge from the team key on some teams — proves logo id is lOnr, not vfEq"
        );

        let csv_path = repo.join("docs/audits/data/assets/nhllegacy/xbox360/identities.csv");
        if std::env::var_os("QA_WRITE").is_some() {
            std::fs::create_dir_all(csv_path.parent().unwrap()).unwrap();
            std::fs::write(&csv_path, &csv).unwrap();
            eprintln!("wrote {} ({} bytes)", csv_path.display(), csv.len());
        } else {
            let committed = std::fs::read_to_string(&csv_path).unwrap_or_default();
            assert_eq!(
                committed.replace("\r\n", "\n"),
                csv,
                "identities.csv drifted — re-run with QA_WRITE=1 to regenerate"
            );
        }
    }

    /// A jersey texlib bundle exposes its body colour map and its number `font` map as **separate**
    /// [`OutfitTexture`] entries (same file, different `texture_index`) — the per-raster listing the
    /// Uniforms kit view rides on. Mount-gated (real NHL Legacy jersey bundle).
    #[test]
    fn legacy_x360_list_team_outfits_surfaces_font_raster() {
        const ROOT: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy";
        let rendering = std::path::Path::new(ROOT).join("rendering");
        if !rendering.is_dir() {
            eprintln!("skipping: no mount");
            return;
        }
        // The full roster is needed for the team-id token position to derive (min 4 teams).
        let teams: Vec<OutfitTeam> = (0..40u64)
            .map(|id| OutfitTeam {
                team_id: id,
                slug: if id == 10 {
                    "detroit_red_wings".into()
                } else {
                    format!("team_{id}")
                },
                abbrev: format!("t{id}"),
                art_ids: Vec::new(),
            })
            .collect();
        let got = list_team_outfits(&rendering, &teams, 10);
        let body = got
            .iter()
            .find(|o| o.piece == "jersey" && o.file_name == "texlib_0_10_0.rx2");
        let font = got
            .iter()
            .find(|o| o.piece == "font" && o.file_name == "texlib_0_10_0.rx2");
        let body = body.expect("jersey body entry present");
        let font = font.expect("font entry present");
        assert_ne!(
            body.texture_index, font.texture_index,
            "body and font are distinct rasters"
        );
        assert!(
            got.iter().any(|o| o.is_nameplate),
            "nameplate sheet still surfaces"
        );
    }

    /// Phase 5.2 (uniforms) + 5.3 (equipment) — NHL Legacy Xbox 360 texture
    /// decode confirmation from the extracted tree. Content-driven (try
    /// `texlib::parse`; a mesh `.rx2` yields zero textures and is skipped),
    /// mirroring `process_one_outfit`'s real decode. Asserts every decoded raster
    /// is structurally valid and that each family's color content decodes
    /// somewhere (a color map is richly-colored). Skips without the mount;
    /// `QA_ASSET_OUT` dumps one sample PNG per family for eyeball review.
    #[test]
    fn legacy_x360_outfit_and_equipment_decode_from_extracted_tree() {
        const ROOT: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy";
        let rendering = std::path::Path::new(ROOT).join("rendering");
        if !rendering.is_dir() {
            eprintln!("skipping: {} not present", rendering.display());
            return;
        }

        // 5.2 uniforms, then 5.3 equipment. Each is a `rendering/<folder>` of loose `.rx2`
        // (mesh + per-map-suffix texture rasters).
        let families: &[&str] = &[
            "jersey",
            "pant",
            "sock", // 5.2
            "helmet",
            "glove",
            "blocker",
            "trapper",
            "goaliepad",
            "goaliehelmet",
            "goaliestick",
            "playerstick", // 5.3
        ];

        let distinct_colors = |rgba: &[u8], cap: usize| -> usize {
            let mut seen = std::collections::HashSet::new();
            for px in rgba.chunks_exact(4) {
                seen.insert([px[0], px[1], px[2], px[3]]);
                if seen.len() >= cap {
                    break;
                }
            }
            seen.len()
        };
        let out = std::env::var_os("QA_ASSET_OUT").map(std::path::PathBuf::from);
        if let Some(d) = &out {
            std::fs::create_dir_all(d).ok();
        }

        let mut families_seen = 0usize;
        let mut global_max_colors = 0usize;
        for &family in families {
            let dir = rendering.join(family);
            let Ok(rd) = std::fs::read_dir(&dir) else {
                eprintln!("  {family}: folder absent — skipped");
                continue;
            };
            let mut files: Vec<std::path::PathBuf> = rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("rx2")))
                .collect();
            files.sort();
            // Prefer color-map (`_cm`) files up front so a rich raster is in the sample.
            files.sort_by_key(|p| {
                let s = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                !s.to_ascii_lowercase().ends_with("_cm")
            });
            files.truncate(18);
            if files.is_empty() {
                continue;
            }
            families_seen += 1;

            let mut textures = 0usize;
            let mut meshes = 0usize;
            let mut max_colors = 0usize;
            let mut dumped = false;
            for path in &files {
                let bytes = read_asset_file(path).expect("read outfit rx2");
                let Ok(lib) = tdb_rx2::texlib::parse(&bytes) else {
                    meshes += 1; // parse-fail (unexpected shape) — treat as non-texture
                    continue;
                };
                if lib.textures.is_empty() {
                    meshes += 1; // a mesh `.rx2` (FXRS, no texture slots)
                    continue;
                }
                for t in &lib.textures {
                    assert!(
                        (4..=4096).contains(&t.width) && (4..=4096).contains(&t.height),
                        "{}: implausible {family} texture dims {}x{}",
                        path.display(),
                        t.width,
                        t.height
                    );
                    let rgba = linear_to_rgba(t.format, t.width, t.height, &t.base_mip_linear)
                        .unwrap_or_else(|e| panic!("{}: to rgba — {e}", path.display()));
                    assert_eq!(rgba.len(), (t.width as usize) * (t.height as usize) * 4);
                    let colors = distinct_colors(&rgba, 64);
                    max_colors = max_colors.max(colors);
                    textures += 1;
                    // Dump the first richly-colored raster in this family.
                    if !dumped && colors >= 32 {
                        if let Some(d) = &out {
                            let stem = path.file_stem().unwrap().to_string_lossy();
                            if let Ok(png) = (DecodedImage {
                                width: t.width,
                                height: t.height,
                                rgba,
                            })
                            .to_png()
                            {
                                let _ = std::fs::write(d.join(format!("{family}_{stem}.png")), png);
                                dumped = true;
                            }
                        }
                    }
                }
            }
            eprintln!(
                "  {family}: {} files sampled — {textures} texture rasters, {meshes} meshes, \
                 max {max_colors} colors",
                files.len()
            );
            // Each family ships texture rasters, and its color content decodes to a
            // real (richly-colored) image — the Xbox-360 texture path is right.
            assert!(
                textures > 0,
                "{family}: no texture rasters decoded from {} files",
                files.len()
            );
            // Degeneracy guard: the family's color content decodes to a real image,
            // not a flat/garbage buffer. (A few families sample mostly normal/detail
            // maps, so the floor is lenient; the global check below proves rich
            // color decode outright.)
            assert!(
                max_colors >= 16,
                "{family}: richest raster only {max_colors} colors — decode suspect"
            );
            global_max_colors = global_max_colors.max(max_colors);
        }
        assert!(
            families_seen >= 8,
            "expected most outfit/equipment folders present"
        );
        // At least one family decoded a fully rich color map (the Ducks goalie mask
        // etc.) — proof the Xbox-360 texlib path is right, not just non-degenerate.
        assert!(
            global_max_colors >= 48,
            "no richly-colored outfit/equipment raster decoded (global max {global_max_colors})"
        );
        eprintln!("outfit/equipment: {families_seen} families confirmed (global max {global_max_colors} colors)");
    }

    /// Build a solid-magenta linear (untiled, LE) DXT base mip of the given
    /// format + dims — an unmistakable in-game marker.
    fn magenta_base_mip(format: tdb_rx2::texlib::PixelFormat, width: u32, height: u32) -> Vec<u8> {
        use tdb_rx2::texlib::PixelFormat;
        let bw = width.div_ceil(4).max(1) as usize;
        let bh = height.div_ceil(4).max(1) as usize;
        // Magenta in RGB565 = R31 G0 B31 = 0xF81F, little-endian [0x1F, 0xF8].
        let color = [0x1F, 0xF8, 0x1F, 0xF8, 0x00, 0x00, 0x00, 0x00];
        let block: Vec<u8> = match format {
            PixelFormat::Dxt1 => color.to_vec(),
            // DXT3: 8-byte explicit-alpha block (all 0xFF = opaque) + color block.
            PixelFormat::Dxt3 => [[0xFFu8; 8].as_slice(), &color].concat(),
            // DXT5: alpha0=alpha1=0xFF (opaque) + 6 index bytes + color block.
            PixelFormat::Dxt5 => [[0xFF, 0xFF, 0, 0, 0, 0, 0, 0].as_slice(), &color].concat(),
            PixelFormat::Argb8888 => vec![0xFF, 0x00, 0xFF, 0xFF], // A R G B per texel
        };
        block.repeat(bw * bh)
    }

    /// **Game-verified probe generator (write path, option B1).** Recolor one boot
    /// texture solid magenta, store it **raw** into `boot.big` via the new
    /// `eb::EbFile` writer (original `+12` checksum kept), and write the repacked
    /// archive to `QA_ASSET_OUT`. Loading it in an emulator answers both write-path
    /// unknowns at once: does the loader accept a substituted (raw) codec where
    /// `chunklzx` was, and does it validate the `+12` checksum? Mount-gated; also
    /// dumps a preview PNG of each boot texlib so the visible target is known.
    #[test]
    fn legacy_x360_boot_write_probe() {
        use tdb_archive_big::eb::EbFile;
        const ROOT: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy";
        let boot_big = std::path::Path::new(ROOT).join("boot.big");
        let Ok(archive_bytes) = std::fs::read(&boot_big) else {
            eprintln!("skipping: {} not present", boot_big.display());
            return;
        };
        let Some(out) = std::env::var_os("QA_ASSET_OUT").map(std::path::PathBuf::from) else {
            eprintln!("skipping: set QA_ASSET_OUT to generate the probe");
            return;
        };
        std::fs::create_dir_all(&out).ok();

        // Preview every boot texlib so the visible target is unambiguous.
        for n in [0u32, 1, 2, 3, 4, 5, 6] {
            let p = std::path::Path::new(ROOT)
                .join("rendering")
                .join("boot")
                .join(format!("texlib_{n}.rx2"));
            let Ok(bytes) = read_asset_file(&p) else {
                continue;
            };
            let Ok(lib) = tdb_rx2::texlib::parse(&bytes) else {
                continue;
            };
            for (i, t) in lib.textures.iter().enumerate() {
                if let Ok(rgba) = linear_to_rgba(t.format, t.width, t.height, &t.base_mip_linear) {
                    let img = DecodedImage {
                        width: t.width,
                        height: t.height,
                        rgba,
                    };
                    if let Ok(png) = img.to_png() {
                        let _ = std::fs::write(
                            out.join(format!("boot_texlib_{n}_{i}_{}x{}.png", t.width, t.height)),
                            png,
                        );
                    }
                }
            }
        }

        // Targets: both 1280x720 HD boot-splash copies (texlib_0 and texlib_5), so the
        // magenta shows whichever the game displays. Override with BOOT_TARGET=n[,m].
        let targets: Vec<u32> = std::env::var("BOOT_TARGET")
            .ok()
            .map(|s| s.split(',').filter_map(|t| t.trim().parse().ok()).collect())
            .unwrap_or_else(|| vec![0, 5]);

        let mut eb = EbFile::parse(&archive_bytes).expect("parse boot.big");
        for target in &targets {
            let src_path = std::path::Path::new(ROOT)
                .join("rendering")
                .join("boot")
                .join(format!("texlib_{target}.rx2"));
            let src = read_asset_file(&src_path).expect("read extracted boot texlib");
            let lib = tdb_rx2::texlib::parse(&src).expect("parse boot texlib");
            let (idx, tw, th, tf) = lib
                .textures
                .iter()
                .enumerate()
                .max_by_key(|(_, t)| t.width * t.height)
                .map(|(i, t)| (i, t.width, t.height, t.format))
                .expect("texlib has a texture");
            let magenta = magenta_base_mip(tf, tw, th);
            let modified_rx2 = lib
                .replace_same_size(idx, &magenta)
                .expect("recolor base mip");

            // Substitute the codec: wrap the recolored RW4 as `chunkzip` (which the loader's codec
            // recognises) instead of the original `chunklzx`, then pad to the EXACT original stored
            // size so the edit is fully in-place — no reflow, archive byte-length unchanged.
            let chunkzip = tdb_archive_codec::compress_chunkzip(&modified_rx2);
            assert_eq!(
                tdb_archive_codec::decompress_if_wrapped(&chunkzip).unwrap(),
                modified_rx2,
                "our chunkzip must round-trip"
            );

            let entry_path = format!("rendering/boot/texlib_{target}.rx2");
            let entry_idx = eb
                .slots()
                .iter()
                .position(|s| s.path == entry_path)
                .unwrap_or_else(|| panic!("{entry_path} not in boot.big"));
            let orig_stored = eb.slots()[entry_idx].bytes.len();
            assert!(
                chunkzip.len() <= orig_stored,
                "{entry_path}: chunkzip {} > original slot {orig_stored}; cannot stay in-place",
                chunkzip.len()
            );
            let mut payload = chunkzip;
            let chunkzip_len = payload.len();
            payload.resize(orig_stored, 0); // pad to exact original size → same-size in-place
            eb.replace_payload_in_place(entry_idx, payload)
                .expect("in-place replace");
            eprintln!(
                "  {entry_path} texture[{idx}] {tw}x{th} {tf:?}: magenta RW4 {} B \
                 -> chunkzip {chunkzip_len} B padded to {orig_stored} B (in-place, no reflow)",
                modified_rx2.len()
            );
        }

        let repacked = eb.encode();
        assert_eq!(
            repacked.len(),
            archive_bytes.len(),
            "in-place edit must keep the archive byte-length identical"
        );
        // Sanity: the independent reader still parses it, and each edited payload inflates to our
        // magenta RW4.
        let re = tdb_archive_big::eb::EbArchive::parse(&repacked).expect("re-parse repacked");
        for target in &targets {
            let path = format!("rendering/boot/texlib_{target}.rx2");
            let e = re
                .entries
                .iter()
                .find(|e| e.path == path)
                .expect("entry present");
            let raw = repacked[e.offset as usize..e.offset as usize + e.size as usize].to_vec();
            let inflated = tdb_archive_codec::decompress_if_wrapped(&raw).expect("inflates");
            assert!(
                inflated.starts_with(b"\x89RW4xb2"),
                "{path} inflates to an RW4"
            );
        }

        let dst = out.join("boot.big");
        std::fs::write(&dst, &repacked).expect("write probe archive");
        eprintln!(
            "WROTE in-place chunkzip probe {} ({} bytes == original {}; +12 checksums kept; no reflow)",
            dst.display(),
            repacked.len(),
            archive_bytes.len()
        );
    }

    /// **Checksum-isolation probe (write path, B1 follow-up).** Two write probes
    /// (raw+reflow, and chunkzip in-place) both crashed. This one changes NOTHING
    /// about the payloads — the original, valid `chunklzx` bytes are kept — and only
    /// corrupts the `+12` field of the two boot-splash entries. It isolates the last
    /// ambiguity:
    ///   * boots normally  -> `+12` is NOT validated => the crash was the payload
    ///     codec => the game requires `chunklzx` specifically => implement LZX (B2);
    ///   * crashes         -> `+12` IS validated => must reverse the `+12` algorithm.
    ///
    /// Mount-gated; writes to `QA_ASSET_OUT`.
    #[test]
    fn legacy_x360_boot_checksum_probe() {
        use tdb_archive_big::eb::EbFile;
        const ROOT: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy";
        let boot_big = std::path::Path::new(ROOT).join("boot.big");
        let Ok(mut archive_bytes) = std::fs::read(&boot_big) else {
            eprintln!("skipping: {} not present", boot_big.display());
            return;
        };
        let Some(out) = std::env::var_os("QA_ASSET_OUT").map(std::path::PathBuf::from) else {
            eprintln!("skipping: set QA_ASSET_OUT to generate the probe");
            return;
        };
        std::fs::create_dir_all(&out).ok();

        // 16-byte entries, file table at 0x30; the `+12` word is at entry+12.
        const FILE_TABLE_OFFSET: usize = 0x30;
        const ENTRY_SIZE: usize = 16;
        let eb = EbFile::parse(&archive_bytes).expect("parse boot.big");
        for target in [0u32, 5] {
            let path = format!("rendering/boot/texlib_{target}.rx2");
            let idx = eb
                .slots()
                .iter()
                .position(|s| s.path == path)
                .unwrap_or_else(|| panic!("{path} not found"));
            let crc_off = FILE_TABLE_OFFSET + idx * ENTRY_SIZE + 12;
            let orig = u32::from_be_bytes(archive_bytes[crc_off..crc_off + 4].try_into().unwrap());
            // Deliberately-wrong, non-zero value (0 could be a "skip" sentinel).
            archive_bytes[crc_off..crc_off + 4].copy_from_slice(&0xDEAD_BEEFu32.to_be_bytes());
            eprintln!("  corrupted +12 of {path} (entry {idx}): 0x{orig:08X} -> 0xDEADBEEF");
        }
        // Everything else — payloads (valid chunklzx), offsets, sizes — is untouched.
        let dst = out.join("boot.big");
        std::fs::write(&dst, &archive_bytes).expect("write probe");
        eprintln!(
            "WROTE checksum probe {} ({} bytes; only two +12 words changed)",
            dst.display(),
            archive_bytes.len()
        );
    }

    /// The team-logo preview must resolve the **front-end crest**
    /// (`fe/ion/artassets/teamlogos/t{artid}.big`), not the in-rink wordmark in
    /// `rendering/logo/`. Red Wings (artid 10) → the 256×256 winged-wheel crest,
    /// richly covered — a wordmark would be 256×128 and ~7% covered. Mount-gated.
    #[test]
    fn legacy_x360_team_logo_uses_fe_crest() {
        const ROOT: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy";
        if !std::path::Path::new(ROOT).join("fe").is_dir() {
            eprintln!("skipping: {ROOT} mount not present");
            return;
        }
        let roots = AssetRoots::resolve(std::path::Path::new(ROOT), default_platform());
        let img = decode_team_logo(&roots, 10).expect("Red Wings crest resolves");
        assert_eq!((img.width, img.height), (256, 256), "fe crest is 256×256");
        let covered = img.rgba.chunks_exact(4).filter(|p| p[3] > 8).count();
        let pct = covered as f64 / (img.width * img.height) as f64;
        assert!(
            pct > 0.10,
            "crest should be richly covered ({:.0}%), not a sparse wordmark",
            pct * 100.0
        );
    }

    /// **Extracted-format WRITE path** (the actual goal): read a real extracted texlib `.rx2`, edit a
    /// texture, and write the modified `.rx2` back — verifying (a) a **lossless round-trip** (writing
    /// an unchanged base mip re-decodes byte-for-byte) and (b) an **edit** (recolor) survives a
    /// write→re-parse→decode cycle. No `.big`, no LZX — this is `tdb_rx2::texlib::replace_*` on the
    /// extracted file, which is all this workflow needs. Mount-gated; dumps before/after PNGs.
    #[test]
    fn legacy_x360_extracted_texlib_write_roundtrip() {
        const ROOT: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy";
        let src_path = std::path::Path::new(ROOT)
            .join("rendering")
            .join("logo")
            .join("texlib_10_0_0.rx2"); // Detroit Red Wings primary logo
        let Ok(bytes) = std::fs::read(&src_path) else {
            eprintln!("skipping: {} not present", src_path.display());
            return;
        };
        let out = std::env::var_os("QA_ASSET_OUT").map(std::path::PathBuf::from);
        if let Some(d) = &out {
            std::fs::create_dir_all(d).ok();
        }

        let lib = tdb_rx2::texlib::parse(&bytes).expect("parse texlib");
        let (idx, tw, th, tf) = lib
            .textures
            .iter()
            .enumerate()
            .max_by_key(|(_, t)| t.width * t.height)
            .map(|(i, t)| (i, t.width, t.height, t.format))
            .expect("texlib has a texture");
        let original_base = lib.textures[idx].base_mip_linear.clone();

        // (a) Lossless round-trip: write the *unchanged* base mip back and confirm it re-decodes
        // byte-for-byte (the tile/untile + endian-swap survive a write→read cycle intact).
        let rewritten = lib
            .replace_base_keep_tail(idx, &original_base)
            .expect("rewrite unchanged base");
        let relib = tdb_rx2::texlib::parse(&rewritten).expect("re-parse rewritten");
        assert_eq!(
            relib.textures[idx].base_mip_linear, original_base,
            "unchanged rewrite must re-decode identically (lossless)"
        );

        // (b) Real edit: recolor the logo magenta, write the extracted .rx2, re-parse + decode.
        let magenta = magenta_base_mip(tf, tw, th);
        let edited = lib.replace_same_size(idx, &magenta).expect("recolor");
        let edited_lib = tdb_rx2::texlib::parse(&edited).expect("re-parse edited");
        let et = &edited_lib.textures[idx];
        assert_eq!(
            (et.width, et.height, et.format),
            (tw, th, tf),
            "dims/format preserved"
        );
        let rgba =
            linear_to_rgba(et.format, et.width, et.height, &et.base_mip_linear).expect("rgba");
        // Every pixel is magenta (R=255,G=0,B=255) — the edit took.
        let all_magenta = rgba
            .chunks_exact(4)
            .all(|p| p[0] == 255 && p[1] == 0 && p[2] == 255);
        assert!(
            all_magenta,
            "edited texture must read back as solid magenta"
        );

        if let Some(d) = &out {
            // before
            if let Ok(png) = (DecodedImage {
                width: tw,
                height: th,
                rgba: linear_to_rgba(tf, tw, th, &original_base).unwrap(),
            })
            .to_png()
            {
                let _ = std::fs::write(d.join("extracted_write_before.png"), png);
            }
            if let Ok(png) = (DecodedImage {
                width: tw,
                height: th,
                rgba,
            })
            .to_png()
            {
                let _ = std::fs::write(d.join("extracted_write_after.png"), png);
            }
            let _ = std::fs::write(d.join("texlib_10_edited.rx2"), &edited);
        }
        eprintln!(
            "extracted write OK: texlib_10 {tw}x{th} {tf:?} — lossless round-trip + magenta edit \
             re-decoded ({} bytes written)",
            edited.len()
        );
    }

    /// **Extracted PORTRAIT write** (ported from the prior app's proven pipeline): take a real
    /// extracted `p{id}.big`, replace its XPR2 with a gradient, rebuild the archive, write it, and
    /// re-`decode_portrait` the rebuilt file — confirming it's a loadable extracted portrait whose
    /// pixels are now the new image. Mount-gated; dumps before/after PNGs to `QA_ASSET_OUT`.
    #[test]
    fn legacy_x360_extracted_portrait_write_roundtrip() {
        const ROOT: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy";
        let p =
            std::path::Path::new(ROOT).join(r"fe\ion\artassets\playerheadssmall\p0_4000\p1.big");
        let Ok(bytes) = std::fs::read(&p) else {
            eprintln!("skipping: {} not present", p.display());
            return;
        };
        let before = decode_portrait(&p).expect("decode original portrait");
        let (w, h) = (before.width, before.height);
        assert!(
            w % 4 == 0 && h % 4 == 0,
            "portrait dims {w}x{h} DXT-aligned"
        );

        // Gradient source: R ramps across, B ramps down — unmistakable + directional.
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                rgba[i] = (x * 255 / w.max(1)) as u8;
                rgba[i + 2] = (y * 255 / h.max(1)) as u8;
                rgba[i + 3] = 255;
            }
        }

        let rebuilt = replace_portrait_texture(bytes, &rgba, w, h).expect("portrait replace");

        // Write the rebuilt extracted p*.big and re-decode it through the same read path.
        let dir = std::env::var_os("QA_ASSET_OUT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        std::fs::create_dir_all(&dir).ok();
        let rebuilt_path = dir.join("p1_rebuilt.big");
        std::fs::write(&rebuilt_path, &rebuilt).expect("write rebuilt");
        let after = decode_portrait(&rebuilt_path).expect("decode rebuilt portrait");

        assert_eq!((after.width, after.height), (w, h), "geometry preserved");
        // The gradient survived (proves our pixels landed, not the original): bottom-right is
        // redder + bluer than top-left. DXT5 is lossy, so allow slack.
        let px = |x: u32, y: u32| {
            let i = ((y * w + x) * 4) as usize;
            [after.rgba[i], after.rgba[i + 1], after.rgba[i + 2]]
        };
        let tl = px(0, 0);
        let br = px(w - 1, h - 1);
        assert!(
            br[0] as i32 > tl[0] as i32 + 40,
            "red gradient lost ({tl:?} -> {br:?})"
        );
        assert!(
            br[2] as i32 > tl[2] as i32 + 40,
            "blue gradient lost ({tl:?} -> {br:?})"
        );

        if std::env::var_os("QA_ASSET_OUT").is_some() {
            let _ = std::fs::write(dir.join("portrait_before.png"), before.to_png().unwrap());
            let _ = std::fs::write(dir.join("portrait_after.png"), after.to_png().unwrap());
        }
        eprintln!(
            "extracted portrait write OK: {w}x{h} XPR2 -> RefPack -> BIGF rebuilt ({} bytes), \
             re-decoded with gradient intact",
            rebuilt.len()
        );
    }

    /// Build a directional gradient RGBA8 (R ramps across, B ramps down) — an
    /// unmistakable, orientation-revealing test image.
    fn gradient_rgba(w: u32, h: u32) -> Vec<u8> {
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                rgba[i] = (x * 255 / w.max(1)) as u8;
                rgba[i + 2] = (y * 255 / h.max(1)) as u8;
                rgba[i + 3] = 255;
            }
        }
        rgba
    }

    /// **PS3 DDS portrait write** — the analogue of the Xbox XPR2 test above, on
    /// the PS3 container (`BIGF` → RefPack → little-endian `DDS `). Fully
    /// synthetic (no mount): build a PS3-style portrait archive from a DXT5 DDS,
    /// replace it via `replace_portrait_texture`, and confirm (a) the rebuilt
    /// entry is a full-mip-chain DDS — proving the DDS branch (not XPR2) ran —
    /// and (b) it re-decodes through `decode_portrait` with the new gradient
    /// intact. Proves the extracted PS3-portrait write path end-to-end.
    #[test]
    fn ps3_dds_portrait_write_roundtrip_synthetic() {
        let (w, h) = (64u32, 64u32);

        // Synthesize a PS3-style portrait `.big`: a DXT5 DDS (RefPack-wrapped)
        // plus a small non-texture entry so the picker must choose by area.
        let original = gradient_rgba(w, h);
        // Reverse the ramp so the "before" and "after" are clearly distinct.
        let mut original_flipped = original.clone();
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                original_flipped[i] = 255 - original[i];
            }
        }
        // A real EA portrait DDS ships the full mip chain; the writer preserves the source's
        // level structure (P1 audit, 21 Sep 2026), so the synthetic source carries it too.
        let levels = compress_bc_mip_levels(tdb_archive_codec::DdsFormat::Dxt5, w, h, &original_flipped);
        let chain: Vec<u8> = levels.iter().flat_map(|(_, _, d)| d.iter().copied()).collect();
        let dds = tdb_archive_codec::encode_dds(
            tdb_archive_codec::DdsFormat::Dxt5,
            w,
            h,
            levels.len() as u32,
            &chain,
        );
        let wrapped = tdb_archive_codec::refpack_compress(&dds);

        let mut builder = tdb_archive_big::ArchiveBuilder::new();
        builder.add_entry("apt", b"Apt Data\0not-a-texture".to_vec());
        builder.add_entry("p1", wrapped);
        let archive_bytes = builder.build().expect("build synthetic portrait .big");

        // Sanity: the synthetic archive reads back as the flipped gradient. Receipt 66: an exclusive per-run folder (created
        // new, never the old shared `nhlms_ps3_dds_portrait`, whose files are left alone), removed by its guard.
        struct Owned(std::path::PathBuf);
        impl Drop for Owned {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let owned = Owned(std::env::temp_dir().join(format!("nhlms_ps3_dds_portrait-{}-{nanos}", std::process::id())));
        std::fs::create_dir(&owned.0).expect("an exclusive scratch folder");
        let dir = owned.0.clone();
        let src_path = dir.join("p1_src.big");
        std::fs::write(&src_path, &archive_bytes).unwrap();
        let before = decode_portrait(&src_path).expect("decode synthetic PS3 portrait");
        assert_eq!((before.width, before.height), (w, h), "synthetic geometry");

        // Replace with a fresh (non-flipped) gradient through the write path.
        let replacement = gradient_rgba(w, h);
        let rebuilt =
            replace_portrait_texture(archive_bytes, &replacement, w, h).expect("PS3 DDS replace");

        // (a) The rebuilt `p1` entry must be a RefPack-wrapped DDS carrying the source's FULL
        // mip chain (proves the DDS arm ran, not the XPR2 arm, and that the shipped level
        // structure is preserved).
        let rebuilt_archive = tdb_archive_big::Archive::parse(rebuilt.clone()).expect("re-parse");
        let raw = rebuilt_archive.read_entry_raw("p1").expect("p1 present");
        let inner = tdb_archive_codec::refpack_decompress(&raw).expect("entry is RefPack-wrapped");
        assert!(
            tdb_archive_codec::is_dds(&inner),
            "rebuilt entry must be a DDS"
        );
        let mip_count = u32::from_le_bytes(inner[28..32].try_into().unwrap());
        assert_eq!(
            mip_count,
            full_mip_count(w, h),
            "DDS must carry the full mip chain"
        );

        // (b) Re-decode through the read path; the new gradient must have landed.
        let rebuilt_path = dir.join("p1_rebuilt.big");
        std::fs::write(&rebuilt_path, &rebuilt).unwrap();
        let after = decode_portrait(&rebuilt_path).expect("decode rebuilt PS3 portrait");
        assert_eq!((after.width, after.height), (w, h), "geometry preserved");
        let px = |x: u32, y: u32| {
            let i = ((y * w + x) * 4) as usize;
            [after.rgba[i], after.rgba[i + 1], after.rgba[i + 2]]
        };
        let tl = px(0, 0);
        let br = px(w - 1, h - 1);
        assert!(
            br[0] as i32 > tl[0] as i32 + 40,
            "red gradient lost ({tl:?} -> {br:?})"
        );
        assert!(
            br[2] as i32 > tl[2] as i32 + 40,
            "blue gradient lost ({tl:?} -> {br:?})"
        );

        eprintln!(
            "PS3 DDS portrait write OK: {w}x{h} DXT5 -> DDS ({mip_count} mips) -> RefPack -> BIGF \
             rebuilt ({} bytes), re-decoded with gradient intact",
            rebuilt.len()
        );
    }

    /// **PS4 GNF texture write** — self-contained (no mount). Build a BC1 GNF from
    /// a gradient, replace it via `replace_gnf_texture` with a fresh gradient, and
    /// confirm (a) the 256-byte GNF header is preserved verbatim and (b) it
    /// re-decodes through `gnf_to_rgba` with the new gradient intact. Proves the
    /// RGBA → BC1 → GNM re-tile → header-preserving splice write end-to-end.
    #[test]
    fn ps4_gnf_write_roundtrip_synthetic() {
        let (w, h) = (64u32, 64u32);

        // Source GNF: a flipped-red gradient, BC1-compressed and wrapped.
        let base = gradient_rgba(w, h);
        let mut flipped = base.clone();
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                flipped[i] = 255 - base[i];
            }
        }
        let src_blocks =
            tdb_archive_codec::compress_bc(tdb_archive_codec::DdsFormat::Dxt1, w, h, &flipped);
        let gnf = tdb_rx2::gnf::encode_gnf(tdb_rx2::gnf::BcFormat::Bc1, w, h, &src_blocks);

        // Replace with a fresh (non-flipped) gradient.
        let replacement = gradient_rgba(w, h);
        let rebuilt = replace_gnf_texture(&gnf, &replacement, w, h).expect("GNF replace");
        assert_eq!(
            &rebuilt[..0x100],
            &gnf[..0x100],
            "GNF header preserved verbatim"
        );
        assert_eq!(
            rebuilt.len(),
            gnf.len(),
            "same-size edit keeps the file length"
        );

        let tex = tdb_rx2::gnf::parse(&rebuilt).expect("re-parse rebuilt GNF");
        assert_eq!((tex.width, tex.height), (w, h));
        assert_eq!(tex.format, tdb_rx2::gnf::BcFormat::Bc1);
        let after = gnf_to_rgba(&tex).expect("decode rebuilt GNF");
        let px = |x: u32, y: u32| {
            let i = ((y * w + x) * 4) as usize;
            [after[i], after[i + 1], after[i + 2]]
        };
        let tl = px(0, 0);
        let br = px(w - 1, h - 1);
        assert!(
            br[0] as i32 > tl[0] as i32 + 40,
            "red gradient lost ({tl:?} -> {br:?})"
        );
        assert!(
            br[2] as i32 > tl[2] as i32 + 40,
            "blue gradient lost ({tl:?} -> {br:?})"
        );
        eprintln!("PS4 GNF write OK: {w}x{h} BC1 re-tiled + header-preserved, gradient intact");
    }

    /// Decode level `level` of a rule-conformant GNF chain by re-wrapping its tiled bytes in a
    /// one-level header of that level's size (test helper; the codec only decodes the base).
    fn decode_gnf_level(gnf: &[u8], level: u32) -> Vec<u8> {
        let (w, h, format, _) = tdb_rx2::gnf::check_resize_rule(gnf).expect("rule-conformant");
        let elem = format.block_bytes();
        let off = 0x100 + tdb_rx2::gnf::chain_tiled_bytes(w, h, elem, level);
        let (lw, lh) = ((w >> level).max(1), (h >> level).max(1));
        let len = tdb_rx2::gnf::chain_tiled_bytes(lw, lh, elem, 1);
        let mut one = tdb_rx2::gnf::encode_gnf(format, lw, lh, &vec![0u8; (lw.div_ceil(4) * lh.div_ceil(4)) as usize * elem]);
        one[0x100..0x100 + len].copy_from_slice(&gnf[off..off + len]);
        gnf_to_rgba(&tdb_rx2::gnf::parse(&one).unwrap()).unwrap()
    }

    /// A rule-conformant `w`×`h` BC1 GNF of `levels` levels, every level solid `rgb`.
    fn solid_gnf_chain(w: u32, h: u32, levels: u32, rgb: [u8; 3]) -> Vec<u8> {
        let fmt = tdb_archive_codec::DdsFormat::Dxt1;
        let chain: Vec<(u32, u32, Vec<u8>)> = (0..levels)
            .map(|l| {
                let (lw, lh) = ((w >> l).max(1), (h >> l).max(1));
                let px: Vec<u8> = (0..lw * lh).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect();
                (lw, lh, tdb_archive_codec::compress_bc(fmt, lw, lh, &px))
            })
            .collect();
        let one = tdb_rx2::gnf::encode_gnf(tdb_rx2::gnf::BcFormat::Bc1, w, h, &chain[0].2);
        let lv: Vec<tdb_rx2::gnf::LinearLevel<'_>> = chain.iter().map(|(lw, lh, d)| tdb_rx2::gnf::LinearLevel { width: *lw, height: *lh, linear: d }).collect();
        tdb_rx2::gnf::replace_resized(&one, &lv).unwrap()
    }

    /// Roadmap P0-1 (PS4 parity review, 13 Oct 2026): a same-size GNF replace used to rewrite only
    /// the base level, so the old image survived in every smaller mip and showed at a distance.
    /// Now every level of a measured chain decodes to the replacement, the header is untouched,
    /// and the pre-Apply plan is the writer's own (checked, no disclosure).
    #[test]
    fn ps4_gnf_same_size_replace_rewrites_every_mip_level() {
        let (w, h, levels) = (64u32, 64u32, 4u32);
        let src = solid_gnf_chain(w, h, levels, [0, 0, 255]);
        let red: Vec<u8> = (0..w * h).flat_map(|_| [255u8, 0, 0, 255]).collect();
        let plan = plan_texture_replace(&src, w, h, None).unwrap();
        assert!(plan.checked && plan.allowed && plan.notes.is_empty(), "{plan:?}");
        let write = replace_texture_in_file(src.clone(), &red, w, h, None).unwrap();
        assert!(write.notes.is_empty(), "{:?}", write.notes);
        assert_eq!(&write.bytes[..0x100], &src[..0x100], "header kept verbatim");
        assert_eq!(write.bytes.len(), src.len());
        for level in 0..levels {
            let px = decode_gnf_level(&write.bytes, level);
            assert!(px.chunks(4).all(|p| p[0] > 200 && p[2] < 50), "level {level} still holds the old image");
        }
    }

    /// A GNF whose lower levels are not in the measured layout (here: trailing bytes break the
    /// `streamSize` rule) keeps the base-only write, and both the plan and the write disclose that
    /// the smaller levels keep the original image. A resize of the same file is refused by the plan
    /// with the writer's own reason.
    #[test]
    fn ps4_gnf_unmeasured_chain_discloses_the_kept_levels_and_plans_refusals() {
        let (w, h) = (64u32, 64u32);
        let mut src = solid_gnf_chain(w, h, 3, [0, 0, 255]);
        src.extend_from_slice(&[0u8; 16]);
        let red: Vec<u8> = (0..w * h).flat_map(|_| [255u8, 0, 0, 255]).collect();
        let plan = plan_texture_replace(&src, w, h, None).unwrap();
        assert!(plan.checked && plan.allowed, "{plan:?}");
        assert_eq!(plan.notes.len(), 1);
        assert!(plan.notes[0].contains("2 smaller mip level(s) keep the original image"), "{}", plan.notes[0]);
        let write = replace_texture_in_file(src.clone(), &red, w, h, None).unwrap();
        assert_eq!(write.notes, plan.notes, "the write discloses exactly what the plan showed");
        let base = tdb_rx2::gnf::parse(&write.bytes).unwrap();
        assert!(gnf_to_rgba(&base).unwrap().chunks(4).all(|p| p[0] > 200), "base replaced");
        assert_eq!(&write.bytes[0x100 + tdb_rx2::gnf::chain_tiled_bytes(w, h, 8, 1)..], &src[0x100 + tdb_rx2::gnf::chain_tiled_bytes(w, h, 8, 1)..], "lower levels kept, as disclosed");
        // Resize: the plan refuses with the writer's reason.
        let big: Vec<u8> = (0..128 * 128).flat_map(|_| [255u8, 0, 0, 255]).collect();
        let plan = plan_texture_replace(&src, 128, 128, None).unwrap();
        assert!(plan.checked && !plan.allowed, "{plan:?}");
        let err = replace_texture_in_file(src, &big, 128, 128, None).unwrap_err().to_string();
        assert!(err.contains(plan.refusal.as_deref().unwrap()), "plan `{:?}` vs write `{err}`", plan.refusal);
    }

    /// **PS4 BC4 / BC5 map write** — single-channel (roughness) and dual-channel
    /// (normal) GNF maps. Build each from a gradient, replace via
    /// `replace_gnf_texture`, and confirm the header is preserved and the edited
    /// channel(s) survive a decode round-trip.
    #[test]
    fn ps4_gnf_bc4_bc5_map_write_roundtrip_synthetic() {
        let (w, h) = (64u32, 64u32);

        // BC4: a horizontal grayscale gradient.
        let mut gray = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                let v = (x * 255 / w) as u8;
                gray[i..i + 4].copy_from_slice(&[v, v, v, 255]);
            }
        }
        let gnf4 = tdb_rx2::gnf::encode_gnf(
            tdb_rx2::gnf::BcFormat::Bc4,
            w,
            h,
            &tdb_rx2::bc45::encode_bc4(&vec![0u8; (w * h * 4) as usize], w, h),
        );
        let rebuilt4 = replace_gnf_texture(&gnf4, &gray, w, h).expect("BC4 replace");
        assert_eq!(
            &rebuilt4[..0x100],
            &gnf4[..0x100],
            "BC4 GNF header preserved"
        );
        let back4 = gnf_to_rgba(&tdb_rx2::gnf::parse(&rebuilt4).unwrap()).unwrap();
        let r4 = |x: u32| back4[(x * 4) as usize] as i32; // row 0
        assert!(r4(w - 1) > r4(0) + 40, "BC4 gradient lost");

        // BC5: R varies with x, G with y.
        let mut rg = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                rg[i] = (x * 255 / w) as u8;
                rg[i + 1] = (y * 255 / h) as u8;
                rg[i + 3] = 255;
            }
        }
        let gnf5 = tdb_rx2::gnf::encode_gnf(
            tdb_rx2::gnf::BcFormat::Bc5,
            w,
            h,
            &tdb_rx2::bc45::encode_bc5(&vec![0u8; (w * h * 4) as usize], w, h),
        );
        let rebuilt5 = replace_gnf_texture(&gnf5, &rg, w, h).expect("BC5 replace");
        assert_eq!(
            &rebuilt5[..0x100],
            &gnf5[..0x100],
            "BC5 GNF header preserved"
        );
        let back5 = gnf_to_rgba(&tdb_rx2::gnf::parse(&rebuilt5).unwrap()).unwrap();
        let at = |x: u32, y: u32, c: usize| back5[((y * w + x) * 4) as usize + c] as i32;
        assert!(
            at(w - 1, 0, 0) > at(0, 0, 0) + 40,
            "BC5 red (x) gradient lost"
        );
        assert!(
            at(0, h - 1, 1) > at(0, 0, 1) + 40,
            "BC5 green (y) gradient lost"
        );
    }

    /// **PS4 BC7 portrait write** — the NHL 16+ portrait format. Build a BC7 GNF
    /// from a gradient, replace it via `replace_gnf_texture`, and confirm the
    /// header is preserved and the new gradient survives a decode round-trip.
    /// Exercises the full RGBA → BC7 (mode 6) → GNM re-tile → splice path.
    #[test]
    fn ps4_gnf_bc7_portrait_write_roundtrip_synthetic() {
        let (w, h) = (64u32, 64u32);
        // Source BC7 GNF from a flipped gradient.
        let base = gradient_rgba(w, h);
        let mut flipped = base.clone();
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                flipped[i] = 255 - base[i];
            }
        }
        let src_blocks = tdb_rx2::bc7::encode(&flipped, w, h);
        let gnf = tdb_rx2::gnf::encode_gnf(tdb_rx2::gnf::BcFormat::Bc7, w, h, &src_blocks);

        let replacement = gradient_rgba(w, h);
        let rebuilt = replace_gnf_texture(&gnf, &replacement, w, h).expect("BC7 GNF replace");
        assert_eq!(
            &rebuilt[..0x100],
            &gnf[..0x100],
            "GNF header preserved verbatim"
        );
        assert_eq!(
            rebuilt.len(),
            gnf.len(),
            "same-size edit keeps the file length"
        );

        let tex = tdb_rx2::gnf::parse(&rebuilt).expect("re-parse rebuilt BC7 GNF");
        assert_eq!(tex.format, tdb_rx2::gnf::BcFormat::Bc7);
        let after = gnf_to_rgba(&tex).expect("decode rebuilt BC7 GNF");
        let px = |x: u32, y: u32| {
            let i = ((y * w + x) * 4) as usize;
            [after[i], after[i + 1], after[i + 2]]
        };
        let tl = px(0, 0);
        let br = px(w - 1, h - 1);
        assert!(
            br[0] as i32 > tl[0] as i32 + 40,
            "red gradient lost ({tl:?} -> {br:?})"
        );
        assert!(
            br[2] as i32 > tl[2] as i32 + 40,
            "blue gradient lost ({tl:?} -> {br:?})"
        );
        eprintln!("PS4 BC7 portrait write OK: {w}x{h} mode-6 re-tiled + header-preserved, gradient intact");
    }

    /// **Real PS4 head-texture edit** (mount-gated): take a real BC1 `head_*_cm.gnf`,
    /// paint a red patch, write it back via `replace_gnf_texture`, and confirm the
    /// patch survives a decode round-trip while the header + file length are
    /// preserved. Proves the write on genuine EA-tiled data.
    #[test]
    fn ps4_gnf_head_write_roundtrip_real() {
        const ROOT: &str = r"H:\Emulators\ShadPS4\Games\CUSA00561\databigs\Extracted";
        let p = format!(
            r"{ROOT}\nhl\data\kettle\rendering\characters\player\head_alexander_ovechkin_cm.gnf"
        );
        let Ok(bytes) = std::fs::read(&p) else {
            eprintln!("skipping: {p} not present");
            return;
        };
        let tex = tdb_rx2::gnf::parse(&bytes).expect("parse real head GNF");
        assert_eq!(tex.format, tdb_rx2::gnf::BcFormat::Bc1);
        let (w, h) = (tex.width, tex.height);

        // Paint a 64×64 red patch in the top-left of the decoded image.
        let mut edited = gnf_to_rgba(&tex).expect("decode head");
        for y in 0..64u32 {
            for x in 0..64u32 {
                let i = ((y * w + x) * 4) as usize;
                edited[i..i + 4].copy_from_slice(&[255, 0, 0, 255]);
            }
        }

        let rebuilt = replace_gnf_texture(&bytes, &edited, w, h).expect("GNF head replace");
        assert_eq!(&rebuilt[..0x100], &bytes[..0x100], "header byte-identical");
        assert_eq!(
            rebuilt.len(),
            bytes.len(),
            "same-size edit keeps file length"
        );

        let after = gnf_to_rgba(&tdb_rx2::gnf::parse(&rebuilt).unwrap()).expect("re-decode");
        let c = ((32 * w + 32) * 4) as usize; // centre of the painted patch
        assert!(
            after[c] > 180 && after[c + 1] < 80 && after[c + 2] < 80,
            "red patch must survive the write ({:?})",
            &after[c..c + 3]
        );
        eprintln!(
            "PS4 real head write OK: {w}x{h} BC1, red patch round-tripped ({} bytes)",
            rebuilt.len()
        );
    }

    /// **Real PS4 BC7 portrait edit** (mount-gated): take a real NHL 16 BC7
    /// `p{id}.gnf` portrait, paint a red patch, write it back via
    /// `replace_gnf_texture` (the mode-6 encoder path), and confirm the patch
    /// survives a decode round-trip with the header + file length preserved.
    /// Proves BC7 write on genuine EA-tiled portrait data.
    #[test]
    fn ps4_gnf_bc7_portrait_write_roundtrip_real() {
        const ROOT: &str = r"H:\Emulators\ShadPS4\Games\CUSA01909\databigs\Extracted";
        let p = format!(r"{ROOT}\nhl\ui\kettle_assets\playerportraits\p10000.gnf");
        let Ok(bytes) = std::fs::read(&p) else {
            eprintln!("skipping: {p} not present");
            return;
        };
        let tex = tdb_rx2::gnf::parse(&bytes).expect("parse real portrait GNF");
        assert_eq!(tex.format, tdb_rx2::gnf::BcFormat::Bc7);
        let (w, h) = (tex.width, tex.height);

        // Paint a 64×64 red patch in the top-left of the decoded portrait.
        let mut edited = gnf_to_rgba(&tex).expect("decode portrait");
        for y in 0..64u32 {
            for x in 0..64u32 {
                let i = ((y * w + x) * 4) as usize;
                edited[i..i + 4].copy_from_slice(&[255, 0, 0, 255]);
            }
        }

        let rebuilt = replace_gnf_texture(&bytes, &edited, w, h).expect("BC7 portrait replace");
        assert_eq!(&rebuilt[..0x100], &bytes[..0x100], "header byte-identical");
        assert_eq!(
            rebuilt.len(),
            bytes.len(),
            "same-size edit keeps file length"
        );

        let after = gnf_to_rgba(&tdb_rx2::gnf::parse(&rebuilt).unwrap()).expect("re-decode");
        let c = ((32 * w + 32) * 4) as usize; // centre of the painted patch
        assert!(
            after[c] > 180 && after[c + 1] < 80 && after[c + 2] < 80,
            "red patch must survive the BC7 write ({:?})",
            &after[c..c + 3]
        );
        eprintln!(
            "PS4 real BC7 portrait write OK: {w}x{h}, red patch round-tripped ({} bytes)",
            rebuilt.len()
        );
    }

    /// The unified `replace_texture_in_file` dispatcher must route by container
    /// magic (GNF / BIGF portrait / RW4 texlib), round-trip a real edit for the
    /// containers it fully handles, and give clear errors for the rest.
    #[test]
    fn replace_texture_in_file_dispatches_by_container() {
        let (w, h) = (64u32, 64u32);
        let rgba = gradient_rgba(w, h);

        // --- GNF (PS4) ---
        let gnf = tdb_rx2::gnf::encode_gnf(
            tdb_rx2::gnf::BcFormat::Bc1,
            w,
            h,
            &tdb_archive_codec::compress_bc(tdb_archive_codec::DdsFormat::Dxt1, w, h, &rgba),
        );
        let out = replace_texture_in_file(gnf, &rgba, w, h, None).expect("GNF dispatch");
        assert_eq!(out.container, TextureContainer::Gnf);
        assert_eq!(out.container.label(), "gnf");
        assert!(
            tdb_rx2::gnf::parse(&out.bytes).is_ok(),
            "GNF rebuilt parses"
        );

        // --- Portrait (BIGF → RefPack → DDS) ---
        let dds_blocks =
            tdb_archive_codec::compress_bc(tdb_archive_codec::DdsFormat::Dxt5, w, h, &rgba);
        let dds =
            tdb_archive_codec::encode_dds(tdb_archive_codec::DdsFormat::Dxt5, w, h, 1, &dds_blocks);
        let mut builder = tdb_archive_big::ArchiveBuilder::new();
        builder.add_entry("apt", b"Apt Data\0".to_vec());
        builder.add_entry("p1", tdb_archive_codec::refpack_compress(&dds));
        let big = builder.build().unwrap();
        let out = replace_texture_in_file(big, &rgba, w, h, None).expect("portrait dispatch");
        assert_eq!(out.container, TextureContainer::Portrait);

        // --- Texlib routing: `\x89RW4` magic reaches the texlib branch (parse
        // error proves the dispatch, without needing a full synthetic texlib). ---
        let err = replace_texture_in_file(b"\x89RW4bogustexlibbytes".to_vec(), &rgba, w, h, None)
            .unwrap_err();
        assert!(
            format!("{err}").contains("texlib"),
            "RW4 routed to texlib: {err}"
        );

        // --- Whole-file RefPack is unwrapped and dispatched on the inner container; an inner
        // payload that is no container is still rejected as unrecognized. ---
        let refpacked = tdb_archive_codec::refpack_compress(b"whatever payload bytes here");
        let err = replace_texture_in_file(refpacked, &rgba, w, h, None).unwrap_err();
        assert!(
            format!("{err}").contains("unrecognized"),
            "refpack unwrapped then rejected: {err}"
        );

        // --- Unrecognized container. ---
        let err = replace_texture_in_file(vec![0u8; 64], &rgba, w, h, None).unwrap_err();
        assert!(
            format!("{err}").contains("unrecognized"),
            "unknown rejected: {err}"
        );
    }

    /// The read-side `decode_texture_file` must recognise the same containers as
    /// the writer, tag which one it found, and report the texture's real
    /// dimensions — this is what the file browser's before/after preview and row
    /// thumbnails ride on.
    #[test]
    fn decode_texture_file_reads_each_container() {
        let (w, h) = (64u32, 64u32);
        let rgba = gradient_rgba(w, h);
        let tmp = std::env::temp_dir().join("nhlms_decode_texture_file_test");
        std::fs::create_dir_all(&tmp).unwrap();

        // --- GNF (PS4) ---
        let gnf = tdb_rx2::gnf::encode_gnf(
            tdb_rx2::gnf::BcFormat::Bc1,
            w,
            h,
            &tdb_archive_codec::compress_bc(tdb_archive_codec::DdsFormat::Dxt1, w, h, &rgba),
        );
        let gnf_path = tmp.join("t.gnf");
        std::fs::write(&gnf_path, &gnf).unwrap();
        let (container, img) = decode_texture_file(&gnf_path, None).expect("decode GNF");
        assert_eq!(container, TextureContainer::Gnf);
        assert_eq!((img.width, img.height), (w, h));

        // --- Portrait (BIGF → RefPack → DDS) ---
        let dds_blocks =
            tdb_archive_codec::compress_bc(tdb_archive_codec::DdsFormat::Dxt5, w, h, &rgba);
        let dds =
            tdb_archive_codec::encode_dds(tdb_archive_codec::DdsFormat::Dxt5, w, h, 1, &dds_blocks);
        let mut builder = tdb_archive_big::ArchiveBuilder::new();
        builder.add_entry("apt", b"Apt Data\0".to_vec());
        builder.add_entry("p1", tdb_archive_codec::refpack_compress(&dds));
        let big = builder.build().unwrap();
        let big_path = tmp.join("p1.big");
        std::fs::write(&big_path, &big).unwrap();
        let (container, img) = decode_texture_file(&big_path, None).expect("decode portrait");
        assert_eq!(container, TextureContainer::Portrait);
        assert_eq!((img.width, img.height), (w, h));

        // --- Unrecognized container gives a clear error, not a panic. ---
        let junk = tmp.join("junk.bin");
        std::fs::write(&junk, [0u8; 64]).unwrap();
        match decode_texture_file(&junk, None) {
            Err(e) => assert!(
                format!("{e}").contains("unrecognized"),
                "junk rejected: {e}"
            ),
            Ok(_) => panic!("junk bytes should not decode as a texture"),
        }

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// `thumbnail` downscales to a max longest edge (aspect preserved) and never
    /// upscales — the row-thumbnail decode rides on this to keep data URLs small.
    #[test]
    fn thumbnail_downscales_longest_edge_only() {
        let big = DecodedImage {
            width: 100,
            height: 40,
            rgba: vec![0u8; 100 * 40 * 4],
        };
        let t = big.thumbnail(48);
        assert_eq!(t.width, 48, "longest edge clamped to 48");
        assert_eq!(t.height, 19, "aspect preserved (40 * 48/100 ≈ 19)");
        assert_eq!(t.rgba.len(), 48 * 19 * 4, "buffer matches new dims");

        // Already within bounds → returned unchanged (no upscale).
        let small = DecodedImage {
            width: 30,
            height: 20,
            rgba: vec![1u8; 30 * 20 * 4],
        };
        let same = small.thumbnail(48);
        assert_eq!((same.width, same.height), (30, 20));
        // max_edge 0 = full size, unchanged.
        let full = DecodedImage {
            width: 80,
            height: 80,
            rgba: vec![2u8; 80 * 80 * 4],
        };
        assert_eq!(full.thumbnail(0).width, 80);
    }

    fn player_with_portrait(record: u32, portrait_path: Option<String>) -> PlayerAssets {
        PlayerAssets {
            bio_record: record,
            full_name: format!("Player {record}"),
            slug: Some(format!("player_{record}")),
            portrait_id: record as u64,
            head_id: 60000,
            bespoke_head: false,
            pool_head: true,
            head_shared: false,
            portrait_path,
            portrait_small_path: None,
            mesh_path: None,
            texture_paths: Vec::new(),
            generic_portrait: false,
        }
    }

    /// A portrait whose exact bytes are shared by several players is a generic
    /// placeholder; a portrait unique to one player is real. `mark_generic_portraits`
    /// must flag the shared ones and leave the unique one alone.
    #[test]
    fn generic_portraits_are_the_shared_ones() {
        let tmp = std::env::temp_dir().join("nhlms_generic_portrait_test");
        std::fs::create_dir_all(&tmp).unwrap();
        // Two ids with byte-identical "blank" content, one with unique content.
        let blank_a = tmp.join("p0.big");
        let blank_b = tmp.join("p3000.big");
        let real = tmp.join("p42.big");
        std::fs::write(&blank_a, b"BLANK-CONTENT").unwrap();
        std::fs::write(&blank_b, b"BLANK-CONTENT").unwrap(); // identical bytes, different file
        std::fs::write(&real, b"a genuine unique portrait raster").unwrap();

        let path = |p: &Path| Some(p.to_string_lossy().into_owned());
        let mut players = vec![
            player_with_portrait(0, path(&blank_a)),
            player_with_portrait(1, path(&blank_b)),
            player_with_portrait(2, path(&blank_a)), // third sharer of the blank
            player_with_portrait(3, path(&real)),
            player_with_portrait(4, None), // no portrait at all
        ];
        mark_generic_portraits(&mut players);

        assert!(players[0].generic_portrait, "blank A is shared → generic");
        assert!(
            players[1].generic_portrait,
            "blank B is byte-identical → generic"
        );
        assert!(players[2].generic_portrait, "third blank sharer → generic");
        assert!(!players[3].generic_portrait, "unique portrait → real");
        assert!(!players[4].generic_portrait, "no portrait → not generic");

        // has_any reflects it: the blank-only players have nothing worth exporting.
        assert!(!players[0].has_any());
        assert!(players[3].has_any());

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn outfit_output_name_replaces_team_with_slug() {
        // Jersey / pant rasters carry the team at index 2; the slug leads and the set+variant stay.
        assert_eq!(
            outfit_output_name("detroit_red_wings", "jersey_0_10_0_cm", 2, 10),
            "detroit_red_wings_jersey_0_0_cm"
        );
        assert_eq!(
            outfit_output_name("detroit_red_wings", "font_0_10_1_sm", 2, 10),
            "detroit_red_wings_font_0_1_sm"
        );
        // Socks carry the team at index 3.
        assert_eq!(
            outfit_output_name("detroit_red_wings", "sock_0_0_10_0_0_cm", 3, 10),
            "detroit_red_wings_sock_0_0_0_0_cm"
        );
        // If the token at the index isn't this team's id, the stem is kept intact (id-bearing but
        // still correct) — guards a title whose raster names don't match its filename layout.
        assert_eq!(
            outfit_output_name("detroit_red_wings", "jersey_0_7_0_cm", 2, 10),
            "detroit_red_wings_jersey_0_7_0_cm"
        );
    }

    /// EA jersey/equipment texlibs bundle a colour map and a **same-size** grayscale spec map, so the
    /// visible-raster pick must prefer the colour map (`_cm`) — otherwise the jersey previews/writes
    /// as the grayscale spec map (the reported bug). This guards the ranking that
    /// [`primary_raster_index`] pairs ahead of area (end-to-end pick verified against the real bundle:
    /// a 1024² `_cm` tied in area with a 1024² `_sm` selects the `_cm`).
    #[test]
    fn colour_map_raster_outranks_same_size_spec() {
        assert_eq!(
            raster_colour_rank("jersey_0_10_0_cm.Raster"),
            2,
            "colour map ranks top"
        );
        assert_eq!(
            raster_colour_rank("jersey_0_10_0_sm.Raster"),
            0,
            "spec map ranks bottom"
        );
        assert_eq!(
            raster_colour_rank("jersey_0_10_0_nm.Raster"),
            0,
            "normal map ranks bottom"
        );
        assert_eq!(
            raster_colour_rank("logo_0.Raster"),
            1,
            "unsuffixed is neutral"
        );

        // The composite selection key `primary_raster_index` maximises is `(colour_rank, area)`. At
        // equal area the colour map must win even though the spec map is listed last (where plain
        // max-by-area — which keeps the *last* equal max — would land).
        let key = |name: &str, area: u64| (raster_colour_rank(name), area);
        let sz = 1024u64 * 1024;
        assert!(
            key("jersey_0_10_0_cm.Raster", sz) > key("jersey_0_10_0_sm.Raster", sz),
            "colour map beats a same-size spec map"
        );
        // A colour map still wins when it's the smaller of the two (real bundles ship a full-res cm).
        assert!(key("jersey_0_10_0_cm.Raster", 512 * 512) > key("jersey_0_10_0_sm.Raster", sz));
    }

    #[test]
    fn outfit_label_is_team_scoped_and_human() {
        // Team token dropped (surface is already team-scoped), piece word becomes the heading, the
        // map code becomes a word, set/variant digits stay.
        assert_eq!(
            outfit_label("jersey", "jersey_0_10_0_cm", 2, 10),
            "Jersey · 0 · 0 · color map"
        );
        assert_eq!(
            outfit_label("pant", "pant_1_10_2_nm", 2, 10),
            "Pant · 1 · 2 · normal map"
        );
        // A texlib bundle stem: the `texlib_` lead is dropped like a piece word.
        assert_eq!(
            outfit_label("jersey", "texlib_0_10_0_cm", 2, 10),
            "Jersey · 0 · 0 · color map"
        );
        // No map suffix → just piece + digits.
        assert_eq!(outfit_label("sock", "sock_0_10_1", 2, 10), "Sock · 0 · 1");
    }

    /// `decode_source_image_bytes` (the nameplate editor's save path decodes the baked PNG) must
    /// round-trip a PNG back to the same pixels — the bytes twin of `decode_source_image`.
    #[test]
    fn decode_source_image_bytes_round_trips_png() {
        let (w, h) = (8u32, 6u32);
        let src = DecodedImage {
            width: w,
            height: h,
            rgba: gradient_rgba(w, h),
        };
        let png = src.to_png().expect("encode png");
        let back = decode_source_image_bytes(&png).expect("decode png bytes");
        assert_eq!((back.width, back.height), (w, h));
        assert_eq!(back.rgba, src.rgba, "pixels survive the PNG round-trip");
        // Garbage is a clean error, not a panic.
        assert!(decode_source_image_bytes(b"not a png").is_err());
    }

    /// `list_team_outfits` scans the RenderWare `jersey/pant/sock` folders by *name only* (no decode),
    /// derives the team-id token position across the roster, and returns just the requested team's
    /// files, grouped-ready and human-labelled. Uses empty `.rx2` files — the scan never parses them.
    #[test]
    fn list_team_outfits_filters_to_one_team() {
        let tmp = std::env::temp_dir().join("nhlms_list_team_outfits_test");
        let _ = std::fs::remove_dir_all(&tmp);
        // Four teams' jerseys (so the team-id token position is derivable), plus pant+sock for team 10.
        for (folder, teams) in [
            ("jersey", &[10u64, 11, 12, 13][..]),
            ("pant", &[10, 11, 12, 13][..]),
            ("sock", &[10, 11, 12, 13][..]),
        ] {
            let dir = tmp.join("rendering").join(folder);
            std::fs::create_dir_all(&dir).unwrap();
            for &t in teams {
                std::fs::write(dir.join(format!("{folder}_0_{t}_0_cm.rx2")), b"").unwrap();
            }
        }
        // A mesh-style file (bare index tail, no map suffix) must be ignored by the scan.
        std::fs::write(
            tmp.join("rendering")
                .join("jersey")
                .join("jersey_0_10_0_0.rx2"),
            b"",
        )
        .unwrap();
        // Nameplate sheets live in the jersey folder too, keyed the same way.
        for t in [10u64, 11, 12, 13] {
            std::fs::write(
                tmp.join("rendering")
                    .join("jersey")
                    .join(format!("name_0_{t}_0_cm.rx2")),
                b"",
            )
            .unwrap();
        }

        let teams: Vec<OutfitTeam> = [10u64, 11, 12, 13]
            .iter()
            .map(|&id| OutfitTeam {
                team_id: id,
                slug: format!("team_slug_{id}"),
                abbrev: format!("t{id}"),
                art_ids: Vec::new(),
            })
            .collect();

        let got = list_team_outfits(&tmp.join("rendering"), &teams, 10);
        // jersey + pant + sock + nameplate for team 10 (not the mesh file, not other teams).
        assert_eq!(
            got.len(),
            4,
            "team 10 outfits, got {:?}",
            got.iter().map(|o| &o.file_name).collect::<Vec<_>>()
        );
        assert!(got
            .iter()
            .any(|o| o.piece == "jersey" && o.file_name == "jersey_0_10_0_cm.rx2"));
        assert!(got.iter().any(|o| o.piece == "pant"));
        assert!(got.iter().any(|o| o.piece == "sock"));
        // No other team leaked in.
        assert!(got.iter().all(|o| o.file_name.contains("_10_")));
        // The `name_*` sheet is re-tagged as a nameplate (own piece + flag), not a paintable jersey.
        let np = got
            .iter()
            .find(|o| o.file_name == "name_0_10_0_cm.rx2")
            .unwrap();
        assert!(np.is_nameplate, "name_* sheet flagged as nameplate");
        assert_eq!(np.piece, "name");
        // Paintable jersey body is not a nameplate.
        let jersey = got.iter().find(|o| o.piece == "jersey").unwrap();
        assert!(!jersey.is_nameplate);
        assert_eq!(jersey.label, "Jersey · 0 · 0 · color map");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn team_id_at_parses_the_indexed_token() {
        assert_eq!(team_id_at("texlib_0_10_0", 2), Some(10));
        assert_eq!(team_id_at("sock_0_0_10_1_0_cm", 3), Some(10));
        assert_eq!(team_id_at("texlib_0_999", 2), Some(999)); // the special/all-star slot
        assert_eq!(team_id_at("name_1_130_1_cm", 2), Some(130)); // not a team id — caller filters
        assert_eq!(team_id_at("texlib_0", 2), None);
    }

    #[test]
    fn ps4_ambiguous_outfits_require_explicit_candidate_browsing() {
        let teams = vec![
            OutfitTeam { team_id:0,slug:"anaheim_ducks".into(),abbrev:"ANA".into(),art_ids:vec![] },
            OutfitTeam { team_id:285,slug:"anaheim_ducks_alumni".into(),abbrev:"ANA".into(),art_ids:vec![] },
        ];
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("nhlms-outfit-candidates-{}-{stamp}",std::process::id()));
        let player = root.join("characters/player");
        std::fs::create_dir_all(&player).unwrap();
        // Listing is metadata-only; raster decoding is covered by the native GNF tests.
        std::fs::write(player.join("jersey_nineties_anaheim_away_cm.gnf"),[]).unwrap();
        std::fs::write(player.join("jersey_anaheim_ducks_alumni_home_cm.gnf"),[]).unwrap();
        assert!(list_team_outfits(&root,&teams,0).is_empty());
        let candidates = list_team_outfit_candidates(&root,&teams,0);
        assert_eq!(candidates.len(),1,"an explicitly resolved alumni texture must not become a current-team candidate");
        assert!(candidates[0].label.starts_with("Unverified filename match"));
        assert!(ps4_output_stem("jersey_nineties_anaheim_away_cm",&build_ps4_team_resolver(&teams)).is_none(),"export must still refuse ambiguous ownership");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn ps4_outfit_naming_resolves_team_and_keeps_variant() {
        let teams = vec![
            OutfitTeam {
                team_id: 10,
                slug: "detroit_red_wings".into(),
                abbrev: "DET".into(),
                art_ids: Vec::new(),
            },
            OutfitTeam {
                team_id: 17,
                slug: "new_jersey_devils".into(),
                abbrev: "NJ".into(),
                art_ids: Vec::new(),
            },
            OutfitTeam {
                team_id: 13,
                slug: "los_angeles_kings".into(),
                abbrev: "LA".into(),
                art_ids: Vec::new(),
            },
            OutfitTeam {
                team_id: 19,
                slug: "new_york_rangers".into(),
                abbrev: "NYR".into(),
                art_ids: Vec::new(),
            },
            OutfitTeam {
                team_id: 18,
                slug: "new_york_islanders".into(),
                abbrev: "NYI".into(),
                art_ids: Vec::new(),
            },
        ];
        let r = build_ps4_team_resolver(&teams);
        let name = |s: &str| ps4_output_stem(s, &r);

        // City slug → full DB name; the era + variant name + map stay in place.
        assert_eq!(
            name("jersey_rbk_detroit_home_cm").as_deref(),
            Some("detroit_red_wings_jersey_rbk_home_cm")
        );
        assert_eq!(
            name("pant_nineties_detroit_away_cm").as_deref(),
            Some("detroit_red_wings_pant_nineties_away_cm")
        );
        // Squished multi-word cities resolve via normalized matching.
        assert_eq!(
            name("jersey_nineties_newjersey_third_cm").as_deref(),
            Some("new_jersey_devils_jersey_nineties_third_cm")
        );
        assert_eq!(
            name("sock_nineties_losangeles_home_cm").as_deref(),
            Some("los_angeles_kings_sock_nineties_home_cm")
        );
        // NY teams key by abbreviation; each stays distinct.
        assert_eq!(
            name("jersey_nineties_nyr_home_cm").as_deref(),
            Some("new_york_rangers_jersey_nineties_home_cm")
        );
        assert_eq!(
            name("jersey_nineties_nyi_away_cm").as_deref(),
            Some("new_york_islanders_jersey_nineties_away_cm")
        );
        // A club not in this DB (European/junior) resolves to nothing → caller keeps the native name.
        assert_eq!(name("jersey_rbk_adler_mannheim_home_cm"), None);
        // Alaska Aces isn't in this DB either — still None (exported under its native stem upstream).
        assert_eq!(name("sock_rbk_alaska_home_cm"), None);
        // An extra descriptor token (sock left/right) is preserved around the resolved team.
        assert_eq!(
            name("sock_nineties_left_detroit_away_cm").as_deref(),
            Some("detroit_red_wings_sock_nineties_left_away_cm")
        );
    }

    #[test]
    fn outfit_texture_file_accepts_bundles_and_loose_rasters_not_meshes() {
        // NHL 09+ texlib bundles.
        assert!(outfit_texture_file("texlib_0_10_0.rx2"));
        // NHL 07/08 loose rasters (map suffix in the filename).
        assert!(outfit_texture_file("jersey_0_10_0_cm.rx2"));
        assert!(outfit_texture_file("jersey_0_10_0_0_nm.rx2"));
        assert!(outfit_texture_file("pant_0_0_0_cm.rx2"));
        assert!(outfit_texture_file("sock_0_0_10_0_0_cm.rx2"));
        assert!(outfit_texture_file("font_0_10_0_sm.rx2"));
        // Mesh files (bare trailing index, no map suffix) and non-rx2 are rejected.
        assert!(!outfit_texture_file("pant_0_0_0_0_0_0.rx2"));
        assert!(!outfit_texture_file("jersey_0_10_2.rx2"));
        assert!(!outfit_texture_file("player_10057.glb"));
    }

    #[test]
    fn derive_team_index_picks_the_widest_spanning_position() {
        let by_id: HashMap<u64, (u64, String)> =
            (0u64..32).map(|i| (i, (i, "team".to_string()))).collect();
        // NHL 09-style jersey filenames: set at index 1 (0/1), team at index 2 (0..31), variant at 3.
        let jerseys: Vec<String> = (0..32)
            .flat_map(|team| {
                [0, 1]
                    .into_iter()
                    .flat_map(move |set| (0..3).map(move |v| format!("texlib_{set}_{team}_{v}")))
            })
            .collect();
        assert_eq!(derive_team_index(&jerseys, &by_id), Some(2));

        // NHL 09-style sock filenames: team at index 3.
        let socks: Vec<String> = (0..32)
            .map(|team| format!("sock_0_0_{team}_0_0_cm"))
            .collect();
        assert_eq!(derive_team_index(&socks, &by_id), Some(3));

        // A folder that isn't team-keyed (all ids out of range) yields nothing rather than a guess.
        let junk: Vec<String> = (0..10).map(|i| format!("texlib_0_{}_0", 900 + i)).collect();
        assert_eq!(derive_team_index(&junk, &by_id), None);
    }

    #[test]
    fn classify_asset_kind_by_name() {
        let k = |s: &str| classify_asset_kind(Path::new(s));
        // Meshes.
        assert_eq!(k("player_10057.rx2"), Some(AssetKind::Mesh));
        assert_eq!(k("hair_10000.rx2"), Some(AssetKind::Mesh));
        assert_eq!(k("generic_player_5.mcd"), Some(AssetKind::Mesh));
        // Textures: the texlib bundle and NHL 10's loose per-map rasters.
        assert_eq!(k("texlib_67.rx2"), Some(AssetKind::Texture));
        assert_eq!(k("head_10000_cm.rx2"), Some(AssetKind::Texture));
        assert_eq!(k("head_10000_0_0_0_nm.rx2"), Some(AssetKind::Texture));
        assert_eq!(k("head_cm.gnf"), Some(AssetKind::Texture));
        // Portraits.
        assert_eq!(k("p1234.big"), Some(AssetKind::Portrait));
        assert_eq!(k("p1234.png"), Some(AssetKind::Portrait));
        assert_eq!(k("p1234.gnf"), Some(AssetKind::Portrait));
        // Unrecognized.
        assert_eq!(k("nhlng.db"), None);
        assert_eq!(k("notes.txt"), None);
    }

    #[test]
    fn safe_join_rel_refuses_escape_and_allows_root() {
        let root = Path::new("C:/game/USRDIR");
        // Empty rel = the root itself (the browser opens there).
        assert_eq!(safe_join_rel(root, ""), Some(root.to_path_buf()));
        assert_eq!(
            safe_join_rel(root, "rendering/player"),
            Some(root.join("rendering").join("player"))
        );
        // Traversal (`..`) and drive-prefixed paths are refused.
        assert_eq!(safe_join_rel(root, ".."), None);
        assert_eq!(safe_join_rel(root, "rendering/../../etc"), None);
        assert_eq!(safe_join_rel(root, "D:/other"), None);
        // A leading slash just drops to an empty (skipped) component, so it resolves *within* root
        // rather than escaping — the same containment the extractor's guard relies on.
        assert_eq!(
            safe_join_rel(root, "/etc/passwd"),
            Some(root.join("etc").join("passwd"))
        );
    }

    #[test]
    fn strip_ext_keeps_dir_prefix() {
        assert_eq!(
            strip_ext("rendering/player/player_10057.rx2"),
            "rendering/player/player_10057"
        );
        assert_eq!(strip_ext("head_10000_cm.rx2"), "head_10000_cm");
        // No extension → unchanged; a dot only in a directory name is left alone.
        assert_eq!(strip_ext("some/dir.name/file"), "some/dir.name/file");
    }

    #[test]
    fn export_folder_expands_and_mirrors_layout() {
        let tmp = std::env::temp_dir().join(format!("nms_fx_{}", std::process::id()));
        let root = tmp.join("install");
        let sub = root.join("rendering").join("misc");
        std::fs::create_dir_all(&sub).unwrap();
        // Two unrecognized files in a nested folder — copy_through preserves the tree.
        std::fs::write(sub.join("readme.txt"), b"hello").unwrap();
        std::fs::write(root.join("top.dat"), b"xyz").unwrap();

        let out = tmp.join("out");
        let opts = FileExportOptions {
            meshes_glb: true,
            portraits_png: true,
            textures: true,
            copy_unrecognized: true,
            crop_portraits: false,
        };
        let summary = export_asset_files(
            &root,
            &["rendering".to_string(), "top.dat".to_string()],
            &out,
            opts,
        )
        .unwrap();

        assert_eq!(summary.files_considered, 2);
        assert_eq!(summary.raw_copied, 2);
        assert!(out
            .join("rendering")
            .join("misc")
            .join("readme.txt")
            .is_file());
        assert!(out.join("top.dat").is_file());

        // With copy_unrecognized off, the same selection skips instead of copying.
        let out2 = tmp.join("out2");
        let summary2 = export_asset_files(
            &root,
            &["rendering".to_string()],
            &out2,
            FileExportOptions {
                copy_unrecognized: false,
                ..opts
            },
        )
        .unwrap();
        assert_eq!(summary2.skipped_unrecognized, 1);
        assert_eq!(summary2.raw_copied, 0);

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// Phase 1 gate proof (mount-gated): across the committed title anchors + the
    /// real rendering mounts, the file-truth rule resolves a head mesh for every
    /// player whose non-pool `pPaD` has a file on disk — including the divergent-id
    /// bespoke heads (`pPaD != lOnr`) the old equality silently dropped. Legacy was
    /// the equality's calibration anchor so it barely moves there; the recovery
    /// shows up on the titles whose head-id and portrait-id spaces diverge. Reads
    /// committed DBs (CI-safe); skips titles whose rendering tree isn't mounted.
    #[test]
    fn phase1_file_truth_recovers_divergent_id_heads() {
        use tdb_core::TdbFile;
        use tdb_defs::FieldMetaIndex;

        // (title, rendering mount) — anchor DB is anchors/xbox360/<title>/databases.
        let titles: &[(&str, &str)] = &[
            (
                "nhl08",
                r"X:\Games\Emulation\Roms\XBOX360\NHL 08\NHL 08 (USA)\rendering",
            ),
            (
                "nhl09",
                r"X:\Games\Emulation\Roms\XBOX360\NHL 09\NHL 09 (USA)\rendering",
            ),
            (
                "nhl10",
                r"X:\Games\Emulation\Roms\XBOX360\NHL 10\NHL 10\rendering",
            ),
            (
                "nhl11",
                r"X:\Games\Emulation\Roms\XBOX360\NHL 11\NHL 11\rendering",
            ),
            (
                "nhl12",
                r"X:\Games\Emulation\Roms\XBOX360\NHL 12\NHL 12\rendering",
            ),
            (
                "nhl13",
                r"X:\Games\Emulation\Roms\XBOX360\NHL 13\NHL 13\rendering",
            ),
            (
                "nhl14",
                r"X:\Games\Emulation\Roms\XBOX360\NHL 14\NHL 14\rendering",
            ),
            (
                "nhl15",
                r"X:\Games\Emulation\Roms\XBOX360\NHL 15\NHL 15\rendering",
            ),
            (
                "nhllegacy",
                r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\rendering",
            ),
        ];
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let mut ran = 0usize;
        let mut total_recovered = 0usize;

        for (title, rendering) in titles {
            let rendering = std::path::Path::new(rendering);
            let base = repo
                .join("anchors")
                .join("xbox360")
                .join(title)
                .join("databases");
            if !rendering.join("player").is_dir() || !base.join("nhlng.db").is_file() {
                eprintln!("{title}: skip (mount or anchor absent)");
                continue;
            }
            ran += 1;
            let db_bytes = std::fs::read(base.join("nhlng.db")).unwrap();
            let db = TdbFile::parse(&db_bytes).expect("parse anchor");
            let meta = FieldMetaIndex::from_xml(
                &std::fs::read_to_string(base.join("nhlng-meta.xml")).unwrap(),
            )
            .expect("meta");
            let rendering_str = rendering.to_string_lossy().into_owned();
            let roots = AssetRoots::resolve(std::path::Path::new("/unused"), Platform::Xbox)
                .with_overrides(Some(&rendering_str), None);
            let players = scan_player_assets(&db, &meta, &roots).expect("scan");

            let with_mesh = players.iter().filter(|p| p.mesh_path.is_some()).count();
            let equality_gate = players
                .iter()
                .filter(|p| p.head_id == p.portrait_id && p.head_id < POOL_HEAD_MIN)
                .count();
            // Heads the uniqueness rule recovers that the equality would have
            // dropped: a mesh resolved for a head whose id differs from the
            // portrait id (the divergent-id bespoke heads, e.g. Ovechkin).
            let recovered = players
                .iter()
                .filter(|p| p.mesh_path.is_some() && p.head_id != p.portrait_id)
                .count();
            total_recovered += recovered;
            eprintln!(
                "{title}: {} players — {with_mesh} own-head ({} shared, {} pool skipped) — equality would pick {equality_gate}, recovered divergent={recovered}",
                players.len(),
                players.iter().filter(|p| p.head_shared).count(),
                players.iter().filter(|p| p.pool_head).count(),
            );

            // Invariants on every title:
            // 1. A resolved head is never a shared or pool head — no generic is
            //    ever attributed to a single player.
            assert!(
                players
                    .iter()
                    .all(|p| p.mesh_path.is_none() || (!p.pool_head && !p.head_shared)),
                "{title}: a shared/pool head was attributed to a player"
            );
            // 2. A resolved head id is used by exactly one *person* (duplicate
            //    all-star/legends entries of the same player are allowed to share it).
            {
                use std::collections::{HashMap as HM, HashSet};
                let mut people: HM<u64, HashSet<&str>> = HM::new();
                for p in &players {
                    people
                        .entry(p.head_id)
                        .or_default()
                        .insert(p.full_name.as_str());
                }
                for p in players.iter().filter(|p| p.mesh_path.is_some()) {
                    assert_eq!(
                        people[&p.head_id].len(),
                        1,
                        "{title}: head {} of {} is shared by several people",
                        p.head_id,
                        p.full_name
                    );
                }
            }
            // 3. Every resolved path is a real file.
            for p in players.iter().filter(|p| p.mesh_path.is_some()) {
                assert!(
                    std::path::Path::new(p.mesh_path.as_deref().unwrap()).is_file(),
                    "{title}: {} mesh_path missing on disk",
                    p.full_name
                );
            }
            // 4. The bespoke set is a sane fraction of the roster — not the
            //    over-attribution the raw pool-threshold rule produced (which
            //    named thousands of shared heads after individuals).
            assert!(
                with_mesh < players.len() / 2,
                "{title}: {with_mesh}/{} resolved — over-attribution, not bespoke-only",
                players.len()
            );
        }

        if ran == 0 {
            eprintln!("skipping: no title mounts present");
            return;
        }
        // The whole point of Phase 1: somewhere across the mounted titles, real
        // heads are recovered that the equality gate would have dropped.
        assert!(
            total_recovered > 0,
            "no divergent-id heads recovered across {ran} titles — the fix changed nothing"
        );
    }

    /// Phase 3.4 (22 Sep 2026) — the **pool-head floor is measured per file**, not fixed at 60 000.
    ///
    /// Research 2026-09-21 pm handoff R53 §1 measured the gen7 pool ranges: 10 000+ on NHL 09–13
    /// (10 000–10 099 on 09 / 10, 10 000–10 181 on 11 / 13, to 30 100 on 12) and 60 000+ from
    /// NHL 14 on. Studio's fixed floor labelled every NHL 09–13 pool head as *not* pool, which also
    /// let the mesh resolver treat a shared generic as a player's own head unless the uniqueness
    /// rule caught it.
    ///
    /// On the committed anchors, per title: the pool population is non-empty and every pool head is
    /// **shared** (referenced by more than one person) — the two independent signals must agree —
    /// and no player whose head is in the pool is credited with an own mesh.
    #[test]
    fn the_pool_head_floor_follows_the_title_era() {
        use std::path::Path;
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        // (anchor, expected floor) — the early gen7 titles have no 60 000+ head ids at all, the
        // later ones do, and that is exactly what the per-file rule keys on.
        for (dir, game, expect_floor) in [
            ("xbox360", "nhl09", POOL_HEAD_MIN_EARLY_GEN7),
            ("xbox360", "nhl11", POOL_HEAD_MIN_EARLY_GEN7),
            ("xbox360", "nhl13", POOL_HEAD_MIN_EARLY_GEN7),
            ("xbox360", "nhl15", POOL_HEAD_MIN),
            ("xbox360", "nhllegacy", POOL_HEAD_MIN),
        ] {
            let base = repo.join(format!("anchors/{dir}/{game}/databases"));
            let db_bytes = std::fs::read(base.join("nhlng.db")).unwrap_or_else(|e| panic!("{game}: {e}"));
            let db = TdbFile::parse(&db_bytes).expect("parse anchor");
            let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(base.join("nhlng-meta.xml")).unwrap()).expect("meta");
            // No rendering tree: file resolution is irrelevant here, the classification is not.
            let roots = AssetRoots::resolve(Path::new("/unused"), Platform::Xbox);
            let players = scan_player_assets(&db, &meta, &roots).expect("scan");

            let ids: Vec<u64> = players.iter().map(|p| p.head_id).collect();
            let has_high = ids.iter().any(|i| *i >= POOL_HEAD_MIN);
            assert_eq!(has_high, expect_floor == POOL_HEAD_MIN, "{game}: 60 000+ head ids present = {has_high}, but the expected floor is {expect_floor}");

            let pool: Vec<&PlayerAssets> = players.iter().filter(|p| p.pool_head).collect();
            assert!(!pool.is_empty(), "{game}: the pool population is empty — the floor did not match this era");
            assert!(pool.iter().all(|p| p.head_id >= expect_floor), "{game}: a pool head is below the era floor");
            // The era floor and the portable uniqueness signal are independent and agree on the
            // overwhelming majority. They are not identical, and the difference is the point: a
            // generic head that happens to be worn by exactly one roster player (NHL 09's 10097,
            // for instance) is invisible to the uniqueness rule but is still a pool head by its id.
            // Catching those is what the era floor adds; the check is therefore "almost all", with
            // the exceptions named.
            let mut unshared: Vec<u64> = pool.iter().filter(|p| !p.head_shared).map(|p| p.head_id).collect();
            unshared.sort_unstable();
            unshared.dedup();
            let pool_ids: std::collections::BTreeSet<u64> = pool.iter().map(|p| p.head_id).collect();
            // Measured over *players*, not distinct ids: a handful of singly-worn generic ids is
            // normal (6 % of Legacy's 201 pool ids), but they account for a negligible share of the
            // players wearing pool heads. A floor from the wrong era would put most of the roster's
            // bespoke heads in this bucket and the share would collapse.
            let on_shared = pool.iter().filter(|p| p.head_shared).count();
            assert!(
                on_shared * 100 >= pool.len() * 99,
                "{game}: only {on_shared} of {} players with a pool head are on a shared head — the floor does not look like this era's ({} singly-worn ids of {})",
                pool.len(),
                unshared.len(),
                pool_ids.len()
            );
            if !unshared.is_empty() {
                eprintln!("{game}: {} pool head id(s) worn by exactly one player (the uniqueness rule alone would miss them): {unshared:?}", unshared.len());
            }
            // And a pool head is never credited as the player's own mesh.
            assert!(pool.iter().all(|p| p.mesh_path.is_none()), "{game}: a pool head resolved as an own mesh");

            // The regression the fix closes: with the old fixed floor the early titles had *no*
            // pool population at all.
            if expect_floor == POOL_HEAD_MIN_EARLY_GEN7 {
                assert_eq!(players.iter().filter(|p| p.head_id >= POOL_HEAD_MIN).count(), 0, "{game}: the old 60 000 floor would have found no pool heads");
            }
            eprintln!("{game}: floor {expect_floor}, {} pool heads of {} players", pool.len(), players.len());
        }
    }

}

#[cfg(test)]
mod ps3_outfit_export_qa {
    use super::*;

    #[test]
    #[ignore = "requires NHL_QA_OUTFIT_PS3 and fresh NHL_QA_OUTFIT_EXPORT; reads source only"]
    fn native_ps3_bundle_exports_every_raster() {
        let source = PathBuf::from(std::env::var("NHL_QA_OUTFIT_PS3").unwrap());
        let out = PathBuf::from(std::env::var("NHL_QA_OUTFIT_EXPORT").unwrap());
        assert!(!out.exists());
        let original = std::fs::read(&source).unwrap();
        let bytes = read_asset_file(&source).unwrap();
        let lib = tdb_rx2::texlib_ps3::parse(&bytes).unwrap();
        assert!(lib.textures.len() >= 4);
        let item = OutfitWork { path: source.clone(), file_stem: "texlib_0_0_0".into(), slug: "anaheim".into(), team_id: 0, token: 0, team_idx: 2, folder: "jersey" };
        let mut result = OutfitExportSummary::default();
        process_one_outfit(&item, &out, &mut result);
        assert!(result.failures.is_empty(), "{:?}", result.failures);
        assert_eq!(result.jerseys_written as usize, lib.textures.len());
        for (index, texture) in lib.textures.iter().enumerate() {
            let stem = texture.name.split('.').next().unwrap();
            let path = out.join("jersey").join(format!("{}.png", outfit_output_name("anaheim", stem, 2, 0)));
            let (_, preview) = decode_texture_file(&source, Some(index)).unwrap();
            assert_eq!(std::fs::read(path).unwrap(), preview.to_png_fast().unwrap());
        }
        assert_eq!(std::fs::read(source).unwrap(), original);
    }
}
