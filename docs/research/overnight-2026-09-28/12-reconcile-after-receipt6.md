# Checkpoint 12 — reconciliation after receipt 6: developer receipts 61–65, Recompiled routes per operation, cards on r64, C38 confidence

Research lane, 28 September 2026, published 15:19 MDT (system clock). After-receipt-6 prompt, "Reconcile the handoff". Claim
`research-q11-reconcile-c38`. Inputs:
- the coordinator's [receipt-6 review](../../validation/research-20260928-receipt6-review.md) and
  [receipt-65 review](../../validation/development-20260928-receipt65-review.md);
- developer receipts 61–65;
- a read-only Studio source reading, done by a delegated read-only search with the key lines re-read by research, cited as `file:line`
  at the time of reading.

Research changed no application code.

## Plain-language summary

The developer's latest review build is r64, and it includes everything up to receipt 64.
- **Hockey Shop "failed undo" defect:** closed for the situations the tests cover.
- **Still open (developer):**
  - Studio can reuse a cached head catalogue after the game files change;
  - a temporary-file ownership gap.
- **Legacy Recompiled** is now named operation by operation:
  - the database, sound banks, portrait / head reassignment and logo archives have working routes;
  - speech lists, picture previews, kits, arena art, Hockey Shop and the containment card do not. Those still look for loose folders
    that a Recompiled install does not have.

## 1. Developer status (delivery ≠ review ≠ acceptance)

| Item | Status |
|---|---|
| r64 bundle | holds developer receipts through 64 (receipt 65 §6); internal review candidate, `release_ready: false`. The coordinator re-verified the archive (`verify.py check` passes) |
| F1 Hockey Shop rollback (61) | **closed for its tested in-memory recovery scope** (11 unit tests, independently run). No crash safety, and no protection against concurrent writers |
| **F2** head-catalogue cache keyed by length / mtime (`Registry::ps4_head_catalogue`) | **active developer correction**. Apply must revalidate current content and identity. Normal head tests do not close it |
| Temporary-file ownership in `WriteTxn::write` | **active developer correction** (source inspection) |
| PS4 head reassignment (63) | delivered, with independent integration tests 1 / 1. Hair continuation waits for F2 |
| Recompiled public edits (62) | delivered, with independent integration test 1 / 1 |
| NHL 11–13 companion-style lock (64) | delivered; matches the coordinator's S3 decision |

## 2. Legacy Recompiled — route per operation (install = 13 EB archives, no loose `rendering` / `fe` / `audio` / `db`)

Loose routes go through `AssetRoots::resolve` (`studio-domain/src/assets.rs:228`). It resolves `<folder with default.xex>/[_compiled/]<name>`,
which is **absent** on the registered Recompiled install (R138).

