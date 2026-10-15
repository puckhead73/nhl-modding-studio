# PS4 editing review and parity roadmap — 13 October 2026

Status: **proposal for architect review.** Not accepted, and it promotes no support level. It sits
under the [entity editing and parity roadmap](entity-editing-parity-roadmap.md) and the
[October 1 beta decisions](beta-20261001-scope-decisions.md), and does not override either one.

## Summary

PS4 (NHL 15–21 on PS4, seven shipping targets) can already **edit nhl_game.db** about as well as
the earlier generation (Xbox 360/PS3 NHL 07–15 and Legacy). In a few places it does better: native
league names, stored-default settings, and schedules. What PS4 lacks is getting edits into the
running game, plus most asset families beyond logos.

There are three main gaps:

1. **We don't know which copy of a file the game actually loads.** Studio deploys a *loose*
   `nhl_game.db`, while the only runtime-proven route is replacing the member inside
   `inxp_00.big` (NHL21-E1). The copied-archive route exists in the backend, but no screen reaches
   it. Base/patch precedence on NHL 15 and NHL 19 is also unknown. Until this is settled, every PS4
   loose-file edit (portraits, kits, ice, equipment, UI textures) may never reach the game.
2. **The vault is read-only.** On PS4, arenas/ice, kits, heads and equipment are bound through the
   Attrib/VLT vault in `boot.big`. We can read it but cannot write it. This blocks every
   reassignment and registration beyond the shipped head/hair re-points.
3. **No PS4 audio is editable, and most of it cannot even be previewed.** NHL 18–21 speech uses
   EA-Opus, which Studio refuses to preview, and no encoder exists for EA-MP3 or EA-Opus.

There are also four correctness defects to fix first:
- the GNF same-size replace leaves the old image in the lower mips;
- the PS4 trailer CRC is written inside the trailer on the older roster-save path;
- the committed NHL 20 anchor is really NHL 19;
- the NHL 15 save codec and the save-matrix test are uncommitted, and that test fails.

Apart from the NHL21-E1 league rename the user accepted, **no PS4 cell is game-verified**. Every
status below is implemented or automated-tested at most.

## Evidence legend

| Mark | Meaning |
|---|---|
| ✅T | implemented, with automated tests that run by default |
| ✅P | implemented; tested only by `#[ignore]` private-fixture/install cases (recorded passing in Oct 3 QA unless stated) |
| 🟨 | partial: read-only, listing only, a subset of titles, or no UI |
| ⬜ | not implemented |
| 🔬 | research unknown: the fact needed to implement is not established |
| ⛔ | blocked or refused by design, with the blocker recorded |
| — | not applicable |

"Gen7" means Xbox 360/PS3 NHL 07–15 + Legacy (+ Recompiled where noted). Nothing in either
column is game-verified unless explicitly said.

## 1. Capability matrix — database

