# Receipt 04 (R4) — Teams 2.6: archive-backed kit members preview through the render archives

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). The real PS3 NHL 13 render archive on `X:`
read **only** (front tables plus one member); the seam test's install root is a disposable temp
directory. Nothing staged, committed, pushed, launched or written live. Dispatch: item 4 of
[the coordinator review of receipts 10–17](../../validation/overnight-development-20260922-receipts10-17-review.md)
(Teams, operation by operation), the exact next task named by receipt 03. Claim:
`dev-r4-archive-member-preview`. Read-only; no kit reassignment and no archive write.

## 1. Plain-language summary

Receipt 03 listed the PS3 NHL 12 / 13 kit textures from their two render archives by name only, so
the kit card showed a "big" marker where the loose-file kits show a thumbnail. Studio can already
decode the archive members (the `chunklzx` payloads), so the card now gets the same thumbnail
evidence for archived members: a new read-only route takes the archive's install-relative path and
the member's exact path, reads just that member from the archive's tables, decodes it and previews
it like a loose file. The kit component carries the archive path and the member path as two fields
so the card never parses a display string. On the real NHL 13 archive a 1024² jersey colour map
decodes this way; nothing is written and no write route for an archive member exists.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/assets.rs` | `decode_texture_file` split into `read_asset_file` + **`decode_texture_bytes`**; **`read_eb_member`** (front tables, one member streamed, `decode_payload`; a member the table lacks or an undecodable payload is an error naming it); **`decode_archive_member_texture`** |
| `crates/studio-domain/src/kit_files.rs` | `KitComponent { archive, member_path }` beside `archive_member` (the display string) |
| `crates/studio-domain/src/lib.rs` | re-exports |
| `crates/studio-api/src/{dto,convert,registry,commands,lib}.rs` | `KitComponentDto { archive_rel_path, member_path }` (the archive resolved beside the rendering root and made install-relative); `Registry::archive_member_texture_preview` (safe-join of the archive path; exact member match); the `archive_member_texture_preview` command. No new export root (`TexturePreviewDto` is reused) |
| `crates/studio-api/tests/archive_member_preview.rs` | **new** — synthetic v`0x00000400` archives holding a real NHL 12 stock row's jersey / pants / both socks (raw RW4ps3 members): the card fields address the members, the route decodes them at full size and as a thumbnail, refusals for a missing member, a non-texture member, an archive path escaping the root and a missing archive; `--ignored`: the real PS3 NHL 13 jersey member |
| `app/src/bindings/KitComponentDto.ts` | **generated** |
| `app/src/components/TextureReplace.tsx` | `ArchiveMemberThumbnail` (the archive twin of `TextureThumbnail`; the `big` marker while decoding or on a decode error, with the error in the tooltip) |
| `app/src/components/TeamKitsCard.tsx` (+ `.test.tsx` fixture) | archived present components render the thumbnail through the new route |

## 3. Evidence

`cargo test -p studio-api --test archive_member_preview -- --ignored` —
[r4-ps3-nhl13-archive-member-preview.log](../../validation/evidence/overnight-development-2026-09-23/logs/r4-ps3-nhl13-archive-member-preview.log),
**1 passed**: `nocacherender.big:rendering/jersey/texlib_0_0_0.rpsgl` → 1024 × 1024, container
`texlib`, not a flat grey. (Measured on the way: NHL 13 keeps its jerseys in `nocacherender.big`
and its pants / socks in `cacherender.big` — 1 782 / 882 / 900 members — which is why the kit
resolver consults both.)

Default seam case (synthetic): 1 passed. `cargo test -p studio-api --lib`: 31 passed (export roots
unchanged at 145). `export-bindings --check`: in sync. `npx vitest run
src/components/TeamKitsCard.test.tsx`: 5 passed. `npx tsc --noEmit`: clean.

## 4. Coverage effect (Phase 2 matrix 2.6)

| Row | Before (receipt 03) | After |
|---|---|---|
| 2.6 PS3 NHL 12 / 13 kit components | present by archive listing; not decoded or previewed | **previewed from the archive member: impl+tested, dev** (synthetic archives; the real NHL 13 jersey member decodes at 1024²); still read-only, no reassignment, no archive write |

## 5. Not done / limitations

- **Correction, 24 Sep 2026 (research R76, consumed by [receipt 03 of the 24 Sep campaign](../overnight-development-2026-09-24/03-t2-ps3-kit-member-replacement.md)):** the PS3 NHL 12 / 13 kit members are `chunkzip`-wrapped, not `chunklzx`; the encoder blocker below never described them. Replacement through the archive-member route is now implemented (synthetic archive route test + a plan-only pass on the real NHL 13 jersey member). The wording below is kept as written.
- Preview only. Writing a texture back into a render archive member is not offered (the `chunklzx`
  form has no encoder in Studio — the existing R30-LZX-1 position stands).
- The real-tree case decodes one member of one title; the twelve-tree kit join stays a listing
  check (it does not decode every member).
- Whether the console draws the archived member for that kit is not claimed.

## 6. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `patch_member_preview.py`, `patch_real_case.py` | deleted at closeout |
| Temp roots `nhlms-ebprev-*` | removed on drop; verified absent |
| Evidence log `r4-ps3-nhl13-archive-member-preview.log` | retained |
| Background processes | none |

## 7. Next action

Closeout of this continuation ([STATUS](STATUS.md)); then the next Teams operation.
