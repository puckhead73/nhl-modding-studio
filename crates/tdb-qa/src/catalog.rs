//! The authoritative Phase-0 catalog: every game×platform unit the QA ledger tracks, the feature
//! families each is scored against, and the fixed status vocabulary. This is inventory metadata only
//! — it never claims a feature works; status is derived from fixture presence and existing automated
//! gates by [`crate::matrix`].
//!
//! Sources reconciled here (all in-repo): the compatibility matrix in
//! `docs/01-scope-and-compatibility.md`, the Rust install targets in
//! `studio-domain/src/discovery.rs` (`title_ids`/`ps4_title_ids`/`Platform`), and the frontend
//! catalog `app/src/lib/catalog.ts`. Game ids are the same lowercase strings used across the code.

/// Whether a game×platform unit is a supported catalog target or a research-only copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    /// A supported catalog platform for the title (a ✅/🟡/⬜ cell in the compatibility matrix).
    Supported,
    /// Present as an anchor for research but **not** a catalog platform (a `—` cell). The PS3 NHL 07
    /// database is the sole instance: it is a research copy, not a shipping target.
    Research,
}

/// One game×platform unit tracked by the QA ledger.
#[derive(Debug, Clone, Copy)]
pub struct CatalogUnit {
    pub game: &'static str,
    /// Filesystem/platform token: `ps3` | `xbox360` | `ps4` | `recomp`.
    pub platform: &'static str,
    pub kind: UnitKind,
    /// `gen6` | `gen7` | `gen8` | `recompiled-gen7`.
    pub generation: &'static str,
    /// The main-DB anchor (repo-relative) that carries this unit's schema, or a note for units with
    /// no independent anchor (Legacy Recompiled shares the Xbox-derived Legacy schema).
    pub schema_target: &'static str,
}

impl CatalogUnit {
    /// The anchor directory name for this platform (`xbox` token maps to the `xbox360` tree; here the
    /// token already is `xbox360`). `recomp` has no anchor tree.
    pub fn platform_dir(&self) -> &'static str {
        self.platform
    }
}

/// The 28 game×platform units: 27 supported catalog cells + the PS3 NHL 07 research copy.
pub const CATALOG: &[CatalogUnit] = &[
    // ---- Gen6 (NHL 07) ----
    unit("nhl07", "xbox360", UnitKind::Supported, "gen6", "anchors/xbox360/nhl07/databases/NHL07NG.db"),
    unit("nhl07", "ps3", UnitKind::Research, "gen6", "anchors/ps3/nhl07/databases/NHL07NG.db"),
    // ---- Gen7 big-endian (NHL 08–15, Legacy) — Xbox 360 ----
    unit("nhl08", "xbox360", UnitKind::Supported, "gen7", "anchors/xbox360/nhl08/databases/nhlng.db"),
    unit("nhl09", "xbox360", UnitKind::Supported, "gen7", "anchors/xbox360/nhl09/databases/nhlng.db"),
    unit("nhl10", "xbox360", UnitKind::Supported, "gen7", "anchors/xbox360/nhl10/databases/nhlng.db"),
    unit("nhl11", "xbox360", UnitKind::Supported, "gen7", "anchors/xbox360/nhl11/databases/nhlng.db"),
    unit("nhl12", "xbox360", UnitKind::Supported, "gen7", "anchors/xbox360/nhl12/databases/nhlng.db"),
    unit("nhl13", "xbox360", UnitKind::Supported, "gen7", "anchors/xbox360/nhl13/databases/nhlng.db"),
    unit("nhl14", "xbox360", UnitKind::Supported, "gen7", "anchors/xbox360/nhl14/databases/nhlng.db"),
    unit("nhl15", "xbox360", UnitKind::Supported, "gen7", "anchors/xbox360/nhl15/databases/nhlng.db"),
    unit("nhllegacy", "xbox360", UnitKind::Supported, "gen7", "anchors/xbox360/nhllegacy/databases/nhlng.db"),
    // ---- Gen7 big-endian — PS3 (byte-schema twin of Xbox 360; NHL 07 PS3 is research, above) ----
    unit("nhl08", "ps3", UnitKind::Supported, "gen7", "anchors/ps3/nhl08/databases/nhlng.db"),
    unit("nhl09", "ps3", UnitKind::Supported, "gen7", "anchors/ps3/nhl09/databases/nhlng.db"),
    unit("nhl10", "ps3", UnitKind::Supported, "gen7", "anchors/ps3/nhl10/databases/nhlng.db"),
    unit("nhl11", "ps3", UnitKind::Supported, "gen7", "anchors/ps3/nhl11/databases/nhlng.db"),
    unit("nhl12", "ps3", UnitKind::Supported, "gen7", "anchors/ps3/nhl12/databases/nhlng.db"),
    unit("nhl13", "ps3", UnitKind::Supported, "gen7", "anchors/ps3/nhl13/databases/nhlng.db"),
    unit("nhl14", "ps3", UnitKind::Supported, "gen7", "anchors/ps3/nhl14/databases/nhlng.db"),
    unit("nhl15", "ps3", UnitKind::Supported, "gen7", "anchors/ps3/nhl15/databases/nhlng.db"),
    unit("nhllegacy", "ps3", UnitKind::Supported, "gen7", "anchors/ps3/nhllegacy/databases/nhlng.db"),
    // ---- Gen8 little-endian — PS4 (NHL 15–21) ----
    unit("nhl15", "ps4", UnitKind::Supported, "gen8", "anchors/ps4/nhl15/databases/nhl_game.db"),
    unit("nhl16", "ps4", UnitKind::Supported, "gen8", "anchors/ps4/nhl16/databases/nhl_game.db"),
    unit("nhl17", "ps4", UnitKind::Supported, "gen8", "anchors/ps4/nhl17/databases/nhl_game.db"),
    unit("nhl18", "ps4", UnitKind::Supported, "gen8", "anchors/ps4/nhl18/databases/nhl_game.db"),
    unit("nhl19", "ps4", UnitKind::Supported, "gen8", "anchors/ps4/nhl19/databases/nhl_game.db"),
    unit("nhl20", "ps4", UnitKind::Supported, "gen8", "anchors/ps4/nhl20/databases/nhl_game.db"),
    unit("nhl21", "ps4", UnitKind::Supported, "gen8", "anchors/ps4/nhl21/databases/nhl_game.db"),
    // ---- Recompiled Gen7 — no independent anchor; uses the Xbox-derived Legacy schema ----
    unit(
        "legacyrecomp",
        "recomp",
        UnitKind::Supported,
        "recompiled-gen7",
        "(shared: anchors/xbox360/nhllegacy/databases/nhlng.db)",
    ),
];

