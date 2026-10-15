// Pure geometry + shape-detection for the jersey nameplate (name / number / patch location) editor.
// No DOM or canvas here — just math over an RGBA buffer — so it's unit-testable in isolation.
//
// A `name_*` sheet is a fully-opaque template: marker "zones" (a name-region rectangle, a curved
// nameplate strip, patch/number squares — painted yellow in the stock files) on a flat background
// (white). The game reads only WHERE the zones are — it draws the player's name over the name region
// and patches at the slots at runtime. The editor detects each zone as a connected island that
// DIFFERS from the background colour (sampled from a corner, so non-white templates still work), lets
// the operator select / nudge / resize / delete them, then bakes a fresh sheet: background filled
// flat, zones repainted at their new positions and sizes.

/** Manhattan RGB distance above which a pixel counts as a marker (vs the flat background). Anti-aliased
 *  edges sit either side of this. */
export const MARKER_THRESHOLD = 48;

/** Background colour = the top-left corner texel. Stock nameplates are white; sampling makes the
 *  editor robust to any other flat background. */
export function backgroundColor(
  rgba: Uint8Array | Uint8ClampedArray,
): [number, number, number] {
  return [rgba[0] ?? 255, rgba[1] ?? 255, rgba[2] ?? 255];
}

/** `true` when (r,g,b) is far enough from the background to be a marker. */
export function isMarkerColor(
  r: number,
  g: number,
  b: number,
  bg: [number, number, number],
): boolean {
  return Math.abs(r - bg[0]) + Math.abs(g - bg[1]) + Math.abs(b - bg[2]) > MARKER_THRESHOLD;
}

/** One detected nameplate zone the editor can select / move / resize. Zones are returned ordered
 *  top-to-bottom then left-to-right; the editor numbers them Zone 1..N in that reading order. */
export interface NameplateZone {
  id: string;
  /** Source bounding box in texture pixels (where the marker lives in the original template). The
   *  repaint stamp is cropped from here. */
  srcX: number;
  srcY: number;
  w: number;
  h: number;
  /** Current top-left position in texture pixels (where it repaints). */
  x: number;
  y: number;
  /** Independent horizontal / vertical scale applied to the stamp on repaint (1 = original size).
   *  Footprint = (w*scaleX, h*scaleY). Split per-axis so the operator can reshape a zone's rectangle
   *  freely — the auto-detected bbox is a rectangle, but curved/irregular templates (e.g. Colorado's
   *  arched name strip) need the name and number rects sized independently, not one uniform scale. */
  scaleX: number;
  scaleY: number;
  /** Optional curved text baseline. When present (and the zone renders text), glyphs are laid along
   *  this quadratic curve instead of straight across the rect — for arched / diagonal names (e.g.
   *  Colorado's bowl-shaped nameplate). Control points + thickness are NORMALIZED to the zone
   *  footprint ([0,1] × [0,1]), so the curve rides along as the zone is moved/resized. */
  curve?: ZoneCurve;
}

/** A quadratic text baseline in footprint-normalized coordinates ([0,1] within the zone footprint;
 *  values can stray slightly outside when the operator drags a handle past an edge). `p1` is the
 *  Bézier control point (the "bend"), not a point on the curve. */
export interface ZoneCurve {
  /** Left end of the baseline. */
  p0: [number, number];
  /** Bend control point. */
  p1: [number, number];
  /** Right end of the baseline. */
  p2: [number, number];
  /** Glyph height as a fraction of the footprint height. */
  thickness: number;
}

/** A resize grip on a zone's rectangle: the four corners + four edge midpoints (compass points). */
export type ResizeHandle = "nw" | "n" | "ne" | "e" | "se" | "s" | "sw" | "w";
export const RESIZE_HANDLES: ResizeHandle[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];

interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

// Band-split tuning. Two stacked rectangles of different widths (e.g. a narrow name region directly
// on a wider number region — common when they touch) form ONE connected island; we split it into
// row-bands wherever the marker row-span steps in width or centre by more than these px thresholds.
// Bands thinner than MIN_BAND_ROWS are merged into a neighbour so anti-aliased transition rows don't
// spawn slivers.
const BAND_WIDTH_STEP = 6;
const BAND_CENTER_STEP = 6;
const MIN_BAND_ROWS = 3;

/** Detect marker islands (connected non-background regions), then split each island into row-bands
 *  where its horizontal span steps (so a name region and a number region that touch separate
 *  cleanly). Returns one zone per band in reading order. Islands smaller than `minSide` on both axes
 *  are dropped as noise. */
