"""Check the four current documentation edits against the actual frozen F5 summary."""
import hashlib
import json
import re
import subprocess
import time
from pathlib import Path

OUT = Path(__file__).resolve().parent
B = OUT.parent.parent
H = B / 'whole-proof-helper-v4/trident'
FILES = ['audit/self-hosting-progress.md', 'docs/guides/self-hosted-compilation.md',
         'reference/roadmap.md', 'CHANGELOG.md']
SUMMARY = B / 'whole-proof-final-delivery-preparation/staging/actual-1/summary.json'
FINAL = H / 'audit/self-hosting/bootstrap-results/whole-proof-final/acceptance'
report = dict(status='running', started_ns=time.time_ns(), commands=[], checks=[])


def identity(path):
    data = path.read_bytes()
    return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())


def check(name, value):
    assert value, name
    report['checks'].append(name)


def command(label, args):
    argv = ['/usr/bin/git', '-C', str(H), *args]
    row = dict(argv=argv, cwd=str(OUT), started_ns=time.time_ns())
    report['commands'].append(row)
    with (OUT / (label + '.stdout')).open('xb') as out, (OUT / (label + '.stderr')).open('xb') as err:
        result = subprocess.run(argv, cwd=OUT, stdout=out, stderr=err, timeout=30)
    row.update(exit_code=result.returncode, ended_ns=time.time_ns(),
               streams={n: identity(OUT / (label + '.' + n)) for n in ('stdout', 'stderr')})
    check(label, result.returncode == 0)
    return (OUT / (label + '.stdout')).read_text()


try:
    report['summary'] = dict(path=str(SUMMARY), **identity(SUMMARY))
    summary = json.loads(SUMMARY.read_text())
    check('actual F5 identity and accepted projection', summary['status'] == 'accepted-local-F5-projection'
          and summary['original_F5']['sha256'] == '46e7171ede86ce006afcb850b610590aa1bbcbd7bcf5ee3a37ec3f59ca617d66')
    check('final linked summary equals frozen actual summary', (FINAL / 'packet/summary.json').read_bytes() == SUMMARY.read_bytes())
    check('declared public profile and host observation scope', summary['profile'] == 'joy-nox-disclosed-compiler-v1'
          and summary['disclosure'] == 'complete public witness' and summary['physical_resource_claim'] == 'unattested')
    check('original failure statuses retained', summary['prior_statuses'] == {'v2': 'failed', 'v3': 'failed', 'v4': 'input-changed'})
    check('both generations and measured counts', len(summary['generations']) == 2
          and [g['generation'] for g in summary['generations']] == [1, 2]
          and all(g['corpus_observations'] == 547 and g['distinct_rejections'] == 23
                  and g['original_controls'] == 2 for g in summary['generations']))
    check('C2/C3 bytes and digests equal', summary['generations'][0]['compiler'] == summary['generations'][1]['compiler'])
    current = {name: (H / name).read_text() for name in FILES}
    before = {name: (OUT / 'before' / name).read_text() for name in FILES}
    report['files'] = {name: dict(before=identity(OUT / 'before' / name), after=identity(H / name)) for name in FILES}
    check('exact frozen S1 retained', '77213171d39b88c5f41221912251cc4813ac2b11' in current[FILES[0]])
    for generation in summary['generations']:
        for name in ('prove_seconds', 'verify_seconds'):
            text = format(generation[name], '.8f')
            check('measured generation %s %s %s' % (generation['generation'], name, text), text in current[FILES[0]])
    for name in FILES:
        check(name + ' heading structure unchanged', re.findall(r'^#+ .*$', before[name], re.M)
              == re.findall(r'^#+ .*$', current[name], re.M))
    check('guide execution commands unchanged', re.findall(r'```.*?```', before[FILES[1]], re.S)
          == re.findall(r'```.*?```', current[FILES[1]], re.S))
    def kelvin_table(text):
        return [line.split('←')[0].rstrip() for line in text.splitlines()
                if re.match(r'^[A-Za-z][A-Za-z.* ]+\s+\d+K\s+\d+K', line)]
    check('all Kelvin temperatures unchanged', kelvin_table(before[FILES[2]]) == kelvin_table(current[FILES[2]]))
    check('released changelog history unchanged', before[FILES[3]].split('## 0.3.0', 1)[1]
          == current[FILES[3]].split('## 0.3.0', 1)[1])
    check('ledger delivery history unchanged', before[FILES[0]].split('## Delivery history and retained receipts', 1)[1]
          == current[FILES[0]].split('## Delivery history and retained receipts', 1)[1])
    stale = ['SH8 remains open', 'pending SH8 proof acceptance', 'final adversarial/checker acceptance pending',
             'final adversarial/checker acceptance remains required', 'final native compilation-proof acceptance remains',
             'their unfinished status', 'complete self-build proofs remain SH8', 'Final adversarial\n   acceptance remains']
    check('no listed stale current SH8 statements', all(term not in text for text in current.values() for term in stale))
    check('SH8 checklist and closed gate', '- [x] Prove both complete frozen self-builds under SH8' in current[FILES[0]]
          and '| Closed — complete public-profile proofs for frozen S1 |' in current[FILES[0]])
    report['new_links'] = []
    for name in FILES:
        new = set(re.findall(r'\]\(([^)]+)\)', current[name])) - set(re.findall(r'\]\(([^)]+)\)', before[name]))
        for target in sorted(new):
            check('new evidence URL is local and fragment-free', not target.startswith(('http:', 'https:')) and '#' not in target)
            resolved = ((H / name).parent / target).resolve()
            check('new target exists: ' + target, resolved.is_file())
            report['new_links'].append(dict(source=name, target=target, resolved=str(resolved), **identity(resolved)))
    check('SH8 canonical heading exists', '## SH8. Proved self-compilation' in (H / 'reference/self-hosting.md').read_text())
    command('diff-check', ['diff', '--check', '--', *FILES])
    command('diff', ['diff', '--', *FILES])
    modified = command('owned-scope', ['diff', '--name-only', '--', *FILES]).splitlines()
    check('exact four owned modified paths', set(modified) == set(FILES))
    check('canonical contract unchanged', command('canonical-contract-diff', ['diff', '--', 'reference/self-hosting.md']) == '')
    report.update(status='passed-current-docs-summary-link-status-checks', no_native_commands=True,
                  no_stage_or_commit=True, independent_packet_review_owned_by_root=True)
except BaseException as error:
    report.update(status='failed', error=type(error).__name__ + ': ' + str(error))
    raise
finally:
    report['ended_ns'] = time.time_ns()
    with (OUT / 'receipt.json').open('x') as out:
        out.write(json.dumps(report, indent=2) + '\n')
print(json.dumps(dict(status=report['status'], checks=len(report['checks']), files=report.get('files'),
                     receipt=identity(OUT / 'receipt.json'))))
