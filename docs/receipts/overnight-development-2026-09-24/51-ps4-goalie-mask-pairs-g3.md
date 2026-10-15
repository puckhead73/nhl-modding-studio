# Receipt 51 — Equipment 4.5 G3: PS4 goalie mask pairs checked against each title's own vault catalogue (and a Rust Attrib/VLT reader)

**Developer-verified; not Codex-accepted; nothing launched or written.** HEAD `8d60a6d` (unchanged; nothing staged). Claim
`dev-ps4-goalie-mask-pairs-g3`. Research consumed: 2026-09-28 handoff 02 (R135 / R135a; G3 contract), with the vault format record of
R7 (handoff 07, 19 Sep). Consumed under the dispatch's fallback 3 (a ready contract), read-only.

## 1. Plain-language summary

- **PS4 goalies show whether their mask pair exists.** On PS4 a goalie's mask is chosen by two numbers together, the design and the
  helmet type. Player → Equipment now shows a "Mask pair (PS4)" card, checked against the mask catalogue inside that game's own files:
  - "one catalogue mask";
  - a helmet type the catalogue does not have (99 on NHL 19–21);
  - a design that exists only for other helmet types (named);
  - or no catalogue record.

  Anything unusual is **reported, not refused**. Whether the game rejects such a pair is unknown.
- **Pending edits are checked too.** The card reads the working values, so a changed number shows its own check before saving.
- **Team mask slots on PS4** now list the helmet types each paint exists for, or "no catalogue record" (e.g. the TPR / TPW teams' 248 /
  249).
- **The right game's catalogue or none.** The catalogue is read only from the attached install's `boot.big`, and only when its four
  vault files match the recorded identity of the open project's title. An NHL 19 project with an NHL 20 install attached is refused as
  a source mismatch; the numbers are still shown.
- **Beyond G3:** Studio can now read the PS4 Attrib/VLT vault in Rust (identity-checked). That is the prerequisite research names for
  PS4 head, venue and kit joins.

## 2. Files

| Path | Change |
|---|---|
| `crates/studio-domain/src/ps4_vault.rs` (**new**) | lookup2 key (10 values equal research's `vlt_hash`); chunk chain, ExpN, PtrN segments, schema classes (names from the unframed string pool after `EndC` in `gcschema.bin`), collection records, blob / inline attribute decode (Text, RefSpec, integers, enums), `texture_file_ref`; `read_install_vault` (the four `boot.big` members, `chunkzip` unwrapped, identified against defs); every read bounds-checked |
| `defs/ps4/vault-identities.json` (**new**) | full SHA-256 of the four vault members per title NHL 15–21 (from R7's JSON; R135a showed they equal the `boot.big` members) |
| `crates/studio-domain/src/ps4_goalie_masks.rs` (**new**) | `MaskCatalogue` (identity-checked load; the design / helmet type found by their **enum types**, as R135 reads them), `PairClass`, `classify`, `paint_helmet_types`, `goalie_rows` / `EditSession::ps4_goalie_rows` |
| `crates/studio-api/src/{dto,registry,commands,lib,bindings}.rs` | `Ps4GoalieMaskPairDto`; `TeamMaskPaintDto.helmet_types`; cached catalogue on the Registry (key: title, install, `boot.big` length + mtime); `ps4_goalie_mask_pair`; `team_goalie_masks` now resolves defs (PS4 helmet types) |
| `app/src/components/Ps4GoalieMaskPairCard.tsx` (+ test), `PlayerEquipmentTab.tsx`, `TeamGoalieMasksCard.tsx`, `lib/client.ts`, 2 bindings | the card; helmet types beside PS4 paints |
| tests (**new**) `studio-domain/tests/ps4_goalie_mask_pairs.rs`, `studio-api/tests/ps4_goalie_mask_pair.rs` | below |

## 3. Evidence (real installs `H:\Emulators\ShadPS4\Games\<CUSA>` — only `boot.big` read; anchors; the genuine NHL 20 fixture)

- **`ps4_goalie_mask_pairs` 2 / 2 (`--ignored`), every figure equal to R135:**
  - **pack counts:** NHL 15 506, 16 492, 17 526, 18 578, 19 940, 20 974, 21 991. Helmet types are 0–7 on NHL 15–18 and 0–8 on NHL
    19–21. Every pack resolves its colour map. Each title loads in 2.0–3.3 s.
  - **goalie census on the anchors** (unique / other types / absent / type not in the catalogue):
    - NHL 15: 559 / 0 / 0 / 0;
    - NHL 16: 556 / 3 / 7 / 0;
    - NHL 17: 596 / 0 / 5 / 0;
    - NHL 18: 714 / 0 / 0 / 0;
    - NHL 19: 743 / 0 / 0 / 4;
    - NHL 21: 744 / 0 / 0 / 5.
  - **contract cases:**
    - genuine NHL 20 row 1 (125, 0) → `goaliehelmet_itech_generic_8_texpack`, colour map `…generic_8_cm`;
    - row 88 (0, 99) → type not in the catalogue;
    - genuine NHL 20 totals: 773 rows, 768 unique, 5 type 99;
    - NHL 16 row 0 (198, 0) → absent; row 78 (77, 0) → "exists only for helmet type 1, not 0";
    - NHL 21 paints 248 / 249 → no record;
    - the NHL 20 install for an NHL 19 project → source mismatch;
    - `anchors/ps4/nhl20` classifies exactly like the NHL 19 anchor, because it is the NHL 19 base copy.
  - **First run failed (recorded):** the schema's type names were empty. `gcschema.bin` keeps them unframed after an empty `StrE` /
    `EndC`. Then the design value was not found: its attribute key is not the hash of "GoalieMaskTexture", so it is found by its enum
    type, as research does. A temporary diagnostic test was used and deleted.
- **API `ps4_goalie_mask_pair` 1 / 1 (`--ignored`):**
  - 400 genuine NHL 20 goalies: 399 unique, 1 type 99, with no catalogue problem (one vault read, then cached);
  - TPR's paint 248 → no record, with the catalogue note;
  - an NHL 19 project → a named source mismatch, numbers still shown.
- **Other suites:** `ps4_vault` lookup2 unit test 1 / 1; vitest `Ps4GoalieMaskPairCard` 3 / 3 and `TeamGoalieMasksCard` 2 / 2; `tsc`
  clean; bindings written.

## 4. Limits

- **Not claimed:**
  - which pack the game draws, whether it rejects a missing pair, how team slots combine with a goalie's pair (card G-MASK-1), and
    what helmet type 99 means;
  - no labels: "helmet type k" only, with brand geometry names and pack names behind disclosure.
- **Not done:**
  - Creation Zone thumbnails (contract item 6, optional);
  - localized display keys.
- **Performance:** the Player page asks per goalie. The first request on an install reads the vault (2–3 s, cached for the session).
