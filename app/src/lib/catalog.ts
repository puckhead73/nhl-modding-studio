// The ingested title/platform catalog — the single source of truth for which NHL games exist, which
// consoles each runs on, and the UI art for both. Shared by onboarding (OpenProject) and Settings so
// the two never drift.

import type { Platform } from "./client";

// Resolve every UI image once (game box-art, console icons) by filename.
const UI_IMAGES = import.meta.glob("../assets/ui/*.png", {
  eager: true,
  query: "?url",
  import: "default",
}) as Record<string, string>;

/** Resolve a bundled UI image by filename (e.g. "nhl14.png"); "" if missing. */
export function img(file: string): string {
  const hit = Object.entries(UI_IMAGES).find(([p]) => p.endsWith("/" + file));
  return hit ? hit[1] : "";
}

export type GameDef = {
  id: string;
  label: string;
  file: string;
  platforms: Platform[];
};

const PS3_XBOX: Platform[] = ["ps3", "xbox"];
const PS3_XBOX_PS4: Platform[] = ["ps3", "xbox", "ps4"];

// Every ingested title, in release order. Multi-system titles prompt for the console after selection;
// NHL 15 spans PS3/Xbox/PS4 (the bridge title); NHL 16–21 are PS4; Legacy Recompiled is PC (recomp).
export const GAMES: GameDef[] = [
  { id: "nhl10", label: "NHL 10", file: "nhl10.png", platforms: PS3_XBOX },
  { id: "nhl11", label: "NHL 11", file: "nhl11.png", platforms: PS3_XBOX },
  { id: "nhl12", label: "NHL 12", file: "nhl12.png", platforms: PS3_XBOX },
  { id: "nhl13", label: "NHL 13", file: "nhl13.png", platforms: PS3_XBOX },
  { id: "nhl14", label: "NHL 14", file: "nhl14.png", platforms: PS3_XBOX },
  { id: "nhl15", label: "NHL 15", file: "nhl15.png", platforms: PS3_XBOX_PS4 },
  { id: "nhl16", label: "NHL 16", file: "nhl16.png", platforms: ["ps4"] },
  { id: "nhllegacy", label: "NHL Legacy", file: "nhlleg.png", platforms: PS3_XBOX },
  { id: "legacyrecomp", label: "NHL Legacy Recompiled", file: "legacyrecomp.png", platforms: ["recomp"] },
  { id: "nhl17", label: "NHL 17", file: "nhl17.png", platforms: ["ps4"] },
  { id: "nhl18", label: "NHL 18", file: "nhl18.png", platforms: ["ps4"] },
  { id: "nhl19", label: "NHL 19", file: "nhl19.png", platforms: ["ps4"] },
  { id: "nhl20", label: "NHL 20", file: "nhl20.png", platforms: ["ps4"] },
  { id: "nhl21", label: "NHL 21", file: "nhl21.png", platforms: ["ps4"] },
];

export function gameById(id: string | null | undefined): GameDef | null {
  return GAMES.find((g) => g.id === id) ?? null;
}

// The consoles we target, in display order.
export const PLATFORM_ORDER: Platform[] = ["ps3", "xbox", "ps4", "recomp"];

export const CONSOLE_LABEL: Record<Platform, string> = {
  ps3: "PS3",
  xbox: "Xbox 360",
  ps4: "PS4",
  recomp: "NHL Legacy Recompiled",
};

// Console icon by platform (Recomp has no icon yet — callers fall back to a text card).
export const CONSOLE_ICON: Partial<Record<Platform, string>> = {
  ps3: "PS3.png",
  xbox: "360.png",
  ps4: "ps4.png",
};

// Platforms whose emulator/runtime saves aren't in a consistent location — the user points us at the
// save folder directly. PS3 (RPCS3) and PS4 (shadPS4) save under the emulator root, so we find those
// automatically and don't ask.
export const SAVE_PATH_PLATFORMS: Platform[] = ["xbox", "recomp"];

export function needsSavePath(p: Platform | null): boolean {
  return p != null && SAVE_PATH_PLATFORMS.includes(p);
}

/** Games available on a given platform, in catalog order. */
export function gamesForPlatform(p: Platform): GameDef[] {
  return GAMES.filter((g) => g.platforms.includes(p));
}
