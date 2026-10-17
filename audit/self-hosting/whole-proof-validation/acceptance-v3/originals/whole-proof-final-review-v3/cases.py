"""Independently replay original v2 cases and the disjoint fresh v3 cases."""
import json
from pathlib import Path
from common import (C, CHECKER, require, identity, load, same, files, within,
                    semantic, profile_flags, ERRORS, PRIOR, FRESH)


def names(rows, expected, label):
    require([row['name'] for row in rows] == list(expected), label + ': exact ordered unique case set')


def admitted_context(coordinates, admission, compiler, job):
    """Pinned production admissions authenticate nouns; JOB limits own context."""
    roots = []
    for path in (compiler, job):
        with path.open('rb') as stream:
            header = stream.read(40)
        require(len(header) == 40 and header[:8] == b'NOXDAG01', 'admitted noun header')
        roots.append(header[8:40].hex())
    require(roots[0] == coordinates['program_particle'] == admission['compiler_particle']
            and roots[1] == admission['job_particle'], 'actual admitted program/JOB coordinates')
    particles = [roots[0], coordinates['formula_particle'], roots[1]]
    for value in particles:
        require(isinstance(value, str) and len(value) == 64, 'particle dimensions')
        raw = bytes.fromhex(value)
        require(raw.hex() == value and all(int.from_bytes(raw[i:i+8], 'little') < 0xffffffff00000001
                                          for i in range(0, 32, 8)), 'canonical particle')
    limits = admission['limits']
    budget, frames = limits['reductions'], limits['evaluator_frames']
    require(type(budget) is int and 0 < budget < 1 << 64
            and type(frames) is int and 0 < frames < 1 << 32, 'admitted positive bounded limits')
    return [*particles, '1', str(budget), str(frames)]


def diagnostic(directory, argv, exit_code, metadata, inputs, kind, version, pins):
    if version == 2:
        row = C.diagnostic(directory, argv, exit_code, metadata, inputs, kind)
        command_samples(directory, row)
        return row
    require(version == 3, 'known diagnostic provenance')
    row = C.command_receipt(directory, exit_code)
    require(row['schema'] == 'trident/whole-proof-attack-command/v2'
            and row['argv'] == list(map(str, argv)) and row['metadata'] == metadata
            and row['cwd'] == str(directory) and row['environment'] == {'PATH': ''},
            'exact fresh diagnostic invocation')
    require(row['inputs_before'] == inputs, 'exact fresh diagnostic input identities')
    g = metadata['generation']
    require(row['driver']['sha256'] == pins[f'whole-proof-attacks-v3-c{g}/guard.py'],
            'reviewed fresh guard')
    shared = directory.parents[2] / 'whole-proof-attacks-completion-v3'
    plan = load(shared / 'plan.json')
    caps = (dict(wall=1800, cpu=1800, rss=1024**3,
                 file=min(24 * 1024**3 + 65536, plan['proofs'][str(g)]['bytes'] + 32 * 1024**2))
            if kind == 'helper' else dict(wall=7500, cpu=7500, rss=6 * 1024**3, file=32 * 1024**2))
    require(row['caps'] == caps and row['sampled_scope_disk_cap'] == 26 * 1024**3
            and row['free_floor'] == 8 * 1024**3 and row['sample_interval_seconds'] == 1,
            'unchanged fresh diagnostic resource contract')
    for key, name in (('shared_profile', 'plan.json'), ('shared_driver', 'resources.py'), ('admission', 'admission.json')):
        same(shared / name, row[key])
    require(row['per_stream_log_bytes'] == 1024**2 and
            all(row['files'][name]['bytes'] <= 1024**2 for name in ('stdout', 'stderr')), 'bounded streams')
    registered = load(shared / 'native-processes' / (directory.name + '.json'))
    admission = load(shared / 'admission.json')
    require(registered['pid'] == registered['pgid'] == row['pid']
            and registered['parent'] == admission['children'][str(g)]
            and registered['coordinator'] == admission['pid'] and registered['argv'] == row['argv'],
            'fresh native process registered before execution')
    coordinator = load(shared / 'run-1/receipt.json')
    require(all(type(value) is int for value in (row['started_ns'], row['ended_ns'], registered['registered_ns']))
            and coordinator['started_ns'] <= row['started_ns'] <= registered['registered_ns']
            <= row['ended_ns'] <= coordinator['ended_ns'], 'fresh command belongs to coordinator interval')
    command_samples(directory, row)
    return row


