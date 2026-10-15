# wrap_head.py — donor-head shrinkwrap morph (Path B), generalized.
#
# Invoked by the blender-job harness:
#   blender --background --factory-startup --python wrap_head.py -- request.json
#
# Contract: read the request JSON (op == "wrap_head"), morph the donor head onto the
# target geometry, write the .pv positions file (one "x y z 0 0" line per donor vertex,
# donor order), and — as the very last act — write response.json next to the request.
# Handled failures write {"status": "error", "message": ...} and exit 0; only unhandled
# crashes exit nonzero.
#
# The rules this script encodes were proven in-game (see the studio's
# MESH_IMPORT_EXCLUSIVE_PATH doc):
#   * coordinate frame is preserved end-to-end (Y-up, face +Z, meters) — vertex.co is
#     written verbatim, no axis remap;
#   * donor boundary loops (neck, mouth) are frozen with a BFS falloff ring band —
#     without it the wrap collapses eyes/nostrils/mouth/neck;
#   * the eyelid pocket is protected by a hard freeze radius around each eyeball center
#     (collapsing the pocket is unrecoverable; eyeballs themselves are never touched
#     here — they are separate submeshes kept verbatim);
#   * every tunable arrives in the request — the script holds no donor-specific numbers.

import json
import sys
import time
import traceback

import bpy
import bmesh
from mathutils import Vector


def arg_after_dashes():
    argv = sys.argv
    if "--" in argv:
        tail = argv[argv.index("--") + 1 :]
        if tail:
            return tail[0]
    raise RuntimeError("usage: blender --background --python wrap_head.py -- request.json")


def write_response(request_path, payload):
    import os

    out = os.path.join(os.path.dirname(os.path.abspath(request_path)), "response.json")
    with open(out, "w", encoding="utf-8") as f:
        json.dump(payload, f)


def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()


def import_obj(path):
    before = set(bpy.data.objects)
    bpy.ops.wm.obj_import(filepath=path)
    new = [o for o in bpy.data.objects if o not in before and o.type == "MESH"]
    if len(new) != 1:
        raise RuntimeError(f"expected exactly one mesh from {path}, got {len(new)}")
    return new[0]


def boundary_vertices(mesh_obj):
    """Indices of vertices on open boundary loops (edges with exactly one face)."""
    bm = bmesh.new()
    bm.from_mesh(mesh_obj.data)
    bm.verts.ensure_lookup_table()
    boundary = set()
    for e in bm.edges:
        if len(e.link_faces) == 1:
            boundary.add(e.verts[0].index)
            boundary.add(e.verts[1].index)
    bm.free()
    return boundary


def fill_eye_holes(target, y_lo, y_hi):
    """Fill boundary loops that sit in the eye band (centroid y in range, |x| off-axis).

    The neck/mouth holes are left open — they match the donor's own openings.
    Returns the number of loops filled.
    """
    bm = bmesh.new()
    bm.from_mesh(target.data)
    bm.verts.ensure_lookup_table()
    bm.edges.ensure_lookup_table()

    boundary_edges = [e for e in bm.edges if len(e.link_faces) == 1]
    # Group boundary edges into loops by walking shared vertices.
    edge_by_vert = {}
    for e in boundary_edges:
        for v in e.verts:
            edge_by_vert.setdefault(v.index, []).append(e)
    unvisited = set(boundary_edges)
    loops = []
    while unvisited:
        seed = unvisited.pop()
        loop = [seed]
        frontier = [seed]
        while frontier:
            e = frontier.pop()
            for v in e.verts:
                for nxt in edge_by_vert.get(v.index, []):
                    if nxt in unvisited:
                        unvisited.discard(nxt)
                        loop.append(nxt)
                        frontier.append(nxt)
        loops.append(loop)

    filled = 0
    fill_edges = []
    for loop in loops:
        verts = {v for e in loop for v in e.verts}
        centroid = sum((v.co for v in verts), Vector()) / max(len(verts), 1)
        if y_lo <= centroid.y <= y_hi and abs(centroid.x) > 0.01:
            fill_edges.extend(loop)
            filled += 1
    if fill_edges:
        bmesh.ops.holes_fill(bm, edges=list(set(fill_edges)), sides=0)
        bm.to_mesh(target.data)
        target.data.update()
    bm.free()
    return filled


