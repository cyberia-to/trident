"""Actual Git autocrlf filtering plus materialized audit replay, no repo writes."""
from pathlib import Path
import hashlib,json,subprocess,sys,tempfile,time
B=Path(__file__).resolve().parents[1];R=B/'trident';O=Path(__file__).resolve().parent/'final/crlf';O.mkdir(exist_ok=False)
T='27228fb1b10997fecffc34c037ab63db42b08a29';RESULT='audit/self-hosting/whole-proof-corpus-v2-result';PREP='audit/self-hosting/whole-proof-validation/corpus-v2'
def git(*a):return subprocess.check_output(['git',*a],cwd=R)
def ident(b):return dict(bytes=len(b),sha256=hashlib.sha256(b).hexdigest())
paths=git('ls-tree','-r','--name-only',T).decode().splitlines();names={n for n in paths if n.startswith(RESULT+'/')}
prepared=json.loads(git('show',T+':'+PREP+'/prepared-files.json'));names.update(PREP+'/'+n for n in prepared);names.update([PREP+'/prepared-files.json',PREP+'/original-corpus-v1-failed.tar.gz']);bound=set(names)
names.update(PREP+'/'+n for n in ['archive.json','original-files.json','check_delivery.py','.gitattributes']);rows=[];commands=[]
with tempfile.TemporaryDirectory(prefix='materialized-',dir=O) as tmp:
 checkout=Path(tmp)
 for name in sorted(names):
  raw=git('show',T+':'+name);argv=['git','-c','core.autocrlf=true','cat-file','--filters',T+':'+name];r=subprocess.run(argv,cwd=R,capture_output=True,check=True)
  if name in bound:assert raw==r.stdout,name
  p=checkout/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(r.stdout)
  rows.append(dict(path=name,hash_bound=name in bound,blob=ident(raw),filtered=ident(r.stdout),argv=argv,exit_code=r.returncode))
 for label,rel in [('result',RESULT+'/check_delivery.py'),('prepared',PREP+'/check_delivery.py')]:
  argv=[str(Path(sys.executable).resolve()),'-B','-W','error',str(checkout/rel)];start=time.time_ns()
  with (O/(label+'.stdout')).open('xb') as stdout,(O/(label+'.stderr')).open('xb') as stderr:r=subprocess.run(argv,cwd=checkout,stdout=stdout,stderr=stderr,timeout=60)
  commands.append(dict(label=label,argv=argv,exit_code=r.returncode,started_ns=start,ended_ns=time.time_ns()));assert r.returncode==0,label
receipt=dict(status='passed',tree=T,hash_bound_inputs=len(bound),materialized_inputs=len(names),rows=rows,commands=commands,scope='Actual Git Windows CRLF filter simulation on macOS; independent materialized corpus audit replay, not a Windows runtime measurement')
(O/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps({k:receipt[k] for k in ['status','tree','hash_bound_inputs','materialized_inputs']}))
