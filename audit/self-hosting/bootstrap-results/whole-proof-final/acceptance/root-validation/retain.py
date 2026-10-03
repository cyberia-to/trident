"""Retain the reviewed actual packet and a public projection of its commands."""
import hashlib
import json
from pathlib import Path
import sys

B = Path('/Users/master/cyber/.worktrees/selfhost-0.4-finalization-20261002')
ROOT = Path(__file__).resolve().parent
PACKET = B / 'whole-proof-final-delivery-preparation/staging/actual-1'
DEST = B / 'whole-proof-helper-v4/trident/audit/self-hosting/bootstrap-results/whole-proof-final/acceptance'
sys.path.insert(0, str(B / 'whole-proof-final-delivery-preparation'))
from privacy import public_bytes


def identity(data):
    return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())


def ordinary(path):
    assert path.is_file() and not path.is_symlink() and path.stat().st_nlink == 1
    assert path.stat().st_size < 16 * 1024**2
    data = path.read_bytes()
    public_bytes(data)
    return data


def main():
    review = json.loads(ordinary(ROOT / 'review.json'))
    assert review['status'] == 'passed-root-packet-review'
    receipt = json.loads(ordinary(PACKET / 'receipt.json'))
    assert receipt['status'] == 'passed-local-packet'
    assert receipt['source']['review']['sha256'] == '29360024cf5ba6736599c53477b242af5ad3153da99cddf3d19464beb944b7c7'
    files = {p.relative_to(PACKET).as_posix(): ordinary(p)
             for p in PACKET.rglob('*') if p.is_file()}
    assert {name: identity(data) for name, data in files.items()} == review['files']
    commands = []
    for generation in (1, 2):
        for stage in ('selfbuild', 'fresh-verification'):
            path = B / f'whole-proof/attempts/c{generation}-{stage}-1/receipt.json'
            # Full native receipts contain private process samples. Project only
            # these explicit public fields, preserving the original identity.
            raw = path.read_bytes()
            row = json.loads(raw)
            assert row['status'] == 'passed' and row['exit_code'] == 0
            assert row['inputs_before'] == row['inputs_after']
            commands.append(dict(
                generation=generation, stage=stage,
                original=dict(path=str(path), **identity(raw)),
                **{key: row[key] for key in (
                    'command', 'cwd', 'binary', 'profile_identity',
                    'installed_source_receipt', 'exit_code',
                    'elapsed_seconds', 'sampled_peak_rss_bytes')},
            ))
    checker_path = B / 'whole-proof-final-launch-v5/orchestration/final-review.json'
    checker_raw = checker_path.read_bytes()
    assert identity(checker_raw) == dict(bytes=137215, sha256='46e7171ede86ce006afcb850b610590aa1bbcbd7bcf5ee3a37ec3f59ca617d66')
    checker = json.loads(checker_raw)
    assert checker['status'] == 'passed-composite-replay'
    projection = dict(
        schema='trident/actual-sh8-command-projection/v1',
        source_revision='77213171d39b88c5f41221912251cc4813ac2b11',
        joy_revision='6e0ec4d8440e2521df08f442d64f54e667044716',
        native_commands=commands,
        checker=dict(command=checker['command'], original=dict(path=str(checker_path), **identity(checker_raw))),
        collector_command=[
            '/opt/homebrew/opt/python@3.14/bin/python3.14', '-B', '-W', 'error',
            'collect.py', '--review-sha256',
            '29360024cf5ba6736599c53477b242af5ad3153da99cddf3d19464beb944b7c7',
            '--output-name', 'actual-1'],
        collector_cwd=str(B / 'whole-proof-final-delivery-preparation'),
        scope='Public projection of exact original commands and measured fields. Full receipts remain local and retain their byte identities.',
    )
    command_data = (json.dumps(projection, indent=2) + '\n').encode()
    public_bytes(command_data)
    assert not DEST.exists() and not DEST.is_symlink()
    DEST.mkdir()
    for name, data in files.items():
        target = DEST / 'packet' / name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as output:
            output.write(data)
    (DEST / 'commands.json').write_bytes(command_data)
    (DEST / 'root-validation').mkdir()
    for name in ('review.json', 'stdout', 'stderr', 'retain.py'):
        data = ordinary(ROOT / name)
        (DEST / 'root-validation' / name).write_bytes(data)
    assert {name: identity(ordinary(DEST / 'packet' / name)) for name in files} == review['files']
    result = dict(status='passed-exact-packet-copy', source=str(PACKET), destination=str(DEST),
                  packet_files=len(files), packet_bytes=sum(map(len, files.values())),
                  files=review['files'], commands=identity(command_data),
                  source_review=identity(ordinary(ROOT / 'review.json')))
    data = (json.dumps(result, indent=2) + '\n').encode()
    public_bytes(data)
    (ROOT / 'copy.json').write_bytes(data)
    (DEST / 'root-validation/copy.json').write_bytes(data)
    print(json.dumps({key: result[key] for key in ('status', 'packet_files', 'packet_bytes')}))


if __name__ == '__main__':
    main()
