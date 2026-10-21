import json
from contextlib import closing
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from append_headmesh_catalogs import append_library
from headmesh_library import compile_library, pack_subset
from test_headmesh_library import sample


class AppendTests(unittest.TestCase):
    def test_patch_texture_provenance_survives_geometry_dedup_and_repeat_is_idempotent(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            catalogs = []
            for layer in ('Data', 'Patch'):
                game = 'nhl26' if layer == 'Data' else 'nhl27'
                folder = root / layer
                folder.mkdir()
                doc, raw = sample()
                (folder / 'head.glb').write_bytes(pack_subset(doc, raw, [0])[0])
                (folder / 'color.dds').write_bytes(b'DDS ' + layer.encode())
                entry = dict(name='example_player_01_02_1980', origin='player', game=game,
                             mesh_file='head.glb', textures=['color.dds'], source='native/head', **{'class':'head'})
                (folder / 'catalog.json').write_text(json.dumps(dict(game=game, layer=layer, entries=[entry])))
                catalogs.append({'catalog': str(folder / 'catalog.json'), 'platform':'ps5'})
            compile_library(catalogs[:1], root / 'base')
            before = (root / 'base/headmeshes.sqlite').read_bytes()
            append_library(root / 'base', catalogs[1:], root / 'updated')
            self.assertEqual(before, (root / 'base/headmeshes.sqlite').read_bytes())
            with closing(sqlite3.connect(root / 'updated/headmeshes.sqlite')) as db:
                self.assertEqual(db.execute('SELECT count(*) FROM assets').fetchone()[0], 1)
                self.assertEqual(db.execute('SELECT game FROM assets').fetchone()[0], 'nhl27')
                rows = db.execute('SELECT entry_json,textures_json FROM appearances').fetchall()
                self.assertEqual(len(rows), 2)
                self.assertEqual(json.loads(rows[1][0])['layer'], 'Patch')
                self.assertNotEqual(rows[0][1], rows[1][1])
            append_library(root / 'updated', catalogs[1:], root / 'again')
            with closing(sqlite3.connect(root / 'again/headmeshes.sqlite')) as db:
                self.assertEqual(db.execute('SELECT count(*) FROM appearances').fetchone()[0], 2)


if __name__ == '__main__':
    unittest.main()
