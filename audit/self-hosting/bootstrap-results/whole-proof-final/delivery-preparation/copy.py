"""Copy only explicit reviewed small preparation originals into a fresh local packet."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import time

ROOT=Path(__file__).resolve().parent
BASE=ROOT.parent
SOURCE=BASE/'whole-proof-final-delivery-preparation'
SOURCE_MAP='d2f216163f07add07ef7278d998656eccb2f1f60190322e085761191fa64f59f'
ROOT_REVIEW='29360024cf5ba6736599c53477b242af5ad3153da99cddf3d19464beb944b7c7'
PEER_REVIEW='f0c3d06ff103e83ed469f92bfdf2092682de267232d184f4bf358d9b0e4e3e38'
VERIFIER='7084b3e31fff02d78533e48998ac2994afc1cbd2744a6586fe9335315ceae446'
CAPS=dict(wall_seconds=60,individual_read_bytes=8*1024**2,total_read_bytes=32*1024**2,output_bytes=8*1024**2)


def require(ok,message):
    if not ok:raise ValueError(message)


def identity(data):
    return dict(bytes=len(data),sha256=hashlib.sha256(data).hexdigest())


def encoded(value):return (json.dumps(value,indent=2,sort_keys=True)+'\n').encode()


class IO:
    def __init__(self):self.start=time.monotonic();self.read_bytes=0;self.write_bytes=0
    def tick(self):require(time.monotonic()-self.start<=CAPS['wall_seconds'],'staging wall cap')
    def read(self,path):
        self.tick();path=Path(path)
        require(path.is_absolute() and '..' not in path.parts,'exact absolute original')
        require(path.is_relative_to(SOURCE) or path.is_relative_to(ROOT)
                or path.is_relative_to(BASE/'whole-proof-final-delivery-independent-review'),'declared owned original scope')
        for parent in [path,*path.parents]:require(not parent.is_symlink(),'no symlink original path')
        before=path.lstat();require(stat.S_ISREG(before.st_mode) and before.st_nlink==1,'single-link regular original')
        require(before.st_size<=CAPS['individual_read_bytes'] and self.read_bytes+before.st_size<=CAPS['total_read_bytes'],'read caps')
        fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK)
        try:
            current=os.fstat(fd);fields=('st_dev','st_ino','st_size','st_nlink','st_mtime_ns','st_ctime_ns')
            stamp=lambda s:tuple(getattr(s,k) for k in fields)
            require(stamp(before)==stamp(current),'original changed before open')
            chunks=[];size=0
            while True:
                self.tick();block=os.read(fd,min(1024*1024,CAPS['individual_read_bytes']+1-size))
                if not block:break
                chunks.append(block);size+=len(block);require(size<=CAPS['individual_read_bytes'],'original grew')
            require(size==before.st_size and stamp(before)==stamp(os.fstat(fd))==stamp(path.lstat()),'stable complete original')
            self.read_bytes+=size;return b''.join(chunks)
        finally:os.close(fd)
    def write(self,name,data):
        self.tick();path=Path(name);require(not path.is_absolute() and '..' not in path.parts,'relative packet path')
        require(self.write_bytes+len(data)<=CAPS['output_bytes'],'write cap')
        destination=ROOT/'packet'/path;destination.parent.mkdir(parents=True,exist_ok=True)
        for parent in destination.parents:require(not parent.is_symlink(),'packet ancestor symlink')
        with destination.open('xb') as stream:stream.write(data);stream.flush();os.fsync(stream.fileno())
        fd=os.open(destination.parent,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW)
        try:os.fsync(fd)
        finally:os.close(fd)
        self.write_bytes+=len(data)


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--source-sha256',required=True);p.add_argument('--inputs-sha256',required=True);args=p.parse_args()
    io=IO();script=io.read(ROOT/'copy.py');manifest_raw=io.read(ROOT/'inputs.json')
    require(identity(script)['sha256']==args.source_sha256 and identity(manifest_raw)['sha256']==args.inputs_sha256,'exact proposed copy/source inputs')
    manifest=json.loads(manifest_raw);require(manifest['caps']==CAPS and manifest['destination']=='packet','fixed staging contract')
    raw_map=io.read(SOURCE/'sources.json');require(identity(raw_map)['sha256']==SOURCE_MAP,'frozen preparation map')
    source_map=json.loads(raw_map)
    for name,value in source_map.items():require(identity(io.read(SOURCE/name))==value,'frozen preparation source')
    root_review=io.read(SOURCE/'independent-review.json');peer_review=io.read(BASE/'whole-proof-final-delivery-independent-review/peer-review.json')
    for data,sha in ((root_review,ROOT_REVIEW),(peer_review,PEER_REVIEW)):
        review=json.loads(data);require(identity(data)['sha256']==sha and review['status']=='passed-source-review' and review['sources']==source_map,'exact independent source approval')
    require(json.loads(root_review)['actual_collector_authorized'] is False,'preparation-only approval')
    sys.path.insert(0,str(SOURCE));from privacy import public_bytes
    rows=manifest['public_originals'];require(len(rows)<=200 and len({r['original'] for r in rows})==len(rows),'bounded distinct originals')
    originals={};exceptions={}
    for row in rows:
        data=io.read(Path(row['original']));require(identity(data)=={k:row[k] for k in ('bytes','sha256')},'exact public original')
        public_bytes(data);originals[row['original']]=data
        options=[]
        if any(line.rstrip(b' \t')!=line for line in data.splitlines()):options.append('-blank-at-eol')
        if data.endswith(b'\n\n'):options.append('-blank-at-eof')
        if options:exceptions[row['sha256']]=','.join(options)
    for row in manifest['references_only']:
        require(identity(io.read(Path(row['path'])))=={k:row[k] for k in ('bytes','sha256')},'exact excluded original reference')
    verifier=io.read(SOURCE/'verify-retained.py');require(identity(verifier)['sha256']==VERIFIER,'exact retained verifier')
    readme=io.read(ROOT/'README.md');public_bytes(readme)
    destination=ROOT/'packet';require(not destination.exists() and not destination.is_symlink(),'fresh staging packet')
    destination.mkdir(mode=0o700);objects=set();retained=[]
    try:
        for row in rows:
            stored='objects/'+row['sha256']
            if stored not in objects:io.write(stored,originals[row['original']]);objects.add(stored)
            retained.append(dict(row,stored=stored))
        attrs='* -text\n'+''.join(f'objects/{sha} whitespace={options}\n' for sha,options in sorted(exceptions.items()))
        for name,data in [('retained-files.json',encoded(retained)),('references-only.json',encoded(manifest['references_only'])),('README.md',readme),('verify-retained.py',verifier),('inputs.json',manifest_raw),('copy.py',script),('.gitattributes',attrs.encode())]:
            public_bytes(data);io.write(name,data)
        for path,data in originals.items():require(identity(io.read(Path(path)))==identity(data),'original changed during staging')
        require(io.read(ROOT/'copy.py')==script and io.read(ROOT/'inputs.json')==manifest_raw,'copy source/selection unchanged')
        require(io.read(SOURCE/'sources.json')==raw_map and io.read(SOURCE/'independent-review.json')==root_review,'frozen source/review unchanged')
        for name,value in source_map.items():require(identity(io.read(SOURCE/name))==value,'frozen source bytes unchanged after copy')
        files={p.relative_to(destination).as_posix():identity(io.read(p)) for p in destination.rglob('*') if p.is_file()}
        receipt=dict(status='passed-preparation-staging',source=identity(script),inputs=identity(manifest_raw),files=files,original_files=len(rows),original_bytes=sum(r['bytes'] for r in rows),objects=len(objects),references_only=len(manifest['references_only']),caps=CAPS,read_bytes=io.read_bytes,staged_bytes_before_receipt=io.write_bytes,elapsed_seconds=time.monotonic()-io.start,scope='Preparation archive only; no actual collector, native/proof workload, helper mutation or acceptance.')
        io.write('copy-receipt.json',encoded(receipt));print(json.dumps({k:receipt[k] for k in ('status','original_files','objects','references_only')}))
    except BaseException as error:
        try:io.write('failed.json',encoded(dict(status='failed-preparation-staging',exception_type=type(error).__name__)))
        except BaseException:pass
        raise


if __name__=='__main__':main()
