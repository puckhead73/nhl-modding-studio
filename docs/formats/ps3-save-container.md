# PS3 Save Container Format (verified — read/extract)

> **Verification status: VERIFIED (read/extract).** Container structure, save-type taxonomy, and the
> plaintext/compressed boundary were confirmed against every PS3 save in `anchors/ps3/**`. The
> anchors are already-decrypted emulator exports, so no PFD/AES decryption was needed. Write-path
> (checksum recompute, re-encrypt) is **deferred** per the audit plan.

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

## Single vs multi-part payload

- **Single `SYS-DATA`**: SETTINGS, ROSTER, HOCKEYCARD always; and BEAGM/BEAPRO/SEASON in
  **NHL10–NHL12**.
- **Multi-part `DATA0000..N`**: BEAGM/BEAPRO/SEASON in **NHL13 and later** (the body is split across
  ~1 MB / 3 MB chunks that concatenate into one payload). The split begins at NHL13.

## Plaintext vs compressed payload (key boundary)

The inner database is stored either as a **plaintext TDB** (`DB\x00\x08\x01…`) or **compressed**:

- **NHL10–NHL13**: payloads embed a **plaintext TDB** (extracted and header-validated for SETTINGS,
  BEAGM, ROSTER, BEAPRO, SEASON; HOCKEYCARD carries no TDB).
- **NHL14 / NHL15 / Legacy**: large payloads (ROSTER, BEAGM, BEAPRO, SEASON) embed a **compressed
  TDB** — no plaintext `DB` magic is present. Small SETTINGS stays plaintext even here.

This matches the NHLViewNG EXE's `CompressionSizeTrigger` + Huffman evidence (see
[../audits/nhlviewng-internals.md](../audits/nhlviewng-internals.md)): payloads above a size threshold
are compressed. **Decompression is required to read NHL14+ roster/dynasty saves** and is deferred to
the engine's save phase.

## Verified vs deferred

- ✅ Verified: SFO `TITLE` parse; container magic + save-type map; single/multi-part split point
  (NHL13); plaintext-TDB extraction + header validation for NHL10–13 payloads; identification of the
  compression boundary at NHL14+.
- ⏳ Deferred (engine save phase): container header field semantics (length/checksum offsets), the
  compression codec (Huffman) for NHL14+, `DATA000N` exact reassembly semantics, checksum recompute,
  and any PFD/AES re-encryption for write-back. PS4/Xbox360 containers are out of scope (no anchors).
