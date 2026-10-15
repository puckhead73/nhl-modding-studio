# DB-06 — NHL07 Xbox360 Arena Editing

Status: **PASSED — user sign-off, 2026-10-11.** All nine supplied review checks passed. DB-06 is closed for the verified NHL07 Xbox360 stock-arena scope. [Review project, evidence and instructions](../validation/db-06-nhl07-review-2026-10-11.md). DB-05's non-asset workflows are accepted; its asset review remains with DB-08.

## Implementation

1. Inventory the actual NHL07 arena selectors, stock/custom sources, field meanings and team links. Distinguish populated source data from empty mode-local tables and unused capacity.
2. Provide a clean arena editor for name, city, capacity, attendance and ice type. Use named dropdowns for established enums and existing venue assignments. Protect identity keys and unsupported selectors.
3. Resolve shared arenas by their underlying source record. List every affected team, keep Team Data's home-arena selector coherent, and make shared edits explicit.
4. Surface existing presentation, art and audio assignments as choices identified by source venue, without claiming a file or sound has been independently verified. Full texture, mesh and audio authoring remains AA-02/AUD-01.
5. Integrate edits with the shared write queue, immediate feedback, Undo/Redo, Overview links and per-edit restoration, save/reopen, developer archives, build/install and exact restoration.
6. Qualify creation/duplication only if an allocation, selector registration and asset dependency contract is demonstrated. Empty/custom capacity alone is insufficient; otherwise document the unsupported state without a placeholder creation action.
7. Exercise production APIs against disposable projects, run targeted UI/regression checks, and prepare a reproducible review project and one consolidated native/UI checklist.

## Acceptance

The user verifies the supported controls, shared-team effects, undo/revert, persistence and release round trip. Native checks use a fresh game/mode where necessary. File correctness does not establish geometry, audio playback or save-mode behavior. Unmapped fields, new venue creation and other titles/platforms must not inherit this sign-off.
