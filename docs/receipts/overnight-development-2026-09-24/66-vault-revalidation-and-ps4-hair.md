# Receipt 66 (consolidated) — dispatch after receipt 65: source revalidation at Apply, scratch ownership, PS4 hair, r66

**Developer-verified; not Codex-accepted; nothing launched, listened to, game-tested, staged, committed, uploaded or sent.** HEAD
`8d60a6d582c1bf208e0a5bc590b28d5add7d5051` (unchanged). Direction: [dispatch after receipt 65](../../plans/development-20260928-after-receipt65.md),
[receipt-65 review](../../validation/development-20260928-receipt65-review.md). Claim `dev-vault-revalidation-and-ps4-hair`.

## 1. Plain-language summary

- **A PS4 head change now checks the game files as they are at the moment you press Apply.**
  - Before, Studio could reuse its memory of the game's head list when the file's size and date looked unchanged. Different
    bytes with the same size and date could pass.
  - Now every Check and every Apply re-reads the relevant game data and fingerprints its exact contents, plus the file Studio uses
    to recognise which game it is. If anything changed since you checked, nothing is written.
  - Studio still keeps a parsed copy for speed, but only reuses it when the fingerprints match exactly.
- **What "the plan changed" means is now honest.** A review is invalidated when anything you were shown or anything the write
  depends on changes: the cells it writes, its evidence, the game data's fingerprints, or the warnings and notes. A change to an
  unrelated field of the same player does not invalidate it.
- **PS4 players can be given another hair style the game ships** (NHL 17–21, including genuine NHL 20).
  - Studio checks the number against the open game's own hair list, and changes only the hair number, as one undoable edit.
  - It does not claim anything about colour, shape, helmet fit or what the game actually draws.
  - NHL 15 / 16 are refused: their data has no hair list to check against.
- **Hockey Shop installs can no longer delete another program's file** that appears with Studio's temporary name during an install.
- **Tests no longer reuse or overwrite shared temporary folders.** The old `nhlms_ps3_dds_portrait` folder is left untouched.
- **New internal-review bundle r66**, with receipts through 66. r64 and earlier are kept.

## 2. F2 — the head route's source is revalidated at Check and Apply

**Audit (head and hair write paths first).** `Registry::ps4_head_catalogue` keyed its cache on title, install path, `boot.big`
length and modification time; Check and Apply both called it. The same shortcut exists in `ps4_mask_catalogue` and
`ps4_gear_catalogue` (`registry.rs`), which feed **read-only** displays only (goalie mask pair, team mask paints, skater gear). They
take part in no write and are recorded here, not changed. No other length/time cache key exists in `crates/`.

**Correction.**
- `ps4_vault::read_install_vault` hashes the identity-definition file (`defs/ps4/vault-identities.json`) as read, next to the four
  member hashes it already computed (`VaultSource.identities_sha256`).
- `VaultSource::identity_key()` = archive + definitions hash + every member's full hash. `require_title()` holds the source-mismatch
  and no-identity refusals, now shared by heads and hair.
- `Registry::ps4_vault_catalogue` (the new shared helper for write paths) reads, hashes and identity-checks the members on
  **every** call. It reuses the cached parse only when `title | identity_key` matches exactly. A cache hit can no longer stand in for
  source validation.
- The plan carries `source` (the five exact hash lines), and the DTO exposes it as `source_identity`, behind a disclosure in the UI.
- **Fingerprint scope** now binds: the title, platform, record, player name and target; each written cell's stored bits; the
  evidence; the exact source identity; the refusals; and every note (sharing, duplicate face, exceptions). Unrelated fields of the
  row are deliberately not bound. The Apply refusal says exactly that: "the cells it writes, its evidence, the attached source's
  identity or what it discloses changed".

**Intentional granularity.**
- Definitions are bound as the **whole file**. Any edit refuses a pending review, including another title's entry or the `about`
  text.
- The defs **root path** is not bound; its content is. Another root with identical content keeps the review valid. This is tested.
- The archive path is not in the key; the member bytes are.

**Limits.**
- The vault is read outside the workspace lock, and the write follows inside it. A change in that window is not detected. This is
  source revalidation per call, not protection against a concurrent writer.
