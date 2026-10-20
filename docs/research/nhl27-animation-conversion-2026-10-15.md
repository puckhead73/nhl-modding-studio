# NHL 27 animation conversion: DCT motion extraction

The first conversion stage is implemented: 20 sampled NHL 27 animation resources convert to neutral float32 channel buffers with checked EBX/resource identities. This is **not yet a playable NHL 24 animation port**. Skeleton/channel binding, target ANT serialization and engine evaluation remain unresolved.

## Implemented and measured

- Twenty source resources decoded, including bank-deke left/right, tap-deke variants, poke-lunge blends, a get-up and facial-animation data. They contain 739 sampled key poses in total.
- Every resource's stored CAS SHA-1 and decoded size validate. Its resource ID matches the embedded DCT EBX instance, its DCT schema signature matches `0x03821ab8`, and its quaternion/vector/float-vector counts agree with the EBX fields.
- The compressed stream consumes exactly the calculated bit budget. All buffers fit the observed 16-byte rounding and trailing zero guard.
- For clips containing quaternion channels, the largest deviation of quaternion length from one is 0.0010673. Rotations were **not normalized to conceal errors**. The facial clip has no quaternion channels; its rotation checks are explicitly inapplicable.
- Exporting the decoded doubles to little-endian float32 introduces at most 4.705e-7 absolute component error in this sample. This is export precision, not error relative to engine playback.
- Twenty-six synthetic reader/converter tests pass, including known constant and frequency-domain motion signals, signed bit order, corruption rejection, non-rotation channels and duplicate bank-table cross-checks.

Follow-up: [expanded conversion and isolated trial](nhl27-deke-trial-2026-10-15.md) supersedes this initial sample's unresolved-binding and native-layout status. The findings below describe the original smaller pass.

The first control, `af/assets/dk21_bank_deke_left`, contains 46 stored sample times and 116 decoded channels: 93 quaternion, 18 vector and five packed float-vector channels. Its partition identity is `a621951d-1dc9-15f2-a621-951d1dc915f2`. Its first little-endian 64-bit identifier is present in both target game-bank ID tables. This is an identity correspondence, not a demonstrated object-layout match.

## Format findings

The observed source resource has a 32-byte envelope, an optional uint16 sample-time table, then a DCT body. The body offset equals `32 + 2 * sample_time_count`. The envelope's declared body length excludes alignment and the final two words, 4 and 16. The reader validates that exact observed trailer without claiming its runtime semantics.

The DCT body starts with key/channel counts and quantization parameters. One descriptor byte per channel gives its coefficient count in the high nibble; the descriptor array aligns to two bytes. Each channel then has four signed 16-bit base coefficients followed immediately by its component bit-width descriptors. Treating all base coefficients as one contiguous array corrupts the interpretation.

These NHL resources encode values **least-significant-bit first**, including the bit significance within each value. The public AssetBankPlugin's older DCT reader uses a different bit order. Using that order consumed the right amount of data but produced invalid rotations; the corrected NHL decoder passes both independent synthetic signals and the sampled quaternion checks.

The decoder evaluates an eight-point inverse DCT mathematically. Engine SIMD evaluation, rounded coefficient tables, interpolation and floating-point equivalence have not been qualified. The output preserves sampled channel components; it does not synthesize intermediate frames, retarget bones, normalize quaternions, or assign physical units to unknown float channels.

## Target bank and binding evidence

Both target banks have version-3 outer headers with two agreeing ID counts. Header size is exactly `36 + 24 * count`: a 16-byte-per-ID table followed by an eight-byte-per-ID table. The first table's upper halves are zero, its lower halves agree with the second table, and IDs are unique in both files. Counts are 71486 for base and 71311 for 1.60.

Five sampled clip-root IDs occur in both target game-bank tables: get-up backshot, bank-deke left/right and the two sampled tap-deke `_pu` variants. The other sampled IDs are not thereby absent from NHL 24; only the two game banks were checked.

The source binding pass inspected 45693 unique AF EBX payload candidates from the sampled Data/Patch game/globals inventories, excluding arena paths, without parser errors. Four directly imported partitions resolve by GUID to these definitions:

- `af/assets/common/rigs/player/faceposerdsl`
- `af/assets/common/rigs/player/bodyhandsrigdsl`
- `af/assets/common/rigs/player/bodyhandshelmetrigdsl`
- `af/assets/common/rigs/player/animrigdsl`

Five other directly imported partition GUIDs remain unresolved in that inventory scope, including the bank-deke's channel-to-DOF dependency. The resolved definitions are DofSetList assets; their nested references still require traversal. No skeleton topology or per-channel bone-name mapping is claimed.

Internal target-bank object offsets, relocation handling and serialization are **not** qualified. Experimental offset interpretations were not turned into a writer. The older public reader expects `GD.DATA`/`GD.REFL` sections, which are absent from these two target banks; it cannot be used as a drop-in NHL 24 bank writer.

## Artifacts and reproduction

`scripts/nhl27/animation_dct.py` is the bounded DCT reader and experimental motion exporter. `scripts/nhl27/animation_stage.py` verifies metadata, stages float32 buffers and searches the sampled source bindings. Both leave installed files unchanged.

```powershell
$py = 'C:\Users\puckh\AppData\Local\Programs\Python\Python312\python.exe'
& $py scripts/nhl27/animation_stage.py local/nhl27-eboot/compatibility --scan-all-af
& $py -m unittest discover -s scripts/nhl27 -p 'test_*.py'
```

Inputs are the existing compatibility inventories, source type evidence and extracted target banks. Private outputs live under `local/nhl27-eboot/compatibility/animation/`. `stage-report.json` is authoritative for this pass; earlier exploratory JSON files are not qualification receipts. Each SHA-1-named export has a `.json` manifest, `.f32` channel buffer, original `.ebx` and decoded `.resource`. Float buffer order is `[sample][channel][x,y,z,w]`; channel categories are quaternion, vector, then float-vector. Scalar float counts and packed vector lanes must be interpreted through the binding metadata before retargeting.

Next implementation dependencies: resolve channel-to-DOF imports and nested DOF lists; decode target records for the matched bank-deke IDs; compare skeleton and channel semantics; then build one isolated target-bank replacement with round-trip and runtime checks. Adding an entirely new move also needs input eligibility, transitions, events and puck/stick synchronization.

Format cross-check: [AssetBankPlugin DCT source](https://github.com/marv7000/AssetBankPlugin/tree/fd88e55fc79c8bcdb38161dbd0f1e713a9946331/Ant/DCT). Its clone stays in the private reference directory. No upstream source files or extracted game assets were added to production Studio code.
