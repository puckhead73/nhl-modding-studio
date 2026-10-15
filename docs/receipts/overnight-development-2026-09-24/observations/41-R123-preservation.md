# Developer observation — R123 R2 / R3 / R9 as Studio now implements them (27 Sep 2026, receipt 41)

For research, recorded by the developer; it does not edit research files. It is not a contract change.

1. **R2 (kept rows unchanged in value and position): now enforced by both generators.** The earlier
   `merge_by_date` moved kept rows and rewrote their index. It was replaced (receipt 41). Evidence: the red run
   shows the old behaviour and the green run the new one; both are logged under `logs/r41-*`.
2. **R3 (`index` = position): observation.** On the gen7 layouts the index field (`vfEq` on NHL 07 X360) is not
   updatable through an edit. Studio's generators therefore keep each position's stored index and never write
   it. Where a source already has index ≠ position, Studio keeps it and the existing validation warning
   counts it. R3 holds on every stock table that research's replay covered.
3. **R9 (no slot twice on a date): Studio enforces a stronger rule.** A generated game never shares a date with a
   kept game of either of its clubs; the matchday moves forward a day, or the plan is refused. Double bookings
   among kept rows only are reported as pre-existing.
4. **Date order around a kept row (not an R-code): Studio's conservative choice.** Games placed before a kept row
   are dated no later than it, and games after it no earlier. Matchdays may move later; they never move
   earlier. Otherwise the plan is refused. This is not claimed as a runtime requirement.
5. **Open question for research, if useful:** does any shipped league table contain rows involving a
   non-participant under the default participants (research §4 says none), other than NHL All-Star rows? If
   none, a real kept row occurs only when a user deselects clubs. In that case the refusal documented in
   receipt 41 §4 (29 of 30 clubs) is the typical outcome for a dense table.
