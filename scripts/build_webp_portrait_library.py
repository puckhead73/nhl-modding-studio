"""Build a verified, resumable lossless WebP library without changing PNG masters.

Requires Pillow with WebP support. Run with --source, --destination, --workers.
The journal is append-only; incomplete runs never publish a completed manifest.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import csv
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import time
from PIL import Image, features

def digest(data):
    return hashlib.sha256(data).hexdigest()

def atomic_json(path, value):
    temp = path.with_suffix('.tmp')
    temp.write_text(json.dumps(value, indent=2), encoding='utf-8')
    temp.replace(path)

def build(source, destination, workers):
    source, destination = source.resolve(), destination.resolve()
    assert source != destination and source not in destination.parents
    assert features.check('webp'), 'Pillow WebP support is required'
    destination.mkdir(parents=True, exist_ok=True)
    audit = destination / '_build'
    audit.mkdir(exist_ok=True)
    lock = audit / 'running.lock'
    # An interrupted run requires removal of this lock after confirming its PID stopped.
    with lock.open('x') as f:
        f.write(str(os.getpid()))
    started = time.time()
    journal = audit / 'verified_images.jsonl'
    progress = audit / 'progress.json'
    try:
        files = sorted(source.rglob('*.png'))
        assert files, 'No PNGs found'
        manifest = source / 'portrait_sources.csv'
        manifest_hash = digest(manifest.read_bytes())
        previous = {}
        if journal.exists():
            for line in journal.read_text(encoding='utf-8').splitlines():
                try:
                    row = json.loads(line)
                    previous[row['source_file']] = row
                except json.JSONDecodeError:
                    pass  # A power loss may leave a partial last journal entry.
        records = {}
        errors = []

        def convert(path):
            relative = path.relative_to(source).as_posix()
            output = destination / Path(relative).with_suffix('.webp')
            raw = path.read_bytes()
            source_hash = digest(raw)
            old = previous.get(relative)
            if old and old['source_sha256'] == source_hash and output.exists() and digest(output.read_bytes()) == old['webp_sha256']:
                return old
            with Image.open(io.BytesIO(raw)) as image:
                rgba = image.convert('RGBA')
                expected = rgba.tobytes()
                if output.exists() and not old:
                    # Recover verified outputs written just before a journal checkpoint.
                    with Image.open(output) as existing:
                        assert existing.size == rgba.size and existing.convert('RGBA').tobytes() == expected, f'Untracked output differs: {output}'
                metadata = {k: image.info[k] for k in ['icc_profile', 'exif', 'xmp'] if k in image.info}
                encoded = io.BytesIO()
                rgba.save(encoded, 'WEBP', lossless=True, quality=100, method=6, exact=True, **metadata)
                data = encoded.getvalue()
                with Image.open(io.BytesIO(data)) as decoded:
                    assert decoded.size == rgba.size and decoded.convert('RGBA').tobytes() == expected, relative
                width, height = rgba.size
            output.parent.mkdir(parents=True, exist_ok=True)
            temporary = output.with_suffix('.partial')
            temporary.write_bytes(data)
            assert digest(temporary.read_bytes()) == digest(data)
            temporary.replace(output)
            return {'source_file': relative, 'portrait_file': output.relative_to(destination).as_posix(),
                    'source_sha256': source_hash, 'webp_sha256': digest(data),
                    'rgba_sha256': digest(expected), 'source_bytes': len(raw), 'webp_bytes': len(data),
                    'width': width, 'height': height}

        def safe_convert(path):
            try:
                return convert(path), None
            except Exception as exc:
                return None, {'file': str(path), 'error': f'{type(exc).__name__}: {exc}'}

        print(f'Converting {len(files):,} portraits using {workers} workers.', flush=True)
        with ThreadPoolExecutor(max_workers=workers) as pool, journal.open('a', encoding='utf-8') as log:
            # Bound queued work and checkpoint each batch.
            for offset in range(0, len(files), 128):
                for row, error in pool.map(safe_convert, files[offset:offset+128]):
                    if error:
                        errors.append(error)
                    else:
                        records[row['source_file']] = row
                        log.write(json.dumps(row) + '\n')
                log.flush()
                elapsed = time.time()-started
                state = {'status':'running', 'verified':len(records), 'total':len(files), 'errors':errors,
                         'elapsed_seconds':elapsed, 'source_bytes':sum(r['source_bytes'] for r in records.values()),
                         'webp_bytes':sum(r['webp_bytes'] for r in records.values())}
                atomic_json(progress, state)
                print(f"Verified {len(records):,}/{len(files):,}; errors {len(errors)}; elapsed {elapsed/60:.1f} min", flush=True)
        assert not errors, f'{len(errors)} conversion errors; see progress.json'
        assert len(records) == len(files)
        assert digest(manifest.read_bytes()) == manifest_hash, 'Source manifest changed during conversion'
        csv.field_size_limit(64*1024*1024)
        updated = destination / 'portrait_sources.csv.partial'
        seen = set()
        missing_sources = []
        with manifest.open(encoding='utf-8-sig', newline='') as fi, updated.open('w', encoding='utf-8-sig', newline='') as fo:
            reader = csv.DictReader(fi)
            extra = ['distribution_source_png', 'distribution_source_png_sha256', 'distribution_webp_sha256', 'distribution_rgba_sha256', 'distribution_encoding']
            writer = csv.DictWriter(fo, fieldnames=list(reader.fieldnames)+extra)
            writer.writeheader()
            for row in reader:
                name = row['portrait_file'].replace('\\','/')
                if name not in records:
                    missing_sources.append(row)
                    continue
                assert name in records and name not in seen, f'Manifest mismatch: {name}'
                seen.add(name)
                record = records[name]
                row.update(portrait_file=record['portrait_file'], distribution_source_png=name,
                           distribution_source_png_sha256=record['source_sha256'],
                           distribution_webp_sha256=record['webp_sha256'], distribution_rgba_sha256=record['rgba_sha256'],
                           distribution_encoding='WebP lossless; exact=True; quality=100; method=6')
                if 'representative_sha256' in row:
                    row['representative_sha256'] = record['webp_sha256']
                # Historical source filenames/hashes and game/mod provenance stay unchanged.
                writer.writerow(row)
        assert seen == set(records), 'Images missing from source manifest'
        shutil.copy2(manifest, audit / 'portrait_sources.original.csv')
        atomic_json(audit / 'missing_source_entries.json', missing_sources)
        updated.replace(destination / 'portrait_sources.csv')
        state.update(status='complete', elapsed_seconds=time.time()-started, manifest_rows=len(seen), exact_rgba_verified=True, missing_source_entries=len(missing_sources))
        atomic_json(progress, state)
        atomic_json(destination / 'build_summary.json', state)
        print(json.dumps(state, indent=2), flush=True)
    except BaseException as exc:
        atomic_json(audit/'failure.json', {'error':repr(exc), 'time':time.time()})
        raise
    finally:
        lock.unlink(missing_ok=True)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--destination', type=Path, required=True)
    parser.add_argument('--workers', type=int, default=12)
    args = parser.parse_args()
    build(args.source, args.destination, args.workers)
