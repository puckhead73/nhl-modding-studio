// Jersey nameplate (name / number / patch location) editor — a jersey-scoped modal.
//
// A `name_*` sheet is a fully-opaque template: marker "zones" (a name region, a curved nameplate
// strip, patch/number squares — yellow in the stock files) on a flat background. The game draws the
// player's name over the name region and the patches at the slots at runtime; the texture only
// encodes WHERE. This editor detects each marker island as a zone, shows them at 40% over the jersey
// body colour map (a rough UV-aligned reference), and lets the operator select a zone and nudge /
// resize / delete it. On save it bakes a fresh sheet — background filled flat (clearing the old
// zones), each zone repainted at its new position + scale — and writes it back through
// `import_texture_bytes` (the name itself is NOT baked; the game renders it dynamically).

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { Button, ErrorBanner, Spinner } from "./primitives";
import {
  backgroundColor,
  clampZone,
  detectZones,
  footprint,
  hitTest,
  isMarkerColor,
  type NameplateZone,
} from "../lib/nameplateGeometry";
import { buildFontMap, type FontMap, type GlyphRect } from "../lib/fontAtlas";
import { captainCellOf, classifyZones, layoutText, nameCellOf, numberCellOf } from "../lib/nameplateCompositor";
import { NameplateJersey3D } from "./NameplateJersey3D";
import { useAssetRootArgs } from "../lib/project";

/** How a detected zone is rendered in the preview. */
type ZoneType =
  | "name"
  | "number"
  | "sleeveL"
  | "sleeveR"
  | "captain"
  | "stanley"
  | "ignore";
const ZONE_TYPE_LABELS: Record<ZoneType, string> = {
  name: "Name",
  number: "Number (back)",
  sleeveR: "Sleeve — right",
  sleeveL: "Sleeve — left",
  captain: "Captain patch (C/A)",
  stanley: "Stanley Cup patch",
  ignore: "Ignore",
};
// Rotation of a zone's text relative to the upright back number. Canvas rotate()
// is clockwise (y-down): sleeves sit 90° off (left = +90° CW, right = −90° CCW);
// the captain patch is inverted 180°.
const ZONE_ANGLE: Partial<Record<ZoneType, number>> = {
  sleeveL: Math.PI / 2,
  sleeveR: -Math.PI / 2,
  captain: Math.PI,
};

/**
 * Draw a text run into a target rect (already in destination-pixel space),
 * optionally rotated about its centre. Atlas glyphs are stored **180°-rotated**
 * (upside-down), so each is blitted with a 180° rotation (`scale(-1,-1)`) — a
 * single flip only mirrors them.
 */
function renderZoneRun(
  ctx: CanvasRenderingContext2D,
  atlas: CanvasImageSource,
  text: string,
  cellOf: (ch: string) => GlyphRect | null,
  tx: number,
  ty: number,
  tw: number,
  th: number,
  angleRad: number,
) {
  if (!text) return;
  const rotated = angleRad !== 0;
  // When rotated 90°, the run's length spans the zone's height and vice versa.
  const along = rotated ? th : tw;
  const across = rotated ? tw : th;
  // Each glyph is blitted with a 180° rotation (atlas glyphs are stored
  // upside-down), which reverses the run's reading order — so lay the characters
  // out reversed here to cancel it (BACKES not SEKCAB, 42 not 24).
  const reversed = [...text].reverse().join("");
  const placements = layoutText(reversed, cellOf, { x: -along / 2, y: -across / 2, w: along, h: across });
  ctx.save();
  ctx.translate(tx + tw / 2, ty + th / 2);
  if (rotated) ctx.rotate(angleRad);
  for (const g of placements) {
    ctx.save();
    ctx.translate(g.dst.x + g.dst.w, g.dst.y + g.dst.h);
    ctx.scale(-1, -1);
    ctx.drawImage(atlas, g.src.x, g.src.y, g.src.w, g.src.h, 0, 0, g.dst.w, g.dst.h);
    ctx.restore();
  }
  ctx.restore();
}

const MAX_DISPLAY = 512;
const ZONE_EDIT_ALPHA = 0.4;
const REFERENCE_ALPHA = 0.6;
const MIN_SCALE = 0.1;
const MAX_SCALE = 8;

