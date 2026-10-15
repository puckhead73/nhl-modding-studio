# Handoff R128 (P24) — Legacy Recompiled source identity: the registered Recompiled install's game data is byte-identical to the Xbox 360 NHL Legacy tree, and its archived main database equals the committed Xbox Legacy anchor

Research lane, 27 September 2026, published 05:33 MDT (system clock). Follow-on from [BETA-HANDOFF.md](BETA-HANDOFF.md) (coverage of the 27
shipping targets; the prompt asks to verify Recompiled rather than inherit Legacy). Claim `research-p24-recompiled-source-identity`. Read-only:
files hashed, one archive member read by offset, save files hashed (no content decoded or copied). Nothing launched.

Machine-readable: [`data/r128-recompiled-source-identity.json`](data/r128-recompiled-source-identity.json).

## Plain-language summary

Studio has treated NHL Legacy Recompiled as "not measured" because no database from it was ever committed. The one Recompiled install on
this machine turns out to ship exactly the same game files as the Xbox 360 NHL Legacy copy — every file, byte for byte — *(corrected 28 Sep, checkpoint 00 K5: the 20 paths the Recompiled `game\` shares with the X360 tree are byte-identical; one extra Recompiled file `default - Copy.xex` = `default.xex`; the X360 tree's loose extracted folders have no Recompiled counterpart; archive identity is not loose-route availability or loader precedence)* and the database
inside its archives is the same file Studio already tests as the Xbox Legacy anchor. So anything Studio proves about the Xbox Legacy
database is proven about this Recompiled install's database too, by measurement rather than assumption. What can still differ is how
the recompiled program behaves, where it keeps saves, and which copy it loads — those need their own evidence.

## 1. Measured

| Item | Result |
|---|---|
| Install (registered 2026-09-18) | `H:\Emulators\games\XBOX\NHL Legacy Decomp Test\` — `nhllegacy.exe` SHA-256 `7264bec6…` (unchanged since 18 Sep), `game\` |
| `game\` vs `X:\…\XBOX360\NHL Legacy\NHL Legacy\` | **20 / 20 shared paths byte-identical** (SHA-256, 6.30 GB: the 13 top-level `.big`, `gamedata\{aidata, gps, nis}.big`, `gamedata\anim\faceposer.big`, `default.xex` `d818eb15…`, `options.viv`, the file `nxeart`); one Recompiled-only file, `default - Copy.xex`, has the same bytes as `default.xex` |
| Main database | not loose in the Recompiled install; archive member `db/nhlng.db` of `cacheboot.big` (offset 18 177 856, 2 901 504 B, **stored**, head `DB\0\x08`), SHA-256 `105d2186…` = **`anchors/xbox360/nhllegacy/databases/nhlng.db`** = the X: tree's loose `db/nhlng.db` |
| Other archived databases | `db/draft_roster.db`, `db/dynamic_ratings.db`, `fe/loc/nhl_*_*.db` (8 locales) in the same `cacheboot.big` — identical to the Xbox copies by archive identity (not individually hashed) |
| Saves | `C:\Users\puckh\Documents\nhllegacy\B13EBABEBABEBABE\454109EC\00000001\<TYPE ts>\<TYPE ts>` + `Headers\00000001\<TYPE ts>.header`; files are the **raw** inner saves (ROSTER starts `RosterFile`, 2 456 120 B) — the same layout and size as the committed Xbox Legacy anchor saves (`anchors/xbox360/nhllegacy/savefiles/ROSTER …`, 2 456 120 B); contents are the user's (not compared) |

## 2. What this changes (for the developer and the coverage statements)

- **Database-level coverage** of Legacy Recompiled for this install = the Xbox Legacy anchor's, by **measured byte identity** (SHA-256
  above), not by inheritance. Automated cases that ran on `anchors/xbox360/nhllegacy` exercised the same database bytes. Cite this handoff
  when a receipt states Recompiled database coverage; keep "this install" as the scope (other Recompiled builds are not measured).
- **Asset-level** reads that resolve through the Xbox Legacy archives see identical bytes; loose-tree routes do not apply (the Recompiled
  install has no loose `rendering\` / `db\`); a `_compiled\` loose route is a user convention seen only in another folder (18 Sep §5).
- **Still separate, not established by bytes:** which copy the recompiled executable loads (`update:\patch.big` / `cache:` probed first
  per its 15 Jun log — absent in this install); the save container / header sidecar handling (Studio's `detect_recomp`); every runtime
  behaviour; audio decoder admissions (receipts refuse Recompiled by name for lack of a vector — the banks are the same bytes, so the X360
  Legacy vectors apply to the bytes, while the admission decision stays the developer's).

## 3. Reproduce

```
python -B local/runs/overnight-research-2026-09-26/probes/r128_recompiled_source_identity.py <out.json>   (≈ 1.5 min, reads ≈ 12.6 GB)
```
Executed once; the archived-database comparison was an additional one-member read by offset (recorded in the JSON).
