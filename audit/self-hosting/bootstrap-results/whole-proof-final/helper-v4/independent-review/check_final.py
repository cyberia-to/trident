"""Read-only exact helper, build and bounded actual mutation evidence replay."""
from pathlib import Path
import hashlib,json,os,re,subprocess,time
R=Path(__file__).resolve().parent.parent;O=Path(__file__).resolve().parent;N=R/'whole-proof-helper-v4';H=N/'helper';OLD=R/'whole-proof-attacks/fixture-check'
def identity(p):
 p=Path(p)
 with p.open('rb') as f:h=hashlib.file_digest(f,'sha256').hexdigest()
 return dict(bytes=p.stat().st_size,sha256=h)
def load(p):return json.loads(Path(p).read_text())
def require(ok,why):
 if not ok:raise ValueError(why)
def match(p,x):require(identity(p)==x,'identity '+str(p))
build=load(N/'build-1/receipt.json');require(build['status']=='passed','build status');sources={}
D=O/'reviewed-helper';D.mkdir()
for name,wanted in build['sources'].items():
 p=H/name;match(p,wanted);sources[str(p)]=wanted;(D/name).write_bytes(p.read_bytes())
require(set(build['sources'])=={'Cargo.toml','Cargo.lock','main.rs','frames.rs','mutation.rs','index.rs','noun.rs'},'complete helper source set')
require([x['name'] for x in build['commands']]==['rustc','cargo','fmt','test','build'],'exact build command sequence')
for c in build['commands']:
 require(c['exit_code']==0,'build command failed')
 for stream in ['stdout','stderr']:
  p=N/'build-1'/(c['name']+'.'+stream);match(p,c[stream]);require(not re.search(r'\bwarning(?:\[|:)',p.read_text(),re.I),'build warning')
require('test result: ok. 10 passed; 0 failed; 0 ignored;' in (N/'build-1/test.stdout').read_text(),'ten actual tests')
require('rustc 1.89.0 ' in (N/'build-1/rustc.stdout').read_text() and 'host: aarch64-apple-darwin' in (N/'build-1/rustc.stdout').read_text(),'actual native rust')
require('cargo 1.89.0 ' in (N/'build-1/cargo.stdout').read_text(),'actual cargo')
match(N/'target-helper/release/whole-proof-mutator',build['binary'])
for name in ['main.rs','index.rs','noun.rs','Cargo.toml','Cargo.lock']:
 require((H/name).read_bytes()==(R/'whole-proof-attacks-v3-c1/helper'/name).read_bytes(),'unchanged '+name)
fixture=load(N/'fixture-1/receipt.json');prior=load(OLD/'receipt.json');require(fixture['status']==prior['status']=='passed','actual fixture status')
require(len(fixture['cases'])==len(prior['cases'])==14 and len(fixture['commands'])==28,'exact fixture counts')
require(fixture['helper']==build['binary'] and fixture['inputs_before']==fixture['inputs_after'],'helper/input bindings')
match(N/'fixture.py',fixture['driver'])
for path,wanted in fixture['inputs_before'].items():match(Path(path),wanted)
for c in fixture['commands']:
 for stream in ['stdout','stderr']:match(N/'fixture-1'/(c['name']+'.'+stream),c[stream])
 require(c['environment']=={'PATH':'','RUST_BACKTRACE':'0'} and c['timeout_seconds']==120 and c['file_limit_bytes']==16*1024**2,'unchanged bounded fixture command')
 commands_by_name={x['name']:x for x in fixture['commands']}