/** Load a PNG data URL into an offscreen canvas so its pixels can be read back (getImageData). */
function loadImageToCanvas(dataUrl: string): Promise<HTMLCanvasElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      const c = document.createElement("canvas");
      c.width = img.naturalWidth;
      c.height = img.naturalHeight;
      const ctx = c.getContext("2d");
      if (!ctx) return reject(new Error("no 2d context"));
      ctx.drawImage(img, 0, 0);
      resolve(c);
    };
    img.onerror = () => reject(new Error("image decode failed"));
    img.src = dataUrl;
  });
}

/** A repaint stamp for one zone: its bbox crop with background pixels keyed to transparent, so only
 *  the marker paints when redrawn at a new position. */
function buildStamp(
  src: HTMLCanvasElement,
  el: NameplateZone,
  bg: [number, number, number],
): HTMLCanvasElement {
  const c = document.createElement("canvas");
  c.width = el.w;
  c.height = el.h;
  const ctx = c.getContext("2d")!;
  const img = src.getContext("2d")!.getImageData(el.srcX, el.srcY, el.w, el.h);
  const d = img.data;
  for (let i = 0; i < d.length; i += 4) {
    if (!isMarkerColor(d[i], d[i + 1], d[i + 2], bg)) d[i + 3] = 0;
  }
  ctx.putImageData(img, 0, 0);
  return c;
}

function arrayBufferToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}

