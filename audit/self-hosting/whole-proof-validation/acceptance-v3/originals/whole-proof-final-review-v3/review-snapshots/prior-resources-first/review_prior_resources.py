"""Read only the actual original v2 resource evidence; no composite acceptance."""
import json
from pathlib import Path
import sys
import time
import traceback
import cases
import history
from common import ROOT, C, identity, load, require


def main():
    output = ROOT/'actual-prior-resources-1.json'
    require(not output.exists(), 'fresh prior-resource review receipt')
    base = ROOT.parent
    report = dict(scope='Read-only replay of actual failed v2 resource evidence; no v3 execution or SH8 acceptance',
                  command=[sys.executable,'-B',str(Path(__file__).resolve())], started_ns=time.time_ns(),
                  sources={p.name:identity(p) for p in ROOT.glob('*.py')}, status='running', commands=[])
    try:
        for name, expected in C.PINS.items():
            require(identity(base/name)['sha256'] == expected, 'original reviewed source pin')
        report['aggregate'] = history.prior_schedule(base)
        for g in (1,2):
            root = base/f'whole-proof-attacks-v2-c{g}'
            suite = load(root/f'whole-c{g}/receipt.json')
            paths = [root/f'attempts/whole-c{g}-index/receipt.json']
            for row in [suite['controls'][1],*suite['rejections']]:
                paths.append(root/row['verification_receipt'])
                if row['recipe']:
                    paths.append(root/row['recipe']['construction_receipt'])
            for path in paths:
                row = load(path)
                C.command_receipt(path.parent,row['expected_exit'])
                cases.command_samples(path.parent,row)
                report['commands'].append(dict(path=str(path),receipt=identity(path),
                    resource_rows=identity(path.parent/'resources.jsonl'),caps=row['caps']))
        report['status']='passed-original-resource-replay-only'
    except BaseException:
        report.update(status='failed',error=traceback.format_exc())
        raise
    finally:
        report['ended_ns']=time.time_ns()
        with output.open('x') as stream:
            json.dump(report,stream,indent=2)
            stream.write('\n')
    print(json.dumps(dict(status=report['status'],commands=len(report['commands']),receipt=identity(output))))


if __name__=='__main__':
    main()
