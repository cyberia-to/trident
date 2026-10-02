import hashlib,json,os,shutil,subprocess,time,traceback
from pathlib import Path
r=Path(__file__).resolve().parent;h=r/'helper';o=r/'build-1';o.mkdir()
t=Path('/Users/master/.rustup/toolchains/1.89.0-aarch64-apple-darwin/bin')
env=dict(os.environ,PATH=str(t)+':/Users/master/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin',RUSTC=str(t/'rustc'),RUSTDOC=str(t/'rustdoc'),CARGO_TARGET_DIR=str(r/'target-helper'),CARGO_BUILD_JOBS='2',RUSTFLAGS='-D warnings')
def identity(p):
 with p.open('rb') as f:d=hashlib.file_digest(f,'sha256').hexdigest()
 return dict(bytes=p.stat().st_size,sha256=d)
report=dict(status='running',started_ns=time.time_ns(),commands=[],env={k:env[k] for k in ['PATH','RUSTC','RUSTDOC','CARGO_TARGET_DIR','CARGO_BUILD_JOBS','RUSTFLAGS']})
def save(): (o/'receipt.json').write_text(json.dumps(report,indent=2)+'\n')
try:
 for name,args in [('rustc',[str(t/'rustc'),'-vV']),('cargo',[str(t/'cargo'),'-vV']),('fmt',[str(t/'rustfmt'),'--edition','2021','main.rs']),('test',[str(t/'cargo'),'test','--release','--locked','--offline']),('build',[str(t/'cargo'),'build','--release','--locked','--offline'])]:
  assert shutil.disk_usage(r).free>8*1024**3
  tick=time.monotonic();row=dict(name=name,command=args,cwd=str(h),started_ns=time.time_ns());report['commands'].append(row);save()
  with (o/(name+'.stdout')).open('xb') as out,(o/(name+'.stderr')).open('xb') as err:
   p=subprocess.run(args,cwd=h,env=env,stdout=out,stderr=err,timeout=1800)
  row.update(exit_code=p.returncode,elapsed_seconds=time.monotonic()-tick,stdout=identity(o/(name+'.stdout')),stderr=identity(o/(name+'.stderr')));save()
  if p.returncode:raise ValueError(name+' failed')
 report.update(status='passed',sources={p.name:identity(p) for p in sorted(h.iterdir()) if p.is_file()},binary=identity(r/'target-helper/release/whole-proof-mutator'))
except BaseException:
 report.update(status='failed',error=traceback.format_exc());raise
finally:
 report['ended_ns']=time.time_ns();save()
print(json.dumps({k:report[k] for k in ['status','binary']}))
