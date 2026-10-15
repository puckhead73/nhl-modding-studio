//! Phase 2.5 (gen7 + PS4, 22 Sep 2026) — an arena's **shipped assets**, joined to the database
//! only where the research measured a join, and listed as "shipped, no established binding"
//! everywhere else.
//!
//! ## Gen7 (NHL 09 → Legacy, Xbox 360 / PS3 / Recompiled) — research 2026-09-21 pm handoff 19 (R62)
//!
//! `exhibitionarena.arenaartid` is literally the name of the venue's files:
//!
//! ```text
//! rendering/stadium/stadium_<arenaartid>_container_<n>.rx2   (.rpsgl on PS3)   100 % of rows, 09 → Legacy
//! rendering/jumbotron/texlib_<arenaartid>.rx2                                 100 % of rows, 12 → Legacy
//! ```
//!
//! measured on every arena row of every title on both consoles, with **zero** rows naming a missing
//! file. Ten to fourteen stadium ids per title belong to no row: shipped spare venues. NHL 09 / 10 /
//! 11 ship no `jumbotron/` directory at all.
//!
//! Two families one might expect on the arena are keyed by **team** art id, not by the arena:
//! `icesurface/centerlogo_<teamartid>_cm` (191 of 218 ids are team art ids) and
//! `banner/banner_<teamartid>…` (210 of 218). They are shown here through the arena's **home
//! teams**, and the card says so. `icesurface/icesurface_<id>_bm` is a **candidate** for the
//! `bouncemap` column (every row's value names a file, but `puck` and `powerring` fit the same id
//! space); the centre-line / line-art / lightmap layers and the named ice bases (`ice_nhl`,
//! `ice_intl`, …) have no established key and are listed as shipped files only.
//!
//! ## PS4 (NHL 15 → 21) — handoff 21 (R64)
//!
//! Venues are **named directories** (`rendering/stadiums/<market>/…`) and no arena column reaches
//! those names (`arenaname` never matches; `cityname` matches only 22 directories per title). The
//! binding runs through the Attrib / VLT vault, which Studio does not read yet, so on PS4 this
//! module lists the shipped venue directories and the numbered `stadiums/common/` ice families as
//! evidence, labels `arenaartid` a candidate key, and claims no binding.
//!
//! Nothing here writes; nothing claims what the console draws.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::project::Platform;

/// How an asset family relates to the database row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Binding {
    /// Measured on 100 % of rows (R62): the arena column names the file.
    Established,
    /// Keyed by the home team's art id (R62 §2), shown through the team, not the arena row.
    ViaHomeTeam,
    /// The column fits the id space but so do others (R62 §3): named, never asserted.
    Candidate,
    /// Shipped files with no established key.
    NoEstablishedBinding,
}

/// One file the join resolved (or looked for) — a family entry for one key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetFile {
    /// The key the file was resolved for (an art id, a team art id, a bounce-map id …).
    pub key: u64,
    /// Where the key came from, in words (`arenaartid`, `home team Anaheim Ducks (art id 12)`, …).
    pub key_source: String,
    pub file_stem: String,
    /// Absolute path when the file exists.
    pub path: Option<String>,
    /// `present` / `missing` / `unknown`.
    pub status: crate::kit_files::ComponentStatus,
    pub reason: Option<String>,
}

/// One asset family on the card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetFamily {
    /// `venue_model`, `jumbotron`, `centre_ice_logo`, `banners`, `bounce_map`, `ice_layers`, …
    pub family: String,
    pub label: String,
    /// The `rendering/` sub-directory.
    pub dir: String,
    pub binding: Binding,
    /// The measured or candidate rule, in words, with its source.
    pub rule: String,
    /// The files resolved for this arena (empty for `NoEstablishedBinding` families).
    pub files: Vec<AssetFile>,
    /// How many files the directory holds for this family's grammar, when it was listed.
    pub shipped_files: Option<usize>,
    /// Keys the directory ships that no arena row (or team, for team-keyed families) names — the
    /// spare pool. Only computed for `Established` families.
    pub spare_keys: Vec<u64>,
    pub notes: Vec<String>,
}

/// What the title / platform's arena assets are measured to look like.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ArenaAssetContract {
    /// Gen7 file-name rule (R62); `jumbotron` false on NHL 09 / 10 / 11.
    Gen7 { extension: String, jumbotron_shipped: bool, source: String },
    /// PS4: named venue directories, vault-bound (R64); Studio does not read the vault.
    Ps4VaultReference { reason: String },
    /// Nothing measured (NHL 07 / 08, unknown titles).
    Unmeasured { reason: String },
}

impl ArenaAssetContract {
    pub fn describe(&self) -> String {
        match self {
            Self::Gen7 { extension, jumbotron_shipped, source } => format!(
                "venue model `stadium/stadium_<arenaartid>_container_<n>.{extension}`{}; centre-ice logo `icesurface/centerlogo_<team art id>_cm` and banners `banner/banner_<team art id>` through the home team; bounce map `icesurface/icesurface_<bouncemap>_bm` as a candidate ({source})",
                if *jumbotron_shipped { format!(" and jumbotron `jumbotron/texlib_<arenaartid>.{extension}`") } else { "; no `jumbotron/` directory ships on this title".to_string() }
            ),
            Self::Ps4VaultReference { reason } | Self::Unmeasured { reason } => reason.clone(),
        }
    }
}

/// The contract for a title / platform, decided from identity alone.
pub fn arena_asset_contract(game_id: Option<&str>, platform: Platform) -> ArenaAssetContract {
    let extension = match platform {
        Platform::Ps3 => "rpsgl",
        Platform::Xbox | Platform::Recomp => "rx2",
        Platform::Ps4 => {
            return ArenaAssetContract::Ps4VaultReference {
                reason: "PS4 venues are named directories under `rendering/stadiums/` selected through the Attrib / VLT vault (research 2026-09-21 pm handoff R64): no arena column names them, and Studio does not read the vault yet — the shipped venue directories and the numbered `stadiums/common/` ice families are listed as evidence only".into(),
            }
        }
    };
    match game_id {
        Some("nhl09" | "nhl10" | "nhl11") => ArenaAssetContract::Gen7 {
            extension: extension.into(),
            jumbotron_shipped: false,
            source: "research 2026-09-21 pm handoff R62 §1: 100 % of arena rows on NHL 09 → Legacy, both consoles".into(),
        },
        Some("nhl12" | "nhl13" | "nhl14" | "nhl15" | "nhllegacy") => ArenaAssetContract::Gen7 {
            extension: extension.into(),
            jumbotron_shipped: true,
            source: "research 2026-09-21 pm handoff R62 §1: 100 % of arena rows on NHL 09 → Legacy, both consoles".into(),
        },
        Some(other) => ArenaAssetContract::Unmeasured { reason: format!("the arena-asset grammar of `{other}` is not measured (research R62 covers NHL 09 → Legacy, R64 the PS4 titles); nothing is joined") },
        None => ArenaAssetContract::Unmeasured { reason: "the open project has no recorded title, so the arena-asset grammar cannot be chosen; nothing is joined".into() },
    }
}

