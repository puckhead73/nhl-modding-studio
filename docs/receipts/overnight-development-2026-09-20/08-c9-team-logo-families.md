# Receipt 08 — C9: Phase 2.4 — Team logo families (preview / replace / placement per title)

Developer lane, overnight 2026-09-20. **Developer-verified; not Codex-accepted.** Baseline `8d60a6d`.

## Plain-language summary

The Team page gets a **Team logos** card: every logo image the game ships for that club, listed by
family (the big crest, the small one, the wide one, the calendar/dynasty/reflection versions on the
old consoles; the large/medium/small, mono, glossy and HUT-card versions on PS4), each with a
thumbnail, a loose **Replace…** and — where the console reads a packed archive — **Replace in
archive…**, exactly like the League logo card from Phase 1. Which families a title has was not
guessed from the research notes: the developer listed the folders of every shipped archive and loose
tree (27 platform/title sources: ten Xbox 360, nine PS3 plus the PS3 Legacy `cache.big`, seven PS4)
and recorded folder, name pattern, member count and id range; only the families keyed by the team's
art id with a team-sized member count are offered, and the small sets (HUT playoff/heritage cards,
nations, Spengler Cup, mono `t_#`) are named in the notes and left alone. One member of **every**
family and pattern (285 files) was then previewed and replaced twice with a marked image through the
loose route: 284 succeeded, and the one refusal — an Xbox One (`capilano`) DDS twin inside NHL 15's
PS4 archive — is a family the card does not offer. Nothing was written to a game. The user cards ask
which screen draws which family.

## 1. Package, disposition, scope

