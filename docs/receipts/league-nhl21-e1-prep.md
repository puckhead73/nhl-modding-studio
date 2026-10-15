# NHL21-E1-PREP — archive-only candidate preparation receipt (2026-09-17)

**Plain-language summary.** The exact file for the first modified NHL 21 run is now built,
checked and parked, and nothing in the game installation was changed. Studio's own save path
produced the edited game archive (league 8's name key `Allsvenskan` → `TXT_ROW_LEAGUE`, one
pending change, using NHL 21's own dictionary), and its hashes are exactly the ones every earlier
offline and desktop run produced. Every changed byte is accounted for: 14 bytes of the new key and
4 bytes of one recomputed checksum inside the database member; the other 2 949 archive members,
all table metadata and every other byte are identical to the original. A hash-gated install/rollback
script that swaps the live archive only by renaming fully verified files is ready but was **not
run** beyond its read-only preflight, and a user-driven checklist covers the stock restart check
still owed from E0, the modified boot, and the exact restore. Ready for E1 once Codex approves the
live replacement.

---

## 1. Package, disposition, scope

* **Package:** NHL21-E1-PREP per [league-nhl21-e0-review-and-e1-prep-dispatch](../validation/league-nhl21-e0-review-and-e1-prep-dispatch.md).
* **Disposition: ready for E1 execution** (no blocker). Live install untouched (`inxp_00.big`
  re-hashed `1c41f578…` at closeout); no emulator launch; no source edits; nothing staged,
  committed or pushed; no anchor added.
* **Studio path used.** A scratch binary (`local/runs/league-nhl21-e1/prep/`, retained as a review
  input) drives the same Registry seam the app's Pending Edits page calls: `open_workspace_from_paths`
  → `attach_dictionary` → `list_leagues` (resolve by key) → `set_field` → `pending_changes` →
  `copied_target_review_archive_member` → `copied_target_apply` → `copied_target_restore`. No
  hand-patching. It was compiled once into a package-owned target directory (removed, §7).

## 2. Source revalidation (all OK, `prep-report.txt`)

| Source | SHA-256 |
|---|---|
| `H:\…\CUSA18182\databigs\inxp_00.big` (777 373 328 B) | `1c41f578a595dbd5aae7ad921445626636f3a842d8fb869bc054d35e3e849e87` |
| member 2652 `nhl/data/common/gamemodes/database/t3db/nhl_game.db` (raw, 11 209 672 B @ 726 230 496, `+0x0C 0xe6955448`, `+0x10 0x00429ca7`) | `90ee1ec4…` = anchor |
| `anchors/ps4/nhl21/databases/nhl_game.db` / `-meta.xml` | `90ee1ec4…` / `0b262e4a…` |
| `H:\…\databigs\Extracted\nhl\data\kettle\loc\eng_us.db` (title's own dictionary) | `24693e2f…` |
| `sce_sys/param.sfo` | `bb8c97e3…` (CUSA18182, APP_VER 01.00) |

Archive layout re-bound: version `0x00110400`, 20-byte rows, 2 950 files, path hashes verified.

## 3. Studio run (developer, this host)

* Copied root `E:\…\local\runs\league-nhl21-e1\copied\CUSA18182` (marked): anchor DB + meta at the
  game-tree `t3db/` path, `databigs/inxp_00.big` copied from the install; all three hash-verified.
* Dictionary attached: `eng_us` 55 766 rows, SHA-256 `24693e2f…`. Catalog: league **code 8,
  record 8, key `Allsvenskan`, label "HockeyAllsvenskan", 14 members**; code 29 `TXT_ROW_LEAGUE` →
  "Rest Of World".
* Edit: `stockleagues[8].name := TXT_ROW_LEAGUE`. Pending review: **exactly one cell**
  (`stockleagues / record 8 / name : "Allsvenskan" → "TXT_ROW_LEAGUE"`), no structural changes,
  integrity: file header unchanged, one table header (`mkrU`) changed, 0 other bytes. Catalog
  after the edit: code 8 label "Rest Of World".
* Review (archive member): handle 0, txn `000001-1789709618663-0555655fccf8`, mode `copied-target`,
  **qualification_state `unqualified`, live_install_allowed `false`**. Declared outputs: primary
  `343404153ae5117708e737ec69c9824a816bc77560c1b7a6ea825833da59b569` (11 209 672 B); archive
  `4876cd2b3f4b51878fcbf08f8a56249df2eecbc6c086340b869fc9eada9da582` (777 373 328 B).
  Qualification items carried verbatim: resolved — `+16` path hash, `+4` word, header `0x20..0x30`,
  gaps/pads, `archive_size`; **unresolved** — `+12` PS4 sort key (preserved verbatim, formula
  unknown), header word `0x10` (222480, meaning unknown), entry 2652 slot replacement acceptance,
  native loader acceptance, native acceptance of the saved database. These are the warnings the
  E1 run is designed to observe against.
* Apply: `committed`, `applied_unqualified = true`, both destinations written with the declared
  identities.

## 4. Verification (all passed; one harness label corrected below)

| Check | Result |
|---|---|
| Loose output hash = declared = accepted X (`343404153ae51177…`) | **yes** (full hash above) |
| Archive output hash = declared = accepted X (`4876cd2b3f4b…`) | **yes** (full hash above) |
| DB record-level audit (`tdb-core` parse of both files) | only `fsLK#8` differs; header CRC ok; size ok; length unchanged |
| DB byte-level diff | exactly **2 ranges**: `[0x63ef82,0x63ef90)` 14 B = the `name` field (`Allsvenskan\0\0\0` → `TXT_ROW_LEAGUE`); `[0x63f2b8,0x63f2bc)` 4 B = `942d1888` → `fcb88812` |
| The 4-byte range accounted for | it is the next table's (`mkrU`) `prior_crc` word — CRC32-MPEG2 over fsLK's body, LE-stored — recomputed: anchor `88182d94` ↔ stored `942d1888`; X `1288b8fc` ↔ stored `fcb88812` (independent Python recomputation from `tdb-core`'s documented scope). This matches the pending review's "table header `mkrU`" integrity line. **The harness printed `UNEXPLAINED` for it and exited 2 — that was a gap in my classifier (it only knew the fsLK block), not in the artifact.** |
| Archive member bytes == loose DB bytes | yes, `[0x2b4965e0, 0x2bf471a8)` |
| Archive entry metadata (2 950 rows: path, offset, size, `+4`, `+12`, `+16`) | 0 rows differ; header/table prefix (284 496 B) identical; path hashes verified |
| Archive byte-level diff | 2 ranges, 18 bytes, **all inside member 2652**; 0 ranges outside; the member-internal ranges equal the DB ranges shifted by the member offset |
| Candidate copies | `artifacts/inxp_00.big.X-candidate` = `4876cd2b…`; `artifacts/nhl_game.db.X-loose-evidence` = `343404…` |
| Restore of the copied root | `restored`; DB back to `90ee1ec4…`, archive back to `1c41f578…`; root status not blocked, apply allowed, 0 handles/reservations/leftovers; fresh reopen shows key `Allsvenskan` |
| `deploy.ps1 -Mode preflight` (read-only) | passed: emulator stopped; no loose game-tree DB; mods/patches/cheats/custom configs empty; H: 28 159 MB free; profile and cache present; live = original, candidate = X, rollback copy = original. **Nothing changed** (`deploy.log`) |

Placement readiness is therefore **not paused**: every differing byte is explained and the outputs
are the accepted deterministic X identities.

## 5. Manifest, deployment method, checklist

Manifest with absolute paths, hashes, sizes, disk needs, preconditions and the install/rollback
sequences: `E:\Repositories\nhl-modding-studio\local\runs\league-nhl21-e1\MANIFEST.md`
(SHA-256 `3b5b3165…`). Method: `deploy.ps1` (`3630a8d1…`) — the live `inxp_00.big` is changed only
by same-directory renames of fully copied, hash-verified files (candidate → `.e1-incoming` →
rename; original kept as `.e1-original`); every mode refuses unexpected hashes, a running
emulator, a loose game-tree DB or non-empty mod/patch/cheat folders. Only `databigs\inxp_00.big`
is a destination; the loose DB artifact is evidence and is never deployed for E1.

**User-driven run checklist (for the E1 dispatch; not executed here).** Automated game input
stays off unless the user asks for it.

1. **Close the E0 gap (stock).** Emulator stopped → `deploy.ps1 -Mode preflight` → cold boot the
   unmodified game with the existing `Profile0` → note whether the legal/favourite-team prompts
   reappear and that the profile loads → Rosters / Edit Player: HOCKEYALLSVENSKAN (F12) → back out
   and re-enter the same screen (F12) → clean exit. Record boot time.
2. **Only after Codex approves the exact candidate:** `deploy.ps1 -Mode install` (hash-gated) →
   cold boot → **Rosters / Edit Player first** (expected **REST OF WORLD** in the Europe group where
   HOCKEYALLSVENSKAN was; F12) → **Play Now / Select Team second** (home-side wheel; F12) →
   unrelated checks: SHL, DEL, LIIGA, ICE HOCKEY LEAGUE, NHL™ tiles unchanged; one Allsvenskan team
   (AIK) still listed with a player; Jets label/ratings on Select Team → re-enter Edit Player (F12)
   → clean exit → second cold boot to Edit Player (F12) → clean exit. A boot alone, or an unchanged
   label, proves no source/consumer (card §1.3); record observations only.
3. **E5 restore.** `deploy.ps1 -Mode rollback` → verify live = `1c41f578…` → cold boot → Edit
   Player shows HOCKEYALLSVENSKAN (F12) → clean exit. Preserve `Profile0`, caches, logs; copy the
   run log after each boot; record any new state rather than clearing it.

## 6. Retained artifacts and their dependency

All under `E:\Repositories\nhl-modding-studio\local\runs\league-nhl21-e1\` (hash list in
`SHA256SUMS.txt`); **successor dependency: E1 execution after review**, then removable after E5
closes:

| Item | Bytes | Why retained |
|---|---|---|
| `artifacts/inxp_00.big.X-candidate` | 777 373 328 | the E1 candidate |
| `artifacts/nhl_game.db.X-loose-evidence` | 11 209 672 | evidence of the member bytes (never deployed) |
| `copied/CUSA18182/` (marked root; DB/meta/archive restored to originals) | 789 MB | rollback source 2; reusable for a re-review if Codex asks for a re-run |
| `copied/CUSA18182/.tdb-txn/000001-…/` (journal + `backups/000.bak`, `001.bak`) | 789 MB | engine-owned recovery backups of the transaction (state `restored`); left to the engine's lifecycle, not deleted by hand |
| `MANIFEST.md`, `deploy.ps1`, `deploy.log`, `prep-report.txt`, `prep/` (source + lockfile), `SHA256SUMS.txt` | small | manifest, method, verification transcript, reproducibility |

Total ≈ 2.35 GB on E: (36 GB free after cleanup). No copy of any other game file was made.

## 7. Cleanup

* `local/runs/league-nhl21-e1/target/` (1.3 GB release build cache) — **removed** (literal path,
  containment/reparse checks; absent afterwards).
* No process left running; no emulator started; installed game read only (live archive re-hashed
  `1c41f578…`; `databigs/` contains only `inxp_00.big`, no `.e1-*` files).
* Index empty; anchors untouched.

## 8. Next action

Codex: review §4 (especially the `mkrU.prior_crc` accounting), the manifest and `deploy.ps1`.
If accepted, dispatch **NHL21-E1** as a user-driven run following §5 — step 1 (stock restart /
re-entry) first, the live install only on explicit approval of candidate `4876cd2b…`, then the
modified boots and the E5 rollback. Stopping here; no live replacement, no launch.
