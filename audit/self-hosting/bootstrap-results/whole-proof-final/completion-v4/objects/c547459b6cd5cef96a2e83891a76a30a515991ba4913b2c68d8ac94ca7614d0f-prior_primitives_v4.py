"""Hash-gated access to the immutable, independently reviewed v2 primitives."""
import hashlib
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parent
CHECKER_SHA = '503bb17b57ec2141788f1536b1750e875b5d1f8b108751e40e30d58ab2658c9c'
CHECKER = ROOT.parent / 'whole-proof-final-review-v2/check.py'
if hashlib.sha256(CHECKER.read_bytes()).hexdigest() != CHECKER_SHA:
    raise ValueError('immutable original v2 checker changed')
spec = importlib.util.spec_from_file_location('reviewed_v2_primitives', CHECKER)
C = importlib.util.module_from_spec(spec)
spec.loader.exec_module(C)
require, identity, load, same = C.require, C.identity, C.load, C.same
within, files, semantic, profile_flags = C.within, C.files, C.semantic, C.profile_flags
OUTPUT, PINS, ERRORS = C.OUTPUT, dict(C.PINS), dict(C.ERRORS)
CORPUS_FIXTURE_REVISION = C.CORPUS_FIXTURE_REVISION
PRIOR = tuple(name for kind in ('compiler', 'source', 'dependency', 'cfg')
              for name in ('binding-' + kind, 'rebound-' + kind)) + ('binding-job-limit',)
FRESH = ('rebound-job-limit',) + tuple(name for name in ERRORS if not name.startswith(('binding-', 'rebound-')))
require(len(PRIOR) == 9 and len(FRESH) == 14 and not set(PRIOR) & set(FRESH)
        and set(PRIOR) | set(FRESH) == set(ERRORS), 'complete distinct composite case contract')


def v3_pins(base):
    pin_path = ROOT.parent / 'whole-proof-final-review-v3/pins.json'
    require(identity(pin_path)['sha256'] == 'b0bc46b7ee07d2e4ec2cb1ae5796c97e99dd2e645156c383639618b7e68c8837', 'exact historical pin selector')
    pin_file = load(pin_path)
    require(pin_file['status'] == 'frozen-reviewed-v3-sources', 'v3 source freeze is pending')
    require(all(name not in PINS or value == PINS[name] for name, value in pin_file['sha256'].items()),
            'original reviewed pins remain immutable')
    require(all(not Path(name).is_absolute() and '..' not in Path(name).parts
                for name in pin_file['sha256']), 'relative source pin paths')
    result = dict(PINS)
    result.update(pin_file['sha256'])
    required = {'whole-proof-attacks-completion-v3/' + name for name in (
        'run.py', 'resources.py', 'plan.json', 'sources.json', 'independent-review.json',
        'prior_cases.py', 'binding_context.py', 'binding-fixture-acceptance.json',
        'transition.json', 'copied-inputs.json')}
    required.update(f'whole-proof-attacks-v3-c{g}/{name}' for g in (1, 2)
                    for name in ('guard.py', 'whole_suite.py', 'preparation.json',
                                 'target-helper/release/whole-proof-mutator', 'helper-build-2/receipt.json'))
    require(required <= pin_file['sha256'].keys(), 'all reviewed v3 source/provenance pins required')
    for name, expected in result.items():
        require(identity(base / name)['sha256'] == expected, 'reviewed source/runtime pin: ' + name)
    return result
