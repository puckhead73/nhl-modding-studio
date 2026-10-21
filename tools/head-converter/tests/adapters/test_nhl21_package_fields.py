import copy,importlib.util,json,unittest
from pathlib import Path
repo=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('layout_fields',repo/'experiments/codex/nhl21_package/layout_fields.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
class NativeSourceSlotTests(unittest.TestCase):
    def setUp(self):
        self.profile=json.loads((repo/'profiles/nhl21-ps4-cusa18182-ekblad-v1.json').read_text())
    def sub(self,name):return next(s for s in self.profile['layout']['submeshes'] if s['name']==name)
    def test_combined_eyes_attributes_are_in_second_stream(self):
        f=module.resolve_fields(self.profile,self.sub('geo_eyesShape'))
        self.assertEqual((f['p0']['source_slot'],f['n0']['source_slot'],f['t0']['source_slot']),(0,1,1))
        self.assertEqual((f['n0']['data_off'],f['n0']['stride'],f['t0']['offset']),(285387,24,16))
    def test_head_attributes_remain_in_position_stream(self):
        f=module.resolve_fields(self.profile,self.sub('geo_headShape'))
        self.assertEqual((f['n0']['source_slot'],f['n0']['offset'],f['g0']['offset'],f['t0']['offset']),(0,12,20,28))
    def test_bad_source_slot_refused(self):
        self.profile['formats'][1]['declaration']=self.profile['formats'][1]['declaration'].replace('n0:00:01','n0:00:02')
        with self.assertRaises(ValueError):module.resolve_fields(self.profile,self.sub('geo_eyesShape'))
    def test_bad_attribute_extent_refused(self):
        self.profile['formats'][1]['declaration']=self.profile['formats'][1]['declaration'].replace('n0:00:01','n0:FF:01')
        with self.assertRaises(ValueError):module.resolve_fields(self.profile,self.sub('geo_eyesShape'))
if __name__=='__main__':unittest.main()