def command_samples(directory, row):
    count = peak = 0
    latest = None
    with (directory/'resources.jsonl').open() as stream:
        while line := stream.readline(1024**2+1):
            require(len(line) <= 1024**2, 'bounded command resource row')
            observed = json.loads(line)
            require(observed['shared_stop'] is False and 0 <= observed['elapsed'] <= row['caps']['wall']
                    and 0 <= observed['rss_bytes'] <= row['caps']['rss'], 'actual command sample obeys original caps')
            require(all(type(p['pid']) is int and p['pid'] > 0 and type(p['rss_bytes']) is int
                        and p['rss_bytes'] >= 0 for p in observed['processes'])
                    and len({p['pid'] for p in observed['processes']}) == len(observed['processes'])
                    and sum(p['rss_bytes'] for p in observed['processes']) == observed['rss_bytes'],
                    'actual command process RSS accounting')
            require(row['started_ns'] <= observed['time_ns'] <= row['ended_ns']
                    and (latest is None or latest['time_ns'] <= observed['time_ns']), 'command sample time binding')
            latest = observed
            count += 1
            peak = max(peak, observed['rss_bytes'])
    require(count > 0 and peak == row['sampled_peak_rss_bytes'] and latest == row['latest_sample'],
            'complete command samples and peak binding')


def replay_rows(base, number, version, rows, positive, pins, old_index=None):
    attacks = base / f'whole-proof-attacks-v{version}-c{number}'
    work = attacks / f'whole-c{number}'
    helper = attacks / 'target-helper/release/whole-proof-mutator'
    index, noun = work / 'index.json', work / 'result.dag'
    proof = base / f'whole-proof/attempts/c{number}-selfbuild-1/proof.joysc'
    compiler = base / f'whole-proof/attempts/c{number}-fresh-verification-1/compiler.dag'
    proof_identity, result = positive['proof'], positive['verification']
    same(proof, proof_identity)
    if version == 2:
        diagnostic(attacks / f'attempts/whole-c{number}-index',
                   [helper, 'index', proof, index, noun, proof_identity['sha256']], 0,
                   dict(generation=number, complete_proof=True),
                   {str(helper): identity(helper), str(proof): proof_identity}, 'helper', version, pins)
    else:
        require(old_index is not None, 'authenticated original index required')
        same(index, old_index['index'])
        same(noun, old_index['result'])
    indexed = load(index)
    require(indexed['schema'] == 'trident/whole-proof-mutation-index/v1'
            and indexed['source_sha256'] == proof_identity['sha256']
            and indexed['source_bytes'] == proof_identity['bytes']
            and indexed['records'] == result['records']
            and indexed['decoded_bytes'] == result['transport']['decoded_bytes'], 'complete original index')
    construction_inputs = {str(path): identity(path) for path in (helper, index, noun)}
    construction_inputs[str(proof)] = proof_identity
    preparation = load(attacks / 'preparation.json')
    frozen = attacks / 'inputs/frozen'
    profile = load(base / 'whole-proof/profile.json')
    files(frozen, load(base / 'whole-proof/preparation.json')['files'])
    for row in rows:
        name, negative = row['name'], row['name'] != 'rechain'
        target_compiler, target_job, mode, context = C.case_recipe(name, number, preparation, frozen, attacks)
        if version == 3 and name == 'rebound-job-limit':
            variant = preparation['generations'][str(number)]['variants']['job-limit']
            coords = preparation['compiler_coordinates'][str(variant['compiler_generation'])]
            context = admitted_context(coords, variant['admission'], target_compiler, target_job)
        certificate = work / ('certificate-' + name + '.joysc') if mode else proof
        output = work / (name + '.dag')
        directory = attacks / f'attempts/whole-c{number}-verify-{name}'
        require(within(attacks, row['verification_receipt']) == directory / 'receipt.json', 'distinct verifier receipt path')
        joy = base / 'production-install/installed/bin/joy'
        argv = [joy, 'verify-artifact', target_compiler, '--input', target_job, '--proof', certificate,
                '--output', output, '--emit', 'program', *profile_flags(profile)]
        inputs = {str(path): identity(path) for path in (joy, target_compiler, target_job)}
        inputs[str(certificate)] = row['certificate']
        if negative:
            argv.append('--force')
            inputs[str(output)] = row['protected_output']
        diagnostic(directory, argv, 1 if negative else 0,
                   dict(generation=number, expected_error=ERRORS[name] if negative else None),
                   inputs, 'verify', version, pins)
        if mode is None:
            require(row['certificate'] == proof_identity, 'unaltered whole proof binding')
        if negative:
            require(row['error'] == ERRORS[name] and (directory / 'stdout').stat().st_size == 0
                    and row['error'] in (directory / 'stderr').read_text(), 'specific required rejection')
            same(output, row['protected_output'])
        else:
            require(version == 2 and name == 'rechain' and row['certificate'] == proof_identity,
                    'only explicitly reused byte-identical rechain control')
            same(output, identity(compiler))
            response = load(directory / 'stdout')
            require(response['ok'] is True and response['schema'] == 'joy/artifact-verification/v1'
                    and 'prover_observations' not in response['verification']
                    and semantic(response['verification']) == semantic(result), 'actual rechain semantic agreement')
        recipe = row['recipe']
        require((recipe is None) == (mode is None), 'required construction')
        if mode is None:
            continue
        construction_dir = attacks / f'attempts/whole-c{number}-construct-{name}'
        require(within(attacks, recipe['construction_receipt']) == construction_dir / 'receipt.json',
                'distinct named construction receipt')
        diagnostic(construction_dir, [helper, 'mutate', proof, index, noun, certificate, mode, *context],
                   0, dict(generation=number, mode=mode), construction_inputs, 'helper', version, pins)
        require(recipe['certificate'] == row['certificate'] and recipe['mode'] == mode
                and recipe['context'] == context, 'exact construction request and context')
        built = load(construction_dir / 'stdout')
        require(built['mode'] == mode and built['wire_bytes'] == row['certificate']['bytes']
                and built['source_records'] == indexed['records'], 'actual helper output dimensions')
        if mode in ('drop-first', 'swap-first-two', 'omit-completion', 'truncate-last-byte', 'trailing-byte'):
            require(built['source_decoded_bytes'] == indexed['decoded_bytes'], 'framing source dimensions')
        else:
            require(built['source_sha256'] == proof_identity['sha256'], 'semantic original whole source')
        needed = name.startswith('valid-output-')
        require(('canonical_output_sidecar' in recipe) == needed, 'all canonical output sidecars required')
        if needed:
            path = within(attacks, recipe['canonical_output_sidecar']['path'])
            require(path == certificate.with_suffix('.dag'), 'exact canonical sidecar path')
            same(path, recipe['canonical_output_sidecar'])
    require({str(path): identity(path) for path in (helper, proof, index, noun)} == construction_inputs,
            'original proof/index/helper/result stable after case replay')
    return dict(index=dict(path=str(index), **identity(index)), result=dict(path=str(noun), **identity(noun)))


