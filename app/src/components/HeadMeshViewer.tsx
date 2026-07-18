// 3D head-mesh viewer with per-submesh visibility toggles.
//
// Purpose: identify what each submesh of a player head is (head / hair / eyes /
// mouth interior) so we can work out how to handle NHL 10's meshes. Adapted from
// the prior app's HeadMeshPreview (react-three-fiber + drei), pared to what
// identification needs:
//   * renders RIGID — positions are already in bind-pose model space, so no
//     skinning, no bones, no poses (which also sidesteps NHL 10's ambiguous
//     per-submesh rig entirely);
//   * each submesh has a visibility checkbox and can be shown textured or in a
//     flat per-submesh colour, so even untextured parts (eyes) are distinct;
//   * a hockey-rink backdrop sits behind the transparent canvas.

import { useMemo, useRef, useState, useEffect } from "react";
import { Canvas } from "@react-three/fiber";
import { OrbitControls } from "@react-three/drei";
import * as THREE from "three";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { Spinner, ErrorBanner, Pill } from "./primitives";
import type { AssetRootsArgsDto } from "../bindings/AssetRootsArgsDto";
import type { HeadPreviewDto } from "../bindings/HeadPreviewDto";
import type { PreviewSubmeshDto } from "../bindings/PreviewSubmeshDto";
import rinkBackground from "../assets/rink-background.png";

/** Distinct hues per submesh index for "flat colour" mode — so eyes, mouthbag
 *  and the head read apart at a glance regardless of texture. */
const SUBMESH_COLORS = [
  "#38bdf8", // sky
  "#f472b6", // pink
  "#a3e635", // lime
  "#fbbf24", // amber
  "#c084fc", // violet
  "#fb7185", // rose
  "#2dd4bf", // teal
  "#facc15", // yellow
];

export function HeadMeshViewer({
  roots,
  headId,
}: {
  roots: AssetRootsArgsDto;
  headId: number;
}) {
  const preview = useQuery({
    queryKey: ["headPreview", roots.install_dir, headId],
    queryFn: () => call("head_preview", { roots, headId }),
  });

  if (preview.isLoading) {
    return (
      <ViewerFrame>
        <div className="grid h-full w-full place-items-center">
          <Spinner label="Decoding head mesh…" />
        </div>
      </ViewerFrame>
    );
  }
  if (preview.error) {
    return (
      <ViewerFrame>
        <div className="grid h-full w-full place-items-center p-4">
          <ErrorBanner
            title="Couldn't load this head mesh"
            message={String(preview.error)}
          />
        </div>
      </ViewerFrame>
    );
  }
  if (!preview.data) return null;
  return <LoadedViewer data={preview.data} />;
}

/** The rink-backed canvas box (used for loading / error / loaded states). */
function ViewerFrame({ children }: { children: React.ReactNode }) {
  return (
    <div
      className="relative h-[26rem] w-full overflow-hidden rounded-md border border-slate-800 bg-slate-950"
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

function LoadedViewer({ data }: { data: HeadPreviewDto }) {
  const [visible, setVisible] = useState<boolean[]>(() =>
    data.submeshes.map(() => true),
  );
  const [textured, setTextured] = useState(true);
  const [wireframe, setWireframe] = useState(false);

  // A new head (different submesh set) resets visibility.
  useEffect(() => setVisible(data.submeshes.map(() => true)), [data]);

  const totalTris = data.submeshes.reduce((a, s) => a + s.triangles.length, 0);

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5 text-[11px]">
          <Toggle label="Textured" checked={textured} onChange={setTextured} />
          <Toggle label="Wireframe" checked={wireframe} onChange={setWireframe} />
        </div>
        {data.lenient_rig && (
          <Pill tone="warn">rig unresolved — geometry only</Pill>
        )}
      </div>

      <ViewerFrame>
        <HeadCanvas
          data={data}
          visible={visible}
          textured={textured}
          wireframe={wireframe}
        />
      </ViewerFrame>

      <div className="rounded-md border border-slate-800 bg-slate-900/50 p-2">
        <div className="mb-1 flex items-center justify-between">
          <span className="text-[10px] font-semibold uppercase tracking-wide text-slate-500">
            Submeshes — toggle to identify
          </span>
          <div className="flex gap-2 text-[10px]">
            <button
              className="text-sky-400 hover:text-sky-300"
              onClick={() => setVisible(data.submeshes.map(() => true))}
            >
              All
            </button>
            <button
              className="text-sky-400 hover:text-sky-300"
              onClick={() => setVisible(data.submeshes.map(() => false))}
            >
              None
            </button>
          </div>
        </div>
        <ul className="space-y-0.5">
          {data.submeshes.map((sm, i) => (
            <li key={`${sm.name}-${i}`}>
              <label className="flex cursor-pointer items-center gap-2 rounded px-1 py-0.5 text-xs hover:bg-slate-800/60">
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
                />
                {/* Colour chip matches the flat-colour render so a submesh is
                    findable in both places. */}
                <span
                  className="inline-block h-2.5 w-2.5 shrink-0 rounded-sm"
                  style={{ background: SUBMESH_COLORS[i % SUBMESH_COLORS.length] }}
                />
                <span className="font-mono text-slate-300">
                  {cleanName(sm.name)}
                </span>
                <span className="ml-auto font-mono text-[10px] text-slate-500">
                  {sm.positions.length.toLocaleString()}v ·{" "}
                  {sm.triangles.length.toLocaleString()}t
                  {sm.material_index === null ? " · untextured" : ""}
                  {sm.transparent ? " · alpha" : ""}
                </span>
              </label>
            </li>
          ))}
        </ul>
      </div>

      <div className="text-center font-mono text-[10px] text-slate-500">
        {data.submeshes.length} submeshes · {totalTris.toLocaleString()} tris ·{" "}
        {data.textures.length} maps · drag to rotate, scroll to zoom
      </div>
    </div>
  );
}

/** Strip the rx2 name-table `.Mesh` suffix for display (`head_.Mesh` → `head_`). */
function cleanName(name: string): string {
  return name.replace(/\.Mesh$/i, "");
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
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="h-3 w-3 accent-sky-500"
      />
      {label}
    </label>
  );
}

