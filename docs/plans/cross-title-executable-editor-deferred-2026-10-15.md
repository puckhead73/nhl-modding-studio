# Deferred cross-title executable/editor research

Saved at the user's request to retain the earlier plan while pivoting to NHL 27 → NHL 24 backports. Do not treat this document as evidence that every supported title has been researched. The current pivot is documented in `docs/research/nhl27-backport-2026-10-15.md`.

Inventory each Studio-supported title/platform/build from adapter/discovery definitions and user-owned installations. Hash every eboot.bin/default.xex; distinguish base and updates. Identify executable format, architecture, compression/encryption, load map, relocations and available unwind/symbol information. Use existing REA/Ghidra and repository tools where applicable. Preserve installations; work on analysis copies.

Research, in user priority order: league size (up to 48); additional leagues; safe 8–10-skater rosters; full league control in Franchise/Dynasty; historical rules; new leagues in career modes; strategies; AI; pressure systems; attribute influence. Also cover heads/portraits, every gameplay slider, offline Ones participants, hit attribution, takeaway/giveaway attribution, game simulation, player/jersey/equipment/team/league/looks capacities, and performance-selected All-Star games in Season/Dynasty.

For each topic build an evidence chain from database/schema or registration through executable consumer, save representation and gameplay/simulation effect. Names, constants and field widths are leads rather than complete functionality. Record source evidence, build-specific addresses, semantics, dependencies, confidence, reversible patch design and unsupported cases. Do not reuse PS4 x86 addresses or structures for Xbox 360 PowerPC XEX builds.

Deliver per-title dossiers and machine-readable build profiles; slider/attribute descriptions grounded in code; qualified patch recipes; rollback and conflict handling; runtime boundary/save tests; and Studio editor contracts. Keep discovery controls read-only until write paths are independently qualified. A partial profile is valid when its unsupported areas are explicit; it must not imply complete title support.