require(len(commands_by_name)==28,'distinct command names')
case_results=[]
for old,new in zip(prior['cases'],fixture['cases']):
 mode=old['mode'];require(new['mode']==mode and new['status']=='passed' and new['fresh_exit']==old['expected_exit'],'ordered cases/outcomes')
 p=N/'fixture-1'/(mode+'.joysc');q=OLD/(mode+'.joysc');match(p,new['exact_old_mutant']);match(q,old['identity']);require(new['exact_old_mutant']==old['identity'],'old/new identity')
 require(p.read_bytes()==q.read_bytes(),'whole actual bounded fixture equality')
 require(commands_by_name[mode+'-construct']['exit_code']==0 and commands_by_name[mode+'-verify']['exit_code']==old['expected_exit'],'actual command outcomes')
 if p.with_suffix('.dag').exists():require(p.with_suffix('.dag').read_bytes()==q.with_suffix('.dag').read_bytes(),'sidecar equality')
 if old['expected_exit']:
  require((N/'fixture-1'/(mode+'-verify.stdout')).read_bytes()==b'','negative success output')
  require((N/'fixture-1'/(mode+'-verify.stderr')).read_bytes()==(R/'whole-proof-attacks/attempts'/('fixture-verify-'+mode)/'stderr').read_bytes(),'exact original diagnostic')
  require((N/'fixture-1'/(mode+'.result')).read_bytes()==b'protected output must remain unchanged\n','protected output')
 else:require((N/'fixture-1'/(mode+'.result')).read_bytes()==(OLD/'result.dag').read_bytes(),'positive result')
 case_results.append(dict(mode=mode,bytes=new['exact_old_mutant']['bytes'],sha256=new['exact_old_mutant']['sha256'],fresh_exit=new['fresh_exit']))
prefix=load(N/'prefix-observation-1/receipt.json');require(prefix['status']=='passed-exact-duplicate-prefix','prefix receipt');match(N/'observe-prefix.py',prefix['driver'])
require(prefix['stat_before']==prefix['stat_after'] and prefix['compared_bytes']==prefix['stat_before'][1]['st_size']==11901028947,'prefix exact length/stable stats')
require(prefix['source_prefix_sha256']==prefix['partial_sha256']=='a4108874af7f843bb86e89442c5ca4071a638d45c3d24b8aa7f20fa1f96b95ed','prefix identities')
require(prefix['original_whole_sha256']=='80212be832ce0a5caafa69d9dd346ff20d51ce20fc86892f0c7039492619bbb0','original full identity')
for name in ['coordinator','stop','quiescence']:match(N/'prefix-observation-1'/(name+'.json'),prefix[name])
for name,wanted in prefix['samples'].items():match(N/'prefix-observation-1'/name,wanted)
for path,stat in zip([prefix['original_path'],prefix['partial_path']],prefix['stat_after']):
 p=Path(path);now=p.lstat();require({k:getattr(now,k) for k in stat}==stat,'observed prefix/original still same identity')
 with p.open('rb') as f:
  require(f.read(65536)==(N/'prefix-observation-1/first-prefix.bin').read_bytes(),'first prefix sample')
  f.seek(prefix['compared_bytes']-65536);require(f.read(65536)==(N/'prefix-observation-1/last-prefix.bin').read_bytes(),'last prefix sample')
reports={str(p):identity(p) for p in [N/'build-1/receipt.json',N/'fixture-1/receipt.json',N/'fixture.py',N/'observe-prefix.py',N/'prefix-observation-1/receipt.json']}
result=dict(status='passed-read-only-evidence-replay',observed_unix_ns=time.time_ns(),sources=sources,build_binary=build['binary'],actual_fixture_cases=case_results,commands_replayed=28,original_reports=reports,scope='Rehashed exact seven helper inputs, binary, build logs and all28 actual fixture command streams; compared all14 entire bounded old/new outputs and canonical sidecars. Prefix driver/receipt/copies/current stable stats and first/last samples reviewed; full11.9GB comparison is the original root observation, not newly repeated here. No build/test/proof/native verifier rerun, network, deletion or source mutation.')
(O/'final-evidence-replay.json').write_text(json.dumps(result,indent=2)+'\n');print(identity(O/'final-evidence-replay.json'))
