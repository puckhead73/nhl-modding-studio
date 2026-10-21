# Dashboard and Validation

**Settings › Dashboard** (called **Validation** in a Save Editor session) shows the open project's vital statistics and the result of the integrity audit, with repair buttons for the problems Studio can fix itself. The red **N errors detected** pill in the header is a shortcut to this page; it appears only for error-level findings.

## Statistics

Platform, role, source, the number of tables, the file size, whether the header CRC checks out (**OK** or **MISMATCH**) and whether there are unsaved changes. A problem with the attached dictionary shows as an amber line.

## Validation

**Re-run audit** repeats the whole-database audit. The status reads **Clean — safe to write** or **Errors present**. Severity chips (errors, warnings, infos) toggle what is shown. Findings are grouped by category, worst first, each with its location, a message, and **Technical details** (the check that fired, the location and the detail).

Repairs:

- **Fix duplicate jerseys**: keeps the first wearer of each duplicated number and gives the others the lowest free number.
- **Repair roster blocks**: re-orders roster slots into contiguous per-team blocks ("Rebuilt the roster layout: N slots moved back into place…"). See [Roster Model](Roster-Model) for why this matters.

Both run automatically when a project opens ([Core Concepts](Core-Concepts)); here you can run them again after your own edits. Error groups without a repair are tagged **Manual fix**. A repair is staged as a pending edit: **Save Session** to keep it, or undo.

## What the audit checks

Three layers run at different times: each field's declared range when you edit it; entity invariants (companion rows present, slots coherent, keys unique) around every operation, which roll an operation back if it would break one; and the whole-database audit on save and on demand here. The audit never changes anything by itself.

Last reviewed: 2026-10-20