| Capability | PS4 (15–21) | Gen7 | Notes / evidence |
|---|---|---|---|
| Little-endian TDB read/write, CRCs | ✅T | ✅T | `tdb-core/tests/golden.rs`, mutation/operation matrices over all 27 anchors |
| Schema family / encoding contract | ✅T | ✅T | `SchemaFamily::Ps4`: ratings 0..99 offset 0, weight base 120, cap ×$5k, 0-indexed birthdate (`studio-domain/src/encoding.rs`) |
| Capability probe | ✅T | ✅T | No PS4 special-casing; probe is evidence-driven |
| **League identity** | ✅T, **E1 user-accepted on NHL 21** | ✅T (dictionary re-point) | PS4 renames by reassigning the native `stockleagues` key; gen7 needs dictionary re-pointing |
| Competition groups / team moves | ✅T / ✅P | ✅T | Per-title group tables; NHL 17 ids 43–60 and NHL 18 74–76 are *inferred* |
| League creation/resizing | ⬜ | ⬜ | Outside beta |
| Schedules: browse/edit/CSV/generate | ✅T (+✅P real NHL 20) | ✅T | Three PS4 layouts: 15, 16/17, 18+ |
| Settings / stored defaults | ✅T (NHL 15 DB; 16–21 `setting_csv`) | ⛔ (compiled into executable) | **PS4 ahead.** Runtime precedence 🔬 |
| Team identity, colours, art-id | ✅T | ✅T | NHL 16 canonical team-name binding unresolved (no `fullname`/`teamname`) |
| Team links: membership, lines, captains, numbers, strategies, trades | ✅T (operation matrix, all 7 anchors) | ✅T | Phase 2 matrix still calls edit/reload/restore parity "not established" |
| Arena assignment (DB FK) | 🟨 read | 🟨 read (NHL 07 editor) | `Nhl07ArenaEditor` is NHL 07-only |
| Team schedule tab, competition-name/structure cards | ⛔ UI NHL 07-only | NHL 07 only | `TeamDetail.tsx:58`, `LeagueDetail.tsx` |
| Player identity, ratings, styles, contracts, draft | ✅T | ✅T | |
| Create/clone/delete/move/sign/release | ✅T | ✅T | Free-agent signing blocked for players with national/All-Star memberships |
| Career stats add/delete | ✅T 15–17; ⛔ 18–21 | ✅T | `tsSy` lacks inline team code on 18–21 |
| Roster capacity | ✅T | ✅T | Main DB `max_records` |
| Overall formula | ✅T | ✅T (era-calibrated) | |
| Historical import | ✅T (lower sweep on every anchor) | ✅T | API roster-builder test is Legacy PS3 only |
| Equipment cells (stick/glove/helmet/tape) | 🟨 edits raw numbers; read-only catalogue/CZ cards | 🟨 Legacy catalogue labels | No reassignment validation or new-id registration on any platform |
| Canonical adapters | 🟨 NHL 19/20 representation-only | 🟨 | NHL 20 anchor alias (below) |

## 2. Capability matrix — localization, saves, deployment

| Capability | PS4 | Gen7 | Notes |
|---|---|---|---|
| Dictionary re-point/retext | ✅T | ✅T | Same `GJCv` format, little-endian |
| Dictionary growth/rebuild | ✅P (99 locale members) | ✅T/✅P | Runtime acceptance of a grown file and new-key lookup 🔬 |
| Dictionary inside `inxp_00.big` (chunkzip member) | ✅T | — | Patch copies (NHL 15, NHL 19 ×6) never targeted |
| Bundled dictionary | ⛔ NHL 19, 21 (no `eng_us` anchor); NHL 20 unavailable | ✅ | |
| Roster save extract/edit/repack | ✅T NHL 16–21; NHL 15 chunkzip **uncommitted** | ✅T (v2/v4; v1 experimental) | Structural only; record-header tag, `INDEX`/`param.sfo` 🔬 |
| Save Editor (identify, overlay, edit) | ✅T | ✅T | Growth refused when a trailer is present |
| **Trailer CRC on NHL 17–19 roster path** | ⛔ **defect** | — | EOF CRC lands inside the 18–20-byte trailer; Save Editor avoids it, older roster/package path does not |
| Mode saves (Season/Franchise/BeAGM) | 🟨 read-only / 🔬 roster source | 🟨 | Save-matrix classification conflicts with the Save Editor record |
| Settings/Profile saves | ⛔ opaque | ⛔ (container v5) | |
| NHL 20/21 vanilla roster save | missing-fixture | — | Only ThreeOnThree saves committed |
| Discovery (shadPS4 game + save roots) | ✅T | ✅T | NHL 19 title-id discrepancy CUSA11126 vs 11116 (FR9) |
| Base-PKG onboarding | ✅P (real NHL 20 job) | ✅ ISO/folder | Patch/DLC PKGs refused; PS4 ISO onboarding not established |
| **Deploy edited DB** | 🟨 loose file only from UI | ✅ | Loose-vs-archive precedence 🔬 |
| **Copied `inxp_00.big` member apply/restore** | ✅P all 7 titles, **no UI** | n/a | `CopiedTargetPanel.tsx` is imported only by its test |
| Patch-archive targeting (NHL 15, NHL 19) | ⛔ 🔬 precedence | 🔬 (PS3 `patch.big`) | Extraction blocks on 3,487 (NHL 15) / 16,894 (NHL 19) colliding members |
| Mod package with archive edits | ⛔ "no installer adapter" | 🟨 | PS4 archive edits cannot be distributed |
| Original-console PS4 packaging (Q04) | ⬜ | ⬜ / ⛔ STFS | Required scope |