export function detectZones(
  rgba: Uint8Array | Uint8ClampedArray,
  width: number,
  height: number,
  opts: { minSide?: number } = {},
): NameplateZone[] {
  const minSide = opts.minSide ?? 2;
  const bg = backgroundColor(rgba);
  const visited = new Uint8Array(width * height);
  const boxes: Rect[] = [];
  const stack: number[] = [];
  const marker = (i: number) => isMarkerColor(rgba[i * 4], rgba[i * 4 + 1], rgba[i * 4 + 2], bg);

  for (let start = 0; start < width * height; start++) {
    if (visited[start] || !marker(start)) continue;
    // Flood-fill the island, collecting its pixels + bounding box.
    let minX = width;
    let minY = height;
    let maxX = -1;
    let maxY = -1;
    const pixels: number[] = [];
    stack.length = 0;
    stack.push(start);
    visited[start] = 1;
    while (stack.length) {
      const idx = stack.pop()!;
      pixels.push(idx);
      const px = idx % width;
      const py = (idx - px) / width;
      if (px < minX) minX = px;
      if (py < minY) minY = py;
      if (px > maxX) maxX = px;
      if (py > maxY) maxY = py;
      if (px > 0 && !visited[idx - 1] && marker(idx - 1)) {
        visited[idx - 1] = 1;
        stack.push(idx - 1);
      }
      if (px < width - 1 && !visited[idx + 1] && marker(idx + 1)) {
        visited[idx + 1] = 1;
        stack.push(idx + 1);
      }
      if (py > 0 && !visited[idx - width] && marker(idx - width)) {
        visited[idx - width] = 1;
        stack.push(idx - width);
      }
      if (py < height - 1 && !visited[idx + width] && marker(idx + width)) {
        visited[idx + width] = 1;
        stack.push(idx + width);
      }
    }
    const ch = maxY - minY + 1;
    if (maxX - minX + 1 < minSide && ch < minSide) continue;

    // Per-row marker span (absolute x) within this island.
    const left = new Int32Array(ch).fill(width);
    const right = new Int32Array(ch).fill(-1);
    for (const idx of pixels) {
      const px = idx % width;
      const ly = (idx - px) / width - minY;
      if (px < left[ly]) left[ly] = px;
      if (px > right[ly]) right[ly] = px;
    }
    const rowW = (ly: number) => (right[ly] >= left[ly] ? right[ly] - left[ly] + 1 : 0);
    const rowC = (ly: number) => ((left[ly] + right[ly]) / 2) | 0;

    // Band boundaries where width or centre steps vs the previous row.
    const cuts = [0];
    for (let ly = 1; ly < ch; ly++) {
      if (
        Math.abs(rowW(ly) - rowW(ly - 1)) > BAND_WIDTH_STEP ||
        Math.abs(rowC(ly) - rowC(ly - 1)) > BAND_CENTER_STEP
      ) {
        cuts.push(ly);
      }
    }
    cuts.push(ch);
    const bands: Array<[number, number]> = [];
    for (let i = 0; i < cuts.length - 1; i++) bands.push([cuts[i], cuts[i + 1]]);
    // Merge thin bands (transition rows) into the previous band.
    const merged: Array<[number, number]> = [];
    for (const b of bands) {
      const last = merged[merged.length - 1];
      if (last && (b[1] - b[0] < MIN_BAND_ROWS || last[1] - last[0] < MIN_BAND_ROWS)) {
        last[1] = b[1];
        continue;
      }
      merged.push([b[0], b[1]]);
    }
    // Emit each band as its own tight zone box.
    for (const [sy, ey] of merged) {
      let l = width;
      let r = -1;
      for (let ly = sy; ly < ey; ly++) {
        if (right[ly] >= left[ly]) {
          if (left[ly] < l) l = left[ly];
          if (right[ly] > r) r = right[ly];
        }
      }
      if (r < l) continue;
      boxes.push({ x: l, y: minY + sy, w: r - l + 1, h: ey - sy });
    }
  }

  // Order top-to-bottom, then left-to-right within each row (rows grouped by vertical overlap). The
  // editor numbers zones in this reading order, which is stable and template-agnostic.
  boxes.sort((a, b) => a.y - b.y || a.x - b.x);
  const ordered: Rect[] = [];
  let row: Rect[] = [];
  let rowBottom = -Infinity;
  const flush = () => {
    row.sort((p, q) => p.x - q.x);
    ordered.push(...row);
    row = [];
  };
  for (const b of boxes) {
    if (row.length === 0) {
      row.push(b);
      rowBottom = b.y + b.h - 1;
    } else if (b.y > rowBottom) {
      flush();
      row.push(b);
      rowBottom = b.y + b.h - 1;
    } else {
      row.push(b);
      rowBottom = Math.max(rowBottom, b.y + b.h - 1);
    }
  }
  flush();

  return ordered.map((b, i) => ({
    id: `zone${i}`,
    srcX: b.x,
    srcY: b.y,
    w: b.w,
    h: b.h,
    x: b.x,
    y: b.y,
    scaleX: 1,
    scaleY: 1,
  }));
}

