#!/usr/bin/env python3
"""Check the reviewed inventory/case map; never build or run a compiler.

Exit zero means mapping integrity (and selected Rust suite evidence only when
--require-rust-pass is supplied). Review of the semantic mapping remains necessary; it does not establish C2/fixed-point acceptance.
Historical JSON receipts are hash-bound evidence, not re-executed artifacts.
"""
import argparse
import hashlib
import gzip
import json
import re
import sys
from pathlib import Path


class Invalid(ValueError):
    pass


def need(condition, message):
    if not condition:
        raise Invalid(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_text())


def relative(root, text):
    path = Path(text)
    need(not path.is_absolute() and '..' not in path.parts, f'unsafe map path: {text}')
    return root / path


def bound(root, row, path_key='file', hash_key='sha256'):
    path = relative(root, row[path_key])
    need(sha(path) == row[hash_key], f'changed {path_key}: {row[path_key]}')
    return path


def compiler_identity(receipt):
    if 'compiler' in receipt:
        value = receipt['compiler']
    elif 'compiler_sha256' in receipt:
        value = {'sha256': receipt['compiler_sha256'], 'particle': receipt['compiler_particle']}
    elif 'artifacts' in receipt:
        value = receipt['artifacts']['compiler']
    else:
        value = next(x for x in receipt['files'] if x['path'] == 'c1.dag')
    return {key: value[key] for key in ['sha256', 'particle']}


