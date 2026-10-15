// Typed wrapper over Tauri's `invoke`, keyed by command name → { args, return }.
//
// Convention: Tauri v2 maps **camelCase** JS argument keys to the snake_case Rust command
// parameters, so args here use camelCase (projectId, playerId, …). A Rust `Err(String)` surfaces as
// a promise rejection with that string — React Query turns it into the query/mutation `error`.

import { invoke } from "@tauri-apps/api/core";
import { awaitFieldWrites } from "./fieldWrites";

/** NHL07 original-database combinations; keys are opaque, validated by the backend. */
export type PlayerAssignment = {
  key: string; label: string; section: string; current: string;
  options: { key: string; label: string; users: number }[]; availability: string;
};

/** P1-A request shapes: the Rust DTOs take u64 (bigint in the generated types); the app sends plain numbers. */
export type ScheduleGameEditArgs = { record: number; home_team_id?: number | null; away_team_id?: number | null; month?: number | null; day?: number | null };
export type ScheduleTeamMappingArgs = { key: string; team_id: number };
export type ScheduleGenerationConfigArgs = {
  participants: number[];
  rounds: number;
  start_month: number;
  start_day: number;
  days_between_matchdays: number;
  pair_quotas?: number[][];
  odd_home?: "lower_slot" | "higher_slot" | "balance" | "per_pair";
  /** Receipt 48: single-phase groups (team ids) with games per pair inside / across groups. */
  groups?: { groups: number[][]; within: number; across: number; origin?: string };
  /** `per_pair`: the host of each odd pair. */
  odd_hosts?: { home: number; away: number }[];
};
import type { ProjectSummaryDto } from "../bindings/ProjectSummaryDto";
import type { SaveFileIdentityDto } from "../bindings/SaveFileIdentityDto";
import type { SaveEditorOpenDto } from "../bindings/SaveEditorOpenDto";
import type { SaveEditorSaveDto } from "../bindings/SaveEditorSaveDto";
import type { AboutDto } from "../bindings/AboutDto";
import type { Ps4GoalieMaskPairDto } from "../bindings/Ps4GoalieMaskPairDto";
import type { Ps4SkaterGearDto } from "../bindings/Ps4SkaterGearDto";
import type { VenueArtDto } from "../bindings/VenueArtDto";
import type { VenueRasterPreviewDto } from "../bindings/VenueRasterPreviewDto";
import type { ScheduleStockGroupsDto } from "../bindings/ScheduleStockGroupsDto";
import type { IntegrityReportDto } from "../bindings/IntegrityReportDto";
import type { PlayerSummaryDto } from "../bindings/PlayerSummaryDto";
import type { PlayerViewDto } from "../bindings/PlayerViewDto";
import type { TeamSummaryDto } from "../bindings/TeamSummaryDto";
import type { TeamViewDto } from "../bindings/TeamViewDto";
import type { LeagueCatalogDto } from "../bindings/LeagueCatalogDto";
import type { LeagueViewDto } from "../bindings/LeagueViewDto";
import type { PendingReviewDto } from "../bindings/PendingReviewDto";
import type { DictionaryRefDto } from "../bindings/DictionaryRefDto";
import type { CopiedTargetRootDto } from "../bindings/CopiedTargetRootDto";
import type { CopiedTargetReviewDto } from "../bindings/CopiedTargetReviewDto";
import type { ArchiveMemberSpecDto } from "../bindings/ArchiveMemberSpecDto";
import type { DictionaryRepointPreviewDto } from "../bindings/DictionaryRepointPreviewDto";
import type { CopiedTargetApplyDto } from "../bindings/CopiedTargetApplyDto";
import type { CopiedTargetStatusDto } from "../bindings/CopiedTargetStatusDto";
import type { CopiedTargetRestoreDto } from "../bindings/CopiedTargetRestoreDto";
import type { CopiedTargetRecoveryDto } from "../bindings/CopiedTargetRecoveryDto";
import type { ProjectBaselineDto } from "../bindings/ProjectBaselineDto";
import type { CloseProjectDto } from "../bindings/CloseProjectDto";
import type { ArenaSummaryDto } from "../bindings/ArenaSummaryDto";
import type { ArenaViewDto } from "../bindings/ArenaViewDto";
import type { ArenaAssignmentPreviewDto } from "../bindings/ArenaAssignmentPreviewDto";
import type { ArenaVenueDto } from "../bindings/ArenaVenueDto";
import type { LinesViewDto } from "../bindings/LinesViewDto";
import type { CareerStatsDto } from "../bindings/CareerStatsDto";
import type { AddCareerSeasonDto } from "../bindings/AddCareerSeasonDto";
import type { OpOutcomeDto } from "../bindings/OpOutcomeDto";
import type { InstallReportDto } from "../bindings/InstallReportDto";
import type { SaveRootSuggestionDto } from "../bindings/SaveRootSuggestionDto";
import type { BaselineInventoryDto } from "../bindings/BaselineInventoryDto";
import type { ImportSourcesDto } from "../bindings/ImportSourcesDto";
import type { ImportReviewDto } from "../bindings/ImportReviewDto";
import type { ProjectInfoDto } from "../bindings/ProjectInfoDto";
import type { ProjectDetailsDto } from "../bindings/ProjectDetailsDto";
import type { ProjectOverviewDto } from "../bindings/ProjectOverviewDto";
import type { ProjectBuildDto } from "../bindings/ProjectBuildDto";
import type { ProjectArchiveDto } from "../bindings/ProjectArchiveDto";
import type { ProjectOpenDto } from "../bindings/ProjectOpenDto";
import type { ProjectSaveDto } from "../bindings/ProjectSaveDto";
import type { DeployPlanDto } from "../bindings/DeployPlanDto";
import type { DeployApplyDto } from "../bindings/DeployApplyDto";
import type { DeployRestoreDto } from "../bindings/DeployRestoreDto";
import type { DeploymentRecordDto } from "../bindings/DeploymentRecordDto";
import type { ExtractPreflightDto } from "../bindings/ExtractPreflightDto";
import type { ExtractJobDto } from "../bindings/ExtractJobDto";
import type { ProjectRelinkDto } from "../bindings/ProjectRelinkDto";
import type { PackageInventoryDto } from "../bindings/PackageInventoryDto";
import type { PackageBuildRequestDto } from "../bindings/PackageBuildRequestDto";
import type { PackageBuildDto } from "../bindings/PackageBuildDto";
import type { PackageInspectDto } from "../bindings/PackageInspectDto";
import type { ModLibraryDto } from "../bindings/ModLibraryDto";
import type { InstallPlanDto } from "../bindings/InstallPlanDto";
import type { InstallApplyDto } from "../bindings/InstallApplyDto";
import type { InstallRecordDto } from "../bindings/InstallRecordDto";
import type { AutocorrectSummaryDto } from "../bindings/AutocorrectSummaryDto";
import type { RepairBlocksSummaryDto } from "../bindings/RepairBlocksSummaryDto";
import type { CreatePlayerDto } from "../bindings/CreatePlayerDto";
import type { TeamStrategiesDto } from "../bindings/TeamStrategiesDto";
import type { DraftPickDto } from "../bindings/DraftPickDto";
import type { TeamJerseyDto } from "../bindings/TeamJerseyDto";
import type { TeamKitsDto } from "../bindings/TeamKitsDto";
import type { ArenaAssetsDto } from "../bindings/ArenaAssetsDto";
import type { PlayerContainmentViewDto } from "../bindings/PlayerContainmentViewDto";
import type { PlayerReassignPlanDto } from "../bindings/PlayerReassignPlanDto";
import type { SongCatalogueDto } from "../bindings/SongCatalogueDto";
import type { TeamEquipmentDto } from "../bindings/TeamEquipmentDto";
import type { CloneJerseyResultDto } from "../bindings/CloneJerseyResultDto";
import type { JerseyThumbnailDto } from "../bindings/JerseyThumbnailDto";
import type { CloneKitTexturesSummaryDto } from "../bindings/CloneKitTexturesSummaryDto";
import type { HistoricalFiltersDto } from "../bindings/HistoricalFiltersDto";
import type { HistoricalPersonSummaryDto } from "../bindings/HistoricalPersonSummaryDto";
import type { HistoricalPersonDto } from "../bindings/HistoricalPersonDto";
import type { HistoricalTeamSummaryDto } from "../bindings/HistoricalTeamSummaryDto";
import type { HistoricalTeamDto } from "../bindings/HistoricalTeamDto";
import type { HistoricalTeamFiltersDto } from "../bindings/HistoricalTeamFiltersDto";
import type { HistoricalTeamIndexEntryDto } from "../bindings/HistoricalTeamIndexEntryDto";
import type { HistoricalTeamFranchiseDto } from "../bindings/HistoricalTeamFranchiseDto";
import type { EaComputedRatingsDto } from "../bindings/EaComputedRatingsDto";
import type { HistoricalCareerRatingsDto } from "../bindings/HistoricalCareerRatingsDto";
import type { PlayerManifestSummaryDto } from "../bindings/PlayerManifestSummaryDto";
import type { LegendsStatusDto } from "../bindings/LegendsStatusDto";
import type { LegendsComputeSummaryDto } from "../bindings/LegendsComputeSummaryDto";
import type { LegendsTeamSummaryDto } from "../bindings/LegendsTeamSummaryDto";
import type { LegendsMemberDto } from "../bindings/LegendsMemberDto";
import type { FormulaProfileDto } from "../bindings/FormulaProfileDto";
import type { FormulaCatalogDto } from "../bindings/FormulaCatalogDto";
import type { AssetRootsDto } from "../bindings/AssetRootsDto";
import type { AssetRootsArgsDto } from "../bindings/AssetRootsArgsDto";
import type { PlayerAssetsDto } from "../bindings/PlayerAssetsDto";
import type { ExportOptionsDto } from "../bindings/ExportOptionsDto";
import type { AssetExportSummaryDto } from "../bindings/AssetExportSummaryDto";
import type { HeadCatalogSummaryDto } from "../bindings/HeadCatalogSummaryDto";
import type { CatalogManifestDto } from "../bindings/CatalogManifestDto";
import type { HeadPreviewDto } from "../bindings/HeadPreviewDto";
import type { KitPreviewDto } from "../bindings/KitPreviewDto";
import type { EquipmentSlotDto } from "../bindings/EquipmentSlotDto";
import type { EquipmentSlotAssetDto } from "../bindings/EquipmentSlotAssetDto";
import type { HockeyShopItemDto } from "../bindings/HockeyShopItemDto";
import type { HockeyShopSyncSummaryDto } from "../bindings/HockeyShopSyncSummaryDto";
import type { InstallStoreItemRequestDto } from "../bindings/InstallStoreItemRequestDto";
import type { InstallStoreItemPlanDto } from "../bindings/InstallStoreItemPlanDto";
import type { InstallStoreItemApplyResponseDto } from "../bindings/InstallStoreItemApplyResponseDto";
import type { KitPieceInputDto } from "../bindings/KitPieceInputDto";
import type { BrowseListingDto } from "../bindings/BrowseListingDto";
import type { FileExportOptionsDto } from "../bindings/FileExportOptionsDto";
import type { FileExportSummaryDto } from "../bindings/FileExportSummaryDto";
import type { OutfitExportSummaryDto } from "../bindings/OutfitExportSummaryDto";
import type { ImportTextureSummaryDto } from "../bindings/ImportTextureSummaryDto";
import type { TexturePreviewDto } from "../bindings/TexturePreviewDto";
import type { TextureReplacePlanDto } from "../bindings/TextureReplacePlanDto";
import type { OutfitTextureDto } from "../bindings/OutfitTextureDto";
import type { MatchCandidateDto } from "../bindings/MatchCandidateDto";
import type { ImportItemDto } from "../bindings/ImportItemDto";
import type { ChangeSetPreviewDto } from "../bindings/ChangeSetPreviewDto";
import type { ImportResultDto } from "../bindings/ImportResultDto";
import type { IsoInspectDto } from "../bindings/IsoInspectDto";
import type { IsoStagedDto } from "../bindings/IsoStagedDto";
import type { IsoPlanDto } from "../bindings/IsoPlanDto";
import type { IsoApplyDto } from "../bindings/IsoApplyDto";
import type { IsoRestoreDto } from "../bindings/IsoRestoreDto";
import type { IsoRecordDto } from "../bindings/IsoRecordDto";
import type { IsoStatusDto } from "../bindings/IsoStatusDto";
import type { MembershipOptionsDto } from "../bindings/MembershipOptionsDto";
import type { MembershipResultDto } from "../bindings/MembershipResultDto";
import type { ScheduleTableDto } from "../bindings/ScheduleTableDto";
import type { ScheduleViewDto } from "../bindings/ScheduleViewDto";
import type { ScheduleChangePlanDto } from "../bindings/ScheduleChangePlanDto";
import type { ScheduleApplyResultDto } from "../bindings/ScheduleApplyResultDto";
import type { ScheduleExportDto } from "../bindings/ScheduleExportDto";
import type { ScheduleImportPlanDto } from "../bindings/ScheduleImportPlanDto";
import type { ScheduleGenerationOptionsDto } from "../bindings/ScheduleGenerationOptionsDto";
import type { ScheduleGenerationPlanDto } from "../bindings/ScheduleGenerationPlanDto";
import type { LeagueLogoDto } from "../bindings/LeagueLogoDto";
import type { TeamLogoSetDto } from "../bindings/TeamLogoSetDto";
import type { LeagueImageryDto } from "../bindings/LeagueImageryDto";
import type { SpeechCatalogueDto } from "../bindings/SpeechCatalogueDto";
import type { TeamNameCallsDto } from "../bindings/TeamNameCallsDto";
import type { PlayerNameCallsDto } from "../bindings/PlayerNameCallsDto";
import type { EquipmentFileCheckDto } from "../bindings/EquipmentFileCheckDto";
import type { TeamGoalieMasksDto } from "../bindings/TeamGoalieMasksDto";
import type { ContextCatalogueDto } from "../bindings/ContextCatalogueDto";
import type { Ps4SpeechBanksDto } from "../bindings/Ps4SpeechBanksDto";
import type { Ps4SpeechSamplesDto } from "../bindings/Ps4SpeechSamplesDto";
import type { Ps4SpeechPreviewDto } from "../bindings/Ps4SpeechPreviewDto";
import type { Gen7SpeechPreviewDto } from "../bindings/Gen7SpeechPreviewDto";
import type { Gen7BankListingDto } from "../bindings/Gen7BankListingDto";
import type { Gen7SfxBankListDto } from "../bindings/Gen7SfxBankListDto";
import type { Gen7SfxBankTreeDto } from "../bindings/Gen7SfxBankTreeDto";
import type { Gen7SfxPreviewDto } from "../bindings/Gen7SfxPreviewDto";
import type { TeamIdentityDependenciesDto } from "../bindings/TeamIdentityDependenciesDto";
import type { ArtIdSwapDto } from "../bindings/ArtIdSwapDto";
import type { AssetMemberPreviewDto } from "../bindings/AssetMemberPreviewDto";
import type { BoardReplacementPreviewDto } from "../bindings/BoardReplacementPreviewDto";
import type { AssetMemberApplyDto } from "../bindings/AssetMemberApplyDto";
import type { DictionaryRetextPreviewDto } from "../bindings/DictionaryRetextPreviewDto";
import type { DictionaryGrowthOpDto } from "../bindings/DictionaryGrowthOpDto";
import type { DictionaryGrowthOptionsDto } from "../bindings/DictionaryGrowthOptionsDto";
import type { DictionaryGrowthPreviewDto } from "../bindings/DictionaryGrowthPreviewDto";
import type { TeamTextDto } from "../bindings/TeamTextDto";
import type { SettingsCatalogueResultDto } from "../bindings/SettingsCatalogueResultDto";
import type { SettingsEditPreviewDto } from "../bindings/SettingsEditPreviewDto";
import type { SettingsEditApplyDto } from "../bindings/SettingsEditApplyDto";

