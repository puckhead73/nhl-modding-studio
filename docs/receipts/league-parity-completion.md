# LEAGUE-PARITY-COMPLETION — REG-1, EB-V1, LOC-W1 and batch integration (2026-09-18)

**Plain-language summary.** The completion queue is done and nothing was deployed, launched,
staged, committed or pushed. The mislabelled anchors are now recorded for what they are (the
"NHL 20" set is NHL 19's base game; the PS3 NHL 08/09 dictionaries are the Xbox 360 files), the
correctly identified files are private fixtures with tests that fail loudly when absent, and every
consumer of the old labels is annotated. Studio reads and stages the NHL 15 PS4 archive layout
after all 17 of its archives were measured — including the three whose declared size is smaller
than the file — and both NHL 15 databases have candidate archives. Older titles can now be renamed
in Studio by pointing a dictionary key at another key's text: preview, hold, pending review,
copied-target apply, reopen and restore work as a loose file and as an archive member, the desktop
app was driven through the whole loop on NHL Legacy PS3, and thirteen dictionary candidates exist
for the installed PS3 / Xbox 360 / Recompiled titles, each labelled by how well its league-code →
key binding is known. One manifest-driven install/rollback helper covers all 21 candidate rows
(38 fixture cases), a hash-bound two-file quarantine plan exists for NHL 19's loose tree (tested
on fixtures, not run), and the ordered runtime cards are ready for the joint pass. NHL 07 is
blocked with a specific finding: its league names are not in its dictionary at all.

---

## 1. Package dispositions

| Package | Result |
|---|---|
| **REG-1** | done — provenance recorded ([anchors-manifest](../audits/anchors-manifest.md) "Provenance corrections"; adapter headers `nhl19-ps4.toml` = witness recorded, `nhl20-ps4.toml` = mislabelled; parity matrix §0.2 rows rewritten); hash-bound sources registered as private fixtures under `NHL_STUDIO_FIXTURES` (root on this machine `E:\nhl-studio-fixtures`, 11 files); 8 ignored fail-loud cases (`tdb-loc` 6, `studio-api` 2) executed with the root set; consumers annotated (`tdb-loc/tests/dictionaries.rs`, `studio-api/tests/{leagues,league_offline_campaign}.rs`). **Deferred to the coordinator:** promoting the `nhl19-ps4` adapter blocks (a canonical rewrite) and relabelling/capturing anchors |
| **EB-V1** | done — 17-archive characterisation (format doc §NHL 15 variant): 16-byte rows, path hash on every row, rows sorted by hash, `archive_size` below the length on 3 archives with entries ending at the length; reader tag `0x0010_0400`; stager accepts only that measured shape (`ArchiveSizeInconsistent` otherwise), records it unresolved, preserves the word, refuses length changes; 3 synthetic cases + 2 env-gated real-archive cases (all 17 archives bind; base and patch members replace in place; growth refused on the patch archive); candidates `nhl15-ps4-base` / `-patch` through Studio |
| **LOC-W1** | done — `tdb_loc::{plan_repoint, apply_repoint}` (exact-key then unambiguous-hash lookup; null/equal/collision refusals; per-file CRC verification and allowed-difference audit; round-trip invariant guard); workspace pending rename (preview / set / clear; dirty + revision; adopt on commit; refresh/reload/stale rules); `dictionary` copied-target artifact re-derived at staging; multi-member archive review (`copied_target_review_archive_members`, `primary`/`dictionary` specs); commands + DTOs + bindings; Leagues detail panel, Pending Edits card, copied-target dictionary member option; `defs/leagues/gen7-*.toml` `dictionary_key`/`key_evidence`; catalog labels through the attached dictionary with hash-folded lookup; 13 gen7 candidates; desktop loop exercised (14 screenshots). **NHL 07:** bounded investigation — no league-key family in its 3 952-row dictionary; label source untraced; blocked |
| **Integration** | done — 21 manifests (PS4 + gen7) with `platform` and precise `loose_db_paths`; `deploy.ps1` manifest loader (validation, normalisation, target/identity/root checks, per-platform emulator and overlay gates, MANUAL gates recorded) exercised by 5 new fixture cases (38/38); `quarantine.ps1` (NHL 19: exactly two files, hash-bound, reversible; 8/8 fixture cases; live `status` only); read-only `status` on all 21 rows; refreshed matrix + ordered cards in [league-parity-completion-2026-09-18](../validation/league-parity-completion-2026-09-18.md) |

