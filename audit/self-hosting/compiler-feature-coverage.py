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


COUNT_KEYS = ('passed', 'failed', 'ignored', 'filtered_out')


def sum_counts(rows):
    return {key: sum(row[key] for row in rows) for key in COUNT_KEYS}


def archive_bytes(directory, row, path_key='path', encoding_key='encoding'):
    need(row[encoding_key] == 'gzip', 'gate log encoding')
    path = bound(directory, row, path_key, 'stored_sha256')
    with gzip.open(path, 'rb') as stream:
        raw = stream.read(16 * 1024 * 1024 + 1)
    need(len(raw) <= 16 * 1024 * 1024, 'gate log size bound')
    need(hashlib.sha256(raw).hexdigest() == row['raw_sha256'], 'decompressed gate log differs')
    return raw


def gate_targets(root, directory, gate):
    capture = gate['cargo_metadata']
    need(capture['argv'] == ['cargo', 'metadata', '--format-version', '1', '--no-deps', '--locked', '--offline'],
         'metadata command differs')
    for file, key in [('Cargo.toml', 'cargo_toml_sha256'), ('Cargo.lock', 'cargo_lock_sha256')]:
        need(sha(root / file) == capture[key], 'Cargo input changed: ' + file)
    metadata = json.loads(archive_bytes(directory, capture))
    prefix = capture['cwd'].replace('\\', '/').rstrip('/') + '/'
    packages = [p for p in metadata['packages'] if p['manifest_path'].replace('\\', '/') == prefix + 'Cargo.toml']
    need(len(packages) == 1 and metadata['workspace_default_members'] == [packages[0]['id']],
         'metadata does not describe the complete default package')
    targets = {}
    for target in packages[0]['targets']:
        kind = target['kind'][0]
        if kind not in ('lib', 'bin', 'test'):
            continue
        source = target['src_path'].replace('\\', '/')
        need(source.startswith(prefix), 'metadata target source escapes package')
        source = source[len(prefix):]
        need(relative(root, source).is_file(), 'metadata target source missing')
        if target['test']:
            targets[kind + ':' + target['name']] = source
        if kind == 'lib' and target['doctest']:
            targets['doc:' + target['name']] = source
    need(set(targets) == set(gate['coverage']['metadata_targets']) and
         len(targets) == len(gate['coverage']['metadata_targets']), 'metadata target coverage differs')
    # Auto-discovered integration targets must also match the current checkout.
    need({v for k, v in targets.items() if k.startswith('test:')} ==
         {p.relative_to(root).as_posix() for p in (root / 'tests').glob('*.rs')}, 'integration target set changed')
    return targets


def log_suites(raw, targets):
    text = re.sub(r'\x1b\[[0-9;]*m', '', raw.decode('utf-8'))
    need(not re.search(r'^\s*warning(?:\[.*?\])?:', text, re.M), 'Rust warning in gate log')
    headers = list(re.finditer(r'^\s*(?:Running (?:unittests )?([^\n]+?) \([^\n]*\)|Doc-tests ([\w-]+))\s*$', text, re.M))
    suites = []
    for index, header in enumerate(headers):
        if header[2]:
            candidates = ['doc:' + header[2]]
        else:
            source = header[1].replace('\\', '/')
            candidates = [key for key, value in targets.items() if value == source and not key.startswith('doc:')]
        need(len(candidates) == 1 and candidates[0] in targets, 'unknown or ambiguous logged test target')
        tail = text[header.end():headers[index + 1].start() if index + 1 < len(headers) else len(text)]
        summaries = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', tail, re.M)
        need(len(summaries) == 1, 'missing, failed or repeated suite summary')
        passed, failed, ignored, measured, filtered = map(int, summaries[0])
        need(failed == measured == filtered == 0, 'failed or filtered Rust suite')
        cases = re.findall(r'^test (.+?) \.\.\. (ok|ignored(?:,.*)?|FAILED)$', tail, re.M)
        # Concurrent diagnostic stderr can interleave individual case lines.
        # Full-suite counts come from the terminal libtest summary; the selected
        # mapped cases are independently required by rust_evidence below.
        need(len({name for name, _ in cases}) == len(cases) and
             sum(state == 'ok' for _, state in cases) <= passed and
             sum(state.startswith('ignored') for _, state in cases) <= ignored,
             'test names disagree with suite summary')
        suites.append(dict(target=candidates[0], passed=passed, failed=failed, ignored=ignored, filtered_out=filtered))
    need(suites and len({s['target'] for s in suites}) == len(suites), 'missing or reused logged test target')
    return suites


