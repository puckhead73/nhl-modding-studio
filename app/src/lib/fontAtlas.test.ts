import { describe, it, expect } from "vitest";
import { detectGlyphBoxes, groupRows, buildFontMap } from "./fontAtlas";

// Paint a synthetic atlas matching the verified layout: two short letter rows
// (15 + 11), two tall big-digit rows (5 + 5) with an extra offset glyph on the
// right, one small-digit row (10), plus a duplicate small-digit row below.
function synthAtlas(): { rgba: Uint8Array; width: number; height: number } {
  const width = 420;
  const height = 460;
  const rgba = new Uint8Array(width * height * 4); // alpha 0 background
  const paint = (x: number, y: number, w: number, h: number) => {
    for (let yy = y; yy < y + h; yy++) {
      for (let xx = x; xx < x + w; xx++) {
        const i = (yy * width + xx) * 4;
        rgba[i] = 200;
        rgba[i + 1] = 200;
        rgba[i + 2] = 200;
        rgba[i + 3] = 255;
      }
    }
  };
  const row = (y: number, h: number, count: number, w: number, gap: number, x0 = 5) => {
    for (let i = 0; i < count; i++) paint(x0 + i * (w + gap), y, w, h);
  };
  row(10, 30, 15, 15, 5); // letters A–O
  row(50, 30, 11, 15, 5); // letters P–Z
  row(100, 90, 5, 50, 10); // big 0–4
  paint(360, 100, 40, 90); // offset C on the right — large (captain patch)
  row(210, 90, 5, 50, 10); // big 5–9
  paint(360, 230, 40, 80); // offset A on the right — large (alternate patch)
  row(320, 55, 10, 25, 5); // small 0–9
  row(320 + 55 + 5, 55, 10, 25, 5); // duplicate small 0–9 (skipped)
  return { rgba, width, height };
}

describe("fontAtlas", () => {
  it("ignores short punctuation embedded in the alphabet rows", () => {
    const { rgba, width, height } = synthAtlas();
    // Insert an apostrophe into the gap between L and M.
    for (let y = 10; y < 18; y++) for (let x = 240; x < 244; x++) rgba[(y * width + x) * 4 + 3] = 255;
    const map = buildFontMap(rgba, width, height)!;
    expect(map.letters.get("M")).toMatchObject({ x: 245, y: 10 });
    expect(map.letters.get("T")).toMatchObject({ x: 85, y: 50 });
    expect(map.letters.get("Y")).toMatchObject({ x: 185, y: 50 });
    expect(map.letters.get("Z")).toMatchObject({ x: 205, y: 50 });
  });
  it("detects each painted glyph as its own box", () => {
    const { rgba, width, height } = synthAtlas();
    const boxes = detectGlyphBoxes(rgba, width, height);
    // 15 + 11 + 5 + 1 + 5 + 1 + 10 + 10 = 58
    expect(boxes.length).toBe(58);
  });

  it("groups boxes into the expected rows", () => {
    const { rgba, width, height } = synthAtlas();
    const rows = groupRows(detectGlyphBoxes(rgba, width, height));
    const counts = rows.map((r) => r.length);
    // A–O, P–Z, (0–4 + offset C), (5–9 + offset A), small, small-dup
    expect(counts).toEqual([15, 11, 6, 6, 10, 10]);
  });

  it("maps A–Z, big 0–9 and small 0–9, ignoring the offset + duplicate glyphs", () => {
    const { rgba, width, height } = synthAtlas();
    const map = buildFontMap(rgba, width, height)!;
    expect(map).not.toBeNull();
    // All 26 letters present, in reading order (A leftmost of the top row).
    expect(map.letters.size).toBe(26);
    expect(map.letters.get("A")!.x).toBeLessThan(map.letters.get("O")!.x);
    expect(map.letters.get("A")!.y).toBeLessThan(map.letters.get("P")!.y);
    // Big digits: 0 leftmost of the top tall row, 5 on the second tall row; the
    // offset C at x=360 must NOT be picked as a digit.
    expect(map.bigDigits.size).toBe(10);
    expect(map.bigDigits.get("0")!.y).toBeLessThan(map.bigDigits.get("5")!.y);
    for (const d of ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]) {
      expect(map.bigDigits.get(d)!.x).toBeLessThan(360);
    }
    // Small digits: exactly the first small row.
    expect(map.smallDigits.size).toBe(10);
    expect(map.smallDigits.get("0")!.h).toBeLessThan(map.bigDigits.get("0")!.h);
    // Captain C (upper) / alternate A (lower): the two large offset glyphs on the
    // right, not confused with the left-aligned big digits.
    expect(map.captainC).toBeTruthy();
    expect(map.captainA).toBeTruthy();
    expect(map.captainC!.y).toBeLessThan(map.captainA!.y);
    expect(map.captainC!.x).toBeGreaterThan(300); // right side
    expect(map.bigDigits.get("0")!.x).toBeLessThan(300); // digits stay left
  });

  it("returns null when the letter count is short (mis-decode guard)", () => {
    const width = 200;
    const height = 100;
    const rgba = new Uint8Array(width * height * 4);
    // Only a few glyphs — not a real atlas.
    for (let i = 0; i < 5; i++) {
      for (let y = 10; y < 40; y++)
        for (let x = 5 + i * 20; x < 5 + i * 20 + 15; x++) {
          const j = (y * width + x) * 4 + 3;
          rgba[j] = 255;
        }
    }
    expect(buildFontMap(rgba, width, height)).toBeNull();
  });
});
