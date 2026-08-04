import { describe, expect, it } from "vitest";
import {
  backgroundColor,
  clampZone,
  detectZones,
  footprint,
  hitHandle,
  hitTest,
  isMarkerColor,
  resizeZone,
  type NameplateZone,
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
    // srcX/srcY seed the position; per-axis scale starts at 1.
    expect(zones[0].scaleX).toBe(1);
    expect(zones[0].scaleY).toBe(1);
    expect([zones[0].srcX, zones[0].srcY]).toEqual([2, 2]);
  });

  it("footprint scales each axis independently", () => {
    const z: NameplateZone = { id: "z", srcX: 0, srcY: 0, w: 10, h: 6, x: 0, y: 0, scaleX: 1.5, scaleY: 2 };
    expect(footprint(z)).toEqual({ w: 15, h: 12 });
  });

  it("clamps a zone's scaled footprint inside the canvas", () => {
    const z: NameplateZone = { id: "z", srcX: 0, srcY: 0, w: 10, h: 10, x: 95, y: -4, scaleX: 1, scaleY: 1 };
    const c = clampZone(z, 100, 100);
    expect(c.x).toBe(90); // 100 - 10
    expect(c.y).toBe(0);
  });

  it("resizes from a corner, anchoring the opposite corner", () => {
    // 10×10 zone at (10,10) → its SE corner is at (20,20). Drag SE to (26,30):
    // NW stays put, footprint becomes 16×20 (scaleX 1.6, scaleY 2.0).
    const z: NameplateZone = { id: "z", srcX: 10, srcY: 10, w: 10, h: 10, x: 10, y: 10, scaleX: 1, scaleY: 1 };
    const r = resizeZone(z, "se", 26, 30);
    expect([r.x, r.y]).toEqual([10, 10]);
    expect(footprint(r)).toEqual({ w: 16, h: 20 });
  });

  it("resizes from a west edge, moving x and anchoring the right edge", () => {
    const z: NameplateZone = { id: "z", srcX: 10, srcY: 10, w: 10, h: 10, x: 10, y: 10, scaleX: 1, scaleY: 1 };
    // Right edge is at x=20. Drag the W handle to x=4 → left=4, width=16, y/height unchanged.
    const r = resizeZone(z, "w", 4, 15);
    expect(r.x).toBe(4);
    expect(r.y).toBe(10);
    expect(footprint(r)).toEqual({ w: 16, h: 10 });
  });

  it("enforces a minimum footprint when a handle crosses the opposite edge", () => {
    const z: NameplateZone = { id: "z", srcX: 0, srcY: 0, w: 10, h: 10, x: 0, y: 0, scaleX: 1, scaleY: 1 };
    // Drag the E handle past the left edge → clamped to the 4px minimum, not inverted.
    const r = resizeZone(z, "e", -50, 5, 4);
    expect(footprint(r).w).toBe(4);
    expect(r.x).toBe(0);
  });

  it("hit-tests resize handles (corner beats edge) within tolerance", () => {
    const z: NameplateZone = { id: "z", srcX: 0, srcY: 0, w: 10, h: 10, x: 10, y: 10, scaleX: 1, scaleY: 1 };
    // NW corner at (10,10); SE corner at (20,20); centre of the east edge at (20,15).
    expect(hitHandle(z, 10, 10, 2)).toBe("nw");
    expect(hitHandle(z, 20, 15, 2)).toBe("e");
    expect(hitHandle(z, 15, 15, 2)).toBeNull(); // interior — no handle
  });

  it("hit-tests the topmost zone under a point", () => {
    const zones = detectZones(sheet(20, 20, [[2, 2, 4, 4], [12, 2, 4, 4]]), 20, 20);
    expect(hitTest(zones, 3, 3)?.id).toBe(zones[0].id);
    expect(hitTest(zones, 13, 3)?.id).toBe(zones[1].id);
    expect(hitTest(zones, 8, 8)).toBeNull(); // between them, on background
  });
});
