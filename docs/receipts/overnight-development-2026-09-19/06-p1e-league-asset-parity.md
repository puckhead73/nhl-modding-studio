# Receipt 06 — D6 / P1-E league asset parity (+ P1-G imagery inventory, P1-H matrix)

> **Correction note (OVERNIGHT-DEV-R1 F3, 19 Sep).** Left as written below; the canonical statement is the
> [coverage matrix](../../validation/phase-1-coverage-matrix-2026-09-18.md) (R1-reconciled) and the
> [R1 record](../../validation/overnight-dev-r1-corrections-2026-09-19.md). Narrowings: "every league-logo
> package … previews and takes a same-size replacement on every title and platform" is a **codec** result —
> a same-pixel decode → re-encode round trip on the index-0 package of each family in a fixture set taken from
> one extracted build per title; different-pixel replacement is proven on synthetic RW4ps3/DXT5 cases only;
> the `import_texture` API route and the Replace dialog were not exercised on a logo container. The non-logo
> imagery is **listed** (tested); its preview/replace is impl, test missing (no real non-logo file decoded).
> "Tiles/banners n/a by registry evidence" is research unknown (only the named registries were searched).
> §3 P1-H: the 1.5 "n/a (evidence)" for a league-named bank was a member-name scan, not n/a — the matrix now
> says researched absence / research unknown; audio event identifiers do not establish sample replacement.

Developer lane, overnight 2026-09-19. **Developer-verified; not Codex-accepted.**

## Plain-language summary

