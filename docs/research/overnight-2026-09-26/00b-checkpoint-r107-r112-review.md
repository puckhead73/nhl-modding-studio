# Correction checkpoint — review of R107–R112 (`docs/validation/research-20260926-r107-r112-review.md`)

Research lane, 26 September 2026, published 02:37 MDT (system clock at publication). Wording corrections only: no new measurement, no raw
result erased, no census rerun. Documents from this point are written through a validated temporary sibling and then replaced
(`local/runs/overnight-research-2026-09-26/probes/safe_patch.py`: strict UTF-8, no NUL, inserted text present, JSON parses), and every
time in them comes from the system clock at writing.

## Plain-language summary

Three things in yesterday's packages said more than the evidence supports, and are now corrected where they appear. The Swiss groups
were checked against six team-table fields only, so the right statement is "none of those fields records the groups", not "the database
does not record them anywhere". The leagues with uneven schedules have not been shown to copy real seasons; they are simply irregular
shipped schedules for which no general rule was found, and importing them or entering counts by hand remain the ways to rebuild them. The
four-and-six-game groups describe the shipped schedules on the titles measured, not a rule the game is known to follow. For renaming team
art, the pool of about 750 "free" numbers is only free in the database columns and art families that were checked — centre-ice logos,
which are numbered by arena, were left out — and a rename changes the database's checksums as well as the art-id cells. A design for a
same-length rename is still not a working team re-key.

## Corrections placed (all dated, originals struck through, not deleted)

| Finding | Where corrected |
|---|---|
| 1. Swiss absence overstated — R111b inspected `exhibitionteams.conference / division / conferencegroup / divisiongroup / group / leaguegroup` only | handoff 05 (title, header note, summary, §2, §3); handoff 06 (header note, summary, table, §2); R13 note on 2026-09-20 handoff 06; queue C11; RECEIPT; STATUS; claim p5 |
| 2. Shape is not provenance / not an impossibility result; partitions are stock-shaped inputs on the measured sources; dates, order and row counts stay observed data | handoff 05 (header note, §1 AHL wording); handoff 06 (title, header note, summary, four table rows, §2); R13 note; queue C11 (proposal, not an implementation contract) and C12; RECEIPT; claim p6 |
| 3. Rename scope — pool = absence in enumerated DB columns + families; centre-ice logo family excluded as an arena-keyed namespace (boundary); DB byte mask includes the required TDB CRC updates; same-length design ≠ working re-key | handoff 03 (header note, summary, layer E, §2 items 4–5); claim p3 |

The inspected / uninspected location list that finding 1 asks for is published with the Finnish / Austrian check in
[07-group-partitions-field-check-and-contract.md](07-group-partitions-field-check-and-contract.md).

## Coordination read at this checkpoint

Claims: research p1–p6 ready (p3 / p5 / p6 carry revision notes); no active research claim. Developer: all claims under
`local/coordination/overnight-2026-09-24/claims/dev-*` are `complete`; the newest, `dev-f1f2b-review-corrections` (receipt 23), consumes
no research and carries one observation for research — "PS4 NHL 16–21 metadata declare month 0..11 / day 0..30; every other anchor
declares 0..15 / 0..31 (same widths)" — queued for this session (receipt item). The development review of packages 20–22 names the
archive-backed PS3 NHL 09 / 10 / 12 / 13 bank listing as the developer's next step, which queue row C10 serves.
