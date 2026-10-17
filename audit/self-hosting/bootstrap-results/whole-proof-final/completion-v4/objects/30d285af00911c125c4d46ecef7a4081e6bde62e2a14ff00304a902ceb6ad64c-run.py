"""Complete the remaining diagnostic cases with a restricted retained-cost prelude."""
import contextlib
import fcntl
import hashlib
import json
import os
from pathlib import Path
import selectors
import shutil
import signal
import stat
import subprocess
import sys
import time
import traceback

import ownership
import resources as shared

ROOT, BASE, PLAN = shared.ROOT, shared.BASE, shared.PLAN
sys.path.insert(0, str(BASE / 'whole-prefix-reclamation-v4'))
from safe_files import Held, digest as held_digest
COST_ID = dict(bytes=10569174820,
               sha256='246cf2d72b443d2b1e85e3b366936075107fc2387ffc741f0d3e08aa2878eb4d')


def require(value, message):
    if not value:
        raise ValueError(message)


def load(path):
    return json.loads(Path(path).read_text())


def cancelled(signum, _frame):
    raise InterruptedError('coordinator signal: ' + str(signum))


def stat_record(value):
    return {key: getattr(value, key) for key in
            ('st_dev', 'st_ino', 'st_mode', 'st_nlink', 'st_size', 'st_mtime_ns', 'st_ctime_ns')}


def reservation(existing, full):
    mutations = (sum(row['bytes'] + PLAN['mutant_growth_ceiling_bytes']
                     for row in PLAN['proofs'].values()) if full else 0)
    growth = (mutations + 2 * PLAN['canonical_and_output_bucket_per_generation'] +
              3 * PLAN['metadata_bucket_per_generation'])
    total = sum(existing.values()) + growth
    free = shutil.disk_usage(ROOT).free
    required_free = max(growth + PLAN['free_floor_bytes'], mutations + 10 * 1024**3)
    require(total <= PLAN['shared_disk_bytes'], 'actual existing bytes plus reservation exceed shared cap')
    require(free >= required_free, 'free space below reserved growth and unchanged headroom')
    return dict(existing_bytes=sum(existing.values()), growth_bytes=growth,
                reservation_bytes=total, free_bytes=free, required_free_bytes=required_free)


