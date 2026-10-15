"""Report first-person overlay acceptance at a fixed, explicit presence threshold.

Usage: python bench/overlay_analysis.py data/joined/overlay5_clip1_h.csv ...
Consumes fire-overlay CSVs; labels are used only for scoring, never decisions.
Each input is reported separately; this tool never tunes a threshold on a clip.
"""
import argparse
import csv
import json
import math

REQUIRED = ('n', 'has_overlay', 'truth_row', 'fin_row', 'fin_ev', 'vis', 'veil')


def load(path):
    with open(path, newline='', encoding='utf-8-sig') as stream:
        reader = csv.DictReader(stream)
        missing = set(REQUIRED) - set(reader.fieldnames or [])
        if missing:
            raise ValueError(f'{path}: missing columns: {sorted(missing)}')
        return [{key: float(row[key]) for key in REQUIRED} for row in reader]


def ratio(n, d):
    return n / d if d else None


def _metrics(rows, threshold):
    pos = [r for r in rows if r['has_overlay'] == 1]
    neg = [r for r in rows if r['has_overlay'] == 0]
    labelled = [r for r in pos if r['truth_row'] >= 0]
    tp = sum(r['fin_ev'] >= threshold for r in pos)
    fp = sum(r['fin_ev'] >= threshold for r in neg)
    correct = sum(r['fin_row'] == r['truth_row'] for r in labelled)
    detected_correct = sum(r['fin_row'] == r['truth_row'] and r['fin_ev'] >= threshold for r in labelled)
    return dict(
        frames=len(rows),
        presence=dict(tp=tp, fn=len(pos)-tp, fp=fp, tn=len(neg)-fp,
                      recall=ratio(tp, len(pos)), false_positive_rate=ratio(fp, len(neg))),
        rows=dict(correct=correct, total=len(labelled), accuracy=ratio(correct, len(labelled))),
        end_to_end=dict(correct=detected_correct, total=len(labelled),
                       accuracy=ratio(detected_correct, len(labelled))))


def analyze(rows, threshold=0.5):
    if not math.isfinite(threshold):
        raise ValueError('threshold must be finite')
    rows = list(rows)
    seen = set()
    for r in rows:
        if any(key not in r or not math.isfinite(r[key]) for key in REQUIRED):
            raise ValueError('missing or nonfinite required value')
        if r['n'] < 0 or int(r['n']) != r['n'] or r['n'] in seen:
            raise ValueError('frame numbers must be unique nonnegative integers')
        seen.add(r['n'])
        if r['has_overlay'] not in (0, 1):
            raise ValueError('has_overlay must be 0 or 1')
        if r['truth_row'] not in range(-1, 32) or r['fin_row'] not in range(32):
            raise ValueError('texture rows must be integers 0..31 (truth may be -1)')
        if not 0 <= r['vis'] <= 1 or not 0 <= r['veil'] <= 1:
            raise ValueError('visibility and veil must be in [0, 1]')
    result = _metrics(rows, threshold)
    result['threshold'] = threshold
    pos = [r for r in rows if r['has_overlay']]
    result['groups'] = {
        name: _metrics([r for r in pos if predicate(r)], threshold)
        for name, predicate in (
            ('visibility_lt_0.15', lambda r: r['vis'] < 0.15),
            ('visibility_0.15_to_0.3', lambda r: 0.15 <= r['vis'] < 0.3),
            ('visibility_ge_0.3', lambda r: r['vis'] >= 0.3),
            ('veiled', lambda r: r['veil'] >= 0.6),
            ('unveiled', lambda r: r['veil'] < 0.6))}
    result['false_negatives'] = [int(r['n']) for r in pos if r['fin_ev'] < threshold]
    result['false_positives'] = [int(r['n']) for r in rows if not r['has_overlay'] and r['fin_ev'] >= threshold]
    result['row_errors'] = [
        dict(n=int(r['n']), truth=int(r['truth_row']), predicted=int(r['fin_row']),
             offset=int((r['fin_row'] - r['truth_row'] + 16) % 32 - 16),
             visibility=r['vis'], veil=r['veil'], evidence=r['fin_ev'],
             detected=r['fin_ev'] >= threshold)
        for r in pos if r['truth_row'] >= 0 and r['fin_row'] != r['truth_row']]
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('csv', nargs='+')
    parser.add_argument('--threshold', type=float, default=0.5,
                        help='fixed fin_ev threshold (default: 0.5); no automatic tuning')
    args = parser.parse_args()
    try:
        reports = [dict(source=path, **analyze(load(path), args.threshold)) for path in args.csv]
    except (ValueError, OSError, TypeError) as error:
        parser.error(str(error))
    print(json.dumps(reports, indent=2, allow_nan=False))


if __name__ == '__main__':
    main()
