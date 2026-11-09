"""Read-only shipped appearance inventory (Xbox 360 07–Legacy, PS4 15–24).

Uses this repository's bounded archive readers; never extracts or modifies games.
Writes source/member/span provenance and errors under --output.
"""
import argparse
import collections
import json
import re
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--only", help="Comma-separated source IDs, e.g. nhl07-xbox360")
    parser.add_argument("--research-repository", type=Path, default=Path(__file__).resolve().parents[1],
                        help="Checkout containing scripts/arena_library and its documented research readers")
    args = parser.parse_args()
    import sys
    sys.path.insert(0, str(args.research_repository / "scripts"))
    from arena_library import census
    args.output.mkdir(parents=True, exist_ok=True)
    census.RELATED = re.compile(
        r"(?:rendering/(?:characters/)?(?:player|skater|coachhead|coach|referee|trainer|commissioner|timekeeper|czhead|czhair|common)/)|"
        r"(?:heads?|hair|facialhair|referee|coach|trainer|commissioner|createaplayer|genericplayer)", re.I)
    reports = []
    for source in census.source_list():
        if source["platform"] not in ("xbox360", "ps4"):
            continue
        if args.only and source["id"] not in args.only.split(","):
            continue
        report = dict(source=source, errors=[])
        counts = collections.Counter()
        print("Scanning", source["id"], flush=True)
        with (args.output / f'{source["id"]}.jsonl').open("w", encoding="utf-8") as out:
            def emit(row):
                counts[row.get("kind", "unknown")] += 1
                out.write(json.dumps(row) + "\n")
            try:
                (census.frost_scan if source["engine"] == "frostbite" else census.legacy_scan)(source, emit, report)
            except (OSError, ValueError) as exc:
                report["errors"].append(str(exc))
        report["counts"] = dict(counts)
        reports.append(report)
        census.write_json(args.output / "sources.json", reports)
        print(source["id"], counts, "errors", len(report["errors"]), flush=True)


if __name__ == "__main__":
    main()