- Portrait and gen7 head evidence is file presence by path, re-checked at Apply. It is not content-hashed; `source` is empty there.
- Cost: every Check/Apply now reads and hashes about 4 MB of decompressed members. In the debug test, the warm Check took 2.6 s
  and the cold one 4.5 s. Release timing is not measured.

## 3. Scratch and temporary-file ownership

- `studio-domain` `assets.rs` `ps3_dds_portrait_write_roundtrip_synthetic` now uses an exclusive per-run folder. It is created with
  `create_dir` and removed by a guard. `H:\TEMP\nhlms_ps3_dds_portrait` was verified unchanged after the run (same two files, 07:46:37
  timestamps).
- Hockey Shop `write_new` now returns `NotCreated` (for example `AlreadyExists` after the earlier check) or `Created` (this call made
  the file, then failed). `write_new_owned` removes a path only for `Created`, and names it if that removal fails.
  - This is used for the temporary file, the backup, the recovery copy and the restore temporary file.
  - A partial backup that this call created is now removed; before, it was left behind.
- F1 behaviour is preserved: the backup is kept on a failed restore, and cleanup is reported honestly.
- **Claim limit:** this is name ownership per call. It is not general concurrent-writer protection.
- The `hockey_shop` test scratch is now exclusive too: `create_dir` with a process + counter + time name, and no pre-clear.

## 4. PS4 hair reassignment

- **Measured before coding (temporary probe, deleted).** The NHL 20 (CUSA15439) and NHL 21 (CUSA18182) schemas both have the class
  `ge_nhl_skaterhairtexturepack`. It has exactly one attribute whose type is
  `EA::AttribSys::Enums::GenericHairStyle::GenericHairStyle` (key `0x9def4631`), and the attribute's name hashes to the same key.
  - Each title has 86 records and 86 distinct values (0–1012), with none ambiguous.
  - NHL 15 has no class.
- **Domain.** `ps4_hair::HairCatalogue` identifies the attribute by its type's last segment. It must be unique on the class, and
  values come from the records, never from record order.
  - `plan_hair_reassign` admits PS4 NHL 17–21 only. NHL 15 / 16 are refused by name, and so are other platforms.
  - It resolves `hairid` by metadata name. The NHL 21 field is 11 bits, −1..2046, with −1 as the "none" sentinel.
  - Refused: a missing or unreadable field; negative or out-of-range values; the same value; absent or ambiguous targets; a foreign
    or unidentified vault; no install attached.
  - Only `hairid` is written. The plan says `headid`, `headtexture` and other appearance fields stay as stored.
  - A stored value without a vault record is disclosed as a stock exception, never normalized.
  - Labels stay neutral ("hair style N"); names are not established.
- **API.** `player_reassign(kind = "hair")` goes through the same reviewed-plan path. `target` is now `i64` (Tauri command and
  Registry) so a negative id reaches the plan and is refused there. For portrait and head, a negative target is refused before
  planning.
- **UI.** `PlayerContainmentCard` shows a PS4-only "Reassign hair style" row. It shows the stored value from the plan, and the exact
  source hashes in a `<details>`. Apply sends the reviewed fingerprint.

## 5. Tests (all `CARGO_INCREMENTAL=0`; `NHL_STUDIO_FIXTURES=E:\nhl-studio-fixtures` where needed)

| Suite | Result |
|---|---|
| `studio-api` `ps4_vault_revalidation` (**new**, `--ignored`) | 1 / 1 (82 s) — details below |
| `studio-api` `ps4_hair_reassign` (**new**, `--ignored`) | 1 / 1 (156 s) |
| `studio-api` `ps4_head_reassign` (`--ignored`, refusal wording updated) | 1 / 1 (330 s) |
| `studio-api --lib` (incl. 13 `hockey_shop`, 2 new fault tests; bindings drift test) | 42 / 42 |
| `studio-domain` `player_reassign` (`--include-ignored`; new synthetic hair case) | 5 / 5 (435 s) |
| `studio-domain --lib ps4` · `ps3_dds_portrait_write_roundtrip_synthetic` | 31 / 31 · 1 / 1 |
| frontend `PlayerContainmentCard.test.tsx` (new hair case) · `tsc --noEmit` · `export-bindings --check` | 6 / 6 · clean · in sync (`PlayerReassignPlanDto` regenerated) |
| r66 `verify.py check` · `selftest` · bundle tests from `E:\tmp` | ok · 21 / 21 refused · 3 / 3 |

