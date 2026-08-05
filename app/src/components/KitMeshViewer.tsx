// 3D full-kit viewer — renders a hockey player's uniform pieces (head, helmet,
// jersey, gloves, pants, socks, skates) assembled together in the shared
// bind-pose model space, so the operator sees the whole outfit as one figure.
//
// Sibling to HeadMeshViewer (react-three-fiber + drei), with two differences:
//   * it draws MANY pieces at once (each an independent BufferGeometry) rather
//     than one head's submeshes — the backend already laid them into a common
//     model space, so no per-piece transform is applied here;
//   * visibility toggles are per PIECE (Jersey / Gloves / …), the natural unit
//     for a uniform, not per raw submesh.
// Renders rigid (bind pose) — skeletal re-posing is a later, separate step.

import { useEffect, useMemo, useRef, useState } from "react";
import { Canvas, useThree } from "@react-three/fiber";
import { OrbitControls } from "@react-three/drei";
import * as THREE from "three";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { Spinner, ErrorBanner } from "./primitives";
import type { KitPreviewDto } from "../bindings/KitPreviewDto";
import type { KitPieceDto } from "../bindings/KitPieceDto";
import type { KitPieceInputDto } from "../bindings/KitPieceInputDto";
import type { PreviewSubmeshDto } from "../bindings/PreviewSubmeshDto";
import type { AssetRootsArgsDto } from "../bindings/AssetRootsArgsDto";
import rinkBackground from "../assets/rink-background.png";

/** Distinct hue per piece for "flat colour" mode, keyed by kind so the same
 *  garment reads the same colour across kits. */
const PIECE_COLORS: Record<string, string> = {
  head: "#f6c89f",
  helmet: "#38bdf8",
  jersey: "#f472b6",
  gloves: "#a3e635",
  pants: "#fbbf24",
  socks: "#c084fc",
  skates: "#2dd4bf",
  // Goalie gear.
  mask: "#38bdf8",
  cage: "#64748b",
  pads: "#f59e0b",
  blocker: "#a3e635",
  trapper: "#84cc16",
};
const FALLBACK_COLOR = "#94a3b8";

export function KitMeshViewer({
  installDir,
  pieces,
}: {
  installDir: string;
  pieces: KitPieceInputDto[];
}) {
  const preview = useQuery({
    queryKey: ["kitPreview", installDir, pieces],
    queryFn: () => call("kit_preview_paths", { installDir, pieces }),
    enabled: pieces.length > 0,
    staleTime: Infinity,
    retry: false,
  });

  if (pieces.length === 0) {
    return (
      <ViewerFrame>
        <div className="grid h-full w-full place-items-center p-4 text-center text-xs text-slate-400">
          No kit pieces resolved to preview.
        </div>
      </ViewerFrame>
    );
  }
  if (preview.isLoading) {
    return (
      <ViewerFrame>
        <div className="grid h-full w-full place-items-center">
          <Spinner label="Assembling kit…" />
        </div>
      </ViewerFrame>
    );
  }
  if (preview.error) {
    return (
      <ViewerFrame>
        <div className="grid h-full w-full place-items-center p-4">
          <ErrorBanner title="Couldn't assemble this kit" message={String(preview.error)} />
        </div>
      </ViewerFrame>
    );
  }
  if (!preview.data) return null;
  return <LoadedKit data={preview.data} />;
}

/** Team-driven kit viewer: resolves the selected kit's colorway + a
 *  representative player's head + glove/helmet server-side and renders the
 *  assembled figure, with all controls (head picker, view toggles, piece list)
 *  in a side column so the 3D window fills the available height. */
export function TeamKitViewer({
  projectId,
  roots,
  teamId,
  style,
  variant,
  kitKey,
}: {
  projectId: number;
  roots: AssetRootsArgsDto;
  teamId: number;
  style: number;
  variant: number;
  /** Changing kit resets the head pick back to the default. */
  kitKey: string;
}) {
  const [playerId, setPlayerId] = useState<number | null>(null);
  useEffect(() => setPlayerId(null), [kitKey]);

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

  const preview = useQuery({
    queryKey: ["teamKitPreview", projectId, teamId, style, variant, playerId ?? null],
    queryFn: () =>
      call("team_kit_preview", {
        projectId,
        roots,
        teamId,
        style,
        variant,
        playerId: playerId ?? null,
      }),
    staleTime: Infinity,
    retry: false,
  });

  const picker = (
    <label className="flex items-center gap-2 text-[11px] text-slate-400">
      <span className="shrink-0 font-semibold uppercase tracking-wide text-slate-500">
        Head / equipment
      </span>
      <select
        value={playerId ?? ""}
        onChange={(e) => setPlayerId(e.target.value === "" ? null : Number(e.target.value))}
        className="max-w-[16rem] rounded border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
      >
        <option value="">Top-overall player (default)</option>
        {teamPlayers.map((p) => (
          <option key={p.player_id} value={p.player_id}>
            {[p.last_name, p.first_name].filter(Boolean).join(", ") || `#${p.player_id}`}
            {p.overall != null ? ` · ${p.overall} OVR` : ""}
          </option>
        ))}
      </select>
    </label>
  );

  if (preview.isLoading) {
    return (
      <ViewerFrame>
        <div className="grid h-full w-full place-items-center">
          <Spinner label="Assembling kit…" />
        </div>
      </ViewerFrame>
    );
  }
  if (preview.error) {
    return (
      <ViewerFrame>
        <div className="grid h-full w-full place-items-center p-4">
          <ErrorBanner title="Couldn't assemble this kit" message={String(preview.error)} />
        </div>
      </ViewerFrame>
    );
  }
  if (!preview.data) return null;
  return <LoadedKit data={preview.data} pickerNode={picker} />;
}

