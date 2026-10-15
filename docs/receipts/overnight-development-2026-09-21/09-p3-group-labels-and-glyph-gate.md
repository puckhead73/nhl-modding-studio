# Receipt 09 — Priority 2-C / 3: PS4 pseudo-league group labels (handoff 08) and the static glyph gate (handoff 09)

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
baseline `8d60a6d`. Committed anchors read only. Claim: `dev-p3-group-labels-glyph-gate`.
Research consumed: 2026-09-21 handoff 08 (`research-r36-tournament-groups`) §2–3 and handoff 09
(`research-r34-fonts-encoding`) §1–4 with its `r34-font-glyph-coverage.json`.

## 1. Plain-language summary

Two small things the research settled, now in the product. (1) On the PS4 titles two competition
groups had no name anywhere in the game's text — the flat group shared by the NHL Alumni and
All-Time teams (id 77) and the one holding the Pro-Am template teams (id 78) — and Studio showed
them as "Unnamed group 77 / 78". The research explains they are single-group pseudo-leagues that
never need a displayed name, so Studio now names them from the leagues that carry them
("NHL Alumni / NHL All-Time (single group)"), marks the label as derived, and keeps the research's
evidence class and hypothesis visible. The Spengler / CHL / EBEL / Allsvenskan ids (62–64, 75, 76)
were already named from the dictionary. (2) Editing localization text now tells you, character by
character, whether the game's shipped fonts can draw what you typed: the research measured every
shipped TrueType font's character set per title, and Studio checks a new or grown string against
it — "every character is in the shipped fonts" or "Ł U+0141 not in any shipped font of
ps3-nhllegacy". The practical trap the research names (Polish / Hungarian capitals on the Xbox
360 / PS3 titles) is caught; text containing C1 control characters is refused on input, as the
research asks. Whether a screen's particular font draws a character, and whether the console falls
back to a system font, is still a runtime question.

## 2. What users can now do

- League page (PS4 19–21, 18 for 77): the Alumni / All-Time and Pro-Am groups carry a readable
  name with a "named from its leagues" hint instead of "Unnamed group".
- Dictionary text edit / growth preview (any title with a measured font tree — Xbox 360 09–Legacy,
  PS3 11 / 13 / 14 / 15 / Legacy, PS4 15–21, Recompiled by identity): a static glyph verdict in the
  warnings; C1 controls refused before planning. PS3 08 / 09 / 10 / 12 (fonts in-archive) and
  NHL 07 / 08 say so instead of guessing.

## 3. Implementation

| Component | Change |
|---|---|
| `crates/studio-domain/src/competition_groups.rs` | `ResolvedGroups::derive_unknown_league_group_labels(display)`: a keyless `league`-kind group is labelled "<league> / <league> (single group)" from the leagues carrying it (deduplicated; NHL 19 keys both rows ALUMNI → one name), `label_source = derived-from-leagues`; evidence / note / hypothesis untouched; other kinds keep the unnamed label. |
| `crates/studio-domain/src/leagues.rs` | applied after `ResolvedGroups::resolve` with the catalog's league display (dictionary text when attached, else key). |
| `app/src/pages/LeagueDetail.tsx` | "named from its leagues" hint (title = the rule and its source). |
| `defs/fonts/glyph-coverage.toml` (new) | the 24 trees of `r34-font-glyph-coverage.json` (`source_sha256` recorded): fonts, union count, union ranges; `measured = false` for the four PS3 in-archive trees. Generated from the research JSON, not hand-typed. |
| `crates/studio-domain/src/font_glyphs.rs` (new) | `GlyphCoverageDefs` (load / parse / `tree_for` — Recompiled → `xbox360-nhllegacy`), `check(game, platform, text) → GlyphVerdict {Covered / Missing(chars) / Unmeasured / NoTree, controls}`, `warning()`, `refuse_input_controls`. C0 controls are layout, not glyphs; C1 reported and refused. |
| `crates/studio-domain/src/project.rs` | loads the defs (`glyph_coverage()`); empty from parts. |
| `crates/studio-domain/src/workspace.rs` | retext + growth previews and holds: C1 refusal first, then the glyph verdict line appended to `warnings` (all new / grown texts of a growth op set, joined). |

**Data over prose:** handoff 09 §3's prose says the Polish lowercase forms are covered on gen7; its
measured union ranges (which the gate uses) do not contain `ą ć ł ń ś ź ż` — only the Czech
Extended-A letters. The gate follows the JSON; the unit test records the difference.

## 4. Regression evidence (developer)

| Case | Asserts |
|---|---|
| `studio-domain/tests/ps4_groups.rs` (`…ids_77_and_78_are_labelled_from_their_leagues…` + the per-anchor evidence walk + the membership walk) | NHL 21: 77 = "TXT_LEAGUE_NHL_ALUMNI / TXT_LEAGUE_NHL_ALL_TIME (single group)", NHL 19: one name; 78 = "TXT_LEAGUE_PROAM (single group)"; `label_source = derived-from-leagues`; evidence stays `unknown`; `group_label` and the team view's `conference_label` carry it through edit / undo / redo / save / reopen; any other keyless group stays "Unnamed group N" |
| `font_glyphs::tests::verdicts_follow_the_research_findings` | 24 trees; Legacy X360: `Ł Ą ć` missing, Czech / Cyrillic / Latin-1 covered; Recompiled = Legacy tree; CJK missing; PS3 Legacy covered text; PS4 21 covers the capitals; PS3 12 unmeasured; NHL 07 no tree; C1 detected and refused; warning wording |
| `studio-api/tests/dictionary_growth.rs` (2 cases extended) | X360 Legacy growth preview: "every character … xbox360-nhllegacy"; `Norway\u{81}` refused with "C1 control … U+0081" before planning; PS3 Legacy rebuild preview: `~` covered; `Ł` named missing on `ps3-nhllegacy` while `ó` is not |

Runs: `cargo test -p studio-domain --test ps4_groups` (10 passed, 2 ignored — mounted-data cases);
`cargo test -p studio-domain --lib font_glyphs` (1 passed);
[p3-glyph-gate-growth.log](../../validation/evidence/overnight-development-2026-09-21/logs/p3-glyph-gate-growth.log)
(4 passed; the previous run failed on my own separator newline being checked as a glyph — C0
controls now skipped, diagnosed here); `npx tsc` clean. Aggregate: see receipt 10's closing run.

## 5. Coverage effect

- P1-C group names: no PS4 group is "unknown"-labelled any more (ids 77 / 78 derived); evidence
  classes unchanged.
- C5 glyph gate: **static half implemented** on 20 measured trees; the runtime half stays card
  LOC-G1 (MORNING-REVIEW).

## 6. Open / omissions

- The four PS3 in-archive font trees (08 / 09 / 10 / 12) are unmeasured: the gate says so.
- The gate checks the union of all shipped fonts; per-screen font selection is not modelled.
- Handoff 08 §3's membership rule (a club moved into league 9 / 20 must carry that pseudo-league's
  group id) was already enforced by the pool validation; not re-tested tonight.

## 7. Cleanup

No scratch files. No temp roots, no processes.
