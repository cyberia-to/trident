"""Read-only preparation: retain failures and freeze one proposed duplicate unlink."""
import hashlib
import json
import os
from pathlib import Path
import time

from safe_files import Held, identity, read_json, require, save, state, durable_bytes, ensure_directory

ROOT = Path(__file__).resolve().parent
BASE = ROOT.parent
ORIGINAL = 'whole-proof/attempts/c1-selfbuild-1/proof.joysc'
TARGET = 'whole-proof-attacks-v3-c1/whole-c1/certificate-cost.joysc'
COORD = 'whole-proof-attacks-completion-v3'
OBS = 'whole-proof-helper-v4/prefix-observation-1'
QUIET = 'whole-v3-emergency-quiescence'
SOURCES = ['safe_files.py', 'processes.py', 'prepare.py', 'reclaim.py', 'reconstruct.py', 'test_reclamation.py', 'PLAN.md']
LOCKS = [COORD + '/coordinator.lock', COORD + '/accounting.lock',
         'whole-proof-attacks-v3-c1/whole-suite.lock', 'whole-proof-attacks-v3-c2/whole-suite.lock',
         'whole-proof-attacks/whole-suite.lock', 'whole-proof-attacks-parallel-v2/coordinator.lock',
         'whole-proof-attacks-v2-c1/whole-suite.lock', 'whole-proof-attacks-v2-c2/whole-suite.lock']


