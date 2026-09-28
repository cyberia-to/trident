#!/usr/bin/env python3
"""Copy the frozen S1 package for supplied-C2 compilation; see self-hosting-jobs.md."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys

COMPILER = '76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8'
PARTICLE = '2eea2ac5f611012877b4e7291a3a6f534aee7281bb358a0b8e5fabe2ac1f9fbe'
INVENTORY = 'd35d263c7f9f27cbe7ea760a34393105ae8140dc6ab84d484151b6fe04960571'
SOURCE_MAP = '3b53df97b8b9d3731c1bcccaeaedc96e46b425b788c1c6970f427fba449b9286'
MAX_FILE, MAX_FILES, MAX_KIT = 16 << 20, 32, 128 << 20
OPTIONS = dict(target=0, input_profile=1, output_profile=1, optimization=0, cfg_flags=[])
LIMITS = dict(source_bytes=370544, modules=128, diagnostics=16, sequence_length=65536,
              validation_visits=16777216, artifact_bytes=16777216, artifact_nodes=196608,
              artifact_depth=4096, reductions=20000000000, arena_nodes=1000000000,
              evaluator_frames=65536)
HOST_FLAGS = ['--arena-nodes', '1000000000', '--budget', '20000000000', '--frames',
              '65536', '--time-ms', '7200000', '--validation-visits', '16777216',
              '--resident-nodes', '3145728', '--collection-work', '10000000000']


def require(condition, message):
    if not condition:
        raise ValueError(message)


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=True).encode()


def identity(data):
    return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())


def digest(value):
    require(isinstance(value, str) and re.fullmatch('[0-9a-f]{64}', value), 'SHA256 required')
    return value


def ordinary_path(path, *, directory=False):
    path = Path(path).absolute()
    require('..' not in path.parts, 'parent path component')
    for component in [*reversed(path.parents), path]:
        require(not component.is_symlink(), f'symlink path component: {component}')
    mode = path.stat().st_mode
    require(stat.S_ISDIR(mode) if directory else stat.S_ISREG(mode), f'ordinary path required: {path}')
    return path


def read(path, maximum=MAX_FILE):
    ordinary_path(path)
    # Bound before allocating; nonblocking open prevents FIFO substitution stalls.
    flags = os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0) | getattr(os, 'O_NONBLOCK', 0)
    flags |= getattr(os, 'O_BINARY', 0)
    with os.fdopen(os.open(path, flags), 'rb') as source:
        current = os.fstat(source.fileno())
        require(stat.S_ISREG(current.st_mode) and current.st_size <= maximum, f'bounded ordinary file: {path}')
        data = source.read(maximum + 1)
    require(len(data) <= maximum, f'file grew beyond bound: {path}')
    return data


def pairs(entries):
    result = {}
    for key, value in entries:
        require(key not in result, f'duplicate JSON key: {key}')
        result[key] = value
    return result


def load(data):
    return json.loads(data, object_pairs_hook=pairs,
                      parse_constant=lambda value: require(False, f'nonfinite JSON: {value}'))


def relative(name):
    require(isinstance(name, str) and '\\' not in name, 'canonical relative path')
    parts = name.split('/')
    reserved = {'CON', 'PRN', 'AUX', 'NUL', *(f'COM{i}' for i in range(1, 10)),
                *(f'LPT{i}' for i in range(1, 10))}
    for part in parts:
        require(re.fullmatch('[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}', part) and
                not part.endswith('.') and part.split('.')[0].upper() not in reserved,
                'portable relative path')
    return Path(*parts)


def preflight(kit, source, output):
    kit, source = ordinary_path(kit, directory=True), ordinary_path(source, directory=True)
    output = Path(output).absolute()
    ordinary_path(output.parent, directory=True)
    require(not output.exists() and not output.is_symlink(), 'output must be fresh')
    require(output.name not in ('', '.', '..'), 'output directory name')
    for incoming in [kit, source]:
        require(not output.is_relative_to(incoming) and not incoming.is_relative_to(output),
                'input/output paths overlap')
        require(not any(parent.samefile(incoming) for parent in [output.parent, *output.parent.parents]),
                'physical input/output paths overlap')
    return kit, source, output


def kit_inputs(kit, expected, rehearsal):
    manifest_bytes = read(kit / 'kit.json')
    require(identity(manifest_bytes)['sha256'] == digest(expected), 'pinned kit manifest changed')
    manifest = load(manifest_bytes)
    require(manifest['schema'] == 'trident/selfhost-kit/v1', 'kit schema')
    require(manifest['status'] in (('accepted', 'rehearsal') if rehearsal else ('accepted',)),
            'accepted kit required; historical kit needs --rehearsal')
    names, total, actual, payloads = set(), len(manifest_bytes), {}, {}
    entries = manifest['files']
    require(isinstance(entries, dict) and len(entries) < MAX_FILES, 'kit file count')
    with os.scandir(kit) as scanned:
        for entry in scanned:
            require(len(names) < MAX_FILES, 'kit file count')
            require(len(relative(entry.name).parts) == 1 and entry.name.casefold() not in names,
                    'distinct flat kit paths')
            names.add(entry.name.casefold())
            if entry.name == 'kit.json':
                continue
            require(entry.name in entries, 'unlisted kit file')
            declared = entries[entry.name]
            require(set(declared) == {'bytes', 'sha256'} and type(declared['bytes']) is int and
                    0 <= declared['bytes'] <= MAX_FILE, 'kit file size')
            digest(declared['sha256'])
            total += declared['bytes']
            require(total <= MAX_KIT, 'kit total bytes')
            data = read(kit / entry.name, declared['bytes'])
            actual[entry.name] = identity(data)
            require(actual[entry.name] == declared, 'kit file identity: ' + entry.name)
            if entry.name in ('compiler.dag', 'inventory.json', 'fixed-point.json'):
                payloads[entry.name] = data
    require(actual == entries and len(names) == len(entries) + 1, 'complete kit file set')
    require(actual['compiler.dag'] == dict(bytes=9691488, sha256=COMPILER), 'frozen C2 bytes')
    require(actual['inventory.json']['sha256'] == INVENTORY, 'frozen inventory')
    require(manifest['compiler'] == dict(role='C2', particle=PARTICLE, **actual['compiler.dag']), 'C2 profile identity')
    return manifest, identity(manifest_bytes), actual, payloads


def source_inputs(source, payloads):
    inventory = load(payloads['inventory.json'])
    fixed_receipt = load(payloads['fixed-point.json'])
    require(fixed_receipt['status'] == 'passed', 'fixed-point receipt status')
    fixed = fixed_receipt['fixed_point']
    require(fixed['artifact_sha256'] == COMPILER and fixed['particle'] == PARTICLE and
            fixed['artifact_bytes'] == 9691488 and fixed['compiler_chain_bound'] is True and
            fixed['exact_artifact_bytes_equal'] is True, 'fixed-point compiler binding')
    require(canonical(fixed['options']) == canonical(OPTIONS) and
            canonical(fixed['limits']) == canonical(LIMITS), 'frozen options and limits')
    sources = fixed['source_sha256_set']
    require(identity(canonical(sources))['sha256'] == SOURCE_MAP, 'frozen source map')
    require(inventory['schema'] == 1 and inventory['roots'] == ['native_compiler'] and
            inventory['module_count'] == 94 and len(sources) == 94 and
            set(sources) == set(inventory['modules']), 'complete inventory/source map')
    result, paths, total = [], set(), 0
    for index, (name, row) in enumerate(sorted(sources.items())):
        path = relative(row['path'])
        expected = 'compiler/nox/main.tri' if name == 'native_compiler' else 'lib/' + name.replace('.', '/') + '.tri'
        require(path.as_posix() == expected == inventory['modules'][name]['path'], 'canonical source path')
        require(path.as_posix().casefold() not in paths, 'source path alias')
        paths.add(path.as_posix().casefold())
        size = row['source_bytes']
        require(type(size) is int and 0 <= size <= 65536 and
                size == inventory['modules'][name]['source_bytes'], 'source length bound')
        data = read(source / path, size)
        require(identity(data) == dict(bytes=size, sha256=row['sha256']), 'source bytes changed: ' + name)
        total += size
        require(total <= LIMITS['source_bytes'], 'total source bytes')
        result.append((dict(logical_path=name, file=f'sources/{index}.tri',
                            origin_name=row['origin_name'], origin_version=row['origin_version']), data,
                       dict(path=path.as_posix(), **identity(data))))
    require(total == inventory['source_bytes'] == LIMITS['source_bytes'], 'complete source bytes')
    return result


def write(path, data):
    with path.open('xb') as out:
        out.write(data)
    require(read(path, len(data)) == data, 'written bytes changed: ' + str(path))


def prepare(kit, source, output, expected, rehearsal=False):
    implementation = identity(read(Path(__file__)))
    kit, source, output = preflight(kit, source, output)
    manifest, kit_identity, actual, payloads = kit_inputs(kit, expected, rehearsal)
    sources = source_inputs(source, payloads)
    package = dict(version=1, entry_module='native_compiler', entry_function='main',
                   modules=[entry for entry, _, _ in sources], options=OPTIONS, limits=LIMITS)
    report = dict(schema='trident/frozen-source-package/v1', status='preparing',
                  scope='Exact byte transport preparation only; no guest compilation or acceptance',
                  kit_status=manifest['status'], rehearsal_requested=rehearsal,
                  implementation=implementation,
                  kit=dict(path=str(kit), manifest=kit_identity), source_root=str(source),
                  inputs=actual, source_map_sha256=SOURCE_MAP, host_flags=HOST_FLAGS,
                  output=str(output), sources={}, files={})
    output.mkdir()
    with (output / 'receipt.json').open('x', encoding='utf-8', newline='\n') as receipt:
        def flush():
            receipt.seek(0)
            receipt.truncate()
            json.dump(report, receipt, indent=2, sort_keys=True)
            receipt.write('\n')
            receipt.flush()
        flush()
        try:
            (output / 'sources').mkdir()
            products = [('compiler.dag', payloads['compiler.dag'])]
            for entry, data, binding in sources:
                products.append((entry['file'], data))
                report['sources'][entry['logical_path']] = dict(copy=entry['file'], **binding)
            products.append(('package.json', json.dumps(package, indent=2).encode() + b'\n'))
            for name, data in products:
                write(output / name, data)
                report['files'][name] = identity(data)
            # Bind all inputs again before announcing completion, including source bytes.
            require(kit_inputs(kit, expected, rehearsal)[1:3] == (kit_identity, actual), 'kit changed')
            source_inputs(source, payloads)
            require(identity(read(Path(__file__))) == implementation, 'preparer changed')
            require(all(identity(read(output / name, row['bytes'])) == row
                        for name, row in report['files'].items()), 'output changed')
            report['status'] = 'prepared'
        except BaseException as error:
            report.update(status='failed', error=dict(kind=type(error).__name__, message=str(error)))
            raise
        finally:
            flush()
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('kit', 'source-root', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--kit-manifest-sha256', required=True,
                        help='kit.json SHA from the independently checksum-verified distribution')
    parser.add_argument('--rehearsal', action='store_true', help='explicitly admit a historical unaccepted kit')
    args = parser.parse_args()
    try:
        report = prepare(args.kit, args.source_root, args.output, args.kit_manifest_sha256, args.rehearsal)
    except (OSError, ValueError, KeyError, TypeError, RecursionError) as error:
        print(f'error: {error}', file=sys.stderr)
        return 1
    print(json.dumps(dict(status=report['status'], kit_status=report['kit_status'],
                          modules=len(report['sources']), source_bytes=LIMITS['source_bytes'],
                          receipt=str(Path(report['output']) / 'receipt.json'))))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