/// The database side of the join: what the arena row and its tenants carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArenaAssetInputs {
    pub arena_id: u64,
    pub arena_art_id: Option<u64>,
    /// `bouncemap` when the title's arena table carries the column.
    pub bounce_map: Option<u64>,
    /// Whether this title's team table carries the home-arena link (`kOtt.buDa`) at all — the
    /// early gen7 titles do not, so no team can be resolved as a tenant there.
    pub home_arena_link: bool,
    /// `(team name, team art id)` for every team whose home arena this is.
    pub home_teams: Vec<(String, u64)>,
    /// Every `arenaartid` value in the table — the row side of the spare-venue residual.
    pub all_arena_art_ids: BTreeSet<u64>,
    /// Every team art id in the team table — the row side of the team-keyed residuals.
    pub all_team_art_ids: BTreeSet<u64>,
}

/// The card's payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArenaAssetsView {
    pub arena_id: u64,
    pub arena_art_id: Option<u64>,
    pub contract: ArenaAssetContract,
    pub rendering_root: Option<String>,
    pub families: Vec<AssetFamily>,
    /// PS4 only: the shipped venue directories (names, member counts).
    pub venue_directories: Vec<(String, usize)>,
    /// PS4 only: the vault-level listing for this arena's `arenaartid` (research R73, 22 Sep 2026
    /// pm) — every level record carrying the id, each with its folder, its texpack keys and rink
    /// id; classed, never disambiguated silently. `None` when no level table is available.
    pub ps4_venue: Option<Ps4VenueListing>,
    pub notes: Vec<String>,
}

// ---------------------------------------------------------------------------
// PS4 vault-level listing (research R73 — read-only discovery, no reassignment)
// ---------------------------------------------------------------------------

/// One vault level record as the developer-owned table `defs/arena/ps4-venue-levels.json` carries
/// it (derived from research R73's join; provenance in the file). `id` is the value an arena row's
/// `arenaartid` must equal; `dir` is the folder name under `rendering/stadiums/` or `None` when
/// the research found no shipped folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ps4VenueLevel {
    pub id: u64,
    pub name: String,
    pub dir: Option<String>,
    pub arena_id_attr: Option<i64>,
    pub rink_id: Option<i64>,
    pub center_logo: Option<i64>,
    pub center_line: Option<i64>,
    pub ice_line: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Ps4TexpackMisses {
    #[serde(default)]
    pub center_logo: Vec<i64>,
    #[serde(default)]
    pub center_line: Vec<i64>,
    #[serde(default)]
    pub ice_line: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Ps4TexpackSetSizes {
    #[serde(default)]
    pub center_logo: usize,
    #[serde(default)]
    pub center_line: usize,
    #[serde(default)]
    pub ice_line: usize,
}

/// One title's level table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ps4VenueTitle {
    pub arena_rows_measured: usize,
    pub levels: Vec<Ps4VenueLevel>,
    #[serde(default)]
    pub texpack_misses: Ps4TexpackMisses,
    #[serde(default)]
    pub texpack_set_sizes: Ps4TexpackSetSizes,
    #[serde(default)]
    pub typed_refs: BTreeMap<String, usize>,
    #[serde(default)]
    pub dirs_without_level_name: Vec<String>,
    #[serde(default)]
    pub level_names_without_dir: Vec<String>,
    #[serde(default)]
    pub shared_ids: BTreeMap<String, Vec<String>>,
}

/// The whole table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ps4VenueLevels {
    pub provenance: String,
    pub titles: BTreeMap<String, Ps4VenueTitle>,
}

impl Ps4VenueLevels {
    /// Load `defs/arena/ps4-venue-levels.json` under `defs_root`.
    pub fn load(defs_root: &Path) -> crate::Result<Self> {
        let p = defs_root.join("arena").join("ps4-venue-levels.json");
        let text = std::fs::read_to_string(&p).map_err(|e| crate::DomainError::Validation(format!("{}: {e}", p.display())))?;
        serde_json::from_str(&text).map_err(|e| crate::DomainError::Validation(format!("{}: {e}", p.display())))
    }

    pub fn title(&self, game_id: &str) -> Option<&Ps4VenueTitle> {
        self.titles.get(game_id)
    }
}

/// A texpack key a level carries, resolved against the research-measured texpack set of the title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ps4TexpackRef {
    /// `center_logo` / `center_line` / `ice_line`.
    pub family: String,
    pub key: i64,
    /// `true` when the key is inside the title's measured set; `false` for the two known misses
    /// (research R73 §2: 1001 on NHL 17, 100 on NHL 20 / 21).
    pub in_set: bool,
}

/// How a level's folder resolved on the attached tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderStatus {
    /// `rendering/stadiums/<dir>/` exists in the attached tree.
    Present,
    /// The research names a folder but the attached tree lacks it.
    Missing,
    /// The research found no shipped folder for this level (NHL 15 `carolinageneric`).
    NoFolderShipped,
    /// No rendering tree is attached: not looked for.
    Unknown,
}

/// One candidate level for the arena's `arenaartid`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ps4VenueCandidate {
    pub level_name: String,
    /// `stadiums/<dir>` relative to the rendering root, when the research names one.
    pub dir: Option<String>,
    pub folder: FolderStatus,
    pub members: Option<usize>,
    pub texpacks: Vec<Ps4TexpackRef>,
    pub rink_id: Option<i64>,
    /// The record's own `ArenaID` attribute — **not** a second key to the arena table (R73 §2).
    pub arena_id_attr: Option<i64>,
}

/// The row's class, as research R73 §1 classes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ps4VenueClass {
    /// Exactly one level carries the id and it names a shipped folder.
    Unambiguous,
    /// Several level records carry the id: every candidate is listed, none is chosen.
    Ambiguous,
    /// One level carries the id but the research found no shipped folder for it.
    LevelWithoutFolder,
    /// No level record carries this id in the research join.
    NoLevel,
    /// The title is not in the level table.
    Unmeasured,
}

impl Ps4VenueClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unambiguous => "unambiguous",
            Self::Ambiguous => "ambiguous",
            Self::LevelWithoutFolder => "level_without_folder",
            Self::NoLevel => "no_level",
            Self::Unmeasured => "unmeasured",
        }
    }
}

/// The listing for one arena row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ps4VenueListing {
    /// "vault level name (observed on every shipped title; not a documented selector)".
    pub key_label: String,
    pub class: Ps4VenueClass,
    /// Why the row has this class, in the user's words.
    pub reason: String,
    pub candidates: Vec<Ps4VenueCandidate>,
    /// Shipped `stadiums/` directories no level names (`common*`, `unused`, `vancouvertest`) — listed
    /// once, never under an arena.
    pub shared_content: Vec<(String, Option<usize>)>,
    /// What changing `arenaartid` would swap at once, per the research — stated so no edit is
    /// offered here.
    pub dependencies: String,
    pub typed_refs: BTreeMap<String, usize>,
    pub reassignment_supported: bool,
    pub reassignment_reason: String,
}

