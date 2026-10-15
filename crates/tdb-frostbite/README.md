# NHL Frostbite container reader

Read-only NHL 22–24 PS4 (`Gen4b`) structures: D1CE01 payloads, DbObjects,
binary superbundle indexes, Huffman bundle names, mixed-endian bundle manifests,
absolute Data/Patch CAS spans and block framing. Unknown variants are refused.
No vendor binaries or third-party tool source are included.

```powershell
cargo test -p tdb-frostbite --offline
cargo run -p tdb-frostbite --bin frostbite-inspect -- 'H:/Emulators/ShadPS4/Games/CUSA26280' --audit
cargo run -p tdb-frostbite --bin frostbite-inspect -- 'H:/Emulators/ShadPS4/Games/CUSA37933' --names
```

`--audit` reads manifests in CAS offset order, validates manifest/span counts and
CAS bounds, and prints counts. It does not decode every asset or establish runtime
patch precedence. `--names` checks unique names and corresponding base-name matches.

`install` exposes inspection, paginated bundle/chunk queries, manifest assets and
non-overwriting exports outside the source root. Raw exports retain CAS framing;
decoded exports support stored blocks only. `blocks::decode_with` is the bounded
extension point for a future codec provider. Index reads are capped at 128 MiB,
individual spans/decoded assets at 256 MiB and DbObject recursion at 64 levels.

The app route `/frostbite` is reachable by selecting NHL 22–24 in the project picker.
See [findings and remaining work](../../docs/plans/frostbite-implementation-2026-10-14.md).
