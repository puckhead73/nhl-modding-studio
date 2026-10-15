# 01 — Scope & Compatibility

> Architecture: [DESIGN.md](DESIGN.md) · Sequencing:
> [project completion roadmap](plans/project-completion-roadmap.md) · Verification:
> [full-game QA plan](plans/full-game-qa-editability.md).
>
> **Evidence reconciliation (2026-09-07, C0.3).** The matrix below now states an *evidence level*
> per cell rather than inherited confidence from the reference app. Nothing was promoted by this
> edit: **no cell is game-verified**, and none becomes so until a manual run on the exact
> title/platform/build is recorded. Per-feature evidence lives in
> [audits/qa-ledger.md](audits/qa-ledger.md) and the asset `capability.csv` tree, not here.

## Titles in scope

NHL Modding Studio targets **17 catalog entries**: NHL 07 through NHL 21, NHL Legacy, and NHL
Legacy Recompiled.

## Platforms in scope

- **Xbox 360**
- **PlayStation 3 (PS3)**
- **PlayStation 4 (PS4)**
- **NHL Legacy Recompilation** — a recompilation based on the **Xbox** build of the game

There is **no Xbox One target**. Titles and platforms combine only as the catalog actually shipped
them: **27 shipping title × platform targets** across the 17 entries, not a Cartesian product.

Platform differences that the codec layer must model:

- **Endianness** — big-endian on PowerPC (Xbox 360 / PS3); **little-endian on PS4** (x86-64).
- **Save-data containers** — each platform wraps save data differently (see
  [05-save-data.md](05-save-data.md)).
- **Database packaging** — how the TDB databases are stored/packed in the title's files.

> The **Legacy Recompilation** is Xbox-derived, so it tracks the Xbox data layout most closely.

## Ground truth and target counts

- **27 shipping targets** — what the finished product must qualify. User-confirmed runtimes cover
  all of them (Xenia: NHL 07–15 + Legacy; RPCS3: NHL 08–15 + Legacy; shadPS4: NHL 15–21; Legacy
  Recompiled natively).
- **27 main-database anchors** under `anchors/` — a **different set**: 10 PS3, 10 Xbox 360, 7 PS4.
  They include the NHL 07 PS3 research copy and there is **no independent Recompiled anchor**, so
  Recompiled needs its own runtime and save evidence despite sharing Legacy's schema.

## Proven core vs. extended scope

The prior reference app proved **NHL Legacy / 14 / 15** on **PS3 / Xbox / Recomp**. Everything
beyond that — PS4 (little-endian engine paths + a different save container) and the rest of the
NHL 07–21 range (per-title schema reversing) — was extension work. Both are now implemented and
carry automated anchor coverage; neither is qualified. Remaining sequencing lives in the
[completion roadmap](plans/project-completion-roadmap.md), whose milestone IDs (C0–C10) supersede
the original ROADMAP phase numbers.

## Compatibility matrix

Tracks the **evidence level** per title × platform for the main-database editing path. A cell flips
to ✅ only after a recorded manual run on that exact target.

| Symbol | Meaning |
|---|---|
| ✅ | **game-verified** — edited output loaded, behaved and persisted on that exact title/platform/build, recorded. **No cell holds this today.** |
| 🟡 | **implemented + automated-tested for the main-database path** — the engine handles this target and executed automated cases assert the main-DB read/edit path against its anchor. Not qualification, and **not a claim about any other operation class**. |
| 🔬 | **research-only** — an anchor exists and is studied, but the combination is not a shipping target. |
| ⬜ | implemented, **no automated coverage yet** |
| ⛔ | **blocked** — implementation or evidence blocked; blocker recorded |
| ❓ | fixture or runtime **missing**; nothing can be asserted |
| `—` | not a catalog combination |

**Coverage is operation-specific; a cell symbol is not a per-target summary.** Each anchor matrix
covers its own operation class and its own set of units, and they do **not** all cover the same
targets:

