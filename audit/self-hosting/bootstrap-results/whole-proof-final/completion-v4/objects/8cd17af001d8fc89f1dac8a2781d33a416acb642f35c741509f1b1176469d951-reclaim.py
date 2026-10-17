"""One explicit reviewed unlink; never run by preparation or automatic cleanup."""
import argparse
import contextlib
import fcntl
import os
from pathlib import Path
import sys
import time
import traceback

from processes import observe
from safe_files import Held, compare_prefix, deadline, digest, identity, make_directory, read_json, require, save

ROOT = Path(__file__).resolve().parent
BASE = ROOT.parent
TARGET = BASE / 'whole-proof-attacks-v3-c1/whole-c1/certificate-cost.joysc'
ORIGINAL = BASE / 'whole-proof/attempts/c1-selfbuild-1/proof.joysc'


def evidence(plan, root, end):
    for row in plan['retained']:
        require(identity(Path(row['source']), end) == row['identity'] == identity(root / row['copy'], end),
                'original failed evidence or exact retained copy changed')
    paths = sorted(str(p) for p in Path(plan['registration_directory']).glob('*.json'))
    require(sorted(p.name for p in Path(plan['registration_directory']).iterdir()) == plan['registration_entries'],
            'native registry directory membership changed')
    require(paths == sorted(r['path'] for r in plan['registrations']), 'native registration set changed')
    for row in plan['registrations']:
        require(identity(Path(row['path']), end) == row['identity'], 'native registration changed')