| Operation | Current consumer | Recompiled route today | Concrete missing path |
|---|---|---|---|
| Main database open / edit | `detect_recomp` (`discovery.rs:277`), `extract_install` (`extract.rs:96`) | **works**, after extraction beside the archives (receipt 62, public-path regression) | — |
| Dictionary (names) | `attach_dictionary` (`registry.rs:327`) after extraction | works once extracted. Without extraction it fails **silently** (`let _ =`), so names show as keys | disclose a failed attach |
| Sound banks / streams | `gen7_sfx_banks.rs:106–252` | **works** (member route; receipt 59) | — |
| Speech catalogue, name calls, context | `speech_catalogue.rs:146, 358, 465, 648` | **none**: loose `audio/speech` only ("archive route is not built yet") | member route for `audio/speech/*` (the members exist; R138 class "speech") |
| Speech preview | `gen7_speech_preview.rs:276–321` | member route, but tries `install_root/<archive>` **without** the `game/` prefix, so it works only when the `game` folder is attached | accept both roots, as the sfx layout does |
| Portrait / head **containment card** | `player_containment` (`registry.rs:3609` → `player_containment.rs:209`) | loose only. On Recompiled every status is `unknown`. The contract's source text (`player_containment.rs:78`) says "checked against this install's own archives", which this card does not do | pass the receipt-55 `ArchiveIndex` to the card, or correct the disclosure |
| Portrait / head **reassignment** | `player_reassign` (`registry.rs:3666`) with `ArchiveIndex` | **works** (existence by member path; receipt 55) | — |
| Portrait / head preview; texture replace | `portrait_preview` / `head_preview` / `texture_preview` (`registry.rs:417, 681, 608`) | none (loose only). `archive_member_texture_preview` (`registry.rs:636`) exists but is not wired here | a member route for `fe/ion/artassets/playerheads*` and `rendering/player` |
| Team / league logos | `team_logos.rs:205`, `league_logos.rs:110–127` (`cache.big`), replace via `asset_member_preview` (`registry.rs:2839`) | **member route for the logo table and replacement**. The team header logo (`team_logo`, `registry.rs:3220`) is loose only, so it is absent | member route for the header logo |
| League imagery | `league_imagery.rs:213` | none: `defs/league_imagery.toml` has no `legacyrecomp` row (catalogue id split, `nhllegacy` vs `legacyrecomp`) | a row or alias decision (developer / coordinator) |
| Kits / uniforms / nameplates | `kit_contract` (`kit_files.rs:152`) → `Unmeasured` for `legacyrecomp` | refused as unmeasured. The `nhllegacy` path would read `cacherender.big` / `nocacherender.big` | id alignment plus the member route (bytes = X360 Legacy, R128) |
| Arena assets / venue art | `arena_assets.rs:127`, `venue_art.rs:113` (Recomp not in `MEASURED`) | refused | member route for `cacherender.big` `rendering/stadium` (67 members equal the X360 loose pair: 09 K9 / K10) |
| Font glyphs | `font_glyphs.rs:63` | static definitions borrowed from X360 Legacy; no install read | — |
| Equipment / kit mesh previews | `equipment_slots.rs:407`, `equipment_piece.rs:90`, `registry.rs` kit preview | none (loose only) | a member route for `rendering/<slot>` |
| **Hockey Shop sync / install** | `hockey_shop.rs:31` admits `legacyrecomp`; `sync` / `plan_install` / `apply_install` (`:91–537`) use loose `rendering` | lists nothing; an install would write **loose files** under a folder that Recompiled does not ship | **research observation O12-1:** refuse or disclose. Whether the recompiled executable reads a loose override is unobserved, and no archive route is planned |
| Roster save | `detect_recomp` → `scan_saves(save_path)`; mirror (`registry.rs:303`) | user-supplied save path; raw container | runtime compatibility and limits: **not inferred** |

Nothing above infers runtime loading, save compatibility or limits (unchanged: user card).

## 3. C38 confidence and wording (receipt-6 review)

- The research **picture** checks cover X360 Legacy pads (6 match, 1 plausible, 3 undetermined), Legacy blockers (4 match), Legacy
  trappers (2 plausible) and NHL 12 pads (4 match).
- NHL 13 / 14 / 15 and PS3 15 / Legacy rest on equal id sets only. **Equal sets are not picture identity.**
- The NHL 14 glove / trapper fallback (handoff 13) added a **counter-signal**: glove 48's lettering resembles tint mask 47. Gloves are
  therefore not supported.
- A stored mask number without a picture is described neutrally: "no picture available". Absence does not make it a team mask.
- Handoff 11 was corrected with dated notes (§4 confidence paragraph; mask wording).

**Retained review set (outside the repository; proprietary images are never committed):** `H:\TEMP\nhlms-research-c38-visual-2026-09-28\`.
- Contents: 7 sheets, 1.75 MB.
- Manifest: [`data/r143f-visual-review-set.json`](data/r143f-visual-review-set.json), with every source file's SHA-256, the ids and the
  N − 1 / N / N + 1 controls. The NHL 14 fallback hashes are in `data/r143d-visual-checks.json`.
- Built by `r143f_visual_review_set.py`.
- Owner: research. **Successor dependency:** the coordinator's review of C38 (research receipt 7). The folder is deleted after that
  review.

## 4. Manual cards mapped to r64 (nothing closed by this mapping)

- **Desktop cards to run on r64:**
  - V-82 → V-93 (earlier r59 cards; the features are unchanged in r64 unless a later receipt says otherwise);
  - **V-94** (PS4 head reassignment) and **V-95** (stale-apply refusal), both new in r64;
  - the earlier V-45 → V-81 cards stay outstanding where not yet run.
- **Listening:** A-XMA-2 → 8.
- **Game observations:** P1-A / P1-A-q (Austrian phases), T-3b, P34-1 / R15-HEAD-1 (the drawn head), P34-G08 (NHL 07 / 08 generic
  heads), G-MASK-1 (slot vs personal mask), R36-PS3-KIT-1, R30-LZX-1 (the chunklzx re-wrap card, still gating X360 board / kit
  archive writes).
- **A Recompiled runtime card** is still to be written (loader precedence, saves). It would be an in-game observation, which research
  does not run.
