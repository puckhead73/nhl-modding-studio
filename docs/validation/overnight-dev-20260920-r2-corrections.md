# OVERNIGHT-DEV-20260920-R2 — colour reapply correction

Package record and receipt. Dispatch: [overnight-dev-20260920-r2-dispatch.md](overnight-dev-20260920-r2-dispatch.md)
(R1 scoped acceptance of the ownership guard and the colour target-switch fix; not the campaign).
Developer: Claude. **Developer-verified; not Codex-accepted; nothing game-verified.** Nothing
staged, committed or pushed; no launch, no live write; frontend only — no backend or binding change.

## 1. Plain-language summary

After R1, a colour you had successfully written was remembered by the colour box as "just sent",
so that pressing Enter and then clicking away would not send it twice. That memory was only cleared
when a write failed — never when the project caught up with the edit, and never when you undid it.
So after edit → undo you could not apply the same colour again without picking a different colour
first or leaving the page. The box now keeps two separate notes: "a request is actually running"
(nothing else is sent until it settles) and "this value succeeded but the screen has not refreshed
yet" (the same value is not sent twice in that window). The second note is dropped the moment the
stored colour changes for any authoritative reason — the edit being acknowledged, an undo, a reload
— so the same colour is a real new edit again. Everything R1 established (one editor per team /
project / field, arguments captured at dispatch, refresh for the owning project) is unchanged.

## 2. Fix

[app/src/components/ColorField.tsx](../../app/src/components/ColorField.tsx) (`ColorEditor`):

| Before (R1) | After (R2) |
|---|---|
| one ref `inFlight` = the hex dispatched; cleared only on error | `inFlight` = the hex of the request actually pending; set at dispatch, cleared in `onSettled` (success or error) |
| — | `awaitingAck` = the hex of a write that succeeded but whose authoritative value has not arrived yet; set from `inFlight` in `onSettled` on success only; **cleared whenever the stored colour of this target changes** (the same render-time seed reconciliation that re-seeds the draft: acknowledgement, undo, reload, another client's edit) |
| `commit`: skip when `hex === current \|\| hex === inFlight` | `commit`: skip while `inFlight !== null` (a draft typed meanwhile stays in the box and commits on the next Enter/blur after settle); then skip when `hex === current \|\| hex === awaitingAck` |

Target-bound editor key, captured mutation arguments and the owning-project refresh are untouched.

## 3. Regression evidence (developer)

[app/src/components/ColorField.test.tsx](../../app/src/components/ColorField.test.tsx) — 17 cases
(13 from R1 unchanged, 4 new):

| Case | Asserts |
|---|---|
| `reapplies a value after edit → acknowledgement → undo` (the reviewer's reproduction) | `#123456` blur → 1 `set_cells_int`, summary refresh, input re-enabled; authoritative `0x123456` (acknowledgement) → box shows it, a blur dispatches nothing; authoritative `0xdd9b26` (undo) → box shows it; `#123456` again → **2nd** `set_cells_int` with exact args `{projectId 7, kOtt, record 3, feprimarycolor 0x123456}`, 2nd `update`, box `#123456` |
| `releases the post-success suppression on any authoritative change` | after success, a blur of the same hex before refetch → still 1 write; authoritative `0x0f0f0f` → box `#0f0f0f`; `#123456` → 2nd write; acknowledged `0x123456` → Enter + blur → still 2, box `#123456` |
| `holds a draft changed while a request is pending` | held request; `#0c0c0c` typed + Enter + blur while pending → still 1 write, box keeps `#0c0c0c`; on resolve: `update` ×1, input enabled, draft kept; blur → 2nd request with `0x0c0c0c`; resolve → 2 writes, box `#0c0c0c` |
| `retries the same value after a failed write …` | refusal shown, 0 `update`; blur → 2nd write with identical args; backend recovers → Enter → 3rd write, `update` ×1; Enter again → still 3 (duplicate before acknowledgement); acknowledged → box `#123456`, no refusal text |
| (R1, unchanged) `dispatches once for Enter followed by blur, and once for repeated blurs before the refetch lands` | Enter + blur + blur → 1 request; after success another blur → still 1; a different value → 2nd write |
| (R1, unchanged) target-switch, in-flight-after-retarget, unmount, error-cleared-on-switch cases | as recorded in R1 |

Runs:

| Command | Result |
|---|---|
| `npx vitest run src/components/{ColorField,FieldProfile,ArchiveMemberReplace,TeamLogoCard,TextureReplace,LeagueImageryCard}.test.tsx` | 6 files, **41 passed** |
| `npx vitest run` (full frontend) | **20 files, 150 tests passed** on 8 of 9 runs; the **first** full run reported 1 failed / 149 passed and its failing case was not captured (only the totals line was kept). Eight consecutive full reruns were green. Recorded as an unattributed single failure, not explained away; if Codex wants it chased, the next run should keep the whole log. |
| `npx tsc --noEmit -p .` | clean |

Not run: Rust suites (no backend change), bindings check (no DTO change), fixture sweeps, any launch.

## 4. Changed identities

| Path | Change |
|---|---|
| `app/src/components/ColorField.tsx` | `awaitingAck` ref; `inFlight` cleared on settle; commit blocked while pending; suppression released on authoritative change |
| `app/src/components/ColorField.test.tsx` | +4 cases |
| `docs/validation/overnight-dev-20260920-r2-corrections.md` | this record |

## 5. Cleanup

This session's scratchpad: the nine vitest run logs written for the rerun check were deleted; the
scratchpad is empty. No temporary files in the repository, no processes left, nothing else touched.

## 6. Omissions and review areas

- jsdom evidence only; no desktop reproduction.
- The unattributed first-run failure (§3).
- Review the chosen semantics: a commit attempted while a request is pending is deferred (draft
  kept), not queued as a second write; an authoritative change of any kind releases the
  post-success suppression (so a same-value Enter *after* the acknowledgement is caught by the
  `hex === current` rule, and after an undo it is a new write by design).
- Phase 1 remains open; the provisional Phase 2 matrix implies no feature acceptance; the
  canonical rebuild, BIG4/replan, resize writers and broader team coverage still await their own
  reviews; NHL21-E1 stays closed.

Stop for Codex review.