## 2. Changed paths (working tree; nothing staged)

**Crates.** `crates/tdb-loc/{Cargo.toml, src/lib.rs, src/repoint.rs (new), tests/dictionaries.rs, tests/private_dictionaries.rs (new)}`;
`crates/tdb-archive-big/{src/eb.rs, src/stage.rs, src/stage/tests.rs, tests/ps4_real_archives.rs}`;
`crates/studio-domain/{src/lib.rs, src/dictionary_edit.rs (new), src/workspace.rs, src/copied_target.rs, src/txn.rs, src/leagues.rs, tests/copied_target.rs}`;
`crates/studio-api/{Cargo.toml, src/bindings.rs, src/convert.rs, src/dto.rs, src/lib.rs, src/commands.rs, src/registry.rs, src/registry/copied_target.rs, tests/dictionary_rename.rs (new), tests/league_private_sources.rs (new), tests/leagues.rs, tests/league_offline_campaign.rs}`.
**Frontend.** `app/src/lib/client.ts`, `app/src/components/{DictionaryRenamePanel.tsx (new), CopiedTargetPanel.tsx, CopiedTargetPanel.test.tsx}`, `app/src/pages/{LeagueDetail.tsx, Leagues.tsx, PendingEdits.tsx}`; generated `app/src/bindings/` (`ArchiveMemberSpecDto`, `DictionaryRepointPreviewDto`, `DictionaryRowDto`, `PendingDictionaryDto` new; `PendingReviewDto`, `LeagueSummaryDto` changed) via `export-bindings --write`; drift gate green (84 roots).
**Defs.** `defs/leagues/gen7-{nhl08,nhl09,nhl10,nhl11,nhl12-15,nhllegacy}.toml` (keys + evidence); `defs/adapters/{nhl19-ps4,nhl20-ps4}.toml` (header notes only).
**Docs.** `docs/audits/anchors-manifest.md` (+§ provenance corrections); `docs/formats/ps4-eb-big.md` (+§ NHL 15 variant); `docs/plans/league-identity-parity-matrix.md` (NHL 19/20 rows, banners); `docs/validation/league-parity-batch-2026-09-18.md` (banner); `docs/validation/league-parity-completion-2026-09-18.md` (new); `docs/validation/evidence/league-locw1-desktop/` (new, 14 PNG + driver); this receipt.
**Package (untracked).** `local/runs/league-parity-batch/` — see §6.

## 3. Checks executed (this session)

