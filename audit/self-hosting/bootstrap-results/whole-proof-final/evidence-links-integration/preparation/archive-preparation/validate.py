"""Offline exact bytes, public payload scan and real Git CRLF checkout probe."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent
PACKET = ROOT / 'packet'
OUT = ROOT / 'validation'
OUT.mkdir()


def identity(data):
    return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def write(name, data):
    with (OUT / name).open('xb') as f:
        f.write(data)
    return identity(data)


commands = []


def run(argv, cwd):
    r = subprocess.run(argv, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    n = len(commands)
    commands.append({'argv': argv, 'cwd': str(cwd), 'exit_code': r.returncode,
                     'stdout': {'path': f'{n}.stdout', **write(f'{n}.stdout', r.stdout)},
                     'stderr': {'path': f'{n}.stderr', **write(f'{n}.stderr', r.stderr)}})
    assert r.returncode == 0, (argv, r.returncode)
    return r.stdout


expected = json.loads((ROOT / 'copy-inventory.json').read_text())
assert set(expected) == {str(p.relative_to(PACKET)) for p in PACKET.rglob('*') if p.is_file()}
for name, ref in expected.items():
    data = (PACKET / name).read_bytes()
    assert identity(data) == ref
    # Supplement the unchanged retained verifier with unbranded header checks.
    assert not re.search(rb'(?im)^\s*(?:proxy-)?authorization\s*:\s*\S+', data)
    assert not re.search(rb'(?i)"(?:authorization|proxy-authorization)"\s*:\s*"[^"\s]', data)

prior_validation = json.loads((ROOT / 'verify.stdout').read_text())
assert prior_validation['status'] == 'passed-retained-byte-validation'
assert prior_validation['originals_checked'] and prior_validation['original_files'] == 286
assert prior_validation['map'] == identity((PACKET / 'retained-files.json').read_bytes())
fixture = OUT / 'git-fixture'
fixture.mkdir()
run(['git', 'init', '-q', '--template='], fixture)
run(['git', 'config', 'core.autocrlf', 'true'], fixture)
tree = fixture / 'joy-source-bridge'
tree.mkdir()
for name in expected:
    p = tree / name
    p.parent.mkdir(parents=True, exist_ok=True)
    with p.open('xb') as f:
        f.write((PACKET / name).read_bytes())
paths = ['joy-source-bridge/' + name for name in sorted(expected)]
run(['git', 'add', '--', *paths], fixture)
run(['git', 'diff', '--cached', '--check'], fixture)
index_tree = run(['git', 'write-tree'], fixture).decode().strip()
checkout = OUT / 'checkout'
checkout.mkdir()
run(['git', '-c', 'core.autocrlf=true', 'checkout-index', '--all', '--prefix=' + str(checkout) + '/'], fixture)
checked = 0
for name, ref in expected.items():
    assert identity((checkout / 'joy-source-bridge' / name).read_bytes()) == ref
    checked += 1
assert all(identity((PACKET / name).read_bytes()) == ref for name, ref in expected.items())
write('commands.json', (json.dumps(commands, indent=2) + '\n').encode())
receipt = {
    'status': 'passed-offline-public-archive-validation',
    'scope': 'Retained original bytes, privacy and real Git filters only; no native workload or source mutation.',
    'driver': identity(Path(__file__).read_bytes()),
    'copy_inventory': identity((ROOT / 'copy-inventory.json').read_bytes()),
    'retained_map': identity((PACKET / 'retained-files.json').read_bytes()),
    'verifier': identity((PACKET / 'verify-retained.py').read_bytes()),
    'retained_verifier_originals_exit': 0,
    'extra_header_privacy_scan': 'passed for every packet file',
    'core_autocrlf': True, 'git_index_tree': index_tree,
    'actual_checkout_files': checked, 'all_checkout_identities_equal': True,
    'git_diff_check_exit': 0,
    'attributes': identity((PACKET / '.gitattributes').read_bytes()),
    'source_originals': 286, 'references_only': 21,
    'original_verifier_receipt': identity((ROOT / 'verify.stdout').read_bytes()),
    'commands': identity((OUT / 'commands.json').read_bytes()),
}
write('receipt.json', (json.dumps(receipt, indent=2) + '\n').encode())
print(json.dumps(receipt))
