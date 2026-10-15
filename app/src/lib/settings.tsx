// Studio-wide UI preferences, persisted to localStorage (these are user preferences, not game data —
// no backend/profile involvement). Two things live here:
//   • the default startup target — the game + platform onboarding jumps straight into, and
//   • saved paths per (game, platform) — the install root and, where relevant, the save-file folder,
//     so a returning user doesn't re-browse every time.
// Consumed by OpenProject (onboarding pre-fill) and the Settings page (the editor for all of this).

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import type { Platform } from "./client";

/** One saved location for a (game, platform): the install root and the save-file folder (blank when
 *  the platform finds saves automatically). PS4 also remembers the roster save chosen from the
 *  inventory (`saveFile`; absent on entries saved before N21-001). Legacy `projectDir` values
 *  remain readable, but new projects use the global projectsRoot plus an explicit name. */
export type PathEntry = { root: string; savePath: string; saveFile?: string; projectDir?: string };

export type StudioSettings = {
  autosaveMinutes: number;
  /** Shared parent folder for newly created projects, independent of game/platform. */
  projectsRoot: string;
  /** Game onboarding loads into on startup, or null for the game picker. */
  defaultGameId: string | null;
  /** Platform paired with `defaultGameId`, or null. */
  defaultPlatform: Platform | null;
  /** Saved paths keyed by `${gameId}:${platform}`. */
  paths: Record<string, PathEntry>;
  /** ONB-003: project folders opened most recently, newest first (a convenience list; opening
   *  one still goes through the full open with its validation). */
  recentProjects: string[];
};

const STORAGE_KEY = "nhl-modding-studio.settings.v1";
const EMPTY: StudioSettings = { autosaveMinutes: 10, projectsRoot: "", defaultGameId: null, defaultPlatform: null, paths: {}, recentProjects: [] };
export function validAutosaveMinutes(value: unknown): number {
  return typeof value === "number" && Number.isInteger(value) && value >= 1 && value <= 1440 ? value : 10;
}
const RECENT_LIMIT = 8;
const BLANK_ENTRY: PathEntry = { root: "", savePath: "" };

export function pathKey(gameId: string, platform: Platform): string {
  return `${gameId}:${platform}`;
}

function load(): StudioSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return EMPTY;
    const parsed = JSON.parse(raw) as Partial<StudioSettings>;
    return {
      autosaveMinutes: validAutosaveMinutes(parsed.autosaveMinutes),
      projectsRoot: typeof parsed.projectsRoot === "string" ? parsed.projectsRoot : "",
      defaultGameId: parsed.defaultGameId ?? null,
      defaultPlatform: parsed.defaultPlatform ?? null,
      paths: parsed.paths ?? {},
      recentProjects: Array.isArray(parsed.recentProjects) ? parsed.recentProjects.filter((p) => typeof p === "string") : [],
    };
  } catch {
    return EMPTY;
  }
}

type SettingsCtx = {
  setAutosaveMinutes: (minutes: number) => void;
  settings: StudioSettings;
  setProjectsRoot: (root: string) => void;
  /** Set (or clear, with nulls) the game+platform onboarding starts in. */
  setDefaultTarget: (gameId: string | null, platform: Platform | null) => void;
  /** Read the saved paths for a (game, platform); a blank entry if none stored. */
  getPathEntry: (gameId: string, platform: Platform) => PathEntry;
  /** Persist the saved paths for a (game, platform). */
  setPathEntry: (gameId: string, platform: Platform, entry: PathEntry) => void;
  /** ONB-003: move (or add) a project folder to the top of the recent list. */
  rememberProject: (dir: string) => void;
  /** ONB-003: drop a project folder from the recent list (it was moved or deleted). */
  forgetProject: (dir: string) => void;
};

const Ctx = createContext<SettingsCtx | null>(null);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<StudioSettings>(load);

  // Apply a functional update and mirror the result to localStorage in one step.
  const commit = useCallback((fn: (s: StudioSettings) => StudioSettings) => {
    setSettings((s) => {
      const next = fn(s);
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      } catch {
        // localStorage unavailable (private mode / quota) — keep the in-memory value; nothing else to do.
      }
      return next;
    });
  }, []);

  const setDefaultTarget = useCallback(
    (gameId: string | null, platform: Platform | null) =>
      commit((s) => ({ ...s, defaultGameId: gameId, defaultPlatform: platform })),
    [commit],
  );

  const setProjectsRoot = useCallback(
    (projectsRoot: string) => commit((s) => ({ ...s, projectsRoot })),
    [commit],
  );
  const setAutosaveMinutes = useCallback((minutes: number) => commit(s => ({ ...s, autosaveMinutes: validAutosaveMinutes(minutes) })), [commit]);

  const setPathEntry = useCallback(
    (gameId: string, platform: Platform, entry: PathEntry) =>
      commit((s) => ({ ...s, paths: { ...s.paths, [pathKey(gameId, platform)]: entry } })),
    [commit],
  );

  const rememberProject = useCallback(
    (dir: string) =>
      commit((s) => ({ ...s, recentProjects: [dir, ...s.recentProjects.filter((p) => p.toLowerCase() !== dir.toLowerCase())].slice(0, RECENT_LIMIT) })),
    [commit],
  );
  const forgetProject = useCallback(
    (dir: string) => commit((s) => ({ ...s, recentProjects: s.recentProjects.filter((p) => p.toLowerCase() !== dir.toLowerCase()) })),
    [commit],
  );

  const getPathEntry = useCallback(
    (gameId: string, platform: Platform): PathEntry =>
      settings.paths[pathKey(gameId, platform)] ?? BLANK_ENTRY,
    [settings.paths],
  );

  const value = useMemo<SettingsCtx>(
    () => ({ settings, setAutosaveMinutes, setProjectsRoot, setDefaultTarget, getPathEntry, setPathEntry, rememberProject, forgetProject }),
    [settings, setAutosaveMinutes, setProjectsRoot, setDefaultTarget, getPathEntry, setPathEntry, rememberProject, forgetProject],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useSettings(): SettingsCtx {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useSettings must be used within a SettingsProvider");
  return ctx;
}