**`ps4_vault_revalidation`** runs on an owned scratch copy of NHL 21's `boot.big` alone (19 MB, not the install), a copied defs
root, and the NHL 21 anchor database copy. The steps were:
1. **Warm the cache and review** a head (pool 60000) and a hair (0 → 1) change. The plan's five source lines equal an independent
   re-read's full member hashes and the definitions hash. An unchanged re-check gives the same fingerprint (cache reused).
2. **Change a member with length and mtime preserved** (asserted), twice, restoring after each:
   - a byte at 3 840 912 of the `chunkzip` `gccollections.bin` still decodes, to other bytes of the same length. The member
     re-hashes, no title matches, and Apply is refused as "matches no recorded PS4 title identity";
   - the last stored byte breaks the compression; Apply is refused through the decode error.
   - In both cases, for head **and** hair, the player row is unchanged and nothing is pending.
3. **Timestamp-changing replacements:**
   - with the original bytes, the original fingerprint is valid again (identity is content, not metadata);
   - with NHL 20's `boot.big`, Apply is refused as a source mismatch.
4. **Definitions:**
   - editing unrelated `about` text refuses both the head and hair reviews ("differs from the one reviewed … Nothing was written");
   - changing this title's `gccollections.vlt` hash refuses with "matches no recorded … identity";
   - the repository defs root, with identical content, accepts the original review. The apply wrote `headid` and `headtexture` only
     (hair untouched), and one undo restored it.
5. **Scope limit:** the fault tests do not reproduce a real concurrent writer, and nothing ran against the original code in this
   receipt. The pre-fix exposure rests on the coordinator's key-collision probe and on source reading.

**`ps4_hair_reassign`** covers genuine NHL 20 (private fixture `databases/nhl20-ps4-cusa15439/inxp_00.big`, copy SHA-256
`1e200478…a57b`; install `boot.big` `fed97304…40db`) and the NHL 21 anchor (copy `90ee1ec4…e9f9`; `boot.big` `1b794ece…6e68`). The
`anchors/ps4/nhl20` files are NHL 19 base and were not used as NHL 20. Per title:
- the valid target is Paul Stastny, 0 → 1: `hairid` alone, five source lines, the helmet/colour disclaimer;
- refused, and an apply of each writes nothing: absent (73), −1 (negative), 2047 (out of range);
- the shipped vault has 0 ambiguous values; the ambiguous refusal is the synthetic domain case;
- a stale review is refused after the cell changed, and the stock-exception note appears;
- one apply, undo and redo; save to a copy and reopen keep the value, with `headid` / `headtexture` unchanged throughout.

Also covered: the foreign vault (NHL 21 project, NHL 20 install: source mismatch); a missing `hairid` (metadata copy: refused,
nothing pending); NHL 15 (refused, vault not read).

**First failures, kept:**
- Three test files first failed to compile: `unwrap_err` needs `Debug` on the DTO. They were fixed with `.err().expect`.
- A closure type mismatch in the registry was fixed with a shared constant.
- The first revalidation run exercised only the decode path (the flipped byte broke compression). The test was strengthened to
  also find a decodable same-length change, which is recorded above.
- The first assembly embedded a placeholder `built` value. I removed that zip, and only that one, after a hash check
  (`acf83ef4…48d3`), and assembled again with the real times.

**Not run:**
- full `cargo test --all` or the full frontend vitest suite;
- fmt / clippy (C0.2 baseline);
- Windows CI;
- any app launch, listening or game run.

## 6. Bundle r66

