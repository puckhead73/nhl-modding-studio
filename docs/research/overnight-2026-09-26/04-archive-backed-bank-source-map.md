# Handoff R110 (P4) — archive-backed gen7 sound banks: a per-bank source ownership map for PS3 NHL 09 / 10 / 12 / 13 and X360 NHL 09 (1 003 bank identities, 9 987 stream entries), for the developer's next action (receipt 22 §8); **every archive-backed `.abk` and `.ast` member is stored unwrapped** (starts with `ABKC` / `S10S`), correcting R82's "RefPack / chunkzip" row labels for the banks; no bank has disagreeing copies; X360 NHL 09 has a byte-identical loose copy of every bank; PS3 NHL 10 keeps four stream files loose only

Research lane, 26 September 2026, 01:16 → 01:17 MDT (real clock). Review package 4 (reconcile the queue with developer packages 20–22;
prefer source-ownership and archive-backed coverage that unblock a concrete listing / preview). Claim
`research-p4-archive-backed-bank-source-map`. Derived from the published R98 JSON (member identities and hashes) plus one read-only raw-
header check of every bank member (R110b). No decode, no new framing sweep, nothing game-verified.

Machine-readable: [`data/r110-archive-backed-bank-source-map.json`](data/r110-archive-backed-bank-source-map.json) (per bank: every `.abk`
copy with source, path, wrapper, stored / decoded size and SHA-256; stream entries; `.ast` copies with id check and framing status),
[`data/r110b-abk-member-wrapper-check.json`](data/r110b-abk-member-wrapper-check.json).

## Plain-language summary

Studio's new sound-bank card works on the older titles that keep their sound banks as loose files, and refuses the five titles that
keep them inside archives. The developer's next step is to read those banks from the archives. This package gives that step a map: for
each bank on each of the five titles, which archive (or loose folder) holds the bank file and its stream file, with their exact sizes and
fingerprints. It also corrects an earlier note: the bank files inside these archives are not compressed at all — they start with their
own header — so no decompression step is needed for them (music files beside them are compressed, which is where the earlier label came
from). No bank exists in two different versions; on Xbox 360 NHL 09 each bank exists twice, as an archive member and a loose file, with
identical bytes, and which of the two the game reads is not known.

## 1. Map summary (R110)

| Tree | bank identities | `.abk` sources (all stored) | copies per bank | banks with stream records | `.ast` sources (all stored) | stream entries framed (R98) |
|---|---|---|---|---|---|---|
| PS3 NHL 09 | 186 | `data0.big` 180, `audioboot.big` 6 | 1 | 7 | `data0.big` 7 | 506 |
| PS3 NHL 10 | 199 | `audio.big` 185, `stadium.big` 8, `audioboot2.big` 5, `audioboot.big` 1 | 1 | 7 | `audio.big` 3, **loose 4** (`colors1`, `colors2`, `crowds`, `playercalls`) | 1 108 |
| PS3 NHL 12 | 219 | `nocache.big` 209, `audioboot2.big` 9, `audioboot.big` 1 | 1 | 15 | `nocache.big` 15 | 3 719 |
| PS3 NHL 13 | 213 | `nocache.big` 202, `audioboot2.big` 10, `audioboot.big` 1 | 1 | 15 | `nocache.big` 8, `cacheaudio.big` 7 | 3 708 |
| X360 NHL 09 | 186 | `data0.big` 180 + `audioboot.big` 6 **and** loose 186 | 2 (byte-identical) | 7 | `data0.big` 7 **and** loose 7 (byte-identical) | 473 distinct (946 counting both copies) |

Banks whose copies disagree after unwrap: **none** on any tree. Residuals / bank-tree refusals in R98: none.

## 2. Correction to R82 (wrapper labels)

R110b read the first 16 bytes of every `.abk` / `.ast` member in every archive of the five trees: **all begin with the payload magic**
(`ABKC` for 959 bank members, `S10S` for 47 stream members); none is RefPack, chunkzip or chunklzx. R82 handoff 01 §1 (population rows
"archived PS3 NHL 09 / 10 … RefPack", "PS3 NHL 12 / 13 … chunkzip") and §3 (`.abk` rows "`audioboot.big` (RefPack)", "(chunkzip)")
applied the wrapper of the music `.sbr` members to the banks; for the banks the label is wrong. R82's record counts and grammar results
are unaffected (its reader decoded "stored" members as-is). A dated note is placed on handoff 01.

## 3. Developer-facing contract (listing / framing / preview inputs; no writer)

- **Source resolution:** per bank, the `.abk` member path and archive in the JSON; the `.ast` is the same-stem member (or loose file on
  PS3 NHL 10) whose `S10S` id word equals the bank's `0x2c` word (R98: 51 / 51 used copies match). Wrapper handling for these members:
  none required (a wrapper-detecting read stays correct). Where two identical copies exist (X360 NHL 09), list the member and note the
  loose alias; do **not** state which one the game uses.
- **Framing / decode:** as current queue rows C1 / C2 (PS3 EAL3 one- and two-channel non-loop records decode diagnostically; X360 XMA is
  framed, decode pending the developer's decoder decision; X360 NHL 09 needs the frame-walk rule of C1 before trusting sample words).
- **Pins available** (research, not Studio runs): R98 per-tree counts above; R100 decoded vectors on PS3 09 / 10 / 12 / 13 (15 sub-streams)
  and X360 NHL 09 (3); per-bank SHA-256 in R110.
- **Still separate:** audible quality, game consumption, replacement (C6).

## 4. Queue reconciliation with developer packages 20–22

Receipt 20 (encoding contract) and 21 (equipment vocabularies) do not consume audio research. Receipt 22 consumed R82 / R89 / R91 / R92 /
R94 and R98 / R100 as refusal wording; its four observations are carried in the 2026-09-25 STATUS (22:10). Current queue rows C1 / C2
stand; row C10 (this map) is added; nothing in C1–C9 is superseded by receipts 20–22.

```
python local/runs/overnight-research-2026-09-26/probes/r110_archive_backed_bank_source_map.py docs/research/overnight-2026-09-26/data/r110-archive-backed-bank-source-map.json
python local/runs/overnight-research-2026-09-26/probes/r110b_abk_member_wrapper_check.py docs/research/overnight-2026-09-26/data/r110b-abk-member-wrapper-check.json
```
