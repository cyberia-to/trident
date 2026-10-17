"""Compare every existing bounded mutation byte-for-byte and verify it freshly."""
import hashlib,json,os,resource,subprocess,time,traceback
from pathlib import Path
R=Path(__file__).resolve().parent;BASE=R.parent;OUT=R/'fixture-1';OLD=BASE/'whole-proof-attacks/fixture-check';PILOT=BASE/'compiler-pilot';HELPER=R/'target-helper/release/whole-proof-mutator';JOY=BASE/'production-install/installed/bin/joy'
def identity(p):
 with Path(p).open('rb') as s:h=hashlib.file_digest(s,'sha256').hexdigest()
 return dict(bytes=Path(p).stat().st_size,sha256=h)
def require(ok,msg):
 if not ok:raise ValueError(msg)
def limited():resource.setrlimit(resource.RLIMIT_FSIZE,(16*1024**2,16*1024**2))
OUT.mkdir();report=dict(status='running',started_ns=time.time_ns(),driver=identity(__file__),helper=identity(HELPER),joy=identity(JOY),commands=[],cases=[])
def save(): (OUT/'receipt.json').write_text(json.dumps(report,indent=2)+'\n')
def run(name,args,expected):
 tick=time.monotonic();row=dict(name=name,argv=[str(x) for x in args],started_ns=time.time_ns(),cwd=str(OUT),environment={'PATH':'','RUST_BACKTRACE':'0'},timeout_seconds=120,file_limit_bytes=16*1024**2);report['commands'].append(row);save()
 with (OUT/(name+'.stdout')).open('xb') as out,(OUT/(name+'.stderr')).open('xb') as err:
  p=subprocess.run(row['argv'],cwd=OUT,env=row['environment'],stdout=out,stderr=err,timeout=120,start_new_session=True,preexec_fn=limited)
 row.update(exit_code=p.returncode,elapsed_seconds=time.monotonic()-tick,stdout=identity(OUT/(name+'.stdout')),stderr=identity(OUT/(name+'.stderr')));save();require(p.returncode==expected,name+' exit')
 require(row['stdout']['bytes']<1024**2 and row['stderr']['bytes']<1024**2,name+' streams')
try:
 old=json.loads((OLD/'receipt.json').read_text());require(old['status']=='passed' and len(old['cases'])==14,'exact prior fixture')
 proof=PILOT/'attempts/prove-aggregate-1/proof.joysc';require(identity(proof)==old['source_proof'],'original fixture proof')
 require(identity(JOY)['sha256']=='8f42591ece35f192ff6f2328a8360fe0f0959f48a173248b211cd0d8f4d984f9','production Joy')
 require(report['helper']==json.loads((R/'build-1/receipt.json').read_text())['binary'],'new built helper')
 inputs=[proof,OLD/'index.json',OLD/'result.dag',PILOT/'inputs/compiler.dag',PILOT/'cases/aggregate/job.dag',JOY,HELPER]
 report['inputs_before']={str(p):identity(p) for p in inputs}
 flags=json.loads((PILOT/'attempts/verify-aggregate-result-1/receipt.json').read_text())['argv'];flags=flags[flags.index('--budget'):]
 for case in old['cases']:
  mode=case['mode'];file=OUT/(mode+'.joysc');run(mode+'-construct',[HELPER,'mutate',proof,OLD/'index.json',OLD/'result.dag',file,mode],0)
  require(identity(file)==case['identity']==identity(OLD/(mode+'.joysc')),mode+' exact old bytes')
  require(file.read_bytes()==(OLD/(mode+'.joysc')).read_bytes(),mode+' actual byte equality')
  if file.with_suffix('.dag').exists():require(file.with_suffix('.dag').read_bytes()==(OLD/(mode+'.dag')).read_bytes(),mode+' canonical sidecar equality')
  dest=OUT/(mode+'.result');code=case['expected_exit']
  if code:dest.write_bytes(b'protected output must remain unchanged\n')
  protected=identity(dest) if code else None
  argv=[JOY,'verify-artifact',PILOT/'inputs/compiler.dag','--input',PILOT/'cases/aggregate/job.dag','--proof',file,'--output',dest,*flags]
  if code:argv.append('--force')
  run(mode+'-verify',argv,code)
  if code:
   require(identity(dest)==protected,mode+' protected result')
   require((OUT/(mode+'-verify.stdout')).stat().st_size==0,mode+' no success output')
   original_error=(BASE/'whole-proof-attacks/attempts'/('fixture-verify-'+mode)/'stderr').read_bytes()
   require((OUT/(mode+'-verify.stderr')).read_bytes()==original_error,mode+' exact rejection class')
  else:require(dest.read_bytes()==(OLD/'result.dag').read_bytes(),'positive exact result')
  report['cases'].append(dict(mode=mode,status='passed',exact_old_mutant=identity(file),fresh_exit=code));save()
 report['inputs_after']={str(p):identity(p) for p in inputs};require(report['inputs_after']==report['inputs_before'],'immutable inputs unchanged')
 report['status']='passed'
except BaseException:
 report.update(status='failed',error=traceback.format_exc());raise
finally:
 report['ended_ns']=time.time_ns();save()
print(json.dumps(dict(status=report['status'],cases=len(report['cases']),commands=len(report['commands']))))
