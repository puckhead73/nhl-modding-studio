"""Prepare the pinned, unmodified standalone PS4 helper with source and notices.

Run with --archive <upstream Windows zip> --out <fresh directory>. No emulator
settings or game files are touched. Only the executable and its two DLLs ship;
the upstream install_pkg.bat is deliberately not part of Studio's workflow.
"""
import argparse
import hashlib
import json
from pathlib import Path
import urllib.request
import zipfile

ZIP_SHA = "a6a6dee84c1283949245c868e580ff8d0049d0e73df9de3096460ee26c6f04d7"
REV = "771a8d58aff540448c7cbace7c40a66a5daf010d"
CRYPTO_REV = "effed0d0b865afc23ed67e0916f83734e4b9b3b7"
RELEASE = "https://github.com/AzaharPlus/shadPS4Plus/releases/tag/PKG_EXTRACTOR_1_1"
FILES = ("pkg_extractor.exe", "libc++.dll", "zlib1.dll")


def sha(data):
    return hashlib.sha256(data).hexdigest()


def prepare(archive, out):
    data = archive.read_bytes()
    if sha(data) != ZIP_SHA:
        raise ValueError("The archive is not the qualified upstream 1.1 Windows build")
    out.mkdir(parents=True, exist_ok=False)
    provenance = {}
    with zipfile.ZipFile(archive) as z:
        for name in FILES:
            matches = [p for p in z.namelist() if p.endswith("/" + name)]
            if len(matches) != 1:
                raise ValueError(f"Expected exactly one {name}")
            (out / name).write_bytes(z.read(matches[0]))

    # Include exact GPL program source and its pinned static dependency. Runtime
    # versions were identified in the shipped DLL strings, not guessed by date.
    downloads = {
        "source/shadPS4Plus.zip": f"https://codeload.github.com/AzaharPlus/shadPS4Plus/zip/{REV}",
        "source/cryptopp.zip": f"https://codeload.github.com/shadps4-emu/ext-cryptopp/zip/{CRYPTO_REV}",
        "licenses/shadPS4Plus.txt": f"https://raw.githubusercontent.com/AzaharPlus/shadPS4Plus/{REV}/LICENSE",
        "licenses/cryptopp.txt": f"https://raw.githubusercontent.com/shadps4-emu/ext-cryptopp/{CRYPTO_REV}/License.txt",
        "licenses/libcxx.txt": "https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-21.1.8/libcxx/LICENSE.TXT",
        "licenses/libcxxabi.txt": "https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-21.1.8/libcxxabi/LICENSE.TXT",
        "licenses/libunwind.txt": "https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-21.1.8/libunwind/LICENSE.TXT",
        "licenses/zlib.txt": "https://raw.githubusercontent.com/madler/zlib/v1.3.1/LICENSE",
    }
    for rel, url in downloads.items():
        request = urllib.request.Request(url, headers={"User-Agent": "NHL-Modding-Studio-release-tools"})
        with urllib.request.urlopen(request, timeout=120) as response:
            content = response.read()
        if not content:
            raise ValueError(f"Empty upstream resource: {url}")
        dest = out / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(content)
        provenance[rel] = {"url": url, "sha256": sha(content)}
        print(f"Saved {rel}: {len(content)} bytes", flush=True)
    # Check source archives contain the expected build recipe and dependency.
    with zipfile.ZipFile(out / "source/shadPS4Plus.zip") as z:
        assert any(n.endswith("/extractor/CMakeLists.txt") for n in z.namelist())
    with zipfile.ZipFile(out / "source/cryptopp.zip") as z:
        assert any(n.endswith("/GNUmakefile") for n in z.namelist())
    (out / "README.txt").write_text(
        "Standalone shadPS4Plus PKG Extractor 1.1, unmodified upstream Windows build.\n"
        f"Upstream: {RELEASE}\n"
        "Studio invokes this separate process to prepare base-game packages.\n"
        "The helper and runtime notices are in licenses/. No emulator is included.\n"
        f"Program source: source/shadPS4Plus.zip, commit {REV}.\n"
        f"Static Crypto++ source: source/cryptopp.zip, commit {CRYPTO_REV}.\n"
        "To reconstruct the source tree, unpack shadPS4Plus.zip, then unpack the\n"
        "contents of cryptopp.zip's top folder into externals/cryptopp/.\n"
        "The upstream build instructions are extractor/CMakeLists.txt and\n"
        "extractor/build/{buildall,lconf,cryptobuild,lbuild,wpack}. Windows uses\n"
        "MSYS2 clang64; the shipped DLLs identify LLVM 21.1.8 and zlib 1.3.1.\n"
        "This bundle preserves upstream binaries; it is not a reproducible-build claim.\n",
        encoding="utf-8")
    record = {"release": RELEASE, "upstream_zip_sha256": ZIP_SHA,
              "source_commit": REV, "cryptopp_commit": CRYPTO_REV,
              "downloads": provenance,
              "files": {p.relative_to(out).as_posix(): sha(p.read_bytes())
                        for p in sorted(out.rglob("*")) if p.is_file()}}
    (out / "BUNDLE.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    prepare(args.archive, args.out)
