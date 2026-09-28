#!/usr/bin/env python3
"""Bind full-project package invariance to one actual successful guest execution."""
import argparse
import gzip
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[3]
FIXED_PATH = ROOT / 'audit/self-hosting/check-selfhost-fixed-point.py'
SPEC = importlib.util.spec_from_file_location('fixed_point', FIXED_PATH)
FIXED = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FIXED)
SCOPE = ('full-package identity/relocation/reorder only; exact JOB1 equality binds '
         'the original successful execution; no new guest compilation or C2 usability claim')


def require(condition, message):
    FIXED.require(condition, message)


def sha(path):
    return FIXED.sha_file(path)


def preflight(path, expected_sha, joy, inventory_checker):
    receipt, receipt_sha = FIXED.read_json(path)
    require(receipt_sha == expected_sha, 'original receipt SHA256 differs')
    require(receipt['status'] == 'compiler-returned' and receipt['published_kind'] == 'program',
            'original receipt did not publish a program')
    compiler = Path(receipt['compiler'])
    require(sha(compiler) == receipt['compiler_sha256'] == receipt['compiler_sha256_end'],
            'original compiler SHA256 differs')
    require(sha(joy) == receipt['binary_sha256'] == receipt['binary_sha256_end'],
            'Joy binary SHA256 differs')
    require(inventory_checker.is_file(), 'inventory checker missing')
    return receipt


def command(argv, cwd, report):
    row = dict(command=[str(x) for x in argv], cwd=str(cwd), exit_code=None,
               stdout='', stderr='', status='running')
    report['commands'].append(row)
    result = subprocess.run(row['command'], cwd=cwd, text=True, capture_output=True,
                            check=False, timeout=60)
    row.update(exit_code=result.returncode, stdout=result.stdout, stderr=result.stderr,
               status='completed')
    require(result.returncode == 0, 'audit command failed')
    return row


def pack(joy, compiler, manifest, output, temporary, host, report):
    before = sha(joy), sha(compiler)
    row = command([joy, 'pack-job', '--compiler', compiler, '--manifest', manifest,
                   '-o', output, *host], temporary, report)
    require(before == (sha(joy), sha(compiler)), 'tool/compiler changed during pack')
    admitted = json.loads(row['stdout'])
    require(admitted['ok'] is True and admitted['schema'] == 'joy/job-pack/v1',
            'Joy package admission failed')
    data = output.read_bytes()
    require(data[:8] == b'NOXDAG01' and data[8:40].hex() == admitted['package']['job_particle'],
            'packed bytes/particle framing differs')
    return data, admitted


def comment_mutation(content):
    # Change one letter after // on its own comment line; retain every byte count.
    match = re.search(rb'(?m)^[ \t]*//[^\r\n]*?[A-Za-z]', content)
    require(match is not None, 'dependency has no ASCII comment letter')
    offset = match.end() - 1
    changed = content[:offset] + bytes([content[offset] ^ 32]) + content[offset + 1:]
    require(len(changed) == len(content) and changed != content, 'comment mutation failed')
    return changed, dict(offset=offset, before_byte=content[offset], after_byte=changed[offset])


def archive_inputs(temporary, output):
    # Sources/manifests are retained. Duplicate compiler/JOB1 bytes stay bound to
    # the original receipt rather than being copied into this small archive.
    with output.open('xb') as raw:
        with gzip.GzipFile(fileobj=raw, mode='wb', mtime=0, filename='') as compressed:
            with tarfile.open(fileobj=compressed, mode='w|') as archive:
                for path in sorted(temporary.rglob('*')):
                    if not path.is_file() or path.suffix not in ('.tri', '.json'):
                        continue
                    data = path.read_bytes()
                    info = tarfile.TarInfo(str(path.relative_to(temporary)))
                    info.size, info.mode, info.mtime = len(data), 0o644, 0
                    archive.addfile(info, io.BytesIO(data))


