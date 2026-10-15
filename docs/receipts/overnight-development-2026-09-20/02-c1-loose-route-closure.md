# Receipt 02 — C1: loose-file texture route closed on every family (Priority 1)

Developer lane, overnight 2026-09-20. **Developer-verified; not Codex-accepted.** Baseline `8d60a6d`.

## Plain-language summary

Replacing a league logo, a presentation image (playoff-tree conference logos, trophies, broadcast
trophies, MUD logos) or a portrait in an extracted game tree now patches the package in place the way
EA laid it out — only the texture, its size word and the entries after it change; the rest of the file,
including two opaque values EA writes, is kept byte for byte. Before this, Studio rebuilt the whole
package in its own layout. The "Replace…" dialog was also given the same ownership discipline as the
archive dialog: a picker or a write that finishes after you switched to another install can no longer
land in the wrong tree, and the Write button cannot be double-sent. All of this was exercised with
**visibly edited** images (a magenta band and an inverted quadrant, not the same pixels) on every real
league-logo package of every title and platform, on every PS4 member, and — for the first time — on the
real non-logo imagery files of every title. Whether a game reads a loose file at all remains the user's
runtime card (loose vs archive precedence is research unknown).

## 1. Package, disposition, scope

C1 — delivered, developer-verified, pending independent review. Closes the matrix cells "replace a
loose package — API route + UI flow: impl, test missing" and "non-logo imagery preview/replace: impl,
test missing" on every family. Not in scope: archive placement (C2/C3), different dimensions (later).

## 2. Changed paths (SHA-256 prefixes)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/tdb-archive-big/src/replan.rs` | `15b7b50fb8346189` | **new** — `PackageLayout::parse`, `replan_member`: span-preserving BIGF member replacement (research 02 §4.2 generalised: followers at `align64(previous end) + residual`, residual ∈ {0, 64} read from the source; gap after the member zero-verified before a length change; table trailer, inter-member gaps and the tail carried verbatim; `size_le` = last member end; output re-parsed and every other member checked equal); 5 unit cases |
| `crates/tdb-archive-big/src/lib.rs` | `cc28f44e72372938` | `pub mod replan`; `Archive::bytes()` |
| `crates/studio-domain/src/assets.rs` | `22b738dc652e65f1` | `replace_portrait_texture` split into `rewrite_package_texture` + `place_package_member`; **the re-plan is the package writer**; the generic builder remains only for a package whose *shape* the re-plan does not recognise and is then reported in `TextureWrite.notes` (never silent); `replace_portrait_texture_noted` |
| `crates/studio-api/src/dto.rs` / `registry.rs` | `791bdf6bbde1a77e` / `5805ccbf05d371a3` | `ImportTextureSummaryDto.notes` |
| `app/src/bindings/ImportTextureSummaryDto.ts` | generated (`--write`; `--check` in sync, 116 roots) | |
| `app/src/components/TextureReplace.tsx` | `a24d3c9889021601` | `useTextureReplace` ownership: pick bound to install root + generation, the write targets the pick's root, late completions dropped, one write at a time, Cancel/backdrop inert while writing, notes shown on the result card; `data-testid`s |
| `app/src/components/TextureReplace.test.tsx` | `f7f7b28dcba0feec` | **new** — 6 jsdom cases |
| `crates/studio-domain/tests/league_logo_packages.rs` | `43dc52098339a856` | +1 `--ignored`: re-plan identity + ±1/±63/±64/±1024 follower properties on all 77 real packages |
| `crates/studio-api/tests/loose_texture_route.rs` | `0e7381f9e9b7eca7` | **new** — 1 default synthetic case + 4 `--ignored` sweeps (77 gen7 logo packages, 34 PS4 logo members, 64 gen7 imagery packages, 22 PS4 imagery members) through `texture_preview` → `import_texture` → re-preview, twice each |
| `local/runs/overnight-development-2026-09-20/extract_imagery_fixtures.py` | `a45fcc644a4e5ee4` | **new** — developer-owned extractor (read-only on the sources; imports the 2026-09-18 research readers) |
| `E:\nhl-studio-fixtures\imagery\**` + `manifest.json` (`02b8494380f6e47c`) | — | **new private fixtures** (8.7 MB): 64 gen7 packages (`divisionlogos/d0`, `trophies/t0`, `trophiesbroadcast/t0`, `mudleaguelogos/l0` on the 20 gen7 sources) + 22 PS4 members (`large_league_logo` 0, `memorial_cup_logo`, `champions_logo`, `gmc_league_icon` 0, `trophy_broadcast` 0; `playoffs_tree_trophy_logo` not resolved by the research ids — not extracted) |

## 3. Behavior and decisions

- **Span-preserving re-plan is the writer for every BIGF package** (league logos, imagery, portraits —
  the Legacy `p1.big` portrait has the same 6-member / stride-64 shape). Identity on all 77 logo packages
  is byte-exact; the 5 shapes measured: 4 members (X-A) with and without tail, 6 members stride 0 / 64,
  tail 0 / 16. The builder fallback is reachable only through a shape refusal and is disclosed in the
  result card (amber note); a *content* refusal (non-zero gap after the member) is an error.
