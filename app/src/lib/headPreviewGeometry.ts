import * as THREE from "three";

/** Smooth triangle-soup previews without welding native IDs or UV seams. */
export function ensurePreviewNormals(geometry: THREE.BufferGeometry, smooth: boolean) {
  if (geometry.getAttribute("normal")) return;
  if (!smooth) { geometry.computeVertexNormals(); return; }
  const p = geometry.getAttribute("position");
  const groups = new Map<string, THREE.Vector3>();
  const sums: THREE.Vector3[] = [];
  for (let i = 0; i < p.count; i++) {
    const key = `${p.getX(i)},${p.getY(i)},${p.getZ(i)}`;
    let sum = groups.get(key);
    if (!sum) { sum = new THREE.Vector3(); groups.set(key, sum); }
    sums.push(sum);
  }
  const index = geometry.index;
  const a = new THREE.Vector3(), b = new THREE.Vector3(), c = new THREE.Vector3();
  for (let i = 0; i < (index?.count ?? p.count); i += 3) {
    const ids = [0, 1, 2].map(j => index ? index.getX(i + j) : i + j);
    a.fromBufferAttribute(p, ids[0]); b.fromBufferAttribute(p, ids[1]); c.fromBufferAttribute(p, ids[2]);
    // Unnormalized cross products give area-weighted normals across coincident records.
    b.sub(a).cross(c.sub(a));
    for (const id of ids) sums[id].add(b);
  }
  for (const sum of groups.values()) sum.normalize();
  const normals = new Float32Array(p.count * 3);
  sums.forEach((sum, i) => sum.toArray(normals, i * 3));
  geometry.setAttribute("normal", new THREE.BufferAttribute(normals, 3));
}

/** Native eyes can occupy negative UV tiles; clamp-to-edge hides an iris. */
export function configureEyeSampler(texture: THREE.Texture, eye: boolean) {
  if (eye) { texture.wrapS = THREE.RepeatWrapping; texture.wrapT = THREE.RepeatWrapping; }
}
