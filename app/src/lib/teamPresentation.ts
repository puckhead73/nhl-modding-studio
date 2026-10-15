// Team-subtab presentation vocabularies — labels for the Jerseys / Equipment / Strategies / Lines
// tabs, ported from the reference app's operator-anchored research (Heck DB Editor vocabularies for
// jerseys/equipment; the in-game strategy UI for the strategy sliders; the NHL roster screen's
// 3-letter style codes). Pure display maps: everything resolves raw values the backend already
// verified; unknown raws fall through to a visible `#N` form rather than being hidden.

import type { TeamJerseyDto } from "../bindings/TeamJerseyDto";
import type { TeamEquipmentDto } from "../bindings/TeamEquipmentDto";

// ---------------------------------------------------------------------------
// Jerseys (`xjgw`)
// ---------------------------------------------------------------------------

export const JERSEY_STYLE_LABELS: Record<number, string> = {
  0: "90s / CCM / Retro",
  1: "Reebok / Modern",
};

/** `Ughr` — "Is Light Color": Home (dark) vs Away (light). */
export const LIGHT_COLOR_LABELS: Record<number, string> = {
  0: "Home (dark)",
  1: "Away (light)",
};

export const LOC_STRING_LABELS: Record<number, string> = {
  0: "Home",
  1: "Away",
  2: "Alt",
  3: "2006-07 Home",
  4: "2006-07 Away",
  5: "2006-07 2nd Home",
  6: "Vintage (auto key)",
};

export const jerseyStyleLabel = (raw: number) => JERSEY_STYLE_LABELS[raw] ?? `Style #${raw}`;
export const lightColorLabel = (raw: number) => LIGHT_COLOR_LABELS[raw] ?? `Light #${raw}`;
export const locStringLabel = (raw: number) => LOC_STRING_LABELS[raw] ?? String(raw);

/** `{style}_{art_id}_{variant}` — the asset id that keys the jersey's texture files. */
export const jerseyAssetId = (r: TeamJerseyDto) =>
  `${r.style_raw}_${r.team_art_id}_${r.variant_raw}`;

/** Reference left-pane order: `(jersey_index, record)`. */
export function compareJerseys(a: TeamJerseyDto, b: TeamJerseyDto): number {
  if (a.jersey_index !== b.jersey_index) return a.jersey_index - b.jersey_index;
  return a.record - b.record;
}

// ---------------------------------------------------------------------------
// Equipment (`hHbv`)
// ---------------------------------------------------------------------------

/** `jJpM` labels with confirmed in-game meaning; other codes surface as `Equipment #N`. */
export const EQUIPMENT_TYPE_LABELS: Record<number, string> = {
  // Skater.
  5: "Gloves",
  8: "Helmet",
  // Goalie (hHbv types 1/2/3). The type→piece assignment is a FLAGGED best-guess
  // that must match GOALIE_TYPE_* in crates/studio-api/src/registry.rs — correct
  // both together once verified in-game.
  1: "Blocker",
  2: "Trapper",
  3: "Pads",
};

export const ERA_LABELS: Record<number, string> = { 0: "Reebok", 1: "90's" };

/** Per-zone names by equipment type (only Gloves/Helmet are anchored). Index [i] = zone i+1. */
export const ZONE_NAMES_BY_EQUIPMENT: Record<number, readonly string[]> = {
  5: [
    "Main Color",
    "Middle Color",
    "Middle Piping",
    "Thumb Piping",
    "Lower Cuff",
    "Wrist Main Color",
    "Wrist Logo",
    "Thumb Color",
    "Extra Piping (Thumb)",
  ],
  8: ["Main Color", "Strap Color", "Logo", "Zone 4", "Zone 5", "Zone 6", "Zone 7", "Zone 8", "Zone 9"],
};

/** Per-zone `(blue, green, red)` studio field ids — the literal BGR storage order on `hHbv`.
 *  Mirrors the backend's `EQUIPMENT_ZONE_FIELDS`; used to address `set_field` channel edits. */