## 3. Capability matrix — archives and textures

| Capability | PS4 | Gen7 | Notes |
|---|---|---|---|
| EB read/extract (`0x00110400`; NHL 15 `0x00100400`) | ✅T/✅P | ✅T (BIGF/BIG4/Legacy EB) | NHL 15 variant supported since 18 Sep (EB-V1) |
| chunkzip decode/encode | ✅T | ✅T | chunklzx decode/store only (Legacy X360) |
| Replace existing member (fit/append) | ✅T/✅P | ✅T | Output "unqualified": `+0x0C` sort key unresolved |
| Add new member/path | ⬜ 🔬 | ✅ (BIGF) | Needs sort-key formula |
| Size-anomaly archives | ⛔ NHL 16 `game_90`; same-length only on NHL 15 `game20`/`inxp20`/`patch_inxp00` | — | V-EB-ANOM |
| GNF decode BC1/2/3/7 | ✅T/✅P | ✅T (XPR2/RW4/DDS) | Base mip only de-tiled |
| GNF BC4/BC5 | 🟨 small BC5 de-tile drifts | ✅ | |
| GNF BC6H | ⛔ UnsupportedFormat | — | 2,821 files; size rule modelled, decoder absent |
| GNF two-stream (NHL 15: 1,115 files; NHL 19: 5,342) | 🟨 first stream decode; write refused | — | |
| **GNF same-size replace** | ✅T/✅P, **defect: lower mips not regenerated** | ✅T (full chain) | `gnf.rs:266-291`; resize path regenerates correctly |
| GNF resize | ✅P (6 real NHL 16/21 sources) | 🟨 (mipped XPR refused) | Runtime card R31-GNF-1 open |
| Texture history/undo | ✅T | ✅T | |

## 4. Capability matrix — asset families

| Family | Operation | PS4 | Gen7 |
|---|---|---|---|
| League logos/imagery | preview / replace loose / replace member (`inxp_10`) | ✅T / ✅P / ✅P | ✅ / ✅ / ✅ |
| Team logos | preview / replace / resize | ✅ / ✅P same-size / ⛔ (team matrix) | ✅ / ✅ / ✅P (168 packages) |
| Portraits | preview / replace | ✅ / 🟨 loose only (game reads `game_60.big`) | ✅ / ✅ (BIGF→RefPack→XPR2) |
| Portrait reassignment | DB re-point | ✅P | ✅ |
| Heads | decode/preview (MCD) | ✅ (15/16 drift handled) | ✅ RW4 |
| Heads | reassign to shipped head | ✅P (vault-checked; `headtexture` co-written 17–21) | ✅ |
| Heads | import new geometry | 🟨 `mcd_write.rs` exists (synthetic tests), **no Studio route**; NHL 19 imports by out-of-tree scripts | ✅ Path A (Legacy X360) |
| Heads | register new bespoke head id | 🔬 `headid→head_<name>` unresolved; ~133 orphan collections; may need executable patching | 🟨 |
| Hair | reassign | ✅P 17–21; ⛔ 15/16 (no hair class) | ✅ |
| Jerseys/kits | browse / replace texture | 🟨 filename candidates / 🟨 loose | ✅ measured selectors 11–Legacy, 3D preview |
| Jerseys/kits | kit binding, ordering, nameplates/fonts/numbers | ⬜ 🔬 vault `(Team, UniformVariation, JerseyFit, IsTopScorer)` | ✅ nameplate editor; 🟨 selectors |
| Arenas/ice | list venues | 🟨 static snapshot (`defs/arena/ps4-venue-levels.json`) | ✅ `arenaartid` joins |
| Arenas/ice | centre-ice texture replace | ✅P loose | ✅ |
| Arenas/ice | reassign venue/ice | ⛔ vault cascade (Q-R69-1) | 🟨 NHL 07 editor |
| Boards | any | ⬜ | ✅ PS3 board replace |
| Scorebug | any | ⬜ | ⬜ |
| UI/menu textures | browse/replace | 🟨 generic loose GNF | 🟨 generic |
| Equipment | catalogue/thumbnails | 🟨 CZ names/thumbs 17–21, enum-only 15/16 | ✅ Hockey Shop (RW only) |
| Equipment | texture replace / clone / new id | 🟨 loose replace / ⬜ / ⬜ | ✅ / ✅ / 🟨 |
| Goalie masks | pairs / paint editing | 🟨 read-only pair card | 🟨 |
| Vault (`gccollections`/`gcschema`) | read | ✅ (identity-gated) | — |
| Vault | write | ⬜ | — |

