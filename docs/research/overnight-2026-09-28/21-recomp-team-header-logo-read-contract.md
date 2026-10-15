# Handoff 21 (1 Oct) — Legacy Recompiled team header logo read route: crest in `cache.big`, rink-logo fallback in `cacherender.big`

Research lane, 1 October 2026, published 01:50 (system clock). After-receipt-8 prompt, continuation. Receipt 67 §4 lists the team
header logo as "not built (loose only)". Claim `research-q20-recomp-team-header-logo`, written before work began.
- Read-only: archive directories, every member's head, and a 12-member in-memory sample. Nothing launched or written; no application
  edit.

Machine-readable: [`data/r149-recomp-team-header-logo.json`](data/r149-recomp-team-header-logo.json) (`fb677632…`). Probe:
`probes/r149_recomp_team_header_logo.py` (`ece131eb…`), one run.

## Plain-language summary

- The team picture in Studio's team header comes from one of two places:
  - first choice: the front-end team crest, `fe/ion/artassets/teamlogos/t<art id>.big`;
  - otherwise: an in-rink logo texture, `rendering/logo/texlib_<art id>_0_0.rx2`.
- On Recompiled:
  - all 320 crest files (318 teams plus a blank and a custom placeholder) sit in `cache.big`, unpacked;
  - all 1 926 rink-logo files sit in `cacherender.big`, packed with chunklzx;
  - nothing is duplicated, the names match Xbox 360 Legacy, and the samples match byte for byte.
- Studio's logo **table** for Recompiled already reads `cache.big`. Only the header picture still looks for loose files, so on
  Recompiled it shows the neutral placeholder.

## 1. Measured (registered install)

| Source (`decode_team_logo`, `assets.rs:3024`) | Members | Archive | Wrapper | Duplicates (within / across) | Names = X360 Legacy loose | Sample = X360 loose |
|---|---|---|---|---|---|---|
| Crest `fe/ion/artassets/teamlogos/t<artid>.big` (primary) | 320: 318 `t<N>.big` (ids 0–20087) + `tblank.big`, `tcustom.big` | `cache.big` | **stored** (BIGF, head `42494746`) | 0 / 0 | yes, 320 | 6 / 6 |
| Rink logo `rendering/logo/texlib_<artid>_<a>_<b>.rx2` (fallback) | 1 926, all of shape `texlib_N_N_N` | `cacherender.big` | **chunklzx** | 0 / 0 | yes, 1 926 | 6 / 6 |

**Fallback name forms.** The code tries `texlib_<artid>_0_0`, then `texlib_<artid>`, then `texlib_<artid>_0`.
- On this install only the first form exists: it is present for 275 of the 318 crest ids.
- The other two forms match **no** member.

**Sample** (unwrapped SHA-256, first 16 hex; full values in the JSON):

| Class | Members and hashes |
|---|---|
| Crests | `tblank` `8ad133cc…`, `t43` `e8128dad…`, `t47` `1b3f0b87…`, `t95` `186ae033…`, `t20007` `3903a2e3…`, `t366` `7fe0aa08…` |
| Rink logos | `texlib_30_0_0` `4cad4102…` (as R138), `texlib_31_0_0` `afad20c6…`, `texlib_31_1_1` `acdffcf3…`, `texlib_83_0_2` `38613a80…`, `texlib_393_0_1` `1625b1c5…`, `texlib_2_1_0` `9e8ebaec…` |

The largest crest is 77 296 B and the largest stored rink logo 13 674 B. Whole-member reads are fine.

## 2. Current consumers (source reading; not run)

- `Registry::team_logo` (`registry.rs:3270`) calls `decode_team_logo(&roots, artid)`. On Recompiled, `roots.fe` / `roots.rendering`
  are the non-existent `game\fe` / `game\rendering`, so the result is `None` and the UI shows the neutral placeholder.
  - It is honest, in that no wrong logo appears.
  - It does not distinguish "not shipped" from "archive-held, not read".
- The existing loose code falls back to the rink texlib **whenever the crest fails to decode**, not only when it is absent. That
  silently shows a different asset, a wordmark or crest guess, in place of a corrupt crest.
- `team_logo_set` (`team_logos.rs:205`) already names the archive for Recompiled (`F13_LEGACY` includes `legacyrecomp`;
  `gen7_logo_archive_at`).
  - Its `exists` field reports **loose** presence, so it is `false` for every member on this install even though the members ship.
  - Its note "the console reads the packages from {archive}" is not observed for Recompiled (18 K20). Wording only; no wrong write.

## 3. Contract (developer; read only)

**Title and roots.** `legacyrecomp` with `Platform::Recomp`. Two root forms, resolved deterministically, refusing ambiguity (18 K21).

**Order and paths.**
1. Crest member `fe/ion/artassets/teamlogos/t<artid>.big`.
2. Only when the crest is **absent** from a readable complete index: the rink member `rendering/logo/texlib_<artid>_0_0.rx2`. The
   other two loose name forms can stay in the candidate list; they match nothing here.

**Corrupt or unknown.**
- A crest member that exists but does not decode (BIGF → RefPack → XPR2) is an **error**, shown as "crest unreadable". It never falls
  through to the rink texture.
- An unlistable archive makes the logo **unknown**, never "no logo".
- `tblank` / `tcustom` are placeholders, never a team.

**Decode.**
- Crest: stored. The existing `decode_portrait` path on the member bytes (the same BIGF chain as portraits, R146).
- Rink: the shared chunklzx unwrap, then `tdb_rx2::texlib::parse` with the existing square-raster preference.

**Index reuse.** Use the same archive index as the logo table / containment route. Do not open a parallel resolver.

**Provenance.** "crest `cache.big:fe/ion/artassets/teamlogos/t<artid>.big` (stored, n B)", or "rink logo `cacherender.big:…`
(chunklzx)". No runtime claim.

**Loose policy.** Loose first is Studio policy (18 K21), with per-file fallback.

**Not included:** logo replacement on Recompiled. The logo-table replace route is a separate, earlier delivery and is not re-assessed
here.

## 4. Smallest regressions

**Positive** (registered install, both root forms; X360 Legacy loose tree as reference):
1. The header logo for one team whose art id has a crest is pixel-equal to the loose route.
2. One art id among the 43 crest ids without a `_0_0` texlib: crest shown, no fallback needed.
3. A synthetic index **without** the crest but with `texlib_<artid>_0_0`: the rink fallback is shown and labelled as the fallback.

**Negative:**
1. An owned synthetic `cache.big` with a corrupted crest BIGF: "crest unreadable". **Not** the rink texture.
2. A corrupted chunklzx rink member (crest absent): an error naming it.
3. One unlistable archive with the crest not found: unknown, not "no logo".
4. Art id `tblank` / `tcustom` are never offered as a team.
5. The X360 Legacy loose tree: unchanged.
