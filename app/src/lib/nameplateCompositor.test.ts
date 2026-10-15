import { describe, it, expect } from "vitest";
import { layoutText, classifyZones, type GlyphPlacement } from "./nameplateCompositor";
import type { GlyphRect } from "./fontAtlas";

// A tiny fake atlas: each letter a 10×20 cell, resolved by code point.
const cell = (ch: string): GlyphRect | null =>
  /[A-Z]/.test(ch) ? { x: (ch.charCodeAt(0) - 65) * 10, y: 0, w: 10, h: 20 } : null;

const boundsOf = (p: GlyphPlacement[]) => {
  const xs = p.map((g) => g.dst.x);
  const rights = p.map((g) => g.dst.x + g.dst.w);
  return { left: Math.min(...xs), right: Math.max(...rights) };
};

describe("layoutText", () => {
  it("centers a run within the zone and fills its height", () => {
    const zone = { x: 100, y: 200, w: 300, h: 40 };
    const p = layoutText("AB", cell, zone, { fill: 0.9, tracking: 0.1 });
    expect(p.length).toBe(2);
    // Height fills 0.9 × zone.h = 36 (cells are 20 tall → scale 1.8).
    expect(p[0].dst.h).toBeCloseTo(36, 1);
    // Vertically centered on the zone mid-line.
    expect(p[0].dst.y + p[0].dst.h / 2).toBeCloseTo(zone.y + zone.h / 2, 1);
    // Horizontally centered: equal margins either side.
    const b = boundsOf(p);
    expect(b.left - zone.x).toBeCloseTo(zone.x + zone.w - b.right, 0);
    // Reading order preserved.
    expect(p[0].dst.x).toBeLessThan(p[1].dst.x);
  });

  it("shrinks a long run to fit the zone width", () => {
    const zone = { x: 0, y: 0, w: 60, h: 40 };
    const p = layoutText("ABCDEFGH", cell, zone, { fill: 1 });
    const b = boundsOf(p);
    expect(b.right - b.left).toBeLessThanOrEqual(zone.w + 0.5);
    expect(b.left).toBeGreaterThanOrEqual(-0.5);
  });

  it("treats spaces/unknowns as gaps and ignores empty input", () => {
    const zone = { x: 0, y: 0, w: 300, h: 40 };
    expect(layoutText("   ", cell, zone).length).toBe(0);
    const p = layoutText("A B", cell, zone);
    expect(p.length).toBe(2); // two letters, the space is a gap
    expect(p[1].dst.x - (p[0].dst.x + p[0].dst.w)).toBeGreaterThan(0);
  });
});

describe("classifyZones", () => {
  it("orders number squares largest-first and picks the widest as the name", () => {
    const zones = [
      { id: "name", w: 200, h: 30 }, // wide → name
      { id: "back", w: 60, h: 66 }, // largest square → back number
      { id: "sleeveL", w: 22, h: 20 }, // small square → sleeve
      { id: "sleeveR", w: 20, h: 22 }, // small square → sleeve
    ];
    const { nameZoneId, numberZoneIds } = classifyZones(zones, (z) => ({ w: z.w, h: z.h }));
    expect(nameZoneId).toBe("name");
    expect(numberZoneIds[0]).toBe("back"); // largest first
    expect(numberZoneIds).toContain("sleeveL");
    expect(numberZoneIds).toContain("sleeveR");
    expect(numberZoneIds).not.toContain("name");
  });

  it("is safe with no zones", () => {
    expect(classifyZones([], () => ({ w: 0, h: 0 }))).toEqual({ numberZoneIds: [] });
  });
});
