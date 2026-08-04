//! **Surface capabilities** — a typed availability verdict for every user-facing database surface,
//! probed from the *loaded* file (table + key + join + required fields), never from the title id.
//!
//! This is the presentation-layer contract the UI gates tabs and queries on. It sits above the
//! field-level [`crate::Capabilities`] flags (which gate individual optional *fields* inside a
//! surface): a `SurfaceCapability` answers "should this tab exist, and in what state?" The probe
//! validates the *complete minimum requirement* of each surface — table presence alone is not enough
//! when the required key, join, or fields are missing — so an empty optional table (feature present,
//! no rows) is distinguished from an absent feature (`NotApplicable`).
//!
//! Phase 1 assigns only availability: `NotApplicable` (omit), `Readable`/`Editable` (show), `Disabled`
//! (show with a reason — asset authoring not built), `Unverified` (show read-only with caution). The
//! `Editable` verdict means a write path is implemented and regression-covered today; the per-anchor
//! `edit-roundtrip-pass` promotion is Phase 2's job, not this probe's.

use serde::Serialize;
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_relations::RosterModel;

use crate::Capabilities;

/// The availability state of one surface. Serialized kebab-case for the DTO/TS union.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurfaceState {
    /// The schema does not carry this feature — the UI omits the tab entirely.
    NotApplicable,
    /// Present and read-verified; no whole-surface write is claimed here (read-only surface, or
    /// edits are conditional — e.g. career depends on aligned display mode).
    Readable,
    /// Present, read-verified, and a write path is implemented + regression-covered today.
    Editable,
    /// Present but intentionally not enabled yet (asset authoring not built) — shown with a reason.
    Disabled,
    /// Present but its semantics/encoding are not ground-truthed — shown read-only with caution.
    Unverified,
}

/// One surface's verdict plus an optional human reason (for `Disabled`/`Unverified`/`NotApplicable`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SurfaceCapability {
    pub state: SurfaceState,
    pub reason: Option<&'static str>,
}

impl SurfaceCapability {
    fn new(state: SurfaceState, reason: Option<&'static str>) -> Self {
        Self { state, reason }
    }
    fn editable() -> Self {
        Self::new(SurfaceState::Editable, None)
    }
    fn readable() -> Self {
        Self::new(SurfaceState::Readable, None)
    }
    fn absent(reason: &'static str) -> Self {
        Self::new(SurfaceState::NotApplicable, Some(reason))
    }
    fn disabled(reason: &'static str) -> Self {
        Self::new(SurfaceState::Disabled, Some(reason))
    }
    /// True when the surface should render a tab at all (anything but `NotApplicable`).
    pub fn is_available(&self) -> bool {
        self.state != SurfaceState::NotApplicable
    }
}

/// The full surface contract for a loaded database. One entry per tracked surface (Phase 1B list).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SurfaceCapabilities {
    pub players: SurfaceCapability,
    pub teams: SurfaceCapability,
    pub arenas: SurfaceCapability,
    pub career: SurfaceCapability,
    pub attributes: SurfaceCapability,
    pub traits: SurfaceCapability,
    pub lines: SurfaceCapability,
    pub strategies: SurfaceCapability,
    pub captaincy: SurfaceCapability,
    pub contracts: SurfaceCapability,
    pub draft_picks: SurfaceCapability,
    pub jerseys: SurfaceCapability,
    pub team_equipment: SurfaceCapability,
    pub player_equipment: SurfaceCapability,
    pub portraits: SurfaceCapability,
    pub logos: SurfaceCapability,
    pub heads: SurfaceCapability,
    pub audio: SurfaceCapability,
}

