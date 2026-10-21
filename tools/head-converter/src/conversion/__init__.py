"""Claude-owned conversion modules (numpy only; Blender-dependent drivers live in experiments/claude).

Frame convention for every function here: metres, Z up, -Y forward (Blender), +X = character's left.
Native NHL (x, y_up, z_fwd) <-> Blender is handled by `geometry.native_to_blender` / `blender_to_native`.
"""
