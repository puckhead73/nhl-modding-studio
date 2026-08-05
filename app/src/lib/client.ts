// Typed wrapper over Tauri's `invoke`, keyed by command name → { args, return }.
//
// Convention: Tauri v2 maps **camelCase** JS argument keys to the snake_case Rust command
// parameters, so args here use camelCase (projectId, playerId, …). A Rust `Err(String)` surfaces as
// a promise rejection with that string — React Query turns it into the query/mutation `error`.

import { invoke } from "@tauri-apps/api/core";
import type { ProjectSummaryDto } from "../bindings/ProjectSummaryDto";
import type { IntegrityReportDto } from "../bindings/IntegrityReportDto";
import type { PlayerSummaryDto } from "../bindings/PlayerSummaryDto";
import type { PlayerViewDto } from "../bindings/PlayerViewDto";
import type { TeamSummaryDto } from "../bindings/TeamSummaryDto";
import type { TeamViewDto } from "../bindings/TeamViewDto";
import type { ArenaSummaryDto } from "../bindings/ArenaSummaryDto";
import type { ArenaViewDto } from "../bindings/ArenaViewDto";
import type { LinesViewDto } from "../bindings/LinesViewDto";
import type { CareerStatsDto } from "../bindings/CareerStatsDto";
import type { AddCareerSeasonDto } from "../bindings/AddCareerSeasonDto";
import type { OpOutcomeDto } from "../bindings/OpOutcomeDto";
import type { InstallReportDto } from "../bindings/InstallReportDto";
import type { ExtractSummaryDto } from "../bindings/ExtractSummaryDto";
import type { AutocorrectSummaryDto } from "../bindings/AutocorrectSummaryDto";
import type { RepairBlocksSummaryDto } from "../bindings/RepairBlocksSummaryDto";
import type { CreatePlayerDto } from "../bindings/CreatePlayerDto";
import type { TeamStrategiesDto } from "../bindings/TeamStrategiesDto";
import type { DraftPickDto } from "../bindings/DraftPickDto";
import type { TeamJerseyDto } from "../bindings/TeamJerseyDto";
import type { TeamEquipmentDto } from "../bindings/TeamEquipmentDto";
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
import type { FormulaProfileDto } from "../bindings/FormulaProfileDto";
import type { FormulaCatalogDto } from "../bindings/FormulaCatalogDto";
import type { AssetRootsDto } from "../bindings/AssetRootsDto";
import type { AssetRootsArgsDto } from "../bindings/AssetRootsArgsDto";
import type { PlayerAssetsDto } from "../bindings/PlayerAssetsDto";
import type { ExportOptionsDto } from "../bindings/ExportOptionsDto";
import type { AssetExportSummaryDto } from "../bindings/AssetExportSummaryDto";
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
import type { OutfitTextureDto } from "../bindings/OutfitTextureDto";
import type { MatchCandidateDto } from "../bindings/MatchCandidateDto";
import type { ImportItemDto } from "../bindings/ImportItemDto";
import type { ChangeSetPreviewDto } from "../bindings/ChangeSetPreviewDto";
import type { ImportResultDto } from "../bindings/ImportResultDto";

export type Platform = "ps3" | "xbox" | "ps4" | "recomp";
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
  // locates the roster/profile saves for Xbox/Recomp (whose emulator saves aren't in a fixed place).
  detect_install: {
    args: { gameId: string; platform: Platform; root: string; savePath?: string | null };
    ret: InstallReportDto;
  };
  // Open the project from a root folder; enforces the modding minimum and writes the profile sidecar.
  open_workspace: {
    args: {
      gameId: string;
      platform: Platform;
      root: string;
      savePath?: string | null;
      bigFilesExtracted: boolean;
    };
    ret: ProjectSummaryDto;
  };
  // Extract a game's `.big` archives in place; returns a summary of what was written.
  extract_big_files: {
    args: { gameId: string; platform: Platform; root: string };
    ret: ExtractSummaryDto;
  };
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
  list_arenas: { args: { projectId: number }; ret: ArenaSummaryDto[] };
  arena_view: { args: { projectId: number; arenaId: number }; ret: ArenaViewDto | null };
  lines_view: { args: { projectId: number; teamId: number }; ret: LinesViewDto | null };
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
    args: { projectId: number; roots: AssetRootsArgsDto; teamId: number };
    ret: OutfitTextureDto[];
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
    args: { roots: AssetRootsArgsDto; slot: string };
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
    args: { roots: AssetRootsArgsDto; storeRoot: string; slot: string };
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
  hockey_shop_install_plan: {
    args: { roots: AssetRootsArgsDto; storeRoot: string; request: InstallStoreItemRequestDto };
    ret: InstallStoreItemPlanDto;
  };
  hockey_shop_install_apply: {
    args: { roots: AssetRootsArgsDto; storeRoot: string; request: InstallStoreItemRequestDto };
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
  // BIGF portrait / RW4 texlib). Writes a one-time `.bak` when `writeBackup` is set.
  import_texture: {
    args: {
      installDir: string;
      relPath: string;
      sourceImagePath: string;
      writeBackup: boolean;
      // Raster within a multi-texture texlib (jersey body vs. number font); omit for the primary.
      textureIndex?: number;
    };
    ret: ImportTextureSummaryDto;
  };
  // Write an in-memory PNG (base64) back into an extracted texture — the bytes twin of import_texture,
  // for a UI-baked texture (the nameplate editor). Writes a one-time `.bak` when `writeBackup` is set.
  import_texture_bytes: {
    args: {
      installDir: string;
      relPath: string;
      pngBase64: string;
      writeBackup: boolean;
      textureIndex?: number;
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
};

export async function call<C extends keyof Commands>(
  cmd: C,
  args: Commands[C]["args"],
): Promise<Commands[C]["ret"]> {
  return invoke<Commands[C]["ret"]>(cmd, args as Record<string, unknown>);
}