def transition(plan, root, output, private, observer=observe, hook=lambda _stage: None, bindings=None):
    """Core also exercised only on tiny isolated synthetic fixtures in tests."""
    make_directory(output)
    make_directory(private)
    end = time.monotonic() + plan['transition_wall_seconds']
    charge = plan['recipe']['prefix']['bytes']
    record = {'status': 'running', 'started_ns': time.time_ns(), 'unlink_performed': False,
              'released_owned_bytes': 0, 'charged_duplicate_bytes': charge,
              'original_failure_status_unchanged': True, 'bindings': bindings or {},
              'command': [sys.executable, *sys.argv], 'cwd': os.getcwd()}
    def bound_evidence():
        for path, expected in (bindings or {}).items():
            require(identity(Path(path), end) == expected, 'reviewed action binding changed')
        evidence(plan, root, end)
    target = None
    save(output / 'started.json', record)
    try:
        space = os.statvfs(root)
        require(space.f_bavail * space.f_frsize >= 8589934592, 'unchanged 8 GiB free-space floor')
        with contextlib.ExitStack() as stack:
            for row in plan['locks']:
                lock = stack.enter_context(Held(Path(row['path']), row['state']))
                fcntl.flock(lock.fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            bound_evidence()
            original = stack.enter_context(Held(Path(plan['original']['path']), plan['original']['state']))
            target = stack.enter_context(Held(Path(plan['target']['path']), plan['target']['state']))
            guarded = [(stack.enter_context(Held(Path(r['path']), r['state'])), r['identity'])
                       for r in plan['protected']]
            first = observer(private, 'quiescence-1', plan['owners'], plan['quiescence_cutoff_ns'])
            require(first['no_live_prior_owner'] is True, 'first owner observation rejected')
            save(output / 'quiescence-1.json', first)
            before = compare_prefix(original, target, plan['recipe'], end)
            for held, expected in guarded:
                require(digest(held, end) == expected, 'protected held bytes changed before action')
            minimum_ns = first['observed_ns'] + int(plan['quiescence_min_interval_seconds'] * 1_000_000_000)
            remaining = max(0, (minimum_ns - time.time_ns()) / 1_000_000_000)
            require(remaining <= 2, 'invalid observation interval')
            time.sleep(remaining)
            second = observer(private, 'quiescence-2', plan['owners'], plan['quiescence_cutoff_ns'])
            require(second['no_live_prior_owner'] is True, 'second owner observation rejected')
            require(second['started_ns'] >= minimum_ns, 'two distinct separated quiescence observations')
            save(output / 'quiescence-2.json', second)
            bound_evidence()
            hook('before-journal')
            for held in [original, target, *(h for h, _ in guarded)]:
                held.check()
            record.update(status='validated-before-unlink', before=before,
                          original_state=original.expected, duplicate_state=target.expected)
            save(output / 'before-unlink.json', record)
            hook('after-journal')
            # Check both objects again after the durable pre-transition receipt.
            for held in [original, target, *(h for h, _ in guarded)]:
                held.check()
            deadline(end)
            space = os.statvfs(root)
            require(space.f_bavail * space.f_frsize >= 8589934592, 'free-space floor before transition')
            target.unlink()
            record.update(status='unlinked-held-inode', unlink_performed=True,
                          unlinked_state=dict(target.expected))
            save(output / 'after-unlink.json', record)
            hook('after-unlink')
            # The duplicate descriptor still holds the unlinked bytes and their disk charge.
            after = compare_prefix(original, target, plan['recipe'], end)
            require(after == before, 'post-unlink complete equality changed')
            for held, expected in guarded:
                require(digest(held, end) == expected, 'protected held bytes changed after action')
            bound_evidence()
            record.update(status='verified-unlinked-inode-still-held', after=after)
            save(output / 'verified-held.json', record)
        # Descriptors have closed. Only now may this one logical scope charge be released.
        require(target.unlinked and target.fd is None and not os.path.lexists(plan['target']['path']),
                'classified name must remain absent after descriptor close')
        record.update(status='passed-reclaimed-exact-duplicate', charged_duplicate_bytes=0,
                      released_owned_bytes=charge, ended_ns=time.time_ns(),
                      physical_free_space_change_claimed=False, signals_sent=[])
        save(output / 'final.json', record)
        return record
    except BaseException:
        record.update(status='failed', unlink_performed=bool(target and target.unlinked),
                      error=traceback.format_exc(), ended_ns=time.time_ns(),
                      released_owned_bytes=0, charged_duplicate_bytes=charge,
                      accounting_policy='Conservative charge retained if terminal verification fails; no automatic retry or reconstruction.')
        save(output / 'failed.json', record)
        raise


def authorize(review_sha256):
    require(ROOT.name == 'whole-prefix-reclamation-v4', 'exact reviewed driver directory')
    review_path = ROOT / 'review.json'
    require(identity(review_path)['sha256'] == review_sha256, 'exact explicit review receipt')
    review = read_json(review_path)
    require(review['status'] == 'passed-prefix-reclamation-source-review', 'independent source review passed')
    require(review['sources'] == identity(ROOT / 'sources.json') and
            review['plan'] == identity(ROOT / 'plan.json'), 'review binds exact sources and plan')
    source_files = read_json(ROOT / 'sources.json')
    from prepare import SOURCES
    require(set(source_files) == set(SOURCES), 'complete reviewed source inventory')
    for name, expected in source_files.items():
        require(identity(ROOT / name) == expected, 'reviewed source changed: ' + name)
    plan = read_json(ROOT / 'plan.json')
    require(plan['schema'] == 'exact-duplicate-prefix-reclamation/v4' and
            plan['status'] == 'prepared-not-executed', 'explicit one-transition plan')
    require(plan['target']['path'] == str(TARGET) and plan['original']['path'] == str(ORIGINAL),
            'only the exact owned C1 prefix may be unlinked')
    require(plan['recipe']['prefix'] == {'bytes': 11901028947, 'sha256': 'a4108874af7f843bb86e89442c5ca4071a638d45c3d24b8aa7f20fa1f96b95ed'} and
            plan['recipe']['original'] == {'bytes': 11977015727, 'sha256': '80212be832ce0a5caafa69d9dd346ff20d51ce20fc86892f0c7039492619bbb0'}, 'fixed actual classification')
    require(plan['transition_wall_seconds'] == 600 and plan['quiescence_min_interval_seconds'] == 2,
            'fixed transition observation bounds')
    require(plan['accounting']['released_bytes'] == 0, 'preparation cannot release a charge')
    return plan


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--review-sha256', required=True)
    args = parser.parse_args()
    plan = authorize(args.review_sha256)
    source_names = read_json(ROOT / 'sources.json')
    bindings = {str(ROOT / n): identity(ROOT / n)
                for n in [*source_names, 'review.json', 'sources.json', 'plan.json']}
    result = transition(plan, ROOT, ROOT / 'action-1', ROOT / 'private-action-1', bindings=bindings)
    print(result['status'])


if __name__ == '__main__':
    main()