/** The rink-backed canvas box (shared across loading / error / loaded states). */
function ViewerFrame({ children }: { children: React.ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);
  // Middle-button drag pans the camera — but the browser's default middle-click
  // action is autoscroll, which drags the whole page's scrollbar along with it.
  // Cancel that default (natively, non-passive) so panning stays inside the
  // viewer. Right-button also pans, so suppress its context menu too.
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const onDown = (e: MouseEvent) => {
      if (e.button === 1) e.preventDefault();
    };
    el.addEventListener("mousedown", onDown, { passive: false });
    return () => el.removeEventListener("mousedown", onDown);
  }, []);
  return (
    <div
      ref={ref}
      onContextMenu={(e) => e.preventDefault()}
      className="relative h-full min-h-[30rem] w-full overflow-hidden rounded-md border border-slate-800 bg-slate-950"
      style={{
        backgroundImage: `url("${rinkBackground}")`,
        backgroundPosition: "center",
        backgroundSize: "cover",
      }}
    >
      {children}
    </div>
  );
}

function LoadedKit({ data, pickerNode }: { data: KitPreviewDto; pickerNode?: React.ReactNode }) {
  const [visible, setVisible] = useState<boolean[]>(() => data.pieces.map(() => true));
  const [textured, setTextured] = useState(true);
  const [wireframe, setWireframe] = useState(false);
  // Which piece the camera is framed on (null = the whole figure). Clicking a
  // piece name zooms to it; clicking it again returns to the full figure.
  const [focusIndex, setFocusIndex] = useState<number | null>(null);

  // A new kit (different piece set) resets visibility and framing.
  useEffect(() => {
    setVisible(data.pieces.map(() => true));
    setFocusIndex(null);
  }, [data]);

  const totalTris = data.pieces.reduce(
    (a, p) => a + p.submeshes.reduce((b, s) => b + s.triangles.length, 0),
    0,
  );

  return (
    // Controls in a fixed side column; the 3D window fills the rest and the
    // Window on top (fills the height), controls laid out in a row beneath it —
    // both the same width, expanding with the preview column.
    <div className="flex h-full min-h-[34rem] flex-col gap-2">
      <div className="min-h-0 flex-1">
        <ViewerFrame>
          <KitCanvas
            data={data}
            visible={visible}
            textured={textured}
            wireframe={wireframe}
            focusIndex={focusIndex}
          />
        </ViewerFrame>
      </div>

      <div className="shrink-0 space-y-2">
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5">
          {pickerNode}
          <div className="flex items-center gap-1.5 text-[11px]">
            <Toggle label="Textured" checked={textured} onChange={setTextured} />
            <Toggle label="Wireframe" checked={wireframe} onChange={setWireframe} />
          </div>
          <div className="flex items-center gap-2 text-[10px] text-slate-500">
            <span className="font-semibold uppercase tracking-wide">Pieces</span>
            <button
              className="text-sky-400 hover:text-sky-300"
              onClick={() => setVisible(data.pieces.map(() => true))}
            >
              All
            </button>
            <button
              className="text-sky-400 hover:text-sky-300"
              onClick={() => setVisible(data.pieces.map(() => false))}
            >
              None
            </button>
          </div>
          <span className="ml-auto font-mono text-[10px] leading-tight text-slate-500">
            {data.pieces.length} pieces · {totalTris.toLocaleString()} tris · {data.textures.length}{" "}
            maps · left-drag rotate · middle-drag move · scroll zoom
          </span>
        </div>

        {/* Piece toggles / zoom — wrap horizontally so they take few rows. */}
        <ul className="flex flex-wrap gap-x-3 gap-y-1">
          {data.pieces.map((p, i) => {
            const focused = focusIndex === i;
            const untextured = p.submeshes.every((s) => s.material_index === null);
            return (
              <li
                key={`${p.kind}-${i}`}
                className="flex items-center gap-1.5 rounded border border-slate-800 bg-slate-900/50 px-1.5 py-0.5 text-xs"
              >
                <input
                  type="checkbox"
                  checked={visible[i] ?? true}
                  onChange={(e) =>
                    setVisible((v) => {
                      const next = [...v];
                      next[i] = e.target.checked;
                      return next;
                    })
                  }
                  className="cursor-pointer"
                  aria-label={`Show ${p.label}`}
                />
                <span
                  className="inline-block h-2.5 w-2.5 shrink-0 rounded-sm"
                  style={{ background: PIECE_COLORS[p.kind] ?? FALLBACK_COLOR }}
                />
                <button
                  type="button"
                  onClick={() => setFocusIndex((f) => (f === i ? null : i))}
                  title={focused ? "Click again to frame the whole figure" : `Zoom to ${p.label}`}
                  className={
                    "font-medium transition-colors " +
                    (focused ? "text-sky-300" : "text-slate-200 hover:text-sky-300")
                  }
                >
                  {p.label}
                </button>
                {untextured ? <span className="text-[9px] text-slate-600">untex</span> : null}
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}

/** Bounding center + longest-edge span of one piece, from its vertex positions. */
function pieceBounds(piece: KitPieceDto): { center: [number, number, number]; span: number } {
  const lo = [Infinity, Infinity, Infinity];
  const hi = [-Infinity, -Infinity, -Infinity];
  for (const s of piece.submeshes) {
    for (const p of s.positions) {
      for (let k = 0; k < 3; k++) {
        lo[k] = Math.min(lo[k], p[k]);
        hi[k] = Math.max(hi[k], p[k]);
      }
    }
  }
  if (!Number.isFinite(lo[0])) return { center: [0, 0, 0], span: 1 };
  return {
    center: [(lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2],
    span: Math.max(hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]) || 1,
  };
}

/** Structural view of OrbitControls we touch (target + update). */
type OrbitLike = { target: THREE.Vector3; update: () => void };

/** Reframe the camera onto a target point at a given distance, preserving the
 *  current view direction — driven by `(cx, cy, cz, distance)` so it only fires
 *  when the focus actually changes (not on unrelated re-renders like toggles). */
function CameraFocus({
  cx,
  cy,
  cz,
  distance,
}: {
  cx: number;
  cy: number;
  cz: number;
  distance: number;
}) {
  const camera = useThree((s) => s.camera);
  const controls = useThree((s) => s.controls) as unknown as OrbitLike | null;
  useEffect(() => {
    if (!controls) return;
    const tgt = new THREE.Vector3(cx, cy, cz);
    const dir = new THREE.Vector3().subVectors(camera.position, controls.target);
    if (dir.lengthSq() < 1e-8) dir.set(0, 0, 1);
    dir.normalize();
    camera.position.copy(tgt).addScaledVector(dir, distance);
    controls.target.copy(tgt);
    controls.update();
  }, [controls, camera, cx, cy, cz, distance]);
  return null;
}

function KitCanvas({
  data,
  visible,
  textured,
  wireframe,
  focusIndex,
}: {
  data: KitPreviewDto;
  visible: boolean[];
  textured: boolean;
  wireframe: boolean;
  focusIndex: number | null;
}) {
  const cameraDistance = data.span * 2.2;
  // Focus target: a chosen piece, or the whole figure. Distance scales to the
  // focused piece so small parts (helmet) zoom in close, the figure frames wide.
  const focus =
    focusIndex != null && data.pieces[focusIndex]
      ? pieceBounds(data.pieces[focusIndex])
      : { center: data.center, span: data.span };
  const focusDistance = focus.span * 2.2;
  const maps = useMemo(
    () => data.textures.map((t) => buildRgbaTexture(t.width, t.height, t.rgba_base64)),
    [data.textures],
  );
  useEffect(() => () => maps.forEach((m) => m.dispose()), [maps]);

  return (
    <Canvas
      style={{ background: "transparent", height: "100%", width: "100%" }}
      gl={{ antialias: true }}
      camera={{
        position: [data.center[0], data.center[1], data.center[2] + cameraDistance],
        fov: 40,
        near: cameraDistance * 0.001,
        far: cameraDistance * 50,
      }}
      dpr={[1, 2]}
    >
      <ambientLight intensity={0.55} />
      <directionalLight position={[1, 2, 2]} intensity={0.7} />
      <directionalLight position={[-1.5, 0.5, 1]} intensity={0.45} />
      <directionalLight position={[0, -1, -1.5]} intensity={0.2} />
      {data.pieces.map((piece, pi) =>
        (visible[pi] ?? true)
          ? piece.submeshes.map((sm, si) => (
              <SubmeshMesh
                key={`${piece.kind}-${pi}-${sm.name}-${si}`}
                submesh={sm}
                map={
                  textured && sm.material_index !== null
                    ? (maps[sm.material_index] ?? null)
                    : null
                }
                flatColor={textured ? null : (PIECE_COLORS[piece.kind] ?? FALLBACK_COLOR)}
                wireframe={wireframe}
              />
            ))
          : null,
      )}
      <OrbitControls
        makeDefault
        target={data.center}
        enablePan
        screenSpacePanning
        minDistance={data.span * 0.15}
        maxDistance={data.span * 10}
        // Middle-button (scroll-wheel) drag moves the camera vertically and
        // strafes horizontally (pan) instead of dollying; scroll still zooms.
        mouseButtons={{
          LEFT: THREE.MOUSE.ROTATE,
          MIDDLE: THREE.MOUSE.PAN,
          RIGHT: THREE.MOUSE.PAN,
        }}
      />
      <CameraFocus cx={focus.center[0]} cy={focus.center[1]} cz={focus.center[2]} distance={focusDistance} />
    </Canvas>
  );
}

function SubmeshMesh({
  submesh,
  map,
  flatColor,
  wireframe,
}: {
  submesh: PreviewSubmeshDto;
  map: THREE.DataTexture | null;
  flatColor: string | null;
  wireframe: boolean;
}) {
  const geometry = useMemo(() => buildGeometry(submesh), [submesh]);
  useEffect(() => () => geometry.dispose(), [geometry]);

  // Toggling map null↔texture changes the compiled shader; three only recompiles
  // on needsUpdate, which R3F doesn't set for the map prop alone.
  const matRef = useRef<THREE.MeshStandardMaterial>(null);
  useEffect(() => {
    if (matRef.current) matRef.current.needsUpdate = true;
  }, [map, flatColor]);

  const alphaProps =
    submesh.transparent && !wireframe ? { alphaTest: 0.5, alphaToCoverage: true } : {};

  return (
    <mesh geometry={geometry} renderOrder={submesh.render_order}>
      <meshStandardMaterial
        ref={matRef}
        map={map ?? undefined}
        color={flatColor ?? "#ffffff"}
        side={THREE.DoubleSide}
        roughness={0.6}
        metalness={0}
        wireframe={wireframe}
        {...alphaProps}
      />
    </mesh>
  );
}

function buildGeometry(sm: PreviewSubmeshDto): THREE.BufferGeometry {
  const geom = new THREE.BufferGeometry();
  const n = sm.positions.length;
  const positions = new Float32Array(n * 3);
  const normals = new Float32Array(n * 3);
  const uv = new Float32Array(n * 2);
  for (let i = 0; i < n; i++) {
    positions[i * 3] = sm.positions[i][0];
    positions[i * 3 + 1] = sm.positions[i][1];
    positions[i * 3 + 2] = sm.positions[i][2];
    normals[i * 3] = sm.normals[i][0];
    normals[i * 3 + 1] = sm.normals[i][1];
    normals[i * 3 + 2] = sm.normals[i][2];
    uv[i * 2] = sm.uv1[i][0];
    uv[i * 2 + 1] = sm.uv1[i][1];
  }
  const indices = new Uint32Array(sm.triangles.length * 3);
  for (let i = 0; i < sm.triangles.length; i++) {
    indices[i * 3] = sm.triangles[i][0];
    indices[i * 3 + 1] = sm.triangles[i][1];
    indices[i * 3 + 2] = sm.triangles[i][2];
  }
  geom.setAttribute("position", new THREE.BufferAttribute(positions, 3));
  geom.setAttribute("normal", new THREE.BufferAttribute(normals, 3));
  geom.setAttribute("uv", new THREE.BufferAttribute(uv, 2));
  geom.setIndex(new THREE.BufferAttribute(indices, 1));
  geom.computeBoundingSphere();
  return geom;
}

/** Base64 RGBA8 → DataTexture (flipY=false preserves the decoded row order). */
function buildRgbaTexture(width: number, height: number, rgbaBase64: string): THREE.DataTexture {
  const bin = atob(rgbaBase64);
  const rgba = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) rgba[i] = bin.charCodeAt(i);
  const tex = new THREE.DataTexture(rgba, width, height, THREE.RGBAFormat, THREE.UnsignedByteType);
  tex.flipY = false;
  tex.minFilter = THREE.LinearFilter;
  tex.magFilter = THREE.LinearFilter;
  tex.needsUpdate = true;
  return tex;
}

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex cursor-pointer items-center gap-1 rounded border border-slate-700 bg-slate-900 px-1.5 py-0.5 text-slate-300">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      {label}
    </label>
  );
}
