# Xbox extraction engine

The Windows Xbox disc path embeds and launches the headless exiso.exe supplied alongside the user's `E:\Tools\XBOX ISO Extract\XBOX360 ISO Extract.exe`. No GUI code is used. The original tool directory is not needed at runtime.

- Reported version: extract-xiso 2.7.1 (01.11.14), win32.
- Binary SHA-256: 366fb9673ba72f3fed287fccdafe515cccfc23283cff5aa32049fbbd433df6c6.
- Invocation: separate process arguments `-q -s -x -d <destination> <image>`, no shell, hidden window.
- The supplied readme identifies exiso as its engine and skipping `$SystemUpdate` as its default. This version's skip implementation matches the case-sensitive substring `$SystemUpdate` in paths.
- Source reference: https://github.com/XboxDev/extract-xiso/blob/master/extract-xiso.c. `upstream-extract-xiso.c` is a reference snapshot retrieved 2026-10-05, not a claim of reproducible provenance for the supplied binary.
- Licence source: https://github.com/XboxDev/extract-xiso/blob/master/LICENSE.TXT. Retained in LICENSE.TXT and embedded in the Studio About screens.

This product includes software developed by in <in@fishtank.com>.

Studio validates member paths/ranges before invoking the helper, claims a fresh output directory, supports cancellation and compares extracted file sizes and bytes to the source before reporting completion. It never invokes image rewrite, image creation, source deletion, FTP or the tool's GUI. ISO9660 extraction and non-Windows hosts retain the existing native path. The source ISO is never intentionally modified. Native emulator acceptance and loose-file override precedence require separate tests.