def retire_cost():
    """Retire one declared historical temporary only after its fresh negative passes."""
    suite = shared.ROOTS[2]
    work = suite / 'whole-c2'
    ready = load(work / 'prelude-complete.json')
    require(ready['status'] == 'passed' and ready['generation'] == 2, 'actual fresh prelude required')
    require(ready['suite_receipt'] == shared.identity(work / 'receipt.json'), 'prelude suite bytes')
    require(ready['admission'] == shared.identity(work / 'retained-cost-admission.json'), 'construction admission bytes')
    historical = load(work / 'retained-cost-admission.json')
    require(historical['status'] == 'passed-construction-admission' and
            historical['accepted_as_rejection'] is False and
            historical['fresh_verification_required'] is True and
            historical['expected_fresh_error'] == 'semantic terminal: Claim', 'construction-only admission')
    report = load(work / 'receipt.json')
    row = ready['rejection']
    require(report['rejections'] == [row] and row['name'] == 'cost' and
            row['certificate'] == COST_ID and row['error'] == 'semantic terminal: Claim', 'single expected fresh negative')
    command_dir = suite / 'attempts/whole-c2-verify-cost'
    command = load(command_dir / 'receipt.json')
    require(command['status'] == 'passed' and command['exit_code'] == command['expected_exit'] == 1 and
            command['argv'] == PLAN['prelude_command'] and 'resource_stop' not in command and
            command['inputs_before'] == command['inputs_after'], 'exact successful fresh verification command')
    require(command['caps'] == dict(wall=7500, cpu=7500, rss=6 * 1024**3, file=32 * 1024**2),
            'original verifier ceilings')
    for name, expected in command['files'].items():
        require(shared.identity(command_dir / name) == expected, 'fresh raw verification evidence')
    require((command_dir / 'stdout').read_bytes() == b'' and
            'semantic terminal: Claim' in (command_dir / 'stderr').read_text(), 'exact failure output contract')
    require(shared.identity(work / 'cost.dag') == row['protected_output'], 'protected output unchanged')
    binding = command['process_binding']
    require(ownership.observe(binding)['status'] == 'empty' and command['final_group']['status'] == 'empty',
            'fresh verifier group actually empty')
    shared.process_snapshot(os.getpid())
    registration = ROOT / 'native-processes/whole-c2-verify-cost.json'
    require(registration.with_suffix('.retired').exists(), 'fresh native group retirement observed')
    path = BASE / PLAN['historical_cost']
    require(command['inputs_after'][str(path)] == COST_ID, 'exact retained cost was verified')
    require(not shared.stop_requested(), 'no failure before declared retirement')
    record, held = None, None
    try:
        with shared.accounting(exclusive=True), Held(path) as held:
            before = held.expected
            end = time.monotonic() + 600
            require(held_digest(held, end) == COST_ID, 'full retained temporary hash')
            admitted_stat = historical['certificate']['stat']
            keys = dict(device='st_dev', inode='st_ino', bytes='st_size', mtime_ns='st_mtime_ns',
                        ctime_ns='st_ctime_ns', links='st_nlink', mode='st_mode')
            require(all(admitted_stat[key] == before[value] for key, value in keys.items()),
                    'same exact file as construction admission before fresh verification')
            record = dict(status='classified-before-unlink', removed=False, path=str(path), identity=COST_ID,
                          stat=before, construction_admission=ready['admission'],
                          fresh_verification=shared.identity(command_dir / 'receipt.json'),
                          failed_original_verification_remains_failed=True,
                          process_retirement=shared.identity(registration.with_suffix('.retired')),
                          prelude=shared.identity(work / 'prelude-complete.json'), time_ns=time.time_ns())
            shared.write(ROOT / 'retained-cost-retirement-before.json', record)
            require(not shared.stop_requested(), 'no stop immediately before unlink')
            held.unlink()
            record.update(status='unlinked-held-inode', removed=True, charged_bytes=COST_ID['bytes'])
            shared.write(ROOT / 'retained-cost-retirement-unlinked.json', record)
            require(held_digest(held, end) == COST_ID, 'same retained bytes after unlink')
        require(not path.exists() and held.fd is None, 'exact temporary absent and descriptor closed')
        record.update(status='passed', removed=True, charged_bytes=0, completed_ns=time.time_ns(),
                      before=shared.identity(ROOT / 'retained-cost-retirement-before.json'))
        shared.write(ROOT / 'retained-cost-retirement.json', record)
    except BaseException:
        failure = dict(record or {}, status='failed', removed=bool(held and held.unlinked),
                       charged_bytes=COST_ID['bytes'], error=traceback.format_exc(), ended_ns=time.time_ns())
        shared.write(ROOT / 'retained-cost-retirement-failed.json', failure)
        raise


def stop_all(children, bindings):
    results = []
    # Native groups first. Failure of one cleanup must not skip another group.
    def native_sweep(label):
        records = []
        # This barrier covers a native registration already in preexec. The
        # registration rechecks shared stop while holding the same lock.
        with shared.accounting(exclusive=True):
            for path in sorted((ROOT / 'native-processes').glob('*.json')):
                try:
                    record = load(path)
                    observed = path.with_suffix('.observed')
                    if observed.exists():
                        later = load(observed)
                        require(all(later[key] == record[key] for key in ('pid', 'pgid', 'started')),
                                'observed process binding changed')
                        record = later
                    records.append(record)
                except BaseException:
                    results.append(dict(status='failed', sweep=label, registration=str(path), error=traceback.format_exc()))
        for record in records:
            try:
                child = next((p for p in children.values() if p.pid == record['pid']), None)
                result = ownership.stop_group(record, child)
                results.append(dict(result, sweep=label))
            except BaseException:
                results.append(dict(status='failed', sweep=label, binding=record, error=traceback.format_exc()))
    native_sweep('before-suite-shutdown')
    for generation, child in children.items():
        try:
            results.append(ownership.stop_child(child, bindings.get(generation)))
        except BaseException:
            results.append(dict(status='failed', pid=child.pid, error=traceback.format_exc()))
    native_sweep('after-suite-shutdown')
    result = dict(status='passed' if all(row['status'] == 'passed' for row in results) else 'failed',
                  groups=results, time_ns=time.time_ns())
    shared.write(ROOT / 'shutdown.json', result)
    return result


