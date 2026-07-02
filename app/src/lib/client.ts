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
import type { OpOutcomeDto } from "../bindings/OpOutcomeDto";
import type { InstallReportDto } from "../bindings/InstallReportDto";
import type { ExtractSummaryDto } from "../bindings/ExtractSummaryDto";
import type { AutocorrectSummaryDto } from "../bindings/AutocorrectSummaryDto";

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
  // On-load duplicate-jersey auto-correction; returns the corrections applied (staged as a pending edit).
  autocorrect_jerseys: { args: { projectId: number }; ret: AutocorrectSummaryDto };
  list_players: { args: { projectId: number }; ret: PlayerSummaryDto[] };
  player_view: { args: { projectId: number; playerId: number }; ret: PlayerViewDto | null };
  career_stats: { args: { projectId: number; playerId: number }; ret: CareerStatsDto };
  list_teams: { args: { projectId: number }; ret: TeamSummaryDto[] };
  team_view: { args: { projectId: number; teamId: number }; ret: TeamViewDto | null };
  list_arenas: { args: { projectId: number }; ret: ArenaSummaryDto[] };
  arena_view: { args: { projectId: number; arenaId: number }; ret: ArenaViewDto | null };
  lines_view: { args: { projectId: number; teamId: number }; ret: LinesViewDto | null };
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
  undo: { args: { projectId: number }; ret: boolean };
  redo: { args: { projectId: number }; ret: boolean };
  save_project: { args: { projectId: number; outPath: string }; ret: number };
  // In-place dual save: writes nhlng.db and (when attached) the roster save back to their origins.
  save_workspace: { args: { projectId: number }; ret: number };
};

export async function call<C extends keyof Commands>(
  cmd: C,
  args: Commands[C]["args"],
): Promise<Commands[C]["ret"]> {
  return invoke<Commands[C]["ret"]>(cmd, args as Record<string, unknown>);
}
