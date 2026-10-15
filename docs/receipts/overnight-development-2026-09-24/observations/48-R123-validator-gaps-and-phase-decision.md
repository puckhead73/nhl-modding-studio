# Developer observation — R123 reference-validator gaps, and the smallest open Austrian phase decision (28 Sep 2026, receipt 48)

This note is for research. It is recorded by the developer, edits no research file, and is not a contract change.
Studio's production checks are in receipt 48. This note lists where the research reference validator
(`r123_schedule_generation_contract.py`) accepts inputs that Studio now refuses, so the reference can be tightened
if research wishes.

## 1. Validator gaps (production refuses; the reference accepts)

1. **Odd-host rule value (Q11).**
   - The reference checks that `odd_home` is present, not what it says. The coordinator's independent request
     with `odd_home = "invalid-rule"` passed both the request and the proposal checks.
   - Studio accepts only `lower_slot`, `higher_slot`, `balance` and `per_pair`. Any other value is refused before
     anything is built. The API test sends `invalid-rule`, the empty string, `LOWER_SLOT` and `home`.
   - Studio's API used to read an unknown string as "no rule", which was safe (an odd count was then refused
     under Q11) but misleading.
2. **Per-pair hosts.** R123 allows an explicit per-pair map, but the reference does not check it for
   completeness. Studio refuses a host list that:
   - misses an odd pair;
   - names a pair twice;
   - names an even pair;
   - names a pair that includes a non-participant;
   - is given together with another rule.
3. **R3 (`index == position`).** This describes the stock replay; it is not a write rule. The gen7 index field is
   not updatable. Studio keeps every stored index, and its post-check requires the index at each position to be
   unchanged. It never rewrites an index to its position.
4. **R9 among generated rows only.** Production also refuses a generated game on a date where its club already
   plays a kept game (receipt 41). The post-check counts that as an R9 failure.
5. **Balance rule.** The reference does not define `balance`. Studio's definition (receipt 48):
   - odd pairs are taken in participant order;
   - the extra home game goes to the club with fewer home games so far, counting even halves and extras already
     given, and to the lower league slot on a tie;
   - every chosen host is listed in the plan's notes, with none elided.
6. **Stock facts stay facts.** Studio generates its own dates and refuses new double bookings. It does not treat the
   shipped anomalies R123 lists as universal rules:
   - uneven splits in irregular tables;
   - the NHL 13 Swiss and NHL 15 Finnish same-day double bookings;
   - unsorted Czech and QMJHL tables.

   `place_generated`'s comment was corrected: shipped tables are sorted except those R123 lists.

## 2. The smallest outstanding Austrian phase decision (writer withheld)

Studio still refuses to regenerate the PS4 18–21 `austrianschedule`, because its replaced rows carry round 1. The
stock-groups file does not list it. A phased writer needs, as the smallest set:

1. **The runtime meaning of `round = 1`:** is the tagged block a second phase the game plays after round 0, or
   something else (a placement round, a playoff, or data the mode ignores)? Static evidence shows the block is dated
   after round 0 in all five tables, which is not the same thing. A runtime observation on one title (for example
   NHL 21) would settle it for that title.
2. **Phase-1 participants:** does the game read the round-1 block's clubs from the table, or select them from
   round-0 standings at run time? If the latter, a generated round-1 block must not fix its participants.
   Studio will not derive playoff or participant selection.
3. **Phase-1 dates:** whether the game re-dates the block, or requires it after the last round-0 date (R10 as a
   rule rather than a fact).

Until then, a user can still regenerate the Austrian table as one phase only by accepting the refusal's advice.
Nothing in receipt 48 changes that refusal.
