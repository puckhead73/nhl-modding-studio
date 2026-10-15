# Handoff R133 (P29) — League 1.2 conference / division names on NHL 08–11: the executables' name lists are recovered, but the local division number does not index them by position (or by reversal) — names stay unassigned; conferences fit position

Research lane, 27 September 2026, published 06:16 MDT (system clock). BETA-HANDOFF B7 / ledger row "1.2 conference / division names NHL
08–11: research unknown — a name source". Claim `research-p29-nhl08-11-group-names`. Executable images regenerated with the 18 September
`xex_image.py` (retail decrypt + LZX; read-only on the inputs) from Xbox 360 NHL 10 `default.xex` (SHA-256 `ccd996d6…`) and NHL 11
(`e769d808…`) into the session scratchpad (deleted after this package); databases = the anchors. Nothing written.

## Plain-language summary

On NHL 08–11 each team stores only a conference number (0 / 1) and a division number (0–5) inside its league. The games' programs carry the
names — "Western Conference", "Northwest Division" and so on — in a fixed list, and the question was whether the numbers simply count down
that list. For conferences they do (0 = the western teams = "Western Conference"). For divisions they do not: division 0 holds Boston,
Buffalo, Montreal, Ottawa and Toronto, but the first division name in the list is "Northwest". Counting from the end works for the AHL but
not for the NHL. So Studio should keep showing division numbers — best as "Division 3: CBJ · CHI · DET · NSH · STL" — until the part of
the program that picks the name is traced.

## 1. The name lists (executable, VA → strings)

| Title (VA of the pointer array) | League blocks, in list order |
|---|---|
| NHL 10 (`0x8333f948`) | NHL [Western, Eastern] [Northwest, Pacific, Central, Southeast, Atlantic, Northeast]; AHL [Western, Eastern] [Atlantic, East, North, West]; Extraliga, Swiss, SM-liiga, DEL, Elitserien (single); tournament block (PoolA / PoolB / Playoffs / ConferenceA–B / DivisionA–D) |
| NHL 11 (`0x834cef70`) | the NHL and AHL blocks unchanged; **+ OHL** [Western, Eastern] [Central, East, Midwest, West]; **QMJHL** [East, Maritimes, West] (no conference names); **WHL** [Western, Eastern] [Bc, Central, East, Us]; European leagues; tournament block |

The strings are dictionary keys (`WesternConference`, `NorthwestDivision`, …), i.e. what the 12 → Legacy group tables name; NHL 08 / 09
images were not regenerated (18 Sep: no such table found in NHL 09 / 07 by the same search).

## 2. The title's own grouping (anchors; identical on NHL 08, 09, 10, 11)

| League | conference | division | Members |
|---|---|---|---|
| NHL | 0 | 3 | CBJ, CHI, DET, NSH, STL |
| NHL | 0 | 4 | CGY, COL, EDM, MIN, VAN |
| NHL | 0 | 5 | ANA, DAL, LA, PHX, SJ |
| NHL | 1 | 0 | BOS, BUF, MTL, OTT, TOR |
| NHL | 1 | 1 | NJ, NYI, NYR, PHI, PIT |
| NHL | 1 | 2 | ATL, CAR, FLA, TB, WSH |
| AHL | 0 | 0 / 1 | (NHL 10) CHA, HOU, MIL, PEO, RFD, SAN, TEX / ABB, GRA, HAM, LEM, MTB, RCH, TOA |
| AHL | 1 | 2 / 3 | (NHL 10) ADK, ALB, BNG, HER, NRF, SYR, WBS / BRI, HFD, LOW, MCH, POR, PRO, SPR, WOR |

(The All-Star rows `EAS` / `WES` sit at conference 0, division 0 of the NHL block.)

## 3. Hypotheses tested

| Hypothesis | Conferences | NHL divisions | AHL divisions |
|---|---|---|---|
| local number k → k-th name of the block | **fits** (0 → Western for the western teams, both leagues) | **fails**: 0 → "Northwest" for BOS / BUF / MTL / OTT / TOR | fails (0 → "Atlantic" for the western teams) |
| k → name counted from the block's end | fails (0 → Eastern) | 4 / 6: fits 0 → Northeast, 1 → Atlantic, 2 → Southeast, 3 → Central, **fails 4 → Pacific (CGY … VAN), 5 → Northwest (ANA … SJ)** | 4 / 4 |

The "fits" / "fails" judgements use the clubs' real-world 2007–2011 alignment (external knowledge, as corroboration only); no title-owned
string ties a division number to a name. Neither index rule holds for every block, so the binding is **not** positional — the consumer that
picks the name was not traced (needs code disassembly).

## 4. Developer use

- Conference names on NHL 08–11: candidate by position (0 → `WesternConference`, 1 → `EasternConference`, dictionary text when attached),
  labelled "inferred"; or keep numbers.
- Division names: **do not assign**. A human-readable neutral label is available from title-owned data: "Division *k*: <member abbreviations>"
  (or the conference plus members), with the raw number behind disclosure.
- Validated moves already use the witnessed arrangement; nothing changes there.

Negative scope: NHL 08 / 09 images not regenerated; no code path traced; PS3 executables not examined.

## 5. Addendum 06:18 — consumer trace attempt (negative; B7 stops here)

On the regenerated NHL 10 image (`probes/r133b_group_name_xrefs.py`): no `lis` / `addi` (or `ori` / `lwz`) pair within 8–12 instructions
produces the array address `0x8333f948` or any address inside its 31 entries; the address is not stored as a data word anywhere in the
image; the only `lis 0x8334`-based references near it land 584–1 740 bytes away (other data). The array is therefore reached through a
computed base or a startup constructor that this bounded scan does not follow. A real disassembler session (function boundaries, register
data flow) is the remaining route; research stops here under the investigation budget. The image was deleted after the scan.
