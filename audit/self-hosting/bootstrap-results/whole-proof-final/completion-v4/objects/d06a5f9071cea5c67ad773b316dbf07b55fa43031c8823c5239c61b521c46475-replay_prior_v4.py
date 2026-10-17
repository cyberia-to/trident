"""Read original evidence only; never run a native proof command."""
import json
from pathlib import Path
import sys
import time
import traceback
from prior_cases_v4 import review, admit_c2_cost
from prior_primitives_v4 import identity
ROOT = Path(__file__).resolve().parent
BASE = ROOT.parent

def main():
    if len(sys.argv) != 2:
        raise ValueError('one fresh output directory required')
    output = Path(sys.argv[1]).resolve()
    output.mkdir()
    sources = {str(p): identity(p) for p in sorted(ROOT.glob('prior_*v4.py'))}
    sources[str(Path(__file__).resolve())] = identity(Path(__file__).resolve())
    report = dict(status='running', command=[sys.executable, *sys.argv], cwd=str(Path.cwd()),
                  started_ns=time.time_ns(), sources=sources, operation='read-only historical evidence replay')
    try:
        for generation in (1, 2):
            proof = BASE / f'whole-proof/attempts/c{generation}-selfbuild-1/proof.joysc'
            verifier = BASE / f'whole-proof/attempts/c{generation}-fresh-verification-1'
            expected = json.loads((verifier/'stdout').read_text())['verification']
            value = review(BASE, generation, proof, verifier, expected)
            (output/f'c{generation}.json').write_text(json.dumps(value, indent=2)+'\n')
            print(f'c{generation}: 12 selected cases, 2 controls authenticated', flush=True)
        value = admit_c2_cost(BASE, proof, verifier, expected)
        (output/'c2-cost-construction.json').write_text(json.dumps(value, indent=2)+'\n')
        report['status'] = 'passed'
    except BaseException:
        report.update(status='failed', error=traceback.format_exc())
        raise
    finally:
        report['ended_ns'] = time.time_ns()
        report['sources_after'] = {name: identity(Path(name)) for name in sources}
        report['files'] = {p.name: identity(p) for p in output.iterdir() if p.is_file()}
        (output/'receipt.json').write_text(json.dumps(report, indent=2)+'\n')

if __name__ == '__main__':
    main()
