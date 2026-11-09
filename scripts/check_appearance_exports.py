"""Cross-title appearance export QA; writes only under the supplied audit directory.

Requires `cargo build -p studio-domain --example appearance_audit` and the Xbox
fixtures from stage_appearance_fixtures.py. PS4 sources are read in place.
Usage: python scripts/check_appearance_exports.py AUDIT_DIR
"""
import json
import struct
import subprocess
import sys
from pathlib import Path


def validate(root):
    counts = {"glb": 0, "dds": 0}
    for file in root.rglob("*"):
        if file.suffix not in (".glb", ".dds"):
            continue
        if file.suffix == ".glb":
            data = file.read_bytes()
            magic, version, length = struct.unpack_from("<4sII", data)
            assert magic == b"glTF" and version == 2 and length == len(data), file
            size, kind = struct.unpack_from("<II", data, 12)
            assert kind == 0x4E4F534A, file
            gltf = json.loads(data[20:20 + size])
            binary_size, binary_kind = struct.unpack_from("<II", data, 20 + size)
            assert binary_kind == 0x004E4942 and 28 + size + binary_size == len(data), file
            for view in gltf.get("bufferViews", []):
                assert view.get("byteOffset", 0) + view["byteLength"] <= binary_size, file
            counts["glb"] += 1
        else:
            # Only the DDS header is needed here; payload size comes from stat.
            with file.open("rb") as stream:
                data = stream.read(148)
            assert data[:4] == b"DDS " and struct.unpack_from("<I", data, 4)[0] == 124, file
            height, width = struct.unpack_from("<II", data, 12)
            assert width and height, file
            fourcc = data[84:88]
            header = 128
            if fourcc == b"DX10":
                header = 148
                dxgi = struct.unpack_from("<I", data, 128)[0]
                assert dxgi in (28, 71, 72, 77, 78, 80, 83, 98, 99), file
                block = None if dxgi == 28 else (8 if dxgi in (71, 72, 80) else 16)
            elif fourcc in (b"DXT1", b"DXT3", b"DXT5"):
                block = 8 if fourcc == b"DXT1" else 16
            else:
                block = None
            expected = max(1, (width + 3) // 4) * max(1, (height + 3) // 4) * block if block else width * height * 4
            assert file.stat().st_size == header + expected, (file, file.stat().st_size, header + expected)
            counts["dds"] += 1
    return counts


def main():
    output = Path(sys.argv[1]).resolve()
    repo = Path(__file__).resolve().parents[1]
    exe = repo / "target/debug/examples/appearance_audit.exe"
    titles = [(f"nhl{year:02}", "xbox", output / "xbox-fixtures" / f"nhl{year:02}-xbox360") for year in range(7, 16)]
    titles.append(("nhllegacy", "xbox", output / "xbox-fixtures/nhllegacy-xbox360"))
    cusas = ["CUSA00561", "CUSA01909", "CUSA04326", "CUSA07569", "CUSA11126", "CUSA15439", "CUSA18182"]
    titles += [(f"nhl{year}", "ps4", Path("H:/Emulators/ShadPS4/Games") / cusa) for year, cusa in zip(range(15, 22), cusas)]
    if len(sys.argv) > 2:
        titles = [title for title in titles if title[1] == sys.argv[2]]
    results_path = output / "export-results.json"
    results = json.loads(results_path.read_text()) if results_path.exists() else []
    for game, platform, source in titles:
        results = [row for row in results if (row["game"], row["platform"]) != (game, platform)]
        if platform == "xbox":
            subprocess.run([str(exe), platform, game, str(source), str(output),
                            "--inflate-fixtures-only"], check=True)
        folder = repo / "anchors" / ("xbox360" if platform == "xbox" else "ps4") / game / "databases"
        db = next((folder / name for name in ["NHL07NG.db", "nhlng.db", "nhl_game.db"] if (folder / name).exists()), None)
        command = [str(exe), platform, game, str(source), str(output)]
        if db:
            command += [str(db), str(db.with_name(db.stem + "-meta.xml"))]
        command += ["--export"]
        print(f"Exporting {platform} {game}", flush=True)
        with (output / f"{platform}-{game}.log").open("w") as log:
            result = subprocess.run(command, stdout=log, stderr=log)
        if result.returncode:
            results.append(dict(game=game, platform=platform, error=f"exit {result.returncode}"))
        else:
            summary = json.loads((output / f"{platform}-{game}-summary.json").read_text())
            try:
                counts = validate(output / f"{platform}-{game}-catalog")
                assert counts["glb"] == summary["heads_written"] + summary["hair_written"] + summary["facial_hair_written"]
                assert counts["dds"] == summary["textures_written"]
                results.append(dict(game=game, platform=platform, verified=counts,
                                    heads=summary["heads_written"], hair=summary["hair_written"],
                                    failures=summary["failures"]))
            except Exception as exc:
                results.append(dict(game=game, platform=platform, validation_error=str(exc)))
        (output / "export-results.json").write_text(json.dumps(results, indent=2))
        brief = dict(results[-1])
        if "failures" in brief:
            brief["failure_count"] = len(brief.pop("failures"))
        print(brief, flush=True)
    return int(any(row.get("error") or row.get("validation_error") or row.get("failures")
                   for row in results))


if __name__ == "__main__":
    sys.exit(main())
