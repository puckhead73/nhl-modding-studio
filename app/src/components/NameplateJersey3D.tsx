// Small 3D jersey preview for the nameplate editor: renders the jersey mesh with
// a LIVE canvas texture (the jersey colour map + the player's name/number
// composited at the zone UVs), so dragging a zone updates the wrap in real time.

import { useEffect, useMemo, useRef } from "react";
import { Canvas } from "@react-three/fiber";
import { OrbitControls } from "@react-three/drei";
import * as THREE from "three";
import type { PreviewSubmeshDto } from "../bindings/PreviewSubmeshDto";

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

export function NameplateJersey3D({
  submeshes,
  textureCanvas,
  tick,
  center,
  span,
}: {
  submeshes: PreviewSubmeshDto[];
  /** The live jersey texture (colour map + composited glyphs). */
  textureCanvas: HTMLCanvasElement | null;
  /** Bumped whenever `textureCanvas` is redrawn, to refresh the GPU texture. */
  tick: number;
  center: [number, number, number];
  span: number;
}) {
  const texture = useMemo(() => {
    if (!textureCanvas) return null;
    const t = new THREE.CanvasTexture(textureCanvas);
    t.flipY = false; // match the decoded-texture row order the UVs expect
    t.colorSpace = THREE.SRGBColorSpace;
    return t;
  }, [textureCanvas]);
  useEffect(() => {
    if (texture) texture.needsUpdate = true;
  }, [texture, tick]);
  useEffect(() => () => texture?.dispose(), [texture]);

  const geoms = useMemo(() => submeshes.map(buildGeometry), [submeshes]);
  useEffect(() => () => geoms.forEach((g) => g.dispose()), [geoms]);

  // Flipping `map` from undefined (texture not ready on first mount) to a real
  // texture changes the compiled shader; three only recompiles when the material
  // is flagged, which R3F doesn't do for the `map` prop alone. Without this the
  // jersey renders white until a remount (the old off/on-toggle workaround).
  const matRefs = useRef<(THREE.MeshStandardMaterial | null)[]>([]);
  useEffect(() => {
    for (const m of matRefs.current) if (m) m.needsUpdate = true;
  }, [texture]);

  const dist = span * 2.2;
  return (
    <Canvas
      camera={{
        position: [center[0], center[1], center[2] + dist],
        fov: 40,
        near: dist * 0.01,
        far: dist * 50,
      }}
      dpr={[1, 2]}
      style={{ width: "100%", height: "100%" }}
    >
      <ambientLight intensity={0.7} />
      <directionalLight position={[1, 2, 2]} intensity={0.6} />
      <directionalLight position={[-1.5, 0.5, 1]} intensity={0.4} />
      {geoms.map((g, i) => (
        <mesh key={i} geometry={g}>
          <meshStandardMaterial
            ref={(m) => {
              matRefs.current[i] = m;
            }}
            map={texture ?? undefined}
            color="#ffffff"
            side={THREE.DoubleSide}
            roughness={0.7}
            metalness={0}
          />
        </mesh>
      ))}
      <OrbitControls target={center} enablePan={false} minDistance={span * 0.4} maxDistance={span * 8} />
    </Canvas>
  );
}
