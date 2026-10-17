"""Retain read-only classification evidence in this review scope only."""
import gzip
import hashlib
import json
from pathlib import Path
import stat
import time

ROOT = Path(__file__).resolve().parent
BASE = ROOT.parent
ATTACK = BASE / 'whole-proof-attacks-v2-c2'


def require(value, message):
    if not value:
        raise ValueError(message)


def identity(path):
    require(stat.S_ISREG(path.lstat().st_mode), 'regular file: ' + str(path))
    require(path.stat().st_size <= 16 * 1024 * 1024, 'small evidence cap')
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        while block := stream.read(65536):
            digest.update(block)
    return dict(bytes=path.stat().st_size, sha256=digest.hexdigest())


def load(path):
    require(path.stat().st_size <= 4 * 1024 * 1024, 'JSON cap')
    return json.loads(path.read_text())


def retain(path):
    key = str(path.relative_to(BASE))
    target = ROOT / 'retained' / key
    target.parent.mkdir(parents=True, exist_ok=True)
    before = identity(path)
    with path.open('rb') as source, target.open('xb') as output:
        while block := source.read(65536):
            output.write(block)
    require(identity(target) == before == identity(path), 'retained bytes')
    return key, before


def main():
    comparison = load(ROOT / 'comparison.json')
    require(comparison['status'] == 'passed-readonly-comparison', 'whole comparison')
    whole = dict(zip(comparison['comparison']['paths'],
                     [dict(bytes=n, sha256=h) for n, h in zip(
                         comparison['comparison']['bytes'], comparison['comparison']['sha256'])]))
    selected = set()
    command_receipts = {}
    for role in ('construct', 'verify'):
        directory = ATTACK / f'attempts/whole-c2-{role}-rebound-job-limit'
        receipt = load(directory / 'receipt.json')
        require(receipt['inputs_before'] == receipt['inputs_after'], 'unchanged command inputs')
        for name, expected in receipt['files'].items():
            require(identity(directory / name) == expected, 'original command log identity')
            selected.add(directory / name)
        for path, expected in receipt['inputs_before'].items():
            actual = whole.get(path) or identity(Path(path))
            require(actual == expected, 'original command input identity')
        selected.add(directory / 'receipt.json')
        command_receipts[role] = receipt
    require(command_receipts['construct']['exit_code'] == 0, 'completed constructor')
    require(command_receipts['verify']['exit_code'] == 1, 'actual negative exit')
    stderr = (ATTACK / 'attempts/whole-c2-verify-rebound-job-limit/stderr').read_text()
    require('format/context mismatch' in stderr and 'semantic record: Key' not in stderr,
            'preserve actual earlier rejection')
    suite = load(ATTACK / 'whole-c2/receipt.json')
    require(suite['status'] == 'failed', 'failed suite remains failed')
    require(len(suite['rejections']) == 9 and len(suite['controls']) == 2, 'recorded completed rows')
    require(all(row['name'] != 'rebound-job-limit' for row in suite['rejections']),
            'intended case has no successful row')
    preparation = load(ATTACK / 'preparation.json')
    admission = preparation['generations']['2']['variants']['job-limit']['admission']
    recipe_budget = int(command_receipts['construct']['argv'][-2])
    require((recipe_budget, admission['limits']['reductions']) == (20000000000, 19999999999),
            'exact context disagreement')
    common = BASE / 'whole-proof-attacks-parallel-v2'
    require(load(common / 'shutdown.json')['all_owned_groups_empty'] is True, 'recorded shutdown')
    require(load(common / 'shutdown.json')['remaining'] == [], 'recorded empty owned groups')
    for relative in ('preparation.json', 'whole_suite.py', 'guard.py', 'helper/Cargo.toml',
                     'helper/Cargo.lock', 'whole-c2/receipt.json', 'whole-c2/index.json'):
        selected.add(ATTACK / relative)
    selected.update((ATTACK / 'helper').glob('*.rs'))
    for relative in ('resources.py', 'run.py', 'sources.json', 'plan.json', 'stop.json',
                     'shutdown.json', 'run-1/receipt.json', 'transition.json'):
        selected.add(common / relative)
    production = BASE / 'production-install/joy/rs/structured/certificate'
    selected.update(production / p for p in ('admission.rs', 'types.rs', 'transport.rs'))
    selected.update((production / 'transport').glob('*.rs'))
    archived = BASE / 'final-integration/trident/audit/self-hosting/bootstrap-results/final-tooling-integration/negative-v2-stop'
    archive_checks = {}
    for packed in sorted(archived.rglob('*.gz')):
        original = BASE / str(packed.relative_to(archived))[:-3]
        with gzip.open(packed, 'rb') as stream:
            decoded = stream.read(4 * 1024 * 1024 + 1)
        require(len(decoded) <= 4 * 1024 * 1024, 'archive expansion cap')
        expected = identity(original)
        require(dict(bytes=len(decoded), sha256=hashlib.sha256(decoded).hexdigest()) == expected,
                'archived original replay')
        archive_checks[str(packed.relative_to(BASE))] = dict(packed=identity(packed), original=expected)
        selected.update((packed, original))
    selected.add(archived / 'observation.json')
    retained = dict(retain(path) for path in sorted(selected))
    partials = {}
    for rel in ('whole-proof-attacks/whole-c2/certificate-rechain.joysc',
                'whole-proof-attacks-v2-c1/whole-c1/certificate-rebound-job-limit.joysc'):
        path = BASE / rel
        s = path.lstat()
        require(stat.S_ISREG(s.st_mode), 'partial regular file')
        partials[rel] = dict(bytes=s.st_size, inode=s.st_ino, mtime_ns=s.st_mtime_ns,
                            content_read=False, unchanged_and_still_charged=True)
    result = dict(
        schema='trident/completed-mutant-classification/v1',
        status='passed-readonly-classification', created_ns=time.time_ns(),
        scope='one completed reproducible owned C2 temporary; future reclamation review only',
        sources={p.name: identity(p) for p in (ROOT/'compare.py', ROOT/'retain.py')},
        comparison=identity(ROOT/'comparison.json'), retained=retained,
        archive_replay=archive_checks, partials=partials,
        classification=dict(
            original_proof=whole[str(BASE/'whole-proof/attempts/c2-selfbuild-1/proof.joysc')],
            completed_mutant=whole[str(ATTACK/'whole-c2/certificate-rebound-job-limit.joysc')],
            constructor_exit=0, verifier_exit=1,
            actual_error=stderr.strip(), intended_error='semantic record: Key',
            recipe_budget=recipe_budget, admitted_job_budget=admission['limits']['reductions'],
            original_suite_status='failed', intended_semantic_case_status='failed',
            recorded_distinct_rejections=9, recorded_controls=2,
            complete_payload_and_nonchain_header_equality=True,
            original_and_mutant_whole_sha256_rechecked=True,
            hemera_chain_rechecked=False, evaluator_executed=False),
        disposition=dict(
            eligible_for_separately_reviewed_poststop_transition=True,
            existing_v2_reclaim_refuses_stop=True,
            existing_v2_sources_and_failed_receipts_must_remain_immutable=True,
            failed_case_must_not_be_counted_as_semantic_rejection=True,
            retain_exact_original_proof_recipe_helper_inputs_sources_and_failed_logs=True,
            future_transition_must_recheck_exact_path_inode_size_sha256_and_no_live_owners=True,
            only_completed_c2_mutant_may_be_named=True,
            deletion_performed=False, original_files_modified=False,
            incomplete_partials_must_remain_charged=True,
            v3_composite_acceptance_requires_explicit_prior_result_provenance=True))
    with (ROOT/'classification.json').open('x') as output:
        json.dump(result, output, indent=2)
        output.write('\n')
    print(json.dumps(dict(status=result['status'], retained_files=len(retained),
                         archived_replays=len(archive_checks), classification=identity(ROOT/'classification.json'))))


if __name__ == '__main__':
    main()