def execute():
    output = ROOT / 'run-1'
    output.mkdir()
    report = dict(schema='trident/complete-proof-missing-cases-orchestration/v4', status='prepared',
                  started_ns=time.time_ns(), plan=shared.identity(ROOT / 'plan.json'),
                  generations=[], sampled_peak_rss_bytes=0)
    children, bindings, streams = {}, {}, []
    selector = selectors.DefaultSelector()

    def save():
        shared.write(output / 'receipt.json', report)

    save()
    try:
        manifest = load(ROOT / 'sources.json')
        review = load(ROOT / 'independent-review.json')
        require(review['status'] == 'passed-source-review' and review['sources'] == manifest,
                'exact frozen independent source review required')
        require(all(shared.identity(p) == value for p, value in manifest.items()), 'reviewed input changed')
        prefix = load(ROOT / 'prefix-transition.json')
        require(shared.identity(prefix['path']) == prefix['identity'], 'reviewed prefix reclamation receipt')
        prefix_result = load(prefix['path'])
        require(prefix_result['status'] == 'passed-reclaimed-exact-duplicate' and
                prefix_result['unlink_performed'] is True and
                prefix_result['released_owned_bytes'] == 11901028947 and
                prefix_result['charged_duplicate_bytes'] == 0 and
                prefix_result['before'] == prefix_result['after'] and
                prefix_result['before']['original'] == PLAN['proofs']['1'] and
                prefix_result['before']['prefix'] == dict(bytes=11901028947,
                    sha256='a4108874af7f843bb86e89442c5ca4071a638d45c3d24b8aa7f20fa1f96b95ed'),
                'actual separately reviewed prefix reclamation required')
        require(all(shared.identity(path) == value for path, value in prefix_result['bindings'].items()),
                'prefix transition exact reviewed source, plan and review bindings')
        require(not (BASE / 'whole-proof-attacks-v3-c1/whole-c1/certificate-cost.joysc').exists(),
                'classified duplicate prefix must have been reclaimed')
        for g, root in shared.ROOTS.items():
            require(not (root / 'attempts').exists() and not (root / f'whole-c{g}').exists(), 'fresh generation scope')
        cost = str(BASE / PLAN['historical_cost'])
        require(shared.identity(cost) == COST_ID, 'original complete cost temporary identity')
        baseline = shared.inventory()
        # Only this declared historical temporary may be retired. Everything
        # else in old scopes stays hash-bound, including all failed receipts.
        immutable = {p: shared.identity(p) for p in baseline if p != cost and
                     any(Path(p).is_relative_to(root) for root in shared.RETAINED_ROOTS)}
        shared.write(ROOT / 'baseline.json', dict(files=baseline, immutable=immutable,
                                                declared_retirement={cost: COST_ID}))
        prelude = reservation(baseline, full=False)
        report.update(status='running', phase='restricted-cost-verification', admission=prelude,
                      source=manifest, independent_review=shared.identity(ROOT / 'independent-review.json'))
        for g, root in shared.ROOTS.items():
            command = [sys.executable, '-B', str(root / 'whole_suite.py'), '--generation', str(g)]
            child = subprocess.Popen(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                     env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'), start_new_session=True)
            children[g] = child
            bindings[g] = ownership.bind(child.pid)
            report['generations'].append(dict(generation=g, status='running', command=command,
                                              cwd=str(root), pid=child.pid, binding=bindings[g]))
            for label, pipe in (('stdout', child.stdout), ('stderr', child.stderr)):
                stream = (output / f'c{g}.{label}').open('xb')
                streams.append(stream)
                os.set_blocking(pipe.fileno(), False)
                selector.register(pipe, selectors.EVENT_READ, stream)
        shared.write(ROOT / 'admission.json', dict(pid=os.getpid(), children={str(g): p.pid for g, p in children.items()},
                     phase='restricted-cost-verification', reservation=prelude,
                     plan=shared.identity(ROOT / 'plan.json'), source_manifest=shared.identity(ROOT / 'sources.json'),
                     time_ns=time.time_ns()))
        save()
        started, next_sample = time.monotonic(), 0
        with (output / 'resources.jsonl').open('x') as samples:
            while any(p.poll() is None for p in children.values()) or selector.get_map():
                for key, _ in selector.select(timeout=0.2):
                    chunk = os.read(key.fileobj.fileno(), 65536)
                    if not chunk:
                        selector.unregister(key.fileobj)
                        key.fileobj.close()
                        continue
                    remaining = PLAN['per_stream_log_bytes'] - key.data.tell()
                    key.data.write(chunk[:max(remaining, 0)])
                    key.data.flush()
                    if len(chunk) > remaining:
                        shared.fail('suite-log-bytes', key.data.name)
                now = time.monotonic()
                if now >= next_sample:
                    next_sample = now + 1
                    sample = shared.sample(baseline, os.getpid())
                    samples.write(json.dumps(sample) + '\n')
                    samples.flush()
                    report['sampled_peak_rss_bytes'] = max(report['sampled_peak_rss_bytes'], sample['rss_bytes'])
                    report['latest_sample'] = sample
                    save()
                for g, child in children.items():
                    if child.poll() is not None and child.returncode != 0:
                        shared.fail('suite-failed', dict(generation=g, exit_code=child.returncode))
                if now - started > PLAN['outer_schedule_seconds']:
                    shared.fail('schedule-wall')
                if shared.stop_requested():
                    raise RuntimeError('shared failure: ' + (ROOT / 'stop.json').read_text())
                ready = shared.ROOTS[2] / 'whole-c2/prelude-complete.json'
                if not (ROOT / 'full-admission.json').exists() and ready.exists():
                    retire_cost()
                    current = shared.inventory()  # Real bytes after retirement, never a projected subtraction.
                    full = reservation(current, full=True)
                    shared.write(ROOT / 'full-admission.json', dict(admission=shared.identity(ROOT / 'admission.json'),
                                 retirement=shared.identity(ROOT / 'retained-cost-retirement.json'),
                                 reservation=full, time_ns=time.time_ns()))
                    report.update(phase='remaining-mutations', full_admission=full)
                    save()
        report['final_sample'] = shared.sample(baseline, os.getpid())
        require(not shared.stop_requested(), 'final shared resource verdict')
        require((ROOT / 'full-admission.json').exists(), 'actual full admission required')
        for row in report['generations']:
            g = row['generation']
            receipt_path = shared.ROOTS[g] / f'whole-c{g}/receipt.json'
            suite = load(receipt_path)
            require(children[g].wait() == 0 and suite['status'] == 'passed-completion' and
                    suite['prior_cases']['status'] == 'passed-selected-cases' and
                    len(suite['prior_cases']['controls']) == 2 and len(suite['prior_cases']['rejections']) == 12 and
                    len(suite['rejections']) == 11, 'complete unchanged distinct case matrix')
            require(ownership.observe(bindings[g])['status'] == 'empty', 'suite group empty')
            row.update(status='passed', exit_code=0, suite_receipt=shared.identity(receipt_path),
                       logs={label: shared.identity(output / f'c{g}.{label}') for label in ('stdout', 'stderr')})
        require(all(shared.identity(p) == value for p, value in immutable.items()), 'original retained evidence changed')
        require(not Path(cost).exists() and load(ROOT / 'retained-cost-retirement.json')['status'] == 'passed',
                'only declared historical retirement')
        require(all(shared.identity(p) == value for p, value in manifest.items()), 'reviewed source changed')
        require(not shared.stop_requested(), 'shared failure recorded at completion')
        report.update(status='passed', inputs_unchanged=True)
    except BaseException:
        original_error = traceback.format_exc()
        report.update(status='failed', error=original_error)
        save()
        shared.fail('coordinator-exception', original_error)
        try:
            report['cleanup'] = stop_all(children, bindings)
        except BaseException:
            report['cleanup_error'] = traceback.format_exc()
        raise
    finally:
        selector.close()
        for stream in streams:
            stream.close()
        report['children_final'] = {str(g): dict(pid=p.pid, exit_code=p.poll()) for g, p in children.items()}
        report.update(ended_ns=time.time_ns(), files={p.name: shared.identity(p) for p in output.iterdir()
                      if p.is_file() and p.name != 'receipt.json' and '.pending-' not in p.name})
        save()


def main():
    signal.signal(signal.SIGTERM, cancelled)
    with contextlib.ExitStack() as locks:
        paths = [ROOT / 'coordinator.lock', shared.ORIGINAL / 'whole-suite.lock']
        for version, scope in ((2, 'whole-proof-attacks-parallel-v2'), (3, 'whole-proof-attacks-completion-v3')):
            paths += [BASE / scope / 'coordinator.lock']
            paths += [BASE / f'whole-proof-attacks-v{version}-c{g}/whole-suite.lock' for g in (1, 2)]
        for path in paths:
            handle = locks.enter_context(path.open('a+'))
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        require(not shared.stop_requested() and not (ROOT / 'admission.json').exists(), 'one fresh v4 schedule only')
        execute()


if __name__ == '__main__':
    main()
