"""Replay checker-timeout plus cleanup failure without launching any child."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent


class SyntheticChild:
    pid = 700

    def wait(self, timeout):
        raise subprocess.TimeoutExpired('synthetic-checker-not-launched', timeout)


def replay(source, label):
    spec = importlib.util.spec_from_file_location('watcher_' + label, source)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        ready = {'synthetic-gate': dict(status='passed', required='passed', identity={'sha256': '00'})}
        with patch.object(module, 'ROOT', root), patch.object(module, 'BASE', root), \
             patch.object(module, 'source_gate', return_value={}), \
             patch.object(module, 'observe', return_value=ready), \
             patch.object(module.time, 'sleep'), \
             patch.object(module.subprocess, 'Popen', return_value=SyntheticChild()) as child, \
             patch.object(module, 'stop', side_effect=RuntimeError('synthetic cleanup failure')), \
             patch('sys.argv', ['when_ready.py', '--source-manifest-sha256', '00', '--review-sha256', '11']):
            try:
                module.main()
                exception = None
            except BaseException as error:
                exception = dict(type=type(error).__name__, message=str(error))
            if child.call_count != 1:
                raise ValueError('the synthetic checker-launch path was not exercised')
        receipt = json.loads((root/'orchestration/receipt.json').read_text())
        return dict(status=receipt['status'], checker_launched=receipt['checker_launched'],
            original_failure_preserved='synthetic-checker-not-launched' in receipt.get('error', ''),
            cleanup_failure_retained='synthetic cleanup failure' in receipt.get('cleanup_error', ''),
            propagated_exception=exception, actual_child_launched=False)


if __name__ == '__main__':
    before = replay(ROOT/'first-snapshot/when_ready.py', 'before')
    after = replay(ROOT.parent/'whole-proof-final-review-v3/when_ready.py', 'after')
    print(json.dumps(dict(before=before, after=after), indent=2))
    if (before['status'] != 'running-checker' or after['status'] != 'failed-checker'
            or not after['original_failure_preserved'] or not after['cleanup_failure_retained']
            or after['propagated_exception']['type'] != 'TimeoutExpired'):
        raise ValueError('checker timeout/cleanup regression')