## 5. Capability matrix — audio

| Capability | PS4 | Gen7 |
|---|---|---|
| Bank listing (SND10 speech) | ✅ | ✅ |
| Speech preview NHL 15–17 (EA-MP3) | 🟨 diagnostic (A-MP3-1) | 🟨 EALayer3 diagnostic |
| Speech preview NHL 18–21 (EA-Opus) | ⛔ refused; decode contract documented | — |
| Rink announcer (EA-Speex) | ✅ preview | — |
| Player name calls | ⛔ no binding (H1–H5 negative) | ✅ catalogue 11–Legacy |
| Commentary contexts (CTX0/0TVE) | ✅ listing | ✅ |
| Goal horns | 🟨 catalogue 17–21 (names only; horn→bank in undecoded `sfx_ptc.big`) | 🟨 horn sources |
| Songs/menu music | 🟨 catalogue 17–21 | 🟨 |
| Any replace/encode | ⬜ | ⬜ (XMA via helper for decode) |

## 6. Where PS4 stands relative to gen7

- **PS4 ahead:** league identity (native key), stored-default settings, PS4 package onboarding, and
  the only user-accepted runtime edit (E1).
- **At par:** DB entity editing, encoding contract, schedules, localization formats, roster-save
  structure, logo replacement, head/hair reassignment.
- **Behind, delivery:** loose-vs-archive deployment, patch precedence, archive-edit packaging,
  and the archive route has no UI.
- **Behind, assets:** portraits in archives, kits/nameplates, arena/ice rebinding, boards, equipment
  authoring, new-head import route, GNF format coverage (BC6H, two-stream, small BC5).
- **Behind, audio:** EA-Opus decode, player-name binding, horn bank join.
- **Fixture gaps:** a genuine NHL 20 anchor, `eng_us` for NHL 19/21, roster saves for NHL 20/21.

## 7. Roadmap

The waves follow the parent roadmap's order (leagues → teams → players → equipment → presentation),
preceded by two PS4-specific waves that everything else depends on. Each wave lists developer work
(D), research questions (R) for the research partner, and runtime cards (G) for batched user
acceptance under shadPS4. A wave closes per target, never by a representative title.

### Wave 0 — Correctness and hygiene (small; do first)

| ID | Work | Type | Exit |
|---|---|---|---|
| P0-1 | Regenerate the full GNF mip chain on same-size replace (as the resize path already does) | D | Test asserts every mip differs from the original after a changed-pixel replace; unchanged input still byte-identical. **Implemented + automated-tested 13 Oct (uncommitted, pending review):** `gnf::replace_chain_same_size`; unmeasured layouts keep base-only with a disclosure; GNF pre-Apply plan is now the writer's policy |
| P0-2 | Fix trailer-aware CRC placement on the NHL 17–19 roster/package path, sharing the Save Editor's strip/restore logic; allow growth by rewriting the embedded TDB size | D | Trailer bytes preserved; CRC at TDB end; growth test on NHL 17/18/19 |
| P0-3 | Review and commit the NHL 15 chunkzip save codec; regenerate the save matrix through the authorized generator path (not hand-edited) so its test passes | D (generator run needs explicit authorization) | Save matrix shows NHL 15 roster as edit-roundtrip |
| P0-4 | Replace the aliased `anchors/ps4/nhl20` with the genuine CUSA15439 DB/dictionary (owner decision on EA-data policy); make NHL 19/20 adapters operative | D + decision | NHL 20 tests run unignored |
| P0-5 | Resolve the NHL 19 title id (CUSA11126 vs 11116) and the NHL 16 team-name binding | R → D | Discovery and adapter agree with installs |
| P0-6 | Correct stale docs: `ps4-gnf-texture.md`, `ps4-rsf-mesh.md`, `arena_assets.rs` header, `assets-logos.toml:44`, STATE-OF-THE-REPO audio/mip claims, `cross-platform-format-audit.md` | D | No stale "read-only"/"excluded" claims |

