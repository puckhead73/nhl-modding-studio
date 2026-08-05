// Phase 1 presentation-capability contract (frontend side). Pure helpers over the backend-probed
// `SurfaceCapabilitiesDto` so the UI gates tabs and queries on evidence, never on the title id. The
// tab-selection logic here is deliberately side-effect-free so it can be unit-tested without a
// browser (see app/src/lib/surfaces.test.ts).

import type { SurfaceCapabilitiesDto } from "../bindings/SurfaceCapabilitiesDto";
import type { SurfaceCapabilityDto } from "../bindings/SurfaceCapabilityDto";

/** The verdict states, mirroring `studio_domain::SurfaceState` (kebab-serialized). */
export type SurfaceState = "not-applicable" | "readable" | "editable" | "disabled" | "unverified";

export type SurfaceName = keyof SurfaceCapabilitiesDto;

/** A surface renders a tab at all unless its schema doesn't carry it. */
export function isAvailable(cap: SurfaceCapabilityDto | undefined): boolean {
  return !!cap && cap.state !== "not-applicable";
}
/** Present but not writable yet (asset authoring) — shown with a reason, no editing. */
export function isDisabled(cap: SurfaceCapabilityDto | undefined): boolean {
  return cap?.state === "disabled";
}
/** Present but semantics unconfirmed — shown read-only with caution. */
export function isUnverified(cap: SurfaceCapabilityDto | undefined): boolean {
  return cap?.state === "unverified";
}

/** How one tab maps to a backing surface. `surface: null` = always shown (roster-derived views that
 *  need no optional table, e.g. Depth Chart / Contracts, both driven by the player list). */
export interface TabGate {
  key: string;
  surface: SurfaceName | null;
}

/** The verdict for a single tab after applying the capability contract. */
export interface TabVisibility {
  key: string;
  /** Render the tab at all (false ⇒ omit — the schema lacks the feature). */
  show: boolean;
  /** Show the tab but disable its editing/content with a reason (asset placeholders). */
  disabled: boolean;
  /** Show the tab read-only with a caution note. */
  unverified: boolean;
  /** The backend's human explanation for a disabled/unverified/absent verdict. */
  reason: string | null;
}

/** Player-detail tabs, in display order, with their backing surface. */
export const PLAYER_TABS: readonly TabGate[] = [
  { key: "player-data", surface: "players" },
  { key: "career", surface: "career" },
  { key: "attributes", surface: "attributes" },
  { key: "equipment", surface: "player_equipment" },
];

/** Team-detail tabs, in display order, with their backing surface. Depth Chart and Contracts are
 *  roster-list-driven (no optional table required) → always shown. */
export const TEAM_TABS: readonly TabGate[] = [
  { key: "team-data", surface: "teams" },
  { key: "depth-chart", surface: null },
  { key: "lines", surface: "lines" },
  { key: "strategies", surface: "strategies" },
  { key: "jerseys-captaincy", surface: "captaincy" },
  { key: "contracts", surface: "contracts" },
  { key: "draft", surface: "draft_picks" },
  { key: "jerseys", surface: "jerseys" },
  { key: "equipment", surface: "team_equipment" },
];

/** Resolve each tab's visibility against the probed surfaces. A `null` surface is always shown; an
 *  absent surface (`not-applicable`) is omitted; a `disabled`/`unverified` surface is shown with its
 *  reason. When `surfaces` is undefined (older summary / not yet loaded) every tab shows, so the UI
 *  degrades to the pre-capability behavior rather than hiding everything. */
export function resolveTabs(
  gates: readonly TabGate[],
  surfaces: SurfaceCapabilitiesDto | undefined,
): TabVisibility[] {
  return gates.map((g) => {
    if (g.surface === null || !surfaces) {
      return { key: g.key, show: true, disabled: false, unverified: false, reason: null };
    }
    const cap = surfaces[g.surface];
    return {
      key: g.key,
      show: isAvailable(cap),
      disabled: isDisabled(cap),
      unverified: isUnverified(cap),
      reason: cap?.reason ?? null,
    };
  });
}

/** The set of tab keys that should render (convenience over {@link resolveTabs}). */
export function visibleTabKeys(
  gates: readonly TabGate[],
  surfaces: SurfaceCapabilitiesDto | undefined,
): Set<string> {
  return new Set(resolveTabs(gates, surfaces).filter((t) => t.show).map((t) => t.key));
}
