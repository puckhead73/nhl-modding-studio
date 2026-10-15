import type { Platform } from "./client";
import { gameById } from "./catalog";
import type { ModLibraryEntryDto } from "../bindings/ModLibraryEntryDto";
import type { InstallRecordDto } from "../bindings/InstallRecordDto";
export type ManagerTarget = { gameId: string; platform: Platform; root: string };
export type ManagerPreferences = { downloads: string; targets: ManagerTarget[]; selected: string };
export const MANAGER_KEY = "nhl-modding-studio.mod-manager.v1";
export const MANAGER_BUILDS_KEY = "nhl-modding-studio.mod-manager.builds.v1";
export function loadSentBuilds(): string[] {
  try { const paths = JSON.parse(localStorage.getItem(MANAGER_BUILDS_KEY) ?? "[]"); return Array.isArray(paths) ? paths.filter((p): p is string => typeof p === "string") : []; }
  catch { return []; }
}
export function sendBuilds(paths: string[]) {
  localStorage.setItem(MANAGER_BUILDS_KEY, JSON.stringify([...new Set([...loadSentBuilds(), ...paths])]));
}
export const containingFolder = (path: string) => path.slice(0, Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")));
export const targetKey = (t: ManagerTarget) => `${t.gameId}:${t.platform}`;
const normalizeGame = (game: string) => game === "legacyrecomp" ? "nhllegacy" : game;
const normalizePath = (path: string) => path.replace(/\\/g, "/").replace(/^\/\/\?\//, "").replace(/\/+$/, "").toLowerCase();
export const sameManagerPath = (a: string, b: string) => normalizePath(a) === normalizePath(b);
/** Match package identity and the installation, never just the displayed mod name. */
export function installationRecords(records: InstallRecordDto[], mod: ModLibraryEntryDto, target: ManagerTarget) {
  return targetInstallationRecords(records, target).filter(r => r.package_id === mod.package_id);
}
export function targetInstallationRecords(records: InstallRecordDto[], target: ManagerTarget) {
  return records.filter(r => r.platform === target.platform &&
    normalizeGame(r.game_id) === normalizeGame(target.gameId) && normalizePath(r.picked_root) === normalizePath(target.root) && r.restorable)
    .sort((a, b) => b.seq - a.seq);
}
export function loadManager(): ManagerPreferences {
  try {
    const value = JSON.parse(localStorage.getItem(MANAGER_KEY) ?? "null");
    if (value && typeof value.downloads === "string" && Array.isArray(value.targets)) {
      return { downloads: value.downloads, selected: typeof value.selected === "string" ? value.selected : "",
        targets: value.targets.filter((t: ManagerTarget) => t && typeof t.root === "string" && t.root.trim() && gameById(t.gameId)?.platforms.includes(t.platform)) };
    }
  } catch { /* Start setup when preferences are absent or unreadable. */ }
  return { downloads: "", targets: [], selected: "" };
}
export function matchesTarget(mod: ModLibraryEntryDto, target: ManagerTarget) {
  return normalizeGame(mod.game_id) === normalizeGame(target.gameId) && mod.platforms.includes(target.platform);
}