### Wave 1 — Delivery path: make PS4 edits reach the game

This is the critical gate. Without it, asset work beyond `inxp` members cannot be accepted.

| ID | Work | Type | Exit |
|---|---|---|---|
| P1-1 | Runtime precedence cards: loose `nhl_game.db` vs `inxp_00.big` member; loose texture vs archive member; base vs patch (NHL 15 `patch_inxp00`, NHL 19 `patch_inxp_51..56`, `patch_game_91/93`). One controlled marker edit each | G | Recorded per-title precedence verdict with emulator build |
| P1-2 | Wire the copied-archive route into the UI (`CopiedTargetPanel` or equivalent) as the default PS4 deploy, with patch-aware target selection driven by the P1-1 verdicts | D | Studio deploys DB/dictionary edits to the archive the game reads; restore byte-exact |
| P1-3 | Generalize the member route to every archive family the game reads (`game_60` portraits, `game_90` heads, kits/arena archives); retire loose-only routes where P1-1 shows they are ignored | D | Every PS4 asset replace writes to a loading archive |
| P1-4 | EB `+0x0C` sort key and header word `0x10`; NHL 16 `game_90` size anomaly | R | Formula verified across 15–21, or blocker recorded; unblocks add-new-member and "qualified" outputs |
| P1-5 | Package installer adapter for archive-member edits (same title/platform) | D | `.nhlmod` with PS4 archive edits builds and installs |
| P1-6 | Save write acceptance: edited roster save load under shadPS4; record-header tag and `INDEX`/`param.sfo` requirements | G + R | Roster save edit game-observed on ≥1 title per framing (15 chunkzip, 16 bare, 17–21 codec byte) |
| P1-7 | Fixtures: `eng_us` for NHL 19/21; vanilla roster saves for NHL 20/21 | fixture | Missing-fixture cells cleared |

### Wave 2 — Leagues and teams closure (parent Phases 1–2)

| ID | Work | Type | Exit |
|---|---|---|---|
| P2-1 | Runtime cards for league identity, groups, schedule, stored defaults and dictionary growth on 15–20 (E1 covers 21 only) | G | Per-title verdicts |
| P2-2 | Settings precedence (DB/CSV defaults vs runtime) | R → G | Known which source a new mode reads |
| P2-3 | Team links edit/reload/restore parity (2.2), making the existing operation-matrix evidence an accepted Studio workflow | D + G | Phase 2 matrix 2.2 closed for PS4 |
| P2-4 | Lift NHL 07-only UI gates where PS4 data exists (team schedule tab, competition structure) | D | Tabs gated by capability, not `game_id` |
| P2-5 | Team logo resize parity (GNF resize already exists; close the team-matrix block) after R31-GNF-1 | D + G | Team rebrand with resized logo game-observed |
| P2-6 | **Vault writer** (minimum: attribute value edit of existing records, preserving chunk chain and lookup2 keys) | R + D | Byte-exact no-op round trip; one attribute change reopens; restore |
| P2-7 | Arena/ice reassignment on top of P2-6: live vault walk replacing the static snapshot, `arenaartid`/`ge_level` cascade, ambiguous venues resolved | R + D + G | Team plays in reassigned venue with correct ice |
| P2-8 | Kits: decode the vault jersey binding (`Team, UniformVariation, JerseyFit, IsTopScorer`), list team kits, replace kit textures in-archive; nameplate/font/number zones on PS4 sheets | R + D + G | Complete kit replacement correct on ice and in menus |
| P2-9 | Team audio: decode `sfx_ptc.big` Harmony data for horn→bank; EA-Opus decode for preview | R + D | Horn catalogue rows resolve to playable sounds |

