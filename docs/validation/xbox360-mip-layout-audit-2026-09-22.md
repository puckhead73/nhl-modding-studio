# Xbox 360 `RW4xb2` mip-layout audit — 22 September 2026, restated 23 September 2026

**Developer-owned audit; developer-verified; not Codex-accepted; nothing game-verified.** Ordered by
the coordinator's fresh-session dispatch (item 4) and research handoff R65 F1; **restated on
23 September 2026 after the coordinator review of receipts 10–17 (F1 / F2)**: the classifier is
declared and applied uniformly, no class is relabelled after the fact, the sampled titles / formats
/ shapes are stated exactly, the comparison's strength is named for what it is, the withdrawn
handoff 17 citation is removed as support, the debug-blue comment is kept only as an attributed
historical note, and the writer now carries an explicit applicability gate and byte-write mask.
Real trees on `X:` read only; every write in the tests below went to memory or a disposable copy.

> **Supersession note (23 Sep 2026).** The 22 September wording "every level larger than 16 × 16
> blocks is where Studio says it is (0 misplaced, 2 borderline)" and "all 2 427 large levels
> verified" is withdrawn. The measurement is unchanged; its classes are now reported as the
> classifier yields them: on the original three families 1 764 large levels are *matched-decisive*,
> 661 *matched-nondecisive* and 2 *unmatched* — the two NHL 11 levels over the threshold were
> previously relabelled "borderline" because they beat the controls; they are unmatched, and
> "beats both controls" is recorded beside the class as data. The 22 September per-family figures
> stay in §8 for the record.

## 1. Plain-language summary

Studio writes an Xbox 360 texture's smaller mip levels one after another, each on its own
32 × 32-block tile grid. Nobody had checked where the console actually keeps those levels, and the
one measurement that looked like it did (research R60, the last non-zero byte in each slot) was
withdrawn on 22 September because a byte count says nothing about *position*.

This audit checks position with pictures instead of byte counts. For 4 700 real shipped rasters
(15 families across 8 Xbox 360 titles) it reads every level at the offset Studio predicts, decodes
it, and compares it with the base image shrunk to that level's size. A level whose decode looks like
the shrunken base is *matched* at that offset; one that does not is *unmatched by this
comparison* — which says the model's offset does not hold a box-filtered copy of the base, not
where the level actually is and not what the GPU reads. Two deliberately wrong offsets are decoded
the same way as controls, so a *matched* level is only called *decisive* when the wrong offsets
match worse; flat or repetitive levels that match anywhere are *nondecisive*, which is not
placement proof.

The result, stated at the classifier's own strength:

- **Large levels (both axes above 64 texels): 9 185 levels, 7 224 matched-decisive, 1 953
  matched-nondecisive, 0 zero, 8 unmatched.** All 8 unmatched are 128 × 128 DXT5 levels of two
  jersey rasters (`texlib_0_11_1` / `texlib_0_12_1`) repeated across NHL 09 / 10 / 11 / 12, with
  `d` between 16.4 and 20.8 against the 16.0 threshold; each beats both controls by more than 30,
  which is recorded as data. They are counted as unmatched.
- **Packed-tail levels (64 texels or less): 14 100 levels, 9 084 matched-decisive, 788
  matched-nondecisive, 0 zero, 4 228 unmatched** (30 %), concentrated at 16 × 16 texels and
  below, with some 32 × 32 and 64 × 64 DXT5 misses. The sequential model does not describe the
  tail.
- **Studio's chain allocation is larger than every shipped slot** (4 700 / 4 700 rasters).

**What changed because of it (22 Sep, corrected 23 Sep).** The Xbox 360 Replace… route is now
bounded by what the audit covers, decided from the file's own bytes before anything is encoded:

- **same size** — a *partial* replacement: the base is rewritten; for a raster inside the audited
  population (§5) the large levels too, at the sequential offsets; every packed-tail level keeps the
  shipped bytes. The dialog shows this before Apply and the result repeats it word for word;
- **changed size on a mipped raster** — **refused** before backup, write or any other mutation; the
  chain is not shortened to dodge the refusal;
- **changed size on a single-level raster** — one level re-emitted under the descriptor rule, as
  before;
- rasters outside the audited population (other formats, shapes, descriptor forms, cubes) take a
  **base-only** partial replacement and say why.

Establishing the packed-tail contract stays the research lane's first priority (Q-R65-1); the
codec-level chain writers remain available outside the Replace flow (§6) and are not qualified
coverage.

## 2. Method