def review(base, number, positive, pins):
    old_root, new_root = (base / f'whole-proof-attacks-v{v}-c{number}' for v in (2, 3))
    old_path, new_path = (p / f'whole-c{number}/receipt.json' for p in (old_root, new_root))
    old, new = load(old_path), load(new_path)
    profile = load(base / 'whole-proof/profile.json')
    for suite, version in ((old, 2), (new, 3)):
        schema, status = (('trident/whole-self-build-certificate-checks/v2', 'failed') if version == 2
                          else ('trident/whole-self-build-certificate-completion/v3', 'passed-completion'))
        require(suite['schema'] == schema and suite['status'] == status and suite['generation'] == number
                and suite['profile'] == profile and suite['original_proof'] == positive['proof'], 'distinct enclosing attempt states')
        require(suite['immutable_inputs_before'] == suite['immutable_inputs_after'], 'suite immutable inputs')
        for path, expected in suite['immutable_inputs_after'].items():
            same(Path(path), expected)
    names(old['rejections'], PRIOR, 'reused v2 cases')
    names(new['rejections'], FRESH, 'fresh v3 cases')
    names(old['controls'], ('original-fresh-verification', 'rechain'), 'reused controls')
    require('controls' not in new, 'v3 has no freshly executed controls')
    original = old['controls'][0]
    verifier = base / f'whole-proof/attempts/c{number}-fresh-verification-1'
    require(original['reused_existing_actual_control'] is True and original['receipt'] == str(verifier/'receipt.json'),
            'explicit original fresh verification reuse')
    same(verifier/'receipt.json', original['receipt_identity'])
    require(original['certificate'] == positive['proof'] and original['output'] == positive['compiler'],
            'original control exact complete proof/output')
    old_index = replay_rows(base, number, 2, [old['controls'][1], *old['rejections']], positive, pins)
    expected_prior = dict(schema='trident/prior-complete-proof-cases/v1', status='passed-selected-cases',
                          scope='Case-level replay only; immutable enclosing v2 suite remains failed; no fresh case execution',
                          generation=number, original_suite=dict(path=str(old_path), **identity(old_path)),
                          original_checker=dict(path=str(CHECKER), **identity(CHECKER)),
                          original_proof=positive['proof'], profile=profile, controls=old['controls'],
                          rejections=old['rejections'], **old_index)
    require(new['prior_cases'] == expected_prior, 'embedded prior result exactly matches independent original replay')
    replay_rows(base, number, 3, new['rejections'], positive, pins, old_index)
    return dict(schema='trident/composite-complete-proof-cases/v1', status='passed-case-replay',
                original_v2_suite_status='failed', original_v2_suite=identity(old_path),
                fresh_v3_suite=identity(new_path), reused_controls=2,
                reused_rejections=list(PRIOR), fresh_rejections=list(FRESH), distinct_rejections=23,
                source_provenance='Each row replayed against its original v2 or v3 commands and registrations.')
