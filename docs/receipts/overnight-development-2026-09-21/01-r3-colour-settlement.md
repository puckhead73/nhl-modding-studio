# Receipt 01 — OVERNIGHT-DEV-20260920-R3: colour settlement bound to its authoritative generation

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
baseline `8d60a6d`. Frontend only; no backend, DTO or binding change. Nothing staged, committed,
pushed, launched or written live. Dispatch: [r3 dispatch](../../validation/overnight-dev-20260920-r3-dispatch.md).

## 1. Plain-language summary

The colour box remembers "this value was just sent" so that pressing Enter and then clicking away
does not send it twice. That memory is written when the request finishes — and the request only
finishes after the screen has been refreshed, so the acknowledgement (and even an undo) can arrive
*before* it. In that ordering the old memory was written late and wrongly, and the same colour could
not be re-applied after the undo. The memory is now tied to the exact request **and** to the state
the request was sent against: if anything authoritative changed in between, a late completion no
longer reinstates the guard. Everything else — one editor per team/project/field, one request in
flight, no duplicate before acknowledgement, retry after failure, reapply after undo, refresh of the
owning project — is unchanged.

## 2. Fix

[app/src/components/ColorField.tsx](../../../app/src/components/ColorField.tsx) `ColorEditor`
(`640460662bbe21e1`):

| Before (R2) | After (R3) |
|---|---|
| `inFlight: string \| null` (the hex pending) | `inFlight: { hex, gen } \| null` — the hex **and** the authoritative generation at dispatch |
| — | `authGen` ref, incremented by the same render-time reconciliation that re-seeds the draft and clears `awaitingAck` on any stored-colour change of this (keyed) target |
| `onSettled`: `if (!error) awaitingAck = inFlight` — unconditional | `onSettled`: arms `awaitingAck = done.hex` **only if** `done.gen === authGen.current` — i.e. no acknowledgement, undo or reload arrived between dispatch and settlement |

Preserved: keyed editor (`targetKey`), captured mutation arguments, `update(project_summary(args.projectId))`
for the owning project, commit refused while `inFlight !== null` (draft kept), `hex === current`
and `hex === awaitingAck` duplicate rules, release on error.

## 3. Regression evidence (developer)

[app/src/components/ColorField.test.tsx](../../../app/src/components/ColorField.test.tsx)
(`83b365f2a14efcfb`) — 23 cases: 17 from R1/R2 unchanged, **6 new** under "R3 F1: settlement
ordering", all with the `project_summary` response held (`gated`) so settlement is deferred:

| Case | Asserts |
|---|---|
| acknowledgement before settlement | write 1; ack B while summary held; resolve; Enter+blur B → still 1; reload to `#0f0f0f`; B → **write 2** with exact args |
| **acknowledgement + undo before settlement (the R3 reproduction)** | write 1; ack B then undo A while summary held; resolve; Enter B → **write 2** (R2: 1); blur → still 2; two summary refreshes for project 7 |
| a different authoritative value before settlement | write 1; `#0f0f0f` arrives while held; resolve; B → write 2 |
| ordinary acknowledgement after settlement | resolve first; blur+Enter B → 1; ack B → Enter → 1; undo A → B → write 2 |
| no-ack duplicate events | blur/Enter/blur while held → 1; resolve; blur/Enter → 1; different value → 2 |
| failed mutation retry, then a held settlement with ack+undo | refusal, 0 summaries; retry → write 2 same args; ack+undo while held; resolve; refusal gone; B → write 3 |
| existing R1/R2: record / project / field switches with equal colour, in-flight after retarget, unmount, Enter+blur, refusal retry, reapply after ack→undo, draft held while pending | unchanged, passing |

**Discrimination check:** with only the generation comparison removed (R2 `onSettled` semantics
restored), the suite fails exactly the R3 reproduction, the "different authoritative value" case and
the failed-retry case (3 failed / 20 passed); with the fix, 23/23. The temporary revert was undone
from a scratch copy (hash above).

| Command | Result | Log |
|---|---|---|
| `npx vitest run src/components/{ColorField,FieldProfile,ArchiveMemberReplace,TeamLogoCard,TextureReplace,LeagueImageryCard}.test.tsx` | 6 files, **47 passed**, exit 0 | [r3-focused-ui.log](../../validation/evidence/overnight-development-2026-09-21/logs/r3-focused-ui.log) |
| `npx tsc --noEmit -p .` | clean, exit 0 | [r3-tsc.log](../../validation/evidence/overnight-development-2026-09-21/logs/r3-tsc.log) |
| `npx vitest run` (one full run, complete output retained) | **20 files, 156 passed**, exit 0 | [r3-frontend-full.log](../../validation/evidence/overnight-development-2026-09-21/logs/r3-frontend-full.log) |

Not run: Rust suites, bindings check (no DTO change), fixture sweeps, any launch. Ignored cases: none in these suites.

**The earlier unattributed R2 first-run failure remains unexplained.** This run did not reproduce it;
that is not evidence it was fixed. No log of that failure exists to delete; the full log of this run
is retained above.

## 4. Omissions / review areas

- jsdom evidence only; no desktop reproduction.
- Semantics chosen: the guard is tied to (write, generation). Consequence: after ack-before-settle
  the same hex is caught by `hex === current` (not by `awaitingAck`), and any later authoritative
  move makes it a new edit — by design.
- Codex may want to review whether `authGen` should also gate the summary `update` — not done: the
  shared `ProjectProvider.update` already rejects foreign-project summaries, and a stale same-project
  summary is superseded by the next refresh.

## 5. Cleanup

Scratchpad (`H:\TEMP\claude\…\10937252-…\scratchpad`): `add_r3_tests.py`, `ColorField.fixed.tsx` —
package-owned, deleted at the next checkpoint. No processes left. Logs are durable under
`docs/validation/evidence/overnight-development-2026-09-21/logs/`.

## 6. Next

Priority 1 — foundation audit (canonical dictionary rebuild first), receipt 02.
