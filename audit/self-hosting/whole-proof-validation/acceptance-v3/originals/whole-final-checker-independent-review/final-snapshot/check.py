"""Replay explicit composite local SH8 evidence; no compiler/verifier execution."""
import argparse
import datetime
import importlib.util
import json
from pathlib import Path
import sys
import traceback
from common import ROOT, C, identity, require, v3_pins
import positive
import cases
import schedule
import history


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), 'fresh review destination required')
    base = args.base.resolve()
    report = dict(schema='trident/proved-self-build-local-review/v3', status='running',
                  command=[sys.executable, *sys.argv],
                  checker_sources={p.name: identity(p) for p in sorted(ROOT.glob('*.py'))},
                  source_pins=identity(ROOT/'pins.json'), generations=[],
                  scope='Composite local SH8 evidence replay: failed v2 attempt remains failed; original case/control executions are explicitly reused. Public complete-witness relation; no succinctness, zero knowledge, language-semantics preservation, durable transport or release publication claim.')
    try:
        pins = v3_pins(base)
        helper = base / 'whole-acceptance/trident/audit/self-hosting/bootstrap-runner.py'
        require(identity(helper)['sha256'] == C.PINS[str(helper.relative_to(base))], 'pinned corpus helper')
        spec = importlib.util.spec_from_file_location('retained_bootstrap', helper)
        bootstrap = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bootstrap)
        report['original_failed_schedule_resource_replay'] = history.prior_schedule(base)
        report['completion_schedule'] = schedule.review(base)
        for number in (1, 2):
            result = positive.review(base, number, bootstrap)
            result['composite_cases'] = cases.review(base, number, result, pins)
            report['generations'].append(result)
        require(report['generations'][0]['compiler'] == report['generations'][1]['compiler'], 'C2 equals C3')
        report['status'] = 'passed-composite-replay'
    except BaseException:
        report.update(status='failed', error=traceback.format_exc())
        raise
    finally:
        report['completed_utc'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
        with args.output.open('x') as stream:
            json.dump(report, stream, indent=2)
            stream.write('\n')
    print(json.dumps(dict(status=report['status'], generations=len(report['generations']))))


if __name__ == '__main__':
    main()