impl SurfaceCapabilities {
    /// Probe every surface against the loaded file. `caps` is reused for the rating tallies so the
    /// classification (which needs the metadata) isn't recomputed. `model` supplies the entity-graph
    /// requirement (a surface backed by a table but with no players/teams is not usable).
    pub fn probe(
        db: &TdbFile,
        meta: &FieldMetaIndex,
        model: &RosterModel,
        caps: &Capabilities,
    ) -> SurfaceCapabilities {
        let has_players = has_table(db, "ubPc") && !model.players.is_empty();
        let has_teams = has_table(db, "kOtt") && !model.teams.is_empty();

        // Players: the universal entity — bio table + at least one player in the model.
        let players = if has_players {
            SurfaceCapability::editable()
        } else {
            SurfaceCapability::absent("no player bio table (ubPc) or no players")
        };

        // Teams: team table + at least one team (name-source join handled inside the view).
        let teams = if has_teams {
            SurfaceCapability::editable()
        } else {
            SurfaceCapability::absent("no team table (kOtt) or no teams")
        };

        // Arenas: the arena table with a resolvable arena-name field.
        let arenas = if has_table(db, "StEO") && has_named_field(db, meta, "StEO", "arenaname") {
            SurfaceCapability::readable()
        } else {
            SurfaceCapability::absent("no arena table (StEO)")
        };

        // Career: at least one of the four stock career tables, carrying the game_id join. Note this
        // is probed on the *displayed* file, so a roster save that lacks career tables reads absent.
        let career_tables = ["tsSy", "OAqO", "USfF", "nCDp"];
        let has_career = career_tables
            .iter()
            .any(|t| has_table(db, t) && has_named_field(db, meta, t, "game_id"));
        let career = if has_career {
            // Edits are display-mode-sensitive (nhlng-only / aligned mirror) — Readable, not Editable.
            SurfaceCapability::readable()
        } else {
            SurfaceCapability::absent("no stock career tables in this file")
        };

        // Attributes: at least one classified rating on either AI companion (reuse the tallies).
        let has_ratings = caps.skater_ratings_total > 0 || caps.goalie_ratings_total > 0;
        let attributes = if !has_ratings {
            SurfaceCapability::absent("no rating companion tables (dSvy/mHuy)")
        } else if caps.skater_ratings_labelled < caps.skater_ratings_total
            || caps.goalie_ratings_labelled < caps.goalie_ratings_total
        {
            // Ratings classify but some carry no operator label yet — read/edit works, tab hedges.
            SurfaceCapability::new(SurfaceState::Editable, Some("some ratings are not yet named"))
        } else {
            SurfaceCapability::editable()
        };

        // Traits: the 32-slot trait system (first slot id present on the skater AI table). Gen6 has
        // no trait system → NotApplicable.
        let traits = if has_field(db, "dSvy", "wBhb") || has_field(db, "mHuy", "wBhb") {
            SurfaceCapability::editable()
        } else {
            SurfaceCapability::absent("no player-trait slots in this schema (gen6)")
        };

        // Lines / Depth Chart: the roster-lines table with at least one named slot field.
        let lines = if has_table(db, "eGlu") && has_named_slot(db, meta, "eGlu") {
            SurfaceCapability::editable()
        } else {
            SurfaceCapability::absent("no roster-lines table (eGlu)")
        };

        // Captaincy / jersey-number slots ride on eGlu as well.
        let captaincy = if has_table(db, "eGlu") {
            SurfaceCapability::editable()
        } else {
            SurfaceCapability::absent("no roster-slot table (eGlu) for captaincy/jersey")
        };

        // Strategies: the strategy sheet keyed by vfEq (career mirror JUfQ handled inside the write).
        let strategies = if has_table(db, "EeYo") && has_field(db, "EeYo", "vfEq") {
            SurfaceCapability::editable()
        } else {
            SurfaceCapability::absent("no team-strategy table (EeYo)")
        };

        // Contracts: bio table carrying a cap-hit or contract-length field.
        let contracts = if has_field(db, "ubPc", "kKhd") || has_field(db, "ubPc", "IhDG") {
            SurfaceCapability::editable()
        } else {
            SurfaceCapability::absent("no contract fields on the bio table")
        };

        // Draft picks (future-pick assets): the draft-pick table keyed by current owner.
        let draft_picks = if has_table(db, "qHav") && has_field(db, "qHav", "Uwdb") {
            SurfaceCapability::editable()
        } else {
            SurfaceCapability::absent("no draft-pick asset table (qHav)")
        };

        // Team jerseys.
        let jerseys = if has_table(db, "xjgw") && has_field(db, "xjgw", "mCXg") {
            SurfaceCapability::editable()
        } else {
            SurfaceCapability::absent("no team-jersey table (xjgw)")
        };

        // Team equipment.
        let team_equipment = if has_table(db, "hHbv") && has_field(db, "hHbv", "dXSB") {
            SurfaceCapability::editable()
        } else {
            SurfaceCapability::absent("no team-equipment table (hHbv)")
        };

        // Per-player equipment: the separate equipment companions exist on gen7+, but no read/edit
        // surface is built yet (Phase 6) → Disabled where present, NotApplicable on gen6.
        let player_equipment = if caps.has_skater_equip || caps.has_goalie_equip {
            SurfaceCapability::disabled("per-player equipment editor not built (Phase 6)")
        } else {
            SurfaceCapability::absent("no per-player equipment tables (gen6)")
        };

        // Asset surfaces: authoring is Phase 5/6 — never promoted here (rule 12). Shown Disabled with
        // an explicit reason rather than a bare placeholder.
        let portraits = SurfaceCapability::disabled("portrait authoring is a later phase (preview only)");
        let logos = SurfaceCapability::disabled("logo authoring is a later phase (preview only)");
        let heads = SurfaceCapability::disabled("head/appearance authoring is a later phase");
        let audio = SurfaceCapability::disabled("audio tooling is a later phase");

        SurfaceCapabilities {
            players,
            teams,
            arenas,
            career,
            attributes,
            traits,
            lines,
            strategies,
            captaincy,
            contracts,
            draft_picks,
            jerseys,
            team_equipment,
            player_equipment,
            portraits,
            logos,
            heads,
            audio,
        }
    }
}

