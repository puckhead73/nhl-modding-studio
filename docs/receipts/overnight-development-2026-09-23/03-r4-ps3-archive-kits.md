# Receipt 03 (R4) — Kit inspection parity: PS3 NHL 12 / 13 kits listed from the render archives

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Real Xbox 360 / PS3 trees on `X:` read
**only** (the PS3 archives by their front tables); the unit test's archives are synthetic files in
a temp directory that removes itself. Nothing staged, committed, pushed, launched or written live.
Dispatch: [the coordinator review of receipts 10–17](../../validation/overnight-development-20260922-receipts10-17-review.md),
fresh-session item 3. Claim: `dev-r4-ps3-archive-kits`. Kit reassignment is **not** offered; the
accepted completeness rule of receipt 10 is unchanged.

## 1. Plain-language summary

Receipt 10 left two targets uncovered: PS3 NHL 12 and NHL 13 keep their jersey, pants and sock
textures inside two archives (`cacherender.big`, `nocacherender.big`) instead of loose folders, so
the kit card had nothing to read there. Studio already reads that archive form (the `EB\0\3`
v`0x00000400` tables the PS4 / Legacy readers use), so the kit resolver now lists an archive's
**file table** — names only, no payloads — whenever a kit folder is absent and the archives sit
beside the tree. Everything else stays as accepted: the title's contract still decides what a kit
needs (both sock files on NHL 12), a member in either archive is *present* and names its archive,
a member in neither is *missing*, an archive that cannot be listed makes the part *unknown* rather
than absent, and a loose folder — when one exists — keeps its own evidence. Presence from an
archive is listing evidence: the card marks such components and does not decode or preview them.

On the real PS3 trees, NHL 12 resolves all 676 rows and NHL 13 all 684 — the counts research R56
measured by the same archive listing — with 2 704 and 2 052 components taken from the archives.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/kit_files.rs` | `DirEvidence::Archived { archive, files }`, `RENDER_ARCHIVES`, `list_eb_members` (front tables only), `RenderingListing::read_with_archives` (loose first; absent parts from the archives; unlistable archive → unreadable), `ListedFile` (name + archive), `KitComponent::archive_member`; `team_kits` uses the archive-aware read and adds the archive note. **New** unit case `ps3_archive_backed_kits_are_listed_from_the_render_archives_under_the_same_rule` with a synthetic v`0x00000400` writer |
| `crates/studio-domain/tests/kit_selector_join.rs` | PS3 NHL 12 / 13 added (twelve trees); a present component must be a real loose file **or** name a render-archive member with the platform extension |
| `crates/studio-api/src/{dto,convert}.rs` | `KitComponentDto::archive_member`, `KitDirectoryDto { status: "archived", archive }` |
| `app/src/bindings/{KitComponentDto,KitDirectoryDto}.ts` | **generated** |
| `app/src/components/TeamKitsCard.tsx` (+ `.test.tsx` fixture) | "N members in cacherender.big + nocacherender.big" per directory; an archived component shows a `big` marker instead of a thumbnail with the member path and the "listed from the file table; not decoded or previewed" tooltip |

## 3. Evidence

`cargo test -p studio-domain --test kit_selector_join -- --ignored` —
[r4-kit-join-twelve-trees.log](../../validation/evidence/overnight-development-2026-09-23/logs/r4-kit-join-twelve-trees.log), **1 passed** (75 s):

| Tree | Rows (R56) | Complete | Sock components | From archives | Spares complete / incomplete |
|---|---|---|---|---|---|
| X360 NHL 11 / 12 / 13 / 14 / 15 / Legacy | 627 / 676 / 684 / 704 / 659 / 703 | all | 1 254 / 1 352 / 684 / 704 / 659 / 703 | 0 | 97/20 · 255/22 · 207/1 · 315/2 · 54/0 · 228/58 |
| PS3 NHL 11 / 14 / 15 / Legacy | 627 / 704 / 659 / 703 | all | 1 254 / 704 / 659 / 703 | 0 | 97/20 · 316/2 · 54/0 · 228/58 |
| **PS3 NHL 12** (`cacherender.big` + `nocacherender.big`) | **676** | **676** | **1 352** (both files per kit) | **2 704** | 255 / 22 |
| **PS3 NHL 13** (same archives) | **684** | **684** | **684** | **2 052** | 208 / 1 |

Every present PS3 12 / 13 component names `cacherender.big:` or `nocacherender.big:` with an
`.rpsgl` member; no row was refused; no spare is complete without every member.

Unit (15, was 14): the synthetic two-archive case — both sock files across the two archives make a
complete kit, a jersey-only triple is an incomplete spare, a loose `pant/` beside the archives keeps
loose evidence and is not supplemented from the archive, a garbage archive makes socks *unknown*
(kit *unknown*), no archives at all leaves the part *absent*.

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-domain --lib kit_files` | **15 passed** |
| `cargo test -p studio-api --test team_kits` | **3 passed** |
| `cargo test -p studio-domain --test kit_selector_join -- --ignored` (twelve real trees) | **1 passed** |
| `export-bindings --write` / `--check` | 2 written, in sync |
| `npx vitest run src/components/TeamKitsCard.test.tsx` | **5 passed** |
| `npx tsc --noEmit` | clean |

## 5. Coverage effect (Phase 2 matrix 2.6, gen7 kits)

| Row | Before | After |
|---|---|---|
| 2.6 PS3 NHL 12 / 13 kits | not covered (in-archive) | **read-only join from the render archives' file tables: impl+tested, dev** on the real trees — 676 / 684 rows, 100 % complete under the contract rule; members not decoded or previewed |
| Other ten trees | unchanged | unchanged (rerun, identical counts) |

## 6. Not done / limitations

- Archive members are evidence of presence by name; their bytes are not decoded, so no thumbnail
  and no format check. Previewing an in-archive kit texture would need the archive-member preview
  route, which is not part of this package.
- PS4 and NHL 07 / 08 kits remain unmeasured; NHL 09 / 10 still have no selector table.
- No reassignment, no writer; the sock 0 / 1 sub-index meaning on NHL 11 / 12 is still unmeasured.

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Temp roots `nhlms-kit-eb-*` | removed by the test; verified absent |
| Evidence log `r4-kit-join-twelve-trees.log` | retained |
| Background processes | none |

## 8. Next action

The wider roadmap (dispatch item 4), operation by operation, as usage allows.