def inspect(args, report, archive):
    receipt = preflight(args.receipt, args.receipt_sha256, args.joy, args.inventory_checker)
    report.update(original_receipt=str(args.receipt), original_receipt_sha256=args.receipt_sha256,
                  compiler_sha256=receipt['compiler_sha256'], binary_sha256=sha(args.joy),
                  inventory_checker_sha256_start=sha(args.inventory_checker),
                  job_checker=str(args.joy), job_checker_sha256_start=sha(args.joy),
                  fixed_point_checker_sha256_start=sha(FIXED_PATH),
                  inventory_checks=[], job_checks=[], steps=[])
    original = FIXED.step(args.receipt, args.inventory_checker, report)
    baseline_job = (Path(receipt['artifact_directory']) / 'job.dag').read_bytes()
    report['baseline_execution'] = dict(status='success', published_kind='program',
                                      **original['summary'])
    with tempfile.TemporaryDirectory(prefix='trident-full-package-relocated-') as raw_temporary:
        temporary = Path(raw_temporary).resolve()
        report['independent_directory'] = str(temporary)
        require(not temporary.is_relative_to(ROOT) and
                not temporary.is_relative_to(Path(receipt['artifact_directory'])),
                'relocation did not leave original directories')
        compiler = temporary / 'compiler.dag'
        compiler.write_bytes(original['compiler'])
        copies = {}
        for name, source in original['sources'].items():
            target = temporary / 'source-root' / source['path']
            target.parent.mkdir(parents=True, exist_ok=True)
            content = Path(receipt['sources'][name]['copy']).read_bytes()
            require(hashlib.sha256(content).hexdigest() == source['sha256'], 'source changed during relocation')
            target.write_bytes(content)
            copies[name] = dict(source, archive_path=str(target.relative_to(temporary)))
        report['relocated_sources'] = copies
        command([args.inventory_checker, '--root', temporary / 'source-root', '--entry',
                 'compiler/nox/main.tri', '--output', receipt['inventory'], '--check'], temporary, report)
        manifest = dict(receipt['manifest'])
        manifest['modules'] = [dict(module, file=copies[module['logical_path']]['archive_path'])
                               for module in reversed(receipt['manifest']['modules'])]
        require([m['logical_path'] for m in manifest['modules']] !=
                [m['logical_path'] for m in receipt['manifest']['modules']], 'module order did not change')
        relocated_manifest = temporary / 'relocated-reversed.json'
        relocated_manifest.write_text(json.dumps(manifest, indent=2) + '\n')
        data, admission = pack(args.joy, compiler, relocated_manifest, temporary / 'relocated-job.dag',
                               temporary, receipt['host_flags'], report)
        require(data == baseline_job, 'relocated/reordered JOB1 bytes differ')
        require(admission['package'] == receipt['admission']['package'], 'relocated admission metadata differs')
        report['relocated_reversed'] = dict(exact_job_bytes_equal=True,
            job_sha256=hashlib.sha256(data).hexdigest(), job_bytes=len(data),
            package_particle=admission['package']['package_particle'],
            admission=admission, manifest=manifest,
            consequence='same compiler bytes and canonical JOB1 as the completed original execution; no rerun claimed')
        dependency = 'std.nox.bytes'
        old = temporary / copies[dependency]['archive_path']
        changed, change = comment_mutation(old.read_bytes())
        changed_path = temporary / 'changed-source' / copies[dependency]['path']
        changed_path.parent.mkdir(parents=True, exist_ok=True)
        changed_path.write_bytes(changed)
        mutated = dict(receipt['manifest'])
        mutated['modules'] = [dict(module, file=str(changed_path.relative_to(temporary))
                                  if module['logical_path'] == dependency else
                                  copies[module['logical_path']]['archive_path'])
                              for module in receipt['manifest']['modules']]
        mutated_manifest = temporary / 'changed-dependency.json'
        mutated_manifest.write_text(json.dumps(mutated, indent=2) + '\n')
        changed_job, changed_admission = pack(args.joy, compiler, mutated_manifest,
            temporary / 'changed-job.dag', temporary, receipt['host_flags'], report)
        package = changed_admission['package']
        before = receipt['admission']['package']
        require(changed_job != baseline_job and package['job_particle'] != before['job_particle'] and
                package['package_particle'] != before['package_particle'], 'dependency change did not change package/job identity')
        old_modules = {m['logical_path']: m for m in before['modules']}
        for module in package['modules']:
            previous = old_modules[module['logical_path']]
            if module['logical_path'] == dependency:
                require(module['source_particle'] != previous['source_particle'] and
                        module['particle'] != previous['particle'], 'dependency identities unchanged')
            else:
                require(module == previous, 'unmodified module identity changed')
        require(len(package['modules']) == len(old_modules), 'mutated module set differs')
        for key in ['limits', 'options', 'entry_module', 'entry_function', 'compiler_particle']:
            require(package[key] == before[key], 'mutation changed ' + key)
        report['changed_dependency'] = dict(module=dependency, edit=change,
            original_sha256=copies[dependency]['sha256'], changed_sha256=sha(changed_path),
            byte_length_unchanged=len(changed), source_limit_unchanged=True,
            job_sha256=hashlib.sha256(changed_job).hexdigest(), job_bytes=len(changed_job),
            admission=changed_admission, manifest=mutated,
            other_module_identities_unchanged=True, compiler_execution='not performed')
        archive_inputs(temporary, archive)
        report['archive'] = dict(path=str(archive), sha256=sha(archive), bytes=archive.stat().st_size,
                               scope='94 relocated exact sources, one changed dependency, two manifests; duplicate compiler/JOB1 omitted')
    preflight(args.receipt, args.receipt_sha256, args.joy, args.inventory_checker)
    require(sha(args.inventory_checker) == report['inventory_checker_sha256_start'], 'inventory checker changed')
    require(sha(FIXED_PATH) == report['fixed_point_checker_sha256_start'], 'fixed-point checker changed')
    require(sha(Path(receipt['artifact_directory']) / 'job.dag') == receipt['job_sha256'], 'original JOB1 changed')
    require(sha(Path(receipt['artifact_directory']) / 'result.dag') == receipt['result_sha256'], 'original emitted artifact changed')
    report['status'] = 'passed'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['receipt', 'joy', 'inventory-checker', 'output']:
        parser.add_argument('--' + key, type=Path, required=True)
    parser.add_argument('--receipt-sha256', required=True)
    args = parser.parse_args()
    proposed_archive = args.output.with_suffix('.inputs.tar.gz')
    if args.output.exists() or args.output.is_symlink() or proposed_archive.exists() or proposed_archive.is_symlink():
        parser.error('choose fresh receipt/archive paths; existing evidence is preserved')
    for key in ['receipt', 'joy', 'inventory_checker', 'output']:
        setattr(args, key, getattr(args, key).resolve())
    archive = args.output.with_suffix('.inputs.tar.gz')
    if args.output.exists() or args.output.is_symlink() or archive.exists() or archive.is_symlink():
        parser.error('choose fresh receipt/archive paths; existing evidence is preserved')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    report = dict(schema='trident/full-package-determinism/v1', status='running', scope=SCOPE,
                  invocation=sys.argv, script_sha256=sha(Path(__file__)), commands=[])
    with args.output.open('x') as output:
        try:
            inspect(args, report, archive)
        except Exception as error:
            report.update(status='rejected', error=dict(kind=type(error).__name__, message=str(error)))
        finally:
            json.dump(report, output, indent=2)
            output.write('\n')
    print(json.dumps(dict(status=report['status'], receipt=str(args.output), scope=SCOPE)))
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    sys.exit(main())