/// Build the listing for `arena_art_id` from the title's level table and the attached tree.
pub fn ps4_venue_listing(game_id: Option<&str>, levels: Option<&Ps4VenueLevels>, arena_art_id: Option<u64>, rendering_root: Option<&Path>, venue_directories: &[(String, usize)]) -> Ps4VenueListing {
    let key_label = "vault level name (observed on every shipped title; not a documented selector)".to_string();
    let reassignment_reason = "re-pointing `arenaartid` is a cascade, not a cell edit: one vault level record binds the venue folder, the centre-logo / centre-line / ice-line texture packs, a rink id (NHL 19–21), crowd placement and NIS override groups, and what the runtime reads is not established (research R73 §2; open question Q-R69-1) — no edit affordance is offered".to_string();
    let base = |class: Ps4VenueClass, reason: String| Ps4VenueListing {
        key_label: key_label.clone(),
        class,
        reason,
        candidates: Vec::new(),
        shared_content: Vec::new(),
        dependencies: String::new(),
        typed_refs: BTreeMap::new(),
        reassignment_supported: false,
        reassignment_reason: reassignment_reason.clone(),
    };
    let Some(game_id) = game_id else {
        return base(Ps4VenueClass::Unmeasured, "the open project has no recorded title, so no level table applies".into());
    };
    let Some(title) = levels.and_then(|l| l.title(game_id)) else {
        return base(Ps4VenueClass::Unmeasured, format!("no vault-level table is measured for `{game_id}` (research R73 covers PS4 NHL 15–21)"));
    };
    let Some(art) = arena_art_id else {
        return base(Ps4VenueClass::NoLevel, "this arena row carries no `arenaartid`, so no level record can be joined".into());
    };
    let dir_members: BTreeMap<&str, usize> = venue_directories.iter().map(|(n, m)| (n.as_str(), *m)).collect();
    let texpack = |family: &str, key: Option<i64>, misses: &[i64]| key.map(|k| Ps4TexpackRef { family: family.into(), key: k, in_set: !misses.contains(&k) });
    let candidates: Vec<Ps4VenueCandidate> = title
        .levels
        .iter()
        .filter(|l| l.id == art)
        .map(|l| {
            let (folder, members) = match (&l.dir, rendering_root) {
                (None, _) => (FolderStatus::NoFolderShipped, None),
                (Some(_), None) => (FolderStatus::Unknown, None),
                (Some(d), Some(root)) => {
                    if root.join("stadiums").join(d).is_dir() {
                        (FolderStatus::Present, dir_members.get(d.as_str()).copied().or_else(|| std::fs::read_dir(root.join("stadiums").join(d)).map(|r| r.flatten().count()).ok()))
                    } else {
                        (FolderStatus::Missing, None)
                    }
                }
            };
            let mut texpacks = Vec::new();
            texpacks.extend(texpack("center_logo", l.center_logo, &title.texpack_misses.center_logo));
            texpacks.extend(texpack("center_line", l.center_line, &title.texpack_misses.center_line));
            texpacks.extend(texpack("ice_line", l.ice_line, &title.texpack_misses.ice_line));
            Ps4VenueCandidate {
                level_name: l.name.clone(),
                dir: l.dir.as_ref().map(|d| format!("stadiums/{d}")),
                folder,
                members,
                texpacks,
                rink_id: l.rink_id,
                arena_id_attr: l.arena_id_attr,
            }
        })
        .collect();
    let (class, reason) = match candidates.len() {
        0 => (Ps4VenueClass::NoLevel, format!("no vault level record carries id {art} in the research join for `{game_id}` ({} arena rows measured); nothing is guessed", title.arena_rows_measured)),
        1 if candidates[0].dir.is_none() => (Ps4VenueClass::LevelWithoutFolder, format!("one level record carries id {art} (`{}`) but the research found no shipped folder for it", candidates[0].level_name)),
        1 => (Ps4VenueClass::Unambiguous, format!("exactly one level record carries id {art} and it names a shipped folder")),
        n => (
            Ps4VenueClass::Ambiguous,
            format!(
                "{n} level records carry id {art}: {}; the vault holds no attribute that settles which record the game uses, so every candidate is listed and none is chosen (research R73 §1)",
                candidates.iter().map(|c| format!("`{}`", c.level_name)).collect::<Vec<_>>().join(", ")
            ),
        ),
    };
    // Shared / unreferenced content: shipped directories no level of this title names.
    let named: std::collections::BTreeSet<&str> = title.levels.iter().filter_map(|l| l.dir.as_deref()).collect();
    let mut shared_content: Vec<(String, Option<usize>)> = venue_directories.iter().filter(|(n, _)| !named.contains(n.as_str())).map(|(n, m)| (n.clone(), Some(*m))).collect();
    for d in &title.dirs_without_level_name {
        if !shared_content.iter().any(|(n, _)| n == d) && !venue_directories.is_empty() {
            // Research-listed shared directory absent from this tree: still named, with no count.
            shared_content.push((d.clone(), None));
        }
    }
    if venue_directories.is_empty() {
        shared_content = title.dirs_without_level_name.iter().map(|d| (d.clone(), None)).collect();
    }
    shared_content.sort();
    let dependencies = format!(
        "a level record binds: the venue folder (by name), the centre-logo / centre-line / ice-line texture packs (numeric keys into three vault sets of {} / {} / {} entries on this title{}), {}crowd placement{}",
        title.texpack_set_sizes.center_logo,
        title.texpack_set_sizes.center_line,
        title.texpack_set_sizes.ice_line,
        if title.texpack_misses.center_logo.is_empty() { String::new() } else { format!("; centre-logo key{} {} outside the set", if title.texpack_misses.center_logo.len() == 1 { "" } else { "s" }, title.texpack_misses.center_logo.iter().map(|k| k.to_string()).collect::<Vec<_>>().join(", ")) },
        if title.levels.iter().any(|l| l.rink_id.is_some()) { "a rink id, " } else { "" },
        if title.typed_refs.keys().any(|k| k.contains("nis")) { " and NIS override groups" } else { "" }
    );
    Ps4VenueListing {
        key_label,
        class,
        reason,
        candidates,
        shared_content,
        dependencies,
        typed_refs: title.typed_refs.clone(),
        reassignment_supported: false,
        reassignment_reason,
    }
}

// ---------------------------------------------------------------------------
// Directory evidence
// ---------------------------------------------------------------------------

enum Dir {
    Listed(BTreeMap<String, String>),
    Absent,
    Unreadable(String),
    /// No rendering tree is attached: every file is unknown, nothing is counted.
    NotAttached,
}

fn list_dir(root: &Path, dir: &str, extension: &str) -> Dir {
    let d = root.join(dir);
    match std::fs::read_dir(&d) {
        Ok(rd) => {
            let mut stems: BTreeMap<String, String> = BTreeMap::new();
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                let Some((stem, ext)) = name.rsplit_once('.') else { continue };
                if ext != "rx2" && ext != "rpsgl" {
                    continue;
                }
                match stems.get(stem) {
                    Some(existing) if existing.ends_with(&format!(".{extension}")) => {}
                    _ => {
                        stems.insert(stem.to_string(), name.clone());
                    }
                }
            }
            Dir::Listed(stems)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Dir::Absent,
        Err(e) => Dir::Unreadable(format!("{}: {e}", d.display())),
    }
}

/// Parse `<prefix>_<id><suffix>` where `suffix` is a literal tail (`_container_0`, `_cm`, `_bm`,
/// `` …). Returns the id when the stem matches the grammar exactly.
fn parse_id(stem: &str, prefix: &str, suffix: &str) -> Option<u64> {
    let rest = stem.strip_prefix(prefix)?;
    let num = rest.strip_suffix(suffix)?;
    if num.is_empty() || (num.len() > 1 && num.starts_with('0')) || !num.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    num.parse().ok()
}

