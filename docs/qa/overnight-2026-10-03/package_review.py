"""Package the completed October 4 review build without publishing it."""
from pathlib import Path
import datetime
import hashlib
import importlib.util
import json
import subprocess
import sys
import argparse

repo = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--attempt", type=int, default=1)
parser.add_argument("--build-log", default="target/overnight-20261003/review-release-build-resumed.log",
                    help="Log of the completed build being packaged, relative to the repository")
args = parser.parse_args()
assert args.attempt > 0
evidence = repo / f"target/overnight-20261003/review-package-{args.attempt}"
evidence.mkdir(exist_ok=False)
build_log = repo / args.build_log
assert "Built application at:" in build_log.read_text(encoding="utf-8-sig")
spec = importlib.util.spec_from_file_location("manifest", repo / "tools/review-build/manifest.py")
manifest = importlib.util.module_from_spec(spec)
spec.loader.exec_module(manifest)
manifest.inputs(str(evidence / "dirty-inputs-after-build.tsv"))
exe = repo / "target/release/nhl-studio.exe"
helper = Path("E:/nhlms-review-build-r73-2026-10-02/NHL Modding Studio rc-20261002-r73")
name = "NHL Modding Studio QA 20261004"
record = {
    "candidate": name,
    "recorded_at": datetime.datetime.now().astimezone().isoformat(),
    "command": "npm run build -- --no-bundle",
    "build_log_sha256": manifest.sha(build_log),
    "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip(),
    "dirty_manifest_sha256": manifest.sha(evidence / "dirty-inputs-after-build.tsv"),
    "source_manifest_timing": "after completed build; no before-build equality claim",
    "exe_sha256": manifest.sha(exe),
    "exe_bytes": exe.stat().st_size,
    "helper_exe_sha256": manifest.sha(helper / "helpers/nhlms-xma2-helper.exe"),
    "ffmpeg_source_sha256": "7138d28c96d9d3e3af4ee3d8cad72741f8ffb40da90c1112235dea3ecd3178a3",
    "included_receipts": "docs/qa/overnight-2026-10-03; supported games through NHL21. Gameplay verification incomplete.",
    "launched": False,
    "published": False,
}
for folder, key in [(repo / "defs", "defs"), (repo / "app/dist", "frontend_dist"),
                    (repo / "anchors", "anchors"), (helper / "helpers", "helpers")]:
    path = evidence / f"{key}-tree.tsv"
    manifest.tree(str(folder), str(path))
    rows = path.read_text(encoding="utf-8").splitlines()[1:]
    record[key + "_tree_sha256"] = hashlib.sha256("".join(
        r.split("\t")[0] + "\t" + r.split("\t")[-1] + "\n" for r in rows).encode()).hexdigest()
record_path = evidence / "build-record-input.json"
record_path.write_text(json.dumps(record, indent=2), encoding="utf-8")
subprocess.run([
    sys.executable, str(repo / "tools/release-archive/assemble.py"),
    "--exe", str(exe), "--helper-bundle", str(helper),
    "--out", str(repo / f"target/internal-candidates/nhlms-release-candidate-20261004-qa{args.attempt}"),
    "--name", name, "--build-record", str(record_path),
    "--corpus", str(repo / "corpus/historical.db"),
    "--webview-runtime", str(repo / "target/playtester-downloads/runtime/Microsoft.WebView2.FixedVersionRuntime.154.0.4258.53.x64"),
    "--pkg-helper-bundle", str(repo / "target/overnight-20261003/tools/pkg-helper-bundle-1"),
], cwd=repo, check=True)
