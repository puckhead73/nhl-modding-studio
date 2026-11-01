// NHL 24 roster editing command contract (crates/studio-api/src/registry/nhl24.rs).
// Saving the opened session writes Studio's overlay into the install's game-mods folder.
import { invoke } from "@tauri-apps/api/core";
import type { ProjectSummaryDto } from "../bindings/ProjectSummaryDto";
export type Nhl24Status = {
  install: string;
  overlay_files: string[];
  other_mod_files: number;
  emulator_running: string | null;
  open_project: number | null;
};
export type Nhl24Opened = { summary: ProjectSummaryDto; db_path: string; from_overlay: boolean; status: Nhl24Status };
export const nhl24Status = (install: string) => invoke<Nhl24Status>("nhl24_roster_status", { install });
export const nhl24Open = (install: string, oodle: string) => invoke<Nhl24Opened>("nhl24_roster_open", { install, oodle });
export const nhl24Restore = (install: string) => invoke<Nhl24Status>("nhl24_roster_restore", { install });
