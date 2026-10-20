"""Read NHL24's legacy files through ChunkFileCollector -> TOC chunk -> batch.

No game writes. Output is restricted to the repository's local staging directory.
Usage: python scripts/arena_library/nhl24_collectors.py
"""
import hashlib
import json
import struct
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / 'scripts/nhl27'))
from asset_manifest import part
from compatibility import Install, Oodle, frames
from riff_evidence import chunks, fixup


def toc_chunks(raw):
    b = raw[0x22c:] if raw[:4] == bytes.fromhex('00d1ce01') else raw
    w = struct.unpack('>15I', part(b, 0, 60))
    if w[0] != 60 or w[10] != w[5] * 3:
        raise ValueError('Unsupported PS4 TOC chunk profile')
    part(b, w[4], w[5] * 20)
    part(b, w[9], w[10] * 4)
    result = {}
    for i in range(w[5]):
        p = w[4] + i * 20
        ref = struct.unpack('>I', part(b, p + 16, 4))[0]
        index = ref & 0xffffff
        if ref >> 24 != 1 or index % 3 or index + 3 > w[10]:
            raise ValueError('Invalid TOC chunk reference')
        ident, offset, size = struct.unpack('>III', part(b, w[9] + index * 4, 12))
        if ident >> 24 or (ident >> 16) > 1:
            raise ValueError('Unsupported CAS identity')
        # TOC GUID bytes are reversed relative to the collector's native GUID.
        guid = part(b, p, 16)[::-1].hex()
        if guid in result:
            raise ValueError('Duplicate TOC chunk GUID')
        result[guid] = dict(layer=ident >> 16, catalog=(ident >> 8) & 255,
                            cas=ident & 255, offset=offset, size=size)
    return result


def collector_manifest_guid(raw):
    f = fixup(raw, True)
    if f['types'] != {'3e3768ac-e3bd-7d47-dad2-d75d0e2f57e8': '0x5250e107'} or f['instance_offsets'] != [16]:
        raise ValueError('Unsupported NHL24 collector schema')
    # This target schema's Manifest struct starts at EBXD+76. Do not apply
    # NHL27 field offsets: even equal type GUIDs have different signatures.
    return part(chunks(raw)['EBXD'], 76, 16).hex()


def directory(b):
    """Bounded native collector manifest, preserving batch range metadata."""
    count = struct.unpack('<I', part(b, 12, 4))[0]
    start = struct.unpack('<Q', part(b, 16, 8))[0]
    if start != 64 or count > 1000000:
        raise ValueError('Unsupported collector file table')
    part(b, start, count * 56)
    result = []
    seen = set()
    for i in range(count):
        p = start + i * 56
        name_at, compressed_start, compressed_end, offset, size = struct.unpack('<5Q', part(b, p, 40))
        if name_at < start + count * 56 or name_at >= len(b):
            raise ValueError('Collector filename outside string pool')
        end = b.find(b'\0', name_at, min(len(b), name_at + 4097))
        if end < 0:
            raise ValueError('Unterminated collector filename')
        name = b[name_at:end].decode('utf8')
        if not name or name in seen or '\\' in name or ':' in name or any(x in ('', '.', '..') for x in name.split('/')):
            raise ValueError('Duplicate or unsafe collector filename')
        if compressed_start > compressed_end or size > 256 * 1024 * 1024:
            raise ValueError('Invalid collector range')
        seen.add(name)
        result.append(dict(name=name, ordinal=i, chunk_guid=part(b, p + 40, 16).hex(),
                           offset=offset, size=size, compressed_start=compressed_start,
                           compressed_end=compressed_end))
    return result


class LegacyFiles:
    def __init__(self):
        overlay = Path('F:/Games/NHL 24/game-mods')
        # This resolver qualifies base/update only. Refuse to ignore a future
        # overlay contribution to these registries or their referenced CAS data.
        if overlay.exists() and any(p.name.lower() in ('globals.toc', 'globalsfull.toc') or
            (p.suffix.lower() == '.cas' and 'nhl_installpackage_00' in p.parts)
            for p in overlay.rglob('*') if p.is_file()):
            raise ValueError('Overlay overrides collector inputs; composite resolver required')
        self.decoder = Oodle()
        self.chunks = {}
        self.collectors = {}
        self.cache = {}
        self.tocs = []
        # Patch chunk entries override base entries; missing patch entries retain base.
        for label, layer in [('nhl24_base', 'Data'), ('nhl24_160', 'Patch')]:
            ins = Install(label)
            for sb in ('globals', 'globalsfull'):
                relative = f'{layer}/Gen4b/{sb}.toc'
                path = ins.root / relative
                if not path.exists():
                    continue
                raw = path.read_bytes()
                self.tocs.append(dict(path=str(path), sha256=hashlib.sha256(raw).hexdigest()))
                for guid, span in toc_chunks(raw).items():
                    self.chunks[guid] = dict(build=label, toc=relative, span=span)
                for bundle in ins.inventory(relative)['bundles']:
                    for a in bundle['assets']:
                        if a['kind'] == 'ebx' and 'chunkfilecollector' in a['name']:
                            self.collectors[a['name']] = dict(a, build=label, toc=relative, bundle=bundle['bundle'])
        self.files = {}
        for name, a in self.collectors.items():
            raw = Install(a['build']).read(a['span'])
            if hashlib.sha1(raw).hexdigest() != a['sha1']:
                raise ValueError('Collector stored SHA1 mismatch')
            b = frames(raw, self.decoder)[1]
            if b is None or len(b) != a['size']:
                raise ValueError('Collector decoded size mismatch')
            guid = collector_manifest_guid(b)
            for entry in directory(self.chunk(guid)):
                self.files.setdefault(entry['name'], []).append(dict(entry, collector=name, manifest_guid=guid))
        self.cache.clear()

    def chunk(self, guid):
        if guid not in self.cache:
            a = self.chunks[guid]
            raw = Install(a['build']).read(a['span'])
            b = frames(raw, self.decoder)[1]
            if b is None:
                raise ValueError('Undecoded chunk')
            # Keep a single batch resident; large UI collectors otherwise accumulate.
            self.cache = {guid: b}
        return self.cache[guid]

    def extract(self, name):
        candidates = self.files[name]
        if len({(a['chunk_guid'], a['offset'], a['size']) for a in candidates}) != 1:
            raise ValueError('Ambiguous collector file binding: ' + name)
        a = candidates[0]
        return part(self.chunk(a['chunk_guid']), a['offset'], a['size'])


def run():
    out = REPO / 'local/arena-library/nhl24-resolved'
    out.mkdir(parents=True, exist_ok=True)
    lib = LegacyFiles()
    report = dict(profile='nhl24-ps4-1.60', tocs=lib.tocs, collectors=lib.collectors,
                  files=lib.files, chunks=lib.chunks, extracted=[])
    for name in sorted(lib.files):
        if name.startswith('gamemodes/database/t3db/') or (
            any(s in name for s in ('stadium', 'arena', 'rink')) and
            not name.startswith('gamemodes/service') and name.endswith(('.json', '.csv', '.xml'))):
            b = lib.extract(name)
            path = out / 'files' / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b)
            report['extracted'].append(dict(name=name, size=len(b), sha256=hashlib.sha256(b).hexdigest()))
    (out / 'bindings.json').write_text(json.dumps(report, indent=2), encoding='utf8')
    print(json.dumps(dict(files=len(lib.files), collectors=len(lib.collectors), extracted=len(report['extracted']), output=str(out))))


if __name__ == '__main__':
    run()