/** Scaled footprint (painted size) of a zone in texture pixels. */
export function footprint(el: NameplateZone): { w: number; h: number } {
  return { w: el.w * el.scaleX, h: el.h * el.scaleY };
}

/** The eight resize-handle centres of a zone's footprint, in texture pixels. */
export function handleCenters(el: NameplateZone): Record<ResizeHandle, { x: number; y: number }> {
  const fp = footprint(el);
  const l = el.x;
  const t = el.y;
  const r = el.x + fp.w;
  const b = el.y + fp.h;
  const cx = (l + r) / 2;
  const cy = (t + b) / 2;
  return {
    nw: { x: l, y: t },
    n: { x: cx, y: t },
    ne: { x: r, y: t },
    e: { x: r, y: cy },
    se: { x: r, y: b },
    s: { x: cx, y: b },
    sw: { x: l, y: b },
    w: { x: l, y: cy },
  };
}

/** The handle whose centre is within `tol` (texture px) of the point, or null. Corners are tested
 *  before edges so a corner grab wins the overlap. */
export function hitHandle(
  el: NameplateZone,
  tx: number,
  ty: number,
  tol: number,
): ResizeHandle | null {
  const centers = handleCenters(el);
  // Corners first (they sit at edge endpoints, so an edge test could otherwise shadow them).
  const order: ResizeHandle[] = ["nw", "ne", "se", "sw", "n", "e", "s", "w"];
  for (const h of order) {
    const c = centers[h];
    if (Math.abs(tx - c.x) <= tol && Math.abs(ty - c.y) <= tol) return h;
  }
  return null;
}

/** Reshape a zone by dragging `handle` to texture-space point (px, py). The opposite edge/corner is
 *  anchored; each dragged axis keeps a minimum footprint of `min` px. Recomputes x/y + scaleX/scaleY
 *  (the source bbox w/h stay fixed — they're the stamp crop). Pure, so it's unit-tested in isolation. */
export function resizeZone(
  el: NameplateZone,
  handle: ResizeHandle,
  px: number,
  py: number,
  min = 4,
): NameplateZone {
  const fp = footprint(el);
  let left = el.x;
  let top = el.y;
  let right = el.x + fp.w;
  let bottom = el.y + fp.h;
  if (handle.includes("e")) right = Math.max(left + min, px);
  if (handle.includes("w")) left = Math.min(right - min, px);
  if (handle.includes("s")) bottom = Math.max(top + min, py);
  if (handle.includes("n")) top = Math.min(bottom - min, py);
  const w = right - left;
  const h = bottom - top;
  return {
    ...el,
    x: left,
    y: top,
    scaleX: el.w > 0 ? w / el.w : el.scaleX,
    scaleY: el.h > 0 ? h / el.h : el.scaleY,
  };
}

/** Clamp a zone's position so its scaled footprint stays inside the canvas. Keeps `x`/`y` as floats
 *  (callers round at bake / readout). */
export function clampZone(el: NameplateZone, width: number, height: number): NameplateZone {
  const fp = footprint(el);
  const x = Math.max(0, Math.min(Math.max(0, width - fp.w), el.x));
  const y = Math.max(0, Math.min(Math.max(0, height - fp.h), el.y));
  return { ...el, x, y };
}

// ── Curved text baselines ──────────────────────────────────────────────────────────────────────

/** Map a footprint-normalized point (u,v ∈ [0,1]) to texture space using a zone's current position
 *  and scaled footprint. */
export function curveDest(
  el: NameplateZone,
  norm: [number, number],
): { x: number; y: number } {
  const fp = footprint(el);
  return { x: el.x + norm[0] * fp.w, y: el.y + norm[1] * fp.h };
}

/** Inverse of {@link curveDest}: texture-space point → footprint-normalized (u,v). */
export function curveNorm(el: NameplateZone, tx: number, ty: number): [number, number] {
  const fp = footprint(el);
  return [fp.w > 0 ? (tx - el.x) / fp.w : 0, fp.h > 0 ? (ty - el.y) / fp.h : 0];
}

/** Evaluate a quadratic Bézier (control points already in the target space) at parameter t ∈ [0,1],
 *  returning the point and the tangent angle (radians). */
export function quadAt(
  p0: [number, number],
  p1: [number, number],
  p2: [number, number],
  t: number,
): { x: number; y: number; angle: number } {
  const mt = 1 - t;
  const x = mt * mt * p0[0] + 2 * mt * t * p1[0] + t * t * p2[0];
  const y = mt * mt * p0[1] + 2 * mt * t * p1[1] + t * t * p2[1];
  // B'(t) = 2(1-t)(P1-P0) + 2t(P2-P1)
  const dx = 2 * mt * (p1[0] - p0[0]) + 2 * t * (p2[0] - p1[0]);
  const dy = 2 * mt * (p1[1] - p0[1]) + 2 * t * (p2[1] - p1[1]);
  return { x, y, angle: Math.atan2(dy, dx) };
}