### Wave 3 — Players (parent Phase 3)

| ID | Work | Type | Exit |
|---|---|---|---|
| P3-1 | Career stats on NHL 18–21 (team code semantics of `tsSy`) | R → D | Career add works or is evidence-backed refused |
| P3-2 | Free-agent signing with residual national/All-Star memberships | D | Sign succeeds with memberships preserved |
| P3-3 | Portrait member route (`game_60.big`) and resize | D | Portrait replace game-observed |
| P3-4 | Head import route: bring `mcd_write`/`pack_mcd` behind a reviewed Studio route **only when the external converter integrates** (headmesh direction); add an nhl19-ps4 matrix row from existing evidence without promoting it | D (gated) | Integration package per converter hand-off |
| P3-5 | Bespoke head registration (`headid→head_<name>`, orphan collections, executable dependency) | R | Registration contract or recorded executable blocker |
| P3-6 | Player name-call binding (code-level investigation after H1–H5) | R | Binding found, or absence evidenced |

### Wave 4 — Equipment (parent Phase 4)

| ID | Work | Type | Exit |
|---|---|---|---|
| P4-1 | Measure PS4 equipment component sets and vault bindings per category | R | Catalogue→label→vault→texture chain per category |
| P4-2 | PS4 Hockey Shop equivalent: labelled pickers, validated reassignment, texture replace in-archive (GNF instead of RW) | D | Stick/glove/helmet/skate assignment and texture game-observed |
| P4-3 | Goalie mask paint editing on top of the mask-pair card | D | Team mask paint replaced in-archive |
| P4-4 | New-id registration (after P2-6 vault writer and P1-4 add-member) | R + D | Clone/new entry qualified separately |

### Wave 5 — Presentation and audio replacement (parent Phase 5)

| ID | Work | Type | Exit |
|---|---|---|---|
| P5-1 | Boards and scorebug inventory on PS4 (no code exists on either generation for the scorebug) | R | Asset locations/bindings |
| P5-2 | Board/scorebug texture replacement via the member route | D | Correct venue/overlay in play and replay |
| P5-3 | GNF coverage: BC6H decode/encode, two-stream write, small-BC5 de-tile, cubes | D | Census decode/write 100% or recorded refusals |
| P5-4 | Audio encoders: EA-MP3 (15–17) and EA-Opus (18–21) for existing-slot replacement; menu music | R + D | Replaced cue plays in game |
| P5-5 | Original-console PS4 packaging (Q04) | R + D | Packaged output installable on console |

### Sequencing notes

- Wave 0 and P1-1 can start immediately and in parallel. P1-1 is user-run runtime work, and its
  verdicts determine P1-2/P1-3.
- The vault writer (P2-6) is the longest pole for asset parity; start its research during Wave 1.
- Headmesh import stays with the external converter stream. Studio prepares the route only on
  integration.
- League/team creation, executable patching and arbitrary vault record creation remain outside the
  beta per the October 1 decisions.

## 8. Open questions for the architect

1. Is committing the genuine NHL 20 database/dictionary as an anchor acceptable under the EA-data
   policy, or should NHL 20 stay private-fixture-only?
2. Should Wave 1 (delivery path) be treated as a gate in front of further PS4 asset packages in
   Phases 2–5, as recommended here?
3. Is a vault writer in scope for the beta, or final release only?
4. Who runs P1-1 precedence cards, and on which shadPS4 build (E1 used a modified v0.16.1)?

## Sources

Developer audits of `crates/`, `app/src` and `defs/` on 13 October 2026. Key records:
`docs/validation/phase-{1,2,3,4}-coverage-matrix-*.md`, `docs/qa/overnight-2026-10-03/nhl15–21.md`,
`docs/validation/save-editor-2026-10-12.md`, `docs/receipts/league-nhl21-e1.md`,
`docs/formats/ps4-{eb-big,gnf-texture,rsf-mesh,save-container}.md`,
`docs/audits/nhl21-ps4-head-asset-binding.md`, `docs/plans/builder-knowledge-gap-register.md`,
`docs/validation/c0-05-fixture-runtime-register.md` and the overnight research folders 2026-09-18
through 2026-09-28.
