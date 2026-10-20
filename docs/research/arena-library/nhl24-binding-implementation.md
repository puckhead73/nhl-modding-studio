# NHL24 arena binding implementation — 2026-10-17

The live NHL24 1.60 roster database and its shipped metadata are now located and extracted reproducibly. A repository-local database coexistence proof adds an arena record and assigns it to one team. Nothing has been installed into the foundation.

## Resolved database carrier

`Patch/Gen4b/globals.toc` → `globals/chunkfilecollector` → collector manifest chunk → `gamemodes/database/t3db/nhl_game.db`.

The collector uses type GUID `3e3768ac-e3bd-7d47-dad2-d75d0e2f57e8`, signature `0x5250e107`. The native manifest GUID is at EBXD offset 76 for this qualified layout. TOC chunk GUID bytes are reversed relative to native collector GUID bytes. Do not substitute NHL27's layout for this type.

The patched collector manifest is native GUID bytes `47a230bb2cfe55e5a3bf938bde520b43`, in Patch catalog 0, CAS 1, stored offset 98,258,009, stored length 80,255. Its directory contains 3,225 entries. The database and `nhl_game-meta.xml` share packed chunk `fbf5f5999419b17b6c5e69020d850bad`, Patch catalog 0, CAS 1, stored offset 98,371,920, stored length 2,085,904. Decode the batch before slicing the directory's decoded offsets:

| File | Decoded offset | Length |
|---|---:|---:|
| `nhl_game-meta.xml` | 523,714 | 588,938 |
| `nhl_game.db` | 1,112,652 | 13,726,652 |

Database SHA256: `4af9e2f8fbc3efe7bc5a7ae996a6c38c1b43efbd6a629720124951ff2ae1ec5a`.

The reader resolves nine collectors and 10,950 logical filenames, including packed stadium cameras and arena CSV/JSON configuration. It checks bounds, stored EBX hashes, decoded lengths, unsafe filenames and ambiguous file bindings. TOC-only chunks do not supply the bundle manifest's SHA1; their source spans and TOC fingerprints are retained, without claiming an independent stored-content hash match.

## Database keys and proof

| Table | Native ID | Population / allocation | Binding |
|---|---|---:|---|
| exhibitionteams | `kOtt` | 531 / 577 | `vfEq` index → `buDa` arenaid |
| exhibitionarena | `StEO` | 552 / 598 | `vfEq` index → `hDRl` arenaartid |
| customarena | `Usqf` | 112 populated | Separate custom arena records |
| leaguearena | `vFSP` | 0 populated | No shipped populated rows |

All 531 exhibition team arena keys resolve. Arena index, arena art ID and team arena ID are 10-bit fields (0–1023); this storage limit does not prove runtime registry capacity. Negative metadata lower bounds are encoding biases, applied in the report. Strings use the shipped legacy encoding.

The staged proof appends a copy of Boston's arena record into existing slack, sets its index to 96 and its name to `TD Garden Library Proof`, and changes Anaheim team 0's arena ID from 0 to 96. Boston team 2 retains arena 2. Both records still use arena art ID 2. **This proves separate database records, not separate scene/resource identities.** ID 96 is unused in this table; mode-specific acceptance remains unverified.

The existing Rust TDB writer successfully reparses and serializes the original database byte-exactly. The proof checks every existing record in every table against its source, allowing only the intended assignment field change. Source and staged CRC round trips pass; restoring the assignment on the original database produces exactly the original bytes. No database growth was needed.

## Dependency evidence and remaining work

The Boston stadium/structure subset contains 151 distinct named/hash EBX variants. All decode and parse. They carry 3,284 import references; 2,676 lead to partitions outside this subset. Remaining in-subset matches are partition candidates, not validated instance matches. Base/update variants remain distinct.

The report includes exact source spans, partition GUIDs, imported instance GUIDs, candidate targets and reverse team sharers. It explicitly leaves complete dependency closure and scene registration unresolved. Numeric `arenaartid=2` and `boston_2` agree, but executable or typed configuration routing must establish the actual selector before allocating a new scene ID.

Next implementation gates:

1. Resolve art-ID selection to the level/scene and mode-specific sublevels; index imported partitions beyond Boston and validate exported instance identities.
2. Decode typed RES/chunk references, streaming registrations and shared materials/controllers to close the control arena graph.
3. Clone the qualified scene with new identities and registration; retain the original scene and independent team assignments.
4. Repack the legacy database batch and collector directory, then compose TOC/CAS contributions with the current overlay. The enlarged/edited legacy file must update directory ranges and content identities.
5. Validate the staged composite and test loading, assignment persistence and rollback in-game before activation.

The 11 files in the foundation snapshot still match their SHA256 values after this work. The reader refuses an overlay that overrides the collector TOCs or catalog-0 CAS inputs; such an overlay requires a composite resolver. The existing moves-only installer is not invoked.

## Reproduce

From the repository root, with the existing NHL24 source installation and local Oodle decoder available:

```powershell
python scripts/arena_library/nhl24_collectors.py
python scripts/arena_library/nhl24_binding_report.py
cargo run -p tdb-core --example nhl24_arena_stage
python -m unittest discover -s scripts/arena_library -p test_nhl24_collectors.py
```

Private extracted files and detailed reports are under `local/arena-library/nhl24-resolved/`. The staged database and proof receipt are under `local/arena-library/nhl24-stage/`. These commands do not install or launch the game. The collector and report scripts reuse existing local research readers; promotion into Studio's domain/API remains a later integration step.
