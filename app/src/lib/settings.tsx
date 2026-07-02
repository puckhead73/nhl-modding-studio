// Studio-wide UI preferences, persisted to localStorage (these are user preferences, not game data —
// no backend/profile involvement). Two things live here:
//   • the default startup target — the game + platform onboarding jumps straight into, and
//   • saved paths per (game, platform) — the install root and, where relevant, the save-file folder,
//     so a returning user doesn't re-browse every time.
// Consumed by OpenProject (onboarding pre-fill) and the Settings page (the editor for all of this).

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import type { Platform } from "./client";

/** One saved location for a (game, platform): the install root and the save-file folder (blank when
 *  the platform finds saves automatically). */
export type PathEntry = { root: string; savePath: string };

export type StudioSettings = {
  /** Game onboarding loads into on startup, or null for the game picker. */
  defaultGameId: string | null;
  /** Platform paired with `defaultGameId`, or null. */
  defaultPlatform: Platform | null;
  /** Saved paths keyed by `${gameId}:${platform}`. */
  paths: Record<string, PathEntry>;
};

const STORAGE_KEY = "nhl-modding-studio.settings.v1";
const EMPTY: StudioSettings = { defaultGameId: null, defaultPlatform: null, paths: {} };
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
      defaultGameId: parsed.defaultGameId ?? null,
      defaultPlatform: parsed.defaultPlatform ?? null,
      paths: parsed.paths ?? {},
    };
  } catch {
    return EMPTY;
  }
}

type SettingsCtx = {
  settings: StudioSettings;
  /** Set (or clear, with nulls) the game+platform onboarding starts in. */
  setDefaultTarget: (gameId: string | null, platform: Platform | null) => void;
  /** Read the saved paths for a (game, platform); a blank entry if none stored. */
  getPathEntry: (gameId: string, platform: Platform) => PathEntry;
  /** Persist the saved paths for a (game, platform). */
  setPathEntry: (gameId: string, platform: Platform, entry: PathEntry) => void;
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

  const setPathEntry = useCallback(
    (gameId: string, platform: Platform, entry: PathEntry) =>
      commit((s) => ({ ...s, paths: { ...s.paths, [pathKey(gameId, platform)]: entry } })),
    [commit],
  );

  const getPathEntry = useCallback(
    (gameId: string, platform: Platform): PathEntry =>
      settings.paths[pathKey(gameId, platform)] ?? BLANK_ENTRY,
    [settings.paths],
  );

  const value = useMemo<SettingsCtx>(
    () => ({ settings, setDefaultTarget, getPathEntry, setPathEntry }),
    [settings, setDefaultTarget, getPathEntry, setPathEntry],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useSettings(): SettingsCtx {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useSettings must be used within a SettingsProvider");
  return ctx;
}