`crates/tdb-rx2/tests/rw4xb2_mip_placement.rs` (`--ignored`, real trees):

| Step | Detail |
|---|---|
| Population | tiled 2D DXT rasters ≥ 256 × 256 with ≥ 2 declared levels; cubes excluded (slot ≥ 6 × chain); first N per family in sorted file order (N = 400 for the Legacy families, 300 elsewhere) |
| Model under test | level `k` at `Σ_{i<k} tiled_buffer_size(w_i, h_i, bpb)` — the offsets `replace_with_mip_chain` and `replace_with_resize` write to |
| Read | slot bytes at the predicted offset, `swap16` + `untile_dxt` for the level's own dimensions, a self-contained BC1 / BC2 / BC3 colour decode — **RGB only; DXT3 / DXT5 alpha is not compared** |
| Reference | the shipped base level (Studio's validated de-tile) box-downscaled by `2^k` — an approximate stand-in for EA's authoring filter, not a reconstruction |
| Decision value | `d` = mean absolute RGB difference, 0–255, kept unrounded (printed to three decimals) |
| Controls | the same decode one 4 KiB page earlier and one later |
| Split | **large** = both axes > 16 blocks (> 64 texels); **tail** = at or below 16 blocks. The split is what this audit's own content comparison separates; no other measurement is cited for it |
| Allocation | Studio's padded chain total at the shipped shape vs `Texture::buffer_capacity` |

### Declared classifier (applied uniformly to every level; nothing is relabelled afterwards)

| Class | Rule | What it means, and no more |
|---|---|---|
| matched-decisive | `d ≤ 16.0` and every control differs by more than `d + 8.0` | content-located at the predicted offset under an approximate box-filter comparison |
| matched-nondecisive | `d ≤ 16.0` but a control is within the margin | the level matches at the predicted offset **and** at a neighbour — flat or repetitive content; **not positive placement proof** |
| unmatched | `d > 16.0` | the predicted offset does not hold a box-filtered copy of the base. It does not say where the level is; a legitimate authoring filter could in principle produce `d > 16` on a correctly placed level. Whether the level still beat both controls is recorded beside the class as data and never moves it |
| zero | the level bytes at the predicted offset are all zero | — |

The threshold `16 / 255` and margin `8` are declared constants (`MATCH_THRESHOLD`,
`CONTROL_MARGIN`), not tuned per family; the p50 of matched levels is 1.2–2.5.

## 3. Results — 23 September 2026 run (restated classifier, 15 families, 8 titles)

`docs/validation/evidence/overnight-development-2026-09-23/logs/r4-xb2-mip-placement-audit-restated.log`
(832 s). Every raster carries the NHL 09–Legacy descriptor form `DW0 = 0x00400003`. Shapes are
the base dimensions; every audited raster is square.

| Family | Rasters | Shapes (format, base, rasters) | Large: n / decisive / nondecisive / **unmatched** (beat controls) / d p50 / max | Tail: n / decisive / nondecisive / **unmatched** (beat controls) / d p50 / p90 / max |
|---|---|---|---|---|
| `legacy/jersey` | 400 | DXT1 512² ×100, DXT1 1024² ×200, DXT5 512² ×100 | 1000 / 580 / 420 / **0** (0) / 1.168 / 15.541 | 1200 / 822 / 147 / **231** (216) / 6.223 / 79.451 / 134.773 |
| `legacy/player` | 400 | DXT1 512² ×202, DXT3 256² ×198 | 602 / 556 / 46 / **0** (0) / 2.111 / 3.434 | 1200 / 797 / 44 / **359** (215) / 3.495 / 89.788 / 154.508 |
| `nhl11/jersey` | 300 | DXT1 512² ×75, DXT1 1024² ×150, DXT5 1024² ×75 | 825 / 628 / 195 / **2** (2) / 1.477 / 20.773 | 900 / 547 / 51 / **302** (229) / 8.816 / 65.171 / 133.018 |
| `nhl11/player` | 300 | DXT3 256² ×300 | 300 / 296 / 4 / **0** (0) / 1.991 / 4.334 | 900 / 590 / 16 / **294** (38) / 4.807 / 60.955 / 105.417 |
| `nhl09/jersey` | 300 | DXT1 512² ×75, DXT1 1024² ×150, DXT5 1024² ×75 | 825 / 638 / 185 / **2** (2) / 2.464 / 20.379 | 900 / 505 / 91 / **304** (234) / 9.882 / 74.637 / 143.114 |
| `nhl09/player` | 300 | DXT3 256² ×300 | 300 / 287 / 13 / **0** (0) / 2.048 / 3.411 | 900 / 573 / 45 / **282** (27) / 4.927 / 55.779 / 90.986 |
| `nhl10/jersey` | 300 | DXT1 512² ×75, DXT1 1024² ×150, DXT5 1024² ×75 | 825 / 630 / 193 / **2** (2) / 1.489 / 20.773 | 900 / 533 / 63 / **304** (233) / 9.363 / 73.343 / 134.496 |
| `nhl10/player` | 300 | DXT3 256² ×300 | 300 / 290 / 10 / **0** (0) / 1.942 / 3.719 | 900 / 583 / 30 / **287** (33) / 4.638 / 56.920 / 105.417 |
| `nhl12/jersey` | 300 | DXT1 512² ×75, DXT1 1024² ×150, DXT5 512² ×75 | 750 / 577 / 171 / **2** (2) / 1.639 / 19.825 | 900 / 544 / 49 / **307** (235) / 9.477 / 72.938 / 133.192 |
| `nhl12/player` | 300 | DXT3 256² ×300 | 300 / 295 / 5 / **0** (0) / 1.990 / 4.340 | 900 / 592 / 14 / **294** (48) / 4.950 / 62.884 / 105.327 |
| `nhl13/jersey` | 300 | DXT1 512² ×75, DXT1 1024² ×150, DXT5 512² ×75 | 750 / 572 / 178 / **0** (0) / 1.938 / 15.532 | 900 / 598 / 42 / **260** (226) / 6.907 / 75.452 / 137.020 |
| `nhl13/player` | 300 | DXT1 512² ×158, DXT3 256² ×142 | 458 / 452 / 6 / **0** (0) / 2.021 / 4.292 | 900 / 601 / 2 / **297** (153) / 3.387 / 93.890 / 107.345 |
| `nhl14/jersey` | 300 | DXT1 512² ×75, DXT1 1024² ×150, DXT5 512² ×75 | 750 / 534 / 216 / **0** (0) / 1.811 / 15.541 | 900 / 599 / 46 / **255** (229) / 6.497 / 76.279 / 137.020 |
| `nhl14/player` | 300 | DXT1 512² ×150, DXT3 256² ×150 | 450 / 433 / 17 / **0** (0) / 2.174 / 4.241 | 900 / 594 / 23 / **283** (142) / 3.554 / 89.360 / 103.580 |
| `nhl15/jersey` | 300 | DXT1 512² ×75, DXT1 1024² ×150, DXT5 512² ×75 | 750 / 456 / 294 / **0** (0) / 1.167 / 15.541 | 900 / 606 / 125 / **169** (160) / 6.498 / 76.279 / 134.773 |
| **all** | **4 700** | DXT1 / DXT3 / DXT5; 256², 512², 1024² | **9 185 / 7 224 / 1 953 / 8 (8) / 1.903 / 20.773** | **14 100 / 9 084 / 788 / 4 228 (2 418) / 5.654 / 74.589 / 154.508** |

Chain allocation: 0 equal, **4 700 Studio larger**, 0 Studio smaller.

The 8 unmatched large levels, exactly: `jersey/texlib_0_11_1.rx2` and `jersey/texlib_0_12_1.rx2`,
raster 0, level 3 of the 1024² DXT5 chain (128 × 128) on NHL 09 / 10 / 11 (`d` 20.379 / 20.773 /
17.289 / 17.524), and level 2 of the 512² DXT5 chain (128 × 128) on NHL 12 (`d` 19.825 / 16.408);
controls 51–56 in every case. The same two source images recur across titles. They are counted as
unmatched; the box-filter comparison's approximate nature (§2) is the reason they are not called
misplaced either.

Per-shape totals are in the log (`shape …` lines per family). The unmatched tail levels are 16 × 16
levels and 32 × 32 / 64 × 64 DXT5 levels of 512² / 1024² jerseys; the first tail level often
matches, which is exactly why a byte-extent measurement could not see the problem.

## 4. Supported versus unqualified (the record the dispatch asked for; restated 23 Sep)

| Path | Evidence | Status |
|---|---|---|
| Base level tiling / placement (offset 0, `XGAddress2DTiledOffset`) | every preview Studio draws; the same-pixel round trips of D6; this audit's reference decode | **supported** (content-verified) |
| Large-level placement (both axes > 64 texels) under the sequential model, **inside the audited population** (§5) | this audit: 9 185 levels on 8 titles, 7 224 matched-decisive, 1 953 matched-nondecisive, 8 unmatched (all beating controls), 0 zero | **supported at this strength** — a box-filter content match at the predicted offset (developer); nondecisive levels are not positive proof |
| Descriptor rule (`DW7` pitch, `DW9`, `DW11`, `DW12` page = tiled base / 4 096, `DW13`) | research 02 §2 census, Studio's census test (100 % of gated descriptors on five trees) | **supported** (field values; says nothing about sub-level positions) |
| Slot-size oracle per shipped shape | research 02 §6 | **an allocation record only** — Studio's chain total exceeds it on 4 700 / 4 700 audited rasters |
| Packed-tail placement (any axis ≤ 64 texels) under the sequential model | this audit: 4 228 of 14 100 tail levels unmatched (30 %) | **unqualified — contradicted by content on a substantial share**; never written by the Replace flow |
| Changed-size chain of a mipped raster | would place tail levels under the unqualified model | **refused** by the Replace flow before any mutation (`Xb2DxtPolicy::ResizeRefused`); public-route regression `loose_texture_route.rs::a_changed_size_write_into_a_mipped_xbox_raster_is_refused…` |
| Changed-size single-level raster | descriptor rule + base placement | **supported** as before (one level re-emitted); runtime card R32-XB2-1 unchanged |
| Same-size partial replacement, large-level route | §5 gate + the audit | **implemented, developer-tested**; disclosed before Apply and in the result as *not a full-image replacement* |
| Same-size partial replacement, base-only route (outside the gate) | base placement only | **implemented, developer-tested**; disclosed with the excluded property named |
| Non-tiled, cube, < 128-texel rasters, NHL 07 container | refused by the rule (unchanged) | refused |

## 5. Applicability gate and byte-write mask (23 Sep 2026)

`Texlib::large_level_support(idx)` decides, from the file's own bytes, whether the large-level
route applies. **Inside the gate** (all of): tiled; `DXT1` / `DXT3` / `DXT5`; descriptor form
`DW0 = 0x00400003`; two or more declared levels; both axes 256–1024 texels; not a cube (slot
< 6 × chain). Anything else — `8_8_8_8`, DXN / CTX1 (not parsed as DXT), linear, the NHL 07 / 08
form, single-level, other shapes, cubes — takes the base-only route with the excluded property
named. The gate is by **form**: it cannot tell NHL 13 from Legacy, so a raster of a title not in
§3 passes it by form, and §3 lists which trees were actually run. Narrow / non-square shapes were
not present in the audited families; they are outside the gate only where an axis is outside
256–1024, and a non-square raster inside those bounds passes by form without a run of its own — a
stated limitation, not a claim.

`KeepTailWrite::written_ranges` is the exact byte-write mask: the `[start, end)` file ranges
written, contiguous from the slot start (base, then each large level in order). The unit cases
`keep_tail_writer_rewrites_only_the_content_verified_levels` and
`keep_tail_writer_falls_back_to_base_only_outside_the_audited_population` assert every byte outside
the mask equals the input on the large-level route, the base-only route (128² chain; 512² chain with
the NHL 07 / 08 form) and the tail-sized single-level raster; the public-route regression asserts it
again on the written file (packed tail, the other raster, the header). Dimensions, format, mip
declarations and slot identity are unchanged by construction (the descriptor is not touched).

The disclosure text is one function (`xb2_policy_notes`) used by both the pre-Apply plan
(`texture_replace_plan`) and the write result, and the seam test asserts the two are equal word for
word. It states that the kept levels still carry the previous image's content and that this is not
a full-image replacement; it does not predict what the console shows at any distance.

## 6. Code (22 Sep, revised 23 Sep)

| Path | Change |
|---|---|
| `crates/tdb-rx2/tests/rw4xb2_mip_placement.rs` | the content-based placement audit (real trees, `--ignored`); 23 Sep: declared classifier (`classify`), unrounded values, per-shape tallies, 15 families, `unmatched … beat both controls` recorded as data |
| `crates/tdb-rx2/src/texlib.rs` | `is_packed_tail_level`; 23 Sep: `LargeLevelSupport`, `Texlib::large_level_support`, `AUDITED_MIN_DIM` / `AUDITED_MAX_DIM`, `Texlib::declared_mip_count`, `Texlib::header_size`; `KeepTailWrite { support, written_ranges, … }`; `replace_large_levels_keep_tail` takes the base-only route outside the gate; the handoff 17 citation removed; the debug-blue text kept only as an attributed historical note |
| `crates/studio-domain/src/assets.rs` | 23 Sep: `Xb2DxtPolicy`, `xb2_dxt_policy`, `xb2_policy_notes` (shared by writer and plan); `replace_texlib_texture_noted` refuses a changed size on a mipped raster before encoding, re-emits one level for a single-level raster, and asserts the writer matched the policy; `plan_texture_replace` / `TextureReplacePlan` |
| `crates/studio-api/src/{dto,registry,commands,lib,bindings}.rs` | 23 Sep: `TextureReplacePlanDto`, `Registry::texture_replace_plan`, the `texture_replace_plan` command |
| `crates/studio-api/tests/loose_texture_route.rs` | 23 Sep: the public-route regression (refusal before backup / write; same-size partial with plan = result; single-level resize; containers without a pre-check) |
| `app/src/components/TextureReplace.tsx` (+ `.test.tsx`), `app/src/lib/client.ts`, `app/src/bindings/TextureReplacePlanDto.ts` | 23 Sep: the dialog asks the plan once both previews are decoded; a refusal disables Write with the reason; partial-replacement notes are shown before Write and repeated with the result; an unavailable plan is said so and does not block the writer's own rules |

**Retained outside the Replace flow, not qualified coverage:** `Texlib::replace_with_mip_chain`
(no Studio caller) and `Texlib::replace_with_resize` for multi-level chains (called by the external
headmesh crate `tdb-headmesh` and by the census / placement tests, never by the Replace flow for a
mipped raster). They are the model under test, kept for research and for that separate workstream.

## 7. Limitations

- Content comparison cannot tell *where* an unmatched tail level actually is — only that the
  predicted offset does not hold a box-filtered copy of the base. Deriving the packed layout is the
  research lane's Q-R65-1 (its H1 candidate is not consumed here); this audit gives it a diagnostic
  with declared classes and a population.
- Eight titles, fifteen `jersey/` / `player/` families; `fe/` and `logo/` rasters not audited.
  Formats DXT1 / DXT3 / DXT5 only; shapes 256², 512², 1024² only, all square; alpha not compared.
- A matched-nondecisive level says nothing about placement; 21 % of the large levels are in that
  class (flat jersey regions), so the large-level claim rests on the 7 224 decisive levels.
- Keeping the shipped tail means a same-size replacement is partial: the kept levels carry the
  previous image's content. The dialog and the result say so; what the console renders from them is
  not observed here.

## 8. The 22 September run, for the record (superseded classifier labels)

`docs/validation/evidence/overnight-development-2026-09-22/logs/r3-xb2-mip-placement-audit.log`
(≈ 4.5 min, three families). The labels "placed / ambiguous / misplaced" of that run are the
classes now called matched-decisive / matched-nondecisive / unmatched; the numbers are the same as
the first three rows of §3.

| Tree / family | Rasters | Large levels: placed / ambiguous / zero / misplaced | Tail levels: placed / ambiguous / zero / misplaced |
|---|---|---|---|
| Legacy X360 `player/` | 400 | 602: 556 / 46 / 0 / 0 | 1 200: 797 / 44 / 0 / 359 |
| NHL 11 X360 `jersey/` | 300 | 825: 628 / 195 / 0 / 2 | 900: 547 / 51 / 0 / 302 |
| Legacy X360 `jersey/` | 400 | 1 000: 580 / 420 / 0 / 0 | 1 200: 822 / 147 / 0 / 231 |

The 22 September text called the two NHL 11 levels "borderline matches, not contradictions"; that
relabelling is withdrawn (they are unmatched, beating controls). It also cited research handoff 17
§3 for the tail split and "the PR11 in-game finding: debug-blue players at gameplay distance" as
corroboration; the former is withdrawn as support (handoff 17's extent data is invalid for its
intended slots per research R65), and the latter is a source comment introduced with commit
`99ce2a5` whose original evidence is not in this repository — historical context only.

## 9. Checks

| Run | Result |
|---|---|
| `cargo test -p tdb-rx2 --test rw4xb2_mip_placement -- --ignored` (23 Sep, 15 families) | 1 passed, 832 s — the log above |
| `cargo test -p tdb-rx2 --lib` | 120 passed, 26 ignored (private-fixture cases) |
| `cargo test -p studio-domain --lib` | 216 passed, 0 failed, 2 ignored (before the 23 Sep kit / text-key additions; see the 23 Sep receipts for the later totals) |
| `cargo test -p studio-api --test loose_texture_route` | 3 passed, 5 ignored (private fixtures) |
| `npx vitest run src/components/TextureReplace.test.tsx` | 10 passed |
