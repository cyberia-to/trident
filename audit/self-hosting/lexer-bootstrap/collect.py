#!/usr/bin/env python3
"""Retain completed local S1 measurements; never run a compiler or change sources."""
from pathlib import Path
import datetime, gzip, hashlib, io, json, subprocess, sys, tarfile
if sys.flags.optimize:
    raise RuntimeError('archive verification requires unoptimized Python')
A = Path(__file__).resolve().parent
R = A.parents[3]
M = R / 'measurements'
REV = '77213171d39b88c5f41221912251cc4813ac2b11'
def digest(b): return hashlib.sha256(b).hexdigest()
def identity(p):
    p = Path(p); b = p.read_bytes()
    return {'sha256': digest(b), 'bytes': len(b)}
def write(p, b):
    p.parent.mkdir(parents=True, exist_ok=True)
    if p.exists(): assert p.read_bytes() == b, str(p)
    else: p.write_bytes(b)
def dump(p, obj): write(p, (json.dumps(obj, indent=2) + '\n').encode())
def copy(src, dest):
    src = Path(src); b = src.read_bytes(); write(A / dest, b)
    return {'source': str(src), 'path': dest, **identity(src)}
def archive(files, dest):
    buffer = io.BytesIO(); rows = []
    with tarfile.open(fileobj=buffer, mode='w') as tar:
        for name, p in sorted(files.items()):
            b = p.read_bytes(); item = tarfile.TarInfo(name)
            item.size = len(b); item.mode = 0o644; tar.addfile(item, io.BytesIO(b))
            rows.append({'path': name, 'source': str(p), 'sha256': digest(b), 'bytes': len(b)})
    write(A / dest, gzip.compress(buffer.getvalue(), mtime=0))
    with tarfile.open(A / dest) as tar:
        assert tar.getnames() == [x['path'] for x in rows]
        for row in rows: assert digest(tar.extractfile(row['path']).read()) == row['sha256']
    return {'path': dest, **identity(A / dest), 'files': rows}
original_names = ['artifacts.json','c1-to-c2.json','c1.dag.gz','c2.dag.gz','inventory.json','job1.dag.gz','launch.json','sources.tar.gz']
original = {n: identity(A / n) for n in original_names}
build = json.loads((A / 'c1-to-c2.json').read_text())
assert (A / 'c1-to-c2.json').read_bytes() == (M / 'lexer-v9-c1-to-c2.json').read_bytes()
assert build['status'] == 'compiler-returned'
artifacts = json.loads((A / 'artifacts.json').read_text())
for name, row in artifacts['files'].items():
    assert identity(A / name)['sha256'] == row['sha256']
    if 'original_sha256' in row:
        b = (A / name).read_bytes()
        if name.endswith('.gz'): b = gzip.decompress(b)
        assert digest(b) == row['original_sha256'] and len(b) == row['original_bytes']
source_rows = []
with tarfile.open(A / 'sources.tar.gz') as tar:
    manifest_bytes = tar.extractfile('package.json').read()
    manifest = json.loads(manifest_bytes)
    assert manifest == build['manifest']
    assert manifest_bytes == (Path(build['artifact_directory']) / 'package.json').read_bytes()
    assert len(tar.getmembers()) == 95
    assert set(tar.getnames()) == {'package.json'} | {x['file'] for x in manifest['modules']}
    assert len(build['sources']) == len(manifest['modules']) == 94
    for module in manifest['modules']:
        name = module['logical_path']; row = build['sources'][name]
        b = tar.extractfile(module['file']).read()
        assert digest(b) == row['sha256'] and len(b) == row['source_bytes']
        assert b == Path(row['copy']).read_bytes()
        blob = subprocess.check_output(['git','show',REV + ':' + row['path']],cwd=A.parents[2])
        assert b == blob
        source_rows.append({'logical_path': name, 'archive_path': module['file'], 'git_path': row['path'], 'sha256': digest(b), 'bytes': len(b)})
scale_path = M / 'lexer-v9-c2-source-scale.json'; scale = json.loads(scale_path.read_text())
assert scale['status'] == 'passed' and scale['complete_corpus'] and len(scale['observations']) == 6
assert scale['compiler_sha256_start'] == scale['compiler_sha256_end'] == build['result_sha256']
assert scale['producer_receipt_sha256'] == identity(A / 'c1-to-c2.json')['sha256']
assert scale['host_flags'] == build['host_flags']
scale_receipt = copy(scale_path, 'source-scale/receipt.json')
tree = Path(scale['artifact_directory'])
scale_archive = archive({str(p.relative_to(tree)): p for p in tree.rglob('*') if p.is_file()}, 'source-scale/artifacts.tar.gz')
for row in scale['observations']:
    assert row['status'] == 'passed'
    assert identity(row['source_file'])['sha256'] == row['source_sha256']
    assert row['compiler_execution']['execution']['program_particle'] == scale['compiler_particle']