def command_targets(command, targets):
    argv = command['argv']
    need(argv[:5] == ['cargo', 'test', '--release', '--locked', '--offline'] and
         argv.count('--') == 1, 'unexpected Rust gate command/filter')
    separator = argv.index('--')
    arguments = argv[separator + 1:]
    need(len(arguments) == len(set(arguments)) and
         sum(bool(re.fullmatch(r'--test-threads=[1-9][0-9]*', arg)) for arg in arguments) == 1 and
         all(arg == '--nocapture' or re.fullmatch(r'--test-threads=[1-9][0-9]*', arg) for arg in arguments),
         'unexpected Rust gate command/filter')
    selected, options, index = [], argv[5:separator], 0
    while index < len(options):
        option = options[index]
        if option in ('--lib', '--bins', '--doc'):
            prefix = {'--lib': 'lib:', '--bins': 'bin:', '--doc': 'doc:'}[option]
            selected.extend(t for t in targets if t.startswith(prefix))
        else:
            need(option == '--test' and index + 1 < len(options), 'unsupported Rust target selector')
            index += 1
            selected.append('test:' + options[index])
        index += 1
    need(set(selected) == set(command['targets']) and len(selected) == len(set(selected)) == len(command['targets']) and
         set(selected) <= set(targets), 'command/target binding differs')
    return set(selected)


def split_gate(root, directory, gate, mapping):
    need(gate['status'] == 'passed' and gate['checks'] == {'warnings': 0, 'failures': 0, 'source_unchanged': True},
         'split Rust gate not green')
    need(gate.get('committed_sources_verified') is True, 'committed gate sources not verified')
    need(gate['committed_source_revision'] == mapping['source_revision'] and
         gate['source_inventory_sha256'] == mapping['inventory']['sha256'], 'gate/source binding differs')
    need(sha(relative(directory, gate['source_inventory_path'])) == gate['source_inventory_sha256'], 'gate inventory changed')
    sources = {row['logical_path']: row for row in gate['sources']}
    need(len(sources) == len(gate['sources']) and set(sources) == set(mapping['sources']), 'gate source set differs')
    for name, row in sources.items():
        expected = mapping['sources'][name]
        need(row['path'] == expected['path'] and row['sha256'] == expected['sha256'] and
             row['bytes'] == relative(root, row['path']).stat().st_size, 'gate source identity differs')
    source_map = {name: {k: row[k] for k in ('sha256', 'bytes')} for name, row in sources.items()}
    need(hashlib.sha256(json.dumps(source_map, sort_keys=True, separators=(',', ':')).encode()).hexdigest() ==
         gate['source_map_sha256'], 'source map digest differs')
    targets = gate_targets(root, directory, gate)
    ids, paths, hashes, covered, latest, logs = set(), set(), set(), set(), {}, []
    required = {p.relative_to(root).as_posix() for p in (root / 'tests').rglob('*') if p.is_file()}
    required |= {'Cargo.toml', 'Cargo.lock', 'lib/std/compiler/nox/lexer.tri'}
    versions, used_versions = {}, set()
    for version in gate.get('source_versions', []):
        key = (version['source_path'], version['raw_sha256'], version['superseded_by_command'])
        need(key not in versions, 'duplicate test source version')
        versions[key] = archive_bytes(directory, version)
    buckets = {'primary': [], 'supplemental': []}
    for command in gate['commands']:
        need(command['id'] not in ids and command['raw_log'] not in paths and command['raw_sha256'] not in hashes,
             'reused Rust command or log')
        ids.add(command['id']); paths.add(command['raw_log']); hashes.add(command['raw_sha256'])
        need(type(command['exit_code']) is int and command['exit_code'] == 0, 'failed Rust subcommand')
        need(command['cwd'] == gate['cargo_metadata']['cwd'] and set(command['env']) == {'CARGO_TARGET_DIR'}, 'command environment differs')
        selected = command_targets(command, targets)
        kind = command['kind']
        need(kind in buckets, 'unknown command accounting kind')
        need(not (covered & selected) if kind == 'primary' else selected <= covered, 'duplicate primary or unbound supplemental targets')
        if kind == 'primary':
            covered.update(selected)
        raw = archive_bytes(directory, command, 'raw_log', 'log_encoding')
        suites = log_suites(raw, targets)
        need(suites == command['suite_results'] and {s['target'] for s in suites} == selected, 'logged target coverage differs')
        counts = sum_counts(suites)
        need(counts == command['counts'], 'command counts differ')
        buckets[kind].append(counts); logs.append(raw)
        need(required <= set(command['source_hashes']), 'test input source bindings incomplete')
        for path, identity in command['source_hashes'].items():
            if path in latest and latest[path] != identity:
                need(kind == 'supplemental' and any(targets[t] == path for t in selected), 'source changed without matching full-target rerun')
                key = (path, latest[path], command['id'])
                need(key in versions, 'superseded test source is not retained')
                used_versions.add(key)
            latest[path] = identity
    need(used_versions == set(versions), 'unbound historical test source version')
    for path, identity in latest.items():
        need(sha(relative(root, path)) == identity, 'gate test source changed: ' + path)
    coverage = gate['coverage']
    need(covered == set(targets) == set(coverage['covered_targets']) and coverage['missing_targets'] == [], 'incomplete primary target coverage')
    primary, supplemental = (sum_counts(buckets[k]) for k in ('primary', 'supplemental'))
    observed = sum_counts([primary, supplemental])
    need(gate['counts'] == dict(primary=primary, supplemental=supplemental, observed=observed, unique=primary), 'gate totals double-count or differ')
    need(coverage['unique_registered_tests'] == sum(primary[k] for k in ('passed', 'failed', 'ignored')) and
         coverage['observed_test_executions'] == observed['passed'] + observed['failed'], 'execution/registration counts differ')
    combined = gate['combined_log']
    need(combined['concatenation_only'] is True and combined['ordered_commands'] == [c['id'] for c in gate['commands']] and
         combined['ordered_raw_logs'] == [c['raw_log'] for c in gate['commands']], 'combined log provenance differs')
    need(archive_bytes(directory, combined) == b''.join(logs), 'combined log is not exact command concatenation')
    return primary, len(covered), b''.join(raw for raw, command in zip(logs, gate['commands']) if command['kind'] == 'primary')


