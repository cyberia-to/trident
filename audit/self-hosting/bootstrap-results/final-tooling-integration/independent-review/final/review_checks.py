"""Read-only staged integration identity and retained receipt replay."""
from pathlib import Path
import hashlib,json,re,subprocess,sys,time
BASE=Path(__file__).resolve().parent
R=BASE/'trident';O=BASE/'independent-review/final';O.mkdir(parents=True,exist_ok=False)
def git(*args):return subprocess.check_output(['git',*args],cwd=R)
def identity(raw):return dict(bytes=len(raw),sha256=hashlib.sha256(raw).hexdigest())
def fileid(path):return identity(path.read_bytes())
def tree(ref):
 return {p.decode():(m.split()[0].decode(),m.split()[2].decode()) for m,p in (x.split(b'\t',1) for x in git('ls-tree','-rz','--full-tree',ref).split(b'\0') if x)}
index={}
rawindex=git('ls-files','--stage','-z')
for row in rawindex.split(b'\0'):
 if not row:continue
 meta,path=row.split(b'\t',1);mode,blob,stage=meta.split();assert stage==b'0';index[path.decode()]=(mode.decode(),blob.decode())
T='27228fb1b10997fecffc34c037ab63db42b08a29'
assert index==tree(T);assert not git('diff','--name-only');assert not git('ls-files','--others','--exclude-standard')
committed='dc6700ce3552e9f5e0f27d81600162f62b26382c'
parents=git('show','-s','--format=%P',committed).decode().split()
head=parents[0];assert head=='87e2909f6ab2dfc01a97add0fe0402cbd9cd838c'
assert git('rev-parse',committed+'^{tree}').decode().strip()==T
merge=parents[1:]
assert merge==['01e45e55ef9f68eaeccca270dd0bb00bc58a4bba','2f1a575a6fbc0704c268f1ae21667830a3c996df']
expected=tree(head)
for ancestor,revision in [('da0d1a59',merge[0]),('e57f2b4',merge[1])]:
 before,after=tree(ancestor),tree(revision)
 for name in set(before)|set(after):
  if before.get(name)!=after.get(name):
   if name in after:expected[name]=after[name]
   else:expected.pop(name,None)
plan='.claude/plans/native-compiler-arena.md'
different=[p for p in set(expected)|set(index) if expected.get(p)!=index.get(p)];assert different==[plan] or sorted(different)==[plan]
claude=[p for p in index if p.startswith('.claude/') and p.endswith('.md')]
claude_lines=sum(git('show',':'+p).count(b'\n') for p in claude);assert claude_lines==1000
protected=['src','lib','compiler','catalog','silicon','tests','Cargo.toml','Cargo.lock','build.rs']
oldraw=git('ls-tree','-rz','--full-tree','bbcd7e455af1a92c6f2bb971dd3470325fad922f','--',*protected)
newraw=git('ls-tree','-rz','--full-tree',T,'--',*protected);assert oldraw==newraw
assert identity(newraw)==dict(bytes=56915,sha256='a63b62baf01a096dabf4a56d56fce71fb5e20c37cd4c60f92dc4fe4a40650b52')
for name in ['.github/workflows/selfhost-bootstrap.yml','.github/workflows/selfhost-bootstrap-repeat.yml','audit/self-hosting/bootstrap-runner.py','audit/self-hosting/bootstrap-phases.py','audit/self-hosting/bootstrap_phase_checks.py']:
 assert git('show',':'+name)==git('show','bbcd7e45:'+name),name
for name in ['audit/self-hosting/check-generated-compiler-profile.py','audit/self-hosting/test_generated_profile_metadata.py','audit/self-hosting/test_compiler_selection.py']:
 assert git('show',':'+name)==git('show',merge[1]+':'+name),name
for directory in ['precommit','rust-source-and-check']:
 original=BASE/directory; receipt=json.loads((original/'receipt.json').read_text());assert receipt['status']=='passed' and receipt['index_tree']==T
 paths=['receipt.json'];
 if directory=='precommit':
  for cmd in receipt['commands']:
   assert cmd['exit_code']==0
   for ch in ['stdout','stderr']:
    name=cmd['name']+'.'+ch;assert fileid(original/name)==cmd[ch],name;paths.append(name)
 else:
  assert receipt['exit_code']==0 and receipt['warning_lines']==[]
  for name,value in receipt['files'].items():assert fileid(original/name)==value;paths.append(name)
  assert not re.search(rb'(?im)^\s*warning(?:\[|:)',(original/'stderr').read_bytes())
  for name in ['rustc','cargo','rustdoc']:
   p=original/(name+'-version.stdout');text=p.read_text();assert '1.89.0' in text and 'aarch64-apple-darwin' in text;paths.append(p.name)
 for name in paths:
  dest=O/'retained'/directory/name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes((original/name).read_bytes())
original=R/'audit/self-hosting/whole-proof-validation/parallel-v2/delivery/inherited-rust-workspace';receipt=json.loads((original/'receipt.json').read_text())
assert fileid(original/'receipt.json')['sha256']=='4032ec0da9b1e51a5efed6bdd9cfd7f681dd2de289aa5e1d3789b9043e1a12aa'
for name,value in receipt['files'].items():assert fileid(original/name)==value,name
counts=[tuple(map(int,m)) for m in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', (original/'stdout').read_text())]
assert tuple(sum(v[i] for v in counts) for i in range(3))==(1231,0,5)
result=dict(status='passed',commit=committed,index_tree=T,head=head,merge_heads=merge,index_inventory=identity(rawindex),tracked_files=len(index),only_resolved_override=different,claude_markdown_files=len(claude),claude_lines=claude_lines,protected_inventory=identity(newraw),original_rust_counts=[1231,0,5],scope='Exact staged integration and actual retained logs; no Rust test rerun')
(O/'identity-review.json').write_text(json.dumps(result,indent=2)+'\n');(O/'index-stage.z').write_bytes(rawindex)
for name in ['.gitattributes',plan,'audit/self-hosting/check-generated-compiler-profile.py']:
 dest=O/'source'/name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(git('show',':'+name))
print(json.dumps(result,indent=2))
