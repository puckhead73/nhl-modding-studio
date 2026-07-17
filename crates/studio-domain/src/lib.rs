//! `studio-domain` — the GUI-agnostic orchestration layer (the upper half of the old
//! `tdb-gui-api`, with no Tauri dependency so it is fully unit-testable).
//!
//! It binds the Phase 1 engine crates into the two things the application actually needs:
//!
//! * [`Project`] — a loaded database plus its metadata/enum/relationship registries, tagged with the
//!   `(platform, role)` it targets. Knows how it was loaded (raw `.db` vs. save container) so it can
//!   save back through the same path.
//! * [`EditSession`] — the live editing spine over one project: `begin → apply ops → validate →
//!   save`, with an undo/redo history. Each operation runs through the Layer 3 repairing-transaction
//!   harness ([`tdb_edit::txn::EditTransaction`]) and is auto-audited; `validate` reuses the same
//!   Layer 4 predicates so operation-time and save-time never disagree.
//!
//! Phase 2 deliberately exposes a representative slice of operations (the ones Phase 1 shipped)
//! rather than the full command surface — the seams (registry of ops, source-aware save, DTO bridge
//! in `studio-api`) are what later phases extend.

pub mod attributes;
pub mod audit;
pub mod capabilities;
pub mod capacity;
pub mod discovery;
pub mod extract;
pub mod formula;
pub mod historical;
mod error;
mod overview;
mod profile;
mod project;
mod session;
mod team_tabs;
mod view;
mod workspace;

pub use attributes::{
    build_player_attributes, AttributeView, PlayerAttributes, TraitOption, TraitView,
};
pub use audit::{humanize, HumanFinding, HumanReport};
pub use capabilities::Capabilities;
pub use capacity::CapacityGrowth;
pub use discovery::{
    detect_install, ps4_title_ids, title_ids, DbPaths, InstallReport, SaveSlot, TitleIds,
};
pub use extract::{extract_install, find_big_archives, ExtractSummary};
pub use historical::{FhmCorpus, FhmError};
pub use formula::{
    compute_person_career_ratings, compute_person_ratings, default_profile, CareerRatings,
    CareerSeasonOverall, ComputedRatings,
};
pub use tdb_fhm_store::{compute_maxima_at, AttributeMaxima};
pub use error::DomainError;
pub use tdb_formula::{EaRating, EaRatingSet, FormulaProfile, NormalizationMode};
pub use overview::{DbOverview, TableOverview};
pub use profile::ProjectProfile;
pub use project::{Platform, Project, ProjectRole, Source};
pub use session::{EditSession, FieldValue};
pub use workspace::Workspace;
pub use team_tabs::{
    team_draft_picks, team_equipment, team_jerseys, team_strategies, DraftPick, EquipmentZone,
    StrategyBucket, StrategyCell, TeamEquipment, TeamJersey, TeamStrategies,
    EQUIPMENT_ZONE_FIELDS, STRATEGY_FIELDS,
};
pub use view::{
    arena_summaries, arena_view, career_stats, entity_fields, lines_view, player_summaries,
    player_view, read_field, team_summaries, team_view, ArenaRef, ArenaSummary, ArenaView,
    CareerColumn, CareerRow, CareerSection, CareerStats, FieldKind, FieldRaw, FieldView, LineEntry,
    LinesView, PlayerData, PlayerHeader, PlayerSummary, PlayerView, RosterMembership, SlotField,
    TeamSummary, TeamView, CAREER_GOALIE_PLAYOFFS, CAREER_GOALIE_SEASON, CAREER_SKATER_PLAYOFFS,
    CAREER_SKATER_SEASON,
};

/// A convenience result alias for the domain layer.
pub type Result<T> = std::result::Result<T, DomainError>;

// Re-export the engine types that cross the domain boundary, so `studio-api` (and tests) depend on
// `studio-domain` alone rather than reaching past it into individual engine crates.
pub use tdb_core::Endian;
pub use tdb_domain_validation::{Finding, IntegrityReport, Severity};
pub use tdb_edit::txn::OpOutcome;
pub use tdb_relations::{Player, RosterModel, RosterSlot, Team};
