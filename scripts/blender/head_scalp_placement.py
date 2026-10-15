"""Place an already-corresponded head in Blender's metre, Z-up frame.

Neck limits belong to the destination template, never to a universal game rule.
Native vertex order and the destination's lower attachment remain intact.
"""
from mathutils import Vector


def place_head(original, fitted, neck_base, neck_top):
    if len(original) != len(fitted) or not original or neck_base >= neck_top:
        raise ValueError('Matching nonempty topology and increasing neck limits required')
    donor_top = max(p.z for p in original)
    incoming_top = max(p.z for p in fitted)
    shift = Vector((0, 0, donor_top - incoming_top))
    result = []
    frozen = []
    for i, (old, new) in enumerate(zip(original, fitted)):
        t = min(1., max(0., (old.z - neck_base) / (neck_top - neck_base)))
        weight = t * t * (3 - 2 * t)
        result.append(old.copy() if weight == 0 else old.lerp(new + shift, weight))
        if weight == 0:
            frozen.append(i)
    if abs(max(p.z for p in result) - donor_top) > 1e-6:
        raise ValueError('Neck transition interferes with crown alignment')
    assert all(result[i] == original[i] for i in frozen)
    return result, shift, {
        'axis': 'Blender Z (native Y)', 'donor_scalp_z': donor_top,
        'incoming_scalp_z': incoming_top, 'translation': list(shift),
        'neck_base_z': neck_base, 'neck_top_z': neck_top,
        'frozen_native_ids': frozen, 'frozen_count': len(frozen),
        'final_scalp_z': max(p.z for p in result),
    }