export type Platform = "ps3" | "xbox" | "ps4" | "ps5" | "recomp";
export type Role = "nhlng" | "roster" | "localization";

type Commands = {
  load_project: {
    args: { dbPath: string; platform: Platform; role: Role; metaXml: string; defsRoot: string };
    ret: ProjectSummaryDto;
  };
  open_project: {
    args: { dbPath: string; platform: Platform; role: Role };
    ret: ProjectSummaryDto;
  };
  // Root-folder onboarding: scan a folder for the game's databases + save inventory. `savePath`
  // locates the roster/profile saves for Xbox/Recomp (whose emulator saves aren't in a fixed place)
  // and PS4 (the shadPS4 save-data folder, chosen independently of the game folder).
  detect_install: {
    args: { gameId: string; platform: Platform; root: string; savePath?: string | null };
    ret: InstallReportDto;
  };
  // The current Windows user's shadPS4 save-data folder, suggested at PS4 onboarding.
  ps4_save_root_suggestion: { args: Record<string, never>; ret: SaveRootSuggestionDto };
  // ONB-002 — bundled baselines, project folders, extraction service, deployment.
  baseline_inventory: { args: Record<string, never>; ret: BaselineInventoryDto };
  project_create_fresh: {
    args: { projectDir: string; gameId: string; platform: Platform; root: string; savePath?: string | null; rosterSave?: string | null };
    ret: ProjectOpenDto;
  };
  project_import_review: { args: { gameId: string; platform: Platform; sources: ImportSourcesDto }; ret: ImportReviewDto };
  project_create_import: {
    args: { projectDir: string; gameId: string; platform: Platform; sources: ImportSourcesDto; root: string; savePath?: string | null; rosterSave?: string | null };
    ret: ProjectOpenDto;
  };
  project_delete: { args: { projectDir: string }; ret: null };
  project_open: { args: { projectDir: string }; ret: ProjectOpenDto };
  // ONB-003 — resume, relink, mod package builder and installer.
  project_set_view: { args: { projectId: number; route: string }; ret: null };
  project_relink: {
    args: { projectId: number; root?: string | null; savePath?: string | null; rosterSave?: string | null; assetsDir?: string | null };
    ret: ProjectRelinkDto;
  };
  package_inventory: { args: { projectId: number }; ret: PackageInventoryDto };
  package_build: { args: { projectId: number; request: PackageBuildRequestDto }; ret: PackageBuildDto };
  package_inspect: { args: { path: string }; ret: PackageInspectDto };
  mod_library_scan: { args: { root: string }; ret: ModLibraryDto };
  mod_manager_check_target: { args: { root: string; platform: Platform }; ret: string };
  package_preflight: {
    args: { packagePath: string; gameId: string; platform: Platform; root: string; savePath?: string | null; rosterSave?: string | null; allowConflicts: boolean };
    ret: InstallPlanDto;
  };
  package_apply: { args: { handle: number }; ret: InstallApplyDto };
  package_installs: { args: Record<string, never>; ret: InstallRecordDto[] };
  package_restore: { args: { recordDir: string }; ret: InstallApplyDto };
  project_info: { args: { projectId: number }; ret: ProjectInfoDto };
  project_revert_change: { args: { projectId: number; changeId: string }; ret: OpOutcomeDto };
  project_overview: { args: { projectId: number }; ret: ProjectOverviewDto };
  project_build: { args: { projectId: number; projectsRoot: string }; ret: ProjectBuildDto };
  project_archive: { args: { projectId: number; projectsRoot: string }; ret: ProjectArchiveDto };
  project_copy_to: { args: { projectId: number; destination: string }; ret: string };
  project_archive_preview: { args: { archive: string }; ret: { name: string; game_id: string; platform: string } };
  project_import_archive: { args: { archive: string; destination: string; sourceRoot: string | null }; ret: string };
  project_set_details: { args: { projectId: number; details: ProjectDetailsDto }; ret: ProjectDetailsDto };
  project_save: { args: { projectId: number }; ret: ProjectSaveDto };
  project_set_extraction: { args: { projectId: number; choice: "skipped" | "extracted" | "not_needed" | "pending" }; ret: ProjectInfoDto };
  project_deploy_plan: { args: { projectId: number; allowChanged: boolean }; ret: DeployPlanDto };
  project_deploy_apply: { args: { projectId: number; handle: number }; ret: DeployApplyDto };
  project_deploy_restore: { args: { projectId: number; seq: number }; ret: DeployRestoreDto };
  project_deployments: { args: { projectId: number }; ret: DeploymentRecordDto[] };
  game_files_check: { args: { root: string; gameId: string; platform: Platform }; ret: import("../components/VanillaCheck").GameFilesReport };
  extract_preflight: { args: { root: string; platform: Platform; destination?: string }; ret: ExtractPreflightDto };
  extract_start: { args: { root: string; platform: Platform; destination?: string }; ret: ExtractJobDto };
  disc_extract_start: { args: { image: string; root: string }; ret: ExtractJobDto };
  pkg_extract_start: { args: { image: string; root: string; gameId: string; extractor?: string }; ret: ExtractJobDto };
  extract_status: { args: { job: number }; ret: ExtractJobDto };
  extract_cancel: { args: { job: number }; ret: ExtractJobDto };
  extract_running_for: { args: { root: string }; ret: ExtractJobDto | null };
  project_summary: { args: { projectId: number }; ret: ProjectSummaryDto };
  validate_project: { args: { projectId: number }; ret: IntegrityReportDto };
  // Acknowledged corruption-override: allow edits on a DB that fails the structural audit (unsupported).
  set_edit_override: { args: { projectId: number; allow: boolean }; ret: null };
  // Switch read views between nhlng.db (default) and the mirrored roster save. Aligned pair: writes
  // still hit both files. Misaligned pair: roster-only mode — edits apply strictly to the roster file.
  set_display_roster: { args: { projectId: number; on: boolean }; ret: ProjectSummaryDto };
  // On-load duplicate-jersey auto-correction; returns the corrections applied (staged as a pending edit).
  autocorrect_jerseys: { args: { projectId: number }; ret: AutocorrectSummaryDto };
  // Roster-block structural repair (order-only permutation of eGlu; staged as a pending edit).
  repair_roster_blocks: { args: { projectId: number }; ret: RepairBlocksSummaryDto };
  list_players: { args: { projectId: number }; ret: PlayerSummaryDto[] };
  player_view: { args: { projectId: number; playerId: number }; ret: PlayerViewDto | null };
  player_assignments: { args: { projectId: number; playerId: number }; ret: PlayerAssignment[] };
  set_player_assignment: { args: { projectId: number; playerId: number; assignment: string; choice: string }; ret: OpOutcomeDto };
  career_stats: { args: { projectId: number; playerId: number }; ret: CareerStatsDto };
  // Add a stock career season row (table = one of the four career tables); stat cells start zero.
  add_career_season: {
    args: {
      projectId: number;
      table: string;
      playerId: number;
      yearEnd: number;
      league: number;
      team: string;
    };
    ret: AddCareerSeasonDto;
  };
  // Delete one stock career row by record index (ordered shift).
  delete_career_row: {
    args: { projectId: number; table: string; record: number };
    ret: OpOutcomeDto;
  };
  list_teams: { args: { projectId: number }; ret: TeamSummaryDto[] };
  team_view: { args: { projectId: number; teamId: number }; ret: TeamViewDto | null };
  // The loaded target's league catalog (per-target resolver: stockleagues rows on PS4, labelled
  // team codes elsewhere). Key leagues by `code`, never by display text.
  list_leagues: { args: { projectId: number }; ret: LeagueCatalogDto };
  league_view: { args: { projectId: number; code: number }; ret: LeagueViewDto | null };
  // Attach the target's own localization dictionary (CRC-verified) for league label resolution.
  attach_dictionary: { args: { projectId: number; path: string }; ret: DictionaryRefDto };
  // The pending review: baseline (files on disk) vs working state, per file role; `limit` caps the
  // decoded cell list while the counts stay exact.
  pending_changes: { args: { projectId: number; limit: number }; ret: PendingReviewDto };

  // Copied-target save (S2-C): review → apply under a server-owned handle; status / recover /
  // restore / discard / cancel / reload. Mode is always copied-target; nothing installs live.
  copied_target_root_for: { args: { projectId: number }; ret: CopiedTargetRootDto };
  copied_target_create_root: { args: { path: string }; ret: string };
  copied_target_review: { args: { projectId: number; root: string }; ret: CopiedTargetReviewDto };
  // As above, also replacing one existing raw member of a copied EB archive (PS4 inxp_00.big /
  // nhl_game.db) with the packaged primary database in the same transaction.
  copied_target_review_archive_member: {
    args: { projectId: number; root: string; archiveRelative: string; memberPath: string };
    ret: CopiedTargetReviewDto;
  };
  // As above for one or more members: each spec names the archive, the member and which plan
  // artifact supplies the bytes (`primary` database or the pending `dictionary` rename).
  copied_target_review_archive_members: {
    args: { projectId: number; root: string; specs: ArchiveMemberSpecDto[] };
    ret: CopiedTargetReviewDto;
  };
  // LOC-W1 dictionary rename: key A shows key B's text. Preview is read-only; set holds it as the
  // project's pending dictionary change (one at a time); clear drops it.
  dictionary_texts: { args: { projectId: number; keys: string[] }; ret: (string | null)[] };
  dictionary_rename_preview: {
    args: { projectId: number; key: string; donor: string };
    ret: DictionaryRepointPreviewDto;
  };
  dictionary_rename_set: {
    args: { projectId: number; key: string; donor: string };
    ret: DictionaryRepointPreviewDto;
  };
  dictionary_rename_clear: { args: { projectId: number }; ret: boolean };
  // LOC-W2: free text re-encoded in place with the dictionary's own tree (must fit the old string).
  dictionary_retext_preview: { args: { projectId: number; key: string; text: string }; ret: DictionaryRetextPreviewDto };
  dictionary_retext_set: { args: { projectId: number; key: string; text: string }; ret: DictionaryRetextPreviewDto };
  dictionary_growth_preview: { args: { projectId: number; ops: DictionaryGrowthOpDto[]; options?: DictionaryGrowthOptionsDto | null }; ret: DictionaryGrowthPreviewDto };
  dictionary_growth_set: { args: { projectId: number; ops: DictionaryGrowthOpDto[]; options?: DictionaryGrowthOptionsDto | null }; ret: DictionaryGrowthPreviewDto };
  detach_dictionary: { args: { projectId: number }; ret: boolean };
  // Xbox 360 disc images: inspect (read-only), stage + open, plan / apply / restore range patches.
  iso_inspect: { args: { image: string }; ret: IsoInspectDto };
  iso_open: {
    args: { image: string; gameId: string; platform: Platform; stagingRoot: string; metaXml?: string | null };
    ret: [IsoStagedDto, ProjectSummaryDto];
  };
  // R2 F2: project-bound; the plan is a server-held review the apply names.
  iso_plan: { args: { projectId: number }; ret: IsoPlanDto };
  iso_status: { args: { projectId: number }; ret: IsoStatusDto };
  iso_apply: { args: { projectId: number; reviewId: bigint }; ret: IsoApplyDto };
  iso_restore: { args: { projectId: number; recordsDir: string }; ret: IsoRestoreDto };
  iso_records: { args: { projectId: number }; ret: IsoRecordDto[] };
  // Phase 1.2 — league membership (league + conference/division groups as this title models them).
  team_membership_options: { args: { projectId: number; teamId: number }; ret: MembershipOptionsDto };
  swap_team_memberships: { args: { projectId:number; first:number; second:number }; ret:OpOutcomeDto };
  set_team_membership: {
    args: { projectId: number; teamId: number; leagueCode: number; conference?: number | null; division?: number | null };
    ret: MembershipResultDto;
  };
  // Phase 1.2 — source schedules (browse + validation).
  schedule_tables: { args: { projectId: number }; ret: ScheduleTableDto[] };
  schedule_view: { args: { projectId: number; studioTable: string }; ret: ScheduleViewDto | null };
  team_schedule_edit: { args: { projectId: number; studioTable: string; team: number; fingerprint: string; record: number | null; remove: boolean; opponent: number | null; home: boolean; month: number; day: number; expectedGames: Array<{ index:number; month:number; day:number; home_slot:number; away_slot:number; round:number; status:number|null }> }; ret: ScheduleViewDto };
  // P1-A — schedule editing / import / export / generation. Every preview writes nothing and is
  // held server-side under an opaque `ticket` (bound to the issuing project instance, its revision and, for imports, the file's
  // bytes); `schedule_apply_review` writes exactly the previewed plan as one undoable transaction
  // or refuses (project changed, file changed, foreign/consumed id, plan with errors).
  schedule_edit_preview: { args: { projectId: number; studioTable: string; edit: ScheduleGameEditArgs }; ret: ScheduleChangePlanDto };
  schedule_apply_review: { args: { projectId: number; ticket: string }; ret: ScheduleApplyResultDto };
  schedule_export: { args: { projectId: number; studioTable: string; format: "csv" | "json"; outPath?: string | null }; ret: ScheduleExportDto };
  schedule_import_preview: {
    args: { projectId: number; studioTable: string; format: "csv" | "json"; text?: string | null; path?: string | null; mapping: ScheduleTeamMappingArgs[] };
    ret: ScheduleImportPlanDto;
  };
  schedule_generation_options: { args: { projectId: number; studioTable: string }; ret: ScheduleGenerationOptionsDto };
  // Receipt 48 — the table's shipped single-phase groups, only by exact source identity (else the reason).
  schedule_stock_groups: { args: { projectId: number; studioTable: string }; ret: ScheduleStockGroupsDto };
  schedule_generate_preview: { args: { projectId: number; studioTable: string; config: ScheduleGenerationConfigArgs }; ret: ScheduleGenerationPlanDto };
  // Phase 1.4 — a league's logo art index and package files (preview/replace via texture_preview / import_texture).
  league_logo: { args: { projectId: number; code: number; installRoot?: string | null }; ret: LeagueLogoDto | null };
  // Phase 2.4 (C9) — a team's logo families (art-id keyed, measured per title), same texture surface.
  team_logo_set: { args: { projectId: number; teamId: number; installRoot?: string | null }; ret: TeamLogoSetDto | null };
  // P1-G — the title's league-keyed presentation imagery beyond the logo families (same texture surface).
  league_imagery: { args: { projectId: number; installRoot?: string | null }; ret: LeagueImageryDto | null };
  // Phase 1.5 / 2.7 / 3.5 — the read-only gen7 commentary catalogue (events, sentences sharing an event's name, phrases → candidate samples → bank).
  speech_catalogue: { args: { projectId: number; installRoot: string; query?: string; leagueOnly?: boolean; limit?: number }; ret: SpeechCatalogueDto };
  team_name_calls: { args: { projectId: number; teamId: number; installRoot: string }; ret: TeamNameCallsDto };
  player_name_calls: { args: { projectId: number; playerId: number; installRoot: string }; ret: PlayerNameCallsDto };
  player_equipment_files: { args: { projectId: number; playerId: number; roots: AssetRootsArgsDto }; ret: EquipmentFileCheckDto[] };
  // Equipment 4.5 G2 (receipt 42) — a team's goalie mask paints (slot × starter / backup) with the gen7 file check.
  team_goalie_masks: { args: { projectId: number; teamId: number; roots: AssetRootsArgsDto }; ret: TeamGoalieMasksDto };
  // Equipment 4.5 G3 (receipt 51) — a PS4 goalie's mask pair against the title's own vault catalogue.
  ps4_goalie_mask_pair: { args: { projectId: number; playerId: number; roots: AssetRootsArgsDto }; ret: Ps4GoalieMaskPairDto };
  // Receipt 52 — a PS4 skater's gear against the title's own Creation Zone lists, and a list thumbnail.
  ps4_skater_gear: { args: { projectId: number; playerId: number; roots: AssetRootsArgsDto }; ret: Ps4SkaterGearDto };
  ps4_gear_thumbnail: { args: { roots: AssetRootsArgsDto; archive: string; member: string }; ret: VenueRasterPreviewDto };
  // About & licences (receipt 46) — the packaged licence, notices and release status, read beside the executable.
  about_studio: { args: Record<string, never>; ret: AboutDto };
  about_licence_file: { args: { path: string }; ret: string };
  context_catalogue: { args: { projectId: number; installRoot: string; query?: string; limit?: number }; ret: ContextCatalogueDto };
  // P4 (24 Sep 2026) — PS4 NHL 15–21 speech banks: listing, per-bank samples with measured SNR headers,
  // and the diagnostic EA-MP3 preview (never game-verified; no replacement route).
  ps4_speech_banks: { args: { projectId: number; installRoot: string }; ret: Ps4SpeechBanksDto };
  ps4_speech_samples: { args: { projectId: number; installRoot: string; bankFile: string; query?: string; limit?: number }; ret: Ps4SpeechSamplesDto };
  ps4_speech_preview: { args: { projectId: number; installRoot: string; bankFile: string; sid: number; rendering: "mpeg_only" | "candidate_a" }; ret: Ps4SpeechPreviewDto };
  // P4.3 — the diagnostic gen7 EALayer3 v1 preview of one speech sample (measured tuples only; never game-verified).
  gen7_speech_preview: { args: { projectId: number; installRoot: string; bankStem: "pbp_eng" | "ann_eng" | "music_samples"; sampleId: number }; ret: Gen7SpeechPreviewDto };
  // Receipt 33 — cancel the project's in-flight gen7 sample preview (stops the EA-XMA helper); true when one was running.
  gen7_speech_preview_cancel: { args: { projectId: number }; ret: boolean };
  // P4.6 — a measured gen7 bank listed from its SBKR table with each sample's SNR header (listing only).
  gen7_bank_samples: { args: { projectId: number; installRoot: string; bankStem: "pbp_eng" | "ann_eng" | "music_samples"; query?: string; limit?: number }; ret: Gen7BankListingDto };
  gen7_sfx_banks: { args: { projectId: number; installRoot: string }; ret: Gen7SfxBankListDto };
  gen7_sfx_bank_tree: { args: { projectId: number; installRoot: string; bank: string }; ret: Gen7SfxBankTreeDto };
  gen7_sfx_preview: { args: { projectId: number; installRoot: string; bank: string; record: number; subStream?: number | null }; ret: Gen7SfxPreviewDto };
  // Misc 5.4 (22 Sep 2026): the PS4 NHL 17–21 song catalogue — selectable songs, never the team's song
  // (the team's song cells have no catalogue join). Read-only; `available: false` carries the reason.
  song_catalogue: { args: { projectId: number; installRoot: string; query?: string; limit?: number }; ret: SongCatalogueDto };
  // Phase 2.1 — a team's identity dependencies and the art-id swap (the only supported art-id change).
  team_identity_dependencies: { args: { projectId: number; teamId: number }; ret: TeamIdentityDependenciesDto };
  team_artid_options: { args: { projectId: number }; ret: [number, number, string][] };
  team_artid_swap: { args: { projectId: number; teamA: number; teamB: number }; ret: ArtIdSwapDto };
  // D8 — replace a texture member inside a packed archive through the copied-target route (PS4 logos / imagery).
  asset_member_preview: { args: { projectId: number; installRoot: string; archiveRelative: string; memberPath: string; sourceImagePath: string; wholeGroup?: boolean }; ret: AssetMemberPreviewDto };
  ps3_board_replacement_preview: { args: { projectId: number; installRoot: string; package: string; descriptor: number; sourceImagePath: string }; ret: BoardReplacementPreviewDto };
  asset_member_apply_review: { args: { projectId: number; ticket: string }; ret: AssetMemberApplyDto };
  asset_member_undo: { args: { projectId: number }; ret: boolean };
  // Phase 2.3 — a team's dictionary key families (keyed by artabbr) beside its database strings.
  team_text: { args: { projectId: number; teamId: number }; ret: TeamTextDto | null };
  settings_catalogue: { args: { projectId: number; installRoot?: string | null }; ret: SettingsCatalogueResultDto };
  settings_default_preview: { args: { projectId: number; legacyId: bigint; context: bigint | null; code: bigint; installRoot?: string | null }; ret: SettingsEditPreviewDto };
  settings_csv_undo: { args: { projectId: number }; ret: boolean };
  settings_apply_review: { args: { projectId: number; ticket: string }; ret: SettingsEditApplyDto };
  copied_target_apply: { args: { projectId: number; handle: number }; ret: CopiedTargetApplyDto };
  copied_target_cancel: { args: { handle: number }; ret: boolean };
  copied_target_discard: { args: { handle: number }; ret: CopiedTargetStatusDto };
  copied_target_status: { args: { root: string }; ret: CopiedTargetStatusDto };
  copied_target_recover: { args: { root: string }; ret: CopiedTargetRecoveryDto };
  copied_target_restore: { args: { root: string; txnId: string }; ret: CopiedTargetRestoreDto };
  copied_target_reload: { args: { projectId: number; discardEdits: boolean }; ret: ProjectBaselineDto };
  list_arenas: { args: { projectId: number }; ret: ArenaSummaryDto[] };
  arena_view: { args: { projectId: number; arenaId: number }; ret: ArenaViewDto | null };
  // Teams 2.2 — what reassigning a team's home arena would change (the exact cell, who stays at the
  // old arena or that it becomes vacant, who already plays at the new one), before the write.
  arena_assignment_preview: { args: { projectId: number; teamId: number; toArena: number }; ret: ArenaAssignmentPreviewDto };
  arena_venue_view: { args: { projectId: number; arenaId: number; installRoot?: string }; ret: ArenaVenueDto | null };
  // Phase 2.5 (22 Sep 2026): the arena's shipped assets — venue model / jumbotron by art id on gen7,
  // team-keyed families through the home teams, candidate / unbound ice layers, PS4 venue directories
  // with no binding claimed. Read-only; null when the arena id is not in the table.
  arena_assets: { args: { projectId: number; roots: AssetRootsArgsDto; arenaId: number }; ret: ArenaAssetsDto | null };
  // Presentation 5.2 (receipt 49) — the read-only venue-art gallery and a SHA-checked raster preview.
  arena_venue_art: { args: { projectId: number; roots: AssetRootsArgsDto; arenaId: number }; ret: VenueArtDto | null };
  arena_venue_raster: { args: { projectId: number; roots: AssetRootsArgsDto; arenaId: number; file: string; sha256: string; ordinal: number; maxEdge: number }; ret: VenueRasterPreviewDto };
  // Phase 3 (22 Sep 2026): every player's portrait / head containment — the ids, `hasportrait` as
  // stored, what the install ships, the shipped-but-unreferenced pools, "shipped without asset"
  // names. Read-only.
  player_containment: { args: { projectId: number; roots: AssetRootsArgsDto }; ret: PlayerContainmentViewDto };
  // Receipt 38 — plan (apply false) or perform (apply true) a portrait / head reassignment to a shipped asset.
  player_reassign: { args: { projectId: number; roots: AssetRootsArgsDto; bioRecord: number; kind: "portrait" | "head" | "hair"; target: number; apply: boolean; reviewed: string | null }; ret: PlayerReassignPlanDto };
  lines_view: { args: { projectId: number; teamId: number }; ret: LinesViewDto | null };
  set_team_roster: { args: { projectId: number; team: number; writes: Array<[number, string, number]> }; ret: OpOutcomeDto };
  team_strategies: { args: { projectId: number; teamId: number }; ret: TeamStrategiesDto | null };
  team_draft_picks: { args: { projectId: number; teamId: number }; ret: DraftPickDto[] };
  // Trade a future draft pick (qHav row) to another team; provenance (original owner) is kept.
  trade_draft_pick: {
    args: { projectId: number; record: number; toTeam: number };
    ret: OpOutcomeDto;
  };
  // Player transaction ops (the Transactions page's verbs).
  move_player: {
    args: { projectId: number; player: number; fromTeam: number; toTeam: number };
    ret: OpOutcomeDto;
  };
  clone_player: {
    args: { projectId: number; sourcePlayer: number; toTeam: number };
    ret: CreatePlayerDto;
  };
  release_to_fa: { args: { projectId: number; fromTeam: number; player: number }; ret: OpOutcomeDto };
  sign_from_fa: { args: { projectId: number; player: number; toTeam: number }; ret: OpOutcomeDto };
  // Swap two occupants in place (both blocks full) — captaincy cleared, colliding jerseys repaired.
  trade_slots: {
    args: { projectId: number; aTeam: number; aPlayer: number; bTeam: number; bPlayer: number };
    ret: OpOutcomeDto;
  };
  // The free-agent pool: indexed players occupying no roster slot (Spike S1 occupancy model).
  list_free_agents: { args: { projectId: number }; ret: PlayerSummaryDto[] };
  delete_player: { args: { projectId: number; player: number }; ret: OpOutcomeDto };
  list_team_jerseys: { args: { projectId: number; teamId: number }; ret: TeamJerseyDto[] };
  list_team_equipment: { args: { projectId: number; teamId: number }; ret: TeamEquipmentDto[] };
  // Player ids occupying a team's eGlu roster block — the roster for membership-only teams
  // (Winter Classic / all-star / national), whose players keep another main team.
  team_roster_ids: { args: { projectId: number; teamId: number }; ret: number[] };
  // Set a player's captaincy on a team's roster (0 = None, 1 = Captain, 2 = Alternate) —
  // auto-demotes a prior captain; only dressed players may hold a role.
  set_captaincy: {
    args: { projectId: number; team: number; player: number; role: number };
    ret: OpOutcomeDto;
  };
  // Per-team slot jersey number (eGlu.sVRt); unique within the team, 0 clears.
  set_jersey_number: {
    args: { projectId: number; team: number; player: number; number: number };
    ret: OpOutcomeDto;
  };
  retire_jersey: { args: { projectId: number; team: number; number: number }; ret: OpOutcomeDto };
  unretire_jersey: { args: { projectId: number; team: number; number: number }; ret: OpOutcomeDto };
  // Lineup status on the player's slot (eGlu.hSZj): dressed vs. scratched. ("In minors" is not a
  // stored status — it is a depth-chart identifier derived from the player's main team.)
  set_dressed: {
    args: { projectId: number; team: number; player: number; dressed: boolean };
    ret: OpOutcomeDto;
  };
  set_field: {
    args: {
      projectId: number;
      table: string;
      record: number;
      field: string;
      intValue?: number | null;
      textValue?: string | null;
    };
    ret: OpOutcomeDto;
  };
  // C8: several integer cells of one record as one operation (team colour `_r/_g/_b` triples or
  // the packed word) — one history entry, one undo.
  set_cells_int: {
    args: { projectId: number; table: string; record: number; writes: Array<{ field: string; value: number }> };
    ret: OpOutcomeDto;
  };
  // Activate/deactivate a trait on a player's ratings record (dSvy/mHuy). One atomic slot edit.
  set_player_trait: {
    args: {
      projectId: number;
      table: string;
      record: number;
      value: number;
      active: boolean;
    };
    ret: OpOutcomeDto;
  };
  // Add an extra roster membership (national / all-star / alternate roster); primary team untouched.
  add_membership: {
    args: { projectId: number; player: number; toTeam: number };
    ret: OpOutcomeDto;
  };
  // Remove one extra roster membership (vacate the slot); primary team untouched.
  remove_membership: {
    args: { projectId: number; fromTeam: number; player: number };
    ret: OpOutcomeDto;
  };
  undo: { args: { projectId: number }; ret: boolean };
  redo: { args: { projectId: number }; ret: boolean };
  save_project: { args: { projectId: number; outPath: string }; ret: number };
  // In-place dual save: writes nhlng.db and (when attached) the roster save back to their origins.
  save_workspace: { args: { projectId: number }; ret: number };
  // Save Editor: identify a chosen save / database (reads only), open it as its own session under
  // the chosen game and platform, and save it back (the original is backed up on the first save).
  save_editor_identify: { args: { path: string }; ret: SaveFileIdentityDto };
  save_editor_open: {
    args: { path: string; gameId: string; platform: string; acknowledged: boolean; metaOverride: string | null };
    ret: SaveEditorOpenDto;
  };
  save_editor_save: { args: { projectId: number }; ret: SaveEditorSaveDto };
  // Release a project's server-side workspace (sessions + any prepared copied-target review).
  // Acknowledged outcome: `closed` / `already_closed` (dispose the owner) or `busy` / `failed` (keep it).
  close_project: { args: { projectId: number }; ret: CloseProjectDto };

  // Historical (FHM) corpus — app-global, read-only; no projectId. Open a data pack once, then query.
  open_corpus: { args: { path: string }; ret: number };
  corpus_open: { args: Record<string, never>; ret: boolean };
  historical_filters: { args: Record<string, never>; ret: HistoricalFiltersDto };
  // Person-centric player browse. goalieFilter: true = goalies only, false = skaters only, null = all.
  list_historical_players: {
    args: { search: string; goalieFilter: boolean | null; limit: number };
    ret: HistoricalPersonSummaryDto[];
  };
  historical_player: { args: { personId: number }; ret: HistoricalPersonDto | null };
  // Team browse is season-scoped (team ids are per-season).
  list_historical_teams: {
    args: { seasonYear: number; search: string; limit: number };
    ret: HistoricalTeamSummaryDto[];
  };
  historical_team: {
    args: { seasonYear: number; srcTeamId: number };
    ret: HistoricalTeamDto | null;
  };
  // Franchise-consolidated team browse: NHL relocations/renames collapse to one franchise entry;
  // non-NHL clubs stay standalone. Filtered by league (`"NHL"` → the franchises; null → everything).
  historical_team_filters: { args: Record<string, never>; ret: HistoricalTeamFiltersDto };
  list_historical_franchises: {
    args: { league: string | null; search: string; limit: number };
    ret: HistoricalTeamIndexEntryDto[];
  };
  // One franchise/standalone-team's detail for a season (seasonYear null → newest). kind = "franchise"
  // (key = franchise id) or "team" (key = team_key). Returns all constituents active that season.
  historical_franchise: {
    args: { kind: string; key: string; seasonYear: number | null };
    ret: HistoricalTeamFranchiseDto | null;
  };

  // Roster Builder (Stage 3) — bridge the corpus and an open EA project. Each needs both an open
  // corpus AND `projectId`.
  // Auto-detect EA identity matches for a corpus person (the cart's Match/Create toggle).
  match_ea_player: {
    args: { projectId: number; personId: number };
    ret: MatchCandidateDto[];
  };
  // Dry-run a cart: per-item outcomes, player-count delta, audit, blockers, ledger dedup flag.
  import_preview: {
    args: { projectId: number; items: ImportItemDto[] };
    ret: ChangeSetPreviewDto;
  };
  // Commit a cart: the same lowering applied for real as one transaction / one undo entry, then ledgered.
  import_commit: {
    args: { projectId: number; items: ImportItemDto[] };
    ret: ImportResultDto;
  };

  // Formula engine (Stage 2) — compute EA ratings from FHM attrs; profiles are corpus-global.
  formula_default: { args: Record<string, never>; ret: FormulaProfileDto };
  formula_catalog: { args: Record<string, never>; ret: FormulaCatalogDto };
  compute_ratings: {
    args: { personId: number; seasonYear: number; profile: FormulaProfileDto };
    ret: EaComputedRatingsDto | null;
  };
  // Cross-career EA rating compilations (Full Career + Peak 3 + per-season overalls + best season).
  compute_career_ratings: {
    args: { personId: number; profile: FormulaProfileDto };
    ret: HistoricalCareerRatingsDto | null;
  };
  // The active (working) formula profile — auto-persisted as the operator edits, and the default
  // the Attributes preview computes against across the app.
  get_active_formula: { args: Record<string, never>; ret: FormulaProfileDto };
  save_active_formula: { args: { profile: FormulaProfileDto }; ret: null };
  list_formula_profiles: { args: Record<string, never>; ret: string[] };
  load_formula_profile: { args: { name: string }; ret: FormulaProfileDto };
  save_formula_profile: { args: { name: string; profile: FormulaProfileDto }; ret: null };
  delete_formula_profile: { args: { name: string }; ret: null };

  // One CSV row per installed player: identity, export slug, team, league, and whether a portrait
  // and head mesh resolve on disk. Needs the project's install tree (the art columns are file truth).
  export_player_manifest: {
    args: { projectId: number; roots: AssetRootsArgsDto; outPath: string };
    ret: PlayerManifestSummaryDto;
  };

  // ---- Legends builder ----
  // Ratings are precomputed per formula profile into the `legends_cache.db` sidecar; the roster
  // reads are cheap lookups over that cache, `compute_legends_ratings` is the long full-corpus pass.
  // The Formulas live preview's "Legends" option — computes one player's composite against the
  // profile being edited, so unsaved weight changes show through (the cache only holds saved ones).
  compute_legends_preview: {
    args: { personId: number; profile: FormulaProfileDto };
    ret: EaComputedRatingsDto | null;
  };
  legends_status: { args: Record<string, never>; ret: LegendsStatusDto };
  set_active_profile: { args: { name: string }; ret: null };
  set_legends_pending: { args: { pending: boolean; dismissed: boolean }; ret: null };
  compute_legends_ratings: { args: Record<string, never>; ret: LegendsComputeSummaryDto };
  legends_teams: { args: Record<string, never>; ret: LegendsTeamSummaryDto[] };
  legends_roster: { args: { teamKey: string }; ret: LegendsMemberDto[] };

  // ---- Assets: player portraits / head meshes / head textures ----
  // `roots` is a struct (not flat args), so its fields stay snake_case like the DTO.
  asset_roots: { args: { roots: AssetRootsArgsDto }; ret: AssetRootsDto };
  list_player_assets: {
    args: { projectId: number; roots: AssetRootsArgsDto };
    ret: PlayerAssetsDto[];
  };
  // Returns a `data:image/png;base64,...` URL.
  portrait_preview: { args: { path: string; crop: boolean }; ret: string };
  // Portrait framed for the bottom-anchored header tile (cropped to content + lowest solid row).
  portrait_header_preview: { args: { path: string }; ret: string };
  // A team's outfit textures (jersey/pants/socks) resolved from the install art tree; empty when the
  // title ships none. Each entry's `rel_path` feeds texture_preview / import_texture.
  team_outfits: {
    args: { projectId: number; roots: AssetRootsArgsDto; teamId: number; includeAmbiguous?: boolean };
    ret: OutfitTextureDto[];
  };
  // Phase 2.6 (22 Sep 2026): the team's kit rows joined to their jersey / pant / sock textures under
  // the title's measured contract — per-component present / missing / unknown evidence, the shipped
  // spare candidates (same completeness rule) and why re-pointing is not offered. Read-only.
  team_kits: {
    args: { projectId: number; roots: AssetRootsArgsDto; teamId: number };
    ret: TeamKitsDto;
  };
  // The team's logo as a cropped PNG data URL for the header, or null when the title ships no logo.
  team_logo: {
    args: { projectId: number; roots: AssetRootsArgsDto; teamId: number };
    ret: string | null;
  };
  // Decode a player's head mesh (by head id) for the 3D viewer.
  head_preview: {
    args: { roots: AssetRootsArgsDto; headId: number };
    ret: HeadPreviewDto;
  };
  // Decode a head from already-resolved paths (PS4 bespoke, name-slug keyed).
  head_preview_paths: {
    args: { meshPaths: string[]; texturePaths: string[] };
    ret: HeadPreviewDto;
  };
  // Assemble a full player kit (head + helmet + jersey + gloves + pants + socks
  // + skates) for the 3D viewer from install-relative piece paths.
  kit_preview_paths: {
    args: { installDir: string; pieces: KitPieceInputDto[] };
    ret: KitPreviewDto;
  };
  // Assemble a team's full kit (selected colorway + representative player head +
  // glove/helmet) for the 3D viewer. playerId picks the head; absent = top-overall.
  team_kit_preview: {
    args: {
      projectId: number;
      roots: AssetRootsArgsDto;
      teamId: number;
      style: number;
      variant: number;
      playerId?: number | null;
      // Resolve gear from the previewed player's own selected equipment ids
      // (`xmja`/`fMVl`) rather than the team defaults — the player Equipment tab.
      playerGear?: boolean;
    };
    ret: KitPreviewDto;
  };
  // Hockey Shop — the equipment-slot catalog + per-slot item browse + item 3D preview.
  equipment_slots: { args: Record<string, never>; ret: EquipmentSlotDto[] };
  list_equipment_slot_assets: {
    args: { roots: AssetRootsArgsDto; slot: string; game?: string | null };
    ret: EquipmentSlotAssetDto[];
  };
  equipment_item_preview: {
    args: { roots: AssetRootsArgsDto; slot: string; id: number };
    ret: KitPreviewDto;
  };
  hockey_shop_list: {
    args: { storeRoot: string; slot: string };
    ret: HockeyShopItemDto[];
  };
  hockey_shop_sync: {
    args: { projectId?: number | null; roots: AssetRootsArgsDto; storeRoot: string; slot: string; game?: string; gameId?: string | null };
    ret: HockeyShopSyncSummaryDto;
  };
  hockey_shop_import: {
    args: { storeRoot: string; slot: string; name: string; author: string; imageBase64: string };
    ret: HockeyShopItemDto;
  };
  hockey_shop_delete: {
    args: { storeRoot: string; slot: string; entryId: string };
    ret: null;
  };
  hockey_shop_load_texture: {
    args: { storeRoot: string; slot: string; entryId: string };
    ret: string;
  };
  // Receipt 47: the open project's title names the files; apply must name the reviewed plan's fingerprint.
  hockey_shop_install_plan: {
    args: { projectId?: number | null; roots: AssetRootsArgsDto; storeRoot: string; request: InstallStoreItemRequestDto };
    ret: InstallStoreItemPlanDto;
  };
  hockey_shop_install_apply: {
    args: { projectId?: number | null; roots: AssetRootsArgsDto; storeRoot: string; request: InstallStoreItemRequestDto };
    ret: InstallStoreItemApplyResponseDto;
  };
  // `bioRecords: []` exports every scanned player.
  export_assets: {
    args: {
      projectId: number;
      roots: AssetRootsArgsDto;
      outDir: string;
      bioRecords: number[];
      options: ExportOptionsDto;
    };
    ret: AssetExportSummaryDto;
  };
  // Appearance catalog: every head + hair + facial hair (players + non-players),
  // GLB + PNG with a catalog.json manifest.
  export_head_catalog: {
    args: {
      projectId: number;
      roots: AssetRootsArgsDto;
      outDir: string;
      textures: boolean;
    };
    ret: HeadCatalogSummaryDto;
  };
  // Read a built catalog's catalog.json back for the Appearance Studio browser.
  read_catalog_manifest: { args: { dir: string }; ret: CatalogManifestDto };
  // Decode one catalog asset (source container filtered to the entry's
  // submeshes), textured from its source texture containers. `textureSources`
  // is optional — omitted/empty makes the backend re-resolve maps from the
  // source folder, so preview stays textured even for pre-`texture_sources`
  // catalogs.
  catalog_entry_preview: {
    args: { source: string; submeshes: string[]; textureSources?: string[] };
    ret: HeadPreviewDto;
  };
  // Read an exported catalog PNG off disk as a `data:image/png;base64,…` URL.
  read_png_data_url: { args: { path: string }; ret: string };

  // ---- Assets: install file browser (decode any picked file/folder) ----
  // `rel` is a forward-slash path relative to the install root (`""` = the root).
  browse_install_files: {
    args: { installDir: string; rel: string };
    ret: BrowseListingDto;
  };
  // Decode the selected files/folders (rel paths) into `outDir`, mirroring the install layout.
  export_install_files: {
    args: {
      installDir: string;
      relPaths: string[];
      outDir: string;
      options: FileExportOptionsDto;
    };
    ret: FileExportSummaryDto;
  };

  // ---- Assets: team outfit export (jersey/pants/socks per team → PNGs named by team) ----
  export_team_outfits: {
    args: { projectId: number; roots: AssetRootsArgsDto; outDir: string };
    ret: OutfitExportSummaryDto;
  };

  // ---- Assets: texture import (write an edited PNG back into an extracted file) ----
  // `relPath` is resolved safely under `installDir`; dispatches on the file's container (GNF /
  // BIGF portrait / RW4 texlib). Writes a one-time `.bak` when `writeBackup` is set. `reviewId`
  // names the texture_replace_plan the operator saw (24 Sep 2026 review F1): the write refuses,
  // before any backup or mutation, when the target's bytes, the source, the raster, the selection
  // or the policy differ from that review. A review is consumed by the write that names it.
  import_texture: {
    args: {
      installDir: string;
      relPath: string;
      sourceImagePath: string;
      writeBackup: boolean;
      // Raster within a multi-texture texlib (jersey body vs. number font); omit for the primary.
      textureIndex?: number;
      reviewId: bigint;
    };
    ret: ImportTextureSummaryDto;
  };
  // Write the UI-baked PNG held by `reviewId` (from texture_replace_plan_bytes) back into an
  // extracted texture — the bytes twin of import_texture (the nameplate editor). Same drift
  // refusals; one-time `.bak` when `writeBackup` is set.
  import_texture_bytes: {
    args: {
      installDir: string;
      relPath: string;
      writeBackup: boolean;
      textureIndex?: number;
      reviewId: bigint;
    };
    ret: ImportTextureSummaryDto;
  };
  // Decode the current on-disk texture (the Replace "before" / a row thumbnail). `maxEdge > 0`
  // downscales for a thumbnail; `0` = full size. `textureIndex` targets one raster of a texlib
  // bundle (jersey body vs. font); omit for the auto-picked colour map. Reports dims + container.
  texture_preview: {
    args: { installDir: string; relPath: string; maxEdge: number; textureIndex?: number };
    ret: TexturePreviewDto;
  };
  // Decode an operator-picked source image (the Replace "after") to a PNG data URL + dims.
  source_image_preview: {
    args: { sourceImagePath: string };
    ret: TexturePreviewDto;
  };
  // Preview a texture inside an EB render archive (PS3 NHL 12 / 13 kit members): the archive by
  // install-relative path, the member by exact path. Read-only; no write route.
  archive_member_texture_preview: {
    args: { installDir: string; archiveRelPath: string; memberPath: string; maxEdge: number; textureIndex?: number };
    ret: TexturePreviewDto;
  };
  // What a Replace… of `relPath` with the image at `sourceImagePath` would do, before Apply: the
  // refusal or the partial-replacement disclosure the write itself returns (Xbox 360 RW4xb2 DXT
  // rasters); `checked: false` for containers with no pre-check (the writer's rule decides). The
  // answer is a server-held review (`review_id`) the write must name.
  texture_replace_plan: {
    args: { installDir: string; relPath: string; sourceImagePath: string; textureIndex?: number };
    ret: TextureReplacePlanDto;
  };
  // texture_replace_plan for a PNG the UI baked (base64); the decoded image is held in the review
  // and import_texture_bytes writes exactly that image.
  texture_replace_plan_bytes: {
    args: { installDir: string; relPath: string; pngBase64: string; textureIndex?: number };
    ret: TextureReplacePlanDto;
  };

  // ---- Uniforms: Add New Jersey (clone) + the in-game jersey-select thumbnail ----
  // Clone an `xjgw` jersey row into a new kit (append + copy + identity stamp), staged as a
  // pending, undoable edit. `jerseyIndex` omitted → smallest unused per-team index.
  clone_jersey: {
    args: {
      projectId: number;
      sourceRecord: number;
      team: number;
      style: number;
      artId: number;
      variant: number;
      jerseyIndex?: number | null;
    };
    ret: CloneJerseyResultDto;
  };
  // The in-game jersey-select preview image for a kit's {style}_{art}_{variant} asset id,
  // resolved through the game's own fallback chain (stock archive → shared custom*.big).
  team_jersey_thumbnail: {
    args: { roots: AssetRootsArgsDto; style: number; artId: number; variant: number };
    ret: JerseyThumbnailDto;
  };
  // Clone a kit's texture files (outfit folders + FE thumbnail) onto a new asset id — the file
  // half of Add New Jersey. Same-length asset ids only; never clobbers existing files.
  clone_kit_textures: {
    args: {
      roots: AssetRootsArgsDto;
      srcStyle: number;
      srcArtId: number;
      srcVariant: number;
      dstStyle: number;
      dstArtId: number;
      dstVariant: number;
    };
    ret: CloneKitTexturesSummaryDto;
  };
};

export async function call<C extends keyof Commands>(
  cmd: C,
  args: Commands[C]["args"],
): Promise<Commands[C]["ret"]> {
  // These operations must observe all field edits already submitted by the user.
  if (["project_save", "save_workspace", "save_editor_save", "close_project", "undo", "redo", "project_build", "package_build", "project_archive", "project_copy_to"].includes(cmd)) {
    const projectId = (args as { projectId?: number }).projectId;
    if (projectId != null) await awaitFieldWrites(projectId);
  }
  return invoke<Commands[C]["ret"]>(cmd, args as Record<string, unknown>);
}
