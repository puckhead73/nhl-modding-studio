# Research receipt 9 — after receipt 8: corrections, then Recompiled read contracts for arena, equipment, team header logo and kits

Research lane, 1 October 2026, published 01:54 (system clock). Prompt:
[`research-20260928-after-receipt8.md`](../../plans/research-20260928-after-receipt8.md).
- Every package's claim was written **before** its work. No agents and no messages to other sessions.
- Nothing launched, listened to, installed, staged, committed or published. No game, cache, application, defs or developer-test file
  was written.
- Disk at start: E: 133 GB free, H: 122 GB free.

## Summary

- **Corrections first** ([18](18-after-receipt8-corrections.md)).
  - Handoff 15 overstated what the logs show. They list only failed file opens, so they cannot show load order, and a vanished error
    proves nothing. The test card is rewritten:
    - one location per trial;
    - an established on-screen asset as the positive control;
    - "no error, same screen" counts as inconclusive.
  - The card is **parked**: the recompiled game always uses `Documents\nhllegacy` for saves and cache, and nothing available shows a way
    to isolate it. It waits for your backup / restore choice (about 70 MB).
  - Handoffs 16 / 17 now separate "what was read" from "what matched Xbox 360 Legacy once". They also spell out duplicate, missing,
    unreadable and corrupt cases and incomplete loose folders, and say that loose-first is Studio's own choice.
- **Four new read contracts**, all for Legacy Recompiled, all read-only:
  - arena art ([19](19-recomp-arena-read-contract.md));
  - equipment previews ([20](20-recomp-equipment-read-contract.md));
  - the team header logo ([21](21-recomp-team-header-logo-read-contract.md));
  - kits ([22](22-recomp-kit-read-contract.md)).
  In each, every needed file is inside the archives, appears once, carries the Xbox 360 Legacy name, and matched on every sample.
- **Two findings worth your attention:**
  - Shared 3D shapes (gloves, skates, sticks, cages, pads, shirts, socks) live in a **different archive**, `renderboot.big`, from the
    textures. A reader that looks only where the textures are would show colours with no shape.
  - The two archives the developer could not list hold no art. That could turn 1 129 "unknown" portraits into firm answers, if the
    developer and coordinator choose to accept a names-only listing.
- **The developer is frozen on r67** while you verify the beta. These contracts wait for review and a later dispatch.

## 1. Packages

| # | Claim | Output | Result |
|---|---|---|---|
| 1 | `research-q17-after-receipt8-corrections` | [18](18-after-receipt8-corrections.md); dated notes in 15 / 16 / 17 | authority reconciled (K19); 15 corrected and card revision 2 pending (K20); 16 / 17 amendments plus a reading of receipt 67's delivered speech route against them (K21) |
| 2 | `research-q18-recomp-arena-read` | [19](19-recomp-arena-read-contract.md); `data/r147*`, `data/r147b*` | arena read contract; the gamedata "unlistable" archives identified by name |
| 3 | `research-q19-recomp-equipment-read` | [20](20-recomp-equipment-read-contract.md); `data/r148*` | equipment-preview read contract |
| 4 | `research-q20-recomp-team-header-logo` | [21](21-recomp-team-header-logo-read-contract.md); `data/r149*` | team header logo read contract |
| 5 | `research-q21-recomp-kit-read` | [22](22-recomp-kit-read-contract.md); `data/r150*` | kit card read contract |

At package boundaries research read developer receipt 67, the receipt-67 review and the r67 verification freeze, once each.
- The concrete developer source question found there (the "unlistable" `gps.big` / `faceposer.big`) is answered in 19 §6.
- The review's speech-route findings (first-match, partial loose folder, bank / stream) agree with 18 K21.

## 2. Corrected claims

| Claim | Correction |
|---|---|
| 15: the runtime is "first trying" loose files, then archives | **withdrawn**: failed-open logs show no order or precedence |
| 15 §4 card: two cache roots at once; a vanished error proves loading; copying the install isolates it | **replaced** by revision 2 (18 K20): one location per trial; inconclusive without an affirmative positive; `Documents\nhllegacy` is shared; pending a user backup / restore plan; no install copy needed for the cache trial |
| 16 / 17: "identical to X360 Legacy" as a property of the source | narrowed to history for this install; current provenance is shown, and a match is claimed only after hashing |
| 16 / 17: duplicate handling via `read_eb_member` | it decodes and locates by first match; a separate index pass must reject duplicates within and across archives |
| 16 / 17: loose first, member only when the folder is absent | loose first is Studio policy; per-file fallback; coherent `.sbr` / `.sbs`; refuse unreadable sources |
| 17: "missing in that archive = not shipped" | only for a readable complete index; any unlistable archive makes absent ids unknown |

## 3. Developer-consumable scope (priority; after the freeze, by dispatch)

1. **Speech-route robustness** (18 K21 table; the receipt-67 review agrees):
   - within-archive duplicates;
   - first-match bank lookup;
   - a partial loose folder hiding members;
   - bank / stream source coherence;
   - absent versus unreadable.
   Add the partial-tree regression.