const fn unit(
    game: &'static str,
    platform: &'static str,
    kind: UnitKind,
    generation: &'static str,
    schema_target: &'static str,
) -> CatalogUnit {
    CatalogUnit { game, platform, kind, generation, schema_target }
}

/// A tracked feature family (a column axis of the QA matrix).
#[derive(Debug, Clone, Copy)]
pub struct Family {
    pub id: &'static str,
    /// Human description (for the ledger doc, not the CSV).
    pub about: &'static str,
}

const fn fam(id: &'static str, about: &'static str) -> Family {
    Family { id, about }
}

/// The 14 feature families every unit is scored against, in a fixed order.
pub const FAMILIES: &[Family] = &[
    fam("db-presentation", "players/teams/arenas/career/attributes + raw field disclosure"),
    fam("db-operations", "scalar/string/enum/FK edits + structural roster operations"),
    fam("save-raw-db", "loose main-DB read/write round trip"),
    fam("save-inner-container", "platform inner save container extract/repack"),
    fam("save-outer-package", "outer platform package (Xbox STFS / PS package)"),
    fam("onboarding-install", "install discovery + metadata resolution"),
    fam("onboarding-localization", "localization DB resolution"),
    fam("assets-portraits", "player portrait discover/preview/export/replace/install"),
    fam("assets-logos", "team logo asset family"),
    fam("assets-jerseys", "jersey asset family"),
    fam("assets-equipment", "equipment asset family"),
    fam("assets-heads", "head/hair/appearance asset family"),
    fam("assets-arenas", "arena asset family"),
    fam("assets-audio", "audio asset family"),
];

/// The status vocabulary a QA-matrix cell may carry (Phase 0 never assigns `game-verified`).
pub const STATUS_VOCAB: &[&str] = &[
    "not-applicable",
    "missing-fixture",
    "discovered",
    "read-pass",
    "edit-roundtrip-pass",
    "game-verified",
    "blocked",
];

/// The 17 catalog entries (distinct titles), release order — used by the ledger doc's target map.
pub const CATALOG_ENTRIES: &[&str] = &[
    "nhl07", "nhl08", "nhl09", "nhl10", "nhl11", "nhl12", "nhl13", "nhl14", "nhl15", "nhl16",
    "nhl17", "nhl18", "nhl19", "nhl20", "nhl21", "nhllegacy", "legacyrecomp",
];

/// The generation of a (game, platform) pair, evidence-independent (PS4 ⇒ gen8, NHL 07 ⇒ gen6,
/// recomp ⇒ recompiled-gen7, else gen7). NHL 15 is gen7 on PS3/Xbox and gen8 on PS4.
pub fn generation_of(game: &str, platform: &str) -> &'static str {
    match platform {
        "ps4" => "gen8",
        "recomp" => "recompiled-gen7",
        _ if game == "nhl07" => "gen6",
        _ => "gen7",
    }
}
