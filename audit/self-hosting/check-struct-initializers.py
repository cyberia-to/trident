import argparse,hashlib,importlib.util,json,subprocess,sys,tempfile
from pathlib import Path
parser=argparse.ArgumentParser()
parser.add_argument('--joy',type=Path,required=True)
parser.add_argument('--output',type=Path,required=True)
parser.add_argument('--before',type=Path,help='Optional prior-revision witness containing source and ART1 hex')
args=parser.parse_args()
repo=Path(__file__).resolve().parents[2];root=repo.parent;binary=args.joy.resolve()
sys.path.insert(0,str(repo/'audit/self-hosting'))
spec=importlib.util.spec_from_file_location('native_acceptance',repo/'audit/self-hosting/run-native-compiler.py');mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
commands=[];observations=[]
def run(args,expected=0):
 cmd=[str(binary),*map(str,args)];r=subprocess.run(cmd,cwd=repo,capture_output=True,text=True);commands.append({'command':cmd,'exit_code':r.returncode,'stdout':r.stdout,'stderr':r.stderr});assert r.returncode==expected,commands[-1];return r
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
with tempfile.TemporaryDirectory(prefix='trident-struct-fields-') as tmp:
 work=Path(tmp);zero=work/'zero.dag';vectors=json.loads((root/'joy/cli/tests/compiler_vectors.json').read_text());zero.write_bytes(bytes.fromhex(vectors['files']['zero']))
 prior=None
 for index,init in enumerate(['Point{x,y:9}','Point{y:9,x}']):
  source=f'program sample struct Point{{x:Field,y:Field}} fn main(input:Noun)->Noun{{let x=7 let p={init} nox_noun_atom(p.x*10+p.y)}}'
  path=work/f'valid{index}.tri';path.write_text(source);program=work/f'valid{index}.dag'
  run(['build',path,'--emit','artifact','--artifact-profile','raw','-o',program])
  output=work/f'output{index}.dag';run(['run-artifact',program,'--input',zero,'-o',output]);assert mod.decode(output)==79
  if prior is not None: assert program.read_bytes()==prior
  prior=program.read_bytes();observations.append({'case':f'valid-{index}','source_hex':source.encode().hex(),'expected':79,'program_particle':prior[8:40].hex()})
 for index,init in enumerate(['Point{x:7,y:9,x:11}','Point{x:7,x:true,y:9}','Point{x,y:9,x}']):
  source=f'program sample struct Point{{x:Field,y:Field}} fn main(input:Noun)->Noun{{let x=7 let p={init} input}}'
  path=work/f'duplicate{index}.tri';path.write_text(source);protected=work/f'protected{index}.dag';protected.write_bytes(prior)
  r=run(['build',path,'--emit','artifact','--artifact-profile','raw','-o',protected,'--force'],expected=1)
  assert "duplicate field 'x' in struct init" in r.stderr+r.stdout
  assert protected.read_bytes()==prior
  observations.append({'case':f'duplicate-{index}','source_hex':source.encode().hex(),'previous_program_preserved':True})
 before=None
 if args.before:
  before=json.loads(args.before.read_text());assert before['exit_code']==0
  old=work/'before.dag';old.write_bytes(bytes.fromhex(before['program_hex']));output=work/'old-output.dag'
  run(['run-artifact',old,'--input',zero,'-o',output]);assert mod.decode(output)==before['expected']
  before.update(program_sha256=sha(old),program_particle=old.read_bytes()[8:40].hex(),executed_value=mod.decode(output))
 compiler=work/'compiler.dag';run(['build','compiler/nox/main.tri','--emit','artifact','--artifact-profile','compiler-job','-o',compiler])
 accepted=json.loads((repo/'audit/self-hosting/native-tuples-cli.json').read_text());assert sha(compiler)==accepted['compiler_sha256']
 result={'schema':'trident/unique-struct-initializers-cli/v1','kind':'local-development','binary_sha256':sha(binary),'commands':commands,'observations':observations,'seed_before':before,'compiler_sha256':sha(compiler),'compiler_particle':compiler.read_bytes()[8:40].hex(),'entire_c1_artifact_unchanged':True,'c1_baseline':'audit/self-hosting/native-tuples-cli.json'}
args.output.parent.mkdir(parents=True,exist_ok=True)
args.output.write_text(json.dumps(result,indent=2)+'\n');print(len(commands),'commands; all checks pass; C1 unchanged')
