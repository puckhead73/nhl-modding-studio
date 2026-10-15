"""Validate/copy the qualified standalone PKG helper and its complete notices."""
import hashlib
import json
from pathlib import Path
import shutil
import pe_info

PIN = Path(__file__).with_name("pkg-helper-manifest.json")


def validate(folder):
    folder = Path(folder)
    expected = json.loads(PIN.read_text(encoding="utf-8"))
    record = json.loads((folder / "BUNDLE.json").read_text(encoding="utf-8"))
    if record != expected:
        raise ValueError("PKG helper provenance differs from the qualified bundle")
    actual = {p.relative_to(folder).as_posix() for p in folder.rglob("*") if p.is_file()}
    if actual != set(expected["files"]) | {"BUNDLE.json"}:
        raise ValueError("PKG helper bundle is incomplete or contains unexpected files")
    for rel, digest in expected["files"].items():
        path = folder / rel
        if path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise ValueError(f"PKG helper file changed: {rel}")
    allowed = {"libc++.dll", "zlib1.dll"}
    imports = {}
    for name in ("pkg_extractor.exe", "libc++.dll", "zlib1.dll"):
        info = pe_info.info(str(folder / name))
        deps = pe_info.non_system(info["imports"] + info["delay_imports"])
        if {d.lower() for d in deps} - allowed:
            raise ValueError(f"Unbundled PKG helper dependencies in {name}: {deps}")
        imports[name] = deps
    return {"release": expected["release"], "source_commit": expected["source_commit"],
            "exe_sha256": expected["files"]["pkg_extractor.exe"], "runtime_imports": imports}


def add(root, bundle):
    record = validate(bundle)
    destination = Path(root) / "tools" / "pkg-extractor"
    shutil.copytree(bundle, destination)
    validate(destination)
    return record
