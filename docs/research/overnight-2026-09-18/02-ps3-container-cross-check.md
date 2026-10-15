# Cross-check — PS3 disc trees vs the Xbox 360 container families (read-only magic scan)

Research lane, overnight 2026-09-18. Scope: the RPCS3 disc trees under `X:\Games\Emulation\Roms\Sony Playstation 3\`
(`NHL 08` … `NHL 15`, `NHL Legacy - Vanilla`; NHL 12 via its extracted folder). Only the top-level `USRDIR`
listing and the first 12 bytes of each `.big` were read. Evidence: **observed**.

| Title (PS3) | Archive family | Archive set | Loose dirs on disc |
|---|---|---|---|
| NHL 08 | `BIG4` + `Viv4` `.bh` | `audioboot, boot, data0, data1` (+ a loose `data0/` dir) | `audio db fe gamedata rendering` |
| NHL 09 | BIG4 + `.bh` | `audioboot, boot, data0, fe, player, rendercache, rendernocache, stadium` | `audio db fe gamedata rendering` |
| NHL 10 | BIG4 + `.bh` | + `audio, audioboot2` | same |
| NHL 11 | **`EB\0\3` v`0x00000400`** | `audioboot, audioboot2, audiostream, boot, data0, fe, rendercache, rendernocache, stadium, transitions` | `attribdb audio db debug fe gamedata ice rendering sku` + `secure0.big.sdat` |
| NHL 12 | EB v0x400 | `audioboot, audioboot2, boot, cache, cacherender, data0, nocache, nocacherender` | same + `scrape` |
| NHL 13 | EB v0x400 | + `cacheaudio, cachemovie, cacheoverlay` | + `scenario` |
| NHL 14 / 15 / Legacy | EB v0x400 | + `cacheboot, renderboot` | same (+ `audioevents` dir on 14) |

Findings:

1. **The archive family is platform-independent**: PS3 and Xbox 360 use the same container generation
   per title (BIG4/Viv4 through NHL 10, EB v`0x00000400` from NHL 11) and the same archive *set names*.
   Entry counts differ slightly (PS3 `cache.big` 0x455e vs Xbox 0x3d-row `cacheboot` etc.) because the
   payload formats differ (DDS vs XPR2, PS3 vs Xbox meshes), so **member paths transfer, bytes do not**.
   Studio's EB v0x400 binder should therefore open every PS3 NHL 11–Legacy archive as well (cheap bind check).
2. **PS3 discs ship the loose `db/`, `fe/`, `rendering/` trees on disc for every title** — unlike Xbox
   NHL 11+, whose discs carry only the archives. This is why the PS3 loose-file candidate route (S2-C) is
   disc-faithful on PS3 but *extraction-only* on Xbox 11+.
3. `secure0.big.sdat` (NHL 11+, PS3) is an SDAT-encrypted archive with no Xbox counterpart — contents
   unknown (not opened).
4. Payload codecs differ: PS3 EB members are `chunkzip` (`rendering/logo/texlib_0_0_0.rpsgl`, `attribdb/renddb.bin`),
   Xbox 11–Legacy members `chunklzx` (handoff 40).
5. Legacy PS3 FE league-logo packages are byte-for-byte the same *shape* as Xbox (six-member BIGF, RefPack
   member) with DDS inside (P1.4-X §3); the same is expected for the other FE families (not verified).

Scope of these claims (narrowed, R1 item 3): this was a header/tree scan — it establishes the **observed
container layout and member families** per title, not byte-identical contents between platforms and not the
original-disc provenance of every loose file (the PS3 trees are RPCS3 disc dumps as found on X:; no PS3 disc
image was hashed here). Known PS3/Xbox differences stay explicit: payload codecs (chunkzip vs chunklzx),
texture formats (DDS vs XPR2, `.rpsgl` vs `.rx2`), entry counts, `secure0.big.sdat`, and — from the earlier
league-parity batch — the PS3 08/09 dictionary anchors are Xbox files, so the dictionaries are **not** assumed
identical across platforms. Any stronger equivalence statement needs the specific files hashed on both sides
(`data/xbox360-key-member-map.json` has the Xbox hashes; PS3 hashes were not computed tonight).

Nothing here changes the batch/completion records' PS3 rows; it corroborates the "same-generation
equivalence" hypothesis at the container level only.
