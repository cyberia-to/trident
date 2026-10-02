"""Receipt consistency checks; this does not rerun Joy or verify certificates."""
import hashlib
import json

FIXTURE = '2f1a575a6fbc0704c268f1ae21667830a3c996df'
JOY_REVISION = '6e0ec4d8440e2521df08f442d64f54e667044716'
COMPILER = '76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8'
PARTICLE = '2eea2ac5f611012877b4e7291a3a6f534aee7281bb358a0b8e5fabe2ac1f9fbe'
JOY = '8f42591ece35f192ff6f2328a8360fe0f0959f48a173248b211cd0d8f4d984f9'
CASES = {
    'run-native-compiler': (402, 1197),
    'check-guest-constant-linking': (31, 140),
    'check-guest-function-imports': (32, 148),
    'check-guest-type-imports': (24, 110),
    'check-guest-intrinsics': (37, 157),
    'check-generated-compiler-profile': (21, 64),
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def identity(data):
    return {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def expected(row):
    return {key: row[key] for key in ('bytes', 'sha256')}


def check_flags(value):
    """Assertion flags must pass; optional oracle-coverage flags may be false."""
    names = {'complete_seed_output_equal', 'complete_output_checked', 'previous_output_preserved',
             'previous_program_preserved',
             'all_four_identity_limbs_checked'}
    if isinstance(value, dict):
        for key, child in value.items():
            if key in names:
                require(child is True, f'failed observation flag: {key}')
            check_flags(child)
    elif isinstance(value, list):
        for child in value:
            check_flags(child)


def check_fresh(payload, generation, compiler, binary, installed):
    prefix = f'fresh/c{generation}'
    receipt = json.loads(payload[f'{prefix}/receipt.json'])
    stdout = json.loads(payload[f'{prefix}/stdout'])
    producer = json.loads(payload[f'producer/c{generation}/receipt.json'])
    require(receipt['status'] == producer['status'] == 'passed', 'producer/fresh verification status')
    require(receipt['action'] == 'verify' and producer['action'] == 'prove', 'producer/verifier roles')
    require(receipt['generation'] == producer['generation'] == generation, 'proof generation')
    require(receipt['exit_code'] == producer['exit_code'] == 0, 'producer/verifier exit')
    for row in (receipt, producer):
        require(row['binary'] == row['binary_after'] == binary, 'proof binary binding')
        require(row['inputs_before'] == row['inputs_after'], 'proof immutable inputs')
        require(row['installed_source_receipt'] == installed, 'installed source receipt')
        require(row['environment']['PATH'] == '', 'proof PATH isolation')
    require(receipt['proof_input_after'] == expected(receipt['proof_input'])
            == producer['files']['proof.joysc'], 'producer to fresh proof identity')
    for name, row in receipt['files'].items():
        require(identity(payload[f'{prefix}/{name}']) == row, f'fresh verification file: {name}')
    for name in ('stdout', 'stderr'):
        require(identity(payload[f'producer/c{generation}/{name}']) == producer['files'][name],
                f'producer file: {name}')
    require(receipt['files']['compiler.dag'] == compiler, 'fresh compiler identity')
    require(stdout['ok'] is True and stdout['schema'] == 'joy/artifact-verification/v1', 'fresh output')
    require(stdout['published_particle'] == PARTICLE, 'published compiler particle')
    verification = stdout['verification']
    require(verification['compiler_job']['status'] == 'success'
            and verification['compiler_job']['compiled_particle'] == PARTICLE, 'successful compiler JOB1')
    require(verification['physical_resource_claim'] == 'unattested', 'physical claim scope')
    for name, value in receipt['accepted_execution_coordinates'].items():
        require(verification[name] == value, f'verified coordinates: {name}')
    require(receipt['accepted_execution_coordinates'] == producer['accepted_execution_coordinates'],
            'producer/verifier logical coordinates')
    require(receipt['command'][0] == producer['command'][0] and receipt['command'][1] == 'verify-artifact',
            'fresh actual command')
    return receipt


def check_corpus(payload, index, generation, binary, compiler, prepared):
    prefix = f'corpus/c{generation}'
    receipt = json.loads(payload[f'{prefix}/receipt.json'])
    require(receipt['schema'] == 'trident/proof-extracted-compiler-corpus/v2'
            and receipt['status'] == 'passed', 'corpus verdict')
    require(receipt['generation'] == generation and receipt['proof_generation'] == generation - 1,
            'corpus generation')
    require(receipt['fixture_revision'] == FIXTURE and receipt['joy_fixture_revision'] == JOY_REVISION,
            'corpus source pins')
    require(receipt['observations'] == 547 and len(receipt['commands']) == 6, '547 corpus cardinality')
    require(receipt['runtime_environment'] == {'PATH': '', 'PYTHONUTF8': '1',
            'PYTHONDONTWRITEBYTECODE': '1', 'TRIDENT_AUDIT_GIT': '/usr/bin/git'}, 'runtime environment')
    require(receipt['limits'] == {'per_corpus_wall_seconds': 5400,
            'evidence_files': 50000, 'evidence_bytes': 4294967296}, 'corpus limits')
    require(receipt['inputs_before'] == receipt['inputs_after'] and len(receipt['inputs_before']) == 6,
            'six immutable corpus inputs')
    by_original = {row['original']: name for name, row in index.items()}
    for path, row in receipt['inputs_before'].items():
        require(path in by_original and identity(payload[by_original[path]]) == row,
                f'corpus actual input: {path}')
    require(receipt['driver'] == identity(payload['corpus/run.py']), 'actual corpus driver')
    runner = next(name for name in prepared if name.endswith('/bootstrap-runner.py'))
    require(receipt['original_runner'] == identity(payload[f'reviewed/{runner}']), 'original bootstrap runner')
    fresh_prefix = f'fresh/c{generation - 1}'
    require(receipt['verified_receipt'] == identity(payload[f'{fresh_prefix}/receipt.json']),
            'fresh receipt binding')
    files = json.loads(payload[f'{prefix}/files.json'])
    require(identity(payload[f'{prefix}/files.json']) == expected(receipt['files']), 'original file index')
    actual = {name.removeprefix(prefix + '/'): expected(row)
              for name, row in index.items() if name.startswith(prefix + '/')}
    require(set(actual) == set(files) | {'receipt.json', 'files.json'}, 'complete corpus membership')
    for name, row in files.items():
        require(actual[name] == row, f'corpus retained file: {name}')
    compiler_path = index[f'{fresh_prefix}/compiler.dag']['original']
    joy_path = index['inputs/joy']['original']
    input_names = [f'{fresh_prefix}/compiler.dag', 'inputs/joy', f'{fresh_prefix}/receipt.json',
                   f'{fresh_prefix}/stdout', 'inputs/compiler_vectors.json', 'inputs/git']
    require(receipt['inputs_before'] == {index[name]['original']: identity(payload[name])
            for name in input_names}, 'generation-specific six inputs')
    require(receipt['inputs_before'][compiler_path] == compiler
            and receipt['inputs_before'][joy_path] == binary, 'selected input identities')
    require(set(receipt['corpora']) == {f'c{generation}-{name}' for name in CASES}, 'six named corpora')
    scope = index[f'{prefix}/receipt.json']['original'].removesuffix('/receipt.json')
    for position, (name, (count, command_count)) in enumerate(CASES.items()):
        report_name = f'corpora/c{generation}-{name}.json'
        report_data = payload[f'{prefix}/{report_name}']
        row = receipt['corpora'][f'c{generation}-{name}']
        require(row['path'] == report_name and expected(row) == identity(report_data), 'report identity')
        command = receipt['commands'][position]
        require(command['status'] == 'completed' and command['exit_code'] == 0, 'child completion')
        require(command['cwd'] == receipt['fixture_repository'], 'child working directory')
        argv = command['command']
        require(argv[1:] == [receipt['fixture_repository'] + '/audit/self-hosting/bootstrap-runner.py',
            '--corpus-child', receipt['fixture_repository'] + f'/audit/self-hosting/{name}.py',
            '--retain', scope + f'/corpora/c{generation}-{name}-files', '--joy', joy_path,
            '--compiler', compiler_path, '--output', scope + '/' + report_name], 'child exact argv')
        for channel in ('stdout', 'stderr'):
            require(command[channel] == f'commands/{position}.{channel}'
                    and f'{prefix}/{command[channel]}' in payload, 'child raw output')
        report = json.loads(report_data)
        require(report['status'] == 'passed' and report['compiler_mode'] == 'provided', 'provided compiler verdict')
        require(report['compiler_path'] == compiler_path, 'provided compiler path')
        require(report['compiler_sha256_start'] == report['compiler_sha256_end'] == COMPILER,
                'provided compiler digest')
        require(report.get('binary_sha256', report.get('binary_sha256_start')) == JOY, 'corpus Joy digest')
        for key in ('binary_sha256_start', 'binary_sha256_end'):
            if key in report:
                require(report[key] == JOY, 'unchanged corpus Joy')
        require(len(report['observations']) == count and len(report['commands']) == command_count,
                f'corpus counts: {name}')
        check_flags(report['observations'])
        for inner in report['commands']:
            if 'expected_exit' in inner:
                require(inner['exit_code'] == inner['expected_exit'], 'expected command exit')
        if name == 'check-generated-compiler-profile':
            require(report['revision'] == FIXTURE and report['working_tree'] == '', 'metadata Git source state')
            require(report['metadata_git'] == {'path': '/usr/bin/git',
                    'sha256': identity(payload['inputs/git'])['sha256']}, 'metadata Git binary')
            require(report['commands'][0]['command'] == ['/usr/bin/git', 'rev-parse', 'HEAD']
                    and report['commands'][0]['stdout'] == FIXTURE + '\n', 'actual metadata invocation')
    return {'generation': generation, 'proof_generation': generation - 1, 'observations': 547,
            'corpora': 6, 'compiler': compiler, 'receipt': identity(payload[f'{prefix}/receipt.json'])}


def check_relations(payload, index, manifest, prepared):
    require(manifest['fixture_revision'] == FIXTURE and manifest['joy_revision'] == JOY_REVISION, 'delivery pins')
    corpus_files = [name for name in payload if name.startswith('corpus/')]
    require(len(corpus_files) == manifest['complete_original_files']
            and sum(len(payload[name]) for name in corpus_files) == manifest['complete_original_bytes'],
            'complete original scope accounting')
    compiler = identity(payload['fresh/c1/compiler.dag'])
    binary = identity(payload['inputs/joy'])
    require(compiler == identity(payload['fresh/c2/compiler.dag'])
            and compiler['sha256'] == COMPILER and compiler['bytes'] == 9691488, 'C2/C3 artifact identity')
    require(binary['sha256'] == JOY and binary['bytes'] == 5635776, 'exact Joy bytes')
    install_data = payload['inputs/joy-postinstall.json']
    install = json.loads(install_data)
    require(install['exit'] == 0 and install['warnings'] == [] and install['inputs']['joy'] == JOY_REVISION
            and install['binary_sha256'] == JOY and install['binary_size'] == binary['bytes'], 'installed source binding')
    review = json.loads(payload['corpus/independent-review.json'])
    sources = json.loads(payload['corpus/sources.json'])
    require(review['status'] == 'passed-source-review' and review['sources'] == sources, 'source review')
    by_original = {row['original']: f'reviewed/{name}' for name, row in prepared.items()}
    for original, row in sources.items():
        require(identity(payload[by_original[original]]) == row, 'reviewed executable source')
        for name in ('run.py', 'run_ready.py'):
            if original == index[f'corpus/{name}']['original']:
                require(identity(payload[f'corpus/{name}']) == row, 'executed wrapper source')
    orchestration = json.loads(payload['corpus/orchestration/receipt.json'])
    require(orchestration['status'] == 'passed' and orchestration['fixture_revision'] == FIXTURE,
            'orchestration verdict')
    require(orchestration['reviewed_sources'] == sources
            and orchestration['driver_sha256'] == identity(payload['corpus/run.py'])['sha256']
            and orchestration['independent_review_sha256'] == identity(payload['corpus/independent-review.json'])['sha256'],
            'orchestration source identity')
    require([(row['proof_generation'], row['compiler_generation']) for row in orchestration['generations']]
            == [(2, 3), (1, 2)], 'orchestration generation order')
    for row in orchestration['generations']:
        require(row['status'] == 'passed' and row['exit_code'] == 0, 'orchestration child exit')
        child = json.loads(payload[f"corpus/c{row['compiler_generation']}/receipt.json"])
        require(row['command'][1:] == ['-B', index['corpus/run.py']['original'],
                '--generation', str(row['proof_generation']), '--fixtures', child['fixture_repository'],
                '--revision', FIXTURE], 'orchestration actual invocation')
        require(row['started_ns'] <= child['started_ns'] < child['ended_ns'] <= row['ended_ns'],
                'orchestration actual timing')
    for generation in (1, 2):
        check_fresh(payload, generation, compiler, binary, identity(install_data))
    results = [check_corpus(payload, index, generation, binary, compiler, prepared) for generation in (2, 3)]
    return {'generations': results,
            'scope': 'Integrity/receipt replay of completed corpus measurements; no new execution or SH8 acceptance.'}
