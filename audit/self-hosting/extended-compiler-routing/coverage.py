"""Count retained observations and defined case vectors; execute no compiler."""
import ast
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parents[1]
REPO = HERE.parents[1]
sys.path.insert(0, str(HERE))
BASE = 'b991d901e6585a40bedd0e0a3d4382c2ad3d89c1'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cases_ast(source):
    return ast.dump(next(node for node in ast.parse(source).body
                         if isinstance(node, ast.FunctionDef) and node.name == 'cases'))


report = dict(scope='Retained run counts and static case definitions; no new corpus or C2 execution',
              base_revision=BASE, command=['python3', 'audit/self-hosting/extended-compiler-routing/coverage.py'],
              retained_runs={}, defined_cases={})
for suffix in ['full', 'constants', 'callable', 'types', 'intrinsic', 'profile', 'graph', 'capacity']:
    path = HERE / ('source-capacity-' + suffix + '-cli.json')
    receipt = json.loads(path.read_text())
    report['retained_runs'][suffix] = dict(path=str(path.relative_to(REPO)), sha256=sha(path),
        commands=len(receipt['commands']), observations=len(receipt['observations']),
        status=receipt.get('status'), covered_by_supplied_compiler=suffix not in ('graph', 'capacity'))
for name in ['check-guest-constant-linking', 'check-guest-function-imports',
             'check-guest-type-imports', 'check-guest-intrinsics', 'check-guest-module-graph']:
    path = HERE / (name + '.py')
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    cases = list(module.cases())
    before = subprocess.check_output(['git', 'show', BASE + ':' + str(path.relative_to(REPO))], cwd=REPO, text=True)
    unchanged = cases_ast(before) == cases_ast(path.read_text())
    if not unchanged:
        raise RuntimeError(f'case vectors changed: {name}')
    report['defined_cases'][name] = dict(count=len(cases), names=[case['case'] for case in cases],
        case_provider_ast_unchanged=True, script_sha256=sha(path))
print(json.dumps(report, indent=2))