C9 — delivered, developer-verified. Roadmap 2.4 ("Logos/crests, menu imagery, team branding …
asset-ID bindings and all required variants"): the art-id-keyed logo families. **Not** in this
package: rink logos (`rendering/logo/texlib_{artid}_*.rx2`), centre-ice logos and pucks (2.5), the
jersey thumbnails (2.6), `artid` re-pointing (research handoff 07 §1 cascade), PS4 `img://LogoCro*`
processor semantics (a UI directive — the file is the path after it; nothing to write).

## 2. Changed paths (SHA-256 prefixes)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/studio-domain/src/team_logos.rs` | `bcbc6d5886186dcf` | **new** — measured family table per title (gen7 Xbox/PS3 07–Legacy, PS4 15–21), `team_logo_set(game, platform, artid, root)` → `TeamLogoSet {artid, evidence: "measured", variants: LogoVariant[], notes}` (loose existence + the archive the console reads, reusing the Phase 1 archive binding), `families_for` for tests |
| `crates/studio-domain/src/session.rs`, `lib.rs` | `d705f178bfc4fd4b` / `03b4134db1a33d15` | `EditSession::team_logo_set(team_id, root)` (None without an art id) |
| `crates/studio-api/src/{registry,dto,commands,lib,bindings}.rs` | `55a407853b97d084` / `59f9a85c0787f084` / `9b4c2d51e01dc65f` / `568c98e9cbe327c2` / `836c0ce961802dd0` | `Registry::team_logo_set`, `TeamLogoSetDto`, command `team_logo_set`, root added (119) |
| `app/src/bindings/TeamLogoSetDto.ts` | generated | `--write`; `--check` in sync (227 files) |
| `app/src/components/TeamLogoCard.tsx` (+ `.test.tsx`), `pages/TeamDetail.tsx`, `lib/client.ts` | `7c195bfab7d9022b` / `754d069bb3766d7d` / `c6dc8f0fa541e032` / `66a64b90b5f16ca4` | the card (same hooks as the League logo card: `useTextureReplace`, `useArchiveMemberReplace`); 4 jsdom cases |
| `crates/studio-domain/tests/team_logos.rs` | `c2c096f045bc4c43` | **new** — 2 default cases (grammar on Legacy / NHL 12 / PS4 21 / PS4 15 anchors; existence + unmeasured titles) + 1 `--ignored`: the table against the census (every offered family is a team-sized census row; every team-sized census row is offered; 27 sources) |
| `crates/studio-api/tests/gen7_asset_member_route.rs` | `37d37189fa22f798` | `NHL_STUDIO_GEN7_MEMBERS` member selection for the real sweep (default unchanged) |
| `crates/studio-api/tests/loose_texture_route.rs` | `0eff4bc780d8ceaa` | +1 `--ignored`: every fixture member through preview → marked replace ×2 → `.bak`; package-shape check now identifies the changed member instead of assuming the largest (a `tblank` placeholder's texture is smaller than its Apt data) |
| `local/runs/overnight-development-2026-09-20/extract_teamlogo_fixtures.py` | `b584685e3cfb4055` | **new** — developer census + extractor (read-only on the sources; imports the 2026-09-18 research readers) |
| `E:\nhl-studio-fixtures\teamlogos\**` + `manifest.json` (`3331882f2548f92b`) | — | **new private fixtures** (21 MB): 285 census rows / 285 members (one per family × pattern × source) |

## 3. Measured facts (developer evidence)

- **gen7 families** (identical folders and name grammar on Xbox 360 and PS3 for every title both
  ship; PS3 11–Legacy loose trees, PS3 08–10 archives, PS3 Legacy `cache.big` all agree):
  NHL 07 `teamlogos`, `teamlogoscalendar`, `teamlogossmall`, `teamlogoswide` (`t#`, 56 = its team
  table); NHL 08–09 + `teamlogosbroadcast` (131, ids 0–223) and `teamlogosdynasty`; NHL 10–12 the
  five without broadcast (`t#` everywhere); NHL 13–Legacy the letter prefixes `t`/`c`/`d`/`s`/`w`
  and `teamlogosreflection` `r#` (30 members, ids 0–29). Placeholders per folder: `tblank`,
  `tcustom` (13+), `dblank`, `t.big` (broadcast) — not team members. Member dimensions (preview):
  `teamlogos` 256×256, `small` 128×64, `wide` 256×256 (07: 512×248), `calendar` 100×88 (≤ 11) /
  128×128 (12+), `dynasty` 128×128, `broadcast` 128×64 / 112×5x, `reflection` 256×512.
- **PS4 families**: NHL 15 PNG under `ui/assets` (`fullsizeflat` 512², `halfsizeflat` 256²,
  `jerseylogo` 632×200, `overlaylogo` 256², plus `playnow/teamlogos` 256², 235 members) — a
  `kettle_assets/overlaylogo` GNF twin and a `capilano_assets/overlaylogo` DDS twin also ship;
  NHL 16 GNF `fullsizeflat`, `halfsizeflat`, `jerseylogo`, `overlaylogo`, `overlaylogohalf`;
  NHL 17 `fullsizeflat`, `fullsizebw` (1024²), `halfsizeflat`, `smallsizeflat` (80²), `overlaylogo`,
  `overlaylogohalf`, `hutcardbw`; NHL 18 `large`, `largemono`, `extralargemono`, `medium`,
  `mediumglossy`, `smallglossy`, `extrasmall`, `hutcardbw`; NHL 19 + `hutcardmedium`,
  `hutcardsmalllogo`, `hutcardrare/t_{bronze,silver,gold}_#`; NHL 20 + `extrasmallmono` (and six
  `spengler_#` members per family); NHL 21 + `small`. Not offered (count not team-sized):
  `iconsizeflat` (23), `hutcardplayoffs` (30 + `_fr`), `hutcardfreeagency` (30), `hutcardheritage`
  (2), `hutcardstadium` (2), `hutcardwinternational` / `hutnationsofhockey` (21, NHL 21),
  `mediummono` `t_#` (33, NHL 21), `spengler_#` (6).
- **Loose route on every member**: 284 / 285 previewed and replaced twice with the package shape
  preserved; the refusal is the `capilano_assets` DDS (not decoded, not offered). Every gen7 team
  logo package is the same BIGF shape as the league logos (RefPack'd XPR2 / RW4 / DDS), so the
  Phase 1 writers (span-preserving re-plan, archive placement, dimension change on XPR2/DDS 1:1
  families) apply unchanged.

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `cargo test -p studio-domain --test team_logos` | 2 passed, 1 ignored |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-domain --test team_logos -- --include-ignored` | **3 passed** (27 sources checked) |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-api --test loose_texture_route every_team_logo -- --ignored` | **1 passed** (264 s; 284 ok + the asserted refusal) |
| `cargo test -p studio-domain --test league_logos --test field_profile`; `-p studio-api --test loose_texture_route --test leagues --test registry`; `--lib bindings` | 3 / 8 / 1 (+5 ign) / 3 / 7 / 25 passed |
| `export-bindings --check` | in sync (119 roots, 227 files) |
| `cd app && npx tsc --noEmit -p . && npx vitest run` | clean / **19 files, 136 passed** (+1 file, +4) |

**Real-archive placement of team logos (run after the BIG4 sweep freed `%TEMP%`):** the gen7 route
sweep gained `NHL_STUDIO_GEN7_MEMBERS` (comma list of `path[:bound]`; `gen7_asset_member_route.rs`,
hash below) and was run on the real Legacy `cache.big` of both consoles with
`teamlogos/t10.big`, `teamlogossmall/s10.big`, `teamlogoswide/w10.big` (art id 10): Xbox — 11 376 /
10 800 / 46 128 bytes fit their slots (39 856 / 18 416 / 81 392), committed 141 s, restored
byte-exact 191 s; PS3 — 10 032 / 10 416 / 39 920 fit (37 296 / 18 096 / 73 072), 139 s / 188 s;
**2 / 2 passed** (`logs/c9_real_teamlogos.log`). Every other title's archives and stored forms are
the ones the Phase 1 sweeps exercised (same `fe.big` / `data0.big` / `inxp_10*.big`, same BIGF /
chunkzip members), so placement there is **by identity** with those sweeps.

## 5. Background sweep results recorded here

- **gen7 EB (C3), 9 / 9 passed** (`gen7_asset_member_route.rs`, real archives copied to `%TEMP%`):
  Xbox Legacy `cache.big` 312 439 568 B / 17 760 entries, PS3 Legacy 308 267 648 / 17 758, Xbox 11
  `fe.big` 348 445 424 / 16 675, Xbox 12 280 576 432 / 12 930, Xbox 13 447 904 176 / 14 235, Xbox
  14 328 811 712 / 16 167, Xbox 15 288 538 928 / 16 346, PS3 12 276 458 720 / 12 902, PS3 15
  284 865 616 / 16 344. `leaguelogos/l0` and `divisionlogos/d0` fit their slots everywhere;
  `leaguelogossmall/l0` fits on Xbox and is **appended and re-pointed** on every PS3 archive (the
  PS3 DDS package is larger than its slot: 4 272 > 3 888 on Legacy/15, 4 400 > 4 016 on 12);
  ~4 min per title; restore byte-exact.
- **BIG4 (C4)**: recorded in receipt 09 §5 (7 / 7 after the binder fix).

## 6. Coverage after C9 (Phase 2, 2.4 — art-id-keyed logo families)

| Operation | X360 07 | X360 08–11 | X360 12–15 / Legacy | PS3 08–10 | PS3 11 | PS3 12–15 / Legacy | PS4 15 | PS4 16–21 | Recomp |
|---|---|---|---|---|---|---|---|---|---|
| List the team's logo families (measured) | impl+tested (4) | impl+tested (6 / 5) | impl+tested (5 / 6) | impl+tested | impl+tested | impl+tested | impl+tested (5) | impl+tested (5 … 15) | by identity |
| Preview + loose replace (same size) | impl+tested (C9 sweep) | impl+tested | impl+tested | impl+tested | impl+tested | impl+tested | impl+tested | impl+tested | by identity |
| Different dimensions | blocked (RW4) | blocked (RW4) | impl (C6, XPR2 64/256-wide, 1:1 families) — team packages not swept | blocked (RW4ps3) | blocked | impl (C6, DDS) — not swept | impl (PNG) | blocked (GNF) | by identity |
| Archive placement | blocked by policy (NHL 07) | impl by identity (C4 BIG4 / C3 EB) | **impl+tested on the real Legacy `cache.big`** (t/s/w art id 10); 12–15 by identity (C3) | impl by identity (C4) | loose only | **impl+tested on the real PS3 Legacy `cache.big`**; 12–15 by identity | impl by identity (C2) | impl by identity (C2) | by identity |
| Which screen draws which family | research unknown → cards T-3 | ← | ← | ← | ← | ← | ← | ← | ← |

## 7. Temporary artifacts / processes

- Scratchpad `c9_*.py`, `c9_ps4_teamlogos.py/.json` — disposable. Temp roots removed by `Drop`.
- Retained: `teamlogos/` fixtures (private root; regenerable by the extractor).

## 8. Runnable commands

```sh
cargo test -p studio-domain --test team_logos
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test team_logos -- --include-ignored
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test loose_texture_route every_team_logo -- --ignored --nocapture
cd app && npx vitest run src/components/TeamLogoCard.test.tsx
python local/runs/overnight-development-2026-09-20/extract_teamlogo_fixtures.py   # re-census (read-only on the sources)
```
