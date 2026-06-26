# 01 — Scope & Compatibility

> Architecture: [DESIGN.md](DESIGN.md) · Sequencing: [ROADMAP.md](ROADMAP.md).

## Titles in scope

NHL Modding Studio targets **NHL 10 through NHL 21** (the full EA NHL run across that span).

## Platforms in scope

- **Xbox** family
- **PlayStation 3 (PS3)**
- **PlayStation 4 (PS4)**
- **NHL Legacy Recompilation** — a recompilation based on the **Xbox** build of the game

Platform differences that the codec layer must model:

- **Endianness** — big-endian on PowerPC (Xbox 360 / PS3); **little-endian on PS4** (x86-64).
- **Save-data containers** — each platform wraps save data differently (see
  [05-save-data.md](05-save-data.md)).
- **Database packaging** — how the TDB databases are stored/packed in the title's files.

> The **Legacy Recompilation** is Xbox-derived, so it tracks the Xbox data layout most closely.

## Proven core vs. net-new scope

The prior reference app proved **NHL Legacy / 14 / 15** on **PS3 / Xbox / Recomp**. The release
candidate's full matrix below extends that with **net-new** work tracked in
[ROADMAP.md](ROADMAP.md):

- **PS4** (Phase 9) — net-new little-endian engine paths + a new save container.
- **The rest of NHL 10–21** (Phase 10) — per-title schema reversing for years the reference app did
  not cover.

## Compatibility matrix

Tracks confidence per title × platform. A cell flips to ✅ only when round-trip + in-game verified.
Status legend: ✅ supported & verified · 🟡 partial / in progress · ⬜ planned (RC target) · ❓ release
existence unverified · `—` believed not to exist on that platform.

| Title  | Xbox | PS3 | PS4 | Legacy Recomp |
|--------|:----:|:---:|:---:|:-------------:|
| NHL 10 |  ⬜  | ⬜  |  —  |      ⬜       |
| NHL 11 |  ⬜  | ⬜  |  —  |      ⬜       |
| NHL 12 |  ⬜  | ⬜  |  —  |      ⬜       |
| NHL 13 |  ⬜  | ⬜  |  —  |      ⬜       |
| NHL 14 |  🟡  | 🟡  |  —  |      🟡       |
| NHL 15 |  🟡  | 🟡  | ⬜  |      🟡       |
| NHL 16 |  ⬜  | —  | ⬜  |      ⬜       |
| NHL 17 |  ⬜  | —  | ⬜  |      ⬜       |
| NHL 18 |  ⬜  | —  | ⬜  |      ⬜       |
| NHL 19 |  ⬜  | —  | ⬜  |      ⬜       |
| NHL 20 |  ⬜  | —  | ⬜  |      ⬜       |
| NHL 21 |  ⬜  | —  | ⬜  |      ⬜       |

> 🟡 cells reflect the reference app's proven Legacy/14/15 lineage and must be re-verified in the new
> engine before promotion to ✅. The hardware-generation cells (which platforms shipped which titles)
> are an initial assumption and **must be verified** against real release history; correct `—`/`⬜`
> as confirmed.

## Out of scope (initially)

- Titles before NHL 10 or after NHL 21.
- PC platform (unless a title's PC build is explicitly added later).
- The deferred research tracks (recomp, NHL2K10 RE, rig-audit, head-import, NHL24 weight-remap, FMT
  plugins, EBOOT patcher) — see [ROADMAP.md](ROADMAP.md#out-of-scope-deferred-to-a-separate-nhl-research-roadmap).
