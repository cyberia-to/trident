"""Bound source/test preparation; no proof evaluator or mutation command."""
import json
from pathlib import Path
import subprocess
import sys
import time
from prior_primitives_v4 import identity
ROOT=Path(__file__).resolve().parent
NAMES=('prior_cases_v4.py','prior_primitives_v4.py','prior_replay_v4.py','prior_quiescence_v4.py',
       'prior_fixture_v4.py','test_prior_cases_v4.py','test_prior_cost_v4.py','test_prior_quiescence_v4.py',
       'replay_prior_v4.py','validate_prior_v4.py','PRIOR-ADMISSION.md')

def main():
    if len(sys.argv)!=2:raise ValueError('one fresh validation directory')
    output=Path(sys.argv[1]).resolve();output.mkdir()
    before={str(ROOT/name):identity(ROOT/name) for name in NAMES}
    command=[sys.executable,'-B','-W','error','-m','unittest','test_prior_cases_v4','test_prior_cost_v4',
             'test_prior_quiescence_v4','-v']
    started=time.time_ns()
    with (output/'stdout').open('xb') as stdout,(output/'stderr').open('xb') as stderr:
        result=subprocess.run(command,cwd=ROOT,stdout=stdout,stderr=stderr,timeout=60)
    after={path:identity(Path(path)) for path in before}
    passed=result.returncode==0 and before==after
    report=dict(status='passed' if passed else 'failed',command=command,cwd=str(ROOT),
                python=identity(Path(sys.executable).resolve()),started_ns=started,ended_ns=time.time_ns(),
                exit_code=result.returncode,source_before=before,source_after=after,
                files={name:identity(output/name) for name in ('stdout','stderr')},
                scope='Offline synthetic tests only; all original proof/evidence inputs read-only.')
    (output/'receipt.json').write_text(json.dumps(report,indent=2)+'\n')
    print(report['status'])
    if not passed:raise SystemExit(1)

if __name__=='__main__':main()
