"""Wait for actual composite prerequisites, then run the exact reviewed checker once."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import traceback

ROOT = Path(__file__).resolve().parent
BASE = ROOT.parent
SOURCE_NAMES = ('PLAN.md','pins.json','common.py','positive.py','cases.py','schedule.py',
                'history.py','quiescence.py','check.py','when_ready.py','test_checker.py',
                'test_when_ready.py','review_prior_resources.py')


class Blocked(ValueError):
    pass


def require(value, message):
    if not value:
        raise Blocked(message)


def identity(path):
    require(path.is_file() and not path.is_symlink(), 'regular source/evidence file')
    with path.open('rb') as stream:
        digest = hashlib.file_digest(stream,'sha256').hexdigest()
    return dict(bytes=path.stat().st_size,sha256=digest)


def source_gate(root, manifest_sha, review_sha):
    manifest_path, review_path = root/'sources.json',root/'independent-review.json'
    require(identity(manifest_path)['sha256'] == manifest_sha, 'explicit checker source-manifest identity')
    require(identity(review_path)['sha256'] == review_sha, 'explicit independent checker review identity')
    manifest, review = json.loads(manifest_path.read_text()),json.loads(review_path.read_text())
    require(review['status'] == 'passed-source-review' and review['sources'] == manifest, 'exact independent checker review')
    require(set(manifest) == {str(root/name) for name in SOURCE_NAMES}, 'complete checker and watcher source closure')
    for name, expected in manifest.items():
        require(identity(Path(name)) == expected, 'reviewed checker source changed: '+name)
    return manifest


def prerequisites(base):
    paths = {base/f'whole-proof/attempts/c{g}-{stage}-1/receipt.json':'passed'
             for g in (1,2) for stage in ('selfbuild','fresh-verification')}
    paths.update({base/f'whole-proof-attacks-v3-c{g}/whole-c{g}/receipt.json':'passed-completion' for g in (1,2)})
    paths[base/'whole-proof-attacks-completion-v3/run-1/receipt.json']='passed'
    paths[base/'whole-proof-corpus-v2/orchestration/receipt.json']='passed'
    paths.update({base/f'whole-proof-corpus-v2/c{g}/receipt.json':'passed' for g in (2,3)})
    return paths


def observe(paths):
    states = {}
    for path, expected in paths.items():
        try:
            require(path.stat().st_size <= 16*1024**2 and not path.is_symlink(), 'bounded regular prerequisite')
            raw = path.read_bytes()
            value = json.loads(raw)
            states[str(path)] = dict(status=value['status'],required=expected,
                                    identity=dict(bytes=len(raw),sha256=hashlib.sha256(raw).hexdigest()))
        except (FileNotFoundError,json.JSONDecodeError):
            states[str(path)] = dict(status='waiting',required=expected)
    return states


def verdict(states):
    pending = {'waiting','prepared','running'}
    require(states and all(row['status'] == row['required'] or row['status'] in pending
                           for row in states.values()), 'an actual prerequisite failed; checker was not launched')
    return all(row['status'] == row['required'] for row in states.values())


def stop(child):
    if child is not None and child.poll() is None:
        os.killpg(child.pid,signal.SIGTERM)
        try:
            child.wait(timeout=30)
        except subprocess.TimeoutExpired:
            os.killpg(child.pid,signal.SIGKILL)
            child.wait(timeout=30)


def cancel(signum,_frame):
    raise InterruptedError('owned final watcher interrupted: '+str(signum))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-manifest-sha256',required=True)
    parser.add_argument('--review-sha256',required=True)
    args=parser.parse_args()
    output=ROOT/'orchestration'
    output.mkdir()
    report=dict(schema='trident/composite-final-review-watcher/v1',status='waiting',
                started_ns=time.time_ns(),command=[sys.executable,*sys.argv],cwd=os.getcwd(),
                source_manifest_sha256=args.source_manifest_sha256,independent_review_sha256=args.review_sha256,
                readiness_timeout_seconds=86400,runtime_timeout_seconds=3600,poll_seconds=30,
                stable_pass_observations_required=2,checker_launched=False)
    def save():
        pending=output/'receipt.next.json'
        pending.write_text(json.dumps(report,indent=2)+'\n')
        pending.replace(output/'receipt.json')
    child=None
    signal.signal(signal.SIGTERM,cancel)
    signal.signal(signal.SIGINT,cancel)
    began=time.monotonic()
    try:
        report['reviewed_sources']=source_gate(ROOT,args.source_manifest_sha256,args.review_sha256)
        paths=prerequisites(BASE)
        previous=None
        while True:
            states=observe(paths)
            report['observed_statuses']=states
            save()
            ready=verdict(states)
            if ready and states == previous:
                break
            previous=states if ready else None
            if time.monotonic()-began>86400:
                raise Blocked('actual prerequisite readiness deadline; checker was not launched')
            time.sleep(30)
        source_gate(ROOT,args.source_manifest_sha256,args.review_sha256)
        require(observe(paths) == states, 'stable actual prerequisite identities before launch')
        command=[sys.executable,'-B',str(ROOT/'check.py'),'--base',str(BASE),'--output',str(output/'final-review.json')]
        environment={'PATH':'','LANG':'C','PYTHONDONTWRITEBYTECODE':'1'}
        with (output/'stdout').open('xb') as out,(output/'stderr').open('xb') as err:
            child=subprocess.Popen(command,cwd=ROOT,env=environment,stdout=out,stderr=err,start_new_session=True)
            report.update(status='running-checker',checker_launched=True,checker_command=command,
                          checker_cwd=str(ROOT),checker_environment=environment,pid=child.pid)
            save()
            code=child.wait(timeout=3600)
        report['exit_code']=code
        require(code == 0 and json.loads((output/'final-review.json').read_text())['status'] == 'passed-composite-replay',
                'actual final checker did not pass')
        source_gate(ROOT,args.source_manifest_sha256,args.review_sha256)
        require(observe(paths) == states, 'actual gate receipts unchanged during final replay')
        report['status']='passed'
    except BaseException:
        report.update(status='failed-checker' if report['checker_launched'] else 'blocked-prerequisite',error=traceback.format_exc())
        try:
            save()
        except BaseException:
            report['failure_save_error']=traceback.format_exc()
        try:
            stop(child)
        except BaseException:
            report['cleanup_error']=traceback.format_exc()
        raise
    finally:
        report['ended_ns']=time.time_ns()
        report['files']={p.name:identity(p) for p in output.iterdir()
                         if p.is_file() and p.name not in ('receipt.json','receipt.next.json')}
        save()


if __name__=='__main__':
    main()
