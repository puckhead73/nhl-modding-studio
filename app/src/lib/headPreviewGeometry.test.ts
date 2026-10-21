import { describe, it, expect } from "vitest";
import * as THREE from "three";
import { ensurePreviewNormals, configureEyeSampler } from "./headPreviewGeometry";

describe("head preview geometry", () => {
  it("smooths split triangle records without changing UV seams, positions or indices", () => {
    const g = new THREE.BufferGeometry();
    g.setAttribute("position", new THREE.Float32BufferAttribute([
      0,0,0, 1,0,0, 0,1,0,  0,0,0, 0,1,0, -1,0,1,
    ], 3));
    g.setAttribute("uv", new THREE.Float32BufferAttribute([0,0, 1,0, 0,1, 1,1, 1,0, 0,0], 2));
    g.setIndex([0,1,2,3,4,5]);
    const before = [g.getAttribute("position").array.slice(), g.getAttribute("uv").array.slice(), g.index!.array.slice()];
    ensurePreviewNormals(g, true);
    const n = g.getAttribute("normal");
    expect(new THREE.Vector3().fromBufferAttribute(n, 0).toArray()).toEqual(new THREE.Vector3().fromBufferAttribute(n, 3).toArray());
    expect(n.getX(0)).toBeGreaterThan(0); // Blended with the tilted neighbouring face.
    expect(new THREE.Vector3().fromBufferAttribute(n, 0).length()).toBeCloseTo(1);
    expect([g.getAttribute("position").array, g.getAttribute("uv").array, g.index!.array]).toEqual(before);
  });
  it("keeps authored normals unchanged", () => {
    const g = new THREE.BufferGeometry();
    const normal = new THREE.Float32BufferAttribute([0,1,0], 3);
    g.setAttribute("normal", normal); ensurePreviewNormals(g, true);
    expect(g.getAttribute("normal")).toBe(normal);
  });
  it("samples both the normal and negative eye UV tiles instead of clamping one iris", () => {
    const eye = new THREE.Texture(); eye.flipY = false;
    configureEyeSampler(eye, true);
    expect(eye.transformUv(new THREE.Vector2(-1.75, .5)).toArray()).toEqual([.25,.5]);
    expect(eye.transformUv(new THREE.Vector2(.25, .5)).toArray()).toEqual([.25,.5]);
    const skin = new THREE.Texture(); configureEyeSampler(skin, false);
    expect(skin.wrapS).toBe(THREE.ClampToEdgeWrapping);
  });
});
