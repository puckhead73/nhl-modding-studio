"""Synthetic regressions for shared UV correspondence; run with Blender Python."""
import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]/'blender'))
from uv_template_match import match_templates

def square():
    return {'positions': [[0,0,0],[1,0,0],[1,1,0],[0,1,0]],
            'uv': [[0,0],[1,0],[1,1],[0,1]], 'triangles': [[0,1,2],[0,2,3]]}

class TemplateTests(unittest.TestCase):
    def test_reordered_source_ids_and_different_shape(self):
        d = square(); s = square(); order = [2,0,3,1]
        inverse = {old:new for new,old in enumerate(order)}
        s['positions'] = [[x*2,y*3,1] for x,y,z in [s['positions'][i] for i in order]]
        s['uv'] = [s['uv'][i] for i in order]
        s['triangles'] = [[inverse[i] for i in t] for t in s['triangles']]
        self.assertEqual(match_templates(d,s)['mapping'],inverse)
    def test_other_quad_diagonal_keeps_boundary_evidence(self):
        d=square();s=square();s['triangles']=[[0,1,3],[1,2,3]]
        r=match_templates(d,s)
        self.assertEqual(r['shared_triangles'],0)
        self.assertEqual(len(r['mapping']),4)
    def test_overlapping_uv_distinct_surfaces_are_rejected(self):
        d=square();s=square();s['positions'] += [[0,0,1]];s['uv'] += [[0,0]]
        s['triangles'] += [[4,1,2]]
        r=match_templates(d,s)
        self.assertIn(0,r['ambiguous']);self.assertNotIn(0,r['mapping'])
    def test_uv_rounding_and_unmatched_region(self):
        d=square();s=square();s['uv'][1][0]-=.000244
        self.assertEqual(len(match_templates(d,s)['mapping']),4)
        s['uv'][1][0]=.9
        self.assertIn(1,match_templates(d,s)['unsupported'])
    def test_invalid_geometry_refused(self):
        d=square();s=square();s['positions'][0][0]=float('nan')
        with self.assertRaises(ValueError):match_templates(d,s)
        s=square();s['triangles'][0][0]=8
        with self.assertRaises(ValueError):match_templates(d,s)

if __name__=='__main__':
    result=unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(TemplateTests))
    if not result.wasSuccessful():raise RuntimeError('UV-template regression failed')
