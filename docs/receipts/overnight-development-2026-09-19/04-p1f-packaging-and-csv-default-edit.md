# Receipt 04 — D4 / P1-F (packaging) and the NHL 16–21 stored-default edit (P1-BS-4)

Developer lane, overnight 2026-09-19. **Developer-verified; not Codex-accepted.**

## Plain-language summary

Studio can now re-compress a text file exactly the way EA's PS4 archives store it (every measured stock
member comes out byte-for-byte identical), and it can install such a rewritten file into a copied game
archive — overwriting it in place when it still fits, or appending it and re-pointing the entry when it
does not — through the same reviewed, restorable transaction as any other save. On that basis the Game
Settings tab edits stored defaults on NHL 16–21 too: one cell of one CSV member (on NHL 18–21 the global
default's two stored copies together, as one operation), held as a pending edit, listed in Pending edits,
and written by the next copied-target save. Everything else in the archive stays byte-identical; a
restore returns the original bytes exactly. Whether the game reads a re-encoded member or shows the
edited default is still a user test.

## 1. Package, disposition, scope

- **D4 / P1-F** — the shared primitives the settings work needs: native-framed `chunkzip` encoder
  with decoded-content preservation and a recorded backend; the text-member artifact route on the
  copied-target review reusing the existing planner/stager; **and the D3 extension**: the NHL 16–21
  one-cell CSV stored-default edit incl. the NHL 18–21 paired global edit. Developer-verified, pending
  independent review.
- Not in this checkpoint (recorded): RefPack encoder ratio work and the BIGF inner-package re-plan
  (research 02 §4.2/§4.3 — logo chains on Xbox 360/PS3), the GNF logo member route on PS4 (the encoder
  + stager are ready; the artifact route is text-only), shared-span (dedup) member rule, the
  `0x00110400` declared-size acceptance. Disc-image writes stay same-length only; no live writes.

## 2. Changed paths and hashes (baseline `8d60a6d`; prefixes)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/tdb-archive-codec/Cargo.toml` | `faa5a6d21706363b…` | `flate2 = { features = ["zlib"] }` (libz-sys 1.1.29, **bundled zlib 1.3.2, statically linked** — the only new dependency; `Cargo.lock` gains `libz-sys` + `cc`/`pkg-config` deps) ; dev-dep `serde_json` |
| `crates/tdb-archive-codec/src/lib.rs` | `47b792cf0bc7b931…` | `compress_chunkzip_native`, `is_native_chunkzip_framing`, `CHUNKZIP_NATIVE_WINDOW`, `CHUNKZIP_NATIVE_BACKEND`; the old single-chunk `compress_chunkzip` untouched |
| `crates/tdb-archive-codec/tests/chunkzip_native.rs` | `94ea4afe607a124f…` | **new** — 4 synthetic + 2 `--ignored` reproduction cases |
| `crates/tdb-archive-big/src/stage.rs` | `227ad5976a54c03a…` | the native-framing qualification note narrowed to the tested statement (a reproduction target, not loader acceptance) |
| `crates/studio-domain/src/settings_csv_edit.rs` | `374f19059bb93a2c…` | **new** — subset span parser, `plan_csv_default_edit`, splice + `verify_splice`, per-member re-encode/decode, `ArchiveSlots` placement prediction, `apply_to_members` |
| `crates/studio-domain/src/settings_catalogue.rs` | `55e6923095de66da…` | `csv_stored_identities` |
| `crates/studio-domain/src/workspace.rs` | `092a5c8d8daebb24…` | `CsvWorkingSet`, `SettingsCsvReview`; `settings_catalogue` (overlay), `settings_csv_default_preview/review/apply_review`, `settings_csv_edits`, `undo_settings_csv_edit`, `rebind_csv_after_copied_save`; `WorkspaceRevision.text_members`; dirty/reload lifecycle |
| `crates/studio-domain/src/copied_target.rs` | `340ac8a3e9392973…` | `CopiedTargetPlan.text_members` (`TextMemberReplacements`), `text_member_replacements` (identity/form/decode checks → `Replacement`s) |
| `crates/studio-domain/src/lib.rs` | `2ecd01170c76dc94…` | module + exports |
| `crates/studio-domain/tests/settings_csv_edit.rs` | `58b3d6194a427d15…` | **new** — 6 cases + 1 `--ignored` (real NHL 16 archive) |
| `crates/studio-api/src/registry/copied_target.rs` | `33ed22a07d8df7c2…` | `stage_text_members` (bind → replacements → plan → stage → `add_archive("archive-csv")`) run by every copied-target review; the archive joins the reservation; post-commit `rebind_csv_after_copied_save` |
| `crates/studio-api/src/{registry,dto,convert,commands,lib}.rs` | `e72c34c4…` / `6cfe2c7b…` / `ee81eb33…` / `95dc1298…` / `322ca62e…` | `settings_default_preview(.., install_root)` routes CSV titles; `settings_apply_review` dispatches by ticket kind; `settings_csv_undo`; `SettingsEditPlanDto.kind/paired/members/archive`, `SettingsCsvMemberDto`, `PendingSettingsCsvDto`, `PendingReviewDto.settings_csv{,_archive}` |
| `crates/studio-api/tests/settings_csv_route.rs` | `be1172b4dd1cd516…` | **new** — 2 route cases |
| `app/src/bindings/*` | generated | `export-bindings --write`; `--check` clean |
| `app/src/components/StoredDefaultEditor.tsx` (+test) | `bb8baece…` / `ccee2570…` | install root on preview; CSV plan members/placement/pair pill; "pending" wording; +1 jsdom case |
| `app/src/pages/GameSettings.tsx` (+test) | `4f277679…` / `da55826d…` | editor on CSV titles with the pending note |
| `app/src/pages/PendingEdits.tsx` | `62b764b3f6bfe0de…` | pending catalogue-edit card with "Drop the last catalogue edit" |
| `app/src/lib/client.ts` | `23b0487c3808668e…` | command typings |
| `docs/validation/phase-1-coverage-matrix-2026-09-18.md` | `b8a38e012211da5c…` | 1.2 rules row (NHL 16–21 impl+tested), 1.4 archive-placement row (PS4 16–21 partly impl) |

Private fixtures added (read-only inputs): `E:\nhl-studio-fixtures\chunkzip\nhl21-ps4-cusa18182\overlays_21.big\{guestinthebootharrived,guestintheboothteaser}.gfx.chunkzip`
(stored bytes, identities as recorded in `tdb-archive-codec/src/fixtures.rs`: `2f310dba…`, `2447f071…`; from
`overlays_21.big` `cdcf3889…`), and `E:\nhl-studio-fixtures\archives\nhl16-ps4-cusa01909\databigs\inxp_00.big`
(299 883 104 B, the real NHL 16 archive; copied once so the `--ignored` round trip reads the fixture root,
never the install).

## 3. Behavior and decisions

- **Encoder.** `compress_chunkzip_native`: 40-byte header (`"chunkzip"`, 2, fullsize, window `0x20000`,
  `ceil(n / 0x20000)` chunks, `0x10`, 12 zero bytes), each chunk prefix at the next offset ≡ 8 (mod 16),
  raw deflate level 9 with the **store-if-not-smaller** rule (flag 4 when `len(deflate) ≥ len(chunk)`), no
  trailing bytes. Backend: `flate2` on `libz-sys` bundled **zlib 1.3.2** (research used CPython's zlib
  1.3.1; the deflate output is identical on every vector). Reproduction: **53/53** NHL 16–21 CSV members
  (stored length and SHA-256 equal to EA's), **2/2** NHL 21 stored-final-chunk members; the 12 raw members
  stay raw. Recorded as evidence of consistency, not as a loader rule; the stager's qualification item
  stays open with narrowed wording tied to these tests. `is_native_chunkzip_framing` is the structural
  check the write path applies before the independent decode. The legacy single-chunk
  `compress_chunkzip` is unchanged (still the chunklzx substitution), and it fails the native check.
- **Text-member route.** `stage_workspace` puts the workspace's pending CSV replacements on the plan
  (`text_members`: archive relative `databigs/inxp_00.big`, the members' held plain + stored bytes and
  the source entries' stored identities). Every `copied_target_review` then binds the copied root's
  archive, checks each member's stored identity equals the one the edit was planned on (a differing copy
  refuses: "the copy differs from the opened install — plan the edit on this copy"), the stored form,
  and that the held stored bytes decode to the held text, builds the `Replacement`s, runs the existing
  `stage::plan` (fit-in-slot / append-and-retarget from the actual encoded length) and
  `txn::stage_archive` (verify re-run by `archive_artifact`), and adds `archive-csv` to the change set —
  one prepared set, one apply, one restore with the sessions. Unchanged members keep their stored
  bytes (only edited members are re-encoded). After commit the workspace adopts the rewritten texts as
  the baseline and drops the pending edits; the archive directory is re-read lazily (slots move on an
  append).
- **CSV edit.** Planned on the *current working texts* (baseline + pending edits, so a second edit
  stacks and re-validates on the edited text). Eligibility per handoff 01 §3: 21 ids minus the title's
  absent ones; NHL 16 domain `min..max`, 17–21 the target row's own choice list (88's tournament rows
  use list 12), empty cells / `is_enabled 0` / `is_visible 0` / uninterpreted NHL 16 overrides /
  ambiguous lists / range rows refused; the reserved `_DEFAULT_` context is never a screen target; NHL
  18–21 global = **paired edit** of `setting.csv` + the `_DEFAULT_` row, requiring equal values and the
  same list (an unequal pair refuses with both values shown; no repair route); NHL 17 global = the
  `_DEFAULT_` row alone; NHL 16 global = the `setting.csv` cell. Writer subset enforced before any
  splice: ASCII, no BOM, no `"`, CRLF on every record incl. the last, no blank lines, unique headers,
  no ragged records; header matched by name, extra columns carried. Splice = `plain[..s] + code +
  plain[e..]`; verification = whole-buffer equality outside the span + re-parse equality except the one
  cell; then native re-encode, framing check, independent decode == new text. Placement is predicted
  from the directory (slot slack) and decided by the stager.
- **Held review / pending.** Same ticket namespace and revision binding as D3; `WorkspaceRevision`
  carries the working-text identity, so an apply after any change refuses; undo pops the last pending
  edit (a pair is one entry); the catalogue and the pending review show the overlay; the copied-target
  review refuses (before prepare) when the copied root lacks the archive, keeping the pending edit.
- **UI.** The editor now works on CSV titles: preview passes the install root, the plan lists each
  member's line/cell, stored sizes and predicted placement, the pair pill; apply reports "Pending
  (written by the next copied-target save)". Pending edits shows the catalogue card with the drop
  action. Ownership guards unchanged.

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `NHL_STUDIO_FIXTURES=… cargo test -p tdb-archive-codec --test chunkzip_native -- --include-ignored` | **6 passed**: 53/53 CSV members byte-identical, 2/2 NHL 21 stored-chunk members identical |
| `cargo test -p tdb-archive-codec` / `-p tdb-savedata` / `-p tdb-archive-big` | 45 (+6 ignored) / 6 + 8 / 56 (+1 ignored) passed — the flate2 backend swap changes no decode result |
| `NHL_STUDIO_FIXTURES=… cargo test -p tdb-archive-codec --lib fixtures::tests::ps4 -- --include-ignored` | 2 passed (the four PS3 fixture cases remain absent from the root, as C0-FX1 recorded) |
| `cargo test -p studio-domain --test settings_csv_edit` | **6 passed** (+1 ignored); `real_nhl16_… -- --ignored`: **passed** (188 s; V1 1 556 → 1 556 B fit-in-slot, every other catalogue member identical, restore byte-exact) |
| `cargo test -p studio-api --test settings_csv_route` | **2 passed** (route: preview → apply → pending card → paired edit → undo → copied-target review with `archive-csv` → apply → installed texts → rebind clean → further edit eligible → restore byte-exact; missing archive refused) |
| `cargo test -p studio-api --test copied_target` / `-p studio-domain --test copied_target --test settings_edit --test settings_catalogue --test pending` | 15 / 6 + 3 + 11 + 8 passed |
| `cargo test -p studio-api --lib bindings` / `export-bindings --check` | 25 passed / in sync |
| `cd app && npx tsc --noEmit -p .` / `npx vitest run` | clean / **14 files, 106 passed** |

Not run: full workspace suite, `zz_diag`, clippy/fmt (baseline defects), desktop launch, any game.
Temp copies of the 300 MB archive are removed by the test's `Drop`.

## 5. Remaining blockers / limitations

- Loader acceptance of a re-encoded / re-pointed member and what a creation screen shows: user cards
  (P1BS-CSV-1 with its same-title controls, per the addendum).
- Only `databigs/inxp_00.big` under the install root is addressed; a project opened without an install
  root (file pick) gets the `no-install-root` refusal.
- Logo chains (P1-E/F): the encoder and the EB member stager are shared and ready; the GNF/PNG artifact
  route, the BIGF inner re-plan, BIG4 `.bh` placement, RefPack ratio and the shared-span rule are not
  built (research 02 §4.2–4.3, §5). Disc images: same-length only.
- The 20-byte-row `0x00110400` `archive_size ≠ length` anomaly (NHL 16 `game_90.big`) is still refused
  by the stager as before; `inxp_00.big` on all six titles has `archive_size == length`.
- Synthetic route tests use the stager's qualified 16-byte-row layout; the real 20-byte-row archive is
  covered by the `--ignored` NHL 16 case only.

## 6. Temporary artifacts / processes

- Session scratchpad `d4_ws.py`, `d4_route.py`, `d4_api.py`, `d4_ui.py`, `Cargo.lock.before-d4` — disposable.
- Fixture additions (above) — retained, read-only inputs of the `--ignored` cases (the 300 MB archive
  copy is the one large item; regenerable from the install).
- Test temp dirs `nhlms-d4-*` — removed by `Drop`; no processes left.

## 7. Next

D5 — localization parity (P1-D / LOC-W3 revision 3).
