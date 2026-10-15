# L1.1-Q1 — league qualification readiness receipt (2026-09-17)

**Plain-language summary.** The league parity matrix now has a current-state section that says,
per game, exactly what has been proven where (schema, offline editing, dictionary, packaging,
desktop, in-game) and marks the older pre-implementation text as history. NHL 21's own language
dictionary turned out to be available all along inside the installed game's archive: it was
extracted read-only, tied by hash to the archive member and to the earlier localization handoff,
and checked with Studio's own reader — the two keys the planned edit uses ("Allsvenskan" =
"HockeyAllsvenskan", "TXT_ROW_LEAGUE" = "Rest Of World") are both present, and four league labels
differ from the NHL 20 stand-in used earlier, which settles that no other game's dictionary may be
used as a proxy. A step-by-step NHL 21 test card, with equivalents for every other game, now says
how to run the in-game check, what each experiment can and cannot prove, and what to do after
each outcome. Nothing was run in a game, nothing was installed live, and no game acceptance is
claimed for any target.

---

## 1. Package, disposition, scope

* **Package:** L1.1-Q1 per [league-resp-1-review-and-qualification-dispatch](../validation/league-resp-1-review-and-qualification-dispatch.md).
  Read-only investigation plus documentation. LEAGUE-RESP-1 stays accepted; Projects/Builds stay
  final optional stretch.
* **Disposition:** tasks 1–6 done. Phase 1.1 **remains open** (no game acceptance). No application
  code, tests, bindings, anchors or installed files were modified; nothing staged, committed or
  pushed; the pre-existing dirty tree was preserved and built on.
* **Baseline:** `feat/phase5-asset-tooling` at `8d60a6d` + the overnight/R1/R2/SMOKE-1/SMOKE-R1/RESP-1
  working tree (all league receipts and code still untracked/modified as before).

## 2. Changed paths (SHA-256 of the working-tree file after this package)

```text
45a8941f5a12abdb188931cffeaf596c34628faf8e14c57e501409b09711ec86  docs/plans/league-identity-parity-matrix.md        (new §0 current state; dated-assessment notes on §1.4, §1.5, §2, §3.4, §5, §6)
669613cf9a0b108e482fe4d409604a9f345800df8db0553d54613745c99a2636  docs/plans/league-identity-runtime-qualification.md (new)
                                                                  docs/receipts/league-l11-q1.md                     (this file)
```

No other file changed. `git status anchors` empty; index empty.

## 3. What was done, per task

### T1 — parity matrix reconciliation

Added **§0 "Current state per target"** at the top of the matrix: an evidence-class legend
(schema / editor / dictionary / packaging / desktop / in-game, with who established each and
whether it was independently rerun), one row per shipping target (27 + the NHL 07 PS3 research
copy) with the six columns separated, the NHL 21 provenance chain (§0.3) and the own-vs-proxy
dictionary delta. Sections 1–6 are kept verbatim as the dated L1.1-A assessment, each
pre-implementation section now carrying a "Dated assessment" note pointing at what superseded it
(Leagues page, pending list, copied-target UI, `tdb-loc`, the PS4 stager) and at the runtime
cards. Identical fixture hashes (PS3/X360 pairs, NHL 19/20) are labelled as one *measurement*,
never as provenance; NHL 21 is the only title with a verified install→fixture chain.

### T2 — NHL 21 inputs and runtime blockers (read-only)