function HeadCanvas({
  data,
  visible,
  textured,
  wireframe,
}: {
  data: HeadPreviewDto;
  visible: boolean[];
  textured: boolean;
  wireframe: boolean;
}) {
  const cameraDistance = data.span * 2.5;
  const maps = useMemo(
    () =>
      data.textures.map((t) =>
        buildRgbaTexture(t.width, t.height, t.rgba_base64),
      ),
    [data.textures],
  );
  useEffect(() => () => maps.forEach((m) => m.dispose()), [maps]);

  return (
    <Canvas
      style={{ background: "transparent", height: "100%", width: "100%" }}
      gl={{ antialias: true }}
      camera={{
        position: [
          data.center[0],
          data.center[1],
          data.center[2] + cameraDistance,
        ],
        fov: 40,
        near: cameraDistance * 0.001,
        far: cameraDistance * 50,
      }}
      dpr={[1, 2]}
    >
      <ambientLight intensity={0.5} />
      <directionalLight position={[1, 2, 2]} intensity={0.7} />
      <directionalLight position={[-1.5, 0.5, 1]} intensity={0.45} />
      <directionalLight position={[0, -1, -1.5]} intensity={0.2} />
      {data.submeshes.map((sm, i) =>
        (visible[i] ?? true) ? (
          <SubmeshMesh
            key={`${sm.name}-${i}`}
            submesh={sm}
            map={
              textured && sm.material_index !== null
                ? (maps[sm.material_index] ?? null)
                : null
            }
            flatColor={
              textured ? null : SUBMESH_COLORS[i % SUBMESH_COLORS.length]
            }
            wireframe={wireframe}
          />
        ) : null,
      )}
      <OrbitControls
        target={data.center}
        enablePan={false}
        minDistance={data.span * 0.5}
        maxDistance={data.span * 10}
      />
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

  // Toggling map null↔texture changes the compiled shader; three only
  // recompiles on needsUpdate, which R3F doesn't set for the map prop alone.
  const matRef = useRef<THREE.MeshStandardMaterial>(null);
  useEffect(() => {
    if (matRef.current) matRef.current.needsUpdate = true;
  }, [map, flatColor]);

  const alphaProps =
    submesh.transparent && !wireframe
      ? { alphaTest: 0.5, alphaToCoverage: true }
      : {};

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
function buildRgbaTexture(
  width: number,
  height: number,
  rgbaBase64: string,
): THREE.DataTexture {
  const bin = atob(rgbaBase64);
  const rgba = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) rgba[i] = bin.charCodeAt(i);
  const tex = new THREE.DataTexture(
    rgba,
    width,
    height,
    THREE.RGBAFormat,
    THREE.UnsignedByteType,
  );
  tex.flipY = false;
  tex.minFilter = THREE.LinearFilter;
  tex.magFilter = THREE.LinearFilter;
  tex.needsUpdate = true;
  return tex;
}
