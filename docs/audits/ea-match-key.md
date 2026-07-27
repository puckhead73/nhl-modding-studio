# EA match key — DOB coverage + name-collision ambiguity (Phase 5.0 Spike S2)

> **Status:** ✅ measured 2026-07-26. **Gates** Phase 5.1 (`EaSlugIndex` / `match_player`).
> **Provenance:** `crates/studio-domain/tests/ea_match_key.rs` (the `#[ignore]` harness
> `ea_match_key_coverage`), run over the committed anchors via the **exact app read path**
> (`EditSession::player_summaries` + `player_view`), so the birthdate is composed with the correct
> per-title probe (year base + the NHL10 month/day 1-index drift), not a re-derived offset.
> **Confidence:** high for the measured anchors; see caveats.

## Question (from the plan)

Matching is by normalized **name + DOB**, but nothing indexes EA players this way today
(`person_slug` is corpus-only). Unknown: how many EA bios carry a **usable** DOB vs a sentinel/zero,
and the resulting ambiguity rate (same name, no DOB). The rate decides how prominent the
disambiguation UI must be.

## Method

For each anchor, enumerate every bio (`player_summaries`), read the composed `header.birthdate`
(`player_view`), and compute:

- **`dob_ok%`** — share whose composed birth **year** lands in a plausible window `[1940, 2010]`. A
  raw-zero bio composes to a ~1900 date (not `None`), and a raw-`0xFF` sentinel to a post-2100 date,
  so "usable DOB" must be detected by plausibility, not `Option::None`. `sentinel` counts bios with a
  present-but-out-of-window date.
- **`name-ambig%`** — share of players sitting in a colliding bucket under the **name-only** fold
  `normalize_name(last)|normalize_name(first)` (the matcher's fallback when DOB is absent).
- **`name+dob-ambig%`** — the residual collision share once a plausible DOB is folded in via
  `tdb_fhm::slug::person_slug` (the real key). Name-only folding is reused verbatim so both sides of
  the eventual join fold identically.

## Results (committed anchors, 2026-07-26)

| anchor | players | dob_ok% | sentinel | name-ambig% | name+dob-ambig% |
|---|---:|---:|---:|---:|---:|
| nhl07/ps3 | 2250 | 100.0 | 0 | 1.2 | 1.0 |
| nhl08/ps3 | 2962 | 100.0 | 0 | 6.3 | 6.1 |
| nhl09/ps3 | 3968 | 99.7 | 10 | 5.1 | 4.8 |
| nhl10/ps3 | — | — | — | — | — (skipped at run time; meta since restored — see caveats) |
| nhl11/ps3 | 5310 | 100.0 | 0 | 3.9 | 3.4 |
| nhl12/ps3 | 5395 | 100.0 | 1 | 4.0 | 3.3 |
| nhl13/ps3 | 5412 | 100.0 | 0 | 4.0 | 3.3 |
| nhl14/ps3 | 5374 | 100.0 | 0 | 3.8 | 3.3 |
| nhl15/ps3 | 5534 | 100.0 | 0 | 3.9 | 3.3 |
| nhllegacy/ps3 | 5632 | 100.0 | 0 | 3.9 | 3.2 |
| nhl15/ps4 | 5555 | 100.0 | 0 | 3.9 | 3.2 |
| nhl16/ps4 | 5632 | 100.0 | 0 | 3.9 | 3.2 |
| nhl17/ps4 | 6128 | 100.0 | 1 | 0.9 | 0.2 |
| nhl18/ps4 | 7236 | 100.0 | 3 | 5.1 | 4.0 |
| nhl19/ps4 | 7136 | 99.7 | 21 | 1.0 | 0.1 |
| nhl20/ps4 | 7136 | 99.7 | 21 | 1.0 | 0.1 |
| nhl21/ps4 | 7247 | 99.4 | 43 | 0.8 | 0.1 |

## Findings

1. **DOB is effectively universal.** Every measured anchor is ≥99.4% plausible-DOB; sentinel/zero
   bios number in the low tens at most (max 43 on nhl21 of 7247). **Name + DOB is a viable *primary*
   high-confidence key** — the corpus side is 100% DOB (`slug.rs`), so both sides of the join carry it.

2. **DOB is not a *unique* key on the EA side.** Folding DOB in reduces collisions but never to zero
   (0.1%–6.1% residual). On several PS3 titles it barely moves the needle vs name-only
   (nhl08: 6.3→6.1, nhl11: 3.9→3.4), because the residual collisions are **genuine duplicate /
   multi-pool bios** (the same real player carried in more than one pool — roster counts of 5000–7000
   far exceed the ~700 active NHL skaters), not DOB gaps. On PS4 (nhl17/19/20/21) DOB is far more
   discriminating (down to ~0.1%).

   ⇒ `match_player` **must return ambiguity-aware candidates (`Vec<MatchCandidate>`), never assume the
   name+DOB key is unique.** This confirms the 5.1 design.

3. **Name-only fallback is safe but rarely needed.** Because DOB is ~universal, the name-only path is
   reached only for the handful of sentinel bios; its own collision rate (≤6.3%) is acceptable for a
   flagged-ambiguous fallback.

4. **The disambiguation UI is first-class, not an edge case.** Since even the primary key collides up
   to ~6% (mostly multi-pool duplicates), the cart's per-item **Match vs Create** + "review all
   matches" affordance must be prominent, not buried — a single auto-pick would silently target the
   wrong pool copy.

## Pinned match key + ambiguity policy (output)

- **Primary key:** `person_slug(last, first, dob)` — `normalize_name` fold on both name parts + a
  plausible composed EA birthdate (`normalize_dob`). Reuse `tdb_fhm::slug` verbatim so the corpus and
  EA sides fold identically.
- **DOB plausibility gate:** treat a composed birth year outside `[1940, 2010]` as **no usable DOB**
  and degrade that bio to the name-only key, flagged ambiguous. (Zero → ~1900, sentinel → >2100.)
- **Candidates, not a winner:** `match_player` returns all bios sharing the key, ranked, with an
  `ambiguous` flag when >1 remains after DOB. The cart decides per item (Match/Create).
- **Fallback order:** name+DOB (high confidence) → name-only (flagged ambiguous, for DOB-less bios) →
  no candidate (→ Create). Mirrors the portrait matcher (Spike P-S2) so the two ranked keys agree.

## Caveats / follow-ups

- **nhl10/ps3 meta restored (2026-07-26).** At measurement time
  `anchors/ps3/nhl10/databases/nhlng-meta.xml` was missing, so nhl10 (the birthdate month/day
  **1-index** title) was skipped. It has since been restored from EA's committed **Xbox360** nhl10
  meta — the t3db meta is platform-invariant (PS3 ≡ Xbox360 metas are byte-identical for every title
  that has both: nhl09/11/14/legacy verified) — and the match is confirmed by `nhl10_meta_smoke.rs`
  (loads, enumerates a plausible roster, composes birthdates, probes 1-indexed). The full S2 table row
  for nhl10 can be filled on the next `--ignored` sweep; coverage/ambiguity are not expected to differ
  in kind from nhl09/nhl11.
- The harness is `#[ignore]` (a full sweep runs >1h because `player_view` rebuilds the entity model
  per call). It is a spike/reproduction tool, not a suite gate. If it becomes a recurring check,
  switch the DOB read to a raw `qFnd`/`JKLp`/`Kswi` read to drop the per-player rebuild.
- Plausibility window `[1940, 2010]` is deliberately generous; tightening it changes only the tiny
  sentinel counts, not the conclusion.
