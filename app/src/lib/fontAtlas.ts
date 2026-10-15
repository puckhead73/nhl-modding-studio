// Jersey font-atlas glyph mapping — locates each character's cell in a `font_*_cm`
// texture so the nameplate preview can render a player's name/number "through the
// font map" (sampling the real glyphs the game uses), not a substitute font.
//
// Verified layout (NHL Legacy `font_*_cm`, 512², glyphs on a transparent bg):
//   • two short rows of letters — row 1 A–O (15), row 2 P–Z (11), + offset glyphs
//     on the right that we ignore for names;
//   • two TALL rows of big digits — 0–4 then 5–9 (left ~80%), with the captain
//     `C` / alternate `A` offset glyphs on the right;
//   • two rows of small digits 0–9 — identical; the game uses the first, so we
//     skip the duplicate second row.
// Glyphs are found as connected (alpha-inked) components, grouped into rows, and
// classified by row height (letters short, small digits medium, big digits tall),
// then mapped in reading order. Counts are asserted, so a mis-decode yields null
// rather than garbled text.

export interface GlyphRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface FontMap {
  /** Uppercase A–Z → cell. */
  letters: Map<string, GlyphRect>;
  /** '0'–'9' → the BIG number cell (back/front number). */
  bigDigits: Map<string, GlyphRect>;
  /** '0'–'9' → the small number cell (sleeve/secondary), or empty if absent. */
  smallDigits: Map<string, GlyphRect>;
  /** The large captain-patch "C" — the upper offset glyph on the atlas's right. */
  captainC?: GlyphRect;
  /** The large alternate-patch "A" — the lower offset glyph on the right. */
  captainA?: GlyphRect;
}

const ALPHA_INK = 96; // a pixel is glyph ink when its alpha exceeds this
const MIN_AREA = 20;
const MIN_W = 4;
const MIN_H = 6;

/** Connected-component glyph boxes (4-neighbour flood fill over alpha-inked pixels). */
export function detectGlyphBoxes(
  rgba: Uint8Array | Uint8ClampedArray,
  width: number,
  height: number,
): GlyphRect[] {
  const ink = (x: number, y: number) => rgba[(y * width + x) * 4 + 3] > ALPHA_INK;
  const visited = new Uint8Array(width * height);
  const boxes: GlyphRect[] = [];
  const stack: number[] = [];

  for (let sy = 0; sy < height; sy++) {
    for (let sx = 0; sx < width; sx++) {
      const s = sy * width + sx;
      if (visited[s] || !ink(sx, sy)) continue;
      let minX = sx;
      let minY = sy;
      let maxX = sx;
      let maxY = sy;
      let area = 0;
      stack.length = 0;
      stack.push(s);
      visited[s] = 1;
      while (stack.length) {
        const idx = stack.pop()!;
        const x = idx % width;
        const y = (idx - x) / width;
        area++;
        if (x < minX) minX = x;
        if (y < minY) minY = y;
        if (x > maxX) maxX = x;
        if (y > maxY) maxY = y;
        if (x > 0 && !visited[idx - 1] && ink(x - 1, y)) {
          visited[idx - 1] = 1;
          stack.push(idx - 1);
        }
        if (x < width - 1 && !visited[idx + 1] && ink(x + 1, y)) {
          visited[idx + 1] = 1;
          stack.push(idx + 1);
        }
        if (y > 0 && !visited[idx - width] && ink(x, y - 1)) {
          visited[idx - width] = 1;
          stack.push(idx - width);
        }
        if (y < height - 1 && !visited[idx + width] && ink(x, y + 1)) {
          visited[idx + width] = 1;
          stack.push(idx + width);
        }
      }
      const bw = maxX - minX + 1;
      const bh = maxY - minY + 1;
      if (area >= MIN_AREA && bw >= MIN_W && bh >= MIN_H) {
        boxes.push({ x: minX, y: minY, w: bw, h: bh });
      }
    }
  }
  return boxes;
}

/** Group glyph boxes into rows (a box starts a new row when its top clears the
 *  first box of the current row by >60% of that box's height), each row sorted
 *  left-to-right, rows sorted top-to-bottom. */
