# Receipt 13 — R3 / Teams 2.4: the different-dimension replacement route swept over every gen7 team-logo package

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Private fixtures (`E:\nhl-studio-fixtures\teamlogos`,
hash-checked) read only; every write went to a disposable temp copy that removes itself. Nothing
staged, committed, pushed, launched or written live. Claim: `dev-r3-team-logo-dimension-sweep`.
Dispatch: item 5 of the coordinator review of 2026-09-22 (Teams: assets). No new research consumed
— this closes the Phase 2 matrix's "impl, test missing — team packages **not swept**" cell with
the existing route and the existing fixture census (C9, 20 Sep).

## 1. Plain-language summary

Studio could already replace a team logo with an image of a *different* size on the newer
Xbox 360 / PS3 titles (the code was written for league logos and proven there), but nobody had run
it across the team-logo packages, so the matrix honestly said "test missing". This package runs it
across all 168 gen7 team-logo packages in the fixture set — every family of every title from
NHL 07 to Legacy on both consoles, plus the Legacy PS3 cache copies — and checks each outcome
against what a correct result must look like.

The outcomes fall into exactly three groups, and no package fell outside them:

- **NHL 07–11 (Xbox 360) and NHL 08–11 (PS3): refused, by name.** Those packages wrap their
  texture in forms the package route keeps same-size (RefPack'd whole packages and the older
  RenderWare rasters inside a BIGF), and every one of them says so — "same-size only" — instead of
  writing something.
- **NHL 12–Legacy on both consoles: resized where the family's quad is the texture's own size.**
  The small, wide and reflection families resize on every package; the main and dynasty families
  resize on the packages whose on-screen quad is the full texture and refuse the rest as display
  crops; the calendar family is a display crop on every title and refuses every time. Each success
  re-opens at the new size with the imported picture, every other member of the package is
  byte-identical, the trailer and tail are untouched, and the geometry record carries the new size.
- **Nothing panicked, nothing succeeded silently on a same-size-only form, and no refusal was of an
  unknown kind.**

What this does *not* say: whether the console draws a resized team logo correctly. That is still
runtime card T-3 (which screen draws which family) and the archive cards R2-BIG4-1 / R2-EB-1.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/tests/team_logo_dimension_sweep.rs` | **new** — the `--ignored` sweep over `teamlogos/manifest.json`: identity check, decode, two dimension targets per package (Xbox `8_8_8_8` XPR2: 64 ↔ 256 and doubled height; DXT / PS3: 2× and half), success invariants (size, pixels MAD ≤ 14, other members byte-identical, trailer / tail, Apt size words), refusal classification by the route's own reasons, a per-family outcome matrix, and a failure on any panic or unclassified error. |

No production code changed. The route under test is `replace_texture_in_file` as delivered in
C6 (20 Sep) and audited in P1 (21 Sep).

## 3. Evidence

`NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test team_logo_dimension_sweep -- --ignored`
(14.5 s) — [r3-team-logo-dimension-sweep.log](../../validation/evidence/overnight-development-2026-09-22/logs/r3-team-logo-dimension-sweep.log):
**1 passed**, 168 packages, 0 unexpected outcomes.

| Titles | Families | Packages | Outcome |
|---|---|---|---|
| X360 NHL 07 / 08 / 09 / 10 (RefPack'd packages, `RW4xb2` inside) and NHL 11 (BIGF, `RW4xb2` inside) | teamlogos, -small, -wide, -calendar, -dynasty, -broadcast (08 / 09), misc (08 / 09) | 44 | **every target refused: "same-size only (package form)"** |
| PS3 NHL 08 / 09 / 10 (BIG4, `RW4ps3`) and NHL 11 (loose, `RW4ps3`) | the same families | 41 | **every target refused: "same-size only (package form)"** |
| X360 NHL 12 / 13 / 14 / 15 / Legacy (XPR2 `8_8_8_8` / DXT5) | teamlogossmall, teamlogoswide, teamlogosreflection (13+) | 14 | **resized** on every package (both targets) |
| | teamlogos | 14 | **resized** on 9 (the packages whose Apt quad is the texture's own size); 5 refused as **display-crop quad** |
| | teamlogosdynasty | 10 | resized on 5; 5 refused as display-crop quad |
| | teamlogoscalendar | 5 | **refused** on every title: display-crop quad |
| PS3 NHL 12 / 13 / 14 / 15 / Legacy (DDS) and the Legacy `cache.big` members | the same families | 40 | the **same pattern** as the Xbox 360 rows, family for family (small / wide / reflection resize everywhere; teamlogos 12 of 17 + dynasty 6 of 12 resize; calendar refuses) |

Every success passed all invariants: the file re-opens at the requested size, the decoded pixels
match the imported gradient within the DXT bound, every member other than the texture, its
geometry record and member 0 is byte-identical, the trailer and tail are unchanged, and the Apt
size words (integers and floats) equal the new size on every package that carries the record
(163 of 168 do; the 5 `misc` packages carry none and were all same-size-only refusals).

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-domain --test team_logo_dimension_sweep -- --ignored` (private fixtures) | **1 passed** (168 packages) |

## 5. Coverage effect (Phase 2 matrix 2.4, "Different dimensions")

| Column | Before | After |
|---|---|---|
| X360 07 | blocked (RW4xb2 descriptor) | **refused by name, tested** — the RefPack'd package form is same-size only (5 packages) |
| X360 08–11 | blocked (RW4xb2) | **refused by name, tested** — 39 packages, every target "same-size only" |
| X360 12–15 / Legacy | impl, test missing — team packages not swept | **impl+tested (dev)** — 43 packages: small / wide / reflection resize on every package; teamlogos 9 / 14 and dynasty 5 / 10 resize, the rest and every calendar package refuse as display-crop quads |
| PS3 08–10, PS3 11 | blocked (RW4ps3), blocked | **refused by name, tested** — 41 packages |
| PS3 12–15 / Legacy (+ Legacy cache) | impl, test missing — not swept | **impl+tested (dev)** — 40 packages, the same family pattern |
| PS4 | unchanged (receipt 02 / D6 route) | unchanged — not part of this sweep |

## 6. Not done / limitations

- The 07–11 forms remain same-size only. The loose `RW4xb2` resize (21 Sep) and the `RW4ps3`
  resize exist for loose rasters; lifting the package route's same-size rule for those forms is a
  separate package (it needs the RefPack re-wrap and the archive-slot decision, and the Xbox
  `RW4xb2` route now carries receipt 11's tail disclosure).
- "Display-crop quad" is the route's existing refusal: the family's on-screen quad is not the
  texture's own size, so a dimension change would change the crop. Whether a resized crop family
  *could* be offered with a recomputed quad is unmeasured.
- The console's rendering of a resized logo is unverified (cards T-3, R2-BIG4-1, R2-EB-1).

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Temp root `nhlms-teamlogo-dims-<pid>` | removed on drop; verified absent |
| Scratchpad | empty |
| Background processes | none |
| Evidence log `r3-team-logo-dimension-sweep.log` | retained |

## 8. Next action

Phase 3 — Players: the gen7 portrait / head containment card (research handoff 11, R53) and the
PS4 portrait resolution (handoff 20), as read-only Player-page evidence.