Every league-logo package Studio knows about now previews and takes a same-size replacement on every
title and platform: the PS3 NHL 08–11 texture container that had no reader (`RW4ps3`) is decoded and
written, the Xbox 360 NHL 07–11 packages (RW4 texlibs inside the archive package) can be written, the PS3
NHL 12–Legacy logos (uncompressed DDS, previously invisible) read and write, PS4 NHL 15's PNG logos are a
recognised container, and the NHL 07–10 packages that the Xbox archive stores RefPack-wrapped are
unwrapped and re-wrapped. Two existing decoder faults surfaced and were fixed on the way: the small
Xbox league logo (a 128×64 DXT5) decoded garbled on every title, and an Xbox tiled ARGB logo was written
back as if linear (corrupting NHL 07's 100×104 logo). All 77 shipped index-0 packages (four families ×
19 sources) and all 34 PS4 members round-trip on temp copies, and the PS3 decode equals the Xbox decode
pixel-for-pixel on all 35 same-size pairs. Beyond the logos, the league-keyed presentation imagery from
research 03 (playoff-tree conference logos, trophies, broadcast trophies, MUD logos; the PS4 registry's
large logos, Memorial Cup, Champions tile, scouting-region aliases) is listed per title on a new
"presentation imagery" page under Leagues with the same preview/replace. League audio (research 04)
is recorded as blocked on the undecoded bank format; no audio code was written. Not built: replacing at
different dimensions (the Apt geometry record) and placing a changed-length logo package back inside a
packed archive / disc image (P1-F's gen7 side; PS4 GNF member route).

## 1. Package, disposition, scope

- **D6 / P1-E** — delivered, developer-verified, pending independent review. Checkpoints: (1) codec
  coverage (RW4ps3 read/write, RW4xb2 in-package write, uncompressed DDS, PNG, whole-file RefPack);
  (2) real-fixture round trips per chain; (3) PS4 variant paths per title; (4) **P1-G** inventory +
  listing + preview/replace parameterised by family; (5) **P1-H** matrix wording from research 04.
- Not started (recorded): different-dimension replace (Apt record); gen7 archive placement of a
  changed-length package (BIGF/BIG4 re-plan, `.bh`); the PS4 GNF logo member route through the
  copied-target stager; shared-span (dedup) members.

## 2. Changed paths and hashes (baseline `8d60a6d`; SHA-256 prefixes)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/tdb-rx2/src/texlib_ps3.rs` | `f37c642aa751594a…` | **new** — `\x89RW4ps3` parser (header 0x20/0x30/0x44, slot/descriptor pairing in table order, format by descriptor signature `6007/8035` → ARGB8, `83F3/FFFF` → DXT5, slot-length gate with the 16-byte DXT5 floor), `base_mip_linear`, `replace_slot` (exact length; header/table/descriptor untouched); 3 unit cases |
| `crates/tdb-rx2/src/texlib.rs` | `340ad04375fdbfc9…` | **fixes**: DXT base level read from the full 32×32-block macro-tile span (a 128×64 DXT5 read only its first half → garbled); `Texture.tiled` recorded and honoured by `replace_same_size` / `replace_base_keep_tail` for `Argb8888` (tiled FE art was written linear) |
| `crates/tdb-rx2/src/lib.rs` | `36cc5aa9cb3ba110…` | module |
| `crates/studio-domain/src/assets.rs` | `cb8418126bdcf012…` | `rasters_in`: RW4ps3 + uncompressed A8R8G8B8 DDS; `decode_texture_file`: RW4ps3 and PNG branches; `replace_portrait_texture`: `Rw4Xb2`/`Rw4Ps3`/`DdsArgb` slots (member wrapping preserved, shipped DDS header reused); `replace_texlib_texture`: RW4ps3; `replace_texture_in_file`: PNG + whole-file RefPack unwrap/re-wrap; `TextureContainer::Png`; dispatch test updated |
| `crates/studio-domain/src/league_logos.rs` | `cfe9b7d9701166f0…` | PS4 variants per title (NHL 15 PNG ×2 sizes, NHL 16 GNF ×2, 17–21 ×3 + `_fr`) |
| `crates/studio-domain/src/league_imagery.rs` | `51c31be31dc61a4d…` | **new** (P1-G) — defs model, PS4 template expansion (`#`, `LANG_TAG_FR`/`[loc FRE_FR]` → `""`/`_fr`), per-title listing with existence; Recompiled by identity with Xbox |
| `crates/studio-domain/src/{project,session,lib}.rs` | `a51aa53442aee470…` / `b3fd4c4a8a7571f4…` / `03d3d5b6377fcc61…` | `defs/league_imagery.toml` loaded with the other defs; `EditSession::league_imagery` |
| `defs/league_imagery.toml` | `448c475f3d836aa8…` | **new, generated** — 19 gen7 sources (families, ids, readings, consumers, decoded texture facts) + 7 PS4 registries (types, templates, ids, French ids) |
| `crates/studio-domain/tests/league_logo_packages.rs` | `5b02d80ec8eb914b…` | **new** — 2 synthetic cases (RW4ps3 ARGB8 package raw/RefPack'd member + whole-package RefPack; DXT5 slot with mip chain) + 2 `--ignored` sweeps (77 gen7 packages incl. PS3 = Xbox equality; 34 PS4 members) |
| `crates/studio-domain/tests/league_imagery.rs` | `dbce9289696a2ea9…` | **new** — 3 cases (every source/title resolves; Legacy/Recomp/NHL 10/PS3 08 facts; PS4 twins + existence) |
| `crates/studio-domain/tests/league_logos.rs` | `7b61c335a88f5db2…` | PS4 NHL 15/16 variant sets |
| `crates/studio-api/src/{dto,registry,commands,lib,bindings}.rs` | `994f6d8a…` / `d2f7a7bc…` / `370144be…` / `6a1cc759…` / `4b6b311f…` | `LeagueImageryDto` (+family, member), `league_imagery` command; container doc strings; export roots 113 → 114 |
| `app/src/bindings/*` | generated | `--write`; `--check` clean |
| `app/src/lib/client.ts` | `006f33df37b4cbb9…` | command typing |
| `app/src/components/LeagueImageryCard.tsx` (+test) | `30a746b95febc8c8…` / `1dea5c78582c0629…` | **new** — families, readings, consumers, member rows, Replace / Replace French through the shared surface; 3 jsdom cases |
| `app/src/pages/LeagueImagery.tsx`, `app/src/pages/Leagues.tsx`, `app/src/routes.tsx` | `47daf61481979944…` / `d645f381ad6f1967…` / `d88255efea81587b…` | route `database/leagues/imagery` + toolbar link |
| `app/src/components/TextureReplace.tsx` | `e847f7c49a1c3f47…` | `png` label; PNG resizable in the dialog gate |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md` | `9fb8c6a394d87d0a…` | 1.4 preview/replace/non-logo rows, 1.5 rows, P1-E/G/H package rows |
| `local/runs/overnight-development-2026-09-19/{extract_logo_fixtures.py, gen_league_imagery_defs.py}` | `c4e54724f4bc102f…` / `d6d2a37ca189712d…` | fixture extraction (read-only on sources); defs generator |

Private fixtures added (read-only inputs, 6.2 MB): `E:\nhl-studio-fixtures\logos\<platform>-<title>\<family>\l0.big`
(77 gen7 packages, stored bytes as the archive/disc holds them) and `logos\ps4-<title>\<size>\l0[_fr].{gnf,png}`
(34 decoded members) with `logos\manifest.json` (`2898a5ff02fd34e3…`: source, entry, offset, stored size,
SHA-256, container/outer form).

## 3. Behavior and decisions

- **RW4ps3 (research 18 §3 exactly).** Slots and descriptors are paired in file-table order (research's
  "ordinal" pairing agrees on every measured file — the slot-length gate would reject a mismatch). Only
  the two observed signatures decode; anything else is refused naming the words. The writer re-encodes
  the whole slot at its exact length: ARGB8 plain (`[A,R,G,B]`, mips box-filtered if declared), DXT5 base +
  chain with the 16-byte floor; header, table and descriptor bytes are never touched; a no-op ARGB8
  round trip is byte-identical at the RW4 level.
- **Packages.** `replace_portrait_texture` (the BIGF package writer) gained three slot kinds beside
  XPR2/DDS-DXT: RW4xb2 (Xbox 07–11), RW4ps3 (PS3 08–11), uncompressed DDS (PS3 12–Legacy `leaguelogos` /
  `big` / `wide`). A member keeps its own wrapping: raw stays raw (NHL 08 PS3, NHL 07–10 Xbox inside the
  RefPack'd package), RefPack'd members are re-RefPack'd. The uncompressed DDS keeps EA's 128-byte header
  verbatim and writes texels in its declared channel order (masks read, not assumed). RW4/DDS-ARGB
  members are **same-size only** (the Xenos descriptor / Apt geometry are not regenerated); the
  replace dialog's resize hint still applies to XPR2/DDS-DXT portraits and PNG, and the server reports the
  refusal for the others.
- **Whole-file RefPack** (`replace_texture_in_file`): unwrapped, rewritten, re-wrapped. Studio's RefPack
  stream differs in length from EA's, so the rewritten *extracted* file is complete but placing it back
  inside `fe.big`/`data0.big` (BIG4) is the archive route's (P1-F) open item.
- **Decoder fixes in `tdb-rx2::texlib`** (affect every consumer of Xbox texlibs, not only logos): the
  DXT base mip of a texture under 32 blocks in a dimension is read from the full macro-tile-aligned span
  (previously half the rows of a 128×64 DXT5 were missing); the parser now records the fetch-constant
  `tiled` bit on each texture and the same-size writers tile `Argb8888` when set (previously an NHL 07
  100×104 tiled logo was written linear → garbage). Both are covered by the `--ignored` sweep (Xbox small
  logos now equal the PS3 decode; NHL 07 round trip exact) and the existing `tdb-rx2` suite is unchanged.
- **PS4 variants.** `league_logo` lists what the registry actually ships: NHL 15 `nhl/ui/assets/…/l{idx}.png`
  (64/256), NHL 16 GNF 64/256 without `_fr`, NHL 17–21 64/256/512 with `_fr` twins.
- **P1-G.** `defs/league_imagery.toml` is generated from research 03's data file (source SHA recorded in
  the file); the listing is per title (the gen7 id ↔ league mapping is game code); readings are labelled
  inferred; consumers are the observed screens (NHL 11–Legacy `generalhelpers`; 07–10 "not traced" note;
  PS3 by reference). The PS4 entries expand the registry templates (French tag → `_fr`) and expose the
  alias facts (region logos = league logos; `l100` = `l5`; `league_logo` = `collection_league_logo`).
  Preview/replace is the same texture surface, so every codec claim above applies by construction; no
  non-logo package was round-tripped on a real file (fixtures not extracted for them — recorded).
- **P1-H.** Matrix only: 1.5 cells become n/a (evidence) for a league-named bank and **blocked (format)**
  for cue replacement, with research 04's vocabulary recorded.

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `cargo test -p tdb-rx2` | 112 passed, 26 ignored (unchanged set + 3 new `texlib_ps3` cases) |
| `cargo test -p studio-domain --test league_logo_packages` | 2 passed, 2 ignored |
| `NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test league_logo_packages -- --include-ignored` | **4 passed** — 77 gen7 packages (chains: x360 refpack 14, x360 BIGF 24, ps3 BIGF 35, ps3-cache 4), 35 PS3-vs-Xbox same-dims pairs at MAD 0.0, 34 PS4 members (2 PNG exact, 32 GNF ≤ 6.0 MAD) |
| `cargo test -p studio-domain --test league_imagery` / `--test league_logos` | 3 passed / 2 passed (1 ignored, unchanged) |
| `cargo test -p studio-domain --lib` | see the final tally below (run in the background at receipt time) |
| `cargo test -p studio-api --lib bindings` / `export-bindings --write` then `--check` | 25 passed (roots 114) / in sync |
| `cd app && npx tsc --noEmit -p .` / `npx vitest run` | clean / **15 files, 111 passed** |

Not run: full workspace suite; `zz_diag`; clippy/fmt; desktop; any game.

## 5. Remaining blockers / limitations

- Different-dimension replace (gen7 Apt record; PS4 registry expects the file's own size) — not impl.
- Archive placement of a changed-length gen7 package (BIG4 `fe.big`/`data0.big`, `.bh`, loose BIGF re-plan)
  and the PS4 GNF member route through the copied-target stager — not impl (P1-F gen7 side).
- NHL 07 `leaguelogossmall`/`leaguelogoswide` and `divisionlogos/d0` use an RW4 table variant (tag
  `00000000`, `hdr_size` = file length) that the parser accepts for the logo packages; it was not measured
  beyond those files.
- Non-logo families (P1-G) not round-tripped on real files; the gen7 id ↔ league mapping and the PS4
  unreferenced icon types are research unknowns → cards R3-IMG-1/2.
- Runtime precedence (loose vs archive vs disc image) stays research unknown → user cards.

## 6. Temporary artifacts / processes

- Scratch example `crates/studio-domain/examples/logo_probe.rs` — created for debugging, **removed**.
- Session scratchpad `d6_probe.py`, `n07/`, `s08x/`, `s08p/` PNG dumps — disposable (scratchpad).
- Test temp dirs `nhlms-logos-*`, `nhlms-imagery-*` — removed by the tests; no processes left.
- Fixtures above — retained (regenerable by `extract_logo_fixtures.py`).

## 7. Runnable commands

```sh
cargo test -p tdb-rx2 --lib texlib_ps3
cargo test -p studio-domain --test league_logo_packages --test league_imagery --test league_logos
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test league_logo_packages -- --include-ignored --nocapture
cargo run -p studio-api --bin export-bindings -- --check
cd app && npx vitest run src/components/LeagueImageryCard.test.tsx
```

## 8. Next

D7 — Phase 1 offline parity accounting (per-target table across 1.1–1.5), MORNING-REVIEW cards, final receipt.