export function NameplateEditor({
  projectId,
  teamId,
  installDir,
  relPath,
  textureIndex,
  label,
  referenceRelPath,
  referenceTextureIndex,
  fontRelPath,
  fontTextureIndex,
  kitStyle,
  kitVariant,
  onClose,
  onSaved,
}: {
  projectId: number;
  teamId: number;
  installDir: string;
  relPath: string;
  /** The raster inside the name-sheet file (texlibs are multi-raster; a name sheet is single). */
  textureIndex?: number;
  label: string;
  /** The matching jersey-body colour map to underlay as a rough alignment reference (or null). */
  referenceRelPath: string | null;
  /** The body raster within the reference file (a jersey bundle's body index). */
  referenceTextureIndex?: number;
  /** The kit's number `font` map, used to render the live name/number preview (or null). */
  fontRelPath?: string | null;
  /** The font raster within its bundle. */
  fontTextureIndex?: number;
  /** The kit's (style, variant) — used to fetch the jersey mesh for the 3D preview. */
  kitStyle?: number | null;
  kitVariant?: number | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const { rootArgs } = useAssetRootArgs();
  const sheet = useQuery({
    queryKey: ["texturePreview", installDir, relPath, 0, textureIndex],
    queryFn: () => call("texture_preview", { installDir, relPath, maxEdge: 0, textureIndex }),
    staleTime: Infinity,
    retry: false,
  });
  const reference = useQuery({
    queryKey: ["texturePreview", installDir, referenceRelPath, 0, referenceTextureIndex],
    queryFn: () =>
      call("texture_preview", {
        installDir,
        relPath: referenceRelPath!,
        maxEdge: 0,
        textureIndex: referenceTextureIndex,
      }),
    enabled: !!referenceRelPath,
    staleTime: Infinity,
    retry: false,
  });

  // The kit's number `font` map — decoded to an atlas + glyph rects, so the
  // preview renders the name/number through the real font the game samples.
  const fontTex = useQuery({
    queryKey: ["texturePreview", installDir, fontRelPath, 0, fontTextureIndex],
    queryFn: () =>
      call("texture_preview", { installDir, relPath: fontRelPath!, maxEdge: 0, textureIndex: fontTextureIndex }),
    enabled: !!fontRelPath,
    staleTime: Infinity,
    retry: false,
  });

  // The team's roster — pick whose name/number to preview.
  const roster = useQuery({
    queryKey: ["players", projectId],
    queryFn: () => call("list_players", { projectId }),
  });
  const teamPlayers = useMemo(
    () =>
      (roster.data ?? [])
        .filter((p) => p.team_id === teamId)
        .sort((a, b) => (b.overall ?? -1) - (a.overall ?? -1)),
    [roster.data, teamId],
  );
  const [playerId, setPlayerId] = useState<number | null>(null);
  useEffect(() => {
    if (playerId == null && teamPlayers[0]) setPlayerId(teamPlayers[0].player_id);
  }, [teamPlayers, playerId]);
  const player = teamPlayers.find((p) => p.player_id === playerId) ?? null;
  const nameText = (player?.last_name ?? "").toUpperCase();
  const numberText = player?.jersey_number != null ? String(player.jersey_number) : "";

  // Captaincy (roster-slot `eGlu.mCcl`, via lines_view): 1 = captain → "C",
  // 2 = alternate → "A". Drives the captain-patch letter.
  const lines = useQuery({
    queryKey: ["lines", projectId, teamId],
    queryFn: () => call("lines_view", { projectId, teamId }),
  });
  const captaincyOf = useMemo(() => {
    const m = new Map<number, number>();
    for (const e of lines.data?.entries ?? [])
      if (e.player_id != null && e.captaincy) m.set(e.player_id, e.captaincy);
    return m;
  }, [lines.data]);
  const role = player ? captaincyOf.get(player.player_id) : undefined;
  const captainLetter = role === 1 ? "C" : role === 2 ? "A" : "";

  // The jersey mesh for the 3D preview — the jersey piece of the assembled kit.
  const canPreview3d = kitStyle != null && kitVariant != null && !!installDir;
  const kitPreview = useQuery({
    queryKey: ["teamKitPreview", projectId, teamId, kitStyle, kitVariant, null],
    queryFn: () =>
      call("team_kit_preview", {
        projectId,
        roots: rootArgs,
        teamId,
        style: kitStyle!,
        variant: kitVariant!,
        playerId: null,
      }),
    enabled: canPreview3d,
    staleTime: Infinity,
    retry: false,
  });
  const jersey = kitPreview.data?.pieces.find((p) => p.kind === "jersey") ?? null;
  const jerseyBounds = useMemo(() => {
    if (!jersey) return { center: [0, 1.3, 0] as [number, number, number], span: 1 };
    const lo = [Infinity, Infinity, Infinity];
    const hi = [-Infinity, -Infinity, -Infinity];
    for (const s of jersey.submeshes)
      for (const p of s.positions)
        for (let k = 0; k < 3; k++) {
          lo[k] = Math.min(lo[k], p[k]);
          hi[k] = Math.max(hi[k], p[k]);
        }
    if (!Number.isFinite(lo[0])) return { center: [0, 1.3, 0] as [number, number, number], span: 1 };
    return {
      center: [(lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2] as [number, number, number],
      span: Math.max(hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]) || 1,
    };
  }, [jersey]);

  const [show3d, setShow3d] = useState(true);
  // The live jersey texture (colour map + composited glyphs). The canvas is built
  // in a ref, then published to state so the 3D viewer reliably picks it up
  // (a `tick` refreshes the GPU texture on subsequent redraws).
  const jerseyTexRef = useRef<HTMLCanvasElement | null>(null);
  const [jerseyTex, setJerseyTex] = useState<HTMLCanvasElement | null>(null);
  const [jerseyTick, setJerseyTick] = useState(0);

  const save = useMutation({
    mutationFn: (pngBase64: string) =>
      call("import_texture_bytes", { installDir, relPath, pngBase64, writeBackup: true, textureIndex }),
    onSuccess: onSaved,
  });

  const sourceRef = useRef<HTMLCanvasElement | null>(null);
  const jerseyRef = useRef<HTMLCanvasElement | null>(null);
  const displayRef = useRef<HTMLCanvasElement | null>(null);
  const stampsRef = useRef<Map<string, HTMLCanvasElement>>(new Map());
  const bgColorRef = useRef<[number, number, number]>([255, 255, 255]);
  const atlasRef = useRef<HTMLCanvasElement | null>(null);
  const fontMapRef = useRef<FontMap | null>(null);

  const [dims, setDims] = useState<{ w: number; h: number } | null>(null);
  const [zones, setZones] = useState<NameplateZone[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [ready, setReady] = useState(0); // bumps when source/reference/stamps/atlas rebuild

  // Which detected zones are the name vs the numbers (back + sleeves) — geometry.
  const { nameZoneId, numberZoneIds } = useMemo(
    () => classifyZones(zones, footprint),
    [zones],
  );

  // Per-zone type — auto-classified, overridable so the operator can mark the
  // captain patch (which geometry can't tell from a sleeve number). Reset per sheet.
  const [zoneTypes, setZoneTypes] = useState<Record<string, ZoneType>>({});
  useEffect(() => setZoneTypes({}), [relPath]);
  // Auto-classify: the widest marker is the name and the LARGEST square is the
  // back number; everything else (sleeves, patches) is left to the operator to
  // assign — left/right and captain/Stanley can't be told apart by geometry.
  const autoType = useCallback(
    (id: string): ZoneType =>
      id === nameZoneId ? "name" : id === numberZoneIds[0] ? "number" : "ignore",
    [nameZoneId, numberZoneIds],
  );
  const effType = useCallback(
    (id: string): ZoneType => zoneTypes[id] ?? autoType(id),
    [zoneTypes, autoType],
  );

  // What to render in each zone (name letters / number digits / captain C-A) and
  // at what rotation — shared by the 2D and 3D compositors. The back number is
  // the big font; sleeves the small font, rotated 90°.
  const buildRenderList = useCallback(
    (
      fontMap: FontMap,
    ): Array<{
      zone: NameplateZone;
      text: string;
      cellOf: (ch: string) => GlyphRect | null;
      angle: number;
    }> => {
      const out: Array<{
        zone: NameplateZone;
        text: string;
        cellOf: (ch: string) => GlyphRect | null;
        angle: number;
      }> = [];
      for (const z of zones) {
        const t = effType(z.id);
        const angle = ZONE_ANGLE[t] ?? 0;
        if (t === "name" && nameText) {
          out.push({ zone: z, text: nameText, cellOf: nameCellOf(fontMap), angle });
        } else if ((t === "number" || t === "sleeveL" || t === "sleeveR") && numberText) {
          out.push({ zone: z, text: numberText, cellOf: numberCellOf(fontMap, t === "number"), angle });
        } else if (t === "captain" && captainLetter) {
          out.push({ zone: z, text: captainLetter, cellOf: captainCellOf(fontMap), angle });
        }
        // "stanley" has no font glyph (a logo patch we don't render) and "ignore"
        // renders nothing — both keep their marker instead.
      }
      return out;
    },
    [zones, effType, nameText, numberText, captainLetter],
  );

  // Decode the template → detect zones → build per-zone repaint stamps.
  useEffect(() => {
    if (!sheet.data) return;
    let cancelled = false;
    loadImageToCanvas(sheet.data.data_url).then((c) => {
      if (cancelled) return;
      sourceRef.current = c;
      const rgba = c.getContext("2d")!.getImageData(0, 0, c.width, c.height).data;
      const bg = backgroundColor(rgba);
      bgColorRef.current = bg;
      const detected = detectZones(rgba, c.width, c.height);
      const stamps = new Map<string, HTMLCanvasElement>();
      for (const z of detected) stamps.set(z.id, buildStamp(c, z, bg));
      stampsRef.current = stamps;
      setDims({ w: c.width, h: c.height });
      setZones(detected);
      setSelectedId(detected[0]?.id ?? null);
      setReady((n) => n + 1);
    });
    return () => {
      cancelled = true;
    };
  }, [sheet.data]);

  // Decode the reference jersey body (best-effort underlay).
  useEffect(() => {
    if (!reference.data) {
      jerseyRef.current = null;
      return;
    }
    let cancelled = false;
    loadImageToCanvas(reference.data.data_url).then((c) => {
      if (cancelled) return;
      jerseyRef.current = c;
      setReady((n) => n + 1);
    });
    return () => {
      cancelled = true;
    };
  }, [reference.data]);

  // Decode the font atlas → glyph map (best-effort; null when it can't be read).
  useEffect(() => {
    if (!fontTex.data) {
      atlasRef.current = null;
      fontMapRef.current = null;
      return;
    }
    let cancelled = false;
    loadImageToCanvas(fontTex.data.data_url).then((c) => {
      if (cancelled) return;
      atlasRef.current = c;
      const rgba = c.getContext("2d")!.getImageData(0, 0, c.width, c.height).data;
      fontMapRef.current = buildFontMap(rgba, c.width, c.height);
      setReady((n) => n + 1);
    });
    return () => {
      cancelled = true;
    };
  }, [fontTex.data]);

  const scale = useMemo(() => {
    if (!dims) return 1;
    return Math.max(1, Math.min(MAX_DISPLAY / dims.w, MAX_DISPLAY / dims.h));
  }, [dims]);

  // Redraw: jersey reference underlay → zones (40%, background keyed out) → outlines.
  useEffect(() => {
    const canvas = displayRef.current;
    if (!canvas || !dims) return;
    const dispW = Math.round(dims.w * scale);
    const dispH = Math.round(dims.h * scale);
    canvas.width = dispW;
    canvas.height = dispH;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, dispW, dispH);

    const jersey = jerseyRef.current;
    if (jersey) {
      ctx.globalAlpha = REFERENCE_ALPHA;
      ctx.drawImage(jersey, 0, 0, jersey.width, jersey.height, 0, 0, dispW, dispH);
    }

    const atlas = atlasRef.current;
    const fontMap = fontMapRef.current;
    const renderList = atlas && fontMap ? buildRenderList(fontMap) : [];
    const renderedIds = new Set(renderList.map((r) => r.zone.id));

    const OUTLINE: Partial<Record<ZoneType, string>> = {
      name: "rgba(52,211,153,0.7)",
      number: "rgba(251,191,36,0.7)",
      sleeveL: "rgba(96,165,250,0.7)",
      sleeveR: "rgba(96,165,250,0.7)",
      captain: "rgba(192,132,252,0.7)",
      stanley: "rgba(250,204,21,0.8)",
    };
    for (const el of zones) {
      const stamp = stampsRef.current.get(el.id);
      const fp = footprint(el);
      const type = effType(el.id);
      // A zone that renders glyphs hides its marker; zones that don't (Ignore,
      // Stanley, an empty patch) keep the marker so their location stays visible.
      if (stamp && !renderedIds.has(el.id)) {
        ctx.globalAlpha = ZONE_EDIT_ALPHA;
        ctx.drawImage(stamp, 0, 0, el.w, el.h, el.x * scale, el.y * scale, fp.w * scale, fp.h * scale);
      }
      ctx.globalAlpha = 1;
      const isSel = el.id === selectedId;
      ctx.strokeStyle = isSel ? "#38bdf8" : OUTLINE[type] ?? "rgba(148,163,184,0.7)";
      ctx.lineWidth = isSel ? 2 : 1;
      ctx.strokeRect(el.x * scale + 0.5, el.y * scale + 0.5, fp.w * scale - 1, fp.h * scale - 1);
    }

    // Render name / number / sleeve / captain through the font atlas.
    if (atlas) {
      ctx.globalAlpha = 1;
      for (const { zone, text, cellOf, angle } of renderList) {
        const fp = footprint(zone);
        renderZoneRun(
          ctx,
          atlas,
          text,
          cellOf,
          zone.x * scale,
          zone.y * scale,
          fp.w * scale,
          fp.h * scale,
          angle,
        );
      }
    }
    ctx.globalAlpha = 1;
  }, [zones, selectedId, dims, scale, ready, effType, buildRenderList]);

  // Build the live JERSEY texture for the 3D preview: the jersey colour map with
  // the name/number composited at the zones' UV positions (the name sheet and the
  // jersey body share UV space, so a zone at (x/w, y/h) on the sheet maps to the
  // same fraction of the jersey map). Redrawn whenever the layout changes.
  useEffect(() => {
    const jerseyMap = jerseyRef.current;
    if (!jerseyMap || !dims) return;
    let out = jerseyTexRef.current;
    if (!out || out.width !== jerseyMap.width || out.height !== jerseyMap.height) {
      out = document.createElement("canvas");
      out.width = jerseyMap.width;
      out.height = jerseyMap.height;
      jerseyTexRef.current = out;
    }
    const ctx = out.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, out.width, out.height);
    ctx.drawImage(jerseyMap, 0, 0);

    const atlas = atlasRef.current;
    const fontMap = fontMapRef.current;
    if (atlas && fontMap) {
      const sx = out.width / dims.w;
      const sy = out.height / dims.h;
      for (const { zone, text, cellOf, angle } of buildRenderList(fontMap)) {
        const fp = footprint(zone);
        renderZoneRun(ctx, atlas, text, cellOf, zone.x * sx, zone.y * sy, fp.w * sx, fp.h * sy, angle);
      }
    }
    setJerseyTex(out);
    setJerseyTick((n) => n + 1);
  }, [zones, dims, ready, buildRenderList]);

  const onPointerDown = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if (!dims) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const tx = (e.clientX - rect.left) / scale;
    const ty = (e.clientY - rect.top) / scale;
    const hit = hitTest(zones, tx, ty);
    if (hit) setSelectedId(hit.id);
  };

  const mutateSelected = useCallback(
    (fn: (el: NameplateZone) => NameplateZone) => {
      if (!dims || !selectedId) return;
      setZones((prev) =>
        prev.map((el) => (el.id === selectedId ? clampZone(fn(el), dims.w, dims.h) : el)),
      );
    },
    [dims, selectedId],
  );

  const nudge = useCallback(
    (dx: number, dy: number) => mutateSelected((el) => ({ ...el, x: el.x + dx, y: el.y + dy })),
    [mutateSelected],
  );
  const resize = useCallback(
    (factor: number) =>
      mutateSelected((el) => ({
        ...el,
        scale: Math.max(MIN_SCALE, Math.min(MAX_SCALE, el.scale * factor)),
      })),
    [mutateSelected],
  );
  const deleteSelected = () => {
    if (!selectedId) return;
    setZones((prev) => prev.filter((e) => e.id !== selectedId));
    setSelectedId(null);
  };

  // Keyboard: Esc closes; arrows nudge (Shift = 10px); +/- resize.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") return onClose();
      const step = e.shiftKey ? 10 : 1;
      switch (e.key) {
        case "ArrowLeft":
          nudge(-step, 0);
          break;
        case "ArrowRight":
          nudge(step, 0);
          break;
        case "ArrowUp":
          nudge(0, -step);
          break;
        case "ArrowDown":
          nudge(0, step);
          break;
        case "+":
        case "=":
          resize(1.1);
          break;
        case "-":
        case "_":
          resize(1 / 1.1);
          break;
        default:
          return;
      }
      e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [nudge, resize, onClose]);

  const handleSave = () => {
    if (!dims) return;
    const out = document.createElement("canvas");
    out.width = dims.w;
    out.height = dims.h;
    const ctx = out.getContext("2d");
    if (!ctx) return;
    ctx.imageSmoothingEnabled = false;
    // Clear to the flat background colour (erases the original zones)…
    const [r, g, b] = bgColorRef.current;
    ctx.fillStyle = `rgb(${r},${g},${b})`;
    ctx.fillRect(0, 0, dims.w, dims.h);
    // …then repaint each zone at its new position + scale, full opacity.
    for (const el of zones) {
      const stamp = stampsRef.current.get(el.id);
      if (!stamp) continue;
      const fp = footprint(el);
      ctx.drawImage(
        stamp,
        0,
        0,
        el.w,
        el.h,
        Math.round(el.x),
        Math.round(el.y),
        Math.round(fp.w),
        Math.round(fp.h),
      );
    }
    out.toBlob((blob) => {
      if (!blob) return;
      void blob.arrayBuffer().then((buf) => save.mutate(arrayBufferToBase64(buf)));
    }, "image/png");
  };

  const selected = zones.find((e) => e.id === selectedId) ?? null;
  const decodeError = sheet.error ? String(sheet.error) : null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4"
      onClick={onClose}
    >
      <div
        className="flex max-h-full w-[min(1240px,96vw)] flex-col overflow-hidden rounded-lg border border-slate-800 bg-slate-900 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <header className="flex items-center justify-between gap-4 border-b border-slate-800 px-5 py-3">
          <div>
            <h2 className="text-base font-semibold text-slate-100">Name / number / patch locations</h2>
            <p className="mt-0.5 text-[11px] text-slate-400">
              {label} · select a zone (click it or the list), then nudge with the arrow keys (Shift =
              10px) and resize with +/−. Zones show at {Math.round(ZONE_EDIT_ALPHA * 100)}% over the
              jersey body for reference. Save repaints them onto a fresh sheet (keeps a{" "}
              <code>.bak</code>).
            </p>
          </div>
          <div className="flex items-center gap-2">
            {canPreview3d && (
              <label className="flex items-center gap-1.5 rounded border border-slate-700 px-2 py-1 text-xs text-slate-300">
                <input
                  type="checkbox"
                  checked={show3d}
                  onChange={(e) => setShow3d(e.target.checked)}
                />
                3D preview
              </label>
            )}
            <button
              type="button"
              onClick={onClose}
              className="rounded border border-slate-700 px-2 py-1 text-xs text-slate-200 hover:bg-slate-800"
            >
              Close
            </button>
          </div>
        </header>

        <div className="flex min-h-0 flex-1 gap-4 p-4">
          <div className="flex min-w-0 flex-1 items-center justify-center">
            {decodeError ? (
              <ErrorBanner title="Couldn't decode this nameplate" message={decodeError} />
            ) : !sheet.data || !dims ? (
              <Spinner label="Decoding…" />
            ) : (
              <div
                className="rounded"
                style={{
                  backgroundImage: "repeating-conic-gradient(#1e293b 0% 25%, #0f172a 0% 50%)",
                  backgroundSize: "16px 16px",
                }}
              >
                <canvas
                  ref={displayRef}
                  onPointerDown={onPointerDown}
                  style={{ imageRendering: "pixelated", touchAction: "none", cursor: "pointer" }}
                />
              </div>
            )}
          </div>

          {show3d && canPreview3d ? (
            <div className="flex w-[22rem] shrink-0 flex-col gap-1">
              <div className="text-[10px] uppercase tracking-wider text-slate-500">
                3D preview · live
              </div>
              <div className="min-h-[20rem] flex-1 overflow-hidden rounded border border-slate-800 bg-slate-950">
                {jersey ? (
                  <NameplateJersey3D
                    submeshes={jersey.submeshes}
                    textureCanvas={jerseyTex}
                    tick={jerseyTick}
                    center={jerseyBounds.center}
                    span={jerseyBounds.span}
                  />
                ) : kitPreview.isLoading ? (
                  <div className="grid h-full w-full place-items-center">
                    <Spinner label="Loading jersey…" />
                  </div>
                ) : (
                  <div className="grid h-full w-full place-items-center p-3 text-center text-[11px] text-slate-500">
                    No jersey mesh available for this kit.
                  </div>
                )}
              </div>
              <p className="text-[10px] leading-tight text-slate-500">
                Drag to rotate. The name/number update live as you move the zones.
              </p>
            </div>
          ) : null}

          <aside className="flex w-64 shrink-0 flex-col gap-3 overflow-y-auto">
            <div>
              <label className="text-[10px] uppercase tracking-wider text-slate-500">
                Preview player
              </label>
              <select
                value={playerId ?? ""}
                onChange={(e) => setPlayerId(e.target.value === "" ? null : Number(e.target.value))}
                className="mt-0.5 w-full rounded border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
              >
                {teamPlayers.length === 0 && <option value="">No roster players</option>}
                {teamPlayers.map((p) => (
                  <option key={p.player_id} value={p.player_id}>
                    {[p.last_name, p.first_name].filter(Boolean).join(", ") || `#${p.player_id}`}
                    {p.jersey_number != null ? ` · #${p.jersey_number}` : ""}
                  </option>
                ))}
              </select>
              <div className="mt-1 text-[10px] leading-tight text-slate-500">
                {!fontRelPath ? (
                  <span className="text-amber-300/70">No font map for this kit — preview shows zones only.</span>
                ) : fontTex.isLoading ? (
                  "Decoding font atlas…"
                ) : fontMapRef.current ? (
                  <>
                    <span className="text-emerald-300">name</span> ·{" "}
                    <span className="text-amber-300">number</span> ·{" "}
                    <span className="text-violet-300">patch</span> zones rendered through the font map.
                    Set each zone's type below (mark the captain patch to show{" "}
                    {captainLetter ? `“${captainLetter}”` : "C/A"}). Drag zones to move them.
                  </>
                ) : (
                  <span className="text-amber-300/70">
                    Couldn't read the font atlas — preview shows zones only.
                  </span>
                )}
              </div>
            </div>

            <div className="text-[11px] text-slate-400">
              {dims ? `${dims.w} × ${dims.h}` : "—"}
              {referenceRelPath ? (
                <div className="text-slate-500">
                  Reference: jersey body {reference.isLoading ? "(loading…)" : reference.data ? "" : "(unavailable)"}
                </div>
              ) : (
                <div className="text-amber-300/70">No jersey body reference found.</div>
              )}
              <div className="mt-1 text-[10px] text-slate-500">
                Reference alignment is approximate — verify in-game.
              </div>
            </div>

            <div className="space-y-1">
              <div className="text-[10px] uppercase tracking-wider text-slate-500">
                Zones — set each to name / number / patch
              </div>
              {zones.map((el, i) => (
                <div
                  key={el.id}
                  className={
                    "flex items-center gap-1 rounded border px-1.5 py-1 text-[11px] " +
                    (el.id === selectedId
                      ? "border-sky-600 bg-sky-950/40 text-sky-100"
                      : "border-slate-800 bg-slate-950/40 text-slate-300")
                  }
                >
                  <button
                    type="button"
                    onClick={() => setSelectedId(el.id)}
                    className="min-w-0 flex-1 truncate text-left hover:text-sky-200"
                  >
                    Zone {i + 1}
                    <span className="ml-1 font-mono text-[10px] text-slate-500">
                      {Math.round(el.x)},{Math.round(el.y)}
                    </span>
                  </button>
                  <select
                    value={effType(el.id)}
                    onChange={(e) =>
                      setZoneTypes((prev) => ({ ...prev, [el.id]: e.target.value as ZoneType }))
                    }
                    className="rounded border border-slate-700 bg-slate-950 px-1 py-0.5 text-[10px] text-slate-100 focus:border-sky-600 focus:outline-none"
                  >
                    {(Object.keys(ZONE_TYPE_LABELS) as ZoneType[]).map((t) => (
                      <option key={t} value={t}>
                        {ZONE_TYPE_LABELS[t]}
                      </option>
                    ))}
                  </select>
                </div>
              ))}
              {zones.length === 0 && sheet.data && (
                <p className="text-[11px] text-slate-500">No zones detected in this template.</p>
              )}
            </div>

            <div>
              <div className="mb-1 text-[10px] uppercase tracking-wider text-slate-500">Move</div>
              <div className="flex flex-wrap gap-1">
                <NudgeBtn label="←" onClick={() => nudge(-1, 0)} disabled={!selected} />
                <NudgeBtn label="→" onClick={() => nudge(1, 0)} disabled={!selected} />
                <NudgeBtn label="↑" onClick={() => nudge(0, -1)} disabled={!selected} />
                <NudgeBtn label="↓" onClick={() => nudge(0, 1)} disabled={!selected} />
              </div>
            </div>

            <div>
              <div className="mb-1 text-[10px] uppercase tracking-wider text-slate-500">
                Size {selected ? `(${Math.round(selected.scale * 100)}%)` : ""}
              </div>
              <div className="flex gap-1">
                <button
                  type="button"
                  onClick={() => resize(1 / 1.1)}
                  disabled={!selected}
                  className="rounded border border-slate-700 px-2 py-1 text-[11px] text-slate-200 hover:bg-slate-800 disabled:opacity-40"
                >
                  − Smaller
                </button>
                <button
                  type="button"
                  onClick={() => resize(1.1)}
                  disabled={!selected}
                  className="rounded border border-slate-700 px-2 py-1 text-[11px] text-slate-200 hover:bg-slate-800 disabled:opacity-40"
                >
                  + Larger
                </button>
              </div>
            </div>

            <button
              type="button"
              onClick={deleteSelected}
              disabled={!selected}
              className="rounded border border-slate-700 px-2 py-1 text-[11px] text-slate-200 hover:bg-slate-800 disabled:opacity-40"
            >
              Delete selected zone
            </button>
          </aside>
        </div>

        <footer className="flex items-center justify-between gap-3 border-t border-slate-800 px-5 py-3">
          <div className="text-[11px]">
            {save.isError && (
              <span className="text-amber-300/80">{String(save.error)}</span>
            )}
            {save.isSuccess && !save.isPending && (
              <span className="text-emerald-300">Saved — a .bak was written.</span>
            )}
          </div>
          <div className="flex gap-2">
            <Button variant="secondary" onClick={onClose}>
              {save.isSuccess ? "Done" : "Cancel"}
            </Button>
            <Button onClick={handleSave} disabled={save.isPending || !dims || !!decodeError}>
              {save.isPending ? "Baking…" : "Save to texture"}
            </Button>
          </div>
        </footer>
      </div>
    </div>
  );
}

function NudgeBtn({
  label,
  onClick,
  disabled,
}: {
  label: string;
  onClick: () => void;
  disabled: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className="h-7 w-7 rounded border border-slate-700 text-xs text-slate-200 hover:bg-slate-800 disabled:opacity-40"
    >
      {label}
    </button>
  );
}