| Matrix | Operation class | Units covered |
|---|---|---|
| `read_matrix` / `surface_matrix` | main-DB presentation and surface capability | 27 main-DB anchors |
| `mutation_matrix` | scalar/string/enum/FK codec read-write inversion | 27 main-DB anchors |
| `operation_matrix` | structural roster operations | 27 main-DB anchors |
| `save_matrix` | save-container detect / extract / edit / repack / reopen | 26 units — **no PS3 NHL 07 rows** |
| — | any Legacy Recompiled operation | **none** — no Recompiled anchor exists |

Consequences to respect when reading the table:

- Main-database coverage **does not imply container coverage**. A 🟡 cell says nothing about that
  target's inner save container, outer package, or asset families — those are tracked per family in
  [audits/qa-ledger.md](audits/qa-ledger.md), not here.
- **Original-console outer packaging (Q04) is required and is separately unproven** for every target;
  Xbox 360 STFS is recorded as blocked. No cell above carries a packaging claim.
- Rust test totals from the C0.2 baseline describe suite health and are **not** substituted for a
  target's qualification. C0.2 §4.4 recorded 15 cases that report `ok` while skipping an absent
  fixture.

| Title  | Xbox | PS3 | PS4 | Legacy Recomp |
|--------|:----:|:---:|:---:|:-------------:|
| NHL 07 |  🟡  | 🔬 |  —  |      —       |
| NHL 08 |  🟡  | 🟡 |  —  |      —       |
| NHL 09 |  🟡  | 🟡 |  —  |      —       |
| NHL 10 |  🟡  | 🟡 |  —  |      —       |
| NHL 11 |  🟡  | 🟡 |  —  |      —       |
| NHL 12 |  🟡  | 🟡 |  —  |      —       |
| NHL 13 |  🟡  | 🟡 |  —  |      —       |
| NHL 14 |  🟡  | 🟡 |  —  |      —       |
| NHL 15 |  🟡  | 🟡 | 🟡 |      —       |
| NHL 16 |  —   | —  | 🟡 |      —       |
| NHL 17 |  —   | —  | 🟡 |      —       |
| NHL 18 |  —   | —  | 🟡 |      —       |
| NHL 19 |  —   | —  | 🟡 |      —       |
| NHL 20 |  —   | —  | 🟡 |      —       |
| NHL 21 |  —   | —  | 🟡 |      —       |
| NHL Legacy | 🟡 | 🟡 | — | — |
| Legacy Recompiled | — | — | — | ⬜ |

> 🟡 = implemented in this engine with automated anchor coverage **of the main-database path**. It
> carries **no** in-game claim, no container or packaging claim, no asset claim, and it is not
> inherited confidence from the reference app. Every 🟡 must still pass a manual game-acceptance run
> (milestone C9) before it can become ✅.
>
> ⬜ Legacy Recompiled has **no independent anchor**, so no automated case asserts it as its own
> target. It shares Legacy's schema, but an adjacent target does not inherit a pass: it needs its
> own runtime, save and deployment evidence.
>
> 🔬 NHL 07 PS3 is a **research copy**: its anchor is part of the 27-anchor set and is exercised by
> the main-DB matrices — but **not** by `save_matrix`, which has no rows for it. It is not one of the
> 27 shipping targets and is not a completion gate.
>
> The hardware-generation cells (which platforms shipped which titles) follow the current catalog and
> the user-confirmed runtime inventory. Correct any `—` that turns out to be a real release.

## Out of scope (initially)

- Titles before NHL 07 or after NHL 21.
- PC platform (unless a title's PC build is explicitly added later).
- The deferred research tracks (recomp, NHL2K10 RE, rig-audit, head-import, NHL24 weight-remap, FMT
  plugins, EBOOT patcher) — see [ROADMAP.md](ROADMAP.md#out-of-scope-deferred-to-a-separate-nhl-research-roadmap).
  The [completion roadmap](plans/project-completion-roadmap.md) section 1 keeps this list current;
  NHL24 is a headmesh *source*, not an editing target.
- Desktop platforms other than Windows. Linux and macOS are stretch goals and do not gate the
  initial release (decision Q05).