- **Research 02 §2 correction (developer evidence, not edited into the research doc):** the 16-byte tail
  after the last member on NHL 10+ packages is **not zero**. On the 60 tailed fixture packages it is a
  per-family constant, identical across titles and platforms and independent of the texture content
  (`2d26295c12155c7ec219ba066f199a0a` on every `leaguelogos` package from NHL 10 Xbox to Legacy PS3;
  `a0b81f8d…` big, `1f748f17…` wide, `3c91b938…` small on 11+, `b539b18f…` small on NHL 10; portraits and
  imagery packages carry their own). It is not an MD5 of any member or of the body. Carried verbatim as
  an opaque value; whether a **different-dimension** replacement must change it is an open question for
  that later package (it does not depend on the pixels, and the NHL 10 → 11 small change coincides with
  a layout change).
- **Loose route ownership** mirrors R1 F1 exactly (root + generation, one dispatch, truthful close).
- The loose route's write itself is unchanged: a plain `std::fs::write` after a one-time `.bak` — not a
  transaction. Recorded as a known limitation (below); the archive route is the transactional one.

## 4. Checks executed

| Command | Result |
|---|---|
| `cargo test -p tdb-archive-big --lib replan` | 5 passed |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-domain --test league_logo_packages every_gen7_package_replans -- --ignored` | 1 passed (77 packages; shapes `{4/None/0: 10, 4/None/16: 4, 6/0/0: 7, 6/0/16: 39, 6/64/16: 17}`) |
| `cargo test -p studio-api --test loose_texture_route` | 1 passed (synthetic), 4 ignored |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-api --test loose_texture_route -- --ignored` | **4 passed**: 77 + 34 + 64 + 22 files, each written twice with different marked images; `.bak` = original and kept; written pixels = edited (MAD 0 on ARGB8/8_8_8_8/PNG, ≤ 14 on DXT5/BC7) with the band present; package shape preserved (members, trailer, tail); **no builder fallback on any file** (`notes` empty) |
| `cargo test -p studio-domain --lib assets` / `-p studio-api --test registry` | 55 / 6 passed |
| `cargo run -p studio-api --bin export-bindings -- --write` then `--check` | 1 written; in sync |
| `cd app && npx tsc --noEmit -p .` / `npx vitest run src/components/TextureReplace.test.tsx` | clean / 6 passed |

Not run: full workspace suite; clippy/fmt; desktop; any game.

## 5. Per-target coverage after C1 (1.4 loose route)

| Operation | X360 07–11 | X360 12–Legacy | PS3 08–11 | PS3 12–Legacy | PS4 15 | PS4 16–21 | Recomp |
|---|---|---|---|---|---|---|---|
| Loose same-dimension replace, API route with changed pixels | impl+tested (X-A/X-B fixtures) | impl+tested | impl+tested (P-A/P-B) | impl+tested (P-C loose + cache copy) | impl+tested (PNG) | impl+tested (GNF, extraction tree) | impl+tested by identity (Legacy X360 files) |
| Non-logo imagery preview + changed-pixel replace (`divisionlogos`, `trophies`, `trophiesbroadcast`, `mudleaguelogos` / PS4 registry members) | impl+tested (07: `divisionlogos`+`trophies`; 08–11 as shipped) | impl+tested | impl+tested | impl+tested | impl+tested (`gmc_league_icon`, `trophy_broadcast`) | impl+tested (`large_league_logo` 17–21, `memorial_cup` 17–21, `champions` 19–21, icons) | by identity |
| UI flow (dialog → `import_texture` exact args, ownership) | impl+tested (jsdom, container-agnostic) | ″ | ″ | ″ | ″ | ″ | ″ |

Family exceptions carried: `awards`, `flags*`, `lobbyimg`, `onlineleague*`, `winterclassicmode`,
`mainmenuwallimages` are not league-keyed per research 03 and were not extracted; `playoffs_tree_trophy_logo`
(PS4) not extracted.

## 6. Limitations / open

- Loose write is not transactional (one-time `.bak`, direct overwrite) — pre-existing; the archive route
  is the recoverable one. A loose write on a PS3 12–Legacy tree does not touch the `cache.big` copy of
  the same package (precedence research unknown; C3 will disclose the pair).
- Runtime precedence loose vs archive vs image: research unknown → user cards (unchanged).
- Different dimensions: not built (the tail question above is added to that package's inputs).

## 7. Temporary artifacts / processes

- Scratchpad `c1_texture_replace.py` (edit script) — disposable. Test temp dirs `nhlms-c1-loose-*`,
  `nhlms-logos-*` removed by `Drop` / the tests. No processes left.
- Retained: the `imagery` fixture set (private root, read-only input of the `--ignored` sweeps;
  regenerable by the extractor).

## 8. Runnable commands

```sh
cargo test -p tdb-archive-big --lib replan
cargo test -p studio-api --test loose_texture_route
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test loose_texture_route -- --ignored
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-domain --test league_logo_packages -- --ignored
cd app && npx vitest run src/components/TextureReplace.test.tsx
```