def rust_evidence(path, tests, counts):
    """Read complete selected test binaries; do not infer full Cargo exit status."""
    if path is None:
        return {'status': 'not supplied', 'missing_test_count': len(tests)}
    if path.suffix == '.gz':
        with gzip.open(path, 'rt') as stream:
            text = stream.read()
    else:
        text = path.read_text()
    log = re.sub(r'\x1b\[[0-9;]*m', '', text)
    sections = {}
    for match in re.finditer(r'Running tests/([\w]+)\.rs[^\n]*\n', log):
        tail = log[match.end():]
        end = re.search(r'\n\s*(?:Running |Doc-tests )', tail)
        sections[match[1]] = tail[:end.start()] if end else tail
    missing = []
    incomplete = []
    for binary in sorted({test['binary'] for test in tests.values()}):
        section = sections.get(binary, '')
        summary = re.search(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;', section)
        if not summary or int(summary[1]) != counts[binary]:
            incomplete.append(binary)
        for name, row in tests.items():
            if row['binary'] == binary and not re.search(
                r'^test ' + re.escape(row['test']) + r' \.\.\. ok$', section, re.M
            ):
                missing.append(name)
    return {'path': str(path), 'sha256': sha(path),
            'status': 'selected suites passed' if not missing and not incomplete else 'incomplete or failed',
            'missing': sorted(missing), 'incomplete_suites': incomplete,
            'scope': 'Rust-built C1 test binaries only; no installed-C2 or whole Cargo exit claim'}


def known_only_calls(inv, name, row):
    owner, member = name.rsplit('.', 1)
    paths = {name, owner.rsplit('.', 1)[-1] + '.' + member}
    count = sum(n for module_owner, module in inv['modules'].items()
                for call, n in module['calls'].items()
                if call in paths or (module_owner == owner and call == member))
    need(count == row['syntactic_calls_to_full_or_short_owner'] == 0,
         f'known-only intrinsic now has syntactic callers: {name}')
    member_count = sum(n for module in inv['modules'].values()
                       for call, n in module['calls'].items()
                       if call == member or call.endswith('.' + member))
    need(member_count == row['syntactic_calls_with_same_final_member'] == 0,
         f'known-only intrinsic member now appears in calls: {name}')


def validate(root, mapping, inventory, log_path=None, require_rust=False):
    need(mapping['schema'] == 'trident/compiler-feature-coverage/v1', 'wrong map schema')
    meta = mapping['inventory']
    need(sha(inventory) == meta['sha256'], 'inventory changed: recapture and review coverage')
    inv = read(inventory)
    for key in ['module_count', 'functions', 'source_bytes', 'source_lines', 'roots', 'scope']:
        need(inv[key] == meta[key], f'inventory metadata mismatch: {key}')
    need(len(inv['features']) == meta['feature_kinds'], 'feature kind count differs')
    need(set(inv['features']) == set(mapping['features']), 'feature keys lack exact coverage')
    need(set(inv['modules']) == set(mapping['sources']), 'closure module set differs')
    for name, row in mapping['sources'].items():
        source = bound(root, row, 'path')
        module = inv['modules'][name]
        need(row['path'] == module['path'], f'changed source path: {name}')
        need(row['source_blake3'] == module['source_blake3'], f'inventory identity differs: {name}')
        need(source.stat().st_size == module['source_bytes'], f'source length differs: {name}')
    evidence_meta = mapping['rust_evidence']
    gate = read(bound(root, evidence_meta, 'gate_receipt', 'gate_receipt_sha256'))
    need(gate['status'] == 'passed' and gate['exit_code'] == 0 and
         gate['failed'] == 0 and gate['rust_warnings'] == 0, 'full Rust gate not green')
    need(gate['source_commit'] == mapping['source_revision'] and
         gate['source_inventory_sha256'] == meta['sha256'], 'gate/source binding differs')
    need(gate['command'] == evidence_meta['command'], 'gate command differs')
    for key in ['completed_suites', 'passed', 'ignored', 'failed', 'rust_warnings',
                'log_sha256', 'decompressed_log_sha256']:
        need(gate[key] == evidence_meta[key], f'gate evidence differs: {key}')
    archive = bound(root, evidence_meta, 'log', 'log_sha256')
    with gzip.open(archive, 'rb') as stream:
        raw_sha = hashlib.sha256(stream.read()).hexdigest()
    need(raw_sha == evidence_meta['decompressed_log_sha256'], 'decompressed log differs')
    if log_path is not None:
        need(sha(log_path) in [evidence_meta['log_sha256'], raw_sha],
             'Rust log differs from retained full gate')
    tests = mapping['rust_tests']
    for name, row in tests.items():
        source = bound(root, row).read_text()
        need(re.search(r'#\[test\]\s*fn ' + re.escape(name) + r'\(', source),
             f'missing Rust test declaration: {name}')
    runners = {}
    for name, row in mapping['installed_runners'].items():
        need(relative(root, row['script']).is_file(), f'missing runner: {name}')
        receipt = read(bound(root, row, 'receipt', 'receipt_sha256'))
        need(len(receipt['observations']) == row['observations'], f'observation count: {name}')
        need(len(receipt['commands']) == row['commands'], f'command count: {name}')
        need(compiler_identity(receipt) == row['compiler'], f'compiler receipt binding: {name}')
        binary = receipt.get('binary_sha256', receipt.get('binary_sha256_start'))
        need(binary == row['binary_sha256'], f'Joy receipt binding: {name}')
        if 'binary_sha256_end' in receipt:
            need(binary == receipt['binary_sha256_end'], f'Joy changed during receipt: {name}')
        runners[name] = receipt
    selected = set()

    def installed(ref, positive):
        runner, case = ref.split(':', 1)
        need(runner in runners, f'unknown runner: {runner}')
        rows = [row for row in runners[runner]['observations'] if row.get('case') == case]
        need(len(rows) == 1, f'case is absent or ambiguous: {ref}')
        row = rows[0]
        result = row.get('compiler_execution', {})
        execution = result.get('execution', {})
        job = execution.get('compiler_job', {})
        need(result.get('ok') is True, f'compiler did not execute: {ref}')
        need(execution.get('program_particle') == mapping['installed_runners'][runner]['compiler']['particle'],
             f'case uses another compiler: {ref}')
        if positive:
            need('expected' in row and job.get('status') == 'success', f'not positive: {ref}')
            runtime = row.get('program_execution', row.get('runtime_execution', {}))
            need(runtime.get('ok') is True, f'no emitted-program execution: {ref}')
            need(runtime['execution']['program_particle'] == job.get('compiled_particle'),
                 f'executed another emitted program: {ref}')
        else:
            need(job.get('status') == 'compile_error' and job.get('compiled_particle') is None and row.get('diagnostics'),
                 f'not compile rejection: {ref}')
            need(row['diagnostics'] == job['diagnostics'], f'diagnostic mismatch: {ref}')
            need(row.get('previous_program_preserved', row.get('previous_output_preserved')) is True,
                 f'missing no-publication check: {ref}')
        selected.add(ref)

    for feature, row in mapping['features'].items():
        need(row['count'] == inv['features'][feature], f'feature count: {feature}')
        need(row['groups'], f'feature unmapped: {feature}')
        for group in row['groups']:
            need(group in mapping['groups'], f'unknown group: {group}')
    for name, group in mapping['groups'].items():
        for polarity in ['positive', 'rejection']:
            need(group[polarity + '_rust'] and group['installed_' + polarity],
                 f'incomplete evidence directions: {name}')
            for test in group[polarity + '_rust']:
                need(test in tests, f'unknown Rust evidence: {test}')
            for ref in group['installed_' + polarity]:
                installed(ref, polarity == 'positive')
    for requirement in mapping['supplemental_requirements'].values():
        for test in requirement['positive'] + requirement['rejection']:
            need(test in tests, f'unknown supplemental test: {test}')
    declarations = {owner + '.' + member: identity for owner, module in inv['modules'].items()
                    for member, identity in module['intrinsic_declarations'].items()}
    need(set(declarations) == set(mapping['intrinsics']), 'intrinsic declaration set changed')
    for name, row in mapping['intrinsics'].items():
        need(row['identity'] == declarations[name], f'intrinsic identity changed: {name}')
        need(row['rust_positive'] in tests and row['abi_rejection_component'] in tests,
             f'intrinsic missing Rust evidence: {name}')
        installed(row['installed_positive'], True)
        if row['installed_rejection']:
            installed(row['installed_rejection'], False)
        if 'syntactic_calls_to_full_or_short_owner' in row:
            known_only_calls(inv, name, row)
    for ref in mapping['required_regressions'].values():
        installed(ref, True)
    evidence = rust_evidence(log_path, tests, mapping['rust_evidence']['suite_counts'])
    if require_rust:
        need(evidence['status'] == 'selected suites passed',
             'selected Rust suites not complete and green: ' + json.dumps(evidence))
    return {'status': 'mapping integrity passed', 'features': len(inv['features']),
            'modules': len(inv['modules']), 'rust_test_references': len(tests),
            'selected_installed_cases': len(selected), 'historical_receipts': len(runners),
            'rust_evidence': evidence, 'acceptance': 'SH3 feature criterion met for reviewed map; SH4/SH5, C2 corpus and fixed point remain open'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    root = Path(__file__).resolve().parents[2]
    parser.add_argument('--map', type=Path, default=Path(__file__).with_suffix('.json'))
    parser.add_argument('--inventory', type=Path)
    parser.add_argument('--rust-log', type=Path)
    parser.add_argument('--require-rust-pass', action='store_true')
    args = parser.parse_args()
    try:
        mapping = read(args.map)
        inventory = args.inventory or root / mapping['inventory']['path']
        result = validate(root, mapping, inventory, args.rust_log, args.require_rust_pass)
        print(json.dumps(result, indent=2))
        return 0
    except (OSError, ValueError, KeyError, TypeError, StopIteration, EOFError) as error:
        print(json.dumps({'status': 'failed', 'error': str(error)}), file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
