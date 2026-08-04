# Save-container audit (Phase 4)

> **Status:** established 2026-08-02 (Phase 4 of the
> [Full-game QA and editability plan](../plans/full-game-qa-editability.md)).
> **Automated gate:** `studio_domain::save_matrix::save_matrix_over_all_fixtures`
> (`crates/studio-domain/tests/save_matrix.rs`).
> **Machine-readable companion:** [`data/save-matrix.csv`](data/save-matrix.csv) — the content-detected
> type + round-trip outcome of every committed save fixture, drift-guarded by the same test.

Phases 1–3 proved the *database* is fully editable. Phase 4 proves the **save container** that wraps a
roster faithfully carries an edit: for every committed save fixture the harness detects its type from
the bytes, extracts and validates the inner DB, repacks a modified payload and re-extracts it
byte-for-byte with valid checksums, drives a real scalar edit through the app write seam on ROSTER
saves, and refuses truncated input — all on in-memory copies, never writing the anchor.

This is the **automated** half of Phase 4. In-game / on-console acceptance (`game-verified`) still
requires the manual emulator campaign; NHL 10/11 `v1` and PS4 stay visibly experimental until it runs.

## Detection is by content, not folder name

Every fixture is classified from its bytes alone — `TdbFile::parse` → raw DB, else
`SaveContainer::parse` → a PS3/Xbox360 container (whose header self-describes the variant *and* the
save type via its magic), else `is_ps4_save` → a PS4 LZ4 `DATA` roster, else unsupported. The directory
token (`… - ROSTER`, `DYNASTY …`, `RosterCustom0`) is never consulted for the verdict; it appears in
`save-matrix.csv` only as a label beside the byte-derived `detected_type`, so a reader can confirm they
agree. This is the plan's "detect type by content, not folder name," enforced structurally.

## Container variants and what each proves

Across the 121 committed save fixtures the matrix records **24 `edit-roundtrip`** (proven roster
round-trip + a real edit through the app seam), **4 `experimental-roundtrip`** (NHL 10/11 `v1`),
**19 `roundtrip`** (a non-roster inner TDB whose codec fidelity is proven but which is not a roster
editor target), and **74 `read-only`** (no coherent editable inner DB) — see
[`save-matrix.csv`](data/save-matrix.csv) for the per-fixture verdict. The **ROSTER** round-trips are:

| Codec | ROSTER saves | Write | Proven here |
|---|---|---|---|
| `RawV3` | NHL 07–09 ROSTER (`Rosters`; note NHL 08/09 carry a gen7 inner DB in a v3 container) | yes | extract → identity repack → modified payload faithful; single `checksum_a` recomputed |
| `RawV1` | NHL 10/11 ROSTER | **experimental** | full round-trip + a real edit survives, but `tag_1c` is un-reversed (stale on an edited body) → on-console load unverified |
| `RawV2` | NHL 12/13 ROSTER | yes | extract → identity repack → modified payload faithful; both checksums recomputed |
| `ZlibV4` | NHL 14/15/Legacy ROSTER | yes | zlib inflate/deflate round-trip: inner DB identical, checksums valid, deterministic; zero-pad allocation preserved |
| PS4 LZ4 | NHL 16–21 `RosterCustom` `DATA` | experimental | LZ4 frame decode/encode round-trip, both framings (bare NHL 16, `01`-prefixed 17+); other records preserved verbatim |

25 of the 28 ROSTER fixtures additionally took a real scalar edit through `EditSession` (`semantic_edit`
= `yes`); the three that did not — a couple of the gen6/early `v3` rosters whose inner subset did not
resolve an editable field against the title metadata — still prove **full container fidelity** via the
modified-payload round-trip, so the edited body is provably carried even there.

Non-roster and unsupported fixtures are classified from content and recorded **read-only** or
**roundtrip** (codec-only), never edited. The verified findings:

- **Settings / profile** saves span several container versions: PS3 NHL 10/11/14/15 SETTINGS use
  version word **`5`**, which the discriminator refuses (`UnsupportedVersion`) → read-only; other
  PROFILE saves classify as `v1`/`v2`/`v4` and are read-only unless they carry a coherent inner TDB
  (one `v2` PROFILE round-trips at the codec level). There is **no single "settings = v5"** rule — the
  version is per-title and always taken from the bytes.
- **NHL 15 PS4 roster** (`… - ROSTER` `DATA`) is **not recognised** by the LZ4 roster detector (its
  framing differs from the NHL 16 bare / NHL 17+ `01` variants) → read-only, an honest unverified gap.
  PS4 `Profile`/`BeAGM` `DATA` likewise carry no roster record and are read-only.
- **Multi-part / late Xbox saves** (NHL 14/15/Legacy Xbox `DYNASTY`/`SEASON`/`BEAPRO`/`PROFILE`, and
  the PS3 multi-part dynasty/season) do not parse to a single complete inner TDB — the codec does not
  reassemble `DATA000N` segments — so they are **read-only/blocked** pending explicit multi-part work.
- **HockeyCard** and any save whose payload is not a TDB — no inner DB, read-only.
- **Xbox 360 STFS outer packaging** — out of scope; the anchors ship already-unwrapped inner payloads,
  and the outer-package layer stays `blocked` in the QA matrix.

## The per-fixture contract

For every fixture that carries a coherent inner TDB:

1. **Extract + validate** — the inner bytes parse as a TDB.
2. **Identity round-trip** — repacking the *unmodified* inner DB re-extracts it byte-for-byte, the
   repacked container's checksums verify, and the repack is **deterministic** (repacking twice is
   byte-identical). For the raw variants (`v1`/`v2`/`v3`) the whole container is byte-identical on a
   no-op; `v4` re-deflates, so only its inner DB is guaranteed identical (the plan's "preserved opaque
   regions" — the trailing non-DB bytes and header are carried verbatim either way).
3. **Modified-payload fidelity** — flipping one interior byte of the inner DB, repacking, and
   re-extracting reproduces the edited bytes exactly with valid checksums, proving the codec carries an
   arbitrary edited body of *any* save type, not just a no-op.

For every **ROSTER** save additionally (the app write seam):

4. **No-op save** through `Project::load` → `EditSession::save` leaves the inner DB byte-identical.
5. **Representative scalar edit** — a bounded, non-key integer field on the inner roster subset is set
   through `EditSession::set_field`, the session saves (repacks), and reopening the container shows the
   edit present in the inner DB with a valid inner CRC and valid container checksums.

For every fixture:

6. **Malformed rejection** — header-relative truncations (below the header, header-only, and a body
   sliver — *not* a fraction of the zero-padded file, which would remove only padding) never yield a
   size-valid inner DB, and the probing never mutates the source bytes.

## QA-matrix promotion

`save-raw-db` and `save-inner-container` for every anchor whose fixture round-trips move from
`discovered` to `edit-roundtrip-pass`, citing this gate + `save-matrix.csv`. The experimental caveats
are preserved verbatim in each cell's `blocker`: **NHL 10/11 `v1`** (`tag_1c` un-reversed) and **PS4**
(repack unverified on-console) remain flagged, and **multi-part** dynasty/season stay
`missing-fixture`/`blocked` rather than promoted. **Xbox STFS outer packaging** stays `blocked`. No
cell is promoted to `game-verified` — that axis still requires the manual campaign.