fn resolve(root: &Path, dir: &Dir, sub: &str, stem: String, key: u64, key_source: &str) -> AssetFile {
    use crate::kit_files::ComponentStatus;
    let (status, path, reason) = match dir {
        Dir::Listed(stems) => match stems.get(&stem) {
            Some(name) => (ComponentStatus::Present, Some(root.join(sub).join(name).to_string_lossy().into_owned()), None),
            None => (ComponentStatus::Missing, None, Some(format!("`{sub}/{stem}` is not in the listed directory"))),
        },
        Dir::Absent => (ComponentStatus::Missing, None, Some(format!("the `{sub}/` directory is absent from the rendering tree"))),
        Dir::Unreadable(r) => (ComponentStatus::Unknown, None, Some(format!("the `{sub}/` directory could not be read ({r}); unknown, not absent"))),
        Dir::NotAttached => (ComponentStatus::Unknown, None, Some("no rendering tree is attached to this project, so the file was not looked for".into())),
    };
    AssetFile { key, key_source: key_source.to_string(), file_stem: stem, path, status, reason }
}

fn shipped_count(dir: &Dir, prefix: &str, suffix: &str) -> Option<usize> {
    match dir {
        Dir::Listed(stems) => Some(stems.keys().filter(|s| parse_id(s, prefix, suffix).is_some()).count()),
        _ => None,
    }
}

fn shipped_ids(dir: &Dir, prefix: &str, suffix: &str) -> BTreeSet<u64> {
    match dir {
        Dir::Listed(stems) => stems.keys().filter_map(|s| parse_id(s, prefix, suffix)).collect(),
        _ => BTreeSet::new(),
    }
}

fn dir_note(dir: &Dir, sub: &str) -> Option<String> {
    match dir {
        Dir::Listed(_) | Dir::NotAttached => None,
        Dir::Absent => Some(format!("the `{sub}/` directory is absent: its files are missing")),
        Dir::Unreadable(r) => Some(format!("the `{sub}/` directory could not be read ({r}): its files are unknown, not proven absent")),
    }
}

// ---------------------------------------------------------------------------
// The view
// ---------------------------------------------------------------------------

/// Build the arena's asset view. `rendering_root` is the platform's rendering tree (gen7
/// `rendering/`, PS4 `…/kettle/rendering`).
pub fn arena_assets(inputs: &ArenaAssetInputs, contract: ArenaAssetContract, rendering_root: Option<&Path>) -> ArenaAssetsView {
    arena_assets_with_levels(inputs, contract, rendering_root, None, None)
}