/// `true` iff `studio_table` exists in this file.
fn has_table(db: &TdbFile, studio_table: &str) -> bool {
    db.table_canonical(studio_table).is_some()
}

/// `true` iff `studio_table` exists and carries a field with the given studio (canonical) id.
fn has_field(db: &TdbFile, studio_table: &str, studio_field: &str) -> bool {
    db.table_canonical(studio_table)
        .map(|t| t.fields.iter().any(|f| f.id.canonical(db.endian) == studio_field))
        .unwrap_or(false)
}

/// `true` iff `studio_table` exists and carries a field whose *metadata name* equals `name` — used
/// where the surface resolves a field by name rather than a fixed id (arena name, career join).
fn has_named_field(db: &TdbFile, meta: &FieldMetaIndex, studio_table: &str, name: &str) -> bool {
    let Some(t) = db.table_canonical(studio_table) else {
        return false;
    };
    let raw = t.id.raw();
    t.fields
        .iter()
        .any(|f| meta.field(&raw, &f.id.raw()).map(|m| m.name == name).unwrap_or(false))
}

/// `true` iff the roster-lines table carries at least one named slot field (a named 1-bit field other
/// than `rosterstatus`) — the layout the Lines/Depth-Chart surface derives its slots from.
fn has_named_slot(db: &TdbFile, meta: &FieldMetaIndex, studio_table: &str) -> bool {
    let Some(t) = db.table_canonical(studio_table) else {
        return false;
    };
    let raw = t.id.raw();
    t.fields.iter().any(|f| {
        f.bit_width == 1
            && meta
                .field(&raw, &f.id.raw())
                .map(|m| !m.name.is_empty() && m.name != "rosterstatus")
                .unwrap_or(false)
    })
}
