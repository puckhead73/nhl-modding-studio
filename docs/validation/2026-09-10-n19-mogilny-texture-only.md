# NHL19 Mogilny 2K diffuse isolation test

Baseline: user-confirmed successful scalp/neck mesh-only installation.
Package: `H:\TEMP\nhl19-mogilny-texture-only-2k`.
Builder: `scripts/nhl19_prepare_texture_only_test.py`.

Exactly one change: `head_jack_eichel_cm.gnf` in `game_90-008.big`.
No MCD rows or payloads, patch archives, normal maps or hair maps are modified.
The 2048x2048 BC1 diffuse is the exact payload prepared for the earlier combined
revision: SHA256 `725d8ba0947f6ff7d6dbb763f85169bc98773f8edf4507dc121c34b59147cc5d`,
2,798,848 decoded bytes and 12 mip levels. Production chunkzip/GNF checks and
the virtual EB append/unchanged-entry check passed.

Rollback restores the successful scalp mesh with its working 1024 diffuse.
Keep this package and both earlier working package folders; restoration must
proceed newest first. The original combined revision's crash is not treated as
a diagnosed GNF failure; this run isolates the texture after mesh success.

Installed after the user closed shadPS4. The installer completed full baseline
and resulting archive hashes and reported `COMPLETE apply`; receipt status is
`installed`. Production extraction of the live diffuse matches the staged SHA256.
All three live MCD payloads also match the successful mesh-only package exactly.
`live_readback.json` records these four checks. Ready for the user's texture-only
runtime test; successful loading/appearance is not yet claimed.