def prepare():
    require(not (ROOT / 'plan.json').exists() and not (ROOT / 'retained').exists(), 'fresh preparation only')
    observation = read_json(BASE / OBS / 'receipt.json')
    require(observation['status'] == 'passed-exact-duplicate-prefix', 'original full comparison passed')
    require(observation['stat_before'] == observation['stat_after'], 'original files were stable')
    require(observation['original_path'] == str(BASE / ORIGINAL) and
            observation['partial_path'] == str(BASE / TARGET), 'exact classified paths')
    recipe = {'original': {'bytes': 11977015727, 'sha256': '80212be832ce0a5caafa69d9dd346ff20d51ce20fc86892f0c7039492619bbb0'},
              'prefix': {'bytes': 11901028947, 'sha256': 'a4108874af7f843bb86e89442c5ca4071a638d45c3d24b8aa7f20fa1f96b95ed'},
              'range': '[0,11901028947)', 'operation': 'Authenticate the complete original, then copy exactly the first n bytes to a fresh destination and authenticate them.'}
    require(observation['compared_bytes'] == recipe['prefix']['bytes'] and
            observation['source_prefix_sha256'] == observation['partial_sha256'] == recipe['prefix']['sha256'] and
            observation['original_whole_sha256'] == recipe['original']['sha256'], 'exact classified bytes')
    for rel, expected in zip([ORIGINAL, TARGET], observation['stat_after']):
        with Held(BASE / rel, expected):
            pass  # Metadata only: action-time full scans are still required.
    coordinator = read_json(BASE / COORD / 'run-1/receipt.json')
    require(coordinator['status'] == 'failed' and coordinator.get('cleanup_error'), 'original failed cleanup retained')
    quiet = read_json(BASE / QUIET / 'receipt.json')
    require(quiet['status'] == 'observed-quiescent' and quiet['matching_current_processes'] == [],
            'later independent historical quiescence')
    paths = {str(p.relative_to(BASE)) for p in (BASE / OBS).iterdir() if p.is_file()}
    paths.update({COORD + '/admission.json', COORD + '/stop.json', COORD + '/plan.json',
                  COORD + '/run.py', COORD + '/resources.py', QUIET + '/receipt.json', QUIET + '/first.json',
                  'whole-helper-digest-independent-review/design-review.json', 'whole-proof-helper-v4/observe-prefix.py'})
    for directory in [BASE / COORD / 'run-1', BASE / COORD / 'native-processes']:
        paths.update(str(p.relative_to(BASE)) for p in directory.iterdir() if p.is_file())
    for generation in (1, 2):
        suite = f'whole-proof-attacks-v3-c{generation}'
        paths.update([suite + '/whole_suite.py', suite + '/guard.py', suite + f'/whole-c{generation}/receipt.json'])
        for kind in (['construct-cost'] if generation == 1 else ['construct-cost', 'verify-cost']):
            directory = BASE / suite / 'attempts' / f'whole-c{generation}-{kind}'
            paths.update(str(p.relative_to(BASE)) for p in directory.iterdir() if p.is_file())
    retained = []
    for rel in sorted(paths):
        src = BASE / rel
        require(src.stat().st_size < 16 * 1024 * 1024, 'retain bounded metadata only')
        before = identity(src)
        with Held(src) as held:
            data = os.pread(held.fd, held.expected['st_size'], 0)
            held.check()
        require({'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()} == before, 'retention read changed')
        dest = ROOT / 'retained' / rel
        ensure_directory(dest.parent)
        durable_bytes(dest, data)
        require(identity(src) == identity(dest) == before, 'retained original changed')
        retained.append({'source': str(src), 'copy': str(dest.relative_to(ROOT)), 'identity': before})
    prior = read_json(BASE / COORD / 'reclamation-plan.json')['retained_partials']
    guarded_paths = list(prior) + ['whole-proof/attempts/c2-selfbuild-1/proof.joysc',
                                  'whole-proof-attacks-v3-c2/whole-c2/certificate-cost.joysc']
    protected = []
    for rel in guarded_paths:
        with Held(BASE / rel) as held:
            before = dict(held.expected)
            measured = identity(BASE / rel)
            held.check()
        if rel in prior:
            require(measured == prior[rel], 'old partial must remain exact')
        if rel.startswith('whole-proof/'):
            require(measured == {'bytes': 10569174820, 'sha256': '4db898cbc5133e0b96c758507d32b62aa82c4f39271cedd949b96d641a67de86'},
                    'original C2 proof exact')
        protected.append({'path': str(BASE / rel), 'state': before, 'identity': measured})
    registrations = []
    for path in sorted((BASE / COORD / 'native-processes').glob('*.json')):
        record = read_json(path)
        require(record['coordinator'] == 96065 and record['parent'] in [96107, 96108], 'known original owner')
        registrations.append({'path': str(path), 'identity': identity(path), 'record': record})
    require(len(registrations) == quiet['registration_count'] == 15, 'complete native owner set')
    require(sorted({r['record']['pgid'] for r in registrations}) == quiet['registered_pgids'], 'historical owner set exact')
    owners = sorted({96065, 96107, 96108} | {r['record']['pgid'] for r in registrations} |
                    {r['record']['pid'] for r in registrations})
    private = []
    for name in ['ps-first.stdout', 'ps-first.stderr', 'ps-second.stdout', 'ps-second.stderr']:
        p = BASE / QUIET / name
        private.append({'local_only_path': str(p), **identity(p)})
    plan = {'schema': 'exact-duplicate-prefix-reclamation/v4', 'status': 'prepared-not-executed',
            'prepared_ns': time.time_ns(), 'original': {'path': str(BASE / ORIGINAL), 'state': observation['stat_after'][0]},
            'target': {'path': str(BASE / TARGET), 'state': observation['stat_after'][1]}, 'recipe': recipe,
            'original_observation': identity(BASE / OBS / 'receipt.json'), 'protected': protected,
            'retained': retained, 'private_originals': private, 'owners': owners,
            'quiescence_cutoff_ns': quiet['second_observed_ns'], 'registrations': registrations,
            'registration_directory': str(BASE / COORD / 'native-processes'),
            'registration_entries': sorted(p.name for p in (BASE / COORD / 'native-processes').iterdir()),
            'locks': [{'path': str(BASE / p), 'state': state((BASE / p).lstat())} for p in sorted(LOCKS)],
            'accounting': {'charged_duplicate_bytes': recipe['prefix']['bytes'], 'released_bytes': 0,
                           'other_held_bytes_remain_charged': True, 'existing_profile_caps_changed': False},
            'transition_wall_seconds': 600, 'quiescence_min_interval_seconds': 2,
            'prohibition': 'Only the exact owned duplicate pathname is eligible. No cleanup, proof acceptance, failure status replacement, cap change or publication.'}
    save(ROOT / 'plan.json', plan)
    save(ROOT / 'sources.json', {name: identity(ROOT / name) for name in SOURCES})
    print(json.dumps({'status': plan['status'], 'plan': identity(ROOT / 'plan.json'),
                      'sources': identity(ROOT / 'sources.json'), 'retained_files': len(retained),
                      'released_bytes': 0}))


if __name__ == '__main__':
    prepare()
