# Receipt 04 — Priority 2-A: the gen7 `chunklzx` decoder promoted into Studio

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
baseline `8d60a6d`. Read-only against the real Xbox 360 installs on `X:`; nothing written live.
Claim: `dev-p2a-chunklzx-promotion`.

## 1. Plain-language summary

Every Xbox 360 NHL 11–Legacy archive member stored with EA's LZX wrapper (`chunklzx`) — rink
logos, centre-ice and jersey texture libraries, pucks, the attribute databases — was unreadable in
Studio: the decoder existed but walked the container wrongly and used the wrong LZX window, so the
public paths reported it as unsupported. The research lane pinned the exact framing on 18
September; tonight Studio's decoder was corrected to it and checked against the research vector
manifest: **all 50 members of six titles decode byte-for-byte** to the game's own loose files, and
the older three-member oracle passes too. Those members can now be extracted, listed and previewed
through the ordinary paths. Writing them back is **still not offered**: there is no LZX encoder, and
whether the game would accept a `chunkzip` substitute is an open runtime question — the replace
dialog says exactly that instead of substituting silently.

## 2. What users can now do

- Extract and read `chunklzx` members from `boot.big`, `cache.big`, `cacherender.big` /
  `rendercache.big` (NHL 11–Legacy Xbox 360) with the archive extractor and the ISO/archive
  readers; previews of textures inside them follow through the existing texture surface.
- See a precise refusal ("stored as chunklzx … no LZX encoder … replacement is not offered, card
  R2-LZX-1") when trying to replace such a member.

## 3. Implementation

| Component | Change |
|---|---|
| `crates/tdb-archive-codec/src/lzx.rs` (`bbee5f8036190321`) | `FRAME_SIZE = 32 768`; the bit reader realigns to a 16-bit word at every output frame boundary (libmspack `REMOVE_BITS(bits_left & 15)`); a match that would cross a frame boundary is refused (mspack's "decode beyond output frame limits"). |
| `crates/tdb-archive-codec/src/lib.rs` (`c00ebb045f556047`) | `decompress_chunklzx` rewritten to handoff 40 §2b: chunk header at `((cursor − 8 + 15) & !15) + 8`; flag 3 = sub-frame walk (`[u16 csize]` full frames, `[FF][u16 usize][u16 csize]` last frame, exactly 5 trailer bytes) concatenated into one LZX stream per chunk decoded with **window bits 17** (`CHUNKLZX_WINDOW_BITS`, never the header word) to exactly `full × 32 768 + usize` bytes; flag 0 = stored (defensive); any other flag, an overrun, a short output, a bad prefix, `n_chunks = 0` with data, a last-frame size > 32 768 → `CodecError::Malformed` / `ShortOutput` / `TruncatedChunk` / `TruncatedHeader`, never a panic or zero padding. `decompress_if_wrapped` decodes `chunklzx` (was `UnsupportedLzx`). |
| `crates/tdb-archive-big/src/eb.rs` (`05a438a83c6d794e`) | `decode_payload` decodes `chunklzx` (`Payload::UnsupportedLzx` retained for write paths, no longer produced); `is_chunklzx`; the old three-member oracle test made strict (missing mount fails) and its WIP note removed. |
| `crates/tdb-archive-big/tests/chunklzx_vectors.rs` (`a06582f291138ff6`, `--ignored`) | the 50-vector manifest: each member read from its real archive by offset/size, input SHA-256 checked, decoded, compared with the manifest's output SHA-256 + length **and** with the loose extracted plaintext; covers two-chunk members and full-32 768 last frames. |
| `crates/studio-domain/src/asset_member_edit.rs` (`b845d2e5c61c9da8`) | EB member replace refuses a `chunklzx` member with the encoder/substitution reason (card R2-LZX-1) before decoding. |
| `crates/studio-domain/src/extract.rs` / `xiso_source.rs` | doc wording (members now decode; only decode failures are skipped). |

Research consumed: 2026-09-18 handoff 40 (`codec-chunklzx-framing`, `ready`) §1 framing, §2 vectors,
§2b required behaviour + malformed cases. The R1 corrections record §4.2 (20 Sep) had listed this
promotion as **not done** — it is done now; the encoder and the substitution question are unchanged.

## 4. Checks executed (developer)

| Command | Result | Log |
|---|---|---|
| `cargo test -p tdb-archive-big --test chunklzx_vectors -- --ignored --nocapture` | **1 passed: 50/50 byte-exact** (6 titles, 8 two-chunk, 6 full-32K last frames), 1.3 s | [p2a-chunklzx-vectors.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2a-chunklzx-vectors.log) |
| `cargo test -p tdb-archive-big --lib eb -- --ignored chunklzx_decodes` | 1 passed (the older 3-member oracle incl. the 495 KB two-chunk texlib) | — |
| `cargo test -p tdb-archive-codec -p tdb-archive-big --lib` | 45 + 80 passed (new: stored-chunk decode through the alignment rule; 11 malformed-input refusals) | in aggregate |
| `cargo test -p studio-domain --lib extract` | 20 passed | — |
| aggregate `cargo test -p tdb-archive-codec -p tdb-archive-big -p studio-domain -p studio-api` | see §4a | [p2a-aggregate-default.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2a-aggregate-default.log) |

### 4a. Aggregate

`cargo test -p tdb-archive-codec -p tdb-archive-big -p studio-domain -p studio-api` (default mode, one run
after receipts 03 + 04): **exit 0**, every test binary `ok` (`p2a-aggregate-default.log`; ignored =
fixture / mount-gated, never counted as passing).

Not run: any launch; no write of a `chunklzx` member is possible. Ignored = fixture/mount-gated.

## 5. Coverage effect

- Phase 2 matrix 2.5 (gen7 `rendering/*` members): **read / preview no longer blocked** — the
  decoder is promoted and vector-proven; **write remains blocked** (no encoder; substitution
  unqualified — card R2-LZX-1).
- Phase 1 matrix 1.4 archive placement on gen7 EB: unchanged (logo packages are `chunkzip`).

## 6. Cleanup

Scratchpad `chunklzx.py`, `lzx_promote.py`, `lzx_consumers.py`, `lzx_consumers2.py` deleted at
checkpoint. No temp roots, no processes.
