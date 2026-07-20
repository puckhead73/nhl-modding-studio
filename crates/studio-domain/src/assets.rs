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

use crate::{DomainError, Platform, Result};

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
        let anchor = anchor_dir(install_dir);
        AssetRoots {
            rendering: art_root(&anchor, "rendering").to_string_lossy().into_owned(),
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
    pub fn extra_head_mesh_paths(&self, head_id: u64, _name_slug: Option<&str>) -> Vec<PathBuf> {
        // PS4 heads bundle hair (`geo_haircapShape`/`hair_cardsShape`) as
        // submeshes inside the one `.mcd`, so there are no companion meshes.
        if self.platform == Platform::Ps4 {
            return Vec::new();
        }
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
    let nhl_data = ps4_nhl_data_root(install_dir);
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
        } else if ch.is_whitespace() {
            if !slug.ends_with('_') {
                slug.push('_');
            }
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
    if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case("gnf")) {
        return false;
    }
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    let lower = stem.to_ascii_lowercase();
    for part in ["head_", "hair_"] {
        if let Some(rest) = lower.strip_prefix(part) {
            if let Some(map) = rest.strip_prefix(name_slug).and_then(|r| r.strip_prefix('_')) {
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

        // The PS4 art filename slug (`first_last`) is distinct from the export
        // slug above (`last_first_dd_mm_yyyy`). Only needed on PS4.
        let ps4_slug = (roots.platform() == Platform::Ps4)
            .then(|| ps4_name_slug(first.trim(), last.trim()))
            .flatten();
        // "This player's own head": on the RenderWare titles, `pPaD == lOnr`; on
        // PS4, the existence of a name-slug `{slug}.mcd` (pool heads have only a
        // numeric `generic_player_*`).
        let bespoke_head = match roots.platform() {
            Platform::Ps4 => roots.ps4_bespoke_mesh(ps4_slug.as_deref()).is_some(),
            _ => head_id == portrait_id,
        };
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
            mesh_path: path_str(
                bespoke_head
                    .then(|| roots.mesh_path(head_id, ps4_slug.as_deref()))
                    .flatten(),
            ),
            texture_paths: if bespoke_head {
                roots
                    .texture_paths(head_id, ps4_slug.as_deref())
                    .into_iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect()
            } else {
                Vec::new()
            },
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
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("mcd")) {
        return tdb_rx2::mcd::parse_player_mesh(bytes).map_err(|e| err(e.to_string()));
    }
    if preview {
        tdb_rx2::playermesh::parse_player_mesh_preview(bytes).map_err(|e| err(e.to_string()))
    } else {
        tdb_rx2::playermesh::parse_player_mesh(bytes).map_err(|e| err(e.to_string()))
    }
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
        // BC4/BC5 (single/dual-channel — normal/roughness maps) decode is not yet
        // correct on real data (the index bits come out scrambled); skipped so a
        // noisy thumbnail never reaches the UI. The diffuse `cm` (BC1) and hair
        // (BC3) maps — everything the model actually samples — decode cleanly.
        BcFormat::Bc4 | BcFormat::Bc5 => Err(DomainError::Extract(format!(
            "{:?} GNF decode not yet supported",
            tex.format
        ))),
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
    /// Portraits skipped because they're a generic placeholder shared by many
    /// un-portraited players, not the player's own — see [`mark_generic_portraits`].
    pub portraits_skipped_generic: usize,
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

        if let Some(src) = p.mesh_path.as_deref().filter(|_| opts.meshes_glb) {
            match read_asset_file(Path::new(src))
                .and_then(|b| parse_head_mesh(Path::new(src), &b, false)) {
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
                        .and_then(|tex| {
                            gnf_to_rgba(&tex).map(|rgba| (tex.width, tex.height, rgba))
                        })
                        .and_then(|(w, h, rgba)| DecodedImage { width: w, height: h, rgba }.to_png())
                    {
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
                match read_asset_file(Path::new(src))
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

        let roots = AssetRoots::resolve(&tmp, Platform::Xbox);
        assert_eq!(roots.rendering, game.join("rendering").to_string_lossy());
        assert_eq!(roots.fe, game.join("fe").to_string_lossy());

        std::fs::remove_dir_all(&tmp).ok();
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
        assert_eq!(roots.rendering, compiled.join("rendering").to_string_lossy());
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
        assert!(mean_r > mean_g && mean_g > mean_b, "skin tone: r>g>b, got {mean_r},{mean_g},{mean_b}");
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
        assert!(seam_ratio < 1.6, "tile-boundary seam ratio too high: {seam_ratio:.2}");
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
        assert!(!ps4_is_texture_for(Path::new("body_alexander_ovechkin_cm.gnf"), slug));
        assert!(!ps4_is_texture_for(Path::new("head_alexander_ovechkin_cm.dds"), slug));
        assert!(!ps4_is_texture_for(Path::new("head_alexander_ovechkin_xx.gnf"), slug));
        assert!(!ps4_is_texture_for(Path::new("head_alexander_ovechkinov_cm.gnf"), slug));
    }

    /// PS4 resolves art off the kettle tree, derived by walking up from the
    /// (deep) database dir to `nhl/data`; meshes are name-slug keyed under
    /// `characters/player/`, with a numeric `generic_player_*` fallback.
    #[test]
    fn ps4_asset_roots_resolve_kettle_tree_and_bespoke_mesh() {
        let tmp = std::env::temp_dir().join(format!("nhlms_ps4_roots_{}", std::process::id()));
        let game = tmp.join("Games").join("CUSA00561");
        let nhl_data = game.join("nhl").join("data");
        let player = nhl_data.join("kettle").join("rendering").join("characters").join("player");
        std::fs::create_dir_all(&player).unwrap();
        // Portraits live under <nhl>/ui/assets/portraits (sibling of nhl/data).
        let portraits = game.join("nhl").join("ui").join("assets").join("portraits");
        std::fs::create_dir_all(&portraits).unwrap();
        std::fs::write(portraits.join("p4242.png"), b"\x89PNG stub").unwrap();
        // The dir discovery hands us is deep inside nhl/data.
        let db_dir = nhl_data.join("common").join("gamemodes").join("database").join("t3db");
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

    fn player_with_portrait(record: u32, portrait_path: Option<String>) -> PlayerAssets {
        PlayerAssets {
            bio_record: record,
            full_name: format!("Player {record}"),
            slug: Some(format!("player_{record}")),
            portrait_id: record as u64,
            head_id: 60000,
            bespoke_head: false,
            pool_head: true,
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
        assert!(players[1].generic_portrait, "blank B is byte-identical → generic");
        assert!(players[2].generic_portrait, "third blank sharer → generic");
        assert!(!players[3].generic_portrait, "unique portrait → real");
        assert!(!players[4].generic_portrait, "no portrait → not generic");

        // has_any reflects it: the blank-only players have nothing worth exporting.
        assert!(!players[0].has_any());
        assert!(players[3].has_any());

        std::fs::remove_dir_all(&tmp).ok();
    }
}
