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
import type { OpOutcomeDto } from "../bindings/OpOutcomeDto";

export type Platform = "ps3" | "xbox" | "ps4" | "recomp";
export type Role = "nhlng" | "roster" | "localization";

type Commands = {
  load_project: {
    args: { dbPath: string; platform: Platform; role: Role; metaXml: string; defsRoot: string };
    ret: ProjectSummaryDto;
  };
  project_summary: { args: { projectId: number }; ret: ProjectSummaryDto };
  validate_project: { args: { projectId: number }; ret: IntegrityReportDto };
  list_players: { args: { projectId: number }; ret: PlayerSummaryDto[] };
  player_view: { args: { projectId: number; playerId: number }; ret: PlayerViewDto | null };
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
};

export async function call<C extends keyof Commands>(
  cmd: C,
  args: Commands[C]["args"],
): Promise<Commands[C]["ret"]> {
  return invoke<Commands[C]["ret"]>(cmd, args as Record<string, unknown>);
}
