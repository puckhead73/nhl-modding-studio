import { describe, expect, it } from "vitest";
import {
  backgroundColor,
  clampZone,
  detectZones,
  footprint,
  hitTest,
  isMarkerColor,
} from "./nameplateGeometry";

/** Build a white RGBA buffer and paint filled yellow rects (the stock nameplate colours). */
function sheet(width: number, height: number, rects: Array<[number, number, number, number]>) {
  const rgba = new Uint8ClampedArray(width * height * 4).fill(255); // white, opaque
  for (const [x, y, w, h] of rects) {
    for (let py = y; py < y + h; py++) {
      for (let px = x; px < x + w; px++) {
        const i = (py * width + px) * 4;
        rgba[i] = 255;
        rgba[i + 1] = 255;
        rgba[i + 2] = 0; // yellow marker
        rgba[i + 3] = 255;
      }
    }
  }
  return rgba;
}

describe("nameplateGeometry", () => {
  it("samples the corner as background and thresholds markers", () => {
    const rgba = sheet(4, 4, []);
    expect(backgroundColor(rgba)).toEqual([255, 255, 255]);
    expect(isMarkerColor(255, 255, 0, [255, 255, 255])).toBe(true); // yellow ≠ white
    expect(isMarkerColor(250, 250, 250, [255, 255, 255])).toBe(false); // near-white
  });

  it("detects each yellow island as a zone, in reading order", () => {
    // Two squares on one row (left, right) + one below → reading order: left, right, then bottom.
    const rgba = sheet(20, 20, [
      [2, 2, 4, 4], // top-left
      [12, 2, 4, 4], // top-right
      [6, 12, 4, 4], // bottom
    ]);
    const zones = detectZones(rgba, 20, 20);
    expect(zones.length).toBe(3);
    expect(zones.map((z) => [z.x, z.y])).toEqual([
      [2, 2],
      [12, 2],
      [6, 12],
    ]);
    // Tight bbox for the first square.
    expect([zones[0].w, zones[0].h]).toEqual([4, 4]);
    // srcX/srcY seed the position; scale starts at 1.
    expect(zones[0].scale).toBe(1);
    expect([zones[0].srcX, zones[0].srcY]).toEqual([2, 2]);
  });

  it("footprint scales with the zone scale", () => {
    const z = { id: "z", srcX: 0, srcY: 0, w: 10, h: 6, x: 0, y: 0, scale: 1.5 };
    expect(footprint(z)).toEqual({ w: 15, h: 9 });
  });

  it("clamps a zone's scaled footprint inside the canvas", () => {
    const z = { id: "z", srcX: 0, srcY: 0, w: 10, h: 10, x: 95, y: -4, scale: 1 };
    const c = clampZone(z, 100, 100);
    expect(c.x).toBe(90); // 100 - 10
    expect(c.y).toBe(0);
  });

  it("hit-tests the topmost zone under a point", () => {
    const zones = detectZones(sheet(20, 20, [[2, 2, 4, 4], [12, 2, 4, 4]]), 20, 20);
    expect(hitTest(zones, 3, 3)?.id).toBe(zones[0].id);
    expect(hitTest(zones, 13, 3)?.id).toBe(zones[1].id);
    expect(hitTest(zones, 8, 8)).toBeNull(); // between them, on background
  });
});
