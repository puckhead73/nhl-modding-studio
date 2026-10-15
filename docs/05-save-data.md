# 05 — Save Data Handling

> **Superseded for PS3 by the VERIFIED spec:** [formats/ps3-save-container.md](formats/ps3-save-container.md)
> (structure + read/extract verified against every anchor save). This draft remains for general/
> cross-platform context.

> Engine home: the `tdb-savedata` crate ([DESIGN.md](DESIGN.md#engine-crate-set-consolidated)).
> Sequencing: [ROADMAP.md](ROADMAP.md) (PS3/Xbox parity in Phases 1–8; **PS4 is net-new in Phase 9**).
>
> **Verification status: DRAFT.** Platform save-data containers have exact, platform-specific
> structures. Details below marked `[UNVERIFIED]` must be confirmed against the reference
> implementation or real (locally held, never-committed) saves before being relied upon.

NHL Modding Studio must open, inspect, and edit **platform save data** in addition to the game's
TDB databases. Save data is distinct from the shipped databases: it is per-user, per-console, and
typically wrapped in a platform container with its own metadata and integrity protection.

## Platforms

- **Xbox** — save container format(s) for the relevant Xbox generation(s) `[UNVERIFIED]`.
- **PlayStation 3 (PS3)** — PS3 save-data container, historically with platform-level protection
  (e.g., secure file IDs / signing) that must be respected when re-writing `[UNVERIFIED]`.
- **PlayStation 4 (PS4)** — PS4 save-data container/format `[UNVERIFIED]`.
- **NHL Legacy Recompilation** — Xbox-derived; expected to align with the Xbox save layout
  `[UNVERIFIED]`.

## Concerns specific to save data

- **Container vs payload:** distinguish the platform wrapper (metadata, headers, integrity) from
  the game payload we actually edit.
- **Integrity / signatures:** several platforms checksum or sign saves; edits must recompute or
  preserve these or the console will reject the save. Identify per platform.
- **Endianness:** same big-/little-endian split as the databases (PowerPC vs x86-64).
- **Round-trip safety:** never write a save that the target platform will reject; back up originals.

## Facts to confirm and record here

- [ ] Save container layout per platform (header, metadata, payload boundaries).
- [ ] Integrity/signature scheme per platform and whether/how it can be recomputed.
- [ ] How the in-save game data relates to the TDB schema (shared structures? subset?).
- [ ] Per-title differences in save layout across NHL 07–21.

## Design requirements for the `savedata` crate

- Read/inspect without modifying originals; explicit, validated writes only.
- Reuse the `tdb`/`domain` layers where the in-save payload mirrors database structures.
- Surface clearly when a save cannot be safely re-signed/edited for a given platform.
