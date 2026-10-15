# N19 deployment experiment specification — 2026-09-10 (rev. 2)

Prepared, not executed. No live file drop, archive modification or game launch has been
performed. This document exists so the operator can run a decisive test; it does not
authorize one. Revision 2 applies the H2a dispatch's corrections.

## The artifacts

| Role | File | SHA256 | Size |
|---|---|---|---|
| Diagnostic | `patched.mcd` | `E19190EAFAA9A381097218186A309DE310CBFB3279CAB070520A275CA833E322` | 1,043,168 |
| Control | `noop.mcd` | `4DA8F5D5C9EAA64F3AAA38B7E9E4CB0B0AEA74B91A1884FF863BF0CC61243195` | 1,043,168 |
| Donor (vanilla) | archive entry | `4DA8F5D5…` (same as control) | 1,043,168 |

Both install as `jack_eichel.mcd`. The diagnostic displaces 26 of 3,157 `geo_headShape`
vertices by +0.010 in Z — a bounded upper-forehead bulge (`y ∈ [1.895, 1.930]`,
`z ≥ 0.045`, `|x| ≤ 0.055`), clear of eyelid pockets, the lip seam, the oral cavity and
the shoulder/chest shell boundary. 78 bytes differ from the donor, all inside the writer's
allowlisted spans. The control is byte-identical to the donor.

**Verify the staged donor separately** before installing anything: confirm the vanilla
archive entry still decodes to `4DA8F5D5…`.

## What the control does and does not prove

The control exists to catch false positives, not to prove loading. A control that renders
normally is consistent with the override working *and* with it being ignored entirely —
it cannot distinguish them. Only the diagnostic can produce a positive.

Likewise, a trace showing the game *requested*
`nhl/data/kettle/rendering/characters/player/jack_eichel.mcd` establishes that the logical
asset was asked for. It does **not** establish which physical payload — loose file or
archive entry — satisfied the request. Therefore **no visible change plus a generic
request trace remains inconclusive**, and must be recorded as inconclusive rather than as
evidence that overrides do not work.

The truncated-file probe from revision 1 is **removed**. It substitutes a crash/error
signal for a rendering signal, which is less specific than a valid-asset A/B test and
risks attributing an unrelated failure to the override path.

## Candidate install paths, in priority order

1. `<install>/nhl/data/kettle/rendering/characters/player/jack_eichel.mcd` — the loose
   tree that already exists at the install root and mirrors the archives' internal path
   namespace. **Precedence unproven:** its 2,335 files are disjoint from every archive
   (no path in it appears in any `.big`), so nothing observed so far shows it is consulted
   for a path an archive also provides.
2. Archive replacement in `patch_game_93.big` entry 3036 — the highest-numbered patch
   carrying the asset. Planned delivery fallback; needs the H5 payload work.

`databigs/Extracted/` is **not** a candidate. It is a third-party dump whose
`jack_eichel.mcd` is three concatenated archive payloads, and the writer refuses it.

## Procedure

1. **Record the pre-test state of the exact target path.** Note whether the file exists.
   If it does, record its bytes and hash. This is what restoration returns to.
2. **Baseline.** Record runtime and exact build (shadPS4 version, or PS4 firmware),
   `APP_VER=01.60`, `CONTENT_ID=UP0006-CUSA11126_00-NHLICEHOCKEY2019`. Reach a scene that
   shows Eichel's head as **demonstrably 3D** — one where the camera moves or the head
   turns, so a pre-rendered portrait cannot be mistaken for the mesh. Capture **front and
   profile** screenshots. A crash before asset loading is a runtime baseline blocker, not
   mesh evidence.
3. **Install the control**, cold-restart the game, return to the same scene, capture front
   and profile. Record the installed file's hash.
4. **Install the diagnostic**, cold-restart again, return to the same scene, capture front
   and profile. Record the installed file's hash.
5. **Collect physical-payload evidence where the runtime offers it** — a file-open log
   naming the loose path, or an archive-entry read at offset 263,945,840 of
   `patch_game_93.big`. This is the only thing that distinguishes which payload served the
   request.
6. **Restore.** If the path was absent before the test, remove the created file and verify
   its absence. If it was present, restore the recorded original bytes and verify the
   recorded hash. Re-verify the baseline scene.

Use a cold restart between runs rather than clearing caches. Do **not** broadly delete
shader caches or save data: that changes more state than the experiment controls for.

## Reading the result

* **Forehead bulge visible in step 4, absent in step 3** → the loose path is consulted;
  deployment for the remaining packages is a file copy.
* **No change in step 4, with step 5 showing the archive entry was read** → the loose path
  is ignored; archive replacement is required.
* **No change in step 4 without physical-payload evidence** → inconclusive. Record it as
  such.
* Crash, missing head or corrupt render → capture the log before restoring, and do not
  attribute an emulator failure to the mesh without evidence the mesh was loaded.

## What this experiment does not establish

Runtime archive precedence between `game_90-008.big`, `patch_game_91.big` and
`patch_game_93.big` remains unproven. Base and patch 93 decode identically and patch 91
differs only in `SKTN` joint-name ordering — but the semantics of that ordering are not
established, so patch 91 cannot be declared runtime-equivalent, only geometrically
identical. The offline donor choice is unaffected either way.

For the archive route, plan a qualified **fit-in-slot** replacement (recompressed payload
no larger than the original stored size) or a qualified **append-and-retarget** (payload
appended, that entry's table row repointed). Whole-archive offset reflow is **not** the
default and should not be assumed.