| Input | Status | Evidence |
|---|---|---|
| Installed game identity | **verified**: `H:\Emulators\ShadPS4\Games\CUSA18182`, `param.sfo` TITLE_ID `CUSA18182`, CONTENT_ID `EP0006-CUSA18182_00-NHLICEHOCKEY2021`, APP_VER `01.00`, VERSION `01.01`; no update/patch folder | `param.sfo` parsed read-only |
| `databigs/inxp_00.big` | **verified**: 777 373 328 B, SHA-256 `1c41f578a595dbd5aae7ad921445626636f3a842d8fb869bc054d35e3e849e87` — equals the copy used by SMOKE-1/SMOKE-R1/RESP-1 | `eb_member.out` |
| DB member | **verified**: entry 2652 `nhl/data/common/gamemodes/database/t3db/nhl_game.db`, raw, 11 209 672 B @ 726 230 496, SHA-256 `90ee1ec4…` **= anchor**; `+0x10` = djb2(path) `0x00429ca7`; `+0x0C` `0xe6955448` (unresolved) | `eb_member.out` |
| Metadata members | verified: 1479 `nhl_game-meta.xml` `0b262e4a…` = anchor; 2888 `loc/eng_us-meta.xml` `54febbf9…` = anchor | `eb_member.out` |
| **Title's own dictionary** | **available, was not captured**: entry 1156 `nhl/data/kettle/loc/eng_us.db`, raw, 3 921 060 B @ 746 617 088, SHA-256 `24693e2f1ae87bde23066d083458bae2efea9a633269518f4fede2354fbac969`. Byte-identical to the pre-existing `databigs/Extracted/nhl/data/kettle/loc/eng_us.db` and to the fingerprint recorded in the user's localization handoff (`crc-test-vectors.json`, `eng_us.db`). Extracted into package scratch, verified, then removed (§6) | `eb_member.out`, `loc_keys.out` |
| Dictionary verification with the **existing repo reader** (`tdb-loc`, via a throw-away scratch crate) | 4/4 CRC scopes match (`directory e13102a9`, `table-header 56d0cac3`, `body 3d45308b`, `file-header 6b3275c1`); 55 766 rows = allocated; stride 16; LE; 0 invalid UTF-8; 0 hash mismatches; 161 symbols. **`Allsvenskan` → "HockeyAllsvenskan"; `TXT_ROW_LEAGUE` → "Rest Of World"** (the chosen old/new keys). All 31 `stockleagues.name` keys looked up: 25 present, 6 absent (`AHL`, `CUSTOM_TEAMS`, `HUT`, `TXT_LEAGUE_MASCOT`, `TXT_LEAGUE_3ON3`, `TXT_LEAGUE_PROAM`) | `loc_keys.out` |
| Own vs proxy delta | NHL 21 differs from the NHL 20 and NHL 18 anchors on `TXT_LEAGUE_EBEL` ("ICE Hockey League" vs "Erste Bank Eishockey Liga") and has `TXT_LEAGUE_NHL_ALL_TIME`, `TXT_USA_LEAGUE`, `TXT_RUSSIA_LEAGUE` where the proxies have none — the matrix §1.4 proxy table was wrong for NHL 21 on four league keys | `loc_keys_siblings.out` |
| Baseline labels for the other PS4 equivalents (each title's **own** anchor dictionary) | NHL 16/17 `TXT_LEAGUE_NORWAY` → "Norway"; NHL 15 `TXT_LEGEND_TEAMS` → "Legend Teams"; `TXT_ROW_LEAGUE` → "Rest Of World" on 15/16/17/18/20 | `loc_keys_siblings.out` |
| Member occurrence | `nhl_game.db` and `loc/eng_us.db` occur exactly once across all 16 `databigs/*.big` (front tables of every archive scanned, no payload reads) | `eb_scan.out` |
| Loose game-tree files | the install ships loose `nhl/data/common/config/*.cfg` and `nhl/data/kettle/config/*.ini`; **no loose `nhl_game.db` in the game tree** — the only loose copy is under `databigs/Extracted/` (Studio's extraction, not a game path). Studio's PS4 discovery falls back to a bounded recursive search and would resolve that Extracted copy as the install dir; the test card therefore builds a copied root in the game-tree layout instead | `discovery.rs:344–360` read |
| Consumer lead | `boot.big` entry 3730 `…/service-defs/view/roster/leagues_and_teams.json` (chunkzip, 1 876 B decoded, SHA-256 `43e0a81e…`): UI roster service schema with `LeagueList[]{LeagueId, LeagueName "localized string for league name", LeagueArtId, OrderId, TeamGroupList[]{GroupName ≤20 …}}` — schema-level corroboration of the key→dictionary model and the first named origin candidate for the group labels. Not a trace; recorded for 1.2/1.3 | `leagues_and_teams_json.out` |
| Emulator environment | `H:\Emulators\ShadPS4\shadPS4.exe` (69 042 176 B, 2026-09-09); last log: `shadps4 v0.16.1 DMC`, rev `d96133b0…`, branch `dev-next`, `checkpoint/p4-entry-0-gd96133b0a-dirty`, remote `puckhead73/shadPS4` — a **locally modified fork**, with `.pre-tp2f` / `.tp2f-final.bak` backups beside it. Config `enableMods = true`, empty `mods_folder`. **No NHL 21 boot is recorded anywhere** (last run: NHL 24, 2026-09-15); no `savedata` directory exists for any title → first boot will create a profile | read-only listing/log |
| Runtime blockers (unchanged) | staged PS4 archive is structurally unqualified (`+0x0C`, header `0x10`, loader acceptance); loose-vs-archive precedence unknown; literal-fallback unknown. These are exactly what the E1/E2/E4 experiments separate | — |

**Nothing missing for NHL 21** except execution authorization: every file input the card needs
exists and is identified by hash. Still **missing-fixture in the repository**: `anchors/ps4/nhl21/databases/eng_us.db`
(and NHL 19's) — committing them is a Codex decision (§5).

### T3 — `docs/plans/league-identity-runtime-qualification.md`

Common rules (G0 stock viability vs G1 mod acceptance; hashes; cold start; restore boot;
evidence layout; "never" list), the NHL 21 card (identity table, the edit and its baseline/expected
labels from the title's own dictionary, predicted surfaces, Studio artifact steps, experiments
E0–E5 with loose/archive/both/literal controls, result table, verdicts and what the card cannot
establish), the decision map (T4), and target-specific equivalents: the six other PS4 titles with
their installed CUSA ids/APP_VERs, equivalent rows, own-dictionary baseline labels and the inputs
still needed (provenance checks for NHL 19/20, own dictionaries, member measurements), and the
twenty gen6/gen7/Recompiled targets as G0-only cards (vocabulary corroboration) with the identity
experiment correctly assigned to Phase 1.3.

### T4 — decision map

Eight outcome rows (G0 blocked; pass archive; pass loose; fail packaging/loader; fail precedence;
fail lookup; inconclusive/field not consumed; missing input) each with attribution, the smallest
next action and the phase it belongs to, plus a table of questions that belong to 1.2 / 1.3 / 1.4
/ packaging research so the run need not answer them.

## 4. Checks actually executed (developer, this host)

| Check | Command / method | Result |
|---|---|---|
| Archive member probe | Python (`eb_member.py`, SHA-256 `b8c82319…`, removed) parsing the `EB\0\3` front tables per `docs/formats/ps4-eb-big.md`; payload reads of the four members only; wrote one file into package scratch | see T2 (`eb_member.out`) |
| Archive occurrence scan | Python (`eb_scan.py`, `f1b636f6…`, removed), front tables of 16 archives | `eb_scan.out` |
| Dictionary verification | throw-away crate `loc-probe` (`main.rs` `292e6a77…`, removed) depending on `crates/tdb-loc` by path; `cargo run --release --offline` with `CARGO_TARGET_DIR=local/runs/league-l11-q1/target` (removed) on the extracted NHL 21 member and the NHL 15/16/17/18/20 anchors | `loc_keys.out`, `loc_keys_siblings.out` |
| Service schema decode | one-off Python inflate of `boot.big` entry 3730 (padded chunkzip, raw deflate) | `leagues_and_teams_json.out` |
| `param.sfo` / log / config reads | Python `struct` parse; `ls`/`cat` | T2 |
| Matrix splice | `splice_matrix.py` (`061f17b6…`, removed), LF preserved, guarded against double insertion | headings verified after run |
| Full build / test suites | **not run** (documentation package; the dispatch forbids it) | — |
| Desktop / game / live install | **not executed, not authorized** | — |

Independent (Codex) results referenced in the matrix are cited to their review records; none is
presented as fresh. The reference reader in the localization handoff (`tool.mjs`/`reader.mjs`)
was **read but not executed** in this package — running external code was refused by the session's
permission classifier; the repo's `tdb-loc` (which was ported from it and is the reader Studio
ships) was used instead, and its CRC/row results agree with the handoff's recorded vectors for
`eng_us.db` (body CRC end 3 921 056, 55 766 rows).

## 5. Unresolved questions for Codex

1. **Commit the NHL 21 (and NHL 19) `eng_us.db` as anchors?** They are EA data that ships with the
   tool in the sense of `anchors/`; NHL 21's is identified by archive member and hash (matrix
   §0.3). Doing so lets `tdb-loc` tests and the Leagues page cover NHL 21 without the install.
   Not done here (anchors policy is Codex's).
2. **Reference environment.** The only PS4 emulator on this host is a locally modified shadPS4
   fork. A pass on it is a pass on *that build*; whether that is the accepted evidence environment
   for `nhl21-ps4` is a user/Codex decision to record before E0.
3. **`mods_folder` semantics** of that build are unknown; the card excludes it until documented.

## 6. Temporary artifacts, processes, cleanup

Package root: `E:\Repositories\nhl-modding-studio\local\runs\league-l11-q1\` (untracked).

| Item | Purpose | Disposition |
|---|---|---|
| `eng_us.db` (3 921 060 B, `24693e2f…`) | the extracted NHL 21 dictionary member | **removed** after verification (byte-identical copies remain in the installed game's `databigs/Extracted/…/loc/` and inside `inxp_00.big` entry 1156; the card names both) |
| `eb_member.py`, `eb_scan.py`, `splice_matrix.py`, `matrix_insert.md` | read-only probes / one-shot doc splice | **removed** (hashes above; effects are the outputs and the matrix diff) |
| `loc-probe/` (scratch crate) and `target/` (its build cache) | run `tdb-loc` without touching repo code | **removed** |
| `eb_member.out`, `eb_scan.out`, `loc_keys.out`, `loc_keys_siblings.out`, `leagues_and_teams_json.out`, `hashes.txt` (≈11 KB) | concise outputs/hashes cited in this receipt | **retained** under the package root for Codex's review of this receipt; disposable after review (all values are transcribed into the matrix and this receipt) |
| Processes | `python`, `cargo`/`loc-probe` ran to completion; no app, emulator or game process launched | none running |

Removals used exact literal paths with root-containment and reparse-point checks (`Remove-Item
-LiteralPath`), each verified absent afterwards. Installed game read-only (`inxp_00.big` and the
Extracted dictionary mtimes unchanged, 2026-06-25); anchors untouched; no shared `local`/`target`
tree existed or was touched; no `git clean`/`reset`; index empty.

## 7. Exact next review action

Codex: review the matrix §0 rows against the cited receipts and the runtime card's E0–E5 design.
The smallest concrete next action this handoff enables is **E0 (G0 stock boot) of the NHL 21 card**
— a separately authorized run that boots the unmodified CUSA18182 on the recorded emulator build,
reaches two league surfaces, screenshots "HockeyAllsvenskan" and records the environment — followed,
only if G0 passes, by E1 with the staged archive whose outputs the card already identifies. Decide
§5.1 (anchor commit) and §5.2 (reference environment) before or alongside that dispatch. Stopping
here; no game run, no live install, no further package started.
