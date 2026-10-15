import unittest

from overlay_analysis import analyze, load


def frame(n, has=1, truth=4, predicted=4, evidence=0.8, vis=0.5, veil=0):
    return dict(n=n, has_overlay=has, truth_row=truth, fin_row=predicted,
                fin_ev=evidence, vis=vis, veil=veil)


class OverlayAnalysisTests(unittest.TestCase):
    def test_presence_threshold_is_inclusive_and_rows_do_not_count_as_detection(self):
        r = analyze([frame(0, evidence=0.5), frame(5, evidence=0.49),
                     frame(10, has=0, truth=-1, evidence=0.5),
                     frame(15, has=0, truth=-1, evidence=-1)])
        self.assertEqual(r['presence'], dict(tp=1, fn=1, fp=1, tn=1, recall=0.5, false_positive_rate=0.5))
        self.assertEqual(r['rows']['correct'], 2)
        self.assertEqual(r['end_to_end']['correct'], 1)

    def test_wrong_row_is_separate_from_missed_presence(self):
        r = analyze([frame(0, predicted=5), frame(5, evidence=0.1)])
        self.assertEqual(r['rows']['accuracy'], 0.5)
        self.assertEqual(r['end_to_end']['accuracy'], 0)
        self.assertEqual(r['row_errors'][0]['offset'], 1)
        self.assertEqual(r['false_negatives'], [5])

    def test_circular_offsets_wrap_at_32(self):
        r = analyze([frame(0, truth=31, predicted=0), frame(5, truth=0, predicted=31)])
        self.assertEqual([e['offset'] for e in r['row_errors']], [1, -1])

    def test_unknown_row_still_counts_for_presence_but_not_row_accuracy(self):
        r = analyze([frame(0, truth=-1), frame(5)])
        self.assertEqual(r['presence']['tp'], 2)
        self.assertEqual(r['rows']['total'], 1)

    def test_empty_denominators_are_none(self):
        r = analyze([])
        self.assertIsNone(r['presence']['recall'])
        self.assertIsNone(r['presence']['false_positive_rate'])
        self.assertIsNone(r['rows']['accuracy'])

    def test_visibility_and_veil_groups_keep_difficult_frames(self):
        r = analyze([frame(0, vis=0), frame(5, vis=0.15, veil=0.6),
                     frame(10, vis=0.3), frame(15, has=0, truth=-1)])
        self.assertEqual(r['groups']['visibility_lt_0.15']['rows']['total'], 1)
        self.assertEqual(r['groups']['visibility_0.15_to_0.3']['rows']['total'], 1)
        self.assertEqual(r['groups']['visibility_ge_0.3']['rows']['total'], 1)
        self.assertEqual(r['groups']['veiled']['rows']['total'], 1)

    def test_rejects_duplicate_frames_and_nonfinite_evidence(self):
        for rows in ([frame(0), frame(0)], [frame(0, evidence=float('nan'))],
                     [frame(0, evidence=float('inf'))], [frame(0, truth=32)],
                     [frame(0, predicted=1.5)], [frame(0, vis=-0.1)]):
            with self.subTest(rows=rows), self.assertRaises(ValueError):
                analyze(rows)

    def test_fixed_threshold_changes_only_presence_decision(self):
        rows = [frame(0, evidence=0.4)]
        self.assertEqual(analyze(rows, threshold=0.3)['end_to_end']['correct'], 1)
        self.assertEqual(analyze(rows, threshold=0.5)['end_to_end']['correct'], 0)

    def test_load_requires_current_schema(self):
        import tempfile
        from pathlib import Path
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / 'scores.csv'
            p.write_text('n,has_overlay,truth_row,fin_row,fin_ev,vis,veil\n0,1,4,4,0.8,0.5,0\n')
            self.assertEqual(analyze(load(p))['rows']['accuracy'], 1)
            p.write_text('n,has_overlay\n0,1\n')
            with self.assertRaises(ValueError):
                load(p)


if __name__ == '__main__':
    unittest.main()