export function groupRows(boxes: GlyphRect[]): GlyphRect[][] {
  const sorted = [...boxes].sort((a, b) => a.y - b.y || a.x - b.x);
  const rows: GlyphRect[][] = [];
  for (const b of sorted) {
    const row = rows[rows.length - 1];
    if (row && b.y < row[0].y + (row[0].h * 3) / 5) {
      row.push(b);
    } else {
      rows.push([b]);
    }
  }
  for (const r of rows) r.sort((a, b) => a.x - b.x);
  return rows;
}

/** Median height of a row's glyphs. */
function rowHeight(row: GlyphRect[]): number {
  const hs = row.map((b) => b.h).sort((a, b) => a - b);
  return hs[Math.floor(hs.length / 2)] ?? 0;
}

/**
 * Build the character→cell map from a decoded font atlas (RGBA8, `flipY=false`
 * texel order). Returns null when the glyph counts don't match the known layout
 * (so callers fall back rather than render garbage).
 */
export function buildFontMap(
  rgba: Uint8Array | Uint8ClampedArray,
  width: number,
  height: number,
): FontMap | null {
  const boxes = detectGlyphBoxes(rgba, width, height);
  const rows = groupRows(boxes);
  if (rows.length === 0) return null;

  // Classify rows by median height relative to the TALLEST row (the big digits):
  // letters ≈0.36×, small digits ≈0.61×, big digits 1.0×, which the 0.5 / 0.72
  // splits separate cleanly. Anchoring on the max (not the min) is robust to a
  // stray short/clipped row.
  const maxH = Math.max(...rows.map(rowHeight));
  const isLetterRow = (r: GlyphRect[]) => rowHeight(r) < maxH * 0.5;
  const isBigDigitRow = (r: GlyphRect[]) => rowHeight(r) >= maxH * 0.72 && r.length >= 5;
  const isSmallDigitRow = (r: GlyphRect[]) =>
    rowHeight(r) >= maxH * 0.5 && rowHeight(r) < maxH * 0.72 && r.length >= 10;

  // Letters A–Z: the short rows, top-to-bottom, left-to-right, first 26.
  const letterCells = rows
    .filter(isLetterRow)
    .sort((a, b) => a[0].y - b[0].y)
    // Punctuation shares the alphabet rows (Legacy places an apostrophe
    // between L and M). It must not consume an A–Z slot.
    .map((row) => row.filter((cell) => cell.h >= rowHeight(row) * 0.65))
    .flat()
    .slice(0, 26);
  if (letterCells.length < 26) return null;
  const letters = new Map<string, GlyphRect>();
  for (let i = 0; i < 26; i++) letters.set(String.fromCharCode(65 + i), letterCells[i]);

  // Big digits 0–9: the two tall (≥5-glyph) rows; leftmost 5 of each (drops the
  // offset C/A on the right), top row 0–4, bottom row 5–9.
  const bigRows = rows.filter(isBigDigitRow).sort((a, b) => a[0].y - b[0].y);
  if (bigRows.length < 2) return null;
  const bigCells = [...bigRows[0].slice(0, 5), ...bigRows[1].slice(0, 5)];
  const bigDigits = new Map<string, GlyphRect>();
  for (let i = 0; i < 10; i++) bigDigits.set(String(i), bigCells[i]);

  // Small digits 0–9: the first medium (≥10-glyph) row; the duplicate below is
  // skipped. Optional — absent on some fonts.
  const smallRow = rows.filter(isSmallDigitRow).sort((a, b) => a[0].y - b[0].y)[0];
  const smallDigits = new Map<string, GlyphRect>();
  if (smallRow && smallRow.length >= 10) {
    for (let i = 0; i < 10; i++) smallDigits.set(String(i), smallRow[i]);
  }

  // Captain-patch C / alternate-patch A: the two large glyphs offset on the
  // right side of the atlas (beyond the left-aligned big-digit block), tall like
  // the big digits. Upper = C, lower = A.
  const maxBoxH = Math.max(...boxes.map((b) => b.h), 1);
  const offsets = boxes
    .filter((b) => b.h >= maxBoxH * 0.7 && b.x >= width * 0.72)
    .sort((a, b) => a.y - b.y);
  const captainC = offsets[0];
  const captainA = offsets[1];

  return { letters, bigDigits, smallDigits, captainC, captainA };
}
