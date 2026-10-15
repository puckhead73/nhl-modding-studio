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

pub mod arena_venue;
pub mod arena_edit;
pub mod asset_member_edit;
pub mod board_replace;
pub mod archive_index;
pub mod ps4_cz_names;
pub mod ps4_hair;
pub mod ps4_heads;
pub mod assets;
pub mod attributes;
pub mod audit;
pub mod baselines;
pub mod game_manifest;
pub mod change_list_overlay;
pub mod roster_difflist;
pub mod save_identify;
pub mod table_overlay;
pub mod speech_legacy;
pub mod capabilities;
pub mod capacity;
pub mod competition_groups;
pub mod content_header;
pub mod discovery;
pub mod disk;
pub mod encoding;
pub mod equipment_piece;
pub mod equipment_slots;
pub mod equipment_texture;
mod error;
pub mod extract;
pub mod extract_service;
pub mod pkg_extract;
pub mod field_profile;
pub mod formula;
pub mod goalhorn_catalogue;
pub mod arena_assets;
pub mod equipment_components;
pub mod kit_files;
pub mod song_catalogue;
pub mod player_containment;
pub mod player_reassign;
pub mod team_goalie_masks;
pub mod head_catalog;
pub mod head_census;
pub mod head_preview;
pub mod head_submesh;
pub mod historical;
pub mod import;
pub mod in_place_save;
pub mod xiso_source;
pub mod disc_extract;
mod disc_iso9660;
pub mod iso_patch;
pub mod iso_staging;
pub mod kit_preview;
pub mod font_glyphs;
pub mod league_imagery;
pub mod speech_catalogue;
pub mod ps4_speech_bank;
pub mod gen7_speech_preview;
pub mod gen7_horn_sources;
pub mod gen7_sfx_banks;
pub mod league_logos;
pub mod team_logos;
pub mod team_reid;
pub mod leagues;
pub mod membership;
pub mod pending;
pub mod referential;
pub mod schedule_edit;
pub mod ps4_goalie_masks;
pub mod ps4_skater_gear;
pub mod ps4_vault;
pub mod schedule_stock;
pub mod venue_art;
pub mod schedules;
pub mod settings_catalogue;
pub mod settings_edit;
pub mod settings_csv_edit;
pub mod legends;
pub mod orphan_heads;
mod overview;
pub mod player_manifest;
mod profile;
pub mod mod_package;
pub mod project_store;
mod project;
mod session;
pub mod surfaces;
pub mod txn;
pub mod copied_target;
pub mod dictionary_edit;
pub mod targets;
pub mod team_tabs;
pub mod team_text;
pub mod text_keys;
mod view;
mod workspace;

