"""Run with Blender --background --python this.py."""
import sys, unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'blender'))
from mathutils import Vector
from head_scalp_placement import place_head

class PlacementTests(unittest.TestCase):
    def test_long_head_keeps_attachment_and_moves_face_rigidly(self):
        old=list(map(Vector,[(0,0,0),(0,0,1),(0,0,2),(0,0,3)]))
        incoming=list(map(Vector,[(.1,0,-.1),(.2,0,1.2),(.3,0,2.2),(.4,0,3.2)]))
        result,shift,report=place_head(old,incoming,.5,1.5)
        self.assertEqual(result[0],old[0])
        self.assertAlmostEqual(max(p.z for p in result),3)
        self.assertAlmostEqual(shift.z,-.2,places=6)
        for i in (2,3):self.assertLess((result[i]-incoming[i]-shift).length,1e-6)
        self.assertEqual(report['frozen_native_ids'],[0])
        self.assertAlmostEqual(result[1].x,.1,places=6)

    def test_short_head_moves_up(self):
        old=[Vector((0,0,0)),Vector((0,0,3))]
        result,shift,_=place_head(old,[Vector((0,0,0)),Vector((0,0,2))],.5,1.5)
        self.assertEqual(shift.z,1)
        self.assertEqual(result,old)

if __name__=='__main__':
    outcome=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(PlacementTests))
    if not outcome.wasSuccessful():raise RuntimeError('placement tests failed')
