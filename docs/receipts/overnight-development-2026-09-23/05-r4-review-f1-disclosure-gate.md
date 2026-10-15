# Receipt 05 (R4 review F1) — Correction checkpoint: no partial replacement without its current disclosure

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Component (jsdom) and seam evidence only;
nothing staged, committed, pushed, launched or written live. Dispatch:
[the R4 focused coordinator review](../../validation/development-20260923-r4-review.md), F1.
Claim: `dev-r4-disclosure-gate`. The backend refusal of receipt 01 is unchanged.

## 1. Plain-language summary

The coordinator reproduced a gap in the Replace… dialog: the Write button came on as soon as the
two preview images had loaded, before the pre-write check had answered — and stayed on when that
check failed. So a same-size Xbox 360 write, which is a *partial* replacement, could run without
its promised advance disclosure. The backend was never wrong (it still refuses resized writes into
mipped rasters and decides every write from the bytes on disk), but the review's point stands: a
partial operation must show what it keeps before it runs.

Now the Write button is enabled only by a **current, successful** answer from the writer's own
policy for exactly this target and this source. While the check is pending nothing can be written;
if it fails the dialog says so, offers **Retry check**, keeps Cancel, and does not proceed; the
answer is keyed to the target path and raster, the source path and its dimensions, and is re-read
on every open, so a file replaced at the same path or a different source never inherits a policy
shown earlier. Containers with no pre-check still write, but only because the backend's answer
says so, never because a request failed.

## 2. Files changed

| Path | Change |
|---|---|
| `app/src/components/TextureReplace.tsx` | the plan query keyed on target / raster / source path / source size with `staleTime 0`, `gcTime 0`, `refetchOnMount: "always"` (the "before" preview likewise); `planReady` (a current successful non-refusing answer) gates Write; a failed check renders as a note with **Retry check** |
| `app/src/components/TextureReplace.test.tsx` | the "failure keeps Write available" case **replaced**; four new cases — pending plan (Write disabled, click dispatches nothing, enables on the answer), failed plan (disabled, Cancel available, Retry → enabled → one dispatch), switched source (re-plan with the new dimensions, that source's refusal shown, no dispatch), stale at the same path (second open re-reads the target and re-plans; a refusal now blocks) |
| `crates/studio-api/tests/loose_texture_route.rs` | the stale-plan write: a plan for a single-level file says allowed, the file at that path is replaced by the mipped raster, the write refuses from the current bytes, file and backup state untouched, a fresh plan refuses |

## 3. Reproduction of the review's case

Current preview `texlib`, source at the same size, plan held unresolved: after the previews load,
`texture-replace-plan-loading` is shown and the Write button is **disabled**; clicking it dispatches
no `import_texture`. Resolving the plan with the partial-replacement note enables Write; the write
then dispatches once. (Vitest `keeps Write disabled while the plan is pending and never dispatches
until it answers`.)

## 4. Checks run

| Run | Result |
|---|---|
| `npx vitest run src/components/TextureReplace.test.tsx` | **13 passed** (was 10; one replaced, four added) |
| `npx tsc --noEmit` | clean |
| `cargo test -p studio-api --test loose_texture_route` | **3 passed**, 5 ignored (private fixtures) — the stale-plan assertions run inside the F1 boundary case |

## 5. Limitations

- The frontend holds no dictionary of target file identities; freshness comes from re-reading the
  target and re-planning on every open and on every source change, and from the backend deciding
  each write from the bytes on disk. A file replaced *while* the dialog is open and before Write is
  still caught by the backend (the refusal path), not by the displayed plan.
- Other containers' writer rules (GNF / BIGF / PS3 texlib) are still applied at write time; the
  plan for them states that no pre-check exists, and Write is enabled on that statement.

## 6. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `patch_f1_gate.py` | deleted at closeout |
| Temp roots | none new (the route test reuses its own disposable root) |
| Background processes | none |

## 7. Next action

Teams 2.5 PS4 venue listing under research R73 ([receipt 06](06-r4-ps4-venue-listing.md)).