2. **C40 portrait / head preview** (17 as amended).
3. **C41 arena** (19): admit `legacyrecomp` only with the member route. Fix the `nhllegacy`-project-on-Recompiled-root "missing" edge.
4. **C42 equipment** (20): list all archives, `renderboot.big` required; the same grammar via one listing source.
5. **C43 team header logo** (21): crest first, rink fallback only when the crest is absent. A corrupt crest is an error, not a
   fallback.
6. **C44 kits** (22): `Measured SingleFile` for `legacyrecomp` with the member route.
7. **Optional**, for the developer and coordinator to decide: name-only listing of `gamedata/gps.big` (inline BIG4) and
   `faceposer.big` (EB 0x00200400 name table) to retire "unknown" in containment. Offsets for 0x00200400 are not established.

## 4. Checks actually run

| Command (repository root) | Result |
|---|---|
| `grep -n -i -E "user data\|cache root\|…\|\.json" nhllegacy_00[12].log` | only the `Game directory` / `User data` / `Cache root` start lines and game asset paths; no switch or configuration |
| `du -sb` the install / `Documents\nhllegacy`; `find … -type f \| wc -l` | 6 588 039 707 B; 70 118 745 B, 16 files |
| `python …/probes/r147_recomp_arena_members.py data/r147-recomp-arena-members.json` | stadium 67, jumbotron 66, icesurface 474 (2 in renderboot), banner 318; all chunklzx; 0 / 0 duplicates; name sets equal; 24 / 24 samples equal |
| `python …/probes/r147b_gamedata_small_archives.py data/r147b-…json` | `gps.big` inline BIG4, 15 names, in bounds; `faceposer.big` 904 `.cba` names (offsets not in bounds under the 16-byte row assumption); 0 `rendering/` / `fe/` paths |
| `python …/probes/r148_recomp_equipment_members.py data/r148-…json` | 9 folders, 5 045 members, all chunklzx, 0 / 0 duplicates, name sets equal, 36 / 36 samples equal; `renderboot.big` holds the shared meshes |
| `python …/probes/r149_recomp_team_header_logo.py data/r149-…json` | crests 320 stored in `cache.big`; rink logos 1 926 chunklzx in `cacherender.big`; `_0_0` fallback for 275 / 318 ids; 12 / 12 samples equal |
| `python …/probes/r150_recomp_kit_members.py data/r150-…json` | jersey 1 883 / pant 976 / sock 981; templates accept 905 + 904 / 959 / 946; 0 / 0 duplicates; 12 / 12 samples equal |
| `sha256sum` of data and probes | `r147` `3fac4659…`, `r147b` `f1fabf81…`, `r148` `37ea0ad5…`, `r149` `fb677632…`, `r150` `ee97db80…`; probes `e05da0c2…`, `d3e778fd…`, `98e60593…`, `ece131eb…`, `68f92875…` |

All probes ran with `PYTHONDONTWRITEBYTECODE=1`. Source reading, not executed:
- `speech_catalogue.rs`, `gen7_speech_preview.rs`;
- `arena_assets.rs`, `venue_art.rs`;
- `equipment_slots.rs`, `equipment_piece.rs`;
- `assets.rs` `decode_team_logo`, `team_logos.rs`;
- `kit_files.rs`;
- `registry.rs` consumers.

No `cargo` command was run.

## 5. Measured versus inferred

| Measured | Inferred / not established |
|---|---|
| Member paths, archives, wrappers (every member), duplicates, name sets against X360 Legacy loose, template counts, 84 sampled byte equalities | that a later or modified install keeps any of this (18 K21) |
| Venue packages fully equal via R137 plus R128 archive identity | full byte equality for the other arena, equipment, logo and kit directories (samples only) |
| `gps.big` inline directory and `faceposer.big` names | `faceposer.big` 0x00200400 offsets / row layout |
| Log text: mounts, cache root, failed `cache:` opens | any loader precedence, or a loose file being used |
| Current consumer behaviour on Recompiled | from source reading only; not run, nothing reproduced |

## 6. Remaining blockers and owners

| Blocker | Owner |
|---|---|
| Review of 18–22 (and C41–C44 queue rows) | coordinator |
| r67 verification pass; card R-RECOMP-LOOSE-1 revision 2 (pending backup / restore choice and an established positive control) | user |
| Implementation of C39 robustness, C40–C44, after the freeze | developer, by dispatch |
| Name-only archive listing as evidence for "missing" | developer / coordinator decision |
| League imagery row for `legacyrecomp` (alias decision open) | coordinator |
| Earlier unreviewed developer work and the desktop / game cards | coordinator / user |

## 7. Cleanup and retention

- **Kept:**
  - handoffs 18–22 and this receipt;
  - `data/r147*`–`r150*` JSON;
  - probes `r147*`–`r150*` under `local/runs/overnight-research-2026-09-28/probes/`;
  - claims q17–q21, all `ready`.
- **Session scratchpad:**
  - temporary Markdown drafts and two helper scripts (`notes.py`, `mk150.py`), all in the session scratchpad;
  - removed after publication, after a resolved-path and reparse check.
- No extract, copy or install was made. No bytecode was created. Nothing outside the scratchpad was deleted.

## 8. Next research task

Apply the coordinator's review of 18–22. Until then, start no new route: the developer is frozen and the remaining named route
(league imagery) waits on the coordinator's `legacyrecomp` alias decision.