pub use assets::{
    browse_asset_dir, clone_kit_textures, clone_kit_textures_to, decode_portrait, decode_source_image,
    decode_source_image_bytes, decode_team_logo, decode_texture_file, decode_texture_bytes, decode_archive_member_texture, read_eb_member, export_asset_files,
    export_player_assets, export_team_outfits, jersey_thumbnail, list_team_outfits, list_team_outfit_candidates,
    mark_generic_portraits, probe_birthdate_encoding, probe_weight_base, replace_gnf_texture,
    replace_portrait_texture, replace_texlib_texture, replace_texlib_texture_noted, replace_texture_in_file, plan_texture_replace, xb2_dxt_policy, xb2_policy_notes, TextureReplacePlan, Xb2DxtPolicy,
    resolve_install_path, scan_player_assets, AssetExportSummary, AssetKind, AssetRoots,
    BirthdateEncoding, BrowseEntry, BrowseListing, ClonedKitFile, DecodedImage, ExportOptions,
    FileExportOptions, FileExportSummary, JerseyThumbnail, KitTextureClone, OutfitExportSummary,
    OutfitTeam, OutfitTexture, PlayerAssets, TextureContainer, TextureWrite,
};
pub use attributes::{
    build_player_attributes, AttributeView, PlayerAttributes, TraitOption, TraitView,
};
pub use audit::{humanize, HumanFinding, HumanReport};
pub use capabilities::Capabilities;
pub use capacity::{project_capacity, CapacityGrowth, PendingCreates, TableCapacity};
pub use encoding::{
    content_diagnostics, encoding_conflict, encoding_for_family, player_encoding, schema_family,
    Evidence, PlayerEncoding, SchemaFamily,
};
pub use discovery::{
    detect_install, ps4_default_save_root, ps4_title_ids, title_ids, DbPaths, DiscoveryProblem,
    GameCandidate, InstallReport, SaveSlot, TitleIds,
};
pub use equipment_piece::{
    color_map_slot, resolve_equipment_piece, resolve_helmet_pieces, resolve_visor_mesh,
    slot_preview_pieces, EquipmentPiece, HelmetPieces,
};
pub use equipment_slots::{
    check_player_equipment_files, list_equipment_slot_assets, match_stem_id, rename_stem, slot_enum_label, slot_spec,
    EquipmentFileCheck, EquipmentSlotAsset, EquipmentSlotSpec, SLOT_SPECS,
};
pub use equipment_texture::composite_equipment_texture;
pub use extract::{extract_install, find_big_archives, ExtractSummary};
pub use field_profile::{CellControl, EntityFieldProfile, ProfileCell, ProfileSection};
pub use formula::{
    compute_person_career_ratings, compute_person_ratings, default_profile, CareerRatings,
    CareerSeasonOverall, ComputedRatings,
};
pub use head_catalog::{
    catalog_base_from_slug, export_full_catalog, export_player_head_catalog, read_catalog_manifest,
    sweep_nonplayer_heads, textures_for_source, write_head_group, write_manifest, CatalogEntry,
    CatalogManifest, CatalogOptions, HeadCatalogSummary,
};
pub use head_preview::{
    decode_eye_texture, decode_head_preview, decode_head_preview_named, HeadPreview,
    PreviewSubmesh, PreviewTexture,
};
pub use head_submesh::{
    classify_submesh, head_asset_indices, partition as partition_submeshes, SubmeshPart,
    SubmeshPartition,
};
pub use historical::{FhmCorpus, FhmError};
pub use import::{
    count_creates, lower_change_set, lower_create, lower_update, person_season_from_corpus,
    EaSlugIndex, ImportIdentity, ImportItem, ImportPreview, ItemOutcome, LoweredChangeSet,
    LoweredItem, MatchCandidate, MatchConfidence, PersonSeason, SkippedField,
};
pub use kit_preview::{
    decode_kit_preview, goalie_kit_mesh_defaults, kit_mesh_defaults, GoalieKitMeshDefaults,
    KitMeshDefaults, KitPiece, KitPieceInput, KitPreview,
};
pub use surfaces::{SurfaceCapabilities, SurfaceCapability, SurfaceState};
pub use pending::{CellChange, IntegrityChange, PendingChanges, TableChange};
pub use leagues::{
    AttachedDictionary, DictionaryRef, LabelConfidence, LeagueCatalog, LeagueIdentity, LeagueSource, LeagueView, LeagueVocabularies,
    LeagueVocabulary,
};
pub use tdb_enums::EnumRegistry;
pub use tdb_fhm_store::{compute_maxima_at, AttributeMaxima};
// The Legends sidecar cache + corpus-pool types cross the domain boundary, so `studio-api` reaches
// them through here rather than depending on the store crate directly.
pub use error::DomainError;
pub use overview::{DbOverview, TableOverview};
pub use player_manifest::{
    build_player_manifest, manifest_to_csv, summarize as summarize_manifest, ManifestRow,
    ManifestSummary,
};
pub use profile::ProjectProfile;
pub use targets::{anchor_platform_dir, shipping_target, ShippingTarget, SHIPPING_TARGETS};
pub use baselines::BaselineStore;
pub use project_store::ProjectStore;
pub use disk::free_space;
pub use project::{Platform, Project, ProjectRole, Source};
pub use session::{EditSession, FieldValue, PreparedReload};
pub use tdb_fhm_store::legends::{
    CachedMember, CachedRating, CachedTeam, FranchiseTenure, LegendsCache, LegendsCandidate,
    LegendsRun,
};
pub use tdb_formula::{EaRating, EaRatingSet, FormulaProfile, NormalizationMode};
pub use team_tabs::{
    team_draft_picks, team_equipment, team_jerseys, team_strategies, DraftPick, EquipmentZone,
    StrategyBucket, StrategyCell, TeamEquipment, TeamJersey, TeamStrategies, EQUIPMENT_ZONE_FIELDS,
    STRATEGY_FIELDS,
};
pub use view::{
    arena_summaries, arena_view, career_stats, entity_fields, lines_view, player_summaries,
    player_view, probe_main_team_encoding, read_field, team_summaries, team_view, ArenaRef,
    ArenaSummary, ArenaView, CareerColumn, CareerRow, CareerSection, CareerStats, FieldKind,
    FieldRaw, FieldView, LineEntry, LinesView, MainTeamEncoding, PlayerData, PlayerHeader,
    PlayerSummary, PlayerView, RosterMembership, SlotField, TeamSummary, TeamView,
    CAREER_GOALIE_PLAYOFFS, CAREER_GOALIE_SEASON, CAREER_SKATER_PLAYOFFS, CAREER_SKATER_SEASON,
};
pub use workspace::{BaselineAction, BaselineRefresh, CsvWorkingSet, ScheduleOperation, ScheduleReview, SettingsCsvReview, SettingsReview, Workspace, WorkspaceRevision};

/// A convenience result alias for the domain layer.
pub type Result<T> = std::result::Result<T, DomainError>;

// Re-export the engine types that cross the domain boundary, so `studio-api` (and tests) depend on
// `studio-domain` alone rather than reaching past it into individual engine crates.
pub use tdb_core::Endian;
// The transaction engine's types (identities, states, errors, hooks) cross the domain boundary
// unchanged; `studio-api` reaches them through here.
pub use tdb_txn;
/// Re-exported so the API layer can plan archive replacements through the same stager the
/// transaction adapter verifies.
pub use tdb_archive_big;
pub use tdb_archive_codec;
pub use tdb_loc;
pub use tdb_domain_validation::{Finding, IntegrityReport, Severity};
pub use tdb_edit::txn::{
    ImportApplied, ImportCell, ImportOp, ImportTarget, ImportWrite, OpOutcome,
};
pub use tdb_relations::{Player, RosterModel, RosterSlot, Team};

pub use tdb_edit::txn::explicit_free_agents;