/** An arc-length lookup over a quadratic Bézier: total `length` and `at(s)` → point + tangent at arc
 *  distance s (px), so glyphs can be spaced evenly along the curve regardless of its curvature. */
export function arcTable(
  p0: [number, number],
  p1: [number, number],
  p2: [number, number],
  samples = 48,
): { length: number; at: (s: number) => { x: number; y: number; angle: number } } {
  const ts: number[] = [];
  const cum: number[] = [];
  let prev = quadAt(p0, p1, p2, 0);
  ts.push(0);
  cum.push(0);
  let total = 0;
  for (let i = 1; i <= samples; i++) {
    const t = i / samples;
    const pt = quadAt(p0, p1, p2, t);
    total += Math.hypot(pt.x - prev.x, pt.y - prev.y);
    ts.push(t);
    cum.push(total);
    prev = pt;
  }
  const at = (s: number) => {
    const target = Math.max(0, Math.min(total, s));
    // Binary-search the cumulative table for the segment containing `target`.
    let lo = 0;
    let hi = cum.length - 1;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (cum[mid] < target) lo = mid + 1;
      else hi = mid;
    }
    const i = Math.max(1, lo);
    const seg = cum[i] - cum[i - 1] || 1;
    const f = (target - cum[i - 1]) / seg;
    const t = ts[i - 1] + (ts[i] - ts[i - 1]) * f;
    return quadAt(p0, p1, p2, t);
  };
  return { length: total, at };
}

/** Auto-seed a curved baseline for a zone by tracing the marker's per-column vertical midline within
 *  its source bbox, fitting a quadratic through the midline at the two ends + centre, and measuring
 *  the glyph height from the band thickness. Returns the curve plus how far the midline bends
 *  (`curvature`) and tilts (`slant`) — both as fractions of the bbox height — so the caller can decide
 *  whether a zone is curved enough to warrant a baseline (vs. staying a straight rect). Returns null
 *  when the marker is too sparse to trace. */
export function deriveZoneCurve(
  rgba: Uint8Array | Uint8ClampedArray,
  width: number,
  el: NameplateZone,
  bg: [number, number, number],
): { curve: ZoneCurve; curvature: number; slant: number } | null {
  const { srcX, srcY, w, h } = el;
  if (w < 2 || h < 1) return null;
  const mids: Array<number | null> = new Array(w).fill(null);
  const thicks: number[] = [];
  for (let u = 0; u < w; u++) {
    let lo = h;
    let hi = -1;
    for (let v = 0; v < h; v++) {
      const i = ((srcY + v) * width + (srcX + u)) * 4;
      if (isMarkerColor(rgba[i], rgba[i + 1], rgba[i + 2], bg)) {
        if (v < lo) lo = v;
        if (v > hi) hi = v;
      }
    }
    if (hi >= lo) {
      mids[u] = (lo + hi) / 2;
      thicks.push(hi - lo + 1);
    }
  }
  if (thicks.length < 2) return null;
  // Nearest-non-null sample at a normalized column position.
  const sampleAt = (frac: number): number => {
    const target = Math.round(frac * (w - 1));
    for (let d = 0; d < w; d++) {
      const a = mids[Math.max(0, target - d)];
      if (a != null) return a;
      const b = mids[Math.min(w - 1, target + d)];
      if (b != null) return b;
    }
    return h / 2;
  };
  const m0 = sampleAt(0);
  const mc = sampleAt(0.5);
  const m1 = sampleAt(1);
  // Bézier control whose curve passes THROUGH the measured centre midline point (B(0.5) = mc).
  const p1y = 2 * mc - (m0 + m1) / 2;
  const median = [...thicks].sort((a, b) => a - b)[thicks.length >> 1];
  const curvature = Math.abs((m0 + m1) / 2 - mc) / h; // bend depth vs the chord
  const slant = Math.abs(m1 - m0) / h; // end-to-end tilt
  const curve: ZoneCurve = {
    p0: [0, m0 / h],
    p1: [0.5, p1y / h],
    p2: [1, m1 / h],
    thickness: Math.min(1, Math.max(0.05, median / h)),
  };
  return { curve, curvature, slant };
}

/** Hit-test: the topmost zone whose scaled footprint contains the texture-space point, or null. */
export function hitTest(zones: NameplateZone[], tx: number, ty: number): NameplateZone | null {
  for (let i = zones.length - 1; i >= 0; i--) {
    const e = zones[i];
    const fp = footprint(e);
    if (tx >= e.x && tx < e.x + fp.w && ty >= e.y && ty < e.y + fp.h) return e;
  }
  return null;
}