def build_morph_group(donor, params):
    """Vertex group 'morph': BFS boundary falloff ∧ eye-pocket freeze.

    Returns (group, eye_frozen_count).
    """
    mesh = donor.data
    n = params["boundary_freeze_rings"]

    # BFS edge-distance from the donor's open boundary loops.
    adjacency = [[] for _ in mesh.vertices]
    for e in mesh.edges:
        a, b = e.vertices
        adjacency[a].append(b)
        adjacency[b].append(a)
    INF = 1 << 30
    dist = [INF] * len(mesh.vertices)
    frontier = list(boundary_vertices(donor))
    for v in frontier:
        dist[v] = 0
    while frontier:
        nxt = []
        for v in frontier:
            for w in adjacency[v]:
                if dist[w] > dist[v] + 1:
                    dist[w] = dist[v] + 1
                    nxt.append(w)
        frontier = nxt

    eye_l = Vector(params["eye_center_l"])
    eye_r = Vector(params["eye_center_r"])
    r_in = params["eye_freeze_inner_m"]
    r_out = params["eye_freeze_outer_m"]

    group = donor.vertex_groups.new(name="morph")
    eye_frozen = 0
    for v in mesh.vertices:
        boundary_w = min(dist[v.index] / n, 1.0) if dist[v.index] < INF else 1.0
        d_eye = min((v.co - eye_l).length, (v.co - eye_r).length)
        if d_eye <= r_in:
            eye_factor = 0.0
            eye_frozen += 1
        elif d_eye >= r_out:
            eye_factor = 1.0
        else:
            eye_factor = (d_eye - r_in) / (r_out - r_in)
        group.add([v.index], min(boundary_w, eye_factor), "REPLACE")
    return group, eye_frozen


def apply_modifier(obj, modifier_name):
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.modifier_apply(modifier=modifier_name)


def run(request):
    params = request["params"]
    t0 = time.time()

    clear_scene()
    donor = import_obj(request["donor_obj"])
    target = import_obj(request["target_obj"])

    expected = params["expected_vertex_count"]
    if len(donor.data.vertices) != expected:
        raise RuntimeError(
            f"donor vertex count {len(donor.data.vertices)} != expected {expected} — "
            "wrong donor file or a donor that has already been modified"
        )

    y_lo, y_hi = params["eye_hole_y_range"]
    filled = fill_eye_holes(target, y_lo, y_hi)

    if params["subdivision_levels"] > 0:
        mod = target.modifiers.new(name="wrap_subdiv", type="SUBSURF")
        mod.levels = params["subdivision_levels"]
        mod.render_levels = params["subdivision_levels"]
        apply_modifier(target, mod.name)

    _, eye_frozen = build_morph_group(donor, params)
    if eye_frozen == 0:
        raise RuntimeError(
            "eye-pocket freeze engaged zero vertices — eye centers do not match this "
            "donor (the eyelid pocket would collapse); check the donor profile"
        )

    shrink = donor.modifiers.new(name="wrap_shrink", type="SHRINKWRAP")
    shrink.target = target
    shrink.wrap_method = "NEAREST_SURFACEPOINT"
    shrink.vertex_group = "morph"
    apply_modifier(donor, shrink.name)

    smooth = donor.modifiers.new(name="wrap_smooth", type="SMOOTH")
    smooth.factor = params["smooth_factor"]
    smooth.iterations = params["smooth_iterations"]
    smooth.vertex_group = "morph"
    apply_modifier(donor, smooth.name)

    # Positions out, donor order, frame preserved. UV columns are dummies — the packer
    # keeps the donor's UV template (THE UV RULE: never transfer target UVs).
    with open(request["out_pv"], "w", encoding="utf-8") as f:
        for v in donor.data.vertices:
            f.write("%.6f %.6f %.6f 0 0\n" % (v.co.x, v.co.y, v.co.z))

    if request.get("preview_png"):
        render_preview(donor, request["preview_png"])

    return {
        "status": "ok",
        "message": f"filled {filled} eye hole loop(s)",
        "vertex_count": len(donor.data.vertices),
        "eye_frozen_vertices": eye_frozen,
        "elapsed_s": round(time.time() - t0, 3),
    }


def render_preview(donor, out_path):
    """Front-view verification render (eyeball/pocket sanity check)."""
    ys = [v.co.y for v in donor.data.vertices]
    zs = [v.co.z for v in donor.data.vertices]
    cam_data = bpy.data.cameras.new("preview_cam")
    cam_data.lens = 50
    cam = bpy.data.objects.new("preview_cam", cam_data)
    bpy.context.collection.objects.link(cam)
    cam.location = (0.0, min(ys) - 0.4, (min(zs) + max(zs)) / 2 + 0.05)
    cam.rotation_euler = (1.5707963, 0.0, 0.0)
    sun_data = bpy.data.lights.new("preview_sun", type="SUN")
    sun_data.energy = 4.0
    sun = bpy.data.objects.new("preview_sun", sun_data)
    bpy.context.collection.objects.link(sun)
    scene = bpy.context.scene
    scene.camera = cam
    scene.render.resolution_x = 480
    scene.render.resolution_y = 600
    scene.render.filepath = out_path
    bpy.ops.render.render(write_still=True)


def main():
    request_path = arg_after_dashes()
    try:
        with open(request_path, "r", encoding="utf-8") as f:
            request = json.load(f)
        if request.get("op") != "wrap_head":
            raise RuntimeError(f"unsupported op {request.get('op')!r}")
        response = run(request)
    except Exception as exc:  # noqa: BLE001 — everything becomes a typed job failure
        traceback.print_exc()
        write_response(request_path, {"status": "error", "message": str(exc)})
        return
    write_response(request_path, response)


main()