/// [`arena_assets`] with the PS4 vault-level table (research R73) for `game_id`, so the arena's
/// `arenaartid` lists its level candidates. Read-only.
pub fn arena_assets_with_levels(inputs: &ArenaAssetInputs, contract: ArenaAssetContract, rendering_root: Option<&Path>, game_id: Option<&str>, levels: Option<&Ps4VenueLevels>) -> ArenaAssetsView {
    let mut notes = Vec::new();
    let mut families = Vec::new();
    let mut venue_directories = Vec::new();

    match (&contract, rendering_root) {
        (ArenaAssetContract::Gen7 { extension, jumbotron_shipped, source }, root) => {
            // Without a tree every directory is "not attached": the same families and stems are
            // produced with unknown evidence, so the card's shape does not depend on the tree.
            let (root, stadium, jumbotron, icesurface, banner) = match root {
                Some(root) => (root, list_dir(root, "stadium", extension), list_dir(root, "jumbotron", extension), list_dir(root, "icesurface", extension), list_dir(root, "banner", extension)),
                None => {
                    notes.push("no rendering tree is attached, so the arena's files are named but not looked for".to_string());
                    (Path::new(""), Dir::NotAttached, Dir::NotAttached, Dir::NotAttached, Dir::NotAttached)
                }
            };
            for (d, name) in [(&stadium, "stadium"), (&icesurface, "icesurface"), (&banner, "banner")] {
                if let Some(n) = dir_note(d, name) {
                    notes.push(n);
                }
            }
            if *jumbotron_shipped {
                if let Some(n) = dir_note(&jumbotron, "jumbotron") {
                    notes.push(n);
                }
            }

            // Venue model: every container the id names (the shipped trees carry one per id).
            let art_source = "arenaartid";
            let mut venue = AssetFamily {
                family: "venue_model".into(),
                label: "Venue model".into(),
                dir: "stadium".into(),
                binding: Binding::Established,
                rule: format!("`stadium/stadium_<arenaartid>_container_<n>` — {source}"),
                files: Vec::new(),
                shipped_files: shipped_count(&stadium, "stadium_", "_container_0"),
                spare_keys: Vec::new(),
                notes: Vec::new(),
            };
            match inputs.arena_art_id {
                Some(art) => {
                    // Containers 0..n for this id: list every one the directory holds, or the
                    // expected `_container_0` as missing / unknown.
                    let mut found = Vec::new();
                    if let Dir::Listed(stems) = &stadium {
                        for stem in stems.keys() {
                            if let Some(n) = parse_id(stem, &format!("stadium_{art}_container_"), "") {
                                found.push((n, stem.clone()));
                            }
                        }
                    }
                    if found.is_empty() {
                        venue.files.push(resolve(root, &stadium, "stadium", format!("stadium_{art}_container_0"), art, art_source));
                    } else {
                        found.sort();
                        for (_, stem) in found {
                            venue.files.push(resolve(root, &stadium, "stadium", stem, art, art_source));
                        }
                    }
                    let shipped = shipped_ids(&stadium, "stadium_", "_container_0");
                    venue.spare_keys = shipped.difference(&inputs.all_arena_art_ids).copied().collect();
                }
                None => venue.notes.push("this arena row carries no `arenaartid`, so no venue file can be named".into()),
            }
            families.push(venue);

            if *jumbotron_shipped {
                let mut jt = AssetFamily {
                    family: "jumbotron".into(),
                    label: "Jumbotron".into(),
                    dir: "jumbotron".into(),
                    binding: Binding::Established,
                    rule: format!("`jumbotron/texlib_<arenaartid>` — {source}"),
                    files: Vec::new(),
                    shipped_files: shipped_count(&jumbotron, "texlib_", ""),
                    spare_keys: Vec::new(),
                    notes: Vec::new(),
                };
                if let Some(art) = inputs.arena_art_id {
                    jt.files.push(resolve(root, &jumbotron, "jumbotron", format!("texlib_{art}"), art, art_source));
                    let shipped = shipped_ids(&jumbotron, "texlib_", "");
                    jt.spare_keys = shipped.difference(&inputs.all_arena_art_ids).copied().collect();
                }
                families.push(jt);
            } else {
                families.push(AssetFamily {
                    family: "jumbotron".into(),
                    label: "Jumbotron".into(),
                    dir: "jumbotron".into(),
                    binding: Binding::NoEstablishedBinding,
                    rule: "no `jumbotron/` directory ships on NHL 09 / 10 / 11 (R62 §1)".into(),
                    files: Vec::new(),
                    shipped_files: None,
                    spare_keys: Vec::new(),
                    notes: vec!["not shipped on this title".into()],
                });
            }

            // Team-keyed families through the home teams.
            for (family, label, sub, dir, prefix, suffix, rule) in [
                ("centre_ice_logo", "Centre-ice logo", "icesurface", &icesurface, "centerlogo_", "_cm", "`icesurface/centerlogo_<team art id>_cm` — keyed by the home team's art id, not the arena (R62 §2: 191 of 218 ids are team art ids; no arena column explains them)"),
                ("banners", "Rafter banners", "banner", &banner, "banner_", "_cm", "`banner/banner_<team art id>_cm` — keyed by the home team's art id, not the arena (R62 §2: 210 of 218)"),
            ] {
                let mut fam = AssetFamily {
                    family: family.into(),
                    label: label.into(),
                    dir: sub.into(),
                    binding: Binding::ViaHomeTeam,
                    rule: rule.into(),
                    files: Vec::new(),
                    shipped_files: shipped_count(dir, prefix, suffix),
                    spare_keys: Vec::new(),
                    notes: Vec::new(),
                };
                if !inputs.home_arena_link {
                    fam.notes.push("this title's team table has no home-arena link (`kOtt.buDa` is absent on the early gen7 titles), so no team can be resolved as this arena's tenant and this family cannot be shown through a home team here".into());
                } else if inputs.home_teams.is_empty() {
                    fam.notes.push("no team names this arena as its home, so there is no team art id to resolve through".into());
                }
                for (team, art) in &inputs.home_teams {
                    fam.files.push(resolve(root, dir, sub, format!("{prefix}{art}{suffix}"), *art, &format!("home team {team} (art id {art})")));
                }
                families.push(fam);
            }

            // Bounce map: a candidate.
            let mut bm = AssetFamily {
                family: "bounce_map".into(),
                label: "Ice bounce map".into(),
                dir: "icesurface".into(),
                binding: Binding::Candidate,
                rule: "`icesurface/icesurface_<bouncemap>_bm` — every row's value names a file (R62 §3), but `puck` and `powerring` fit the same id space: a candidate, not a binding".into(),
                files: Vec::new(),
                shipped_files: shipped_count(&icesurface, "icesurface_", "_bm"),
                spare_keys: Vec::new(),
                notes: Vec::new(),
            };
            match inputs.bounce_map {
                Some(v) => bm.files.push(resolve(root, &icesurface, "icesurface", format!("icesurface_{v}_bm"), v, "bouncemap (candidate key)")),
                None => bm.notes.push("this title's arena table carries no `bouncemap` column".into()),
            }
            families.push(bm);

            // The remaining ice layers and named bases: shipped, no established key.
            let mut layers = AssetFamily {
                family: "ice_layers".into(),
                label: "Other ice layers".into(),
                dir: "icesurface".into(),
                binding: Binding::NoEstablishedBinding,
                rule: "`ice_cline_<id>_cm`, `ice_lines_<id>_cm`, `ice_lightmap_<id>_lm`, `ice_hotspot_<id>_<n>_cm` and the named ice bases under `stadium/` (`ice_nhl`, `ice_intl`, …): small id spaces several arena columns cover — no established binding (R62 §3)".into(),
                files: Vec::new(),
                shipped_files: None,
                spare_keys: Vec::new(),
                notes: Vec::new(),
            };
            if let Dir::Listed(stems) = &icesurface {
                let counts: Vec<(&str, usize)> = [("ice_cline_", "_cm"), ("ice_lines_", "_cm"), ("ice_lightmap_", "_lm")]
                    .iter()
                    .map(|(p, s)| (*p, stems.keys().filter(|k| parse_id(k, p, s).is_some()).count()))
                    .collect();
                let hotspots = stems.keys().filter(|k| k.starts_with("ice_hotspot_") && k.ends_with("_cm")).count();
                layers.shipped_files = Some(counts.iter().map(|(_, n)| n).sum::<usize>() + hotspots);
                layers.notes.push(format!(
                    "shipped: {} centre-line, {} line-art, {} lightmap, {hotspots} hotspot files",
                    counts[0].1, counts[1].1, counts[2].1
                ));
            }
            if let Dir::Listed(stems) = &stadium {
                let bases: Vec<&str> = stems.keys().filter(|k| k.starts_with("ice_") && k.ends_with("_container_0")).map(|k| k.trim_end_matches("_container_0")).collect();
                if !bases.is_empty() {
                    layers.notes.push(format!("named ice bases under `stadium/`: {} — selected by something other than the arena art id (unresolved)", bases.join(", ")));
                }
            }
            families.push(layers);
        }
        (ArenaAssetContract::Ps4VaultReference { .. }, Some(root)) => {
            match std::fs::read_dir(root.join("stadiums")) {
                Ok(rd) => {
                    for e in rd.flatten() {
                        if e.path().is_dir() {
                            let members = std::fs::read_dir(e.path()).map(|r| r.flatten().count()).unwrap_or(0);
                            venue_directories.push((e.file_name().to_string_lossy().into_owned(), members));
                        }
                    }
                    venue_directories.sort();
                    notes.push(format!(
                        "{} shipped venue directories under `stadiums/`; no arena column names them (`arenaname` never matches, `cityname` reaches 22 per title — R64 §1); the join runs through the vault level record whose id equals `arenaartid` (research R73, table `defs/arena/ps4-venue-levels.json`), which Studio does not read from the vault itself",
                        venue_directories.len()
                    ));
                    // The numbered common ice families, listed as evidence with the candidate key.
                    let common = root.join("stadiums").join("common");
                    if let Ok(rd) = std::fs::read_dir(&common) {
                        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
                        for e in rd.flatten() {
                            let name = e.file_name().to_string_lossy().into_owned();
                            for p in ["ice_centerlogo", "ice_cline", "powerring", "ice_hotspot"] {
                                if name.starts_with(p) {
                                    *counts.entry(p).or_default() += 1;
                                }
                            }
                        }
                        let mut fam = AssetFamily {
                            family: "common_ice".into(),
                            label: "Shared ice families (`stadiums/common/`)".into(),
                            dir: "stadiums/common".into(),
                            binding: Binding::Candidate,
                            rule: "numbered `ice_centerlogo_<id>`, `ice_cline_<id>`, `powerring_<id>`, `ice_hotspot_<id>` files; `arenaartid` fits the `ice_cline` id space on most rows but so do other columns — a candidate key, not a binding (R64 §2)".into(),
                            files: Vec::new(),
                            shipped_files: Some(counts.values().sum()),
                            spare_keys: Vec::new(),
                            notes: counts.iter().map(|(p, n)| format!("{n} `{p}*` files")).collect(),
                        };
                        if let Some(art) = inputs.arena_art_id {
                            fam.notes.push(format!("this arena's `arenaartid` is {art}; whether it selects any of these files is not established"));
                        }
                        families.push(fam);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => notes.push("the attached PS4 tree has no `stadiums/` directory under its rendering root, so no venue directory is listed".into()),
                Err(e) => notes.push(format!("the `stadiums/` directory could not be read ({e}): unknown, not proven absent")),
            }
        }
        (ArenaAssetContract::Ps4VaultReference { .. }, None) => {
            notes.push("no rendering tree is attached, so no venue directory is listed and no level folder is looked for".into());
        }
        (ArenaAssetContract::Unmeasured { .. }, _) => {}
    }
    let ps4_venue = match &contract {
        ArenaAssetContract::Ps4VaultReference { .. } => Some(ps4_venue_listing(game_id, levels, inputs.arena_art_id, rendering_root, &venue_directories)),
        _ => None,
    };
    finish(inputs, contract, rendering_root, families, venue_directories, ps4_venue, notes)
}

fn finish(inputs: &ArenaAssetInputs, contract: ArenaAssetContract, rendering_root: Option<&Path>, families: Vec<AssetFamily>, venue_directories: Vec<(String, usize)>, ps4_venue: Option<Ps4VenueListing>, mut notes: Vec<String>) -> ArenaAssetsView {
    match &contract {
        ArenaAssetContract::Gen7 { .. } => {}
        ArenaAssetContract::Ps4VaultReference { reason } | ArenaAssetContract::Unmeasured { reason } => notes.insert(0, reason.clone()),
    }
    notes.push("shown, never written: which venue, logo or ice layer the console draws for a game is runtime and is not claimed; families marked candidate or no-established-binding must not be edited as if their key were known".into());
    ArenaAssetsView {
        arena_id: inputs.arena_id,
        arena_art_id: inputs.arena_art_id,
        contract,
        rendering_root: rendering_root.map(|r| r.to_string_lossy().into_owned()),
        families,
        venue_directories,
        ps4_venue,
        notes,
    }
}

// ---------------------------------------------------------------------------
// Database side
// ---------------------------------------------------------------------------

/// Read the join's inputs from the open database: the arena row's `arenaartid` / `bouncemap`,
/// its home teams with their art ids, and the residual id sets.
pub fn arena_asset_inputs(db: &tdb_core::TdbFile, meta: &tdb_defs::FieldMetaIndex, arena_id: u64) -> Option<ArenaAssetInputs> {
    use crate::view::{read_u64_by_studio, team_name_source};
    let arena = db.table_canonical("StEO")?;
    // The arena key is `StEO.vfEq` (the same key `view::arena_record` uses).
    let rec = (0..arena.current_records as u32).find(|&i| read_u64_by_studio(db, arena, i, "vfEq") == Some(arena_id))?;
    let f_art = field_by_name(meta, arena, "arenaartid");
    let f_bm = field_by_name(meta, arena, "bouncemap");
    let arena_art_id = f_art.and_then(|f| db.read_u64(arena, rec, f));
    let bounce_map = f_bm.and_then(|f| db.read_u64(arena, rec, f));
    let all_arena_art_ids: BTreeSet<u64> = f_art.map(|f| (0..arena.current_records as u32).filter_map(|i| db.read_u64(arena, i, f)).collect()).unwrap_or_default();

    let mut home_teams = Vec::new();
    let mut all_team_art_ids = BTreeSet::new();
    let mut home_arena_link = false;
    if let Some(kott) = db.table_canonical(tdb_relations::entity::ids::TEAM) {
        home_arena_link = kott.fields.iter().any(|f| f.id.canonical(db.endian) == "buDa");
        if let Some(f_tart) = field_by_name(meta, kott, "artid") {
            for trec in 0..kott.current_records as u32 {
                let Some(art) = db.read_u64(kott, trec, f_tart) else { continue };
                all_team_art_ids.insert(art);
                // `kOtt.buDa` → `StEO.vfEq`: the team's home arena (the same FK `view::arena_view` uses).
                if read_u64_by_studio(db, kott, trec, "buDa") == Some(arena_id) {
                    let src = team_name_source(db, meta, kott, trec);
                    let mut name = crate::view::team_display_name(db, meta, &src);
                    if name.trim().is_empty() {
                        name = format!("team record {trec}");
                    }
                    home_teams.push((name, art));
                }
            }
        }
    }
    Some(ArenaAssetInputs { arena_id, arena_art_id, bounce_map, home_arena_link, home_teams, all_arena_art_ids, all_team_art_ids })
}

/// A table field by its metadata `name`.
fn field_by_name<'t>(meta: &tdb_defs::FieldMetaIndex, table: &'t tdb_core::Table, name: &str) -> Option<&'t tdb_core::FieldDescriptor> {
    table.fields.iter().find(|f| meta.field(&table.id.raw(), &f.id.raw()).map(|m| m.name.as_str() == name).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kit_files::ComponentStatus;
    use std::path::PathBuf;

    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Self {
            let d = std::env::temp_dir().join(format!("nhlms-arena-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            Tmp(d)
        }
        fn put(&self, dir: &str, name: &str) {
            let d = self.0.join(dir);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join(name), b"x").unwrap();
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn inputs() -> ArenaAssetInputs {
        ArenaAssetInputs {
            arena_id: 5,
            arena_art_id: Some(12),
            bounce_map: Some(3),
            home_arena_link: true,
            home_teams: vec![("Anaheim Ducks".into(), 40)],
            all_arena_art_ids: [12u64, 13].into_iter().collect(),
            all_team_art_ids: [40u64, 41].into_iter().collect(),
        }
    }

    fn family<'a>(v: &'a ArenaAssetsView, name: &str) -> &'a AssetFamily {
        v.families.iter().find(|f| f.family == name).unwrap_or_else(|| panic!("family {name} missing: {:?}", v.families.iter().map(|f| &f.family).collect::<Vec<_>>()))
    }

    #[test]
    fn gen7_resolves_the_venue_and_jumbotron_from_the_art_id_and_the_logo_through_the_home_team() {
        let d = Tmp::new("gen7");
        d.put("stadium", "stadium_12_container_0.rx2");
        d.put("stadium", "stadium_13_container_0.rx2");
        d.put("stadium", "stadium_600_container_0.rx2"); // a spare venue
        d.put("stadium", "ice_nhl_container_0.rx2");
        d.put("jumbotron", "texlib_12.rx2");
        d.put("jumbotron", "texlib_600.rx2");
        d.put("icesurface", "centerlogo_40_cm.rx2");
        d.put("icesurface", "icesurface_3_bm.rx2");
        d.put("icesurface", "ice_cline_1_cm.rx2");
        d.put("banner", "banner_40_cm.rx2");
        let v = arena_assets(&inputs(), arena_asset_contract(Some("nhllegacy"), Platform::Xbox), Some(d.0.as_path()));
        let venue = family(&v, "venue_model");
        assert_eq!(venue.binding, Binding::Established);
        assert_eq!(venue.files.len(), 1);
        assert_eq!(venue.files[0].status, ComponentStatus::Present);
        assert!(venue.files[0].path.as_ref().unwrap().ends_with("stadium_12_container_0.rx2"));
        assert_eq!(venue.spare_keys, vec![600], "the venue no arena row names is the spare");
        assert_eq!(venue.shipped_files, Some(3), "named ice bases are not counted as venues");
        let jt = family(&v, "jumbotron");
        assert_eq!(jt.files[0].status, ComponentStatus::Present);
        assert_eq!(jt.spare_keys, vec![600]);
        let logo = family(&v, "centre_ice_logo");
        assert_eq!(logo.binding, Binding::ViaHomeTeam);
        assert_eq!(logo.files[0].key_source, "home team Anaheim Ducks (art id 40)");
        assert_eq!(logo.files[0].status, ComponentStatus::Present);
        assert_eq!(family(&v, "banners").files[0].status, ComponentStatus::Present);
        let bm = family(&v, "bounce_map");
        assert_eq!(bm.binding, Binding::Candidate);
        assert_eq!(bm.files[0].status, ComponentStatus::Present);
        let layers = family(&v, "ice_layers");
        assert_eq!(layers.binding, Binding::NoEstablishedBinding);
        assert!(layers.files.is_empty());
        assert!(layers.notes.iter().any(|n| n.contains("1 centre-line")) && layers.notes.iter().any(|n| n.contains("ice_nhl")), "{:?}", layers.notes);
        assert!(v.notes.iter().any(|n| n.contains("never written")));
    }

    #[test]
    fn missing_and_absent_files_are_reported_not_hidden() {
        let d = Tmp::new("missing");
        d.put("stadium", "stadium_13_container_0.rx2"); // not this arena's
        // no jumbotron directory, no icesurface, no banner
        let v = arena_assets(&inputs(), arena_asset_contract(Some("nhl15"), Platform::Xbox), Some(d.0.as_path()));
        let venue = family(&v, "venue_model");
        assert_eq!(venue.files[0].status, ComponentStatus::Missing);
        assert!(venue.files[0].reason.as_deref().unwrap().contains("not in the listed directory"));
        assert_eq!(venue.spare_keys, vec![13].into_iter().filter(|_| false).collect::<Vec<_>>(), "13 is another arena's id, not a spare");
        let jt = family(&v, "jumbotron");
        assert_eq!(jt.files[0].status, ComponentStatus::Missing);
        assert!(jt.files[0].reason.as_deref().unwrap().contains("absent"));
        assert!(v.notes.iter().any(|n| n.contains("`jumbotron/` directory is absent")));
        assert_eq!(family(&v, "centre_ice_logo").files[0].status, ComponentStatus::Missing);
    }

    #[test]
    fn an_unreadable_directory_is_unknown_and_no_jumbotron_ships_before_nhl_12() {
        let d = Tmp::new("unreadable");
        d.put("stadium", "stadium_12_container_0.rx2");
        std::fs::write(d.0.join("icesurface"), b"not a directory").unwrap();
        let v = arena_assets(&inputs(), arena_asset_contract(Some("nhl11"), Platform::Xbox), Some(d.0.as_path()));
        assert_eq!(family(&v, "centre_ice_logo").files[0].status, ComponentStatus::Unknown);
        assert_eq!(family(&v, "bounce_map").files[0].status, ComponentStatus::Unknown);
        assert!(v.notes.iter().any(|n| n.contains("unknown, not proven absent")));
        let jt = family(&v, "jumbotron");
        assert_eq!(jt.binding, Binding::NoEstablishedBinding);
        assert!(jt.files.is_empty() && jt.notes[0].contains("not shipped"));
    }

    #[test]
    fn a_ps3_tree_resolves_rpsgl_and_a_row_without_art_id_names_nothing() {
        let d = Tmp::new("ps3");
        d.put("stadium", "stadium_12_container_0.rpsgl");
        d.put("stadium", "stadium_12_container_1.rpsgl");
        d.put("jumbotron", "texlib_12.rpsgl");
        let v = arena_assets(&inputs(), arena_asset_contract(Some("nhl14"), Platform::Ps3), Some(d.0.as_path()));
        let venue = family(&v, "venue_model");
        assert_eq!(venue.files.len(), 2, "every container of the id is listed");
        assert!(venue.files.iter().all(|f| f.path.as_ref().unwrap().ends_with(".rpsgl")));
        let mut no_art = inputs();
        no_art.arena_art_id = None;
        let v = arena_assets(&no_art, arena_asset_contract(Some("nhl14"), Platform::Ps3), Some(d.0.as_path()));
        assert!(family(&v, "venue_model").files.is_empty());
        assert!(family(&v, "venue_model").notes[0].contains("no `arenaartid`"));
    }

    #[test]
    fn without_a_tree_the_gen7_files_are_named_but_unknown_and_unmeasured_titles_join_nothing() {
        let v = arena_assets(&inputs(), arena_asset_contract(Some("nhllegacy"), Platform::Xbox), None);
        assert!(v.families.iter().all(|f| f.files.iter().all(|x| x.status == ComponentStatus::Unknown)));
        assert_eq!(family(&v, "venue_model").files[0].file_stem, "stadium_12_container_0");
        assert!(v.notes.iter().any(|n| n.contains("no rendering tree")));
        for (game, platform) in [(Some("nhl08"), Platform::Xbox), (None, Platform::Xbox), (Some("nhl07"), Platform::Ps3)] {
            let c = arena_asset_contract(game, platform);
            assert!(matches!(c, ArenaAssetContract::Unmeasured { .. }));
            let v = arena_assets(&inputs(), c, None);
            assert!(v.families.is_empty());
            assert!(v.notes[0].contains("not measured") || v.notes[0].contains("no recorded title"));
        }
    }

    #[test]
    fn ps4_lists_the_venue_directories_and_common_families_without_claiming_a_binding() {
        let d = Tmp::new("ps4");
        for m in ["detroit", "boston", "eashl_tier3", "outdoor_canal"] {
            d.put(&format!("stadiums/{m}"), "a.gnf");
        }
        d.put("stadiums/common", "ice_centerlogo_12.gnf");
        d.put("stadiums/common", "ice_cline_3.gnf");
        d.put("stadiums/common", "powerring_1.gnf");
        let c = arena_asset_contract(Some("nhl21"), Platform::Ps4);
        assert!(matches!(c, ArenaAssetContract::Ps4VaultReference { .. }));
        let v = arena_assets(&inputs(), c, Some(d.0.as_path()));
        assert_eq!(v.venue_directories.len(), 5, "common is a directory too");
        assert!(v.venue_directories.iter().any(|(n, m)| n == "eashl_tier3" && *m == 1));
        let common = family(&v, "common_ice");
        assert_eq!(common.binding, Binding::Candidate);
        assert_eq!(common.shipped_files, Some(3));
        assert!(common.files.is_empty(), "no file is resolved for the arena: the key is a candidate");
        assert!(common.notes.iter().any(|n| n.contains("not established")));
        assert!(v.notes[0].contains("vault"));
        // No tree: still says why nothing is listed.
        let v = arena_assets(&inputs(), arena_asset_contract(Some("nhl21"), Platform::Ps4), None);
        assert!(v.venue_directories.is_empty() && v.families.is_empty());
        assert!(v.notes.iter().any(|n| n.contains("no rendering tree")));
    }

    /// R4 continuation (23 Sep 2026), Teams 2.5 PS4, research R73: the arena's `arenaartid` lists
    /// every vault level record carrying the id — each with its folder status on the attached
    /// tree, its texpack keys resolved into the measured sets and its rink id — classed as the
    /// research classes them and never disambiguated silently; shared directories appear once;
    /// nothing offers a re-point. Uses the shipped table and a synthetic tree.
    #[test]
    fn ps4_arena_art_id_lists_every_vault_level_candidate_and_never_chooses() {
        let defs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join("defs");
        let levels = Ps4VenueLevels::load(&defs).expect("the shipped level table loads");
        assert_eq!(levels.titles.len(), 7);
        let d = Tmp::new("ps4-levels");
        for m in ["anaheim", "common_ice_int", "boston", "common", "unused"] {
            d.put(&format!("stadiums/{m}"), "a.gnf");
        }
        let c = || arena_asset_contract(Some("nhl21"), Platform::Ps4);
        // Honda Center on NHL 21: art id 0 is shared by `anaheim` and `common_ice_int` — ambiguous.
        let mut i = inputs();
        i.arena_art_id = Some(0);
        let v = arena_assets_with_levels(&i, c(), Some(d.0.as_path()), Some("nhl21"), Some(&levels));
        let l = v.ps4_venue.as_ref().expect("a PS4 listing");
        assert_eq!(l.class, Ps4VenueClass::Ambiguous, "{}", l.reason);
        assert!(l.reason.contains("`anaheim`") && l.reason.contains("`common_ice_int`") && l.reason.contains("none is chosen"), "{}", l.reason);
        assert_eq!(l.candidates.len(), 2);
        assert!(l.candidates.iter().all(|c| c.folder == FolderStatus::Present && c.members == Some(1)));
        let anaheim = l.candidates.iter().find(|c| c.level_name == "anaheim").unwrap();
        assert_eq!(anaheim.dir.as_deref(), Some("stadiums/anaheim"));
        assert!(anaheim.texpacks.iter().any(|t| t.family == "center_line" && t.key == 13 && t.in_set));
        assert!(anaheim.rink_id.is_some(), "NHL 19–21 levels carry a rink id");
        assert!(!l.reassignment_supported && l.reassignment_reason.contains("cascade"));
        assert!(l.key_label.contains("not a documented selector"));
        // Shared content: the shipped directories no level names, once.
        assert!(l.shared_content.iter().any(|(n, m)| n == "common" && *m == Some(1)));
        assert!(l.shared_content.iter().any(|(n, _)| n == "unused"));
        assert!(!l.shared_content.iter().any(|(n, _)| n == "anaheim" || n == "boston"));
        assert!(l.dependencies.contains("NIS override groups"), "{}", l.dependencies);
        // Boston on NHL 21 is unambiguous; a level whose folder the tree lacks is `missing`.
        let boston_id = levels.title("nhl21").unwrap().levels.iter().find(|x| x.name == "boston").unwrap().id;
        i.arena_art_id = Some(boston_id);
        let v = arena_assets_with_levels(&i, c(), Some(d.0.as_path()), Some("nhl21"), Some(&levels));
        let l = v.ps4_venue.as_ref().unwrap();
        assert_eq!((l.class, l.candidates.len()), (Ps4VenueClass::Unambiguous, 1));
        assert_eq!(l.candidates[0].folder, FolderStatus::Present);
        let chicago_id = levels.title("nhl21").unwrap().levels.iter().find(|x| x.name == "chicago").unwrap().id;
        i.arena_art_id = Some(chicago_id);
        let v = arena_assets_with_levels(&i, c(), Some(d.0.as_path()), Some("nhl21"), Some(&levels));
        assert_eq!(v.ps4_venue.as_ref().unwrap().candidates[0].folder, FolderStatus::Missing);
        // NHL 15 Carolina Arena: a level with no shipped folder; NHL 17's centre-logo miss 1001.
        i.arena_art_id = Some(5);
        let v = arena_assets_with_levels(&i, arena_asset_contract(Some("nhl15"), Platform::Ps4), Some(d.0.as_path()), Some("nhl15"), Some(&levels));
        let l = v.ps4_venue.as_ref().unwrap();
        assert_eq!(l.class, Ps4VenueClass::LevelWithoutFolder, "{}", l.reason);
        assert_eq!(l.candidates[0].folder, FolderStatus::NoFolderShipped);
        // R73's texpack misses (1001 on NHL 17, 100 on NHL 20 / 21) belong to levels no arena row
        // references, so the shipped table records them as misses without a referenced level
        // carrying them; the `in_set` resolution is exercised on a synthetic table.
        assert_eq!(levels.title("nhl17").unwrap().texpack_misses.center_logo, vec![1001]);
        assert_eq!(levels.title("nhl21").unwrap().texpack_misses.center_logo, vec![100]);
        let mut synthetic = Ps4VenueLevels { provenance: "test".into(), titles: BTreeMap::new() };
        synthetic.titles.insert(
            "nhl17".into(),
            Ps4VenueTitle {
                arena_rows_measured: 1,
                levels: vec![Ps4VenueLevel { id: 77, name: "custom_vancouver".into(), dir: Some("custom_vancouver".into()), arena_id_attr: Some(77), rink_id: None, center_logo: Some(1001), center_line: Some(3), ice_line: Some(0) }],
                texpack_misses: Ps4TexpackMisses { center_logo: vec![1001], ..Default::default() },
                ..levels.title("nhl17").unwrap().clone()
            },
        );
        i.arena_art_id = Some(77);
        let v = arena_assets_with_levels(&i, arena_asset_contract(Some("nhl17"), Platform::Ps4), None, Some("nhl17"), Some(&synthetic));
        let l = v.ps4_venue.as_ref().unwrap();
        assert_eq!(l.class, Ps4VenueClass::Unambiguous);
        let cand = &l.candidates[0];
        assert!(cand.texpacks.iter().any(|t| t.family == "center_logo" && t.key == 1001 && !t.in_set), "{:?}", cand.texpacks);
        assert!(cand.texpacks.iter().any(|t| t.family == "center_line" && t.key == 3 && t.in_set));
        assert_eq!(cand.folder, FolderStatus::Unknown, "no tree: folders are unknown, not missing");
        assert!(l.dependencies.contains("1001 outside the set"), "{}", l.dependencies);
        // An id no level carries; a title outside the table; no table at all.
        i.arena_art_id = Some(987_654);
        let v = arena_assets_with_levels(&i, c(), None, Some("nhl21"), Some(&levels));
        assert_eq!(v.ps4_venue.as_ref().unwrap().class, Ps4VenueClass::NoLevel);
        let v = arena_assets_with_levels(&i, c(), None, Some("nhl22"), Some(&levels));
        assert_eq!(v.ps4_venue.as_ref().unwrap().class, Ps4VenueClass::Unmeasured);
        let v = arena_assets(&i, c(), None);
        assert_eq!(v.ps4_venue.as_ref().unwrap().class, Ps4VenueClass::Unmeasured);
        // gen7 has no PS4 listing at all.
        let v = arena_assets(&inputs(), arena_asset_contract(Some("nhllegacy"), Platform::Xbox), None);
        assert!(v.ps4_venue.is_none());
    }

    #[test]
    fn stem_parsing_is_strict() {
        assert_eq!(parse_id("stadium_12_container_0", "stadium_", "_container_0"), Some(12));
        assert_eq!(parse_id("stadium_12_container_1", "stadium_", "_container_0"), None);
        assert_eq!(parse_id("ice_nhl_container_0", "stadium_", "_container_0"), None);
        assert_eq!(parse_id("centerlogo_40_cm", "centerlogo_", "_cm"), Some(40));
        assert_eq!(parse_id("centerlogo_40_nm", "centerlogo_", "_cm"), None);
        assert_eq!(parse_id("centerlogo_040_cm", "centerlogo_", "_cm"), None);
        assert_eq!(parse_id("texlib_66", "texlib_", ""), Some(66));
        assert_eq!(parse_id("banner_40_3_tm", "banner_", "_cm"), None);
    }
}

