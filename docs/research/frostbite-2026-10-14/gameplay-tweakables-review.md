# Gameplay tweakables catalogue review — 14 October 2026

Status: reference ingested and CSV contents audited. No application implementation,
executable patching, game-file writes, or runtime qualification performed.

## Source and provenance

Source: [gameplay_tweakables.csv](../../../Reference/New%20folder/gameplay_tweakables.csv),
provided by the user alongside NHL24Kit and the NHL27-to-NHL24 graphics research.
The source is preserved unchanged.

- Size: 1,433,711 bytes.
- SHA-256: `4ededfcf815c70cbd4b48b9fdc2a67665387c97dfb283c45a0c73cc1b280c65e`.
- Columns: `name,type,default,min,max,editable`.
- 25,474 data records; 24,434 distinct names under ordinal, case-sensitive comparison.

The CSV does **not** identify its title, platform, update/build, extraction tool/version,
executable hash, asset path, address/offset, owning object, or application method.
Its placement beside NHL24Kit does not establish NHL24 provenance. Until matched to
installed bytes, it is an unbound reference catalogue, not a qualified Frostbite schema.
The supplied `default` values and `editable` labels are source claims, not measured
runtime defaults or Studio capability declarations.

## Observed coverage

| Source type | Records | `editable=yes` | `editable=not yet` |
|---|---:|---:|---:|
| float | 15,301 | 12,143 | 3,158 |
| int | 5,117 | 4,150 | 967 |
| bool | 2,699 | 2,699 | 0 |
| curve | 820 | 0 | 820 |
| Unresolved `kind*` types | 1,537 | 0 | 1,537 |
| **Total** | **25,474** | **18,992** | **6,482** |

Unresolved types are `kind1` (1), `kind5` (21), `kind6` (1), `kind7` (8),
`kind9` (12), `kind10` (61), `kind11` (38), `kind13` (468), `kind14` (900),
and `kind15` (27). Their labels do not establish binary layouts or semantic types.

Names suggest AI positioning, skating, checking, goalie behavior, presentation,
camera timing, audio identifiers, and mode-specific configuration. This is broader
than a gameplay-slider list. Examples include `AuthenticPenaltyMultiplier`,
`AI Difficulty Adjustment Option`, `Audio Goal Horn`, and camera-transition timings.
Names support discovery categories, not proof of active consumers or gameplay effects.
Units are sometimes embedded in names (feet, MPH, 30 Hz frames) but have no dedicated
schema column. Preserve those labels and do not impose a global unit conversion.

## Data quality and interpretation

Audit method: PowerShell `Import-Csv`, ordinal name counting, and invariant-culture
numeric parsing. Numeric range checks cover `float` and `int` records only.

- **417 names occur more than once**, accounting for 1,040 records beyond the unique
  name count. Examples include `Accel` (12), `Aggressiveness` (15), and `Agility` (17).
  These may be distinct owners/contexts; do not deduplicate or select the first match.
- **27 names have leading/trailing whitespace.** Retain exact names in provenance;
  a trimmed search label must not become an identity key.
- Missing fields: 2,504 defaults, 6,023 minima, and 5,056 maxima. No names, types,
  or editability labels are blank. Missing is not zero.
- **967 integer records lack both default and minimum**, all marked `not yet`.
  Populated numeric fields parse successfully; populated integer defaults are integral.
  Boolean defaults are all the literal strings `True` or `False`.
- **655 numeric defaults are strictly outside at least one supplied bound.** This
  becomes **652** with tolerance `1e-6 * max(1, abs(default))`. For example,
  `Boost Percent ` has default `0.2` and minimum `0.20000000298023224`, illustrating
  rounding rather than a meaningful constraint violation.
- Other out-of-range examples include `Audio Goal Horn` default `-1`, bounds `0..300`,
  and `Adjust Hips Z` default `-0.2`, bounds `0..15`. Sentinel values, source metadata
  errors, display limits, and context-dependent rules are possibilities, not conclusions.
- **Four records have minimum greater than maximum:** `gInzone_Common_CloseAngle`,
  `gMinHomeTeamDeficitForEmptySeats`, `Max feet from center - focal player`, and
  `Max feet from center - focal pos`. Preserve the supplied values; do not silently
  swap endpoints, clamp defaults, or assume all bounds are ordinary linear intervals.
- Curve defaults include whitespace-separated sequences, e.g.
  `2-3 - Nobody deep - Z back up by off pressure` = `20 16 12 8 4`.
  These do not supply a complete curve definition: axes, knots, interpolation,
  ownership, and serialization remain unknown.

## Integration into project knowledge and proposed work

This source belongs under [DB-07 Game Settings and Executable Tunables](../../plans/feature-work-package-roadmap.md#db-07-game-settings-and-executable-tunables).
It also supplies candidate names for the [Frostbite implementation roadmap](../../plans/frostbite-implementation-2026-10-14.md).
It does not prove that these settings live in EBX, a database, a legacy file, or an
executable. Investigate storage before assigning a reader or writer.

1. **Establish provenance.** Obtain or locate the exporter and source title/platform/build,
   then match representative names, types, and values to that exact source. Retain the
   CSV fingerprint and original record ordinal so duplicate labels remain traceable.
2. **Read-only candidate catalogue.** Preserve raw fields alongside parsed values,
   missing-value flags, source editability, and diagnostics. Keep source claims separate
   from Studio support. Search categories may be inferred but must remain distinguishable
   from verified owner/type information. No need to implement this UI during ingestion.
3. **Resolve ownership and storage.** Bind candidates to versioned assets/object paths or
   executable records, with explicit duplicate-name disambiguation. Compare stored,
   loaded, and effective values and determine mode/update precedence.
4. **Qualify a small scalar set.** Start with located boolean/integer/float records whose
   consumers and units can be established. Follow DB-07's existing progression:
   Located → Referenced → Runtime-observed → Effect verified → Editing qualified.
   The CSV alone does not satisfy the Located stage in a target installation.
5. **Add editing only after qualification.** Use original-value checks, copied-target
   builds, restore, and controlled baseline/modified comparisons. Treat supplied ranges
   as evidence to validate, not automatic hard clamps. Keep executable patching distinct
   from ordinary asset/database writes. Defer curves and unresolved kinds until their
   complete representation and consumers are understood.

This ingestion adds evidence and proposed tasks; it does not approve implementation,
change supported-title claims, or supersede the user's review-before-code checkpoint.
