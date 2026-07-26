# Ratings write codec — write is the inverse of read (Phase 5.0 Spike S1)

> **Status:** ✅ proven 2026-07-26. **Gates** Phase 5.2 (the import `lower()` ratings write).
> **Provenance:** `crates/studio-domain/tests/ratings_write_codec.rs` — three round-trip tests over
> real anchors, one per offset regime. **Confidence:** high.

## Question (from the plan)

The importer writes **computed** ratings on the human 36..99 (PS4: 0..99) scale. Is write the exact
inverse of read? The prior notes framed the risk as "PS4 uses offset 0, apply per `Capabilities`" —
this spike pins the real seam and the real hazard.

## What the read path actually does

A stored rating decodes as:

```
human = meta.decode_raw(raw_bits) + edit_offset
```

- `meta.decode_raw` / `meta.encode` handle the **sign/sentinel bit-packing** (raw_bits ⇄ decoded).
- `edit_offset` is the **semantic rating offset**, resolved from the target field's own metadata
  (name suffix + declared range) **and a db probe**, and carried on `FieldView.edit_offset`
  (`view.rs` `read_field`). It is **not** a constant:

  | Regime | Storage range | `edit_offset` | Human scale |
  |---|---|---:|---|
  | NHL 10 → Legacy / 11–21 (PS3/X360) 6-bit | 0..63 | **36** | 36..99 |
  | **NHL07 gen6** 6-bit | 0..63 | **50** | 50..99 |
  | **PS4** 7-bit (value *is* the rating) | 0..99 | **0** | 0..99 |

  (`attributes::rating_offset` returns `36` as a *sentinel for 6-bit storage*; `read_field` swaps in
  the probed `sixbit_rating_offset(db)` = 36 or 50. PS4's `0` is filtered out of `edit_offset`, so a
  PS4 rating carries `edit_offset == None`, i.e. offset 0.)

## The write seam (and the correction to the plan)

`EditSession::set_field(FieldValue::Int(v))` runs `meta.validate(v)` + `meta.encode(v)` — it takes the
**decoded** value (post sign/sentinel, *pre* semantic offset) and validates it against the **raw**
range (0..63 / 0..99). It does **not** apply `edit_offset`; the caller does. So:

> **The import must convert `human → decoded = human − edit_offset(target)` itself, then call
> `set_field(Int(decoded))`, using the offset carried on the target field's `FieldView`.**

Two corrections to the prior planning notes fall out:

1. **No `rating_offset` visibility change is needed.** The 5.2 import module lives in `studio-domain`
   (same crate as `attributes::rating_offset`, already `pub(crate)`), and in any case the right seam
   is the **resolved** `FieldView.edit_offset` (36/50/0), not the `36`-sentinel `rating_offset`.
2. **The hazard is broader than "PS4 = 0."** NHL07 gen6 uses **50**. Any fixed constant is wrong on
   two of the three regimes.

## Proof (three anchors, one per regime)

`ratings_write_codec.rs` reads a skater's first true rating cell (`build_player_attributes` →
`is_rating`), reconstructs `human = decoded + edit_offset`, writes `human − edit_offset` back through
`set_field`, and re-reads the raw bits:

| Test | Anchor | Resolved offset | Round-trip raw-identical? | Fixed-36 hazard |
|---|---|---:|---|---|
| `..._ps3_6bit` | nhl14/ps3 | 36 | ✅ | (n/a — 36 *is* correct here) |
| `..._ps4_direct` | nhl15/ps4 | 0 | ✅ | ✅ fixed 36 corrupts (wrong-by-36) |
| `..._gen6` | nhl07/ps3 | 50 | ✅ | ✅ fixed 36 corrupts (wrong-by-14) |

`write(read(x)) == x` byte-for-byte in every regime **when the offset comes from the target field**;
trusting a constant (the shape of `EaRating.raw_offset`, always 36) silently writes a wrong rating
with no error on PS4 and gen6.

## Pinned rules (output)

1. **Offset source:** the import derives the rating offset from the **target field's**
   `FieldView.edit_offset` (default 0 when `None`). **Never** use `tdb_formula::EaRating.raw_offset`
   or any hardcoded constant for the on-disk write.
2. **Write value:** `set_field(Int(human − edit_offset))`; `set_field` validates against the raw range,
   so an out-of-range human value is rejected, not silently truncated.
3. **Rating classification / `None` fallthrough:** only cells that `build_player_attributes` marks
   `is_rating` are written as ratings. A companion field whose range matches neither `(0,63)` nor
   `(0,99)` is **not a rating** (`rating_offset → None`) and must be **skipped and reported**, never
   written with a guessed offset. A rating field a loaded title lacks → skip per `Capabilities`
   (Correction 2 in the phase-5 plan).

## Residual risk

The failure mode is **silent** (wrong-by-36/14, no error), so the rule above is a hard invariant, not
a preference. The 5.2 anti-pattern grep must reject any import path that writes a rating from a
constant offset or from `EaRating.raw_offset`. If a computed rating ever falls outside the target's
raw range after offsetting, prefer **skip-and-report** over clamping.