| Check | Result |
|---|---|
| `cargo test -p tdb-loc` (+ new writer cases, 25-dictionary CRC sweep) | ok — 7 + 5 + 0 (6 ignored) |
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p tdb-loc --test private_dictionaries -- --ignored` | 6 passed |
| `… cargo test -p studio-api --test league_private_sources -- --ignored` | 2 passed |
| `cargo test -p tdb-archive-big` (EB-V1 synthetic) | 56 passed (1 ignored) |
| `NHL_STUDIO_PS4_BIGS=<CUSA00561/databigs> … --test ps4_real_archives nhl15 -- --ignored` | 2 passed (17 archives bound; base + patch member replacement; growth refused) |
| `cargo test -p studio-api --test dictionary_rename` (PS3 loose, X360 archive member, in-place save, gen7 labels) | 4 passed |
| `cargo test -p studio-api --test leagues` (hash-fallback expectation updated) | 3 passed |
| `cargo test -p studio-api --lib bindings` | 25 passed |
| `cargo test -p tdb-canonical` | 552 passed |
| final `cargo test -p tdb-loc -p tdb-archive-big -p studio-domain -p studio-api -p tdb-canonical` | see the appended result block below |
| `app`: `tsc --noEmit`, `vitest run` | tsc clean; 57 passed (CopiedTargetPanel members spec cases added) |
| `deploy.ps1` fixture suite (`tests/run-tests.ps1`) | 38 passed (33 inherited incl. adapted case 10 + 5 manifest-loader cases) |
| `quarantine.ps1` fixture suite (`tests/run-quarantine-tests.ps1`) | 8 passed |
| `deploy.ps1 -Mode status` on all 21 manifests (read-only) | `reports/status-all-2026-09-18.txt`: 19 `S0-stock / MUTATE`, NHL 19 ×2 `REFUSE` (loose gate, by design) |
| `quarantine.ps1 -Mode status` (live, read-only) | IN-PLACE; both files hash to the recorded identities |
| Desktop app (tauri dev + CDP) LOC-W1 loop on NHL Legacy PS3 | executed; screenshots 00–13; dictionary `70814551…` → `b24daa9f…` → `70814551…` |
| Not run | `cargo fmt --check` / `clippy -D warnings` repo-wide (known C0.2 baseline failures); full `cargo test --workspace` (unchanged crates) |

Clippy on the two new/changed engine crates (`tdb-loc`, `tdb-archive-big`) is appended below.

## 4. Per-target readiness (summary; full matrix in the completion record §3)

Ready to place (PASS on every automated gate, manual gates listed): NHL 18, 17, 16, 20 PS4; NHL 15
PS4 base and patch (discriminating pair). Refused until the user's quarantine: NHL 19 base and
patch-56. Closed: NHL 21. Experimental dictionary candidates (binding class stated): Legacy Xbox 360
(traced; loose or packed row by the launched tree), Legacy Recompiled, Legacy PS3 (hdd0 then disc),
NHL 15/14/13 PS3, NHL 11/10/09/08 PS3. Blocked: NHL 12 PS3 (ISO-registered), NHL 07 (label source),
Xbox 360 NHL 07–15 (not installed).

## 5. Remaining blockers / decisions

Adapter promotion and anchor relabelling (coordinator); NHL 19 quarantine and the live-copy
choices for Xbox 360 / PS3 (user, in session); NHL 12 ISO route; missing Xbox 360 installs; gen7
bindings other than Legacy Xbox 360 remain hypotheses until a card corroborates them.

## 6. Artifact manifest and cleanup

Retained, `local/runs/league-parity-batch/` (hash list `SHA256SUMS.txt`; owner: the runtime pass and
Codex's review): `artifacts/` 3.2 GB (9 archive candidates: nhl15-ps4-base/-patch, nhl16, nhl17,
nhl18, nhl19-base/-patch56, nhl20, legacy X360 packed + recomp share one; 13 dictionary candidates;
loose evidence), `copied/` 6.4 GB (restored roots — every journal `restored`; secondary rollback
sources), `sources/` 167 MB, `manifests/` (21), `deploy.ps1`, `quarantine.ps1`, `tests/` (+ results),
`reports/`, `prep/` + `probe/` sources, `gen7-offline.sh`. Private fixtures: `E:\nhl-studio-fixtures`
(outside the repo, 11 files, listed in the two private-fixture test modules).
Removed: NHL 21 batch candidate + copied root (E1 closed; evidence durable under
`docs/validation/evidence/league-nhl21-runtime/e1/`), `local/runs/league-nhl21-e1/` (superseded
scratch), package build dir `target/`, fixture roots `tests/fx*`, session scratch (desktop fixture
root, CDP driver copy kept in evidence, extraction scripts). No live install file was modified,
renamed or created; no emulator was launched apart from the desktop app itself; anchors unchanged.
Disk after cleanup: see the appended block.

## 7. Appended results

* Final suite `cargo test -p tdb-loc -p tdb-archive-big -p studio-domain -p studio-api -p tdb-canonical`
  (working tree, 2026-09-18 03:11–03:45): **59 result lines, 977 tests passed, 0 failed**
  (ignored = private-fixture and env-gated cases); the last two lines are the 8 private-fixture
  cases executed with `NHL_STUDIO_FIXTURES` set (6 + 2 passed). `cargo clippy -p tdb-loc -p tdb-archive-big`:
  no warnings.
* Disk after cleanup: E: 18.1 GB free; H: 27.3 GB free. Package size `local/runs/league-parity-batch/`: 9.8 GB
  (`SHA256SUMS.txt`, 132 entries).

Stopping here for Codex's consolidated implementation review.