export const EQUIPMENT_ZONE_FIELDS: ReadonlyArray<readonly [string, string, string]> = [
  ["IByh", "QnCG", "VaOV"],
  ["GHIQ", "OZgb", "Tssg"],
  ["HZib", "PLSw", "SCSn"],
  ["KuQC", "SwoV", "PVgG"],
  ["LIml", "TeyG", "OjCJ"],
  ["NOUA", "VQkn", "QBEw"],
  ["MgSz", "UCSg", "RLCb"],
  ["DYdG", "LmNh", "WbNC"],
  ["CiDJ", "KYzW", "Xtnl"],
];

export const equipmentTypeLabel = (raw: number) =>
  EQUIPMENT_TYPE_LABELS[raw] ?? `Equipment #${raw}`;
export const eraLabel = (raw: number) => ERA_LABELS[raw] ?? `Era #${raw}`;

export function zoneName(equipmentTypeRaw: number, zoneIndex: number): string {
  const names = ZONE_NAMES_BY_EQUIPMENT[equipmentTypeRaw];
  return names?.[zoneIndex] ?? `Zone ${zoneIndex + 1}`;
}

export function rgbToHex(red: number, green: number, blue: number): string {
  const hex = (n: number) =>
    Math.max(0, Math.min(255, Math.round(n))).toString(16).padStart(2, "0").toUpperCase();
  return `#${hex(red)}${hex(green)}${hex(blue)}`;
}

/** Black or white, whichever stays legible on the swatch (0.299R + 0.587G + 0.114B > 128). */
export function readableForeground(red: number, green: number, blue: number): string {
  return 0.299 * red + 0.587 * green + 0.114 * blue > 128 ? "#000000" : "#FFFFFF";
}

/** Reference left-pane order: `(type, era, variant, record)`. */
export function compareEquipment(a: TeamEquipmentDto, b: TeamEquipmentDto): number {
  if (a.equipment_type_raw !== b.equipment_type_raw)
    return a.equipment_type_raw - b.equipment_type_raw;
  if (a.era_raw !== b.era_raw) return a.era_raw - b.era_raw;
  if (a.variant_raw !== b.variant_raw) return a.variant_raw - b.variant_raw;
  return a.record - b.record;
}

// ---------------------------------------------------------------------------
// Strategies — diagram GIFs + slider configs
// ---------------------------------------------------------------------------

// Field-id → raw value → filename under /strategies/ (served from app/public). The gifs are the
// operator-authored rink diagrams shipped with the reference app; filenames kept verbatim.
const STRATEGY_GIFS: Record<string, Record<number, string>> = {
  RBGh: {
    0: "Forecheck- Weak Side Lock.gif",
    1: "Forecheck-  2-3.gif",
    2: "Forecheck- 1-2-2 Passive.gif",
    3: "Forecheck- 1-2-2 Aggressive.gif",
  },
  uAeC: {
    0: "Neutral Zone - 1-3-1.gif",
    1: "Neutral Zone - 1-4.gif",
    2: "Neutral Zone - 1-2-2 Red.gif",
    3: "Neutral Zone - 1-2-2 Blue.gif",
  },
  rKer: {
    0: "Defensive Strategy - Collapsing.gif",
    1: "Defensive Strategy - Staggered.gif",
    2: "Defensive Strategy - Tight Point.gif",
  },
  bame: {
    0: "Powerplay - Shooting.gif",
    1: "Powerplay - Overload.gif",
    2: "Powerplay - Umbrella.gif",
  },
  NzPy: {
    0: "Penalty Kill - Passive Box.gif",
    1: "Penalty Kill - Large Box.gif",
    2: "Penalty Kill - Diamond.gif",
  },
  TTVS: {
    0: "Offensive Pressure - Defend Lead.gif",
    1: "Offensive Pressure - Conservative.gif",
    2: "Offensive Pressure - Standard.gif",
    3: "Offensive Pressure - Aggressive.gif",
    4: "Offensive Pressure - Full Attack.gif",
  },
  xKvv: {
    0: "Defensive Pressure - Protect Net.gif",
    1: "Defensive Pressure - Contain Puck.gif",
    2: "Defensive Pressure - Normal.gif",
    3: "Defensive Pressure - Puck Side Attack.gif",
    4: "Defensive Pressure - High Pressure.gif",
  },
};
// All four line-style fields share the 3-entry vocab and gif set.
const LINE_STYLE_GIFS: Record<number, string> = {
  0: "Forward Strategy - Overload.gif",
  1: "Forward Strategy - Crash the Net.gif",
  2: "Forward Strategy - Behind the Net.gif",
};
for (const id of ["eRIF", "LIlj", "ALBZ", "CPwO"]) STRATEGY_GIFS[id] = LINE_STYLE_GIFS;

