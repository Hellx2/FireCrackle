import importlib.util
from pathlib import Path
import unittest

spec=importlib.util.spec_from_file_location("actual_quad",Path(__file__).with_name("prepare-actual-quad-eval.py"))
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)


class ActualGeometryTest(unittest.TestCase):
    def item(self):
        return dict(c=[-20,10,-10,100,100,110,80,5],uv=[0,0,0,1,1,1,1,0],m=0)

    def test_preserves_offscreen_corners_and_orders_by_texture_vertical_axis(self):
        q,error=module.canonical_quad(self.item(),100,100)
        self.assertIsNone(error)
        self.assertEqual(q,[(-20,10),(80,5),(100,110),(-10,100)])

    def test_mirror_is_not_used_to_choose_geometry(self):
        normal=self.item();mirrored=self.item();mirrored.update(uv=[1,0,1,1,0,1,0,0],m=1)
        self.assertEqual(module.canonical_quad(normal,100,100),module.canonical_quad(mirrored,100,100))

    def test_rotated_texture_axis_keeps_vertex_correspondence(self):
        item=self.item();item['c']=[v for x,y in zip(item['c'][::2],item['c'][1::2]) for v in [-y,x]]
        q,error=module.canonical_quad(item,100,100)
        self.assertIsNone(error)
        self.assertEqual(q,[(-10,-20),(-5,80),(-110,100),(-100,-10)])

    def test_rejects_backface_instead_of_reversing_it(self):
        item=self.item();item['c']=[-20,10,80,5,100,110,-10,100]
        self.assertEqual(module.canonical_quad(item,100,100),(None,"backface_or_small"))

    def test_extreme_projection_is_skipped_without_clamping(self):
        item=self.item();item['c'][0]=-100000
        self.assertEqual(module.canonical_quad(item,100,100),(None,"extreme_projection"))

    def test_missing_and_nonfinite_geometry_are_skipped(self):
        item=self.item();item['c']=None
        self.assertEqual(module.canonical_quad(item,100,100),(None,"missing"))
        item=self.item();item['c'][0]=float('nan')
        self.assertEqual(module.canonical_quad(item,100,100),(None,"nonfinite"))


if __name__=="__main__":unittest.main()
