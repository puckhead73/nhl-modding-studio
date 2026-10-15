// Nameplate glyph layout — places a player's name/number glyphs (cells from the
// font atlas, see fontAtlas.ts) across a target zone, so the editor can render
// text "through the font map" onto the jersey and update it live as zones move.
//
// The layout is pure geometry (source cell rect → destination rect on the
// texture), unit-testable without a canvas; a thin canvas draw step consumes it.

import type { GlyphRect, FontMap } from "./fontAtlas";

export interface GlyphPlacement {
  /** Source rect in the font atlas. */
  src: GlyphRect;
  /** Destination rect on the target texture (jersey), in texture pixels. */
  dst: { x: number; y: number; w: number; h: number };
}

export interface LayoutOptions {
  /** Fraction of the space between glyphs, as a multiple of the scaled glyph
   *  height (0.12 ≈ tight jersey lettering). */
  tracking?: number;
  /** Vertical fill: glyph height as a fraction of the zone height. */
  fill?: number;
}

/**
 * Lay a string's glyphs left-to-right, centered within `zone`. Each character is
 * resolved to an atlas cell via `cellOf` (returns null for spaces / unknowns,
 * which insert a gap). Glyphs share one scale (so a baseline is preserved),
 * chosen to fill `fill × zone.h`; if the run is wider than the zone it's scaled
 * down to fit. Returns the placements to blit (empty when nothing resolves).
 */
export function layoutText(
  text: string,
  cellOf: (ch: string) => GlyphRect | null,
  zone: { x: number; y: number; w: number; h: number },
  opts: LayoutOptions = {},
): GlyphPlacement[] {
  const tracking = opts.tracking ?? 0.12;
  const fill = opts.fill ?? 0.9;

  // Resolve glyphs; spaces/unknowns become gaps (null cell).
  const chars = [...text.toUpperCase()].map(cellOf);
  const present = chars.filter((c): c is GlyphRect => c != null);
  if (present.length === 0) return [];

  // One shared scale so glyphs sit on a common baseline: target glyph height =
  // fill × zone.h, scaled from the tallest cell so nothing exceeds it.
  const refH = Math.max(...present.map((c) => c.h));
  let scale = (zone.h * fill) / refH;
  const gap = () => refH * scale * tracking;

  const widthAt = (s: number) => {
    let w = 0;
    let first = true;
    for (const c of chars) {
      if (!c) {
        w += refH * s * 0.5; // space ≈ half a glyph height
        first = false;
        continue;
      }
      if (!first) w += refH * s * tracking;
      w += c.w * s;
      first = false;
    }
    return w;
  };

  // Shrink to fit the zone width if the run overflows.
  const total = widthAt(scale);
  if (total > zone.w) scale *= zone.w / total;

  const runW = widthAt(scale);
  let x = zone.x + (zone.w - runW) / 2;
  const midY = zone.y + zone.h / 2;

  const out: GlyphPlacement[] = [];
  let first = true;
  for (const c of chars) {
    if (!c) {
      x += refH * scale * 0.5;
      first = false;
      continue;
    }
    if (!first) x += gap();
    const dh = c.h * scale;
    const dw = c.w * scale;
    out.push({ src: c, dst: { x, y: midY - dh / 2, w: dw, h: dh } });
    x += dw;
    first = false;
  }
  return out;
}

/** Resolve name letters (A–Z) from a font map. */
export function nameCellOf(font: FontMap): (ch: string) => GlyphRect | null {
  return (ch) => font.letters.get(ch) ?? null;
}

/** Resolve number digits from a font map — the big set (falls back to small). */
export function numberCellOf(font: FontMap, big = true): (ch: string) => GlyphRect | null {
  return (ch) => (big ? font.bigDigits.get(ch) : font.smallDigits.get(ch)) ?? font.bigDigits.get(ch) ?? null;
}

/** Resolve the captain-patch letter — the large offset "C"/"A" styled for the
 *  patch, falling back to the regular letter cell when the atlas lacks them. */
export function captainCellOf(font: FontMap): (ch: string) => GlyphRect | null {
  return (ch) =>
    ch === "C"
      ? (font.captainC ?? font.letters.get("C") ?? null)
      : ch === "A"
        ? (font.captainA ?? font.letters.get("A") ?? null)
        : null;
}

/**
 * Classify detected nameplate zones by geometry: the number zones are the
 * near-square markers (largest first — back number, then the smaller sleeve /
 * front squares); the name zone is the widest (most horizontal) remaining
 * marker. `footprintOf` gives a zone's painted size (position + scale applied).
 */
export function classifyZones<Z extends { id: string }>(
  zones: Z[],
  footprintOf: (z: Z) => { w: number; h: number },
): { nameZoneId?: string; numberZoneIds: string[] } {
  const withFp = zones.map((z) => ({ z, fp: footprintOf(z) })).filter(({ fp }) => fp.h > 0);
  if (withFp.length === 0) return { numberZoneIds: [] };

  // Number zones: roughly-square markers (0.5–1.8 aspect), largest area first.
  const squares = withFp
    .filter(({ fp }) => fp.w / fp.h >= 0.5 && fp.w / fp.h <= 1.8)
    .sort((a, b) => b.fp.w * b.fp.h - a.fp.w * a.fp.h);
  const numberZoneIds = squares.map((s) => s.z.id);

  // Name: the widest (highest w/h) marker that isn't a number square.
  const rest = withFp.filter(({ z }) => !numberZoneIds.includes(z.id));
  const widest = rest.sort((a, b) => b.fp.w / b.fp.h - a.fp.w / a.fp.h)[0];

  return { nameZoneId: widest?.z.id, numberZoneIds };
}