export function strategyGifUrl(fieldId: string, raw: number): string | undefined {
  const filename = STRATEGY_GIFS[fieldId]?.[raw];
  return filename ? `/strategies/${encodeURIComponent(filename)}` : undefined;
}

export const strategyFieldHasGifs = (fieldId: string) => fieldId in STRATEGY_GIFS;

/** Fields rendered as sliders with named endpoints instead of dropdowns / plain numerics.
 *  Biases store 5-bit but the in-game UI exposes 0..8; NZ Pressure is 3-bit with full 0..7. */
export const TACTIC_SLIDERS: Record<
  string,
  { min: number; max: number; lowLabel: string; highLabel: string }
> = {
  asrd: { min: 0, max: 7, lowLabel: "NZ", highLabel: "FC" },
  JWjc: { min: 0, max: 8, lowLabel: "Carry", highLabel: "Dump" },
};

export const BIAS_MIN = 0;
export const BIAS_MAX = 8;

export interface BiasAxis {
  /** Position within the per-line/per-pair 6-column bias matrix (0-based). */
  index: number;
  label: string;
  lowLabel: string;
  highLabel: string;
}

export const FORWARD_BIAS_AXES: BiasAxis[] = [
  { index: 0, label: "Carry / Dump", lowLabel: "Carry", highLabel: "Dump" },
  { index: 1, label: "Cycle / Shoot", lowLabel: "Cycle", highLabel: "Shoot" },
  { index: 2, label: "Efficiency / Energy", lowLabel: "Efficiency", highLabel: "Energy" },
  { index: 3, label: "Don't Block / Block", lowLabel: "Don't Block", highLabel: "Block" },
];

export const DEFENSE_BIAS_AXES: BiasAxis[] = [
  { index: 0, label: "Hold Line / Pinch", lowLabel: "Hold Line", highLabel: "Pinch" },
  { index: 1, label: "Cycle / Shoot", lowLabel: "Cycle", highLabel: "Shoot" },
];

// ---------------------------------------------------------------------------
// Lines — style codes + borrowed archetype vocab
// ---------------------------------------------------------------------------

/** 3-letter style codes mirroring the NHL game's roster screen. */
const STYLE_CODE_MAP: Record<string, string> = {
  "Defensive Defenseman": "DEF",
  "Offensive Defenseman": "OFF",
  "Enforcer Defenseman": "EFD",
  "Two-Way Defenseman": "TWD",
  Grinder: "GRN",
  Playmaker: "PLY",
  Sniper: "SNP",
  "Power Forward": "PWF",
  "Two-Way Forward": "TWF",
  "Enforcer Forward": "ENF",
  Standup: "STD",
  Hybrid: "HYB",
  Butterfly: "BUT",
};

export function styleCode(label: string | null | undefined): string {
  if (!label) return "---";
  return (
    STYLE_CODE_MAP[label] ??
    (label.replace(/[^A-Za-z0-9]/g, "").slice(0, 3).toUpperCase() || "---")
  );
}

// The roster-rail style labels come from the domain per title (research R122; 27 Sep 2026) — the former
// Legacy-only constants here labelled every title's roster cell with NHL Legacy names.

// ---------------------------------------------------------------------------
// Lineup status (`eGlu.hSZj`) — the real per-slot dressed/scratched flag (Spike S4).
// "In Minors" is NOT a stored status; it is a Depth-Chart identifier for players under contract to a
// team whose main team is elsewhere (see the Depth Chart tab).
// ---------------------------------------------------------------------------

/** A slot is dressed unless explicitly scratched (`dressed === false`). `null` (no slot) reads as dressed. */
export function isDressed(dressed: boolean | null | undefined): boolean {
  return dressed !== false;
}
