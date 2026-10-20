"""Export a read-only slider-curve evidence inventory from the private port results.

This is not a UI-slider dictionary: names alone do not establish gameplay effects.
Usage: python scripts/nhl24/build_slider_evidence.py WORK_DIRECTORY
"""
import argparse
import csv
import json
from pathlib import Path


def build(work):
    source = json.loads((work / 'ported-curves.json').read_text())
    rows = []
    for record in source['items']:
        if 'slider' not in record['source']['name'].lower():
            continue
        # Preserve duplicate names and unresolved records; never select the first ambiguity.
        unique = record['status'] == 'unique_storage_match'
        match = record['matches'][0] if unique else {}
        rows.append({
            'name': record['source']['name'],
            'base_registration': record['source']['id'],
            'status': record['status'],
            'update_registration': match.get('node'),
            'update_array': match.get('array'),
            'update_runtime_pointer_slot': match.get('runtime_pointer_slot'),
            'values': match.get('values'),
            'runtime_slot_references': match.get('runtime_slot_references', []),
            'semantic_status': 'storage evidence; consumer and UI mapping require individual review',
        })
    result = {'scope': 'Named slider curves in the supplied catalogue, not every in-game slider',
              'count': len(rows), 'items': rows}
    (work / 'slider-evidence.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
    with (work / 'slider-evidence.csv').open('w', newline='', encoding='utf-8-sig') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]) if rows else [])
        writer.writeheader()
        for row in rows:
            writer.writerow({k: json.dumps(v) if isinstance(v, list) else v for k, v in row.items()})
    print(json.dumps({'records': len(rows), 'unique_storage_matches': sum(r['status'] == 'unique_storage_match' for r in rows)}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('work', type=Path)
    build(parser.parse_args().work)
