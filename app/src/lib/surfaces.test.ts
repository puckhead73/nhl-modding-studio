import { describe, expect, it } from "vitest";
import type { SurfaceCapabilitiesDto } from "../bindings/SurfaceCapabilitiesDto";
import type { SurfaceCapabilityDto } from "../bindings/SurfaceCapabilityDto";
import {
  PLAYER_TABS,
  TEAM_TABS,
  resolveTabs,
  visibleTabKeys,
  isAvailable,
} from "./surfaces";

const cap = (state: SurfaceCapabilityDto["state"], reason: string | null = null): SurfaceCapabilityDto => ({
  state,
  reason,
});

/** A full surface set with every entry editable, overridable per test. */
function surfaces(overrides: Partial<SurfaceCapabilitiesDto> = {}): SurfaceCapabilitiesDto {
  const base: SurfaceCapabilitiesDto = {
    players: cap("editable"),
    teams: cap("editable"),
    arenas: cap("readable"),
    career: cap("readable"),
    attributes: cap("editable"),
    traits: cap("editable"),
    lines: cap("editable"),
    strategies: cap("editable"),
    captaincy: cap("editable"),
    contracts: cap("editable"),
    draft_picks: cap("editable"),
    jerseys: cap("editable"),
    team_equipment: cap("editable"),
    player_equipment: cap("disabled", "no editor (Phase 6)"),
    portraits: cap("disabled", "preview only"),
    logos: cap("disabled", "preview only"),
    heads: cap("disabled", "preview only"),
    audio: cap("disabled", "preview only"),
  };
  return { ...base, ...overrides };
}

describe("isAvailable", () => {
  it("treats not-applicable as unavailable and everything else as available", () => {
    expect(isAvailable(cap("not-applicable"))).toBe(false);
    expect(isAvailable(cap("readable"))).toBe(true);
    expect(isAvailable(cap("editable"))).toBe(true);
    expect(isAvailable(cap("disabled"))).toBe(true);
    expect(isAvailable(cap("unverified"))).toBe(true);
    expect(isAvailable(undefined)).toBe(false);
  });
});

describe("player tabs", () => {
  it("shows all four when every surface is present", () => {
    const keys = visibleTabKeys(PLAYER_TABS, surfaces());
    expect([...keys].sort()).toEqual(["attributes", "career", "player-data", "portrait"]);
  });

  it("omits the Career tab when the file has no career tables (roster save / gen shape)", () => {
    const keys = visibleTabKeys(PLAYER_TABS, surfaces({ career: cap("not-applicable", "no career tables") }));
    expect(keys.has("career")).toBe(false);
    expect(keys.has("player-data")).toBe(true);
  });

  it("keeps the Portrait tab but marks it disabled with the backend reason", () => {
    const tab = resolveTabs(PLAYER_TABS, surfaces()).find((t) => t.key === "portrait")!;
    expect(tab.show).toBe(true);
    expect(tab.disabled).toBe(true);
    expect(tab.reason).toBe("preview only");
  });
});

describe("team tabs — the gen6/mid-era shapes", () => {
  it("NHL07-like: omits arenas-less surfaces (draft, jerseys, equipment) but keeps roster tabs", () => {
    const s = surfaces({
      draft_picks: cap("not-applicable", "no qHav"),
      jerseys: cap("not-applicable", "no xjgw"),
      team_equipment: cap("not-applicable", "no hHbv"),
    });
    const keys = visibleTabKeys(TEAM_TABS, s);
    expect(keys.has("draft")).toBe(false);
    expect(keys.has("jerseys")).toBe(false);
    expect(keys.has("equipment")).toBe(false);
    // Roster-driven + present surfaces remain.
    expect(keys.has("team-data")).toBe(true);
    expect(keys.has("depth-chart")).toBe(true);
    expect(keys.has("contracts")).toBe(true);
    expect(keys.has("lines")).toBe(true);
    expect(keys.has("strategies")).toBe(true);
  });

  it("omits Lines/Strategies/Jerseys&Captaincy when their tables are absent", () => {
    const s = surfaces({
      lines: cap("not-applicable"),
      strategies: cap("not-applicable"),
      captaincy: cap("not-applicable"),
    });
    const keys = visibleTabKeys(TEAM_TABS, s);
    expect(keys.has("lines")).toBe(false);
    expect(keys.has("strategies")).toBe(false);
    expect(keys.has("jerseys-captaincy")).toBe(false);
  });

  it("shows every team tab on a full-schema title", () => {
    const keys = visibleTabKeys(TEAM_TABS, surfaces());
    expect(keys.size).toBe(TEAM_TABS.length);
  });
});

describe("degradation", () => {
  it("shows all tabs when surfaces are undefined (pre-capability fallback, never hides everything)", () => {
    expect(visibleTabKeys(PLAYER_TABS, undefined).size).toBe(PLAYER_TABS.length);
    expect(visibleTabKeys(TEAM_TABS, undefined).size).toBe(TEAM_TABS.length);
  });

  it("always shows surface:null tabs (depth-chart) regardless of capability", () => {
    const tabs = resolveTabs(TEAM_TABS, surfaces());
    expect(tabs.find((t) => t.key === "depth-chart")!.show).toBe(true);
  });
});