def rust_evidence(path, tests, counts, primary_raw=None):
    """Read complete selected test binaries; do not infer full Cargo exit status."""
    if path is None:
        return {'status': 'not supplied', 'missing_test_count': len(tests)}
    if primary_raw is not None:
        text = primary_raw.decode('utf-8')
    elif path.suffix == '.gz':
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
    gate_path = bound(root, evidence_meta, 'gate_receipt', 'gate_receipt_sha256')
    gate, primary_raw = read(gate_path), None
    if gate.get('schema') == 'trident/split-rust-gate/v1':
        need(evidence_meta.get('schema') == gate['schema'] and 'command' not in evidence_meta, 'split gate schema/command metadata')
        counts, suites, primary_raw = split_gate(root, gate_path.parent, gate, mapping)
        need(evidence_meta['command_ids'] == [c['id'] for c in gate['commands']] and
             evidence_meta['counts'] == gate['counts'], 'split command/count metadata differs')
        expected = dict(completed_suites=suites, passed=counts['passed'], ignored=counts['ignored'], failed=0,
                        rust_warnings=0, log_sha256=gate['combined_log']['stored_sha256'],
                        decompressed_log_sha256=gate['combined_log']['raw_sha256'])
    else:
        need('schema' not in gate, 'unknown Rust gate schema')
        need(gate['status'] == 'passed' and gate['exit_code'] == 0 and
             gate['failed'] == 0 and gate['rust_warnings'] == 0, 'full Rust gate not green')
        need(gate['source_commit'] == mapping['source_revision'] and
             gate['source_inventory_sha256'] == meta['sha256'], 'gate/source binding differs')
        need(gate['command'] == evidence_meta['command'], 'gate command differs')
        expected = gate
    for key in ['completed_suites', 'passed', 'ignored', 'failed', 'rust_warnings', 'log_sha256', 'decompressed_log_sha256']:
        need(expected[key] == evidence_meta[key], f'gate evidence differs: {key}')
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
    evidence = rust_evidence(log_path, tests, mapping['rust_evidence']['suite_counts'], primary_raw)
    if require_rust:
        need(evidence['status'] == 'selected suites passed',
             'selected Rust suites not complete and green: ' + json.dumps(evidence))
    return {'status': 'mapping integrity passed', 'features': len(inv['features']),
            'modules': len(inv['modules']), 'rust_test_references': len(tests),
            'selected_installed_cases': len(selected), 'historical_receipts': len(runners),
            'rust_evidence': evidence, 'acceptance': 'SH3 feature criterion met for reviewed map; SH4/SH5, C2/C3 corpora and fixed point are not checked by this mapping'}


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
