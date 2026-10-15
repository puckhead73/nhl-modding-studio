# Receipt 03 (T2) — Teams 2.6: PS3 NHL 12 / 13 kit components replaced inside their render archives

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Nothing staged, committed, pushed or
launched; the real PS3 NHL 13 tree on `X:` was read only (one member planned, nothing applied);
every write happened in a temp copied-target root. Claim: `dev-t2-ps3-kit-member-replacement`.
Research consumed: R76 (2026-09-22 pm handoff 08 / queue rows 8 and 11). Clock: 05:38 → 05:52 (MDT).

## 1. Plain-language summary

On PS3 NHL 12 and 13 the jersey, pants and sock textures are not loose files: they live inside two
large render archives. Until now Studio listed and previewed them there but refused to write,
citing a missing encoder for the "chunklzx" compression. Research R76 measured that these members
are actually **chunkzip**-wrapped, a form Studio already encodes natively for other archive members.
This package corrects that record (dated notes on the 23 Sep receipt 04 and STATUS) and wires the
kit card to the existing archive-member route: **Replace in archive…** on a present archived
component plans the rewrite (decode → same RW4ps3 texture writer as loose files → re-encode as
chunkzip → placement predicted), Apply holds it as a pending replacement, and the next copied-target
save installs it into the archive copy, with restore. A synthetic PS3 NHL 12 install proves the
whole route end to end (both archives, same-size and changed-size members, other members
byte-identical, restore byte-exact); the **real** NHL 13 jersey member was planned through the
route without writing. Loose kit components on the card gain a **Replace…** through the reviewed
loose route; the NHL 11 / 12 sock *pair* is deliberately not offered file-by-file.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-api/tests/ps3_kit_member_route.rs` (**new**, sha256 `88963b0d…`) | default: synthetic PS3 NHL 12 install (anchor DB in a copied-target root, `cacherender.big` + `nocacherender.big` in the stager's qualified `0x400` layout around a synthetic RW4ps3 raster, chunkzip-wrapped) → kit card addresses (`PS3_GAME/USRDIR/<archive>` + member path) → `asset_member_preview` (texlib, `chunkzip`, no chunklzx wording) → apply → copied-target review / apply committed → installed members decode to the new pixels (64×32 same-size jersey; 128×64 resized sock in the other archive) → other members and the DB byte-identical → the route's own preview reads the new image → restore byte-exact; refusals (`not-found`, `invalid`, `texture`) write nothing. `--ignored`: the real NHL 13 jersey member **planned** only |
| `app/src/components/TeamKitsCard.tsx` (sha256 `3855ece9…`) | hosts `useArchiveMemberReplace` (dialog + result rendered on the card; re-lists after apply); **Replace in archive…** on present archived components; **Replace…** on present loose components through the caller's reviewed loose route; the sock pair (`sock_grammar != single_file`) shows "pair; not replaced file-by-file" with the reason |
| `app/src/components/UniformsTab.tsx` | passes its `useTextureReplace().pick` as `onReplaceLoose` |
| `app/src/components/TeamKitsCard.replace.test.tsx` (**new**, sha256 `0514b247…`) | 2 cases (§4) |
| `app/src/components/TeamKitsCard.test.tsx` | project-context and dialog mocks added (the card now hosts the hook); assertions unchanged |
| `docs/receipts/overnight-development-2026-09-23/04-r4-archive-member-preview.md`, `…/STATUS.md` | dated correction notes (chunkzip, route implemented); original wording kept |

No backend code changed: the archive-member route (`asset_member_edit.rs`, D8 / C3 / C7) already
dispatched on the stored form (`chunkzip` → native encoder, `chunklzx` → store route, raw) and on
the member's container (RW4ps3 through `replace_texture_in_file`).

## 3. Evidence

- Synthetic route ([`t2-checks.log`](../../validation/evidence/overnight-development-2026-09-24/logs/t2-checks.log)):
  jersey plan "156 bytes fit the member's slot (2537 + 7 slack)", sock plan "177 bytes fit …";
  installed members still `chunkzip`-wrapped, decode to the green / blue sources; pant copy of the
  same bytes did **not** follow (distinct block, not a dedup group).
- Real member ([`t2-ps3-kit-member-real-plan.log`](../../validation/evidence/overnight-development-2026-09-24/logs/t2-ps3-kit-member-real-plan.log)):
  `nocacherender.big:rendering/jersey/texlib_0_0_0.rpsgl` — RW4 texlib **1024×1024, chunkzip**,
  1 255 445 → 555 058 stored bytes for a solid source, "fit the member's slot (1 255 445 + 11
  slack)"; the archive's identity (length + hash) unchanged after planning. The 1.6 GB archive was
  not copied; the copied-target apply on a real archive is **not** exercised here (a real-install
  card, R36-PS3-KIT-1).

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-api --test ps3_kit_member_route` | **1 passed**, 1 ignored (real tree) |
| `… -- --ignored` (real NHL 13, read only) | **1 passed** (76 s) |
| `cargo test -p studio-api --test team_kits --test archive_member_preview --test asset_member_route --test gen7_asset_member_route --test arena_assets` | see `t2-checks.log` |
| `npx vitest run src/components/TeamKitsCard.replace.test.tsx src/components/TeamKitsCard.test.tsx` | **7 passed** (2 new: loose Replace… → the caller's route with the exact rel path; Replace in archive… → picker → `asset_member_preview` with the exact `installRoot` / `archiveRelative` / `memberPath` / `wholeGroup: false`, refusal shown, no apply; missing components offer nothing; the sock pair is not offered and says why, pants on the same title still are) |
| `npx vitest run` (full) | **33 files, 221 passed** |
| `npx tsc --noEmit`; `export-bindings --check` | clean; in sync |

## 5. Decisions and limitations

- **Primary raster only.** The member route replaces the member's primary raster (`texture_index`
  none); a PS3 jersey texlib that bundles a number font as a second raster is not addressed per
  raster here (the loose route does that through `texture_index`). Recorded, not claimed.
- **Sock pair.** R74 gives per-title aggregate classes (mirror 459 / 704, identical 153 / 124,
  other 120 / 119, flat 24 / 26), not a per-pair class, so a single-file write cannot claim to
  keep the pair consistent; both files are required parts, and the card says so instead of offering
  the write. A paired write needs a per-pair measurement (developer-side, bounded) or a research row.
- **Loader acceptance not established**: a re-encoded chunkzip member in a rewritten archive copy
  is a runtime question for the user (card R36-PS3-KIT-1). The preview text says the change reaches
  the archive copy, not the game.
- No dedup groups were observed in the synthetic case; a shared block on the real archives would
  take the existing private-copy behaviour (C7), unchanged and not re-tested here.

## 6. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `patch_t2_frontend.py` | kept until the closeout receipt, then deleted |
| Temp roots `nhlms-ps3kit-*` | removed on drop |
| Background processes | the two evidence-log runs (§3) finish on their own; none at closeout |

## 7. Next action

P4 — Teams 2.7 diagnostic audio, starting with the pure-Rust Layer III decoder decision.
