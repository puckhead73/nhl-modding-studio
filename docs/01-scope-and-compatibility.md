# 01 — Scope & Compatibility

## Titles in scope

NHL Modding Studio targets **NHL 10 through NHL 21** (the full EA NHL run across that span).

## Platforms in scope

- **Xbox** family (360-era builds and later as applicable)
- **PlayStation 3 (PS3)**
- **PlayStation 4 (PS4)**
- **NHL Legacy Recompilation** — a recompilation based on the **Xbox** build of the game

Different platforms can carry meaningful differences in:

- **Endianness** — PS3/Xbox 360 are big-endian (PowerPC); PS4 is little-endian (x86-64).
- **Save-data containers** — each platform wraps save data differently (see
  [05-save-data.md](05-save-data.md)).
- **Database packaging** — how the TDB databases are stored/packed in the title's files.

> The **Legacy Recompilation** is derived from the Xbox build, so it is expected to share that
> build's data layout most closely. Confirm specifics against the reference implementation.

## Compatibility matrix (to be confirmed)

This matrix tracks confidence per title × platform as support is implemented and verified.
Status legend: ✅ supported & verified · 🟡 partial / in progress · ⬜ planned · ❓ unverified.

| Title  | Xbox | PS3 | PS4 | Legacy Recomp |
|--------|:----:|:---:|:---:|:-------------:|
| NHL 10 |  ❓  | ❓  |  —  |      ❓       |
| NHL 11 |  ❓  | ❓  |  —  |      ❓       |
| NHL 12 |  ❓  | ❓  |  —  |      ❓       |
| NHL 13 |  ❓  | ❓  |  —  |      ❓       |
| NHL 14 |  ❓  | ❓  |  —  |      ❓       |
| NHL 15 |  ❓  | ❓  | ❓  |      ❓       |
| NHL 16 |  ❓  | —  | ❓  |      ❓       |
| NHL 17 |  ❓  | —  | ❓  |      ❓       |
| NHL 18 |  ❓  | —  | ❓  |      ❓       |
| NHL 19 |  ❓  | —  | ❓  |      ❓       |
| NHL 20 |  ❓  | —  | ❓  |      ❓       |
| NHL 21 |  ❓  | —  | ❓  |      ❓       |

> The hardware-generation cells above (which platforms shipped which titles) are an initial
> assumption and **must be verified** against real release history before they are treated as
> fact. `—` marks combinations believed not to exist; correct as confirmed.

## Out of scope (initially)

- Titles before NHL 10 or after NHL 21.
- PC platform (unless a title's PC build is explicitly added later).
- In-engine asset/3D modding beyond what the data layer covers (the `rexglue-sdk` graphics
  reference may inform later work, but it is not part of the initial scope).
