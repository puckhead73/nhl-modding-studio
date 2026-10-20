"""Qualified NHL24 native interface adjustments, never guessed from source ABI.

Offsets are observed in both original version-6 native banks. Optional casts
not implemented by a class remain null, as in the native transition serializer.
"""

BOOL = 0xbcc2c469
DEACTIVATE = 0x28185be6
BUFFERED = 0x7fd06b26
CONDITION = 0x7d85ab75

OFFSETS = {
    # ScenarioLocatorParamsAsset native cast 245320.
    (0x1b1c4c1f, 0xee8d4bf0): 8,
    (0x1b1c4c1f, CONDITION): 0,
    # FloatAsset cast 8e1fb0: scalar interface +64, writable property primary.
    (0x028883b0, 0x67745308): 64,
    (0x028883b0, 0x7a327d7d): 0,
    (0x028883b0, 0x0aeca644): 0,
    (0x028883b0, CONDITION): 0,
    # PTMPoseFeaturesRuntime serializer requests FeatureSchema interface.
    # Casts: 3ac8d0, 3b3700, 3d6f70, 3d7620, 3e1ce0 respectively.
    (0x302ef52f, 0x19372c2c): 8,
    (0x73533857, 0x19372c2c): 8,
    (0xd4e91eac, 0x19372c2c): 8,
    (0xf4f207b3, 0x19372c2c): 8,
    (0x63632a22, 0x19372c2c): 8,
    # PrevPoseBlend cast 3eba60 returns the primary BlendAsset interface.
    (0x91bd9d2c, 0x90c15c64): 0,
    # Primary Asset casts: 3c92e0, 3ceae0, 3db930, 8e53e0.
    (0x6199cba6, CONDITION): 0,
    (0xb3ff6eac, CONDITION): 0,
    (0x03890581, CONDITION): 0,
    (0xe9329a24, CONDITION): 0,
    (0x33e57bd6, BOOL): 64,
    (0x33e57bd6, DEACTIVATE): 72,
    (0x33e57bd6, BUFFERED): None,
    (0x33e57bd6, CONDITION): 0,
    # BoolAsset cast 0x8e1510, branch 0x8e1561: GestureToSignalMapping
    # requests this primary asset interface for its writable signal.
    (0x33e57bd6, 0x0aeca644): 0,
    (0x89f48d19, BOOL): 8,
    (0x89f48d19, DEACTIVATE): None,
    (0x89f48d19, BUFFERED): None,
    (0x89f48d19, CONDITION): 0,
    (0xd012ad0b, BOOL): 8,
    (0xd012ad0b, CONDITION): 0,
    (0x89ab3b9f, BOOL): 32,
    (0x89ab3b9f, CONDITION): 0,
    (0x805604a0, BOOL): 8,
    (0x805604a0, CONDITION): 0,
    (0x8b978bf3, BOOL): 16,
    (0x8b978bf3, CONDITION): 0,
    # Native cast function 7bbd00, primary vtable a44ab80.
    (0x94141c94, BOOL): 8,
    (0x94141c94, DEACTIVATE): 16,
    (0x94141c94, BUFFERED): None,
    (0x94141c94, CONDITION): 0,
}


def adjustment(type_hash, interface):
    if interface is None or type_hash == interface:
        return 0
    key = (type_hash, interface)
    if key not in OFFSETS:
        raise ValueError(f'Unqualified native interface {type_hash:#x} -> {interface:#x}')
    return OFFSETS[key]
