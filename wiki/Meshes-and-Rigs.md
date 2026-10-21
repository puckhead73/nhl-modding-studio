# Meshes and Rigs

**Evidence:** measured: the Xbox 360 and PS3 RenderWare player-mesh reader and the PS4 RSF reader are verified against real files (Legacy and NHL 10 to 15 on Xbox 360; NHL 15 and 16 on PS4); the shared rig and bind-pose space are verified on Legacy. Mesh writing is bounded to specific donors; general writing is open.

## Gen7 player meshes (RenderWare `.rx2`, PS3 `.rpsgl`)

Player heads, kit pieces and equipment on Xbox 360 are RenderWare files with the same `\x89RW4xb2` magic as texture libraries; PS3 meshes use their own magic.

**Vertex streams:** position as half-float or float; UV0 as unbiased u16 ÷ 65 535; indices as u16 big-endian triangle strips with 0xFFFF restart; **blend indices** as four u16 **shader-register** numbers, every one a multiple of 3 (palette index = value ÷ 3); **blend weights** as four normalised u8 summing to 1.

**AnimationSkin chunk:** `{u32 self_offset, u32 bone_count, 8 bytes pad}` followed by `bone_count` 4×4 big-endian inverse-bind matrices. **122 bones on a Legacy head, 123 on NHL 10.** Bones have no names; identity is the palette index. Each bone carries one NaN sentinel.

**Multi-skin copies.** Legacy, NHL 14 and NHL 15 ship one AnimationSkin shared by every submesh. NHL 09, 10, 11 and 12 repeat it once per submesh (four to eight copies) of the same rig: byte-identical on NHL 10 apart from each copy's own offset; with denormal noise (around 1e−37) in near-zero elements on NHL 09; with an occasional small global re-quantisation on NHL 11 and 12 (one 0.37 mm variant among 821 heads, one 3.3 mm variant among 799). One genuine divergence of about 13 cm, a submesh bound to a different rig, recurs on NHL 11 and 12. The comparison rule is to compare as floats with a 1 cm absolute plus 0.1% relative tolerance: noise and re-quantisation pass, a centimetre-scale per-submesh rig is refused for editing. Previews keep the first copy; edits refuse meshes with more than one skin chunk. Two code comments disagree about NHL 10 (byte-identical copies versus 312 per-submesh-rig heads): treat NHL 10 as preview-only until reconciled.

NHL 07 names each index buffer's payload by file-table index; vertex buffers are the untagged entries the index buffers did not claim, in table order. Standalone hair variants carry no skin chunk.

## The shared 122-bone rig and bind-pose kit preview

Every Legacy kit piece (jersey, pants, socks, skates, hands, gloves, helmet) is skinned to **one shared 122-bone rig**, `skeleton_bindpose_player.rx2`. Equipment meshes carry blend indices and weights but no palette of their own, and every vertex is authored in the same bind-pose model space in metres: sock shins at roughly y 0.2 to 0.8, pant hips 0.7 to 1.2, jersey torso 1.0 to 1.7, head 1.5 to 1.95. Placing every piece at its stored positions therefore gives a correct standing figure with no skinning and no per-piece transform, which is how the Studio's kit viewer works. Re-posing needs the bone hierarchy, which the flat palette lacks (open).

The space matches NHL 24's rig (head bind at y 1.781 against the Legacy template's eye submeshes at 1.833), and nearest bind-position correlation against NHL 24's 433 named bones is how Legacy palette slots get names.

**Palette usage on six vanilla Legacy heads:** 39 of 122 slots carry weight; the dominant ones are 8 (head, 12 064 mass), 16 (jaw), 4 (spine) and the facial slots 17 to 48. Slots **20, 49 and 80 carry zero mass** and slot 5 ("neck") carries 8.61 against 353 on slot 6. Never route remapped weight to slots 5, 20, 49 or 80: they hold garbage matrices and sending weight there has shredded heads in game. One code comment numbers the tongue as register 63 (palette 21) while the census names slot 20; verify before relying on either.

## PS4 RSF containers (`.mcd` mesh, `.skel` skeleton, `.rsf` shader)

Header `RSF\0`, u32 0, u32 0x30, u32 0, then a sequential chunk stream with no directory. Tags are reversed four-character codes: `MOEG` geometry, `TMFV` vertex format, `MRTS` stream, `XDNI` indices, `HSEM` mesh, `TSLT` triangle list, `TIEW` view (LOD container), `FRGS` skeleton root. Each geometry sub-chunk is preceded by a child count.

- Submesh layout is interleaved (format, stream, indices per submesh) for logos and simple meshes, and struct-of-arrays under `TIEW` for heads: all formats, then all streams, then all indices, per LOD. Streams match formats by lead stride.
- The vertex format is ASCII: five fields on NHL 15 (`name:byteOffset:streamIdx:0001:countType`), **seven on NHL 16 and later** (the type is the last field). Attributes: `p0` position, `n0` normal, `g0` tangent, `t0`/`t1` UV, `t7` blend indices (4 × u8), `t8` blend weights (4 × u8 ÷ 255); **four influences per vertex**.
- Strides: logo 24, head base 32, head with UV2 36, skinned head 40, eye 36, morph delta 16. NHL 16 eyes split into a 12-byte position stream and a 24-byte attribute stream.
- Stream chunk: vertex count at +0x09, stride at +0x0D, data at +0x15. Index chunk: count at +0x09, `61DI` 16-bit or `23DI` 32-bit, **triangle list**; baked logos carry about 36% degenerate triangles.
- Per submesh a tail of mesh, triangle count, Maya shape name (`geo_headShape`, `geo_haircapShape`, `hair_cardsShape`, `geo_leyeShape`, `geo_reyeShape`) and a six-float bounding box.
- The **rig is external** (`.skel`, a typed scene graph of groups, transforms, locators and joints with translate, rotate and scale as three floats each; bone names recoverable; `t7` indices follow joint order), so there is no multi-skin ambiguity. Heads carry about 60 named facial morphs as single-attribute delta streams.

**Writing:** a fail-closed `.mcd` editor changes vertex positions only, on fingerprinted NHL 19 donor files; topology change, weight transfer and non-donor files are refused. NHL 19 PS4 head imports were user-reported working in game, installed by scripts outside Studio. There is no general mesh-replace route.

Head-level correspondence between games, and the NHL 24 to Legacy conversion, are on [Head UV Templates and Transfer](Head-UV-Templates-and-Transfer).

Last reviewed: 2026-10-20