| Item | Value |
|---|---|
| Bundle | `E:\nhlms-review-build-r66-2026-09-28\NHL Modding Studio rc-20260928-r66\nhl-studio.exe`, SHA-256 `483b498d941d476048fde56af320121499900c0064bf1e0af77dac0d982d8fbb` (50 691 584 bytes) |
| Archive | `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r66.zip`, SHA-256 `7072c276c8ebd14f8f23256e090ef6f3dd0588a3948446101ad5546f306f73b5`, 620 files, 46 070 445 bytes |
| Inputs | HEAD `8d60a6d` + 955 dirty inputs (manifest `aedf2a10…0f6e`), identical after the build; built 15:28:48 → 15:32:43 MDT (session clock) |
| defs / frontend / helpers | `20c3b183…cf8b` (unchanged from r64) / `ce77b44a…f6bd` / `1caa5f4d…f5c4e`; helper `dc3533ed…8c21`; FFmpeg source `7138d28c…78a3` |
| Contents | r64's contents plus receipt 66 (F2 correction, `write_new` ownership, exclusive scratch, PS4 hair). Receipt 65 is a record only |
| Record | `distribution: internal-review`, `release_ready: false`; blockers: MIT holder unrecorded, 53 artwork files rights unrecorded (internal bundling by the user's decision) |
| Evidence | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r66/` |

New cards (user): **V-96**, PS4 hair Check / Apply / undo in r66, including the disclosure of the checked source. **V-97**, a
stale PS4 head or hair apply after the attached `boot.big` changes. Runtime **R15-HAIR-1** (NHL 21 PS4): does a changed `hairid` show
the other style? It stays a user observation.

## 7. Remaining beta scope and blockers (ranked; nothing is excluded by silence)

| # | Item | Owner | Smallest next action | Kind |
|---|---|---|---|---|
| 1 | User observation of delivered previews and writes (r66 cards V-82 → V-97, A-XMA-5 → 8; game cards P1-A-q, T-3b, P34-1 / R15-HEAD-1, R15-HAIR-1, R36-PS3-KIT-1) | user | run the cards on r66 | observation |
| 2 | Coordinator review of receipts 61–66 and earlier unreviewed parts of 46–59 | coordinator | review | review |
| 3 | **Austrian phased schedule generation** | coordinator (contract) ← user card P1-A | phase-semantics card on a copied NHL 21 save, then a contract decision | withheld (in scope) |
| 4 | **Board-art replacement** (R137 §5) | coordinator → developer | a dispatch lifting the withholding with the review's corrections | withheld (in scope) |
| 5 | **Archive re-key** | coordinator | contract decision | withheld / parked (in scope) |
| 6 | Goalie mask selection at runtime (G-MASK-1) | user | card | observation |
| 7 | Recompiled beyond reads: loader precedence, saves, roster ceilings | user / research | runtime card | observation |
| 8 | NHL 07 / 08 generic heads (P34-G08) | user | card | observation |
| 9 | Read-only mask / gear caches still keyed on length / time (display only, no write) | coordinator | decide whether display freshness needs the same identity key | recorded |

Outside beta (unchanged): league / team creation / cloning / deletion and executable patching. Projects / Builds are deferred.

**Release inputs:**
- The MIT copyright holder is still unrecorded, so `LICENSE` is withheld.
- The artwork stays bundled for internal builds by the user's decision. It keeps `rights: unrecorded`, and external release is
  refused until the backlog is resolved.
- The final distribution review belongs to the coordinator.

## 8. Cleanup

- **Removed:**
  - the temporary schema probe `crates/studio-domain/tests/r66_tmp_probe_hair.rs`;
  - this run's first r66 zip (hash-checked, own);
  - the selftest scratch (removed by itself; confirmed absent);
  - all `nhlms-r66-*`, `nhlms-r66-hair-*` and `nhlms shop txn *` test folders (removed by their guards; confirmed absent).
- **Left, not this run's:**
  - `H:\TEMP\nhlms-league-campaign-all-97012`, `nhlms-logos-gen7-{102420,106396,106856}`, `nhlms-p3-op-{24084,99420}`;
  - `nhlms_ps3_dds_portrait` (now unchanged by tests);
  - `nhlms-research-c38-visual-2026-09-28` (research lane).
- **Kept:**
  - bundles r33 → r66 and the r48 / r53 / r56 / r59 / r64 / r66 zips;
  - `E:\nhlms-release-candidate-20260928\stage` (the assembler's own working folder, reset each run, as for r64);
  - `C:\nhlms-toolchain\`, the fixtures and the shared `target\` cache.
  - E: is tight: about 2.8 GB free before the build.
- The claim `dev-vault-revalidation-and-ps4-hair` is `ready`. No process of this run is running.

## 9. Next task

**Coordinator review of F2's correction and the hair write contract (receipt 66).** The work stays within the recorded scope. If no
defect comes back, the next developer package is the smallest withheld item a coordinator dispatch lifts (rows 3–5). Nothing here
starts it.
