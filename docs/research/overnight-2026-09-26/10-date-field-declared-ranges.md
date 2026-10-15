# Handoff R116 (P10) — developer observation answered: player birth-date declared ranges versus stored values on all 19 distinct sources — PS4 NHL 16–21 declare month 0..11 / day 0..30, every other source 0..15 / 0..31 (same 4 / 5-bit widths); stored values are **1-based on NHL 07–10** and **0-based from NHL 11** on every platform; the declared range tells the family apart but not the index base, and on most titles it admits values that are not dates

Research lane, 26 September 2026, published 03:03 MDT (system clock). Consumes the observation of claim `dev-f1f2b-review-corrections`
(receipt 23): "PS4 NHL 16–21 metadata declare month 0..11 / day 0..30; every other anchor declares 0..15 / 0..31 (same widths)". Claim
`research-p10-date-range-observation`. Read-only; Studio source read, not edited.

Machine-readable: [`data/r116-date-field-declared-ranges.json`](data/r116-date-field-declared-ranges.json) (per distinct source and every
player-bio table: declared depth / rangelow / rangehigh of `month`, `day`, `year`, stored min / max / distinct over current rows).

## Plain-language summary

The developer noticed that the newest PlayStation 4 titles describe the birth month and day fields as 0–11 and 0–30, while every
older file describes them as 0–15 and 0–31. That is confirmed on every source: the fields are the same size everywhere, and only the
declared limits differ. The values actually stored agree with Studio's existing rule — NHL 07 to NHL 10 store January as 1, later titles
store it as 0 — and the declared limits cannot tell those two apart. On most titles the declared limits would also accept a "month 15"
or, on NHL 11 onward, a "day 31" that is not a real date, so an editor has to check calendar validity itself.

## 1. Measured (exhibition player-bio table; other bio tables agree where populated)

| Sources | declared `month` · `day` (depth, low..high) | stored `month` · `day` | index base |
|---|---|---|---|
| NHL 07 (gen6), NHL 08, 09, 10 (X360 = PS3) | 4 b 0..15 · 5 b 0..31 | 1..12 · 1..31 (12 / 31 distinct) | **1-based** |
| NHL 11–15, Legacy (X360 = PS3), PS4 15 and PS4 15 patch 00 | 4 b 0..15 · 5 b 0..31 | 0..11 · 0..30 | 0-based |
| PS4 16, 17, 18, NHL 19 base, NHL 19 patch 56, real NHL 20, NHL 21 | 4 b **0..11** · 5 b **0..30** | 0..11 · 0..30 | 0-based |

Distinct metas: the PS4 16–21 metas are separate files per title (hashes in the JSON); NHL 19 patch 56 and NHL 15 patch 00 use their base
meta (patch archives carry none). `year` is 6 b (NHL 07), 7 b (08), 8 b afterwards, all declared 0..2^w−1.

## 2. What it implies

- **Agrees with Studio's contract** (`crates/studio-domain/src/encoding.rs`: families Gen6Nhl07 / Gen7Nhl08–10 → `OneIndexed`,
  Gen7Nhl11ToLegacy / Ps4 → `ZeroIndexed`; `schema_family` accepts exactly the two declared pairs (4, 0, 15)+(5, 0, 31) and
  (4, 0, 11)+(5, 0, 30)). No research correction needed.
- **The declared pair is a family signature, not an index-base indicator:** NHL 10 (1-based) and PS4 15 (0-based) declare identical
  ranges.
- **Encodable ≠ valid:** on every source except PS4 16–21 the declared range admits months 12–15 (0-based) or 13–15 and 0 (1-based), and
  day 31 on 0-based titles; validity is a calendar rule (and February / 30-day months) above the field range.
- **Small documentation lag (developer-owned, not edited):** the doc comment on `BirthdateEncoding::OneIndexed` in
  `crates/studio-domain/src/assets.rs` reads "NHL 10 only", while the contract maps NHL 07–10 to it, as measured here.

```
python local/runs/overnight-research-2026-09-26/probes/r116_date_field_declared_ranges.py <out.json>
```
Executed once; output published through `safe_patch.py put`.
