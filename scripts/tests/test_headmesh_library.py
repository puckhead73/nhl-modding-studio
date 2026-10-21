import importlib.util
import json
from pathlib import Path
import sqlite3
import struct
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("headmesh_library", Path(__file__).parents[1] / "headmesh_library.py")
lib = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lib)


def sample(name="head"):
    raw = struct.pack("<9f3I", 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 2)
    doc = {"asset": {"version": "2.0"},
           "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 36},
                           {"buffer": 0, "byteOffset": 36, "byteLength": 12}],
           "accessors": [{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
                         {"bufferView": 1, "componentType": 5125, "count": 3, "type": "SCALAR"}],
           "meshes": [{"name": name, "primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}]}
    return doc, raw


class LibraryTests(unittest.TestCase):
    def test_exact_geometry_not_labels_but_distinct_shape_preserved(self):
        a, raw = sample("head_123")
        b, _ = sample("geo_headShape")
        self.assertEqual(lib.pack_subset(a, raw, [0])[1], lib.pack_subset(b, raw, [0])[1])
        changed = bytearray(raw)
        struct.pack_into("<f", changed, 12, 1.001)
        self.assertNotEqual(lib.pack_subset(a, raw, [0])[1], lib.pack_subset(a, changed, [0])[1])

    def test_truncated_accessor_rejected(self):
        a, raw = sample()
        with self.assertRaises(ValueError):
            lib.pack_subset(a, raw[:-1], [0])

    def test_latest_mesh_retains_all_identity_and_texture_provenance(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            sources = []
            for game in ("nhl12", "nhl13"):
                folder = root / game
                folder.mkdir()
                doc, raw = sample()
                (folder / "head.glb").write_bytes(lib.pack_subset(doc, raw, [0])[0])
                (folder / "map.dds").write_bytes(b"DDS " + game.encode())
                entry = dict(name=f"test_player_01_02_1980_head_{game}", origin="player",
                             game=game, **{"class": "head"}, mesh_file="head.glb",
                             textures=["map.dds"], source=f"{game}/native.rx2")
                (folder / "catalog.json").write_text(json.dumps(dict(game=game, entries=[entry])))
                sources.append(dict(catalog=str(folder / "catalog.json"), platform="xbox360"))
            lib.compile_library(sources, root / "library")
            db = sqlite3.connect(root / "library/headmeshes.sqlite")
            self.assertEqual(db.execute("SELECT game FROM assets").fetchall(), [("nhl13",)])
            self.assertEqual(db.execute("SELECT count(*) FROM appearances").fetchone()[0], 2)
            self.assertEqual(len(list((root / "library/textures").rglob("*.dds"))), 2)
            db.close()


if __name__ == "__main__":
    unittest.main()
