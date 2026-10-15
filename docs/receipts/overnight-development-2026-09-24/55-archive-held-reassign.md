# Receipt 55 — portrait and head reassignment on archive-held installs (PS3 NHL 09–14, Legacy Recompiled)

**Developer-verified; not Codex-accepted; nothing launched or written to any install.** HEAD `8d60a6d` (unchanged; nothing
staged). Claim `dev-archive-held-reassign`. Research consumed: 2026-09-26 handoff 17 §9 (R134); 2026-09-28 handoff 04 (R138).

## 1. Plain-language summary

- **Players on PS3 NHL 09 → 14 and on Legacy Recompiled can now be given another portrait or head.** Before, Studio refused
  because these games keep their pictures and heads inside archives.
- Studio now lists the attached install's own archives (directories only; no payload is read). It allows a target only
  when the target's file is a member, and names that member as the evidence.
- The rules are unchanged. From NHL 13 a head also needs its texture library. From NHL 12 a head also needs its head-registry
  row. On PS3 NHL 10–12 the per-head colour texture `head_<id>_cm` is reported, not required. The write is still the one
  player field.
- If no install is attached, the refusal says to attach it. Any archive that cannot be listed is named in the plan.

## 2. Files

| Path | Change |
|---|---|
| `crates/studio-domain/src/archive_index.rs` (**new**) | `ArchiveIndex::build(root)`: member path → archive over `EB\0\3` (PS3 11–14, X360 Legacy / Recompiled) and `BIG4` + `.bh` (PS3 09 / 10); loose `BIGF` packages skipped; unlisted archives recorded; `find`, `portrait(id)` |
| `crates/studio-domain/src/player_containment.rs` | `legacyrecomp` contract (Gen7 `rx2`, `player_`) citing R128 / R138 |
| `crates/studio-domain/src/player_reassign.rs` | `plan_reassign(…, archives, …)`; loose file first, else archive member; the blanket PS3 08–14 and Recompiled refusals are replaced; Legacy / registry rules extended to `legacyrecomp`; PS3 10–12 `head_<id>_cm` note |
| `crates/studio-api/src/registry.rs` | `player_reassign` builds the index outside the workspace lock, only for archive-held titles with an attached install |
| `crates/studio-domain/tests/player_reassign.rs` | existing calls pass `None`; two refusal texts updated; new `recompiled_and_ps3_archive_members` |

## 3. Evidence

- **`player_reassign` 4 / 4 (`--include-ignored`, 104 s):** existing Legacy apply / undo, PS4 and refusal cases unchanged.
- **New real-install case, read only.** The installs are PS3 NHL 10, 11 and 13 (X: roms) and Recompiled (H: install, with the X360 Legacy anchor
  database). For each install, an unused shipped portrait and head are allowed with their archive member as evidence. An id with no
  member is refused, by name.
  - NHL 10: 10 archives, 4 unreadable (named in the plan). Portrait 3 is allowed. Head 1000 is allowed via `player.big: rendering/player/player_1000.rpsgl`, and the `head_<id>_cm` note is present.
  - NHL 11: 14 archives, 1 unreadable. Head 1000 is allowed via `rendercache.big`, and the note is present.
  - NHL 13: 41 archives, 1 unreadable. Head 1000 is allowed with the mesh, `texlib_1000` and the `stockplayerheads` row.
  - Recompiled: 15 archives, 2 unreadable. Portrait 15 is allowed. Head 10 is allowed with the mesh, texlib and registry row from `game/cacherender.big`.
- `cargo build -p studio-api` is clean. **No API-level test covers `player_reassign`.** The registry path is compiled but not
  exercised by a test.

## 4. Not run / limits

- PS3 NHL 09, 12 and 14 were not in the new test. They take the same code path as 10 (BIG4) and 11 / 13 (EB).
- Why some archives are unreadable was not investigated. They are named to the user, never searched around, so a target held only
  in one of them is refused.
- Membership proves that the game ships the file. Whether the game draws the reassigned asset is not observed (no game run).
- NHL 07 / 08 stay refused.
- The Recompiled case plans against the X360 Legacy **anchor** database, because no Recompiled anchor exists (Q47 / FR4). Only
  the archive members are Recompiled's own. No database equivalence is claimed; in the app the plan reads the open project's
  own database.
