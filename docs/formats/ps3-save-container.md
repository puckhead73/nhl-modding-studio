# PS3 Save Container Format (verified)

> **Verification status: VERIFIED, incl. the write checksums.** Container structure, save-type
> taxonomy, plaintext/compressed boundary, and the **v2/v4 container checksums** are confirmed against
> the anchors. Full byte-exact container spec (v1/v2/v4 headers, zlib body, recompute order):
> **[tdb-v6-format-spec.md §4](tdb-v6-format-spec.md#4-save-container-layout)**.
>
> **Checksums (verified on every v2/v4 save tested):** `checksum_a` @`0x10` = **CRC-32/IEEE** over
> `file[0x1C..EOF]` (BE); `checksum_b` @`0x28` = **CRC-32/BZIP2** over `file[0x2C..EOF]` (BE). So the
> **save write-path is unlocked for v2 (NHL12/13) and v4 (NHL14/15/Legacy)**. **v1 (NHL10/11) is
> read-only** — its `tag_1c@0x1C` is un-reversed. **PFD/AES is net-new** (the anchors are already
> RPCS3-decrypted exports; no PFD code exists to port).
>
> **Container version varies by save TYPE, not just game** — e.g. NHL12 ROSTER is v2 but NHL12 BEAGM
> is v1. Detect by body marker, not the version byte (see spec §4).

## Save directory layout

Each save is a directory `BLUS#####<N> - <TYPE>` containing:

- **`PARAM.SFO`** — standard PS3 SFO metadata (magic `\0PSF`). `TITLE` reads cleanly
  (e.g. `NHL® 14`, `NHL® Legacy Edition`). The `BLUS#####` is the title id.
- **`ICON0.PNG`** — save icon (not game data).
- **payload** — either a single **`SYS-DATA`** file, or multi-part **`DATA0000..DATA000N`**.

## Save types (6 per game) and container magics

The payload begins with an ASCII magic `PS3<Type>File`:

| Save type | Container magic | Contents |
|---|---|---|
| SETTINGS | `PS3UserProfileFile` | small user-profile TDB |
| BEAGM | `PS3DynastyFile` | Be-A-GM / dynasty data (largest) |
| ROSTER | `PS3RosterFile` | roster TDB (the primary editing target) |
| BEAPRO | `PS3BeAProFile` | Be-A-Pro career data |
| HOCKEYCARD | `PS3HockeyCardFile` | hockey-card / image data (no TDB) |
| SEASON | `PS3SeasonFile` | season-mode data |

After the magic comes a container header (length/checksum fields) then the payload body (a TDB, or a
compressed TDB — see below). The full per-save inventory is `../audits/data/savefile-inventory.csv`.

## Container header (verified)

Three header variants; full field maps in [tdb-v6-format-spec.md §4](tdb-v6-format-spec.md#4-save-container-layout):

| Version | Header | Games | Body | Writeable |
|---|---:|---|---|---|
| **v1 RawV1** | 32 B (`0x20`) | NHL10/11 (+ some NHL12 types) | raw TDB | **No** (`tag_1c@0x1C` un-reversed) |
| **v2 RawV2** | 44 B (`0x2C`) | NHL12/13 | raw TDB, tight | Yes |
| **v4 ZlibV4** | 48 B (`0x30`) | NHL14/15/Legacy | zlib stream @`0x30` + zero pad | Yes |

Both checksums are at fixed offsets in all variants: `checksum_a`@`0x10` (CRC-32/IEEE over `[0x1C..]`),
`checksum_b`@`0x28` (CRC-32/BZIP2 over `[0x2C..]`) — **both verified**; v4 also has `uncompressed_size`@`0x24`
and `compressed_size`@`0x2C` (LE). The inner content is a plaintext TDB (`DB\x00\x08…`) or, for v4, a
zlib/DEFLATE stream wrapping the TDB.

## ⚠️ Saves contain MORE than the inner DB (handle with care)

A save payload is **not** simply `[header + DB]`. Verified across all anchors:

- **BEAGM / BEAPRO / SEASON** carry **150 KB – 900 KB of additional non-DB data *after* the inner
  database** (dynasty / career / season state). E.g. nhl10 BEAGM: DB is 2.47 MB but 871 KB follows it.
- **SETTINGS** is a tiny **132-byte** DB followed by ~1.15 MB of profile/settings data.
- **HOCKEYCARD** contains **no TDB** at all (card/image data).

**Implication:** "pull the DB out and drop the container" is **lossy** — it would discard these
sections. The checksums are now reversed (above), so a modified DB *can* be re-injected into a v2/v4
single-file save by patching the body and recomputing `checksum_b`→`checksum_a` — but you must
**preserve the whole payload** (header + DB + the trailing non-DB sections), not just the DB.

## Single vs multi-part payload (⚠️ discrepancy with the spec)

- **Single `SYS-DATA`**: SETTINGS, ROSTER, HOCKEYCARD always; and BEAGM/BEAPRO/SEASON in **NHL10–12**.
- **Multi-part `DATA0000..N`**: BEAGM/BEAPRO/SEASON in **NHL13+** (~1 MB / 3 MB chunks).

> The spec ([§4](tdb-v6-format-spec.md#4-save-container-layout)) states the prior app handled
> **single-file saves only** ("`DATA000N` reassembly: NOT PRESENT") — it worked with the ROSTER save
> (single `SYS-DATA`). But our anchors contain multi-part dynasty/season saves, so **`DATA000N`
> reassembly is net-new / unverified** for those types. Our checksum verification was on single-file
> saves; whether plain concatenation is the correct reassembly for multi-part is **not yet confirmed**.

## Plaintext vs zlib payload (key boundary)

The inner content is either a **plaintext TDB** (`DB\x00\x08\x01…`) or a **zlib stream** — note this is
the **save-container zlib** (v4), which is *separate from* the DB-internal Huffman varchar codec
(don't conflate; see spec §3):

- **NHL10–13** (v1/v2 containers): plaintext TDB body.
- **NHL14/15/Legacy** (v4 ZlibV4): the body is an **RFC-1950 zlib/DEFLATE** stream at `0x30` (small
  SETTINGS may stay plaintext). Inflate to get the TDB; the engine deflates + repads + recomputes the
  two checksums on write.

## Verified vs deferred

- ✅ **Verified:** SFO `TITLE`; container magic + save-type map; v1/v2/v4 header layouts;
  **checksum_a/checksum_b** (write-path for v2/v4 single-file saves); plaintext-TDB extraction; the
  zlib boundary at NHL14+.
- ⏳ **Deferred / open:** `DATA000N` multi-part reassembly (net-new — see discrepancy above); **v1
  write** (`tag_1c` un-reversed → NHL10/11 read-only); the DB-internal Huffman varchar codec (spec
  §3.1, verify when decoding pools); **PFD/AES** for raw console saves (net-new). PS4/Xbox360
  containers out of scope (no anchors).