script = Path(scale['invocation'][0]); assert identity(script)['sha256'] == scale['script_sha256']
copy(script, 'source-scale/check.py')
dump(A / 'source-scale/artifacts.json', scale_archive)
pkg_path = M / 'lexer-v9-package-determinism.json'; pkg = json.loads(pkg_path.read_text())
assert pkg['status'] == 'passed' and pkg['original_receipt_sha256'] == identity(A / 'c1-to-c2.json')['sha256']
assert pkg['relocated_reversed']['exact_job_bytes_equal']
pkg_receipt = copy(pkg_path, 'package-invariance/receipt.json')
pkg_archive = copy(pkg['archive']['path'], 'package-invariance/inputs.tar.gz')
assert pkg_archive['sha256'] == pkg['archive']['sha256']
pkg_rows = []
with tarfile.open(A / pkg_archive['path']) as tar:
    for member in tar.getmembers():
        assert member.isfile()
        b = tar.extractfile(member).read(); pkg_rows.append({'path': member.name, 'sha256': digest(b), 'bytes': len(b)})
    for name, row in pkg['relocated_sources'].items():
        b = tar.extractfile(row['archive_path']).read()
        assert digest(b) == build['sources'][name]['sha256'] == row['sha256']
dump(A / 'package-invariance/artifacts.json', {'archive':pkg_archive,'files':pkg_rows})
pkg_script = A.parents[2] / pkg['invocation'][0]
assert identity(pkg_script)['sha256'] == pkg['script_sha256']
copy(pkg_script, 'package-invariance/check.py')
install_dir = M / 'lexer-frame'; install = json.loads((install_dir / 'candidate-install.json').read_text())
install_files = [install_dir / 'candidate-install.json', install_dir / 'install-cargo-metadata.json']
install_files += [Path(c['retained_log']['path']) for c in install['commands']]
install_files += [Path(d['diff']['path']) for d in install['post_build_dependency_state']]
install_rows = []
for p in install_files:
    if p.suffix == '.gz': install_rows.append(copy(p, 'install/' + p.name))
    elif p.name == 'candidate-install.json': install_rows.append(copy(p, 'install/receipt.json'))
    else:
        dest = 'install/' + p.name + '.gz'; b = p.read_bytes()
        write(A / dest, gzip.compress(b, mtime=0))
        install_rows.append({'source':str(p),'path':dest,**identity(A / dest),'raw_sha256':digest(b),'raw_bytes':len(b)})
for c in install['commands']:
    b = gzip.decompress((A / 'install' / Path(c['retained_log']['path']).name).read_bytes())
    assert digest(b) == c['raw_log']['sha256']
dump(A / 'install/files.json', {'files':install_rows,'qualification':'Local installation; lens contains unrelated dirty files. Dependency state was captured after builds. Candidate installation was not used for the retained C1(S) run, which pins install-compiler-work-budget.'})
assert original == {n: identity(A / n) for n in original_names}
result = {'schema':'trident/lexer-bootstrap-archive/v1','status':'passed','source_revision':REV,'source_count':94,'source_bytes':sum(r['bytes'] for r in source_rows),'source_archive_members':95,'source_blobs_verified':True,'producer_source_copies_verified':True,'manifest_verified':True,'original_files_preserved':original,'sources':source_rows,'source_scale':{'receipt':scale_receipt,'archive_sha256':scale_archive['sha256'],'file_count':len(scale_archive['files']),'passed_cases':6},'package_invariance':{'receipt':pkg_receipt,'archive':pkg_archive,'file_count':len(pkg_rows)},'install':{'receipt_sha256':identity(A / 'install/receipt.json')['sha256'],'retained_files':len(install_rows)},'scope':'Completed local first build and SH4 source/package checks only. S1 C2 semantic corpus and C2 to C3 validation are ongoing and unclaimed here.'}
dump(A / 'validation.json', result)
print(json.dumps({k:v for k,v in result.items() if k not in ['sources','original_files_preserved']},indent=2))
